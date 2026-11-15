//! A call on a file open in Figma: carried there through the
//! [`LiveFiles`](crate::port::LiveFiles) port, or refused by name.
//!
//! Nothing of such a file is held here, so nothing is journaled or rolled
//! back: one call is one undo step in Figma, which saves the file itself.

use serde_json::Value;

use crate::data::from_json;
use crate::port::LiveError;
use crate::{DomainError, Live, Output, Tool, ToolError, Toolkit};

/// Carry `tool`'s call to the file `handle` names, its arguments read first as
/// on a file, so that what a file would refuse is refused here too.
pub fn call(kit: &Toolkit, tool: &dyn Tool, handle: &str, args: &Value) -> Result<Output, ToolError> {
    tool.parse(args)?;
    let unanswered = |error| DomainError::Live { handle: handle.into(), tool: tool.name(), error };
    if !kit.ports().live.list().iter().any(|file| file.handle.as_str() == handle) {
        return Err(unanswered(LiveError::NotOpen).into());
    }
    if let Live::Cannot(why) = tool.live() {
        return Err(DomainError::NotInFigma { tool: tool.name(), why }.into());
    }
    let mut carried = args.clone();
    if let Some(fields) = carried.as_object_mut() {
        fields.remove("handle");
    }
    kit.ports()
        .live
        .call(handle, tool.name(), &from_json(&carried))
        .map(Output::Data)
        .map_err(|error| unanswered(error).into())
}
