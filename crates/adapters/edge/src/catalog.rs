//! A file's design system: components, variables, images. The tools' edge
//! halves.

use serde_json::{json, Value};

use super::args::{optional_integer, optional_number, optional_string, paging, string};
use super::schema::{handle_property, merge, paging_properties, schema};
use super::{Request, Tool};
use crate::error::CatalogError;
use crate::tools::catalog;
use crate::{DomainError, ToolError};

pub struct ListComponents;

impl Tool for ListComponents {
    fn name(&self) -> &'static str {
        "list_components"
    }
    fn description(&self) -> &'static str {
        "The components this file defines: every component set (with its variant properties \
         and their values) and every standalone component, with id, name, page, component \
         property definitions and how many instances use it. Variants inside a set are not \
         listed separately; read_node on the set lists them as children. Paged."
    }
    fn input_schema(&self) -> Value {
        schema(
            merge(
                json!({
                    "handle": handle_property(),
                    "name": { "type": "string", "description": "Only components whose name contains this (case-insensitive)." },
                }),
                paging_properties(),
            ),
            &["handle"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(catalog::ListComponents {
            handle: string(args, "handle")?.into(),
            name: optional_string(args, "name")?.map(str::to_owned),
            paging: paging(args)?,
        })))
    }
}

pub struct ComponentInstances;

impl Tool for ComponentInstances {
    fn name(&self) -> &'static str {
        "component_instances"
    }
    fn description(&self) -> &'static str {
        "Where a component is used: every instance of it (of any variant, for a component \
         set), with id, name and path. Paged."
    }
    fn input_schema(&self) -> Value {
        schema(
            merge(
                json!({
                    "handle": handle_property(),
                    "id": { "type": "string", "description": "A component or component set id." },
                }),
                paging_properties(),
            ),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(catalog::ComponentInstances {
            handle: string(args, "handle")?.into(),
            id: string(args, "id")?.into(),
            paging: paging(args)?,
        })))
    }
}

pub struct ListVariables;

impl Tool for ListVariables {
    fn name(&self) -> &'static str {
        "list_variables"
    }
    fn description(&self) -> &'static str {
        "The file's design variables (tokens): each collection with its modes, and each \
         variable's type and value per mode. Colours are hex; an alias is shown as \
         { alias: <variable name> }, not flattened. Paged over variables."
    }
    fn input_schema(&self) -> Value {
        schema(
            merge(
                json!({
                    "handle": handle_property(),
                    "collection": { "type": "string", "description": "Only this collection (name or id)." },
                }),
                paging_properties(),
            ),
            &["handle"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(catalog::ListVariables {
            handle: string(args, "handle")?.into(),
            collection: optional_string(args, "collection")?.map(str::to_owned),
            paging: paging(args)?,
        })))
    }
}

pub struct GetImage;

impl Tool for GetImage {
    fn name(&self) -> &'static str {
        "get_image"
    }
    fn description(&self) -> &'static str {
        "An image the file embeds, by the imageHash a fill carries (read_node shows it). \
         Returned as an image, up to 4 MB; a larger one is described instead."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "hash": { "type": "string", "description": "The fill's imageHash." },
            }),
            &["handle", "hash"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(catalog::GetImage { handle: string(args, "handle")?.into(), hash: string(args, "hash")?.into() })))
    }
}

pub struct RenderNode;

impl Tool for RenderNode {
    fn name(&self) -> &'static str {
        "render_node"
    }
    fn description(&self) -> &'static str {
        "Draw a node (a board, frame, component, instance, shape or text) as a PNG you can look \
         at, from the open file including unsaved edits. Use it to see a screen you read, or to \
         check a change before save_as. Drawn offline: the caption's `notes` name everything \
         drawn approximately or left out, and an empty list means nothing was."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "Node id, e.g. 12:345." },
                "scale": { "type": "number", "minimum": 0.1, "maximum": 4, "description": "Pixels per unit. Default 1." },
                "max_size": { "type": "integer", "minimum": 64, "maximum": 4096, "description": "Largest side in pixels; a larger picture is scaled down to fit. Default 2048." },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let (handle, id) = (string(args, "handle")?.into(), string(args, "id")?.into());
        let scale = optional_number(args, "scale", 1.0)?;
        if !(0.1..=4.0).contains(&scale) {
            return Err(DomainError::Expected {
                parameter: "scale".into(),
                expected: "a number from 0.1 to 4".into(),
                got: scale.to_string(),
            }
            .into());
        }
        let max_size = optional_integer(args, "max_size", 2048, 64..=4096)?;
        Ok(Request::Query(Box::new(catalog::RenderNode { handle, id, scale, max_size })))
    }
}

pub struct CheckContrast;

impl Tool for CheckContrast {
    fn name(&self) -> &'static str {
        "check_contrast"
    }
    fn description(&self) -> &'static str {
        "Check every visible text under a node (a screen, a frame, a component) against WCAG \
         2.2 AA: its colour against what is actually drawn behind it, as render_node draws it, \
         in the frame's mode and with instances' overrides. 4.5:1 is required, 3:1 for large \
         text (24 px, or 18.66 px bold). Lists the texts that fail, with colour, background, \
         ratio and what is required, and those that cannot be measured, with why; `all: true` \
         lists the passing ones too. Over a background of several colours, the lowest ratio is \
         given. Paged."
    }
    fn input_schema(&self) -> Value {
        schema(
            merge(
                json!({
                    "handle": handle_property(),
                    "id": { "type": "string", "description": "The node whose texts are checked." },
                    "all": { "type": "boolean", "description": "List passing texts too. Default false." },
                }),
                paging_properties(),
            ),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let (handle, id) = (string(args, "handle")?.into(), string(args, "id")?.into());
        // `all` as it always was read: true, or anything else false.
        let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
        Ok(Request::Query(Box::new(catalog::CheckContrast { handle, id, all, paging: paging(args)? })))
    }
}

pub struct ExportPdf;

impl Tool for ExportPdf {
    fn name(&self) -> &'static str {
        "export_pdf"
    }
    fn description(&self) -> &'static str {
        "Write nodes (frames, boards, components) to a NEW .pdf file, a page each in the order \
         given, each page the node's size in points: screens for review or handoff. Each page is \
         the drawing render_node makes, unsaved edits and modes included, as vectors, with text \
         grafta wrote kept as selectable text. The result's `pages` give each page's notes, as \
         render_node's do; a shadow or blur makes what it covers an image. Never overwrites."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "ids": { "type": "array", "items": { "type": "string" }, "minItems": 1, "description": "Node ids, one page each, in order." },
                "path": { "type": "string", "description": "A new path ending in .pdf." },
            }),
            &["handle", "ids", "path"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let path = std::path::PathBuf::from(string(args, "path")?);
        if path.extension().and_then(|e| e.to_str()) != Some("pdf") {
            return Err(DomainError::Expected {
                parameter: "path".into(),
                expected: "a path ending in .pdf".into(),
                got: path.display().to_string(),
            }
            .into());
        }
        let ids = match args.get("ids") {
            Some(Value::Array(ids)) if !ids.is_empty() => ids
                .iter()
                .map(|id| {
                    id.as_str().map(str::to_owned).ok_or_else(|| DomainError::Expected {
                        parameter: "ids".into(),
                        expected: "node ids".into(),
                        got: id.to_string(),
                    })
                })
                .collect::<Result<_, _>>()?,
            _ => return Err(CatalogError::NoIds.into()),
        };
        Ok(Request::Query(Box::new(catalog::ExportPdf { handle, path, ids })))
    }
}
