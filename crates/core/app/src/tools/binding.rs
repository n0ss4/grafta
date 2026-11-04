//! Variables bound to fields that are not paints, and modes.
//!
//! A binding is an entry of a node's `variableConsumptionMap`, and a mode is
//! chosen per collection by its `variableModeBySetMap` (677,338 bindings and
//! 1,700 mode maps measured). Beside each binding the node stores the value it
//! resolves to, and Figma draws that stored value, whatever mode the node is
//! in: a card set to Dark with Light's values stored draws in Light (measured
//! in Figma). So whenever a mode is set or a variable changes, the values of
//! every node it reaches are stored again here.

use std::collections::HashSet;

use kiwi::Value as Raw;
use model::{Color, VariableType, VariableValue};
use schema::{object, Field, Guid, VariableDataType, VariableField, VariableResolvedDataType, Wire, WireMut};
use crate::data::{Data, Map};

use super::variable::find_variable;
use super::style::invalid;
use crate::error::BindingError;
use crate::{DomainError, Session, ToolError};

/// A number `set_properties` can bind, and the `variableField`s it binds: a
/// radius and a padding bind each corner and side, as Figma does on 73,833
/// and about 50,000 nodes (a census of the fixtures).
pub const BINDABLE: [(&str, &[VariableField]); 14] = [
    ("itemSpacing", &[VariableField::StackSpacing]),
    ("counterAxisSpacing", &[VariableField::StackCounterSpacing]),
    ("padding", &[VariableField::StackPaddingTop, VariableField::StackPaddingRight, VariableField::StackPaddingBottom, VariableField::StackPaddingLeft]),
    ("paddingTop", &[VariableField::StackPaddingTop]),
    ("paddingRight", &[VariableField::StackPaddingRight]),
    ("paddingBottom", &[VariableField::StackPaddingBottom]),
    ("paddingLeft", &[VariableField::StackPaddingLeft]),
    ("cornerRadius", &CORNER_FIELDS),
    ("topLeftRadius", &[VariableField::RectangleTopLeftCornerRadius]),
    ("topRightRadius", &[VariableField::RectangleTopRightCornerRadius]),
    ("bottomRightRadius", &[VariableField::RectangleBottomRightCornerRadius]),
    ("bottomLeftRadius", &[VariableField::RectangleBottomLeftCornerRadius]),
    ("width", &[VariableField::Width]),
    ("height", &[VariableField::Height]),
];

/// Bindable on frames, components and texts alike.
pub const BINDABLE_EVERYWHERE: [(&str, &[VariableField]); 2] = [("opacity", &[VariableField::Opacity]), ("fontSize", &[VariableField::FontSize])];

const CORNER_FIELDS: [VariableField; 4] = [
    VariableField::RectangleTopLeftCornerRadius,
    VariableField::RectangleTopRightCornerRadius,
    VariableField::RectangleBottomRightCornerRadius,
    VariableField::RectangleBottomLeftCornerRadius,
];

/// What a node binds to a variable: a number, or the colour of one paint in a list.
#[derive(Clone, Copy)]
enum Binding {
    Number(VariableField),
    Paint(Field, usize),
}

/// Where each `variableField` this module writes keeps its value.
fn stored_in(field: VariableField) -> Option<Field> {
    Some(match field {
        VariableField::StackSpacing => Field::StackSpacing,
        VariableField::StackCounterSpacing => Field::StackCounterSpacing,
        VariableField::StackPaddingTop => Field::StackVerticalPadding,
        VariableField::StackPaddingRight => Field::StackPaddingRight,
        VariableField::StackPaddingBottom => Field::StackPaddingBottom,
        VariableField::StackPaddingLeft => Field::StackHorizontalPadding,
        VariableField::RectangleTopLeftCornerRadius => Field::RectangleTopLeftCornerRadius,
        VariableField::RectangleTopRightCornerRadius => Field::RectangleTopRightCornerRadius,
        VariableField::RectangleBottomRightCornerRadius => Field::RectangleBottomRightCornerRadius,
        VariableField::RectangleBottomLeftCornerRadius => Field::RectangleBottomLeftCornerRadius,
        VariableField::FontSize => Field::FontSize,
        VariableField::Opacity => Field::Opacity,
        VariableField::Width | VariableField::Height => Field::Size,
        _ => return None,
    })
}

fn fields_of(property: &str) -> Option<&'static [VariableField]> {
    BINDABLE.iter().chain(&BINDABLE_EVERYWHERE).find(|(p, _)| *p == property).map(|(_, f)| *f)
}

// ── Modes and values ─────────────────────────────────────────────────────────

