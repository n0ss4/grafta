//! Finding and reading nodes.

use model::{NodeType, SceneGraph, SceneNode};
use schema::Guid;

use crate::data::{data, Data, Map};
use crate::Query;
use crate::bound::{page, Budget, Paging};
use crate::view::{self, Limits};
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// The overview `open_file` and `summarize` return.
pub fn summary(session: &Session) -> Data {
    let graph = &session.graph;
    let pages: Vec<Data> = graph
        .pages
        .iter()
        .filter_map(|&id| graph.node(id))
        .map(|page| {
            let mut nodes = 0usize;
            graph.walk(page.id, |_, _| nodes += 1);
            let mut p = data!({
                "id": page.id,
                "name": page.name,
                "nodes": nodes,
                "top_level": page.children.len(),
            });
            if page.internal_only {
                p["internalOnly"] = data!(true);
            }
            p
        })
        .collect();
    let count = |t: NodeType| graph.nodes.values().filter(|n| n.node_type == t).count();
    let product = session.products.of(session.fig.product);
    let mut out = data!({
        "file": session.file_name(),
        "product": product.key(),
        "pages": pages,
        "nodes": graph.nodes.len(),
        "components": count(NodeType::Component),
        "component_sets": count(NodeType::ComponentSet),
        "instances": count(NodeType::Instance),
        "text_nodes": count(NodeType::Text),
        "variable_collections": graph.collections.len(),
        "variables": graph.variables.len(),
        "styles": graph.styles.len(),
        "images": graph.images.len(),
        "prototype": super::prototype::census(session),
        "unsaved_edits": session.edits,
    });
    if let Some((key, value)) = product.summary(graph) {
        out[key] = value;
    }
    out
}

/// `Page / Frame / … / Node`, from the page down.
pub fn path_of(graph: &SceneGraph, node: &SceneNode) -> String {
    let mut names = vec![node.name.as_str()];
    let mut current = node.parent_id;
    while let Some(id) = current {
        let Some(parent) = graph.node(id) else { break };
        if parent.node_type == NodeType::Document {
            break;
        }
        names.push(parent.name.as_str());
        current = parent.parent_id;
    }
    names.reverse();
    names.join(" / ")
}

/// The node a model names by its id's text: one the file has, or a refusal
/// that says how ids look.
pub fn require_node<'a>(session: &'a Session, id: &str) -> Result<&'a SceneNode, ToolError> {
    id.parse().ok().and_then(|id| session.graph.node(id)).ok_or_else(|| unknown(session, id))
}

/// A node by its id: one the file has, or the same refusal.
pub fn node_at(session: &Session, id: Guid) -> Result<&SceneNode, ToolError> {
    session.graph.node(id).ok_or_else(|| unknown(session, &id.to_string()))
}

fn unknown(session: &Session, id: &str) -> ToolError {
    DomainError::UnknownNode { id: id.into(), file: session.file_name() }.into()
}

/// An open file's overview.
pub struct Summarize {
    pub handle: String,
}

impl Query for Summarize {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        Ok(Output::Data(summary(kit.session(&self.handle)?)))
    }
}

/// Nodes by name, type, or both, optionally inside one node, paged.
pub struct SearchNodes {
    pub handle: String,
    pub name: Option<String>,
    /// In the plugin API's spelling, checked here so the words name it.
    pub node_type: Option<String>,
    pub within: Option<String>,
    pub paging: Paging,
}

impl Query for SearchNodes {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let graph = &session.graph;
        let name = self.name.as_deref().map(str::to_lowercase);
        let wanted_type = match self.node_type.as_deref() {
            Some(t) => Some(view::parse_type(t).ok_or_else(|| DomainError::NotANodeType { given: t.into() })?),
            None => None,
        };
        let matches = |node: &SceneNode| {
            name.as_ref().is_none_or(|n| node.name.to_lowercase().contains(n))
                && wanted_type.is_none_or(|t| node.node_type == t)
        };

        let found: Vec<&SceneNode> = match self.within.as_deref() {
            Some(root) => {
                let root = require_node(session, root)?.id;
                let mut ids = Vec::new();
                graph.walk(root, |node, depth| {
                    if depth > 0 && matches(node) {
                        ids.push(node.id);
                    }
                });
                ids.iter().filter_map(|&id| graph.node(id)).collect()
            }
            // The tree from the document down, children in their order: the
            // map holds nodes as they were added, a copy last.
            None => match graph.root_id {
                Some(root) => {
                    let mut ids = Vec::new();
                    graph.walk(root, |node, _| {
                        if matches(node) {
                            ids.push(node.id);
                        }
                    });
                    ids.iter().filter_map(|&id| graph.node(id)).collect()
                }
                None => graph.nodes.values().filter(|n| matches(n)).collect(),
            },
        };

        let out = page(found, &self.paging, session.edits, 50, 0, session.ports.measure.as_ref(), |node| {
            data!({
                "id": node.id,
                "name": node.name,
                "type": view::type_name(node.node_type),
                "path": path_of(graph, node),
            })
        })?;
        Ok(Output::Data(out))
    }
}

/// One node in Figma's terms, its children to a depth.
pub struct ReadNode {
    pub handle: String,
    pub id: String,
    pub limits: Limits,
}

impl Query for ReadNode {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let node = require_node(session, &self.id)?;
        let drawn = |node: &SceneNode| super::vector::drawn_paths(session, node);
        let mut out = view::node(&session.graph, session.raw(node.id), node, &self.limits, &session.ports, session.products, &drawn);
        super::prototype::show(session, node, &mut out);
        out["path"] = data!(path_of(&session.graph, node));
        Ok(Output::Data(out))
    }
}

/// One node as the file stores it, optionally only some of its fields.
pub struct ReadRaw {
    pub handle: String,
    pub id: String,
    pub fields: Option<Vec<String>>,
    pub budget: Budget,
}

impl Query for ReadRaw {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let id = self.id.as_str();
        let raw = id
            .parse()
            .ok()
            .and_then(|id| session.raw(id))
            .ok_or_else(|| DomainError::UnknownRawNode { id: id.into(), file: session.file_name() })?;
        let mut out = view::raw_json(raw);
        if let Some(wanted) = &self.fields {
            if let Data::Map(map) = &mut out {
                map.retain(|key, _| wanted.contains(key));
            }
        }

        let (budget, measure) = (self.budget.0, session.ports.measure.as_ref());
        if measure.bytes(&out) > budget {
            // Cut by field, and say which fields were left out.
            let Data::Map(map) = out else { unreachable!("a node change is an object") };
            let mut kept = Map::new();
            let mut left_out = Vec::new();
            let mut used = 128;
            for (key, value) in map {
                let size = key.len() + measure.bytes(&value) + 4;
                if used + size <= budget {
                    used += size;
                    kept.insert(key, value);
                } else {
                    left_out.push(key);
                }
            }
            kept.insert(
                "omitted".into(),
                data!({
                    "reason": "budget",
                    "fields": left_out,
                    "how": "call read_raw with fields: [...] naming some of these",
                }),
            );
            out = Data::Map(kept);
        }
        Ok(Output::Data(out))
    }
}
