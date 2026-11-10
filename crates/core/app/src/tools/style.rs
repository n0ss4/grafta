//! Styling what a model created: fills, strokes, radii, effects, opacity.
//!
//! Every property is written in the field shape Figma writes most, measured
//! across the fixtures, and no
//! geometry is authored: 2,199 real frames carry fills without it. Every
//! value is validated before anything is written, so a refused call changes
//! nothing.

use kiwi::Value as Raw;
use model::{NodeType, VariableType, VariableValue};
use schema::{fields, object, BlendMode, EffectType, Field, Guid, PaintType, StackAlign, StackCounterAlign, StackJustify, StackMode, StackPositioning, StackSize, StackWrap, StrokeAlign, StrokeCap, StrokeJoin, VariableDataType, VariableResolvedDataType, Wire, WireMut, WireName};

use super::read::require_node;
use crate::data::{data, Data, Map};
use crate::Command;
use crate::error::StyleError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// The keys accepted on an instance: it moves, hides, sizes, and keeps
/// fills, strokes and dashes of its own; its other style is its
/// component's.
pub const ACCEPTED_ON_INSTANCE: [&str; 22] = [
    "x",
    "y",
    "name",
    "visible",
    "layoutGrow",
    "layoutAlign",
    "layoutPositioning",
    "componentPropertyReferences",
    "componentProperties",
    "width",
    "height",
    "rotation",
    "constraints",
    "layoutSizingHorizontal",
    "layoutSizingVertical",
    "fills",
    "strokes",
    "strokeWeight",
    "strokeAlign",
    "dashPattern",
    "reactions",
    "scrollBehavior",
];

/// What a vector takes: no radii, no layout of its own, no
/// clipping; its caps and joins besides.
pub const ACCEPTED_ON_VECTOR: [&str; 32] = [
    "layoutGrow",
    "layoutAlign",
    "layoutPositioning",
    "x",
    "y",
    "width",
    "height",
    "name",
    "visible",
    "fills",
    "strokes",
    "strokeWeight",
    "strokeAlign",
    "strokeCap",
    "strokeJoin",
    "effects",
    "opacity",
    "fillStyle",
    "strokeStyle",
    "effectStyle",
    "componentPropertyReferences",
    "layoutSizingHorizontal",
    "layoutSizingVertical",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "constraints",
    "rotation",
    "dashPattern",
    "reactions",
    "scrollBehavior",
];

/// What a shape takes: a vector's keys, its radius (not on an
/// ellipse or a line; checked in the call), and its own values.
pub const ACCEPTED_ON_SHAPE: [&str; 41] = [
    "layoutGrow",
    "layoutAlign",
    "layoutPositioning",
    "x",
    "y",
    "width",
    "height",
    "name",
    "visible",
    "fills",
    "strokes",
    "strokeWeight",
    "strokeAlign",
    "strokeCap",
    "strokeJoin",
    "effects",
    "opacity",
    "cornerRadius",
    "topLeftRadius",
    "topRightRadius",
    "bottomRightRadius",
    "bottomLeftRadius",
    "arcData",
    "pointCount",
    "innerRadius",
    "fillStyle",
    "strokeStyle",
    "effectStyle",
    "componentPropertyReferences",
    "explicitVariableModes",
    "layoutSizingHorizontal",
    "layoutSizingVertical",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "constraints",
    "rotation",
    "dashPattern",
    "reactions",
    "scrollBehavior",
];

/// Auto-layout on a frame, in the plugin API's names.
pub const LAYOUT: [&str; 13] = [
    "layoutMode",
    "itemSpacing",
    "counterAxisSpacing",
    "padding",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "primaryAxisSizingMode",
    "counterAxisSizingMode",
    "primaryAxisAlignItems",
    "counterAxisAlignItems",
    "layoutWrap",
];

/// How a child sits in an auto-layout parent.
pub const CHILD_LAYOUT: [&str; 3] = ["layoutGrow", "layoutAlign", "layoutPositioning"];

/// The keys accepted on a text layer.
pub const ACCEPTED_ON_TEXT: [&str; 39] = [
    "layoutGrow",
    "layoutAlign",
    "layoutPositioning",
    "x",
    "y",
    "width",
    "name",
    "visible",
    "characters",
    "fontFamily",
    "fontStyle",
    "fontSize",
    "fills",
    "textAlignHorizontal",
    "lineHeight",
    "letterSpacing",
    "opacity",
    "fillStyle",
    "textStyle",
    "componentPropertyReferences",
    "layoutSizingHorizontal",
    "layoutSizingVertical",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "constraints",
    "rotation",
    // Rich text.
    "height",
    "runs",
    "paragraphs",
    "paragraphSpacing",
    "paragraphIndent",
    "textDecoration",
    "textCase",
    "textAlignVertical",
    "textAutoResize",
    "reactions",
    "scrollBehavior",
];

/// The keys `set_properties` accepts on a frame: `read_node`'s own names.
pub const ACCEPTED: [&str; 55] = [
    "layoutMode",
    "itemSpacing",
    "counterAxisSpacing",
    "padding",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "primaryAxisSizingMode",
    "counterAxisSizingMode",
    "primaryAxisAlignItems",
    "counterAxisAlignItems",
    "layoutWrap",
    "layoutGrow",
    "layoutAlign",
    "layoutPositioning",
    "x",
    "y",
    "width",
    "height",
    "name",
    "visible",
    "clipsContent",
    "fills",
    "strokes",
    "strokeWeight",
    "strokeAlign",
    "cornerRadius",
    "topLeftRadius",
    "topRightRadius",
    "bottomRightRadius",
    "bottomLeftRadius",
    "effects",
    "opacity",
    "explicitVariableModes",
    "fillStyle",
    "strokeStyle",
    "effectStyle",
    "componentPropertyReferences",
    "layoutSizingHorizontal",
    "layoutSizingVertical",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "constraints",
    "rotation",
    "dashPattern",
    "reactions",
    "scrollBehavior",
    "overflowDirection",
    "overlayPositionType",
    "overlayBackground",
    "overlayBackgroundInteraction",
    "flowStartingPoint",
];

/// The plugin API's names with no counterpart in the format.
pub const AUTO: &str = "AUTO";
pub const INHERIT: &str = "INHERIT";

pub const STROKE_CAPS: [StrokeCap; 3] = [StrokeCap::None, StrokeCap::Round, StrokeCap::Square];
pub const STROKE_JOINS: [StrokeJoin; 3] = [StrokeJoin::Miter, StrokeJoin::Round, StrokeJoin::Bevel];
/// The plugin API's name for what the format calls FOREGROUND_BLUR.
const LAYER_BLUR: &str = "LAYER_BLUR";

pub const PAINT_TYPES: [PaintType; 4] = [PaintType::Solid, PaintType::GradientLinear, PaintType::GradientRadial, PaintType::Image];
const EFFECT_TYPES: [EffectType; 4] = [EffectType::DropShadow, EffectType::InnerShadow, EffectType::ForegroundBlur, EffectType::BackgroundBlur];

