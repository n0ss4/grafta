#![deny(clippy::all)]
// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![cfg_attr(test, allow(clippy::disallowed_methods))]
#![allow(clippy::missing_errors_doc)]

//! Reads a `.fig` file down to decoded data.
//!
//! A `.fig` is a ZIP archive; inside it sits `canvas.fig`, a `fig-kiwi`
//! container of length-prefixed chunks. The first chunk is a deflated Kiwi
//! schema, the second the message that schema describes. This crate stops at
//! that message — turning it into a scene graph is the `model` crate's job.
//!
//! ```no_run
//! let data = std::fs::read("design.fig")?;
//! let fig = figfile::parse_fig(&data)?;
//! println!("version {}, {} images", fig.version, fig.images.len());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Writing
//!
//! [`write_fig`] puts a `FigFile` back into the bytes of a `.fig`, and
//! [`compare`] says where two of them first differ. Two rules govern the write
//! path, and both exist because the format is proprietary and a refusal by
//! Figma carries no diagnostic:
//!
//! **The schema is carried, never authored.** Figma ships a new schema inside
//! every file, so the description of the message's shape is read out of the
//! file rather than invented. The chunk goes back exactly as it came in.
//! Synthesising one would mean inventing field numbering, and any drift is
//! undetectable until the file fails to open.
//!
//! **What is not interpreted is preserved verbatim.** Chunks past the message,
//! archive entries that are neither canvas nor image, and the name the canvas
//! was stored under are all carried through as opaque bytes. A reader that
//! drops what it cannot read is a reader that cannot write the file back —
//! and giving structure to a region nobody has read would be guessing at a
//! format, which is the mistake this crate exists to avoid.
//!
//! Writing is deterministic: the same input produces the same bytes. Nothing is
//! normalised, repaired or tidied in transit — a round trip that improves the
//! file is not a round trip.

mod archive;
mod compress;
mod container;
mod error;
mod write;

pub use error::FigError;
pub use write::{compare, write_canvas, write_fig, Difference};

use kiwi::{Schema, Value};
use schema::{Field, Wire};

/// Everything a `.fig` file yields before any interpretation.
pub struct FigFile {
    /// Which Figma product wrote it: Design (`.fig`), Slides (`.deck`) or
    /// FigJam (`.jam`), from the container's magic.
    pub product: Product,
    /// The decoded top-level `Message`, whose `nodeChanges` field holds the
    /// document.
    pub node_changes: Value,
    /// Binary blobs the message references by index — vector geometry, mostly.
    pub blobs: Vec<Vec<u8>>,
    /// Embedded images, keyed by the hash-like name the archive stores them
    /// under.
    pub images: Vec<(String, Vec<u8>)>,
    /// Container format version from the `fig-kiwi` header.
    pub version: u32,
    /// The schema the message was decoded against. Figma ships a new one with
    /// every file, so this is the only description of the message's shape.
    pub schema: Schema,

    // ── Preserved, never interpreted ──────────────────────────────────────
    //
    // The three fields below are the parts of a file grafta does not read.
    // They are raw bytes and they stay raw bytes: giving them structure would
    // mean guessing at a format nobody here has read. A writer must put them
    // back unaltered and in order.
    /// The schema chunk exactly as it sat in the container, still deflated.
    ///
    /// The parsed `schema` above is what grafta reads the message with; this is
    /// what a writer puts back. Carrying it verbatim is research D1: the
    /// hardest-looking unknown in writing a proprietary format — *what shape
    /// does Figma expect* — is answered by a description Figma itself shipped
    /// inside the file, so the encoder only has to be faithful, never
    /// clairvoyant. Re-authoring it would invent field numbering, and any drift
    /// is undetectable until the file fails to open.
    pub schema_chunk: Vec<u8>,
    /// The archive entry the canvas was stored under — `canvas.fig` in every
    /// file measured, but the reader accepts two other spellings and a name
    /// grafta guessed would be a difference grafta introduced.
    pub canvas_name: String,
    /// Container chunks after the message, in order. No file measured carries
    /// any (research D8); the field exists so that one which does is not
    /// quietly truncated.
    pub trailing_chunks: Vec<Vec<u8>>,
    /// Archive entries that are neither the canvas nor an image, in archive
    /// order — `meta.json`, `thumbnail.png`, and the bare `images/` directory
    /// entry, in every file measured.
    pub other_entries: Vec<(String, Vec<u8>)>,
}

