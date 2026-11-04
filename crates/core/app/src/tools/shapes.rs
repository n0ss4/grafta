//! Shapes: rectangles, ellipses, lines, polygons and stars as
//! Figma's own node types.
//!
//! Each is written in the twelve fields every local rectangle and ellipse
//! carries (12,064 measured), plus its kind's own, and no geometry: Figma
//! derives it for every kind (measured in Figma), and `render_node` does the same.

use kiwi::Value as Raw;
use model::author::ShapeKind;
use model::NodeType;
use schema::{object, Field};
use crate::data::{Data, Map};
use crate::Command;

use super::write::{author_error, created, record, require_parent, Placement};
use super::style::invalid;
use crate::error::ShapeError;
use crate::{DomainError, Output, Session, ToolError, Toolkit};

/// The shapes `create_shape` makes, in the plugin API's names.
pub const SHAPES: [(&str, ShapeKind); 5] = [
    ("RECTANGLE", ShapeKind::Rectangle),
    ("ELLIPSE", ShapeKind::Ellipse),
    ("LINE", ShapeKind::Line),
    ("POLYGON", ShapeKind::Polygon),
    ("STAR", ShapeKind::Star),
];

/// The kind a node is, if it is a shape.
pub fn kind_of_node(node_type: NodeType) -> Option<ShapeKind> {
    match node_type {
        NodeType::Rectangle | NodeType::RoundedRectangle => Some(ShapeKind::Rectangle),
        NodeType::Ellipse => Some(ShapeKind::Ellipse),
        NodeType::Line => Some(ShapeKind::Line),
        NodeType::Polygon => Some(ShapeKind::Polygon),
        NodeType::Star => Some(ShapeKind::Star),
        _ => None,
    }
}

pub fn float(at: &str, value: &Data) -> Result<f32, ToolError> {
    value.as_f64().map(|n| n as f32).filter(|n| n.is_finite()).ok_or_else(|| invalid(at, "a number", value))
}

/// A shape's own values, in the plugin API's names: an ellipse's `arcData`
/// (radians, the inner radius a fraction of the outer), a polygon's or star's
/// `pointCount`, a star's `innerRadius`. Refused on a kind that has none.
pub fn own_values(kind: ShapeKind, values: &Map) -> Result<Vec<(Field, Raw)>, ToolError> {
    let mut out = Vec::new();
    if let Some(v) = values.get("arcData") {
        if kind != ShapeKind::Ellipse {
            return Err(ShapeError::NoArc.into());
        }
        let a = v.as_map().ok_or_else(|| invalid("arcData", "{ startingAngle, endingAngle, innerRadius }", v))?;
        let get = |k: &str, default: f32| a.get(k).map_or(Ok(default), |x| float(&format!("arcData.{k}"), x));
        let (start, end, inner) = (get("startingAngle", 0.0)?, get("endingAngle", std::f32::consts::TAU)?, get("innerRadius", 0.0)?);
        if !(0.0..1.0).contains(&inner) {
            return Err(invalid("arcData.innerRadius", "a fraction from 0 up to 1", inner));
        }
        out.push((Field::ArcData, arc(start, end, inner)));
    }
    if let Some(v) = values.get("pointCount") {
        if !matches!(kind, ShapeKind::Polygon | ShapeKind::Star) {
            return Err(ShapeError::NoPoints.into());
        }
        let n = v.as_u64().filter(|n| (3..=60).contains(n)).ok_or_else(|| invalid("pointCount", "a whole number from 3 to 60", v))?;
        out.push((Field::Count, Raw::Uint(n as u32)));
    }
    if let Some(v) = values.get("innerRadius") {
        if kind != ShapeKind::Star {
            return Err(ShapeError::NoInnerRadius.into());
        }
        let r = float("innerRadius", v)?;
        if !(r > 0.0 && r < 1.0) {
            return Err(invalid("innerRadius", "a fraction between 0 and 1", r));
        }
        out.push((Field::StarInnerScale, Raw::Float(r)));
    }
    Ok(out)
}

