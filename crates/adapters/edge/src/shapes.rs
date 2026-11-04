//! Shapes: the tool's edge half.

use model::author::ShapeKind;
use serde_json::{json, Value};

use super::args::{optional_string, string};
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::app::data::Data;
use crate::error::ShapeError;
use crate::tools::shapes;
use crate::tools::style::invalid;
use crate::{DomainError, ToolError};

fn kind_of(value: &Data) -> Result<ShapeKind, ToolError> {
    value
        .as_str()
        .and_then(|name| shapes::SHAPES.iter().find(|(n, _)| *n == name).map(|(_, kind)| *kind))
        .ok_or_else(|| invalid("type", "RECTANGLE, ELLIPSE, LINE, POLYGON or STAR", value))
}

fn number(args: &Data, key: &str, default: Option<f32>) -> Result<f32, ToolError> {
    match args.get(key) {
        Some(v) => shapes::float(key, v),
        None => default.ok_or_else(|| DomainError::Required { parameter: key.into() }.into()),
    }
}

fn shape_size(kind: ShapeKind, args: &Data) -> Result<(f32, f32), ToolError> {
    let width = number(args, "width", None)?;
    if width <= 0.0 {
        return Err(DomainError::NotPositive { at: "width".into(), got: Some(width.to_string()) }.into());
    }
    if kind == ShapeKind::Line {
        if args.get("height").is_some() {
            return Err(ShapeError::LineHeight.into());
        }
        return Ok((width, 0.0));
    }
    let height = number(args, "height", None)?;
    if height <= 0.0 {
        return Err(DomainError::NotPositive { at: "height".into(), got: Some(height.to_string()) }.into());
    }
    Ok((width, height))
}

pub struct CreateShape;

impl Tool for CreateShape {
    fn name(&self) -> &'static str {
        "create_shape"
    }
    fn description(&self) -> &'static str {
        "Draw a shape as Figma's own layer type: RECTANGLE (cornerRadius), ELLIPSE (arcData { \
         startingAngle, endingAngle, innerRadius } in radians: a ring, a pie, a half arc), LINE \
         (width is its length; needs strokes), POLYGON (pointCount sides) or STAR (pointCount \
         points, innerRadius 0-1). Fills and strokes as set_properties takes them; nothing is \
         painted unless named. Style it further with set_properties."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page or frame id." },
                "type": { "enum": shapes::SHAPES.map(|(name, _)| name) },
                "name": { "type": "string" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "width": { "type": "number", "exclusiveMinimum": 0 },
                "height": { "type": "number", "exclusiveMinimum": 0, "description": "Not for a LINE." },
                "fills": { "type": "array", "items": { "type": "object" } },
                "strokes": { "type": "array", "items": { "type": "object" } },
                "strokeWeight": { "type": "number", "exclusiveMinimum": 0 },
                "cornerRadius": { "type": "number", "minimum": 0 },
                "arcData": { "type": "object" },
                "pointCount": { "type": "integer", "minimum": 3 },
                "innerRadius": { "type": "number", "exclusiveMinimum": 0, "exclusiveMaximum": 1 },
            }),
            &["handle", "parent", "type", "width"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let asked = data::from_json(args);
        let kind = kind_of(asked.get("type").unwrap_or(&Data::Null))?;
        let values = asked.as_map().cloned().unwrap_or_default();
        let size = shape_size(kind, &asked)?;
        let at = (number(&asked, "x", Some(0.0))?, number(&asked, "y", Some(0.0))?);
        let name = optional_string(args, "name")?.map_or_else(|| shapes::default_name(kind).to_string(), str::to_string);
        let parent = string(args, "parent")?.into();
        Ok(Request::Command(Box::new(shapes::CreateShape { handle, parent, kind, name, size, at, values })))
    }
}
