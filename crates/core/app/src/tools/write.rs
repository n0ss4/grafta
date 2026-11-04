//! Creating nodes and saving a file Figma opens.
//!
//! Only what A2 measured in Figma: a page, a frame on a page or in a frame,
//! and an instance of a component the same file defines. Every created node
//! takes an id in grafta's reserved session. Saving writes a new file, never
//! over an old one, and re-reads it before saying it worked.


use crate::data::{data, Data};
use model::author::{Guid, Provenance};
use model::NodeType;
use schema::{Field, Wire, WireMut};

use super::read::require_node;
pub use crate::product::Placement;
use crate::{Command, Query};
use crate::view::{num, type_name};
use crate::error::WriteError;
use crate::event::NodeEvent;
use crate::port::Creating;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// The archive entries Figma refuses a file without, measured in Figma:
/// missing either one is "Internal error during import".
const REQUIRED_ENTRIES: [&str; 2] = ["meta.json", "thumbnail.png"];

/// Parents A2 measured Figma accepting: a page, a frame. Anything else is
/// refused rather than tried.
pub fn require_parent(session: &mut Session, id: &str, placement: Placement) -> Result<Guid, ToolError> {
    let guid = require_node(session, id)?.id;
    parent_for(session, guid, placement)
}

/// A node `placement` may go in, given by its id.
pub fn parent_for(session: &mut Session, guid: Guid, placement: Placement) -> Result<Guid, ToolError> {
    super::guard::take_layout(session, guid)?;
    let parent = super::read::node_at(session, guid)?;
    // A product's node says what it admits (a slide row, a slide).
    let (parent_id, parent_type) = (parent.id, parent.node_type);
    let admitted = session.products.behaviours_of(parent_type).find_map(|b| b.admits(parent_id, placement)).transpose()?;
    // A section too, as a spike put a frame in one.
    if admitted.is_none() && !matches!(parent_type, NodeType::Canvas | NodeType::Frame | NodeType::Component | NodeType::Section) {
        return Err(WriteError::NotAParent { what: placement.noun(), parent: parent_id, is: type_name(parent_type) }.into());
    }
    // Something added to a component is added to all its instances.
    if let Some(component) = super::guard::enclosing_component(session, guid) {
        session.stale_components.push(component);
    }
    Ok(guid)
}

/// Record an authored node in the session: the graph learns it, the raw index
/// learns it, and cursors issued before it go stale.
pub fn record(session: &mut Session, id: Guid) -> Result<(), ToolError> {
    session
        .insert_appended(&[id])
        .map_err(|error| DomainError::CreatedNotReadBack { id, error: error.to_string() })?;
    session.edits += 1;
    session.created.push(id);
    Ok(())
}

pub fn author_error(error: model::author::AuthorError) -> ToolError {
    DomainError::Authoring(error).into()
}

pub fn created(session: &Session, id: Guid) -> Data {
    let node = session.graph.node(id);
    data!({
        "id": id,
        "type": node.map(|n| type_name(n.node_type)),
        "name": node.map(|n| n.name.clone()),
        "unsaved_edits": session.edits,
    })
}

/// A page at the end of the file's page list.
pub struct CreatePage {
    pub handle: String,
    pub name: String,
}

impl Command for CreatePage {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let id = session.graft().map_err(author_error)?.add_page(&self.name).map_err(author_error)?;
        record(session, id)?;
        Ok(Output::Data(created(session, id)))
    }
}

/// An empty frame on a page or in a frame.
pub struct CreateFrame {
    pub handle: String,
    pub parent: String,
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub at: (f32, f32),
}

impl Command for CreateFrame {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let parent = require_parent(session, &self.parent, Placement::Frame)?;
        let id = session
            .graft()
            .map_err(author_error)?
            .add_frame_at(parent, &self.name, (self.width, self.height), self.at)
            .map_err(author_error)?;
        record(session, id)?;
        Ok(Output::Data(created(session, id)))
    }
}

/// An instance of a component the file defines, on a page or in a frame.
pub struct CreateInstance {
    pub handle: String,
    pub parent: String,
    pub component: String,
    pub name: Option<String>,
    pub at: (f32, f32),
}