/// The mode a node shows `collection` in: its own `variableModeBySetMap`, or
/// its nearest ancestor's, or the collection's first mode.
pub fn mode_for(session: &Session, node: Guid, collection: Guid) -> Option<Guid> {
    let mut current = Some(node);
    while let Some(id) = current {
        let chosen = session.raw(id).and_then(|raw| raw.field(Field::VariableModeBySetMap)).and_then(|m| m.field(Field::Entries)).and_then(Raw::as_array).and_then(|entries| {
            entries.iter().find_map(|e| {
                let set = e.field(Field::VariableSetId).and_then(|s| s.field(Field::Guid)).and_then(Guid::read)?;
                (set == collection).then(|| e.field(Field::VariableModeId).and_then(Guid::read)).flatten()
            })
        });
        if chosen.is_some() {
            return chosen;
        }
        current = session.graph.node(id).and_then(|n| n.parent_id);
    }
    session.graph.collections.get(&collection).and_then(|c| c.modes.first()).and_then(|m| m.id.declared())
}

/// A variable's value as `node` shows it: in the mode in effect there for its
/// collection, following aliases, each in its own collection's mode.
pub fn resolve(session: &Session, node: Guid, variable: Guid) -> Option<VariableValue> {
    let mut current = variable;
    for _ in 0..16 {
        let v = session.graph.variables.get(&current)?;
        let mode = mode_for(session, node, v.collection_id);
        let value = mode.and_then(|m| v.values_by_mode.get(&m)).or_else(|| v.values_by_mode.first().map(|(_, value)| value))?;
        match value {
            VariableValue::Alias { variable_id } => current = *variable_id,
            other => return Some(other.clone()),
        }
    }
    None
}

fn number_at(session: &Session, node: Guid, variable: Guid) -> Option<f32> {
    match resolve(session, node, variable)? {
        VariableValue::Float { value } => Some(value),
        _ => None,
    }
}

// ── Taking bindings out of set_properties ────────────────────────────────────

/// What a call binds and unbinds: `(variableField, variable id)` to write, and
/// every field the call gave a value, bound or not, whose old binding goes.
#[derive(Default)]
pub struct Bindings {
    pub bind: Vec<(VariableField, Guid)>,
    pub touched: Vec<VariableField>,
}

/// Replace each `{ variable }` in `properties` by the number it resolves to on
/// `node`, and say what is bound. Refused: a variable that is not a number, a
/// library variable, and a bound visibility.
pub fn take(session: &Session, node: Guid, properties: &Map) -> Result<(Map, Bindings), ToolError> {
    let mut out = properties.clone();
    let mut bindings = Bindings::default();
    if properties.get("visible").is_some_and(Data::is_map) {
        return Err(BindingError::VisibleBound.into());
    }
    for (property, value) in properties {
        let Some(fields) = fields_of(property) else { continue };
        bindings.touched.extend(fields.iter().copied());
        let Some(object) = value.as_map() else { continue };
        let wanted = object
            .get("variable")
            .and_then(Data::as_str)
            .filter(|_| object.len() == 1)
            .ok_or_else(|| invalid(property, "a number or { \"variable\": \"<name or id>\" }", value))?;
        let variable = find_variable(&session.graph, property, wanted)?;
        if variable.var_type != VariableType::Float {
            return Err(BindingError::NotANumber {
                property: property.clone(),
                name: variable.name.clone(),
                kind: variable.var_type,
            }
            .into());
        }
        if session.raw(variable.id).is_some_and(|raw| raw.field(Field::SourceLibraryKey).is_some()) {
            return Err(DomainError::LibraryVariable { at: property.clone(), name: variable.name.clone() }.into());
        }
        let resolved = number_at(session, node, variable.id)
            .ok_or_else(|| BindingError::NoNumber { property: property.clone(), name: variable.name.clone() })?;
        out.insert(property.clone(), Data::from(f64::from(resolved)));
        bindings.bind.extend(fields.iter().map(|f| (*f, variable.id)));
    }
    Ok((out, bindings))
}

/// One binding, as all 670,704 measured bindings to a number are.
fn entry(field: VariableField, variable: Guid) -> Raw {
    let alias = object([(Field::Guid, variable.to_value())]);
    object([
        (Field::VariableData,
            object([
                (Field::Value, object([(Field::Alias, alias)])),
                (Field::DataType, Raw::from(VariableDataType::Alias)),
                (Field::ResolvedDataType, Raw::from(VariableResolvedDataType::Float)),
            ]),
        ),
        (Field::VariableField, field.into()),
    ])
}

