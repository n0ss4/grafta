#![deny(clippy::all)]
// The one crate allowed to reach the format by string.
#![allow(clippy::disallowed_methods)]

//! The `.fig` format's names, each written once: its fields, its enum values
//! and its node ids, and typed access to a decoded message through them.

mod access;
mod enums;
mod field;
mod generated;
mod guid;
mod table;
mod wire;

pub use access::{fields, object, Wire, WireMut, Wired};
pub use enums::*;
pub use field::{may_index_blobs, Field};
pub use generated::{GeneratedLayer, GENERATED_SESSION};
pub use guid::{Guid, GuidError};
pub use table::{table_entries, table_sizes, TableAxis};
pub use wire::WireName;

