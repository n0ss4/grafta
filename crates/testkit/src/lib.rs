// Builds messages for tests, spelling the wire apart from `schema`.
#![allow(clippy::disallowed_methods)]
//! Test scaffolding shared across the workspace.
//!
//! Every crate's tests used to hand-roll the same handful of scene graphs. They
//! live here instead, so a change to the model updates one builder rather than
//! six copies, and so a test reads as what it asserts rather than as forty lines
//! of setup.

pub mod fixture;
pub mod raw;
pub mod scene;

pub use scene::GraphBuilder;
pub use model;
