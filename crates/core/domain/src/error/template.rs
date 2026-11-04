//! fill_template's failures: the rows, the component they fill, and the
//! values each row sets.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum TemplateError {
    /// A variant set with no variants.
    EmptySet { component: String },
    /// A variant value that is not text.
    VariantValueNotText { at: String, got: String },
    /// Values no variant of the set has.
    NoSuchVariant { row: usize, set: String, variants: Vec<String> },
    /// A frame, which is not yet a component.
    AFrame { component: String },
    /// A node that is not a component.
    NotAComponent { component: String, is: &'static str },
    /// A name that is not text.
    NameNotText { at: String, got: String },
    /// A key that is no property, variant property or layer.
    NoSuchKey { at: String, component: String, keys: Vec<String> },
    /// A key more than one layer is named.
    SeveralLayers { at: String, ids: Vec<String> },
    /// A layer's value that is neither text nor an image path.
    LayerValueNotText { at: String, got: String },
    /// A value for a layer that is not text, not naming an image.
    NotAnImagePath { at: String, given: String },
    /// An image path with no file at it.
    NoImage { at: String, given: String },
    /// An instance placed with no id.
    PlacedWithoutId,
    /// A picture where an instance was placed.
    PictureReturned,
    /// Both rows and csv.
    RowsAndCsv,
    /// A CSV file that does not parse.
    Csv { path: String, error: String },
    /// Neither rows nor csv.
    NoRows,
    /// No rows, or more than `max`.
    RowCount { max: usize, count: usize },
}

impl TemplateError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::VariantValueNotText { .. }
            | Self::NameNotText { .. }
            | Self::NoSuchKey { .. }
            | Self::LayerValueNotText { .. }
            | Self::NotAnImagePath { .. }
            | Self::RowsAndCsv
            | Self::Csv { .. }
            | Self::NoRows
            | Self::RowCount { .. } => Kind::BadArgument,
            Self::NoImage { .. } => Kind::NotFound,
            Self::EmptySet { .. }
            | Self::NoSuchVariant { .. }
            | Self::AFrame { .. }
            | Self::NotAComponent { .. }
            | Self::SeveralLayers { .. } => Kind::Refused,
            Self::PlacedWithoutId | Self::PictureReturned => Kind::Failed,
        }
    }
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySet { component } => write!(f, "{component} is a set with no variants"),
            Self::VariantValueNotText { at, got } => write!(f, "{at}: a variant value is text, got {got}"),
            Self::NoSuchVariant { row, set, variants } => {
                write!(f, "row {row}: {set} has no variant with those values; it has: {}", variants.join("; "))
            }
            Self::AFrame { component } => write!(
                f,
                "{component} is a frame: make it a component first (create_component), so each copy is an \
                 instance whose values the rows set"
            ),
            Self::NotAComponent { component, is } => write!(f, "{component} is a {is}, not a component"),
            Self::NameNotText { at, got } => write!(f, "{at}: a name is text, got {got}"),
            Self::NoSuchKey { at, component, keys } => write!(
                f,
                "{at}: {component} has no property, variant property or layer of that name; the keys there are: {}",
                keys.join(", ")
            ),
            Self::SeveralLayers { at, ids } => write!(
                f,
                "{at}: {} layers are named so ({}); rename them apart, or set one with override_instance",
                ids.len(),
                ids.join(", ")
            ),
            Self::LayerValueNotText { at, got } => {
                write!(f, "{at}: a layer's value is text or an image path, got {got}")
            }
            Self::NotAnImagePath { at, given } => write!(
                f,
                "{at}: the layer is not a text, so its value is an image path ending .png, .jpg or .jpeg, got {given}"
            ),
            Self::NoImage { at, given } => write!(f, "{at}: no image at {given}"),
            Self::PlacedWithoutId => f.write_str("an instance was placed with no id"),
            Self::PictureReturned => f.write_str("create_instance returned a picture"),
            Self::RowsAndCsv => f.write_str("give rows or csv, not both"),
            Self::Csv { path, error } => write!(f, "{path}: {error}"),
            Self::NoRows => f.write_str("give rows (a JSON array of objects) or csv (a file)"),
            Self::RowCount { max, count } => write!(f, "expected 1 to {max} rows, got {count}"),
        }
    }
}