pub const STROKE_ALIGNS: [StrokeAlign; 3] = [StrokeAlign::Inside, StrokeAlign::Center, StrokeAlign::Outside];

/// One of `allowed`, as a call names it.
fn chosen<E: WireName>(key: &str, value: &Data, allowed: &[E]) -> Result<E, ToolError> {
    value
        .as_str()
        .and_then(|v| E::listed(v, allowed))
        .ok_or_else(|| invalid(key, format!("one of {}", E::joined(allowed, ", ")), value))
}

const CORNERS: [(&str, Field); 4] = [
    ("topLeftRadius", Field::RectangleTopLeftCornerRadius),
    ("topRightRadius", Field::RectangleTopRightCornerRadius),
    ("bottomRightRadius", Field::RectangleBottomRightCornerRadius),
    ("bottomLeftRadius", Field::RectangleBottomLeftCornerRadius),
];

/// A node's properties changed, by read_node's names.
pub struct SetProperties {
    pub handle: String,
    pub id: String,
    /// As the call gave them: the base refuses anything but a map with keys.
    pub properties: Data,
}

impl Command for SetProperties {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let (id, node_type) = require_node(session, &self.id).map(|node| (node.id, node.node_type))?;
        super::guard::editable(session, id)?;
        // A product's node takes the keys it owns first (a slide's deck
        // fields, a FigJam node's generated layers); what
        // is left goes to the base, unless a behaviour took every key.
        let mut properties = self.properties.as_map().cloned().unwrap_or_default();
        let (mut set, mut applied_any) = (Vec::new(), false);
        for behaviour in session.products.behaviours_of(node_type) {
            let Some(applied) = behaviour.apply(session, id, &mut properties).transpose()? else { continue };
            if applied.whole {
                let mut out = data!({ "id": id, "set": applied.set, "unsaved_edits": session.edits });
                if let Some(note) = applied.note {
                    out["regenerated"] = data!(note);
                }
                return Ok(Output::Data(out));
            }
            set.extend(applied.set);
            applied_any = true;
        }
        if applied_any && properties.is_empty() {
            session.edits += 1;
            return Ok(Output::Data(data!({ "id": id, "set": set, "unsaved_edits": session.edits })));
        }
        if !applied_any {
            return self.base(kit, &self.properties);
        }
        let mut out = match self.base(kit, &Data::Map(properties))? {
            Output::Data(out) => out,
            image => return Ok(image),
        };
        if let Some(Data::List(keys)) = out.get_mut("set") {
            keys.extend(set.into_iter().map(Data::String));
        }
        Ok(Output::Data(out))
    }
}

impl SetProperties {
    /// The properties every node of a type the base knows takes: a frame's, an
    /// instance's, a text's, a vector's, a shape's.
    fn base(&self, kit: &mut Toolkit, properties: &Data) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        // Owned: staging images borrows the session mutably.
        let node = require_node(session, &self.id)?.clone();
        let id = node.id;
        super::guard::editable(session, id)?;
        let accepted: &[&str] = match node.node_type {
            // A component is a frame its instances follow.
            NodeType::Frame | NodeType::Component => &ACCEPTED,
            NodeType::Instance => &ACCEPTED_ON_INSTANCE,
            NodeType::Text => &ACCEPTED_ON_TEXT,
            NodeType::Vector => &ACCEPTED_ON_VECTOR,
            t if super::shapes::kind_of_node(t).is_some() => &ACCEPTED_ON_SHAPE,
            // A product's node may take a base type's keys (a slide, a
            // frame's).
            other => match session.products.behaviours_of(other).find_map(|b| b.base_keys()) {
                Some(keys) => keys,
                None => return Err(StyleError::NotChangeable { id, is: crate::view::type_name(other) }.into()),
            },
        };
        let properties = match properties {
            Data::Map(map) if !map.is_empty() => map,
            _ => {
                return Err(StyleError::NoProperties { accepted: accepted.to_vec() }.into())
            }
        };
        if node.node_type == NodeType::Instance && properties.contains_key("explicitVariableModes") {
            return Err(StyleError::ModesOnInstance.into());
        }
        if let Some(unknown) = properties.keys().find(|k| !accepted.contains(&k.as_str())) {
            let (key, accepted) = (unknown.clone(), accepted.to_vec());
            return Err(if node.node_type == NodeType::Text && ACCEPTED.contains(&unknown.as_str()) {
                StyleError::NotOnText { key, accepted }
            } else if ACCEPTED.contains(&unknown.as_str()) || ACCEPTED_ON_TEXT.contains(&unknown.as_str()) {
                match node.node_type {
                    NodeType::Instance => StyleError::NotOnInstance { key, accepted },
                    other => StyleError::NotOnThis { key, is: crate::view::type_name(other), accepted },
                }
            } else {
                StyleError::NotAProperty { key, accepted }
            }
            .into());
        }

        // A value bound to a variable is written as the number it resolves to,
        // and the binding beside it.
        let (resolved, bindings) = super::binding::take(session, id, properties)?;
        // Images named by path are read now and embedded once the call succeeds
        //.
        let resolved = super::images::stage(session, &resolved)?;
        // Fill, hug and fixed, onto the auto-layout keys.
        let resolved = super::sizing::translate_sizing(session, &node, &resolved)?;
        let properties = &resolved;

        // Validate everything first. Nothing below touches the file until the
        // whole set has been accepted.
        let is_text = node.node_type == NodeType::Text;
        let is_frame = matches!(node.node_type, NodeType::Frame | NodeType::Component);
        let is_vector = node.node_type == NodeType::Vector;
        let shape = super::shapes::kind_of_node(node.node_type);

