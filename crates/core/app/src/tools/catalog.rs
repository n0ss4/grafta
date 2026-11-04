//! A file's design system: components, variables, images.

use schema::Guid;
use indexmap::IndexMap;
use model::{NodeType, SceneGraph, SceneNode, VariableValue};

use super::read::{path_of, require_node};
use crate::data::{data, Data, Map};
use crate::Query;
use crate::bound::{page, Paging};
use crate::view::{hex, num};
use crate::error::CatalogError;
use crate::port::Creating;
use crate::{DomainError, Output, ToolError, Toolkit};

/// The largest image returned inline. A larger one is described, not sent.
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;

/// The page a node sits on.
fn page_of<'a>(graph: &'a SceneGraph, node: &'a SceneNode) -> Option<&'a SceneNode> {
    let mut current = node;
    loop {
        if current.node_type == NodeType::Canvas {
            return Some(current);
        }
        current = graph.node(current.parent_id?)?;
    }
}

fn in_a_set(graph: &SceneGraph, node: &SceneNode) -> bool {
    node.parent_id
        .and_then(|id| graph.node(id))
        .is_some_and(|parent| parent.node_type == NodeType::ComponentSet)
}

/// Instances per component id, counted once per call.
fn instance_counts(graph: &SceneGraph) -> IndexMap<Guid, usize> {
    let mut counts = IndexMap::new();
    for node in graph.nodes.values().filter(|n| n.node_type == NodeType::Instance) {
        if let Some(id) = node.component_id {
            *counts.entry(id).or_insert(0) += 1;
        }
    }
    counts
}

/// The component ids an id stands for: itself, or every variant of a set.
fn component_ids(graph: &SceneGraph, node: &SceneNode) -> Vec<Guid> {
    if node.node_type == NodeType::ComponentSet {
        graph
            .children(node.id)
            .filter(|c| c.node_type == NodeType::Component)
            .map(|c| c.id)
            .collect()
    } else {
        vec![node.id]
    }
}

/// The components the file defines, paged.
pub struct ListComponents {
    pub handle: String,
    pub name: Option<String>,
    pub paging: Paging,
}

impl Query for ListComponents {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let graph = &session.graph;
        let filter = self.name.as_deref().map(str::to_lowercase);
        let counts = instance_counts(graph);
        let components: Vec<&SceneNode> = graph
            .nodes
            .values()
            .filter(|n| {
                n.node_type == NodeType::ComponentSet
                    || (n.node_type == NodeType::Component && !in_a_set(graph, n))
            })
            .filter(|n| filter.as_ref().is_none_or(|f| n.name.to_lowercase().contains(f)))
            .collect();

        let out = page(components, &self.paging, session.edits, 100, 0, session.ports.measure.as_ref(), |node| {
            let mut c = data!({
                "id": node.id,
                "name": node.name,
                "type": crate::view::type_name(node.node_type),
                "width": num(node.width),
                "height": num(node.height),
                "instances": component_ids(graph, node).iter().map(|id| counts.get(id).copied().unwrap_or(0)).sum::<usize>(),
            });
            if let Some(page) = page_of(graph, node) {
                c["page"] = data!(page.name);
            }
            if node.node_type == NodeType::ComponentSet {
                c["variants"] = data!(component_ids(graph, node).len());
                if !node.variant_options.is_empty() {
                    c["variantOptions"] = data!(node.variant_options);
                }
            }
            let properties: Vec<Data> = node
                .component_properties
                .iter()
                .filter(|p| p.prop_type != model::ComponentPropertyType::Variant)
                .map(|p| data!({ "name": p.name, "type": p.prop_type, "default": p.default_value }))
                .collect();
            if !properties.is_empty() {
                c["properties"] = data!(properties);
            }
            if let Some(description) = node.description.as_deref().filter(|d| !d.is_empty()) {
                c["description"] = data!(description);
            }
            c
        })?;
        Ok(Output::Data(out))
    }
}


/// Where a component is used, paged.
pub struct ComponentInstances {
    pub handle: String,
    pub id: String,
    pub paging: Paging,
}

impl Query for ComponentInstances {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let graph = &session.graph;
        let component = require_node(session, &self.id)?;
        if !matches!(component.node_type, NodeType::Component | NodeType::ComponentSet) {
            return Err(CatalogError::NotAComponent { id: component.id, is: crate::view::type_name(component.node_type) }.into());
        }
        let ids = component_ids(graph, component);
        let instances: Vec<&SceneNode> = graph
            .nodes
            .values()
            .filter(|n| n.node_type == NodeType::Instance)
            .filter(|n| n.component_id.is_some_and(|id| ids.contains(&id)))
            .collect();
        let out = page(instances, &self.paging, session.edits, 50, 0, session.ports.measure.as_ref(), |node| {
            data!({ "id": node.id, "name": node.name, "path": path_of(graph, node) })
        })?;
        Ok(Output::Data(out))
    }
}


