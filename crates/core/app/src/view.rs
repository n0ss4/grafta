//! A node, in Figma's own terms.
//!
//! The names are the plugin API's (`fills`, `layoutMode`, `itemSpacing`,
//! `characters`…), so a model that knows Figma already knows how to read them.
//! Two rules keep a view small without making it ambiguous:
//!
//! - **A property at Figma's default is left out**, and absence means exactly
//!   that. The defaults are listed once, in `contracts/tools.md`. Spending the
//!   budget on `visible: true` a hundred times would halve what a model sees.
//! - **Nothing stored is hidden.** Fields grafta does not model are named in
//!   `undecoded`, and `read_raw` returns them as stored.

use kiwi::Value as Raw;
use model::{
    AxisSizing, Color, Effect, Fill, FillType, LayoutMode, NodeType, SceneGraph, SceneNode,
};
use schema::{BlendMode, ComponentPropNodeField, ConstraintType, Field, Guid, StackCounterAlign, StackMode, StackPositioning, StackSize, StackWrap, TextAutoResize, TextCase, TextDecoration, Wire, WireName, Wired};
use crate::data::{data, Data, Map};
use crate::port::{Measure, Ports};
use crate::product::Products;

use crate::tools::sizing::{FILL, HUG};
use crate::tools::style::AUTO;

/// Every top-level field of a node change that the scene graph models. Anything
/// else a node change carries, and no product's behaviour reads
/// ([`Products::decodes`]), is reported in `undecoded`.
///
/// Taken from the fields `model`'s converter reads, measured on 2026-09-25.
/// A field added to the converter and not here would be reported as undecoded
/// when it is not: a false alarm, never a silent omission.
const DECODED: &[Field] = &[
    Field::Guid, Field::Phase, Field::ParentIndex, Field::Type, Field::Name, Field::Visible, Field::Locked, Field::Opacity, Field::BlendMode,
    Field::Size, Field::Transform, Field::FillPaints, Field::StrokePaints, Field::StrokeWeight, Field::StrokeAlign,
    Field::BorderStrokeWeightsIndependent, Field::BorderTopWeight, Field::BorderRightWeight, Field::BorderBottomWeight,
    Field::BorderLeftWeight, Field::DashPattern, Field::Effects, Field::CornerRadius, Field::RectangleTopLeftCornerRadius,
    Field::RectangleTopRightCornerRadius, Field::RectangleBottomRightCornerRadius,
    Field::RectangleBottomLeftCornerRadius, Field::FrameMaskDisabled, Field::Mask, Field::InternalOnly,
    Field::FillGeometry, Field::StrokeGeometry, Field::StackMode, Field::StackSpacing, Field::StackCounterSpacing,
    Field::StackPadding, Field::StackHorizontalPadding, Field::StackVerticalPadding, Field::StackPaddingRight,
    Field::StackPaddingBottom, Field::StackPrimarySizing, Field::StackCounterSizing, Field::StackPrimaryAlignItems,
    Field::StackCounterAlignItems, Field::StackJustify, Field::StackCounterAlign, Field::StackWrap, Field::StackPositioning,
    Field::StackChildPrimaryGrow, Field::StackChildAlignSelf, Field::StackReverseZIndex,
    Field::HorizontalConstraint, Field::VerticalConstraint, Field::FontName,
    Field::FontSize, Field::LineHeight, Field::LetterSpacing, Field::TextAlignHorizontal, Field::TextAlignVertical,
    Field::TextCase, Field::TextDecoration, Field::TextAutoResize, Field::MaxLines, Field::ParagraphSpacing, Field::ParagraphIndent, Field::StyleIdForFill,
    Field::StyleIdForStrokeFill, Field::StyleIdForText, Field::StyleIdForEffect, Field::VariableConsumptionMap, Field::VariableModeBySetMap,
    Field::SymbolData, Field::ComponentKey, Field::OverrideKey, Field::SymbolDescription, Field::Description,
    Field::ComponentPropDefs, Field::ComponentPropAssignments, Field::ComponentPropRefs, Field::ArcData, Field::Count, Field::StarInnerScale, Field::VariantPropSpecs, Field::SymbolOverrides,
    Field::ExportSettings, Field::IsStateGroup, Field::StateGroupPropertyValueOrders, Field::ImageScaleMode,
    Field::ResizeToFit, Field::WindingRule,
    // A prototype, read by `tools::prototype`.
    Field::PrototypeInteractions, Field::PrototypeStartingPoint, Field::PrototypeDevice, Field::PrototypeBackgroundColor,
    Field::ScrollDirection, Field::ScrollBehavior, Field::OverlayPositionType, Field::OverlayBackgroundInteraction,
    Field::OverlayBackgroundAppearance,
];

