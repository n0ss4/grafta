#![deny(clippy::all)]
// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![cfg_attr(test, allow(clippy::disallowed_methods))]
#![allow(clippy::missing_errors_doc)]

//! Builds a typed scene graph out of the raw `nodeChanges` array decoded from a
//! `.fig` file: the scene tree itself plus the design-system data that hangs off
//! it (variables, shared styles, component definitions, instance overrides).
//!
//! [`author`] goes the other way, writing nodes *into* that array — a page, a
//! frame, an instance, with ids grafta allocated. It lives here because this is
//! the crate that knows what a `FRAME` is, and because a crate that reads one
//! direction is the natural place to learn to write the same one.

pub mod author;
pub mod path;
mod convert;
mod paint;
mod types;
pub mod variables;
pub mod vector;

use figfile::FigFile;
use indexmap::IndexMap;
use kiwi::Value;
use thiserror::Error;

pub use convert::parse_variant_name;
pub use paint::{fill_to_css, VariableResolver};
pub use types::*;

use convert::Converter;
use schema::{Field, Guid, NodePhase, StackMode, Wire};

#[derive(Debug, Error)]
pub enum ModelError {
    #[error("no nodeChanges found in fig file")]
    NoNodeChanges,
}

#[derive(Debug, Default, PartialEq)]
pub struct SceneGraph {
    pub nodes: IndexMap<Guid, SceneNode>,
    /// Canvas node ids, in document order.
    pub pages: Vec<Guid>,
    pub root_id: Option<Guid>,
    pub images: Vec<(String, Vec<u8>)>,
    pub variables: IndexMap<Guid, Variable>,
    pub collections: IndexMap<Guid, VariableCollection>,
    pub styles: IndexMap<Guid, SharedStyle>,
    /// Published-library keys to node ids, read once when the graph is built,
    /// so converting one node later does not re-read the whole file. grafta
    /// authors nothing that carries a key.
    pub asset_refs: std::collections::HashMap<String, Guid>,
}

impl SceneGraph {
    #[must_use]
    pub fn node(&self, id: Guid) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    /// Insert a node and link it to its parent. Used when assembling synthetic
    /// graphs; `build_scene_graph` wires the tree up itself.
    pub fn insert(&mut self, mut node: SceneNode, parent_id: Option<Guid>) {
        node.parent_id = parent_id;
        if let Some(parent) = parent_id.and_then(|id| self.nodes.get_mut(&id)) {
            parent.children.push(node.id);
        }
        self.nodes.insert(node.id, node);
    }

    pub fn children(&self, id: Guid) -> impl Iterator<Item = &SceneNode> {
        self.nodes
            .get(&id)
            .map(|n| n.children.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|child_id| self.nodes.get(child_id))
    }

    /// Pages a designer would consider part of the deliverable.
    pub fn visible_pages(&self) -> impl Iterator<Item = &SceneNode> {
        self.pages
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .filter(|page| !page.internal_only)
    }

    /// The component a node is an instance of, if it resolves inside this file.
    #[must_use]
    pub fn component_of(&self, node: &SceneNode) -> Option<&SceneNode> {
        self.nodes.get(&node.component_id?)
    }

    /// The name of the variable bound to `field` on `node`.
    #[must_use]
    pub fn bound_variable_name(&self, node: &SceneNode, field: &str) -> Option<&str> {
        let id = node.bound_variables.get(field)?;
        self.variables.get(id).map(|v| v.name.as_str())
    }

    /// A variable's name, or its id when the file does not define it.
    #[must_use]
    pub fn variable_label(&self, id: Guid) -> String {
        self.variables.get(&id).map_or_else(|| id.to_string(), |v| v.name.clone())
    }

    /// Walk a subtree depth-first, including the root itself.
    pub fn walk(&self, root_id: Guid, mut visit: impl FnMut(&SceneNode, usize)) {
        let mut stack = vec![(root_id, 0usize)];
        while let Some((id, depth)) = stack.pop() {
            let Some(node) = self.nodes.get(&id) else { continue };
            visit(node, depth);
            for child_id in node.children.iter().rev() {
                stack.push((*child_id, depth + 1));
            }
        }
    }
}

