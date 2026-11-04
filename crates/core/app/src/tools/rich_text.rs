//! Rich text in the plugin API's names: runs, lists and the
//! layer's paragraph, decoration, case and box fields.
//!
//! A run's style is an entry of `textData.styleOverrideTable` holding only what
//! differs from the layer, and its characters carry the entry's id in
//! `textData.characterStyleIDs`, one per UTF-16 unit, trailing zeros left out.
//! Lists are per paragraph, in `textData.lines`. That is how the fixtures store
//! all of it (1,098 styled layers), and Figma laid out every rung of a spike
//! written so, with no layout data.

use kiwi::{Fields, Value as Raw};
use crate::data::{Data, Map};
use model::{LIST_NONE, LIST_ORDERED, LIST_UNORDERED};
use schema::{object, Field, LineType, SourceDirectionality, TextAlignHorizontal, TextAlignVertical, TextAutoResize, TextCase, TextDecoration, Wire, WireMut, WireName};

use super::style::paints;
use super::text::{dimension_raw, font_name, positive, text_data};
use super::style::invalid;
use crate::error::RichTextError;
use crate::{DomainError, Session, ToolError};

/// The plugin API's kind of hyperlink grafta writes.
const URL: &str = "URL";

/// The values a text takes, in the plugin API's names, which are the format's.
pub const TEXT_ALIGNS_HORIZONTAL: [TextAlignHorizontal; 4] =
    [TextAlignHorizontal::Left, TextAlignHorizontal::Center, TextAlignHorizontal::Right, TextAlignHorizontal::Justified];
pub const TEXT_ALIGNS_VERTICAL: [TextAlignVertical; 3] = [TextAlignVertical::Top, TextAlignVertical::Center, TextAlignVertical::Bottom];
pub const TEXT_DECORATIONS: [TextDecoration; 3] = [TextDecoration::None, TextDecoration::Underline, TextDecoration::Strikethrough];
pub const TEXT_CASES: [TextCase; 6] =
    [TextCase::Original, TextCase::Upper, TextCase::Lower, TextCase::Title, TextCase::SmallCaps, TextCase::SmallCapsForced];
pub const TEXT_AUTO_RESIZES: [TextAutoResize; 3] = [TextAutoResize::None, TextAutoResize::WidthAndHeight, TextAutoResize::Height];


/// The keys a run takes besides its characters.
const RUN_KEYS: [&str; 9] = [
    "fontFamily",
    "fontStyle",
    "fontSize",
    "fills",
    "textDecoration",
    "textCase",
    "letterSpacing",
    "lineHeight",
    "hyperlink",
];

/// The layer's own style, which a run's entry differs from.
pub struct Layer {
    pub family: String,
    pub style: String,
    pub fields: Fields,
}

impl Layer {
    /// The style of a text as its node change stores it, with `write` laid
    /// over it: what the layer will be once the call is applied.
    pub fn of(raw: &Raw, write: &[(Field, Raw)]) -> Self {
        let mut fields = match raw {
            Raw::Object(f) => f.clone(),
            _ => Fields::new(),
        };
        for (key, value) in write {
            fields.set(*key, value.clone());
        }
        let font = fields.field(Field::FontName).cloned();
        let at = |key: Field| font.as_ref().and_then(|f| f.field(key)).and_then(Raw::as_str).unwrap_or_default().to_string();
        Self { family: at(Field::Family), style: at(Field::Style), fields }
    }

    fn same(&self, key: Field, value: &Raw) -> bool {
        self.fields.field(key) == Some(value)
    }

    fn decorated(&self) -> bool {
        self.fields.str(Field::TextDecoration).is_some_and(|d| d != TextDecoration::None.as_wire())
    }
}

