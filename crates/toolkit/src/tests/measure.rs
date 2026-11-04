use serde_json::{json, Value};

use crate::edge::measure::Json;
use crate::edge::data::from_json;
use crate::port::memory::Written;
use crate::port::Measure;

/// Both measures count a value as its JSON's length always was counted.
fn counted_alike(value: &Value) {
    let data = from_json(value);
    assert_eq!(Json.bytes(&data), value.to_string().len(), "{value}");
    assert_eq!(Written.bytes(&data), value.to_string().len(), "{value} in memory");
}

#[test]
fn a_value_is_its_json_s_bytes() {
    for value in [
        json!(null),
        json!(1),
        json!(-0.0),
        json!(0.1),
        json!(1e21),
        json!(f32::MAX),
        json!("é 字 🎨 \"quoted\" \u{1f}"),
        json!({ "b": [1, 2.5, "x", null, { "z": {}, "a": [] }], "a": -3 }),
    ] {
        counted_alike(&value);
    }
}

/// Every call and schema on record, and every result the corpus recorded
/// when the fixtures are here.
#[test]
fn every_value_on_record_is_counted_alike() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut seen = 0;
    for dir in [root.join("tests/golden"), root.join("../../fixtures/golden")] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "json")) {
            let value: Value = serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
            for item in value.as_array().into_iter().flatten() {
                counted_alike(item);
                seen += 1;
            }
        }
    }
    assert!(seen > 100, "{seen} values");
}