// ── Graph construction ────────────────────────────────────────────────────────

/// The id a file without a `DOCUMENT` node gives its root.
const DOCUMENT_ROOT: Guid = Guid::new(0, 0);

struct Index<'a> {
    changes: IndexMap<Guid, &'a Value>,
    parents: IndexMap<Guid, Guid>,
    order_keys: IndexMap<Guid, String>,
    children: IndexMap<Guid, Vec<Guid>>,
}

fn index_changes(node_changes: &[Value]) -> Index<'_> {
    let mut index = Index {
        changes: IndexMap::new(),
        parents: IndexMap::new(),
        order_keys: IndexMap::new(),
        children: IndexMap::new(),
    };

    for nc in node_changes {
        let Some(id) = nc.field(Field::Guid).and_then(Guid::read) else { continue };
        if nc.is(Field::Phase, NodePhase::Removed) {
            continue;
        }
        index.changes.insert(id, nc);

        let Some(parent_index) = nc.field(Field::ParentIndex) else { continue };
        let Some(parent_id) = parent_index.field(Field::Guid).and_then(Guid::read) else { continue };
        let position = parent_index.str(Field::Position).unwrap_or("").to_string();
        index.parents.insert(id, parent_id);
        index.order_keys.insert(id, position);
        index.children.entry(parent_id).or_default().push(id);
    }

    // Figma orders siblings by a fractional index string; ties fall back to the
    // position along the parent's stacking axis.
    let axis_of: IndexMap<Guid, Option<Field>> = index
        .children
        .keys()
        .map(|parent_id| {
            let axis = index.changes.get(parent_id).and_then(|nc| match nc.known::<StackMode>(Field::StackMode) {
                Some(StackMode::Horizontal) => Some(Field::M02),
                Some(StackMode::Vertical) => Some(Field::M12),
                _ => None,
            });
            (*parent_id, axis)
        })
        .collect();

    for (parent_id, children) in &mut index.children {
        let axis = axis_of.get(parent_id).copied().flatten();
        children.sort_by(|a, b| {
            let key_a = index.order_keys.get(a).map_or("", String::as_str);
            let key_b = index.order_keys.get(b).map_or("", String::as_str);
            key_a.cmp(key_b).then_with(|| {
                let Some(axis) = axis else { return std::cmp::Ordering::Equal };
                let pos = |id: &Guid| {
                    index
                        .changes
                        .get(id)
                        .and_then(|nc| nc.field(Field::Transform))
                        .and_then(|t| t.f32(axis))
                        .unwrap_or(0.0)
                };
                pos(a).partial_cmp(&pos(b)).unwrap_or(std::cmp::Ordering::Equal)
            })
        });
    }

    index
}

pub fn build_scene_graph(fig_file: &FigFile) -> Result<SceneGraph, ModelError> {
    let node_changes = fig_file
        .node_changes
        .field(Field::NodeChanges)
        .and_then(Value::as_array)
        .ok_or(ModelError::NoNodeChanges)?;

    let index = index_changes(node_changes);
    let asset_refs = variables::build_asset_refs(&index.changes);

    // Shared styles first: nodes referencing them inherit their paints.
    let bootstrap = Converter {
        resolver: VariableResolver { asset_refs: &asset_refs },
        styles: IndexMap::new(),
        blobs: &fig_file.blobs,
    };
    let mut styles = IndexMap::new();
    for (id, nc) in &index.changes {
        if let Some(style) = bootstrap.shared_style(*id, nc) {
            styles.insert(*id, style);
        }
    }

    let converter = Converter {
        resolver: VariableResolver { asset_refs: &asset_refs },
        styles,
        blobs: &fig_file.blobs,
    };
    let variable_data = variables::extract(
        &index.changes,
        &index.parents,
        &VariableResolver { asset_refs: &asset_refs },
    );

    let root_id = index
        .changes
        .iter()
        .find(|(id, nc)| nc.is(Field::Type, schema::NodeType::Document) || **id == DOCUMENT_ROOT)
        .map(|(id, _)| *id);

    let mut graph = SceneGraph {
        root_id,
        images: fig_file.images.clone(),
        variables: variable_data.variables,
        collections: variable_data.collections,
        styles: converter.styles.clone(),
        asset_refs: asset_refs.clone(),
        ..SceneGraph::default()
    };

    build_tree(&mut graph, &index, &converter, root_id);
    resolve_variant_values(&mut graph, &index);
    Ok(graph)
}

