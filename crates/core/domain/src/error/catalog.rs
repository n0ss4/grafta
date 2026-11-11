//! The reading tools' failures: components, variables, images, drawings and
//! PDFs.

use std::fmt;
use std::path::PathBuf;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum CatalogError {
    /// A node that is not a component or a set, asked for its instances.
    NotAComponent { id: Guid, is: &'static str },
    /// A node that is not a page, asked for its screens.
    NotAPage { id: Guid, is: &'static str },
    /// A collection the file does not have.
    NoCollection { name: String },
    /// An image the file does not embed.
    NoImage { hash: String, file: String, count: usize },
    /// A node of no area, drawn.
    NoArea { id: Guid, name: String, width: f32, height: f32 },
    /// An empty list of ids.
    NoIds,
    /// A PDF path that cannot be opened to write.
    Unwritable { path: PathBuf, error: String },
}

impl CatalogError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NotAComponent { .. } | Self::NotAPage { .. } | Self::NoArea { .. } | Self::NoIds => Kind::BadArgument,
            Self::NoCollection { .. } | Self::NoImage { .. } => Kind::NotFound,
            Self::Unwritable { .. } => Kind::Refused,
        }
    }
}

impl fmt::Display for CatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAComponent { id, is } => write!(f, "{id} is a {is}, not a component or component set"),
            Self::NotAPage { id, is } => write!(f, "{id} is a {is}, not a page: summarize lists the pages"),
            Self::NoCollection { name } => write!(f, "no variable collection named `{name}`"),
            Self::NoImage { hash, file, count } => write!(f, "no image with hash {hash} in {file} ({count} images embedded)"),
            Self::NoArea { id, name, width, height } => write!(f, "{id} \"{name}\" has no area to draw ({width}×{height})"),
            Self::NoIds => f.write_str("parameter \"ids\": expected at least one node id"),
            Self::Unwritable { path, error } => write!(f, "cannot write {}: {error}", path.display()),
        }
    }
}
