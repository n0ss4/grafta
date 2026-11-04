//! A web page's layout written as design layers.
//!
//! grafta runs no browser. `scripts/snapshot.js`, run in one, writes what it
//! laid out: each element's box and computed style, each text's lines and
//! font, and what will not map. This reads that snapshot and writes it with the
//! other tools' own implementations, as `fill_template` does, so an
//! import is one call and one journal entry.
//!
//! The spike measured both ways against
//! Chrome's own picture: boxes where the browser put them differ in 0.84% of
//! pixels, flexbox as auto-layout in 2.30%, with sizing read from what the
//! browser measured by three rules (see [`sizing`]).

use std::path::PathBuf;

use schema::{EffectType, ImageScaleMode, PaintType, StackAlign, StackJustify, StackMode, StackSize, StrokeAlign, TextAlignHorizontal, TextCase, TextDecoration, WireName};

use super::sizing::{FILL, HUG};
use super::read::require_node;
use crate::data::{data, Data, Map};
use crate::Command;
use crate::error::ImportError;
use crate::port::Ports;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// The snapshot format this reads.
const VERSION: &str = "layout-snapshot/1";
/// The most nodes one import writes.
const MAX_NODES: u64 = 5000;

/// A web page's layout snapshot, written as design layers.
pub struct ImportLayout {
    pub handle: String,
    pub parent: String,
    /// The snapshot's JSON file.
    pub snapshot: String,
    /// Flexbox as auto-layout, or every box where the browser put it.
    pub auto: bool,
    pub at: (f32, f32),
}

impl Command for ImportLayout {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let (handle, parent, path, auto) = (self.handle.clone(), &self.parent, self.snapshot.as_str(), self.auto);
        let store = kit.ports().store.clone();
        let text = store
            .read_text(std::path::Path::new(path))
            .map_err(|error| DomainError::CannotRead { path: path.into(), error: error.to_string() })?;
        let snapshot = kit.ports().json.parse(&text).map_err(|error| ImportError::NotJson { path: path.into(), error })?;
        match snapshot.get("grafta").and_then(Data::as_str) {
            Some(VERSION) => {}
            other => {
                return Err(ImportError::NotASnapshot {
                    path: path.into(),
                    version: VERSION,
                    says: other.unwrap_or("nothing").into(),
                }
                .into())
            }
        }
        if snapshot.get("nodes").and_then(Data::as_u64).unwrap_or(0) > MAX_NODES {
            return Err(ImportError::TooManyNodes { path: path.into(), max: MAX_NODES }.into());
        }
        let root = snapshot.get("root").filter(|r| r.is_map()).ok_or_else(|| ImportError::NoRoot { path: path.into() })?;
        require_node(kit.session(&handle)?, parent)?;

        let at = (f64::from(self.at.0), f64::from(self.at.1));
        let mut import = Import {
            handle: handle.clone(),
            auto,
            not_imported: Vec::new(),
            fonts: Vec::new(),
            frames: 0,
            texts: 0,
            images: 0,
            scratch: Vec::new(),
            ports: kit.ports().clone(),
        };
        let result = import.element(kit, root, parent, (box_of(root).0 - at.0, box_of(root).1 - at.1), None, None, "");
        for file in &import.scratch {
            let _ = import.ports.store.remove(file);
        }
        let id = result?;
        let session = kit.session(&handle)?;
        let mut out = data!({
            "id": id,
            "layout": if auto { "auto" } else { "absolute" },
            "frames": import.frames,
            "texts": import.texts,
            "images": import.images,
            "from": snapshot.get("url"),
            "unsaved_edits": session.edits,
        });
        if !import.not_imported.is_empty() {
            out["not_imported"] = data!(import.not_imported);
        }
        if !import.fonts.is_empty() {
            out["fonts"] = data!(import.fonts);
        }
        Ok(Output::Data(out))
    }
}

struct Import {
    handle: String,
    auto: bool,
    not_imported: Vec<Data>,
    fonts: Vec<String>,
    frames: usize,
    texts: usize,
    images: usize,
    /// Images decoded from `data:` URLs, removed after the call.
    scratch: Vec<PathBuf>,
    /// Where scratch images are written and paths looked at.
    ports: Ports,
}

