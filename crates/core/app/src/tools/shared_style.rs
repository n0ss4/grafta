//! Styles: listed, made, changed, and applied to nodes.
//!
//! A style is a hidden node with a `styleType` under the Internal Only Canvas,
//! and a node points at it with `styleIdForFill`, `styleIdForStrokeFill`,
//! `styleIdForEffect` or `styleIdForText` (5,056 local styles and 113,849
//! pointers measured). Figma draws a styled node with its style's values,
//! whatever the node stores (measured in Figma), and grafta's reader does
//! the same. So a node given a style stores its values, as Figma's own mostly
//! do, and a changed style only needs its nodes read again.

use kiwi::Value as Raw;
use model::author::{Provenance, StyleKind};
use model::{SceneGraph, SharedStyle, SharedStyleType};
use schema::{object, Field, Guid, NumberUnits, StyleType, Wire, WireMut, WireName};

use super::style::{closest, effects, paints};
use super::text::{dimension_raw, font_name, font_size};
use super::write::author_error;
use crate::data::{data, Data};
use crate::{Command, Query};
use crate::view::{effect, num, paints_json};
use super::style::invalid;
use crate::error::SharedStyleError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

// ── Finding ──────────────────────────────────────────────────────────────────

/// A style that is neither soft-deleted (3,836 of 5,056 are) nor a library's.
fn live(session: &Session, style: &SharedStyle) -> bool {
    session.raw(style.id).is_some_and(|raw| {
        raw.field(Field::IsSoftDeleted).and_then(Raw::as_bool) != Some(true) && raw.field(Field::SourceLibraryKey).is_none()
    })
}

fn kind_name(kind: SharedStyleType) -> &'static str {
    match kind {
        SharedStyleType::Fill => StyleType::Fill,
        SharedStyleType::Text => StyleType::Text,
        SharedStyleType::Effect => StyleType::Effect,
        SharedStyleType::Grid => StyleType::Grid,
    }
    .as_wire()
}

/// A live local style of `kind` by name or id, or why not.
fn find<'a>(session: &'a Session, at: &str, wanted: &str, kind: SharedStyleType) -> Result<&'a SharedStyle, ToolError> {
    let named: Vec<&SharedStyle> = session.graph.styles.values().filter(|s| s.id.to_string() == wanted || s.name == wanted).collect();
    let usable: Vec<&SharedStyle> = named.iter().copied().filter(|s| live(session, s)).collect();
    match usable.as_slice() {
        [one] if one.style_type == kind => Ok(one),
        [one] => Err(SharedStyleError::WrongKind {
            at: at.into(),
            name: one.name.clone(),
            is: kind_name(one.style_type),
            takes: kind_name(kind),
        }
        .into()),
        [] if !named.is_empty() => Err(SharedStyleError::LibraryOrRemoved { at: at.into(), wanted: wanted.into() }.into()),
        [] => {
            let names: Vec<&str> =
                session.graph.styles.values().filter(|s| s.style_type == kind && live(session, s)).map(|s| s.name.as_str()).collect();
            let near = closest(wanted, &names, 3);
            Err(SharedStyleError::NoSuchStyle {
                at: at.into(),
                kind: kind_name(kind),
                wanted: wanted.into(),
                closest: near.iter().map(ToString::to_string).collect(),
            }
            .into())
        }
        many => Err(SharedStyleError::AmbiguousStyle {
            at: at.into(),
            wanted: wanted.into(),
            ids: many.iter().map(|s| s.id.to_string()).collect(),
        }
        .into()),
    }
}

/// A text style's font, for [`super::text::report_faces`]; none for another kind.
fn style_font(style: &SharedStyle) -> Vec<(String, String)> {
    style.text_style.iter().map(|text| (text.font_family.clone(), text.font_style.clone())).collect()
}