/// Bring a graph up to date with node changes appended to `fig` after the graph
/// was built: the nodes a caller just authored, identified by `ids`.
///
/// It converts only those nodes, with the same converter
/// [`build_scene_graph`] uses, and links each one under its parent. Rebuilding
/// the whole graph would give the same answer and costs 3.2 s on a
/// 145,000-node file, which is why this exists. The index is still rebuilt,
/// because a node's order key and parent come from it, and that is the cheap
/// part.
///
/// A node whose parent is not in the graph is skipped, not guessed at: it is
/// either a non-scene node or one whose parent was never converted, and in
/// both cases [`build_scene_graph`] would not have placed it either.
pub fn insert_appended(
    graph: &mut SceneGraph,
    fig_file: &FigFile,
    ids: &[Guid],
) -> Result<(), ModelError> {
    let node_changes = fig_file
        .node_changes
        .field(Field::NodeChanges)
        .and_then(Value::as_array)
        .ok_or(ModelError::NoNodeChanges)?;
    let index = index_changes(node_changes);
    let asset_refs = variables::build_asset_refs(&index.changes);
    let converter = Converter {
        resolver: VariableResolver { asset_refs: &asset_refs },
        styles: graph.styles.clone(),
        blobs: &fig_file.blobs,
    };

    for id in ids {
        let Some(nc) = index.changes.get(id) else { continue };
        if convert::is_non_scene_node(nc) {
            continue;
        }
        let Some(&parent_id) = index.parents.get(id) else { continue };
        if !graph.nodes.contains_key(&parent_id) {
            continue;
        }
        let order_key = index.order_keys.get(id).cloned().unwrap_or_default();
        let mut node = converter.node(nc, *id, Some(parent_id), order_key);
        node.internal_only = nc.bool(Field::InternalOnly).unwrap_or(false);
        if node.node_type == NodeType::Canvas {
            graph.pages.push(*id);
        }
        graph.nodes.insert(*id, node);

        // Siblings in the index's order, which is the order the reader uses.
        if let Some(siblings) = index.children.get(&parent_id) {
            let order: Vec<Guid> =
                siblings.iter().filter(|sibling| graph.nodes.contains_key(*sibling)).copied().collect();
            if let Some(parent) = graph.nodes.get_mut(&parent_id) {
                parent.children = order;
            }
        }
    }
    Ok(())
}

/// Bring a graph up to date with node changes that were edited in `fig` after
/// the graph was built: nodes whose properties changed, not new ones.
///
/// Each node is converted again with the same converter, and keeps its
/// children and its place among its siblings, which an edit to its properties
/// does not change. A node that is not in the graph is skipped: to add one,
/// call [`insert_appended`].
pub fn refresh(graph: &mut SceneGraph, fig_file: &FigFile, ids: &[Guid]) -> Result<(), ModelError> {
    let node_changes = fig_file
        .node_changes
        .field(Field::NodeChanges)
        .and_then(Value::as_array)
        .ok_or(ModelError::NoNodeChanges)?;
    let index = index_changes(node_changes);
    let asset_refs = variables::build_asset_refs(&index.changes);
    let converter = Converter {
        resolver: VariableResolver { asset_refs: &asset_refs },
        styles: graph.styles.clone(),
        blobs: &fig_file.blobs,
    };
    for id in ids {
        let (Some(nc), Some(old)) = (index.changes.get(id), graph.nodes.get(id)) else { continue };
        let mut node = converter.node(nc, *id, old.parent_id, old.order_key.clone());
        node.internal_only = old.internal_only;
        node.children.clone_from(&old.children);
        node.variant_values.clone_from(&old.variant_values);
        graph.nodes.insert(*id, node);
    }
    Ok(())
}

