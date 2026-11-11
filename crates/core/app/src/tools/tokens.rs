//! A file's variables as W3C Design Tokens (DTCG 2025.10), one file per
//! collection and mode, the way Figma imports them: a mode per file.
//!
//! What the format cannot hold as Figma has it is written the way Figma reads
//! it back, and said: a name with a character the format forbids is changed
//! and listed, an alias to a variable the file does not hold is listed and not
//! written, and a string, which the format has no type for, is noted.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use model::{Color, SceneGraph, Variable, VariableCollection, VariableType, VariableValue};
use schema::Guid;

use crate::data::{data, Data, Map};
use crate::error::CatalogError;
use crate::port::Creating;
use crate::view::num;
use crate::{DomainError, Location, Output, Query, ToolError, Toolkit};

/// What every token file's name ends with.
const EXTENSION: &str = ".tokens.json";
const TYPE: &str = "$type";
const VALUE: &str = "$value";
const EXTENSIONS: &str = "$extensions";
/// A group's own token, when a token's name is also a group's.
const ROOT: &str = "$root";
const COLOR: &str = "color";
const NUMBER: &str = "number";
/// Not a type of the format; Figma reads it.
const STRING: &str = "string";
const SRGB: &str = "srgb";
/// Figma's extension keys, as its import reads them.
const FIGMA: &str = "com.figma";
const FIGMA_TYPE: &str = "com.figma.type";
const FIGMA_BOOLEAN: &str = "boolean";
const FIGMA_ALIAS: &str = "com.figma.aliasData";
/// What stands in for a character a name may not hold.
const REPLACEMENT: char = '_';
/// Characters a token's name may not hold (`.`, `{`, `}`), and one it may not
/// start with (`$`).
const FORBIDDEN: [char; 3] = ['.', '{', '}'];
const NO_LEADING: char = '$';
/// Characters a file name may not hold on some system grafta runs on.
const NOT_IN_FILE_NAMES: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
/// Line and paragraph separators, which Figma keeps in a name typed with
/// them and no file name should hold.
const LINE_BREAKS: [char; 2] = ['\u{2028}', '\u{2029}'];
const STRING_NOTE: &str = "STRING variables are written with $type \"string\", which the format does not define; Figma reads it";
const NO_VARIABLES: &str = "the file has no variables, so no token file was written";

/// The file's variables, written as token files into a folder.
pub struct ExportTokens {
    pub handle: String,
    pub dir: PathBuf,
    pub collection: Option<String>,
}

/// One token file, before it is written.
struct Planned {
    path: PathBuf,
    collection: String,
    mode: String,
    tokens: usize,
    doc: Data,
}

/// What the export says beside its files.
#[derive(Default)]
struct Said {
    unresolved: Vec<Data>,
    renamed: Vec<Data>,
    strings: bool,
}

impl Query for ExportTokens {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let graph = &session.graph;
        let wanted = self.collection.as_deref();
        let collections: Vec<&VariableCollection> =
            graph.collections.values().filter(|c| wanted.is_none_or(|w| c.name == w || c.id.to_string() == w)).collect();
        if let (Some(w), true) = (wanted, collections.is_empty()) {
            return Err(CatalogError::NoCollection { name: w.into() }.into());
        }

        let mut said = Said::default();
        let mut used = HashSet::new();
        let mut planned = Vec::new();
        for collection in collections.iter().filter(|c| !c.variable_ids.is_empty()) {
            planned.extend(plan(graph, collection, &self.dir, &mut used, &mut said));
        }

        let store = session.ports.store.as_ref();
        for file in &planned {
            Location::new_file(store, &file.path, None)?;
        }
        for file in &planned {
            let text = session.ports.json.write_pretty(&file.doc);
            store.create(&file.path, text.as_bytes()).map_err(|creating| -> ToolError {
                match creating {
                    Creating::Open(error) => CatalogError::Unwritable { path: file.path.clone(), error: error.to_string() }.into(),
                    Creating::Write(error) => DomainError::CannotWrite { path: file.path.clone(), error: error.to_string() }.into(),
                }
            })?;
        }

