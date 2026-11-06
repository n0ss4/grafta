use kiwi::{Schema, Value};
use schema::{fields, Field, Wire};

use super::without_unused;
use crate::{FigFile, Product};

const USED: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const UNUSED: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn object<const N: usize>(entries: [(Field, Value); N]) -> Value {
    Value::Object(fields(entries))
}

fn blob(bytes: &[u8]) -> Value {
    object([(Field::Bytes, Value::Bytes(bytes.to_vec()))])
}

/// A file whose message holds `nodes` and three blobs.
fn file(nodes: Value, images: &[&str]) -> FigFile {
    let blobs: [&[u8]; 3] = [b"zero", b"one", b"two"];
    FigFile {
        product: Product::Design,
        node_changes: object([
            (Field::NodeChanges, nodes),
            (Field::Blobs, Value::Array(blobs.iter().map(|bytes| blob(bytes)).collect())),
        ]),
        blobs: blobs.iter().map(|bytes| bytes.to_vec()).collect(),
        images: images.iter().map(|name| ((*name).to_string(), vec![1, 2, 3])).collect(),
        version: 1,
        schema: Schema { definitions: Vec::new() },
        schema_chunk: Vec::new(),
        canvas_name: "canvas.fig".into(),
        trailing_chunks: Vec::new(),
        other_entries: Vec::new(),
    }
}

fn commands(index: u32) -> Value {
    Value::Array(vec![object([(Field::CommandsBlob, Value::Uint(index))])])
}

fn hash(name: &str) -> Value {
    let bytes = (0..name.len()).step_by(2).map(|i| u8::from_str_radix(&name[i..i + 2], 16).unwrap()).collect();
    Value::Bytes(bytes)
}

#[test]
fn a_file_with_everything_named_is_left_alone() {
    let nodes = Value::Array(vec![
        object([(Field::FillGeometry, commands(0))]),
        object([(Field::FillGeometry, commands(1))]),
        object([(Field::VectorData, object([(Field::VectorNetworkBlob, Value::Uint(2))]))]),
        object([(Field::Hash, hash(USED))]),
    ]);
    assert!(without_unused(&file(nodes, &[USED])).is_none());
}

#[test]
fn a_blob_nothing_names_goes_and_the_indexes_after_it_follow() {
    let nodes = Value::Array(vec![
        object([(Field::FillGeometry, commands(0))]),
        object([(Field::VectorData, object([(Field::VectorNetworkBlob, Value::Uint(2))]))]),
    ]);
    let pruned = without_unused(&file(nodes, &[])).expect("blob 1 is unused");

    assert_eq!((pruned.blobs, pruned.images, pruned.bytes), (1, 0, 3));
    assert_eq!(pruned.fig.blobs, [b"zero".to_vec(), b"two".to_vec()]);

    let message = &pruned.fig.node_changes;
    assert_eq!(message.field(Field::Blobs).and_then(Value::as_array).map(Vec::len), Some(2));
    let nodes = message.field(Field::NodeChanges).and_then(Value::as_array).unwrap();
    let first = nodes[0].field(Field::FillGeometry).and_then(Value::as_array).unwrap();
    assert_eq!(first[0].field(Field::CommandsBlob), Some(&Value::Uint(0)));
    let vector = nodes[1].field(Field::VectorData).unwrap();
    assert_eq!(vector.field(Field::VectorNetworkBlob), Some(&Value::Uint(1)), "two moved down to one");
}

#[test]
fn an_image_nothing_names_goes_and_one_named_anywhere_stays() {
    let nodes = Value::Array(vec![
        object([(Field::FillGeometry, commands(0))]),
        object([(Field::FillGeometry, commands(1))]),
        object([(Field::FillGeometry, commands(2))]),
        object([(Field::Name, hash(USED))]),
    ]);
    let pruned = without_unused(&file(nodes, &[USED, UNUSED])).expect("one image is unused");

    assert_eq!((pruned.images, pruned.blobs, pruned.bytes), (1, 0, 3));
    assert_eq!(pruned.fig.images.len(), 1);
    assert_eq!(pruned.fig.images[0].0, USED);
}

#[test]
fn an_image_named_as_text_stays() {
    let nodes = Value::Array(vec![
        object([(Field::FillGeometry, commands(0))]),
        object([(Field::FillGeometry, commands(1))]),
        object([(Field::FillGeometry, commands(2))]),
        object([(Field::Name, Value::Str(USED.to_string()))]),
    ]);
    assert!(without_unused(&file(nodes, &[USED])).is_none());
}

#[test]
fn an_entry_not_named_as_a_hash_is_not_judged() {
    let nodes = Value::Array(vec![
        object([(Field::FillGeometry, commands(0))]),
        object([(Field::FillGeometry, commands(1))]),
        object([(Field::FillGeometry, commands(2))]),
    ]);
    assert!(without_unused(&file(nodes, &["cover.png", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"])).is_none());
}

#[test]
fn an_index_it_cannot_rewrite_keeps_every_blob() {
    let mut with_base = file(object([(Field::FillGeometry, commands(0))]), &[UNUSED]);
    if let Value::Object(message) = &mut with_base.node_changes {
        #[allow(clippy::disallowed_methods)]
        message.insert("blobBaseIndex", Value::Uint(7));
    }
    let pruned = without_unused(&with_base).expect("the image still goes");
    assert_eq!((pruned.images, pruned.blobs), (1, 0));
    assert_eq!(pruned.fig.blobs.len(), 3);
}

#[test]
fn an_index_past_the_end_keeps_every_blob() {
    let out_of_range = file(object([(Field::FillGeometry, commands(9))]), &[]);
    assert!(without_unused(&out_of_range).is_none());
}

#[test]
fn blobs_the_reader_could_not_flatten_are_kept() {
    let mut uneven = file(object([(Field::FillGeometry, commands(0))]), &[]);
    uneven.blobs.pop();
    assert!(without_unused(&uneven).is_none());
}
