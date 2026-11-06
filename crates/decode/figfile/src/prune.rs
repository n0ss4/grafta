//! Leaving out of a saved file what nothing in it names any more.
//!
//! Deleting a layer leaves its images and blobs behind, and a blob cannot simply
//! be cut from the archive, because nodes name blobs by position. Figma leaves
//! nothing of the kind: in every file measured from it, each image and blob is
//! named from the message, so nothing outside the message names them.
//!
//! When in doubt this keeps: a file too large is a nuisance, and a layer that
//! lost its geometry is damage. An image counts as named by any 20-byte value or
//! string in the message, whatever its field, and blobs stay as they are if the
//! message holds an index this does not know how to rewrite.

use std::collections::HashSet;

use kiwi::Value;
use schema::{may_index_blobs, Field, Wire, WireMut, WireName};

use crate::FigFile;

/// The length of an image's hash, the name its archive entry goes by.
const HASH_BYTES: usize = 20;

/// A copy of a file without what nothing names, and what that left out.
pub struct Pruned {
    pub fig: FigFile,
    pub images: usize,
    pub blobs: usize,
    /// The size of what was left out, before the archive and the message's
    /// own compression.
    pub bytes: u64,
}

/// `fig` without what nothing names, or `None` if everything in it is named, so
/// that saving a file nobody deleted from stays the same bytes.
pub fn without_unused(fig: &FigFile) -> Option<Pruned> {
    let names = Names::of(fig);
    let keep_image = |name: &str| !is_hash(name) || names.images.contains(name);
    let image_dropped = fig.images.iter().any(|(name, _)| !keep_image(name));
    let blobs_dropped = names.blobs.as_ref().is_some_and(|used| used.iter().any(|named| !named));
    if !image_dropped && !blobs_dropped {
        return None;
    }

    let images: Vec<_> = fig.images.iter().filter(|(name, _)| keep_image(name)).cloned().collect();
    let dropped_images = fig.images.len() - images.len();
    let mut bytes: u64 = fig.images.iter().filter(|(name, _)| !keep_image(name)).map(|(_, b)| b.len() as u64).sum();

    let mut message = fig.node_changes.clone();
    let mut kept_blobs = fig.blobs.clone();
    let mut dropped_blobs = 0;
    if let Some(used) = names.blobs.filter(|used| used.iter().any(|named| !named)) {
        dropped_blobs = used.iter().filter(|named| !**named).count();
        bytes += fig.blobs.iter().zip(&used).filter(|(_, named)| !**named).map(|(b, _)| b.len() as u64).sum::<u64>();
        renumber(&mut message, &used);
        let mut index = 0;
        kept_blobs.retain(|_| {
            index += 1;
            used[index - 1]
        });
    }

    Some(Pruned {
        fig: FigFile {
            product: fig.product,
            node_changes: message,
            blobs: kept_blobs,
            images,
            version: fig.version,
            schema: fig.schema.clone(),
            schema_chunk: fig.schema_chunk.clone(),
            canvas_name: fig.canvas_name.clone(),
            trailing_chunks: fig.trailing_chunks.clone(),
            other_entries: fig.other_entries.clone(),
        },
        images: dropped_images,
        blobs: dropped_blobs,
        bytes,
    })
}

/// An entry not named as a hash cannot be named by one, so it is not ours to
/// judge. Figma's hashes are lower-case; any other spelling stays too.
fn is_hash(name: &str) -> bool {
    name.len() == HASH_BYTES * 2 && name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_blob_index(field: &str) -> bool {
    field == Field::CommandsBlob.as_wire() || field == Field::VectorNetworkBlob.as_wire()
}

/// What the message names.
struct Names {
    images: HashSet<String>,
    /// For each blob, whether a node names it; `None` when they cannot be told
    /// apart, and the blobs are left as they are.
    blobs: Option<Vec<bool>>,
}

impl Names {
    fn of(fig: &FigFile) -> Self {
        let held = fig.node_changes.field(Field::Blobs).and_then(Value::as_array).map_or(0, Vec::len);
        let mut names = Self {
            images: HashSet::new(),
            // A blob the reader could not flatten would put every index after
            // it out by one.
            blobs: (held == fig.blobs.len()).then(|| vec![false; held]),
        };
        if let Value::Object(fields) = &fig.node_changes {
            for (name, value) in fields.iter().filter(|(name, _)| *name != Field::Blobs.as_wire()) {
                names.read(value, name);
            }
        }
        names
    }

    fn read(&mut self, value: &Value, field: &str) {
        match value {
            Value::Object(fields) => {
                for (name, child) in fields.iter() {
                    // Blobs of their own, whose indexes are not the message's.
                    if name == Field::Blobs.as_wire() {
                        self.blobs = None;
                    } else {
                        self.read(child, name);
                    }
                }
            }
            Value::Array(items) => items.iter().for_each(|item| self.read(item, field)),
            Value::Uint(index) if is_blob_index(field) => {
                let named = self.blobs.as_mut().and_then(|used| used.get_mut(*index as usize));
                match named {
                    Some(named) => *named = true,
                    None => self.blobs = None,
                }
            }
            Value::Uint(_) if may_index_blobs(field) => self.blobs = None,
            Value::Bytes(bytes) if bytes.len() == HASH_BYTES => {
                self.images.insert(hex(bytes));
            }
            Value::Str(text) if text.len() == HASH_BYTES * 2 => {
                self.images.insert(text.clone());
            }
            _ => {}
        }
    }
}

/// Take the unused blobs out of the message and point every index at where its
/// blob now is.
fn renumber(message: &mut Value, used: &[bool]) {
    let mut next = 0u32;
    let moved: Vec<u32> = used
        .iter()
        .map(|named| {
            let at = next;
            next += u32::from(*named);
            at
        })
        .collect();

    if let Some(Value::Array(blobs)) = message.field_mut(Field::Blobs) {
        let mut index = 0;
        blobs.retain(|_| {
            index += 1;
            used[index - 1]
        });
    }
    if let Value::Object(fields) = message {
        for (name, value) in fields.iter_mut().filter(|(name, _)| *name != Field::Blobs.as_wire()) {
            rewrite(value, name, &moved);
        }
    }
}

fn rewrite(value: &mut Value, field: &str, moved: &[u32]) {
    match value {
        Value::Object(fields) => {
            for (name, child) in fields.iter_mut() {
                rewrite(child, name, moved);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| rewrite(item, field, moved)),
        Value::Uint(index) if is_blob_index(field) => *index = moved[*index as usize],
        _ => {}
    }
}

#[cfg(test)]
mod tests;
