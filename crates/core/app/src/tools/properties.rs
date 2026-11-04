//! Component properties: a text, boolean or instance-swap
//! property on a component, a layer inside it bound to one, and an instance's
//! value for one.
//!
//! A definition is an entry of the component's `componentPropDefs`; a layer
//! uses it through `componentPropRefs` and a `PROP_REF` in its
//! `parameterConsumptionMap`; an instance sets it through
//! `componentPropAssignments` (42,173 definitions, 203,522 references and
//! 49,349 assignments measured). The author half is `model::author`.

use kiwi::Value as Raw;
use model::author::{Guid, PropValue};
use model::NodeType;
use crate::data::Data;
use schema::{ComponentPropNodeField, ComponentPropType, Field, Wire, WireName};

use super::read::node_at;
use super::style::invalid;
use crate::error::PropertyError;
use crate::{Session, ToolError};

/// A property as its component defines it.
#[derive(Clone)]
pub struct Definition {
    pub id: Guid,
    pub name: String,
    /// `TEXT`, `BOOL`, `INSTANCE_SWAP`, or another the file has.
    pub kind: String,
    pub default: Option<PropValue>,
}

impl Definition {
    pub fn is(&self, kind: ComponentPropType) -> bool {
        ComponentPropType::from_wire(&self.kind) == Some(kind)
    }
}

/// The properties a component defines, from its node change.
pub fn definitions(session: &Session, component: Guid) -> Vec<Definition> {
    session
        .raw(component)
        .and_then(|raw| raw.field(Field::ComponentPropDefs))
        .and_then(Raw::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|d| {
            let initial = d.field(Field::InitialValue);
            let default = initial.and_then(|v| {
                if let Some(t) = v.field(Field::TextValue).and_then(|t| t.field(Field::Characters)).and_then(Raw::as_str) {
                    return Some(PropValue::Text(t.to_string()));
                }
                if let Some(b) = v.field(Field::BoolValue).and_then(Raw::as_bool) {
                    return Some(PropValue::Bool(b));
                }
                v.field(Field::GuidValue).and_then(Guid::read).map(PropValue::Component)
            });
            Some(Definition {
                id: d.field(Field::Id).and_then(Guid::read)?,
                name: d.field(Field::Name).and_then(Raw::as_str)?.to_string(),
                kind: d.field(Field::Type).and_then(Raw::as_str).unwrap_or("").to_string(),
                default,
            })
        })
        .collect()
}

/// The property types grafta writes, by the plugin API's names.
pub const PROPERTY_TYPES: [(&str, ComponentPropType); 3] =
    [("TEXT", ComponentPropType::Text), ("BOOLEAN", ComponentPropType::Bool), ("INSTANCE_SWAP", ComponentPropType::InstanceSwap)];

/// A property's type in the plugin API's names, from the file's.
pub fn plugin_type(kind: &str) -> &str {
    PROPERTY_TYPES.iter().find(|(_, t)| t.as_wire() == kind).map_or(kind, |(name, _)| name)
}

/// A component by name or id, for an instance-swap value.
pub fn component(session: &Session, at: &str, value: &Data) -> Result<Guid, ToolError> {
    let wanted = value.as_str().ok_or_else(|| invalid(at, "a component's name or id", value))?;
    let found: Vec<&model::SceneNode> = session
        .graph
        .nodes
        .values()
        .filter(|n| n.node_type == NodeType::Component && (n.id.to_string() == wanted || n.name == wanted))
        .collect();
    match found.as_slice() {
        [one] => Ok(one.id),
        [] => Err(PropertyError::NoSuchComponent { at: at.into(), wanted: wanted.into() }.into()),
        many => Err(PropertyError::AmbiguousComponent {
            at: at.into(),
            wanted: wanted.into(),
            ids: many.iter().map(|n| n.id.to_string()).collect(),
        }
        .into()),
    }
}

/// A value for a property of `kind`.
pub fn value(session: &Session, at: &str, kind: &str, raw: &Data) -> Result<PropValue, ToolError> {
    match ComponentPropType::from_wire(kind) {
        // A number is its text: a client may send "32" as 32, and the ninth
        // session padded its values with a space to get past a refusal.
        Some(ComponentPropType::Text) => match raw {
            Data::String(s) => Ok(PropValue::Text(s.clone())),
            Data::Number(n) => Ok(PropValue::Text(n.to_string())),
            _ => Err(PropertyError::NotText { at: at.into(), got: raw.to_string() }.into()),
        },
        Some(ComponentPropType::Bool) => raw
            .as_bool()
            .map(PropValue::Bool)
            .ok_or_else(|| PropertyError::NotBoolean { at: at.into(), got: raw.to_string() }.into()),
        Some(ComponentPropType::InstanceSwap) => component(session, at, raw).map(PropValue::Component),
        _ => Err(PropertyError::SetElsewhere { at: at.into(), kind: kind.into() }.into()),
    }
}