impl Import {
    /// Write an element and what is in it, under `parent`, `origin` being the
    /// parent's box in the snapshot's space. Returns the frame's id.
    #[allow(clippy::too_many_arguments)]
    fn element(&mut self, kit: &mut Toolkit, node: &Data, parent: &str, origin: (f64, f64), parent_node: Option<&Data>, parent_flex: Option<&str>, path: &str) -> Result<String, ToolError> {
        let (x, y, w, h) = box_of(node);
        let name = node.get("name").and_then(Data::as_str).unwrap_or("div");
        let path = if path.is_empty() { name.to_string() } else { format!("{path} / {name}") };
        let create = super::write::CreateFrame {
            handle: self.handle.clone(),
            parent: parent.into(),
            name: non_empty("name", name)?,
            width: round(w.max(0.01)) as f32,
            height: round(h.max(0.01)) as f32,
            at: (round(x - origin.0) as f32, round(y - origin.1) as f32),
        };
        let id = id_of(create.run(kit)?)?;
        self.frames += 1;

        let mut props = Map::new();
        let mut fills = Vec::new();
        if let Some(fill) = node.get("background").and_then(Data::as_str).and_then(paint) {
            fills.push(fill);
        }
        if let Some(src) = node.get("image").and_then(Data::as_str) {
            match self.image(src) {
                Some(file) => {
                    fills.push(data!({ "type": PaintType::Image.as_wire(), "image": file, "scaleMode": ImageScaleMode::Fill.as_wire() }));
                    self.images += 1;
                }
                None => self.not_imported.push(data!({ "element": path, "what": format!("an image from {}", short(src)) })),
            }
        }
        props.insert("fills".into(), data!(fills));
        let radius: Vec<f64> = node.get("radius").and_then(Data::as_list).map(|r| r.iter().filter_map(Data::as_f64).collect()).unwrap_or_default();
        if radius.len() == 4 && radius.iter().any(|r| *r > 0.0) {
            if radius.iter().all(|r| (r - radius[0]).abs() < 0.01) {
                props.insert("cornerRadius".into(), data!(radius[0]));
            } else {
                for (key, r) in ["topLeftRadius", "topRightRadius", "bottomRightRadius", "bottomLeftRadius"].iter().zip(&radius) {
                    props.insert((*key).into(), data!(r));
                }
            }
        }
        if let Some(border) = node.get("border").filter(|b| b.is_map()) {
            if let Some(stroke) = border.get("colour").and_then(Data::as_str).and_then(paint) {
                props.insert("strokes".into(), data!([stroke]));
                props.insert("strokeWeight".into(), data!(border.get("width").and_then(Data::as_f64).unwrap_or(1.0)));
                props.insert("strokeAlign".into(), data!(StrokeAlign::Inside.as_wire()));
            }
        }
        if let Some(shadow) = node.get("shadow").and_then(Data::as_str) {
            let effects = shadows(shadow);
            if effects.is_empty() {
                self.not_imported.push(data!({ "element": path, "what": format!("box-shadow: {}", short(shadow)) }));
            } else {
                props.insert("effects".into(), data!(effects));
            }
        }
        if let Some(opacity) = node.get("opacity").and_then(Data::as_f64).filter(|o| *o < 1.0) {
            props.insert("opacity".into(), data!(opacity));
        }
        for what in node.get("unsupported").and_then(Data::as_list).into_iter().flatten().filter_map(Data::as_str) {
            self.not_imported.push(data!({ "element": path, "what": what }));
        }

        let children: Vec<&Data> = node.get("children").and_then(Data::as_list).map(|c| c.iter().collect()).unwrap_or_default();
        let layout = node.get("layout").filter(|l| l.is_map());
        let own_flex = self.auto
            && layout.is_some_and(|l| !l.get("wrap").and_then(Data::as_bool).unwrap_or(false))
            && children.iter().all(|c| c.get("position").and_then(Data::as_str) != Some("absolute"));
        let direction = layout.and_then(|l| l.get("direction")).and_then(Data::as_str).map(|d| if d.starts_with("column") { "column" } else { "row" });
        if own_flex {
            let l = layout.expect("checked above");
            let pad = padding(l);
            let column = direction == Some("column");
            let gap = if column { num(l, "gap") } else { Some(num(l, "columnGap")).filter(|g| *g > 0.0).unwrap_or_else(|| num(l, "gap")) };
            props.insert("layoutMode".into(), data!(if column { StackMode::Vertical } else { StackMode::Horizontal }.as_wire()));
            props.insert("itemSpacing".into(), data!(gap));
            for (key, v) in ["paddingTop", "paddingRight", "paddingBottom", "paddingLeft"].iter().zip(pad) {
                props.insert((*key).into(), data!(v));
            }
            let justify = match l.get("justify").and_then(Data::as_str).unwrap_or("") {
                "space-between" => StackJustify::SpaceBetween,
                "center" => StackJustify::Center,
                "flex-end" | "end" | "right" => StackJustify::Max,
                _ => StackJustify::Min,
            }
            .as_wire();
            let align = match l.get("align").and_then(Data::as_str).unwrap_or("") {
                "center" => StackAlign::Center,
                "flex-end" | "end" => StackAlign::Max,
                _ => StackAlign::Min,
            }
            .as_wire();
            props.insert("primaryAxisAlignItems".into(), data!(justify));
            props.insert("counterAxisAlignItems".into(), data!(align));
        } else if self.auto && layout.is_some() {
            self.not_imported.push(data!({ "element": path, "what": "flex that wraps, or with absolutely placed children: placed where the browser put them" }));
        }
        if self.auto {
            for (key, value) in sizing(node, parent_node, parent_flex) {
                props.insert(key.into(), data!(value));
            }
        }
        let properties = Data::Map(props);
        super::style::SetProperties { handle: self.handle.clone(), id: id.clone(), properties }.run(kit)?;

        let flex = if own_flex { direction } else { None };
        for child in children {
            if child.get("tag").and_then(Data::as_str) == Some("#text") {
                self.text(kit, child, &id, (x, y))?;
            } else {
                self.element(kit, child, &id, (x, y), Some(node), flex, &path)?;
            }
        }
        Ok(id)
    }

