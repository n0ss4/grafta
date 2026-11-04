//! new_file's failures: the template it starts from, and the file it writes.

use std::fmt;
use std::path::PathBuf;

use super::Kind;

#[derive(Debug)]
pub enum NewFileError {
    /// A deck's or board's extension: neither is made from nothing yet.
    NotFromNothing { path: PathBuf, product: &'static str },
    /// A template that cannot be read.
    CannotReadTemplate { path: PathBuf, error: String },
    /// A template that does not decode.
    NotATemplate { path: PathBuf, error: String },
    /// A new file that does not encode.
    CannotEncode { error: String },
    /// A new file that does not decode.
    NotReadBack { error: String },
    /// A new file that decodes to something else.
    ChangedInTransit { at: String },
    /// A template without the entries Figma needs.
    MissingEntries { missing: Vec<&'static str> },
    /// A template that cannot be emptied.
    CannotEmpty(model::author::AuthorError),
    /// A thumbnail that cannot be drawn.
    NoThumbnail { error: String },
}

impl NewFileError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NotFromNothing { .. } | Self::NotATemplate { .. } => Kind::BadArgument,
            Self::CannotReadTemplate { .. } => Kind::NotFound,
            Self::MissingEntries { .. } => Kind::Refused,
            Self::CannotEncode { .. }
            | Self::NotReadBack { .. }
            | Self::ChangedInTransit { .. }
            | Self::CannotEmpty(_)
            | Self::NoThumbnail { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for NewFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFromNothing { path, product } => write!(
                f,
                "{}: a new file is a Figma Design .fig; a {product} file from nothing is not made yet, so open one \
                 and save_as a copy",
                path.display()
            ),
            Self::CannotReadTemplate { path, error } => write!(f, "cannot read the template {}: {error}", path.display()),
            Self::NotATemplate { path, error } => {
                write!(f, "the template {} is not a .fig grafta reads: {error}", path.display())
            }
            Self::CannotEncode { error } => write!(f, "cannot encode the new file: {error}"),
            Self::NotReadBack { error } => write!(f, "the new file does not read back: {error}"),
            Self::ChangedInTransit { at } => write!(f, "the new file changed in transit at {at}"),
            Self::MissingEntries { missing } => write!(
                f,
                "the template has no {}, and Figma refuses a file without it (\"Internal error during \
                 import\", measured). grafta does not invent one; use another .fig as the template",
                missing.join(" or ")
            ),
            Self::CannotEmpty(error) => write!(f, "cannot empty the template: {error}"),
            Self::NoThumbnail { error } => write!(f, "cannot draw the thumbnail: {error}"),
        }
    }
}
