//! A call on a file open in Figma: carried there through the
//! [`LiveFiles`](crate::port::LiveFiles) port, or refused by name.
//!
//! Nothing of such a file is held here, so nothing is journaled or rolled
//! back: one call is one undo step in Figma, which saves the file itself.

use serde_json::Value;

use crate::data::from_json;
use crate::app::data::Data;
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
    let answer = kit.ports().live.call(handle, tool.name(), &from_json(&carried)).map_err(unanswered)?;
    picture(answer).map_err(|error| unanswered(error).into())
}

/// What `set_properties` does on a file open in Figma, unlike on a file.
pub const SET_PROPERTIES: &str = "Figma makes the change and saves it, so the result is `{ id, set }` with no \
     `unsaved_edits`, and one call is one undo step there. Not built there yet, each refused by name with \
     nothing changed: runs, paragraphs, componentPropertyReferences, componentProperties, reactions and the \
     other prototype keys, an image by path (give its imageHash), and Slides' and FigJam's keys";

/// The mime types a picture from Figma may be.
const PICTURES: [&str; 2] = ["image/png", "image/jpeg"];

/// An answer that is a picture, `{ image: { mime, base64 }, caption }`, as one;
/// any other as the data it is.
fn picture(answer: Data) -> Result<Output, LiveError> {
    let Some(image) = answer.get("image") else { return Ok(Output::Data(answer)) };
    let given = image.get("mime").and_then(Data::as_str).unwrap_or_default();
    let mime = PICTURES.into_iter().find(|m| *m == given);
    let bytes = image.get("base64").and_then(Data::as_str).and_then(base64);
    match (mime, bytes) {
        (Some(mime), Some(bytes)) => Ok(Output::Image { mime, bytes, caption: answer.get("caption").cloned() }),
        _ => Err(LiveError::Refused(format!("the panel sent a picture grafta cannot read: a {given:?} not in base64 PNG or JPEG"))),
    }
}

/// Standard base64 back to bytes; `None` for anything else.
fn base64(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let text = text.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for chunk in text.chunks(4) {
        let digits: Vec<u8> = chunk.iter().map(|&c| value(c)).collect::<Option<_>>()?;
        let n = digits.iter().enumerate().fold(0u32, |n, (i, d)| n | (u32::from(*d) << (18 - 6 * i)));
        let take = match digits.len() {
            4 => 3,
            3 => 2,
            2 => 1,
            _ => return None,
        };
        out.extend(n.to_be_bytes()[1..=take].iter());
    }
    Some(out)
}

#[cfg(test)]
mod tests;
