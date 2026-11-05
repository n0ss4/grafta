//! Text a model writes.
//!
//! Figma stores what a text says (`textData`) apart from how it is laid out
//! (`derivedTextData`: glyphs, baselines, positions). grafta writes the first
//! and never the second: a spike opened in Figma on 2026-09-25 showed that
//! Figma lays text out itself when the layout data is absent, even for new
//! characters in a box sized for others.
//!
//! Every default is taken from measurement or from the file itself, never from
//! grafta's taste: the font is the file's most used one, and a line height
//! that is not given is the one the file uses at that size.

use std::collections::HashMap;

use indexmap::IndexMap;
use kiwi::Value as Raw;

use crate::data::{data, Data, Map};
use schema::{object, Field, Guid, LineType, NumberUnits, PaintType, SourceDirectionality, StrokeAlign, TextAlignHorizontal, TextAlignVertical, TextAutoResize, Wire, WireMut, WireName};

use super::rich_text::TEXT_ALIGNS_HORIZONTAL;
use super::style::paints;
use super::write::{author_error, created, record, require_parent, Placement};
use crate::Command;
use crate::view::num;
use super::style::invalid;
use crate::error::TextError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// A text layer on a page or in a frame.
pub struct CreateText {
    pub handle: String,
    pub parent: String,
    /// The text, when it is given as characters rather than runs.
    pub characters: String,
    pub family: Option<String>,
    pub style: Option<String>,
    pub font_size: Option<f32>,
    pub align: Option<TextAlignHorizontal>,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub at: (f32, f32),
    pub name: Option<String>,
    /// The call's arguments, for what is read against the file or is
    /// free-form: line height, letter spacing, fills, runs, paragraphs and
    /// the layer keys.
    pub asked: Map,
}

impl Command for CreateText {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let parent = require_parent(session, &self.parent, Placement::Text)?;
        let (given, asked) = (&self.characters, &self.asked);
        let fonts = FontCatalogue::of(session);
        let style = self.style.as_deref();
        let font = fonts.choose(self.family.as_deref(), style)?;
        let font_size = match self.font_size {
            Some(size) => size,
            None => fonts.size_of(&font),
        };
        let line_height = match asked.get("lineHeight") {
            Some(v) => dimension("lineHeight", v, true)?,
            None => fonts.line_height_at(font_size),
        };
        let letter_spacing = match asked.get("letterSpacing") {
            Some(v) => dimension("letterSpacing", v, false)?,
            // 19,254 measured layers carry 0 %.
            None => Dimension { value: 0.0, unit: NumberUnits::Percent },
        };
        let fills = match asked.get("fills") {
            Some(v) => paints(session, "fills", v)?,
            None => paints(session, "fills", &black())?,
        };
        let (align, width, height, at) = (self.align, self.width, self.height, self.at);

        let mut fields = kiwi::Fields::new();
        fields.set(Field::FontName, font.raw.clone());
        fields.set(Field::FontSize, Raw::Float(font_size));
        fields.set(Field::FontVersion, Raw::Str(String::new()));
        fields.set(Field::LineHeight, line_height.raw());
        fields.set(Field::LetterSpacing, letter_spacing.raw());
        fields.set(Field::TextAlignVertical, Raw::from(TextAlignVertical::Top));
        if let Some(align) = align {
            fields.set(Field::TextAlignHorizontal, align.into());
        }
        // A height fixes the box (`NONE`, which is stored as no field at
        // all: 5,172 measured layers); a width alone lets it grow down.
        match (width, height) {
            (_, Some(_)) => {}
            (Some(_), None) => {
                fields.set(Field::TextAutoResize, Raw::from(TextAutoResize::Height));
            }
            (None, None) => {
                fields.set(Field::TextAutoResize, Raw::from(TextAutoResize::WidthAndHeight));
            }
        }
        fields.set(Field::FillPaints, fills);
        // Measured on 27,744 text layers.
        fields.set(Field::StrokeWeight, Raw::Float(1.0));
        fields.set(Field::StrokeAlign, Raw::from(StrokeAlign::Outside));
        // The layer keys of rich text, after the defaults above so they win.
        let (mut layer_write, mut layer_remove) = (Vec::new(), Vec::new());
        super::rich_text::layer_fields(asked, &mut layer_write, &mut layer_remove)?;
        for (key, value) in layer_write {
            fields.set(key, value);
        }
        for key in layer_remove {
            fields.take(key);
        }

