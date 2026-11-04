//! Sizing's failures: hug and fill, limits and rotation.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum SizingError {
    /// FILL in a parent with no auto-layout.
    FillWithoutLayout { key: String, parent: String },
    /// HUG on a text layer.
    HugText { key: String },
    /// HUG on a frame with no auto-layout.
    HugWithoutLayout { key: String, id: Guid },
    /// A minimum larger than its maximum.
    MinAboveMax,
    /// A rotation on a node with no transform.
    NoTransformToRotate,
}

impl SizingError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::MinAboveMax => Kind::BadArgument,
            Self::FillWithoutLayout { .. } | Self::HugText { .. } | Self::HugWithoutLayout { .. } => Kind::Refused,
            Self::NoTransformToRotate => Kind::Failed,
        }
    }
}

impl fmt::Display for SizingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FillWithoutLayout { key, parent } => {
                write!(f, "{key}: FILL takes the free space of an auto-layout parent, and {parent} has none")
            }
            Self::HugText { key } => write!(
                f,
                "{key}: a text hugs its characters unless it has a fixed width; give it none with width left unset"
            ),
            Self::HugWithoutLayout { key, id } => write!(
                f,
                "{key}: HUG wraps a frame around its auto-layout children, and {id} has no auto-layout; set \
                 layoutMode first"
            ),
            Self::MinAboveMax => f.write_str("a minimum is larger than the maximum"),
            Self::NoTransformToRotate => f.write_str("the node has no transform to rotate"),
        }
    }
}
