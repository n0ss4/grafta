#![deny(clippy::all)]
// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![cfg_attr(test, allow(clippy::disallowed_methods))]

//! Drawing a node of a `.fig`, so that a model can see what it read or made.
//!
//! The node's subtree is written as SVG and rasterised by `resvg`, entirely
//! offline. Two things make the picture exact where it can be and honest
//! where it cannot:
//!
//! - **Figma's own text is drawn from its glyph outlines.** `derivedTextData`
//!   stores every glyph as a path (`commandsBlob`, the same encoding as vector
//!   geometry), so no font needs to be installed. Text grafta wrote has no
//!   such data, and is drawn with a system font; the result says so.
//! - **An instance is drawn from its component**, with its fill, text and
//!   visibility overrides applied one level deep. Deeper overrides are noted,
//!   not guessed.
//!
//! What is drawn approximately, or not at all, is listed in [`Rendered::notes`].

pub mod contrast;
#[cfg(not(target_arch = "wasm32"))]
pub mod pdf;
/// A browser makes no PDF: krilla is most of a PDF's code, and the reader in
/// a page draws pictures alone.
#[cfg(target_arch = "wasm32")]
pub mod pdf {
    use schema::Guid;

    use crate::{FigFile, RenderError, SceneGraph};

    pub use domain::port::Pdf;

    /// Refused, in words.
    pub fn pdf(_fig: &FigFile, _graph: &SceneGraph, _ids: &[Guid]) -> Result<Pdf, RenderError> {
        Err(RenderError("a PDF is not made in a browser: install grafta to export one".to_owned()))
    }
}
pub mod text;

use std::collections::HashMap;
use std::fmt::Write as _;

pub use domain::port::{RenderError, Rendered, TextBox};
use figfile::FigFile;
use kiwi::Value as Raw;
use model::{Effect, EffectType, Fill, FillType, NodeType, SceneGraph, SceneNode};
use schema::{table_entries, table_sizes, BlendMode, ComponentPropNodeField, ConnectorLineStyle, Field, GeneratedLayer, Guid, ImageScaleMode, MaskType, NumberUnits, PaintType, ShapeWithTextType, StrokeAlign, StrokeCap, StrokeJoin, TableAxis, TextAlignHorizontal, TextAlignVertical, TextAutoResize, TextCase, TextDecoration, WindingRule, Wire, WireName};



/// How deep layers are drawn, counted through instances. Measured on 13 real
/// files: at most 30 stored.
const MAX_LAYERS: usize = 256;
const TOO_DEEP: &str = "layers nested deeper than 256 are not drawn";
/// How many layers one drawing holds. Components that each hold two instances of
/// the next double at every level, from a file of a few KB. Tests stop sooner.
const MAX_DRAWN: usize = if cfg!(test) { 20_000 } else { 200_000 };
const TOO_MANY: &str = "a drawing stops at its limit of layers: those after it are not drawn";
/// A drawing recurses per layer, and so does the rasteriser on what it draws, so
/// the stack they need is theirs: callers' run from 1 MiB (Windows' main thread)
/// to 8 MiB. Measured in a release build: 7 KiB per layer, ten times that in debug.
pub const STACK: usize = 64 << 20;

/// `work` on a stack of [`STACK`], whoever is asking. A browser's WebAssembly
/// starts no thread, so there it runs on the caller's stack, which the reader
/// links at [`STACK`] too.
#[cfg(target_arch = "wasm32")]
fn on_own_stack<T: Send>(work: impl FnOnce() -> Result<T, RenderError> + Send) -> Result<T, RenderError> {
    work()
}

/// `work` on a stack of [`STACK`], whoever is asking.
#[cfg(not(target_arch = "wasm32"))]
fn on_own_stack<T: Send>(work: impl FnOnce() -> Result<T, RenderError> + Send) -> Result<T, RenderError> {
    std::thread::scope(|scope| {
        let thread = std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(scope, work)
            .map_err(|error| RenderError(format!("cannot start drawing: {error}")))?;
        thread.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// Draw `id` and its subtree at `scale`, reduced so that neither side exceeds
/// `max_size` pixels.
pub fn render(fig: &FigFile, graph: &SceneGraph, id: Guid, scale: f32, max_size: u32) -> Result<Rendered, RenderError> {
    on_own_stack(|| render_here(fig, graph, id, scale, max_size))
}

fn render_here(fig: &FigFile, graph: &SceneGraph, id: Guid, scale: f32, max_size: u32) -> Result<Rendered, RenderError> {
    let node = graph.node(id).ok_or_else(|| RenderError(format!("unknown node {id}")))?;
    // A line has no height: its picture is as tall as its stroke.
    let (top, tall) = if node.node_type == NodeType::Line && node.height <= 0.0 {
        let weight = node.stroke_weight.max(1.0);
        (-weight / 2.0, weight)
    } else {
        (0.0, node.height)
    };
    if node.width <= 0.0 || tall <= 0.0 {
        return Err(RenderError(format!("{id} has no area to draw ({}×{})", node.width, node.height)));
    }
    let largest = node.width.max(tall) * scale;
    let scale = if largest > max_size as f32 { scale * max_size as f32 / largest } else { scale };
    let width = (node.width * scale).ceil().max(1.0) as u32;
    let height = (tall * scale).ceil().max(1.0) as u32;

    let drawing = draw(fig, graph, node, top, tall, width, height);
    let options = text::options();
    let tree = resvg::usvg::Tree::from_str(&drawing.svg, &options)
        .map_err(|error| RenderError(format!("the drawing did not parse: {error}")))?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RenderError(format!("cannot allocate {width}×{height} pixels")))?;
    // The viewBox already maps the node's units onto the picture's pixels.
    resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());
    let png = pixmap.encode_png().map_err(|error| RenderError(format!("cannot encode the image: {error}")))?;

    Ok(Rendered { png, width, height, scale, notes: drawing.notes })
}

/// A node drawn as SVG, and what was drawn approximately.
struct Drawing {
    svg: String,
    notes: Vec<String>,
    /// Whether a gradient sits inside a group, for a PDF to say so. A browser
    /// makes no PDF, so it is not read there.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    gradient_in_group: bool,
}

/// Write `node` and its subtree as an SVG document of `width`×`height` units
/// showing the node's box from `top`, the one drawing `render` rasterises and
/// `pdf` writes.
fn draw(fig: &FigFile, graph: &SceneGraph, node: &SceneNode, top: f32, tall: f32, width: impl std::fmt::Display, height: impl std::fmt::Display) -> Drawing {
    draw_with(fig, graph, node, top, tall, width, height, false).0
}

/// [`draw`], and when `texts` is set, the drawing has its texts left out and
/// they are returned as they would have been drawn.
#[allow(clippy::too_many_arguments)]
fn draw_with(
    fig: &FigFile,
    graph: &SceneGraph,
    node: &SceneNode,
    top: f32,
    tall: f32,
    width: impl std::fmt::Display,
    height: impl std::fmt::Display,
    texts: bool,
) -> (Drawing, Vec<contrast::TextSample>) {
    let mut svg = Svg::new(fig, graph);
    if texts {
        svg.texts = Some(Vec::new());
    }
    // A node inside a frame set to a mode is drawn in it.
    let mut ancestors = Vec::new();
    let mut at = node.parent_id;
    while let Some(id) = at {
        ancestors.push(id);
        at = graph.node(id).and_then(|n| n.parent_id);
    }
    for id in ancestors.into_iter().rev() {
        let modes = svg.explicit_modes(id);
        svg.modes.push(modes);
    }
    svg.node(node, true);
    let body = std::mem::take(&mut svg.out);
    let document = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{width}" height="{height}" viewBox="0 {top} {w} {h}"><defs>{defs}</defs>{body}</svg>"#,
        w = node.width,
        h = tall,
        defs = svg.defs,
    );
    let mut notes: Vec<String> = svg.notes.into_iter().collect();
    notes.sort();
    (Drawing { svg: document, notes, gradient_in_group: svg.gradient_in_group }, svg.texts.unwrap_or_default())
}

/// A picture of one colour, as PNG: the thumbnail of a file with nothing in it
/// yet. `rgba` is in 0–1, as Figma stores colours.
/// A text's box as `render_node` lays it out: its widest
/// line's right edge and its lines' height, and the font it was measured in
/// when that is a stand-in for one not installed. `None` when the machine has
/// no font at all.
#[must_use]
pub fn measure_text(fig: &FigFile, graph: &SceneGraph, node: &SceneNode) -> Option<TextBox> {
    let mut svg = Svg::bare(fig, graph);
    let layout = svg.layout(node, "", None)?;
    let width = layout.lines.iter().map(|l| l.x + l.width).fold(0.0, f32::max);
    let stand_in = layout.styles.iter().find(|s| s.font.substituted).map(|s| s.font.family.clone());
    Some(TextBox { width, height: layout.height, stand_in })
}


pub fn blank_png(width: u32, height: u32, rgba: [f32; 4]) -> Result<Vec<u8>, RenderError> {
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RenderError(format!("cannot allocate {width}×{height} pixels")))?;
    let [r, g, b, a] = rgba.map(|c| c.clamp(0.0, 1.0));
    pixmap.fill(resvg::tiny_skia::Color::from_rgba(r, g, b, a).expect("clamped to 0–1"));
    pixmap.encode_png().map_err(|error| RenderError(format!("cannot encode the image: {error}")))
}

/// The SVG being built, with its definitions and what it had to approximate.
struct Svg<'a> {
    fig: &'a FigFile,
    graph: &'a SceneGraph,
    raw: HashMap<Guid, &'a Raw>,
    out: String,
    defs: String,
    next_id: usize,
    notes: std::collections::BTreeSet<String>,
    /// The component property values of the instances being drawn, the
    /// innermost last.
    properties: Vec<indexmap::IndexMap<Guid, String>>,
    /// Figma's cached expansion of the instances being drawn
    /// (`derivedSymbolData`): what it resolved for each layer, by path from
    /// the innermost instance. The innermost last.
    derived: Vec<Vec<CachedEntry<'a>>>,
    /// The layers drawn so far.
    drawn: usize,
    /// The blends written so far, to tell whether a frame's contents hold one.
    blends: usize,
    /// The groups being drawn in that a PDF keeps as groups: opacity, a mask, a
    /// blend, isolation. A clip is not one.
    groups: usize,
    /// Whether a gradient was drawn inside one, which Apple's PDF renderer misplaces.
    gradient_in_group: bool,
    /// The modes in effect, collection → mode, the innermost last: the root's ancestors', then each node's being drawn.
    modes: Vec<HashMap<Guid, Guid>>,
    /// Each node being drawn's transform into the root's space, as an SVG
    /// matrix `[a b c d e f]`, and its opacity times its ancestors'.
    at: Vec<([f32; 6], f32)>,
    /// When set, texts are noted here instead of drawn.
    texts: Option<Vec<contrast::TextSample>>,
    /// The instances being drawn, the innermost last.
    instances: Vec<Guid>,
}

impl<'a> Svg<'a> {
    fn new(fig: &'a FigFile, graph: &'a SceneGraph) -> Self {
        let raw = model::author::node_changes(fig)
            .iter()
            .filter_map(|nc| {
                Some((Guid::read(nc.field(Field::Guid)?)?, nc))
            })
            .collect();
        Self {
            fig,
            graph,
            raw,
            out: String::new(),
            defs: String::new(),
            next_id: 0,
            notes: Default::default(),
            properties: Vec::new(),
            derived: Vec::new(),
            drawn: 0,
            blends: 0,
            groups: 0,
            gradient_in_group: false,
            modes: Vec::new(),
            at: Vec::new(),
            texts: None,
            instances: Vec::new(),
        }
    }

    /// One that reads no node changes: for measuring text, which needs none.
    fn bare(fig: &'a FigFile, graph: &'a SceneGraph) -> Self {
        Self {
            fig,
            graph,
            raw: HashMap::new(),
            out: String::new(),
            defs: String::new(),
            next_id: 0,
            notes: Default::default(),
            properties: Vec::new(),
            derived: Vec::new(),
            drawn: 0,
            blends: 0,
            groups: 0,
            gradient_in_group: false,
            modes: Vec::new(),
            at: Vec::new(),
            texts: None,
            instances: Vec::new(),
        }
    }

    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    /// [`Svg::matrix`] as numbers, `[a b c d e f]` in SVG's order.
    fn matrix_values(&self, node: &SceneNode) -> [f32; 6] {
        let cached = self.cached(node, Field::Transform);
        let Some(t) = cached.or_else(|| self.raw.get(&node.id).and_then(|nc| nc.field(Field::Transform))) else {
            return [1.0, 0.0, 0.0, 1.0, node.x, node.y];
        };
        let f = |k: Field, d: f32| t.field(k).and_then(Raw::as_f32).unwrap_or(d);
        [f(Field::M00, 1.0), f(Field::M10, 0.0), f(Field::M01, 0.0), f(Field::M11, 1.0), f(Field::M02, 0.0), f(Field::M12, 0.0)]
    }