/// The file's variables, by collection, paged.
pub struct ListVariables {
    pub handle: String,
    pub collection: Option<String>,
    pub paging: Paging,
}

impl Query for ListVariables {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let graph = &session.graph;
        let wanted = self.collection.as_deref();
        let collections: Vec<&model::VariableCollection> = graph
            .collections
            .values()
            .filter(|c| wanted.is_none_or(|w| c.name == w || c.id.to_string() == w))
            .collect();
        if let (Some(w), true) = (wanted, collections.is_empty()) {
            return Err(CatalogError::NoCollection { name: w.into() }.into());
        }

        let variables: Vec<&model::Variable> = graph
            .variables
            .values()
            .filter(|v| collections.iter().any(|c| c.id == v.collection_id))
            .collect();
        let collections_json = data!(collections
            .iter()
            .map(|c| data!({
                "id": c.id,
                "name": c.name,
                "modes": c.modes.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
                "variables": c.variable_ids.len(),
            }))
            .collect::<Vec<_>>());
        let reserve = session.ports.measure.bytes(&collections_json) + 32;
        let mut out = page(variables, &self.paging, session.edits, 200, reserve, session.ports.measure.as_ref(), |variable| variable_json(graph, variable))?;
        out["collections"] = collections_json;
        Ok(Output::Data(out))
    }
}


/// A variable as `list_variables` shows it: values by mode name, colours in
/// hex, an alias as `{ alias: <name> }`.
pub fn variable_json(graph: &SceneGraph, variable: &model::Variable) -> Data {
    let collection = graph.collections.get(&variable.collection_id);
    let mode_name = |mode_id: Guid| {
        collection
            .and_then(|c| c.modes.iter().find(|m| m.id.declared() == Some(mode_id)))
            .map_or_else(|| mode_id.to_string(), |m| m.name.clone())
    };
    let values: Map = variable
        .values_by_mode
        .iter()
        .map(|(mode, value)| {
            let value = match value {
                VariableValue::Color { value } => {
                    if value.a < 1.0 {
                        data!({ "color": hex(*value), "opacity": num(value.a) })
                    } else {
                        data!(hex(*value))
                    }
                }
                VariableValue::Float { value } => num(*value),
                VariableValue::String { value } => data!(value),
                VariableValue::Boolean { value } => data!(value),
                VariableValue::Alias { variable_id } => {
                    data!({ "alias": graph.variable_label(*variable_id) })
                }
            };
            (mode_name(*mode), value)
        })
        .collect();
    data!({
        "id": variable.id,
        "name": variable.name,
        "collection": collection.map_or_else(|| variable.collection_id.to_string(), |c| c.name.clone()),
        "type": variable.var_type,
        "values": values,
    })
}

/// An image the file embeds, by its hash.
pub struct GetImage {
    pub handle: String,
    pub hash: String,
}

impl Query for GetImage {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let hash = self.hash.as_str();
        let (_, bytes) = session
            .graph
            .images
            .iter()
            .find(|(name, _)| name == hash || name.trim_start_matches("images/") == hash)
            .ok_or_else(|| CatalogError::NoImage {
                hash: hash.into(),
                file: session.file_name(),
                count: session.graph.images.len(),
            })?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Ok(Output::Data(data!({
                "too_large": { "bytes": bytes.len(), "limit": MAX_IMAGE_BYTES },
                "mime": mime(bytes),
            })));
        }
        Ok(Output::Image { mime: mime(bytes), bytes: bytes.clone(), caption: None })
    }
}


/// A node drawn as a PNG.
pub struct RenderNode {
    pub handle: String,
    pub id: String,
    pub scale: f32,
    pub max_size: usize,
}

impl Query for RenderNode {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let node = require_node(session, &self.id)?;
        let (scale, max_size) = (self.scale, self.max_size);
        // A line is drawn as tall as its stroke.
        if node.width <= 0.0 || (node.height <= 0.0 && node.node_type != model::NodeType::Line) {
            return Err(no_area(node));
        }
        let picture = session.ports.raster.render(&session.fig, &session.graph, node.id, scale, max_size as u32)
            .map_err(|error| DomainError::Render { error: error.to_string() })?;
        Ok(Output::Image {
            mime: "image/png",
            caption: Some(data!({
                "id": node.id,
                "name": node.name,
                "width": picture.width,
                "height": picture.height,
                "scale": num(picture.scale),
                "notes": picture.notes,
            })),
            bytes: picture.png,
        })
    }
}


