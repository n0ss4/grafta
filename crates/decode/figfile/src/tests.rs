//! Unit tests for the container layers, which have no public entry point of
//! their own. End-to-end parsing lives in `../tests/`.

use crate::{compress, container, FigError};

fn chunked(version: u32, chunks: &[&[u8]]) -> Vec<u8> {
    let mut out = b"fig-kiwi".to_vec();
    out.extend_from_slice(&version.to_le_bytes());
    for chunk in chunks {
        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(chunk);
    }
    out
}

#[test]
fn reads_the_header_and_splits_chunks() {
    let canvas = chunked(23, &[b"schema", b"message", b"extra"]);
    let parsed = container::parse(&canvas).expect("parses");

    assert_eq!(parsed.version, 23);
    assert_eq!(parsed.schema_chunk, b"schema");
    assert_eq!(parsed.message_chunk, b"message");
}

#[test]
fn rejects_containers_it_cannot_trust() {
    assert!(matches!(container::parse(b"fig-kiwi").unwrap_err(), FigError::Truncated));
    assert!(matches!(
        container::parse(&chunked(1, &[b"only-one"])).unwrap_err(),
        FigError::NotEnoughChunks
    ));

    let mut wrong_magic = chunked(1, &[b"a", b"b"]);
    wrong_magic[0] = b'x';
    assert!(matches!(container::parse(&wrong_magic).unwrap_err(), FigError::InvalidMagic));

    // A length that runs off the end of the buffer must not be trusted.
    let mut lying_length = b"fig-kiwi".to_vec();
    lying_length.extend_from_slice(&1u32.to_le_bytes());
    lying_length.extend_from_slice(&999u32.to_le_bytes());
    lying_length.extend_from_slice(b"short");
    assert!(matches!(container::parse(&lying_length).unwrap_err(), FigError::Truncated));
}

/// Older files leave the message chunk uncompressed, so an unrecognised chunk
/// has to come back as-is rather than as an error.
#[test]
fn passes_uncompressed_chunks_straight_through() {
    let raw = b"not compressed at all";
    assert_eq!(compress::decompress(raw).expect("decompresses"), raw);
}

// ── Preservation ─────────────────────────────────────────────────────────────
//
// Byte equality is the wrong bar for a whole file (research D6) and the right
// one here: preserving a region means it comes back identical, and nothing
// weaker would be preservation.

#[test]
fn keeps_every_chunk_that_follows_the_message() {
    // The common case, and the only one any real file exhibits (D8).
    let canvas = chunked(1, &[b"schema", b"message"]);
    assert!(container::parse(&canvas).expect("parses").trailing_chunks.is_empty());

    let canvas = chunked(1, &[b"schema", b"message", b"third"]);
    assert_eq!(
        container::parse(&canvas).expect("parses").trailing_chunks,
        vec![b"third".as_slice()]
    );

    let canvas = chunked(1, &[b"schema", b"message", b"c", b"d", b"e"]);
    assert_eq!(
        container::parse(&canvas).expect("parses").trailing_chunks,
        vec![b"c".as_slice(), b"d".as_slice(), b"e".as_slice()],
        "order is data — chunk 2 is chunk 2 because of where it sits"
    );

    // A zero-length chunk is a chunk. Dropping it would shift every chunk
    // after it by one position.
    let canvas = chunked(1, &[b"schema", b"message", b"", b"after"]);
    assert_eq!(
        container::parse(&canvas).expect("parses").trailing_chunks,
        vec![b"".as_slice(), b"after".as_slice()]
    );
}

/// Builds a ZIP from `(name, contents)` pairs. A name ending in `/` becomes a
/// directory entry, which is what a real `.fig`'s bare `images/` entry is.
fn zipped(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write;

    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, contents) in entries {
        if name.ends_with('/') {
            writer.add_directory(*name, options).expect("adds a directory");
        } else {
            writer.start_file(*name, options).expect("starts an entry");
            writer.write_all(contents).expect("writes an entry");
        }
    }
    writer.finish().expect("finishes the archive").into_inner()
}

#[test]
fn keeps_every_archive_entry_that_is_neither_canvas_nor_image() {
    let contents = crate::archive::read(&zipped(&[
        ("canvas.fig", b"the canvas"),
        ("meta.json", b"{\"n\":1}"),
        ("thumbnail.png", b"\x89PNG..."),
        ("images/", b""),
        ("images/abc", b"an image"),
    ]))
    .expect("reads");

    assert_eq!(contents.canvas, b"the canvas");
    assert_eq!(contents.canvas_name, "canvas.fig");
    assert_eq!(contents.images, vec![("abc".to_string(), b"an image".to_vec())]);
    assert_eq!(
        contents.others,
        vec![
            ("meta.json".to_string(), b"{\"n\":1}".to_vec()),
            ("thumbnail.png".to_string(), b"\x89PNG...".to_vec()),
            // Not a file. The reader already declined to call it an image;
            // now it is kept, so the writer can put the directory back.
            ("images/".to_string(), Vec::new()),
        ],
        "in archive order, contents byte-identical"
    );
}

