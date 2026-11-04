//! The component tools' failures: making components, variant sets, switching
//! variants and adding properties.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum ComponentError {
    /// A node other than a frame, made a component.
    NotAFrame { id: Guid, is: &'static str },
    /// A frame inside a component.
    InsideComponent { id: Guid, component: Guid, name: String },
    /// A frame inside an instance.
    InsideInstance { id: Guid, instance: Guid, name: String },
    /// No list of variants.
    NoVariantList,
    /// Fewer than two variants.
    TooFewVariants,
    /// A variant with no id.
    NoVariantId { index: usize },
    /// A variant with no object of values.
    NoVariantValues { index: usize },
    /// A property or value with `=` or `,` in it, or blank.
    NotAVariantName { index: usize, property: String, value: String },
    /// A variant that names no property.
    NoVariantProperty { index: usize },
    /// A variant that names other properties than the first.
    OtherProperties { id: String, first: String, properties: Vec<String> },
    /// A variant with the same values as another.
    RepeatedValues { id: String },
    /// A node other than a component, made a variant.
    NotAComponent { id: String, is: &'static str },
    /// A component already in a set.
    AlreadyAVariant { id: String, set: Guid },
    /// A component with another parent than the others.
    NotBeside { id: String },
    /// An instance of a component that is in no set.
    NotAVariant { id: Guid, component: String },
    /// No object of values to switch to.
    NoValues,
    /// A property the set does not have.
    NoSuchProperty { set: String, property: String, properties: Vec<String> },
    /// Values no variant of the set has.
    NoSuchVariant { set: String, variants: Vec<String> },
    /// A property added to a node that is not a component.
    PropertyOffComponent { id: Guid, is: &'static str },
    /// A property added to one variant of a set.
    PropertyOnVariant { id: Guid, set: Guid },
    /// A property name the component already has.
    PropertyTaken { id: Guid, name: String },
    /// A set made and not read back.
    SetNotReadBack { set: Guid, error: String },
}

impl ComponentError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NoVariantList
            | Self::TooFewVariants
            | Self::NoVariantId { .. }
            | Self::NoVariantValues { .. }
            | Self::NotAVariantName { .. }
            | Self::NoVariantProperty { .. }
            | Self::OtherProperties { .. }
            | Self::RepeatedValues { .. }
            | Self::NoValues
            | Self::NoSuchProperty { .. } => Kind::BadArgument,
            Self::NotAFrame { .. }
            | Self::InsideComponent { .. }
            | Self::InsideInstance { .. }
            | Self::NotAComponent { .. }
            | Self::AlreadyAVariant { .. }
            | Self::NotBeside { .. }
            | Self::NotAVariant { .. }
            | Self::NoSuchVariant { .. }
            | Self::PropertyOffComponent { .. }
            | Self::PropertyOnVariant { .. }
            | Self::PropertyTaken { .. } => Kind::Refused,
            Self::SetNotReadBack { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for ComponentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAFrame { id, is } => write!(f, "{id} is a {is}; only a frame can become a component"),
            Self::InsideComponent { id, component, name } => write!(
                f,
                "{id} is inside component {component} ({name}): Figma does not put a component inside another"
            ),
            Self::InsideInstance { id, instance, name } => write!(
                f,
                "{id} is inside instance {instance} ({name}): an instance shows its component, and has no frames \
                 of its own to promote"
            ),
            Self::NoVariantList => f.write_str("parameter \"variants\": a list of { id, values } is required"),
            Self::TooFewVariants => f.write_str("a variant set needs at least two components"),
            Self::NoVariantId { index } => write!(f, "variants[{index}].id is required"),
            Self::NoVariantValues { index } => {
                write!(f, "variants[{index}].values: an object of property → value is required")
            }
            Self::NotAVariantName { index, property, value } => write!(
                f,
                "variants[{index}]: `{property}={value}` cannot be a variant name; neither side may contain = or ,"
            ),
            Self::NoVariantProperty { index } => write!(f, "variants[{index}].values names no property"),
            Self::OtherProperties { id, first, properties } => {
                write!(f, "{id} names other properties than {first}: every variant names {}", properties.join(", "))
            }
            Self::RepeatedValues { id } => write!(f, "{id} repeats another variant's values"),
            Self::NotAComponent { id, is } => write!(f, "{id} is a {is}; only components become variants"),
            Self::AlreadyAVariant { id, set } => write!(f, "{id} is already a variant of set {set}"),
            Self::NotBeside { id } => write!(
                f,
                "{id} is not beside the others: the components of a set must share a parent (move_node first)"
            ),
            Self::NotAVariant { id, component } => {
                write!(f, "{id} is an instance of {component}, which is not a variant of a set")
            }
            Self::NoValues => f.write_str("parameter \"values\": an object of property → value is required"),
            Self::NoSuchProperty { set, property, properties } => {
                write!(f, "{set} has no property {property}; it has {}", properties.join(", "))
            }
            Self::NoSuchVariant { set, variants } => {
                write!(f, "{set} has no variant with those values; it has: {}", variants.join("; "))
            }
            Self::PropertyOffComponent { id, is } => write!(f, "{id} is a {is}; properties belong to a component"),
            Self::PropertyOnVariant { id, set } => write!(
                f,
                "{id} is a variant of {set}. A set's properties are defined on the set and pointed at by each \
                 variant, which has not been tried in Figma yet"
            ),
            Self::PropertyTaken { id, name } => write!(f, "{id} already has a property named `{name}`"),
            Self::SetNotReadBack { set, error } => write!(f, "made set {set} but could not read it back: {error}"),
        }
    }
}
