//! Groups, sections and duplicates.
//!
//! A group is not a `GROUP` node, of which the fixtures have none: it is a
//! `FRAME` with `resizeToFit` and no auto-layout (699 measured alike), placed at
//! its children's bounds with its children relative to it. A section is a
//! `SECTION`. A duplicate is every node of a subtree copied with a new id.
//! The author half is `model::author`; the shapes are those a spike wrote and Figma opened,
//! which Figma opened.

use crate::data::data;
use kiwi::Value as Raw;
use model::author::{position_after, position_between};
use model::{LayoutMode, NodeType};
use schema::{object, Field, Guid, Wire, WireMut};

use super::read::require_node;
use super::write::{author_error, created, parent_for, record, require_parent, Placement};
use crate::Command;
use crate::error::StructureError;
use crate::event::NodeEvent;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// Move a node in the graph to where its node change now says, keeping its
/// children, as `move_node` does.
fn rehome(session: &mut Session, id: Guid) -> Result<(), ToolError> {
    let children = session.graph.node(id).map(|n| n.children.clone()).unwrap_or_default();
    model::remove(&mut session.graph, &[id]);
    session
        .insert_appended(&[id])
        .map_err(|error| DomainError::MovedNotReadBack { id, error: error.to_string() })?;
    if let Some(node) = session.graph.nodes.get_mut(&id) {
        node.children = children;
    }
    Ok(())
}

/// Keys that sort, in order, after `lower` and before `upper`.
fn keys_between(lower: Option<&str>, upper: Option<&str>, n: usize) -> Result<Vec<String>, ToolError> {
    let mut out: Vec<String> = Vec::new();
    for _ in 0..n {
        let key = position_between(out.last().map(String::as_str).or(lower), upper)
            .ok_or(StructureError::NoKeysBetween)?;
        out.push(key);
    }
    Ok(out)
}

/// Fit a group to its children again, keeping each where it is on the canvas,
/// as Figma keeps a group at its children's bounds.
pub fn refit(session: &mut Session, group: Guid) -> Result<(), ToolError> {
    let Some(node) = session.graph.node(group) else { return Ok(()) };
    if node.node_type != NodeType::Group {
        return Ok(());
    }
    let kids: Vec<(Guid, f32, f32, f32, f32)> = session.graph.children(group).map(|c| (c.id, c.x, c.y, c.width, c.height)).collect();
    if kids.is_empty() {
        return Ok(());
    }
    let (x0, y0) = kids.iter().fold((f32::MAX, f32::MAX), |(a, b), k| (a.min(k.1), b.min(k.2)));
    let (x1, y1) = kids.iter().fold((f32::MIN, f32::MIN), |(a, b), k| (a.max(k.1 + k.3), b.max(k.2 + k.4)));
    let (gx, gy, gw, gh) = (node.x, node.y, node.width, node.height);
    if x0.abs() < 1e-3 && y0.abs() < 1e-3 && (x1 - gw).abs() < 1e-3 && (y1 - gh).abs() < 1e-3 {
        return Ok(());
    }
    let mut ids = vec![group];
    set_frame(session, group, Some((gx + x0, gy + y0)), Some((x1 - x0, y1 - y0)));
    for (id, x, y, _, _) in kids {
        set_frame(session, id, Some((x - x0, y - y0)), None);
        ids.push(id);
    }
    session.refresh(&ids);
    Ok(())
}

/// Write a node's translation and size straight into its node change.
pub fn set_frame(session: &mut Session, id: Guid, at: Option<(f32, f32)>, size: Option<(f32, f32)>) {
    let Some(fields) = session.fields_mut(id) else { return };
    if let (Some((x, y)), Some(Raw::Object(transform))) = (at, fields.field_mut(Field::Transform)) {
        transform.set(Field::M02, Raw::Float(x));
        transform.set(Field::M12, Raw::Float(y));
    }
    if let Some((w, h)) = size {
        fields.set(Field::Size,
            object([(Field::X, Raw::Float(w)), (Field::Y, Raw::Float(h))]),
        );
    }
}

// ── The tools ────────────────────────────────────────────────────────────────

/// Layers that share a parent, grouped.
pub struct GroupNodes {
    pub handle: String,
    pub ids: Vec<String>,
    pub name: String,
}

