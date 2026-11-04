//! Text layers: the tool's edge half.

use schema::WireName;
use serde_json::{json, Value};

use super::args::{optional_number, optional_string, string};
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::error::TextError;
use crate::tools::{rich_text, text};
use crate::{DomainError, ToolError};

pub struct CreateText;

impl Tool for CreateText {
    fn name(&self) -> &'static str {
        "create_text"
    }
    fn description(&self) -> &'static str {
        "Create a text layer on a page or in a frame. characters is the text (a line break \
         starts a new paragraph). fontFamily and fontStyle name a font as Figma does (e.g. \
         \"Inter\", \"Semi Bold\"); by default, the file's most used font. fontSize, fills (like \
         set_properties: SOLID colour or a local colour variable; default black), x, y, \
         textAlignHorizontal (LEFT, CENTER, RIGHT, JUSTIFIED), lineHeight (pixels, or \
         { value, unit: PIXELS | PERCENT }), letterSpacing (likewise). Give width to wrap \
         text inside a fixed width; give height too to fix the box (textAutoResize NONE); \
         without either the box fits the text. Rich text: runs instead of characters, \
         [{ characters, fontFamily?, fontStyle?, fontSize?, fills?, textDecoration?, \
         textCase?, letterSpacing?, lineHeight?, hyperlink?: { type: URL, value } }], each \
         run styled apart from the layer (a link is underlined unless it says otherwise); \
         paragraphs, one per line of the text, [{ listType: NONE | UNORDERED | ORDERED, \
         indentation?: 1 | 2 }]; paragraphSpacing, paragraphIndent, textDecoration (NONE, \
         UNDERLINE, STRIKETHROUGH), textCase (ORIGINAL, UPPER, LOWER, TITLE, SMALL_CAPS, \
         SMALL_CAPS_FORCED), textAlignVertical (TOP, CENTER, BOTTOM) and textAutoResize \
         (NONE, WIDTH_AND_HEIGHT, HEIGHT). Figma lays the text out when the file is opened, \
         so the size grafta reports is provisional."
    }
    fn input_schema(&self) -> Value {
        let dimension = json!({ "oneOf": [
            { "type": "number" },
            { "type": "object", "properties": { "value": { "type": "number" }, "unit": { "enum": text::UNITS.map(WireName::as_wire) } }, "required": ["value", "unit"] },
        ] });
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page or frame id." },
                "characters": { "type": "string" },
                "name": { "type": "string", "description": "Layer name. Default: the characters." },
                "fontFamily": { "type": "string" },
                "fontStyle": { "type": "string" },
                "fontSize": { "type": "number", "exclusiveMinimum": 0 },
                "fills": { "type": "array", "items": { "type": "object" } },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "width": { "type": "number", "exclusiveMinimum": 0 },
                "textAlignHorizontal": { "enum": rich_text::TEXT_ALIGNS_HORIZONTAL.map(WireName::as_wire) },
                "lineHeight": dimension.clone(),
                "letterSpacing": dimension,
                "height": { "type": "number", "exclusiveMinimum": 0 },
                "runs": { "type": "array", "items": { "type": "object" }, "description": "The text as styled pieces; instead of characters." },
                "paragraphs": { "type": "array", "items": { "type": "object" } },
                "paragraphSpacing": { "type": "number", "minimum": 0 },
                "paragraphIndent": { "type": "number", "minimum": 0 },
                "textDecoration": { "enum": rich_text::TEXT_DECORATIONS.map(WireName::as_wire) },
                "textCase": { "enum": rich_text::TEXT_CASES.map(WireName::as_wire) },
                "textAlignVertical": { "enum": rich_text::TEXT_ALIGNS_VERTICAL.map(WireName::as_wire) },
                "textAutoResize": { "enum": rich_text::TEXT_AUTO_RESIZES.map(WireName::as_wire) },
            }),
            &["handle", "parent"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        let characters = match (args.get("characters"), args.get("runs")) {
            (Some(_), Some(_)) => return Err(TextError::TwoWays.into()),
            (None, None) => return Err(TextError::NoText.into()),
            (Some(_), None) => {
                let characters = string(args, "characters")?;
                if characters.is_empty() {
                    return Err(DomainError::Unexpected { at: "characters".into(), expected: "non-empty text".into() }.into());
                }
                characters.to_string()
            }
            (None, Some(_)) => String::new(),
        };
        let asked = data::from_json(args).as_map().cloned().unwrap_or_default();
        let family = optional_string(args, "fontFamily")?.map(str::to_owned);
        let style = optional_string(args, "fontStyle")?.map(str::to_owned);
        let positive = |name| asked.get(name).map(|v| text::positive(name, v)).transpose();
        let font_size = positive("fontSize")?;
        let align = optional_string(args, "textAlignHorizontal")?.map(text::alignment).transpose()?;
        let (width, height) = (positive("width")?, positive("height")?);
        let at = (optional_number(args, "x", 0.0)?, optional_number(args, "y", 0.0)?);
        let name = optional_string(args, "name")?.map(str::to_owned);
        Ok(Request::Command(Box::new(text::CreateText {
            handle,
            parent,
            characters,
            family,
            style,
            font_size,
            align,
            width,
            height,
            at,
            name,
            asked,
        })))
    }
}
