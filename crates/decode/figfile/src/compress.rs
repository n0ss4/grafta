//! Chunk decompression. Figma has written the message chunk with both zstd and
//! raw deflate over the years, and older files leave it uncompressed.

use std::io::Read;

use flate2::read::{DeflateDecoder, DeflateEncoder};

use crate::FigError;

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// How hard the message is compressed. Measured on real messages: smaller than
/// Figma's own chunk and faster than deflate was. Level 19 saves a further 20%,
/// in 44 times as long. Fixed, because the output has to be the same bytes every
/// time (I4).
const ZSTD_LEVEL: i32 = 9;

/// The most a schema inflates to. Measured on 13 real files: 75 KB.
const MAX_SCHEMA: u64 = 16 << 20;
/// The most a message inflates to. Measured on 13 real files: 229 MB.
const MAX_MESSAGE: u64 = 2 << 30;
/// The most a message inflates to, for its packed size. Measured on 13 real
/// files: 14.3 times. zstd has no bound of its own, so this is what stops a
/// few bytes reaching [`MAX_MESSAGE`]; a small message always has this much.
const MESSAGE_RATIO: u64 = 256;
const MESSAGE_FLOOR: u64 = 1 << 20;

/// All of `reader`, or `None` past `limit`, whatever it claims about itself.
pub fn read_bounded(reader: impl Read, limit: u64) -> Result<Option<Vec<u8>>, std::io::Error> {
    let mut out = Vec::new();
    reader.take(limit.saturating_add(1)).read_to_end(&mut out)?;
    Ok((out.len() as u64 <= limit).then_some(out))
}

/// The schema chunk, always deflated.
pub fn inflate(data: &[u8]) -> Result<Vec<u8>, FigError> {
    read_bounded(DeflateDecoder::new(data), MAX_SCHEMA)
        .map_err(|e| FigError::Deflate(e.to_string()))?
        .ok_or(FigError::TooLarge { what: "the schema", limit: MAX_SCHEMA })
}

/// The message as a chunk: zstd, the format every file measured from Figma
/// stores it in, unless that packs it past what [`decompress`] takes back.
///
/// Repeated layers reach thousands of times in zstd, where deflate stops near
/// 250, within the limit. A file too repetitive for zstd gets deflate, which
/// Figma has also written and opens from grafta.
pub fn message_chunk(message: &[u8]) -> Result<Vec<u8>, FigError> {
    let packed = ::zstd::bulk::compress(message, ZSTD_LEVEL).map_err(|e| FigError::Zstd(e.to_string()))?;
    if message.len() as u64 <= message_limit(packed.len()) {
        return Ok(packed);
    }
    deflate(message).map_err(|e| FigError::Deflate(e.to_string()))
}

/// Raw deflate. The compression level is fixed, for the same reason as zstd's.
pub fn deflate(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut out = Vec::new();
    DeflateEncoder::new(data, flate2::Compression::default()).read_to_end(&mut out)?;
    Ok(out)
}

/// The most a message of this packed size inflates to.
fn message_limit(packed: usize) -> u64 {
    (packed as u64).saturating_mul(MESSAGE_RATIO).saturating_add(MESSAGE_FLOOR).min(MAX_MESSAGE)
}

/// Decompress a chunk, sniffing the format. A chunk that is neither zstd nor
/// deflate is already plain data; one that inflates past its limit is not.
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, FigError> {
    let limit = message_limit(data.len());
    let too_large = FigError::TooLarge { what: "the message", limit };
    if data.starts_with(&ZSTD_MAGIC) {
        let decoder = ruzstd::StreamingDecoder::new(data).map_err(|e| FigError::Zstd(e.to_string()))?;
        return read_bounded(decoder, limit).map_err(|e| FigError::Zstd(e.to_string()))?.ok_or(too_large);
    }
    match read_bounded(DeflateDecoder::new(data), limit) {
        Ok(Some(out)) => Ok(out),
        Ok(None) => Err(too_large),
        Err(_) => Ok(data.to_vec()),
    }
}