    fn text(&mut self, kit: &mut Toolkit, node: &Data, parent: &str, origin: (f64, f64)) -> Result<(), ToolError> {
        let font = node.get("font").cloned().unwrap_or(Data::Null);
        let size = font.get("size").and_then(Data::as_f64).unwrap_or(16.0);
        let line = font.get("lineHeight").and_then(Data::as_f64).unwrap_or((size * 1.2).round());
        let lines = node.get("lines").and_then(Data::as_list).map(Vec::len).unwrap_or(1);
        let first = node.get("lines").and_then(Data::as_list).and_then(|l| l.first()).cloned().unwrap_or(Data::Null);
        let (x, y, w, _) = box_of(node);
        let glyphs = first.get("height").and_then(Data::as_f64).unwrap_or(line);
        let family = font.get("family").and_then(Data::as_str).unwrap_or("Inter").to_string();
        let weight = font.get("weight").and_then(Data::as_u64).unwrap_or(400) as u32;
        let italic = font.get("italic").and_then(Data::as_bool).unwrap_or(false);
        let style = self.face(kit.session(&self.handle)?, &family, weight, italic);
        // What create_text reads against the file or as free-form: the line
        // height, letter spacing, fills and the layer keys.
        let mut asked = Map::from([("lineHeight".to_owned(), Data::from(line))]);
        if let Some(fill) = font.get("colour").and_then(Data::as_str).and_then(paint) {
            asked.insert("fills".into(), Data::List(vec![fill]));
        }
        if let Some(spacing) = font.get("letterSpacing").and_then(Data::as_f64).filter(|s| s.abs() > 0.0) {
            asked.insert("letterSpacing".into(), Data::from(spacing));
        }
        let align = match font.get("align").and_then(Data::as_str).unwrap_or("") {
            "center" => Some(TextAlignHorizontal::Center),
            "right" | "end" => Some(TextAlignHorizontal::Right),
            "justify" => Some(TextAlignHorizontal::Justified),
            _ => None,
        };
        match font.get("decoration").and_then(Data::as_str).unwrap_or("") {
            d if d.contains("underline") => asked.insert("textDecoration".into(), Data::from(TextDecoration::Underline.as_wire())),
            d if d.contains("line-through") => asked.insert("textDecoration".into(), Data::from(TextDecoration::Strikethrough.as_wire())),
            _ => None,
        };
        match font.get("transform").and_then(Data::as_str).unwrap_or("") {
            "uppercase" => asked.insert("textCase".into(), Data::from(TextCase::Upper.as_wire())),
            "lowercase" => asked.insert("textCase".into(), Data::from(TextCase::Lower.as_wire())),
            "capitalize" => asked.insert("textCase".into(), Data::from(TextCase::Title.as_wire())),
            _ => None,
        };
        // Checked as the tool's own call checks them.
        let create = super::text::CreateText {
            handle: self.handle.clone(),
            parent: parent.into(),
            characters: non_empty("characters", node.get("text").and_then(Data::as_str).unwrap_or(""))?,
            family: Some(non_empty("fontFamily", &family)?),
            style: Some(non_empty("fontStyle", &style)?),
            font_size: Some(super::text::positive("fontSize", &Data::from(size))?),
            align: align.filter(|_| lines > 1),
            width: if lines > 1 { Some(super::text::positive("width", &Data::from(round(w + 1.0)))?) } else { None },
            height: None,
            // The browser's text box is its glyphs'; Figma's is its line's.
            at: (round(x - origin.0) as f32, round(y - origin.1 - (line - glyphs) / 2.0) as f32),
            name: None,
            asked,
        };
        create.run(kit)?;
        self.texts += 1;
        Ok(())
    }

