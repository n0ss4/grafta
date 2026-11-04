//! How close grafta's text layout comes to Figma's. Every text
//! layer Figma laid out stores its lines (`derivedTextData.baselines`); each
//! is laid out again here, ignoring them, and the line breaks compared. Skips
//! without fixtures. Prints the figures the ARCHITECTURE record quotes.

// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![allow(clippy::disallowed_methods)]

use std::collections::HashMap;

use render::text::{advances, break_lines, find};
use resvg::usvg::fontdb::Database;

#[derive(Default)]
struct Tally {
    texts: usize,
    same_breaks: usize,
    /// Texts of a fixed width that Figma broke into more than one line: the
    /// only ones where breaking can go wrong.
    wrapped: usize,
    wrapped_same: usize,
    widths: Vec<f32>,
}

#[test]
fn line_breaks_against_figmas_own_layout() {
    let mut db = Database::new();
    db.load_system_fonts();
    for path in testkit::fixture::all() {
        let name = path.file_name().unwrap().to_str().unwrap();
        let Some((fig, graph)) = testkit::fixture::shared(name) else { continue };
        let raw: HashMap<schema::Guid, &kiwi::Value> = model::author::node_changes(fig)
            .iter()
            .filter_map(|nc| {
                let g = nc.get("guid")?;
                Some((schema::Guid::new(g.get("sessionID")?.as_u32()?, g.get("localID")?.as_u32()?), nc))
            })
            .collect();
        let mut tallies: HashMap<bool, Tally> = HashMap::new();
        let mut fonts = HashMap::new();
        for node in graph.nodes.values().filter(|n| n.node_type == model::NodeType::Text) {
            let (Some(text), Some(style), Some(nc)) = (node.text.as_deref(), node.text_style.as_ref(), raw.get(&node.id)) else { continue };
            let Some(baselines) = nc.get("derivedTextData").and_then(|d| d.get("baselines")).and_then(kiwi::Value::as_array) else { continue };
            if baselines.is_empty() || text.is_empty() || !text.is_ascii() {
                continue;
            }
            let key = (style.font_family.clone(), style.font_weight, style.italic);
            let Some(font) = fonts.entry(key).or_insert_with(|| find(&db, &style.font_family, style.font_weight, style.italic)).clone() else { continue };
            let shown = match style.text_case.as_str() {
                "UPPER" => text.to_uppercase(),
                "LOWER" => text.to_lowercase(),
                _ => text.to_string(),
            };
            let spacing = style.letter_spacing.map_or(0.0, |l| l.to_px(style.font_size));
            let widths = advances(&db, font.id, &shown, style.font_size, spacing);
            let limit = (style.text_auto_resize != "WIDTH_AND_HEIGHT").then_some(node.width);
            let ours = break_lines(&shown, &widths, limit);
            let figma: Vec<(usize, f32)> = baselines
                .iter()
                .filter_map(|b| Some((b.get("firstCharacter")?.as_u32()? as usize, b.get("width")?.as_f32()?)))
                .collect();
            let tally = tallies.entry(font.substituted).or_default();
            tally.texts += 1;
            let starts: Vec<usize> = ours.iter().map(|l| l.start).collect();
            let same = starts == figma.iter().map(|(s, _)| *s).collect::<Vec<_>>();
            tally.same_breaks += usize::from(same);
            if limit.is_some() && figma.len() > 1 && !text.contains('\n') {
                tally.wrapped += 1;
                tally.wrapped_same += usize::from(same);
            }
            if ours.len() == 1 && figma.len() == 1 && figma[0].1 > 1.0 {
                tally.widths.push((ours[0].width - figma[0].1) / figma[0].1);
            }
        }
        for (substituted, mut tally) in tallies {
            tally.widths.sort_by(f32::total_cmp);
            let median = tally.widths.get(tally.widths.len() / 2).copied().unwrap_or(0.0);
            let within = tally.widths.iter().filter(|w| w.abs() <= 0.02).count();
            println!(
                "{name}: {} fonts: {} texts, {:.1}% break as Figma does; wrapped by width: {}, {:.1}% the same; single lines: median width error {:+.1}%, {:.1}% within 2%",
                if substituted { "stand-in" } else { "installed" },
                tally.texts,
                100.0 * tally.same_breaks as f64 / tally.texts.max(1) as f64,
                tally.wrapped,
                100.0 * tally.wrapped_same as f64 / tally.wrapped.max(1) as f64,
                100.0 * median,
                100.0 * within as f64 / tally.widths.len().max(1) as f64,
            );
        }
    }
}