#[test]
fn keeps_the_name_the_canvas_was_stored_under() {
    let contents =
        crate::archive::read(&zipped(&[("canvas", b"the canvas"), ("meta.json", b"{}")]))
            .expect("reads");

    assert_eq!(contents.canvas, b"the canvas");
    assert_eq!(contents.canvas_name, "canvas", "not the `canvas.fig` grafta would have guessed");
    assert_eq!(contents.others, vec![("meta.json".to_string(), b"{}".to_vec())]);
}

/// The canvas found by the largest-candidate fallback must not also appear in
/// `others`, and the candidates it beat must.
#[test]
fn a_canvas_found_by_size_is_not_also_kept_as_another_entry() {
    let contents = crate::archive::read(&zipped(&[
        ("small.bin", b"tiny"),
        ("document", b"much larger than the other candidate"),
        ("meta.json", b"{}"),
    ]))
    .expect("reads");

    assert_eq!(contents.canvas, b"much larger than the other candidate");
    assert_eq!(contents.canvas_name, "document");
    assert_eq!(
        contents.others,
        vec![
            ("small.bin".to_string(), b"tiny".to_vec()),
            ("meta.json".to_string(), b"{}".to_vec()),
        ],
        "the candidate that lost is kept, not dropped"
    );
}

#[test]
fn an_archive_with_no_images_keeps_everything_else() {
    let contents =
        crate::archive::read(&zipped(&[("canvas.fig", b"the canvas"), ("meta.json", b"{}")]))
            .expect("reads");

    assert!(contents.images.is_empty());
    assert_eq!(contents.others, vec![("meta.json".to_string(), b"{}".to_vec())]);
}

// ── Writing ──────────────────────────────────────────────────────────────────

#[test]
fn a_written_container_parses_back_to_what_went_in() {
    let written = container::write(crate::Product::Design, 23, &[b"schema", b"message"]);
    let parsed = container::parse(&written).expect("parses");

    assert_eq!(parsed.version, 23);
    assert_eq!(parsed.schema_chunk, b"schema");
    assert_eq!(parsed.message_chunk, b"message");
    assert!(parsed.trailing_chunks.is_empty());

    // The magic and the version are where the reader looks for them.
    assert_eq!(&written[..8], b"fig-kiwi");
    assert_eq!(&written[8..12], &23u32.to_le_bytes());
}

#[test]
fn a_written_container_carries_trailing_chunks_in_order() {
    let written = container::write(crate::Product::Design, 1, &[b"schema", b"message", b"", b"third", b"fourth"]);
    let parsed = container::parse(&written).expect("parses");

    assert_eq!(
        parsed.trailing_chunks,
        vec![b"".as_slice(), b"third".as_slice(), b"fourth".as_slice()]
    );
}

#[test]
fn a_written_archive_reads_back_as_what_went_in() {
    let others =
        vec![("thumbnail.png".to_string(), b"\x89PNG".to_vec()), ("images/".to_string(), vec![])];
    let images = vec![("abc".to_string(), b"an image".to_vec())];

    let written = crate::archive::write("canvas.fig", b"the canvas", &others, &images)
        .expect("writes an archive");
    let contents = crate::archive::read(&written).expect("reads it back");

    assert_eq!(contents.canvas, b"the canvas");
    assert_eq!(contents.canvas_name, "canvas.fig");
    assert_eq!(contents.images, images);
    assert_eq!(contents.others, others, "including the bare directory entry");
}

#[test]
fn a_written_archive_keeps_the_canvas_name_it_was_given() {
    let written = crate::archive::write("canvas", b"the canvas", &[], &[]).expect("writes");
    assert_eq!(crate::archive::read(&written).expect("reads").canvas_name, "canvas");
}

/// I4. Two writes of the same input must be the same bytes, which is why the
/// timestamps are fixed rather than taken from the clock.
#[test]
fn writing_the_same_archive_twice_produces_the_same_bytes() {
    let others = vec![("meta.json".to_string(), b"{}".to_vec())];
    let images = vec![("abc".to_string(), b"an image".to_vec())];

    let once = crate::archive::write("canvas.fig", b"the canvas", &others, &images).expect("writes");
    let twice =
        crate::archive::write("canvas.fig", b"the canvas", &others, &images).expect("writes");

    assert_eq!(once, twice);
}