/// How deep and how big a view may get.
#[derive(Clone, Copy)]
pub struct Limits {
    pub depth: usize,
    pub budget: usize,
    pub geometry: bool,
    /// For the root only: skip this many children, to resume a cut.
    pub children_from: usize,
}

/// A vector's paths where the file stores no geometry, only the network it is
/// drawn from: `(data, winding rule)`. Figma derives geometry from a network when it
/// opens a file, so a vector grafta wrote has none until then.
pub type DrawnPaths<'a> = dyn Fn(&SceneNode) -> Vec<(String, &'static str)> + 'a;

/// What a view's walk reads with, the same at every node.
#[derive(Clone, Copy)]
struct Reading<'a> {
    limits: &'a Limits,
    measure: &'a dyn Measure,
    drawn: &'a DrawnPaths<'a>,
}

/// A node's view, with its children to `limits.depth`, cut at a node boundary
/// if the next child would cross the budget.
#[must_use]
pub fn node(session_graph: &SceneGraph, raw: Option<&Raw>, node: &SceneNode, limits: &Limits, ports: &Ports, products: Products, drawn: &DrawnPaths) -> Data {
    // An eighth of the budget is held back for what the accounting below does
    // not see: the arrays and keys that join views together, the path, the
    // `undecoded` list and the `omitted` note itself.
    let inner = Limits { budget: limits.budget - limits.budget / 8, ..*limits };
    let mut used = 0;
    let reading = Reading { limits: &inner, measure: ports.measure.as_ref(), drawn };
    let mut out = subtree(session_graph, node, limits.depth, limits.children_from, &reading, &mut used);
    if let Some(references) = raw.and_then(|raw| property_references(session_graph, node, raw)) {
        out["componentPropertyReferences"] = references;
    }
    if let Some(modes) = raw.and_then(|raw| explicit_modes(session_graph, raw)) {
        out["explicitVariableModes"] = modes;
    }
    // What a node of one product shows besides: a FigJam node's generated
    // layers, a slide's transition and notes.
    if let Some(raw) = raw {
        for behaviour in products.behaviours_of(node.node_type) {
            for (key, value) in behaviour.shows(node, raw, ports.json.as_ref()).into_iter().flatten() {
                out[key.as_str()] = value;
            }
        }
    }
    if let Some(undecoded) = raw.map(|raw| undecoded(raw, products)).filter(|u| !u.is_empty()) {
        out["undecoded"] = data!(undecoded);
    }
    out
}

/// A layer's bindings to its component's properties, by the plugin API's
/// field names and the properties' names.
fn property_references(graph: &SceneGraph, node: &SceneNode, raw: &Raw) -> Option<Data> {
    let refs = raw.field(Field::ComponentPropRefs)?.as_array()?;
    let mut component = node.parent_id.and_then(|p| graph.node(p));
    while let Some(c) = component {
        if c.node_type == NodeType::Component {
            break;
        }
        component = c.parent_id.and_then(|p| graph.node(p));
    }
    let definitions = component.map(|c| c.component_properties.as_slice()).unwrap_or_default();
    let out: Map = refs
        .iter()
        .filter_map(|r| {
            let key = match r.enum_of::<ComponentPropNodeField>(Field::ComponentPropNodeField)? {
                Wired::Known(ComponentPropNodeField::TextData) => "characters",
                Wired::Known(ComponentPropNodeField::Visible) => "visible",
                Wired::Known(ComponentPropNodeField::OverriddenSymbolId) => "mainComponent",
                Wired::Known(other) => other.as_wire(),
                Wired::Unknown(other) => other,
            };
            let def = Guid::read(r.field(Field::DefId)?)?;
            let name = definitions.iter().find(|p| p.id == def).map_or_else(|| def.to_string(), |p| p.name.clone());
            Some((key.to_string(), data!(name)))
        })
        .collect();
    (!out.is_empty()).then_some(Data::Map(out))
}

/// A node's transform origin: the reader keeps a rotated node's unrotated
/// top-left, and this undoes that.
fn plugin_origin(node: &SceneNode) -> (f32, f32) {
    if node.rotation == 0.0 {
        return (node.x, node.y);
    }
    let (sin, cos) = node.rotation.to_radians().sin_cos();
    let x = node.x + (node.width / 2.0) * (1.0 - cos) + sin * (node.height / 2.0);
    let y = node.y + (node.height / 2.0) * (1.0 - cos) - sin * (node.width / 2.0);
    (x, y)
}