impl Command for GroupNodes {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let (ids, name) = (&self.ids, &self.name);
        let mut parent: Option<Option<Guid>> = None;
        let mut members = Vec::new();
        for id in ids {
            let guid = require_node(session, id)?.id;
            super::guard::editable(session, guid)?;
            let node = super::read::node_at(session, guid)?;
            if matches!(node.node_type, NodeType::Document | NodeType::Canvas) {
                return Err(StructureError::GroupedPage { id: id.clone() }.into());
            }
            if node.rotation != 0.0 {
                return Err(StructureError::Rotated { id: id.clone() }.into());
            }
            let here = node.parent_id;
            if parent.is_some_and(|p| p != here) {
                return Err(StructureError::OtherParent { id: id.clone(), first: ids[0].clone() }.into());
            }
            parent = Some(here);
            members.push((guid, node.x, node.y, node.width, node.height, node.order_key.clone()));
        }
        let parent = parent.flatten().ok_or_else(|| DomainError::NoParent { id: ids[0].clone() })?;
        if session.graph.node(parent).is_some_and(|p| p.layout.mode != LayoutMode::None) {
            return Err(StructureError::InAutoLayout { parent }.into());
        }
        let parent_guid = parent_for(session, parent, Placement::Group)?;
        members.sort_by(|a, b| a.5.cmp(&b.5));
        let (x0, y0) = members.iter().fold((f32::MAX, f32::MAX), |(a, b), m| (a.min(m.1), b.min(m.2)));
        let (x1, y1) = members.iter().fold((f32::MIN, f32::MIN), |(a, b), m| (a.max(m.1 + m.3), b.max(m.2 + m.4)));
        // The group takes the frontmost layer's place among its parent's children.
        let front = members.last().map(|m| m.5.clone()).unwrap_or_default();
        let keys = keys_between(None, None, members.len())?;
        let group = {
            let mut graft = session.graft().map_err(author_error)?;
            let group = graft.add_group_at(parent_guid, &front, name, (x1 - x0, y1 - y0), (x0, y0)).map_err(author_error)?;
            for ((id, x, y, ..), key) in members.iter().zip(&keys) {
                graft.place_at(*id, group, key, (x - x0, y - y0)).map_err(author_error)?;
            }
            group
        };
        record(session, group)?;
        for (id, ..) in &members {
            rehome(session, *id)?;
        }
        let mut out = created(session, group);
        out["children"] = data!(ids);
        Ok(Output::Data(out))
    }
}

/// A group's layers moved to its parent, and the group removed.
pub struct Ungroup {
    pub handle: String,
    pub id: String,
}

impl Command for Ungroup {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let id = require_node(session, &self.id)?.id;
        super::guard::editable(session, id)?;
        let node = super::read::node_at(session, id)?;
        if node.node_type != NodeType::Group {
            return Err(StructureError::NotAGroup { id, is: crate::view::type_name(node.node_type) }.into());
        }
        let (gx, gy, key) = (node.x, node.y, node.order_key.clone());
        let parent = node.parent_id.ok_or_else(|| DomainError::NoParent { id: id.to_string() })?;
        let kids: Vec<(Guid, f32, f32)> = session.graph.children(id).map(|c| (c.id, c.x, c.y)).collect();
        // The keys between the group and the sibling in front of it.
        let upper = session.graph.children(parent).map(|c| c.order_key.clone()).filter(|k| *k > key).min();
        let keys = keys_between(Some(&key), upper.as_deref(), kids.len())?;
        {
            let mut graft = session.graft().map_err(author_error)?;
            for ((kid, x, y), k) in kids.iter().zip(&keys) {
                graft.place_at(*kid, parent, k, (gx + x, gy + y)).map_err(author_error)?;
            }
        }
        for (kid, ..) in &kids {
            rehome(session, *kid)?;
        }
        let Raw::Object(message) = &mut session.fig.node_changes else {
            return Err(DomainError::MessageNotAnObject.into());
        };
        if let Some(Raw::Array(changes)) = message.field_mut(Field::NodeChanges) {
            session.undo.remove_where(changes, |nc| crate::raw_id_of(nc) == Some(id));
        }
        model::remove(&mut session.graph, &[id]);
        session.created.retain(|c| *c != id);
        session.reindex();
        session.edits += 1;
        let ids: Vec<Guid> = kids.iter().map(|(k, ..)| *k).collect();
        Ok(Output::Data(data!({ "removed": id, "children": ids, "parent": parent, "unsaved_edits": session.edits })))
    }
}