fn style_json(graph: &SceneGraph, style: &SharedStyle) -> Data {
    let mut out = data!({ "id": style.id, "name": style.name, "type": kind_name(style.style_type) });
    match style.style_type {
        SharedStyleType::Fill => out["fills"] = paints_json(graph, &style.fills),
        SharedStyleType::Effect => out["effects"] = data!(style.effects.iter().map(effect).collect::<Vec<_>>()),
        SharedStyleType::Text => {
            if let Some(text) = &style.text_style {
                out["fontFamily"] = data!(text.font_family);
                out["fontStyle"] = data!(text.font_style);
                out["fontSize"] = num(text.font_size);
                if let Some(lh) = &text.line_height {
                    out["lineHeight"] = data!(lh);
                }
                if let Some(ls) = &text.letter_spacing {
                    out["letterSpacing"] = data!(ls);
                }
            }
        }
        SharedStyleType::Grid => {}
    }
    out
}

// ── Values ───────────────────────────────────────────────────────────────────

const TEXT_KEYS: [&str; 5] = ["fontFamily", "fontStyle", "fontSize", "lineHeight", "letterSpacing"];

/// The keys a style's values are given by, whatever its kind.
pub const VALUE_KEYS: [&str; 7] = ["fills", "effects", "fontFamily", "fontStyle", "fontSize", "lineHeight", "letterSpacing"];

/// A style's values, as a call gave them, as the fields its node carries.
/// `current` is the style's node when it is being changed, for a font given
/// by family or style alone.
fn values(session: &Session, kind: StyleKind, given: &Data, current: Option<&Raw>) -> Result<Vec<(Field, Raw)>, ToolError> {
    let creating = current.is_none();
    let allowed: &[&'static str] = match kind {
        StyleKind::Fill => &["fills"],
        StyleKind::Effect => &["effects"],
        StyleKind::Text => &TEXT_KEYS,
    };
    if let Some(wrong) = VALUE_KEYS.iter().find(|k| given.get(k).is_some() && !allowed.contains(k)) {
        return Err(SharedStyleError::NotAValue { key: wrong, allowed: allowed.to_vec() }.into());
    }
    let mut out = Vec::new();
    match kind {
        StyleKind::Fill => match given.get("fills") {
            Some(v) => out.push((Field::FillPaints, paints(session, "fills", v)?)),
            None if creating => return Err(SharedStyleError::NoPaints.into()),
            None => {}
        },
        StyleKind::Effect => match given.get("effects") {
            Some(v) => out.push((Field::Effects, effects(v)?)),
            None if creating => return Err(SharedStyleError::NoEffects.into()),
            None => {}
        },
        StyleKind::Text => {
            if given.get("fontFamily").is_some() || given.get("fontStyle").is_some() || creating {
                let now = |key: Field| current.and_then(|c| c.field(Field::FontName)).and_then(|f| f.field(key)).and_then(Raw::as_str).map(str::to_string);
                let family = given.get("fontFamily").and_then(Data::as_str).map(str::to_string).or_else(|| now(Field::Family));
                let style = given.get("fontStyle").and_then(Data::as_str).map(str::to_string).or_else(|| now(Field::Style));
                out.push((Field::FontName, font_name(session, family.as_deref(), style.as_deref())?));
            }
            match given.get("fontSize") {
                Some(v) => out.push((Field::FontSize, Raw::Float(font_size(v)?))),
                None if creating => return Err(SharedStyleError::NoSize.into()),
                None => {}
            }
            // By default the line height and letter spacing create_text writes.
            let dimension = |key: &str, positive: bool, default: (f32, NumberUnits)| -> Result<Option<Raw>, ToolError> {
                match given.get(key) {
                    Some(v) => Ok(Some(dimension_raw(key, v, positive)?)),
                    None if creating => Ok(Some(object([
                        (Field::Value, Raw::Float(default.0)),
                        (Field::Units, default.1.into()),
                    ]))),
                    None => Ok(None),
                }
            };
            if let Some(lh) = dimension("lineHeight", true, (100.0, NumberUnits::Percent))? {
                out.push((Field::LineHeight, lh));
            }
            if let Some(ls) = dimension("letterSpacing", false, (0.0, NumberUnits::Percent))? {
                out.push((Field::LetterSpacing, ls));
            }
        }
    }
    Ok(out)
}