/// Fill, hug or fixed along one axis, as the plugin API names a node's sizing
///.
fn sizing(graph: &SceneGraph, node: &SceneNode, horizontal: bool) -> Option<&'static str> {
    let parent_mode = node.parent_id.and_then(|p| graph.node(p)).map_or(LayoutMode::None, |p| p.layout.mode);
    let own = node.layout.mode;
    if parent_mode == LayoutMode::None && own == LayoutMode::None {
        return None;
    }
    if parent_mode != LayoutMode::None && node.layout.positioning != StackPositioning::Absolute.as_wire() {
        let along = (parent_mode == LayoutMode::Horizontal) == horizontal;
        if (along && node.layout.grow > 0.0) || (!along && node.layout.align_self == StackCounterAlign::Stretch.as_wire()) {
            return Some(FILL);
        }
    }
    if own != LayoutMode::None {
        let along = (own == LayoutMode::Horizontal) == horizontal;
        let hug = if along { node.layout.primary_axis_sizing } else { node.layout.counter_axis_sizing };
        if hug == model::AxisSizing::Hug {
            return Some(HUG);
        }
    }
    Some(StackSize::Fixed.as_wire())
}

/// The reader names a bound corner as the design crates do;/// The reader names a bound corner as the design crates do; the plugin API,
/// and `set_properties`, as `topLeftRadius`.
fn plugin_name(field: &str) -> &str {
    match field {
        "cornerRadiusTopLeft" => "topLeftRadius",
        "cornerRadiusTopRight" => "topRightRadius",
        "cornerRadiusBottomLeft" => "bottomLeftRadius",
        "cornerRadiusBottomRight" => "bottomRightRadius",
        other => other,
    }
}

/// A node's own modes, `{ collection: mode }` by name, from its
/// `variableModeBySetMap`. A mode the file does not define is shown by id.
fn explicit_modes(graph: &SceneGraph, raw: &Raw) -> Option<Data> {
    let entries = raw.field(Field::VariableModeBySetMap)?.field(Field::Entries)?.as_array()?;
    let modes: Map = entries
        .iter()
        .filter_map(|e| {
            let set = Guid::read(e.field(Field::VariableSetId)?.field(Field::Guid)?)?;
            let mode = Guid::read(e.field(Field::VariableModeId)?)?;
            let collection = graph.collections.get(&set);
            let mode_name = collection.and_then(|c| c.modes.iter().find(|m| m.id.declared() == Some(mode))).map_or_else(|| mode.to_string(), |m| m.name.clone());
            Some((collection.map_or_else(|| set.to_string(), |c| c.name.clone()), data!(mode_name)))
        })
        .collect();
    (!modes.is_empty()).then_some(Data::Map(modes))
}

fn subtree(graph: &SceneGraph, node: &SceneNode, depth: usize, skip: usize, reading: &Reading, used: &mut usize) -> Data {
    let Reading { limits, measure, drawn } = *reading;
    let mut view = properties(graph, node, limits.geometry);
    if node.fill_geometry.is_empty() && node.stroke_geometry.is_empty() {
        let paths = drawn(node);
        if !paths.is_empty() {
            if limits.geometry {
                let listed: Vec<Data> = paths.iter().map(|(d, rule)| data!({ "data": d, "windingRule": *rule })).collect();
                view.insert("vectorPaths".into(), data!(listed));
            } else {
                view.insert("geometry".into(), data!({ "vectorPaths": paths.len(), "how": "read_node with geometry: true" }));
            }
        }
    }
    // The node's own properties, plus the `childCount`/`children` keys and the
    // separators around it.
    *used += measure.bytes(&Data::Map(view.clone())) + 48;
    let count = node.children.len();
    if count == 0 {
        return Data::Map(view);
    }
    view.insert("childCount".into(), data!(count));
    if depth == 0 {
        view.insert("childrenOmitted".into(), data!(true));
        return Data::Map(view);
    }

    let mut children = Vec::new();
    let mut cut_at = None;
    for (position, child) in graph.children(node.id).enumerate().skip(skip) {
        let before = *used;
        let child_view = subtree(graph, child, depth - 1, 0, reading, used);
        if *used > limits.budget && !children.is_empty() {
            *used = before;
            cut_at = Some(position);
            break;
        }
        children.push(child_view);
    }
    view.insert("children".into(), Data::List(children));
    if skip > 0 {
        view.insert("childrenFrom".into(), data!(skip));
    }
    if let Some(position) = cut_at {
        view.insert(
            "omitted".into(),
            data!({
                "reason": "budget",
                "remaining_children": count - position,
                "resume": { "id": node.id, "children_from": position, "depth": depth },
                "how": "call read_node with this id, depth and children_from",
            }),
        );
    }
    Data::Map(view)
}

