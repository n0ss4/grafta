//! A deck's slide links and transitions, kept in its order.
//!
//! Every slide's transition is the one "next slide" link it holds: a prototype
//! interaction, on click navigate (`INTERNAL_NODE`), whose action carries the
//! transition (`transitionType`, `easingType`, `transitionDuration`, the
//! event's `transitionTimeout`), as the plugin API's `getSlideTransition`
//! reads it. In the decks measured:
//!
//! - each slide's link points to the next slide that is not skipped
//!   (`isSkippedSlide`), across the grid's rows in order (`getSlideGrid`);
//! - the last slide not skipped links to itself when its transition is not
//!   instant, and holds no link when it is (a deck saved with every transition
//!   instant);
//! - a skipped slide links on too, its transition instant.
//!
//! So after a slide is duplicated, moved, deleted or skipped, each slide keeps
//! its own link, transition and all, retargeted; a slide that needs one and has
//! none gets an instant one in the shape the decks hold. Any other interaction
//! a slide has is kept as it is.

use kiwi::Value as Raw;
use model::author::Guid;
use model::NodeType;
use domain::data;
use domain::data::{Data, Map};
use domain::port::JsonText;
use schema::{object, AnimationPhase, AnimationType, ConnectionType, EasingType, Field, InteractionType, NavigationType, TransitionType, Wire, WireMut, WireName};

use app::tools::style::invalid;
use domain::error::SlideError;
use domain::{DomainError, Session, ToolError};

/// Whether an interaction is a "next slide" link to one of `slides`.
fn is_next_link(interaction: &Raw, slides: &[Guid]) -> bool {
    let click = matches!(
        interaction.field(Field::Event).and_then(|e| e.known::<InteractionType>(Field::InteractionType)),
        Some(InteractionType::OnClick | InteractionType::AfterTimeout)
    );
    let Some(Raw::Array(actions)) = interaction.field(Field::Actions) else { return false };
    let [action] = actions.as_slice() else { return false };
    let navigates = action.is(Field::NavigationType, NavigationType::Navigate)
        && action.is(Field::ConnectionType, ConnectionType::InternalNode);
    let target = action.field(Field::TransitionNodeId).and_then(Guid::read);
    click && navigates && target.is_some_and(|t| slides.contains(&t))
}

/// The deck's rows, each its slides in order. `slide_or_row` is any slide,
/// row or the grid.
fn rows(session: &Session, slide_or_row: Guid) -> Vec<Vec<Guid>> {
    let mut at = Some(slide_or_row);
    while let Some(id) = at {
        let Some(node) = session.graph.node(id) else { return Vec::new() };
        if node.node_type == NodeType::SlideGrid {
            return grid_rows(&session.graph, id);
        }
        at = node.parent_id;
    }
    Vec::new()
}

/// A grid's rows, each its slides in order.
pub fn grid_rows(graph: &model::SceneGraph, grid: Guid) -> Vec<Vec<Guid>> {
    graph
        .children(grid)
        .filter(|row| row.node_type == NodeType::SlideRow)
        .map(|row| graph.children(row.id).filter(|n| n.node_type == NodeType::Slide).map(|n| n.id).collect())
        .collect()
}

/// The deck's slides in order: its grid's rows, each row's slides.
pub fn order(session: &Session, slide_or_row: Guid) -> Vec<Guid> {
    rows(session, slide_or_row).concat()
}

/// The space Figma Slides leaves between two slides of a row: the decks
/// measured hold 1920-wide slides at x 0, 2160, 4320…
const GAP: f32 = 240.0;

/// Place each row's slides side by side in their order, as Figma Slides lays
/// them out, so a copy or a moved slide is not left where it was: a
/// presentation exported from Figma follows these places.
fn lay_out(session: &mut Session, rows: &[Vec<Guid>], leaving: Option<Guid>) -> Vec<Guid> {
    let mut moved = Vec::new();
    for row in rows {
        let mut x = 0.0_f32;
        for &slide in row.iter().filter(|s| Some(**s) != leaving) {
            let width = session.graph.node(slide).map_or(1920.0, |n| n.width);
            if let Some(Raw::Object(transform)) = session.fields_mut(slide).and_then(|f| f.field_mut(Field::Transform)) {
                if transform.field(Field::M02).and_then(Raw::as_f32) != Some(x) {
                    transform.set(Field::M02, Raw::Float(x));
                    moved.push(slide);
                }
            }
            x += width + GAP;
        }
    }
    moved
}

