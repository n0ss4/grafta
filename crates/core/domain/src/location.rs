//! Where the toolkit writes a file.

use std::path::Path;

use crate::port::DocumentStore;
use crate::{DomainError, ToolError};

/// A path nothing is at yet. The toolkit never overwrites a file, so every
/// file it writes goes to one.
#[derive(Debug, Clone, Copy)]
pub struct Location<'a>(&'a Path);

impl<'a> Location<'a> {
    /// `path`, refused when a file is there; `opened_from` names the file a
    /// session reads, which the refusal then says it is.
    pub fn new_file(store: &dyn DocumentStore, path: &'a Path, opened_from: Option<&Path>) -> Result<Self, ToolError> {
        if !store.exists(path) {
            return Ok(Self(path));
        }
        let own = opened_from.is_some_and(|opened| store.canonical(path).ok().as_deref() == Some(opened));
        Err(DomainError::WouldOverwrite { path: path.into(), own }.into())
    }

    #[must_use]
    pub fn path(self) -> &'a Path {
        self.0
    }
}