// ── Applying ─────────────────────────────────────────────────────────────────

/// A style property: its name, the kind it takes, the pointer it writes, and
/// the node fields it copies from the style (from, to).
type Applied = (&'static str, SharedStyleType, Field, &'static [(Field, Field)]);

/// Fields to write and fields to remove.
type Writes = (Vec<(Field, Raw)>, Vec<Field>);

/// The style properties `set_properties` takes.
pub const APPLIED: [Applied; 4] = [
    ("fillStyle", SharedStyleType::Fill, Field::StyleIdForFill, &[(Field::FillPaints, Field::FillPaints)]),
    ("strokeStyle", SharedStyleType::Fill, Field::StyleIdForStrokeFill, &[(Field::FillPaints, Field::StrokePaints)]),
    ("effectStyle", SharedStyleType::Effect, Field::StyleIdForEffect, &[(Field::Effects, Field::Effects)]),
    (
        "textStyle",
        SharedStyleType::Text,
        Field::StyleIdForText,
        &[(Field::FontName, Field::FontName), (Field::FontSize, Field::FontSize), (Field::LineHeight, Field::LineHeight), (Field::LetterSpacing, Field::LetterSpacing)],
    ),
];

/// What each direct property detaches: setting it is not the style's any more.
const DETACHED_BY: [(&str, Field); 8] = [
    ("fills", Field::StyleIdForFill),
    ("strokes", Field::StyleIdForStrokeFill),
    ("effects", Field::StyleIdForEffect),
    ("fontFamily", Field::StyleIdForText),
    ("fontStyle", Field::StyleIdForText),
    ("fontSize", Field::StyleIdForText),
    ("lineHeight", Field::StyleIdForText),
    ("letterSpacing", Field::StyleIdForText),
];

/// The writes and removals for the style properties in `properties`, and the
/// pointers a direct value detaches.
pub fn apply(session: &Session, properties: &crate::data::Map) -> Result<Writes, ToolError> {
    let (mut write, mut remove) = (Vec::new(), Vec::new());
    for (property, kind, pointer, copies) in APPLIED {
        let Some(value) = properties.get(property) else { continue };
        if let Some((direct, _)) = DETACHED_BY.iter().find(|(direct, p)| *p == pointer && properties.contains_key(*direct)) {
            return Err(SharedStyleError::BothSet { property, direct }.into());
        }
        if value.is_null() {
            remove.push(pointer);
            continue;
        }
        let wanted = value.as_str().ok_or_else(|| invalid(property, "a style name or id, or null", value))?;
        let style = find(session, property, wanted, kind)?;
        let raw = session.raw(style.id).ok_or(DomainError::NoNodeChange { id: style.id })?;
        write.push((pointer, object([(Field::Guid, style.id.to_value())])));
        for (from, to) in copies {
            if let Some(v) = raw.field(*from) {
                write.push((*to, v.clone()));
            }
        }
        if kind == SharedStyleType::Text {
            // Layout data for another font or size would be stale.
            remove.push(Field::DerivedTextData);
        }
    }
    for (direct, pointer) in DETACHED_BY {
        if properties.contains_key(direct) && !write.iter().any(|(k, _)| *k == pointer) {
            remove.push(pointer);
        }
    }
    Ok((write, remove))
}

// ── The tools ────────────────────────────────────────────────────────────────

/// The file's live local styles, optionally of one kind.
pub struct ListStyles {
    pub handle: String,
    pub kind: Option<SharedStyleType>,
}

