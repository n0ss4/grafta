//! Pages, frames, instances; moving, deleting, saving: the tools' edge
//! halves.

use serde_json::{json, Value};

use super::args::{number, optional_number, optional_string, optional_strings, string};
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::tools::write;
use crate::error::WriteError;
use crate::{DomainError, ToolError};

pub struct CreatePage;

impl Tool for CreatePage {
    fn name(&self) -> &'static str {
        "create_page"
    }
    fn description(&self) -> &'static str {
        "Create a page at the end of the file's page list. Returns its id. Nothing is written \
         to disk until save_as."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({ "handle": handle_property(), "name": { "type": "string" } }),
            &["handle", "name"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        Ok(Request::Command(Box::new(write::CreatePage { handle, name: string(args, "name")?.into() })))
    }
}

pub struct CreateFrame;

impl Tool for CreateFrame {
    fn name(&self) -> &'static str {
        "create_frame"
    }
    fn description(&self) -> &'static str {
        "Create an empty frame (no fill, no auto-layout) on a page or inside a frame, at x,y \
         relative to the parent, with a width and height. Returns its id. Fills, strokes, \
         auto-layout and text are not available yet: they have not been measured in Figma."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page or frame id." },
                "name": { "type": "string" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "width": { "type": "number", "exclusiveMinimum": 0 },
                "height": { "type": "number", "exclusiveMinimum": 0 },
            }),
            &["handle", "parent", "name", "width", "height"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        let name = string(args, "name")?.into();
        let (width, height) = (number(args, "width")?, number(args, "height")?);
        if width <= 0.0 || height <= 0.0 {
            return Err(DomainError::NotPositiveSize { width, height }.into());
        }
        let at = (optional_number(args, "x", 0.0)?, optional_number(args, "y", 0.0)?);
        Ok(Request::Command(Box::new(write::CreateFrame { handle, parent, name, width, height, at })))
    }
}

pub struct CreateInstance;

impl Tool for CreateInstance {
    fn name(&self) -> &'static str {
        "create_instance"
    }
    fn description(&self) -> &'static str {
        "Create an instance of a component this file defines, on a page or inside a frame, at \
         x,y relative to the parent. It takes the component's own size. For a component set, \
         pass one variant's id (the set's children, via read_node). Components from another \
         file cannot be instantiated yet."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page or frame id." },
                "component": { "type": "string", "description": "A component (or variant) id." },
                "name": { "type": "string", "description": "Default: the component's name." },
                "x": { "type": "number" },
                "y": { "type": "number" },
            }),
            &["handle", "parent", "component"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        let component = string(args, "component")?.into();
        let name = optional_string(args, "name")?.map(str::to_owned);
        let at = (optional_number(args, "x", 0.0)?, optional_number(args, "y", 0.0)?);
        Ok(Request::Command(Box::new(write::CreateInstance { handle, parent, component, name, at })))
    }
}

pub struct SaveAs;

impl Tool for SaveAs {
    fn name(&self) -> &'static str {
        "save_as"
    }
    fn description(&self) -> &'static str {
        "Write the open file, with its edits, to a NEW path ending in its product's extension: .fig, or .deck for a \
         Figma Slides file, .jam for FigJam, .site for Figma Sites. Never overwrites: a path \
         that exists, or the source file, is refused. The written file is read back and \
         compared before success is reported. Open the result in Figma to see it."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "path": { "type": "string", "description": "A new path ending in .fig, or .deck, .jam or .site for a Figma Slides, FigJam or Figma Sites file." },
            }),
            &["handle", "path"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        Ok(Request::Query(Box::new(write::SaveAs { handle, path: string(args, "path")?.into() })))
    }
}

pub struct DeleteNode;

impl Tool for DeleteNode {
    fn name(&self) -> &'static str {
        "delete_node"
    }
    fn description(&self) -> &'static str {
        "Delete a node (a page, frame, text or instance) and everything inside it. Returns \
         the ids removed. Refused when another node points at it or at something inside it \
         (an instance of a component in it, a prototype link to it, an override of one of its \
         layers), naming who; and refused inside a component. `also` deletes more nodes in \
         the same call: what they point at among themselves does not count, so components \
         that use each other can go together."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string" },
                "also": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "More nodes deleted with it, in one call."
                }
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        Ok(Request::Command(Box::new(write::DeleteNode { handle, id, also: optional_strings(args, "also")? })))
    }
}

pub struct MoveNode;

impl Tool for MoveNode {
    fn name(&self) -> &'static str {
        "move_node"
    }
    fn description(&self) -> &'static str {
        "Move a frame, component or component set, instance, text layer, shape or vector: into \
         another page or frame, or among its \
         siblings. Siblings are drawn in order, the last on top; in an auto-layout frame the \
         order is also the flow. Give `parent` (with `to`: \"front\", the default, or \"back\"), \
         or `behind` / `in_front_of` a sibling to go right behind or in front of it, in that \
         sibling's parent. x,y are relative to the new parent; by default the node keeps them."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string" },
                "parent": { "type": "string", "description": "A page or frame id." },
                "to": { "type": "string", "enum": ["front", "back"], "description": "With parent: the top of the stack (last, the default) or the bottom (first)." },
                "behind": { "type": "string", "description": "A node to go right behind, in its parent." },
                "in_front_of": { "type": "string", "description": "A node to go right in front of, in its parent." },
                "x": { "type": "number" },
                "y": { "type": "number" },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let (behind, in_front_of) = (optional_string(args, "behind")?, optional_string(args, "in_front_of")?);
        // A `parent` given as null is given, and refused as no string.
        let given = [args.get("parent").is_some(), behind.is_some(), in_front_of.is_some()];
        if given.iter().filter(|g| **g).count() != 1 {
            return Err(WriteError::OnePlace.into());
        }
        let to = match (behind, in_front_of) {
            (Some(sibling), _) => write::Destination::Behind(sibling.into()),
            (_, Some(sibling)) => write::Destination::InFrontOf(sibling.into()),
            _ => {
                let back = match optional_string(args, "to")? {
                    None | Some("front") => false,
                    Some("back") => true,
                    Some(other) => {
                        return Err(DomainError::Expected {
                            parameter: "to".into(),
                            expected: "\"front\" or \"back\"".into(),
                            got: other.into(),
                        }
                        .into())
                    }
                };
                write::Destination::Parent { id: string(args, "parent")?.into(), back }
            }
        };
        // Not given, the node keeps where it is.
        let at = |name| match args.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => number(args, name).map(Some),
        };
        let (x, y) = (at("x")?, at("y")?);
        Ok(Request::Command(Box::new(write::MoveNode { handle, id, to, x, y })))
    }
}