fn skipped(session: &Session, slide: Guid) -> bool {
    session.raw(slide).and_then(|r| r.bool(Field::IsSkippedSlide)) == Some(true)
}

/// An id no node or object of the file has, from grafta's allocator.
fn fresh_id(session: &mut Session) -> Option<Guid> {
    session.graft().ok().map(|mut graft| graft.allocate_id())
}

/// An instant link in the shape the decks hold, to `target`, under `id`.
fn instant_link(id: Guid, target: &Raw) -> Raw {
    let action = object([
        (Field::ConnectionType, Raw::from(ConnectionType::InternalNode)),
        (Field::EasingType, Raw::from(EasingType::InCubic)),
        (Field::NavigationType, Raw::from(NavigationType::Navigate)),
        (Field::TransitionDuration, Raw::Float(0.0)),
        (Field::TransitionNodeId, target.clone()),
        (Field::TransitionResetVideoPosition, Raw::Bool(true)),
        (Field::TransitionType, Raw::from(TransitionType::InstantTransition)),
    ]);
    object([
        (Field::Actions, Raw::Array(vec![action])),
        (Field::Event, object([(Field::InteractionType, Raw::from(InteractionType::OnClick)), (Field::TransitionTimeout, Raw::Float(0.0))])),
        (Field::Id, id.to_value()),
        (Field::IsDeleted, Raw::Bool(false)),
        (Field::StateManagementVersion, Raw::Int(1)),
    ])
}

/// Make the deck's links follow its order again, leaving `leaving` (a slide
/// about to be deleted) out. `anchor` is any slide or row of the deck. Returns
/// the slides whose node changes changed.
pub fn relink(session: &mut Session, anchor: Guid, leaving: Option<Guid>) -> Vec<Guid> {
    let every = order(session, anchor);
    let slides: Vec<Guid> = every.iter().copied().filter(|s| Some(*s) != leaving).collect();
    let skips: Vec<bool> = slides.iter().map(|s| skipped(session, *s)).collect();
    let mut seen_ids: Vec<Guid> = Vec::new();
    let mut touched = Vec::new();
    for (i, &slide) in slides.iter().enumerate() {
        // The next slide not skipped; the last not skipped is its own.
        let target = slides.iter().zip(&skips).skip(i + 1).find(|(_, skip)| !**skip).map(|(s, _)| *s);
        let last = target.is_none();
        let target_value = target.unwrap_or(slide).to_value();
        // Its own link, and the rest of its interactions as they are.
        let Some(fields) = session.fields_mut(slide) else { continue };
        let interactions = match fields.field_mut(Field::PrototypeInteractions) {
            Some(Raw::Array(interactions)) => std::mem::take(interactions),
            _ => Vec::new(),
        };
        let mut link = None;
        let mut rest = Vec::new();
        for interaction in interactions {
            if is_next_link(&interaction, &every) {
                if link.is_none() {
                    link = Some(interaction);
                }
            } else {
                rest.push(interaction);
            }
        }
        // A copy holds its original's link under the same id: give it a new one.
        let shared = link.as_ref().and_then(|l| l.field(Field::Id)).and_then(Guid::read).is_some_and(|id| seen_ids.contains(&id));
        if shared {
            if let (Some(fresh), Some(Raw::Object(l))) = (fresh_id(session), link.as_mut()) {
                l.set(Field::Id, fresh.to_value());
            }
        }
        if let Some(id) = link.as_ref().and_then(|l| l.field(Field::Id)).and_then(Guid::read) {
            seen_ids.push(id);
        }
        // The last slide holds its link, to itself, only for a transition of its
        // own: an instant one is no link at all (a deck saved all instant; one
        // saved with transitions kept its last slide's Push).
        let instant = |l: &Raw| l.field(Field::Actions).and_then(Raw::as_array).and_then(|a| a.first()).is_some_and(|a| a.is(Field::TransitionType, TransitionType::InstantTransition));
        let link = if last && link.as_ref().is_some_and(instant) { None } else { link };
        let link = match link {
            Some(mut link) => {
                if let Raw::Object(l) = &mut link {
                    if let Some(Raw::Array(actions)) = l.field_mut(Field::Actions) {
                        if let Some(Raw::Object(action)) = actions.first_mut() {
                            action.set(Field::TransitionNodeId, target_value.clone());
                            // A skipped slide's transition is instant, as Figma
                            // writes it (the x3 deck).
                            if skips[i] {
                                action.set(Field::TransitionType, Raw::from(TransitionType::InstantTransition));
                                action.set(Field::TransitionDuration, Raw::Float(0.0));
                            }
                        }
                    }
                }
                Some(link)
            }
            // The last slide with no transition holds no link.
            None if last => None,
            None => fresh_id(session).map(|id| instant_link(id, &target_value)),
        };
        let Some(fields) = session.fields_mut(slide) else { continue };
        let mut interactions = rest;
        interactions.extend(link);
        if interactions.is_empty() {
            fields.take(Field::PrototypeInteractions);
        } else {
            fields.set(Field::PrototypeInteractions, Raw::Array(interactions));
        }
        touched.push(slide);
    }
    for slide in lay_out(session, &rows(session, anchor), leaving) {
        if !touched.contains(&slide) {
            touched.push(slide);
        }
    }
    session.refresh(&touched);
    touched
}