/// The node's consumption map after `bindings`: its old entries for fields
/// the call did not touch, kept whatever they are, then the new ones. `None`
/// when the map is unchanged.
pub fn consumption_map(session: &Session, node: Guid, bindings: &Bindings) -> Option<Option<Raw>> {
    if bindings.touched.is_empty() {
        return None;
    }
    let old: Vec<Raw> = session
        .raw(node)
        .and_then(|raw| raw.field(Field::VariableConsumptionMap))
        .and_then(|m| m.field(Field::Entries))
        .and_then(Raw::as_array)
        .cloned()
        .unwrap_or_default();
    let had = !old.is_empty();
    let mut entries: Vec<Raw> = old
        .into_iter()
        .filter(|e| e.known::<VariableField>(Field::VariableField).is_none_or(|f| !bindings.touched.contains(&f)))
        .collect();
    entries.extend(bindings.bind.iter().map(|(field, variable)| entry(*field, *variable)));
    if entries.is_empty() {
        return had.then_some(None);
    }
    Some(Some(object([(Field::Entries, Raw::Array(entries))])))
}

// ── Modes ────────────────────────────────────────────────────────────────────

/// The node's `variableModeBySetMap` after `{ collection: mode | null }`,
/// validated against the file's collections.
pub fn modes(session: &Session, node: Guid, value: &Data) -> Result<Option<Raw>, ToolError> {
    let Some(given) = value.as_map() else {
        return Err(invalid("explicitVariableModes", "{ \"<collection>\": \"<mode>\" }", value));
    };
    let mut entries: Vec<Raw> = session
        .raw(node)
        .and_then(|raw| raw.field(Field::VariableModeBySetMap))
        .and_then(|m| m.field(Field::Entries))
        .and_then(Raw::as_array)
        .cloned()
        .unwrap_or_default();
    for (name, mode) in given {
        let collection = session
            .graph
            .collections
            .values()
            .find(|c| c.name == *name || c.id.to_string() == *name)
            .ok_or_else(|| {
                let names: Vec<&str> = session.graph.collections.values().map(|c| c.name.as_str()).collect();
                BindingError::NoSuchCollection {
                    name: name.clone(),
                    collections: names.iter().map(ToString::to_string).collect(),
                }
            })?;
        entries.retain(|e| e.field(Field::VariableSetId).and_then(|s| s.field(Field::Guid)).and_then(Guid::read) != Some(collection.id));
        if mode.is_null() {
            continue;
        }
        let wanted = mode.as_str().unwrap_or_default();
        let chosen = collection.modes.iter().find(|m| m.name == wanted || m.id.to_string() == wanted).ok_or_else(|| {
            let names: Vec<&str> = collection.modes.iter().map(|m| m.name.as_str()).collect();
            BindingError::NoSuchMode {
                name: name.clone(),
                collection: collection.name.clone(),
                mode: mode.to_string(),
                modes: names.iter().map(ToString::to_string).collect(),
            }
        })?;
        let Some(mode) = chosen.id.declared() else {
            return Err(BindingError::NotAGuid { id: collection.id }.into());
        };
        entries.push(object([
            (Field::VariableSetId, object([(Field::Guid, collection.id.to_value())])),
            (Field::VariableModeId, mode.to_value()),
        ]));
    }
    Ok((!entries.is_empty()).then(|| object([(Field::Entries, Raw::Array(entries))])))
}

// ── Storing values again ─────────────────────────────────────────────────────

/// A node and everything under it.
pub fn subtree(session: &Session, root: Guid) -> Vec<Guid> {
    let mut out = Vec::new();
    session.graph.walk(root, |node, _| out.push(node.id));
    out
}

/// Every node bound, directly or through aliases, to one of `variables`.
pub fn reached_by(session: &Session, variables: &HashSet<Guid>) -> Vec<Guid> {
    session
        .graph
        .nodes
        .values()
        .filter(|node| {
            session.raw(node.id).is_some_and(|raw| bound_variables(raw).iter().any(|(_, v)| alias_reaches(session, *v, variables)))
        })
        .map(|node| node.id)
        .collect()
}

/// Whether any mode of `start` aliases, however deep, into `targets`.
fn alias_reaches(session: &Session, start: Guid, targets: &HashSet<Guid>) -> bool {
    let mut stack = vec![(start, 0)];
    while let Some((id, depth)) = stack.pop() {
        if targets.contains(&id) {
            return true;
        }
        if depth > 16 {
            continue;
        }
        if let Some(v) = session.graph.variables.get(&id) {
            for value in v.values_by_mode.values() {
                if let VariableValue::Alias { variable_id } = value {
                    stack.push((*variable_id, depth + 1));
                }
            }
        }
    }
    false
}

