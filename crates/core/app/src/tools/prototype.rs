//! A prototype, in the plugin API's terms: a layer's `reactions`, a page's
//! `flowStartingPoints`, a frame's scrolling and overlay settings.
//!
//! The file stores a layer's interactions as `prototypeInteractions`: each an
//! event and its actions. 77,145 were measured in four fixtures, nearly all
//! interactive components changing to another variant (`SWAP_STATE`, the
//! plugin API's `CHANGE_TO`). Names are mapped where the file and the plugin
//! API are known to mean the same thing (specs/107, research R1). Where they
//! are not yet known, a directional transition or a spring, the file's own
//! name is given, never a guess; a field this module does not map is shown
//! under `notModelled`, as stored.

use kiwi::Value as Raw;
use model::{NodeType, SceneGraph, SceneNode};
use schema::{
    object, ConnectionType, EasingType, Field, Guid, InteractionType, NavigationType, OverlayBackgroundInteraction, OverlayBackgroundType,
    OverlayPositionType, PrototypeDeviceType, ScrollBehavior, ScrollDirection, TransitionType, TriggerDevice, Wire, WireName,
};

use crate::data::{data, Data, Map};
use crate::view::{hex, num, raw_json};
use super::style::{invalid, unaccepted};
use crate::{Session, ToolError};

/// A destination no node has: Figma stores an unset one as all ones.
const UNSET: u32 = u32::MAX;

/// The file's triggers against the plugin API's, where they are known to be
/// the same trigger. `MOUSE_IN`, `MOUSE_OUT` and `ON_VOICE` are not, yet.
const TRIGGERS: &[(InteractionType, &str)] = &[
    (InteractionType::OnClick, "ON_CLICK"),
    (InteractionType::OnHover, "ON_HOVER"),
    (InteractionType::OnPress, "ON_PRESS"),
    (InteractionType::Drag, "ON_DRAG"),
    (InteractionType::AfterTimeout, "AFTER_TIMEOUT"),
    (InteractionType::MouseEnter, "MOUSE_ENTER"),
    (InteractionType::MouseLeave, "MOUSE_LEAVE"),
    (InteractionType::MouseUp, "MOUSE_UP"),
    (InteractionType::MouseDown, "MOUSE_DOWN"),
    (InteractionType::OnKeyDown, "ON_KEY_DOWN"),
    (InteractionType::OnMediaHit, "ON_MEDIA_HIT"),
    (InteractionType::OnMediaEnd, "ON_MEDIA_END"),
];

const NAVIGATIONS: &[(NavigationType, &str)] = &[
    (NavigationType::Navigate, "NAVIGATE"),
    (NavigationType::Overlay, "OVERLAY"),
    (NavigationType::Swap, "SWAP"),
    (NavigationType::ScrollTo, "SCROLL_TO"),
    (NavigationType::SwapState, "CHANGE_TO"),
];

/// Transitions with no direction. The directional ones (`MOVE_FROM_LEFT`…)
/// are given by the file's name until a file saved from Figma settles which
/// way the plugin API's `direction` points (research R2).
const TRANSITIONS: &[(TransitionType, &str)] = &[
    (TransitionType::Dissolve, "DISSOLVE"),
    (TransitionType::SmartAnimate, "SMART_ANIMATE"),
    (TransitionType::ScrollAnimate, "SCROLL_ANIMATE"),
];

/// Two springs are known by the numbers they store (mass, stiffness,
/// damping): `GENTLE_SPRING` 1, 100, 15 is Figma's Gentle and
/// `SPRING_PRESET_TWO` 1, 600, 15 its Bouncy (research R4). The other presets
/// are given by the file's name.
const EASINGS: &[(EasingType, &str)] = &[
    (EasingType::GentleSpring, "GENTLE"),
    (EasingType::SpringPresetTwo, "BOUNCY"),
    (EasingType::InCubic, "EASE_IN"),
    (EasingType::OutCubic, "EASE_OUT"),
    (EasingType::InoutCubic, "EASE_IN_AND_OUT"),
    (EasingType::Linear, "LINEAR"),
    (EasingType::InBackCubic, "EASE_IN_BACK"),
    (EasingType::OutBackCubic, "EASE_OUT_BACK"),
    (EasingType::InoutBackCubic, "EASE_IN_AND_OUT_BACK"),
    (EasingType::CustomCubic, "CUSTOM_CUBIC_BEZIER"),
];

const FLAGS: &[(Field, &str)] = &[
    (Field::TransitionPreserveScroll, "preserveScrollPosition"),
    (Field::TransitionResetScrollPosition, "resetScrollPosition"),
    (Field::TransitionResetVideoPosition, "resetVideoPosition"),
    (Field::TransitionResetInteractiveComponents, "resetInteractiveComponents"),
];

const SCROLL_BEHAVIORS: &[(ScrollBehavior, &str)] = &[
    (ScrollBehavior::Scrolls, "SCROLLS"),
    (ScrollBehavior::FixedWhenChildOfScrollingFrame, "FIXED"),
    (ScrollBehavior::StickyScrolls, "STICKY_SCROLLS"),
];