    /// The node's own transform relative to its parent, as the file stores it,
    /// or its position when there is none (a graph built by hand).
    fn matrix(&self, node: &SceneNode) -> String {
        let cached = self.cached(node, Field::Transform);
        let Some(t) = cached.or_else(|| self.raw.get(&node.id).and_then(|nc| nc.field(Field::Transform))) else {
            return format!("translate({} {})", node.x, node.y);
        };
        let f = |k: Field, d: f32| t.field(k).and_then(Raw::as_f32).unwrap_or(d);
        format!(
            "matrix({} {} {} {} {} {})",
            f(Field::M00, 1.0),
            f(Field::M10, 0.0),
            f(Field::M01, 0.0),
            f(Field::M11, 1.0),
            f(Field::M02, 0.0),
            f(Field::M12, 0.0)
        )
    }

    /// Draw a node and its children. The root is drawn at its own origin.
    fn node(&mut self, node: &SceneNode, root: bool) {
        self.node_with(node, root, &[]);
    }

    fn node_with(&mut self, node: &SceneNode, root: bool, overrides: &[&model::SymbolOverride]) {
        if self.at.len() >= MAX_LAYERS {
            self.notes.insert(TOO_DEEP.into());
            return;
        }
        if self.drawn == MAX_DRAWN {
            self.notes.insert(TOO_MANY.into());
            return;
        }
        self.drawn += 1;
        let over = layer_override(overrides, node);
        let over = over.as_ref();
        // What the enclosing instances' component properties set on this
        // layer: its text, its visibility, its component.
        let prop = |field: ComponentPropNodeField| self.property(node, field);
        let (prop_text, prop_swap) = (prop(ComponentPropNodeField::TextData), prop(ComponentPropNodeField::OverriddenSymbolId));
        let swap = over.and_then(|o| o.symbol_id).or_else(|| prop_swap.as_deref().and_then(|s| s.parse().ok()));
        let inherited = self.inherited(node, swap);
        let node = inherited.as_ref().unwrap_or(node);
        // The size Figma resolved for this layer in the instance drawing it.
        let resized = self.cached(node, Field::Size).and_then(|size| {
            let mut node = node.clone();
            node.width = size.field(Field::X)?.as_f32()?;
            node.height = size.field(Field::Y)?.as_f32()?;
            Some(node)
        });
        let node = resized.as_ref().unwrap_or(node);
        if !self.shown(node, over) && !root {
            return;
        }
        // An instance shows its component's own modes, under its own.
        let mut own_modes = match self.component_drawn(node, swap).filter(|_| node.node_type == NodeType::Instance) {
            Some(component) => self.explicit_modes(component.id),
            None => HashMap::new(),
        };
        own_modes.extend(self.explicit_modes(node.id));
        self.modes.push(own_modes);
        // Inside an instance the layers are its component's, storing the
        // component's values: what is bound resolves in the modes in effect
        // here, as Figma draws it.
        let resolved = (!self.derived.is_empty() || inherited.is_some()).then(|| self.resolved(node)).flatten();
        let node = resolved.as_ref().unwrap_or(node);
        let transform = if root { String::new() } else { format!(" transform=\"{}\"", self.matrix(node)) };
        let (parent, parent_opacity) = self.at.last().copied().unwrap_or(([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 1.0));
        let local = if root { [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] } else { self.matrix_values(node) };
        self.at.push((contrast::compose(parent, local), parent_opacity * node.opacity));
        let opacity = if node.opacity < 1.0 { format!(" opacity=\"{}\"", node.opacity) } else { String::new() };
        let filter = self.effects(&node.effects);
        let blend = if root { String::new() } else { node.blend_mode.as_deref().map_or_else(String::new, |mode| self.blend(mode)) };
        let holds = node.node_type == NodeType::Instance || !node.children.is_empty();
        // A blend isolates by itself; a container stored normal must be told to.
        let isolation = if blend.is_empty() && holds && node.blend_mode.as_deref() == Some(BlendMode::Normal.as_wire()) { " style=\"isolation:isolate\"" } else { "" };
        let grouped = !opacity.is_empty() || !blend.is_empty() || !isolation.is_empty();
        self.groups += usize::from(grouped);
        let _ = write!(self.out, "<g{transform}{opacity}{filter}{blend}{isolation}>");
        let own_paint = self.out.len();

        let fills = over.and_then(|o| o.fills.as_deref()).map(|f| self.resolve_paints(f)).unwrap_or_else(|| node.fills.clone());
        let strokes = over.and_then(|o| o.strokes.as_deref()).map(|f| self.resolve_paints(f)).unwrap_or_else(|| node.strokes.clone());
        // A shape with no stored geometry, as grafta writes one,
        // is drawn from its own values, as Figma derives it.
        let derived = node.fill_geometry.is_empty() && node.stroke_geometry.is_empty() && self.derived_shape(node, &fills, &strokes);
        let operands = self.draws_children(node);
        if !derived {
        match node.node_type {
            t if t.is_generated() => self.generated(node),
            NodeType::Stamp => {
                self.notes.insert("a stamp is FigJam's own art, which the file does not hold: not drawn".into());
            }
            NodeType::Text => self.text(node, &fills, over.and_then(|o| o.text.as_deref()).or(prop_text.as_deref())),
            NodeType::BooleanOperation if operands && self.graph.children(node.id).next().is_some() => {
                self.notes.insert("a boolean with no stored outline is drawn as its operands, not combined".into());
            }
            t if t.is_vector_shape() => self.vector(node, &fills, &strokes),
            NodeType::Ellipse if node.fill_geometry.is_empty() => {
                let shape = format!(
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"",
                    node.width / 2.0,
                    node.height / 2.0,
                    node.width / 2.0,
                    node.height / 2.0
                );
                self.paint_shape(&shape, node, &fills);
                self.stroke_box(node, &strokes);
            }
            NodeType::Ellipse => self.vector(node, &fills, &strokes),
            NodeType::Document | NodeType::Canvas => {}
            _ => {
                let shape = self.box_shape(node);
                self.paint_shape(&shape, node, &fills);
                if !node.stroke_geometry.is_empty() {
                    self.paths(&node.stroke_geometry, &strokes, node);
                } else {
                    self.stroke_box(node, &strokes);
                }
            }
        }
        }

        // Children, clipped to the frame when it clips.
        let clip = if node.clips_content && !matches!(node.node_type, NodeType::Group) {
            let id = self.id("c");
            let shape = self.box_shape(node);
            let _ = write!(self.defs, "<clipPath id=\"{id}\">{shape}/></clipPath>");
            format!(" clip-path=\"url(#{id})\"")
        } else {
            String::new()
        };
        let own_paint = own_paint..self.out.len();
        let blends = self.blends;
        let _ = write!(self.out, "<g{clip}>");
        let inside = self.out.len();
        if node.node_type == NodeType::Instance {
            self.instance(node, overrides, swap, over);
        } else if operands {
            let children: Vec<&SceneNode> = self.graph.children(node.id).collect();
            self.children(node, &children, overrides);
        }
        if !clip.is_empty() && self.blends > blends {
            self.blend_inside_clip(own_paint, inside, &fills);
        }
        self.groups -= usize::from(grouped);
        self.out.push_str("</g></g>");
        self.modes.pop();
        self.at.pop();
    }

    /// A text as it would be drawn here, for the contrast check: its
    /// colours as drawn, the layer's and each run's, with their sizes.
    fn text_sample(&self, node: &SceneNode, fills: &[Fill], override_text: Option<&str>) -> contrast::TextSample {
        let style = node.text_style.clone().unwrap_or_default();
        let (at, opacity) = self.at.last().copied().unwrap_or(([1.0, 0.0, 0.0, 1.0, 0.0, 0.0], 1.0));
        let mut unmeasurable = None;
        let mut runs = Vec::new();
        let mut colours = |paints: &[Fill], size: f32, weight: u32, runs: &mut Vec<([f32; 4], f32, u32)>| {
            for paint in paints.iter().filter(|p| p.visible) {
                match (paint.fill_type, paint.color) {
                    (FillType::Solid, Some(c)) => runs.push(([c.r, c.g, c.b, c.a * paint.opacity], size, weight)),
                    _ => unmeasurable = Some("its fill is not a solid colour, so it has no one colour to measure".to_string()),
                }
            }
        };
        // Only what is drawn is measured: a layer's colour no character shows,
        // or a run's over nothing but spaces, is no colour of this text.
        let text_runs: &[model::TextRun] = if override_text.is_none() { &node.text_runs } else { &[] };
        let (layer_shows, shown) = colours_shown(override_text.or(node.text.as_deref()).unwrap_or_default(), text_runs);
        if layer_shows {
            colours(fills, style.font_size, style.font_weight, &mut runs);
        }
        for (run, _) in text_runs.iter().zip(&shown).filter(|(_, shown)| **shown) {
            let size = run.font_size.unwrap_or(style.font_size);
            let weight = run.font_style.as_deref().map_or(style.font_weight, model::style_to_weight);
            // A run with no visible fill of its own is drawn in the layer's.
            match run.fills.as_deref().filter(|paints| paints.iter().any(|p| p.visible)) {
                Some(paints) => colours(paints, size, weight, &mut runs),
                None if !layer_shows || size != style.font_size || weight != style.font_weight => colours(fills, size, weight, &mut runs),
                None => {}
            }
        }
        if runs.is_empty() && unmeasurable.is_none() {
            unmeasurable = Some("it has no visible fill".to_string());
        }
        contrast::TextSample {
            id: node.id,
            instance: self.instances.last().copied(),
            name: node.name.clone(),
            characters: override_text.map(str::to_string).or_else(|| node.text.clone()).unwrap_or_default(),
            at,
            width: node.width,
            height: node.height,
            opacity,
            runs,
            unmeasurable,
        }
    }

    /// The modes a node sets itself (`variableModeBySetMap`), collection →
    /// mode.
    fn explicit_modes(&self, id: Guid) -> HashMap<Guid, Guid> {
        self.raw
            .get(&id)
            .and_then(|raw| raw.field(Field::VariableModeBySetMap))
            .and_then(|m| m.field(Field::Entries))
            .and_then(Raw::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| Some((Guid::read(e.field(Field::VariableSetId)?.field(Field::Guid)?)?, Guid::read(e.field(Field::VariableModeId)?)?)))
            .collect()
    }

    /// A variable's value in the modes in effect, through aliases, each in
    /// its own collection's mode. `None` for a variable the file
    /// does not have.
    fn resolve_variable(&self, variable: Guid) -> Option<model::VariableValue> {
        let mut current = variable;
        for _ in 0..16 {
            let v = self.graph.variables.get(&current)?;
            let mode = self.modes.iter().rev().find_map(|m| m.get(&v.collection_id)).copied().or_else(|| {
                self.graph.collections.get(&v.collection_id).and_then(|c| c.modes.first()).and_then(|m| m.id.declared())
            });
            let value = mode.and_then(|m| v.values_by_mode.get(&m)).or_else(|| v.values_by_mode.first().map(|(_, value)| value))?;
            match value {
                model::VariableValue::Alias { variable_id } => current = *variable_id,
                other => return Some(other.clone()),
            }
        }
        None
    }

    /// Paints with each bound colour resolved; a paint keeps its own opacity.
    fn resolve_paints(&self, paints: &[Fill]) -> Vec<Fill> {
        paints
            .iter()
            .map(|paint| {
                let mut paint = paint.clone();
                if let Some(model::VariableValue::Color { value }) = paint.variable_id.and_then(|v| self.resolve_variable(v)) {
                    paint.color = Some(model::Color { a: 1.0, ..value });
                }
                paint
            })
            .collect()
    }

    /// The node with what is bound resolved in the modes in effect: paints,
    /// opacity and corner radii. A bound field drawn as stored that resolves
    /// otherwise here is named in the caption. `None` when nothing
    /// changes.
    fn resolved(&mut self, node: &SceneNode) -> Option<SceneNode> {
        if node.bound_variables.is_empty() && !node.fills.iter().chain(&node.strokes).any(|p| p.variable_id.is_some()) {
            return None;
        }
        let mut out = node.clone();
        out.fills = self.resolve_paints(&node.fills);
        out.strokes = self.resolve_paints(&node.strokes);
        for (field, variable) in &node.bound_variables {
            let Some(model::VariableValue::Float { value }) = self.resolve_variable(*variable) else { continue };
            let stored = match field.as_str() {
                "opacity" => {
                    out.opacity = value;
                    continue;
                }
                "cornerRadius" => {
                    out.corner_radii = model::CornerRadii { top_left: value, top_right: value, bottom_right: value, bottom_left: value };
                    continue;
                }
                "cornerRadiusTopLeft" => {
                    out.corner_radii.top_left = value;
                    continue;
                }
                "cornerRadiusTopRight" => {
                    out.corner_radii.top_right = value;
                    continue;
                }
                "cornerRadiusBottomRight" => {
                    out.corner_radii.bottom_right = value;
                    continue;
                }
                "cornerRadiusBottomLeft" => {
                    out.corner_radii.bottom_left = value;
                    continue;
                }
                "width" => node.width,
                "height" => node.height,
                "strokeWeight" => node.stroke_weight,
                "itemSpacing" => node.layout.item_spacing,
                "counterAxisSpacing" => node.layout.counter_axis_spacing,
                "paddingTop" => node.layout.padding_top,
                "paddingRight" => node.layout.padding_right,
                "paddingBottom" => node.layout.padding_bottom,
                "paddingLeft" => node.layout.padding_left,
                "fontSize" => node.text_style.as_ref().map_or(value, |t| t.font_size),
                _ => continue,
            };
            if (stored - value).abs() > 0.01 {
                self.notes.insert(format!(
                    "{field} bound to a variable is drawn as the component stores it, {stored}, though it is {value} in the mode shown: grafta does not lay an instance out again for a mode"
                ));
            }
        }
        Some(out)
    }

    /// An instance grafta created stores none of its component's style, which
    /// Figma derives on open (A2), and records what it changes about itself in
    /// the override entry addressed to its component. Such an instance
    /// takes from its component each of paints, stroke, dashes, radii and
    /// effects that entry does not hold, as Figma showed it.
    /// `None` for an instance that stores its own style, as Figma's do, or
    /// for anything else.
    fn inherited(&self, node: &SceneNode, swap: Option<Guid>) -> Option<SceneNode> {
        if node.node_type != NodeType::Instance {
            return None;
        }
        let raw = self.raw.get(&node.id)?;
        let component = self.component_drawn(node, swap)?;
        let key = component.override_key.unwrap_or(component.id);
        let own: Vec<Field> = raw
            .field(Field::SymbolData)
            .and_then(|s| s.field(Field::SymbolOverrides))
            .and_then(Raw::as_array)
            .into_iter()
            .flatten()
            .find(|entry| {
                let path = entry.field(Field::GuidPath).and_then(|p| p.field(Field::Guids)).and_then(Raw::as_array);
                path.is_some_and(|p| p.len() == 1 && Guid::read(&p[0]) == Some(key))
            })
            .and_then(|entry| match entry {
                Raw::Object(fields) => Some(fields.keys().filter_map(Field::from_wire).collect()),
                _ => None,
            })
            .unwrap_or_default();
        let styled = [Field::FillPaints, Field::StrokePaints, Field::CornerRadius, Field::Effects].iter().any(|k| raw.has(*k) && !own.contains(k));
        if styled {
            return None;
        }
        let mut node = node.clone();
        if !own.contains(&Field::FillPaints) {
            node.fills.clone_from(&component.fills);
        }
        if !own.contains(&Field::StrokePaints) {
            node.strokes.clone_from(&component.strokes);
        }
        if !own.contains(&Field::StrokeWeight) {
            node.stroke_weight = component.stroke_weight;
        }
        if !own.contains(&Field::StrokeAlign) {
            node.stroke_align.clone_from(&component.stroke_align);
        }
        if !own.contains(&Field::DashPattern) {
            node.dash_pattern.clone_from(&component.dash_pattern);
        }
        node.corner_radii = component.corner_radii;
        node.effects.clone_from(&component.effects);
        node.clips_content = component.clips_content;
        // And what the component binds, to resolve in the modes in effect.
        for (field, variable) in &component.bound_variables {
            node.bound_variables.entry(field.clone()).or_insert(*variable);
        }
        Some(node)
    }

    /// An instance is drawn from its component's layers, with its overrides
    ///. An instance inside another takes, first, the outer
    /// instance's overrides whose path runs through it, the first step taken
    /// off, and then its own: the outer ones win, as in Figma. `swap` is a
    /// component the outer instance put in place of this one's, and `over`
    /// what its entries there say of it, its property values among them.
    fn instance(&mut self, node: &SceneNode, outer: &[&model::SymbolOverride], swap: Option<Guid>, over: Option<&model::SymbolOverride>) {
        if swap.is_some_and(|id| self.graph.node(id).is_none_or(|c| c.node_type != NodeType::Component)) {
            self.notes.insert("an instance swapped to a component from another file is drawn as its own component".into());
        }
        let Some(component) = self.component_drawn(node, swap) else {
            self.notes.insert("an instance of a component from another file is drawn as its box only".into());
            return;
        };
        let key = node.override_key.unwrap_or(node.id);
        let through: Vec<model::SymbolOverride> = outer
            .iter()
            .filter(|o| o.path.len() > 1 && o.path[0] == key)
            .map(|o| model::SymbolOverride { path: o.path[1..].to_vec(), ..(*o).clone() })
            .collect();
        let overrides: Vec<&model::SymbolOverride> = through.iter().chain(node.symbol_overrides.iter()).collect();
        let children: Vec<&SceneNode> = self.graph.children(component.id).collect();
        // Figma's expansion: the outer instance's entries through this one
        // first, the first step taken off, then this instance's own.
        let mut scope: Vec<CachedEntry<'a>> = self
            .derived
            .last()
            .map(|outer| {
                outer
                    .iter()
                    .filter(|cached| cached.path.len() > 1 && cached.path[0] == key)
                    .map(|cached| CachedEntry { path: cached.path[1..].to_vec(), entry: cached.entry, outdated: cached.outdated })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(entries) = self.raw.get(&node.id).and_then(|nc| nc.field(Field::DerivedSymbolData)).and_then(Raw::as_array) {
            // A text overridden from outside was set after this instance laid
            // it out: only where it put the layer still holds.
            scope.extend(entries.iter().filter_map(|entry| {
                let path = guid_path(entry)?;
                let outdated = through.iter().any(|o| o.text.is_some() && o.path == path);
                Some(CachedEntry { path, entry, outdated })
            }));
        }
        // A cached expansion is laid out for this instance's size; only
        // without one is it drawn at its component's.
        if scope.is_empty() && ((component.width - node.width).abs() > 0.5 || (component.height - node.height).abs() > 0.5) {
            self.notes.insert("a resized instance is drawn at its component's layout".into());
        }
        let mut assigned = node.property_assignments.clone();
        assigned.extend(over.into_iter().flat_map(|o| o.assignments.clone()));
        self.properties.push(assigned);
        self.derived.push(scope);
        self.instances.push(node.id);
        self.children(node, &children, &overrides);
        self.derived.pop();
        self.properties.pop();
        self.instances.pop();
    }

    /// The component an instance shows: the one swapped in, when the file holds
    /// it, or its own.
    fn component_drawn(&self, node: &SceneNode, swap: Option<Guid>) -> Option<&'a SceneNode> {
        swap.and_then(|id| self.graph.node(id)).filter(|c| c.node_type == NodeType::Component).or_else(|| self.graph.component_of(node))
    }

    /// What Figma's cached expansion of the innermost instance says about this
    /// layer's `field`: from the first entry that holds it, as Figma stores only
    /// what differs at each level, outer entries first.
    fn cached(&self, node: &SceneNode, field: Field) -> Option<&'a Raw> {
        let key = node.override_key.unwrap_or(node.id);
        self.derived
            .last()?
            .iter()
            .filter(|cached| cached.path.len() == 1 && cached.path[0] == key)
            .filter(|cached| !cached.outdated || field == Field::Transform)
            .find_map(|cached| cached.entry.field(field))
    }

    /// The value the innermost enclosing instance assigns to the component
    /// property this layer's `field` is bound to, if it is bound
    /// (`componentPropRefs`) and assigned (`componentPropAssignments`).
    fn property(&self, node: &SceneNode, field: ComponentPropNodeField) -> Option<String> {
        let refs = self.raw.get(&node.id)?.field(Field::ComponentPropRefs)?.as_array()?;
        let definition = refs.iter().find(|r| r.is(Field::ComponentPropNodeField, field))?;
        let id = Guid::read(definition.field(Field::DefId)?)?;
        self.properties.iter().rev().find_map(|assigned| assigned.get(&id).cloned())
    }

    /// A rectangle with the node's corner radii, as an SVG element without its
    /// closing `/>`.
    fn box_shape(&self, node: &SceneNode) -> String {
        let r = node.corner_radii;
        let (w, h) = (node.width, node.height);
        if r.is_uniform() {
            format!("<rect width=\"{w}\" height=\"{h}\" rx=\"{}\"", r.top_left.min(w / 2.0).min(h / 2.0))
        } else {
            let (tl, tr, br, bl) = (r.top_left, r.top_right, r.bottom_right, r.bottom_left);
            format!(
                "<path d=\"M{tl} 0H{}A{tr} {tr} 0 0 1 {w} {tr}V{}A{br} {br} 0 0 1 {} {h}H{bl}A{bl} {bl} 0 0 1 0 {}V{tl}A{tl} {tl} 0 0 1 {tl} 0Z\"",
                w - tr,
                h - br,
                w - br,
                h - bl
            )
        }
    }

    /// Fill a shape once per paint, bottom paint first, as Figma stacks them.
    /// The outline Figma derives for a line, a polygon, a star or an ellipse
    /// with an arc, from the node's own values; `false` for anything else.
    fn derived_shape(&mut self, node: &SceneNode, fills: &[Fill], strokes: &[Fill]) -> bool {
        use std::f32::consts::{FRAC_PI_2, PI, TAU};
        let (w, h) = (node.width, node.height);
        let (cx, cy, rx, ry) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
        let point = |angle: f32, scale: f32| (cx + rx * scale * angle.cos(), cy + ry * scale * angle.sin());
        let polygon = |points: Vec<(f32, f32)>| {
            let mut d = String::new();
            for (i, (x, y)) in points.iter().enumerate() {
                let _ = write!(d, "{}{x} {y} ", if i == 0 { "M" } else { "L" });
            }
            d.push('Z');
            d
        };
        let d = match node.node_type {
            NodeType::Line => format!("M0 0 L{w} 0"),
            NodeType::Polygon => {
                let n = node.point_count.unwrap_or(3).max(3);
                polygon((0..n).map(|k| point(-FRAC_PI_2 + TAU * k as f32 / n as f32, 1.0)).collect())
            }
            NodeType::Star => {
                let n = node.point_count.unwrap_or(5).max(3);
                let inner = node.star_inner_scale.unwrap_or(0.382);
                polygon((0..2 * n).map(|k| point(-FRAC_PI_2 + PI * k as f32 / n as f32, if k % 2 == 0 { 1.0 } else { inner })).collect())
            }
            NodeType::Ellipse => {
                let Some((start, end, inner)) = node.arc else { return false };
                let sweep = (end - start).clamp(-TAU, TAU);
                let arc = |from: f32, to: f32, scale: f32, forward: bool| {
                    // Two halves, so a full turn is drawable.
                    let mid = (from + to) / 2.0;
                    let (mx, my) = point(mid, scale);
                    let (ex, ey) = point(to, scale);
                    let s = u8::from(forward);
                    format!("A{} {} 0 0 {s} {mx} {my} A{} {} 0 0 {s} {ex} {ey}", rx * scale, ry * scale, rx * scale, ry * scale)
                };
                let (sx, sy) = point(start, 1.0);
                if (sweep.abs() - TAU).abs() < 1e-3 {
                    let mut d = format!("M{sx} {sy} {} Z", arc(start, start + TAU, 1.0, true));
                    if inner > 0.0 {
                        let (ix, iy) = point(start, inner);
                        let _ = write!(d, " M{ix} {iy} {} Z", arc(start, start + TAU, inner, true));
                    }
                    d
                } else if inner > 0.0 {
                    let (ix, iy) = point(start + sweep, inner);
                    format!("M{sx} {sy} {} L{ix} {iy} {} Z", arc(start, start + sweep, 1.0, sweep > 0.0), arc(start + sweep, start, inner, sweep < 0.0))
                } else {
                    format!("M{cx} {cy} L{sx} {sy} {} Z", arc(start, start + sweep, 1.0, sweep > 0.0))
                }
            }
            _ => return false,
        };
        if node.node_type != NodeType::Line {
            for fill in fills.iter().filter(|f| f.visible) {
                if let Some(paint) = self.paint(fill, node) {
                    let _ = write!(self.out, "<path d=\"{d}\" fill-rule=\"evenodd\" {paint}/>");
                }
            }
        }
        if node.stroke_weight > 0.0 {
            if node.stroke_align != StrokeAlign::Center.as_wire() && !strokes.is_empty() {
                self.notes.insert("a shape's inside or outside stroke is drawn centred".into());
            }
            for stroke in strokes.iter().filter(|f| f.visible) {
                if let Some(paint) = self.paint(stroke, node) {
                    let paint = paint.replace("fill=", "stroke=").replace("fill-opacity=", "stroke-opacity=");
                    let _ = write!(self.out, "<path d=\"{d}\" fill=\"none\" stroke-width=\"{}\"{} {paint}/>", node.stroke_weight, dashes(node));
                }
            }
        }
        true
    }

    fn paint_shape(&mut self, shape: &str, node: &SceneNode, fills: &[Fill]) {
        for fill in fills.iter().filter(|f| f.visible) {
            if self.image_fill(&format!("{shape}/>"), fill, node) {
                continue;
            }
            if let Some(paint) = self.paint(fill, node) {
                let _ = write!(self.out, "{shape} {paint}/>");
            }
        }
    }

    fn vector(&mut self, node: &SceneNode, fills: &[Fill], strokes: &[Fill]) {
        // What the enclosing instance stores for this layer, each field on its
        // own: the node's outlines are its component's, at the component's size.
        let stored = |field: Field| self.cached(node, field).and_then(Raw::as_array).map(Vec::as_slice);
        let (fill_stored, stroke_stored) = (stored(Field::FillGeometry), stored(Field::StrokeGeometry));
        if node.fill_geometry.is_empty() && node.stroke_geometry.is_empty() && fill_stored.is_none() && stroke_stored.is_none() {
            if !self.network(node, fills, strokes) {
                self.notes.insert("a vector without stored geometry or a vector network is not drawn".into());
            }
            return;
        }
        self.outlines(fill_stored, &node.fill_geometry, fills, node);
        self.outlines(stroke_stored, &node.stroke_geometry, strokes, node);
    }

    /// A boolean's operands are only geometry: the file stores their combined
    /// outline, which shows in the boolean's paints, and they show nothing of
    /// their own. Only with no such outline are they all there is to draw.
    fn draws_children(&self, node: &SceneNode) -> bool {
        node.node_type != NodeType::BooleanOperation
            || (node.fill_geometry.is_empty()
                && node.stroke_geometry.is_empty()
                && self.cached(node, Field::FillGeometry).is_none()
                && self.cached(node, Field::StrokeGeometry).is_none())
    }

    /// One field's outlines: those the instance stores, or the component's, at
    /// the size the instance gave the layer.
    fn outlines(&mut self, stored: Option<&'a [Raw]>, own: &[model::VectorPath], paints: &[Fill], node: &SceneNode) {
        if let Some(stored) = stored {
            let mut paths = Vec::new();
            for geometry in stored {
                match self.outline(geometry) {
                    Some(path) => paths.push(path),
                    None if self.outline_is_empty(geometry) => {}
                    None => {
                        self.notes.insert("a vector's stored outline cannot be read: not drawn".into());
                    }
                }
            }
            self.paths(&paths, paints, node);
            return;
        }
        let source = self.graph.nodes.get(&node.id);
        let resized = |size: f32, of: f32| (size - of).abs() >= 0.01;
        if own.is_empty() || !source.is_some_and(|s| resized(node.width, s.width) || resized(node.height, s.height)) {
            self.paths(own, paints, node);
            return;
        }
        // A flat axis has no scale that reaches a size: it stays as it is.
        let scale = |size: f32, of: f32| if of > 0.0 { size / of } else { 1.0 };
        let (sx, sy) = source.map_or((1.0, 1.0), |s| (scale(node.width, s.width), scale(node.height, s.height)));
        self.notes.insert("a vector in a resized instance is scaled from its component's outlines".into());
        let _ = write!(self.out, "<g transform=\"scale({sx} {sy})\">");
        self.paths(own, paints, node);
        self.out.push_str("</g>");
    }

    /// One stored outline, if the file holds the blob it names.
    fn outline(&self, geometry: &Raw) -> Option<model::VectorPath> {
        let blob = self.fig.blobs.get(geometry.field(Field::CommandsBlob)?.as_u32()? as usize)?;
        let (d, _) = model::vector::decode_path(blob)?;
        let fill_rule = if geometry.is(Field::WindingRule, WindingRule::Odd) { "evenodd" } else { "nonzero" };
        Some(model::VectorPath { d, fill_rule: fill_rule.into() })
    }

    /// Whether a stored outline has no points: Figma stores these, and there is
    /// nothing to draw and nothing missing.
    fn outline_is_empty(&self, geometry: &Raw) -> bool {
        let blob = geometry.field(Field::CommandsBlob).and_then(Raw::as_u32).and_then(|b| self.fig.blobs.get(b as usize));
        blob.is_some_and(|blob| blob.iter().all(|&command| command == 0))
    }

    /// A vector drawn from its network, as Figma derives its geometry: the
    /// regions filled, the segments stroked with the node's weight, caps and
    /// joins. The network is in `normalizedSize` units, scaled to the size.
    fn network(&mut self, node: &SceneNode, fills: &[Fill], strokes: &[Fill]) -> bool {
        let Some(raw) = self.raw.get(&node.id).copied() else { return false };
        let Some(data) = raw.field(Field::VectorData) else { return false };
        let Some(blob) = data.field(Field::VectorNetworkBlob).and_then(Raw::as_u32) else { return false };
        let Some(network) = self.fig.blobs.get(blob as usize).and_then(|b| model::path::Network::decode(b)) else {
            return false;
        };
        let normalized = |key: Field, size: f32| {
            data.field(Field::NormalizedSize).and_then(|n| n.field(key)).and_then(Raw::as_f32).filter(|v| *v > 0.0).unwrap_or(size)
        };
        let scale = (node.width / normalized(Field::X, node.width), node.height / normalized(Field::Y, node.height));
        if let Some((d, nonzero)) = network.fill_path(scale) {
            let rule = if nonzero { "nonzero" } else { "evenodd" };
            for fill in fills.iter().filter(|f| f.visible) {
                if let Some(paint) = self.paint(fill, node) {
                    let _ = write!(self.out, "<path d=\"{d}\" fill-rule=\"{rule}\" {paint}/>");
                }
            }
        }
        if node.stroke_weight > 0.0 {
            let d = network.stroke_path(scale);
            let cap = match raw.known::<StrokeCap>(Field::StrokeCap) {
                Some(StrokeCap::Round) => "round",
                Some(StrokeCap::Square) => "square",
                _ => "butt",
            };
            let join = match raw.known::<StrokeJoin>(Field::StrokeJoin) {
                Some(StrokeJoin::Round) => "round",
                Some(StrokeJoin::Bevel) => "bevel",
                _ => "miter",
            };
            if node.stroke_align != StrokeAlign::Center.as_wire() && !strokes.is_empty() {
                self.notes.insert("a vector's inside or outside stroke is drawn centred".into());
            }
            for stroke in strokes.iter().filter(|f| f.visible) {
                if let Some(paint) = self.paint(stroke, node) {
                    let paint = paint.replace("fill=", "stroke=").replace("fill-opacity=", "stroke-opacity=");
                    let _ = write!(
                        self.out,
                        "<path d=\"{d}\" fill=\"none\" stroke-width=\"{}\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\"{} {paint}/>",
                        node.stroke_weight,
                        dashes(node)
                    );
                }
            }
        }
        true
    }

    fn paths(&mut self, paths: &[model::VectorPath], paints: &[Fill], node: &SceneNode) {
        for fill in paints.iter().filter(|f| f.visible) {
            if fill.fill_type == FillType::Image {
                let outline: String = paths.iter().map(|path| format!("<path d=\"{}\" fill-rule=\"{}\"/>", path_data(&path.d), path.fill_rule)).collect();
                self.image_fill(&outline, fill, node);
                continue;
            }
            if let Some(paint) = self.paint(fill, node) {
                for path in paths {
                    let _ = write!(self.out, "<path d=\"{}\" fill-rule=\"{}\" {paint}/>", path_data(&path.d), path.fill_rule);
                }
            }
        }
    }

    /// A box stroke, for a node whose stroke Figma did not outline.
    fn stroke_box(&mut self, node: &SceneNode, strokes: &[Fill]) {
        let w = node.stroke_weight;
        if w <= 0.0 {
            return;
        }
        let inset = match StrokeAlign::from_wire(&node.stroke_align) {
            Some(StrokeAlign::Inside) => w / 2.0,
            Some(StrokeAlign::Outside) => -w / 2.0,
            _ => 0.0,
        };
        for fill in strokes.iter().filter(|f| f.visible && f.fill_type == FillType::Solid) {
            let Some(color) = fill.color else { continue };
            let rx = node.corner_radii.top_left;
            let _ = write!(
                self.out,
                "<rect x=\"{inset}\" y=\"{inset}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"none\" stroke=\"{}\" stroke-opacity=\"{}\" stroke-width=\"{w}\"{}/>",
                (node.width - 2.0 * inset).max(0.0),
                (node.height - 2.0 * inset).max(0.0),
                (rx - inset).max(0.0),
                color.to_hex(),
                fill.opacity * color.a,
                dashes(node)
            );
        }
    }

    /// `fill="…"` and its opacity for one paint, defining a gradient or an
    /// image pattern when it needs one.
    fn paint(&mut self, fill: &Fill, node: &SceneNode) -> Option<String> {
        let paint = self.colour(fill, node)?;
        let blend = fill.blend_mode.as_deref().map_or_else(String::new, |mode| self.blend(mode));
        let gradient = matches!(fill.fill_type, FillType::GradientLinear | FillType::GradientRadial);
        self.gradient_in_group |= gradient && (self.groups > 0 || !blend.is_empty());
        Some(paint + &blend)
    }

    /// A blend mode as an SVG attribute: nothing for a normal one, or for one
    /// the renderer has not, which the notes name.
    fn blend(&mut self, mode: &str) -> String {
        let css = match BlendMode::from_wire(mode) {
            Some(BlendMode::Multiply) => "multiply",
            Some(BlendMode::Screen) => "screen",
            Some(BlendMode::Overlay) => "overlay",
            Some(BlendMode::Darken) => "darken",
            Some(BlendMode::Lighten) => "lighten",
            Some(BlendMode::ColorDodge) => "color-dodge",
            Some(BlendMode::ColorBurn) => "color-burn",
            Some(BlendMode::HardLight) => "hard-light",
            Some(BlendMode::SoftLight) => "soft-light",
            Some(BlendMode::Difference) => "difference",
            Some(BlendMode::Exclusion) => "exclusion",
            Some(BlendMode::Hue) => "hue",
            Some(BlendMode::Saturation) => "saturation",
            Some(BlendMode::Color) => "color",
            Some(BlendMode::Luminosity) => "luminosity",
            // The nearest the renderer has: within 20 of 255 on average, and 64 at
            // worst, where normal is 84 and 255. Its own has neither, and SVG's
            // filters cannot read what lies behind a layer.
            Some(BlendMode::LinearBurn) => {
                self.notes.insert(format!("{mode} is not a blend the renderer has: drawn as multiply, so midtones come out lighter than in Figma"));
                "multiply"
            }
            Some(BlendMode::LinearDodge) => {
                self.notes.insert(format!("{mode} is not a blend the renderer has: drawn as screen, so midtones come out darker than in Figma"));
                "screen"
            }
            _ => return String::new(),
        };
        self.blends += 1;
        // Only as a style: usvg does not read it as an attribute.
        format!(" style=\"mix-blend-mode:{css}\"")
    }

    /// A clip makes a layer of its own, so what it holds blends only with what
    /// else it holds. The frame's own paint moves inside it, to be blended with;
    /// what lies behind the frame stays out of reach, which shows only when the
    /// frame does not cover it.
    fn blend_inside_clip(&mut self, own_paint: std::ops::Range<usize>, inside: usize, fills: &[Fill]) {
        let paint = self.out[own_paint.clone()].to_string();
        self.out.insert_str(inside, &paint);
        self.out.replace_range(own_paint, "");
        let covers = fills.iter().any(|f| {
            f.visible && f.opacity >= 1.0 && match f.fill_type {
                FillType::Solid => f.color.is_some_and(|c| c.a >= 1.0),
                FillType::Image => true,
                _ => false,
            }
        });
        if !covers {
            self.notes.insert("a blend inside a clipped frame reaches what the frame holds and its own paint, not what lies behind it".into());
        }
    }

    /// An image paint as an element filling the layer, if the file embeds it.
    fn image(&mut self, fill: &Fill, node: &SceneNode) -> Option<String> {
        let hash = fill.image_hash.as_deref()?;
        let Some((_, bytes)) = self.graph.images.iter().find(|(name, _)| name == hash) else {
            self.notes.insert("an image the file references but does not embed is not drawn".into());
            return None;
        };
        let mime = if bytes.starts_with(b"\x89PNG") { "image/png" } else { "image/jpeg" };
        let mode = fill.image_scale_mode.as_deref().and_then(ImageScaleMode::from_wire);
        let fit = match mode {
            Some(ImageScaleMode::Fit) => "xMidYMid meet",
            Some(ImageScaleMode::Stretch) => "none",
            _ => "xMidYMid slice",
        };
        if mode == Some(ImageScaleMode::Tile) {
            self.notes.insert("a tiled image is drawn once, filling the layer".into());
        }
        Some(format!(
            "<image width=\"{w}\" height=\"{h}\" preserveAspectRatio=\"{fit}\" xlink:href=\"data:{mime};base64,{}\"/>",
            base64(bytes),
            w = node.width,
            h = node.height
        ))
    }

    /// An image paint, cut to `outline`: not a pattern, which Apple's PDF
    /// renderer misplaces inside a group. Whether `fill` was one.
    fn image_fill(&mut self, outline: &str, fill: &Fill, node: &SceneNode) -> bool {
        if fill.fill_type != FillType::Image {
            return false;
        }
        if let Some(image) = self.image(fill, node) {
            let clip = self.id("c");
            let outline = outline.replace(" fill-rule=", " clip-rule=");
            let _ = write!(self.defs, "<clipPath id=\"{clip}\">{outline}</clipPath>");
            let opacity = if fill.opacity < 1.0 { format!(" opacity=\"{}\"", fill.opacity) } else { String::new() };
            let blend = fill.blend_mode.as_deref().map_or_else(String::new, |mode| self.blend(mode));
            let _ = write!(self.out, "<g clip-path=\"url(#{clip})\"{opacity}{blend}>{image}</g>");
        }
        true
    }

    fn colour(&mut self, fill: &Fill, node: &SceneNode) -> Option<String> {
        match fill.fill_type {
            FillType::Solid => {
                let color = fill.color?;
                Some(format!("fill=\"{}\" fill-opacity=\"{}\"", color.to_hex(), fill.opacity * color.a))
            }
            FillType::GradientLinear | FillType::GradientRadial => {
                let id = self.id("g");
                let m = fill.gradient_transform.unwrap_or_default();
                // Figma's transform maps the layer's unit square to gradient
                // space; SVG wants the inverse, in the layer's own units.
                let det = m.m00 * m.m11 - m.m01 * m.m10;
                if det.abs() < 1e-9 {
                    return None;
                }
                let (a, b, c, d) = (m.m11 / det, -m.m10 / det, -m.m01 / det, m.m00 / det);
                let e = -(a * m.m02 + c * m.m12);
                let f = -(b * m.m02 + d * m.m12);
                let (w, h) = (node.width, node.height);
                let transform = format!("matrix({} {} {} {} {} {})", a * w, b * h, c * w, d * h, e * w, f * h);
                let stops: String = fill
                    .gradient_stops
                    .iter()
                    .map(|s| format!("<stop offset=\"{}\" stop-color=\"{}\" stop-opacity=\"{}\"/>", s.position, s.color.to_hex(), s.color.a))
                    .collect();
                let element = if fill.fill_type == FillType::GradientLinear {
                    format!("<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0.5\" x2=\"1\" y2=\"0.5\" gradientTransform=\"{transform}\">{stops}</linearGradient>")
                } else {
                    format!("<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" cx=\"0.5\" cy=\"0.5\" r=\"0.5\" gradientTransform=\"{transform}\">{stops}</radialGradient>")
                };
                self.defs.push_str(&element);
                Some(format!("fill=\"url(#{id})\" fill-opacity=\"{}\"", fill.opacity))
            }
            FillType::Image => {
                // A stroke or a text cannot hold an image any other way.
                let image = self.image(fill, node)?;
                let id = self.id("p");
                let _ = write!(self.defs, "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{w}\" height=\"{h}\">{image}</pattern>", w = node.width, h = node.height);
                Some(format!("fill=\"url(#{id})\" fill-opacity=\"{}\"", fill.opacity))
            }
            _ => {
                self.notes.insert(format!("{:?} paints are not drawn", fill.fill_type));
                None
            }
        }
    }

    /// Shadows and blurs as one SVG filter. Each shadow is
    /// cast from the layer's silhouette on its own: drop shadows grown or
    /// shrunk by their spread, offset, blurred and coloured, beneath the
    /// layer; inner shadows the same, cut to what the offset silhouette leaves
    /// uncovered, above it. A layer blur blurs the result. Background blurs
    /// are drawn by [`Svg::backdrop`], not here.
    fn effects(&mut self, effects: &[Effect]) -> String {
        let visible: Vec<&Effect> = effects.iter().filter(|e| e.visible).collect();
        let mut body = String::new();
        let (mut below, mut above) = (Vec::new(), Vec::new());
        let mut blur = None;
        for (i, effect) in visible.iter().enumerate() {
            let color = effect.color.unwrap_or(model::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.25 });
            let flood = format!("flood-color=\"{}\" flood-opacity=\"{}\"", color.to_hex(), color.a);
            let std = effect.radius / 2.0;
            match effect.effect_type {
                EffectType::DropShadow | EffectType::InnerShadow => {
                    let inner = effect.effect_type == EffectType::InnerShadow;
                    // A drop shadow's spread grows the silhouette; an inner
                    // shadow's shrinks the shape that holds it back.
                    let grow = if inner { -effect.spread } else { effect.spread };
                    let source = if grow.abs() > 0.0 {
                        let op = if grow > 0.0 { "dilate" } else { "erode" };
                        let _ = write!(body, "<feMorphology in=\"SourceAlpha\" operator=\"{op}\" radius=\"{}\" result=\"m{i}\"/>", grow.abs());
                        format!("m{i}")
                    } else {
                        "SourceAlpha".to_string()
                    };
                    let _ = write!(
                        body,
                        "<feOffset in=\"{source}\" dx=\"{}\" dy=\"{}\" result=\"o{i}\"/><feGaussianBlur in=\"o{i}\" stdDeviation=\"{std}\" result=\"b{i}\"/><feFlood {flood} result=\"f{i}\"/>",
                        effect.offset_x, effect.offset_y
                    );
                    if inner {
                        let _ = write!(
                            body,
                            "<feComposite in=\"SourceAlpha\" in2=\"b{i}\" operator=\"out\" result=\"u{i}\"/><feComposite in=\"f{i}\" in2=\"u{i}\" operator=\"in\" result=\"s{i}\"/>"
                        );
                        above.push(format!("s{i}"));
                    } else {
                        let _ = write!(body, "<feComposite in=\"f{i}\" in2=\"b{i}\" operator=\"in\" result=\"s{i}\"/>");
                        below.push(format!("s{i}"));
                    }
                }
                EffectType::LayerBlur => blur = Some(std),
                EffectType::BackgroundBlur | EffectType::Unknown => {}
            }
        }
        if below.is_empty() && above.is_empty() && blur.is_none() {
            return String::new();
        }
        let nodes: String = below
            .iter()
            .map(String::as_str)
            .chain(["SourceGraphic"])
            .chain(above.iter().map(String::as_str))
            .map(|n| format!("<feMergeNode in=\"{n}\"/>"))
            .collect();
        let _ = write!(body, "<feMerge result=\"merged\">{nodes}</feMerge>");
        if let Some(std) = blur {
            let _ = write!(body, "<feGaussianBlur in=\"merged\" stdDeviation=\"{std}\"/>");
        }
        let id = self.id("f");
        let _ = write!(self.defs, "<filter id=\"{id}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\">{body}</filter>");
        format!(" filter=\"url(#{id})\"")
    }

