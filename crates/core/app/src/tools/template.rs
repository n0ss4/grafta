//! A component placed once per row of data.
//!
//! Every row is resolved against the component before anything is written, so
//! a bad key refuses the whole call, naming the row. Each row is then the calls
//! a model would make one by one, `create_instance`, `set_properties`
//! `componentProperties` and `override_instance`, run through the tools' own
//! implementations: the fill is one call, journaled once, laid out and
//! its caches dropped once.

use std::path::Path;

use model::NodeType;
use schema::{ComponentPropType, Guid, ImageScaleMode, PaintType, WireName};

use super::read::require_node;
use super::{overrides, style, write};
use crate::data::{data, Data, Map};
use crate::Command;
use crate::error::TemplateError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// The most rows one call places.
pub const MAX_ROWS: usize = 1000;

/// A component placed once per row.
pub struct FillTemplate {
    pub handle: String,
    pub component: String,
    pub parent: String,
    pub rows: Rows,
}

/// Where the rows are: given in the call, or in a CSV file.
pub enum Rows {
    Given(Vec<Map>),
    Csv(String),
}

impl Command for FillTemplate {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let (handle, parent) = (&self.handle, &self.parent);
        let rows = match &self.rows {
            Rows::Given(rows) => rows.clone(),
            Rows::Csv(path) => {
                let text = kit
                    .ports()
                    .store
                    .read_text(Path::new(path))
                    .map_err(|error| DomainError::CannotRead { path: path.into(), error: error.to_string() })?;
                csv_rows(&text).map_err(|error| TemplateError::Csv { path: path.clone(), error })?
            }
        };
        if rows.is_empty() || rows.len() > MAX_ROWS {
            return Err(TemplateError::RowCount { max: MAX_ROWS, count: rows.len() }.into());
        }
        let session = kit.session(handle)?;
        require_node(session, parent)?;
        let plans = rows
            .iter()
            .enumerate()
            .map(|(index, row)| plan(session, &self.component, index, row))
            .collect::<Result<Vec<_>, _>>()?;

        let mut instances = Vec::with_capacity(plans.len());
        for plan in plans {
            let create = write::CreateInstance {
                handle: handle.clone(),
                parent: parent.clone(),
                component: plan.component.to_string(),
                name: plan.name,
                at: (0.0, 0.0),
            };
            let id = id_of(create.run(kit)?)?;
            if !plan.properties.is_empty() {
                let properties = Data::Map(Map::from([("componentProperties".to_owned(), Data::Map(plan.properties))]));
                style::SetProperties { handle: handle.clone(), id: id.clone(), properties }.run(kit)?;
            }
            for (layer, properties) in plan.layers {
                let layer = Some(vec![layer.to_string()]);
                overrides::OverrideInstance { handle: handle.clone(), id: id.clone(), layer, properties }.run(kit)?;
            }
            instances.push(id);
        }
        let session = kit.session(handle)?;
        Ok(Output::Data(data!({
            "component": self.component,
            "parent": parent,
            "created": instances.len(),
            "instances": instances,
            "unsaved_edits": session.edits,
        })))
    }
}

/// What one row becomes: the component to place, its name, its property
/// values, and its layers' overrides.
struct Plan {
    component: Guid,
    name: Option<String>,
    properties: Map,
    layers: Vec<(Guid, Map)>,
}

fn plan(session: &Session, component: &str, index: usize, row: &Map) -> Result<Plan, ToolError> {
    let at = |key: &str| format!("row {index}, `{key}`");
    let template = require_node(session, component)?;
    // A variant set: the first variant's values, changed by the row's.
    let target = match template.node_type {
        NodeType::Component => template.clone(),
        NodeType::ComponentSet => {
            let variants: Vec<&model::SceneNode> = session.graph.children(template.id).filter(|v| v.node_type == NodeType::Component).collect();
            let first = variants.first().ok_or_else(|| TemplateError::EmptySet { component: component.into() })?;
            let mut wanted = first.variant_values.clone();
            for (key, value) in row {
                if wanted.contains_key(key) {
                    let value =
                        text(value).ok_or_else(|| TemplateError::VariantValueNotText { at: at(key), got: value.to_string() })?;
                    wanted.insert(key.clone(), value);
                }
            }
            let found = variants
                .iter()
                .find(|v| v.variant_values.len() == wanted.len() && wanted.iter().all(|(p, x)| v.variant_values.get(p) == Some(x)))
                .ok_or_else(|| {
                    let names: Vec<&str> = variants.iter().map(|v| v.name.as_str()).collect();
                    TemplateError::NoSuchVariant {
                        row: index,
                        set: template.name.clone(),
                        variants: names.iter().map(ToString::to_string).collect(),
                    }
                })?;
            (*found).clone()
        }
        NodeType::Frame => {
            return Err(TemplateError::AFrame { component: component.into() }.into())
        }
        other => {
            return Err(TemplateError::NotAComponent { component: component.into(), is: crate::view::type_name(other) }.into())
        }
    };
    let variant_keys: Vec<&String> = target.variant_values.keys().collect();
    let definitions = super::properties::definitions(session, target.id);
    let layers = layers(session, target.id);

    let mut plan = Plan { component: target.id, name: None, properties: Map::new(), layers: Vec::new() };
    for (key, value) in row {
        if key == "name" {
            let name = text(value).ok_or_else(|| TemplateError::NameNotText { at: at(key), got: value.to_string() })?;
            // A name is a non-empty string, as create_instance reads one.
            if name.is_empty() {
                return Err(DomainError::Expected { parameter: "name".into(), expected: "a non-empty string".into(), got: value.to_string() }.into());
            }
            plan.name = Some(name);
        } else if variant_keys.contains(&key) {
            // Chosen above.
        } else if let Some(definition) = definitions.iter().find(|d| &d.name == key && !d.is(ComponentPropType::Variant)) {
            let value = match value {
                // From CSV, a boolean arrives as text.
                Data::String(s) if definition.is(ComponentPropType::Bool) && s.eq_ignore_ascii_case("true") => Data::Bool(true),
                Data::String(s) if definition.is(ComponentPropType::Bool) && s.eq_ignore_ascii_case("false") => Data::Bool(false),
                _ => value.clone(),
            };
            super::properties::value(session, &at(key), &definition.kind, &value)?;
            plan.properties.insert(key.clone(), value);
        } else {
            let named: Vec<&(Guid, String, NodeType)> = layers.iter().filter(|(_, name, _)| name == key).collect();
            match named.as_slice() {
                [(id, _, kind)] => plan.layers.push((*id, layer_value(session.ports.images.as_ref(), *kind, value, &at(key))?)),
                [] => {
                    let mut known: Vec<String> = vec!["name".into()];
                    known.extend(variant_keys.iter().map(|k| (*k).clone()));
                    known.extend(definitions.iter().filter(|d| !d.is(ComponentPropType::Variant)).map(|d| d.name.clone()));
                    known.extend(layers.iter().map(|(_, name, _)| name.clone()));
                    known.dedup();
                    return Err(TemplateError::NoSuchKey { at: at(key), component: target.name.clone(), keys: known }.into());
                }
                several => {
                    let ids: Vec<String> = several.iter().map(|(id, _, _)| id.to_string()).collect();
                    return Err(TemplateError::SeveralLayers { at: at(key), ids }.into());
                }
            }
        }
    }
    Ok(plan)
}

