//! Vectors: the tool's edge half.

use model::path::Path;
use schema::{StrokeCap, StrokeJoin, WireName};
use serde_json::{json, Value};

use super::args::{optional_number, optional_string, string};
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::error::VectorError;
use crate::tools::{style, vector};
use crate::{DomainError, ToolError};

/// The fill rules a vector takes, in the plugin API's names.
const NONZERO: &str = "NONZERO";
const EVENODD: &str = "EVENODD";
const FILL_RULES: [&str; 2] = [NONZERO, EVENODD];

pub struct CreateVector;

impl Tool for CreateVector {
    fn name(&self) -> &'static str {
        "create_vector"
    }
    fn description(&self) -> &'static str {
        "Draw a vector from SVG path data (M L H V C S Q T A Z), as icon sets publish it: \
         Lucide, Heroicons, Material Symbols. `path` is one path's d, or a list of them. \
         `viewBox` is the box the data is drawn in ([0, 0, 24, 24] for Lucide), scaled to \
         width and height; without it, the path's own bounds. For a line icon give strokes, \
         strokeWeight (in the vector's pixels: Lucide's 2 at 24 px is 4 at 48 px), \
         strokeCap and strokeJoin ROUND, and no fills. Figma draws the geometry itself."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page or frame id." },
                "path": { "description": "SVG path data: a string, or a list of strings drawn together." },
                "viewBox": { "type": "array", "items": { "type": "number" }, "minItems": 4, "maxItems": 4, "description": "[min x, min y, width, height] of the box the path is drawn in." },
                "name": { "type": "string" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "width": { "type": "number", "exclusiveMinimum": 0 },
                "height": { "type": "number", "exclusiveMinimum": 0 },
                "fills": { "type": "array", "items": { "type": "object" } },
                "strokes": { "type": "array", "items": { "type": "object" } },
                "strokeWeight": { "type": "number", "exclusiveMinimum": 0 },
                "strokeCap": { "enum": style::STROKE_CAPS.map(WireName::as_wire) },
                "strokeJoin": { "enum": style::STROKE_JOINS.map(WireName::as_wire) },
                "fillRule": { "enum": FILL_RULES },
            }),
            &["handle", "parent", "path"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        // Each item is its own <path>: a relative move in one is relative to
        // its own start, not to where the previous item ended.
        let items: Vec<&str> = match args.get("path") {
            Some(Value::String(d)) => vec![d.as_str()],
            Some(Value::Array(items)) if !items.is_empty() => items
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
                .ok_or(VectorError::NotStrings)?,
            _ => return Err(VectorError::NoPath.into()),
        };
        let mut path = Path::default();
        for (i, d) in items.iter().enumerate() {
            let part = Path::parse(d).map_err(|error| {
                let which = if items.len() > 1 { format!("path[{i}]") } else { "path".into() };
                VectorError::BadPath { which, error: error.to_string() }
            })?;
            path.subpaths.extend(part.subpaths);
        }

        let (bx, by, bw, bh) = match args.get("viewBox") {
            None | Some(Value::Null) => {
                let (x0, y0, x1, y1) = path.bounds().expect("a parsed path draws something");
                (x0, y0, x1 - x0, y1 - y0)
            }
            Some(Value::Array(v)) if v.len() == 4 && v.iter().all(Value::is_number) => {
                let n: Vec<f32> = v.iter().filter_map(Value::as_f64).map(|n| n as f32).collect();
                (n[0], n[1], n[2], n[3])
            }
            Some(other) => {
                return Err(DomainError::Expected {
                    parameter: "viewBox".into(),
                    expected: "[min x, min y, width, height]".into(),
                    got: other.to_string(),
                }
                .into())
            }
        };
        if bw <= 0.0 || bh <= 0.0 {
            return Err(VectorError::NoArea { width: bw, height: bh }.into());
        }
        let (width, height) = match (args.get("width").and_then(Value::as_f64), args.get("height").and_then(Value::as_f64)) {
            (Some(w), Some(h)) => (w as f32, h as f32),
            (Some(w), None) => (w as f32, w as f32 * bh / bw),
            (None, Some(h)) => (h as f32 * bw / bh, h as f32),
            (None, None) => (bw, bh),
        };
        if width <= 0.0 || height <= 0.0 {
            return Err(DomainError::NotPositiveSize { width, height }.into());
        }
        let (sx, sy) = (width / bw, height / bh);
        let path = path.map(|(x, y)| ((x - bx) * sx, (y - by) * sy));

        fn choice<E: WireName>(args: &Value, name: &str, allowed: &[E], default: E) -> Result<E, ToolError> {
            match optional_string(args, name)? {
                None => Ok(default),
                Some(value) => E::listed(value, allowed).ok_or_else(|| {
                    DomainError::Expected {
                        parameter: name.into(),
                        expected: format!("one of {}", E::joined(allowed, ", ")),
                        got: value.into(),
                    }
                    .into()
                }),
            }
        }
        let cap = choice(args, "strokeCap", &style::STROKE_CAPS, StrokeCap::None)?;
        let join = choice(args, "strokeJoin", &style::STROKE_JOINS, StrokeJoin::Miter)?;
        let nonzero = match optional_string(args, "fillRule")? {
            None | Some(NONZERO) => true,
            Some(EVENODD) => false,
            Some(value) => {
                return Err(DomainError::Expected {
                    parameter: "fillRule".into(),
                    expected: format!("one of {}", FILL_RULES.join(", ")),
                    got: value.into(),
                }
                .into())
            }
        };
        let weight = optional_number(args, "strokeWeight", 1.0)?;
        if weight <= 0.0 {
            return Err(VectorError::ZeroStrokeWeight.into());
        }
        let fills = args.get("fills").map(data::from_json);
        let strokes = args.get("strokes").map(data::from_json);
        let name = optional_string(args, "name")?.unwrap_or("Vector").to_string();
        let at = (optional_number(args, "x", 0.0)?, optional_number(args, "y", 0.0)?);

        Ok(Request::Command(Box::new(vector::CreateVector {
            handle,
            parent,
            path,
            nonzero,
            size: (width, height),
            weight,
            cap,
            join,
            fills,
            strokes,
            name,
            at,
        })))
    }
}
