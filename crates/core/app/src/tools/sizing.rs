//! Sizing and placement in the plugin API's names: fill, hug
//! and fixed sizing, minimum and maximum sizes, constraints and rotation.
//!
//! Constraints are `horizontalConstraint` and `verticalConstraint` (`MIN`, the
//! default, never stored); minimum and maximum sizes one `minSize` and one
//! `maxSize` pair, 0 and infinity meaning none; rotation the transform. Figma
//! honoured each as written (a spike in Figma), and applies constraints only when a
//! frame is resized in its editor, storing the result: so grafta applies them
//! when it resizes one.

use kiwi::Value as Raw;
use model::{LayoutMode, NodeType, SceneNode};
use crate::data::{Data, Map};
use schema::{object, ConstraintType, Field, Guid, StackCounterAlign, StackMode, StackSize, Wire, WireMut, WireName};

use super::style::{AUTO, INHERIT};
use super::style::invalid;
use crate::error::SizingError;
use crate::{DomainError, Session, ToolError};

/// The plugin API's sizings with no counterpart in the format.
pub const HUG: &str = "HUG";
pub const FILL: &str = "FILL";

/// The constraints a layer takes.
const CONSTRAINTS: [ConstraintType; 5] =
    [ConstraintType::Min, ConstraintType::Center, ConstraintType::Max, ConstraintType::Stretch, ConstraintType::Scale];


/// Fields to write and fields to remove.
type Writes = (Vec<(Field, Raw)>, Vec<Field>);

/// A child as its constraints need it: id, x, y, width, height, horizontal
/// and vertical constraint, rotation.
type Placed = (Guid, f32, f32, f32, f32, String, String, f32);

fn raw_pair(x: f32, y: f32) -> Raw {
    object([(Field::Value,
        object([(Field::X, Raw::Float(x)), (Field::Y, Raw::Float(y))]),
    )])
}

fn pair_of(raw: Option<&Raw>) -> Option<(f32, f32)> {
    let v = raw?.field(Field::Value)?;
    Some((v.field(Field::X)?.as_f32()?, v.field(Field::Y)?.as_f32()?))
}

/// Rewrite `layoutSizingHorizontal` and `layoutSizingVertical` onto the
/// auto-layout keys grafta already writes, and check each is possible:
/// `FILL` needs an auto-layout parent, `HUG` an auto-layout frame (or a text).
pub fn translate_sizing(session: &Session, node: &SceneNode, properties: &Map) -> Result<Map, ToolError> {
    let mut out = properties.clone();
    let parent = node.parent_id.and_then(|p| session.graph.node(p));
    let parent_mode = parent.map_or(LayoutMode::None, |p| p.layout.mode);
    let own_mode = match properties.get("layoutMode").and_then(Data::as_str) {
        Some(mode) => match StackMode::from_wire(mode) {
            Some(StackMode::Horizontal) => LayoutMode::Horizontal,
            Some(StackMode::Vertical) => LayoutMode::Vertical,
            _ => LayoutMode::None,
        },
        None => node.layout.mode,
    };
    let fixed = StackSize::Fixed.as_wire();
    for (key, horizontal) in [("layoutSizingHorizontal", true), ("layoutSizingVertical", false)] {
        let Some(value) = properties.get(key) else { continue };
        out.remove(key);
        let sizing =
            value.as_str().filter(|s| [fixed, HUG, FILL].contains(s)).ok_or_else(|| invalid(key, "FIXED, HUG or FILL", value))?;
        let along_parent = parent_mode != LayoutMode::None && (parent_mode == LayoutMode::Horizontal) == horizontal;
        let across_parent = parent_mode != LayoutMode::None && !along_parent;
        let along_own = own_mode != LayoutMode::None && (own_mode == LayoutMode::Horizontal) == horizontal;
        let own_axis = if along_own { "primaryAxisSizingMode" } else { "counterAxisSizingMode" };
        match sizing {
            FILL => {
                if parent_mode == LayoutMode::None {
                    return Err(SizingError::FillWithoutLayout {
                        key: key.into(),
                        parent: node.parent_id.map_or_else(|| "its parent".to_string(), |p| p.to_string()),
                    }
                    .into());
                }
                if along_parent {
                    out.insert("layoutGrow".into(), Data::from(1_u64));
                } else {
                    out.insert("layoutAlign".into(), Data::from(StackCounterAlign::Stretch.as_wire()));
                }
                if own_mode != LayoutMode::None {
                    out.insert(own_axis.into(), Data::from(fixed));
                }
            }
            HUG => {
                if node.node_type == NodeType::Text {
                    return Err(SizingError::HugText { key: key.into() }.into());
                }
                if own_mode == LayoutMode::None {
                    return Err(SizingError::HugWithoutLayout { key: key.into(), id: node.id }.into());
                }
                out.insert(own_axis.into(), Data::from(AUTO));
                if along_parent {
                    out.insert("layoutGrow".into(), Data::from(0_u64));
                } else if across_parent {
                    out.insert("layoutAlign".into(), Data::from(INHERIT));
                }
            }
            _ => {
                if along_parent {
                    out.insert("layoutGrow".into(), Data::from(0_u64));
                } else if across_parent {
                    out.insert("layoutAlign".into(), Data::from(INHERIT));
                }
                if own_mode != LayoutMode::None {
                    out.insert(own_axis.into(), Data::from(fixed));
                }
            }
        }
    }
    Ok(out)
}

