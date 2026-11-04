//! What may be edited, and what an edit must not break.
//!
//! Nodes a file already has are edited under three rules, each from a
//! measurement:
//!
//! - **Nothing inside a component.** Every instance caches its component's
//!   expansion (`derivedSymbolData`, 114,488 references measured), so an edit
//!   there would leave every instance stale.
//! - **Nothing another node points at is deleted.** Nodes point at each other
//!   through dozens of fields: an instance's `symbolID` (79,384 measured), a
//!   prototype's `transitionNodeID` (14,793), overrides' `guidPath` (68,138),
//!   and more. The check is generic, not a list: every guid in every other node
//!   is compared against the subtree being removed, and so is every override
//!   key.
//! - **A child of an auto-layout frame is positioned by its parent.** Figma
//!   does not lay auto-layout out on open, so grafta lays out any such
//!   frame it changes.

use std::collections::HashSet;

use kiwi::Value as Raw;
use model::{LayoutMode, NodeType};
use schema::{Field, Guid, StackPositioning, Wire, WireMut, WireName};

use crate::{DomainError, Session, ToolError};

/// Refuse an edit to a node inside a component definition, or to the document.
pub fn editable(session: &mut Session, id: Guid) -> Result<(), ToolError> {
    let node = super::read::node_at(session, id)?;
    if node.node_type == NodeType::Document {
        return Err(DomainError::TheDocument.into());
    }
    // A change in a component reaches every instance of it, whose cached
    // expansion goes stale: the dispatcher drops those caches once the call
    // has succeeded, and Figma derives them again.
    if let Some(component) = enclosing_component(session, id) {
        session.stale_components.push(component);
    }
    Ok(())
}

/// The component a node is, or is inside of, if any.
pub fn enclosing_component(session: &Session, id: Guid) -> Option<Guid> {
    let mut current = session.graph.node(id);
    while let Some(node) = current {
        if node.node_type == NodeType::Component {
            return Some(node.id);
        }
        current = node.parent_id.and_then(|p| session.graph.node(p));
    }
    None
}

/// Drop the cached expansion (`derivedSymbolData`) of every instance a
/// change to the stale components reaches: instances of them, and instances
/// of any component that holds one of them, however deep. Figma derives an
/// instance with no cache on open (a spike in Figma). Returns how many
/// caches were dropped.
pub fn drop_caches(session: &mut Session) -> usize {
    let mut stale: HashSet<Guid> = session.stale_components.drain(..).collect();
    if stale.is_empty() {
        return 0;
    }
    loop {
        let holders: Vec<Guid> = session
            .graph
            .nodes
            .values()
            .filter(|n| n.node_type == NodeType::Instance && n.component_id.is_some_and(|c| stale.contains(&c)))
            .filter_map(|n| enclosing_component(session, n.id))
            .filter(|c| !stale.contains(c))
            .collect();
        if holders.is_empty() {
            break;
        }
        stale.extend(holders);
    }
    let instances: Vec<usize> = session
        .graph
        .nodes
        .values()
        .filter(|n| n.node_type == NodeType::Instance && n.component_id.is_some_and(|c| stale.contains(&c)))
        .filter_map(|n| session.raw_index.get(&n.id).copied())
        .collect();
    let kiwi::Value::Object(message) = &mut session.fig.node_changes else { return 0 };
    let Some(kiwi::Value::Array(changes)) = message.field_mut(Field::NodeChanges) else { return 0 };
    let mut dropped = 0;
    for at in instances {
        let Some(change) = changes.get_mut(at) else { continue };
        if change.field(Field::DerivedSymbolData).is_none() {
            continue;
        }
        session.undo.changing(change);
        if let kiwi::Value::Object(fields) = change {
            dropped += usize::from(fields.take(Field::DerivedSymbolData).is_some());
        }
    }
    dropped
}

/// Whether grafta lays this frame out: it created it, or it changed the
/// frame's auto-layout, or it added a child to it.
pub fn laid_out_by_grafta(session: &Session, id: Guid) -> bool {
    session.created.contains(&id) || session.laid_out.contains(&id)
}

/// The auto-layout parent that positions this node, if any.
pub fn flowing_in(session: &Session, id: Guid) -> Option<Guid> {
    let node = session.graph.node(id)?;
    if node.layout.positioning == StackPositioning::Absolute.as_wire() {
        return None;
    }
    let parent = session.graph.node(node.parent_id?)?;
    (parent.layout.mode != LayoutMode::None).then_some(parent.id)
}

/// Make grafta responsible for laying out an auto-layout frame of the file's
/// own, because something in it changed and Figma will not lay it out.
pub fn take_layout(session: &mut Session, frame: Guid) -> Result<(), ToolError> {
    if laid_out_by_grafta(session, frame) {
        return Ok(());
    }
    let node = super::read::node_at(session, frame)?;
    if node.layout.mode == LayoutMode::None {
        return Ok(());
    }
    // A node its product lays out itself is not laid out by grafta (Figma
    // Slides places a row's slides, by their order).
    if session.products.behaviours_of(node.node_type).any(|b| !b.lays_out_children()) {
        return Ok(());
    }
    if node.layout.wrap {
        return Err(DomainError::WrappingLayout { frame }.into());
    }
    session.laid_out.push(frame);
    Ok(())
}

/// Every other node that points into `ids` (node ids or override keys): the
/// referrer and the field, at most `limit` of them.
pub fn references_into(session: &Session, ids: &HashSet<Guid>, limit: usize) -> Vec<(Guid, String)> {
    fn walk(v: &Raw, path: &str, ids: &HashSet<Guid>, hits: &mut Vec<String>) {
        match v {
            Raw::Object(fields) => {
                if let Some(g) = Guid::read(v) {
                    if ids.contains(&g) {
                        hits.push(path.to_string());
                    }
                    return;
                }
                for (k, x) in fields {
                    walk(x, &format!("{path}.{k}"), ids, hits);
                }
            }
            Raw::Array(items) => {
                for x in items {
                    walk(x, &format!("{path}[]"), ids, hits);
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for change in model::author::node_changes(&session.fig) {
        let Some(referrer) = crate::raw_id_of(change) else { continue };
        if ids.contains(&referrer) {
            continue;
        }
        let Raw::Object(fields) = change else { continue };
        let mut hits = Vec::new();
        for (key, value) in fields {
            // Where a node used to live, and the component a frame was detached
            // from, are history Figma keeps, not links.
            if matches!(
                Field::from_wire(key),
                Some(Field::Guid | Field::ParentIndex | Field::AncestorPathBeforeDeletion | Field::DetachedSymbolId)
            ) {
                continue;
            }
            walk(value, key, ids, &mut hits);
        }
        if let Some(field) = hits.into_iter().next() {
            out.push((referrer, field));
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}