/// Every variable a node change binds: its consumption map's numbers, and its
/// paints' colours, as `(where, variable id)`.
fn bound_variables(raw: &Raw) -> Vec<(Binding, Guid)> {
    let mut out = Vec::new();
    for e in raw.field(Field::VariableConsumptionMap).and_then(|m| m.field(Field::Entries)).and_then(Raw::as_array).into_iter().flatten() {
        let (Some(field), Some(var)) = (
            e.known::<VariableField>(Field::VariableField),
            e.field(Field::VariableData).and_then(|d| d.field(Field::Value)).and_then(|v| v.field(Field::Alias)).and_then(|a| a.field(Field::Guid)).and_then(Guid::read),
        ) else {
            continue;
        };
        out.push((Binding::Number(field), var));
    }
    for paints in [Field::FillPaints, Field::StrokePaints] {
        for (i, p) in raw.field(paints).and_then(Raw::as_array).into_iter().flatten().enumerate() {
            if let Some(var) = p.field(Field::ColorVar).and_then(|c| c.field(Field::Value)).and_then(|v| v.field(Field::Alias)).and_then(|a| a.field(Field::Guid)).and_then(Guid::read) {
                out.push((Binding::Paint(paints, i), var));
            }
        }
    }
    out
}

/// Store again, on each node named, every bound value as it resolves there:
/// the numbers its consumption map binds and its bound paints' colours. A
/// paint keeps its own opacity (the variable's alpha went into it when it was
/// bound). Nodes inside a component make its instances' caches stale.
/// A text given another font size drops its layout data and has its box
/// estimated again; what that did is returned.
pub fn restore(session: &mut Session, ids: &[Guid]) -> Result<super::text::Fitted, ToolError> {
    let mut changed = Vec::new();
    for &id in ids {
        let Some(raw) = session.raw(id) else { continue };
        let bound = bound_variables(raw);
        if bound.is_empty() {
            continue;
        }
        let mut writes: Vec<(Field, Raw)> = Vec::new();
        let mut corners: Vec<(Field, Guid, f32)> = Vec::new();
        let mut size = raw.field(Field::Size).cloned();
        for &(binding, variable) in &bound {
            let field = match binding {
                Binding::Number(field) => field,
                Binding::Paint(paints, index) => {
                    let Some(VariableValue::Color { value }) = resolve(session, id, variable) else { continue };
                    let current = writes.iter().rev().find(|(k, _)| *k == paints).map(|(_, v)| v.clone()).or_else(|| raw.field(paints).cloned());
                    if let Some(Raw::Array(mut list)) = current {
                        if let Some(Raw::Object(paint)) = list.get_mut(index) {
                            paint.set(Field::Color, colour(value));
                        }
                        writes.push((paints, Raw::Array(list)));
                    }
                    continue;
                }
            };
            let Some(target) = stored_in(field) else { continue };
            let Some(value) = number_at(session, id, variable) else { continue };
            match field {
                VariableField::Width | VariableField::Height => {
                    if let Some(Raw::Object(s)) = size.as_mut() {
                        s.set(if field == VariableField::Width { Field::X } else { Field::Y }, Raw::Float(value));
                    }
                }
                f if CORNER_FIELDS.contains(&f) => corners.push((target, variable, value)),
                _ => writes.push((target, Raw::Float(value))),
            }
        }
        if let Some(size) = size.filter(|s| Some(s) != raw.field(Field::Size)) {
            writes.push((Field::Size, size));
        }
        if !corners.is_empty() {
            // Four corners bound to one variable are one radius, as a spike wrote
            // it and Figma showed it.
            let uniform = corners.len() == 4 && corners.iter().all(|(_, v, _)| *v == corners[0].1);
            for (field, _, value) in &corners {
                writes.push((*field, Raw::Float(*value)));
            }
            if uniform {
                writes.push((Field::CornerRadius, Raw::Float(corners[0].2)));
            }
            writes.push((Field::RectangleCornerRadiiIndependent, Raw::Bool(!uniform)));
        }
        if !writes.is_empty() {
            changed.push((id, writes));
        }
    }
    if changed.is_empty() {
        return Ok(super::text::Fitted::default());
    }
    let ids: Vec<Guid> = changed.iter().map(|(id, _)| *id).collect();
    let mut resized = Vec::new();
    for (id, writes) in changed {
        let Some(fields) = session.fields_mut(id) else { continue };
        let is_text = fields.is(Field::Type, schema::NodeType::Text);
        for (key, value) in writes {
            if is_text && key == Field::FontSize && fields.field(Field::FontSize) != Some(&value) {
                // Layout data for another size would be stale.
                fields.take(Field::DerivedTextData);
                resized.push(id);
            }
            fields.set(key, value);
        }
        if let Some(component) = super::guard::enclosing_component(session, id) {
            session.stale_components.push(component);
        }
    }
    session.refresh(&ids);
    super::text::fit(session, &resized)
}

fn colour(c: Color) -> Raw {
    object([
        (Field::R, Raw::Float(c.r)),
        (Field::G, Raw::Float(c.g)),
        (Field::B, Raw::Float(c.b)),
        (Field::A, Raw::Float(1.0)),
    ])
}