fn arc(start: f32, end: f32, inner: f32) -> Raw {
    object([
        (Field::StartingAngle, Raw::Float(start)),
        (Field::EndingAngle, Raw::Float(end)),
        (Field::InnerRadius, Raw::Float(inner)),
    ])
}

/// A shape, as Figma's own layer type.
pub struct CreateShape {
    pub handle: String,
    pub parent: String,
    pub kind: ShapeKind,
    pub name: String,
    pub size: (f32, f32),
    pub at: (f32, f32),
    /// The call's arguments, for its paints and the kind's own values.
    pub values: Map,
}

impl Command for CreateShape {
    fn run(&self, kit: &mut Toolkit) -> Result<Output, ToolError> {
        let session = kit.session_mut(&self.handle)?;
        let extra = extra(session, self.kind, &self.values)?;
        let parent = require_parent(session, &self.parent, Placement::Shape)?;
        let id = session
            .graft()
            .map_err(author_error)?
            .add_shape_at(parent, self.kind, &self.name, self.size, self.at, extra)
            .map_err(author_error)?;
        record(session, id)?;
        Ok(Output::Data(created(session, id)))
    }
}

pub fn default_name(kind: ShapeKind) -> &'static str {
    match kind {
        ShapeKind::Rectangle => "Rectangle",
        ShapeKind::Ellipse => "Ellipse",
        ShapeKind::Line => "Line",
        ShapeKind::Polygon => "Polygon",
        ShapeKind::Star => "Star",
    }
}

/// Paints, stroke weight, radius and the kind's own values, checked before
/// anything is written.
fn extra(session: &Session, kind: ShapeKind, values: &Map) -> Result<Vec<(Field, Raw)>, ToolError> {
    let mut out = Vec::new();
    if let Some(v) = values.get("fills") {
        if kind == ShapeKind::Line {
            return Err(ShapeError::LineFills.into());
        }
        out.push((Field::FillPaints, super::style::paints(session, "fills", v)?));
    }
    match values.get("strokes") {
        Some(v) => out.push((Field::StrokePaints, super::style::paints(session, "strokes", v)?)),
        // All 872 local lines carry strokes: a line without one is invisible.
        None if kind == ShapeKind::Line => return Err(ShapeError::LineStrokes.into()),
        None => {}
    }
    if let Some(v) = values.get("strokeWeight") {
        let w = float("strokeWeight", v)?;
        if w <= 0.0 {
            return Err(DomainError::NotPositive { at: "strokeWeight".into(), got: Some(w.to_string()) }.into());
        }
        out.push((Field::StrokeWeight, Raw::Float(w)));
    }
    if let Some(v) = values.get("cornerRadius") {
        if matches!(kind, ShapeKind::Ellipse | ShapeKind::Line) {
            return Err(ShapeError::NoCorners { key: "cornerRadius" }.into());
        }
        let r = float("cornerRadius", v)?;
        if r < 0.0 {
            return Err(ShapeError::NegativeRadius { got: r }.into());
        }
        out.push((Field::CornerRadius, Raw::Float(r)));
    }
    let mut own = own_values(kind, values)?;
    match kind {
        // A full circle unless told otherwise, as 2,083 of 2,129 ellipses.
        ShapeKind::Ellipse if !own.iter().any(|(k, _)| *k == Field::ArcData) => own.push((Field::ArcData, arc(0.0, std::f32::consts::TAU, 0.0))),
        ShapeKind::Polygon | ShapeKind::Star => {
            if !own.iter().any(|(k, _)| *k == Field::Count) {
                own.push((Field::Count, Raw::Uint(if kind == ShapeKind::Star { 5 } else { 3 })));
            }
            if kind == ShapeKind::Star && !own.iter().any(|(k, _)| *k == Field::StarInnerScale) {
                own.push((Field::StarInnerScale, Raw::Float(0.382)));
            }
            own.push((Field::ProportionsConstrained, Raw::Bool(true)));
        }
        _ => {}
    }
    out.extend(own);
    Ok(out)
}
