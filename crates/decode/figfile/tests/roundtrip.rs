//! Reading a file, writing it back, and reading it again.
//!
//! This is everything about the round trip that grafta
//! can answer on its own. What it cannot answer is whether Figma opens the
//! result, and a green run here is not that.
//!
//! The fixture-backed tests skip when `fixtures/` is empty; the synthetic ones
//! below run everywhere and cover the cases a real file does not contain.

// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![allow(clippy::disallowed_methods)]

use std::io::Read;

use testkit::model;

// ── Against real files ───────────────────────────────────────────────────────

/// Figma leaves nothing unnamed in a file, in any of its products, so saving one
/// nobody deleted from leaves nothing out.
#[test]
fn a_file_from_figma_has_nothing_to_leave_out() {
    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let Some(original) = testkit::fixture::parsed(&name) else { continue };
        if let Some(pruned) = figfile::without_unused(&original) {
            panic!("{name}: {} images and {} blobs would be left out", pruned.images, pruned.blobs);
        }
    }
}

/// Round-trip a fixture and compare what grafta understands: first the
/// document, then the scene graph built from it.
#[test]
fn a_real_file_survives_a_write_and_a_read() {
    let mut checked = 0;

    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let Some(original) = testkit::fixture::parsed(&name) else { continue };
        checked += 1;

        let written = figfile::write_fig(&original).unwrap_or_else(|e| panic!("{name}: {e}"));
        let reread = figfile::parse_fig(&written).unwrap_or_else(|e| panic!("{name}: re-read: {e}"));

        assert_eq!(
            figfile::compare(&original, &reread),
            None,
            "{name}: the document changed in transit"
        );

        let before = testkit::model::build_scene_graph(&original).expect("a scene graph");
        let after = testkit::model::build_scene_graph(&reread).expect("a scene graph");
        same_graph(&name, &before, &after);

        // Reported rather than asserted: the archive order is reconstructed
        // (canvas, others, images) rather than recorded, and this is the check
        // that the reconstruction matches a real file.
        assert_eq!(
            entry_names(&std::fs::read(&path).expect("the fixture")),
            entry_names(&written),
            "{name}: the archive's entries came back in a different order"
        );
    }

    if checked == 0 {
        eprintln!("no fixtures available — see fixtures/README.md");
    }
}

/// I4 and I5. Writing is deterministic, and writing what was written reproduces
/// it — so a difference between two runs is a bug and never the weather.
#[test]
fn writing_is_deterministic_and_idempotent() {
    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let Some(original) = testkit::fixture::parsed(&name) else { continue };

        let once = figfile::write_fig(&original).expect("writes");
        let twice = figfile::write_fig(&original).expect("writes");
        assert_eq!(once, twice, "{name}: two writes of one input differ (I4)");

        let reread = figfile::parse_fig(&once).expect("re-reads");
        let again = figfile::write_fig(&reread).expect("writes");
        assert_eq!(once, again, "{name}: writing a written file does not reproduce it (I5)");
    }
}

/// Two scene graphs hold the same thing.
///
/// Not `before == after`. A real design file contains NaN — 210 of
/// real files' nodes do — and NaN is not equal to itself, so a
/// graph built from a fixture does not compare equal to *itself*, let alone to
/// its round trip. `==` is the fast path; where it fails, the comparison falls
/// back to a representation in which NaN does equal NaN, which is the question
/// actually being asked: is it the same float that came in?
///
/// The NaN survives the round trip on its own merits — it has a full exponent,
/// so it takes the four-byte float form and comes back bit-identical (D7).
#[track_caller]
fn same_graph(name: &str, before: &model::SceneGraph, after: &model::SceneGraph) {
    assert_eq!(before.pages, after.pages, "{name}: the pages changed");
    assert_eq!(
        before.nodes.keys().collect::<Vec<_>>(),
        after.nodes.keys().collect::<Vec<_>>(),
        "{name}: the nodes are not the same nodes, or not in the same order"
    );

    for (id, node) in &before.nodes {
        let other = &after.nodes[id];
        if node != other {
            assert_eq!(format!("{node:?}"), format!("{other:?}"), "{name}: node {id} changed");
        }
    }

    assert_eq!(before.root_id, after.root_id, "{name}: the root changed");
    assert_eq!(before.images, after.images, "{name}: the images changed");
    assert_eq!(before.collections, after.collections, "{name}: the collections changed");
    assert_eq!(before.styles, after.styles, "{name}: the styles changed");
    assert_eq!(
        before.variables.keys().collect::<Vec<_>>(),
        after.variables.keys().collect::<Vec<_>>(),
        "{name}: the variables changed"
    );
    for (id, variable) in &before.variables {
        let other = &after.variables[id];
        if variable != other {
            assert_eq!(format!("{variable:?}"), format!("{other:?}"), "{name}: variable {id}");
        }
    }
}

fn entry_names(archive: &[u8]) -> Vec<String> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(archive)).expect("a ZIP");
    (0..archive.len())
        .map(|index| archive.by_index(index).expect("an entry").name().to_string())
        .collect()
}

// ── The cases a real file does not contain ───────────────────────────────────
//
// Every fixture to hand is a well-populated Figma export with images and a zstd
// message chunk. These build the rest by hand, so a fresh clone covers them too.

