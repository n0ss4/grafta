//! A file of calls, run in order against one toolkit, so a handle the first
//! call opens serves the rest: how a ladder or a golden corpus is made through
//! the path a model takes.

use std::fmt;

use serde_json::Value;

const REFERENCE_PREFIX: char = '$';
const PATH_SEPARATOR: char = '.';

#[derive(Debug, PartialEq, Eq)]
pub enum ReferenceError {
    NotAnIndex { text: String },
    NotRun { text: String, index: usize },
    Missing { text: String, index: usize, part: String },
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAnIndex { text } => write!(f, "`{text}` does not name a call by its number"),
            Self::NotRun { text, index } => write!(f, "`{text}` refers to call {index}, which has not run"),
            Self::Missing { text, index, part } => write!(f, "`{text}`: call {index}'s result has no `{part}`"),
        }
    }
}

impl std::error::Error for ReferenceError {}

/// Replace every string `"$N.path"` with that field of result N. A path steps
/// through objects by key and arrays by index: `$0.pages.0.id`. A string that
/// does not start with `$` and a digit is left alone.
pub fn substitute(args: &Value, results: &[Value]) -> Result<Value, ReferenceError> {
    Ok(match args {
        Value::String(text) => match text.strip_prefix(REFERENCE_PREFIX).filter(|t| t.starts_with(|c: char| c.is_ascii_digit())) {
            Some(reference) => resolve(text, reference, results)?.clone(),
            None => args.clone(),
        },
        Value::Array(items) => Value::Array(items.iter().map(|item| substitute(item, results)).collect::<Result<_, _>>()?),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), substitute(value, results)?)))
                .collect::<Result<_, ReferenceError>>()?,
        ),
        other => other.clone(),
    })
}

fn resolve<'a>(text: &str, reference: &str, results: &'a [Value]) -> Result<&'a Value, ReferenceError> {
    let mut parts = reference.split(PATH_SEPARATOR);
    let index: usize = parts
        .next()
        .unwrap_or_default()
        .parse()
        .map_err(|_| ReferenceError::NotAnIndex { text: text.to_string() })?;
    let mut value = results.get(index).ok_or_else(|| ReferenceError::NotRun { text: text.to_string(), index })?;
    for part in parts {
        value = match part.parse::<usize>() {
            Ok(i) => value.get(i),
            Err(_) => value.get(part),
        }
        .ok_or_else(|| ReferenceError::Missing { text: text.to_string(), index, part: part.to_string() })?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
