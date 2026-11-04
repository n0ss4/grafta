//! Slides' failures: transitions and object animations.

use std::fmt;

use schema::Guid;

use super::Kind;

#[derive(Debug)]
pub enum SlideError {
    /// A transition with no style.
    NoStyle,
    /// An object animation with no layer.
    NoLayer { key: String },
    /// An object animation of a layer that is not on the slide.
    NotOnSlide { key: String, node: String, slide: Guid },
    /// A phase other than IN or OUT.
    NotInOrOut { key: String, phase: String },
}

impl SlideError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NoStyle | Self::NoLayer { .. } | Self::NotOnSlide { .. } | Self::NotInOrOut { .. } => {
                Kind::BadArgument
            }
        }
    }
}

impl fmt::Display for SlideError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStyle => f.write_str("transition: style is required"),
            Self::NoLayer { key } => write!(f, "{key}.node: the id of a layer on the slide is required"),
            Self::NotOnSlide { key, node, slide } => write!(f, "{key}.node: {node} is not a layer on slide {slide}"),
            Self::NotInOrOut { key, phase } => write!(f, "{key}.phase: `{phase}` is not IN or OUT"),
        }
    }
}