    /// What a layer with a background blur shows through it:
    /// its parent's fill and the siblings drawn before it, drawn again,
    /// blurred, and clipped to the layer's shape. SVG cannot read what is
    /// already on the canvas, so the backdrop is rebuilt from the layers.
    /// Layers further behind, outside the parent, are not in it; the notes
    /// say so when there are any.
    fn backdrop(&mut self, parent: &SceneNode, before: &[&SceneNode], layer: &SceneNode, overrides: &[&model::SymbolOverride]) {
        let Some(radius) = layer
            .effects
            .iter()
            .find(|e| e.visible && e.effect_type == EffectType::BackgroundBlur)
            .map(|e| e.radius)
        else {
            return;
        };
        let clip = self.id("c");
        let shape = self.box_shape(layer);
        let matrix = self.matrix(layer);
        let _ = write!(self.defs, "<clipPath id=\"{clip}\">{shape} transform=\"{matrix}\"/></clipPath>");
        let blur = self.id("f");
        let _ = write!(
            self.defs,
            "<filter id=\"{blur}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"{}\"/></filter>",
            radius / 2.0
        );
        let _ = write!(self.out, "<g clip-path=\"url(#{clip})\"><g filter=\"url(#{blur})\">");
        if !matches!(parent.node_type, NodeType::Canvas | NodeType::Document) {
            let shape = self.box_shape(parent);
            self.paint_shape(&shape, parent, &parent.fills);
        }
        self.layers(None, before, overrides);
        self.out.push_str("</g></g>");
        let behind_parent = parent
            .parent_id
            .and_then(|p| self.graph.node(p))
            .is_some_and(|p| !matches!(p.node_type, NodeType::Canvas | NodeType::Document));
        if behind_parent {
            self.notes.insert(
                "a background blur blurs its parent's fill and the layers before it in that parent, not what lies further behind".into(),
            );
        }
    }