/// What a node of the file adds to its view: its reactions; for a page, its
/// flows, device and background; for a frame, scrolling and overlay
/// settings; for a layer, how it scrolls.
pub fn show(session: &Session, node: &SceneNode, out: &mut Data) {
    let Some(raw) = session.raw(node.id) else { return };
    let graph = &session.graph;
    let reactions: Vec<Data> = raw.array(Field::PrototypeInteractions).iter().map(|i| reaction(graph, i)).collect();
    if !reactions.is_empty() {
        out["reactions"] = Data::List(reactions);
    }
    if node.node_type == NodeType::Canvas {
        let flows = flows(session, node);
        if !flows.is_empty() {
            out["flowStartingPoints"] = Data::List(flows);
        }
        if let Some(device) = raw.field(Field::PrototypeDevice) {
            out["prototypeDevice"] = device_of(device);
        }
        if let Some(color) = raw.field(Field::PrototypeBackgroundColor).and_then(color_of) {
            out["prototypeBackgrounds"] = data!([{ "type": "SOLID", "color": color }]);
        }
    }
    if let Some(start) = raw.field(Field::PrototypeStartingPoint) {
        out["flowStartingPoint"] = data!({
            "name": start.str(Field::Name).unwrap_or_default(),
            "description": start.str(Field::Description).unwrap_or_default(),
        });
    }
    if let Some(direction) = raw.str(Field::ScrollDirection).filter(|_| !raw.is(Field::ScrollDirection, ScrollDirection::None)) {
        out["overflowDirection"] = data!(direction);
    }
    if let Some(behavior) = raw.str(Field::ScrollBehavior).filter(|_| !raw.is(Field::ScrollBehavior, ScrollBehavior::Scrolls)) {
        out["scrollBehavior"] = data!(named(SCROLL_BEHAVIORS, behavior));
    }
    if let Some(position) = raw.str(Field::OverlayPositionType) {
        out["overlayPositionType"] = data!(position);
    }
    if let Some(interaction) = raw.str(Field::OverlayBackgroundInteraction) {
        out["overlayBackgroundInteraction"] = data!(interaction);
    }
    if let Some(background) = raw.field(Field::OverlayBackgroundAppearance) {
        let mut b = data!({ "type": background.str(Field::BackgroundType).unwrap_or(OverlayBackgroundType::None.as_wire()) });
        if let Some(color) = background.field(Field::BackgroundColor).and_then(color_of) {
            b["color"] = color;
        }
        out["overlayBackground"] = b;
    }
}

/// The page's flows, in the order their keys give: each a top frame holding a
/// starting point.
fn flows(session: &Session, page: &SceneNode) -> Vec<Data> {
    let mut starts: Vec<(String, Data)> = page
        .children
        .iter()
        .filter_map(|&id| {
            let start = session.raw(id)?.field(Field::PrototypeStartingPoint)?;
            let name = session.graph.node(id).map(|n| n.name.as_str()).unwrap_or_default();
            Some((
                start.str(Field::Position).unwrap_or_default().to_string(),
                data!({
                    "nodeId": id,
                    "frame": name,
                    "name": start.str(Field::Name).unwrap_or_default(),
                    "description": start.str(Field::Description).unwrap_or_default(),
                }),
            ))
        })
        .collect();
    starts.sort_by(|a, b| a.0.cmp(&b.0));
    starts.into_iter().map(|(_, flow)| flow).collect()
}

/// One stored interaction as a reaction.
fn reaction(graph: &SceneGraph, interaction: &Raw) -> Data {
    let mut out = Map::new();
    out.insert("trigger".into(), interaction.field(Field::Event).map_or(Data::Null, trigger));
    out.insert("actions".into(), Data::List(interaction.array(Field::Actions).iter().map(|a| action(graph, a)).collect()));
    if interaction.bool(Field::IsDeleted) == Some(true) {
        out.insert("deleted".into(), data!(true));
    }
    let rest = rest(interaction, &[Field::Id, Field::Event, Field::Actions, Field::IsDeleted, Field::StateManagementVersion]);
    if !rest.is_empty() {
        out.insert("notModelled".into(), Data::Map(rest));
    }
    Data::Map(out)
}

fn trigger(event: &Raw) -> Data {
    let Some(stored) = event.str(Field::InteractionType) else {
        return if rest(event, &[]).is_empty() { Data::Null } else { data!({ "notModelled": raw_json(event) }) };
    };
    let Some(name) = mapped(TRIGGERS, stored) else {
        return data!({ "notModelled": raw_json(event) });
    };
    let mut out = data!({ "type": name });
    let mut used = vec![Field::InteractionType];
    if event.is(Field::InteractionType, InteractionType::AfterTimeout) {
        out["timeout"] = num(event.f32(Field::TransitionTimeout).unwrap_or(0.0));
        used.push(Field::TransitionTimeout);
    }
    if let Some(key) = event.field(Field::KeyTrigger) {
        out["device"] = data!(key.str(Field::TriggerDevice));
        out["keyCodes"] = Data::List(key.array(Field::KeyCodes).iter().map(raw_json).collect());
        used.push(Field::KeyTrigger);
    }
    let rest = rest(event, &used);
    if !rest.is_empty() {
        out["notModelled"] = Data::Map(rest);
    }
    out
}

