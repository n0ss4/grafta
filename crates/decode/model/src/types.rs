//! Domain types for the scene graph produced from a `.fig` file.

use indexmap::IndexMap;
use schema::Guid;
use serde::{Deserialize, Serialize};
use schema::{ConstraintType, StackCounterAlign, StackJustify, StackPositioning, StrokeAlign, TextAlignHorizontal, TextAlignVertical, TextAutoResize, TextCase, TextDecoration, WireName};

// ── Color & paint ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    #[must_use]
    pub fn to_hex(self) -> String {
        let (r, g, b) = self.rgb8();
        format!("#{r:02X}{g:02X}{b:02X}")
    }

    /// `#RRGGBB` when fully opaque, `rgba(…)` otherwise. `extra_opacity` folds a
    /// paint-level opacity into the alpha channel.
    #[must_use]
    pub fn to_css(self, extra_opacity: f32) -> String {
        let (r, g, b) = self.rgb8();
        let a = (self.a * extra_opacity).clamp(0.0, 1.0);
        if a >= 0.999 {
            format!("#{r:02X}{g:02X}{b:02X}")
        } else {
            format!("rgba({r}, {g}, {b}, {})", trim_float(a))
        }
    }

    #[must_use]
    pub fn rgb8(self) -> (u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }
}