/// One node's own properties, defaults omitted.
#[must_use]
pub fn properties(graph: &SceneGraph, node: &SceneNode, geometry: bool) -> Map {
    let mut v = Map::new();
    v.insert("id".into(), data!(node.id));
    v.insert("type".into(), data!(type_name(node.node_type)));
    v.insert("name".into(), data!(node.name));
    if node.node_type != NodeType::Document && node.node_type != NodeType::Canvas {
        // A rotated node's x and y are its transform's origin, the point
        // set_properties writes, as the plugin API has them.
        let (x, y) = plugin_origin(node);
        v.insert("x".into(), num(x));
        v.insert("y".into(), num(y));
        v.insert("width".into(), num(node.width));
        v.insert("height".into(), num(node.height));
    }
    // A shape's own values, in the plugin API's names.
    if let Some((start, end, inner)) = node.arc {
        let full = (end - start - std::f32::consts::TAU).abs() < 1e-4 && inner == 0.0;
        if !full {
            v.insert("arcData".into(), data!({ "startingAngle": num(start), "endingAngle": num(end), "innerRadius": num(inner) }));
        }
    }
    if let Some(count) = node.point_count {
        v.insert("pointCount".into(), data!(count));
    }
    if let Some(inner) = node.star_inner_scale {
        v.insert("innerRadius".into(), num(inner));
    }
    if node.rotation != 0.0 {
        // Counter-clockwise positive, as Figma's panel shows it: the model
        // keeps the other sign, which the code targets use.
        v.insert("rotation".into(), num(-node.rotation));
    }
    if node.flip_x {
        v.insert("flipX".into(), data!(true));
    }
    if !node.visible {
        v.insert("visible".into(), data!(false));
    }
    if node.locked {
        v.insert("locked".into(), data!(true));
    }
    if node.internal_only {
        v.insert("internalOnly".into(), data!(true));
    }
    if node.opacity < 1.0 {
        v.insert("opacity".into(), num(node.opacity));
    }
    if let Some(mode) = node.blend_mode.as_deref().filter(|m| ![BlendMode::PassThrough, BlendMode::Normal].iter().any(|n| n.as_wire() == *m)) {
        v.insert("blendMode".into(), data!(mode));
    }

    let bound = |field: &str| graph.bound_variable_name(node, field).map(str::to_string);
    if !node.fills.is_empty() {
        v.insert("fills".into(), paints(graph, &node.fills, bound("fills")));
    }
    if let Some(style) = style_name(graph, node.fill_style_id) {
        v.insert("fillStyle".into(), data!(style));
    }
    if !node.strokes.is_empty() {
        v.insert("strokes".into(), paints(graph, &node.strokes, bound("strokes")));
        v.insert("strokeWeight".into(), num(node.stroke_weight));
        v.insert("strokeAlign".into(), data!(node.stroke_align));
        if let Some((top, right, bottom, left)) = node.stroke_weights {
            v.insert(
                "strokeWeights".into(),
                data!({ "top": top, "right": right, "bottom": bottom, "left": left }),
            );
        }
        if !node.dash_pattern.is_empty() {
            v.insert("dashPattern".into(), data!(node.dash_pattern));
        }
    }
    if let Some(style) = style_name(graph, node.stroke_style_id) {
        v.insert("strokeStyle".into(), data!(style));
    }
    if !node.effects.is_empty() {
        v.insert("effects".into(), Data::List(node.effects.iter().map(effect).collect()));
    }
    if let Some(style) = style_name(graph, node.effect_style_id) {
        v.insert("effectStyle".into(), data!(style));
    }
    let radii = node.corner_radii;
    if !radii.is_zero() {
        if radii.is_uniform() {
            v.insert("cornerRadius".into(), num(radii.top_left));
        } else {
            v.insert("topLeftRadius".into(), num(radii.top_left));
            v.insert("topRightRadius".into(), num(radii.top_right));
            v.insert("bottomRightRadius".into(), num(radii.bottom_right));
            v.insert("bottomLeftRadius".into(), num(radii.bottom_left));
        }
    }
    if node.clips_content {
        v.insert("clipsContent".into(), data!(true));
    }
    if node.is_mask {
        v.insert("isMask".into(), data!(true));
    }

    layout(graph, node, &mut v);

    for (key, horizontal) in [("layoutSizingHorizontal", true), ("layoutSizingVertical", false)] {
        if let Some(value) = sizing(graph, node, horizontal) {
            v.insert(key.into(), data!(value));
        }
    }
    let min = ConstraintType::Min.as_wire();
    if node.horizontal_constraint != min || node.vertical_constraint != min {
        v.insert(
            "constraints".into(),
            data!({ "horizontal": node.horizontal_constraint, "vertical": node.vertical_constraint }),
        );
    }
    for (key, value) in [
        ("minWidth", node.min_width),
        ("maxWidth", node.max_width),
        ("minHeight", node.min_height),
        ("maxHeight", node.max_height),
    ] {
        if let Some(value) = value {
            v.insert(key.into(), num(value));
        }
    }

    if let Some(text) = &node.text {
        v.insert("characters".into(), data!(text));
    }
    if let Some(style) = &node.text_style {
        let mut s = Map::new();
        s.insert("fontFamily".into(), data!(style.font_family));
        s.insert("fontStyle".into(), data!(style.font_style));
        s.insert("fontSize".into(), num(style.font_size));
        s.insert("fontWeight".into(), data!(style.font_weight));
        if let Some(line_height) = style.line_height {
            s.insert("lineHeight".into(), data!({ "value": line_height.value, "unit": line_height.unit }));
        }
        if let Some(spacing) = style.letter_spacing {
            s.insert("letterSpacing".into(), data!({ "value": spacing.value, "unit": spacing.unit }));
        }
        s.insert("textAlignHorizontal".into(), data!(style.text_align_horizontal));
        s.insert("textAlignVertical".into(), data!(style.text_align_vertical));
        if style.text_case != TextCase::Original.as_wire() {
            s.insert("textCase".into(), data!(style.text_case));
        }
        if style.text_decoration != TextDecoration::None.as_wire() {
            s.insert("textDecoration".into(), data!(style.text_decoration));
        }
        if style.text_auto_resize != TextAutoResize::None.as_wire() {
            s.insert("textAutoResize".into(), data!(style.text_auto_resize));
        }
        if let Some(lines) = style.max_lines {
            s.insert("maxLines".into(), data!(lines));
        }
        if style.paragraph_spacing != 0.0 {
            s.insert("paragraphSpacing".into(), num(style.paragraph_spacing));
        }
        if style.paragraph_indent != 0.0 {
            s.insert("paragraphIndent".into(), num(style.paragraph_indent));
        }
        v.insert("style".into(), Data::Map(s));
    }
    if let (Some(text), false) = (&node.text, node.text_runs.is_empty()) {
        v.insert("runs".into(), runs(graph, text, &node.text_runs));
    }
    if !node.paragraphs.is_empty() {
        let paragraphs: Vec<Data> = node
            .paragraphs
            .iter()
            .map(|p| {
                if p.list_type == model::LIST_NONE {
                    data!({ "listType": model::LIST_NONE })
                } else {
                    data!({ "listType": p.list_type, "indentation": p.indentation })
                }
            })
            .collect();
        v.insert("paragraphs".into(), data!(paragraphs));
    }
    if let Some(style) = style_name(graph, node.text_style_id) {
        v.insert("textStyle".into(), data!(style));
    }

    let others: Map = node
        .bound_variables
        .iter()
        .filter(|(field, _)| !matches!(field.as_str(), "fills" | "strokes"))
        .map(|(field, id)| (plugin_name(field).to_string(), data!(graph.variable_label(*id))))
        .collect();
    if !others.is_empty() {
        v.insert("boundVariables".into(), Data::Map(others));
    }

    component(graph, node, &mut v);

    if !node.fill_geometry.is_empty() || !node.stroke_geometry.is_empty() {
        if geometry {
            let paths = |paths: &[model::VectorPath]| {
                paths.iter().map(|p| data!({ "d": p.d, "windingRule": p.fill_rule })).collect::<Vec<_>>()
            };
            v.insert("fillGeometry".into(), data!(paths(&node.fill_geometry)));
            v.insert("strokeGeometry".into(), data!(paths(&node.stroke_geometry)));
        } else {
            v.insert(
                "geometry".into(),
                data!({
                    "fillPaths": node.fill_geometry.len(),
                    "strokePaths": node.stroke_geometry.len(),
                    "how": "read_node with geometry: true",
                }),
            );
        }
    }
    if !node.export_formats.is_empty() {
        v.insert("exportSettings".into(), data!(node.export_formats));
    }
    v
}

