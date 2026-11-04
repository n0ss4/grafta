//! Image paints' failures: the files they name and the images a file embeds.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum ImageError {
    /// An image path that cannot be read.
    CannotRead { at: String, path: String, error: String },
    /// A file that is neither a PNG nor a JPEG.
    NotAnImage { at: String, path: String },
    /// An IMAGE paint with neither image nor imageHash.
    NoImage { at: String },
    /// A hash of an image the file does not embed.
    NotEmbedded { at: String, hash: String },
    /// An embedded image grafta cannot size.
    Unsizable { at: String, hash: String },
}

impl ImageError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::CannotRead { .. } | Self::NotAnImage { .. } | Self::NoImage { .. } => Kind::BadArgument,
            Self::NotEmbedded { .. } => Kind::NotFound,
            Self::Unsizable { .. } => Kind::Refused,
        }
    }
}

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CannotRead { at, path, error } => write!(f, "{at}: cannot read {path}: {error}"),
            Self::NotAnImage { at, path } => {
                write!(f, "{at}: {path} is not a PNG or JPEG, the two formats the fixtures embed")
            }
            Self::NoImage { at } => write!(f, "{at}: an IMAGE paint needs image (a PNG or JPEG path) or imageHash"),
            Self::NotEmbedded { at, hash } => {
                write!(f, "{at}.imageHash: this file embeds no image {hash}; name a file with image instead")
            }
            Self::Unsizable { at, hash } => write!(f, "{at}: image {hash} is not a PNG or JPEG grafta can size"),
        }
    }
}