        let mut notes = Vec::new();
        if planned.is_empty() {
            notes.push(NO_VARIABLES);
        }
        if said.strings {
            notes.push(STRING_NOTE);
        }
        let files: Vec<Data> = planned
            .iter()
            .map(|f| data!({ "path": f.path.display().to_string(), "collection": f.collection, "mode": f.mode, "tokens": f.tokens }))
            .collect();
        Ok(Output::Data(data!({ "files": files, "unresolved": said.unresolved, "renamed": said.renamed, "notes": notes })))
    }
}

/// A collection's files, one per mode.
fn plan(graph: &SceneGraph, collection: &VariableCollection, dir: &Path, used: &mut HashSet<String>, said: &mut Said) -> Vec<Planned> {
    let variables: Vec<&Variable> = collection.variable_ids.iter().filter_map(|id| graph.variables.get(id)).collect();
    let mut paths: HashMap<Guid, Vec<String>> = HashMap::new();
    let mut taken = HashSet::new();
    for variable in &variables {
        let mut path = token_path(&variable.name, said);
        // Two names made one by replacing a character: the second keeps its id.
        if !taken.insert(path.clone()) {
            if let Some(last) = path.last_mut() {
                last.push_str(&format!(" ({})", variable.id));
            }
            taken.insert(path.clone());
            said.renamed.push(data!({ "from": variable.name, "to": path.join("/") }));
        }
        paths.insert(variable.id, path);
    }
    let roots: HashSet<Guid> = variables
        .iter()
        .filter(|v| {
            let own = &paths[&v.id];
            paths.values().any(|other| other.len() > own.len() && other.starts_with(own))
        })
        .map(|v| v.id)
        .collect();
    let reference = |id: Guid| {
        let mut parts = paths[&id].clone();
        if roots.contains(&id) {
            parts.push(ROOT.to_owned());
        }
        format!("{{{}}}", parts.join("."))
    };

    let mut files = Vec::new();
    for mode in &collection.modes {
        let mut doc = Map::new();
        let mut tokens = 0;
        for variable in &variables {
            let value = mode.id.declared().and_then(|m| variable.values_by_mode.get(&m)).or_else(|| variable.values_by_mode.first().map(|(_, v)| v));
            let Some(value) = value else { continue };
            let token = match value {
                VariableValue::Alias { variable_id } if paths.contains_key(variable_id) => {
                    Some(Map::from([(TYPE.to_owned(), data!(type_name(variable.var_type))), (VALUE.to_owned(), data!(reference(*variable_id)))]))
                }
                VariableValue::Alias { variable_id } => across(graph, *variable_id, said),
                concrete => concrete_token(concrete, said),
            };
            let Some(mut token) = token else {
                said.unresolved.push(data!({ "variable": variable.name, "mode": mode.name, "alias": alias_id(value) }));
                continue;
            };
            if let Some(original) = (paths[&variable.id].join("/") != variable.name).then(|| variable.name.clone()) {
                extend(&mut token, FIGMA, data!({ "name": original }));
            }
            insert(&mut doc, &paths[&variable.id], roots.contains(&variable.id), token);
            tokens += 1;
        }
        let wanted = format!("{}.{}", file_part(&collection.name), file_part(&mode.name));
        let name = if used.insert(wanted.clone()) {
            wanted.clone()
        } else {
            // Its id, with the colon a Windows file name cannot hold.
            let unique = format!("{} ({})", wanted, collection.id.to_string().replace(':', "-"));
            used.insert(unique.clone());
            unique
        };
        if name != format!("{}.{}", collection.name, mode.name) {
            said.renamed.push(data!({ "from": format!("{}.{}{EXTENSION}", collection.name, mode.name), "to": format!("{name}{EXTENSION}") }));
        }
        files.push(Planned {
            path: dir.join(format!("{name}{EXTENSION}")),
            collection: collection.name.clone(),
            mode: mode.name.clone(),
            tokens,
            doc: Data::Map(doc),
        });
    }
    files
}

