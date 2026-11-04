//! `nodeChange` → [`SceneNode`] conversion.

use std::collections::HashMap;

use indexmap::IndexMap;
use kiwi::Value;
use schema::{BlendMode, ComponentPropType, ConstraintType, Field, Guid, LineType, MaskType, NumberUnits, StackAlign, StackCounterAlign, StackJustify, StackMode, StackPositioning, StackSize, StackWrap, StrokeAlign, StyleType, TextAlignHorizontal, TextAlignVertical, TextAutoResize, TextCase, TextDecoration, VariableField, WindingRule, Wire, WireName, Wired};

use crate::paint::{self, VariableResolver};
use crate::types::{
    AxisSizing, ComponentProperty, ComponentPropertyType, CornerRadii, Dimension, Fill, Layout,
    LayoutMode, NodeType, SceneNode, SharedStyle, SharedStyleType, SymbolOverride, TextStyle, Unit,
    Paragraph, TextRun,
    VectorPath,
};
use crate::{variables, vector};

pub(crate) struct Converter<'a> {
    pub resolver: VariableResolver<'a>,
    pub styles: IndexMap<Guid, SharedStyle>,
    /// The file's binary blobs, which hold every layer's path geometry.
    pub blobs: &'a [Vec<u8>],
}

// ── Node classification ───────────────────────────────────────────────────────

fn base_node_type(nc: &Value) -> NodeType {
    use schema::NodeType as Kind;
    match nc.known::<Kind>(Field::Type) {
        Some(Kind::Document) => NodeType::Document,
        Some(Kind::Canvas) => NodeType::Canvas,
        Some(Kind::Frame) => NodeType::Frame,
        Some(Kind::Group) => NodeType::Group,
        Some(Kind::Rectangle) => NodeType::Rectangle,
        Some(Kind::RoundedRectangle) => NodeType::RoundedRectangle,
        Some(Kind::Ellipse) => NodeType::Ellipse,
        Some(Kind::Text) => NodeType::Text,
        Some(Kind::Line) => NodeType::Line,
        Some(Kind::Star) => NodeType::Star,
        Some(Kind::RegularPolygon) => NodeType::Polygon,
        Some(Kind::Vector) => NodeType::Vector,
        Some(Kind::BooleanOperation) => NodeType::BooleanOperation,
        Some(Kind::Section) => NodeType::Section,
        Some(Kind::Symbol) => NodeType::Component,
        Some(Kind::Instance) => NodeType::Instance,
        Some(Kind::Connector) => NodeType::Connector,
        Some(Kind::ShapeWithText) => NodeType::ShapeWithText,
        Some(Kind::TextPath) => NodeType::TextPath,
        Some(Kind::Slide) => NodeType::Slide,
        Some(Kind::SlideGrid) => NodeType::SlideGrid,
        Some(Kind::SlideRow) => NodeType::SlideRow,
        Some(Kind::InteractiveSlideElement) => NodeType::InteractiveSlideElement,
        Some(Kind::Sticky) => NodeType::Sticky,
        Some(Kind::Table) => NodeType::Table,
        Some(Kind::Stamp) => NodeType::Stamp,
        Some(Kind::Widget) => NodeType::Widget,
        Some(Kind::Slice) => NodeType::Slice,
        Some(Kind::ResponsiveSet) => NodeType::ResponsiveSet,
        _ => NodeType::Unknown,
    }
}

fn declares_variants(nc: &Value) -> bool {
    nc.bool(Field::IsStateGroup).unwrap_or(false)
        || nc
            .array(Field::ComponentPropDefs)
            .iter()
            .any(|d| d.is(Field::Type, ComponentPropType::Variant))
}

/// Figma stores plain groups and component sets as `FRAME` node changes;
/// disambiguate them the way the editor does.
fn resolve_node_type(nc: &Value) -> NodeType {
    let node_type = base_node_type(nc);
    if node_type != NodeType::Frame {
        return node_type;
    }
    if declares_variants(nc) {
        return NodeType::ComponentSet;
    }
    // A hug-sized auto-layout frame also sets `resizeToFit`, so a real group is
    // one without any stack mode.
    let no_stack = nc.str(Field::StackMode).is_none_or(|mode| mode == StackMode::None.as_wire());
    if nc.bool(Field::ResizeToFit).unwrap_or(false) && no_stack {
        return NodeType::Group;
    }
    NodeType::Frame
}

