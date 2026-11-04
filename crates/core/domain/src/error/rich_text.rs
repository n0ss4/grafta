//! Rich text's failures: runs, links and paragraphs.

use std::fmt;

use super::Kind;

#[derive(Debug)]
pub enum RichTextError {
    /// A key a run does not take; `keys` are those it does, besides characters.
    NotARunKey { at: String, key: String, keys: &'static [&'static str] },
    /// A link that is not an http, https or mailto address.
    NotAWebAddress { at: String, url: String },
    /// Paragraphs given for other than every line of the text.
    ParagraphCount { count: usize, given: usize },
    /// A key a paragraph does not take.
    NotAParagraphKey { at: String, key: String },
    /// An indentation on a paragraph that is not a list item.
    IndentedPlain { at: String, value: String },
}

impl RichTextError {
    pub(super) fn kind(&self) -> Kind {
        match self {
            Self::NotARunKey { .. } | Self::ParagraphCount { .. } | Self::NotAParagraphKey { .. } => Kind::BadArgument,
            Self::NotAWebAddress { .. } | Self::IndentedPlain { .. } => Kind::Refused,
        }
    }
}

impl fmt::Display for RichTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotARunKey { at, key, keys } => {
                write!(f, "{at}.{key}: not a run key. A run takes characters and any of: {}", keys.join(", "))
            }
            Self::NotAWebAddress { at, url } => write!(
                f,
                "{at}.hyperlink: {url} is not a web address; a link is an http, https or mailto address"
            ),
            Self::ParagraphCount { count, given } => write!(
                f,
                "paragraphs: the text has {count} paragraph{} (one per line break, plus one), and {given} were given",
                if *count == 1 { "" } else { "s" }
            ),
            Self::NotAParagraphKey { at, key } => write!(f, "{at}.{key}: a paragraph takes listType and indentation"),
            Self::IndentedPlain { at, value } => write!(
                f,
                "{at}.indentation: {value} on a paragraph that is not a list; only list items are indented by level"
            ),
        }
    }
}