/// Format a float without trailing zeros (`0.50` → `0.5`, `12.0` → `12`).
#[must_use]
pub fn trim_float(v: f32) -> String {
    let s = format!("{v:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FillType {
    Solid,
    GradientLinear,
    GradientRadial,
    GradientAngular,
    GradientDiamond,
    Image,
    Pattern,
    Noise,
    Unknown,
}

impl FillType {
    #[must_use]
    pub fn is_gradient(self) -> bool {
        matches!(
            self,
            FillType::GradientLinear
                | FillType::GradientRadial
                | FillType::GradientAngular
                | FillType::GradientDiamond
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GradientStop {
    pub color: Color,
    pub position: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Matrix {
    pub m00: f32,
    pub m01: f32,
    pub m02: f32,
    pub m10: f32,
    pub m11: f32,
    pub m12: f32,
}

impl Default for Matrix {
    fn default() -> Self {
        Self { m00: 1.0, m01: 0.0, m02: 0.0, m10: 0.0, m11: 1.0, m12: 0.0 }
    }
}

/// A single paint. Used for both fills and stroke paints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Fill {
    pub fill_type: FillType,
    pub color: Option<Color>,
    pub opacity: f32,
    pub visible: bool,
    pub blend_mode: Option<String>,
    pub gradient_stops: Vec<GradientStop>,
    pub gradient_transform: Option<Matrix>,
    pub image_hash: Option<String>,
    pub image_scale_mode: Option<String>,
    /// Id of the design variable this paint's colour is bound to, if any.
    pub variable_id: Option<Guid>,
}

impl Fill {
    #[must_use]
    pub fn solid(color: Color) -> Self {
        Self {
            fill_type: FillType::Solid,
            color: Some(color),
            opacity: 1.0,
            visible: true,
            blend_mode: None,
            gradient_stops: Vec::new(),
            gradient_transform: None,
            image_hash: None,
            image_scale_mode: None,
            variable_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectType {
    DropShadow,
    InnerShadow,
    LayerBlur,
    BackgroundBlur,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Effect {
    pub effect_type: EffectType,
    pub color: Option<Color>,
    pub offset_x: f32,
    pub offset_y: f32,
    pub radius: f32,
    pub spread: f32,
    pub visible: bool,
}

impl Effect {
    /// CSS `box-shadow` / `filter` fragment, or `None` for effects with no
    /// direct CSS equivalent.
    #[must_use]
    pub fn to_css(&self) -> Option<String> {
        match self.effect_type {
            EffectType::DropShadow | EffectType::InnerShadow => {
                let color = self.color.unwrap_or(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.25 });
                let inset = if self.effect_type == EffectType::InnerShadow { "inset " } else { "" };
                Some(format!(
                    "{inset}{}px {}px {}px {}px {}",
                    trim_float(self.offset_x),
                    trim_float(self.offset_y),
                    trim_float(self.radius),
                    trim_float(self.spread),
                    color.to_css(1.0)
                ))
            }
            EffectType::LayerBlur => Some(format!("blur({}px)", trim_float(self.radius))),
            EffectType::BackgroundBlur => Some(format!("blur({}px)", trim_float(self.radius))),
            EffectType::Unknown => None,
        }
    }
}

// ── Text ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Unit {
    Pixels,
    Percent,
    /// Multiple of the font size.
    Raw,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Dimension {
    pub value: f32,
    pub unit: Unit,
}

impl Dimension {
    /// CSS value for a line-height/letter-spacing declaration.
    #[must_use]
    pub fn to_css(self) -> String {
        match self.unit {
            Unit::Pixels => format!("{}px", trim_float(self.value)),
            Unit::Percent => format!("{}%", trim_float(self.value)),
            Unit::Raw => trim_float(self.value),
        }
    }

    /// CSS value for a `letter-spacing` declaration.
    ///
    /// Figma stores letter spacing as a percentage of the font size, but
    /// `letter-spacing` does not accept percentages — the browser drops the
    /// whole declaration — so a percentage has to be written as `em`.
    #[must_use]
    pub fn to_letter_spacing_css(self) -> String {
        match self.unit {
            Unit::Pixels => format!("{}px", trim_float(self.value)),
            Unit::Percent => format!("{}em", trim_float(self.value / 100.0)),
            Unit::Raw => format!("{}em", trim_float(self.value)),
        }
    }

    /// Resolve to pixels given the font size the dimension applies to.
    #[must_use]
    pub fn to_px(self, font_size: f32) -> f32 {
        match self.unit {
            Unit::Pixels => self.value,
            Unit::Percent => font_size * self.value / 100.0,
            Unit::Raw => font_size * self.value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TextStyle {
    pub font_family: String,
    /// Figma style name, e.g. `Semi Bold Italic`.
    pub font_style: String,
    pub font_size: f32,
    pub font_weight: u32,
    pub italic: bool,
    pub line_height: Option<Dimension>,
    pub letter_spacing: Option<Dimension>,
    pub text_align_horizontal: String,
    pub text_align_vertical: String,
    pub text_case: String,
    pub text_decoration: String,
    pub text_auto_resize: String,
    pub max_lines: Option<u32>,
    /// Space after each paragraph, and the indent of its first line.
    pub paragraph_spacing: f32,
    pub paragraph_indent: f32,
}

/// Characters of a text styled apart from the layer: `start..end` in UTF-16
/// units, as Figma counts them, and what the style changes. A field
/// left `None` is the layer's own.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextRun {
    pub start: usize,
    pub end: usize,
    pub font_family: Option<String>,
    pub font_style: Option<String>,
    pub font_size: Option<f32>,
    pub fills: Option<Vec<Fill>>,
    pub text_decoration: Option<String>,
    pub text_case: Option<String>,
    pub letter_spacing: Option<Dimension>,
    pub line_height: Option<Dimension>,
    /// A web address the characters link to.
    pub hyperlink: Option<String>,
    /// A text style applied to these characters alone, by id.
    pub text_style_id: Option<Guid>,
    /// Fields the style sets that grafta does not read, by the file's names.
    pub other: Vec<String>,
}
/// A paragraph's list type in the plugin API's names.
pub const LIST_NONE: &str = "NONE";
pub const LIST_UNORDERED: &str = "UNORDERED";
pub const LIST_ORDERED: &str = "ORDERED";


/// A paragraph's list type (`NONE`, `UNORDERED`, `ORDERED`) and its level:
/// 0 for plain text, 1 for a list, 2 nested.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Paragraph {
    pub list_type: String,
    pub indentation: u32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "Inter".to_string(),
            font_style: "Regular".to_string(),
            font_size: 14.0,
            font_weight: 400,
            italic: false,
            line_height: None,
            letter_spacing: None,
            text_align_horizontal: TextAlignHorizontal::Left.as_wire().to_string(),
            text_align_vertical: TextAlignVertical::Top.as_wire().to_string(),
            text_case: TextCase::Original.as_wire().to_string(),
            text_decoration: TextDecoration::None.as_wire().to_string(),
            text_auto_resize: TextAutoResize::None.as_wire().to_string(),
            max_lines: None,
            paragraph_spacing: 0.0,
            paragraph_indent: 0.0,
        }
    }
}

/// Map a Figma font style name (`"Semi Bold Italic"`) to a numeric CSS weight.
#[must_use]
pub fn style_to_weight(style: &str) -> u32 {
    // Figma writes styles as `Semi Bold`, `SemiBold` or `semi-bold`; normalise
    // separators away so one table covers all of them.
    let s = style.to_lowercase().replace([' ', '-', '_'], "");
    // Order matters: `extralight` must win over `light`.
    const TABLE: &[(&str, u32)] = &[
        ("extralight", 200),
        ("ultralight", 200),
        ("extrabold", 800),
        ("ultrabold", 800),
        ("semibold", 600),
        ("demibold", 600),
        ("thin", 100),
        ("hairline", 100),
        ("light", 300),
        ("regular", 400),
        ("normal", 400),
        ("book", 400),
        ("medium", 500),
        ("black", 900),
        ("heavy", 900),
        ("bold", 700),
    ];
    for (needle, weight) in TABLE {
        if s.contains(needle) {
            return *weight;
        }
    }
    400
}

// ── Layout ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LayoutMode {
    None,
    Horizontal,
    Vertical,
}

impl LayoutMode {
    #[must_use]
    pub fn is_auto(self) -> bool {
        self != LayoutMode::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AxisSizing {
    Fixed,
    Hug,
    Fill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layout {
    pub mode: LayoutMode,
    pub item_spacing: f32,
    pub counter_axis_spacing: f32,
    pub padding_top: f32,
    pub padding_right: f32,
    pub padding_bottom: f32,
    pub padding_left: f32,
    pub primary_axis_sizing: AxisSizing,
    pub counter_axis_sizing: AxisSizing,
    /// `MIN` | `CENTER` | `MAX` | `SPACE_BETWEEN`
    pub primary_axis_align: String,
    /// `MIN` | `CENTER` | `MAX` | `STRETCH` | `BASELINE`
    pub counter_axis_align: String,
    pub wrap: bool,
    /// Child-side: `AUTO` | `ABSOLUTE`
    pub positioning: String,
    pub grow: f32,
    /// Child-side: `AUTO` | `MIN` | `CENTER` | `MAX` | `STRETCH`
    pub align_self: String,
    pub reverse_z_index: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            mode: LayoutMode::None,
            item_spacing: 0.0,
            counter_axis_spacing: 0.0,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            primary_axis_sizing: AxisSizing::Fixed,
            counter_axis_sizing: AxisSizing::Fixed,
            primary_axis_align: StackJustify::Min.as_wire().to_string(),
            counter_axis_align: StackCounterAlign::Min.as_wire().to_string(),
            wrap: false,
            positioning: StackPositioning::Auto.as_wire().to_string(),
            grow: 0.0,
            align_self: StackCounterAlign::Auto.as_wire().to_string(),
            reverse_z_index: false,
        }
    }
}

impl Layout {
    #[must_use]
    pub fn has_padding(&self) -> bool {
        self.padding_top != 0.0
            || self.padding_right != 0.0
            || self.padding_bottom != 0.0
            || self.padding_left != 0.0
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CornerRadii {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl CornerRadii {
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_right == 0.0
            && self.bottom_left == 0.0
    }

    #[must_use]
    pub fn is_uniform(self) -> bool {
        self.top_left == self.top_right
            && self.top_left == self.bottom_right
            && self.top_left == self.bottom_left
    }

    /// CSS `border-radius` value.
    #[must_use]
    pub fn to_css(self) -> String {
        if self.is_uniform() {
            format!("{}px", trim_float(self.top_left))
        } else {
            format!(
                "{}px {}px {}px {}px",
                trim_float(self.top_left),
                trim_float(self.top_right),
                trim_float(self.bottom_right),
                trim_float(self.bottom_left)
            )
        }
    }
}

// ── Components ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComponentPropertyType {
    Variant,
    Text,
    Boolean,
    InstanceSwap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentProperty {
    pub id: Guid,
    pub name: String,
    pub prop_type: ComponentPropertyType,
    pub default_value: Option<String>,
    /// Ordered options for `VARIANT` properties.
    pub options: Vec<String>,
}

/// One flattened path of a vector layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorPath {
    /// SVG `d` attribute, in the layer's own coordinates.
    pub d: String,
    /// SVG `fill-rule`: `nonzero` or `evenodd`.
    pub fill_rule: String,
}

/// One entry of an instance's `symbolOverrides`.
///
/// Figma addresses the overridden layer by `path`: a chain of `overrideKey`s
/// made of the nested instances to pass through, ending with the key of the
/// layer itself. Within a single component every layer has a unique key, so
/// most paths are one element long.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SymbolOverride {
    pub path: Vec<Guid>,
    pub name: Option<String>,
    pub text: Option<String>,
    pub visible: Option<bool>,
    pub fills: Option<Vec<Fill>>,
    pub strokes: Option<Vec<Fill>>,
    pub size: Option<(f32, f32)>,
    /// Nested instance swap target.
    pub symbol_id: Option<Guid>,
    /// Component property values this entry assigns to the instance it names.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub assignments: IndexMap<Guid, String>,
}

impl SymbolOverride {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.text.is_none()
            && self.visible.is_none()
            && self.fills.is_none()
            && self.strokes.is_none()
            && self.size.is_none()
            && self.symbol_id.is_none()
    }
}

// ── Nodes ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeType {
    Document,
    Canvas,
    Frame,
    Group,
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Text,
    Line,
    Star,
    Polygon,
    Vector,
    BooleanOperation,
    Section,
    Component,
    ComponentSet,
    Instance,
    Connector,
    ShapeWithText,
    /// Text set along a path (Figma Draw), drawn from its stored geometry.
    TextPath,
    /// Figma Slides: a deck's grid of rows, a row of slides, a slide,
    /// and an interactive element (a poll, a live embed) on one.
    Slide,
    SlideGrid,
    SlideRow,
    InteractiveSlideElement,
    /// FigJam: a sticky note, a table, a stamp, a plugin's widget, and a
    /// slice (an export area). A sticky and a table, like a shape with text
    /// and a connector, show layers FigJam generates (`nodeGenerationData`).
    Sticky,
    Table,
    Stamp,
    Widget,
    Slice,
    /// Figma Sites: a page of the site, named by its route, holding one frame
    /// per breakpoint.
    ResponsiveSet,
    Unknown,
}

impl NodeType {
    #[must_use]
    pub fn is_container(self) -> bool {
        matches!(
            self,
            NodeType::Frame
                | NodeType::Group
                | NodeType::Section
                | NodeType::Component
                | NodeType::ComponentSet
                | NodeType::Instance
                | NodeType::Canvas
                | NodeType::Document
                | NodeType::Slide
                | NodeType::SlideGrid
                | NodeType::SlideRow
                | NodeType::InteractiveSlideElement
                | NodeType::Widget
                | NodeType::ResponsiveSet
        )
    }

    /// Every type a model may name, in the order it is looked for.
    pub const NAMED: [NodeType; 28] = [
        NodeType::Document, NodeType::Canvas, NodeType::Frame, NodeType::Group,
        NodeType::Rectangle, NodeType::Ellipse, NodeType::Text, NodeType::Line, NodeType::Star,
        NodeType::Polygon, NodeType::Vector, NodeType::BooleanOperation, NodeType::Section,
        NodeType::Component, NodeType::ComponentSet, NodeType::Instance, NodeType::Connector,
        NodeType::ShapeWithText, NodeType::Slide, NodeType::SlideGrid, NodeType::SlideRow,
        NodeType::InteractiveSlideElement, NodeType::Sticky, NodeType::Table, NodeType::Stamp,
        NodeType::Widget, NodeType::Slice, NodeType::ResponsiveSet,
    ];

    /// The plugin API's name for the type.
    #[must_use]
    pub fn plugin_name(self) -> &'static str {
        match self {
            NodeType::Document => "DOCUMENT",
            NodeType::Canvas => "PAGE",
            NodeType::Frame => "FRAME",
            NodeType::Group => "GROUP",
            NodeType::Rectangle | NodeType::RoundedRectangle => "RECTANGLE",
            NodeType::Ellipse => "ELLIPSE",
            NodeType::Text => "TEXT",
            NodeType::Line => "LINE",
            NodeType::Star => "STAR",
            NodeType::Polygon => "POLYGON",
            NodeType::Vector => "VECTOR",
            NodeType::BooleanOperation => "BOOLEAN_OPERATION",
            NodeType::Section => "SECTION",
            NodeType::Component => "COMPONENT",
            NodeType::ComponentSet => "COMPONENT_SET",
            NodeType::Instance => "INSTANCE",
            NodeType::Connector => "CONNECTOR",
            NodeType::ShapeWithText => "SHAPE_WITH_TEXT",
            NodeType::TextPath => "TEXT_PATH",
            NodeType::Slide => "SLIDE",
            NodeType::SlideGrid => "SLIDE_GRID",
            NodeType::SlideRow => "SLIDE_ROW",
            NodeType::InteractiveSlideElement => "INTERACTIVE_SLIDE_ELEMENT",
            NodeType::Sticky => "STICKY",
            NodeType::Table => "TABLE",
            NodeType::Stamp => "STAMP",
            NodeType::Widget => "WIDGET",
            NodeType::Slice => "SLICE",
            NodeType::ResponsiveSet => "RESPONSIVE_SET",
            NodeType::Unknown => "UNKNOWN",
        }
    }

    /// FigJam nodes whose look is layers FigJam generates and caches
    /// (`nodeGenerationData`, `derivedImmutableFrameData`) rather than children
    ///.
    #[must_use]
    pub fn is_generated(self) -> bool {
        matches!(self, NodeType::Sticky | NodeType::ShapeWithText | NodeType::Connector | NodeType::Table)
    }

    /// Nodes that are drawn as a single vector shape rather than a box.
    #[must_use]
    pub fn is_vector_shape(self) -> bool {
        matches!(
            self,
            NodeType::Vector
                | NodeType::Line
                | NodeType::Star
                | NodeType::Polygon
                | NodeType::BooleanOperation
                | NodeType::TextPath
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: Guid,
    pub name: String,
    pub node_type: NodeType,
    pub parent_id: Option<Guid>,
    pub children: Vec<Guid>,
    pub order_key: String,

    // Geometry
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
    pub flip_x: bool,

    // Appearance
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
    /// Canvases Figma hides from the file's readers (component workbenches).
    pub internal_only: bool,
    pub blend_mode: Option<String>,
    pub fills: Vec<Fill>,
    pub strokes: Vec<Fill>,
    pub stroke_weight: f32,
    pub stroke_align: String,
    pub stroke_weights: Option<(f32, f32, f32, f32)>,
    pub dash_pattern: Vec<f32>,
    pub effects: Vec<Effect>,
    pub corner_radii: CornerRadii,
    pub clips_content: bool,
    pub is_mask: bool,
    /// How a mask masks, as the file spells it: `ALPHA`, `OUTLINE` or `LUMINANCE`.
    pub mask_type: String,

    // Vector geometry, on the layers Figma draws as paths rather than boxes.
    pub fill_geometry: Vec<VectorPath>,
    /// The layer's stroke, already outlined by Figma into fillable paths.
    pub stroke_geometry: Vec<VectorPath>,
    /// Bounds covering both sets of paths.
    pub vector_bounds: Option<crate::vector::Bounds>,

    // Layout
    pub layout: Layout,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_height: Option<f32>,
    pub horizontal_constraint: String,
    pub vertical_constraint: String,

    // Text
    pub text: Option<String>,
    pub text_style: Option<TextStyle>,
    /// The characters styled apart from the layer, in order. Empty when
    /// the whole text has the layer's style.
    pub text_runs: Vec<TextRun>,
    /// Each paragraph's list type and level, when any paragraph is a list.
    pub paragraphs: Vec<Paragraph>,

    // Design system links
    /// Variable id per bound field (`fills`, `width`, `cornerRadius`, …).
    pub bound_variables: IndexMap<String, Guid>,
    pub fill_style_id: Option<Guid>,
    pub stroke_style_id: Option<Guid>,
    pub text_style_id: Option<Guid>,
    pub effect_style_id: Option<Guid>,

    // Components & instances
    /// For an instance: the id of the component it is an instance of.
    pub component_id: Option<Guid>,
    pub component_key: Option<String>,
    /// Stable per-layer key that instance overrides address this node by.
    pub override_key: Option<Guid>,
    pub description: Option<String>,
    pub component_properties: Vec<ComponentProperty>,
    /// Variant property name → value, for a variant inside a component set.
    pub variant_values: IndexMap<String, String>,
    /// Ordered variant property name → possible values, on a component set.
    pub variant_options: IndexMap<String, Vec<String>>,
    /// Property definition id → assigned value, on an instance.
    pub property_assignments: IndexMap<Guid, String>,
    pub symbol_overrides: Vec<SymbolOverride>,
    pub export_formats: Vec<String>,
    /// An ellipse's arc: start and end angles in radians, and the inner
    /// radius as a fraction of the outer.
    pub arc: Option<(f32, f32, f32)>,
    /// A polygon's sides or a star's points.
    pub point_count: Option<u32>,
    /// A star's inner radius as a fraction of the outer.
    pub star_inner_scale: Option<f32>,
}

impl SceneNode {
    /// An empty node with Figma's defaults, for tests and synthetic trees.
    #[must_use]
    pub fn new(id: Guid, name: impl Into<String>, node_type: NodeType) -> Self {
        Self {
            id,
            name: name.into(),
            node_type,
            parent_id: None,
            children: Vec::new(),
            order_key: String::new(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            rotation: 0.0,
            flip_x: false,
            opacity: 1.0,
            visible: true,
            locked: false,
            internal_only: false,
            blend_mode: None,
            fills: Vec::new(),
            strokes: Vec::new(),
            stroke_weight: 0.0,
            stroke_align: StrokeAlign::Inside.as_wire().to_string(),
            stroke_weights: None,
            dash_pattern: Vec::new(),
            effects: Vec::new(),
            corner_radii: CornerRadii {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            clips_content: false,
            is_mask: false,
            mask_type: "ALPHA".to_string(),
            fill_geometry: Vec::new(),
            stroke_geometry: Vec::new(),
            vector_bounds: None,
            layout: Layout::default(),
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            horizontal_constraint: ConstraintType::Min.as_wire().to_string(),
            vertical_constraint: ConstraintType::Min.as_wire().to_string(),
            text: None,
            text_style: None,
            text_runs: Vec::new(),
            paragraphs: Vec::new(),
            bound_variables: IndexMap::new(),
            fill_style_id: None,
            stroke_style_id: None,
            text_style_id: None,
            effect_style_id: None,
            component_id: None,
            component_key: None,
            override_key: None,
            description: None,
            component_properties: Vec::new(),
            variant_values: IndexMap::new(),
            variant_options: IndexMap::new(),
            property_assignments: IndexMap::new(),
            symbol_overrides: Vec::new(),
            export_formats: Vec::new(),
            arc: None,
            point_count: None,
            star_inner_scale: None,
        }
    }

    /// First visible paint that maps onto a CSS background/colour, if any.
    #[must_use]
    pub fn primary_fill(&self) -> Option<&Fill> {
        self.fills
            .iter()
            .find(|f| f.visible && (f.color.is_some() || !f.gradient_stops.is_empty()))
    }

    pub fn visible_strokes(&self) -> impl Iterator<Item = &Fill> {
        self.strokes.iter().filter(|s| s.visible)
    }

    #[must_use]
    pub fn has_image_fill(&self) -> bool {
        self.fills.iter().any(|f| f.visible && f.fill_type == FillType::Image)
    }
}

// ── Design variables ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VariableType {
    Color,
    Float,
    String,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VariableValue {
    Color { value: Color },
    Float { value: f32 },
    String { value: String },
    Boolean { value: bool },
    Alias { variable_id: Guid },
}

/// A collection's mode: one the file declares, or the one grafta gives a
/// collection whose modes the file does not hold, derived and named so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeId {
    Declared(Guid),
    Default,
}

/// The name of the mode grafta gives a collection that declares none.
const DEFAULT_MODE: &str = "default";

impl ModeId {
    #[must_use]
    pub fn declared(self) -> Option<Guid> {
        match self {
            Self::Declared(id) => Some(id),
            Self::Default => None,
        }
    }
}

impl std::fmt::Display for ModeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Declared(id) => id.fmt(f),
            Self::Default => f.write_str(DEFAULT_MODE),
        }
    }
}

impl Serialize for ModeId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ModeId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        if text == DEFAULT_MODE {
            return Ok(Self::Default);
        }
        text.parse().map(Self::Declared).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableMode {
    pub id: ModeId,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableCollection {
    pub id: Guid,
    pub name: String,
    pub modes: Vec<VariableMode>,
    pub variable_ids: Vec<Guid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Variable {
    pub id: Guid,
    pub name: String,
    pub var_type: VariableType,
    pub collection_id: Guid,
    /// Mode id → value.
    pub values_by_mode: IndexMap<Guid, VariableValue>,
}

// ── Shared styles ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SharedStyleType {
    Fill,
    Text,
    Effect,
    Grid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedStyle {
    pub id: Guid,
    pub name: String,
    pub style_type: SharedStyleType,
    pub description: Option<String>,
    pub fills: Vec<Fill>,
    pub effects: Vec<Effect>,
    pub text_style: Option<TextStyle>,
}
