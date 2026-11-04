//! create_vector's failures: its SVG path data and the box it is drawn in.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum VectorError {
    /// A list of path data with something other than strings in it.
    NotStrings,
    /// No path data.
    NoPath,
    /// Path data that does not parse; `which` names the parameter.
    BadPath { which: String, error: String },
    /// A box of no area.
    NoArea { width: f32, height: f32 },
    /// A stroke weight of 0 or less.
    ZeroStrokeWeight,
}

impl VectorError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NotStrings | Self::NoPath | Self::BadPath { .. } | Self::NoArea { .. } | Self::ZeroStrokeWeight => {
                Kind::BadArgument
            }
        }
    }
}

impl fmt::Display for VectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStrings => f.write_str("parameter \"path\": every item must be a string"),
            Self::NoPath => f.write_str("parameter \"path\": SVG path data, or a list of it, is required"),
            Self::BadPath { which, error } => write!(f, "parameter \"{which}\": {error}"),
            Self::NoArea { width, height } => write!(
                f,
                "the box the path is drawn in is {width}×{height}, with no area. A straight line needs a viewBox"
            ),
            Self::ZeroStrokeWeight => f.write_str("parameter \"strokeWeight\": must be greater than 0"),
        }
    }
}