/// The binary schema for `message Message { 1: name string; }` — small enough
/// to write out, real enough for `decode_message` to parse a document against.
fn schema_bytes() -> Vec<u8> {
    let mut out = vec![1]; // one definition
    out.extend_from_slice(b"Message\0");
    out.push(2); // MESSAGE
    out.push(1); // one field
    out.extend_from_slice(b"name\0");
    out.push(11); // var_int(-6) -> primitive 5, "string"
    out.push(0); // not an array
    out.push(1); // tag 1
    out
}

/// `{ name: "page" }` encoded against that schema: tag 1, the string, the
/// terminator.
const MESSAGE: &[u8] = b"\x01page\x00\x00";

/// The same message, zstd-compressed. Every real `.fig` measured stores its
/// message chunk this way, so the read path must handle it — but `ruzstd`
/// decodes and does not encode, which is why this is a constant rather than
/// something the test compresses for itself (research D2).
const MESSAGE_ZSTD: &[u8] = &[
    0x28, 0xb5, 0x2f, 0xfd, 0x24, 0x07, 0x39, 0x00, 0x00, 0x01, 0x70, 0x61, 0x67, 0x65, 0x00, 0x00,
    0xff, 0xed, 0x14, 0x37,
];

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::DeflateEncoder::new(data, flate2::Compression::default())
        .read_to_end(&mut out)
        .expect("deflates");
    out
}

/// A container holding the schema above and the given message chunk verbatim.
fn canvas(message_chunk: &[u8]) -> Vec<u8> {
    let mut out = b"fig-kiwi".to_vec();
    out.extend_from_slice(&23u32.to_le_bytes());
    for chunk in [deflate(&schema_bytes()).as_slice(), message_chunk] {
        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out
}

/// However the message arrived — zstd, deflate, or not compressed at all — the
/// document that comes back out is the same one.
#[test]
fn a_message_survives_whatever_it_was_compressed_with() {
    for (how, chunk) in [
        ("zstd", MESSAGE_ZSTD.to_vec()),
        ("deflate", deflate(MESSAGE)),
        ("uncompressed", MESSAGE.to_vec()),
    ] {
        let original = figfile::parse_canvas(&canvas(&chunk), Vec::new())
            .unwrap_or_else(|e| panic!("{how}: {e}"));

        let written = figfile::write_canvas(&original).unwrap_or_else(|e| panic!("{how}: {e}"));
        let reread = figfile::parse_canvas(&written, Vec::new())
            .unwrap_or_else(|e| panic!("{how}: re-read: {e}"));

        assert_eq!(figfile::compare(&original, &reread), None, "{how}: the document changed");
        assert_eq!(
            reread.node_changes.get("name").and_then(kiwi::Value::as_str),
            Some("page"),
            "{how}: and it is the document that went in"
        );

        // The output is deflate every time, whatever came in. If Figma refuses
        // a file, this is the first thing to vary (research D2) — and every
        // real file measured arrives as zstd, so this conversion is on the
        // common path, not an edge of it.
        let second_chunk_is_zstd = written.windows(4).any(|w| w == [0x28, 0xb5, 0x2f, 0xfd]);
        assert!(!second_chunk_is_zstd, "{how}: the writer emitted zstd");
    }
}

/// A file with no images at all, which no fixture is.
#[test]
fn a_file_with_no_images_survives_a_write_and_a_read() {
    let source = figfile::parse_canvas(&canvas(MESSAGE), Vec::new()).expect("parses");

    let written = figfile::write_fig(&source).expect("writes");
    let reread = figfile::parse_fig(&written).expect("re-reads");

    assert!(reread.images.is_empty());
    assert_eq!(figfile::compare(&source, &reread), None);
}

/// The whole archive, built from a canvas that never came from one — images,
/// a metadata entry, a bare directory entry, and a canvas under a name that is
/// not `canvas.fig`.
#[test]
fn a_whole_archive_survives_a_write_and_a_read() {
    let mut source = figfile::parse_canvas(&canvas(MESSAGE), Vec::new()).expect("parses");
    source.canvas_name = "canvas".to_string();
    source.images = vec![("abc".into(), b"an image".to_vec())];
    source.other_entries = vec![
        ("thumbnail.png".into(), b"\x89PNG".to_vec()),
        ("meta.json".into(), b"{\"n\":1}".to_vec()),
        ("images/".into(), Vec::new()),
    ];

    let written = figfile::write_fig(&source).expect("writes");
    let reread = figfile::parse_fig(&written).expect("re-reads");

    assert_eq!(figfile::compare(&source, &reread), None);
    assert_eq!(reread.canvas_name, "canvas", "the name was carried, not guessed");
    assert_eq!(
        entry_names(&written),
        vec!["canvas", "thumbnail.png", "meta.json", "images/", "images/abc"],
        "canvas, then the other entries, then the images"
    );

    // I4 and I5, without needing a fixture.
    assert_eq!(figfile::write_fig(&source).expect("writes"), written);
    assert_eq!(figfile::write_fig(&reread).expect("writes"), written);
}

/// Chunks past the message are carried through, though no real file has any
/// (research D8). This is the only place that path is exercised at all.
#[test]
fn trailing_chunks_survive_a_write_and_a_read() {
    let mut source = figfile::parse_canvas(&canvas(MESSAGE), Vec::new()).expect("parses");
    source.trailing_chunks = vec![Vec::new(), b"something grafta cannot read".to_vec()];

    let written = figfile::write_canvas(&source).expect("writes");
    let reread = figfile::parse_canvas(&written, Vec::new()).expect("re-reads");

    assert_eq!(reread.trailing_chunks, source.trailing_chunks, "byte-identical, and in order");
    assert_eq!(figfile::compare(&source, &reread), None);
}