impl Query for ListStyles {
    fn run(&self, kit: &Toolkit) -> Result<Output, ToolError> {
        let session = kit.session(&self.handle)?;
        let wanted = self.kind;
        let styles: Vec<Data> = session
            .graph
            .styles
            .values()
            .filter(|s| s.style_type != SharedStyleType::Grid && wanted.is_none_or(|w| s.style_type == w) && live(session, s))
            .map(|s| style_json(&session.graph, s))
            .collect();
        Ok(Output::Data(data!({ "total": styles.len(), "styles": styles })))
    }
}


/// A colour, text or effect style.
pub struct CreateStyle {
    pub handle: String,
    pub name: String,
    pub kind: StyleKind,
    pub style_type: SharedStyleType,
    /// Its values, by the keys of [`VALUE_KEYS`] the call gave.
    pub values: Data,
}

impl Command for CreateStyle {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let (name, kind, style_type) = (self.name.clone(), self.kind, self.style_type);
        if let Some(taken) = session.graph.styles.values().find(|s| s.name == name && s.style_type == style_type && live(session, s)) {
            return Err(SharedStyleError::NameTaken { kind: kind_name(style_type), name, id: taken.id }.into());
        }
        let written = values(session, kind, &self.values, None)?;
        let ((canvas, wrote_canvas), id) = {
            let mut graft = session.graft().map_err(author_error)?;
            let canvas = graft.internal_canvas().map_err(author_error)?;
            (canvas, graft.add_style(kind, &name, written).map_err(author_error)?)
        };
        if wrote_canvas {
            session.insert_appended(&[canvas]).map_err(|e| DomainError::CanvasNotReadBack { error: e.to_string() })?;
            session.created.push(canvas);
        }
        session.insert_styles(&[id]);
        session.created.push(id);
        session.edits += 1;
        let style = session.graph.styles.get(&id).ok_or(SharedStyleError::MadeNotReadBack { id })?;
        let mut out = style_json(&session.graph, style);
        out["unsaved_edits"] = data!(session.edits);
        super::text::report_faces(&mut out, session, &style_font(style));
        Ok(Output::Data(out))
    }
}

/// A style renamed or given new values: grafta's own, or one the file already
/// has, which takes a new version as Figma's own edit would.
pub struct SetStyle {
    pub handle: String,
    pub id: String,
    pub name: Option<String>,
    /// Its values, by the keys of [`VALUE_KEYS`] the call gave.
    pub values: Data,
}

impl Command for SetStyle {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let wanted = self.id.as_str();
        let matches: Vec<SharedStyle> = session
            .graph
            .styles
            .values()
            .filter(|s| (s.id.to_string() == wanted || s.name == wanted) && live(session, s))
            .cloned()
            .collect();
        let style = match matches.as_slice() {
            [one] => one.clone(),
            [] => return Err(SharedStyleError::NoStyleWithId { wanted: wanted.into() }.into()),
            many => {
                return Err(SharedStyleError::AmbiguousStyle {
                    at: "id".into(),
                    wanted: wanted.into(),
                    ids: many.iter().map(|s| s.id.to_string()).collect(),
                }
                .into())
            }
        };
        let guid = style.id;
        let kind = match style.style_type {
            SharedStyleType::Fill => StyleKind::Fill,
            SharedStyleType::Text => StyleKind::Text,
            SharedStyleType::Effect => StyleKind::Effect,
            SharedStyleType::Grid => return Err(SharedStyleError::GridChanged.into()),
        };
        let name = self.name.clone();
        let current = session.raw(style.id).cloned();
        let mut written = values(session, kind, &self.values, current.as_ref())?;
        if let Some(name) = &name {
            written.push((Field::Name, Raw::Str(name.clone())));
        }
        if written.is_empty() {
            return Err(DomainError::NothingToChange { pass: "name or the style's values" }.into());
        }
        let restyled = matches!(kind, StyleKind::Text) && written.iter().any(|(k, _)| *k != Field::Name);
        if guid.is_graftas() {
            session.graft().map_err(author_error)?.set_fields(guid, written).map_err(author_error)?;
        } else {
            // A style Figma made carries the version a library compares to know
            // it changed (`"7524:66"` on the Untitled UI kit's): Figma gives it a
            // new one on every edit, and so does grafta. Its `key` stays.
            let version = Raw::Str(session.graft().map_err(author_error)?.allocate_id().to_string());
            let has = |field| current.as_ref().is_some_and(|raw| raw.field(field).is_some());
            for field in [Field::Version, Field::UserFacingVersion] {
                if has(field) {
                    written.push((field, version.clone()));
                }
            }
            let fields = session.fields_mut(guid).ok_or(SharedStyleError::ChangedNotReadBack { id: guid })?;
            for (key, value) in written {
                fields.set(key, value);
            }
        }
        // A text style keeps its sample's layout, stale once its font changes.
        if restyled {
            if let Some(fields) = session.fields_mut(guid) {
                fields.take(Field::DerivedTextData);
            }
        }
        session.insert_styles(&[style.id]);