    /// A face of `family` at `weight`, spelt as Figma reads it: the
    /// file's spelling of a face of that weight, else an installed face's, else
    /// the usual name.
    fn face(&mut self, session: &Session, family: &str, weight: u32, italic: bool) -> String {
        let fits = |style: &str| model::style_to_weight(style) == weight && style.to_lowercase().contains("italic") == italic;
        if let Some(style) = super::text::file_styles(session, family).into_iter().find(|s| fits(s)) {
            return style;
        }
        let installed = self.ports.raster.faces(family).unwrap_or_default();
        let usual = usual_style(weight, italic);
        let chosen = installed.into_iter().find(|s| fits(s)).unwrap_or_else(|| usual.clone());
        // One plain word (Regular, Bold) is spelt alike everywhere; a style of
        // two words may be spelt otherwise by Figma's copy (its Semi Bold).
        let compound = chosen.contains(' ') || chosen.chars().skip(1).any(char::is_uppercase);
        let note = format!("{family} {chosen}: not yet in this file, so Figma's spelling of it may differ");
        if compound && !self.fonts.contains(&note) {
            self.fonts.push(note);
        }
        chosen
    }

    /// A local path for an image source: a `data:` URL decoded to a file, a
    /// `file:` URL or path as it is. A remote one is not fetched.
    fn image(&mut self, src: &str) -> Option<String> {
        if let Some(rest) = src.strip_prefix("data:") {
            let (meta, data) = rest.split_once(',')?;
            if !meta.ends_with(";base64") {
                return None;
            }
            let bytes = decode_base64(data)?;
            let extension = if meta.contains("png") { "png" } else if meta.contains("jpeg") || meta.contains("jpg") { "jpg" } else { return None };
            let file = std::env::temp_dir().join(format!("grafta-import-{}-{}.{extension}", std::process::id(), self.scratch.len()));
            self.ports.store.write(&file, &bytes).ok()?;
            self.scratch.push(file.clone());
            return Some(file.display().to_string());
        }
        let path = src.strip_prefix("file://").map(percent_decode).unwrap_or_else(|| src.to_string());
        (!path.contains("://") && self.ports.images.is_file(std::path::Path::new(&path))).then_some(path)
    }
}

