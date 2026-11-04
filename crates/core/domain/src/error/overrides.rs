//! override_instance's failures: which layers of an instance's component it
//! changes, and the overrides it writes.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum OverrideError {
    /// An instance whose component, and so its layers, is not in the file.
    LayersUnknown { id: Guid },
    /// No properties, or not an object of them.
    NoProperties,
    /// A property an override does not take.
    NotOverridable { key: String },
    /// characters or visible, with no layer named.
    LayerNeeded,
    /// A component with no vector layer to recolour.
    NoVectorLayers { component: Guid },
    /// A path of no ids.
    EmptyPath,
    /// characters, on a layer that is not text.
    NotText { layer: String },
    /// No vector layer that has the paint asked to recolour.
    NothingToRecolour { component: Guid, asked: &'static str, other: &'static str },
    /// An id that is not a layer of the component.
    NotALayer { id: String, component: String },
    /// A path continued through a layer that is not an instance.
    NotNested { id: String, is: &'static str },
    /// An instance's symbolData that is not an object.
    SymbolDataNotAnObject { id: Guid },
    /// An instance's symbolOverrides that is not a list.
    OverridesNotAList { id: Guid },
}

impl OverrideError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NoProperties
            | Self::NotOverridable { .. }
            | Self::LayerNeeded
            | Self::EmptyPath
            | Self::NotNested { .. } => Kind::BadArgument,
            Self::NotALayer { .. } => Kind::NotFound,
            Self::LayersUnknown { .. }
            | Self::NoVectorLayers { .. }
            | Self::NotText { .. }
            | Self::NothingToRecolour { .. } => Kind::Refused,
            Self::SymbolDataNotAnObject { .. } | Self::OverridesNotAList { .. } => Kind::Failed,
        }
    }
}

impl fmt::Display for OverrideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayersUnknown { id } => write!(f, "{id}'s component is not in this file, so its layers are unknown"),
            Self::NoProperties => f.write_str(
                "parameter \"properties\": expected an object with any of fills, strokes, characters, visible",
            ),
            Self::NotOverridable { key } => write!(
                f,
                "`{key}` cannot be overridden. An instance override takes fills, strokes, characters and visible"
            ),
            Self::LayerNeeded => f.write_str("characters and visible need a layer: name the component layer to change"),
            Self::NoVectorLayers { component } => {
                write!(f, "{component} has no vector layers to recolour; name a layer instead")
            }
            Self::EmptyPath => f.write_str("layer: an empty path names nothing"),
            Self::NotText { layer } => write!(f, "{layer} is not a text layer, so it has no characters"),
            Self::NothingToRecolour { component, asked, other } => write!(
                f,
                "no vector layer of {component} has {asked} to recolour; it may be drawn with \
                 {other}. read_node on the component shows which"
            ),
            Self::NotALayer { id, component } => {
                write!(f, "{id} is not a layer of component {component}. read_node on the component lists its layers")
            }
            Self::NotNested { id, is } => write!(f, "{id} is a {is}; only a nested instance can continue a path"),
            Self::SymbolDataNotAnObject { id } => write!(f, "{id}'s symbolData is not an object"),
            Self::OverridesNotAList { id } => write!(f, "{id}'s symbolOverrides is not a list"),
        }
    }
}
