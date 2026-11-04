//! Components, variant sets and component properties: the tools' edge
//! halves.

use schema::WireName;
use serde_json::{json, Value};

use super::args::{optional_string, string};
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::error::ComponentError;
use crate::tools::{component, properties, style};
use crate::{DomainError, ToolError};

pub struct CreateComponent;

impl Tool for CreateComponent {
    fn name(&self) -> &'static str {
        "create_component"
    }
    fn description(&self) -> &'static str {
        "Turn a frame into a component, in place, keeping its children, so it can be reused: \
         then place copies with create_instance (component: its id) and change what each copy \
         shows with override_instance. Build the frame first (create_frame, create_text, \
         create_vector, set_properties), then make it a component. A frame inside a component \
         or an instance cannot be one."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "The frame to turn into a component." },
                "name": { "type": "string", "description": "A new name, e.g. \"Button/Primary\"." },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let name = optional_string(args, "name")?.map(str::to_owned);
        Ok(Request::Command(Box::new(component::CreateComponent { handle, id, name })))
    }
}

pub struct CreateVariantSet;

impl Tool for CreateVariantSet {
    fn name(&self) -> &'static str {
        "create_variant_set"
    }
    fn description(&self) -> &'static str {
        "Group components into a variant set, as Figma's own sets: one component with \
         properties, each component a variant of it. variants: [{ id, values: { \"State\": \
         \"Default\" } }], every variant naming the same properties with a different \
         combination. The components must share a parent; the set is placed around them. Then \
         create_instance with any variant's id, and set_variant to switch an instance."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "name": { "type": "string", "description": "The set's name, e.g. \"Product row\"." },
                "variants": {
                    "type": "array",
                    "minItems": 2,
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "A component." },
                            "values": { "type": "object", "description": "Property name → value, e.g. { \"State\": \"Substitute\" }." },
                        },
                        "required": ["id", "values"],
                    },
                },
            }),
            &["handle", "name", "variants"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let name = string(args, "name")?.into();
        let Some(Value::Array(items)) = args.get("variants") else {
            return Err(ComponentError::NoVariantList.into());
        };
        if items.len() < 2 {
            return Err(ComponentError::TooFewVariants.into());
        }
        let mut variants: Vec<(String, Vec<(String, String)>)> = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let id = item.get("id").and_then(Value::as_str).ok_or(ComponentError::NoVariantId { index: i })?;
            let Some(Value::Object(values)) = item.get("values") else {
                return Err(ComponentError::NoVariantValues { index: i }.into());
            };
            let mut pairs = Vec::new();
            for (property, value) in values {
                let value = value.as_str().filter(|v| !v.is_empty()).ok_or_else(|| DomainError::Unexpected {
                    at: format!("variants[{i}].values.{property}"),
                    expected: "a non-empty string".into(),
                })?;
                if [property.as_str(), value].iter().any(|s| s.contains(['=', ',']) || s.trim().is_empty()) {
                    return Err(ComponentError::NotAVariantName { index: i, property: property.clone(), value: value.into() }.into());
                }
                pairs.push((property.clone(), value.to_string()));
            }
            if pairs.is_empty() {
                return Err(ComponentError::NoVariantProperty { index: i }.into());
            }
            variants.push((id.to_string(), pairs));
        }
        Ok(Request::Command(Box::new(component::CreateVariantSet { handle, name, variants })))
    }
}

pub struct SetVariant;

impl Tool for SetVariant {
    fn name(&self) -> &'static str {
        "set_variant"
    }
    fn description(&self) -> &'static str {
        "Switch an instance of a variant to another variant of its set, by the values that \
         change: { \"State\": \"Substitute\" }. The other properties keep their values. Its \
         overrides stay; Figma matches them to the new variant's layers."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "An instance of a variant." },
                "values": { "type": "object", "description": "Property name → the value to switch to." },
            }),
            &["handle", "id", "values"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let Some(Value::Object(asked)) = args.get("values") else {
            return Err(ComponentError::NoValues.into());
        };
        let mut values = Vec::with_capacity(asked.len());
        for (property, value) in asked {
            let value = value.as_str().ok_or_else(|| DomainError::Unexpected {
                at: format!("values.{property}"),
                expected: "a string".into(),
            })?;
            values.push((property.clone(), value.to_owned()));
        }
        Ok(Request::Command(Box::new(component::SetVariant { handle, id, values })))
    }
}

pub struct AddComponentProperty;

impl Tool for AddComponentProperty {
    fn name(&self) -> &'static str {
        "add_component_property"
    }
    fn description(&self) -> &'static str {
        "Give a component a property its instances set: TEXT (a label), BOOLEAN \
         (show a part or not) or INSTANCE_SWAP (which icon), with its default. Then bind a \
         layer inside the component with set_properties componentPropertyReferences \
         ({ characters | visible | mainComponent: <property> }), and set each instance's value \
         with set_properties componentProperties ({ \"Label\": \"Guardar\" })."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "component": { "type": "string", "description": "A component's id." },
                "name": { "type": "string", "description": "e.g. \"Label\", \"Show icon\"." },
                "type": { "enum": properties::PROPERTY_TYPES.map(|(name, _)| name) },
                "default": { "description": "A string, true or false, or a component's name or id." },
            }),
            &["handle", "component", "name", "type", "default"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let component = string(args, "component")?.into();
        let name = string(args, "name")?.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Empty { at: "name".into() }.into());
        }
        let named = args.get("type").and_then(Value::as_str);
        let Some(kind) = named.and_then(|n| properties::PROPERTY_TYPES.iter().find(|(api, _)| *api == n)).map(|(_, kind)| kind.as_wire()) else {
            return Err(style::invalid("type", "TEXT, BOOLEAN or INSTANCE_SWAP", format!("{named:?}")));
        };
        let default = data::from_json(args.get("default").unwrap_or(&Value::Null));
        Ok(Request::Command(Box::new(component::AddComponentProperty { handle, component, name, kind, default })))
    }
}
