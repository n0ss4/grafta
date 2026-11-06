//! Laying out text that has no layout of Figma's.
//!
//! Text grafta wrote carries no `derivedTextData` until Figma opens the file
//!, so it is laid out here: the font the text names, found among the
//! machine's fonts, or the nearest this machine has, and lines broken at the
//! text's width the way Figma breaks them, after spaces, a word too long for a
//! line broken where it runs out. Advances come from the font's own metrics;
//! kerning and ligatures are not applied, so widths are close, not exact.
//! How close is measured against the layout Figma stored for its own text.

use resvg::usvg::fontdb::{self, Database, Family, Query, Stretch, Style, Weight};
use skrifa::instance::{LocationRef, Size};
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider};

/// Families tried, in order, when the text's own is not installed. The order
/// is measured: over every paragraph Figma wrapped by width in the fixtures,
/// Helvetica Neue first breaks 85.8% of them where Figma does, Inter first
/// 62.7%, with Roboto and DM Sans the fonts most stood in for. Calibrating a
/// stand-in on the widths Figma stored for the missing font in the same file
/// was tried: single lines came closer, line breaks further.
const STAND_INS: [&str; 4] = ["Helvetica Neue", "Inter", "Helvetica", "Arial"];

/// A font found for a text.
#[derive(Debug, Clone)]
pub struct Font {
    pub id: fontdb::ID,
    /// The family drawn with, which may not be the one asked for.
    pub family: String,
    pub substituted: bool,
}

/// Find the text's font, or the nearest installed one.
#[must_use]
pub fn find(db: &Database, family: &str, weight: u32, italic: bool) -> Option<Font> {
    let style = if italic { Style::Italic } else { Style::Normal };
    let query = |name: &str| {
        db.query(&Query {
            families: &[Family::Name(name)],
            weight: Weight(u16::try_from(weight).unwrap_or(400)),
            stretch: Stretch::Normal,
            style,
        })
    };
    let installed = |id: fontdb::ID| {
        db.face(id).and_then(|face| face.families.first().map(|(name, _)| name.clone())).unwrap_or_default()
    };
    if let Some(id) = query(family) {
        return Some(Font { id, family: installed(id), substituted: false });
    }
    STAND_INS
        .iter()
        .find_map(|name| query(name))
        .or_else(|| {
            db.query(&Query { families: &[Family::SansSerif], weight: Weight(400), stretch: Stretch::Normal, style: Style::Normal })
        })
        // fontdb's generic sans-serif is "Arial" unless told otherwise, so a
        // Linux machine with only DejaVu or Liberation matched nothing above
        // and drew no text. Any installed font beats none; an upright regular
        // one first.
        .or_else(|| {
            let upright = |face: &&fontdb::FaceInfo| face.style == Style::Normal;
            let regular = |face: &&fontdb::FaceInfo| upright(face) && face.weight == Weight::NORMAL;
            db.faces().find(regular).or_else(|| db.faces().find(upright)).or_else(|| db.faces().next()).map(|face| face.id)
        })
        .map(|id| Font { id, family: installed(id), substituted: true })
}

/// The names of a family's installed faces, as Figma names a font's style:
/// each face's typographic subfamily name, else its subfamily name, and a
/// variable font's named instances. `None` when no face has the family, which
/// says nothing about the fonts Figma has.
#[must_use]
pub fn faces(db: &Database, family: &str) -> Option<Vec<String>> {
    let mut names = Vec::new();
    let mut found = false;
    for face in db.faces().filter(|face| face.families.iter().any(|(name, _)| name.eq_ignore_ascii_case(family))) {
        found = true;
        db.with_face_data(face.id, |data, index| {
            let font = FontRef::from_index(data, index).ok()?;
            let name = |id| font.localized_strings(id).english_or_first().map(|s| s.to_string());
            names.extend(name(StringId::TYPOGRAPHIC_SUBFAMILY_NAME).or_else(|| name(StringId::SUBFAMILY_NAME)));
            names.extend(font.named_instances().iter().filter_map(|instance| name(instance.subfamily_name_id())));
            Some(())
        });
    }
    names.sort();
    names.dedup();
    found.then_some(names)
}

/// Apple's font of last resort, which claims every character there is and draws
/// a box for each.
const LAST_RESORT: &str = "LastResort";

fn is_last_resort(face: &fontdb::FaceInfo) -> bool {
    face.families.iter().any(|(name, _)| name.contains(LAST_RESORT))
}

/// Whether a font draws this character.
#[must_use]
pub fn has_char(db: &Database, id: fontdb::ID, c: char) -> bool {
    db.with_face_data(id, |data, index| FontRef::from_index(data, index).ok().and_then(|font| font.charmap().map(c)).is_some_and(|glyph| glyph.to_u32() != 0)).unwrap_or(false)
}

/// The font to draw a character in that its text's own font lacks: a stand-in
/// if it has it, else the installed face nearest in style, by family name so
/// that every machine's scan gives the same answer. `None` when no font draws it.
#[must_use]
pub fn fallback(db: &Database, c: char, weight: u32, italic: bool) -> Option<Font> {
    let style = if italic { Style::Italic } else { Style::Normal };
    let family = |face: &fontdb::FaceInfo| face.families.first().map(|(name, _)| name.clone()).unwrap_or_default();
    let stand_in = |face: &&fontdb::FaceInfo| STAND_INS.iter().position(|name| family(face) == *name).unwrap_or(STAND_INS.len());
    db.faces()
        .filter(|face| !is_last_resort(face) && has_char(db, face.id, c))
        // A leading dot is a system's own font, which no design names.
        .min_by_key(|face| (stand_in(face), family(face).starts_with('.'), face.style != style, face.weight.0.abs_diff(u16::try_from(weight).unwrap_or(400)), family(face)))
        .map(|face| Font { id: face.id, family: family(face), substituted: false })
}

