//! A new, empty file: the tool's edge half.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::args::{optional_string, string};
use super::schema::schema;
use super::{Request, Tool};
use crate::tools::new_file;
use crate::ToolError;

pub struct NewFile;

impl Tool for NewFile {
    fn name(&self) -> &'static str {
        "new_file"
    }
    fn description(&self) -> &'static str {
        "Create a new, empty .fig at a path that does not exist, and open it: one page and \
         nothing else. `from` is any .fig you have; only its format is used (Figma's schema, \
         which grafta does not invent), none of its content: no pages, components, styles, \
         variables or images. Returns the handle, as open_file does."
    }
    fn input_schema(&self) -> Value {
        schema(
            json!({
                "path": { "type": "string", "description": "A new path ending in .fig." },
                "from": { "type": "string", "description": "Any .fig, whose format the new file uses." },
                "page": { "type": "string", "description": "The page's name. Default \"Page 1\"." },
            }),
            &["path", "from"],
        )
    }
    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let path = Path::new(string(args, "path")?);
        let from = PathBuf::from(string(args, "from")?);
        let page = optional_string(args, "page")?.unwrap_or("Page 1").to_owned();
        Ok(Request::Command(Box::new(new_file::NewFile { path: path.into(), from, page })))
    }
}