/// The component's layers by id, name and type, not entering nested instances:
/// a layer inside one is reached with `override_instance`'s path.
fn layers(session: &Session, component: Guid) -> Vec<(Guid, String, NodeType)> {
    let mut out = Vec::new();
    let mut stack: Vec<&model::SceneNode> = session.graph.children(component).collect();
    while let Some(node) = stack.pop() {
        out.push((node.id, node.name.clone(), node.node_type));
        if node.node_type != NodeType::Instance {
            stack.extend(session.graph.children(node.id));
        }
    }
    out
}

/// A layer's override from a row's value: a text's characters, or another
/// layer's image fill from a PNG or JPEG path.
fn layer_value(images: &dyn crate::port::ImageSource, kind: NodeType, value: &Data, at: &str) -> Result<Map, ToolError> {
    let given = text(value).ok_or_else(|| TemplateError::LayerValueNotText { at: at.into(), got: value.to_string() })?;
    if kind == NodeType::Text {
        return Ok(Map::from([("characters".to_owned(), Data::from(given))]));
    }
    let image = Path::new(&given);
    let is_image = image.extension().and_then(|e| e.to_str()).is_some_and(|e| ["png", "jpg", "jpeg"].contains(&e.to_ascii_lowercase().as_str()));
    if !is_image {
        return Err(TemplateError::NotAnImagePath { at: at.into(), given }.into());
    }
    if !images.is_file(image) {
        return Err(TemplateError::NoImage { at: at.into(), given }.into());
    }
    let paint = Map::from([
        ("type".to_owned(), Data::from(PaintType::Image.as_wire())),
        ("image".to_owned(), Data::from(given)),
        ("scaleMode".to_owned(), Data::from(ImageScaleMode::Fill.as_wire())),
    ]);
    Ok(Map::from([("fills".to_owned(), Data::List(vec![Data::Map(paint)]))]))
}

/// A string, or a number as its text.
fn text(value: &Data) -> Option<String> {
    match value {
        Data::String(s) => Some(s.clone()),
        Data::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn id_of(output: Output) -> Result<String, ToolError> {
    match output {
        Output::Data(value) => {
            value["id"].as_str().map(str::to_string).ok_or_else(|| TemplateError::PlacedWithoutId.into())
        }
        Output::Image { .. } => Err(TemplateError::PictureReturned.into()),
    }
}

/// CSV as RFC 4180 has it: the first record names the keys; a field in double
/// quotes may hold commas, line breaks and doubled quotes. An empty field is
/// left out of its row, so it changes nothing.
pub fn csv_rows(text: &str) -> Result<Vec<Map>, String> {
    let mut records: Vec<Vec<String>> = Vec::new();
    let (mut record, mut field, mut quoted, mut chars) = (Vec::new(), String::new(), false, text.chars().peekable());
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') if field.is_empty() => quoted = true,
            (false, ',') => record.push(std::mem::take(&mut field)),
            (false, '\r') => {}
            (false, '\n') => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            (false, c) => field.push(c),
        }
    }
    if quoted {
        return Err("a quoted field is not closed".into());
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records.retain(|r| !(r.len() == 1 && r[0].is_empty()));
    let mut records = records.into_iter();
    let keys = records.next().ok_or("it is empty")?;
    records
        .enumerate()
        .map(|(i, values)| {
            if values.len() != keys.len() {
                return Err(format!("line {} has {} fields and the first has {}", i + 2, values.len(), keys.len()));
            }
            Ok(keys.iter().zip(values).filter(|(_, v)| !v.is_empty()).map(|(k, v)| (k.clone(), Data::String(v))).collect())
        })
        .collect()
}