fn layout(graph: &SceneGraph, node: &SceneNode, v: &mut Map) {
    let l = &node.layout;
    if l.mode != LayoutMode::None {
        v.insert("layoutMode".into(), data!(if l.mode == LayoutMode::Horizontal { StackMode::Horizontal } else { StackMode::Vertical }.as_wire()));
        v.insert("itemSpacing".into(), num(l.item_spacing));
        if l.counter_axis_spacing != 0.0 {
            v.insert("counterAxisSpacing".into(), num(l.counter_axis_spacing));
        }
        for (key, value) in [
            ("paddingTop", l.padding_top),
            ("paddingRight", l.padding_right),
            ("paddingBottom", l.padding_bottom),
            ("paddingLeft", l.padding_left),
        ] {
            if value != 0.0 {
                v.insert(key.into(), num(value));
            }
        }
        let sizing = |s: AxisSizing| match s {
            AxisSizing::Fixed => StackSize::Fixed.as_wire(),
            AxisSizing::Hug => AUTO,
            AxisSizing::Fill => FILL,
        };
        v.insert("primaryAxisSizingMode".into(), data!(sizing(l.primary_axis_sizing)));
        v.insert("counterAxisSizingMode".into(), data!(sizing(l.counter_axis_sizing)));
        v.insert("primaryAxisAlignItems".into(), data!(l.primary_axis_align));
        v.insert("counterAxisAlignItems".into(), data!(l.counter_axis_align));
        if l.wrap {
            v.insert("layoutWrap".into(), data!(StackWrap::Wrap.as_wire()));
        }
        if l.reverse_z_index {
            v.insert("itemReverseZIndex".into(), data!(true));
        }
    }
    // Child-side properties matter only inside an auto-layout parent.
    let in_auto_layout = node
        .parent_id
        .and_then(|id| graph.node(id))
        .is_some_and(|parent| parent.layout.mode != LayoutMode::None);
    if in_auto_layout {
        if l.positioning == StackPositioning::Absolute.as_wire() {
            v.insert("layoutPositioning".into(), data!(l.positioning));
        }
        if l.grow != 0.0 {
            v.insert("layoutGrow".into(), num(l.grow));
        }
        if l.align_self != StackCounterAlign::Auto.as_wire() {
            v.insert("layoutAlign".into(), data!(l.align_self));
        }
    }
}