/// Node changes that carry data but are not part of the scene tree.
#[must_use]
/// Node changes that are assets of the file rather than layers: variables,
/// styles, and Figma Draw's brushes (25 in `material3.fig`, 15 in `nuxtui.fig`,
/// all on the internal canvas or a group, none with children).
pub(crate) fn is_non_scene_node(nc: &Value) -> bool {
    use schema::NodeType as Kind;
    matches!(nc.known::<Kind>(Field::Type), Some(Kind::Variable | Kind::VariableSet | Kind::Brush)) || nc.has(Field::StyleType)
}

/// A minimum or maximum size, from the `minSize` or `maxSize` pair files
/// store: a minimum of 0 and a maximum of infinity are none.
fn bound(nc: &Value, field: Field, axis: Field) -> Option<f32> {
    let v = nc.field(field)?.field(Field::Value)?.f32(axis)?;
    (v.is_finite() && v > 0.0).then_some(v)
}

// ── Geometry ──────────────────────────────────────────────────────────────────

/// Position, size and rotation from Figma's affine transform. Figma anchors a
/// rotated node by its centre, so the axis-aligned top-left has to be derived.
fn transform(nc: &Value) -> (f32, f32, f32, f32, f32, bool) {
    let width = nc.field(Field::Size).and_then(|s| s.f32(Field::X)).unwrap_or(0.0);
    let height = nc.field(Field::Size).and_then(|s| s.f32(Field::Y)).unwrap_or(0.0);

    let Some(t) = nc.field(Field::Transform) else {
        return (0.0, 0.0, width, height, 0.0, false);
    };
    let m00 = t.f32(Field::M00).unwrap_or(1.0);
    let m01 = t.f32(Field::M01).unwrap_or(0.0);
    let m02 = t.f32(Field::M02).unwrap_or(0.0);
    let m10 = t.f32(Field::M10).unwrap_or(0.0);
    let m11 = t.f32(Field::M11).unwrap_or(1.0);
    let m12 = t.f32(Field::M12).unwrap_or(0.0);

    let flip_x = m00 * m11 - m01 * m10 < 0.0;
    let sign = if flip_x { -1.0 } else { 1.0 };
    let rotation = (m10 * sign).atan2(m00 * sign).to_degrees();

    if rotation != 0.0 && !flip_x {
        let radians = rotation.to_radians();
        let (sin, cos) = radians.sin_cos();
        let x = m02 - (width / 2.0) * (1.0 - cos) - sin * (height / 2.0);
        let y = m12 - (height / 2.0) * (1.0 - cos) + sin * (width / 2.0);
        return (x, y, width, height, rotation, flip_x);
    }

    // Axis-aligned (possibly mirrored): take the bounding box of the corners.
    let corners = [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)];
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    for (px, py) in corners {
        min_x = min_x.min(m00 * px + m01 * py + m02);
        min_y = min_y.min(m10 * px + m11 * py + m12);
    }
    (min_x, min_y, width, height, rotation, flip_x)
}

fn corner_radii(nc: &Value) -> CornerRadii {
    let uniform = nc.f32(Field::CornerRadius).unwrap_or(0.0);
    CornerRadii {
        top_left: nc.f32(Field::RectangleTopLeftCornerRadius).unwrap_or(uniform),
        top_right: nc.f32(Field::RectangleTopRightCornerRadius).unwrap_or(uniform),
        bottom_right: nc.f32(Field::RectangleBottomRightCornerRadius).unwrap_or(uniform),
        bottom_left: nc.f32(Field::RectangleBottomLeftCornerRadius).unwrap_or(uniform),
    }
}

// ── Layout ────────────────────────────────────────────────────────────────────

/// Kiwi drops any field still holding its default, and the two stack axes
/// default differently: a stack hugs its content along the primary axis and is
/// fixed along the counter axis unless the file says otherwise. Reading an
/// absent value as `Fixed` on both axes pins a pixel width onto every frame
/// Figma actually sizes from its content.
fn axis_sizing(nc: &Value, field: Field, absent: AxisSizing) -> AxisSizing {
    match nc.known::<StackSize>(field) {
        Some(StackSize::ResizeToFit | StackSize::ResizeToFitWithImplicitSize) => AxisSizing::Hug,
        Some(StackSize::Fixed) => AxisSizing::Fixed,
        None => absent,
    }
}

