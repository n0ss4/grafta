//! Overriding an instance's layers: the tool's edge half.

use serde_json::{json, Value};

use super::args::string;
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::app::data::Data;
use crate::error::OverrideError;
use crate::tools::overrides;
use crate::tools::style::invalid;
use crate::ToolError;

pub struct OverrideInstance;

impl Tool for OverrideInstance {
    fn name(&self) -> &'static str {
        "override_instance"
    }
    fn description(&self) -> &'static str {
        "Change what an instance shows, without changing its component: the \
         colour of an icon, a button's label, a hidden part. properties: fills and strokes \
         (paints, as in set_properties), characters (text layers), visible. layer names the \
         component's layer by its id, as read_node on the component shows it; for a layer \
         inside a nested instance, give a path of ids [nested instance, layer]. Without layer, \
         fills and strokes recolour every vector layer of the component that already has \
         fills (or strokes), and never add paint to a layer that had none. Many icons are drawn with strokes, not fills: read_node on the component shows \
         which, and a stroked icon is recoloured with strokes."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "An instance you created." },
                "layer": {
                    "oneOf": [
                        { "type": "string" },
                        { "type": "array", "items": { "type": "string" } },
                    ],
                    "description": "A layer id in the component, or a path of ids through nested instances.",
                },
                "properties": {
                    "type": "object",
                    "properties": {
                        "fills": { "type": "array", "description": "Paints, as set_properties takes them, an IMAGE by path included." },
                        "strokes": { "type": "array" },
                        "characters": { "type": "string" },
                        "visible": { "type": "boolean" },
                    },
                    "additionalProperties": false,
                },
            }),
            &["handle", "id", "properties"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let properties = match data::from_json(args.get("properties").unwrap_or(&Value::Null)) {
            Data::Map(map) if !map.is_empty() => map,
            _ => return Err(OverrideError::NoProperties.into()),
        };
        if let Some(unknown) =
            properties.keys().find(|k| !matches!(k.as_str(), "fills" | "strokes" | "characters" | "visible"))
        {
            return Err(OverrideError::NotOverridable { key: unknown.clone() }.into());
        }
        let layer = match args.get("layer") {
            None | Some(Value::Null) => None,
            Some(Value::String(layer)) => Some(vec![layer.clone()]),
            Some(Value::Array(path)) => {
                let path: Vec<String> = path.iter().filter_map(Value::as_str).map(str::to_string).collect();
                if path.is_empty() {
                    return Err(OverrideError::EmptyPath.into());
                }
                Some(path)
            }
            Some(other) => return Err(invalid("layer", "an id or a path of ids", other)),
        };
        Ok(Request::Command(Box::new(overrides::OverrideInstance { handle, id, layer, properties })))
    }
}
