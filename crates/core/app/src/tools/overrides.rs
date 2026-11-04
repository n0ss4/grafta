//! Overriding what an instance shows: a colour, a label, a hidden part.
//!
//! An instance's layers are not in the file. Figma expands them from the
//! component when it opens it. What an instance changes about them is stored
//! on the instance, in `symbolData.symbolOverrides`, one entry per layer,
//! addressed by a `guidPath` of override keys, one step per nested instance.
//! Measured on 2026-09-25: 34,289 entries override a fill, the smallest shape
//! is `{ guidPath, fillPaints }` (1,122 of them), and a layer's key is its
//! `overrideKey`, or its own guid when it has none.
//!
//! So a model names a layer by its id in the component, as `read_node` on the
//! component shows it, and grafta turns that into the key path Figma uses.

use kiwi::Value as Raw;
use model::{NodeType, SceneNode};
use schema::{object, Field, Guid, Wire, WireMut};

use crate::data::{data, Data, Map};
use crate::Command;

use super::read::require_node;
use super::style::paints;
use super::text::text_data;
use super::style::invalid;
use crate::error::OverrideError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// "No style": Figma writes it beside an overridden fill when the component's
/// layer used a colour style, so that the style stops applying (9,642
/// measured entries).
const NO_STYLE: Guid = Guid::new(u32::MAX, u32::MAX);

/// What an instance shows, changed without changing its component.
pub struct OverrideInstance {
    pub handle: String,
    pub id: String,
    /// The component's layer, as its path of ids; none recolours every
    /// vector layer.
    pub layer: Option<Vec<String>>,
    /// Fills, strokes, characters, visible: no other key.
    pub properties: Map,
}

impl Command for OverrideInstance {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let instance = require_node(session, &self.id)?;
        let id = instance.id;
        if instance.node_type != NodeType::Instance {
            return Err(DomainError::NotAnInstance { id, is: crate::view::type_name(instance.node_type) }.into());
        }
        let component = instance.component_id;
        super::guard::editable(session, id)?;
        let component = component.ok_or(OverrideError::LayersUnknown { id })?;
        // An image named by path is read now and embedded once the call
        // succeeds, as set_properties does, so a template's rows can set images.
        let staged = super::images::stage(session, &self.properties)?;
        let properties = &staged;

        // The layers to override, each as its path of layer ids and of keys.
        let targets: Vec<Target> = match &self.layer {
            None => {
                if properties.contains_key("characters") || properties.contains_key("visible") {
                    return Err(OverrideError::LayerNeeded.into());
                }
                let all = vector_layers(session, component, &[]);
                if all.is_empty() {
                    return Err(OverrideError::NoVectorLayers { component }.into());
                }
                all
            }
            Some(path) => vec![resolve(session, component, path)?],
        };

        // Validate every value before writing any.
        let fills = properties.get("fills").map(|v| paints(session, "fills", v)).transpose()?;
        let strokes = properties.get("strokes").map(|v| paints(session, "strokes", v)).transpose()?;
        let characters = match properties.get("characters") {
            Some(v) => {
                let text = v.as_str().filter(|t| !t.is_empty()).ok_or_else(|| invalid("characters", "non-empty text", v))?;
                for target in &targets {
                    let layer = session.graph.node(target.layer());
                    if layer.is_none_or(|l| l.node_type != NodeType::Text) {
                        return Err(OverrideError::NotText { layer: target.shown() }.into());
                    }
                }
                Some(text_data(text))
            }
            None => None,
        };
        let visible = match properties.get("visible") {
            Some(Data::Bool(b)) => Some(*b),
            Some(other) => return Err(invalid("visible", "true or false", other)),
            None => None,
        };

