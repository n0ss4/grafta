//! The variable tools' failures: collections, modes, values and aliases.

use std::fmt;

use model::VariableType;
use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum VariableError {
    /// A collection the file does not have; `names` are those it has.
    NoSuchCollection { wanted: String, names: Vec<String> },
    /// A name more than one collection has.
    AmbiguousCollection { wanted: String, ids: Vec<String> },
    /// A variable the file does not have; `closest` names those near it.
    NoSuchVariable { at: String, wanted: String, closest: Vec<String> },
    /// A collection or variable imported from a library.
    Library { what: &'static str, id: Guid },
    /// A collection or variable the file had before grafta.
    FilesOwn { what: &'static str, id: Guid },
    /// An alias to a library variable.
    AliasToLibrary { at: String, name: String },
    /// An alias to a variable of another type.
    AliasType { at: String, name: String, kind: VariableType, wanted: VariableType },
    /// An alias that leads back to its own variable.
    AliasCycle { at: String, name: String },
    /// A value for a mode the collection does not have.
    NoSuchMode { key: String, collection: String, modes: Vec<String> },
    /// A collection whose modes have no ids it declares.
    NoModeIds { collection: String },
    /// A new variable with no value for one of its collection's modes.
    MissingValue { mode: String, modes: Vec<String> },
    /// A new collection given no list of modes.
    NoModeList,
    /// A mode named twice.
    ModeTwice { index: usize, mode: String },
    /// An empty list of modes.
    NoModes,
    /// A collection name already taken.
    CollectionNameTaken { name: String, id: Guid },
    /// A variable name its collection already has.
    VariableNameTaken { collection: String, name: String, id: Guid },
    /// A new name for a variable that its collection already has.
    RenameTaken { name: String, id: Guid },
    /// A variable whose collection is not in the file.
    NoCollection { id: Guid },
    /// A mode its collection already has.
    ModeTaken { collection: String, mode: String },
}

impl VariableError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::AmbiguousCollection { .. }
            | Self::NoSuchMode { .. }
            | Self::MissingValue { .. }
            | Self::NoModeList
            | Self::ModeTwice { .. }
            | Self::NoModes => Kind::BadArgument,
            Self::NoSuchCollection { .. } | Self::NoSuchVariable { .. } => Kind::NotFound,
            Self::Library { .. }
            | Self::FilesOwn { .. }
            | Self::AliasToLibrary { .. }
            | Self::AliasType { .. }
            | Self::AliasCycle { .. }
            | Self::CollectionNameTaken { .. }
            | Self::VariableNameTaken { .. }
            | Self::RenameTaken { .. }
            | Self::ModeTaken { .. } => Kind::Refused,
            Self::NoModeIds { .. } | Self::NoCollection { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for VariableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchCollection { wanted, names } => write!(
                f,
                "no variable collection `{wanted}`. The collections: {}",
                if names.is_empty() { "none".to_string() } else { names.join(", ") }
            ),
            Self::AmbiguousCollection { wanted, ids } => {
                write!(f, "{} collections are named `{wanted}`; pass one id: {}", ids.len(), ids.join(", "))
            }
            Self::NoSuchVariable { at, wanted, closest } => {
                if closest.is_empty() {
                    write!(f, "{at}: this file has no variable `{wanted}`")
                } else {
                    write!(f, "{at}: this file has no variable `{wanted}`. Closest: {}", closest.join(", "))
                }
            }
            Self::Library { what, id } => {
                write!(f, "{what} {id} is imported from a library; it can only be changed there")
            }
            Self::FilesOwn { what, id } => write!(
                f,
                "{what} {id} is the file's own. Only collections and variables grafta made can be changed: \
                 a change to the file's own reaches every binding in it, which is not measured yet"
            ),
            Self::AliasToLibrary { at, name } => write!(
                f,
                "{at}: `{name}` is imported from a library. None of the 1,579 aliases measured points at one, \
                 so an alias to a library variable is not written"
            ),
            Self::AliasType { at, name, kind, wanted } => write!(
                f,
                "{at}: `{name}` is a {kind:?} variable, and an alias must have its variable's type ({wanted:?})"
            ),
            Self::AliasCycle { at, name } => write!(f, "{at}: aliasing `{name}` would make a cycle"),
            Self::NoSuchMode { key, collection, modes } => {
                write!(f, "values.{key}: `{collection}` has no mode `{key}`. Its modes: {}", modes.join(", "))
            }
            Self::NoModeIds { collection } => write!(f, "`{collection}` declares no mode ids"),
            Self::MissingValue { mode, modes } => write!(
                f,
                "values: no value for mode `{mode}`. A variable has a value in every mode of its collection ({})",
                modes.join(", ")
            ),
            Self::NoModeList => f.write_str("parameter \"modes\": a list of mode names is required"),
            Self::ModeTwice { index, mode } => write!(f, "modes[{index}]: `{mode}` is named twice"),
            Self::NoModes => f.write_str("modes: a collection needs at least one mode"),
            Self::CollectionNameTaken { name, id } => write!(f, "a collection is already named `{name}`: {id}"),
            Self::VariableNameTaken { collection, name, id } => {
                write!(f, "`{collection}` already has a variable named `{name}`: {id}")
            }
            Self::RenameTaken { name, id } => {
                write!(f, "its collection already has a variable named `{name}`: {id}")
            }
            Self::NoCollection { id } => write!(f, "variable {id} has no collection in this file"),
            Self::ModeTaken { collection, mode } => write!(f, "`{collection}` already has a mode `{mode}`"),
        }
    }
}