/// The fields for minimum and maximum sizes, constraints and rotation.
/// `transform` is the transform the call writes otherwise (a move), or the
/// node's own.
pub fn fields(
    raw: &Raw,
    properties: &Map,
    transform: Option<Raw>,
) -> Result<Writes, ToolError> {
    let (mut write, mut remove) = (Vec::new(), Vec::new());
    let limit = |key: &str| -> Result<Option<Option<f32>>, ToolError> {
        match properties.get(key) {
            None => Ok(None),
            Some(Data::Null) => Ok(Some(None)),
            Some(v) => v
                .as_f64()
                .map(|n| n as f32)
                .filter(|n| n.is_finite() && *n >= 0.0)
                .map(|n| Some(Some(n)))
                .ok_or_else(|| invalid(key, "a number of 0 or more, or null", v)),
        }
    };
    // Minimum and maximum: one pair each, 0 and infinity meaning none.
    for (field, keys, none) in [(Field::MinSize, ["minWidth", "minHeight"], 0.0), (Field::MaxSize, ["maxWidth", "maxHeight"], f32::INFINITY)] {
        let (x, y) = (limit(keys[0])?, limit(keys[1])?);
        if x.is_none() && y.is_none() {
            continue;
        }
        let current = pair_of(raw.field(field)).unwrap_or((none, none));
        let x = x.map_or(current.0, |v| v.unwrap_or(none));
        let y = y.map_or(current.1, |v| v.unwrap_or(none));
        if x == none && y == none {
            remove.push(field);
        } else {
            write.push((field, raw_pair(x, y)));
        }
    }
    if let (Some(min), Some(max)) = (pair_of(write.iter().find(|(k, _)| *k == Field::MinSize).map(|(_, v)| v)).or_else(|| pair_of(raw.field(Field::MinSize))), pair_of(write.iter().find(|(k, _)| *k == Field::MaxSize).map(|(_, v)| v)).or_else(|| pair_of(raw.field(Field::MaxSize)))) {
        if min.0 > max.0 || min.1 > max.1 {
            return Err(SizingError::MinAboveMax.into());
        }
    }
    if let Some(value) = properties.get("constraints") {
        let given = value.as_map().ok_or_else(|| invalid("constraints", "{ horizontal?, vertical? }", value))?;
        for (key, field) in [("horizontal", Field::HorizontalConstraint), ("vertical", Field::VerticalConstraint)] {
            let Some(v) = given.get(key) else { continue };
            match v.as_str().and_then(|c| ConstraintType::listed(c, &CONSTRAINTS)) {
                Some(ConstraintType::Min) => remove.push(field),
                Some(constraint) => write.push((field, constraint.into())),
                None => {
                    return Err(invalid(&format!("constraints.{key}"), "MIN, CENTER, MAX, STRETCH or SCALE", v))
                }
            }
        }
        if let Some(unknown) = given.keys().find(|k| !matches!(k.as_str(), "horizontal" | "vertical")) {
            return Err(DomainError::Unexpected {
                at: format!("constraints.{unknown}"),
                expected: "horizontal or vertical".into(),
            }
            .into());
        }
    }
    if let Some(value) = properties.get("rotation") {
        let degrees = value
            .as_f64()
            .filter(|d| d.is_finite())
            .ok_or_else(|| invalid("rotation", "degrees", value))? as f32;
        // Counter-clockwise positive, as Figma's panel shows it,
        // around the transform's origin, as the plugin API sets it.
        let Some(Raw::Object(mut t)) = transform.or_else(|| raw.field(Field::Transform).cloned()) else {
            return Err(SizingError::NoTransformToRotate.into());
        };
        let (s, c) = degrees.to_radians().sin_cos();
        let clean = |v: f32| if v.abs() < 1e-7 { 0.0 } else { v };
        t.set(Field::M00, Raw::Float(clean(c)));
        t.set(Field::M01, Raw::Float(clean(s)));
        t.set(Field::M10, Raw::Float(clean(-s)));
        t.set(Field::M11, Raw::Float(clean(c)));
        write.push((Field::Transform, Raw::Object(t)));
    }
    Ok((write, remove))
}

