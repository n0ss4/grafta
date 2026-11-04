//! Reading a call's JSON arguments, and the words for one that is wrong.

use serde_json::Value;

use crate::bound::{Budget, Paging, DEFAULT_BUDGET, MAX_BUDGET, MIN_BUDGET};
use crate::{DomainError, ToolError};

pub(crate) fn string<'a>(args: &'a Value, name: &str) -> Result<&'a str, ToolError> {
    match args.get(name) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s),
        Some(other) => Err(expected(name, "a non-empty string", other)),
        None => Err(required(name)),
    }
}

pub(crate) fn optional_string<'a>(args: &'a Value, name: &str) -> Result<Option<&'a str>, ToolError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => string(args, name).map(Some),
    }
}

/// A list of non-empty strings, or none.
pub(crate) fn optional_strings(args: &Value, name: &str) -> Result<Vec<String>, ToolError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(s) if !s.is_empty() => Ok(s.clone()),
                other => Err(expected(name, "a list of non-empty strings", other)),
            })
            .collect(),
        Some(other) => Err(expected(name, "a list of non-empty strings", other)),
    }
}

pub(crate) fn number(args: &Value, name: &str) -> Result<f32, ToolError> {
    match args.get(name) {
        Some(value) => value
            .as_f64()
            .filter(|n| n.is_finite())
            .map(|n| n as f32)
            .ok_or_else(|| expected(name, "a number", value)),
        None => Err(required(name)),
    }
}

pub(crate) fn optional_number(args: &Value, name: &str, default: f32) -> Result<f32, ToolError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(_) => number(args, name),
    }
}

pub(crate) fn optional_integer(
    args: &Value,
    name: &str,
    default: usize,
    range: std::ops::RangeInclusive<usize>,
) -> Result<usize, ToolError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => value
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| range.contains(n))
            .ok_or_else(|| expected(name, &format!("an integer from {} to {}", range.start(), range.end()), value)),
    }
}

pub(crate) fn optional_bool(args: &Value, name: &str) -> Result<bool, ToolError> {
    match args.get(name) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        Some(other) => Err(expected(name, "true or false", other)),
    }
}

pub(crate) fn required(name: &str) -> ToolError {
    DomainError::Required { parameter: name.into() }.into()
}

/// A parameter given, but not as `what`.
pub(crate) fn expected(name: &str, what: &str, got: impl std::fmt::Display) -> ToolError {
    DomainError::Expected { parameter: name.into(), expected: what.into(), got: got.to_string() }.into()
}

/// `budget`: bytes the result may take, the default when not given.
pub(crate) fn budget(args: &Value) -> Result<Budget, ToolError> {
    match args.get("budget") {
        None | Some(Value::Null) => Ok(Budget(DEFAULT_BUDGET)),
        Some(value) => {
            let bytes = value.as_u64().ok_or_else(|| DomainError::Expected {
                parameter: "budget".into(),
                expected: "a number of bytes".into(),
                got: value.to_string(),
            })?;
            let bytes = usize::try_from(bytes).unwrap_or(usize::MAX);
            if !(MIN_BUDGET..=MAX_BUDGET).contains(&bytes) {
                return Err(DomainError::Expected {
                    parameter: "budget".into(),
                    expected: format!("{MIN_BUDGET} to {MAX_BUDGET} bytes"),
                    got: bytes.to_string(),
                }
                .into());
            }
            Ok(Budget(bytes))
        }
    }
}

/// `limit`, `cursor` and `budget`, for a paged list. A cursor that is not a
/// string is no cursor; whether it is one, and still current, the list says.
pub(crate) fn paging(args: &Value) -> Result<Paging, ToolError> {
    let limit = match args.get("limit") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let limit = value.as_u64().filter(|n| (1..=1000).contains(n)).ok_or_else(|| DomainError::Expected {
                parameter: "limit".into(),
                expected: "a number from 1 to 1000".into(),
                got: value.to_string(),
            })?;
            Some(usize::try_from(limit).unwrap_or(1000))
        }
    };
    let cursor = args.get("cursor").and_then(Value::as_str).map(str::to_owned);
    Ok(Paging { limit, cursor, budget: budget(args)? })
}

#[cfg(test)]
mod tests;
