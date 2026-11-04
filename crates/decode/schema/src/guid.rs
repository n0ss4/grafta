//! How the format addresses a node: a session and a counter within it.

use std::fmt;
use std::str::FromStr;

use kiwi::Value;

use crate::{object, Field, Wire};

const SEPARATOR: char = ':';

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Guid {
    pub session_id: u32,
    pub local_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{0}` is not a node id; ids look like 12:345")]
pub struct GuidError(pub String);

impl Guid {
    #[must_use]
    pub const fn new(session_id: u32, local_id: u32) -> Self {
        Self { session_id, local_id }
    }

    /// A bare `{ sessionID, localID }`.
    #[must_use]
    pub fn read(value: &Value) -> Option<Self> {
        Some(Self::new(value.u32(Field::SessionId)?, value.u32(Field::LocalId)?))
    }

    /// One in a `{ guid }` envelope, or a bare one.
    #[must_use]
    pub fn read_wrapped(value: &Value) -> Option<Self> {
        value.field(Field::Guid).and_then(Self::read).or_else(|| Self::read(value))
    }

    #[must_use]
    pub fn to_value(self) -> Value {
        object([(Field::SessionId, Value::Uint(self.session_id)), (Field::LocalId, Value::Uint(self.local_id))])
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{SEPARATOR}{}", self.session_id, self.local_id)
    }
}

impl FromStr for Guid {
    type Err = GuidError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        text.split_once(SEPARATOR)
            .and_then(|(session, local)| Some(Self::new(session.parse().ok()?, local.parse().ok()?)))
            .ok_or_else(|| GuidError(text.to_string()))
    }
}

/// As its text, `12:345`, the way every id leaves grafta.
impl serde::Serialize for Guid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Guid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