/// A value of another collection: resolved in that collection's first mode,
/// with the name pair Figma links it back by. `None` when the file does not
/// hold it, or its chain ends nowhere.
fn across(graph: &SceneGraph, target: Guid, said: &mut Said) -> Option<Map> {
    let variable = graph.variables.get(&target)?;
    let collection = graph.collections.get(&variable.collection_id)?;
    let first = collection.modes.first().and_then(|m| m.id.declared());
    let concrete = model::variables::resolve_value(&graph.variables, variable, first)?;
    let mut token = concrete_token(concrete, said)?;
    extend(&mut token, FIGMA_ALIAS, data!({ "targetVariableName": variable.name, "targetVariableSetName": collection.name }));
    Some(token)
}

/// A value the file holds itself, as a token.
fn concrete_token(value: &VariableValue, said: &mut Said) -> Option<Map> {
    let (kind, value, figma_type) = match value {
        VariableValue::Color { value } => (COLOR, color(*value), None),
        VariableValue::Float { value } => (NUMBER, num(*value), None),
        VariableValue::String { value } => {
            said.strings = true;
            (STRING, data!(value), None)
        }
        VariableValue::Boolean { value } => (NUMBER, data!(u8::from(*value)), Some(FIGMA_BOOLEAN)),
        VariableValue::Alias { .. } => return None,
    };
    let mut token = Map::from([(TYPE.to_owned(), data!(kind)), (VALUE.to_owned(), value)]);
    if let Some(figma_type) = figma_type {
        extend(&mut token, FIGMA_TYPE, data!(figma_type));
    }
    Some(token)
}

/// A colour exactly as stored: its components, its alpha, and hex beside them.
fn color(color: Color) -> Data {
    data!({ "colorSpace": SRGB, "components": [num(color.r), num(color.g), num(color.b)], "alpha": num(color.a), "hex": color.to_hex() })
}

fn type_name(kind: VariableType) -> &'static str {
    match kind {
        VariableType::Color => COLOR,
        VariableType::Float | VariableType::Boolean => NUMBER,
        VariableType::String => STRING,
    }
}

fn alias_id(value: &VariableValue) -> String {
    match value {
        VariableValue::Alias { variable_id } => variable_id.to_string(),
        _ => String::new(),
    }
}

/// `key` in a token's `$extensions`.
fn extend(token: &mut Map, key: &str, value: Data) {
    let extensions = token.entry(EXTENSIONS.to_owned()).or_insert_with(|| Data::Map(Map::new()));
    if let Data::Map(map) = extensions {
        map.insert(key.to_owned(), value);
    }
}

/// A variable's name as groups and a token: split on `/`, each part with the
/// characters the format forbids replaced. A change is listed, once.
fn token_path(name: &str, said: &mut Said) -> Vec<String> {
    let parts: Vec<String> = name
        .split('/')
        .map(|part| {
            let mut fixed: String = part.chars().map(|c| if FORBIDDEN.contains(&c) { REPLACEMENT } else { c }).collect();
            if fixed.starts_with(NO_LEADING) {
                fixed.replace_range(..NO_LEADING.len_utf8(), &REPLACEMENT.to_string());
            }
            fixed
        })
        .collect();
    let written = parts.join("/");
    if written != name {
        said.renamed.push(data!({ "from": name, "to": written }));
    }
    parts
}

/// A collection's or mode's name, as part of a file name.
fn file_part(name: &str) -> String {
    name.chars().map(|c| if NOT_IN_FILE_NAMES.contains(&c) || c.is_control() || LINE_BREAKS.contains(&c) { REPLACEMENT } else { c }).collect()
}

/// Put `token` at `path` in `doc`, under `$root` when its name is a group's.
fn insert(doc: &mut Map, path: &[String], root: bool, token: Map) {
    let Some((last, groups)) = path.split_last() else { return };
    let mut at = doc;
    for group in groups {
        let entry = at.entry(group.clone()).or_insert_with(|| Data::Map(Map::new()));
        let Data::Map(next) = entry else { return };
        at = next;
    }
    if root {
        let entry = at.entry(last.clone()).or_insert_with(|| Data::Map(Map::new()));
        if let Data::Map(group) = entry {
            group.insert(ROOT.to_owned(), Data::Map(token));
        }
    } else {
        at.insert(last.clone(), Data::Map(token));
    }
}
