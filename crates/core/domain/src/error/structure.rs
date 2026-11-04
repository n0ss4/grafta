//! The structure tools' failures: grouping, ungrouping, sections and
//! duplicates.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum StructureError {
    /// No order keys fit between two siblings.
    NoKeysBetween,
    /// No list of ids.
    NoIds,
    /// A list with something other than ids in it.
    NotIds,
    /// A page, grouped.
    GroupedPage { id: String },
    /// A rotated layer, grouped.
    Rotated { id: String },
    /// Layers of different parents, grouped.
    OtherParent { id: String, first: String },
    /// A group in an auto-layout flow.
    InAutoLayout { parent: Guid },
    /// A node that is not a group, ungrouped.
    NotAGroup { id: Guid, is: &'static str },
    /// A section off a page.
    NotAPage { id: Guid },
    /// A section of no area.
    SectionSize { width: f32, height: f32 },
    /// A page, duplicated.
    DuplicatedPage { id: Guid },
    /// A component, duplicated.
    DuplicatedComponent { component: Guid },
    /// A copy that could not be read back.
    CopyNotReadBack { id: Guid, error: String },
}

impl StructureError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NoIds | Self::NotIds | Self::SectionSize { .. } => Kind::BadArgument,
            Self::GroupedPage { .. }
            | Self::Rotated { .. }
            | Self::OtherParent { .. }
            | Self::InAutoLayout { .. }
            | Self::NotAGroup { .. }
            | Self::NotAPage { .. }
            | Self::DuplicatedPage { .. }
            | Self::DuplicatedComponent { .. } => Kind::Refused,
            Self::NoKeysBetween | Self::CopyNotReadBack { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for StructureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoKeysBetween => f.write_str("no order key fits between those siblings"),
            Self::NoIds => f.write_str("parameter \"ids\": a list of node ids is required"),
            Self::NotIds => f.write_str("parameter \"ids\": expected one or more node ids"),
            Self::GroupedPage { id } => write!(f, "{id} is a page; layers are grouped, pages are not"),
            Self::Rotated { id } => write!(
                f,
                "{id} is rotated; a group's box is its children's, and a rotated layer's bounds are not measured yet"
            ),
            Self::OtherParent { id, first } => write!(f, "{id} has another parent than {first}: grouped layers share one"),
            Self::InAutoLayout { parent } => write!(
                f,
                "{parent} lays its children out automatically; a group in an auto-layout flow is not measured yet"
            ),
            Self::NotAGroup { id, is } => write!(f, "{id} is a {is}, not a group"),
            Self::NotAPage { id } => write!(f, "{id} is not a page; a section goes on a page"),
            Self::SectionSize { width, height } => {
                write!(f, "a section needs a width and a height above 0, got {width}×{height}")
            }
            Self::DuplicatedPage { id } => write!(f, "{id} is a page; duplicate what is on it"),
            Self::DuplicatedComponent { component } => write!(
                f,
                "{component} is a component: a copy would be a second definition, which is not measured. Place an \
                 instance with create_instance"
            ),
            Self::CopyNotReadBack { id, error } => {
                write!(f, "duplicated {id} but could not read the copy back: {error}")
            }
        }
    }
}