impl Command for CreateInstance {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let parent = require_parent(session, &self.parent, Placement::Instance)?;
        let component_id = self.component.as_str();
        let component = require_node(session, component_id)?;
        match component.node_type {
            NodeType::Component => {}
            NodeType::ComponentSet => {
                return Err(WriteError::InstanceOfSet { id: component_id.into() }.into())
            }
            other => {
                return Err(WriteError::NotADefinedComponent { id: component_id.into(), is: type_name(other) }.into())
            }
        }
        let name = self.name.as_deref().unwrap_or(&component.name).to_string();
        let renamed = name != component.name;
        let at = self.at;
        let symbol = component.id;
        let id = session
            .graft()
            .map_err(author_error)?
            .add_instance_at(parent, &name, symbol, at)
            .map_err(author_error)?;
        record(session, id)?;
        // Figma reads an instance's own name from the entry addressed to its
        // component, not from the instance (measured in Figma).
        if renamed {
            let key = super::overrides::key_of(session, symbol).ok_or(DomainError::NoGuid { id: symbol })?;
            super::overrides::write_overrides(session, id, vec![(vec![key], vec![(Field::Name, kiwi::Value::Str(name.clone()))])])?;
            session.refresh(&[id]);
        }
        let mut out = created(session, id);
        if let Some(node) = session.graph.node(id) {
            out["width"] = num(node.width);
            out["height"] = num(node.height);
        }
        Ok(Output::Data(out))
    }
}

/// The open file written to a new path, read back and compared.
pub struct SaveAs {
    pub handle: String,
    pub path: std::path::PathBuf,
}

impl Query for SaveAs {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let path = self.path.as_path();

        let ports = &session.ports;
        let path = crate::Location::new_file(ports.store.as_ref(), path, Some(&session.path))?.path();
        // A deck stays a deck and a board a board: Figma opens each by
        // its extension, and the container keeps its product's magic.
        let extension = session.fig.product.extension();
        if path.extension().and_then(|e| e.to_str()) != Some(extension) {
            return Err(WriteError::WrongExtension {
                product: session.products.of(session.fig.product).name(),
                extension,
                path: path.into(),
            }
            .into());
        }
        let missing: Vec<&str> = REQUIRED_ENTRIES
            .iter()
            .copied()
            .filter(|entry| !session.fig.other_entries.iter().any(|(name, _)| name == entry))
            .collect();
        if !missing.is_empty() {
            return Err(WriteError::MissingEntries { file: session.file_name(), missing }.into());
        }

        let bytes = ports.codec.encode(&session.fig).map_err(|error| WriteError::CannotEncode { error })?;
        let reread = ports.codec.decode(&bytes).map_err(|error| WriteError::NotReadBack { error })?;
        if let Some(difference) = ports.codec.compare(&session.fig, &reread) {
            return Err(WriteError::ChangedInTransit { at: difference.path.to_string() }.into());
        }

        ports.store.create(path, &bytes).map_err(|creating| match creating {
            Creating::Open(error) => DomainError::CannotCreate { path: path.into(), error: error.to_string() },
            Creating::Write(error) => DomainError::CannotWrite { path: path.into(), error: error.to_string() },
        })?;

        let created: Vec<Data> = session
            .created
            .iter()
            .filter_map(|&id| session.graph.node(id))
            .map(|n| data!({ "id": n.id, "type": type_name(n.node_type), "name": n.name }))
            .collect();
        Ok(Output::Data(data!({
            "path": path.display().to_string(),
            "bytes": bytes.len(),
            "nodes_added": session.created.len(),
            "created": created,
            "reread": "compared equal",
            "next": "open it in Figma: Figma's import is the check grafta cannot do",
        })))
    }
}


/// A node and every node change below it, from the file itself rather than the
/// graph, so that nothing the graph does not show is left dangling.
fn subtree(session: &Session, root: Guid) -> Vec<Guid> {
    let mut children: std::collections::HashMap<Guid, Vec<Guid>> = std::collections::HashMap::new();
    for nc in model::author::node_changes(&session.fig) {
        let parent = nc.field(Field::ParentIndex).and_then(|index| index.field(Field::Guid)).and_then(Guid::read);
        if let (Some(parent), Some(id)) = (parent, crate::raw_id_of(nc)) {
            children.entry(parent).or_default().push(id);
        }
    }
    let mut seen = std::collections::HashSet::from([root]);
    let mut ids = vec![root];
    let mut index = 0;
    while index < ids.len() {
        for id in children.get(&ids[index]).into_iter().flatten() {
            if seen.insert(*id) {
                ids.push(*id);
            }
        }
        index += 1;
    }
    ids
}

/// A node and everything inside it, deleted; with `also`, more nodes in the
/// same call.
pub struct DeleteNode {
    pub handle: String,
    pub id: String,
    pub also: Vec<String>,
}