    /// Draw children in order, each with what shows through it when it
    /// blurs its background.
    fn children(&mut self, parent: &SceneNode, children: &[&SceneNode], overrides: &[&model::SymbolOverride]) {
        self.layers(Some(parent), children, overrides);
    }

    /// Draw layers of one parent in order. A visible mask is not painted: the
    /// layers after it show only where it lets them. Those before it are
    /// untouched. With a parent, each also shows what its background blur
    /// rebuilds.
    fn layers(&mut self, parent: Option<&SceneNode>, layers: &[&SceneNode], overrides: &[&model::SymbolOverride]) {
        let mut masked = false;
        let mut masked_by_clip = false;
        for (i, layer) in layers.iter().enumerate() {
            if layer.is_mask && self.shown(layer, layer_override(overrides, layer).as_ref()) {
                if masked {
                    self.out.push_str("</g>");
                    self.groups -= usize::from(!masked_by_clip);
                    self.notes.insert("two masks in one parent: each is taken to mask up to the next, which no file has measured".into());
                }
                if let Some(clip) = self.plain_mask(layer, overrides) {
                    let _ = write!(self.out, "<g clip-path=\"url(#{clip})\">");
                    masked_by_clip = true;
                    masked = true;
                    continue;
                }
                let mask = self.mask(layer, overrides);
                let _ = write!(self.out, "<g mask=\"url(#{mask})\">");
                self.groups += 1;
                masked_by_clip = false;
                masked = true;
                continue;
            }
            if let Some(parent) = parent {
                self.backdrop(parent, &layers[..i], layer, overrides);
            }
            self.node_with(layer, false, overrides);
        }
        if masked {
            self.out.push_str("</g>");
            self.groups -= usize::from(!masked_by_clip);
        }
    }

