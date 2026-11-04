//! How legible each text is against what is drawn behind it.
//!
//! The checked node is drawn with its texts left out, and each text it would
//! have drawn is noted instead: its box in the node's space, its colours as
//! resolved where it is drawn, its opacity, its runs' sizes and weights. The
//! picture of what is behind is then read under each box, and each colour of
//! the text, laid over each colour behind it, gives a WCAG 2.2 ratio. The
//! lowest is the text's.

use std::collections::HashMap;

pub use domain::port::{Contrast, TextSample};
use schema::Guid;

use crate::{draw_with, FigFile, RenderError, SceneGraph};

/// Pixels per unit the background is drawn at.
const SCALE: f32 = 2.0;


/// Every visible text under `id`, measured against what is drawn behind it.
pub fn contrast(fig: &FigFile, graph: &SceneGraph, id: Guid) -> Result<Vec<Contrast>, RenderError> {
    crate::on_own_stack(|| contrast_here(fig, graph, id))
}

fn contrast_here(fig: &FigFile, graph: &SceneGraph, id: Guid) -> Result<Vec<Contrast>, RenderError> {
    let node = graph.node(id).ok_or_else(|| RenderError(format!("unknown node {id}")))?;
    if node.width <= 0.0 || node.height <= 0.0 {
        return Err(RenderError(format!("{id} has no area to draw ({}×{})", node.width, node.height)));
    }
    let (w, h) = ((node.width * SCALE).ceil().max(1.0) as u32, (node.height * SCALE).ceil().max(1.0) as u32);
    let (drawing, samples) = draw_with(fig, graph, node, 0.0, node.height, w, h, true);
    // The texts left out have nowhere to be named: what is measured says nothing of them.
    if let Some(note) = drawing.notes.iter().find(|note| *note == crate::TOO_DEEP || *note == crate::TOO_MANY) {
        return Err(RenderError(note.clone()));
    }
    let options = resvg::usvg::Options { fontdb: crate::text::fonts(), ..resvg::usvg::Options::default() };
    let tree = resvg::usvg::Tree::from_str(&drawing.svg, &options).map_err(|error| RenderError(format!("the drawing did not parse: {error}")))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or_else(|| RenderError(format!("cannot allocate {w}×{h} pixels")))?;
    resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    Ok(samples.into_iter().map(|sample| measure(sample, &pixmap)).collect())
}

fn measure(mut sample: TextSample, behind: &resvg::tiny_skia::Pixmap) -> Contrast {
    let large = sample.runs.iter().all(|&(_, size, weight)| size >= 24.0 || (size >= 18.66 && weight >= 700));
    let required = if large { 3.0 } else { 4.5 };
    let unmeasured = |sample: TextSample| Contrast { sample, ratio: None, required, foreground: None, background: None, varies: None };
    if sample.unmeasurable.is_some() {
        return unmeasured(sample);
    }
    let colours = colours_behind(&sample, behind);
    if colours.is_empty() {
        sample.unmeasurable = Some("nothing is painted behind it in the checked node".into());
        return unmeasured(sample);
    }
    let mut lowest: Option<(f32, [f32; 4], [f32; 3])> = None;
    let mut highest = 0.0_f32;
    for &(colour, _, _) in &sample.runs {
        let alpha = colour[3] * sample.opacity;
        for back in &colours {
            let front = [0, 1, 2].map(|i| colour[i] * alpha + back[i] * (1.0 - alpha));
            let r = ratio(front, *back);
            highest = highest.max(r);
            if lowest.is_none_or(|(l, _, _)| r < l) {
                lowest = Some((r, colour, *back));
            }
        }
    }
    let (r, front, back) = lowest.expect("runs and colours are not empty");
    Contrast {
        ratio: Some(r),
        required,
        foreground: Some(hex(&[front[0], front[1], front[2]])),
        background: Some(hex(&back)),
        // Antialiased edges blend two colours into ratios between theirs;
        // only a spread wider than that says the background varies.
        varies: (highest > r * 1.1).then_some((r, highest)),
        sample,
    }
}

/// The distinct opaque colours drawn under a text's box, 0–1. A pixel only
/// partly covered is composited over white, as a page shows it; one not
/// covered at all is nothing.
fn colours_behind(sample: &TextSample, behind: &resvg::tiny_skia::Pixmap) -> Vec<[f32; 3]> {
    let corners = [(0.0, 0.0), (sample.width, 0.0), (0.0, sample.height), (sample.width, sample.height)].map(|(x, y)| {
        let m = sample.at;
        ((m[0] * x + m[2] * y + m[4]) * SCALE, (m[1] * x + m[3] * y + m[5]) * SCALE)
    });
    let clamp = |v: f32, max: u32| (v.max(0.0) as u32).min(max);
    let (x0, x1) = corners.iter().map(|c| c.0).fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
    let (y0, y1) = corners.iter().map(|c| c.1).fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
    let (x0, x1, y0, y1) = (clamp(x0.floor(), behind.width()), clamp(x1.ceil(), behind.width()), clamp(y0.floor(), behind.height()), clamp(y1.ceil(), behind.height()));
    let mut seen: HashMap<[u8; 3], ()> = HashMap::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let Some(p) = behind.pixel(x, y) else { continue };
            let p = p.demultiply();
            if p.alpha() == 0 {
                continue;
            }
            let a = f32::from(p.alpha()) / 255.0;
            let over_white = |c: u8| (f32::from(c) * a + 255.0 * (1.0 - a)).round() as u8;
            seen.insert([over_white(p.red()), over_white(p.green()), over_white(p.blue())], ());
        }
    }
    let mut colours: Vec<[f32; 3]> = seen.into_keys().map(|c| c.map(|v| f32::from(v) / 255.0)).collect();
    colours.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    colours
}

/// WCAG 2.2 contrast ratio between two opaque colours, 0–1.
#[must_use]
pub fn ratio(a: [f32; 3], b: [f32; 3]) -> f32 {
    let luminance = |c: [f32; 3]| {
        let channel = |v: f32| if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
        0.2126 * channel(c[0]) + 0.7152 * channel(c[1]) + 0.0722 * channel(c[2])
    };
    let (l1, l2) = (luminance(a), luminance(b));
    (l1.max(l2) + 0.05) / (l1.min(l2) + 0.05)
}

fn hex(c: &[f32]) -> String {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", byte(c[0]), byte(c[1]), byte(c[2]))
}

/// `a` then `b`: the SVG matrix of a child `b` inside a parent `a`.
#[must_use]
pub fn compose(a: [f32; 6], b: [f32; 6]) -> [f32; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}