/// A text given as runs: its characters, and its `characterStyleIDs` and
/// `styleOverrideTable` when any run differs from the layer.
pub struct Runs {
    pub characters: String,
    ids: Vec<u32>,
    table: Vec<Raw>,
    /// The font size of each UTF-16 unit, for the provisional size (D12).
    pub sizes: Vec<Option<f32>>,
}

impl Runs {
    /// Parse `runs` against the layer's own style.
    pub fn parse(session: &Session, value: &Data, layer: &Layer) -> Result<Self, ToolError> {
        let items = value
            .as_list()
            .filter(|a| !a.is_empty())
            .ok_or_else(|| invalid("runs", "a non-empty array of runs", value))?;
        let mut out = Runs { characters: String::new(), ids: Vec::new(), table: Vec::new(), sizes: Vec::new() };
        // Entries by their fields, so runs styled alike share one id.
        let mut entries: Vec<Vec<(Field, Raw)>> = Vec::new();
        for (index, run) in items.iter().enumerate() {
            let at = format!("runs[{index}]");
            let run = run
                .as_map()
                .ok_or_else(|| invalid(&at, "an object", run))?;
            if let Some(unknown) = run.keys().find(|k| *k != "characters" && !RUN_KEYS.contains(&k.as_str())) {
                return Err(RichTextError::NotARunKey { at, key: unknown.clone(), keys: &RUN_KEYS }.into());
            }
            let characters = run
                .get("characters")
                .and_then(Data::as_str)
                .filter(|c| !c.is_empty())
                .ok_or_else(|| DomainError::Unexpected { at: format!("{at}.characters"), expected: "non-empty text".into() })?;
            let fields = entry(session, &at, run, layer)?;
            let size = fields.iter().find(|(k, _)| *k == Field::FontSize).and_then(|(_, v)| v.as_f32());
            let id = if fields.is_empty() {
                0
            } else if let Some(found) = entries.iter().position(|e| *e == fields) {
                found as u32 + 1
            } else {
                entries.push(fields);
                entries.len() as u32
            };
            let units = characters.encode_utf16().count();
            out.ids.extend(std::iter::repeat_n(id, units));
            out.sizes.extend(std::iter::repeat_n(size, units));
            out.characters.push_str(characters);
        }
        out.table = entries
            .into_iter()
            .enumerate()
            .map(|(i, fields)| {
                let mut entry = Fields::new();
                entry.set(Field::StyleId, Raw::Uint(i as u32 + 1));
                for (key, value) in fields {
                    entry.set(key, value);
                }
                Raw::Object(entry)
            })
            .collect();
        Ok(out)
    }

    /// `textData` for these runs, every paragraph plain.
    pub fn text_data(&self) -> Raw {
        let mut data = text_data(&self.characters);
        set_styles(&mut data, self.ids.clone(), self.table.clone());
        data
    }
}