        // Runs differ from the layer as it now stands.
        let (characters, mut data, sizes) = match asked.get("runs") {
            Some(runs) => {
                let layer = super::rich_text::Layer::of(&Raw::Object(fields.clone()), &[]);
                let runs = super::rich_text::Runs::parse(session, runs, &layer)?;
                (runs.characters.clone(), runs.text_data(), runs.sizes)
            }
            None => (given.clone(), text_data(given), Vec::new()),
        };
        if let Some(paragraphs) = asked.get("paragraphs") {
            let lines = super::rich_text::lines(&characters, paragraphs)?;
            if let Raw::Object(d) = &mut data {
                d.set(Field::Lines, lines);
            }
        }
        fields.set(Field::TextData, data);
        let name = self.name.clone().unwrap_or_else(|| layer_name(&characters));
        let mut size = provisional_size(&characters, &sizes, font_size, line_height, width);
        if let Some(h) = height {
            size.1 = h;
        }

        let id = session
            .graft()
            .map_err(author_error)?
            .add_text_at(parent, &name, size, at, fields)
            .map_err(author_error)?;
        record(session, id)?;
        // Measured as render_node lays it out; the estimate above is
        // what a machine with no font keeps.
        let fitted = fit(session, &[id])?;

        let mut out = created(session, id);
        out["font"] = data!({ "family": font.family, "style": font.style, "size": num(font_size) });
        if let Some(asked) = style.filter(|asked| *asked != font.style) {
            out["font_respelled"] = data!({ "asked": asked, "written": font.style, "why": RESPELLED });
        }
        let (width, height) = session.graph.node(id).map_or(size, |n| (n.width, n.height));
        out["size"] = data!({
            "width": num(width),
            "height": num(height),
            "provisional": PROVISIONAL,
        });
        report(&mut out, &Fitted { resized: Vec::new(), ..fitted });
        report_faces(&mut out, session, &fonts_of(session, id));
        Ok(Output::Data(out))
    }
}