fn action(graph: &SceneGraph, action: &Raw) -> Data {
    let mut out = Map::new();
    let mut used = vec![Field::ConnectionType];
    match action.known::<ConnectionType>(Field::ConnectionType) {
        Some(ConnectionType::InternalNode) => {
            out.insert("type".into(), data!("NODE"));
            used.extend([Field::TransitionNodeId, Field::NavigationType]);
            destination(graph, action, &mut out);
            if let Some(navigation) = action.str(Field::NavigationType) {
                out.insert("navigation".into(), data!(named(NAVIGATIONS, navigation)));
            }
            out.insert("transition".into(), transition(action, &mut used));
            for (field, name) in FLAGS {
                if let Some(flag) = action.bool(*field) {
                    out.insert((*name).into(), data!(flag));
                    used.push(*field);
                }
            }
            if let Some(at) = action.field(Field::OverlayRelativePosition) {
                out.insert("overlayRelativePosition".into(), data!({ "x": num(at.f32(Field::X).unwrap_or(0.0)), "y": num(at.f32(Field::Y).unwrap_or(0.0)) }));
                used.push(Field::OverlayRelativePosition);
            }
        }
        Some(ConnectionType::Url) => {
            out.insert("type".into(), data!("URL"));
            out.insert("url".into(), data!(action.str(Field::ConnectionUrl).unwrap_or_default()));
            if let Some(new_tab) = action.bool(Field::OpenUrlInNewTab) {
                out.insert("openInNewTab".into(), data!(new_tab));
            }
            used.extend([Field::ConnectionUrl, Field::OpenUrlInNewTab]);
        }
        Some(kind @ (ConnectionType::Back | ConnectionType::Close)) => {
            out.insert("type".into(), data!(kind.as_wire()));
        }
        // `NONE`, variables, conditions, media, animation, or no kind at all.
        _ => return data!({ "notModelled": raw_json(action) }),
    }
    let rest = rest(action, &used);
    if !rest.is_empty() {
        out.insert("notModelled".into(), Data::Map(rest));
    }
    Data::Map(out)
}

/// The destination's id and name; a stored one that is no node is named as
/// missing, not dropped.
fn destination(graph: &SceneGraph, action: &Raw, out: &mut Map) {
    let Some(id) = action.field(Field::TransitionNodeId).and_then(Guid::read) else {
        out.insert("destinationId".into(), Data::Null);
        return;
    };
    match graph.node(id).filter(|_| id.session_id != UNSET || id.local_id != UNSET) {
        Some(node) => {
            out.insert("destinationId".into(), data!(id));
            out.insert("destination".into(), data!(node.name));
        }
        None => {
            out.insert("destinationId".into(), Data::Null);
            out.insert("missing".into(), data!(id.to_string()));
        }
    }
}

/// `null` for an instant change, as the plugin API gives it.
fn transition(action: &Raw, used: &mut Vec<Field>) -> Data {
    used.extend([Field::TransitionType, Field::TransitionDuration, Field::EasingType, Field::EasingFunction]);
    let Some(stored) = action.str(Field::TransitionType).filter(|_| !action.is(Field::TransitionType, TransitionType::InstantTransition)) else {
        return Data::Null;
    };
    let kind = named(TRANSITIONS, stored);
    let mut out = data!({ "type": kind, "duration": num(action.f32(Field::TransitionDuration).unwrap_or(0.0)) });
    if let Some(easing) = action.str(Field::EasingType) {
        let mut e = data!({ "type": named(EASINGS, easing) });
        if action.is(Field::EasingType, EasingType::CustomCubic) {
            let f: Vec<f32> = action.array(Field::EasingFunction).iter().filter_map(Raw::as_f32).collect();
            if let [x1, y1, x2, y2] = f[..] {
                e["easingFunctionCubicBezier"] = data!({ "x1": num(x1), "y1": num(y1), "x2": num(x2), "y2": num(y2) });
            }
        }
        out["easing"] = e;
    }
    out
}

fn device_of(device: &Raw) -> Data {
    let mut out = data!({ "type": device.str(Field::Type).unwrap_or(PrototypeDeviceType::None.as_wire()) });
    if let Some(size) = device.field(Field::Size) {
        out["size"] = data!({ "width": num(size.f32(Field::X).unwrap_or(0.0)), "height": num(size.f32(Field::Y).unwrap_or(0.0)) });
    }
    if let Some(preset) = device.str(Field::PresetIdentifier) {
        out["presetIdentifier"] = data!(preset);
    }
    if let Some(rotation) = device.str(Field::Rotation) {
        out["rotation"] = data!(rotation);
    }
    out
}