impl Command for DeleteNode {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let mut roots: Vec<(Guid, Option<Guid>)> = Vec::new();
        let mut pages_going = 0;
        for name in std::iter::once(&self.id).chain(&self.also) {
            let id = require_node(session, name)?.id;
            if roots.iter().any(|(root, _)| *root == id) {
                continue;
            }
            super::guard::editable(session, id)?;
            let node = super::read::node_at(session, id)?;
            if node.node_type == NodeType::Canvas {
                pages_going += 1;
            }
            roots.push((id, node.parent_id));
        }
        if pages_going > 0 && session.graph.pages.len() <= pages_going {
            return Err(WriteError::OnlyPage.into());
        }
        let mut removed: Vec<Guid> = Vec::new();
        let mut going = std::collections::HashSet::new();
        for (root, _) in &roots {
            for r in subtree(session, *root) {
                if going.insert(r) {
                    removed.push(r);
                }
            }
        }
        let id = roots[0].0;

        // Nothing another node points at may go: ids, and the override keys
        // instances address layers by. What the deleted nodes point at among
        // themselves goes with them.
        let mut targets: std::collections::HashSet<Guid> = removed.iter().copied().collect();
        for r in &removed {
            if let Some(key) = session.raw(*r).and_then(|raw| raw.field(Field::OverrideKey)).and_then(Guid::read) {
                targets.insert(key);
            }
        }
        // Its product lets go of it first (a deck, of a slide); if
        // anything else still points at it, the call fails and is rolled back
        // whole.
        for (root, parent) in &roots {
            session.products.emit(session, &NodeEvent::Deleted { node: *root, parent: *parent })?;
        }
        let referrers = super::guard::references_into(session, &targets, 5);
        if !referrers.is_empty() {
            let referrers = referrers.iter().map(|(who, field)| format!("{who} ({field})")).collect();
            return Err(WriteError::Referenced { id, referrers }.into());
        }
        for (_, parent) in &roots {
            if let Some(parent) = parent {
                if !parent.is_graftas() && !going.contains(parent) {
                    super::guard::take_layout(session, *parent)?;
                }
            }
        }
        let kiwi::Value::Object(message) = &mut session.fig.node_changes else {
            return Err(DomainError::MessageNotAnObject.into());
        };
        if let Some(kiwi::Value::Array(changes)) = message.field_mut(Field::NodeChanges) {
            session.undo.remove_where(changes, |nc| crate::raw_id_of(nc).is_some_and(|nc_id| going.contains(&nc_id)));
        }
        model::remove(&mut session.graph, &removed);
        session.created.retain(|c| !going.contains(c));
        session.laid_out.retain(|c| !going.contains(c));
        session.reindex();
        session.edits += 1;
        Ok(Output::Data(data!({ "removed": removed, "unsaved_edits": session.edits })))
    }
}

/// A node moved into another parent, or among its siblings.
pub struct MoveNode {
    pub handle: String,
    pub id: String,
    pub to: Destination,
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Where `move_node` was asked to put a node, as the call named it.
pub enum Destination {
    /// Into a parent, at the front of its stack or the back.
    Parent { id: String, back: bool },
    Behind(String),
    InFrontOf(String),
}

/// Where `move_node` puts a node among its new siblings.
enum Place {
    /// Last, the front of the stack: `parent` alone, or `to: "front"`.
    Front,
    /// First, the back of the stack.
    Back,
    Behind(Guid),
    InFrontOf(Guid),
}

impl Command for MoveNode {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let id = require_node(session, &self.id)?.id;
        super::guard::editable(session, id)?;
        let node = super::read::node_at(session, id)?;
        let products = session.products;
        let behaviours = || products.behaviours_of(node.node_type);
        // A node's place is its order key and its transform, whatever its type:
        // shapes and vectors move as frames do, their geometry their own.
        let movable = matches!(
            node.node_type,
            NodeType::Frame
                | NodeType::Component
                | NodeType::ComponentSet
                | NodeType::Instance
                | NodeType::Text
                | NodeType::Rectangle
                | NodeType::RoundedRectangle
                | NodeType::Ellipse
                | NodeType::Line
                | NodeType::Polygon
                | NodeType::Star
                | NodeType::Vector
        );
        if !movable && !behaviours().any(|b| b.movable()) {
            return Err(WriteError::NotMovable { id, is: type_name(node.node_type) }.into());
        }
        let (old_x, old_y, old_parent) = (node.x, node.y, node.parent_id);
        let is_component = matches!(node.node_type, NodeType::Component | NodeType::ComponentSet);
        let moves_into = |parent: &str, parent_type| behaviours().find_map(|b| b.moves_into(id, parent, parent_type));

