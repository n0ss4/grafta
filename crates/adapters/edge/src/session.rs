//! Opening, listing and closing files: the tools' edge halves.

use serde_json::{json, Value};

use super::args::string;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::tools::session;
use crate::{DomainError, ToolError};

pub struct ListOpen;

impl Tool for ListOpen {
    fn name(&self) -> &'static str {
        "list_open"
    }
    fn description(&self) -> &'static str {
        "List the files currently open, with their handles, node counts, unsaved edits, how \
         long each took to open, and the process's memory."
    }
    fn input_schema(&self) -> Value {
        schema(json!({}), &[])
    }
    fn parse(&self, _args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Query(Box::new(session::ListOpen)))
    }
}

pub struct OpenFile;

impl Tool for OpenFile {
    fn name(&self) -> &'static str {
        "open_file"
    }
    fn description(&self) -> &'static str {
        "Open a Figma .fig file and keep it open for later calls. Returns a handle and a \
         summary: pages, node counts, components, variable collections, images. Opening a \
         path that is already open returns the same handle without reading it again. \
         Large files take seconds to open; every later call on the handle is fast. Edits the \
         file had not saved when grafta last stopped are replayed, and `recovered` says how \
         many; edits all saved to another file are not, and it says which file."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "path": { "type": "string", "description": "Path to a .fig file." },
                "recover": { "type": "boolean", "description": "Replay edits this file had not saved when grafta last stopped. Default true; false discards them." },
            }),
            &["path"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let path = string(args, "path")?.into();
        let recover = match args.get("recover") {
            None | Some(Value::Null) => true,
            Some(Value::Bool(b)) => *b,
            Some(other) => {
                return Err(DomainError::Expected {
                    parameter: "recover".into(),
                    expected: "true or false".into(),
                    got: other.to_string(),
                }
                .into())
            }
        };
        Ok(Request::Command(Box::new(session::OpenFile { path, recover })))
    }
}

pub struct CloseFile;

impl Tool for CloseFile {
    fn name(&self) -> &'static str {
        "close_file"
    }
    fn description(&self) -> &'static str {
        "Close an open file and free the memory it holds. Edits not saved with save_as are \
         discarded, for good: they are not recovered on the next open. The result says how \
         many there were."
    }
    fn input_schema(&self) -> Value {
        schema(json!({ "handle": handle_property() }), &["handle"])
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        Ok(Request::Command(Box::new(session::CloseFile { handle: string(args, "handle")?.into() })))
    }
}
