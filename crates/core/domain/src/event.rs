//! What happened to a node, said once by the tool that did it.
//!
//! A product hears an event through `ProductProfile::on`. It is delivered at
//! once, inside the call: what a subscriber does is part of the call's unit of
//! work, rolled back with it, and in place before the tool goes on.
//! `delete_node` depends on that: a deck lets go of a slide before the tool
//! asks what still points at it.

use schema::Guid;

pub enum NodeEvent<'a> {
    /// `node` moved from under `from` to under `to`.
    Moved { node: Guid, from: Option<Guid>, to: Guid },
    /// `node` is about to be deleted from under `parent`: sent while it is
    /// still there.
    Deleted { node: Guid, parent: Option<Guid> },
    /// A node was copied as `copy`; `pairs` maps each node of its subtree to
    /// its copy, the original first.
    Duplicated { copy: Guid, pairs: &'a [(Guid, Guid)] },
}