        let (parent_text, place) = match &self.to {
            Destination::Behind(sibling) | Destination::InFrontOf(sibling) => {
                if *sibling == id.to_string() {
                    return Err(WriteError::BesideItself { id }.into());
                }
                let sibling_node = require_node(session, sibling)?;
                let parent = sibling_node
                    .parent_id
                    .ok_or_else(|| WriteError::SiblingWithoutParent { sibling: sibling.clone(), id })?;
                let place = if matches!(self.to, Destination::Behind(_)) {
                    Place::Behind(sibling_node.id)
                } else {
                    Place::InFrontOf(sibling_node.id)
                };
                (parent.to_string(), place)
            }
            Destination::Parent { id: parent, back } => (parent.clone(), if *back { Place::Back } else { Place::Front }),
        };
        let parent_id = parent_text.as_str();
        let parent_node = parent_id.parse().ok().and_then(|p| session.graph.node(p)).map(|p| (p.id, p.node_type));
        if is_component {
            if let Some(holder) = parent_node.and_then(|(p, _)| super::guard::enclosing_component(session, p)) {
                return Err(WriteError::ComponentInComponent { id, parent: parent_id.into(), holder }.into());
            }
        }
        // A product's node may say where it moves to, and as what (a slide,
        // among the rows of its deck alone).
        let placement = moves_into(parent_id, parent_node.map(|(_, kind)| kind)).transpose()?.unwrap_or(Placement::MovedNode);
        let parent = require_parent(session, parent_id, placement)?;
        if subtree(session, id).contains(&parent) {
            return Err(WriteError::IntoItself { id, parent: parent_id.into() }.into());
        }
        let moves = old_parent != Some(parent);
        let at = (self.x.unwrap_or(old_x), self.y.unwrap_or(old_y));

        // The new neighbours, in the graph's order, without the node itself.
        let siblings: Vec<(Guid, String)> = session
            .graph
            .children(parent)
            .filter(|child| child.id != id)
            .map(|child| (child.id, child.order_key.clone()))
            .collect();
        let key_at = |i: usize| siblings.get(i).map(|(_, key)| key.as_str());
        let index_of = |sibling: &Guid| siblings.iter().position(|(s, _)| s == sibling);
        let bounds = match &place {
            Place::Front => None,
            Place::Back => Some((None, key_at(0))),
            Place::Behind(s) => {
                let i = index_of(s).ok_or_else(|| WriteError::NotAmongChildren { sibling: *s, parent: parent_id.into() })?;
                Some((i.checked_sub(1).and_then(key_at), key_at(i)))
            }
            Place::InFrontOf(s) => {
                let i = index_of(s).ok_or_else(|| WriteError::NotAmongChildren { sibling: *s, parent: parent_id.into() })?;
                Some((key_at(i), key_at(i + 1)))
            }
        };
        match bounds {
            // The front is where a plain move has always put a node.
            None => session.graft().map_err(author_error)?.move_under(id, parent).map_err(author_error)?,
            Some((lower, upper)) => {
                let key = model::author::position_between(lower, upper).ok_or_else(|| WriteError::NoPositionKey {
                    lower: lower.map(str::to_string),
                    upper: upper.map(str::to_string),
                    id,
                })?;
                session.graft().map_err(author_error)?.place(id, parent, &key).map_err(author_error)?;
            }
        }
        if moves || self.x.is_some() || self.y.is_some() {
            if let Some(kiwi::Value::Object(t)) = session.fields_mut(id).and_then(|fields| fields.field_mut(Field::Transform)) {
                t.set(Field::M02, kiwi::Value::Float(at.0));
                t.set(Field::M12, kiwi::Value::Float(at.1));
            }
        }

        // Out of the old place, into the new one, keeping its own children.
        let children = session.graph.node(id).map(|n| n.children.clone()).unwrap_or_default();
        model::remove(&mut session.graph, &[id]);
        session
            .insert_appended(&[id])
            .map_err(|error| DomainError::MovedNotReadBack { id, error: error.to_string() })?;
        if let Some(moved) = session.graph.nodes.get_mut(&id) {
            moved.children = children;
        }
        // An auto-layout frame's order is its flow: both frames are laid out
        // again, as for any change to their children.
        for frame in [old_parent, Some(parent)].into_iter().flatten() {
            if !frame.is_graftas() {
                super::guard::take_layout(session, frame)?;
            }
        }
        session.products.emit(session, &NodeEvent::Moved { node: id, from: old_parent, to: parent })?;
        session.edits += 1;
        let index = session.graph.node(parent).and_then(|p| p.children.iter().position(|c| *c == id));
        let mut out = data!({
            "id": id,
            "parent": parent_id,
            "index": index,
            "of": session.graph.node(parent).map(|p| p.children.len()),
            "unsaved_edits": session.edits,
        });
        // In an auto-layout frame the position is the layout's, not the call's.
        match super::guard::flowing_in(session, id) {
            Some(frame) => out["placed_by"] = data!(format!("{frame}'s auto-layout")),
            None => {
                out["x"] = num(at.0);
                out["y"] = num(at.1);
            }
        }
        Ok(Output::Data(out))
    }
}
