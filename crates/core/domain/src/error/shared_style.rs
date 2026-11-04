//! The shared style tools' failures: making, changing and applying fill, text
//! and effect styles.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum SharedStyleError {
    /// A grid style, made.
    GridMade,
    /// A grid style, changed.
    GridChanged,
    /// A style of one kind where another is taken.
    WrongKind { at: String, name: String, is: &'static str, takes: &'static str },
    /// A library style, or one removed from the file.
    LibraryOrRemoved { at: String, wanted: String },
    /// A style of `kind` the file does not have; `closest` names those near it.
    NoSuchStyle { at: String, kind: &'static str, wanted: String, closest: Vec<String> },
    /// A name more than one style has.
    AmbiguousStyle { at: String, wanted: String, ids: Vec<String> },
    /// A value a style of this kind does not have.
    NotAValue { key: &'static str, allowed: Vec<&'static str> },
    /// A new FILL style with no paints.
    NoPaints,
    /// A new EFFECT style with no effects.
    NoEffects,
    /// A new TEXT style with no size.
    NoSize,
    /// A style and the direct value it would set, both given.
    BothSet { property: &'static str, direct: &'static str },
    /// A style name already taken by a style of its kind.
    NameTaken { kind: &'static str, name: String, id: Guid },
    /// A style made and not read back.
    MadeNotReadBack { id: Guid },
    /// A style changed and not read back.
    ChangedNotReadBack { id: Guid },
    /// A style id or name the file does not have.
    NoStyleWithId { wanted: String },
}

impl SharedStyleError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::AmbiguousStyle { .. }
            | Self::NotAValue { .. }
            | Self::NoPaints
            | Self::NoEffects
            | Self::NoSize
            | Self::BothSet { .. } => Kind::BadArgument,
            Self::NoSuchStyle { .. } | Self::NoStyleWithId { .. } => Kind::NotFound,
            Self::GridMade
            | Self::GridChanged
            | Self::WrongKind { .. }
            | Self::LibraryOrRemoved { .. }
            | Self::NameTaken { .. } => Kind::Refused,
            Self::MadeNotReadBack { .. } | Self::ChangedNotReadBack { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for SharedStyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GridMade => f.write_str("type: grid styles are not written; grafta does not write layout grids"),
            Self::GridChanged => f.write_str("grid styles are not written"),
            Self::WrongKind { at, name, is, takes } => write!(f, "{at}: `{name}` is a {is} style; {at} takes a {takes} style"),
            Self::LibraryOrRemoved { at, wanted } => write!(
                f,
                "{at}: `{wanted}` is a library style or one removed from the file. A library style is pointed at by \
                 an asset key, which has not been tried in Figma"
            ),
            Self::NoSuchStyle { at, kind, wanted, closest } => {
                if closest.is_empty() {
                    write!(f, "{at}: this file has no {kind} style `{wanted}`")
                } else {
                    write!(f, "{at}: this file has no {kind} style `{wanted}`. Closest: {}", closest.join(", "))
                }
            }
            Self::AmbiguousStyle { at, wanted, ids } => {
                write!(f, "{at}: {} styles are named `{wanted}`; pass one id: {}", ids.len(), ids.join(", "))
            }
            Self::NotAValue { key, allowed } => {
                write!(f, "{key} is not a value of this style; it takes {}", allowed.join(", "))
            }
            Self::NoPaints => f.write_str("fills: a FILL style needs its paints"),
            Self::NoEffects => f.write_str("effects: an EFFECT style needs its effects"),
            Self::NoSize => f.write_str("fontSize: a TEXT style needs its size"),
            Self::BothSet { property, direct } => write!(f, "{property} and {direct} both set {direct}; pass one"),
            Self::NameTaken { kind, name, id } => write!(f, "a {kind} style is already named `{name}`: {id}"),
            Self::MadeNotReadBack { id } => write!(f, "made style {id} but could not read it back"),
            Self::ChangedNotReadBack { id } => write!(f, "{id} could not be read back"),
            Self::NoStyleWithId { wanted } => write!(f, "id: this file has no style `{wanted}`"),
        }
    }
}