/// Finds a node change by id. Callers that keep an id → position index answer
/// in constant time, which is what makes the `_with` functions cost what they
/// change rather than what the file holds.
pub type Lookup<'a> = dyn Fn(Guid) -> Option<&'a Value> + 'a;

/// [`insert_appended`], finding each node change through `lookup` instead of
/// indexing the whole file. The node goes among its parent's children by its
/// order key. When a sibling has the same key, Figma's order falls back to the
/// stacking axis, and this falls back to [`insert_appended`], which applies
/// that rule over the full index.
pub fn insert_appended_with(
    graph: &mut SceneGraph,
    fig_file: &FigFile,
    ids: &[Guid],
    lookup: &Lookup<'_>,
) -> Result<(), ModelError> {
    for &id in ids {
        let Some(nc) = lookup(id) else { continue };
        if convert::is_non_scene_node(nc) || nc.is(Field::Phase, NodePhase::Removed) {
            continue;
        }
        let Some(parent_index) = nc.field(Field::ParentIndex) else { continue };
        let Some(parent_id) = parent_index.field(Field::Guid).and_then(Guid::read) else { continue };
        let Some(parent) = graph.nodes.get(&parent_id) else { continue };
        let order_key = parent_index.str(Field::Position).unwrap_or("").to_string();
        let siblings: Vec<Guid> = parent.children.iter().copied().filter(|s| *s != id).collect();
        let keys: Vec<&str> = siblings.iter().map(|s| graph.nodes.get(s).map_or("", |n| n.order_key.as_str())).collect();
        if keys.contains(&order_key.as_str()) {
            insert_appended(graph, fig_file, &[id])?;
            continue;
        }
        let at = keys.iter().position(|key| *key > order_key.as_str()).unwrap_or(siblings.len());
        let mut order = siblings;
        order.insert(at, id);

        let node = with_converter(graph, fig_file, |converter| {
            let mut node = converter.node(nc, id, Some(parent_id), order_key);
            node.internal_only = nc.bool(Field::InternalOnly).unwrap_or(false);
            node
        });
        if node.node_type == NodeType::Canvas && !graph.pages.contains(&id) {
            graph.pages.push(id);
        }
        graph.nodes.insert(id, node);
        if let Some(parent) = graph.nodes.get_mut(&parent_id) {
            parent.children = order;
        }
    }
    Ok(())
}

/// [`refresh`], finding each node change through `lookup` instead of indexing
/// the whole file.
pub fn refresh_with(graph: &mut SceneGraph, fig_file: &FigFile, ids: &[Guid], lookup: &Lookup<'_>) {
    for &id in ids {
        let Some(nc) = lookup(id) else { continue };
        let Some(old) = graph.nodes.get(&id) else { continue };
        let (parent, order_key) = (old.parent_id, old.order_key.clone());
        let mut node = with_converter(graph, fig_file, |converter| converter.node(nc, id, parent, order_key));
        let old = &graph.nodes[&id];
        node.internal_only = old.internal_only;
        node.children.clone_from(&old.children);
        node.variant_values.clone_from(&old.variant_values);
        graph.nodes.insert(id, node);
    }
}

/// Convert style node changes into `graph.styles`, new or changed, with the
/// reader's own conversion.
pub fn insert_styles_with(graph: &mut SceneGraph, fig_file: &FigFile, ids: &[Guid], lookup: &Lookup<'_>) {
    let converted: Vec<SharedStyle> = with_converter(graph, fig_file, |converter| {
        ids.iter().filter_map(|&id| converter.shared_style(id, lookup(id)?)).collect()
    });
    for style in converted {
        graph.styles.insert(style.id, style);
    }
}

