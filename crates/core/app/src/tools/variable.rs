//! Writing variables.
//!
//! A collection is a `VARIABLE_SET` and a variable a `VARIABLE`, both under
//! the file's Internal Only Canvas, measured on 34 collections and 1,725
//! variables in three fixtures. They are written in the shape a spike's
//! first file had, the one Figma opened; the author half is `model::author`. The graph
//! learns each one as it is written, so `set_properties` binds to a variable
//! the call after it is made.
//!
//! Only what grafta made is changed. A file's own variables reach every
//! binding in it, which nothing here has measured.

use model::author::{Guid, Provenance, VarType, VarValue};
use model::{Color, ModeId, SceneGraph, Variable, VariableCollection, VariableMode, VariableType, VariableValue};
use schema::{Field, Wire};

use super::catalog::variable_json;
use super::style::{closest, hex};
use super::write::author_error;
use crate::data::{data, Data};
use crate::Command;
use crate::error::VariableError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

// ── Finding ──────────────────────────────────────────────────────────────────

fn find_collection<'a>(graph: &'a SceneGraph, wanted: &str) -> Result<&'a VariableCollection, ToolError> {
    let matches: Vec<&VariableCollection> =
        graph.collections.values().filter(|c| c.id.to_string() == wanted || c.name == wanted).collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => {
            let names: Vec<&str> = graph.collections.values().map(|c| c.name.as_str()).collect();
            Err(VariableError::NoSuchCollection {
                wanted: wanted.into(),
                names: names.iter().map(ToString::to_string).collect(),
            }
            .into())
        }
        many => Err(VariableError::AmbiguousCollection {
            wanted: wanted.into(),
            ids: many.iter().map(|c| c.id.to_string()).collect(),
        }
        .into()),
    }
}

pub fn find_variable<'a>(graph: &'a SceneGraph, at: &str, wanted: &str) -> Result<&'a Variable, ToolError> {
    let matches: Vec<&Variable> = graph.variables.values().filter(|v| v.id.to_string() == wanted || v.name == wanted).collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => {
            let names: Vec<&str> = graph.variables.values().map(|v| v.name.as_str()).collect();
            let near = closest(wanted, &names, 3);
            Err(VariableError::NoSuchVariable {
                at: at.into(),
                wanted: wanted.into(),
                closest: near.iter().map(ToString::to_string).collect(),
            }
            .into())
        }
        many => Err(DomainError::AmbiguousVariable {
            at: at.into(),
            wanted: wanted.into(),
            ids: many.iter().map(|v| v.id.to_string()).collect(),
        }
        .into()),
    }
}

/// The guid of a collection or variable grafta made, or why it cannot be changed.
fn ours(session: &Session, id: Guid, what: &'static str) -> Result<Guid, ToolError> {
    if session.raw(id).is_some_and(|raw| raw.has(Field::SourceLibraryKey)) {
        return Err(VariableError::Library { what, id }.into());
    }
    if !id.is_graftas() {
        return Err(VariableError::FilesOwn { what, id }.into());
    }
    Ok(id)
}

// ── Values ───────────────────────────────────────────────────────────────────

fn type_of(var_type: VariableType) -> VarType {
    match var_type {
        VariableType::Color => VarType::Color,
        VariableType::Float => VarType::Float,
        VariableType::String => VarType::String,
        VariableType::Boolean => VarType::Boolean,
    }
}

/// Whether following `from`'s aliases, in any mode, reaches `target`.
fn reaches(graph: &SceneGraph, from: Guid, target: Guid, depth: usize) -> bool {
    if from == target {
        return true;
    }
    if depth > 16 {
        return false;
    }
    graph.variables.get(&from).is_some_and(|variable| {
        variable.values_by_mode.values().any(|value| match value {
            VariableValue::Alias { variable_id } => reaches(graph, *variable_id, target, depth + 1),
            _ => false,
        })
    })
}

