//! Finding and reading nodes: the tools' edge halves.

use serde_json::{json, Value};

use super::args::{budget, optional_bool, optional_integer, optional_string, paging, string};
use super::schema::{handle_property, merge, paging_properties, schema};
use super::{Request, Tool};
use crate::tools::read;
use crate::view::Limits;
use crate::ToolError;

pub struct Summarize;

impl Tool for Summarize {
    fn name(&self) -> &'static str {
        "summarize"
    }
    fn description(&self) -> &'static str {
        "Overview of an open file: every page with its id, node count and number of top-level \
         frames, plus totals for nodes, components, component sets, instances, text nodes, \
         variables, styles and images."
    }
    fn input_schema(&self) -> Value {
        schema(json!({ "handle": handle_property() }), &["handle"])
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(read::Summarize { handle: string(args, "handle")?.into() })))
    }
}

pub struct SearchNodes;

impl Tool for SearchNodes {
    fn name(&self) -> &'static str {
        "search_nodes"
    }
    fn description(&self) -> &'static str {
        "Find nodes by name (case-insensitive substring), by type (FRAME, TEXT, INSTANCE, \
         COMPONENT, COMPONENT_SET, PAGE, SECTION, GROUP, RECTANGLE, VECTOR, …), or both, \
         optionally only inside one node. Returns id, name, type and the path from the page, \
         in document order, paged. Hidden nodes are included."
    }
    fn input_schema(&self) -> Value {
        schema(
            merge(
                json!({
                    "handle": handle_property(),
                    "name": { "type": "string", "description": "Case-insensitive substring of the node name." },
                    "type": { "type": "string", "description": "Node type, in the plugin API's spelling." },
                    "within": { "type": "string", "description": "Only search inside this node id (a page, a frame…)." },
                }),
                paging_properties(),
            ),
            &["handle"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(read::SearchNodes {
            handle: string(args, "handle")?.into(),
            name: optional_string(args, "name")?.map(str::to_owned),
            node_type: optional_string(args, "type")?.map(str::to_owned),
            within: optional_string(args, "within")?.map(str::to_owned),
            paging: paging(args)?,
        })))
    }
}

pub struct ReadNode;

impl Tool for ReadNode {
    fn name(&self) -> &'static str {
        "read_node"
    }
    fn description(&self) -> &'static str {
        "Read one node exactly, in Figma's terms: geometry, fills/strokes (hex colours, bound \
         variable names), effects, corner radii, auto-layout, constraints, text and its style, \
         and for an instance its component and variant. Children to `depth` (default 1). \
         Properties at Figma's default are left out: visible true, opacity 1, rotation 0, no \
         fills/strokes/effects, radius 0, no auto-layout, constraints MIN/MIN. `undecoded` \
         names stored fields this view does not model; read_raw returns them."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "Node id, e.g. 12:345." },
                "depth": { "type": "integer", "minimum": 0, "maximum": 20, "description": "Levels of children to include. 0 = the node alone. Default 1." },
                "children_from": { "type": "integer", "minimum": 0, "description": "Skip this many children of the node: to resume a result cut for budget." },
                "geometry": { "type": "boolean", "description": "Include vector path data (SVG d strings). Default false." },
                "budget": { "type": "integer", "minimum": 1024, "maximum": 98304, "description": "Maximum result size in bytes. Default 32768." },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(read::ReadNode {
            handle: string(args, "handle")?.into(),
            id: string(args, "id")?.into(),
            limits: Limits {
                depth: optional_integer(args, "depth", 1, 0..=20)?,
                budget: budget(args)?.0,
                geometry: optional_bool(args, "geometry")?,
                children_from: optional_integer(args, "children_from", 0, 0..=usize::MAX)?,
            },
        })))
    }
}

pub struct ReadRaw;

impl Tool for ReadRaw {
    fn name(&self) -> &'static str {
        "read_raw"
    }
    fn description(&self) -> &'static str {
        "The node exactly as the file stores it: every field of its node change, including \
         the ones read_node does not model (listed there as `undecoded`). Byte fields are \
         shown as their length and a fingerprint."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "Node id, e.g. 12:345." },
                "fields": { "type": "array", "items": { "type": "string" }, "description": "Only these top-level fields. Default: all." },
                "budget": { "type": "integer", "minimum": 1024, "maximum": 98304 },
            }),
            &["handle", "id"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        // `fields` as it always was read: a list's strings, anything else none.
        let fields = args
            .get("fields")
            .and_then(Value::as_array)
            .map(|fields| fields.iter().filter_map(Value::as_str).map(str::to_owned).collect());
        Ok(Request::Query(Box::new(read::ReadRaw {
            handle: string(args, "handle")?.into(),
            id: string(args, "id")?.into(),
            fields,
            budget: budget(args)?,
        })))
    }
}
