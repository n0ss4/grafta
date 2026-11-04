//! Groups, sections and copies: the tools' edge halves.

use serde_json::{json, Value};

use super::args::{number, optional_number, optional_string, string};
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::error::StructureError;
use crate::tools::structure;
use crate::ToolError;

fn ids_of(args: &Value) -> Result<Vec<String>, ToolError> {
    let Some(Value::Array(items)) = args.get("ids") else {
        return Err(StructureError::NoIds.into());
    };
    let ids: Vec<String> = items.iter().filter_map(Value::as_str).map(str::to_string).collect();
    if ids.is_empty() || ids.len() != items.len() {
        return Err(StructureError::NotIds.into());
    }
    Ok(ids)
}

pub struct GroupNodes;

impl Tool for GroupNodes {
    fn name(&self) -> &'static str {
        "group_nodes"
    }
    fn description(&self) -> &'static str {
        "Group layers that share a parent, as Figma's Group: the group sits at their bounds and \
         each layer keeps its place on the canvas. Moving or resizing a grouped layer with \
         set_properties fits the group again. ungroup undoes it."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "ids": { "type": "array", "items": { "type": "string" }, "minItems": 1 },
                "name": { "type": "string", "description": "Default \"Group\"." },
            }),
            &["handle", "ids"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let ids = ids_of(args)?;
        let name = optional_string(args, "name")?.unwrap_or("Group").to_string();
        Ok(Request::Command(Box::new(structure::GroupNodes { handle, ids, name })))
    }
}

pub struct Ungroup;

impl Tool for Ungroup {
    fn name(&self) -> &'static str {
        "ungroup"
    }
    fn description(&self) -> &'static str {
        "Ungroup a group: its layers go to the group's parent, in its place and order, each where \
         it was on the canvas, and the group is removed."
    }
    fn input_schema(&self) -> Value {
        schema(json!({ "handle": handle_property(), "id": { "type": "string", "description": "A group's id." } }), &["handle", "id"])
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        Ok(Request::Command(Box::new(structure::Ungroup { handle, id: string(args, "id")?.into() })))
    }
}

pub struct CreateSection;

impl Tool for CreateSection {
    fn name(&self) -> &'static str {
        "create_section"
    }
    fn description(&self) -> &'static str {
        "Make a section on a page, to organise screens (\"Onboarding\", \"Checkout\"). Then move \
         frames into it with move_node (parent: the section), or create them in it."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "parent": { "type": "string", "description": "A page's id." },
                "name": { "type": "string" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "width": { "type": "number", "exclusiveMinimum": 0 },
                "height": { "type": "number", "exclusiveMinimum": 0 },
            }),
            &["handle", "parent", "width", "height"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let parent = string(args, "parent")?.into();
        let (w, h) = (optional_number(args, "width", 0.0)?, optional_number(args, "height", 0.0)?);
        if w <= 0.0 || h <= 0.0 {
            return Err(StructureError::SectionSize { width: w, height: h }.into());
        }
        let at = (optional_number(args, "x", 0.0)?, optional_number(args, "y", 0.0)?);
        let name = optional_string(args, "name")?.unwrap_or("Section").to_string();
        Ok(Request::Command(Box::new(structure::CreateSection { handle, parent, name, size: (w, h), at })))
    }
}

pub struct DuplicateNode;

impl Tool for DuplicateNode {
    fn name(&self) -> &'static str {
        "duplicate_node"
    }
    fn description(&self) -> &'static str {
        "Duplicate a node with everything in it: every layer copied with a new id, instances \
         still instances of their components, bindings and styles kept. The copy goes in the \
         same parent unless one is named, at x, y (by default the original's), in front of its \
         new siblings; then change it. A component, or anything holding one, is not duplicated."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string" },
                "parent": { "type": "string", "description": "Where the copy goes; by default the original's parent." },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "name": { "type": "string" },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        // Not given, the copy goes where the original is.
        let at = |name| match args.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(_) => number(args, name).map(Some),
        };
        let (x, y) = (at("x")?, at("y")?);
        let parent = optional_string(args, "parent")?.map(str::to_owned);
        let name = optional_string(args, "name")?.map(str::to_owned);
        Ok(Request::Command(Box::new(structure::DuplicateNode { handle, id, parent, x, y, name })))
    }
}
