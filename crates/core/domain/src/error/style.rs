//! set_properties' failures: what a node takes, its paints, effects and the
//! variables they bind.

use std::fmt;

use model::VariableType;
use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum StyleError {
    /// A node of a type set_properties does not change.
    NotChangeable { id: Guid, is: &'static str },
    /// No properties, or not an object of them.
    NoProperties { accepted: Vec<&'static str> },
    /// A property a text layer does not take.
    NotOnText { key: String, accepted: Vec<&'static str> },
    /// A property an instance takes from its component.
    NotOnInstance { key: String, accepted: Vec<&'static str> },
    /// A property another type takes, refused by a node of type `is`.
    NotOnThis { key: String, is: &'static str, accepted: Vec<&'static str> },
    /// A name set_properties does not know.
    NotAProperty { key: String, accepted: Vec<&'static str> },
    /// A variable mode set on an instance.
    ModesOnInstance,
    /// x or y on a node its auto-layout parent places.
    PlacedByLayout { key: &'static str, id: Guid, parent: Guid },
    /// One corner's radius on a shape other than a rectangle.
    CornersOnlyOnRectangle { key: &'static str },
    /// A height on a line.
    LineHeight,
    /// An instance's own value, when its component is not in the file.
    ComponentElsewhere { key: String, id: Guid },
    /// A stroke weight of 0.
    ZeroStrokeWeight,
    /// A dash pattern that is not dash and gap lengths.
    DashPattern { got: String },
    /// Wrapping auto-layout, which grafta cannot lay out.
    Wrap,
    /// x or y on a node with no transform.
    NoTransform { id: Guid },
    /// A paint with neither a colour nor a variable.
    PaintWithoutColour { at: String },
    /// A gradient with fewer than two stops.
    TooFewStops { at: String },
    /// A gradient given both an angle and a transform.
    AngleAndTransform { at: String },
    /// An angle on a radial gradient.
    AngleOnRadial { at: String },
    /// A variable the file does not have; `closest` names those near it.
    NoSuchVariable { at: String, wanted: String, count: usize, closest: Vec<String> },
    /// A variable that is not a colour, bound to a paint.
    NotAColour { at: String, name: String, kind: VariableType },
    /// A colour variable with no colour in its first mode.
    NoColourValue { at: String, name: String },
    /// An effect with no radius.
    NoRadius { at: String },
}

impl StyleError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NoProperties { .. }
            | Self::NotOnText { .. }
            | Self::NotOnInstance { .. }
            | Self::NotOnThis { .. }
            | Self::NotAProperty { .. }
            | Self::ZeroStrokeWeight
            | Self::DashPattern { .. }
            | Self::PaintWithoutColour { .. }
            | Self::TooFewStops { .. }
            | Self::AngleAndTransform { .. }
            | Self::AngleOnRadial { .. }
            | Self::NoRadius { .. } => Kind::BadArgument,
            Self::NoSuchVariable { .. } => Kind::NotFound,
            Self::NotChangeable { .. }
            | Self::ModesOnInstance
            | Self::PlacedByLayout { .. }
            | Self::CornersOnlyOnRectangle { .. }
            | Self::LineHeight
            | Self::ComponentElsewhere { .. }
            | Self::Wrap
            | Self::NotAColour { .. } => Kind::Refused,
            Self::NoTransform { .. } | Self::NoColourValue { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotChangeable { id, is } => {
                write!(f, "{id} is a {is}; only frames, components, instances, text layers and vectors can be changed")
            }
            Self::NoProperties { accepted } => {
                write!(f, "parameter \"properties\": expected an object with any of: {}", accepted.join(", "))
            }
            Self::NotOnText { key, accepted } => {
                write!(f, "`{key}` is not accepted on a text layer. Here it accepts: {}", accepted.join(", "))
            }
            Self::NotOnInstance { key, accepted } => write!(
                f,
                "`{key}` is not accepted on an instance: an instance takes its component's size and style. \
                 Here it accepts: {}",
                accepted.join(", ")
            ),
            Self::NotOnThis { key, is, accepted } => {
                write!(f, "`{key}` is not accepted on a {is} node. Here it accepts: {}", accepted.join(", "))
            }
            Self::NotAProperty { key, accepted } => {
                write!(f, "`{key}` is not a property set_properties accepts. Here it accepts: {}", accepted.join(", "))
            }
            Self::ModesOnInstance => f.write_str(
                "explicitVariableModes: an instance draws its component's stored values, and a mode set on \
                 one has not been opened in Figma; set it on a frame around the instance",
            ),
            Self::PlacedByLayout { key, id, parent } => write!(
                f,
                "{key}: {id} is placed by its auto-layout parent {parent}. Set layoutPositioning \
                 ABSOLUTE first to place it by hand, or change the parent's layout"
            ),
            Self::CornersOnlyOnRectangle { key } => {
                write!(f, "{key}: only a rectangle has corners of its own; use cornerRadius")
            }
            Self::LineHeight => f.write_str("height: a line has none; width is its length"),
            Self::ComponentElsewhere { key, id } => write!(
                f,
                "{key}: {id}'s component is not in this file, so what the instance keeps of it is unknown"
            ),
            Self::ZeroStrokeWeight => {
                f.write_str("strokeWeight must be greater than 0; to remove a border, set strokes: []")
            }
            Self::DashPattern { got } => write!(
                f,
                "dashPattern: expected an even count of lengths of 0 or more, dash then gap, or [] for a \
                 solid stroke; got {got}"
            ),
            Self::Wrap => f.write_str(
                "layoutWrap WRAP is not laid out by grafta yet, and Figma does not lay auto-layout out \
                 on open, so the children would stay where they were written",
            ),
            Self::NoTransform { id } => write!(f, "{id} has no transform to move"),
            Self::PaintWithoutColour { at } => write!(f, "{at}: a paint needs a color or a variable"),
            Self::TooFewStops { at } => write!(f, "{at}.gradientStops: a gradient needs at least two stops"),
            Self::AngleAndTransform { at } => write!(f, "{at}: give angle or gradientTransform, not both"),
            Self::AngleOnRadial { at } => write!(
                f,
                "{at}.angle: only a linear gradient has an angle; a radial one takes gradientTransform"
            ),
            Self::NoSuchVariable { at, wanted, count, closest } => {
                if closest.is_empty() {
                    write!(f, "{at}: this file has no variable `{wanted}` (it has {count} variables)")
                } else {
                    write!(f, "{at}: this file has no variable `{wanted}`. Closest: {}", closest.join(", "))
                }
            }
            Self::NotAColour { at, name, kind } => write!(f, "{at}: `{name}` is a {kind:?} variable, not a colour"),
            Self::NoColourValue { at, name } => write!(f, "{at}: `{name}` has no colour value in its first mode"),
            Self::NoRadius { at } => write!(f, "{at}: radius is required"),
        }
    }
}