/// The name a canvas is stored under when there is no archive to say
/// otherwise. Every fixture measured uses it (research D8).
pub const DEFAULT_CANVAS_NAME: &str = "canvas.fig";

/// The Figma product a file is from. Their files share one container and one
/// schema format; only the magic and the node types inside differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Product {
    #[default]
    Design,
    Slides,
    Whiteboard,
    Site,
}

impl Product {
    /// Every product, Design first.
    pub const ALL: [Product; 4] = [Product::Design, Product::Slides, Product::Whiteboard, Product::Site];

    /// The container's eight-byte magic.
    #[must_use]
    pub fn magic(self) -> &'static [u8; 8] {
        match self {
            Product::Design => b"fig-kiwi",
            Product::Slides => b"fig-deck",
            Product::Whiteboard => b"fig-jam.",
            Product::Site => b"fig-site",
        }
    }

    #[must_use]
    pub fn from_magic(bytes: &[u8]) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.magic().as_slice() == bytes)
    }

    /// The file's extension, as Figma saves a local copy.
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Product::Design => "fig",
            Product::Slides => "deck",
            Product::Whiteboard => "jam",
            Product::Site => "site",
        }
    }

    /// The product by a file's extension, for a path to write.
    #[must_use]
    pub fn from_extension(extension: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.extension().eq_ignore_ascii_case(extension))
    }
}

/// Parse a `.fig` file from its bytes.
pub fn parse_fig(data: &[u8]) -> Result<FigFile, FigError> {
    let archive::Contents { canvas, canvas_name, images, others } = archive::read(data)?;
    let mut fig = parse_canvas(&canvas, images)?;
    fig.canvas_name = canvas_name;
    fig.other_entries = others;
    Ok(fig)
}

/// Parse an already-extracted `canvas.fig` payload. Useful when the container
/// came from somewhere other than a ZIP archive.
///
/// There being no archive, there is nothing to say what the canvas was named or
/// what sat beside it: `canvas_name` takes [`DEFAULT_CANVAS_NAME`] and
/// `other_entries` is empty. [`parse_fig`] fills both in from the archive it
/// read them out of.
pub fn parse_canvas(canvas: &[u8], images: Vec<(String, Vec<u8>)>) -> Result<FigFile, FigError> {
    let container = container::parse(canvas)?;

    let schema_bytes = compress::inflate(container.schema_chunk)?;
    let schema = kiwi::decode_binary_schema(&schema_bytes)?;

    let data_bytes = compress::decompress(container.message_chunk)?;
    let node_changes = kiwi::decode_message("Message", &data_bytes, &schema)?;

    let blobs = extract_blobs(&node_changes);

    Ok(FigFile {
        product: container.product,
        node_changes,
        blobs,
        images,
        version: container.version,
        schema,
        schema_chunk: container.schema_chunk.to_vec(),
        canvas_name: DEFAULT_CANVAS_NAME.to_string(),
        trailing_chunks: container.trailing_chunks.iter().map(|chunk| chunk.to_vec()).collect(),
        other_entries: Vec::new(),
    })
}

/// The message's `blobs` array, flattened to the raw byte strings the scene
/// graph indexes into.
fn extract_blobs(message: &Value) -> Vec<Vec<u8>> {
    let Some(array) = message.field(Field::Blobs).and_then(Value::as_array) else {
        return Vec::new();
    };
    array
        .iter()
        .filter_map(|blob| match blob.field(Field::Bytes)? {
            Value::Bytes(bytes) => Some(bytes.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;