        // Figma draws a styled node with its style's values (s2), and so does
        // the reader: the nodes pointing at it are only read again.
        let pointers: Vec<Field> = APPLIED.iter().map(|(_, _, pointer, _)| *pointer).collect();
        let users: Vec<Guid> = session
            .graph
            .nodes
            .keys()
            .copied()
            .filter(|id| {
                session.raw(*id).is_some_and(|raw| pointers.iter().any(|p| raw.field(*p).and_then(|v| v.field(Field::Guid)).and_then(Guid::read) == Some(style.id)))
            })
            .collect();
        // A text style's new values change the room its texts take: their
        // layout data is stale, and their boxes are estimated again.
        let texts: Vec<Guid> = if restyled {
            users.iter().copied().filter(|u| session.raw(*u).is_some_and(|r| r.is(Field::Type, schema::NodeType::Text))).collect()
        } else {
            Vec::new()
        };
        for &text in &texts {
            if let Some(fields) = session.fields_mut(text) {
                fields.take(Field::DerivedTextData);
            }
        }
        // An instance whose override points at the style takes its values too.
        let overriding: Vec<Guid> = session
            .graph
            .nodes
            .values()
            .filter(|n| n.node_type == model::NodeType::Instance)
            .filter(|n| {
                session.raw(n.id).and_then(|raw| raw.field(Field::SymbolData)).and_then(|d| d.field(Field::SymbolOverrides)).and_then(Raw::as_array).is_some_and(|entries| {
                    entries.iter().any(|entry| pointers.iter().any(|p| entry.field(*p).and_then(|v| v.field(Field::Guid)).and_then(Guid::read) == Some(style.id)))
                })
            })
            .map(|n| n.id)
            .collect();
        session.refresh(&users);
        session.refresh(&overriding);
        // An instance's cached expansion holds its texts' layout and its
        // layers' transforms, not their paints (measured on the Untitled UI
        // kit): a new colour or effect leaves it true, and keeping it
        // keeps the layout Figma computed. A text style's new font or size
        // does not: then every instance holding one of its texts is derived
        // again, as an edit inside a component does.
        if restyled {
            let holders: Vec<Guid> = users
                .iter()
                .filter_map(|&user| super::guard::enclosing_component(session, user))
                .chain(overriding.iter().filter_map(|&i| session.graph.node(i).and_then(|n| n.component_id)))
                .collect();
            session.stale_components.extend(holders);
        }
        let fitted = super::text::fit(session, &texts)?;
        session.edits += 1;
        let changed = session.graph.styles.get(&style.id).ok_or(SharedStyleError::ChangedNotReadBack { id: style.id })?;
        let mut out = style_json(&session.graph, changed);
        out["nodes_using_it"] = data!(users.len());
        out["unsaved_edits"] = data!(session.edits);
        super::text::report(&mut out, &fitted);
        super::text::report_faces(&mut out, session, &style_font(changed));
        Ok(Output::Data(out))
    }
}
