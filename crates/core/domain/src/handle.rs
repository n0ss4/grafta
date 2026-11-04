//! The name a model gives an open file by.

use std::borrow::Borrow;
use std::fmt;
use std::path::Path;

use serde::Serialize;

use crate::fnv;

/// The longest file stem a handle keeps.
const STEM_CHARS: usize = 24;
/// The bits of the path's hash a handle shows: four hex digits.
const HASH_MASK: u64 = 0xffff;

/// A short handle a model can read: the file stem and a hash of the full path,
/// so two files with the same name in different folders do not collide, and
/// the same path always gets the same handle.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct Handle(String);

impl Handle {
    #[must_use]
    pub fn for_path(path: &Path) -> Self {
        let stem = path.file_stem().map_or_else(|| "file".into(), |s| s.to_string_lossy().into_owned());
        let stem: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).take(STEM_CHARS).collect();
        Self(format!("{stem}-{:04x}", fnv(path.to_string_lossy().as_bytes()) & HASH_MASK))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Borrow<str> for Handle {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