/// Every visible text under a node against WCAG 2.2 AA, paged.
pub struct CheckContrast {
    pub handle: String,
    pub id: String,
    pub all: bool,
    pub paging: Paging,
}

impl Query for CheckContrast {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let node = require_node(session, &self.id)?;
        if node.width <= 0.0 || node.height <= 0.0 {
            return Err(no_area(node));
        }
        let all = self.all;
        let checked = session.ports.raster.contrast(&session.fig, &session.graph, node.id)
            .map_err(|error| DomainError::Render { error: error.to_string() })?;
        let verdict = |c: &crate::port::Contrast| match c.ratio {
            None => "unmeasured",
            Some(r) if r + 1e-4 >= c.required => "passes",
            Some(_) => "fails",
        };
        let count = |v: &str| checked.iter().filter(|c| verdict(c) == v).count();
        let (passed, failed, unmeasured) = (count("passes"), count("fails"), count("unmeasured"));
        let mut listed: Vec<&crate::port::Contrast> = checked.iter().filter(|c| all || verdict(c) != "passes").collect();
        // The worst first: unmeasured, then failing by how far they fall short.
        listed.sort_by(|a, b| {
            let short = |c: &crate::port::Contrast| c.ratio.map_or(f32::MIN, |r| r - c.required);
            short(a).partial_cmp(&short(b)).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut out = page(listed, &self.paging, session.edits, 50, 0, session.ports.measure.as_ref(), |c| {
            let characters: String = c.sample.characters.chars().take(60).collect();
            let mut text = data!({
                "id": c.sample.id,
                "name": c.sample.name,
                "characters": characters,
                "verdict": verdict(c),
                "required": num(c.required),
            });
            if let Some(instance) = &c.sample.instance {
                text["in_instance"] = data!(instance);
            }
            match c.ratio {
                Some(ratio) => {
                    text["ratio"] = data!(two_places(ratio));
                    text["colour"] = data!(c.foreground);
                    text["background"] = data!(c.background);
                    if let Some((low, high)) = c.varies {
                        text["background_varies"] = data!({ "lowest": two_places(low), "highest": two_places(high) });
                    }
                }
                None => text["why"] = data!(c.sample.unmeasurable),
            }
            text
        })?;
        out["checked"] = data!(checked.len());
        out["passed"] = data!(passed);
        out["failed"] = data!(failed);
        out["unmeasured"] = data!(unmeasured);
        out["standard"] = data!("WCAG 2.2 AA: 4.5:1, or 3:1 for text of 24 px, or 18.66 px and bold; each text's box against what is drawn behind it");
        Ok(Output::Data(out))
    }
}


/// A ratio cut, not rounded, to two places, so 4.497 never reads as 4.50.
fn two_places(ratio: f32) -> f64 {
    (f64::from(ratio) * 100.0).floor() / 100.0
}

/// Nodes written to a new PDF, a page each.
pub struct ExportPdf {
    pub handle: String,
    pub path: std::path::PathBuf,
    pub ids: Vec<String>,
}

impl Query for ExportPdf {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let path = crate::Location::new_file(session.ports.store.as_ref(), &self.path, None)?.path();
        let mut ids = Vec::with_capacity(self.ids.len());
        for id in &self.ids {
            let node = require_node(session, id)?;
            if node.width <= 0.0 || node.height <= 0.0 {
                return Err(no_area(node));
            }
            ids.push(node.id);
        }
        let pdf = session.ports.raster.pdf(&session.fig, &session.graph, &ids)
            .map_err(|error| DomainError::Render { error: error.to_string() })?;
        session.ports.store.create(path, &pdf.bytes).map_err(|creating| -> ToolError {
            match creating {
                Creating::Open(error) => CatalogError::Unwritable { path: path.into(), error: error.to_string() }.into(),
                Creating::Write(error) => DomainError::CannotWrite { path: path.into(), error: error.to_string() }.into(),
            }
        })?;
        let pages: Vec<Data> = pdf
            .pages
            .iter()
            .filter_map(|(id, notes)| {
                let node = session.graph.node(*id)?;
                Some(data!({ "id": id, "name": node.name, "width": num(node.width), "height": num(node.height), "notes": notes }))
            })
            .collect();
        Ok(Output::Data(data!({ "path": path.display().to_string(), "bytes": pdf.bytes.len(), "pages": pages })))
    }
}


/// The image type, from its first bytes.
#[must_use]
pub fn mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG") {
        "image/png"
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF8") {
        "image/gif"
    } else if bytes.len() > 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

/// A node with nothing to draw.
fn no_area(node: &SceneNode) -> ToolError {
    CatalogError::NoArea { id: node.id, name: node.name.clone(), width: node.width, height: node.height }.into()
}
