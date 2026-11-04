//! Component properties' failures: their values on an instance, and the
//! layers that reference them.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum PropertyError {
    /// An instance-swap value naming no component.
    NoSuchComponent { at: String, wanted: String },
    /// A name more than one component has.
    AmbiguousComponent { at: String, wanted: String, ids: Vec<String> },
    /// A TEXT property's value that is not a string.
    NotText { at: String, got: String },
    /// A BOOLEAN property's value that is not true or false.
    NotBoolean { at: String, got: String },
    /// A property of a kind set elsewhere, variants with set_variant.
    SetElsewhere { at: String, kind: String },
    /// A property the component does not have.
    NoSuchProperty { at: String, wanted: String, properties: Vec<String> },
    /// A reference from a layer outside any component.
    OutsideComponent { id: Guid },
    /// A key a layer's references do not take.
    NotAReference { key: String },
    /// A reference on a layer of the wrong type.
    WrongLayer { key: String, id: Guid, is: &'static str, wanted: &'static str },
    /// A reference to a property of another type.
    WrongType { key: String, name: String, is: String, takes: String },
    /// A property with no default value.
    NoDefault { name: String },
    /// componentProperties on a node that is not an instance.
    NotAnInstance { id: Guid },
    /// An instance with no component.
    NoComponent { id: Guid },
}

impl PropertyError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::AmbiguousComponent { .. } | Self::NotText { .. } | Self::NotBoolean { .. } | Self::NotAReference { .. } => {
                Kind::BadArgument
            }
            Self::NoSuchComponent { .. } | Self::NoSuchProperty { .. } => Kind::NotFound,
            Self::SetElsewhere { .. }
            | Self::OutsideComponent { .. }
            | Self::WrongLayer { .. }
            | Self::WrongType { .. }
            | Self::NotAnInstance { .. } => Kind::Refused,
            Self::NoDefault { .. } | Self::NoComponent { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for PropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSuchComponent { at, wanted } => {
                write!(f, "{at}: no component `{wanted}`; an instance-swap property shows a component")
            }
            Self::AmbiguousComponent { at, wanted, ids } => {
                write!(f, "{at}: {} components are named `{wanted}`; pass one id: {}", ids.len(), ids.join(", "))
            }
            Self::NotText { at, got } => write!(f, "{at}: a TEXT property takes a string, got {got}"),
            Self::NotBoolean { at, got } => write!(f, "{at}: a BOOLEAN property takes true or false, got {got}"),
            Self::SetElsewhere { at, kind } => {
                write!(f, "{at}: a {kind} property is not set here; variants are set with set_variant")
            }
            Self::NoSuchProperty { at, wanted, properties } => write!(
                f,
                "{at}: the component has no property `{wanted}`. Its properties: {}",
                if properties.is_empty() { "none".to_string() } else { properties.join(", ") }
            ),
            Self::OutsideComponent { id } => write!(
                f,
                "componentPropertyReferences: {id} is not inside a component; a property is bound from a layer of \
                 its component"
            ),
            Self::NotAReference { key } => {
                write!(f, "componentPropertyReferences.{key}: expected characters, visible or mainComponent")
            }
            Self::WrongLayer { key, id, is, wanted } => {
                write!(f, "componentPropertyReferences.{key}: {id} is a {is}; {key} is bound on a {wanted}")
            }
            Self::WrongType { key, name, is, takes } => {
                write!(f, "componentPropertyReferences.{key}: `{name}` is a {is} property; {key} takes a {takes}")
            }
            Self::NoDefault { name } => write!(f, "`{name}` has no default value"),
            Self::NotAnInstance { id } => write!(f, "componentProperties: {id} is not an instance"),
            Self::NoComponent { id } => write!(f, "{id} has no component"),
        }
    }
}