fn component(graph: &SceneGraph, node: &SceneNode, v: &mut Map) {
    if let Some(id) = &node.component_id {
        v.insert("componentId".into(), data!(id));
        match graph.component_of(node) {
            Some(definition) => {
                let name = match definition.parent_id.and_then(|p| graph.node(p)) {
                    Some(set) if set.node_type == NodeType::ComponentSet => set.name.clone(),
                    _ => definition.name.clone(),
                };
                v.insert("componentName".into(), data!(name));
                if !definition.variant_values.is_empty() {
                    v.insert("variant".into(), data!(definition.variant_values));
                }
            }
            None => {
                v.insert(
                    "componentNotInFile".into(),
                    data!("the definition lives in another file, a published library"),
                );
            }
        }
    }
    if let Some(key) = &node.component_key {
        v.insert("componentKey".into(), data!(key));
    }
    if node.node_type == NodeType::Component && !node.variant_values.is_empty() {
        v.insert("variantProperties".into(), data!(node.variant_values));
    }
    if !node.variant_options.is_empty() {
        v.insert("variantOptions".into(), data!(node.variant_options));
    }
    if !node.component_properties.is_empty() {
        let definitions: Vec<Data> = node
            .component_properties
            .iter()
            .map(|p| {
                let mut d = data!({ "id": p.id, "name": p.name, "type": p.prop_type });
                if let Some(default) = &p.default_value {
                    d["default"] = data!(default);
                }
                if !p.options.is_empty() {
                    d["options"] = data!(p.options);
                }
                d
            })
            .collect();
        v.insert("componentPropertyDefinitions".into(), Data::List(definitions));
    }
    if !node.property_assignments.is_empty() {
        // By property name, as the plugin API shows them; an id
        // the component does not define is kept as it is.
        let definitions = node.component_id.and_then(|c| graph.node(c)).map(|c| c.component_properties.as_slice()).unwrap_or_default();
        let values: Map = node
            .property_assignments
            .iter()
            .map(|(id, value)| {
                let definition = definitions.iter().find(|p| p.id == *id);
                let shown = match definition.map(|p| &p.prop_type) {
                    Some(model::ComponentPropertyType::InstanceSwap) => {
                        value.parse().ok().and_then(|swap| graph.node(swap)).map_or_else(|| value.clone(), |c| c.name.clone())
                    }
                    _ => value.clone(),
                };
                (definition.map_or_else(|| id.to_string(), |p| p.name.clone()), data!(shown))
            })
            .collect();
        v.insert("componentProperties".into(), Data::Map(values));
    }
    let overrides = crate::tools::overrides::describe(graph, node);
    if !overrides.is_empty() {
        v.insert("overrides".into(), Data::List(overrides));
    }
    if let Some(description) = node.description.as_deref().filter(|d| !d.is_empty()) {
        v.insert("description".into(), data!(description));
    }
}