fn find<'a>(definitions: &'a [Definition], at: &str, wanted: &str) -> Result<&'a Definition, ToolError> {
    definitions.iter().find(|d| d.name == wanted || d.id.to_string() == wanted).ok_or_else(|| {
        let names: Vec<&str> = definitions.iter().filter(|d| !d.is(ComponentPropType::Variant)).map(|d| d.name.as_str()).collect();
        PropertyError::NoSuchProperty {
            at: at.into(),
            wanted: wanted.into(),
            properties: names.iter().map(ToString::to_string).collect(),
        }
        .into()
    })
}

/// A layer's references to its component's properties, from
/// `componentPropertyReferences: { characters?, visible?, mainComponent? }`,
/// each a property name or `null`: the field, and the property with its
/// default, or `None` to unbind.
pub type Reference = (ComponentPropNodeField, Option<(Guid, PropValue)>);

pub fn references(session: &Session, id: Guid, value: &Data) -> Result<Vec<Reference>, ToolError> {
    let node = node_at(session, id)?;
    let Some(given) = value.as_map() else {
        return Err(invalid("componentPropertyReferences", "{ characters?, visible?, mainComponent? }", value));
    };
    let component = node
        .parent_id
        .and_then(|parent| super::guard::enclosing_component(session, parent))
        .ok_or(PropertyError::OutsideComponent { id })?;
    let definitions = definitions(session, component);
    let mut out = Vec::new();
    for (key, wanted) in given {
        let (field, kind, fits) = match key.as_str() {
            "characters" => (ComponentPropNodeField::TextData, ComponentPropType::Text, node.node_type == NodeType::Text),
            "visible" => (ComponentPropNodeField::Visible, ComponentPropType::Bool, true),
            "mainComponent" => (ComponentPropNodeField::OverriddenSymbolId, ComponentPropType::InstanceSwap, node.node_type == NodeType::Instance),
            other => {
                return Err(PropertyError::NotAReference { key: other.into() }.into())
            }
        };
        if !fits {
            return Err(PropertyError::WrongLayer {
                key: key.clone(),
                id,
                is: crate::view::type_name(node.node_type),
                wanted: if field == ComponentPropNodeField::TextData { "text" } else { "nested instance" },
            }
            .into());
        }
        if wanted.is_null() {
            out.push((field, None));
            continue;
        }
        let name = wanted
            .as_str()
            .ok_or_else(|| invalid(&format!("componentPropertyReferences.{key}"), "a property name or null", wanted))?;
        let definition = find(&definitions, &format!("componentPropertyReferences.{key}"), name)?;
        if !definition.is(kind) {
            return Err(PropertyError::WrongType {
                key: key.clone(),
                name: definition.name.clone(),
                is: plugin_type(&definition.kind).into(),
                takes: plugin_type(kind.as_wire()).into(),
            }
            .into());
        }
        let default =
            definition.default.clone().ok_or_else(|| PropertyError::NoDefault { name: definition.name.clone() })?;
        out.push((field, Some((definition.id, default))));
    }
    Ok(out)
}

/// An instance's values, from `componentProperties: { "<name>": value }`.
pub fn assignments(session: &Session, id: Guid, value: &Data) -> Result<Vec<(Guid, PropValue)>, ToolError> {
    let node = node_at(session, id)?;
    if node.node_type != NodeType::Instance {
        return Err(PropertyError::NotAnInstance { id }.into());
    }
    let Some(given) = value.as_map() else {
        return Err(invalid("componentProperties", "{ \"<property>\": value }", value));
    };
    let component = node.component_id.ok_or(PropertyError::NoComponent { id })?;
    let definitions = definitions(session, component);
    let mut out = Vec::new();
    for (name, raw) in given {
        let definition = find(&definitions, &format!("componentProperties.{name}"), name)?;
        out.push((definition.id, value_of(session, name, definition, raw)?));
    }
    Ok(out)
}

fn value_of(session: &Session, name: &str, definition: &Definition, raw: &Data) -> Result<PropValue, ToolError> {
    value(session, &format!("componentProperties.{name}"), &definition.kind, raw)
}