    /// A mask that only says where, as a clip: a shape of its own, whole and
    /// sharp, whose paint cannot change what it lets through. Any other needs
    /// the drawing `mask` makes.
    fn plain_mask(&mut self, layer: &SceneNode, overrides: &[&model::SymbolOverride]) -> Option<String> {
        let opaque = |f: &Fill| f.visible && f.opacity >= 1.0 && f.fill_type == FillType::Solid && f.color.is_some_and(|c| c.a >= 1.0);
        let paints = if layer.mask_type == MaskType::Outline.as_wire() {
            layer.fills.iter().any(|f| f.visible)
        } else {
            layer.mask_type == MaskType::Alpha.as_wire() && layer.fills.iter().any(opaque)
        };
        let shape = matches!(layer.node_type, NodeType::Rectangle | NodeType::RoundedRectangle) || (layer.node_type.is_vector_shape() && !layer.fill_geometry.is_empty()) || (layer.node_type == NodeType::Ellipse && !layer.fill_geometry.is_empty());
        let drawn_plainly = layer.opacity >= 1.0
            && !layer.effects.iter().any(|e| e.visible)
            && !layer.strokes.iter().any(|f| f.visible)
            && self.graph.children(layer.id).next().is_none()
            && layer_override(overrides, layer).is_none()
            && self.cached(layer, Field::Size).is_none()
            && self.cached(layer, Field::FillGeometry).is_none();
        if !(paints && shape && drawn_plainly) {
            return None;
        }
        let transform = self.matrix(layer);
        let outline = if layer.fill_geometry.is_empty() {
            format!("{} transform=\"{transform}\"/>", self.box_shape(layer))
        } else {
            layer
                .fill_geometry
                .iter()
                .map(|path| format!("<path d=\"{}\" clip-rule=\"{}\" transform=\"{transform}\"/>", path_data(&path.d), path.fill_rule))
                .collect()
        };
        let clip = self.id("c");
        let _ = write!(self.defs, "<clipPath id=\"{clip}\">{outline}</clipPath>");
        Some(clip)
    }

    /// A mask's drawing, to mask its later siblings with, by what it stores:
    /// its alpha, its luminance, or its outline whatever its paint. Its region
    /// is all of user space, so that it cuts by nothing but what it draws.
    fn mask(&mut self, layer: &SceneNode, overrides: &[&model::SymbolOverride]) -> String {
        const REGION: &str = "x=\"-100000\" y=\"-100000\" width=\"200000\" height=\"200000\"";
        let texts = self.texts.take();
        let around = std::mem::take(&mut self.out);
        self.groups += 1;
        self.node_with(layer, false, overrides);
        self.groups -= 1;
        let mut drawing = std::mem::replace(&mut self.out, around);
        self.texts = texts;
        let kind = if layer.mask_type == MaskType::Luminance.as_wire() { "" } else { " mask-type=\"alpha\"" };
        if layer.mask_type == MaskType::Outline.as_wire() {
            let whole = self.id("f");
            let _ = write!(
                self.defs,
                "<filter id=\"{whole}\" filterUnits=\"userSpaceOnUse\" {REGION}><feComponentTransfer><feFuncA type=\"linear\" slope=\"255\"/></feComponentTransfer></filter>"
            );
            drawing = format!("<g filter=\"url(#{whole})\">{drawing}</g>");
        }
        let id = self.id("m");
        let _ = write!(self.defs, "<mask id=\"{id}\" maskUnits=\"userSpaceOnUse\" {REGION}{kind}>{drawing}</mask>");
        id
    }

    /// Whether a layer shows where it is drawn: by its instance's override, by
    /// the component property it is bound to, or as it stores.
    fn shown(&self, node: &SceneNode, over: Option<&model::SymbolOverride>) -> bool {
        over.and_then(|o| o.visible)
            .or_else(|| self.property(node, ComponentPropNodeField::Visible).map(|v| v == "true"))
            .unwrap_or(node.visible)
    }