/// A section on a page.
pub struct CreateSection {
    pub handle: String,
    pub parent: String,
    pub name: String,
    pub size: (f32, f32),
    pub at: (f32, f32),
}

impl Command for CreateSection {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let page = require_node(session, &self.parent)?;
        if page.node_type != NodeType::Canvas {
            return Err(StructureError::NotAPage { id: page.id }.into());
        }
        let parent = page.id;
        let id = session.graft().map_err(author_error)?.add_section_at(parent, &self.name, self.size, self.at).map_err(author_error)?;
        record(session, id)?;
        Ok(Output::Data(created(session, id)))
    }
}

/// A node and everything in it, copied with new ids.
pub struct DuplicateNode {
    pub handle: String,
    pub id: String,
    pub parent: Option<String>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub name: Option<String>,
}

impl Command for DuplicateNode {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let node = require_node(session, &self.id)?;
        let id = node.id;
        if matches!(node.node_type, NodeType::Document | NodeType::Canvas) {
            return Err(StructureError::DuplicatedPage { id }.into());
        }
        let (x, y) = (self.x.unwrap_or(node.x), self.y.unwrap_or(node.y));
        let (beside_itself, own_parent) = (session.products.behaviours_of(node.node_type).any(|b| b.copies_beside_itself()), node.parent_id);
        let parent = self.parent.clone().or_else(|| own_parent.map(|p| p.to_string())).unwrap_or_default();
        let mut holds_component = None;
        session.graph.walk(id, |n, _| {
            if holds_component.is_none() && matches!(n.node_type, NodeType::Component | NodeType::ComponentSet) {
                holds_component = Some(n.id);
            }
        });
        if let Some(component) = holds_component {
            return Err(StructureError::DuplicatedComponent { component }.into());
        }
        let parent_guid = require_parent(session, &parent, Placement::Duplicate)?;
        // A copy its product places beside the original goes between it and
        // its next sibling, not at the end (a slide's is the next slide).
        let position = if beside_itself && Some(parent_guid) == own_parent {
            let siblings = session.graph.node(parent_guid).map(|p| p.children.clone()).unwrap_or_default();
            let after = siblings.iter().position(|c| *c == id).and_then(|i| siblings.get(i + 1)).copied();
            let key = |of: Guid| session.raw(of).and_then(|r| r.field(Field::ParentIndex)).and_then(|p| p.str(Field::Position)).map(str::to_string);
            let (lower, upper) = (key(id), after.and_then(key));
            keys_between(lower.as_deref(), upper.as_deref(), 1)?.remove(0)
        } else {
            position_after(model::author::node_changes(&session.fig), parent_guid)
        };
        let pairs = session
            .graft()
            .map_err(author_error)?
            .duplicate(id, parent_guid, &position, (x, y))
            .map_err(author_error)?;
        let copy = pairs[0].1;
        if let Some(name) = self.name.clone() {
            session.index_appended();
            if let Some(fields) = session.fields_mut(copy) {
                fields.set(Field::Name, Raw::Str(name));
            }
        }
        let new_ids: Vec<Guid> = pairs.iter().map(|(_, n)| *n).collect();
        session
            .insert_appended(&new_ids)
            .map_err(|error| StructureError::CopyNotReadBack { id, error: error.to_string() })?;
        session.created.extend(new_ids.iter().copied());
        session.products.emit(session, &NodeEvent::Duplicated { copy, pairs: &pairs })?;
        session.edits += 1;
        let mut out = created(session, copy);
        out["nodes"] = data!(new_ids.len());
        out["from"] = data!(id);
        // Which copy is which, parents first, so the copy's layers can be
        // changed; bounded, and said so.
        const SHOWN: usize = 50;
        out["copies"] = data!(pairs
            .iter()
            .take(SHOWN)
            .map(|(old, new)| data!({ "from": old, "id": new }))
            .collect::<Vec<_>>());
        if pairs.len() > SHOWN {
            out["omitted"] = data!({ "reason": "copies", "remaining": pairs.len() - SHOWN, "how": "read_node the copy with depth to find the rest" });
        }
        Ok(Output::Data(out))
    }
}