/// `stackPrimaryAlignItems`, or the older `stackJustify`, as the plugin API
/// names a primary axis alignment.
fn primary_align(nc: &Value) -> String {
    let field = if nc.has(Field::StackPrimaryAlignItems) { Field::StackPrimaryAlignItems } else { Field::StackJustify };
    match nc.known::<StackJustify>(field) {
        Some(StackJustify::Center) => StackJustify::Center,
        Some(StackJustify::Max) => StackJustify::Max,
        Some(StackJustify::SpaceBetween | StackJustify::SpaceEvenly) => StackJustify::SpaceBetween,
        _ => StackJustify::Min,
    }
    .as_wire()
    .to_string()
}

/// `stackCounterAlignItems`, or the older `stackCounterAlign`, as the plugin
/// API names a counter axis alignment.
fn counter_align(nc: &Value) -> String {
    let aligned = if nc.has(Field::StackCounterAlignItems) {
        match nc.known::<StackAlign>(Field::StackCounterAlignItems) {
            Some(StackAlign::Center) => StackCounterAlign::Center,
            Some(StackAlign::Max) => StackCounterAlign::Max,
            Some(StackAlign::Baseline) => StackCounterAlign::Baseline,
            _ => StackCounterAlign::Min,
        }
    } else {
        match nc.known::<StackCounterAlign>(Field::StackCounterAlign) {
            Some(aligned @ (StackCounterAlign::Center | StackCounterAlign::Max | StackCounterAlign::Stretch | StackCounterAlign::Baseline)) => aligned,
            _ => StackCounterAlign::Min,
        }
    };
    aligned.as_wire().to_string()
}

fn consumes_variable(nc: &Value, field: VariableField) -> bool {
    nc.field(Field::VariableConsumptionMap)
        .map(|m| m.array(Field::Entries))
        .unwrap_or_default()
        .iter()
        .any(|e| e.is(Field::VariableField, field))
}

fn layout(nc: &Value) -> Layout {
    let mode = match nc.known::<StackMode>(Field::StackMode) {
        Some(StackMode::Horizontal) => LayoutMode::Horizontal,
        Some(StackMode::Vertical) => LayoutMode::Vertical,
        _ => LayoutMode::None,
    };

    // Figma collapses symmetric padding into the horizontal/vertical fields and
    // only stores the trailing edges when they differ.
    let base = nc.f32(Field::StackPadding).unwrap_or(0.0);
    let vertical = nc.f32(Field::StackVerticalPadding).unwrap_or(base);
    let horizontal = nc.f32(Field::StackHorizontalPadding).unwrap_or(base);
    let padding_bottom = nc.f32(Field::StackPaddingBottom).unwrap_or_else(|| {
        if consumes_variable(nc, VariableField::StackPaddingTop) {
            base
        } else {
            vertical
        }
    });
    let padding_right = nc.f32(Field::StackPaddingRight).unwrap_or_else(|| {
        if consumes_variable(nc, VariableField::StackPaddingLeft) {
            base
        } else {
            horizontal
        }
    });

    Layout {
        mode,
        item_spacing: nc.f32(Field::StackSpacing).unwrap_or(0.0),
        counter_axis_spacing: nc.f32(Field::StackCounterSpacing).unwrap_or(0.0),
        padding_top: vertical,
        padding_right,
        padding_bottom,
        padding_left: horizontal,
        primary_axis_sizing: axis_sizing(nc, Field::StackPrimarySizing, AxisSizing::Hug),
        counter_axis_sizing: axis_sizing(nc, Field::StackCounterSizing, AxisSizing::Fixed),
        primary_axis_align: primary_align(nc),
        counter_axis_align: counter_align(nc),
        wrap: nc.is(Field::StackWrap, StackWrap::Wrap),
        positioning: if nc.is(Field::StackPositioning, StackPositioning::Absolute) { StackPositioning::Absolute } else { StackPositioning::Auto }
            .as_wire()
            .to_string(),
        grow: nc.f32(Field::StackChildPrimaryGrow).unwrap_or(0.0),
        align_self: match nc.known::<StackCounterAlign>(Field::StackChildAlignSelf) {
            Some(aligned @ (StackCounterAlign::Min | StackCounterAlign::Center | StackCounterAlign::Max | StackCounterAlign::Stretch | StackCounterAlign::Baseline)) => aligned,
            _ => StackCounterAlign::Auto,
        }
        .as_wire()
        .to_string(),
        reverse_z_index: nc.bool(Field::StackReverseZIndex).unwrap_or(false),
    }
}

// ── Text ──────────────────────────────────────────────────────────────────────

fn dimension(v: &Value) -> Option<Dimension> {
    let value = v.f32(Field::Value)?;
    let unit = match v.known::<NumberUnits>(Field::Units) {
        Some(NumberUnits::Pixels) => Unit::Pixels,
        Some(NumberUnits::Percent) => Unit::Percent,
        _ => Unit::Raw,
    };
    Some(Dimension { value, unit })
}