/// `set_properties` on a text layer: the text keys, written into the change
/// it is building. Returns what the change cleared that the model did not
/// name, for the result to say.
pub fn text_properties(
    session: &Session,
    id: Guid,
    properties: &Map,
    write: &mut Vec<(Field, Raw)>,
    remove: &mut Vec<Field>,
) -> Result<Vec<String>, ToolError> {
    use super::rich_text::{has_runs, layer_fields, lines, run_fields_of, strip_runs, Layer, Runs};

    let raw = session.raw(id).ok_or(DomainError::NoNodeChange { id })?;
    let mut relayout = false;
    let mut notes = Vec::new();
    if properties.contains_key("characters") && properties.contains_key("runs") {
        return Err(TextError::TwoWays.into());
    }
    if properties.contains_key("fontFamily") || properties.contains_key("fontStyle") {
        let current = |key: Field| raw.field(Field::FontName).and_then(|f| f.field(key)).and_then(Raw::as_str).map(str::to_string);
        let family = properties.get("fontFamily").and_then(Data::as_str).map(str::to_string).or_else(|| current(Field::Family));
        let style = properties.get("fontStyle").and_then(Data::as_str).map(str::to_string).or_else(|| current(Field::Style));
        let font = FontCatalogue::of(session).choose(family.as_deref(), style.as_deref())?;
        if let Some(asked) = style.as_deref().filter(|asked| *asked != font.style) {
            notes.push(format!("fontStyle {asked:?} written as {:?}: {RESPELLED}", font.style));
        }
        write.push((Field::FontName, font.raw));
        relayout = true;
    }
    if let Some(v) = properties.get("fontSize") {
        write.push((Field::FontSize, Raw::Float(positive("fontSize", v)?)));
        relayout = true;
    }
    if let Some(v) = properties.get("lineHeight") {
        write.push((Field::LineHeight, dimension("lineHeight", v, true)?.raw()));
        relayout = true;
    }
    if let Some(v) = properties.get("letterSpacing") {
        write.push((Field::LetterSpacing, dimension("letterSpacing", v, false)?.raw()));
        relayout = true;
    }
    if let Some(v) = properties.get("textAlignHorizontal") {
        let a = v.as_str().ok_or_else(|| invalid("textAlignHorizontal", "LEFT, CENTER, RIGHT or JUSTIFIED", v))?;
        write.push((Field::TextAlignHorizontal, alignment(a)?.into()));
        relayout = true;
    }
    if properties.contains_key("height") {
        // A height fixes the box: `NONE`, stored as no field.
        remove.push(Field::TextAutoResize);
        relayout = true;
    } else if properties.contains_key("width") {
        // A width fixes the box and lets the text wrap inside it.
        write.push((Field::TextAutoResize, Raw::from(TextAutoResize::Height)));
        relayout = true;
    }
    // The layer keys of 034. A textAutoResize named here wins over the one a
    // width or height implies.
    if properties.contains_key("textAutoResize") {
        write.retain(|(k, _)| *k != Field::TextAutoResize);
        remove.retain(|k| *k != Field::TextAutoResize);
    }
    let before = (write.len(), remove.len());
    layer_fields(properties, write, remove)?;
    relayout |= (write.len(), remove.len()) != before;

    // The text itself: new runs, or new characters, which give it one style
    // again (D9).
    let current = raw.field(Field::TextData).cloned().unwrap_or_else(|| text_data(""));
    let mut data = None;
    if let Some(v) = properties.get("runs") {
        let layer = Layer::of(raw, write);
        data = Some(Runs::parse(session, v, &layer)?.text_data());
    } else if let Some(v) = properties.get("characters") {
        let characters = v
            .as_str()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| invalid("characters", "non-empty text", v))?;
        if has_runs(&current) {
            notes.push("the text had runs styled apart; new characters take the layer's style throughout".to_string());
        }
        if current.field(Field::Lines).and_then(Raw::as_array).is_some_and(|l| l.iter().any(|l| !l.is(Field::LineType, LineType::Plain))) {
            notes.push("the text had lists; new characters are plain paragraphs unless paragraphs is given".to_string());
        }
        data = Some(text_data(characters));
    }
    // A style set on the whole layer applies to every character (D8).
    if !properties.contains_key("runs") {
        // Detaching a fill style changes no character's colour, so runs keep theirs.
        let fields: Vec<Field> = properties
            .iter()
            .filter(|(key, value)| !(key.as_str() == "fillStyle" && value.is_null()))
            .flat_map(|(key, _)| run_fields_of(key).iter().copied())
            .collect();
        if !fields.is_empty() && has_runs(data.as_ref().unwrap_or(&current)) {
            strip_runs(data.get_or_insert_with(|| current.clone()), &fields);
        }
    }
    if let Some(v) = properties.get("paragraphs") {
        let d = data.get_or_insert_with(|| current.clone());
        let characters = d.field(Field::Characters).and_then(Raw::as_str).unwrap_or_default().to_string();
        let lines = lines(&characters, v)?;
        if let Raw::Object(fields) = d {
            fields.set(Field::Lines, lines);
        }
    }
    if let Some(data) = data {
        write.push((Field::TextData, data));
        relayout = true;
    }
    if relayout {
        // Layout data for other characters or another font would be stale.
        remove.push(Field::DerivedTextData);
    }
    Ok(notes)
}

/// A font as the file stores it, or as Figma writes one it does not use yet;
/// for a text style.
pub fn font_name(session: &Session, family: Option<&str>, style: Option<&str>) -> Result<Raw, ToolError> {
    Ok(FontCatalogue::of(session).choose(family, style)?.raw)
}