    /// Text: Figma's own glyph outlines when the file has them, a system font
    /// when it does not.
    fn text(&mut self, node: &SceneNode, fills: &[Fill], override_text: Option<&str>) {
        if let Some(mut texts) = self.texts.take() {
            texts.push(self.text_sample(node, fills, override_text));
            self.texts = Some(texts);
            return;
        }
        // Figma's own layout of this text as the instance shows it, when the
        // expansion has one: exact, overridden or not.
        let glyphs = match self.cached(node, Field::DerivedTextData) {
            Some(laid_out) => self.glyphs_from(laid_out),
            None if override_text.is_none() => self.glyphs(node),
            None => {
                self.notes.insert("an overridden text is laid out on its own: the instance around it keeps its size".into());
                None
            }
        };
        let paint = fills
            .iter()
            .find(|f| f.visible)
            .and_then(|fill| self.paint(fill, node))
            .unwrap_or_else(|| "fill=\"#000000\"".to_string());
        match glyphs {
            Some(paths) => {
                for (d, x, y, size) in paths {
                    let _ = write!(
                        self.out,
                        "<path d=\"{d}\" transform=\"translate({x} {y}) scale({size} {neg})\" {paint}/>",
                        neg = -size
                    );
                }
            }
            None if self.lay_out(node, &paint, override_text) => {}
            None => {
                self.notes.insert(
                    "text without Figma's layout data (text grafta wrote, or an overridden label) is drawn with a system font, so its width and wrapping are approximate".into(),
                );
                let style = node.text_style.clone().unwrap_or_default();
                let characters = override_text.map(str::to_string).or_else(|| node.text.clone()).unwrap_or_default();
                let size = style.font_size;
                let line = style.line_height.map_or(size * 1.2, |l| l.to_px(size));
                let anchor = match TextAlignHorizontal::from_wire(&style.text_align_horizontal) {
                    Some(TextAlignHorizontal::Center) => ("middle", node.width / 2.0),
                    Some(TextAlignHorizontal::Right) => ("end", node.width),
                    _ => ("start", 0.0),
                };
                let family = escape(&style.font_family);
                let _ = write!(
                    self.out,
                    "<text xml:space=\"preserve\" font-family=\"{family}, Inter, Helvetica, Arial, sans-serif\" font-size=\"{size}\" font-weight=\"{}\" text-anchor=\"{}\" {paint}>",
                    style.font_weight,
                    anchor.0
                );
                for (i, paragraph) in characters.split('\n').enumerate() {
                    let y = size * 0.8 + line * i as f32;
                    let _ = write!(self.out, "<tspan x=\"{}\" y=\"{y}\">{}</tspan>", anchor.1, escape(paragraph));
                }
                self.out.push_str("</text>");
            }
        }
    }

    /// Text with no layout of Figma's, laid out by grafta: each character
    /// in its run's font, size, paint, decoration, case and letter spacing
    ///, or the layer's; lines broken at the text's width across runs, as
    /// tall as their tallest run; and each paragraph with its list marker and
    /// indent as Figma sets them, its first-line indent and the spacing after
    /// it. The notes say how close that is, as measured against Figma's own
    /// layout. `false` when the machine has no font at all.
    fn lay_out(&mut self, node: &SceneNode, paint: &str, override_text: Option<&str>) -> bool {
        let Some(TextLayout { styles, chars, lines, height: y }) = self.layout(node, paint, override_text) else { return false };
        let layer = node.text_style.clone().unwrap_or_default();
        self.draw_lines(node, &layer, &styles, &chars, &lines, y);
        true
    }

    /// Give each character its text's font lacks a style of its own, in a font
    /// that has it, so that one character changes neither the font of the rest
    /// nor their line's height. The notes name them, as a drawing that differs
    /// from what the file says should.
    fn fall_back(&mut self, db: &resvg::usvg::fontdb::Database, styles: &mut Vec<RunStyle>, chars: &mut [Styled]) {
        let mut found: std::collections::HashMap<(char, u32, bool), Option<text::Font>> = std::collections::HashMap::new();
        let mut made: std::collections::HashMap<(usize, resvg::usvg::fontdb::ID), usize> = std::collections::HashMap::new();
        let mut missing: std::collections::BTreeMap<(String, Option<String>), Vec<char>> = std::collections::BTreeMap::new();
        let mut before = None;
        for styled in chars.iter_mut() {
            let shown = styled.shown.chars().next().unwrap_or(' ');
            // These only modify the character before, and go where it went.
            if matches!(shown, '\u{200D}' | '\u{FE00}'..='\u{FE0F}') {
                styled.style = before.unwrap_or(styled.style);
                continue;
            }
            let own = styled.style;
            before = Some(own);
            let lacking = |c: &char| !c.is_whitespace() && !c.is_control() && !text::has_char(db, styles[own].font.id, *c);
            let Some(c) = styled.shown.chars().find(lacking) else { continue };
            let (weight, italic) = (styles[own].weight, styles[own].italic);
            let font = found.entry((c, weight, italic)).or_insert_with(|| text::fallback(db, c, weight, italic)).clone();
            let named = missing.entry((styles[own].font.family.clone(), font.as_ref().map(|f| f.family.clone()))).or_default();
            if !named.contains(&c) {
                named.push(c);
            }
            let Some(font) = font else { continue };
            let style = *made.entry((own, font.id)).or_insert_with(|| {
                let mut style = styles[own].clone();
                style.font = font;
                styles.push(style);
                styles.len() - 1
            });
            styled.style = style;
            before = Some(style);
        }
        for ((family, fallback), shown) in missing {
            let mut listed: Vec<String> = shown.iter().take(MISSING_NAMED).map(|c| format!("{c} (U+{:04X})", u32::from(*c))).collect();
            if shown.len() > MISSING_NAMED {
                listed.push(format!("and {} more", shown.len() - MISSING_NAMED));
            }
            let listed = listed.join(", ");
            self.notes.insert(match fallback {
                Some(fallback) => format!("{listed} not in {family}, so drawn in {fallback}; Figma may choose another font for them"),
                None => format!("{listed} not in {family} or any font installed here, so drawn as a box"),
            });
        }
    }

    /// The layout `lay_out` draws, and `measure_text` sizes a text by.
    /// `None` when the machine has no font at all.
    fn layout(&mut self, node: &SceneNode, paint: &str, override_text: Option<&str>) -> Option<TextLayout> {
        let db = text::fonts();
        let layer = node.text_style.clone().unwrap_or_default();
        let characters = override_text.map(str::to_string).or_else(|| node.text.clone()).unwrap_or_default();
        // An overridden label's characters are not the ones the runs index.
        let runs: &[model::TextRun] = if override_text.is_some() { &[] } else { &node.text_runs };
        let paragraphs: &[model::Paragraph] = if override_text.is_some() { &[] } else { &node.paragraphs };

        // The styles: the layer's, then each run's over it.
        let mut styles = vec![self.text_style_for(&db, node, &layer, None, paint)];
        styles[0].as_ref()?;
        for run in runs {
            let style = self.text_style_for(&db, node, &layer, Some(run), paint);
            styles.push(style.or_else(|| styles[0].clone()));
        }
        let mut styles: Vec<RunStyle> = styles.into_iter().map(|s| s.expect("the layer's font was found")).collect();

        // Each character: what is drawn for it, its style, its advance.
        let mut chars: Vec<Styled> = Vec::new();
        let mut offset = 0;
        let mut word_start = true;
        for c in characters.chars() {
            let style = runs.iter().position(|r| (r.start..r.end).contains(&offset)).map_or(0, |i| i + 1);
            offset += c.len_utf16();
            let shown = match TextCase::from_wire(&styles[style].case) {
                Some(TextCase::Upper) => c.to_uppercase().collect(),
                Some(TextCase::Lower) => c.to_lowercase().collect(),
                Some(TextCase::Title) if word_start => c.to_uppercase().collect(),
                _ => c.to_string(),
            };
            word_start = c.is_whitespace();
            chars.push(Styled { shown, style, advance: 0.0 });
        }
        self.fall_back(&db, &mut styles, &mut chars);
        for (i, style) in styles.iter().enumerate() {
            let mine: Vec<usize> = (0..chars.len()).filter(|&c| chars[c].style == i && chars[c].shown != "\n").collect();
            let joined: String = mine.iter().map(|&c| chars[c].shown.as_str()).collect();
            let advances = text::advances(&db, style.font.id, &joined, style.size, style.spacing);
            let mut at = 0;
            for &c in &mine {
                let n = chars[c].shown.chars().count();
                chars[c].advance = advances[at..at + n].iter().sum();
                at += n;
            }
        }

        // Paragraphs into lines.
        let limit = (layer.text_auto_resize != TextAutoResize::WidthAndHeight.as_wire()).then_some(node.width);
        let mut lines: Vec<TextLine> = Vec::new();
        let mut levels: Vec<(u32, String, u32)> = Vec::new();
        let mut y = 0.0;
        let mut start = 0;
        let count = characters.split('\n').count();
        for (p, paragraph) in characters.split('\n').enumerate() {
            let end = start + paragraph.chars().count();
            let first_style = chars.get(start).filter(|_| end > start).map_or(0, |c| c.style);
            let em = styles[first_style].size;
            let list = paragraphs.get(p).filter(|l| l.list_type != model::LIST_NONE && l.indentation > 0);
            let level = list.map_or(0, |l| l.indentation);
            let indent = 1.5 * em * level as f32;
            // A list starts where the previous item at its level is of another type.
            let number = list.map(|l| {
                levels.retain(|(lv, _, _)| *lv <= level);
                let n = match levels.iter().rev().find(|(lv, _, _)| *lv == level) {
                    Some((_, kind, n)) if *kind == l.list_type => n + 1,
                    _ => 1,
                };
                levels.retain(|(lv, _, _)| *lv != level);
                levels.push((level, l.list_type.clone(), n));
                n
            });
            if list.is_none() {
                levels.clear();
            }
            let glyphs: Vec<char> = (start..end).map(|i| chars[i].shown.chars().next().unwrap_or(' ')).collect();
            let advances: Vec<f32> = (start..end).map(|i| chars[i].advance).collect();
            let broken = text::break_paragraph(&glyphs, &advances, limit.map(|w| w - indent), layer.paragraph_indent);
            for (i, l) in broken.iter().enumerate() {
                let range = start + l.start..start + l.end;
                let on_line: Vec<usize> = if range.is_empty() { vec![first_style] } else { range.clone().map(|c| chars[c].style).collect() };
                let height = on_line.iter().map(|&s| styles[s].line).fold(0.0, f32::max);
                let ascent = on_line.iter().map(|&s| styles[s].ascent).fold(0.0, f32::max);
                let descent = on_line.iter().map(|&s| styles[s].descent).fold(0.0, f32::max);
                let x = indent + if i == 0 { layer.paragraph_indent } else { 0.0 };
                let marker = match (i, list, number) {
                    (0, Some(l), Some(n)) => {
                        let style = &styles[first_style];
                        let (shown, width) = if l.list_type == model::LIST_ORDERED {
                            let shown = format!("{n}.");
                            let width: f32 = text::advances(&db, style.font.id, &shown, style.size, 0.0).iter().sum();
                            (shown, width)
                        } else {
                            ("•".to_string(), text::advances(&db, style.font.id, "•", style.size, 0.0).iter().sum())
                        };
                        // A bullet centred in its level's 1.5 em; a number ending
                        // 0.33 em before the text (a census of the fixtures).
                        let at = if l.list_type == model::LIST_ORDERED { indent - 0.33 * em - width } else { indent - 1.5 * em + (1.5 * em - width) / 2.0 };
                        Some((shown, at, first_style))
                    }
                    _ => None,
                };
                lines.push(TextLine { range, x, width: l.width, top: y, height, ascent, descent, marker });
                y += height;
            }
            if p + 1 < count {
                y += layer.paragraph_spacing;
            }
            start = end + 1;
        }
        Some(TextLayout { styles, chars, lines, height: y })
    }

    /// Draw laid-out lines.
    fn draw_lines(&mut self, node: &SceneNode, layer: &model::TextStyle, styles: &[RunStyle], chars: &[Styled], lines: &[TextLine], y: f32) {
        // A box of fixed height places its lines; one that hugs them does not.
        let top = if layer.text_auto_resize == TextAutoResize::None.as_wire() {
            match TextAlignVertical::from_wire(&layer.text_align_vertical) {
                Some(TextAlignVertical::Center) => (node.height - y) / 2.0,
                Some(TextAlignVertical::Bottom) => node.height - y,
                _ => 0.0,
            }
        } else {
            0.0
        };
        for line in lines {
            // Each line's glyphs sit centred in its line height, as Figma sets them.
            let baseline = top + line.top + (line.height - line.ascent - line.descent) / 2.0 + line.ascent;
            let shift = match TextAlignHorizontal::from_wire(&layer.text_align_horizontal) {
                Some(TextAlignHorizontal::Center) => (node.width - line.x - line.width) / 2.0,
                Some(TextAlignHorizontal::Right) => node.width - line.x - line.width,
                _ => 0.0,
            };
            if let Some((shown, at, style)) = &line.marker {
                self.segment(&styles[*style], at + shift, baseline, shown, TextDecoration::None.as_wire());
            }
            // Segments of one style, at their own x.
            let mut x = line.x + shift;
            let mut c = line.range.start;
            while c < line.range.end {
                let style = chars[c].style;
                let mut end = c;
                let mut shown = String::new();
                let mut width = 0.0;
                while end < line.range.end && chars[end].style == style {
                    shown.push_str(&chars[end].shown);
                    width += chars[end].advance;
                    end += 1;
                }
                let shown = if end == line.range.end { shown.trim_end().to_string() } else { shown };
                self.segment(&styles[style], x, baseline, &shown, &styles[style].decoration);
                x += width;
                c = end;
            }
        }
    }

