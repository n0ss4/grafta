//! Figma design variables (`VARIABLE` / `VARIABLE_SET` node changes).
//!
//! Variables are the file's real design tokens: named, typed, and grouped into
//! collections with one value per mode (e.g. light/dark). They live in the same
//! `nodeChanges` array as scene nodes but are not part of the scene tree.

use std::collections::HashMap;

use indexmap::IndexMap;
use kiwi::Value;
use schema::{Field, Guid, NodeType, VariableDataType, VariableField, VariableResolvedDataType, Wire};

use crate::paint::VariableResolver;
use crate::types::{ModeId, Variable, VariableCollection, VariableMode, VariableType, VariableValue};

/// Figma's `variableField` enum → the scene-node field it binds.
pub const VARIABLE_FIELDS: &[(VariableField, &str)] = &[
    (VariableField::CornerRadius, "cornerRadius"),
    (VariableField::RectangleTopLeftCornerRadius, "cornerRadiusTopLeft"),
    (VariableField::RectangleTopRightCornerRadius, "cornerRadiusTopRight"),
    (VariableField::RectangleBottomLeftCornerRadius, "cornerRadiusBottomLeft"),
    (VariableField::RectangleBottomRightCornerRadius, "cornerRadiusBottomRight"),
    (VariableField::StrokeWeight, "strokeWeight"),
    (VariableField::BorderTopWeight, "borderTopWeight"),
    (VariableField::BorderBottomWeight, "borderBottomWeight"),
    (VariableField::BorderLeftWeight, "borderLeftWeight"),
    (VariableField::BorderRightWeight, "borderRightWeight"),
    (VariableField::StackSpacing, "itemSpacing"),
    (VariableField::StackPaddingLeft, "paddingLeft"),
    (VariableField::StackPaddingTop, "paddingTop"),
    (VariableField::StackPaddingRight, "paddingRight"),
    (VariableField::StackPaddingBottom, "paddingBottom"),
    (VariableField::StackCounterSpacing, "counterAxisSpacing"),
    (VariableField::GridRowGap, "rowGap"),
    (VariableField::GridColumnGap, "columnGap"),
    (VariableField::Visible, "visible"),
    (VariableField::Opacity, "opacity"),
    (VariableField::Width, "width"),
    (VariableField::Height, "height"),
    (VariableField::MinWidth, "minWidth"),
    (VariableField::MaxWidth, "maxWidth"),
    (VariableField::MinHeight, "minHeight"),
    (VariableField::MaxHeight, "maxHeight"),
    (VariableField::XPosition, "x"),
    (VariableField::YPosition, "y"),
    (VariableField::Rotation, "rotation"),
    (VariableField::FontSize, "fontSize"),
    (VariableField::LetterSpacing, "letterSpacing"),
    (VariableField::LineHeight, "lineHeight"),
    (VariableField::FontFamily, "fontFamily"),
    (VariableField::TextData, "text"),
];

#[must_use]
pub fn variable_field_name(field: VariableField) -> Option<&'static str> {
    VARIABLE_FIELDS.iter().find(|(k, _)| *k == field).map(|(_, v)| *v)
}

/// Map published-library keys (`key` and `key@version`) to local node ids, so
/// aliases that reference a library asset can be resolved inside this file.
#[must_use]
pub fn build_asset_refs(changes: &IndexMap<Guid, &Value>) -> HashMap<String, Guid> {
    let mut refs = HashMap::new();
    for (id, nc) in changes {
        let Some(key) = nc.str(Field::Key) else { continue };
        refs.insert(key.to_string(), *id);
        for version_field in [Field::Version, Field::UserFacingVersion] {
            if let Some(version) = nc.str(version_field) {
                refs.insert(format!("{key}@{version}"), *id);
            }
        }
    }
    refs
}

fn variable_type(resolved: Option<VariableResolvedDataType>) -> VariableType {
    match resolved {
        Some(VariableResolvedDataType::Color) => VariableType::Color,
        Some(VariableResolvedDataType::Boolean) => VariableType::Boolean,
        Some(VariableResolvedDataType::String) => VariableType::String,
        _ => VariableType::Float,
    }
}

/// A value's type: its own `dataType`, or the `resolvedDataType` it resolves to.
fn value_type(data: &Value) -> VariableType {
    if data.has(Field::DataType) {
        match data.known::<VariableDataType>(Field::DataType) {
            Some(VariableDataType::Color) => VariableType::Color,
            Some(VariableDataType::Boolean) => VariableType::Boolean,
            Some(VariableDataType::String) => VariableType::String,
            _ => VariableType::Float,
        }
    } else {
        variable_type(data.known(Field::ResolvedDataType))
    }
}

fn variable_value(data: &Value, resolver: &VariableResolver<'_>) -> Option<VariableValue> {
    let value = data.field(Field::Value)?;

    if let Some(alias) = value.field(Field::Alias) {
        if let Some(variable_id) = resolver.resolve_alias(alias) {
            return Some(VariableValue::Alias { variable_id });
        }
        return None;
    }
    match value_type(data) {
        VariableType::Color => value
            .field(Field::ColorValue)
            .and_then(crate::paint::color)
            .map(|value| VariableValue::Color { value }),
        VariableType::Boolean => {
            Some(VariableValue::Boolean { value: value.bool(Field::BoolValue).unwrap_or(false) })
        }
        VariableType::String => Some(VariableValue::String {
            value: value.str(Field::TextValue).unwrap_or_default().to_string(),
        }),
        VariableType::Float => value.f32(Field::FloatValue).map(|value| VariableValue::Float { value }),
    }
}