/// A line height or letter spacing, in pixels or `{ value, unit }`.
pub fn dimension_raw(name: &str, value: &Data, positive_only: bool) -> Result<Raw, ToolError> {
    Ok(dimension(name, value, positive_only)?.raw())
}

pub fn font_size(value: &Data) -> Result<f32, ToolError> {
    positive("fontSize", value)
}

// ── Values ───────────────────────────────────────────────────────────────────

fn number(name: &str, value: &Data) -> Result<f32, ToolError> {
    value
        .as_f64()
        .filter(|n| n.is_finite())
        .map(|n| n as f32)
        .ok_or_else(|| invalid(name, "a number", value))
}

pub fn positive(name: &str, value: &Data) -> Result<f32, ToolError> {
    let n = number(name, value)?;
    if n > 0.0 {
        Ok(n)
    } else {
        Err(DomainError::NotPositive { at: name.into(), got: Some(value.to_string()) }.into())
    }
}

pub fn alignment(value: &str) -> Result<TextAlignHorizontal, ToolError> {
    TextAlignHorizontal::listed(value, &TEXT_ALIGNS_HORIZONTAL)
        .ok_or_else(|| invalid("textAlignHorizontal", "LEFT, CENTER, RIGHT or JUSTIFIED", value))
}

/// The units a dimension takes.
pub const UNITS: [NumberUnits; 2] = [NumberUnits::Pixels, NumberUnits::Percent];

#[derive(Clone, Copy)]
struct Dimension {
    value: f32,
    unit: NumberUnits,
}

impl Dimension {
    fn raw(self) -> Raw {
        object([
            (Field::Value, Raw::Float(self.value)),
            (Field::Units, self.unit.into()),
        ])
    }

    fn pixels(self, font_size: f32) -> f32 {
        if self.unit == NumberUnits::Percent {
            font_size * self.value / 100.0
        } else {
            self.value
        }
    }
}

/// A number is pixels; an object gives the unit.
fn dimension(name: &str, value: &Data, positive_only: bool) -> Result<Dimension, ToolError> {
    let (raw_value, unit) = match value {
        Data::Map(map) => {
            let unit = match map.get("unit").and_then(Data::as_str).and_then(|u| NumberUnits::listed(u, &UNITS)) {
                Some(unit) => unit,
                None => {
                    return Err(invalid(
                        &format!("{name}.unit"),
                        "PIXELS or PERCENT",
                        map.get("unit").map_or("nothing".to_string(), ToString::to_string),
                    ))
                }
            };
            (map.get("value").cloned().unwrap_or(Data::Null), unit)
        }
        other => (other.clone(), NumberUnits::Pixels),
    };
    let value = number(name, &raw_value)?;
    if positive_only && value <= 0.0 {
        return Err(DomainError::NotPositive { at: name.into(), got: Some(value.to_string()) }.into());
    }
    Ok(Dimension { value, unit })
}

/// One paragraph per line, in the six-field `PLAIN` shape every measured file
/// writes (35,221 of 35,221 layers).
pub fn text_data(characters: &str) -> Raw {
    let lines: Vec<Raw> = characters
        .split('\n')
        .map(|_| {
            object([
                (Field::LineType, Raw::from(LineType::Plain)),
                (Field::LineStyleId, Raw::Int(0)),
                (Field::IndentationLevel, Raw::Int(0)),
                (Field::SourceDirectionality, Raw::from(SourceDirectionality::Auto)),
                (Field::ListStartOffset, Raw::Int(0)),
                (Field::IsFirstLineOfList, Raw::Bool(false)),
            ])
        })
        .collect();
    object([
        (Field::Characters, Raw::Str(characters.to_string())),
        (Field::Lines, Raw::Array(lines)),
    ])
}

