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
    ConnectionType, EasingType, Field, Guid, InteractionType, NavigationType, OverlayBackgroundType, PrototypeDeviceType, ScrollBehavior,
    ScrollDirection, TransitionType, Wire, WireName,
};

use crate::data::{data, Data, Map};
use crate::view::{hex, num, raw_json};
use crate::Session;

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

/// The springs are given by the file's name for the same reason.
const EASINGS: &[(EasingType, &str)] = &[
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
    Some(data!(hex(c)))
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