pub(crate) struct VariableData {
    pub collections: IndexMap<Guid, VariableCollection>,
    pub variables: IndexMap<Guid, Variable>,
}

pub(crate) fn extract(
    changes: &IndexMap<Guid, &Value>,
    parents: &IndexMap<Guid, Guid>,
    resolver: &VariableResolver<'_>,
) -> VariableData {
    let mut collections: IndexMap<Guid, VariableCollection> = IndexMap::new();
    let mut variables: IndexMap<Guid, Variable> = IndexMap::new();

    for (id, nc) in changes {
        if !nc.is(Field::Type, NodeType::VariableSet) {
            continue;
        }
        let modes: Vec<VariableMode> = nc
            .array(Field::VariableSetModes)
            .iter()
            .filter_map(|m| {
                Some(VariableMode {
                    id: ModeId::Declared(m.field(Field::Id).and_then(Guid::read)?),
                    name: m.str(Field::Name).unwrap_or("Default").to_string(),
                })
            })
            .collect();
        collections.insert(
            *id,
            VariableCollection {
                id: *id,
                name: nc.str(Field::Name).unwrap_or("Variables").to_string(),
                modes,
                variable_ids: Vec::new(),
            },
        );
    }

    for (id, nc) in changes {
        if !nc.is(Field::Type, NodeType::Variable) {
            continue;
        }
        let collection_id = collection_id_for(nc, *id, parents, resolver);
        let mut values_by_mode = IndexMap::new();
        for entry in nc.field(Field::VariableDataValues).map_or(&[][..], |v| v.array(Field::Entries)) {
            let (Some(mode_id), Some(data)) =
                (entry.field(Field::ModeId).and_then(Guid::read), entry.field(Field::VariableData))
            else {
                continue;
            };
            if let Some(value) = variable_value(data, resolver) {
                values_by_mode.insert(mode_id, value);
            }
        }

        variables.insert(
            *id,
            Variable {
                id: *id,
                name: nc.str(Field::Name).unwrap_or("Variable").to_string(),
                var_type: variable_type(nc.known(Field::VariableResolvedType)),
                collection_id,
                values_by_mode,
            },
        );
        if let Some(collection) = collections.get_mut(&collection_id) {
            collection.variable_ids.push(*id);
        }
    }

    // Variables whose collection lives in a library that was not embedded still
    // deserve a home, so they show up in the token output.
    let orphans: Vec<Guid> = variables
        .values()
        .filter(|v| !collections.contains_key(&v.collection_id))
        .map(|v| v.collection_id)
        .collect();
    for collection_id in orphans {
        let name = changes
            .get(&collection_id)
            .and_then(|nc| nc.str(Field::Name))
            .unwrap_or("Variables")
            .to_string();
        let entry = collections.entry(collection_id).or_insert(VariableCollection {
            id: collection_id,
            name,
            modes: vec![VariableMode { id: ModeId::Default, name: "Default".to_string() }],
            variable_ids: Vec::new(),
        });
        entry.variable_ids.extend(
            variables
                .values()
                .filter(|v| v.collection_id == collection_id)
                .map(|v| v.id),
        );
    }

    // Collections default to the modes actually referenced by their variables
    // when the set node itself declared none.
    for collection in collections.values_mut() {
        if !collection.modes.is_empty() {
            continue;
        }
        let mut seen: Vec<Guid> = Vec::new();
        for var_id in &collection.variable_ids {
            for mode in variables.get(var_id).into_iter().flat_map(|v| v.values_by_mode.keys()) {
                if !seen.contains(mode) {
                    seen.push(*mode);
                }
            }
        }
        collection.modes = seen
            .into_iter()
            .enumerate()
            .map(|(i, id)| VariableMode {
                id: ModeId::Declared(id),
                name: if i == 0 { "Default".to_string() } else { format!("Mode {}", i + 1) },
            })
            .collect();
    }

    VariableData { collections, variables }
}

fn collection_id_for(nc: &Value, id: Guid, parents: &IndexMap<Guid, Guid>, resolver: &VariableResolver<'_>) -> Guid {
    if let Some(set) = nc.field(Field::VariableSetId) {
        if let Some(resolved) = resolver.resolve_alias(set) {
            return resolved;
        }
    }
    parents.get(&id).copied().unwrap_or_default()
}

/// Resolve a variable to a concrete value, following alias chains.
#[must_use]
pub fn resolve_value<'a>(
    variables: &'a IndexMap<Guid, Variable>,
    variable: &'a Variable,
    mode_id: Option<Guid>,
) -> Option<&'a VariableValue> {
    resolve_value_inner(variables, variable, mode_id, 0)
}

fn resolve_value_inner<'a>(
    variables: &'a IndexMap<Guid, Variable>,
    variable: &'a Variable,
    mode_id: Option<Guid>,
    depth: usize,
) -> Option<&'a VariableValue> {
    if depth > 8 {
        return None;
    }
    let value = mode_id
        .and_then(|m| variable.values_by_mode.get(&m))
        .or_else(|| variable.values_by_mode.first().map(|(_, v)| v))?;
    match value {
        VariableValue::Alias { variable_id } => {
            let target = variables.get(variable_id)?;
            resolve_value_inner(variables, target, mode_id, depth + 1)
        }
        other => Some(other),
    }
}