/// A first guess at the box, which Figma replaces when it lays the text out:
/// 0.6 em per character on the longest paragraph, and one line height per
/// paragraph. With a fixed width, the width is exact. A paragraph with runs
/// at several sizes is measured at its largest; `sizes` holds a
/// run's size per UTF-16 unit, `None` for the layer's.
fn provisional_size(characters: &str, sizes: &[Option<f32>], font_size: f32, line_height: Dimension, width: Option<f32>) -> (f32, f32) {
    let mut offset = 0;
    let paragraphs: Vec<(usize, f32)> = characters
        .split('\n')
        .map(|p| {
            let units = p.encode_utf16().count();
            let largest = sizes
                .get(offset..(offset + units).min(sizes.len()))
                .unwrap_or_default()
                .iter()
                .map(|s| s.unwrap_or(font_size))
                .fold(font_size, f32::max);
            offset += units + 1;
            (p.chars().count(), largest)
        })
        .collect();
    let line = |size: f32| line_height.pixels(size).max(1.0);
    match width {
        Some(w) => {
            let height: f32 = paragraphs
                .iter()
                .map(|&(chars, size)| (chars as f32 / (w / (size * 0.6)).max(1.0)).ceil().max(1.0) * line(size))
                .sum();
            (w, height.ceil())
        }
        None => {
            let natural = paragraphs.iter().map(|&(chars, size)| chars.max(1) as f32 * size * 0.6).fold(0.0, f32::max);
            (natural.ceil(), paragraphs.iter().map(|&(_, size)| line(size)).sum::<f32>().ceil())
        }
    }
}

/// Why a style was written in the file's spelling.
const RESPELLED: &str = "the file already uses this face, spelt as written; Figma reads a face only by its own spelling and reports any other missing";

/// What a text's reported size is.
pub const PROVISIONAL: &str = "measured by grafta as render_node draws it; Figma lays the text out and sets the real size when the file is opened";

/// Say what [`fit`] did, in a tool's result: how many texts it estimated
/// again, and the wrapping frames it left for Figma.
pub fn report(out: &mut Data, fitted: &Fitted) {
    if !fitted.resized.is_empty() {
        out["texts_resized"] = data!(fitted.resized.len());
    }
    if !fitted.stand_ins.is_empty() {
        let named: Vec<String> = fitted.stand_ins.iter().map(|(font, stand_in)| format!("{font} in {stand_in}")).collect();
        out["measured_in_stand_in"] = data!(format!(
            "{} not installed here, so measured in a stand-in, as render_node draws it: {}. Install the font for a size within about 2% of Figma's",
            if named.len() == 1 { "a font is" } else { "fonts are" },
            named.join(", ")
        ));
    }
    if fitted.unmeasured {
        out["estimated"] = data!("this machine has no fonts, so the size is 0.6 em per character, not measured");
    }
    if !fitted.left_to_figma.is_empty() {
        out["left_to_figma"] = data!({
            "frames": fitted.left_to_figma,
            "why": "a wrapping auto-layout frame, which grafta cannot lay out: Figma lays it out around the texts' new size when the file is opened",
        });
    }
}

/// The styles the file's texts use a family in, as Figma spelt them.
pub fn file_styles(session: &Session, family: &str) -> Vec<String> {
    FontCatalogue::of(session).styles_of(family)
}

/// The fonts a text now draws in: its layer's, and each run's that differs.
pub fn fonts_of(session: &Session, id: Guid) -> Vec<(String, String)> {
    let Some(node) = session.graph.node(id) else { return Vec::new() };
    let Some(layer) = &node.text_style else { return Vec::new() };
    let mut fonts = vec![(layer.font_family.clone(), layer.font_style.clone())];
    for run in &node.text_runs {
        let family = run.font_family.clone().unwrap_or_else(|| layer.font_family.clone());
        let style = run.font_style.clone().unwrap_or_else(|| layer.font_style.clone());
        if !fonts.contains(&(family.clone(), style.clone())) {
            fonts.push((family, style));
        }
    }
    fonts
}

