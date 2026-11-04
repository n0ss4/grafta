//! Colour, text and effect styles: the tools' edge halves.

use model::author::StyleKind;
use model::SharedStyleType;
use schema::{StyleType, WireName};
use serde_json::{json, Map, Value};

use super::schema::{handle_property, schema};
use super::args::{optional_string, string};
use super::data;
use super::{Request, Tool};
use crate::error::SharedStyleError;
use crate::tools::shared_style;
use crate::tools::style::invalid;
use crate::app::data::Data;
use crate::{DomainError, ToolError};

/// The kinds of style grafta writes.
pub(crate) const STYLE_TYPES: [StyleType; 3] = [StyleType::Fill, StyleType::Text, StyleType::Effect];

pub(crate) fn kind_of(value: &Value) -> Result<(StyleKind, SharedStyleType), ToolError> {
    match value.as_str().and_then(StyleType::from_wire) {
        Some(StyleType::Fill) => Ok((StyleKind::Fill, SharedStyleType::Fill)),
        Some(StyleType::Text) => Ok((StyleKind::Text, SharedStyleType::Text)),
        Some(StyleType::Effect) => Ok((StyleKind::Effect, SharedStyleType::Effect)),
        Some(StyleType::Grid) => Err(SharedStyleError::GridMade.into()),
        _ => Err(invalid("type", "FILL, TEXT or EFFECT", value)),
    }
}

pub struct ListStyles;

impl Tool for ListStyles {
    fn name(&self) -> &'static str {
        "list_styles"
    }
    fn description(&self) -> &'static str {
        "The file's own colour, text and effect styles, with their values: paints in hex, a \
         text style's font, size, line height and letter spacing, effects. Styles removed from \
         the file and a library's are left out. Apply one with set_properties: fillStyle, \
         strokeStyle, effectStyle or textStyle."
    }
    fn input_schema(&self) -> Value {
        schema(json!({ "handle": handle_property(), "type": { "enum": STYLE_TYPES.map(WireName::as_wire) } }), &["handle"])
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let kind = match args.get("type") {
            Some(v) => Some(kind_of(v)?.1),
            None => None,
        };
        Ok(Request::Query(Box::new(shared_style::ListStyles { handle, kind })))
    }
}

/// A style's values, the keys the call gave, as it gave them.
fn values(args: &Value) -> Data {
    Data::Map(shared_style::VALUE_KEYS.iter().filter_map(|&key| args.get(key).map(|value| (key.to_owned(), data::from_json(value)))).collect())
}

fn value_schema() -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("fills".into(), json!({ "type": "array", "items": { "type": "object" }, "description": "FILL: paints, as set_properties takes them." }));
    m.insert("effects".into(), json!({ "type": "array", "items": { "type": "object" }, "description": "EFFECT: shadows and blurs, as set_properties takes them." }));
    m.insert("fontFamily".into(), json!({ "type": "string" }));
    m.insert("fontStyle".into(), json!({ "type": "string", "description": "e.g. \"Bold\", \"Semi Bold\"." }));
    m.insert("fontSize".into(), json!({ "type": "number", "exclusiveMinimum": 0 }));
    m.insert("lineHeight".into(), json!({ "description": "Pixels, or { value, unit: PIXELS | PERCENT }." }));
    m.insert("letterSpacing".into(), json!({ "description": "Pixels, or { value, unit: PIXELS | PERCENT }." }));
    m
}

pub struct CreateStyle;

impl Tool for CreateStyle {
    fn name(&self) -> &'static str {
        "create_style"
    }
    fn description(&self) -> &'static str {
        "Make a style: type FILL with fills (paints, as set_properties takes them), TEXT with \
         fontFamily, fontStyle, fontSize, lineHeight and letterSpacing, or EFFECT with effects. \
         Figma lists it under Local styles. Apply it with set_properties: fillStyle or \
         strokeStyle for a FILL style, effectStyle, textStyle."
    }
    fn input_schema(&self) -> Value {
        let mut properties = value_schema();
        properties.insert("handle".into(), handle_property());
        properties.insert("name".into(), json!({ "type": "string", "description": "e.g. \"Brand/Primary\"; slashes group it." }));
        properties.insert("type".into(), json!({ "enum": STYLE_TYPES.map(WireName::as_wire) }));
        schema(Value::Object(properties), &["handle", "name", "type"])
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let name = string(args, "name")?.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Empty { at: "name".into() }.into());
        }
        let (kind, style_type) = kind_of(args.get("type").unwrap_or(&Value::Null))?;
        let values = values(args);
        Ok(Request::Command(Box::new(shared_style::CreateStyle { handle, name, kind, style_type, values })))
    }
}

pub struct SetStyle;

impl Tool for SetStyle {
    fn name(&self) -> &'static str {
        "set_style"
    }
    fn description(&self) -> &'static str {
        "Change a colour, text or effect style, grafta's or one the file already has (a kit's \
         brand colour, changed once): rename it, or change its values (as create_style takes \
         them for its type). Every node using it follows, in Figma and in read_node, instances \
         are derived again, and a text style's texts are estimated again (texts_resized). \
         Library styles are refused."
    }
    fn input_schema(&self) -> Value {
        let mut properties = value_schema();
        properties.insert("handle".into(), handle_property());
        properties.insert("id".into(), json!({ "type": "string", "description": "A style's id, or its name when only one has it." }));
        properties.insert("name".into(), json!({ "type": "string" }));
        schema(Value::Object(properties), &["handle", "id"])
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let name = optional_string(args, "name")?.map(str::trim).map(str::to_string);
        if name.as_deref() == Some("") {
            return Err(DomainError::Empty { at: "name".into() }.into());
        }
        let values = values(args);
        Ok(Request::Command(Box::new(shared_style::SetStyle { handle, id, name, values })))
    }
}