fn paints(graph: &SceneGraph, paints: &[Fill], bound: Option<String>) -> Data {
    Data::List(
        paints
            .iter()
            .map(|paint| {
                let mut p = Map::new();
                p.insert("type".into(), data!(paint.fill_type));
                if let Some(color) = paint.color {
                    p.insert("color".into(), data!(hex(color)));
                }
                let opacity = paint.opacity * paint.color.map_or(1.0, |c| c.a);
                if opacity < 1.0 {
                    p.insert("opacity".into(), num(opacity));
                }
                if !paint.visible {
                    p.insert("visible".into(), data!(false));
                }
                if let Some(mode) = paint.blend_mode.as_deref().filter(|m| *m != BlendMode::Normal.as_wire()) {
                    p.insert("blendMode".into(), data!(mode));
                }
                if paint.fill_type.is_gradient() {
                    let stops: Vec<Data> = paint
                        .gradient_stops
                        .iter()
                        .map(|s| {
                            let mut stop = data!({ "position": s.position, "color": hex(s.color) });
                            if s.color.a < 1.0 {
                                stop["opacity"] = num(s.color.a);
                            }
                            stop
                        })
                        .collect();
                    p.insert("gradientStops".into(), Data::List(stops));
                    if let Some(m) = paint.gradient_transform {
                        p.insert(
                            "gradientTransform".into(),
                            data!([[m.m00, m.m01, m.m02], [m.m10, m.m11, m.m12]]),
                        );
                    }
                }
                if paint.fill_type == FillType::Image {
                    if let Some(hash) = &paint.image_hash {
                        p.insert("imageHash".into(), data!(hash));
                    }
                    if let Some(mode) = &paint.image_scale_mode {
                        p.insert("scaleMode".into(), data!(mode));
                    }
                }
                let variable = paint
                    .variable_id
                    .map(|id| graph.variable_label(id))
                    .or_else(|| bound.clone());
                if let Some(variable) = variable {
                    p.insert("variable".into(), data!(variable));
                }
                Data::Map(p)
            })
            .collect(),
    )
}

/// Paints as `read_node` shows them, for other views to reuse.
#[must_use]
pub fn paints_json(graph: &SceneGraph, fills: &[Fill]) -> Data {
    paints(graph, fills, None)
}

pub fn effect(effect: &Effect) -> Data {
    let mut e = data!({ "type": effect.effect_type, "radius": effect.radius });
    if let Some(color) = effect.color {
        e["color"] = data!(hex(color));
        if color.a < 1.0 {
            e["opacity"] = num(color.a);
        }
    }
    if effect.offset_x != 0.0 || effect.offset_y != 0.0 {
        e["offset"] = data!({ "x": effect.offset_x, "y": effect.offset_y });
    }
    if effect.spread != 0.0 {
        e["spread"] = num(effect.spread);
    }
    if !effect.visible {
        e["visible"] = data!(false);
    }
    e
}

