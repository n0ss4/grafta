//! Importing a web page's layout: the tool's edge half.

use serde_json::{json, Value};

use super::args::{optional_number, optional_string, string};
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::tools::import;
use crate::{DomainError, ToolError};

pub struct ImportLayout;

impl Tool for ImportLayout {
    fn name(&self) -> &'static str {
        "import_layout"
    }
    fn description(&self) -> &'static str {
        "Write a web page's layout as design layers: a layout snapshot (JSON from grafta's \
         snapshot.js, https://n0ss4.github.io/grafta/snapshot.js, run in a browser on the page: \
         graftaSnapshot('<css selector>')) becomes frames, text, images and \
         auto-layout in a page or frame. Flexbox becomes auto-layout with fill, hug or fixed \
         sizing as the browser measured it (layout: \"auto\", the default), or every box is \
         placed where the browser put it (\"absolute\"). Backgrounds, uniform borders, radii, \
         box shadows, opacity, fonts and local or data: images are written; what is not (a \
         gradient, a transform, an inline SVG, a remote image) is listed under not_imported, \
         with its element. One call."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page or frame id." },
                "snapshot": { "type": "string", "description": "The snapshot's JSON file." },
                "layout": { "type": "string", "enum": ["auto", "absolute"], "description": "Flexbox as auto-layout (default), or every box where the browser put it." },
                "x": { "type": "number" },
                "y": { "type": "number" },
            }),
            &["handle", "parent", "snapshot"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        let snapshot = string(args, "snapshot")?.into();
        let auto = match optional_string(args, "layout")? {
            None | Some("auto") => true,
            Some("absolute") => false,
            Some(other) => {
                return Err(DomainError::Expected {
                    parameter: "layout".into(),
                    expected: "auto or absolute".into(),
                    got: other.into(),
                }
                .into())
            }
        };
        let at = (optional_number(args, "x", 0.0)?, optional_number(args, "y", 0.0)?);
        Ok(Request::Command(Box::new(import::ImportLayout { handle, parent, snapshot, auto, at })))
    }
}
