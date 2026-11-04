//! Images as fills.
//!
//! An image fill is a paint of type `IMAGE` carrying the same 16 fields on all
//! 14,730 measured, and the image is an archive entry named by the SHA-1 of its
//! bytes, as all 1,077 embedded are. A model names a PNG or JPEG on disk, or an
//! image the file already holds by its hash. A file is read when the call is
//! checked and embedded only when it succeeds, so a refused call adds nothing.

use std::path::Path;

use kiwi::Value as Raw;
use schema::{object, BlendMode, Field, ImageScaleMode, PaintType, WireName};
use crate::data::{Data, Map};
use sha1::{Digest, Sha1};

use super::style::{invalid, unaccepted};
use crate::error::ImageError;
use crate::{Session, ToolError};

/// The scale modes an image paint takes.
pub const SCALE_MODES: [ImageScaleMode; 3] = [ImageScaleMode::Fill, ImageScaleMode::Fit, ImageScaleMode::Stretch];

/// PNG or JPEG, and its size in pixels, from the bytes themselves.
fn format_and_size(bytes: &[u8]) -> Option<(&'static str, u32, u32)> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
        let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
        return Some(("png", w, h));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // The first start-of-frame marker carries the size.
        let mut at = 2;
        while at + 9 < bytes.len() {
            if bytes[at] != 0xFF {
                at += 1;
                continue;
            }
            let marker = bytes[at + 1];
            let length = usize::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
            if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
                let h = u32::from(u16::from_be_bytes([bytes[at + 5], bytes[at + 6]]));
                let w = u32::from(u16::from_be_bytes([bytes[at + 7], bytes[at + 8]]));
                return Some(("jpeg", w, h));
            }
            at += 2 + length;
        }
    }
    None
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn embedded<'a>(session: &'a Session, hash: &str) -> Option<&'a [u8]> {
    session
        .fig
        .images
        .iter()
        .chain(&session.pending_images)
        .find(|(name, _)| name == hash)
        .map(|(_, bytes)| bytes.as_slice())
}

/// Read every image a call's fills and strokes name by path, holding them for
/// `commit`, and name them by hash in what the paints are built from.
pub fn stage(session: &mut Session, properties: &Map) -> Result<Map, ToolError> {
    session.pending_images.clear();
    let mut out = properties.clone();
    for key in ["fills", "strokes"] {
        let Some(Data::List(paints)) = out.get_mut(key) else { continue };
        for (index, paint) in paints.iter_mut().enumerate() {
            let Some(paint) = paint.as_map_mut() else { continue };
            if paint.get("type").and_then(Data::as_str) != Some(PaintType::Image.as_wire()) {
                continue;
            }
            let Some(path) = paint.get("image").and_then(Data::as_str).map(str::to_string) else { continue };
            let at = format!("{key}[{index}].image");
            let bytes = session
                .ports
                .images
                .read(std::path::Path::new(&path))
                .map_err(|error| ImageError::CannotRead { at: at.clone(), path: path.clone(), error: error.to_string() })?;
            if format_and_size(&bytes).is_none() {
                return Err(ImageError::NotAnImage { at, path }.into());
            }
            let hash = sha1_hex(&bytes);
            if embedded(session, &hash).is_none() {
                session.pending_images.push((hash.clone(), bytes));
            }
            let name = Path::new(&path).file_stem().map_or_else(|| hash.clone(), |s| s.to_string_lossy().into_owned());
            paint.remove("image");
            paint.insert("imageHash".into(), Data::from(hash));
            paint.entry("imageName".into()).or_insert(Data::from(name));
        }
    }
    Ok(out)
}

/// Embed what `stage` read, once the call has succeeded: in the file, and in
/// the graph's copy `render_node` draws from.
pub fn commit(session: &mut Session) {
    for (hash, bytes) in std::mem::take(&mut session.pending_images) {
        if !session.graph.images.iter().any(|(name, _)| *name == hash) {
            session.graph.images.push((hash.clone(), bytes.clone()));
        }
        if !session.fig.images.iter().any(|(name, _)| *name == hash) {
            session.fig.images.push((hash, bytes));
        }
    }
}

/// An image paint in the 16 fields all measured ones carry. `FILL` as it most
/// often is (scale 1, an identity transform: 8,394); `FIT` and `STRETCH` with
/// the scale theirs carry (0.5). The thumbnail is the image itself (5,365).
pub fn paint(session: &Session, at: &str, paint: &Map) -> Result<Raw, ToolError> {
    if let Some(unknown) = paint.keys().find(|k| !matches!(k.as_str(), "type" | "image" | "imageHash" | "imageName" | "scaleMode" | "opacity")) {
        return Err(unaccepted(
            at,
            unknown,
            "an IMAGE paint takes image (a PNG or JPEG path) or imageHash, scaleMode and opacity",
        ));
    }
    let hash = paint.get("imageHash").and_then(Data::as_str).ok_or_else(|| ImageError::NoImage { at: at.into() })?;
    let bytes = embedded(session, hash).ok_or_else(|| ImageError::NotEmbedded { at: at.into(), hash: hash.into() })?;
    let (_, width, height) =
        format_and_size(bytes).ok_or_else(|| ImageError::Unsizable { at: at.into(), hash: hash.into() })?;
    let digest: Vec<u8> = (0..hash.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hash.get(i..i + 2).unwrap_or("zz"), 16))
        .collect::<Result<_, _>>()
        .map_err(|_| invalid(&format!("{at}.imageHash"), "40 hex digits", hash))?;
    let named = paint.get("scaleMode").and_then(Data::as_str).unwrap_or(ImageScaleMode::Fill.as_wire());
    let Some(mode) = ImageScaleMode::listed(named, &SCALE_MODES) else {
        return Err(invalid(&format!("{at}.scaleMode"), "FILL, FIT or STRETCH", named));
    };
    let opacity = match paint.get("opacity") {
        Some(o) => {
            o.as_f64().filter(|o| (0.0..=1.0).contains(o)).ok_or_else(|| invalid(&format!("{at}.opacity"), "a number from 0 to 1", o))?
                as f32
        }
        None => 1.0,
    };
    let name = paint.get("imageName").and_then(Data::as_str).unwrap_or(hash).to_string();
    let reference = || {
        object([
            (Field::Hash, Raw::Bytes(digest.clone())),
            (Field::Name, Raw::Str(name.clone())),
        ])
    };
    let identity = object(
        [(Field::M00, 1.0), (Field::M01, 0.0), (Field::M02, 0.0), (Field::M10, 0.0), (Field::M11, 1.0), (Field::M12, 0.0)].map(|(k, v)| (k, Raw::Float(v))),
    );
    Ok(object([
        (Field::Type, PaintType::Image.into()),
        (Field::Opacity, Raw::Float(opacity)),
        (Field::Visible, Raw::Bool(true)),
        (Field::BlendMode, Raw::from(BlendMode::Normal)),
        (Field::Transform, identity),
        (Field::Image, reference()),
        (Field::ImageThumbnail, reference()),
        (Field::AnimationFrame, Raw::Uint(0)),
        (Field::ImageScaleMode, mode.into()),
        (Field::ImageShouldColorManage, Raw::Bool(false)),
        (Field::Rotation, Raw::Float(0.0)),
        (Field::Scale, Raw::Float(if mode == ImageScaleMode::Fill { 1.0 } else { 0.5 })),
        (Field::OriginalImageWidth, Raw::Uint(width)),
        (Field::OriginalImageHeight, Raw::Uint(height)),
        (Field::ThumbHash, Raw::Bytes(Vec::new())),
        (Field::AltText, Raw::Str(String::new())),
    ]))
}