        // A child of an auto-layout frame is placed by its parent: a position
        // written here would be overwritten by the layout. Its size is its own,
        // and the parent is laid out again around it.
        let parent_layout = super::guard::flowing_in(session, id);
        if let Some(parent) = &parent_layout {
            if let Some(key) = ["x", "y"].into_iter().find(|k| properties.contains_key(*k)) {
                return Err(StyleError::PlacedByLayout { key, id, parent: *parent }.into());
            }
        }
        let mut change = plan(session, properties)?;
        // An instance's own paint is recorded where Figma reads it: in the
        // override entry addressed to its component.
        let own_entry = if node.node_type == NodeType::Instance { own_values(session, &node, properties, &mut change)? } else { None };
        placement(session, id, properties, &mut change)?;
        // Minimum and maximum, constraints and rotation. A
        // rotation keeps a move written above.
        {
            let raw = session.raw(id).cloned().ok_or(DomainError::NoNodeChange { id })?;
            let moved = change.write.iter().rev().find(|(k, _)| *k == Field::Transform).map(|(_, v)| v.clone());
            let (write, remove) = super::sizing::fields(&raw, properties, moved)?;
            change.write.extend(write);
            change.remove.extend(remove);
        }
        // A resized or rotated instance is laid out again by Figma from its
        // component's constraints (measured in Figma).
        if node.node_type == NodeType::Instance && ["width", "height", "rotation"].iter().any(|k| properties.contains_key(*k)) {
            change.remove.push(Field::DerivedSymbolData);
        }
        let notes = if is_text {
            super::text::text_properties(session, id, properties, &mut change.write, &mut change.remove)?
        } else {
            Vec::new()
        };
        layout(session, id, properties, &mut change)?;
        // Geometry Figma derived from the old size, radius or stroke would be
        // stale. Frames without it are a shape Figma writes (2,199 measured).
        if is_frame
            && ["width", "height", "cornerRadius", "topLeftRadius", "topRightRadius", "bottomRightRadius", "bottomLeftRadius", "strokes", "strokeWeight", "strokeAlign"]
                .iter()
                .any(|k| properties.contains_key(*k))
        {
            change.remove.extend([Field::FillGeometry, Field::StrokeGeometry]);
        }
        // A vector's geometry is derived from its network, and Figma derives it
        // again when it is absent (a spike in Figma): a resize makes both stale, a
        // new stroke the stroke's.
        // A shape's own values, and its geometry, derived again by
        // Figma from its size and values (measured in Figma).
        if let Some(kind) = shape {
            use model::author::ShapeKind;
            let corners = ["cornerRadius", "topLeftRadius", "topRightRadius", "bottomRightRadius", "bottomLeftRadius"];
            if matches!(kind, ShapeKind::Ellipse | ShapeKind::Line) {
                if let Some(key) = corners.iter().find(|k| properties.contains_key(**k)) {
                    return Err(crate::error::ShapeError::NoCorners { key }.into());
                }
            }
            if kind != ShapeKind::Rectangle {
                if let Some(key) = corners[1..].iter().find(|k| properties.contains_key(**k)) {
                    return Err(StyleError::CornersOnlyOnRectangle { key }.into());
                }
            }
            if kind == ShapeKind::Line && properties.contains_key("height") {
                return Err(StyleError::LineHeight.into());
            }
            change.write.extend(super::shapes::own_values(kind, properties)?);
            if ["width", "height", "cornerRadius", "arcData", "pointCount", "innerRadius", "strokes", "strokeWeight", "strokeAlign"].iter().any(|k| properties.contains_key(*k)) {
                change.remove.extend([Field::FillGeometry, Field::StrokeGeometry]);
            }
        }
        if is_vector {
            if properties.contains_key("width") || properties.contains_key("height") {
                change.remove.extend([Field::FillGeometry, Field::StrokeGeometry]);
            } else if ["strokes", "strokeWeight", "strokeAlign", "strokeCap", "strokeJoin"].iter().any(|k| properties.contains_key(*k)) {
                change.remove.push(Field::StrokeGeometry);
            }
        }
        // Styles: a pointer and the style's values, or a detach.
        let (style_writes, style_removes) = super::shared_style::apply(session, properties)?;
        change.write.extend(style_writes);
        change.remove.extend(style_removes);
        // A prototype: reactions, scrolling, overlays, a flow.
        super::prototype::plan(session, id, properties, &mut change.write, &mut change.remove)?;
        match super::binding::consumption_map(session, id, &bindings) {
            Some(Some(map)) => change.write.push((Field::VariableConsumptionMap, map)),
            Some(None) => change.remove.push(Field::VariableConsumptionMap),
            None => {}
        }
        let sets_modes = match properties.get("explicitVariableModes") {
            Some(value) => {
                match super::binding::modes(session, id, value)? {
                    Some(map) => change.write.push((Field::VariableModeBySetMap, map)),
                    None => change.remove.push(Field::VariableModeBySetMap),
                }
                true
            }
            None => false,
        };
        let sets_layout = properties.keys().any(|k| LAYOUT.contains(&k.as_str()));
        let resizes = properties.contains_key("width") || properties.contains_key("height") || properties.contains_key("visible");
        let resized_component = (node.node_type == NodeType::Component && resizes).then_some((node.width, node.height));
        // Component properties, validated before anything is written.
        let references = match properties.get("componentPropertyReferences") {
            Some(v) => super::properties::references(session, id, v)?,
            None => Vec::new(),
        };
        let assignments = match properties.get("componentProperties") {
            Some(v) => super::properties::assignments(session, id, v)?,
            None => Vec::new(),
        };
        let before = (node.width, node.height);
        // Every change that alters the room a text takes drops its layout
        // data, and its box is estimated again.
        let refits = is_text && change.remove.contains(&Field::DerivedTextData);
        apply(session, id, change)?;
        if let Some(entry) = own_entry {
            super::overrides::write_overrides(session, id, vec![entry])?;
            session.refresh(&[id]);
        }
        super::images::commit(session);
        // A resized frame's children follow their constraints.
        if properties.contains_key("width") || properties.contains_key("height") {
            super::sizing::follow_constraints(session, id, before);
        }
        if !references.is_empty() || !assignments.is_empty() {
            let guid = id;
            {
                let mut graft = session.graft().map_err(super::write::author_error)?;
                for (field, property) in &references {
                    graft.set_property_reference(guid, *field, property.as_ref().map(|(d, v)| (*d, v))).map_err(super::write::author_error)?;
                }
                for (property, value) in &assignments {
                    graft.assign_property(guid, *property, value).map_err(super::write::author_error)?;
                }
            }
            session.refresh(&[id]);
        }
        // Figma draws what each node stores (measured in Figma): a mode stores
        // everything under it again, and any change its own bound values.
        let reached = if sets_modes { super::binding::subtree(session, id) } else { vec![id] };
        let mut fitted = super::binding::restore(session, &reached)?;
        if refits {
            fitted.merge(super::text::fit(session, &[id])?);
        }
        // A group keeps to its children's bounds.
        if ["x", "y", "width", "height"].iter().any(|k| properties.contains_key(*k)) || fitted.resized.contains(&id) {
            if let Some(parent) = session.graph.node(id).and_then(|n| n.parent_id) {
                super::structure::refit(session, parent)?;
            }
        }
        if let Some(before) = resized_component {
            follow_component_size(session, id, before)?;
        }
        if sets_layout && is_frame && !super::guard::laid_out_by_grafta(session, id) {
            session.laid_out.push(id);
        }
        // A text that changes makes Figma lay its parent out again on open (a
        // ladder of files opened in Figma), so only a size or visibility change needs
        // grafta to take over the parent's layout.
        if let (Some(parent), true) = (parent_layout, resizes) {
            super::guard::take_layout(session, parent)?;
        }

