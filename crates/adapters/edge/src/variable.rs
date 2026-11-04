//! Variable collections and variables: the tools' edge halves.

use model::author::VarType;
use model::VariableType;
use schema::{VariableResolvedDataType, WireName};
use serde_json::{json, Value};

use super::args::{optional_string, string};
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::error::VariableError;
use crate::tools::{style, variable};
use crate::{DomainError, ToolError};

/// The variable types grafta writes.
const VARIABLE_TYPES: [VariableResolvedDataType; 4] =
    [VariableResolvedDataType::Color, VariableResolvedDataType::Float, VariableResolvedDataType::String, VariableResolvedDataType::Boolean];

fn var_type(value: &Value) -> Result<(VarType, VariableType), ToolError> {
    match value.as_str().and_then(|t| VariableResolvedDataType::listed(t, &VARIABLE_TYPES)) {
        Some(VariableResolvedDataType::Color) => Ok((VarType::Color, VariableType::Color)),
        Some(VariableResolvedDataType::Float) => Ok((VarType::Float, VariableType::Float)),
        Some(VariableResolvedDataType::String) => Ok((VarType::String, VariableType::String)),
        Some(VariableResolvedDataType::Boolean) => Ok((VarType::Boolean, VariableType::Boolean)),
        _ => Err(style::invalid("type", "COLOR, FLOAT, STRING or BOOLEAN", value)),
    }
}

fn non_empty(args: &Value, name: &str) -> Result<Option<String>, ToolError> {
    match optional_string(args, name)? {
        Some(s) if s.trim().is_empty() => Err(DomainError::Empty { at: name.into() }.into()),
        other => Ok(other.map(str::to_string)),
    }
}

fn values_schema() -> Value {
    json!({
        "type": "object",
        "description": "Mode name → value: \"#RRGGBB\" or \"#RRGGBBAA\" for COLOR, a number for FLOAT, a string \
                        for STRING, true or false for BOOLEAN, or { \"alias\": \"<variable name or id>\" } for \
                        another local variable of the same type.",
    })
}

pub struct CreateVariableCollection;

impl Tool for CreateVariableCollection {
    fn name(&self) -> &'static str {
        "create_variable_collection"
    }
    fn description(&self) -> &'static str {
        "Make a variable collection (a set of design tokens) with named modes, e.g. modes: \
         [\"Light\", \"Dark\"]. Then create_variable adds variables to it, and set_properties \
         binds a fill to one by name. In a file with no Internal Only Canvas (one from \
         new_file), grafta writes one first, where Figma keeps variables."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "name": { "type": "string", "description": "The collection's name, e.g. \"Colors\"." },
                "modes": { "type": "array", "items": { "type": "string" }, "minItems": 1, "description": "Mode names, in order." },
            }),
            &["handle", "name", "modes"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let name = non_empty(args, "name")?.ok_or_else(|| DomainError::Required { parameter: "name".into() })?;
        let Some(Value::Array(items)) = args.get("modes") else {
            return Err(VariableError::NoModeList.into());
        };
        let mut modes: Vec<String> = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let mode = item.as_str().filter(|m| !m.trim().is_empty()).ok_or_else(|| DomainError::Unexpected {
                at: format!("modes[{i}]"),
                expected: "a non-empty name".into(),
            })?;
            if modes.iter().any(|m| m == mode) {
                return Err(VariableError::ModeTwice { index: i, mode: mode.into() }.into());
            }
            modes.push(mode.to_string());
        }
        if modes.is_empty() {
            return Err(VariableError::NoModes.into());
        }
        Ok(Request::Command(Box::new(variable::CreateVariableCollection { handle, name, modes })))
    }
}

pub struct CreateVariable;

impl Tool for CreateVariable {
    fn name(&self) -> &'static str {
        "create_variable"
    }
    fn description(&self) -> &'static str {
        "Add a variable to a collection, with a value for every mode: { \"Light\": \"#7C3AED\", \
         \"Dark\": \"#A78BFA\" }. type is COLOR, FLOAT, STRING or BOOLEAN. A value may be { alias: \
         <variable name or id> }, another local variable of the same type, as semantic tokens \
         alias primitive ones. A COLOR variable can be bound at once with set_properties, fills: \
         [{ type: SOLID, variable: <name> }]."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "collection": { "type": "string", "description": "A collection's name or id." },
                "name": { "type": "string", "description": "e.g. \"Brand/Primary\"; slashes group it in Figma's panel." },
                "type": { "enum": VARIABLE_TYPES.map(WireName::as_wire) },
                "values": values_schema(),
            }),
            &["handle", "collection", "name", "type", "values"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let collection = string(args, "collection")?.into();
        let name = non_empty(args, "name")?.ok_or_else(|| DomainError::Required { parameter: "name".into() })?;
        let types = var_type(args.get("type").unwrap_or(&Value::Null))?;
        let values = data::from_json(args.get("values").unwrap_or(&Value::Null));
        Ok(Request::Command(Box::new(variable::CreateVariable { handle, collection, name, types, values })))
    }
}

pub struct SetVariable;

impl Tool for SetVariable {
    fn name(&self) -> &'static str {
        "set_variable"
    }
    fn description(&self) -> &'static str {
        "Change a variable grafta made: rename it, or change its value in the modes named (the \
         others keep theirs). Values are as create_variable takes them, aliases included. Every \
         fill bound to it follows, and a text whose size it binds is estimated again \
         (texts_resized). The file's own variables are refused."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "A variable's id, or its name when only one has it." },
                "name": { "type": "string" },
                "values": values_schema(),
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let name = non_empty(args, "name")?;
        // `values` given as null is given, and refused as no object.
        let values = args.get("values").map(data::from_json);
        if name.is_none() && values.is_none() {
            return Err(DomainError::NothingToChange { pass: "name, values or both" }.into());
        }
        Ok(Request::Command(Box::new(variable::SetVariable { handle, id, name, values })))
    }
}

pub struct SetVariableCollection;

impl Tool for SetVariableCollection {
    fn name(&self) -> &'static str {
        "set_variable_collection"
    }
    fn description(&self) -> &'static str {
        "Change a collection grafta made: rename it, or add a mode (add_mode: \"High contrast\"). \
         In a new mode every variable takes its value in the first mode, to change with \
         set_variable. The file's own collections are refused."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "A collection's id or name." },
                "name": { "type": "string" },
                "add_mode": { "type": "string" },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let name = non_empty(args, "name")?;
        let add_mode = non_empty(args, "add_mode")?;
        if name.is_none() && add_mode.is_none() {
            return Err(DomainError::NothingToChange { pass: "name, add_mode or both" }.into());
        }
        Ok(Request::Command(Box::new(variable::SetVariableCollection { handle, id, name, add_mode })))
    }
}