/// One value, checked against the variable's type. `own` is the variable the
/// value belongs to, once it exists, so an alias back to it is a cycle.
fn value(
    session: &Session,
    at: &str,
    (var_type, resolved): (VarType, VariableType),
    raw: &Data,
    own: Option<Guid>,
) -> Result<(VarValue, VariableValue), ToolError> {
    if let Some(alias) = raw.get("alias") {
        let wanted = alias.as_str().ok_or_else(|| DomainError::Unexpected {
            at: format!("{at}.alias"),
            expected: "a variable name or id".into(),
        })?;
        let target = find_variable(&session.graph, &format!("{at}.alias"), wanted)?;
        if session.raw(target.id).is_some_and(|r| r.field(Field::SourceLibraryKey).is_some()) {
            return Err(VariableError::AliasToLibrary { at: at.into(), name: target.name.clone() }.into());
        }
        if target.var_type != resolved {
            return Err(VariableError::AliasType {
                at: at.into(),
                name: target.name.clone(),
                kind: target.var_type,
                wanted: resolved,
            }
            .into());
        }
        if own.is_some_and(|own| reaches(&session.graph, target.id, own, 0)) {
            return Err(VariableError::AliasCycle { at: at.into(), name: target.name.clone() }.into());
        }
        return Ok((VarValue::Alias(target.id), VariableValue::Alias { variable_id: target.id }));
    }
    let wrong = || {
        let expected = match var_type {
            VarType::Color => "#RRGGBB or #RRGGBBAA",
            VarType::Float => "a number",
            VarType::String => "a string",
            VarType::Boolean => "true or false",
        };
        super::style::invalid(at, expected, raw)
    };
    Ok(match var_type {
        VarType::Color => {
            if !raw.is_string() {
                return Err(wrong());
            }
            let (r, g, b, a) = hex(at, raw)?;
            (VarValue::Color(r, g, b, a), VariableValue::Color { value: Color { r, g, b, a } })
        }
        VarType::Float => {
            let n = raw.as_f64().map(|n| n as f32).filter(|n| n.is_finite()).ok_or_else(wrong)?;
            (VarValue::Float(n), VariableValue::Float { value: n })
        }
        VarType::String => {
            let s = raw.as_str().ok_or_else(wrong)?.to_string();
            (VarValue::String(s.clone()), VariableValue::String { value: s })
        }
        VarType::Boolean => {
            let b = raw.as_bool().ok_or_else(wrong)?;
            (VarValue::Boolean(b), VariableValue::Boolean { value: b })
        }
    })
}

/// A value per mode, keyed by mode name (or id). With `every`, each mode of
/// the collection needs one: all 1,725 measured variables have a value in
/// every mode of their collection.
fn values(
    session: &Session,
    collection: &VariableCollection,
    types: (VarType, VariableType),
    raw: &Data,
    every: bool,
    own: Option<Guid>,
) -> Result<Vec<(Guid, VarValue, VariableValue)>, ToolError> {
    let Some(given) = raw.as_map() else {
        return Err(DomainError::Unexpected { at: "values".into(), expected: "an object of mode name → value".into() }.into());
    };
    let names: Vec<&str> = collection.modes.iter().map(|m| m.name.as_str()).collect();
    for key in given.keys() {
        if !collection.modes.iter().any(|m| m.name == *key || m.id.to_string() == *key) {
            return Err(VariableError::NoSuchMode {
                key: key.clone(),
                collection: collection.name.clone(),
                modes: names.iter().map(ToString::to_string).collect(),
            }
            .into());
        }
    }
    let mut out = Vec::new();
    for mode in &collection.modes {
        let given = given.get(&mode.name).or_else(|| given.get(&mode.id.to_string()));
        match given {
            Some(raw) => {
                let (write, read) = value(session, &format!("values.{}", mode.name), types, raw, own)?;
                let declared =
                    mode.id.declared().ok_or_else(|| VariableError::NoModeIds { collection: collection.name.clone() })?;
                out.push((declared, write, read));
            }
            None if every => {
                return Err(VariableError::MissingValue {
                    mode: mode.name.clone(),
                    modes: names.iter().map(ToString::to_string).collect(),
                }
                .into())
            }
            None => {}
        }
    }
    Ok(out)
}