/// What a run changes, as the entry's fields, leaving out what equals the
/// layer's own (D3).
fn entry(session: &Session, at: &str, run: &Map, layer: &Layer) -> Result<Vec<(Field, Raw)>, ToolError> {
    let mut fields: Vec<(Field, Raw)> = Vec::new();
    let push = |fields: &mut Vec<(Field, Raw)>, key: Field, value: Raw| {
        if !layer.same(key, &value) {
            fields.push((key, value));
        }
    };
    let family = run.get("fontFamily").map(|v| text_of(at, "fontFamily", v)).transpose()?;
    let style = run.get("fontStyle").map(|v| text_of(at, "fontStyle", v)).transpose()?;
    if family.is_some() || style.is_some() {
        let family = family.unwrap_or(&layer.family);
        let style = style.unwrap_or(&layer.style);
        if family != layer.family || style != layer.style {
            // Every measured entry with a font carries its version (679 of 679),
            // even one equal to the layer's.
            fields.push((Field::FontName, font_name(session, Some(family), Some(style))?));
            fields.push((Field::FontVersion, Raw::Str(String::new())));
        }
    }
    if let Some(v) = run.get("fontSize") {
        push(&mut fields, Field::FontSize, Raw::Float(positive(&format!("{at}.fontSize"), v)?));
    }
    if let Some(v) = run.get("fills") {
        push(&mut fields, Field::FillPaints, paints(session, &format!("{at}.fills"), v)?);
    }
    let link = run.get("hyperlink").map(|v| hyperlink(at, v)).transpose()?;
    match run.get("textDecoration") {
        Some(v) => push(&mut fields, Field::TextDecoration, decoration(&format!("{at}.textDecoration"), v)?.into()),
        // A link is underlined unless the run says otherwise (115 of 194
        // decorated entries are underlines).
        None if link.is_some() && !layer.decorated() => push(&mut fields, Field::TextDecoration, Raw::from(TextDecoration::Underline)),
        None => {}
    }
    if let Some(v) = run.get("textCase") {
        push(&mut fields, Field::TextCase, case(&format!("{at}.textCase"), v)?.into());
    }
    if let Some(v) = run.get("letterSpacing") {
        push(&mut fields, Field::LetterSpacing, dimension_raw(&format!("{at}.letterSpacing"), v, false)?);
    }
    if let Some(v) = run.get("lineHeight") {
        push(&mut fields, Field::LineHeight, dimension_raw(&format!("{at}.lineHeight"), v, true)?);
    }
    if let Some(url) = link {
        push(&mut fields, Field::Hyperlink, object([(Field::Url, Raw::Str(url))]));
    }
    // A font version alone changes nothing.
    if fields.iter().all(|(k, _)| *k == Field::FontVersion) {
        fields.clear();
    }
    Ok(fields)
}

fn text_of<'a>(at: &str, key: &str, value: &'a Data) -> Result<&'a str, ToolError> {
    value.as_str().filter(|s| !s.is_empty()).ok_or_else(|| invalid(&format!("{at}.{key}"), "a name", value))
}

/// The plugin API's `{ type: "URL", value }`, to a web address (D4).
fn hyperlink(at: &str, value: &Data) -> Result<String, ToolError> {
    let bad = || invalid(&format!("{at}.hyperlink"), "{ \"type\": \"URL\", \"value\": \"https://…\" }", value);
    let link = value.as_map().ok_or_else(bad)?;
    if link.get("type").and_then(Data::as_str) != Some(URL) {
        return Err(bad());
    }
    let url = link.get("value").and_then(Data::as_str).ok_or_else(bad)?;
    let scheme = url.split_once(':').map(|(s, _)| s.to_ascii_lowercase());
    match scheme.as_deref() {
        Some("http" | "https") if url.contains("://") && url.len() > url.find("://").unwrap_or(0) + 3 => Ok(url.to_string()),
        Some("mailto") if url.len() > "mailto:".len() => Ok(url.to_string()),
        _ => Err(RichTextError::NotAWebAddress { at: at.into(), url: url.into() }.into()),
    }
}

/// `characterStyleIDs` and `styleOverrideTable` written into `textData`, the
/// ids cut short of their trailing zeros as Figma writes them (D2), and both
/// left out when nothing is styled apart.
fn set_styles(data: &mut Raw, mut ids: Vec<u32>, table: Vec<Raw>) {
    let Raw::Object(fields) = data else { return };
    while ids.last() == Some(&0) {
        ids.pop();
    }
    if ids.is_empty() || table.is_empty() {
        fields.take(Field::CharacterStyleIDs);
        fields.take(Field::StyleOverrideTable);
    } else {
        fields.set(Field::CharacterStyleIDs, Raw::Array(ids.into_iter().map(Raw::Uint).collect()));
        fields.set(Field::StyleOverrideTable, Raw::Array(table));
    }
}

