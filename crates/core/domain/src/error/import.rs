//! import_layout's failures: the snapshot it reads.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum ImportError {
    /// A file that is not JSON.
    NotJson { path: String, error: String },
    /// JSON that is not a snapshot of this version; `says` is what it says.
    NotASnapshot { path: String, version: &'static str, says: String },
    /// A snapshot of more nodes than an import takes.
    TooManyNodes { path: String, max: u64 },
    /// A snapshot with no root.
    NoRoot { path: String },
    /// A layer made with no id.
    MadeWithoutId,
    /// A picture where a layer was made.
    PictureReturned,
}

impl ImportError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NotJson { .. } | Self::NoRoot { .. } => Kind::BadArgument,
            Self::NotASnapshot { .. } | Self::TooManyNodes { .. } => Kind::Refused,
            Self::MadeWithoutId | Self::PictureReturned => Kind::Failed,
        }
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotJson { path, error } => write!(f, "{path} is not JSON: {error}"),
            Self::NotASnapshot { path, version, says } => write!(
                f,
                "{path} is not a {version} snapshot (it says {says}); take it with grafta's scripts/snapshot.js"
            ),
            Self::TooManyNodes { path, max } => {
                write!(f, "{path} has more than {max} nodes; snapshot a part of the page, by its selector")
            }
            Self::NoRoot { path } => write!(f, "{path} has no root"),
            Self::MadeWithoutId => f.write_str("a layer was made with no id"),
            Self::PictureReturned => f.write_str("a tool returned a picture"),
        }
    }
}
