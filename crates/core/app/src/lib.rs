#![deny(clippy::all)]
// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![cfg_attr(test, allow(clippy::disallowed_methods))]

//! The application: what a call asks, as types, and the work it does. A
//! [`Command`] may change an open document, a [`Query`] does not.
//!
//! The edge reads a call's JSON into one of them before any work is done; the
//! work never sees JSON. A query borrows the toolkit and so cannot change it:
//! it needs no unit of work and is never journaled. A command runs in a
//! [`unit`] of work, rolled back whole if it fails.

/// `Data`, `data!` and the rest, the domain's.
pub mod data {
    pub use domain::data;
    pub use domain::data::*;
}
pub mod bound;
pub mod layout;
pub mod tools;
pub mod unit;
pub mod view;

pub use domain::{error, event, font, port, product};
pub use domain::{fnv, raw_id_of, resident_mb, Before, DomainError, Handle, Location, Output, Session, ToolError, Toolkit};

pub use data::Data;

/// A call that may change what is open: a document, or the files open.
pub trait Command {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError>;
}

/// A call that reads what is open and changes none of it.
pub trait Query {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError>;
}
