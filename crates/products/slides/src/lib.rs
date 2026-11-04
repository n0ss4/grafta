#![deny(clippy::all)]

//! Figma Slides: a deck of slides in rows.

use kiwi::Value as Raw;
use model::{NodeType, SceneGraph, SceneNode};
use schema::{Field, Guid, InteractionType, NavigationType, TransitionType, Wire, WireName};

use domain::product::{Applied, NodeBehavior, ProductProfile};
use domain::port::JsonText;
use domain::data;
use domain::data::{Data, Map};
use domain::error::WriteError;
use domain::event::NodeEvent;
use app::tools::style::ACCEPTED;
use app::tools::write::Placement;
use domain::{Session, ToolError};
use app::view::num;
use deck::{curve_name, notes_text, AFTER_DELAY, NO_TRANSITION};

pub mod deck;

pub struct Slides;

impl ProductProfile for Slides {
    fn product(&self) -> figfile::Product {
        figfile::Product::Slides
    }
    fn name(&self) -> &'static str {
        "Figma Slides"
    }
    fn key(&self) -> &'static str {
        "slides"
    }
    fn summary(&self, graph: &SceneGraph) -> Option<(&'static str, Data)> {
        Some(("slides", data!(graph.nodes.values().filter(|n| n.node_type == NodeType::Slide).count())))
    }
    fn behaviours(&self) -> &'static [&'static dyn NodeBehavior] {
        &[&Slide, &Row, &InteractiveElement]
    }
    /// A deck's "next slide" links follow its slides: a slide moved
    /// relinks the rows it left and joined; one about to be deleted is let go
    /// first, so its deck's links do not keep it; a copy animates its own
    /// layers and joins the links in order.
    fn on(&self, session: &mut Session, event: &NodeEvent) -> Result<(), ToolError> {
        let slide = |session: &Session, id: Guid| session.graph.node(id).filter(|n| n.node_type == NodeType::Slide).map(|n| n.parent_id);
        match *event {
            NodeEvent::Moved { node, from, to } if slide(session, node).is_some() => {
                for row in [from, Some(to)].into_iter().flatten().collect::<std::collections::BTreeSet<_>>() {
                    deck::relink(session, row, None);
                }
            }
            NodeEvent::Deleted { node, parent: Some(row) } if slide(session, node).is_some() => {
                deck::relink(session, row, Some(node));
            }
            NodeEvent::Duplicated { copy, pairs, .. } => {
                if let Some(Some(row)) = slide(session, copy) {
                    deck::own_animations(session, copy, pairs);
                    deck::relink(session, row, None);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// A slide: a frame, with a deck's transition, skip, notes and animations.
struct Slide;

impl NodeBehavior for Slide {
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::Slide]
    }
    fn decoded(&self) -> &'static [Field] {
        &[Field::PrototypeInteractions, Field::ObjectAnimations, Field::IsSkippedSlide, Field::SlideSpeakerNotes]
    }
    /// A slide's transition, skip, notes and animations, its transition
    /// in the plugin API's `getSlideTransition` names.
    fn shows(&self, _node: &SceneNode, raw: &Raw, json: &dyn JsonText) -> Option<Map> {
        let link = raw.field(Field::PrototypeInteractions).and_then(Raw::as_array).into_iter().flatten().find(|p| {
            p.field(Field::Actions).and_then(Raw::as_array).and_then(|a| a.first()).is_some_and(|a| a.is(Field::NavigationType, NavigationType::Navigate))
        });
        let transition = link.map(|link| {
            let action = link.field(Field::Actions).and_then(Raw::as_array).and_then(|a| a.first());
            let get = |k: Field| action.and_then(|a| a.field(k));
            let style = get(Field::TransitionType).and_then(Raw::as_str).unwrap_or(TransitionType::InstantTransition.as_wire());
            let style = if style == TransitionType::InstantTransition.as_wire() { NO_TRANSITION } else { style };
            let delay = link.field(Field::Event).and_then(|e| e.field(Field::TransitionTimeout)).and_then(Raw::as_f32).unwrap_or(0.0);
            let timing = if link.field(Field::Event).is_some_and(|e| e.is(Field::InteractionType, InteractionType::AfterTimeout)) || delay > 0.0 {
                data!({ "type": AFTER_DELAY, "delay": num(delay) })
            } else {
                data!({ "type": InteractionType::OnClick.as_wire() })
            };
            data!({ "style": style, "duration": num(get(Field::TransitionDuration).and_then(Raw::as_f32).unwrap_or(0.0)), "curve": get(Field::EasingType).and_then(Raw::as_str).map(curve_name), "timing": timing })
        });
        let animations: Vec<Data> = raw
            .field(Field::ObjectAnimations)
            .and_then(Raw::as_array)
            .into_iter()
            .flatten()
            .flat_map(|a| a.field(Field::Actions).and_then(Raw::as_array).cloned().unwrap_or_default())
            .map(|action| {
                data!({
                    "node": action.field(Field::TransitionNodeId).and_then(Guid::read).map(|id| id.to_string()),
                    "type": action.field(Field::AnimationType).and_then(Raw::as_str),
                    "phase": action.field(Field::AnimationPhase).and_then(Raw::as_str),
                    "duration": num(action.field(Field::TransitionDuration).and_then(Raw::as_f32).unwrap_or(0.0)),
                })
            })
            .collect();
        let mut out = Map::new();
        out.insert("transition".into(), transition.unwrap_or_else(|| data!({ "style": NO_TRANSITION, "duration": 0, "timing": { "type": InteractionType::OnClick.as_wire() } })));
        if raw.field(Field::IsSkippedSlide).and_then(Raw::as_bool) == Some(true) {
            out.insert("isSkippedSlide".into(), data!(true));
        }
        if let Some(notes) = raw.field(Field::SlideSpeakerNotes).and_then(Raw::as_str).and_then(|text| notes_text(json, text)) {
            out.insert("speakerNotes".into(), data!(notes));
        }
        if !animations.is_empty() {
            out.insert("objectAnimations".into(), data!(animations));
        }
        Some(out)
    }
    /// A slide holds what a frame holds.
    fn admits(&self, _parent: Guid, _placement: Placement) -> Option<Result<(), ToolError>> {
        Some(Ok(()))
    }
    fn movable(&self) -> bool {
        true
    }
    /// A slide moves among the rows of its deck alone.
    fn moves_into(&self, id: Guid, parent: &str, parent_type: Option<NodeType>) -> Option<Result<Placement, ToolError>> {
        Some(if parent_type == Some(NodeType::SlideRow) {
            Ok(Placement::MovedSlide)
        } else {
            Err(WriteError::SlideOutsideRow { id, parent: parent.into() }.into())
        })
    }
    /// A slide's copy is the next slide, not the last of the deck.
    fn copies_beside_itself(&self) -> bool {
        true
    }
    /// A slide's deck fields; the rest are a frame's.
    fn apply(&self, session: &mut Session, id: Guid, properties: &mut Map) -> Option<Result<Applied, ToolError>> {
        let own: Map = deck::KEYS.iter().filter_map(|k| properties.remove(*k).map(|v| (k.to_string(), v))).collect();
        if own.is_empty() {
            return None;
        }
        Some(deck::set(session, id, &own).map(|set| Applied { set, note: None, whole: false }))
    }
    fn base_keys(&self) -> Option<&'static [&'static str]> {
        Some(&ACCEPTED)
    }
}

/// A row of a deck's grid, holding its slides.
struct Row;

impl NodeBehavior for Row {
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::SlideRow]
    }
    /// A row holds slides alone: a slide is duplicated or moved into one.
    fn admits(&self, parent: Guid, placement: Placement) -> Option<Result<(), ToolError>> {
        Some(if matches!(placement, Placement::Duplicate | Placement::MovedSlide) {
            Ok(())
        } else {
            Err(WriteError::NotOnASlide { what: placement.noun(), parent }.into())
        })
    }
    /// Figma Slides places a row's slides itself, by their order.
    fn lays_out_children(&self) -> bool {
        false
    }
}

/// An interactive slide element: a poll, a rating, an embed.
struct InteractiveElement;

impl NodeBehavior for InteractiveElement {
    fn types(&self) -> &'static [NodeType] {
        &[NodeType::InteractiveSlideElement]
    }
    fn decoded(&self) -> &'static [Field] {
        &[Field::FlappType]
    }
    fn shows(&self, _node: &SceneNode, raw: &Raw, _json: &dyn JsonText) -> Option<Map> {
        let mut out = Map::new();
        out.insert("interactiveSlideElementType".into(), data!(raw.field(Field::FlappType).and_then(Raw::as_str)));
        Some(out)
    }
}