fn color_of(color: &Raw) -> Option<Data> {
    let c = model::Color { r: color.f32(Field::R)?, g: color.f32(Field::G)?, b: color.f32(Field::B)?, a: color.f32(Field::A).unwrap_or(1.0) };
    // The alpha too, when it is not opaque: an overlay's background is
    // usually a shade.
    let alpha = (c.a.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some(data!(if alpha == 255 { hex(c) } else { format!("{}{alpha:02X}", hex(c)) }))
}

/// How many flows and live interactions a file holds.
#[must_use]
pub fn census(session: &Session) -> Data {
    let (mut flows, mut interactions) = (0usize, 0usize);
    for id in session.graph.nodes.keys() {
        let Some(raw) = session.raw(*id) else { continue };
        if raw.has(Field::PrototypeStartingPoint) {
            flows += 1;
        }
        interactions += raw.array(Field::PrototypeInteractions).iter().filter(|i| i.bool(Field::IsDeleted) != Some(true)).count();
    }
    data!({ "flows": flows, "interactions": interactions })
}

fn mapped<E: WireName>(table: &[(E, &'static str)], stored: &str) -> Option<&'static str> {
    table.iter().find(|(file, _)| file.as_wire() == stored).map(|(_, api)| *api)
}

/// The plugin API's name, or the file's where none is known.
fn named<E: WireName>(table: &[(E, &'static str)], stored: &str) -> String {
    mapped(table, stored).map_or_else(|| stored.to_string(), str::to_string)
}

/// The fields of a stored object not in `used`, as stored.
fn rest(raw: &Raw, used: &[Field]) -> Map {
    let Raw::Object(fields) = raw else { return Map::new() };
    fields
        .iter()
        .filter(|(key, _)| Field::from_wire(key).is_none_or(|f| !used.contains(&f)))
        .map(|(key, value)| (key.to_string(), raw_json(value)))
        .collect()
}

// ── Writing ──────────────────────────────────────────────────────────────────

/// The curves the files store for each easing grafta writes, measured in the
/// fixtures (research R4). An easing measured nowhere is not written.
const CURVES: &[(EasingType, &str, [f32; 4])] = &[
    (EasingType::InCubic, "EASE_IN", [0.42, 0.0, 1.0, 1.0]),
    (EasingType::OutCubic, "EASE_OUT", [0.0, 0.0, 0.58, 1.0]),
    (EasingType::InoutCubic, "EASE_IN_AND_OUT", [0.42, 0.0, 0.58, 1.0]),
    (EasingType::Linear, "LINEAR", [0.0, 0.0, 1.0, 1.0]),
    (EasingType::InBackCubic, "EASE_IN_BACK", [0.3, -0.05, 0.7, -0.5]),
    (EasingType::OutBackCubic, "EASE_OUT_BACK", [0.45, 1.45, 0.8, 1.0]),
    (EasingType::GentleSpring, "GENTLE", [1.0, 100.0, 15.0, 0.0]),
    (EasingType::SpringPresetTwo, "BOUNCY", [1.0, 600.0, 15.0, 0.0]),
];

/// Triggers grafta writes. Mouse enter and leave wait for a file saved from
/// Figma (research R4: the files disagree on their names), media for a video.
const WRITTEN_TRIGGERS: &[(&str, InteractionType)] = &[
    ("ON_CLICK", InteractionType::OnClick),
    ("ON_HOVER", InteractionType::OnHover),
    ("ON_PRESS", InteractionType::OnPress),
    ("ON_DRAG", InteractionType::Drag),
    ("AFTER_TIMEOUT", InteractionType::AfterTimeout),
    ("MOUSE_UP", InteractionType::MouseUp),
    ("MOUSE_DOWN", InteractionType::MouseDown),
    ("ON_KEY_DOWN", InteractionType::OnKeyDown),
];

const DIRECTIONAL: [&str; 5] = ["MOVE_IN", "MOVE_OUT", "PUSH", "SLIDE_IN", "SLIDE_OUT"];

const OVERFLOWS: [ScrollDirection; 4] = [ScrollDirection::None, ScrollDirection::Horizontal, ScrollDirection::Vertical, ScrollDirection::Both];

/// What `set_properties` writes for the prototype keys it was given; nothing
/// is written unless every one of them is accepted.
pub fn plan(session: &mut Session, id: Guid, properties: &Map, write: &mut Vec<(Field, Raw)>, remove: &mut Vec<Field>) -> Result<(), ToolError> {
    if let Some(value) = properties.get("reactions") {
        let list = value.as_list().ok_or_else(|| invalid("reactions", "a list of { trigger, actions }", value))?;
        let mut interactions = Vec::new();
        for (i, reaction) in list.iter().enumerate() {
            interactions.push(written_reaction(session, id, &format!("reactions[{i}]"), reaction)?);
        }
        if interactions.is_empty() {
            remove.push(Field::PrototypeInteractions);
        } else {
            write.push((Field::PrototypeInteractions, Raw::Array(interactions)));
        }
    }
    if let Some(value) = properties.get("scrollBehavior") {
        let behavior = SCROLL_BEHAVIORS.iter().find(|(_, api)| value.as_str() == Some(api)).map(|(file, _)| *file);
        let behavior = behavior.ok_or_else(|| invalid("scrollBehavior", "SCROLLS, FIXED or STICKY_SCROLLS", value))?;
        write.push((Field::ScrollBehavior, behavior.into()));
    }
    if let Some(value) = properties.get("overflowDirection") {
        match OVERFLOWS.iter().find(|d| value.as_str() == Some(d.as_wire())) {
            Some(ScrollDirection::None) => remove.push(Field::ScrollDirection),
            Some(direction) => write.push((Field::ScrollDirection, (*direction).into())),
            None => return Err(invalid("overflowDirection", "NONE, HORIZONTAL, VERTICAL or BOTH", value)),
        }
    }
    if let Some(value) = properties.get("overlayPositionType") {
        let position = value.as_str().and_then(OverlayPositionType::from_wire);
        let position = position.ok_or_else(|| invalid("overlayPositionType", "CENTER, TOP_LEFT, TOP_CENTER, TOP_RIGHT, BOTTOM_LEFT, BOTTOM_CENTER, BOTTOM_RIGHT or MANUAL", value))?;
        write.push((Field::OverlayPositionType, position.into()));
    }
    if let Some(value) = properties.get("overlayBackgroundInteraction") {
        let interaction = value.as_str().and_then(OverlayBackgroundInteraction::from_wire);
        let interaction = interaction.ok_or_else(|| invalid("overlayBackgroundInteraction", "NONE or CLOSE_ON_CLICK_OUTSIDE", value))?;
        write.push((Field::OverlayBackgroundInteraction, interaction.into()));
    }
    if let Some(value) = properties.get("overlayBackground") {
        write.push((Field::OverlayBackgroundAppearance, overlay_background(value)?));
    }
    if let Some(value) = properties.get("flowStartingPoint") {
        match value {
            Data::Null => remove.push(Field::PrototypeStartingPoint),
            _ => write.push((Field::PrototypeStartingPoint, starting_point(session, id, value)?)),
        }
    }
    Ok(())
}

fn overlay_background(value: &Data) -> Result<Raw, ToolError> {
    let expected = "{ type: NONE } or { type: SOLID_COLOR, color: \"#RRGGBBAA\" }";
    let map = value.as_map().ok_or_else(|| invalid("overlayBackground", expected, value))?;
    if let Some(key) = map.keys().find(|k| !matches!(k.as_str(), "type" | "color")) {
        return Err(unaccepted("overlayBackground", key, "type, color"));
    }
    match map.get("type").and_then(Data::as_str) {
        Some(t) if t == OverlayBackgroundType::None.as_wire() => Ok(object([(Field::BackgroundType, OverlayBackgroundType::None.into())])),
        Some(t) if t == OverlayBackgroundType::SolidColor.as_wire() => {
            let color = map.get("color").ok_or_else(|| invalid("overlayBackground.color", "a colour, \"#RRGGBBAA\"", "nothing"))?;
            Ok(object([
                (Field::BackgroundType, OverlayBackgroundType::SolidColor.into()),
                (Field::BackgroundColor, super::style::color(super::style::hex("overlayBackground.color", color)?)),
            ]))
        }
        _ => Err(invalid("overlayBackground.type", expected, value)),
    }
}

/// A flow starting at a frame at the top of its page, after the page's other
/// flows.
fn starting_point(session: &Session, id: Guid, value: &Data) -> Result<Raw, ToolError> {
    let map = value.as_map().ok_or_else(|| invalid("flowStartingPoint", "{ name, description? } or null", value))?;
    if let Some(key) = map.keys().find(|k| !matches!(k.as_str(), "name" | "description")) {
        return Err(unaccepted("flowStartingPoint", key, "name, description"));
    }
    let name = map.get("name").and_then(Data::as_str).filter(|n| !n.is_empty());
    let name = name.ok_or_else(|| invalid("flowStartingPoint.name", "the flow's name", value))?;
    let description = map.get("description").map(|d| d.as_str().ok_or_else(|| invalid("flowStartingPoint.description", "text", d))).transpose()?.unwrap_or_default();
    let graph = &session.graph;
    let node = graph.node(id).ok_or(crate::DomainError::NoNodeChange { id })?;
    let page = node.parent_id.and_then(|p| graph.node(p)).filter(|p| p.node_type == NodeType::Canvas);
    let Some(page) = page else {
        return Err(invalid("flowStartingPoint", "a frame at the top of a page", format!("{} ({id}), inside another layer", node.name)));
    };
    let last = page
        .children
        .iter()
        .filter(|&&child| child != id)
        .filter_map(|&child| session.raw(child)?.field(Field::PrototypeStartingPoint)?.str(Field::Position).map(str::to_string))
        .max();
    let position = model::author::position_between(last.as_deref(), None).unwrap_or_else(|| "!".into());
    Ok(object([(Field::Name, Raw::Str(name.into())), (Field::Description, Raw::Str(description.into())), (Field::Position, Raw::Str(position))]))
}

fn written_reaction(session: &mut Session, source: Guid, at: &str, value: &Data) -> Result<Raw, ToolError> {
    let map = value.as_map().ok_or_else(|| invalid(at, "{ trigger, actions }", value))?;
    if let Some(key) = map.keys().find(|k| !matches!(k.as_str(), "trigger" | "actions")) {
        return Err(unaccepted(at, key, "trigger, actions"));
    }
    let trigger = map.get("trigger").ok_or_else(|| invalid(&format!("{at}.trigger"), "{ type, … }", "nothing"))?;
    let event = written_trigger(&format!("{at}.trigger"), trigger)?;
    let list = map.get("actions").and_then(Data::as_list).filter(|a| !a.is_empty());
    let list = list.ok_or_else(|| invalid(&format!("{at}.actions"), "a list of one action or more", map.get("actions").unwrap_or(&Data::Null)))?;
    let mut actions = Vec::new();
    for (i, action) in list.iter().enumerate() {
        actions.push(written_action(session, source, &format!("{at}.actions[{i}]"), action)?);
    }
    let id = session.graft().map_err(|e| invalid(at, "a file grafta can add to", e))?.allocate_id();
    Ok(object([
        (Field::Actions, Raw::Array(actions)),
        (Field::Event, event),
        (Field::Id, id.to_value()),
        (Field::IsDeleted, Raw::Bool(false)),
        (Field::StateManagementVersion, Raw::Int(1)),
    ]))
}

fn written_trigger(at: &str, value: &Data) -> Result<Raw, ToolError> {
    let names = "ON_CLICK, ON_HOVER, ON_PRESS, ON_DRAG, AFTER_TIMEOUT, MOUSE_UP, MOUSE_DOWN or ON_KEY_DOWN";
    let map = value.as_map().ok_or_else(|| invalid(at, format!("{{ type: {names} }}"), value))?;
    let name = map.get("type").and_then(Data::as_str).unwrap_or_default();
    let Some((_, kind)) = WRITTEN_TRIGGERS.iter().find(|(api, _)| *api == name) else {
        let why = if matches!(name, "MOUSE_ENTER" | "MOUSE_LEAVE") { " (mouse enter and leave are read, not written yet)" } else { "" };
        return Err(invalid(&format!("{at}.type"), format!("{names}{why}"), value));
    };
    let takes: &[&str] = match kind {
        InteractionType::AfterTimeout => &["type", "timeout"],
        InteractionType::OnKeyDown => &["type", "keyCodes", "device"],
        _ => &["type"],
    };
    if let Some(key) = map.keys().find(|k| !takes.contains(&k.as_str())) {
        return Err(unaccepted(at, key, if takes.len() == 1 { "type" } else if *kind == InteractionType::AfterTimeout { "type, timeout" } else { "type, keyCodes, device" }));
    }
    let mut event = vec![(Field::InteractionType, Raw::from(*kind))];
    if *kind == InteractionType::AfterTimeout {
        let timeout = map.get("timeout").ok_or_else(|| invalid(&format!("{at}.timeout"), "seconds, from 0 to 10", "nothing"))?;
        event.push((Field::TransitionTimeout, Raw::Float(seconds(&format!("{at}.timeout"), timeout)?)));
    }
    if *kind == InteractionType::OnKeyDown {
        let codes = map.get("keyCodes").and_then(Data::as_list).filter(|c| !c.is_empty());
        let codes = codes.ok_or_else(|| invalid(&format!("{at}.keyCodes"), "a list of key codes, e.g. [13] for Enter", map.get("keyCodes").unwrap_or(&Data::Null)))?;
        let codes = codes
            .iter()
            .map(|c| c.as_i64().and_then(|c| i32::try_from(c).ok()).filter(|c| (0..=255).contains(c)).map(Raw::Int).ok_or_else(|| invalid(&format!("{at}.keyCodes"), "key codes from 0 to 255", c)))
            .collect::<Result<Vec<_>, _>>()?;
        let device = match map.get("device") {
            None => TriggerDevice::Keyboard,
            Some(d) => d.as_str().and_then(TriggerDevice::from_wire).ok_or_else(|| invalid(&format!("{at}.device"), "KEYBOARD, XBOX_ONE, PS4, SWITCH_PRO or UNKNOWN_CONTROLLER", d))?,
        };
        event.push((Field::KeyTrigger, object([(Field::KeyCodes, Raw::Array(codes)), (Field::TriggerDevice, device.into())])));
    }
    Ok(Raw::Object(event.into_iter().map(|(f, v)| (f.as_wire(), v)).collect()))
}

fn written_action(session: &Session, source: Guid, at: &str, value: &Data) -> Result<Raw, ToolError> {
    let map = value.as_map().ok_or_else(|| invalid(at, "{ type: NODE | URL | BACK | CLOSE, … }", value))?;
    let kind = map.get("type").and_then(Data::as_str).unwrap_or_default();
    let takes: &[&str] = match kind {
        "NODE" => &[
            "type", "destinationId", "navigation", "transition", "preserveScrollPosition", "resetScrollPosition", "resetVideoPosition",
            "resetInteractiveComponents", "overlayRelativePosition",
        ],
        "URL" => &["type", "url", "openInNewTab"],
        "BACK" | "CLOSE" => &["type"],
        _ => return Err(invalid(&format!("{at}.type"), "NODE, URL, BACK or CLOSE", value)),
    };
    if let Some(key) = map.keys().find(|k| !takes.contains(&k.as_str())) {
        return Err(unaccepted(at, key, "what the plugin API's action of this type takes"));
    }
    let mut out: Vec<(Field, Raw)> = Vec::new();
    match kind {
        "URL" => {
            let url = map.get("url").and_then(Data::as_str).filter(|u| !u.is_empty());
            let url = url.ok_or_else(|| invalid(&format!("{at}.url"), "a URL", map.get("url").unwrap_or(&Data::Null)))?;
            out.push((Field::ConnectionType, ConnectionType::Url.into()));
            out.push((Field::ConnectionUrl, Raw::Str(url.into())));
            if let Some(tab) = map.get("openInNewTab") {
                out.push((Field::OpenUrlInNewTab, Raw::Bool(tab.as_bool().ok_or_else(|| invalid(&format!("{at}.openInNewTab"), "true or false", tab))?)));
            }
        }
        "BACK" => out.push((Field::ConnectionType, ConnectionType::Back.into())),
        "CLOSE" => out.push((Field::ConnectionType, ConnectionType::Close.into())),
        _ => node_action(session, source, at, map, &mut out)?,
    }
    out.sort_by_key(|(f, _)| f.as_wire());
    Ok(Raw::Object(out.into_iter().map(|(f, v)| (f.as_wire(), v)).collect()))
}

fn node_action(session: &Session, source: Guid, at: &str, map: &Map, out: &mut Vec<(Field, Raw)>) -> Result<(), ToolError> {
    let graph = &session.graph;
    let navigation_name = map.get("navigation").and_then(Data::as_str).unwrap_or_default();
    let navigation = NAVIGATIONS.iter().find(|(_, api)| *api == navigation_name).map(|(file, _)| *file);
    let navigation = navigation.ok_or_else(|| invalid(&format!("{at}.navigation"), "NAVIGATE, OVERLAY, SWAP, SCROLL_TO or CHANGE_TO", map.get("navigation").unwrap_or(&Data::Null)))?;
    let given = map.get("destinationId").unwrap_or(&Data::Null);
    let destination = given.as_str().and_then(|d| d.parse::<Guid>().ok()).and_then(|d| graph.node(d));
    let destination = destination.ok_or_else(|| invalid(&format!("{at}.destinationId"), "the id of a node of this file", given))?;
    let named = |n: &SceneNode| format!("{} ({}, {})", n.name, n.id, crate::view::type_name(n.node_type));
    let source_node = graph.node(source).ok_or(crate::DomainError::NoNodeChange { id: source })?;
    match navigation {
        NavigationType::Navigate | NavigationType::Overlay | NavigationType::Swap => {
            let top = destination.parent_id.and_then(|p| graph.node(p)).is_some_and(|p| p.node_type == NodeType::Canvas);
            let framed = matches!(destination.node_type, NodeType::Frame | NodeType::Component | NodeType::Instance | NodeType::ComponentSet);
            if !top || !framed || page_of(graph, destination.id) != page_of(graph, source) {
                return Err(invalid(&format!("{at}.destinationId"), format!("a frame at the top of the page {} is on, for {navigation_name}", source_node.name), named(destination)));
            }
        }
        NavigationType::ScrollTo => {
            let frame = top_frame(graph, source);
            if frame.is_none() || frame == Some(destination.id) || top_frame(graph, destination.id) != frame {
                return Err(invalid(&format!("{at}.destinationId"), "a layer inside the same top frame as the layer that scrolls to it", named(destination)));
            }
        }
        NavigationType::SwapState => {
            let variant = variant_of(graph, source);
            let sibling = variant.is_some_and(|v| {
                let set = graph.node(v).and_then(|v| v.parent_id);
                destination.node_type == NodeType::Component && destination.id != v && destination.parent_id == set
            });
            if !sibling {
                return Err(invalid(&format!("{at}.destinationId"), "another variant of the set the layer's variant belongs to, for CHANGE_TO", named(destination)));
            }
        }
    }
    out.push((Field::ConnectionType, ConnectionType::InternalNode.into()));
    out.push((Field::NavigationType, navigation.into()));
    out.push((Field::TransitionNodeId, destination.id.to_value()));
    written_transition(&format!("{at}.transition"), map.get("transition").unwrap_or(&Data::Null), navigation, out)?;
    for (field, name) in FLAGS {
        if let Some(flag) = map.get(*name) {
            out.push((*field, Raw::Bool(flag.as_bool().ok_or_else(|| invalid(&format!("{at}.{name}"), "true or false", flag))?)));
        }
    }
    if let Some(position) = map.get("overlayRelativePosition") {
        if !matches!(navigation, NavigationType::Overlay | NavigationType::Swap) {
            return Err(unaccepted(at, "overlayRelativePosition", "it only with OVERLAY or SWAP"));
        }
        let coordinate = |k: &str| position.get(k).and_then(Data::as_f64).map(|v| v as f32).filter(|v| v.is_finite());
        let (Some(x), Some(y)) = (coordinate("x"), coordinate("y")) else {
            return Err(invalid(&format!("{at}.overlayRelativePosition"), "{ x, y }", position));
        };
        out.push((Field::OverlayRelativePosition, object([(Field::X, Raw::Float(x)), (Field::Y, Raw::Float(y))])));
    }
    Ok(())
}

fn written_transition(at: &str, value: &Data, navigation: NavigationType, out: &mut Vec<(Field, Raw)>) -> Result<(), ToolError> {
    let Some(map) = value.as_map() else {
        if value.is_null() {
            out.push((Field::TransitionType, TransitionType::InstantTransition.into()));
            return Ok(());
        }
        return Err(invalid(at, "null (instant) or { type, duration, easing }", value));
    };
    if let Some(key) = map.keys().find(|k| !matches!(k.as_str(), "type" | "duration" | "easing")) {
        return Err(unaccepted(at, key, "type, duration, easing"));
    }
    let name = map.get("type").and_then(Data::as_str).unwrap_or_default();
    let kind = TRANSITIONS.iter().find(|(_, api)| *api == name).map(|(file, _)| *file);
    let Some(kind) = kind.filter(|k| *k != TransitionType::ScrollAnimate || navigation == NavigationType::ScrollTo) else {
        let why = if DIRECTIONAL.contains(&name) { " (directional transitions are read, not written yet)" } else { "" };
        return Err(invalid(&format!("{at}.type"), format!("DISSOLVE, SMART_ANIMATE, or SCROLL_ANIMATE with SCROLL_TO{why}"), value));
    };
    let duration = match map.get("duration") {
        Some(d) => seconds(&format!("{at}.duration"), d)?,
        None => 0.3,
    };
    let easing = map.get("easing").map(|e| e.as_map().ok_or_else(|| invalid(&format!("{at}.easing"), "{ type }", e))).transpose()?;
    let easing_name = easing.and_then(|e| e.get("type")).and_then(Data::as_str).unwrap_or("EASE_OUT");
    let (easing_type, curve) = if easing_name == "CUSTOM_CUBIC_BEZIER" {
        let bezier = easing.and_then(|e| e.get("easingFunctionCubicBezier"));
        let part = |k: &str| bezier.and_then(|b| b.get(k)).and_then(Data::as_f64).map(|v| v as f32).filter(|v| v.is_finite());
        let (Some(x1), Some(y1), Some(x2), Some(y2)) = (part("x1"), part("y1"), part("x2"), part("y2")) else {
            return Err(invalid(&format!("{at}.easing.easingFunctionCubicBezier"), "{ x1, y1, x2, y2 }", bezier.unwrap_or(&Data::Null)));
        };
        (EasingType::CustomCubic, [x1, y1, x2, y2])
    } else {
        let found = CURVES.iter().find(|(_, api, _)| *api == easing_name).map(|(file, _, curve)| (*file, *curve));
        found.ok_or_else(|| {
            invalid(
                &format!("{at}.easing.type"),
                "EASE_IN, EASE_OUT, EASE_IN_AND_OUT, LINEAR, EASE_IN_BACK, EASE_OUT_BACK, GENTLE, BOUNCY or CUSTOM_CUBIC_BEZIER (the others are read, not written yet)",
                easing_name,
            )
        })?
    };
    out.push((Field::TransitionType, kind.into()));
    out.push((Field::TransitionDuration, Raw::Float(duration)));
    out.push((Field::EasingType, easing_type.into()));
    out.push((Field::EasingFunction, Raw::Array(curve.into_iter().map(Raw::Float).collect())));
    Ok(())
}

fn seconds(at: &str, value: &Data) -> Result<f32, ToolError> {
    value.as_f64().map(|v| v as f32).filter(|v| v.is_finite() && (0.0..=10.0).contains(v)).ok_or_else(|| invalid(at, "seconds, from 0 to 10", value))
}

/// The page a node is on.
fn page_of(graph: &SceneGraph, mut id: Guid) -> Option<Guid> {
    loop {
        let node = graph.node(id)?;
        if node.node_type == NodeType::Canvas {
            return Some(id);
        }
        id = node.parent_id?;
    }
}

/// The frame at the top of the page that holds a node, the node itself when it is one.
fn top_frame(graph: &SceneGraph, mut id: Guid) -> Option<Guid> {
    loop {
        let node = graph.node(id)?;
        let parent = graph.node(node.parent_id?)?;
        if parent.node_type == NodeType::Canvas {
            return Some(id);
        }
        id = parent.id;
    }
}

/// The variant a layer is, or is inside: a component whose parent is a set.
fn variant_of(graph: &SceneGraph, mut id: Guid) -> Option<Guid> {
    loop {
        let node = graph.node(id)?;
        let parent = graph.node(node.parent_id?)?;
        if node.node_type == NodeType::Component && parent.node_type == NodeType::ComponentSet {
            return Some(id);
        }
        id = parent.id;
    }
}