/// Fill, hug or fixed, read from what the browser measured (the spike's three
/// rules): FILL when a box spans its flex parent's content box on the cross
/// axis and the parent stretches; HUG when a flex box measures its content plus
/// padding and border, unless a child fills it on that axis (Figma would
/// collapse it) or it spreads its children on it (`space-*`); FIXED otherwise.
fn sizing(node: &Data, parent: Option<&Data>, parent_flex: Option<&str>) -> Vec<(&'static str, &'static str)> {
    let (x, y, w, h) = box_of(node);
    let layout = node.get("layout").filter(|l| l.is_map());
    let children: Vec<&Data> = node.get("children").and_then(Data::as_list).map(|c| c.iter().collect()).unwrap_or_default();
    let border = 2.0 * node.get("border").and_then(|b| b.get("width")).and_then(Data::as_f64).unwrap_or(0.0);
    let direction = layout.and_then(|l| l.get("direction")).and_then(Data::as_str).map(|d| if d.starts_with("column") { "column" } else { "row" });
    let mut out = Vec::new();
    for (horizontal, key, size) in [(true, "layoutSizingHorizontal", w), (false, "layoutSizingVertical", h)] {
        let mut mode = StackSize::Fixed.as_wire();
        if let (Some(parent), Some(flex)) = (parent, parent_flex) {
            let pl = parent.get("layout").cloned().unwrap_or(Data::Null);
            let (_, _, pw, ph) = box_of(parent);
            let pad = padding(&pl);
            let pborder = 2.0 * parent.get("border").and_then(|b| b.get("width")).and_then(Data::as_f64).unwrap_or(0.0);
            let content = if horizontal { pw - pad[1] - pad[3] } else { ph - pad[0] - pad[2] } - pborder;
            let cross = (flex == "column") == horizontal;
            let stretches = matches!(pl.get("align").and_then(Data::as_str), Some("normal" | "stretch") | None);
            if cross && stretches && near(size, content) {
                mode = FILL;
            }
        }
        if mode == StackSize::Fixed.as_wire() {
            if let (Some(l), false) = (layout, children.is_empty()) {
                let pad = padding(l);
                let (lo, hi) = children.iter().map(|c| box_of(c)).fold((f64::MAX, f64::MIN), |(lo, hi), (cx, cy, cw, ch)| {
                    if horizontal { (lo.min(cx), hi.max(cx + cw)) } else { (lo.min(cy), hi.max(cy + ch)) }
                });
                let extent = hi - lo + if horizontal { pad[1] + pad[3] } else { pad[0] + pad[2] } + border;
                let primary = (direction == Some("row")) == horizontal;
                let spread = primary && matches!(l.get("justify").and_then(Data::as_str), Some("space-between" | "space-around" | "space-evenly"));
                let filled = children.iter().any(|c| c.get("tag").and_then(Data::as_str) != Some("#text") && sizing(c, Some(node), direction).iter().any(|(k, v)| *k == key && *v == FILL));
                if near(size, extent) && !spread && !filled {
                    mode = HUG;
                }
            }
        }
        out.push((key, mode));
    }
    let _ = (x, y);
    out
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.6
}

fn box_of(node: &Data) -> (f64, f64, f64, f64) {
    let b = node.get("box").cloned().unwrap_or(Data::Null);
    let n = |k: &str| b.get(k).and_then(Data::as_f64).unwrap_or(0.0);
    (n("x"), n("y"), n("width"), n("height"))
}

fn num(value: &Data, key: &str) -> f64 {
    value.get(key).and_then(Data::as_f64).unwrap_or(0.0)
}

fn padding(layout: &Data) -> [f64; 4] {
    let p: Vec<f64> = layout.get("padding").and_then(Data::as_list).map(|p| p.iter().filter_map(Data::as_f64).collect()).unwrap_or_default();
    if p.len() == 4 { [p[0], p[1], p[2], p[3]] } else { [0.0; 4] }
}

