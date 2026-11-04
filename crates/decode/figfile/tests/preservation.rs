//! What the reader keeps must be what the file held.
//!
//! Everything grafta does not interpret — the canvas entry's name, the chunks
//! past the message, the archive entries that are neither canvas nor image —
//! is compared here against the same region read straight out of the source
//! archive. Byte equality is the wrong bar for a whole file (research D6) and
//! the only right one here: preserving a region means it comes back identical.
//!
//! The expectations are recomputed from the raw bytes rather than taken from
//! the parser, so the test is a second opinion and not an echo.
//!
//! Skipped when no fixture is available — see `fixtures/README.md`.

use std::io::Read;

/// The archive as the ZIP itself reports it: every entry, in order.
fn entries(data: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(data)).expect("a ZIP archive");
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).expect("an entry");
            let name = file.name().to_string();
            let mut buf = Vec::new();
            file.read_to_end(&mut buf).expect("a readable entry");
            (name, buf)
        })
        .collect()
}

/// The container's chunks, by the same length-prefix walk the parser does.
fn chunks(canvas: &[u8]) -> Vec<&[u8]> {
    let mut chunks = Vec::new();
    let mut offset = 12; // magic plus version
    while offset + 4 <= canvas.len() {
        let len =
            u32::from_le_bytes(canvas[offset..offset + 4].try_into().expect("four bytes")) as usize;
        offset += 4;
        chunks.push(&canvas[offset..offset + len]);
        offset += len;
    }
    chunks
}

#[test]
fn every_preserved_region_is_byte_identical_to_the_source() {
    let mut checked = 0;

    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let Some(data) = testkit::fixture::bytes(&name) else { continue };
        let Some(fig) = testkit::fixture::parsed(&name) else { continue };
        checked += 1;

        let entries = entries(&data);

        // The canvas is whichever entry the reader named, and the name has to
        // be one the archive actually used.
        let (_, canvas) = entries
            .iter()
            .find(|(entry, _)| *entry == fig.canvas_name)
            .unwrap_or_else(|| panic!("{name}: no entry named `{}`", fig.canvas_name));

        let chunks = chunks(canvas);
        assert_eq!(
            fig.trailing_chunks.len(),
            chunks.len().saturating_sub(2),
            "{name}: kept a different number of chunks than the container holds"
        );
        for (index, (kept, source)) in fig.trailing_chunks.iter().zip(&chunks[2..]).enumerate() {
            assert_eq!(kept, source, "{name}: chunk {} differs from the source", index + 2);
        }

        // Everything that is not the canvas and not an image, in archive order.
        let expected: Vec<&(String, Vec<u8>)> = entries
            .iter()
            .filter(|(entry, _)| *entry != fig.canvas_name)
            .filter(|(entry, _)| entry.strip_prefix("images/").is_none_or(|r| r.is_empty()))
            .collect();

        assert_eq!(
            fig.other_entries.len(),
            expected.len(),
            "{name}: kept {} entries, the archive holds {}",
            fig.other_entries.len(),
            expected.len()
        );
        for (kept, source) in fig.other_entries.iter().zip(&expected) {
            assert_eq!(kept.0, source.0, "{name}: entry names differ, or are out of order");
            assert_eq!(
                kept.1, source.1,
                "{name}: entry `{}` came back with different bytes",
                kept.0
            );
        }

        // Images were already faithful; asserted here so a regression in the
        // part that worked does not hide behind the part that did not.
        let images: Vec<(String, Vec<u8>)> = entries
            .iter()
            .filter(|(entry, _)| *entry != fig.canvas_name)
            .filter_map(|(entry, buf)| {
                let stripped = entry.strip_prefix("images/").filter(|r| !r.is_empty())?;
                Some((stripped.to_string(), buf.clone()))
            })
            .collect();
        assert_eq!(fig.images, images, "{name}: images differ from the source");
    }

    if checked == 0 {
        eprintln!("no fixtures available — see fixtures/README.md");
    }
}