/// A text's runs in `create_text`'s names: the characters each
/// covers, and only what it changes from the layer. Fields a run sets that
/// grafta does not read are named under `other`.
fn runs(graph: &SceneGraph, text: &str, runs: &[model::TextRun]) -> Data {
    let units: Vec<u16> = text.encode_utf16().collect();
    let slice = |start: usize, end: usize| String::from_utf16_lossy(units.get(start..end.min(units.len())).unwrap_or_default());
    let mut out = Vec::new();
    let mut at = 0;
    for run in runs {
        // The characters before a run, in the layer's own style.
        if run.start > at {
            out.push(data!({ "characters": slice(at, run.start) }));
        }
        let mut r = Map::new();
        r.insert("characters".into(), data!(slice(run.start, run.end)));
        if let Some(family) = &run.font_family {
            r.insert("fontFamily".into(), data!(family));
        }
        if let Some(style) = &run.font_style {
            r.insert("fontStyle".into(), data!(style));
        }
        if let Some(size) = run.font_size {
            r.insert("fontSize".into(), num(size));
        }
        if let Some(fills) = &run.fills {
            r.insert("fills".into(), paints_json(graph, fills));
        }
        if let Some(decoration) = &run.text_decoration {
            r.insert("textDecoration".into(), data!(decoration));
        }
        if let Some(case) = &run.text_case {
            r.insert("textCase".into(), data!(case));
        }
        if let Some(spacing) = run.letter_spacing {
            r.insert("letterSpacing".into(), data!({ "value": spacing.value, "unit": spacing.unit }));
        }
        if let Some(line_height) = run.line_height {
            r.insert("lineHeight".into(), data!({ "value": line_height.value, "unit": line_height.unit }));
        }
        if let Some(url) = &run.hyperlink {
            r.insert("hyperlink".into(), data!({ "type": "URL", "value": url }));
        }
        if let Some(id) = run.text_style_id {
            r.insert("textStyle".into(), data!(style_name(graph, Some(id)).map_or_else(|| id.to_string(), str::to_string)));
        }
        if !run.other.is_empty() {
            r.insert("other".into(), data!(run.other));
        }
        out.push(Data::Map(r));
        at = run.end;
    }
    if at < units.len() {
        out.push(data!({ "characters": slice(at, units.len()) }));
    }
    Data::List(out)
}

fn style_name(graph: &SceneGraph, id: Option<Guid>) -> Option<&str> {
    graph.styles.get(&id?).map(|style| style.name.as_str())
}

/// `#RRGGBB`, exact to 8 bits per channel: what Figma's own inspector shows.
#[must_use]
pub fn hex(color: Color) -> String {
    color.to_hex()
}

/// The plugin API's name for a node type.
#[must_use]
pub fn type_name(node_type: NodeType) -> &'static str {
    node_type.plugin_name()
}

/// Parse a type name a model sends, in the plugin API's spelling.
#[must_use]
pub fn parse_type(name: &str) -> Option<NodeType> {
    let wanted = name.to_ascii_uppercase();
    NodeType::NAMED.into_iter().find(|t| type_name(*t) == wanted || (wanted == "CANVAS" && *t == NodeType::Canvas))
}

/// Stored fields this view does not model, by name.
#[must_use]
pub fn undecoded(raw: &Raw, products: Products) -> Vec<String> {
    let Raw::Object(fields) = raw else { return Vec::new() };
    let mut out: Vec<String> =
        fields.keys().filter(|key| Field::from_wire(key).is_none_or(|field| !DECODED.contains(&field) && !products.decodes(field))).map(str::to_string).collect();
    // `textData` is read for its characters, its runs and its paragraphs'
    // lists. The glyphs and line layout Figma stores beside them
    // are not decoded.
    if let Some(Raw::Object(text)) = fields.field(Field::TextData) {
        let rest: Vec<&str> = text
            .keys()
            .filter(|k| !matches!(*k, "characters" | "characterStyleIDs" | "styleOverrideTable" | "lines"))
            .collect();
        out.retain(|key| key != "textData");
        if !rest.is_empty() {
            out.push(format!("textData ({})", rest.join(", ")));
        }
    }
    out
}

/// A raw node change as JSON. Bytes become their length and a fingerprint.
#[must_use]
pub fn raw_json(raw: &Raw) -> Data {
    match raw {
        Raw::Null => Data::Null,
        Raw::Bool(b) => data!(b),
        Raw::Int(n) => data!(n),
        Raw::Uint(n) => data!(n),
        Raw::Int64(n) => data!(n),
        Raw::Uint64(n) => data!(n),
        Raw::Float(n) => num(*n),
        Raw::Str(s) => data!(s),
        Raw::Enum(s) => data!(s),
        Raw::Bytes(bytes) => data!({ "bytes": bytes.len(), "fnv64": format!("{:016x}", crate::fnv(bytes)) }),
        Raw::Array(items) => Data::List(items.iter().map(raw_json).collect()),
        Raw::Object(fields) => {
            Data::Map(fields.iter().map(|(k, v)| (k.to_string(), raw_json(v))).collect())
        }
    }
}

/// A float as JSON, without `12.000000476837158`-style noise from `f32`.
#[must_use]
pub fn num(value: f32) -> Data {
    if value.is_finite() && value == value.trunc() && value.abs() < 1e15 {
        data!(value as i64)
    } else if value.is_finite() {
        let short: f64 = format!("{value}").parse().unwrap_or(f64::from(value));
        data!(short)
    } else {
        Data::Null
    }
}

#[cfg(test)]
mod tests;