/// Say which of these fonts name a style no installed face of their family
/// carries. Figma matches a face by its name and never makes one up, so
/// such a text opens with the font reported missing, unless Figma's machine
/// has the face. A family not installed here is not judged: the result
/// names the stand-in it was measured in.
pub fn report_faces(out: &mut Data, session: &Session, fonts: &[(String, String)]) {
    let file = FontCatalogue::of(session);
    let missing: Vec<Data> = fonts
        .iter()
        .filter_map(|(family, style)| {
            let installed = session.ports.raster.faces(family)?;
            // The text just written is in the file too; its own style proves
            // nothing about Figma's spelling.
            let in_file: Vec<String> = file.styles_of(family).into_iter().filter(|s| s != style).collect();
            let known = installed.iter().chain(&in_file).cloned().collect::<Vec<_>>();
            (!crate::font::is_face(&known, style)).then(|| {
                data!({ "family": family, "style": style, "in_this_file": in_file, "installed_here": installed })
            })
        })
        .collect();
    if !missing.is_empty() {
        out["font_faces"] = data!({
            "missing": missing,
            "why": "no face of the family has that style's name, in this file or installed on this machine; Figma matches a face by its own spelling and never synthesises one, so it may report the font missing. Use a style this file already uses; an installed one may be spelt otherwise by Figma's copy of the family (Inter's \"SemiBold\" here is \"Semi Bold\" in Figma, measured in 044)",
        });
    }
}

/// What [`fit`] did: the texts whose box changed, and the wrapping auto-layout
/// parents it left for Figma to lay out.
#[derive(Default)]
pub struct Fitted {
    pub resized: Vec<Guid>,
    pub left_to_figma: Vec<Guid>,
    /// Fonts not installed, and the stand-in each text was measured in.
    pub stand_ins: Vec<(String, String)>,
    /// Whether a text was estimated at 0.6 em, the machine having no font.
    pub unmeasured: bool,
}

impl Fitted {
    pub fn merge(&mut self, other: Fitted) {
        self.resized.extend(other.resized);
        for pair in other.stand_ins {
            if !self.stand_ins.contains(&pair) {
                self.stand_ins.push(pair);
            }
        }
        self.unmeasured |= other.unmeasured;
        for frame in other.left_to_figma {
            if !self.left_to_figma.contains(&frame) {
                self.left_to_figma.push(frame);
            }
        }
    }
}