    /// A style a text's characters are drawn in: the layer's, or a run's over
    /// it; its font found as the text layout finds one, and the caption told. `None` when
    /// the machine has no font at all.
    fn text_style_for(&mut self, db: &resvg::usvg::fontdb::Database, node: &SceneNode, layer: &model::TextStyle, run: Option<&model::TextRun>, paint: &str) -> Option<RunStyle> {
        let family = run.and_then(|r| r.font_family.clone()).unwrap_or_else(|| layer.font_family.clone());
        let style_name = run.and_then(|r| r.font_style.clone()).unwrap_or_else(|| layer.font_style.clone());
        let (weight, italic) = if run.is_some_and(|r| r.font_style.is_some()) {
            (model::style_to_weight(&style_name), style_name.to_lowercase().contains("italic"))
        } else {
            (layer.font_weight, layer.italic)
        };
        let font = text::find(db, &family, weight, italic)?;
        if font.substituted {
            self.notes.insert(format!(
                "{family} is not installed here, so text grafta laid out in it is drawn in {}: widths within about 2–3% and some line breaks differ, measured against Figma's own layout",
                font.family
            ));
        } else {
            self.notes.insert(format!(
                "text without Figma's layout data is laid out by grafta in {}, without kerning: widths within about 2% of Figma's, measured",
                font.family
            ));
        }
        let size = run.and_then(|r| r.font_size).unwrap_or(layer.font_size);
        let case = run.and_then(|r| r.text_case.clone()).unwrap_or_else(|| layer.text_case.clone());
        if matches!(TextCase::from_wire(&case), Some(TextCase::SmallCaps | TextCase::SmallCapsForced)) {
            self.notes.insert("small capitals are drawn as written".into());
        }
        let paint = match run.and_then(|r| r.fills.as_ref()).and_then(|f| f.iter().find(|f| f.visible)) {
            Some(fill) => self.paint(fill, node).unwrap_or_else(|| paint.to_string()),
            None => paint.to_string(),
        };
        let (ascent, descent, gap) = text::vertical(db, font.id, size);
        let line_height = run.and_then(|r| r.line_height).or(layer.line_height);
        Some(RunStyle {
            spacing: run.and_then(|r| r.letter_spacing).or(layer.letter_spacing).map_or(0.0, |l| l.to_px(size)),
            line: line_height.map_or(ascent + descent + gap, |l| l.to_px(size)),
            decoration: run.and_then(|r| r.text_decoration.clone()).unwrap_or_else(|| layer.text_decoration.clone()),
            font,
            weight,
            italic,
            size,
            case,
            paint,
            ascent,
            descent,
        })
    }

    /// One run of characters in one style, from `x` on the baseline `y`.
    fn segment(&mut self, style: &RunStyle, x: f32, y: f32, shown: &str, decoration: &str) {
        if shown.is_empty() {
            return;
        }
        let italic = if style.italic { " font-style=\"italic\"" } else { "" };
        let decoration = match TextDecoration::from_wire(decoration) {
            Some(TextDecoration::Underline) => " text-decoration=\"underline\"",
            Some(TextDecoration::Strikethrough) => " text-decoration=\"line-through\"",
            _ => "",
        };
        let _ = write!(
            self.out,
            "<text xml:space=\"preserve\" x=\"{x}\" y=\"{y}\" font-family=\"{}\" font-size=\"{}\" font-weight=\"{}\"{italic} letter-spacing=\"{}\"{decoration} {}>{}</text>",
            escape(&style.font.family),
            style.size,
            style.weight,
            style.spacing,
            style.paint,
            escape(shown)
        );
    }

    /// Each glyph's outline, position and size, from `derivedTextData`.
    fn glyphs(&self, node: &SceneNode) -> Option<Vec<(String, f32, f32, f32)>> {
        self.glyphs_from(self.raw.get(&node.id)?.field(Field::DerivedTextData)?)
    }

    /// A FigJam node's generated layers, drawn as FigJam last laid them
    /// out: each layer of `derivedImmutableFrameData` at its transform (inside
    /// its parent layer's, for a path of more than one id), its `fillGeometry`
    /// and `strokeGeometry` in the fills and strokes the same layer has in
    /// `nodeGenerationData`, and its text from its glyphs.
    fn generated(&mut self, node: &SceneNode) {
        let Some(raw) = self.raw.get(&node.id).copied() else { return };
        let own: HashMap<Vec<Guid>, &Raw> = raw
            .field(Field::NodeGenerationData)
            .and_then(|g| g.field(Field::Overrides))
            .and_then(Raw::as_array)
            .into_iter()
            .flatten()
            .filter_map(|o| Some((guid_path(o)?, o)))
            .collect();
        let Some(layers) = raw.field(Field::DerivedImmutableFrameData).and_then(|d| d.field(Field::Overrides)).and_then(Raw::as_array) else {
            // Changed by grafta, so not yet laid out by FigJam: drawn from the
            // values grafta wrote.
            match node.node_type {
                NodeType::Table => self.generated_table(node, raw, &own),
                NodeType::Connector => self.generated_connector(node, raw, &own),
                _ => {
                    if let Some(background) = own.get([GeneratedLayer::Background.guid()].as_slice()) {
                        let radius = background.field(Field::CornerRadius).and_then(Raw::as_f32).unwrap_or(0.0);
                        let ellipse = raw.is(Field::ShapeWithTextType, ShapeWithTextType::Ellipse);
                        for (hex, alpha) in background.field(Field::FillPaints).and_then(Raw::as_array).map(|p| solid_paints(p)).unwrap_or_default() {
                            if ellipse {
                                let (rx, ry) = (node.width / 2.0, node.height / 2.0);
                                let _ = write!(self.out, "<ellipse cx=\"{rx}\" cy=\"{ry}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{hex}\" fill-opacity=\"{alpha}\"/>");
                            } else {
                                let _ = write!(self.out, "<rect width=\"{}\" height=\"{}\" rx=\"{radius}\" fill=\"{hex}\" fill-opacity=\"{alpha}\"/>", node.width, node.height);
                            }
                        }
                    }
                    if let Some(text) = own.get([GeneratedLayer::Text.guid()].as_slice()) {
                        self.generated_text(node, text);
                    }
                }
            }
            self.notes.insert("a FigJam sticky or shape grafta changed is drawn with its text laid out by grafta: FigJam lays it out again when the file is opened".into());
            return;
        };
        let transforms: HashMap<Vec<Guid>, String> = layers.iter().filter_map(|l| Some((guid_path(l)?, raw_matrix(l.field(Field::Transform)?)))).collect();
        for layer in layers {
            let Some(path) = guid_path(layer) else { continue };
            let values = own.get(&path).copied();
            // A sticky's author is generated hidden unless the board shows it.
            if values.and_then(|v| v.field(Field::Visible)).and_then(Raw::as_bool) == Some(false) {
                continue;
            }
            let solid = |field: Field| values.and_then(|v| v.field(field)).and_then(Raw::as_array).map(|paints| solid_paints(paints)).unwrap_or_default();
            // The layer it sits in: in a table, a cell's text `[:1, row, col]`
            // sits in the cell `[:0, row, col]` (the fixture's five tables).
            let mut chain: Vec<&String> = Vec::new();
            if let [first, rest @ ..] = path.as_slice() {
                if !rest.is_empty() {
                    let cell: Vec<Guid> =
                        std::iter::once(Guid::new(first.session_id, GeneratedLayer::Background.local_id())).chain(rest.iter().copied()).collect();
                    if cell != path {
                        chain.extend(transforms.get(&cell));
                    }
                }
            }
            chain.extend(transforms.get(&path));
            let depth = chain.len();
            for m in chain {
                let _ = write!(self.out, "<g transform=\"{m}\">");
            }
            for (field, paints) in [(Field::FillGeometry, solid(Field::FillPaints)), (Field::StrokeGeometry, solid(Field::StrokePaints))] {
                for geometry in layer.field(field).and_then(Raw::as_array).into_iter().flatten() {
                    let Some(path) = self.outline(geometry) else { continue };
                    for (hex, alpha) in &paints {
                        let _ = write!(self.out, "<path d=\"{}\" fill-rule=\"{}\" fill=\"{hex}\" fill-opacity=\"{alpha}\"/>", path_data(&path.d), path.fill_rule);
                    }
                }
            }
            if let Some(text) = layer.field(Field::DerivedTextData) {
                let fill = solid(Field::FillPaints).into_iter().next().unwrap_or(("#000000".into(), 1.0));
                // A colour emoji's stored outline is a box, not its art: left
                // out and said, rather than drawn as a black square.
                let characters: Vec<u16> = values.and_then(|v| v.field(Field::TextData)).and_then(|t| t.field(Field::Characters)).and_then(Raw::as_str).map(|c| c.encode_utf16().collect()).unwrap_or_default();
                let emoji_at = |index: usize| char::decode_utf16(characters.get(index..).unwrap_or_default().iter().copied()).next().and_then(Result::ok).is_some_and(is_emoji);
                let glyphs = self.glyphs_indexed(text).unwrap_or_default();
                for (d, x, y, size, first) in glyphs {
                    if emoji_at(first) {
                        self.notes.insert("colour emoji in FigJam text are not drawn: the file holds a box for each, not its art".into());
                        continue;
                    }
                    let _ = write!(
                        self.out,
                        "<path d=\"{d}\" transform=\"translate({x} {y}) scale({size} {neg})\" fill=\"{}\" fill-opacity=\"{}\"/>",
                        fill.0,
                        fill.1,
                        neg = -size
                    );
                }
            }
            for _ in 0..depth {
                self.out.push_str("</g>");
            }
        }
    }

    /// The text grafta wrote into a FigJam node's generated text layer,
    /// laid out as FigJam places it: a sticky's from its top left, a shape's
    /// centred from top to bottom and aligned as its layer says, both inside a
    /// margin, in the layer's own font and size.
    fn generated_text(&mut self, node: &SceneNode, layer: &Raw) {
        let Some(characters) = layer.field(Field::TextData).and_then(|t| t.field(Field::Characters)).and_then(Raw::as_str) else { return };
        if characters.trim().is_empty() {
            return;
        }
        let family = layer.field(Field::FontName).and_then(|f| f.field(Field::Family)).and_then(Raw::as_str).unwrap_or("Inter").to_string();
        let style = layer.field(Field::FontName).and_then(|f| f.field(Field::Style)).and_then(Raw::as_str).unwrap_or("Medium").to_lowercase();
        let weight = if style.contains("bold") { 700 } else if style.contains("semi") { 600 } else if style.contains("medium") { 500 } else { 400 };
        let size = layer.field(Field::FontSize).and_then(Raw::as_f32).unwrap_or(16.0);
        let line = layer
            .field(Field::LineHeight)
            .filter(|l| l.is(Field::Units, NumberUnits::Pixels))
            .and_then(|l| l.field(Field::Value))
            .and_then(Raw::as_f32)
            .unwrap_or(size * 1.4);
        let (hex, alpha) = layer.field(Field::FillPaints).and_then(Raw::as_array).map(|p| solid_paints(p)).unwrap_or_default().into_iter().next().unwrap_or(("#1E1E1E".into(), 1.0));
        let sticky = node.node_type == NodeType::Sticky;
        let margin = if sticky { 24.0 } else { 16.0 };
        let db = text::fonts();
        let Some(font) = text::find(&db, &family, weight, false) else { return };
        let advances = text::advances(&db, font.id, characters, size, 0.0);
        let lines = text::break_lines(characters, &advances, Some((node.width - 2.0 * margin).max(size)));
        let chars: Vec<char> = characters.chars().collect();
        let (ascent, _, _) = text::vertical(&db, font.id, size);
        let block = line * lines.len() as f32;
        let top = if sticky { margin } else { (node.height - block) / 2.0 };
        for (i, l) in lines.iter().enumerate() {
            let shown: String = chars[l.start..l.end].iter().collect::<String>().trim_end().to_string();
            let x = match layer.known::<TextAlignHorizontal>(Field::TextAlignHorizontal) {
                Some(TextAlignHorizontal::Center) if !sticky => (node.width - l.width) / 2.0,
                Some(TextAlignHorizontal::Right) if !sticky => node.width - margin - l.width,
                _ => margin,
            };
            let y = top + i as f32 * line + (line - size) / 2.0 + ascent;
            let _ = write!(
                self.out,
                "<text xml:space=\"preserve\" x=\"{x}\" y=\"{y}\" font-family=\"{}\" font-size=\"{size}\" font-weight=\"{weight}\" fill=\"{hex}\" fill-opacity=\"{alpha}\">{}</text>",
                escape(&family),
                escape(&shown)
            );
        }
    }