pub(crate) fn text_style(nc: &Value) -> TextStyle {
    let font = nc.field(Field::FontName);
    let family = font
        .and_then(|f| f.str(Field::Family))
        .unwrap_or("Inter")
        .to_string();
    let style = font.and_then(|f| f.str(Field::Style)).unwrap_or("Regular").to_string();
    TextStyle {
        font_weight: crate::types::style_to_weight(&style),
        italic: style.to_lowercase().contains("italic"),
        font_family: family,
        font_style: style,
        font_size: nc.f32(Field::FontSize).unwrap_or(14.0),
        line_height: nc.field(Field::LineHeight).and_then(dimension),
        letter_spacing: nc.field(Field::LetterSpacing).and_then(dimension),
        text_align_horizontal: nc.str(Field::TextAlignHorizontal).unwrap_or(TextAlignHorizontal::Left.as_wire()).to_string(),
        text_align_vertical: nc.str(Field::TextAlignVertical).unwrap_or(TextAlignVertical::Top.as_wire()).to_string(),
        text_case: nc.str(Field::TextCase).unwrap_or(TextCase::Original.as_wire()).to_string(),
        text_decoration: nc.str(Field::TextDecoration).unwrap_or(TextDecoration::None.as_wire()).to_string(),
        text_auto_resize: nc.str(Field::TextAutoResize).unwrap_or(TextAutoResize::None.as_wire()).to_string(),
        max_lines: nc.u32(Field::MaxLines),
        paragraph_spacing: nc.f32(Field::ParagraphSpacing).unwrap_or(0.0),
        paragraph_indent: nc.f32(Field::ParagraphIndent).unwrap_or(0.0),
    }
}

/// A text's paragraphs, when any is not plain: one per entry of
/// `textData.lines`, in the plugin API's list names.
fn paragraphs(nc: &Value) -> Vec<Paragraph> {
    let lines = nc.field(Field::TextData).map_or(&[][..], |t| t.array(Field::Lines));
    if lines.iter().all(|l| l.str(Field::LineType).is_none_or(|t| t == LineType::Plain.as_wire())) {
        return Vec::new();
    }
    lines
        .iter()
        .map(|l| Paragraph {
            list_type: match l.enum_of::<LineType>(Field::LineType) {
                None | Some(Wired::Known(LineType::Plain)) => crate::types::LIST_NONE.to_string(),
                Some(Wired::Known(LineType::UnorderedList)) => crate::types::LIST_UNORDERED.to_string(),
                Some(Wired::Known(LineType::OrderedList)) => crate::types::LIST_ORDERED.to_string(),
                Some(Wired::Known(other)) => other.as_wire().to_string(),
                Some(Wired::Unknown(other)) => other.to_string(),
            },
            indentation: l.u32(Field::IndentationLevel).unwrap_or(0),
        })
        .collect()
}

/// The fields of an override entry a `TextRun` reads; the rest are named in
/// its `other`. `styleID` and `fontVersion` say nothing about the style.
const RUN_FIELDS: [Field; 13] = [
    Field::StyleId, Field::FontName, Field::FontVersion, Field::FontSize, Field::FillPaints, Field::StyleIdForFill, Field::TextDecoration,
    Field::TextCase, Field::LetterSpacing, Field::LineHeight, Field::Hyperlink, Field::StyleIdForText, Field::IsOverrideOverTextStyle,
];

// ── Component metadata ────────────────────────────────────────────────────────

fn property_type(definition: &Value) -> ComponentPropertyType {
    match definition.known::<ComponentPropType>(Field::Type) {
        Some(ComponentPropType::Text) => ComponentPropertyType::Text,
        Some(ComponentPropType::Bool) => ComponentPropertyType::Boolean,
        Some(ComponentPropType::InstanceSwap) => ComponentPropertyType::InstanceSwap,
        _ => ComponentPropertyType::Variant,
    }
}

fn property_value_string(v: &Value) -> Option<String> {
    if let Some(b) = v.bool(Field::BoolValue) {
        return Some(b.to_string());
    }
    if let Some(s) = v.str(Field::TextValue) {
        return Some(s.to_string());
    }
    if let Some(s) = v.field(Field::TextValue).and_then(|t| t.str(Field::Characters)) {
        return Some(s.to_string());
    }
    if let Some(s) = v.field(Field::TextDataValue).and_then(|t| t.str(Field::Characters)) {
        return Some(s.to_string());
    }
    if let Some(id) = v.field(Field::GuidValue).and_then(schema::Guid::read_wrapped).map(|id| id.to_string()) {
        return Some(id);
    }
    v.field(Field::SymbolIdValue).and_then(schema::Guid::read_wrapped).map(|id| id.to_string())
}

