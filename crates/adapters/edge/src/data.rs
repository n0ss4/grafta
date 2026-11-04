//! [`Data`] from a call's JSON, and back.

use serde_json::{Map, Number as JsonNumber, Value};

use crate::app::data::{Data, Number};

/// A JSON value as `Data`: an integer stays an integer, a float a float.
#[must_use]
pub fn from_json(value: &Value) -> Data {
    match value {
        Value::Null => Data::Null,
        Value::Bool(b) => Data::Bool(*b),
        Value::Number(n) => Data::Number(match (n.as_u64(), n.as_i64(), n.as_f64()) {
            (Some(u), _, _) => Number::Unsigned(u),
            (None, Some(i), _) => Number::Signed(i),
            (None, None, Some(f)) => Number::Float(f),
            (None, None, None) => unreachable!("a JSON number is one of the three"),
        }),
        Value::String(s) => Data::String(s.clone()),
        Value::Array(list) => Data::List(list.iter().map(from_json).collect()),
        Value::Object(map) => Data::Map(map.iter().map(|(k, v)| (k.clone(), from_json(v))).collect()),
    }
}

/// `Data` as JSON.
#[must_use]
pub fn to_json(data: &Data) -> Value {
    match data {
        Data::Null => Value::Null,
        Data::Bool(b) => Value::Bool(*b),
        Data::Number(Number::Unsigned(n)) => Value::Number((*n).into()),
        Data::Number(Number::Signed(n)) => Value::Number((*n).into()),
        Data::Number(Number::Float(n)) => JsonNumber::from_f64(*n).map_or(Value::Null, Value::Number),
        Data::String(s) => Value::String(s.clone()),
        Data::List(list) => Value::Array(list.iter().map(to_json).collect()),
        Data::Map(map) => Value::Object(map.iter().map(|(k, v)| (k.clone(), to_json(v))).collect::<Map<_, _>>()),
    }
}

#[cfg(test)]
mod tests;