/// Estimate each text's box again, as `create_text` estimates a new one, from
/// the text as the reader now sees it: a changed text style's values included,
/// which the nodes using it do not store. The estimate follows
/// `textAutoResize`; a fixed box is kept. A text whose box
/// changed is laid out in its auto-layout parent, as a resized child is (D6).
pub fn fit(session: &mut Session, ids: &[Guid]) -> Result<Fitted, ToolError> {
    let fonts = FontCatalogue::of(session);
    let mut sizes = Vec::new();
    let (mut stand_ins, mut unmeasured) = (Vec::new(), false);
    for &id in ids {
        let Some(node) = session.graph.node(id) else { continue };
        let (Some(text), Some(style)) = (&node.text, &node.text_style) else { continue };
        let width = match TextAutoResize::from_wire(&style.text_auto_resize) {
            Some(TextAutoResize::WidthAndHeight) => None,
            Some(TextAutoResize::Height) => Some(node.width),
            _ => continue,
        };
        let line_height = match style.line_height {
            Some(model::Dimension { value, unit: model::Unit::Pixels }) => Dimension { value, unit: NumberUnits::Pixels },
            Some(model::Dimension { value, unit: model::Unit::Percent }) => Dimension { value, unit: NumberUnits::Percent },
            Some(model::Dimension { value, unit: model::Unit::Raw }) => Dimension { value: value * 100.0, unit: NumberUnits::Percent },
            None => fonts.line_height_at(style.font_size),
        };
        let mut per_unit = vec![None; text.encode_utf16().count()];
        for run in &node.text_runs {
            if let Some(size) = run.font_size {
                let end = run.end.min(per_unit.len());
                per_unit.get_mut(run.start.min(end)..end).into_iter().flatten().for_each(|s| *s = Some(size));
            }
        }
        // As render_node lays it out, or 0.6 em per character without
        // any font.
        let size = match session.ports.raster.measure_text(&session.fig, &session.graph, node) {
            Some(measured) => {
                if let Some(stand_in) = measured.stand_in {
                    let pair = (style.font_family.clone(), stand_in);
                    if !stand_ins.contains(&pair) {
                        stand_ins.push(pair);
                    }
                }
                let round = |v: f32| (v * 100.0).round() / 100.0;
                (width.unwrap_or_else(|| round(measured.width)), round(measured.height))
            }
            None => {
                unmeasured = true;
                provisional_size(text, &per_unit, style.font_size, line_height, width)
            }
        };
        if (size.0 - node.width).abs() > 0.01 || (size.1 - node.height).abs() > 0.01 {
            sizes.push((id, size));
        }
    }

    let mut fitted = Fitted { stand_ins, unmeasured, ..Fitted::default() };
    for (id, (w, h)) in sizes {
        let Some(fields) = session.fields_mut(id) else { continue };
        fields.set(Field::Size, object([(Field::X, Raw::Float(w)), (Field::Y, Raw::Float(h))]));
        fitted.resized.push(id);
    }
    session.refresh(&fitted.resized);
    for &id in &fitted.resized {
        // Its component's instances cache the old box (D8).
        if let Some(component) = super::guard::enclosing_component(session, id) {
            session.stale_components.push(component);
        }
        let Some(parent) = super::guard::flowing_in(session, id) else { continue };
        if session.graph.node(parent).is_some_and(|p| p.layout.wrap) && !super::guard::laid_out_by_grafta(session, parent) {
            if !fitted.left_to_figma.contains(&parent) {
                fitted.left_to_figma.push(parent);
            }
        } else {
            super::guard::take_layout(session, parent)?;
        }
    }
    Ok(fitted)
}

/// A layer name from the text: its first line, cut short.
fn layer_name(characters: &str) -> String {
    let first = characters.lines().next().unwrap_or(characters);
    let mut name: String = first.chars().take(40).collect();
    if first.chars().count() > 40 {
        name.push('…');
    }
    name
}

// ── The file's own typography ────────────────────────────────────────────────

struct Font {
    family: String,
    style: String,
    raw: Raw,
}

/// The fonts the file's text layers use, and the sizes and line heights they
/// use them at.
struct FontCatalogue {
    /// (family, style) → (uses, the fontName exactly as stored, most used size).
    fonts: Vec<((String, String), usize, Raw)>,
    sizes: HashMap<(String, String), HashMap<u32, usize>>,
    /// font size (×10) → line height → uses.
    line_heights: HashMap<u32, HashMap<(u32, NumberUnits), usize>>,
}

impl FontCatalogue {
    fn of(session: &Session) -> Self {
        let mut fonts: IndexMap<(String, String), (usize, Raw)> = IndexMap::new();
        let mut sizes: HashMap<(String, String), HashMap<u32, usize>> = HashMap::new();
        let mut line_heights: HashMap<u32, HashMap<(u32, NumberUnits), usize>> = HashMap::new();
        for nc in model::author::node_changes(&session.fig) {
            if !nc.is(Field::Type, schema::NodeType::Text) || nc.has(Field::StyleType) {
                continue;
            }
            let Some(font) = nc.field(Field::FontName) else { continue };
            let key = (
                font.field(Field::Family).and_then(Raw::as_str).unwrap_or_default().to_string(),
                font.field(Field::Style).and_then(Raw::as_str).unwrap_or_default().to_string(),
            );
            fonts.entry(key.clone()).or_insert_with(|| (0, font.clone())).0 += 1;
            let size = nc.field(Field::FontSize).and_then(Raw::as_f32).unwrap_or(0.0);
            let tenth = (size * 10.0).round() as u32;
            *sizes.entry(key).or_default().entry(tenth).or_default() += 1;
            if let Some(lh) = nc.field(Field::LineHeight) {
                let Some(unit @ (NumberUnits::Pixels | NumberUnits::Percent)) = lh.known::<NumberUnits>(Field::Units) else { continue };
                let value = (lh.field(Field::Value).and_then(Raw::as_f32).unwrap_or(0.0) * 10.0).round() as u32;
                *line_heights.entry(tenth).or_default().entry((value, unit)).or_default() += 1;
            }
        }
        let mut fonts: Vec<((String, String), usize, Raw)> =
            fonts.into_iter().map(|(k, (n, raw))| (k, n, raw)).collect();
        fonts.sort_by_key(|font| std::cmp::Reverse(font.1));
        Self { fonts, sizes, line_heights }
    }