fn name_taken(graph: &SceneGraph, collection: Guid, name: &str, except: Option<Guid>) -> Option<Guid> {
    graph.variables.values().find(|v| v.collection_id == collection && v.name == name && Some(v.id) != except).map(|v| v.id)
}

fn collection_json(collection: &VariableCollection) -> Data {
    data!({
        "id": collection.id,
        "name": collection.name,
        "modes": collection.modes.iter().map(|m| data!({ "id": m.id, "name": m.name })).collect::<Vec<_>>(),
        "variables": collection.variable_ids.len(),
    })
}

// ── The tools ────────────────────────────────────────────────────────────────

/// A variable collection with named modes.
pub struct CreateVariableCollection {
    pub handle: String,
    pub name: String,
    pub modes: Vec<String>,
}

impl Command for CreateVariableCollection {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let (name, modes) = (self.name.clone(), &self.modes);
        if let Some(taken) = session.graph.collections.values().find(|c| c.name == name) {
            return Err(VariableError::CollectionNameTaken { name, id: taken.id }.into());
        }

        let names: Vec<&str> = modes.iter().map(String::as_str).collect();
        let ((canvas, wrote_canvas), (set, mode_ids)) = {
            let mut graft = session.graft().map_err(author_error)?;
            let canvas = graft.internal_canvas().map_err(author_error)?;
            (canvas, graft.add_variable_collection(&name, &names).map_err(author_error)?)
        };
        if wrote_canvas {
            session
                .insert_appended(&[canvas])
                .map_err(|e| DomainError::CanvasNotReadBack { error: e.to_string() })?;
            session.created.push(canvas);
        }
        session.index_appended();
        let collection = VariableCollection {
            id: set,
            name,
            modes: mode_ids.iter().zip(modes).map(|(id, name)| VariableMode { id: ModeId::Declared(*id), name: name.clone() }).collect(),
            variable_ids: Vec::new(),
        };
        let mut out = collection_json(&collection);
        session.graph.collections.insert(collection.id, collection);
        session.created.push(set);
        session.edits += 1;
        out["internal_only_canvas"] = data!(if wrote_canvas { "written, as the file had none" } else { "the file's own" });
        out["next"] = data!("create_variable with collection: this id or name, and a value for each mode");
        out["unsaved_edits"] = data!(session.edits);
        Ok(Output::Data(out))
    }
}

/// A variable in a collection, with a value for every mode.
pub struct CreateVariable {
    pub handle: String,
    pub collection: String,
    pub name: String,
    pub types: (VarType, VariableType),
    pub values: Data,
}

impl Command for CreateVariable {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let collection = find_collection(&session.graph, &self.collection)?.clone();
        let set = ours(session, collection.id, "collection")?;
        let (name, types) = (self.name.clone(), self.types);
        if let Some(taken) = name_taken(&session.graph, collection.id, &name, None) {
            return Err(VariableError::VariableNameTaken { collection: collection.name.clone(), name, id: taken }.into());
        }
        let values = values(session, &collection, types, &self.values, true, None)?;
        let written: Vec<(Guid, VarValue)> = values.iter().map(|(mode, write, _)| (*mode, write.clone())).collect();

        let id = session.graft().map_err(author_error)?.add_variable(set, &name, types.0, &written).map_err(author_error)?;
        session.index_appended();
        let variable = Variable {
            id,
            name,
            var_type: types.1,
            collection_id: collection.id,
            values_by_mode: values.into_iter().map(|(mode, _, read)| (mode, read)).collect(),
        };
        if let Some(c) = session.graph.collections.get_mut(&collection.id) {
            c.variable_ids.push(variable.id);
        }
        session.graph.variables.insert(variable.id, variable);
        session.created.push(id);
        session.edits += 1;
        let mut out = variable_json(&session.graph, &session.graph.variables[&id]);
        out["unsaved_edits"] = data!(session.edits);
        Ok(Output::Data(out))
    }
}

/// A variable grafta made, renamed or revalued in the modes named.
pub struct SetVariable {
    pub handle: String,
    pub id: String,
    pub name: Option<String>,
    pub values: Option<Data>,
}