/// A duplicated slide's object animations, made its own: each names the copy
/// of the layer it named, under a new id.
pub fn own_animations(session: &mut Session, copy: Guid, pairs: &[(Guid, Guid)]) {
    let map: std::collections::HashMap<Guid, Guid> = pairs.iter().copied().collect();
    let Some(Raw::Array(animations)) = session.fields_mut(copy).and_then(|f| f.field(Field::ObjectAnimations)).cloned() else { return };
    let mut renewed = Vec::with_capacity(animations.len());
    for mut animation in animations {
        let fresh = fresh_id(session);
        if let Raw::Object(fields) = &mut animation {
            if let Some(Raw::Array(actions)) = fields.field_mut(Field::Actions) {
                for action in actions.iter_mut() {
                    if let Raw::Object(action) = action {
                        if let Some(to) = action.field(Field::TransitionNodeId).and_then(Guid::read).and_then(|from| map.get(&from)) {
                            action.set(Field::TransitionNodeId, to.to_value());
                        }
                    }
                }
            }
            if let Some(fresh) = fresh {
                fields.set(Field::Id, fresh.to_value());
            }
        }
        renewed.push(animation);
    }
    if let Some(fields) = session.fields_mut(copy) {
        fields.set(Field::ObjectAnimations, Raw::Array(renewed));
    }
    session.refresh(&[copy]);
}

/// Speaker notes as the deck holds them: a rich-text editor's JSON (Lexical's),
/// one paragraph per line of `text`.
pub fn notes_json(json: &dyn JsonText, text: &str) -> String {
    let paragraphs: Vec<Data> = text
        .split('\n')
        .map(|line| {
            let children: Vec<Data> = if line.is_empty() {
                Vec::new()
            } else {
                vec![data!({ "detail": 0, "format": 0, "mode": "normal", "style": "", "text": line, "type": "text", "version": 1 })]
            };
            data!({ "children": children, "direction": null, "format": "", "indent": 0, "type": "paragraph", "version": 1, "textFormat": 0, "textStyle": "" })
        })
        .collect();
    json.write(&data!({ "root": { "children": paragraphs, "direction": null, "format": "", "indent": 0, "type": "root", "version": 1 } }))
}