        let set: Vec<&str> = properties.keys().map(String::as_str).collect();
        let mut out = data!({ "id": id, "set": set, "unsaved_edits": session.edits });
        if !notes.is_empty() {
            out["notes"] = data!(notes);
        }
        if node.node_type == NodeType::Instance && properties.contains_key("name") && node.component_id.is_none_or(|c| session.graph.node(c).is_none()) {
            out["name_kept"] = data!("on the instance alone: its component is not in this file, and Figma may show the component's name instead");
        }
        if is_text && fitted.resized.contains(&id) {
            if let Some(text) = session.graph.node(id) {
                out["size"] = data!({ "width": crate::view::num(text.width), "height": crate::view::num(text.height), "provisional": super::text::PROVISIONAL });
            }
            fitted.resized.retain(|t| *t != id);
        }
        super::text::report(&mut out, &fitted);
        if is_text {
            super::text::report_faces(&mut out, session, &super::text::fonts_of(session, id));
        }
        Ok(Output::Data(out))
    }
}

/// The keys an instance takes as its own values.
const OWN_ON_INSTANCE: [(&str, Field); 5] = [
    ("fills", Field::FillPaints),
    ("strokes", Field::StrokePaints),
    ("strokeWeight", Field::StrokeWeight),
    ("strokeAlign", Field::StrokeAlign),
    ("dashPattern", Field::DashPattern),
];

/// An instance's own fills, strokes and dashes: the entry addressed to its
/// component that Figma reads them from (spike v2), and the "no style" sentinel
/// beside a paint the component takes from a colour style (rungs f1, f2), on the
/// entry and on the instance. `None` when none is set.
#[allow(clippy::type_complexity)]
fn own_values(
    session: &Session,
    node: &model::SceneNode,
    properties: &Map,
    change: &mut Change,
) -> Result<Option<(Vec<Guid>, Vec<(Field, Raw)>)>, ToolError> {
    let named: Vec<&str> = OWN_ON_INSTANCE.iter().filter(|(k, _)| properties.contains_key(*k)).map(|(k, _)| *k).collect();
    // A name is the instance's own too, and where the component
    // is unknown it is written on the instance alone, as before.
    let name = properties.get("name").and_then(Data::as_str).filter(|_| node.component_id.is_some_and(|c| session.graph.node(c).is_some()));
    let Some(first) = named.first().or(name.map(|_| &"name")) else { return Ok(None) };
    let component = node
        .component_id
        .filter(|c| session.graph.node(*c).is_some())
        .ok_or_else(|| StyleError::ComponentElsewhere { key: (*first).into(), id: node.id })?;
    let key = super::overrides::key_of(session, component).ok_or(DomainError::NoGuid { id: component })?;
    let stored = session.raw(component);
    let mut fields: Vec<(Field, Raw)> = Vec::new();
    if let Some(name) = name {
        fields.push((Field::Name, Raw::Str(name.to_string())));
    }
    for (property, field) in OWN_ON_INSTANCE {
        if !properties.contains_key(property) {
            continue;
        }
        match change.write.iter().find(|(k, _)| *k == field) {
            Some((_, value)) => fields.push((field, value.clone())),
            // [] clears the component's dashes, as 626 measured entries do.
            None => fields.push((field, Raw::Array(Vec::new()))),
        }
        let pointer = match property {
            "fills" => Field::StyleIdForFill,
            "strokes" => Field::StyleIdForStrokeFill,
            _ => continue,
        };
        if stored.is_some_and(|raw| raw.has(pointer)) {
            fields.push((pointer, super::overrides::no_style()));
            change.write.push((pointer, super::overrides::no_style()));
        }
    }
    Ok(Some((vec![key], fields)))
}

/// The fields to write and the fields to remove.
#[derive(Default)]
struct Change {
    write: Vec<(Field, Raw)>,
    remove: Vec<Field>,
}

fn plan(session: &Session, properties: &Map) -> Result<Change, ToolError> {
    let mut change = Change::default();
    if let Some(value) = properties.get("fills") {
        change.write.push((Field::FillPaints, paints(session, "fills", value)?));
    }
    if let Some(value) = properties.get("strokes") {
        change.write.push((Field::StrokePaints, paints(session, "strokes", value)?));
    }
    if let Some(value) = properties.get("strokeCap") {
        change.write.push((Field::StrokeCap, chosen("strokeCap", value, &STROKE_CAPS)?.into()));
    }
    if let Some(value) = properties.get("strokeJoin") {
        change.write.push((Field::StrokeJoin, chosen("strokeJoin", value, &STROKE_JOINS)?.into()));
    }
    if let Some(value) = properties.get("strokeWeight") {
        let weight = number("strokeWeight", value, 0.0, f32::MAX)?;
        if weight == 0.0 {
            return Err(StyleError::ZeroStrokeWeight.into());
        }
        change.write.push((Field::StrokeWeight, Raw::Float(weight)));
    }
    // Dash then gap lengths, as many pairs as the stroke repeats; [] is solid
    //.
    if let Some(value) = properties.get("dashPattern") {
        let lengths = value.as_list().and_then(|a| a.iter().map(Data::as_f64).collect::<Option<Vec<f64>>>());
        let lengths = lengths
            .filter(|l| l.len() % 2 == 0 && l.iter().all(|d| d.is_finite() && *d >= 0.0))
            .ok_or_else(|| StyleError::DashPattern { got: value.to_string() })?;
        if lengths.is_empty() {
            change.remove.push(Field::DashPattern);
        } else {
            change.write.push((Field::DashPattern, Raw::Array(lengths.into_iter().map(|d| Raw::Float(d as f32)).collect())));
        }
    }
    if let Some(value) = properties.get("strokeAlign") {
        let align = value.as_str().and_then(|a| StrokeAlign::listed(a, &STROKE_ALIGNS));
        let align = align.ok_or_else(|| invalid("strokeAlign", "INSIDE, CENTER or OUTSIDE", value))?;
        change.write.push((Field::StrokeAlign, align.into()));
    }
    radii(properties, &mut change)?;
    if let Some(value) = properties.get("effects") {
        change.write.push((Field::Effects, effects(value)?));
    }
    if let Some(value) = properties.get("opacity") {
        change.write.push((Field::Opacity, Raw::Float(number("opacity", value, 0.0, 1.0)?)));
    }
    Ok(change)
}