impl Command for SetVariable {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let variable = find_variable(&session.graph, "id", &self.id)?.clone();
        let guid = ours(session, variable.id, "variable")?;
        let name = self.name.clone();
        if let Some(name) = &name {
            if let Some(taken) = name_taken(&session.graph, variable.collection_id, name, Some(variable.id)) {
                return Err(VariableError::RenameTaken { name: name.clone(), id: taken }.into());
            }
        }
        let collection = session
            .graph
            .collections
            .get(&variable.collection_id)
            .cloned()
            .ok_or(VariableError::NoCollection { id: variable.id })?;
        let types = (type_of(variable.var_type), variable.var_type);
        let values = match &self.values {
            Some(raw) => values(session, &collection, types, raw, false, Some(variable.id))?,
            None => Vec::new(),
        };
        let written: Vec<(Guid, VarValue)> = values.iter().map(|(mode, write, _)| (*mode, write.clone())).collect();

        {
            let mut graft = session.graft().map_err(author_error)?;
            if let Some(name) = &name {
                graft.rename(guid, name).map_err(author_error)?;
            }
            if !written.is_empty() {
                graft.set_variable_values(guid, types.0, &written).map_err(author_error)?;
            }
        }
        if let Some(stored) = session.graph.variables.get_mut(&variable.id) {
            if let Some(name) = name {
                stored.name = name;
            }
            for (mode, _, read) in values {
                stored.values_by_mode.insert(mode, read);
            }
        }
        // Figma draws the value each bound node stores (measured in Figma), so a
        // revalued variable is stored again wherever it reaches, aliases
        // included.
        let mut fitted = super::text::Fitted::default();
        if !written.is_empty() {
            let reached = super::binding::reached_by(session, &std::collections::HashSet::from([variable.id]));
            fitted = super::binding::restore(session, &reached)?;
        }
        session.edits += 1;
        let mut out = variable_json(&session.graph, &session.graph.variables[&variable.id]);
        out["unsaved_edits"] = data!(session.edits);
        super::text::report(&mut out, &fitted);
        Ok(Output::Data(out))
    }
}

/// A collection grafta made, renamed or given a mode.
pub struct SetVariableCollection {
    pub handle: String,
    pub id: String,
    pub name: Option<String>,
    pub add_mode: Option<String>,
}

impl Command for SetVariableCollection {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let collection = find_collection(&session.graph, &self.id)?.clone();
        let set = ours(session, collection.id, "collection")?;
        let (name, mode) = (self.name.clone(), self.add_mode.clone());
        if let Some(name) = &name {
            if let Some(taken) = session.graph.collections.values().find(|c| c.name == *name && c.id != collection.id) {
                return Err(VariableError::CollectionNameTaken { name: name.clone(), id: taken.id }.into());
            }
        }
        if let Some(mode) = &mode {
            if collection.modes.iter().any(|m| m.name == *mode) {
                return Err(VariableError::ModeTaken { collection: collection.name.clone(), mode: mode.clone() }.into());
            }
        }

        let added = {
            let mut graft = session.graft().map_err(author_error)?;
            if let Some(name) = &name {
                graft.rename(set, name).map_err(author_error)?;
            }
            match &mode {
                Some(mode) => Some(graft.add_mode(set, mode).map_err(author_error)?),
                None => None,
            }
        };
        let first = collection.modes.first().and_then(|m| m.id.declared());
        if let Some(stored) = session.graph.collections.get_mut(&collection.id) {
            if let Some(name) = name {
                stored.name = name;
            }
            if let (Some(id), Some(mode)) = (added, mode) {
                stored.modes.push(VariableMode { id: ModeId::Declared(id), name: mode });
            }
        }
        if let (Some(id), Some(first)) = (added, first) {
            for variable in session.graph.variables.values_mut().filter(|v| v.collection_id == collection.id) {
                if let Some(value) = variable.values_by_mode.get(&first).cloned() {
                    variable.values_by_mode.insert(id, value);
                }
            }
        }
        session.edits += 1;
        let mut out = collection_json(&session.graph.collections[&collection.id]);
        out["unsaved_edits"] = data!(session.edits);
        Ok(Output::Data(out))
    }
}