fn component_properties(nc: &Value) -> Vec<ComponentProperty> {
    nc.array(Field::ComponentPropDefs)
        .iter()
        .filter_map(|def| {
            let id = def.field(Field::Id).and_then(Guid::read)?;
            let prop_type = property_type(def);
            let options = def
                .field(Field::PreferredValues)
                .map(|p| p.array(Field::StringValues))
                .unwrap_or_default()
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            Some(ComponentProperty {
                id,
                name: def.str(Field::Name)?.to_string(),
                prop_type,
                default_value: def.field(Field::InitialValue).and_then(property_value_string),
                options,
            })
        })
        .collect()
}

/// Ordered variant options declared on a component set.
fn variant_options(nc: &Value) -> IndexMap<String, Vec<String>> {
    let mut out = IndexMap::new();
    for entry in nc.array(Field::StateGroupPropertyValueOrders) {
        let Some(property) = entry.str(Field::Property) else { continue };
        let values: Vec<String> = entry
            .array(Field::Values)
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        out.insert(property.to_string(), values);
    }
    out
}

fn property_assignments(nc: &Value) -> IndexMap<Guid, String> {
    let mut out = IndexMap::new();
    for assignment in nc.array(Field::ComponentPropAssignments) {
        let Some(def_id) = assignment.field(Field::DefId).and_then(Guid::read) else { continue };
        let value = assignment
            .field(Field::Value)
            .and_then(property_value_string)
            .or_else(|| {
                assignment.field(Field::VarValue).and_then(|v| v.field(Field::Value)).and_then(property_value_string)
            })
            .unwrap_or_default();
        out.insert(def_id, value);
    }
    out
}