/// Run `f` with a converter built from what the graph already holds. The
/// styles are lent, not copied, and put back.
fn with_converter<T>(graph: &mut SceneGraph, fig_file: &FigFile, f: impl FnOnce(&Converter<'_>) -> T) -> T {
    let styles = std::mem::take(&mut graph.styles);
    let converter = Converter {
        resolver: VariableResolver { asset_refs: &graph.asset_refs },
        styles,
        blobs: &fig_file.blobs,
    };
    let out = f(&converter);
    graph.styles = converter.styles;
    out
}

/// Take nodes out of a graph: out of `nodes`, out of their parents' children,
/// and out of the page list. The caller removes them from the file.
pub fn remove(graph: &mut SceneGraph, ids: &[Guid]) {
    let going: std::collections::HashSet<Guid> = ids.iter().copied().collect();
    let parents: std::collections::HashSet<Guid> = ids.iter().filter_map(|id| graph.nodes.get(id)?.parent_id).collect();
    for parent in parents {
        if let Some(parent) = graph.nodes.get_mut(&parent) {
            parent.children.retain(|child| !going.contains(child));
        }
    }
    graph.nodes.retain(|id, _| !going.contains(id));
    graph.pages.retain(|page| !going.contains(page));
}

fn build_tree(
    graph: &mut SceneGraph,
    index: &Index<'_>,
    converter: &Converter<'_>,
    root_id: Option<Guid>,
) {
    let roots: Vec<(Guid, Option<Guid>)> = match root_id {
        Some(doc_id) => vec![(doc_id, None)],
        None => index.changes.keys().filter(|id| !index.parents.contains_key(*id)).map(|id| (*id, None)).collect(),
    };

    let mut stack = roots;
    while let Some((id, parent_id)) = stack.pop() {
        if graph.nodes.contains_key(&id) {
            continue;
        }
        let Some(nc) = index.changes.get(&id) else { continue };
        if convert::is_non_scene_node(nc) {
            continue;
        }

        let order_key = index.order_keys.get(&id).cloned().unwrap_or_default();
        let mut node = converter.node(nc, id, parent_id, order_key);
        node.internal_only = nc.bool(Field::InternalOnly).unwrap_or(false);

        let children: Vec<Guid> = index
            .children
            .get(&id)
            .map(|ids| {
                ids.iter()
                    .filter(|child_id| {
                        index.changes.get(*child_id).is_some_and(|nc| !convert::is_non_scene_node(nc))
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default();
        node.children.clone_from(&children);

        if node.node_type == NodeType::Canvas {
            graph.pages.push(id);
        }
        graph.nodes.insert(id, node);

        for child_id in children.iter().rev() {
            stack.push((*child_id, Some(id)));
        }
    }

    // Files without a DOCUMENT node still need something to hang screens off.
    if graph.pages.is_empty() {
        if let Some(first_child) = root_id.and_then(|doc_id| index.children.get(&doc_id)).and_then(|children| children.first()) {
            graph.pages.push(*first_child);
        }
    }
}

/// Turn each variant's `variantPropSpecs` (property id → value) into readable
/// `name → value` pairs using the parent component set's definitions.
fn resolve_variant_values(graph: &mut SceneGraph, index: &Index<'_>) {
    let updates: Vec<(Guid, IndexMap<String, String>)> = graph
        .nodes
        .values()
        .filter(|node| node.node_type == NodeType::Component)
        .filter_map(|node| {
            let nc = index.changes.get(&node.id)?;
            let specs = convert::variant_prop_specs(nc);
            if specs.is_empty() {
                return None;
            }
            let parent = graph.nodes.get(&node.parent_id?)?;
            let names: IndexMap<Guid, &str> = parent.component_properties.iter().map(|p| (p.id, p.name.as_str())).collect();
            let values = specs
                .into_iter()
                .map(|(def_id, value)| {
                    let name = names.get(&def_id).map_or_else(|| def_id.to_string(), |n| (*n).to_string());
                    (name, value)
                })
                .collect();
            Some((node.id, values))
        })
        .collect();

    for (id, values) in updates {
        if let Some(node) = graph.nodes.get_mut(&id) {
            node.variant_values = values;
        }
    }
}

#[cfg(test)]
mod tests;