/// The rasteriser's options: the machine's fonts, and a fallback that is never
/// the font of last resort, or a character none has turns its whole line to boxes.
#[must_use]
pub fn options() -> resvg::usvg::Options<'static> {
    let select_fallback: resvg::usvg::FallbackSelectionFn<'static> = Box::new(|c, used, db| {
        let base = db.face(*used.first()?)?;
        let same = |face: &fontdb::FaceInfo| base.style == face.style || base.weight == face.weight || base.stretch == face.stretch;
        db.faces().find(|face| !used.contains(&face.id) && !is_last_resort(face) && same(face) && has_char(db, face.id, c)).map(|face| face.id)
    });
    resvg::usvg::Options {
        fontdb: fonts(),
        font_resolver: resvg::usvg::FontResolver { select_font: resvg::usvg::FontResolver::default_font_selector(), select_fallback },
        ..resvg::usvg::Options::default()
    }
}

pub use domain::font::is_face;

/// The machine's fonts, read once per process and shared by the layout and
/// the rasteriser, so a text is drawn in the font it was measured in.
pub fn fonts() -> std::sync::Arc<Database> {
    static FONTS: std::sync::OnceLock<std::sync::Arc<Database>> = std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = Database::new();
            db.load_system_fonts();
            std::sync::Arc::new(db)
        })
        .clone()
}

/// A font's ascent, descent (positive, below the baseline) and line gap, in
/// pixels at `size`.
#[must_use]
pub fn vertical(db: &Database, id: fontdb::ID, size: f32) -> (f32, f32, f32) {
    db.with_face_data(id, |data, index| {
        let font = FontRef::from_index(data, index).ok()?;
        let metrics = font.metrics(Size::new(size), LocationRef::default());
        Some((metrics.ascent, -metrics.descent, metrics.leading))
    })
    .flatten()
    .unwrap_or((size * 0.8, size * 0.2, 0.0))
}

/// One line: its characters (by index, end exclusive) and its width.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub start: usize,
    pub end: usize,
    pub width: f32,
}

/// Each character's advance in pixels, at `size`, with `spacing` added, in
/// the font `id`.
#[must_use]
pub fn advances(db: &Database, id: fontdb::ID, text: &str, size: f32, spacing: f32) -> Vec<f32> {
    db.with_face_data(id, |data, index| {
        let Ok(font) = FontRef::from_index(data, index) else { return None };
        let charmap = font.charmap();
        let metrics = font.glyph_metrics(Size::new(size), LocationRef::default());
        Some(
            text.chars()
                .map(|c| {
                    let glyph = charmap.map(c).unwrap_or_default();
                    metrics.advance_width(glyph).unwrap_or(size * 0.5) + spacing
                })
                .collect(),
        )
    })
    .flatten()
    .unwrap_or_else(|| text.chars().map(|_| size * 0.5 + spacing).collect())
}

/// Break `text` into lines no wider than `width` (`None`: no limit), at
/// newlines, after spaces, and inside a word only when it alone is too wide.
/// Trailing spaces do not count toward a line's width, as in Figma.
#[must_use]
pub fn break_lines(text: &str, advances: &[f32], width: Option<f32>) -> Vec<Line> {
    let chars: Vec<char> = text.chars().collect();
    let mut lines = Vec::new();
    let mut start = 0;
    while start <= chars.len() {
        // The paragraph: up to the next newline.
        let end = chars[start..].iter().position(|c| *c == '\n').map_or(chars.len(), |i| start + i);
        let mut line_start = start;
        loop {
            let (line_end, line_width) = fit(&chars, advances, line_start, end, width);
            lines.push(Line { start: line_start, end: line_end, width: line_width });
            if line_end >= end {
                break;
            }
            line_start = line_end;
        }
        start = end + 1;
    }
    lines
}

/// Break one paragraph (no newlines) into lines no wider than `width`, the
/// first narrower by `first_indent`, as a paragraph indent makes it.
/// Indices are into `chars`.
#[must_use]
pub fn break_paragraph(chars: &[char], advances: &[f32], width: Option<f32>, first_indent: f32) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    loop {
        let limit = width.map(|w| if lines.is_empty() { (w - first_indent).max(0.0) } else { w });
        let (end, line_width) = fit(chars, advances, start, chars.len(), limit);
        lines.push(Line { start, end, width: line_width });
        if end >= chars.len() {
            return lines;
        }
        start = end;
    }
}

/// How far a line from `start` reaches before `end` within `width`, and its
/// width without trailing spaces.
fn fit(chars: &[char], advances: &[f32], start: usize, end: usize, width: Option<f32>) -> (usize, f32) {
    let visible = |from: usize, to: usize| {
        let mut to = to;
        while to > from && chars[to - 1] == ' ' {
            to -= 1;
        }
        advances[from..to].iter().sum::<f32>()
    };
    let Some(width) = width else { return (end, visible(start, end)) };
    let mut last_break: Option<usize> = None;
    let mut i = start;
    while i < end {
        if chars[i] == ' ' {
            i += 1;
            last_break = Some(i);
            continue;
        }
        if visible(start, i + 1) > width + 0.01 {
            // Break after the last space, or inside the word when there is
            // none, keeping at least one character on the line.
            let at = last_break.unwrap_or(i.max(start + 1));
            return (at, visible(start, at));
        }
        i += 1;
    }
    (end, visible(start, end))
}

#[cfg(test)]
mod tests;