    /// The named font as the file stores it, or the file's most used one.
    fn choose(&self, family: Option<&str>, style: Option<&str>) -> Result<Font, ToolError> {
        let found = self.fonts.iter().find(|((f, s), _, _)| {
            family.is_none_or(|family| f == family) && style.is_none_or(|style| s == style)
        });
        // Figma reads a face only by its own spelling of it, which the file
        // holds for every face it uses: `Semi Bold` asked as `SemiBold` would
        // open as a missing font (a ladder of files opened in Figma).
        let found = found.or_else(|| {
            let (family, style) = (family?, style?);
            self.fonts.iter().find(|((f, s), _, _)| f == family && crate::font::is_face(std::slice::from_ref(s), style))
        });
        match (found, family) {
            (Some(((f, s), _, raw)), _) => Ok(Font { family: f.clone(), style: s.clone(), raw: raw.clone() }),
            // A font the file does not use yet: written as Figma writes 147
            // measured layers, without a PostScript name.
            (None, Some(family)) => {
                let style = style.unwrap_or("Regular");
                Ok(Font {
                    family: family.to_string(),
                    style: style.to_string(),
                    raw: object([
                        (Field::Family, Raw::Str(family.to_string())),
                        (Field::Style, Raw::Str(style.to_string())),
                        (Field::Postscript, Raw::Str(String::new())),
                    ]),
                })
            }
            (None, None) => Ok(Font {
                family: "Inter".into(),
                style: style.unwrap_or("Regular").into(),
                raw: object([
                    (Field::Family, Raw::Str("Inter".into())),
                    (Field::Style, Raw::Str(style.unwrap_or("Regular").into())),
                    (Field::Postscript, Raw::Str(String::new())),
                ]),
            }),
        }
    }

    /// The styles the file's texts use a family in, as Figma spelt them.
    fn styles_of(&self, family: &str) -> Vec<String> {
        let mut styles: Vec<String> = self.fonts.iter().filter(|((f, _), _, _)| f == family).map(|((_, s), _, _)| s.clone()).collect();
        styles.sort();
        styles.dedup();
        styles
    }

    /// The size the file most often uses this font at, or 16.
    fn size_of(&self, font: &Font) -> f32 {
        self.sizes
            .get(&(font.family.clone(), font.style.clone()))
            .and_then(|sizes| sizes.iter().max_by_key(|(_, n)| **n).map(|(size, _)| *size as f32 / 10.0))
            .filter(|size| *size > 0.0)
            .unwrap_or(16.0)
    }

    /// The line height the file most often uses at this size, or 100 %.
    fn line_height_at(&self, font_size: f32) -> Dimension {
        let tenth = (font_size * 10.0).round() as u32;
        self.line_heights
            .get(&tenth)
            .and_then(|lh| lh.iter().max_by_key(|(_, n)| **n).map(|(k, _)| *k))
            .map_or(Dimension { value: 100.0, unit: NumberUnits::Percent }, |(value, unit)| Dimension {
                value: value as f32 / 10.0,
                unit,
            })
    }
}

/// The text's default fill: black.
fn black() -> Data {
    Data::List(vec![Data::Map(Map::from([
        ("type".to_owned(), Data::from(PaintType::Solid.as_wire())),
        ("color".to_owned(), Data::from("#000000")),
    ]))])
}