/// Move and size a resized frame's children by their constraints, and theirs
/// in turn, as Figma does when a frame is resized in its editor.
pub fn follow_constraints(session: &mut Session, id: Guid, (old_w, old_h): (f32, f32)) -> Vec<Guid> {
    let mut touched = Vec::new();
    let Some(node) = session.graph.node(id) else { return touched };
    if node.layout.mode != LayoutMode::None || matches!(node.node_type, NodeType::Group | NodeType::Instance) {
        return touched;
    }
    let (new_w, new_h) = (node.width, node.height);
    if (new_w - old_w).abs() < 1e-3 && (new_h - old_h).abs() < 1e-3 {
        return touched;
    }
    let kids: Vec<Placed> = session
        .graph
        .children(id)
        .map(|c| (c.id, c.x, c.y, c.width, c.height, c.horizontal_constraint.clone(), c.vertical_constraint.clone(), c.rotation))
        .collect();
    for (kid, x, y, w, h, hc, vc, rotation) in kids {
        if rotation != 0.0 {
            continue;
        }
        let place = |c: &str, at: f32, size: f32, old: f32, new: f32| -> (f32, f32) {
            let d = new - old;
            match ConstraintType::from_wire(c) {
                Some(ConstraintType::Max) => (at + d, size),
                Some(ConstraintType::Center) => (at + d / 2.0, size),
                Some(ConstraintType::Stretch) => (at, (size + d).max(0.01)),
                Some(ConstraintType::Scale) if old > 0.0 => (at * new / old, size * new / old),
                _ => (at, size),
            }
        };
        let (nx, nw) = place(&hc, x, w, old_w, new_w);
        let (ny, nh) = place(&vc, y, h, old_h, new_h);
        if (nx - x).abs() < 1e-3 && (ny - y).abs() < 1e-3 && (nw - w).abs() < 1e-3 && (nh - h).abs() < 1e-3 {
            continue;
        }
        let resized = (nw - w).abs() > 1e-3 || (nh - h).abs() > 1e-3;
        super::structure::set_frame(session, kid, Some((nx, ny)), resized.then_some((nw, nh)));
        session.refresh(&[kid]);
        touched.push(kid);
        if resized {
            touched.extend(follow_constraints(session, kid, (w, h)));
        }
    }
    touched
}
