//! The calls grafta's panel runs in Figma as its self-test (its
//! ladder), run here on `order.fig`, the sample the web app opens, and
//! recorded for the panel to compare with what Figma answers: every result's
//! keys, and for a call marked `values`, a read of the sample's own layers,
//! its values. `GRAFTA_RECORD=1` records; otherwise the recording is compared.

use std::fs;

use serde_json::{json, Map, Value};
use toolkit::edge::batch;
use toolkit::{edge, Output};

const RECORD: &str = "GRAFTA_RECORD";
const SAMPLE: &str = "../../web/public/sample/order.fig";
const CALLS: &str = "tests/selftest/calls.json";
const RECORDING: &str = "../../figma-plugin/src/selftest.json";

/// Keys a result on a file has that are about grafta's copy of the file, not
/// the result: never compared, and said so in the recording.
const NOT_COMPARED: [(&str, &str); 5] = [
    ("unsaved_edits", "a file's edits wait for save_as; Figma saves its own"),
    ("measured_in_stand_in", "grafta's measure of a text on a machine without its font"),
    ("font_faces", "whether this machine has a font"),
    ("undecoded", "fields grafta read from the file's bytes and does not model"),
    ("internal_only_canvas", "whether grafta wrote the page a file keeps its variables on; Figma keeps its own"),
];

fn compared(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields.into_iter().filter(|(key, _)| !NOT_COMPARED.iter().any(|(k, _)| k == key)).map(|(k, v)| (k, compared(v))).collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(compared).collect()),
        other => other,
    }
}

fn recording() -> Value {
    let calls: Vec<Value> = serde_json::from_str(&fs::read_to_string(CALLS).expect("the self-test's calls")).expect("calls are a JSON array");
    let mut kit = toolkit::new();
    let opened = edge::call(&mut kit, "open_file", &json!({ "path": SAMPLE })).expect("order.fig opens");
    let Output::Data(opened) = opened else { panic!("open_file answers with data") };
    let handle = edge::data::to_json(&opened)["handle"].clone();

    let mut results = Vec::new();
    let mut cases = Vec::new();
    for (index, call) in calls.iter().enumerate() {
        let tool = call["tool"].as_str().expect("a tool's name");
        let mut args = batch::substitute(&call["args"], &results).unwrap_or_else(|error| panic!("call {index}: {error}"));
        args["handle"] = handle.clone();
        let live = match edge::find(tool).unwrap_or_else(|| panic!("call {index}: no tool {tool}")).live() {
            edge::Live::Same => "same",
            edge::Live::Differs(_) => "differs",
            edge::Live::Cannot(why) => panic!("call {index}: {tool} does not act on the open file ({why}); the self-test is of tools that do"),
        };
        let mut case = Map::new();
        case.insert("tool".into(), json!(tool));
        case.insert("args".into(), call["args"].clone());
        case.insert("live".into(), json!(live));
        match edge::call(&mut kit, tool, &args).unwrap_or_else(|error| panic!("call {index}, {tool}: {error}")) {
            Output::Data(value) => {
                let value = edge::data::to_json(&value);
                results.push(value.clone());
                let value = compared(value);
                let keys: Vec<&String> = value.as_object().map(|o| o.keys().collect()).unwrap_or_default();
                case.insert("keys".into(), json!(keys));
                if call["values"] == json!(true) {
                    case.insert("values".into(), value);
                }
            }
            Output::Image { .. } => {
                results.push(Value::Null);
                case.insert("image".into(), json!(true));
            }
        }
        cases.push(Value::Object(case));
    }
    json!({
        "about": "Recorded by crates/toolkit/tests/selftest.rs from the calls in crates/toolkit/tests/selftest/calls.json, on web/public/sample/order.fig. GRAFTA_RECORD=1 cargo test -p grafta-toolkit --test selftest records it again.",
        "not_compared": NOT_COMPARED.iter().map(|(k, why)| (k.to_string(), json!(why))).collect::<Map<_, _>>(),
        "cases": cases,
    })
}

#[test]
fn the_self_test_is_as_recorded() {
    let actual = serde_json::to_string_pretty(&recording()).expect("the recording serialises") + "\n";
    if std::env::var_os(RECORD).is_some() {
        fs::write(RECORDING, actual).expect("the recording written");
        return;
    }
    let expected = fs::read_to_string(RECORDING).expect("the recorded self-test");
    for (line, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(got, want, "line {} of {RECORDING} changed: GRAFTA_RECORD=1 cargo test -p grafta-toolkit --test selftest", line + 1);
    }
    assert_eq!(actual.lines().count(), expected.lines().count(), "the self-test's recording changed length");
}
