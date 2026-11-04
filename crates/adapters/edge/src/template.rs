//! Placing a component once per row: the tool's edge half.

use serde_json::{json, Value};

use super::args::{optional_string, string};
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::app::data::Data;
use crate::error::TemplateError;
use crate::tools::style::invalid;
use crate::tools::template;
use crate::{DomainError, ToolError};

/// The rows given, or the CSV file that holds them.
fn rows(args: &Value) -> Result<template::Rows, ToolError> {
    Ok(match (args.get("rows"), optional_string(args, "csv")?) {
        (Some(_), Some(_)) => return Err(TemplateError::RowsAndCsv.into()),
        (Some(Value::Array(rows)), None) => template::Rows::Given(
            rows.iter()
                .enumerate()
                .map(|(i, row)| match data::from_json(row) {
                    Data::Map(row) => Ok(row),
                    _ => Err(invalid(&format!("rows[{i}]"), "an object", row)),
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
        (Some(other), None) => {
            return Err(DomainError::Expected {
                parameter: "rows".into(),
                expected: "an array of objects".into(),
                got: other.to_string(),
            }
            .into())
        }
        (None, Some(path)) => template::Rows::Csv(path.into()),
        (None, None) => return Err(TemplateError::NoRows.into()),
    })
}

pub struct FillTemplate;

impl Tool for FillTemplate {
    fn name(&self) -> &'static str {
        "fill_template"
    }
    fn description(&self) -> &'static str {
        "Place a component once per row of data, in a page or frame (an auto-layout frame \
         flows them): product cards, list rows, localised screens. Rows are a JSON array of \
         objects, or a CSV file whose first line names the keys. Each key of a row is, in this \
         order: `name` (the instance's layer name); a variant property, when `component` is a \
         variant set; a component property (TEXT, BOOLEAN, INSTANCE_SWAP); or the name of a \
         layer in the component, whose text it sets, or whose image when the value is a .png or \
         .jpg path. Any other key refuses the call before anything is written. One call, up \
         to 1000 rows."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "component": { "type": "string", "description": "A component, or a variant set." },
                "parent": { "type": "string", "description": "A page or frame id." },
                "rows": { "type": "array", "items": { "type": "object" }, "description": "One object per instance." },
                "csv": { "type": "string", "description": "A CSV file instead of rows: its first line names the keys." },
            }),
            &["handle", "component", "parent"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        let rows = rows(args)?;
        let component = string(args, "component")?.into();
        Ok(Request::Command(Box::new(template::FillTemplate { handle, component, parent, rows })))
    }
}