fn round(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// `rgb(r, g, b)` or `rgba(r, g, b, a)` as hex and alpha.
pub fn rgba(css: &str) -> Option<(String, f64)> {
    let inner = css.trim().strip_prefix("rgba(").or_else(|| css.trim().strip_prefix("rgb("))?.strip_suffix(')')?;
    let parts: Vec<f64> = inner.split(',').map(|p| p.trim().parse::<f64>()).collect::<Result<_, _>>().ok()?;
    let channel = |i: usize| parts.get(i).map(|v| v.round().clamp(0.0, 255.0) as u8);
    let (r, g, b) = (channel(0)?, channel(1)?, channel(2)?);
    Some((format!("#{r:02X}{g:02X}{b:02X}"), parts.get(3).copied().unwrap_or(1.0)))
}

/// A solid paint, or nothing for a transparent colour.
fn paint(css: &str) -> Option<Data> {
    let (hex, alpha) = rgba(css)?;
    if alpha <= 0.0 {
        return None;
    }
    let mut paint = data!({ "type": PaintType::Solid.as_wire(), "color": hex });
    if alpha < 1.0 {
        paint["opacity"] = data!((alpha * 1000.0).round() / 1000.0);
    }
    Some(paint)
}

/// A computed `box-shadow` list as effects: each `colour x y blur spread
/// [inset]`, as Chrome writes it.
pub fn shadows(css: &str) -> Vec<Data> {
    let mut parts = Vec::new();
    let (mut depth, mut start) = (0, 0);
    for (i, c) in css.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&css[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&css[start..]);
    parts
        .into_iter()
        .filter_map(|part| {
            let part = part.trim();
            let close = part.find(')')?;
            let (colour, rest) = part.split_at(close + 1);
            let (hex, alpha) = rgba(colour)?;
            let inset = rest.contains("inset");
            let lengths: Vec<f64> = rest.split_whitespace().filter_map(|t| t.strip_suffix("px")).filter_map(|t| t.parse().ok()).collect();
            if lengths.len() < 3 {
                return None;
            }
            Some(data!({
                "type": if inset { EffectType::InnerShadow } else { EffectType::DropShadow }.as_wire(),
                "color": hex,
                "opacity": alpha,
                "offset": { "x": lengths[0], "y": lengths[1] },
                "radius": lengths[2],
                "spread": lengths.get(3).copied().unwrap_or(0.0),
            }))
        })
        .collect()
}

fn usual_style(weight: u32, italic: bool) -> String {
    let base = match weight {
        0..=150 => "Thin",
        151..=250 => "Extra Light",
        251..=350 => "Light",
        351..=450 => "Regular",
        451..=550 => "Medium",
        551..=650 => "Semi Bold",
        651..=750 => "Bold",
        751..=850 => "Extra Bold",
        _ => "Black",
    };
    match (italic, base) {
        (true, "Regular") => "Italic".into(),
        (true, base) => format!("{base} Italic"),
        (false, base) => base.into(),
    }
}

fn short(text: &str) -> String {
    let mut s: String = text.chars().take(60).collect();
    if text.chars().count() > 60 {
        s.push('…');
    }
    s
}

fn percent_decode(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&path[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn decode_base64(data: &str) -> Option<Vec<u8>> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' | b'-' => Some(62),
        b'/' | b'_' => Some(63),
        _ => None,
    };
    let clean: Vec<u8> = data.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=').collect();
    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    for chunk in clean.chunks(4) {
        let v: Vec<u8> = chunk.iter().map(|c| value(*c)).collect::<Option<_>>()?;
        out.push((v[0] << 2) | (v.get(1).copied().unwrap_or(0) >> 4));
        if v.len() > 2 {
            out.push((v[1] << 4) | (v[2] >> 2));
        }
        if v.len() > 3 {
            out.push((v[2] << 6) | v[3]);
        }
    }
    Some(out)
}

/// A name or text the snapshot gives, refused when empty as the tool's own
/// call refuses it.
fn non_empty(parameter: &str, value: &str) -> Result<String, ToolError> {
    if value.is_empty() {
        return Err(DomainError::Expected { parameter: parameter.into(), expected: "a non-empty string".into(), got: "\"\"".into() }.into());
    }
    Ok(value.to_string())
}

fn id_of(output: Output) -> Result<String, ToolError> {
    match output {
        Output::Data(value) => value["id"].as_str().map(str::to_string).ok_or_else(|| ImportError::MadeWithoutId.into()),
        Output::Image { .. } => Err(ImportError::PictureReturned.into()),
    }
}
