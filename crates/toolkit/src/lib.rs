#![deny(clippy::all)]
// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![cfg_attr(test, allow(clippy::disallowed_methods))]

//! Figma's own language, as tools a model calls.
//!
//! grafta does not interpret designs here. A model does that. What grafta
//! does is the part a model cannot: decode a `.fig` exactly (every node,
//! colour, layout, text, component and variable) and encode a change back into
//! a file Figma opens. This crate puts the layers together: the domain's
//! toolkit on the disk's ports, with the products and the edge's journal.
//!
//! Three measurements shaped it:
//!
//! - **A file stays open.** Opening a 25 MB file costs 3.2 s, and a model
//!   asks many small questions, so a [`Session`] holds the decoded file between
//!   calls.
//! - **An open file is expensive.** About 5 GB resident for that 25 MB file,
//!   nearly all of it the raw decoded message. It is reported rather than
//!   hidden, and a session can be closed.
//! - **Nothing large comes back whole.** 144,666 nodes fit in no model's
//!   context. Every result is bounded, and says what it left out
//!   ([`bound`]).
//!
//! The same [`tools::ALL`] list is served to a model by `grafta mcp` and called
//! from the command line by `grafta tool`. There is no second implementation.

pub use ::edge;
pub mod port;
pub mod product;

/// The application: `Command`, `Query`, `Data` and the unit of work.
pub mod app {
    pub use ::app::{data, unit, Command, Data, Query};
}
pub use ::app::{bound, layout, view};

/// Each tool's work, and the list of every tool.
pub mod tools {
    pub use crate::edge::{Tool, ALL};
    pub use ::app::tools::*;
}

pub use domain::{error, event, font, handle, location};
pub use domain::{fnv, raw_id_of, resident_mb, Base, Before, DomainError, Handle, Location, Output, Recovery, Session, ToolError, Toolkit};

use std::path::PathBuf;
use std::sync::Arc;

/// A toolkit on the disk, knowing every product, that forgets its edits when
/// it stops, as `grafta tool` and batches do.
#[must_use]
pub fn new() -> Toolkit {
    with_ports(port::on_disk(), None)
}

/// A toolkit on the disk that journals every edit under `dir` and recovers it
/// when the file is opened again.
#[must_use]
pub fn with_journal(dir: PathBuf) -> Toolkit {
    with_ports(port::on_disk(), Some(dir))
}

/// A toolkit on other adapters than the disk's; `journal` as for
/// [`with_journal`].
#[must_use]
pub fn with_ports(ports: port::Ports, journal: Option<PathBuf>) -> Toolkit {
    Toolkit::with(ports, product::all(), Arc::new(edge::journal::Journaled), journal)
}

#[cfg(test)]
mod tests;