#[test]
fn deflate_and_inflate_are_inverses() {
    for data in [b"".as_slice(), b"x", &[0u8; 10_000], b"the quick brown fox"] {
        let squashed = compress::deflate(data).expect("deflates");
        assert_eq!(compress::inflate(&squashed).expect("inflates"), data);
        // And the sniffing decompressor recognises it without being told.
        assert_eq!(compress::decompress(&squashed).expect("decompresses"), data);
    }
}

// ── Comparison ───────────────────────────────────────────────────────────────
//
// A boolean would not be enough. What each of these asserts is that the
// location reported is the one that actually differs.

/// A `FigFile` with a message of the given fields, over a schema that is not a
/// real Figma schema and does not need to be.
fn fig_with(fields: Vec<(&str, kiwi::Value)>) -> crate::FigFile {
    use kiwi::{Definition, DefinitionKind, Field, Schema, TypeRef, Value};

    let schema = Schema {
        definitions: vec![Definition {
            name: "Message".to_string(),
            kind: DefinitionKind::Message,
            fields: vec![
                Field { name: "name".into(), type_ref: Some(TypeRef::Str), is_array: false, value: 1 },
                Field { name: "size".into(), type_ref: Some(TypeRef::Uint), is_array: false, value: 2 },
            ],
        }],
    };

    crate::FigFile {
        product: crate::Product::Design,
        node_changes: Value::Object(
            fields.into_iter().map(|(key, value)| (key.to_string(), value)).collect(),
        ),
        blobs: Vec::new(),
        images: Vec::new(),
        version: 23,
        schema,
        schema_chunk: Vec::new(),
        canvas_name: crate::DEFAULT_CANVAS_NAME.to_string(),
        trailing_chunks: Vec::new(),
        other_entries: Vec::new(),
    }
}

#[test]
fn two_equal_documents_have_no_difference() {
    let one = fig_with(vec![("name", kiwi::Value::Str("page".into()))]);
    let two = fig_with(vec![("name", kiwi::Value::Str("page".into()))]);
    assert_eq!(crate::compare(&one, &two), None);
}

#[test]
fn a_changed_field_is_reported_where_it_changed() {
    let one = fig_with(vec![
        ("name", kiwi::Value::Str("page".into())),
        ("size", kiwi::Value::Uint(3)),
    ]);
    let two = fig_with(vec![
        ("name", kiwi::Value::Str("page".into())),
        ("size", kiwi::Value::Uint(4)),
    ]);

    let found = crate::compare(&one, &two).expect("they differ");
    assert_eq!(found.path, "nodeChanges.size", "the field that differs, not the one before it");
    assert!(found.to_string().contains("4"), "and what it became: {found}");
}

#[test]
fn a_changed_node_is_reported_by_its_index() {
    let node = |name: &str| {
        kiwi::Value::Object([("name".to_string(), kiwi::Value::Str(name.into()))].into())
    };
    let one = fig_with(vec![("name", kiwi::Value::Array(vec![node("a"), node("b")]))]);
    let two = fig_with(vec![("name", kiwi::Value::Array(vec![node("a"), node("c")]))]);

    let found = crate::compare(&one, &two).expect("they differ");
    assert_eq!(found.path, "nodeChanges.name[1].name");
}

#[test]
fn a_changed_image_is_reported_by_its_name() {
    let mut one = fig_with(vec![]);
    let mut two = fig_with(vec![]);
    one.images = vec![("keep".into(), b"same".to_vec()), ("alter".into(), b"before".to_vec())];
    two.images = vec![("keep".into(), b"same".to_vec()), ("alter".into(), b"after".to_vec())];

    let found = crate::compare(&one, &two).expect("they differ");
    assert_eq!(found.path, "images[alter]");
}

#[test]
fn the_layers_around_the_message_are_compared_too() {
    let equal = || fig_with(vec![("name", kiwi::Value::Str("page".into()))]);

    let mut different_version = equal();
    different_version.version = 24;
    assert_eq!(crate::compare(&equal(), &different_version).expect("differs").path, "version");

    let mut renamed = equal();
    renamed.canvas_name = "canvas".into();
    assert_eq!(crate::compare(&equal(), &renamed).expect("differs").path, "canvas");

    let mut lost_an_entry = equal();
    lost_an_entry.other_entries = vec![("meta.json".into(), b"{}".to_vec())];
    assert_eq!(crate::compare(&equal(), &lost_an_entry).expect("differs").path, "entries");

    let mut lost_a_chunk = equal();
    lost_a_chunk.trailing_chunks = vec![b"third".to_vec()];
    assert_eq!(crate::compare(&equal(), &lost_a_chunk).expect("differs").path, "chunks");
}