/// The text of speaker notes: each paragraph's text nodes, a line each.
pub fn notes_text(json: &dyn JsonText, text: &str) -> Option<String> {
    let value = json.parse(text).ok()?;
    let paragraphs = value.get("root")?.get("children")?.as_list()?;
    Some(
        paragraphs
            .iter()
            .map(|p| p.get("children").and_then(Data::as_list).into_iter().flatten().filter_map(|t| t.get("text").and_then(Data::as_str)).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// What `set_properties` takes on a slide besides a frame's properties.
pub const KEYS: [&str; 4] = ["isSkippedSlide", "speakerNotes", "transition", "objectAnimations"];

/// The plugin API's name for no transition, which the format calls
/// `INSTANT_TRANSITION`.
pub const NO_TRANSITION: &str = "NONE";

/// The plugin API's transition styles (`setSlideTransition`) besides `NONE`,
/// each the format's `TransitionType` of the same name.
const STYLES: [TransitionType; 22] = [
    TransitionType::Dissolve, TransitionType::SlideFromLeft, TransitionType::SlideFromRight, TransitionType::SlideFromTop,
    TransitionType::SlideFromBottom, TransitionType::PushFromLeft, TransitionType::PushFromRight, TransitionType::PushFromTop,
    TransitionType::PushFromBottom, TransitionType::MoveFromLeft, TransitionType::MoveFromRight, TransitionType::MoveFromTop,
    TransitionType::MoveFromBottom, TransitionType::SlideOutToLeft, TransitionType::SlideOutToRight, TransitionType::SlideOutToTop,
    TransitionType::SlideOutToBottom, TransitionType::MoveOutToLeft, TransitionType::MoveOutToRight, TransitionType::MoveOutToTop,
    TransitionType::MoveOutToBottom, TransitionType::SmartAnimate,
];

/// The plugin API's curves and the format's `EasingType` each is.
const CURVES: [(&str, EasingType); 8] = [
    ("EASE_IN", EasingType::InCubic),
    ("EASE_OUT", EasingType::OutCubic),
    ("EASE_IN_AND_OUT", EasingType::InoutCubic),
    ("LINEAR", EasingType::Linear),
    ("EASE_IN_BACK", EasingType::InBackCubic),
    ("EASE_OUT_BACK", EasingType::OutBackCubic),
    ("EASE_IN_AND_OUT_BACK", EasingType::InoutBackCubic),
    ("GENTLE", EasingType::GentleSpring),
];

/// The plugin API's timings, and its name for what the format calls `AFTER_TIMEOUT`.
pub const AFTER_DELAY: &str = "AFTER_DELAY";

/// The animations an object takes, and their phases.
const ANIMATIONS: [AnimationType; 5] =
    [AnimationType::Fade, AnimationType::SlideFromLeft, AnimationType::SlideFromRight, AnimationType::SlideFromTop, AnimationType::SlideFromBottom];
const PHASES: [AnimationPhase; 2] = [AnimationPhase::In, AnimationPhase::Out];

/// A file's easing in the plugin API's name, or as the file has it.
pub fn curve_name(easing: &str) -> String {
    CURVES.iter().find(|(_, e)| e.as_wire() == easing).map_or(easing, |(c, _)| c).to_string()
}

/// `got` at `at`, which is none of `allowed`.
fn not_one_of(at: &str, got: impl std::fmt::Display, allowed: String) -> ToolError {
    DomainError::NotOneOf { at: at.into(), got: got.to_string(), allowed }.into()
}

/// A number in a range, for `key`.
fn number(value: &Data, key: &str, low: f64, high: f64) -> Result<f64, ToolError> {
    match value.as_f64() {
        Some(n) if (low..=high).contains(&n) => Ok(n),
        _ => Err(invalid(key, format!("a number from {low} to {high}"), value)),
    }
}

/// A transition as the plugin API's `setSlideTransition` takes it, validated:
/// the schema's type, easing, duration, and the delay when it follows one.
struct Transition {
    kind: TransitionType,
    easing: Option<EasingType>,
    duration: Option<f64>,
    delay: Option<f64>,
}

fn transition(value: &Data) -> Result<Transition, ToolError> {
    let Some(object) = value.as_map() else {
        return Err(invalid("transition", "{ style, duration, curve, timing }", value));
    };
    if let Some(unknown) = object.keys().find(|k| !["style", "duration", "curve", "timing"].contains(&k.as_str())) {
        return Err(not_one_of("transition", unknown, "style, duration, curve, timing".into()));
    }
    let style = object.get("style").and_then(Data::as_str).ok_or(SlideError::NoStyle)?;
    let kind = if style == NO_TRANSITION {
        TransitionType::InstantTransition
    } else {
        TransitionType::listed(style, &STYLES).ok_or_else(|| {
            not_one_of("transition.style", style, format!("{NO_TRANSITION}, {}", TransitionType::joined(&STYLES, ", ")))
        })?
    };
    let easing = match object.get("curve") {
        None => None,
        Some(curve) => {
            let name = curve.as_str().unwrap_or_default();
            Some(CURVES.iter().find(|(c, _)| *c == name).map(|(_, e)| *e).ok_or_else(|| {
                not_one_of("transition.curve", curve, CURVES.iter().map(|(c, _)| *c).collect::<Vec<_>>().join(", "))
            })?)
        }
    };
    let duration = object.get("duration").map(|d| number(d, "transition.duration", 0.01, 10.0)).transpose()?;
    let delay = match object.get("timing") {
        None => None,
        Some(timing) => match timing.get("type").and_then(Data::as_str) {
            Some(on_click) if on_click == InteractionType::OnClick.as_wire() => Some(0.0),
            Some(AFTER_DELAY) => Some(number(timing.get("delay").unwrap_or(&Data::Null), "transition.timing.delay", 0.0, 30.0)?),
            _ => {
                return Err(invalid(
                    "transition.timing",
                    "{ \"type\": \"ON_CLICK\" } or { \"type\": \"AFTER_DELAY\", \"delay\": seconds }",
                    timing,
                ))
            }
        },
    };
    Ok(Transition { kind, easing, duration, delay })
}

/// Object animations as `set_properties` takes them, validated: each a layer
/// inside `slide`, a type, a phase and a duration.
fn animations(session: &Session, slide: Guid, value: &Data) -> Result<Vec<(Guid, AnimationType, AnimationPhase, f64)>, ToolError> {
    let Some(list) = value.as_list() else { return Err(invalid("objectAnimations", "a list", value)) };
    list.iter()
        .enumerate()
        .map(|(i, a)| {
            let key = format!("objectAnimations[{i}]");
            if let Some(unknown) = a.as_map().into_iter().flat_map(|o| o.keys()).find(|k| !["node", "type", "phase", "duration"].contains(&k.as_str())) {
                return Err(not_one_of(&key, unknown, "node, type, phase, duration".into()));
            }
            let node = a.get("node").and_then(Data::as_str).ok_or_else(|| SlideError::NoLayer { key: key.clone() })?;
            let layer = node.parse().ok().and_then(|id| session.graph.node(id)).map(|n| n.id);
            let mut at = layer.and_then(|id| session.graph.node(id)).and_then(|n| n.parent_id);
            while at.is_some_and(|p| p != slide) {
                at = at.and_then(|p| session.graph.node(p).and_then(|n| n.parent_id));
            }
            let (Some(layer), Some(_)) = (layer, at) else {
                return Err(SlideError::NotOnSlide { key, node: node.into(), slide }.into());
            };
            let kind = a.get("type").and_then(Data::as_str).unwrap_or(AnimationType::Fade.as_wire());
            let kind = AnimationType::listed(kind, &ANIMATIONS)
                .ok_or_else(|| not_one_of(&format!("{key}.type"), kind, AnimationType::joined(&ANIMATIONS, ", ")))?;
            let phase = a.get("phase").and_then(Data::as_str).unwrap_or(AnimationPhase::In.as_wire());
            let phase = AnimationPhase::listed(phase, &PHASES)
                .ok_or_else(|| SlideError::NotInOrOut { key: key.clone(), phase: phase.into() })?;
            let duration = a.get("duration").map(|d| number(d, &format!("{key}.duration"), 0.01, 10.0)).transpose()?.unwrap_or(0.6);
            Ok((layer, kind, phase, duration))
        })
        .collect()
}

/// An object animation in the shape the decks hold: on click, Figma's own
/// default curve.
fn animation(id: Guid, node: &Raw, kind: AnimationType, phase: AnimationPhase, duration: f64) -> Raw {
    let action = object([
        (Field::AnimationPhase, phase.into()),
        (Field::AnimationType, kind.into()),
        (Field::ConnectionType, Raw::from(ConnectionType::ObjectAnimation)),
        (Field::EasingFunction, Raw::Array([0.17, 0.0, 0.2, 1.0].into_iter().map(Raw::Float).collect())),
        (Field::EasingType, Raw::from(EasingType::CustomCubic)),
        (Field::TransitionDuration, Raw::Float(duration as f32)),
        (Field::TransitionNodeId, node.clone()),
    ]);
    object([
        (Field::Actions, Raw::Array(vec![action])),
        (Field::Event, object([(Field::InteractionType, Raw::from(InteractionType::OnClick))])),
        (Field::Id, id.to_value()),
        (Field::IsDeleted, Raw::Bool(false)),
        (Field::StateManagementVersion, Raw::Int(1)),
    ])
}

/// A slide's own fields (`KEYS`), every value validated before any is written;
/// the links follow a skip or a transition. Returns the keys set.
pub fn set(session: &mut Session, id: Guid, properties: &Map) -> Result<Vec<String>, ToolError> {
    let skip = match properties.get("isSkippedSlide") {
        None => None,
        Some(Data::Bool(b)) => Some(*b),
        Some(other) => return Err(invalid("isSkippedSlide", "true or false", other)),
    };
    let notes = match properties.get("speakerNotes") {
        None => None,
        Some(Data::String(s)) => Some(s.clone()),
        Some(other) => return Err(invalid("speakerNotes", "text", other)),
    };
    let transition = properties.get("transition").map(transition).transpose()?;
    let animations = properties.get("objectAnimations").map(|a| animations(session, id, a)).transpose()?;

    let every = order(session, id);
    let link_id = transition.as_ref().and_then(|_| fresh_id(session));
    let fresh: Vec<Guid> = (0..animations.as_ref().map_or(0, Vec::len)).filter_map(|_| fresh_id(session)).collect();
    let self_value = id.to_value();
    let written_notes = notes.as_deref().filter(|n| !n.is_empty()).map(|n| notes_json(session.ports.json.as_ref(), n));
    let fields = session.fields_mut(id).ok_or(DomainError::NotInFile { id })?;
    let mut set = Vec::new();
    if let Some(skip) = skip {
        if skip {
            fields.set(Field::IsSkippedSlide, Raw::Bool(true));
        } else {
            fields.take(Field::IsSkippedSlide);
        }
        set.push("isSkippedSlide".to_string());
    }
    if notes.is_some() {
        match written_notes {
            None => {
                fields.take(Field::SlideSpeakerNotes);
            }
            Some(written) => {
                fields.set(Field::SlideSpeakerNotes, Raw::Str(written));
            }
        }
        set.push("speakerNotes".to_string());
    }
    if let Some(t) = &transition {
        // Its link, or a new one, to itself until relinking aims it.
        let mut interactions = match fields.field_mut(Field::PrototypeInteractions) {
            Some(Raw::Array(interactions)) => std::mem::take(interactions),
            _ => Vec::new(),
        };
        let at = match (interactions.iter().position(|i| is_next_link(i, &every)), link_id) {
            (Some(at), _) => Some(at),
            (None, Some(new)) => {
                interactions.push(instant_link(new, &self_value));
                Some(interactions.len() - 1)
            }
            (None, None) => None,
        };
        if let Some(Raw::Object(link)) = at.and_then(|at| interactions.get_mut(at)) {
            if let Some(Raw::Array(actions)) = link.field_mut(Field::Actions) {
                if let Some(Raw::Object(action)) = actions.first_mut() {
                    action.set(Field::TransitionType, t.kind.into());
                    if let Some(easing) = t.easing {
                        action.set(Field::EasingType, easing.into());
                    }
                    let had = action.field(Field::TransitionDuration).and_then(Raw::as_f32).unwrap_or(0.0);
                    let duration = match (t.kind, t.duration) {
                        (TransitionType::InstantTransition, _) => 0.0,
                        (_, Some(d)) => d as f32,
                        (_, None) if had > 0.0 => had,
                        // The duration Figma Slides gave the decks' transitions.
                        (_, None) => 0.3,
                    };
                    action.set(Field::TransitionDuration, Raw::Float(duration));
                }
            }
            if let (Some(delay), Some(Raw::Object(event))) = (t.delay, link.field_mut(Field::Event)) {
                event.set(Field::InteractionType, if delay > 0.0 { InteractionType::AfterTimeout } else { InteractionType::OnClick }.into());
                event.set(Field::TransitionTimeout, Raw::Float(delay as f32));
            }
        }
        fields.set(Field::PrototypeInteractions, Raw::Array(interactions));
        set.push("transition".to_string());
    }
    if let Some(list) = &animations {
        let entries: Vec<Raw> = list
            .iter()
            .zip(fresh.iter().copied())
            .map(|((node, kind, phase, duration), fresh)| animation(fresh, &node.to_value(), *kind, *phase, *duration))
            .collect();
        if entries.is_empty() {
            fields.take(Field::ObjectAnimations);
        } else {
            fields.set(Field::ObjectAnimations, Raw::Array(entries));
        }
        set.push("objectAnimations".to_string());
    }
    session.refresh(&[id]);
    if skip.is_some() || transition.is_some() {
        relink(session, id, None);
    }
    Ok(set)
}