/// Auto-layout, written as Figma writes it: a field holding its default is
/// left out, because 51,774 measured auto-layout frames omit the primary
/// sizing (hug) and 49,676 omit the counter sizing (fixed).
fn layout(session: &Session, id: Guid, properties: &Map, change: &mut Change) -> Result<(), ToolError> {
    let pick = |name: &str, allowed: &[&str]| -> Result<Option<String>, ToolError> {
        match properties.get(name) {
            None => Ok(None),
            Some(v) => v
                .as_str()
                .filter(|s| allowed.contains(s))
                .map(|s| Some(s.to_string()))
                .ok_or_else(|| invalid(name, allowed.join(", "), v)),
        }
    };
    fn pick_wire<E: WireName>(properties: &Map, name: &str, allowed: &[E]) -> Result<Option<E>, ToolError> {
        match properties.get(name) {
            None => Ok(None),
            Some(v) => v
                .as_str()
                .and_then(|s| E::listed(s, allowed))
                .map(Some)
                .ok_or_else(|| invalid(name, E::joined(allowed, ", "), v)),
        }
    }
    let non_negative = |name: &str| properties.get(name).map(|v| number(name, v, 0.0, f32::MAX)).transpose();

    // An enum field: written when not the default, removed when it is.
    let mut set_enum = |field: Field, value: Option<Raw>| match value {
        Some(v) => change.write.push((field, v)),
        None => change.remove.push(field),
    };

    if let Some(mode) = pick_wire(properties, "layoutMode", &[StackMode::None, StackMode::Horizontal, StackMode::Vertical])? {
        set_enum(Field::StackMode, (mode != StackMode::None).then(|| mode.into()));
    }
    if let Some(sizing) = pick("primaryAxisSizingMode", &[StackSize::Fixed.as_wire(), AUTO])? {
        set_enum(Field::StackPrimarySizing, (sizing == StackSize::Fixed.as_wire()).then(|| StackSize::Fixed.into()));
    }
    if let Some(sizing) = pick("counterAxisSizingMode", &[StackSize::Fixed.as_wire(), AUTO])? {
        set_enum(Field::StackCounterSizing, (sizing == AUTO).then(|| StackSize::ResizeToFitWithImplicitSize.into()));
    }
    if let Some(align) = pick_wire(properties, "primaryAxisAlignItems", &[StackJustify::Min, StackJustify::Center, StackJustify::Max, StackJustify::SpaceBetween])? {
        // Figma stores "space between" as SPACE_EVENLY in 1,292 of 1,326 frames.
        let stored = match align {
            StackJustify::Min => None,
            StackJustify::SpaceBetween => Some(StackJustify::SpaceEvenly.into()),
            other => Some(other.into()),
        };
        set_enum(Field::StackPrimaryAlignItems, stored);
    }
    if let Some(align) = pick_wire(properties, "counterAxisAlignItems", &[StackAlign::Min, StackAlign::Center, StackAlign::Max])? {
        set_enum(Field::StackCounterAlignItems, (align != StackAlign::Min).then(|| align.into()));
    }
    if let Some(wrap) = pick_wire(properties, "layoutWrap", &[StackWrap::NoWrap, StackWrap::Wrap])? {
        if wrap == StackWrap::Wrap {
            return Err(StyleError::Wrap.into());
        }
        set_enum(Field::StackWrap, None);
    }
    if let Some(align) = pick("layoutAlign", &[INHERIT, StackCounterAlign::Stretch.as_wire()])? {
        set_enum(Field::StackChildAlignSelf, (align == StackCounterAlign::Stretch.as_wire()).then(|| StackCounterAlign::Stretch.into()));
    }
    if let Some(positioning) = pick_wire(properties, "layoutPositioning", &[StackPositioning::Auto, StackPositioning::Absolute])? {
        set_enum(Field::StackPositioning, (positioning == StackPositioning::Absolute).then(|| positioning.into()));
    }
    if let Some(v) = properties.get("layoutGrow") {
        let grow = v.as_f64().filter(|g| *g == 0.0 || *g == 1.0);
        match grow {
            Some(g) if g > 0.0 => change.write.push((Field::StackChildPrimaryGrow, Raw::Float(1.0))),
            Some(_) => change.remove.push(Field::StackChildPrimaryGrow),
            None => return Err(invalid("layoutGrow", "0 or 1", v)),
        }
    }
    if let Some(spacing) = non_negative("itemSpacing")? {
        change.write.push((Field::StackSpacing, Raw::Float(spacing)));
    }
    if let Some(spacing) = non_negative("counterAxisSpacing")? {
        change.write.push((Field::StackCounterSpacing, Raw::Float(spacing)));
    }

    // Padding: all four sides are written together, as on ~16,000 measured
    // frames, because the reader fills a missing right or bottom from the left
    // or top.
    let all = non_negative("padding")?;
    let sides = [
        ("paddingTop", Field::StackVerticalPadding),
        ("paddingRight", Field::StackPaddingRight),
        ("paddingBottom", Field::StackPaddingBottom),
        ("paddingLeft", Field::StackHorizontalPadding),
    ];
    let given: Vec<Option<f32>> = sides.iter().map(|(name, _)| non_negative(name)).collect::<Result<_, _>>()?;
    if all.is_some() || given.iter().any(Option::is_some) {
        let current = session.graph.node(id).map(|n| n.layout.clone()).unwrap_or_default();
        let existing = [current.padding_top, current.padding_right, current.padding_bottom, current.padding_left];
        for (((_, field), value), before) in sides.iter().zip(given).zip(existing) {
            change.write.push((*field, Raw::Float(value.or(all).unwrap_or(before))));
        }
    }
    Ok(())
}

/// Position, size, name, visibility and clipping: values in fields every
/// authored node already carries.
fn placement(
    session: &Session,
    id: Guid,
    properties: &Map,
    change: &mut Change,
) -> Result<(), ToolError> {
    let raw = session.raw(id).ok_or(DomainError::NoNodeChange { id })?;
    let f = |name: &str| properties.get(name).map(|v| number(name, v, f32::MIN, f32::MAX)).transpose();
    let (x, y) = (f("x")?, f("y")?);
    if x.is_some() || y.is_some() {
        let mut transform = match raw.field(Field::Transform) {
            Some(Raw::Object(t)) => t.clone(),
            _ => return Err(StyleError::NoTransform { id }.into()),
        };
        if let Some(x) = x {
            transform.set(Field::M02, Raw::Float(x));
        }
        if let Some(y) = y {
            transform.set(Field::M12, Raw::Float(y));
        }
        change.write.push((Field::Transform, Raw::Object(transform)));
    }
    let size = |name: &str| {
        properties
            .get(name)
            .map(|v| {
                let n = number(name, v, 0.0, f32::MAX)?;
                if n == 0.0 {
                    return Err(ToolError::from(DomainError::NotPositive { at: name.into(), got: None }));
                }
                Ok(n)
            })
            .transpose()
    };
    let (width, height) = (size("width")?, size("height")?);
    if width.is_some() || height.is_some() {
        let current = |axis: Field| raw.field(Field::Size).and_then(|s| s.field(axis)).and_then(Raw::as_f32).unwrap_or(0.0);
        change.write.push((
            Field::Size,
            object([
                (Field::X, Raw::Float(width.unwrap_or_else(|| current(Field::X)))),
                (Field::Y, Raw::Float(height.unwrap_or_else(|| current(Field::Y)))),
            ]),
        ));
    }
    if let Some(value) = properties.get("name") {
        let name = value.as_str().filter(|n| !n.is_empty()).ok_or_else(|| invalid("name", "a non-empty string", value))?;
        change.write.push((Field::Name, Raw::Str(name.to_string())));
    }
    let flag = |name: &str| {
        properties
            .get(name)
            .map(|v| v.as_bool().ok_or_else(|| invalid(name, "true or false", v)))
            .transpose()
    };
    if let Some(visible) = flag("visible")? {
        change.write.push((Field::Visible, Raw::Bool(visible)));
    }
    // Clip content is `frameMaskDisabled: false`, as 12,837 real frames have it.
    if let Some(clip) = flag("clipsContent")? {
        change.write.push((Field::FrameMaskDisabled, Raw::Bool(!clip)));
    }
    Ok(())
}