/// Order is data (D4), so a message whose fields came back in another order is
/// a difference even though it holds the same pairs.
#[test]
fn a_reordered_message_is_a_difference() {
    let one = fig_with(vec![
        ("name", kiwi::Value::Str("page".into())),
        ("size", kiwi::Value::Uint(3)),
    ]);
    let two = fig_with(vec![
        ("size", kiwi::Value::Uint(3)),
        ("name", kiwi::Value::Str("page".into())),
    ]);

    assert_eq!(crate::compare(&one, &two).expect("they differ").path, "nodeChanges");
}

/// A message can round-trip through the container layer alone, which is what
/// `write_canvas` is for.
#[test]
fn a_canvas_survives_a_write_and_a_read() {
    let mut fig = fig_with(vec![
        ("name", kiwi::Value::Str("page".into())),
        ("size", kiwi::Value::Uint(4_000_000)),
    ]);
    // `write_canvas` carries this chunk verbatim, so it has to be a real
    // deflated schema for the read back to parse it.
    fig.schema_chunk = compress::deflate(&schema_bytes()).expect("deflates");

    let canvas = crate::write_canvas(&fig).expect("writes");
    let back = crate::parse_canvas(&canvas, Vec::new()).expect("reads");

    assert_eq!(crate::compare(&fig, &back), None);
    assert_eq!(back.version, 23);
}

/// The binary schema for the two-field `Message` that `fig_with` builds.
fn schema_bytes() -> Vec<u8> {
    let mut out = vec![1]; // one definition
    out.extend_from_slice(b"Message\0");
    out.push(2); // MESSAGE
    out.push(2); // two fields
    out.extend_from_slice(b"name\0");
    out.push(11); // var_int(-6) -> primitive 5, "string"
    out.push(0); // not an array
    out.push(1); // tag 1
    out.extend_from_slice(b"size\0");
    out.push(7); // var_int(-4) -> primitive 3, "uint"
    out.push(0);
    out.push(2); // tag 2
    out
}

#[test]
fn a_deck_and_a_board_are_the_same_container_under_their_own_magic() {
    for (product, magic) in [(crate::Product::Design, b"fig-kiwi"), (crate::Product::Slides, b"fig-deck"), (crate::Product::Whiteboard, b"fig-jam.")] {
        let written = container::write(product, 106, &[b"schema", b"message"]);
        assert_eq!(&written[..8], magic);
        let read = container::parse(&written).unwrap();
        assert_eq!((read.product, read.version), (product, 106));
        assert_eq!(crate::Product::from_extension(product.extension()), Some(product));
    }
    assert!(matches!(container::parse(b"fig-nope\x65\0\0\0"), Err(FigError::InvalidMagic)));
}

#[test]
fn a_bounded_read_stops_past_its_limit_and_only_past_it() {
    let bytes = [7u8; 100];
    assert_eq!(compress::read_bounded(&bytes[..], 101).unwrap().unwrap().len(), 100);
    assert_eq!(compress::read_bounded(&bytes[..], 100).unwrap().unwrap().len(), 100);
    assert!(compress::read_bounded(&bytes[..], 99).unwrap().is_none());
}

#[test]
fn a_message_past_its_ratio_is_refused_and_not_taken_for_plain_data() {
    let zeros = vec![0u8; 8 << 20];
    let packed = compress::deflate(&zeros).expect("deflates");
    assert!(packed.len() * 256 + (1 << 20) < zeros.len(), "compresses past the ratio");
    let error = compress::decompress(&packed).unwrap_err();
    assert!(matches!(error, FigError::TooLarge { what: "the message", .. }), "{error}");
    assert!(error.to_string().contains("bytes"), "{error}");
}

#[test]
fn a_message_within_its_ratio_and_plain_data_are_still_read() {
    let zeros = vec![0u8; 1 << 20];
    let packed = compress::deflate(&zeros).expect("deflates");
    assert_eq!(compress::decompress(&packed).expect("reads"), zeros);
    let plain = b"\xFF\xFF not deflate at all";
    assert_eq!(compress::decompress(plain).expect("reads"), plain);
}

#[test]
fn a_schema_past_its_limit_is_refused() {
    let zeros = vec![0u8; (16 << 20) + 1];
    let packed = compress::deflate(&zeros).expect("deflates");
    assert!(matches!(compress::inflate(&packed), Err(FigError::TooLarge { what: "the schema", .. })));
}

#[test]
fn an_archive_stops_at_its_total_across_entries() {
    let archive = zipped(&[("canvas.fig", &[1u8; 60]), ("images/a", &[0u8; 60])]);
    assert_eq!(crate::archive::read_within(&archive, 120).expect("reads").images.len(), 1);
    let error = crate::archive::read_within(&archive, 119).err().expect("too large");
    assert!(matches!(error, FigError::TooLarge { what: "the archive", limit: 119 }), "{error}");
}
