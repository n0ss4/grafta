//! Variable bindings' failures: numbers bound to variables, and the modes a
//! frame is shown in.

use std::fmt;

use model::VariableType;
use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum BindingError {
    /// A visibility bound to a variable, which did nothing in Figma.
    VisibleBound,
    /// A variable that is not a number, bound to a number.
    NotANumber { property: String, name: String, kind: VariableType },
    /// A number variable with nothing to resolve to.
    NoNumber { property: String, name: String },
    /// A collection the file does not have.
    NoSuchCollection { name: String, collections: Vec<String> },
    /// A mode the collection does not have.
    NoSuchMode { name: String, collection: String, mode: String, modes: Vec<String> },
    /// A mode id that is not a guid.
    NotAGuid { id: Guid },
}

impl BindingError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NoSuchMode { .. } => Kind::BadArgument,
            Self::NoSuchCollection { .. } => Kind::NotFound,
            Self::VisibleBound | Self::NotANumber { .. } => Kind::Refused,
            Self::NoNumber { .. } | Self::NotAGuid { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for BindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VisibleBound => f.write_str(
                "visible: a visibility bound to a variable, in the shape measured on library bindings, did nothing \
                 in Figma, so it is not written",
            ),
            Self::NotANumber { property, name, kind } => {
                write!(f, "{property}: `{name}` is a {kind:?} variable, and {property} takes a number")
            }
            Self::NoNumber { property, name } => write!(f, "{property}: `{name}` has no number to resolve to"),
            Self::NoSuchCollection { name, collections } => write!(
                f,
                "explicitVariableModes: no collection `{name}`. The collections: {}",
                collections.join(", ")
            ),
            Self::NoSuchMode { name, collection, mode, modes } => write!(
                f,
                "explicitVariableModes.{name}: `{collection}` has no mode {mode}. Its modes: {} (null clears it)",
                modes.join(", ")
            ),
            Self::NotAGuid { id } => write!(f, "{id} is not a guid"),
        }
    }
}