/// Resize the instances of a resized component that were its old size, as
/// Figma does: an instance keeps a size of its own only when it was given
/// one.
fn follow_component_size(session: &mut Session, component: Guid, (old_w, old_h): (f32, f32)) -> Result<(), ToolError> {
    let Some(node) = session.graph.node(component) else { return Ok(()) };
    let (w, h) = (node.width, node.height);
    let followers: Vec<Guid> = session
        .graph
        .nodes
        .values()
        .filter(|n| n.node_type == NodeType::Instance && n.component_id == Some(component))
        .filter(|n| (n.width - old_w).abs() < 0.01 && (n.height - old_h).abs() < 0.01)
        .map(|n| n.id)
        .collect();
    for id in followers {
        let size = object([(Field::X, Raw::Float(w)), (Field::Y, Raw::Float(h))]);
        apply(session, id, Change { write: vec![(Field::Size, size)], remove: Vec::new() })?;
        session.edits -= 1;
    }
    Ok(())
}

fn apply(session: &mut Session, id: Guid, change: Change) -> Result<(), ToolError> {
    let position = *session
        .raw_index
        .get(&id)
        .ok_or(DomainError::InGraphNotInFile { id })?;
    let Raw::Object(message) = &mut session.fig.node_changes else {
        return Err(DomainError::MessageNotAnObject.into());
    };
    let Some(Raw::Array(changes)) = message.field_mut(Field::NodeChanges) else {
        return Err(DomainError::NoNodeChanges.into());
    };
    let Some(raw) = changes.get_mut(position) else {
        return Err(DomainError::NodeChangeNotAnObject { id }.into());
    };
    session.undo.changing(raw);
    let Raw::Object(fields) = raw else {
        return Err(DomainError::NodeChangeNotAnObject { id }.into());
    };
    for key in change.remove {
        fields.take(key);
    }
    for (key, value) in change.write {
        fields.set(key, value);
    }
    session.refresh(&[id]);
    session.edits += 1;
    Ok(())
}

// ── Values ───────────────────────────────────────────────────────────────────

fn number(name: &str, value: &Data, min: f32, max: f32) -> Result<f32, ToolError> {
    value
        .as_f64()
        .map(|n| n as f32)
        .filter(|n| n.is_finite() && (min..=max).contains(n))
        .ok_or_else(|| {
            let range = if max == f32::MAX { format!("at least {min}") } else { format!("from {min} to {max}") };
            invalid(name, format!("a number {range}"), value)
        })
}

/// `at` given, but not as `expected`.
pub fn invalid(at: &str, expected: impl Into<String>, got: impl std::fmt::Display) -> ToolError {
    DomainError::Invalid { at: at.into(), expected: expected.into(), got: got.to_string() }.into()
}

/// `key`, which an object at `at` does not take; `takes` is what it does.
pub fn unaccepted(at: &str, key: &str, takes: &'static str) -> ToolError {
    DomainError::Unaccepted { at: at.into(), key: key.into(), takes }.into()
}

/// `#RRGGBB` or `#RRGGBBAA`, as channels from 0 to 1.
pub fn hex(name: &str, value: &Data) -> Result<(f32, f32, f32, f32), ToolError> {
    let text = value.as_str().unwrap_or_default();
    let digits = text.strip_prefix('#').unwrap_or("");
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok().map(|b| f32::from(b) / 255.0);
    let parsed = match digits.len() {
        6 if digits.is_ascii() => (|| Some((byte(0)?, byte(2)?, byte(4)?, 1.0)))(),
        8 if digits.is_ascii() => (|| Some((byte(0)?, byte(2)?, byte(4)?, byte(6)?)))(),
        _ => None,
    };
    parsed.ok_or_else(|| invalid(name, "#RRGGBB or #RRGGBBAA", value))
}

pub(crate) fn color((r, g, b, a): (f32, f32, f32, f32)) -> Raw {
    object([
        (Field::R, Raw::Float(r)),
        (Field::G, Raw::Float(g)),
        (Field::B, Raw::Float(b)),
        (Field::A, Raw::Float(a)),
    ])
}

pub fn paints(session: &Session, name: &str, value: &Data) -> Result<Raw, ToolError> {
    let items = value
        .as_list()
        .ok_or_else(|| invalid(name, "an array of paints", value))?;
    let mut out = Vec::new();
    for (index, paint) in items.iter().enumerate() {
        let at = format!("{name}[{index}]");
        let Some(paint) = paint.as_map() else {
            return Err(invalid(&at, "a paint object", paint));
        };
        let named = paint.get("type").and_then(Data::as_str);
        match named.and_then(|t| PaintType::listed(t, &PAINT_TYPES)) {
            Some(PaintType::Solid) => {}
            Some(kind @ (PaintType::GradientLinear | PaintType::GradientRadial)) => {
                out.push(gradient(&at, kind, paint)?);
                continue;
            }
            Some(PaintType::Image) => {
                out.push(super::images::paint(session, &at, paint)?);
                continue;
            }
            _ => {
                return Err(invalid(
                    &format!("{at}.type"),
                    "SOLID, GRADIENT_LINEAR, GRADIENT_RADIAL or IMAGE",
                    named.unwrap_or("nothing"),
                ))
            }
        }
        if let Some(unknown) = paint.keys().find(|k| !matches!(k.as_str(), "type" | "color" | "opacity" | "variable")) {
            return Err(unaccepted(&at, unknown, "a SOLID paint takes type, color, opacity and variable"));
        }

        let bound = match paint.get("variable") {
            Some(v) => Some(variable(session, &format!("{at}.variable"), v)?),
            None => None,
        };
        let (r, g, b, alpha) = match (paint.get("color"), &bound) {
            (Some(c), _) => hex(&format!("{at}.color"), c)?,
            (None, Some((_, resolved))) => *resolved,
            (None, None) => {
                return Err(StyleError::PaintWithoutColour { at }.into())
            }
        };
        let opacity = match paint.get("opacity") {
            Some(o) => number(&format!("{at}.opacity"), o, 0.0, 1.0)?,
            None => 1.0,
        };

        let mut fields = fields([
            (Field::Type, Raw::from(PaintType::Solid)),
            (Field::Color, color((r, g, b, 1.0))),
            (Field::Opacity, Raw::Float(opacity * alpha)),
            (Field::Visible, Raw::Bool(true)),
            (Field::BlendMode, Raw::from(BlendMode::Normal)),
        ]);
        if let Some((guid, _)) = bound {
            fields.set(Field::ColorVar, color_var(guid));
        }
        out.push(Raw::Object(fields));
    }
    Ok(Raw::Array(out))
}