/// `Size=Large, State=Hover` → `{Size: Large, State: Hover}`.
#[must_use]
pub fn parse_variant_name(name: &str) -> IndexMap<String, String> {
    let mut out = IndexMap::new();
    for part in name.split(',') {
        if let Some((key, value)) = part.split_once('=') {
            out.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    out
}

fn bound_variables(nc: &Value, resolver: &VariableResolver<'_>) -> IndexMap<String, Guid> {
    let mut out = IndexMap::new();
    let entries = nc
        .field(Field::VariableConsumptionMap)
        .map(|m| m.array(Field::Entries))
        .unwrap_or_default();
    for entry in entries {
        let Some(field) = entry.known::<VariableField>(Field::VariableField).and_then(variables::variable_field_name)
        else {
            continue;
        };
        let Some(alias) = entry.field(Field::VariableData).and_then(|d| d.field(Field::Value)).and_then(|v| v.field(Field::Alias))
        else {
            continue;
        };
        if let Some(id) = resolver.resolve_alias(alias) {
            out.insert(field.to_string(), id);
        }
    }
    out
}

// ── Conversion ────────────────────────────────────────────────────────────────

impl Converter<'_> {
    /// The node a style field points at.
    ///
    /// A style is addressed either by GUID or, once it comes from a published
    /// library, by asset key — and a file that consumes a shared library uses
    /// the key form for every one of its references. Reading only the GUID form
    /// silently drops the entire design system: every colour, type style and
    /// effect falls back to whatever placeholder paint the node carries.
    pub(crate) fn style_id(&self, nc: &Value, field: Field) -> Option<Guid> {
        nc.guid(field).or_else(|| self.resolver.resolve_alias(nc.field(field)?))
    }

    fn style<'s>(&'s self, nc: &Value, field: Field, wanted: SharedStyleType) -> Option<&'s SharedStyle> {
        let id = self.style_id(nc, field)?;
        self.styles.get(&id).filter(|s| s.style_type == wanted)
    }

    /// Paints for a field, resolving a shared fill style when one is linked.
    fn paints(&self, nc: &Value, paint_field: Field, style_field: Field) -> Vec<Fill> {
        if let Some(style) = self.style(nc, style_field, SharedStyleType::Fill) {
            if !style.fills.is_empty() {
                return style.fills.clone();
            }
        }
        paint::fills(nc.array(paint_field), &self.resolver)
    }

    /// The flattened paths of one geometry field, growing `bounds` to cover
    /// them. Paths whose blob does not decode are dropped rather than guessed
    /// at, so a layer either draws the geometry Figma stored or falls back to a
    /// box.
    fn geometry(
        &self,
        nc: &Value,
        field: Field,
        bounds: &mut Option<vector::Bounds>,
    ) -> Vec<VectorPath> {
        nc.array(field)
            .iter()
            .filter_map(|path| {
                let index = path.u32(Field::CommandsBlob)? as usize;
                let (d, path_bounds) = vector::decode_path(self.blobs.get(index)?)?;
                vector::extend(bounds, path_bounds);
                Some(VectorPath {
                    d,
                    // Figma calls it ODD; SVG calls it evenodd.
                    fill_rule: if path.is(Field::WindingRule, WindingRule::Odd) {
                        "evenodd"
                    } else {
                        "nonzero"
                    }
                    .to_string(),
                })
            })
            .collect()
    }

    fn effects(&self, nc: &Value) -> Vec<crate::types::Effect> {
        if let Some(style) = self.style(nc, Field::StyleIdForEffect, SharedStyleType::Effect) {
            if !style.effects.is_empty() {
                return style.effects.clone();
            }
        }
        paint::effects(nc.array(Field::Effects))
    }

    /// A text's runs: consecutive characters sharing an id other than 0
    /// in `characterStyleIDs`, which Figma cuts short of its trailing zeros,
    /// and what that id's entry in `styleOverrideTable` changes.
    fn text_runs(&self, nc: &Value) -> Vec<TextRun> {
        let Some(data) = nc.field(Field::TextData) else { return Vec::new() };
        let ids: Vec<u32> = data.array(Field::CharacterStyleIDs).iter().map(|v| v.as_u32().unwrap_or(0)).collect();
        let entries: HashMap<u32, &Value> = data
            .array(Field::StyleOverrideTable)
            .iter()
            .filter_map(|e| Some((e.u32(Field::StyleId)?, e)))
            .collect();
        let mut runs = Vec::new();
        let mut start = 0;
        for end in 1..=ids.len() {
            if end < ids.len() && ids[end] == ids[start] {
                continue;
            }
            if let Some(entry) = entries.get(&ids[start]) {
                runs.push(self.text_run(entry, start, end));
            }
            start = end;
        }
        runs
    }

    fn text_run(&self, entry: &Value, start: usize, end: usize) -> TextRun {
        let font = entry.field(Field::FontName);
        let url = entry.field(Field::Hyperlink).and_then(|h| h.str(Field::Url)).map(str::to_string);
        let mut other: Vec<String> = match entry {
            Value::Object(fields) => {
                fields.keys().map(ToString::to_string).filter(|k| Field::from_wire(k).is_none_or(|field| !RUN_FIELDS.contains(&field))).collect()
            }
            _ => Vec::new(),
        };
        // A link to a node, rather than to a web address, is not read.
        if entry.field(Field::Hyperlink).is_some() && url.is_none() {
            other.push("hyperlink".to_string());
        }
        TextRun {
            start,
            end,
            font_family: font.and_then(|f| f.str(Field::Family)).map(str::to_string),
            font_style: font.and_then(|f| f.str(Field::Style)).map(str::to_string),
            font_size: entry.f32(Field::FontSize),
            fills: (entry.field(Field::FillPaints).is_some() || entry.field(Field::StyleIdForFill).is_some())
                .then(|| self.paints(entry, Field::FillPaints, Field::StyleIdForFill)),
            text_decoration: entry.str(Field::TextDecoration).map(str::to_string),
            text_case: entry.str(Field::TextCase).map(str::to_string),
            letter_spacing: entry.field(Field::LetterSpacing).and_then(dimension),
            line_height: entry.field(Field::LineHeight).and_then(dimension),
            hyperlink: url,
            text_style_id: self.style_id(entry, Field::StyleIdForText),
            other,
        }
    }

    fn text_style_for(&self, nc: &Value) -> TextStyle {
        // A linked text style supplies the typography; the node keeps whatever
        // it overrides locally (alignment, auto-resize).
        let mut base = self
            .style(nc, Field::StyleIdForText, SharedStyleType::Text)
            .and_then(|s| s.text_style.clone())
            .unwrap_or_else(|| text_style(nc));
        let local = text_style(nc);
        base.text_align_horizontal = local.text_align_horizontal;
        base.text_align_vertical = local.text_align_vertical;
        base.text_auto_resize = local.text_auto_resize;
        base.max_lines = local.max_lines;
        base
    }

    pub(crate) fn shared_style(&self, id: Guid, nc: &Value) -> Option<SharedStyle> {
        let style_type = match nc.known::<StyleType>(Field::StyleType)? {
            StyleType::Fill => SharedStyleType::Fill,
            StyleType::Text => SharedStyleType::Text,
            StyleType::Effect => SharedStyleType::Effect,
            StyleType::Grid => SharedStyleType::Grid,
            _ => return None,
        };
        Some(SharedStyle {
            id,
            name: nc.str(Field::Name).unwrap_or("Style").to_string(),
            style_type,
            description: nc.str(Field::Description).map(str::to_string).filter(|d| !d.is_empty()),
            fills: paint::fills(nc.array(Field::FillPaints), &self.resolver),
            effects: paint::effects(nc.array(Field::Effects)),
            text_style: (style_type == SharedStyleType::Text).then(|| text_style(nc)),
        })
    }

    /// Overrides an instance applies to the component subtree it renders.
    fn symbol_overrides(&self, nc: &Value) -> Vec<SymbolOverride> {
        let Some(symbol_data) = nc.field(Field::SymbolData) else { return Vec::new() };
        symbol_data
            .array(Field::SymbolOverrides)
            .iter()
            .filter_map(|ov| {
                let path: Vec<Guid> = ov
                    .field(Field::GuidPath)
                    .map(|p| p.array(Field::Guids))
                    .unwrap_or_default()
                    .iter()
                    .filter_map(Guid::read)
                    .collect();
                if path.is_empty() {
                    return None;
                }

                let fills = self.override_paints(ov, Field::FillPaints, Field::StyleIdForFill);
                let strokes = self.override_paints(ov, Field::StrokePaints, Field::StyleIdForStrokeFill);
                let size = match (
                    ov.field(Field::Size).and_then(|s| s.f32(Field::X)),
                    ov.field(Field::Size).and_then(|s| s.f32(Field::Y)),
                ) {
                    (Some(w), Some(h)) => Some((w, h)),
                    _ => None,
                };

                let over = SymbolOverride {
                    path,
                    name: ov.str(Field::Name).map(str::to_string),
                    text: ov.field(Field::TextData).and_then(|t| t.str(Field::Characters)).map(str::to_string),
                    visible: ov.bool(Field::Visible),
                    fills,
                    strokes,
                    size,
                    symbol_id: ov.guid(Field::OverriddenSymbolId),
                    assignments: property_assignments(ov),
                };
                (!over.is_empty() || !over.assignments.is_empty()).then_some(over)
            })
            .collect()
    }

    fn override_paints(&self, ov: &Value, paint_field: Field, style_field: Field) -> Option<Vec<Fill>> {
        if let Some(style) = self.style(ov, style_field, SharedStyleType::Fill) {
            return Some(style.fills.clone());
        }
        ov.field(paint_field).map(|_| paint::fills(ov.array(paint_field), &self.resolver))
    }

    pub(crate) fn node(
        &self,
        nc: &Value,
        id: Guid,
        parent_id: Option<Guid>,
        order_key: String,
    ) -> SceneNode {
        let node_type = resolve_node_type(nc);
        let (x, y, width, height, rotation, flip_x) = transform(nc);

        let stroke_weights = nc
            .bool(Field::BorderStrokeWeightsIndependent)
            .unwrap_or(false)
            .then(|| {
                (
                    nc.f32(Field::BorderTopWeight).unwrap_or(0.0),
                    nc.f32(Field::BorderRightWeight).unwrap_or(0.0),
                    nc.f32(Field::BorderBottomWeight).unwrap_or(0.0),
                    nc.f32(Field::BorderLeftWeight).unwrap_or(0.0),
                )
            });

        // Only the layers Figma draws as paths; a rectangle's geometry adds
        // nothing a div and a border radius do not already say.
        let (fill_geometry, stroke_geometry, vector_bounds) = if node_type.is_vector_shape() {
            let mut bounds = None;
            let fills = self.geometry(nc, Field::FillGeometry, &mut bounds);
            let strokes = self.geometry(nc, Field::StrokeGeometry, &mut bounds);
            (fills, strokes, bounds)
        } else {
            (Vec::new(), Vec::new(), None)
        };

        let variant_values = if nc.array(Field::VariantPropSpecs).is_empty() {
            parse_variant_name(nc.str(Field::Name).unwrap_or(""))
        } else {
            // Resolved against the parent set's definitions in a later pass.
            IndexMap::new()
        };

        SceneNode {
            name: nc.str(Field::Name).unwrap_or("Layer").to_string(),
            id,
            node_type,
            parent_id,
            children: Vec::new(),
            order_key,

            x,
            y,
            width,
            height,
            rotation,
            flip_x,

            opacity: nc.f32(Field::Opacity).unwrap_or(1.0),
            visible: nc.bool(Field::Visible).unwrap_or(true),
            locked: nc.bool(Field::Locked).unwrap_or(false),
            internal_only: nc.bool(Field::InternalOnly).unwrap_or(false),
            blend_mode: nc.str(Field::BlendMode).filter(|m| *m != BlendMode::PassThrough.as_wire()).map(str::to_string),
            fills: self.paints(nc, Field::FillPaints, Field::StyleIdForFill),
            strokes: self.paints(nc, Field::StrokePaints, Field::StyleIdForStrokeFill),
            stroke_weight: nc.f32(Field::StrokeWeight).unwrap_or(0.0),
            stroke_align: nc.str(Field::StrokeAlign).unwrap_or(StrokeAlign::Inside.as_wire()).to_string(),
            stroke_weights,
            dash_pattern: nc
                .array(Field::DashPattern)
                .iter()
                .filter_map(Value::as_f32)
                .collect(),
            effects: self.effects(nc),
            corner_radii: corner_radii(nc),
            // Figma inverts this flag: content is clipped unless masking is off.
            clips_content: nc.bool(Field::FrameMaskDisabled) == Some(false)
                && !nc.bool(Field::ResizeToFit).unwrap_or(false),
            is_mask: nc.bool(Field::Mask).unwrap_or(false),
            mask_type: nc.str(Field::MaskType).unwrap_or(MaskType::Alpha.as_wire()).to_string(),
            fill_geometry,
            stroke_geometry,
            vector_bounds,

            layout: layout(nc),
            min_width: bound(nc, Field::MinSize, Field::X),
            max_width: bound(nc, Field::MaxSize, Field::X),
            min_height: bound(nc, Field::MinSize, Field::Y),
            max_height: bound(nc, Field::MaxSize, Field::Y),
            horizontal_constraint: nc.str(Field::HorizontalConstraint).unwrap_or(ConstraintType::Min.as_wire()).to_string(),
            vertical_constraint: nc.str(Field::VerticalConstraint).unwrap_or(ConstraintType::Min.as_wire()).to_string(),

            text: nc.field(Field::TextData).and_then(|t| t.str(Field::Characters)).map(str::to_string),
            text_style: (node_type == NodeType::Text).then(|| self.text_style_for(nc)),
            text_runs: self.text_runs(nc),
            paragraphs: paragraphs(nc),

            bound_variables: bound_variables(nc, &self.resolver),
            fill_style_id: self.style_id(nc, Field::StyleIdForFill),
            stroke_style_id: self.style_id(nc, Field::StyleIdForStrokeFill),
            text_style_id: self.style_id(nc, Field::StyleIdForText),
            effect_style_id: self.style_id(nc, Field::StyleIdForEffect),

            component_id: nc.field(Field::SymbolData).and_then(|s| s.guid(Field::SymbolId)),
            component_key: nc.str(Field::ComponentKey).map(str::to_string),
            override_key: nc.guid(Field::OverrideKey),
            description: nc
                .str(Field::SymbolDescription)
                .or_else(|| nc.str(Field::Description))
                .map(str::to_string)
                .filter(|d| !d.is_empty()),
            component_properties: component_properties(nc),
            variant_values,
            variant_options: variant_options(nc),
            property_assignments: property_assignments(nc),
            symbol_overrides: self.symbol_overrides(nc),
            export_formats: nc
                .array(Field::ExportSettings)
                .iter()
                .filter_map(|e| e.str(Field::ImageType).map(str::to_string))
                .collect(),
            arc: nc.field(Field::ArcData).and_then(|a| {
                Some((a.f32(Field::StartingAngle)?, a.f32(Field::EndingAngle)?, a.f32(Field::InnerRadius).unwrap_or(0.0)))
            }),
            point_count: nc.field(Field::Count).and_then(|c| c.as_u32()),
            star_inner_scale: nc.f32(Field::StarInnerScale),
        }
    }
}

/// Variant property values declared through `variantPropSpecs`, which reference
/// the parent component set's property definitions by id.
pub(crate) fn variant_prop_specs(nc: &Value) -> Vec<(Guid, String)> {
    nc.array(Field::VariantPropSpecs)
        .iter()
        .filter_map(|spec| {
            Some((
                spec.field(Field::PropDefId).and_then(Guid::read)?,
                spec.str(Field::Value).unwrap_or("").to_string(),
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests;