    /// A table grafta wrote: its rows and columns at their sizes, each
    /// cell's text from the top left of its cell, the header row shaded.
    fn generated_table(&mut self, node: &SceneNode, raw: &Raw, own: &HashMap<Vec<Guid>, &Raw>) {
        let ordered = |axis: TableAxis, total: f32| -> Vec<(Guid, f32)> {
            let entries = table_entries(raw, axis);
            let known = table_sizes(raw, axis);
            // An entry with no size shares what the sized ones leave.
            let unsized_count = entries.iter().filter(|(_, id)| !known.contains_key(id)).count().max(1) as f32;
            let rest = ((total - known.values().sum::<f32>()) / unsized_count).max(0.0);
            entries.into_iter().map(|(_, id)| (id, known.get(&id).copied().unwrap_or(rest))).collect()
        };
        let rows = ordered(TableAxis::Rows, node.height);
        let columns = ordered(TableAxis::Columns, node.width);
        let _ = write!(self.out, "<rect width=\"{}\" height=\"{}\" rx=\"12\" fill=\"#FFFFFF\" stroke=\"#D9D9D9\" stroke-width=\"2\"/>", node.width, node.height);
        let db = text::fonts();
        let font = text::find(&db, "Inter", 500, false);
        let mut y = 0.0;
        for (r, (row, height)) in rows.iter().enumerate() {
            if r == 0 {
                let _ = write!(self.out, "<rect width=\"{}\" height=\"{height}\" fill=\"#F5F5F5\"/>", node.width);
            } else {
                let _ = write!(self.out, "<rect y=\"{y}\" width=\"{}\" height=\"2\" fill=\"#D9D9D9\"/>", node.width);
            }
            let mut x = 0.0;
            for (c, (column, width)) in columns.iter().enumerate() {
                if r == 0 && c > 0 {
                    let _ = write!(self.out, "<rect x=\"{x}\" width=\"2\" height=\"{}\" fill=\"#D9D9D9\"/>", node.height);
                }
                let text = own.get([GeneratedLayer::Text.guid(), *row, *column].as_slice()).and_then(|o| o.field(Field::TextData)).and_then(|t| t.field(Field::Characters)).and_then(Raw::as_str).unwrap_or_default();
                if let (Some(font), false) = (font.as_ref(), text.trim().is_empty()) {
                    let size = 24.0;
                    let advances = text::advances(&db, font.id, text, size, 0.0);
                    let lines = text::break_lines(text, &advances, Some((width - 32.0).max(size)));
                    let chars: Vec<char> = text.chars().collect();
                    let weight = if r == 0 { 700 } else { 500 };
                    for (i, l) in lines.iter().enumerate() {
                        let shown: String = chars[l.start..l.end].iter().collect::<String>().trim_end().to_string();
                        let _ = write!(
                            self.out,
                            "<text xml:space=\"preserve\" x=\"{}\" y=\"{}\" font-family=\"Inter\" font-size=\"{size}\" font-weight=\"{weight}\" fill=\"#1E1E1E\">{}</text>",
                            x + 16.0,
                            y + 40.0 + i as f32 * 32.0,
                            escape(&shown)
                        );
                    }
                }
                x += width;
            }
            y += height;
        }
    }

    /// A connector grafta wrote: a line from one end to the other, each
    /// end where it is attached, in the stroke FigJam generates for it.
    fn generated_connector(&mut self, node: &SceneNode, raw: &Raw, own: &HashMap<Vec<Guid>, &Raw>) {
        let origin = absolute(self.graph, node.id);
        let end = |key: Field| -> Option<(f32, f32)> {
            let end = raw.field(key)?;
            let target = Guid::read(end.field(Field::EndpointNodeId)?)?;
            let position = end.field(Field::Position)?;
            let (x, y) = (position.field(Field::X)?.as_f32()?, position.field(Field::Y)?.as_f32()?);
            let at = absolute(self.graph, target);
            Some((at.0 + x - origin.0, at.1 + y - origin.1))
        };
        let (Some(a), Some(b)) = (end(Field::ConnectorStart), end(Field::ConnectorEnd)) else { return };
        let background = own.get([GeneratedLayer::Background.guid()].as_slice());
        let stroke = background.and_then(|o| o.field(Field::StrokePaints)).and_then(Raw::as_array).map(|p| solid_paints(p)).unwrap_or_default().into_iter().next().unwrap_or(("#1E1E1E".into(), 1.0));
        let width = background.and_then(|o| o.field(Field::StrokeWeight)).and_then(Raw::as_f32).unwrap_or(4.0);
        let _ = write!(
            self.out,
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-opacity=\"{}\" stroke-width=\"{width}\" stroke-linecap=\"round\"/>",
            a.0, a.1, b.0, b.1, stroke.0, stroke.1
        );
        // An arrowhead where an end's cap is one.
        let arrow = |key: Field| matches!(raw.known::<StrokeCap>(key), Some(StrokeCap::ArrowLines | StrokeCap::ArrowEquilateral | StrokeCap::TriangleFilled));
        if arrow(Field::ConnectorEndCap) {
            self.arrowhead(a, b, width, &stroke.0);
        }
        if arrow(Field::ConnectorStartCap) {
            self.arrowhead(b, a, width, &stroke.0);
        }
        if raw.str(Field::ConnectorLineStyle).is_some_and(|l| l != ConnectorLineStyle::Straight.as_wire()) {
            self.notes.insert("an elbowed or curved FigJam connector grafta changed is drawn straight: FigJam routes it when the file is opened".into());
        }
    }

    /// An arrowhead at `b`, pointing away from `a`.
    fn arrowhead(&mut self, a: (f32, f32), b: (f32, f32), width: f32, stroke: &str) {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / length, dy / length);
        let head = 6.0 * width;
        let (lx, ly) = (b.0 - ux * head - uy * head * 0.6, b.1 - uy * head + ux * head * 0.6);
        let (rx, ry) = (b.0 - ux * head + uy * head * 0.6, b.1 - uy * head - ux * head * 0.6);
        let _ = write!(self.out, "<path d=\"M{lx} {ly} L{} {} L{rx} {ry}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", b.0, b.1);
    }

    /// Glyph outlines and positions from a `derivedTextData`.
    fn glyphs_from(&self, derived: &Raw) -> Option<Vec<(String, f32, f32, f32)>> {
        let glyphs = self.glyphs_indexed(derived)?;
        Some(glyphs.into_iter().map(|(d, x, y, size, _)| (d, x, y, size)).collect())
    }

    /// [`Svg::glyphs_from`], each with the index of its first character.
    fn glyphs_indexed(&self, derived: &Raw) -> Option<Vec<Glyph>> {
        let glyphs = derived.field(Field::Glyphs)?.as_array()?;
        let mut out = Vec::new();
        for glyph in glyphs {
            let Some(blob) = glyph.field(Field::CommandsBlob).and_then(Raw::as_u32) else { continue };
            let Some(bytes) = self.fig.blobs.get(blob as usize) else { continue };
            let Some((d, _)) = model::vector::decode_path(bytes) else { continue };
            let d = path_data(&d);
            if d.is_empty() {
                continue;
            }
            let position = glyph.field(Field::Position)?;
            out.push((
                d.to_string(),
                position.field(Field::X).and_then(Raw::as_f32).unwrap_or(0.0),
                position.field(Field::Y).and_then(Raw::as_f32).unwrap_or(0.0),
                glyph.field(Field::FontSize).and_then(Raw::as_f32).unwrap_or(12.0),
                glyph.field(Field::FirstCharacter).and_then(Raw::as_u32).unwrap_or(0) as usize,
            ));
        }
        (!out.is_empty()).then_some(out)
    }
}

/// A glyph's outline, position, font size and first character's index.
type Glyph = (String, f32, f32, f32, usize);

/// A colour emoji, by its code point's block.
fn is_emoji(c: char) -> bool {
    matches!(u32::from(c), 0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF)
}

/// A stored transform as an SVG matrix.
fn raw_matrix(t: &Raw) -> String {
    let f = |k: Field, d: f32| t.field(k).and_then(Raw::as_f32).unwrap_or(d);
    format!("matrix({} {} {} {} {} {})", f(Field::M00, 1.0), f(Field::M10, 0.0), f(Field::M01, 0.0), f(Field::M11, 1.0), f(Field::M02, 0.0), f(Field::M12, 0.0))
}

/// The visible solid paints of a stored paint list, as hex and alpha.
fn solid_paints(paints: &[Raw]) -> Vec<(String, f32)> {
    paints
        .iter()
        .filter(|p| p.field(Field::Visible).and_then(Raw::as_bool).unwrap_or(true) && p.is(Field::Type, PaintType::Solid))
        .filter_map(|p| {
            let c = p.field(Field::Color)?;
            let channel = |k: Field| c.field(k).and_then(Raw::as_f32).unwrap_or(0.0);
            let colour = model::Color { r: channel(Field::R), g: channel(Field::G), b: channel(Field::B), a: channel(Field::A) };
            Some((colour.to_hex(), colour.a * p.field(Field::Opacity).and_then(Raw::as_f32).unwrap_or(1.0)))
        })
        .collect()
}

/// Path data SVG accepts. Figma's glyph outlines open with a close command,
/// and SVG drops a whole path that does not start with a move.
fn path_data(d: &str) -> &str {
    d.trim_start_matches(['Z', ' '])
}

/// An entry of an instance's cached expansion, by path from the instance being
/// drawn. `outdated`: a text overridden after this entry was laid out.
struct CachedEntry<'a> {
    path: Vec<Guid>,
    entry: &'a Raw,
    outdated: bool,
}

/// What the entries addressed to this layer say of it, each field from the first
/// that sets it: Figma stores several for one layer, and outer entries come first.
fn layer_override(overrides: &[&model::SymbolOverride], node: &SceneNode) -> Option<model::SymbolOverride> {
    overrides
        .iter()
        .filter(|o| o.path.len() == 1 && (Some(o.path[0]) == node.override_key || o.path[0] == node.id))
        .fold(None, |merged: Option<model::SymbolOverride>, entry| {
            let Some(mut merged) = merged else { return Some((*entry).clone()) };
            merged.text = merged.text.or_else(|| entry.text.clone());
            merged.visible = merged.visible.or(entry.visible);
            merged.fills = merged.fills.or_else(|| entry.fills.clone());
            merged.strokes = merged.strokes.or_else(|| entry.strokes.clone());
            merged.size = merged.size.or(entry.size);
            merged.symbol_id = merged.symbol_id.or(entry.symbol_id);
            for (property, value) in &entry.assignments {
                merged.assignments.entry(*property).or_insert_with(|| value.clone());
            }
            Some(merged)
        })
}

/// An entry's `guidPath`.
fn guid_path(entry: &Raw) -> Option<Vec<Guid>> {
    entry.field(Field::GuidPath)?.field(Field::Guids)?.as_array()?.iter().map(Guid::read).collect()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Standard base64, for embedded images.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if i <= chunk.len() {
                out.push(ALPHABET[((n >> shift) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;

/// A stroke's dashes, dash then gap (`dashPattern`), as an SVG
/// attribute; nothing for a solid stroke.
fn dashes(node: &SceneNode) -> String {
    if node.dash_pattern.is_empty() || node.dash_pattern.iter().all(|d| *d == 0.0) {
        return String::new();
    }
    let lengths: Vec<String> = node.dash_pattern.iter().map(ToString::to_string).collect();
    format!(" stroke-dasharray=\"{}\"", lengths.join(" "))
}

/// A text laid out: its styles, characters and lines, and the lines' height.
struct TextLayout {
    styles: Vec<RunStyle>,
    chars: Vec<Styled>,
    lines: Vec<TextLine>,
    height: f32,
}

/// What a run of text is drawn in.
#[derive(Clone)]
struct RunStyle {
    font: text::Font,
    weight: u32,
    italic: bool,
    size: f32,
    spacing: f32,
    line: f32,
    ascent: f32,
    descent: f32,
    case: String,
    decoration: String,
    paint: String,
}

/// How many characters a text's font lacks are named in a note, one by one.
const MISSING_NAMED: usize = 5;

/// A character of a text: what is drawn for it (its case applied), its style
/// (0 the layer's, then each run's), and its advance.
struct Styled {
    shown: String,
    style: usize,
    advance: f32,
}

/// A line of laid-out text: its characters, where it starts, its width, its
/// top, height and the largest font's ascent and descent on it, and a list
/// marker with its x and style.
struct TextLine {
    range: std::ops::Range<usize>,
    x: f32,
    width: f32,
    top: f32,
    height: f32,
    ascent: f32,
    descent: f32,
    marker: Option<(String, f32, usize)>,
}

/// Where a node sits on its page: its own and its ancestors' offsets.
fn absolute(graph: &SceneGraph, id: Guid) -> (f32, f32) {
    let (mut x, mut y) = (0.0, 0.0);
    let mut at = graph.node(id);
    while let Some(node) = at {
        if matches!(node.node_type, NodeType::Canvas | NodeType::Document) {
            break;
        }
        x += node.x;
        y += node.y;
        at = node.parent_id.and_then(|p| graph.node(p));
    }
    (x, y)
}

/// Which colours of a text reach a visible character: whether the layer's does,
/// and each run's. Run offsets count UTF-16 units. A text with nothing visible
/// shows the layer's, so it is still reported rather than passed over.
fn colours_shown(characters: &str, runs: &[model::TextRun]) -> (bool, Vec<bool>) {
    let mut shown = vec![false; runs.len()];
    let mut layer = false;
    let mut offset = 0;
    for c in characters.chars() {
        if !c.is_whitespace() {
            // The first run holding it, as `layout` draws it.
            match runs.iter().position(|run| (run.start..run.end).contains(&offset)) {
                Some(run) => shown[run] = true,
                None => layer = true,
            }
        }
        offset += c.len_utf16();
    }
    (layer || !shown.contains(&true), shown)
}