        // Without a named layer this is a recolour: it changes the paint a layer
        // already has and never adds one. Painting a stroke onto a layer that had
        // none draws an outline nobody asked for, a square around an icon.
        let recolouring = self.layer.is_none();
        let mut entries = Vec::new();
        let mut touched = Vec::new();
        for target in &targets {
            let layer = session.graph.node(target.layer());
            let layer_raw = session.raw(target.layer());
            let has = |field: Field| layer_raw.is_some_and(|raw| raw.has(field));
            let mut fields: Vec<(Field, Raw)> = Vec::new();
            if let Some(f) = &fills {
                if !recolouring || layer.is_some_and(|l| !l.fills.is_empty()) {
                    fields.push((Field::FillPaints, f.clone()));
                    if has(Field::StyleIdForFill) {
                        fields.push((Field::StyleIdForFill, no_style()));
                    }
                }
            }
            if let Some(s) = &strokes {
                if !recolouring || layer.is_some_and(|l| !l.strokes.is_empty()) {
                    fields.push((Field::StrokePaints, s.clone()));
                    if has(Field::StyleIdForStrokeFill) {
                        fields.push((Field::StyleIdForStrokeFill, no_style()));
                    }
                }
            }
            if let Some(t) = &characters {
                fields.push((Field::TextData, t.clone()));
            }
            if let Some(v) = visible {
                fields.push((Field::Visible, Raw::Bool(v)));
            }
            if fields.is_empty() {
                continue;
            }
            touched.push(target.shown());
            entries.push((target.keys.clone(), fields));
        }
        if entries.is_empty() {
            let (asked, other) = if fills.is_some() { ("fills", "strokes") } else { ("strokes", "fills") };
            return Err(OverrideError::NothingToRecolour { component, asked, other }.into());
        }
        write_overrides(session, id, entries)?;
        super::images::commit(session);
        session.refresh(&[id]);
        session.edits += 1;

        let set: Vec<&str> = properties.keys().map(String::as_str).collect();
        Ok(Output::Data(data!({
            "id": id,
            "layers": touched,
            "set": set,
            "unsaved_edits": session.edits,
        })))
    }
}

struct Target {
    /// Layer ids: in the component, then in each nested instance's component.
    ids: Vec<Guid>,
    /// The override keys Figma addresses those layers by.
    keys: Vec<Guid>,
}

impl Target {
    fn layer(&self) -> Guid {
        *self.ids.last().expect("a path")
    }

    fn shown(&self) -> String {
        self.ids.iter().map(ToString::to_string).collect::<Vec<_>>().join(" › ")
    }
}

/// A layer's override key: its `overrideKey`, or its own guid.
pub fn key_of(session: &Session, id: Guid) -> Option<Guid> {
    let raw = session.raw(id)?;
    raw.field(Field::OverrideKey).and_then(Guid::read).or_else(|| raw.field(Field::Guid).and_then(Guid::read))
}

fn descends_from(session: &Session, node: &SceneNode, ancestor: Guid) -> bool {
    let mut current = node.parent_id;
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        current = session.graph.node(id).and_then(|n| n.parent_id);
    }
    false
}

/// Turn a path of layer ids into override keys, checking that each id is a
/// layer of the component the previous step instantiates.
fn resolve(session: &Session, component: Guid, path: &[String]) -> Result<Target, ToolError> {
    let mut within = component;
    let (mut ids, mut keys) = (Vec::new(), Vec::new());
    for (step, id) in path.iter().enumerate() {
        let layer = id.parse().ok().and_then(|id| session.graph.node(id)).filter(|l| descends_from(session, l, within)).ok_or_else(|| {
            let name = session.graph.node(within).map_or_else(|| within.to_string(), |c| format!("{} ({})", c.name, c.id));
            OverrideError::NotALayer { id: id.clone(), component: name }
        })?;
        ids.push(layer.id);
        keys.push(key_of(session, layer.id).ok_or(DomainError::NoGuid { id: layer.id })?);
        if step + 1 < path.len() {
            if layer.node_type != NodeType::Instance {
                return Err(OverrideError::NotNested { id: id.clone(), is: crate::view::type_name(layer.node_type) }.into());
            }
            within = layer.component_id.ok_or(DomainError::NoComponent { id: layer.id })?;
        }
    }
    Ok(Target { ids, keys })
}

/// Every vector layer of a component, through nested instances. Never a frame
/// or a rectangle: painting those would cover the icon.
fn vector_layers(session: &Session, component: Guid, prefix: &[(Guid, Guid)]) -> Vec<Target> {
    if prefix.len() > 8 {
        return Vec::new();
    }
    let mut ids = Vec::new();
    session.graph.walk(component, |node, depth| {
        if depth > 0 {
            ids.push(node.id);
        }
    });
    let mut out = Vec::new();
    for id in ids {
        let Some(node) = session.graph.node(id) else { continue };
        let Some(key) = key_of(session, id) else { continue };
        let mut path = prefix.to_vec();
        path.push((id, key));
        if node.node_type.is_vector_shape() || node.node_type == NodeType::Ellipse {
            out.push(Target { ids: path.iter().map(|(i, _)| *i).collect(), keys: path.iter().map(|(_, k)| *k).collect() });
        } else if node.node_type == NodeType::Instance {
            if let Some(nested) = node.component_id {
                out.extend(vector_layers(session, nested, &path));
            }
        }
    }
    out
}

