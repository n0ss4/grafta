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
    if let Some(made) = tool.answer_from_figma(kit, args, &answer) {
        return made;
    }
    picture(answer).map_err(|error| unanswered(error).into())
}

/// What `set_properties` does on a file open in Figma, unlike on a file.
pub const SET_PROPERTIES: &str = "Figma makes the change and saves it, so the result is `{ id, set }` with no \
     `unsaved_edits`, and one call is one undo step there. Not built there yet, each refused by name with \
     nothing changed: componentPropertyReferences, reactions and the other prototype keys, an image by path \
     (give its imageHash), and Slides' and FigJam's keys. Figma drops the own opacity of a paint bound to a \
     variable when it resolves the paint again for another mode (explicitVariableModes on the layer or a parent): \
     for a translucent token, give the variable a colour with alpha";

/// What a tool that writes does on a file open in Figma, unlike on a file.
pub const WRITTEN: &str = "Figma makes the change and saves it, so the result has no `unsaved_edits`, and one \
     call is one undo step there; what Figma refuses, a plan's limit included, is said in its words";

/// What `create_instance` does there.
pub const CREATE_INSTANCE: &str = "Figma makes the instance and saves it, so the result has no `unsaved_edits`; \
     a library's component is placed from its node in the file, as Figma allows, and placing one the file does not \
     yet use is not measured (research O1)";

/// What `create_text` does there.
pub const CREATE_TEXT: &str = "Figma makes the text and lays it out, so `size` is Figma's own and not provisional, \
     and the result has no `unsaved_edits`; with no fontFamily, the font and size most used by the current page's \
     texts (Figma's default when it has none), and Figma's own line height when none is given";

/// What `create_vector` does there.
pub const CREATE_VECTOR: &str = "Figma reads the path data as SVG and joins the paths into one vector, whose box \
     is its drawing's bounds, placed where the drawing falls in the asked box; the result has no `unsaved_edits`";

/// What `check_contrast` does there.
pub const CHECK_CONTRAST: &str = "the background is Figma's own drawing: the panel copies the node, makes the copy's \
     texts transparent, has Figma draw it and deletes the copy, and grafta reads each text's box in that picture as on a file. \
     Figma resolves the colours in the node's modes; the copy is one more step in Figma's undo, after which the file \
     is as it was";

/// What `delete_node` does there.
pub const DELETE_NODE: &str = "Figma deletes and saves it, so the result has no `unsaved_edits`, and one call is \
     one undo step there; it is refused while an instance outside it uses a component in it, and a prototype link or \
     an override pointing into it is left to Figma";

/// What `summarize` does there.
pub const SUMMARIZE: &str = "counts a page at a time for 2 s: past that, the totals are for the pages listed and \
     the rest are named under `omitted`; `images` and `prototype` are not counted there, and there is no \
     `unsaved_edits`";

/// The mime types a picture from Figma may be: as on a file, what an image
/// fill holds.
const PICTURES: [&str; 4] = ["image/png", "image/jpeg", "image/gif", "image/webp"];

/// An answer that is a picture, `{ image: { mime, base64 }, caption }`, as one;
/// any other as the data it is.
fn picture(answer: Data) -> Result<Output, LiveError> {
    let Some(image) = answer.get("image") else { return Ok(Output::Data(answer)) };
    let given = image.get("mime").and_then(Data::as_str).unwrap_or_default();
    let mime = PICTURES.into_iter().find(|m| *m == given);
    let bytes = image.get("base64").and_then(Data::as_str).and_then(base64);
    match (mime, bytes) {
        (Some(mime), Some(bytes)) => Ok(Output::Image { mime, bytes, caption: answer.get("caption").cloned() }),
        _ => Err(LiveError::Refused(format!("the panel sent a picture grafta cannot read: a {given:?} not in base64 PNG, JPEG, GIF or WebP"))),
    }
}

/// Standard base64 back to bytes; `None` for anything else.
pub(crate) fn base64(text: &str) -> Option<Vec<u8>> {
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