/// A gradient paint in the shape measured on 5,255 real ones: stops, an empty
/// `stopsVar` (as on 4,721 of them), and the plugin API's `gradientTransform`.
fn gradient(at: &str, kind: PaintType, paint: &Map) -> Result<Raw, ToolError> {
    if let Some(unknown) = paint
        .keys()
        .find(|k| !matches!(k.as_str(), "type" | "gradientStops" | "angle" | "gradientTransform" | "opacity"))
    {
        return Err(unaccepted(
            at,
            unknown,
            "a gradient takes type, gradientStops, angle or gradientTransform, and opacity",
        ));
    }
    let stops = paint.get("gradientStops").and_then(Data::as_list).ok_or_else(|| DomainError::Unexpected {
        at: format!("{at}.gradientStops"),
        expected: "an array of { position, color }".into(),
    })?;
    if stops.len() < 2 {
        return Err(StyleError::TooFewStops { at: at.into() }.into());
    }
    let mut raw_stops = Vec::new();
    for (i, stop) in stops.iter().enumerate() {
        let here = format!("{at}.gradientStops[{i}]");
        let position = number(&format!("{here}.position"), stop.get("position").unwrap_or(&Data::Null), 0.0, 1.0)?;
        let (r, g, b, a) = hex(&format!("{here}.color"), stop.get("color").unwrap_or(&Data::Null))?;
        let opacity = match stop.get("opacity") {
            Some(o) => number(&format!("{here}.opacity"), o, 0.0, 1.0)?,
            None => 1.0,
        };
        raw_stops.push(object([
            (Field::Color, color((r, g, b, a * opacity))),
            (Field::Position, Raw::Float(position)),
        ]));
    }
    let matrix = match (paint.get("gradientTransform"), paint.get("angle")) {
        (Some(_), Some(_)) => {
            return Err(StyleError::AngleAndTransform { at: at.into() }.into())
        }
        (Some(m), None) => transform_rows(&format!("{at}.gradientTransform"), m)?,
        (None, Some(angle)) => {
            if kind != PaintType::GradientLinear {
                return Err(StyleError::AngleOnRadial { at: at.into() }.into());
            }
            angle_matrix(number(&format!("{at}.angle"), angle, f32::MIN, f32::MAX)?)
        }
        // Top to bottom for a linear gradient; centred for a radial one.
        (None, None) if kind == PaintType::GradientLinear => angle_matrix(180.0),
        (None, None) => [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    };
    let opacity = match paint.get("opacity") {
        Some(o) => number(&format!("{at}.opacity"), o, 0.0, 1.0)?,
        None => 1.0,
    };
    let [[m00, m01, m02], [m10, m11, m12]] = matrix;
    let transform = object([
        (Field::M00, Raw::Float(m00)),
        (Field::M01, Raw::Float(m01)),
        (Field::M02, Raw::Float(m02)),
        (Field::M10, Raw::Float(m10)),
        (Field::M11, Raw::Float(m11)),
        (Field::M12, Raw::Float(m12)),
    ]);
    Ok(object([
        (Field::Type, kind.into()),
        (Field::Stops, Raw::Array(raw_stops)),
        (Field::StopsVar, Raw::Array(Vec::new())),
        (Field::Transform, transform),
        (Field::Opacity, Raw::Float(opacity)),
        (Field::Visible, Raw::Bool(true)),
        (Field::BlendMode, Raw::from(BlendMode::Normal)),
    ]))
}

/// The plugin API's `gradientTransform` for a CSS-style angle: 180° runs top to
/// bottom and 90° left to right. It maps the layer's unit square (y down) to
/// gradient space, where a linear gradient runs along x from 0 to 1. At 180°
/// this is `[[0,1,0],[-1,0,1]]`, the matrix measured on a real top-to-bottom
/// fade.
#[must_use]
pub fn angle_matrix(degrees: f32) -> [[f32; 3]; 2] {
    let radians = f64::from(degrees).to_radians();
    let (dx, dy) = (radians.sin(), -radians.cos());
    // Snap floating-point dust (6e-17 for cos 90°) so a right angle stays exact.
    let clean = |v: f64| if v.abs() < 1e-9 { 0.0 } else { v as f32 };
    [
        [clean(dx), clean(dy), clean(0.5 - 0.5 * dx - 0.5 * dy)],
        [clean(-dy), clean(dx), clean(0.5 + 0.5 * dy - 0.5 * dx)],
    ]
}

/// `[[a, b, c], [d, e, f]]`, as `read_node` returns a gradient's transform.
fn transform_rows(at: &str, value: &Data) -> Result<[[f32; 3]; 2], ToolError> {
    let bad = || invalid(at, "[[a, b, c], [d, e, f]]", value);
    let rows = value.as_list().filter(|r| r.len() == 2).ok_or_else(bad)?;
    let mut out = [[0.0; 3]; 2];
    for (row, slot) in rows.iter().zip(out.iter_mut()) {
        let cells = row.as_list().filter(|c| c.len() == 3).ok_or_else(bad)?;
        for (cell, target) in cells.iter().zip(slot.iter_mut()) {
            *target = cell.as_f64().filter(|n| n.is_finite()).ok_or_else(bad)? as f32;
        }
    }
    Ok(out)
}

/// A local alias, as `material3.fig` writes 19,883 of them.
fn color_var(variable: Guid) -> Raw {
    let alias = object([(Field::Guid, variable.to_value())]);
    object([
        (Field::Value, object([(Field::Alias, alias)])),
        (Field::DataType, Raw::from(VariableDataType::Alias)),
        (Field::ResolvedDataType, Raw::from(VariableResolvedDataType::Color)),
    ])
}

/// A local colour variable by name or id: its guid and its value in the first
/// mode of its collection.
#[allow(clippy::type_complexity)]
fn variable(
    session: &Session,
    at: &str,
    value: &Data,
) -> Result<(Guid, (f32, f32, f32, f32)), ToolError> {
    let wanted = value
        .as_str()
        .ok_or_else(|| invalid(at, "a variable name or id", value))?;
    let graph = &session.graph;
    let matches: Vec<&model::Variable> =
        graph.variables.values().filter(|v| v.id.to_string() == wanted || v.name == wanted).collect();
    let variable = match matches.as_slice() {
        [one] => *one,
        [] => {
            let names: Vec<&str> = graph.variables.values().map(|v| v.name.as_str()).collect();
            let closest = closest(wanted, &names, 3);
            return Err(StyleError::NoSuchVariable {
                at: at.into(),
                wanted: wanted.into(),
                count: names.len(),
                closest: closest.iter().map(ToString::to_string).collect(),
            }
            .into());
        }
        many => {
            let ids: Vec<String> = many.iter().map(|v| v.id.to_string()).collect();
            return Err(DomainError::AmbiguousVariable { at: at.into(), wanted: wanted.into(), ids }.into());
        }
    };
    if variable.var_type != VariableType::Color {
        return Err(StyleError::NotAColour { at: at.into(), name: variable.name.clone(), kind: variable.var_type }.into());
    }
    if session.raw(variable.id).is_some_and(|raw| raw.field(Field::SourceLibraryKey).is_some()) {
        return Err(DomainError::LibraryVariable { at: at.into(), name: variable.name.clone() }.into());
    }
    let resolved = resolve(graph, variable.id, 0)
        .ok_or_else(|| StyleError::NoColourValue { at: at.into(), name: variable.name.clone() })?;
    Ok((variable.id, resolved))
}

/// A colour variable's value in its collection's first mode, following aliases.
fn resolve(graph: &model::SceneGraph, id: Guid, depth: usize) -> Option<(f32, f32, f32, f32)> {
    if depth > 16 {
        return None;
    }
    let variable = graph.variables.get(&id)?;
    let collection = graph.collections.get(&variable.collection_id)?;
    let value = collection
        .modes
        .first()
        .and_then(|mode| mode.id.declared())
        .and_then(|mode| variable.values_by_mode.get(&mode))
        .or_else(|| variable.values_by_mode.values().next())?;
    match value {
        VariableValue::Color { value } => Some((value.r, value.g, value.b, value.a)),
        VariableValue::Alias { variable_id } => resolve(graph, *variable_id, depth + 1),
        _ => None,
    }
}

fn radii(properties: &Map, change: &mut Change) -> Result<(), ToolError> {
    let uniform = match properties.get("cornerRadius") {
        Some(v) => Some(number("cornerRadius", v, 0.0, f32::MAX)?),
        None => None,
    };
    let mut corners = [None; 4];
    for (slot, (name, _)) in corners.iter_mut().zip(CORNERS) {
        if let Some(v) = properties.get(name) {
            *slot = Some(number(name, v, 0.0, f32::MAX)?);
        }
    }
    if corners.iter().all(Option::is_none) {
        if let Some(radius) = uniform {
            // 933 real frames carry cornerRadius alone.
            change.write.push((Field::CornerRadius, Raw::Float(radius)));
            change.remove.push(Field::RectangleCornerRadiiIndependent);
            change.remove.extend(CORNERS.iter().map(|(_, field)| *field));
        }
        return Ok(());
    }
    // Per-corner: the four fields, the flag, and cornerRadius beside them, as
    // 537 real frames carry them.
    let fallback = uniform.unwrap_or(0.0);
    for (value, (_, field)) in corners.iter().zip(CORNERS) {
        change.write.push((field, Raw::Float(value.unwrap_or(fallback))));
    }
    change.write.push((Field::CornerRadius, Raw::Float(uniform.or(corners[0]).unwrap_or(0.0))));
    change.write.push((Field::RectangleCornerRadiiIndependent, Raw::Bool(true)));
    Ok(())
}

pub fn effects(value: &Data) -> Result<Raw, ToolError> {
    let items = value
        .as_list()
        .ok_or_else(|| invalid("effects", "an array", value))?;
    let mut out = Vec::new();
    for (index, effect) in items.iter().enumerate() {
        let at = format!("effects[{index}]");
        let Some(effect) = effect.as_map() else {
            return Err(invalid(&at, "an object", effect));
        };
        if let Some(unknown) = effect
            .keys()
            .find(|k| !matches!(k.as_str(), "type" | "color" | "opacity" | "offset" | "radius" | "spread"))
        {
            return Err(unaccepted(&at, unknown, "an effect takes type, color, opacity, offset, radius and spread"));
        }
        // The plugin API says LAYER_BLUR; Figma's files say FOREGROUND_BLUR.
        let named = effect.get("type").and_then(Data::as_str);
        let kind = named.and_then(|n| if n == LAYER_BLUR { Some(EffectType::ForegroundBlur) } else { EffectType::listed(n, &EFFECT_TYPES) });
        let Some(kind) = kind else {
            return Err(invalid(
                &format!("{at}.type"),
                "DROP_SHADOW, INNER_SHADOW, LAYER_BLUR or BACKGROUND_BLUR",
                named.unwrap_or("nothing"),
            ));
        };
        let radius = match effect.get("radius") {
            Some(v) => number(&format!("{at}.radius"), v, 0.0, f32::MAX)?,
            None => return Err(StyleError::NoRadius { at }.into()),
        };
        let (r, g, b, a) = match effect.get("color") {
            Some(c) => hex(&format!("{at}.color"), c)?,
            // Figma's own default for a new shadow: black at 25%.
            None => (0.0, 0.0, 0.0, 0.25),
        };
        let a = match effect.get("opacity") {
            Some(o) => number(&format!("{at}.opacity"), o, 0.0, 1.0)? * if effect.contains_key("color") { a } else { 1.0 },
            None => a,
        };
        let (x, y) = match effect.get("offset") {
            Some(o) => {
                let axis = |k: &str| {
                    o.get(k).map_or(Ok(0.0), |v| number(&format!("{at}.offset.{k}"), v, f32::MIN, f32::MAX))
                };
                (axis("x")?, axis("y")?)
            }
            None => (0.0, 0.0),
        };
        let spread = match effect.get("spread") {
            Some(v) => number(&format!("{at}.spread"), v, f32::MIN, f32::MAX)?,
            None => 0.0,
        };
        out.push(object([
            (Field::Type, kind.into()),
            (Field::Color, color((r, g, b, a))),
            (Field::Offset,
                object([(Field::X, Raw::Float(x)), (Field::Y, Raw::Float(y))]),
            ),
            (Field::Radius, Raw::Float(radius)),
            (Field::Spread, Raw::Float(spread)),
            (Field::Visible, Raw::Bool(true)),
            (Field::BlendMode, Raw::from(BlendMode::Normal)),
            (Field::ShowShadowBehindNode, Raw::Bool(false)),
        ]));
    }
    Ok(Raw::Array(out))
}

/// Up to `n` names closest to `wanted`, by edit distance over lower case.
pub fn closest<'a>(wanted: &str, names: &[&'a str], n: usize) -> Vec<&'a str> {
    let wanted = wanted.to_lowercase();
    let mut scored: Vec<(usize, &str)> = names
        .iter()
        .map(|name| (distance(&wanted, &name.to_lowercase()), *name))
        .filter(|(d, name)| *d <= name.len().max(wanted.len()) / 2)
        .collect();
    scored.sort_unstable();
    scored.dedup_by(|a, b| a.1 == b.1);
    scored.into_iter().take(n).map(|(_, name)| name).collect()
}

fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let substitution = previous + usize::from(ca != *cb);
            previous = row[j + 1];
            row[j + 1] = substitution.min(row[j] + 1).min(row[j + 1] + 1);
        }
    }
    row[b.len()]
}
