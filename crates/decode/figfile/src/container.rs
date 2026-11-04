//! The `fig-kiwi` container: an 8-byte magic, a version, then a run of
//! length-prefixed chunks. Figma Slides (`fig-deck`) and FigJam (`fig-jam.`)
//! files are the same container under their own magic: the magic says
//! which product wrote the file, and is written back as it was read.

use crate::{FigError, Product};

/// Magic plus the four version bytes.
const HEADER_LEN: usize = 12;

#[derive(Debug)]
pub struct Container<'a> {
    pub product: Product,
    pub version: u32,
    /// Chunk 0 — the deflated Kiwi schema.
    pub schema_chunk: &'a [u8],
    /// Chunk 1 — the message that schema describes.
    pub message_chunk: &'a [u8],
    /// Chunks 2..n, in order. grafta does not interpret these and no file
    /// measured carries any (research D8), but a reader that drops what it
    /// cannot read is a reader that cannot write the file back.
    pub trailing_chunks: Vec<&'a [u8]>,
}

pub fn parse(canvas: &[u8]) -> Result<Container<'_>, FigError> {
    if canvas.len() < HEADER_LEN {
        return Err(FigError::Truncated);
    }
    let product = Product::from_magic(&canvas[..8]).ok_or(FigError::InvalidMagic)?;
    let version = u32::from_le_bytes(
        canvas[8..HEADER_LEN].try_into().expect("four bytes after the magic"),
    );

    let mut chunks: Vec<&[u8]> = Vec::new();
    let mut offset = HEADER_LEN;
    while offset + 4 <= canvas.len() {
        let len = u32::from_le_bytes(
            canvas[offset..offset + 4].try_into().expect("four length bytes"),
        ) as usize;
        offset += 4;
        let end = offset.checked_add(len).ok_or(FigError::Truncated)?;
        if end > canvas.len() {
            return Err(FigError::Truncated);
        }
        chunks.push(&canvas[offset..end]);
        offset = end;
    }

    let [schema_chunk, message_chunk, trailing @ ..] = &chunks[..] else {
        return Err(FigError::NotEnoughChunks);
    };
    Ok(Container {
        product,
        version,
        schema_chunk,
        message_chunk,
        trailing_chunks: trailing.to_vec(),
    })
}

/// Assemble a container: the magic, the version, then every chunk behind a
/// little-endian `u32` length — schema first, message second, and whatever
/// followed them in the file after that, in order.
pub fn write(product: Product, version: u32, chunks: &[&[u8]]) -> Vec<u8> {
    let capacity = HEADER_LEN + chunks.iter().map(|chunk| chunk.len() + 4).sum::<usize>();
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(product.magic());
    out.extend_from_slice(&version.to_le_bytes());
    for chunk in chunks {
        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out
}
