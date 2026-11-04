//! The ZIP layer: pulling `canvas.fig` and the embedded images out of the
//! archive a `.fig` file really is — and keeping everything else, so the
//! archive can be put back together.

use crate::FigError;

pub struct Contents {
    pub canvas: Vec<u8>,
    /// The name the canvas was stored under. Usually `canvas.fig`, but the
    /// lookup below accepts two other spellings, and writing it back under a
    /// guessed name would be a difference grafta introduced.
    pub canvas_name: String,
    pub images: Vec<(String, Vec<u8>)>,
    /// Every entry that is neither the canvas nor an image, in archive order.
    /// This is where a file's `meta.json` and `thumbnail.png` live, and it is
    /// where the bare `images/` directory entry lands. Opaque bytes: grafta
    /// does not read them and must not alter them.
    pub others: Vec<(String, Vec<u8>)>,
}

/// Extensions that mark an archive entry as something other than the canvas,
/// used only by the fallback below.
const NON_CANVAS_EXTENSIONS: [&str; 3] = [".png", ".jpg", ".json"];

/// The most an archive's entries inflate to together, whatever they claim.
/// Measured on 13 real files: 127 MB.
const MAX_ARCHIVE: u64 = 4 << 30;

pub fn read(data: &[u8]) -> Result<Contents, FigError> {
    read_within(data, MAX_ARCHIVE)
}

pub(crate) fn read_within(data: &[u8], limit: u64) -> Result<Contents, FigError> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data))?;

    // Read every entry first, then decide what each one is. The reader has
    // always needed a second look — the canvas may only be identifiable as the
    // largest candidate — and now that nothing is discarded, order matters, so
    // the entries are kept as they were found.
    let mut entries: Vec<(String, Vec<u8>)> = Vec::with_capacity(archive.len());
    let mut canvas: Option<usize> = None;
    let mut fallback: Option<usize> = None;
    let mut total = 0u64;

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let name = file.name().to_string();
        let buf = crate::compress::read_bounded(&mut file, limit - total)?
            .ok_or(FigError::TooLarge { what: "the archive", limit })?;
        total += buf.len() as u64;

        if name == "canvas.fig" || name == "canvas" {
            canvas = Some(index);
        } else if !is_image(&name)
            && is_canvas_candidate(&name)
            // Some files name the canvas entry something else entirely. The
            // canvas is by far the largest non-image entry, so the biggest
            // candidate is the best guess left.
            && fallback.is_none_or(|current| buf.len() > entries[current].1.len())
        {
            fallback = Some(index);
        }

        entries.push((name, buf));
    }

    let chosen = canvas.or(fallback).ok_or(FigError::NoCanvas)?;

    let mut canvas_name = String::new();
    let mut canvas_data = Vec::new();
    let mut images = Vec::new();
    let mut others = Vec::new();

    for (index, (name, buf)) in entries.into_iter().enumerate() {
        if index == chosen {
            canvas_name = name;
            canvas_data = buf;
        } else if let Some(image_name) = name.strip_prefix("images/").filter(|n| !n.is_empty()) {
            images.push((image_name.to_string(), buf));
        } else {
            others.push((name, buf));
        }
    }

    Ok(Contents { canvas: canvas_data, canvas_name, images, others })
}

fn is_image(name: &str) -> bool {
    name.strip_prefix("images/").is_some_and(|rest| !rest.is_empty())
}

fn is_canvas_candidate(name: &str) -> bool {
    let lower = name.to_lowercase();
    !NON_CANVAS_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// Assemble a `.fig` archive: the canvas under the name it came in under, then
/// the other entries, then the images.
///
/// **Order.** Figma's own archives run canvas, `thumbnail.png`, `meta.json`,
/// the bare `images/` directory, then the images — so canvas, then others, then
/// images reproduces the source order exactly on every file measured. It is
/// reconstructed rather than recorded, which holds as long as no file
/// interleaves an image with something else; `roundtrip.rs` asserts it against
/// the real thing rather than trusting it.
///
/// **Compression.** Every entry is stored, not deflated. Figma stores all of
/// them but `meta.json`, the canvas is already compressed chunk by chunk, and
/// the images are compressed formats — so this matches the input almost
/// everywhere and costs a few hundred bytes where it does not. Per-entry
/// compression is the second thing to vary if Figma refuses the file, after the
/// message chunk's own compression (research D2).
///
/// **Timestamps.** Fixed, because the output must be the same bytes every time
/// (I4) and the clock is not part of the document.
pub fn write(
    canvas_name: &str,
    canvas: &[u8],
    others: &[(String, Vec<u8>)],
    images: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, FigError> {
    use std::io::Write;

    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default());

    let mut entry = |name: &str, contents: &[u8]| -> Result<(), FigError> {
        // A name ending in `/` is a directory entry, not a zero-length file —
        // every `.fig` measured carries a bare `images/` and putting back a
        // file of that name would be a difference grafta introduced.
        if let Some(dir) = name.strip_suffix('/') {
            writer.add_directory(dir, options)?;
        } else {
            writer.start_file(name, options)?;
            writer.write_all(contents)?;
        }
        Ok(())
    };

    entry(canvas_name, canvas)?;
    for (name, contents) in others {
        entry(name, contents)?;
    }
    for (name, contents) in images {
        entry(&format!("images/{name}"), contents)?;
    }

    Ok(writer.finish()?.into_inner())
}