/// The file's fields a layer-level key sets, which leave every run when the
/// key is set on the whole text (D8).
pub fn run_fields_of(key: &str) -> &'static [Field] {
    match key {
        "fontFamily" | "fontStyle" => &[Field::FontName, Field::FontVersion],
        "fontSize" => &[Field::FontSize],
        "fills" | "fillStyle" => &[Field::FillPaints, Field::StyleIdForFill],
        "textDecoration" => &[Field::TextDecoration],
        "textCase" => &[Field::TextCase],
        "letterSpacing" => &[Field::LetterSpacing],
        "lineHeight" => &[Field::LineHeight],
        _ => &[],
    }
}

/// Remove `fields` from every run of `data`. An entry left with nothing but
/// its id is dropped, and its characters take the layer's style.
pub fn strip_runs(data: &mut Raw, fields: &[Field]) {
    let ids: Vec<u32> = data
        .field(Field::CharacterStyleIDs)
        .and_then(Raw::as_array)
        .map(|a| a.iter().map(|v| v.as_u32().unwrap_or(0)).collect())
        .unwrap_or_default();
    let mut table: Vec<Raw> = data.field(Field::StyleOverrideTable).and_then(Raw::as_array).cloned().unwrap_or_default();
    if table.is_empty() {
        return;
    }
    let mut emptied = Vec::new();
    table.retain_mut(|entry| {
        let Raw::Object(e) = entry else { return true };
        for field in fields {
            e.take(*field);
        }
        let left = e.keys().any(|k| !matches!(Field::from_wire(k), Some(Field::StyleId | Field::FontVersion)));
        if !left {
            emptied.extend(e.field(Field::StyleId).and_then(Raw::as_u32));
        }
        left
    });
    let ids = ids.into_iter().map(|id| if emptied.contains(&id) { 0 } else { id }).collect();
    set_styles(data, ids, table);
}

/// Whether a text's `textData` styles any characters apart.
pub fn has_runs(data: &Raw) -> bool {
    data.field(Field::StyleOverrideTable).and_then(Raw::as_array).is_some_and(|t| !t.is_empty())
}

// ── Paragraphs ───────────────────────────────────────────────────────────────

/// `paragraphs` as `textData.lines`, one per paragraph of `characters` (D5).
pub fn lines(characters: &str, value: &Data) -> Result<Raw, ToolError> {
    let count = characters.split('\n').count();
    let items = value
        .as_list()
        .ok_or_else(|| invalid("paragraphs", "an array", value))?;
    if items.len() != count {
        return Err(RichTextError::ParagraphCount { count, given: items.len() }.into());
    }
    let mut parsed: Vec<(LineType, i32)> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let at = format!("paragraphs[{index}]");
        let item = item.as_map().ok_or_else(|| invalid(&at, "an object", item))?;
        if let Some(unknown) = item.keys().find(|k| !matches!(k.as_str(), "listType" | "indentation")) {
            return Err(RichTextError::NotAParagraphKey { at, key: unknown.clone() }.into());
        }
        let kind = match item.get("listType").and_then(Data::as_str) {
            None | Some(LIST_NONE) => LineType::Plain,
            Some(LIST_UNORDERED) => LineType::UnorderedList,
            Some(LIST_ORDERED) => LineType::OrderedList,
            Some(other) => {
                return Err(invalid(&format!("{at}.listType"), "NONE, UNORDERED or ORDERED", other))
            }
        };
        let level = match (kind, item.get("indentation")) {
            (LineType::Plain, None) => 0,
            (_, None) => 1,
            (LineType::Plain, Some(v)) if v.as_i64() == Some(0) => 0,
            (LineType::Plain, Some(v)) => {
                return Err(RichTextError::IndentedPlain { at, value: v.to_string() }.into())
            }
            (_, Some(v)) => match v.as_i64() {
                Some(level @ (1 | 2)) => level as i32,
                _ => return Err(invalid(&format!("{at}.indentation"), "1 or 2", v)),
            },
        };
        parsed.push((kind, level));
    }
    let lines = parsed
        .iter()
        .enumerate()
        .map(|(i, &(kind, level))| {
            // The first item of a list: the item before it at its level,
            // skipping deeper ones, is not of its type.
            let first = kind != LineType::Plain
                && parsed[..i]
                    .iter()
                    .rev()
                    .find(|(k, l)| *k == LineType::Plain || *l <= level)
                    .is_none_or(|&(k, l)| k != kind || l != level);
            line(kind, level, first)
        })
        .collect();
    Ok(Raw::Array(lines))
}

