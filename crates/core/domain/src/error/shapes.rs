//! The shapes' failures: what each kind of shape has, and its size.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum ShapeError {
    /// An arc on a shape other than an ellipse.
    NoArc,
    /// A point count on a shape other than a polygon or a star.
    NoPoints,
    /// An inner radius on a shape other than a star.
    NoInnerRadius,
    /// A height given for a line.
    LineHeight,
    /// Fills on a line.
    LineFills,
    /// A line with no strokes.
    LineStrokes,
    /// A corner radius on an ellipse or a line.
    NoCorners { key: &'static str },
    /// A corner radius below 0.
    NegativeRadius { got: f32 },
}

impl ShapeError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::LineHeight | Self::LineStrokes | Self::NegativeRadius { .. } => Kind::BadArgument,
            Self::NoArc | Self::NoPoints | Self::NoInnerRadius | Self::LineFills | Self::NoCorners { .. } => {
                Kind::Refused
            }
        }
    }
}

impl fmt::Display for ShapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoArc => f.write_str("arcData: only an ellipse has an arc"),
            Self::NoPoints => f.write_str("pointCount: only a polygon or a star has points"),
            Self::NoInnerRadius => {
                f.write_str("innerRadius: only a star has an inner radius; an ellipse's is arcData.innerRadius")
            }
            Self::LineHeight => f.write_str("height: a line has none; width is its length"),
            Self::LineFills => f.write_str("fills: a line has no area; it is drawn by its strokes"),
            Self::LineStrokes => f.write_str("strokes: a line is drawn by its strokes; name at least one"),
            Self::NoCorners { key } => write!(f, "{key}: an ellipse or a line has no corners"),
            Self::NegativeRadius { got } => write!(f, "cornerRadius must be 0 or more, got {got}"),
        }
    }
}