/// A style pointer that points at no style, as Figma writes one.
pub fn no_style() -> Raw {
    object([(Field::Guid, NO_STYLE.to_value())])
}

/// Write each entry into the instance's `symbolOverrides`, merging into an
/// entry that already addresses the same layer.
#[allow(clippy::type_complexity)]
pub fn write_overrides(
    session: &mut Session,
    id: Guid,
    entries: Vec<(Vec<Guid>, Vec<(Field, Raw)>)>,
) -> Result<(), ToolError> {
    let position = *session.raw_index.get(&id).ok_or(DomainError::NoNodeChange { id })?;
    let Raw::Object(message) = &mut session.fig.node_changes else {
        return Err(DomainError::MessageNotAnObject.into());
    };
    let Some(Raw::Array(changes)) = message.field_mut(Field::NodeChanges) else {
        return Err(DomainError::NoNodeChanges.into());
    };
    let Some(raw) = changes.get_mut(position) else {
        return Err(DomainError::NodeChangeNotAnObject { id }.into());
    };
    session.undo.changing(raw);
    let Raw::Object(fields) = raw else {
        return Err(DomainError::NodeChangeNotAnObject { id }.into());
    };
    // The file's own instance may carry Figma's cached expansion, which would
    // show the old values. Figma expands an instance without it (A2).
    fields.take(Field::DerivedSymbolData);
    let symbol_data = fields.field_or_insert_with(Field::SymbolData, || Raw::Object(kiwi::Fields::new()));
    let Raw::Object(symbol_data) = symbol_data else {
        return Err(OverrideError::SymbolDataNotAnObject { id }.into());
    };
    let overrides = symbol_data.field_or_insert_with(Field::SymbolOverrides, || Raw::Array(Vec::new()));
    let Raw::Array(overrides) = overrides else {
        return Err(OverrideError::OverridesNotAList { id }.into());
    };

    for (keys, values) in entries {
        let guids = Raw::Array(keys.iter().map(|key| key.to_value()).collect());
        let path = object([(Field::Guids, guids)]);
        let existing = overrides.iter_mut().find(|entry| entry.field(Field::GuidPath) == Some(&path));
        let entry = match existing {
            Some(entry) => entry,
            None => {
                overrides.push(object([(Field::GuidPath, path)]));
                overrides.last_mut().expect("just pushed")
            }
        };
        if let Raw::Object(entry) = entry {
            for (key, value) in values {
                entry.set(key, value);
            }
        }
    }
    Ok(())
}

/// The overrides on an instance, for `read_node`: which layer each addresses,
/// and what it sets.
#[must_use]
pub fn describe(session_graph: &model::SceneGraph, instance: &SceneNode) -> Vec<Data> {
    let component = instance.component_id;
    let mut by_key: std::collections::HashMap<Guid, &SceneNode> = std::collections::HashMap::new();
    if let Some(component) = component {
        let mut ids = Vec::new();
        session_graph.walk(component, |node, _| ids.push(node.id));
        for &id in &ids {
            if let Some(node) = session_graph.node(id) {
                by_key.insert(node.override_key.unwrap_or(node.id), node);
            }
        }
    }
    // The entry addressed to the component itself holds the instance's own
    // values, which read_node shows on the instance.
    let own = component.and_then(|c| session_graph.node(c)).map(|c| c.override_key.unwrap_or(c.id));
    instance
        .symbol_overrides
        .iter()
        .filter(|ov| !ov.is_empty())
        .filter(|ov| !(ov.path.len() == 1 && Some(&ov.path[0]) == own.as_ref()))
        .map(|ov| {
            let mut entry = Map::new();
            match (ov.path.as_slice(), ov.path.first().and_then(|k| by_key.get(k))) {
                ([_], Some(layer)) => {
                    entry.insert("layer".into(), data!(layer.id));
                    entry.insert("layerName".into(), data!(layer.name));
                }
                _ => {
                    entry.insert("path".into(), data!(ov.path));
                }
            }
            if let Some(fills) = &ov.fills {
                entry.insert("fills".into(), crate::view::paints_json(session_graph, fills));
            }
            if let Some(strokes) = &ov.strokes {
                entry.insert("strokes".into(), crate::view::paints_json(session_graph, strokes));
            }
            if let Some(text) = &ov.text {
                entry.insert("characters".into(), data!(text));
            }
            if let Some(visible) = ov.visible {
                entry.insert("visible".into(), data!(visible));
            }
            Data::Map(entry)
        })
        .collect()
}