fn line(kind: LineType, level: i32, first: bool) -> Raw {
    object([
        (Field::LineType, kind.into()),
        (Field::LineStyleId, Raw::Int(0)),
        (Field::IndentationLevel, Raw::Int(level)),
        (Field::SourceDirectionality, Raw::from(SourceDirectionality::Auto)),
        (Field::ListStartOffset, Raw::Int(0)),
        (Field::IsFirstLineOfList, Raw::Bool(first)),
    ])
}

// ── The layer ────────────────────────────────────────────────────────────────

/// The layer keys as fields to write and fields to remove; a default removes
/// its field, as no file stores one (D6, D7).
pub fn layer_fields(
    properties: &Map,
    write: &mut Vec<(Field, Raw)>,
    remove: &mut Vec<Field>,
) -> Result<(), ToolError> {
    for key in ["paragraphSpacing", "paragraphIndent"] {
        let Some(v) = properties.get(key) else { continue };
        let n = v
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 0.0)
            .ok_or_else(|| invalid(key, "a number of pixels, 0 or more", v))?;
        let field = if key == "paragraphSpacing" { Field::ParagraphSpacing } else { Field::ParagraphIndent };
        if n == 0.0 {
            remove.push(field);
        } else {
            write.push((field, Raw::Float(n as f32)));
        }
    }
    if let Some(v) = properties.get("textDecoration") {
        match decoration("textDecoration", v)? {
            TextDecoration::None => remove.push(Field::TextDecoration),
            d => write.push((Field::TextDecoration, d.into())),
        }
    }
    if let Some(v) = properties.get("textCase") {
        match case("textCase", v)? {
            TextCase::Original => remove.push(Field::TextCase),
            c => write.push((Field::TextCase, c.into())),
        }
    }
    if let Some(v) = properties.get("textAlignVertical") {
        let align = v
            .as_str()
            .and_then(|a| TextAlignVertical::listed(a, &TEXT_ALIGNS_VERTICAL))
            .ok_or_else(|| invalid("textAlignVertical", "TOP, CENTER or BOTTOM", v))?;
        write.push((Field::TextAlignVertical, align.into()));
    }
    if let Some(v) = properties.get("textAutoResize") {
        match v.as_str().and_then(|r| TextAutoResize::listed(r, &TEXT_AUTO_RESIZES)) {
            Some(TextAutoResize::None) => remove.push(Field::TextAutoResize),
            Some(resize) => write.push((Field::TextAutoResize, resize.into())),
            None => {
                return Err(invalid("textAutoResize", "NONE, WIDTH_AND_HEIGHT or HEIGHT", v))
            }
        }
    }
    Ok(())
}

fn decoration(name: &str, value: &Data) -> Result<TextDecoration, ToolError> {
    match value.as_str().and_then(|d| TextDecoration::listed(d, &TEXT_DECORATIONS)) {
        Some(decoration) => Ok(decoration),
        None => Err(invalid(name, "NONE, UNDERLINE or STRIKETHROUGH", value)),
    }
}

fn case(name: &str, value: &Data) -> Result<TextCase, ToolError> {
    match value.as_str().and_then(|c| TextCase::listed(c, &TEXT_CASES)) {
        Some(case) => Ok(case),
        None => Err(invalid(name, "ORIGINAL, UPPER, LOWER, TITLE, SMALL_CAPS or SMALL_CAPS_FORCED", value)),
    }
}

