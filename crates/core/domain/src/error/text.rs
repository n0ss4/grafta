//! The text tools' failures: how the text is given.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum TextError {
    /// Both characters and runs.
    TwoWays,
    /// Neither characters nor runs.
    NoText,
}

impl TextError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::TwoWays | Self::NoText => Kind::BadArgument,
        }
    }
}

impl fmt::Display for TextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TwoWays => f.write_str("characters and runs: give the text one way, not both"),
            Self::NoText => f.write_str("characters: give the text, or runs"),
        }
    }
}
