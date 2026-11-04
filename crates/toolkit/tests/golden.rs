//! Every tool's result, and every file the tools write, compared byte for byte
//! with a recording made before a refactor. Results depend on the machine's
//! fonts, so a recording lives with the fixtures, not in the repository; with
//! none, each batch skips. `GRAFTA_RECORD=1` records instead of comparing.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use toolkit::edge::batch;
use toolkit::{tools, Output};

const RECORD: &str = "GRAFTA_RECORD";
const HAND_BUILT_PATH: &str = "/tmp/hand-built.fig";
const CALLS_DIR: &str = "tests/golden";
const SCRATCH_DIR: &str = "../../target/golden";
const RESULTS_DIR: &str = "golden";

const TMP: &str = "{tmp}";
const FIXTURES: &str = "{fixtures}";
const HANDLE: &str = "{handle}";

/// Measured on each run, so never the same twice.
const MEASURED: [&str; 4] = ["open_seconds", "opened_at", "opened_delta_mb", "process_resident_mb"];

struct Batch {
    name: &'static str,
    fixtures: &'static [&'static str],
    hand_built: bool,
}

const BATCHES: [Batch; 4] = [
    Batch { name: "hand_built", fixtures: &[], hand_built: true },
    Batch { name: "design", fixtures: &["material3.fig"], hand_built: false },
    Batch { name: "deck", fixtures: &["Figma Deck Transitions.deck"], hand_built: false },
    Batch { name: "board", fixtures: &["Roadmap review template (Copy).jam"], hand_built: false },
];

fn without_measured(mut value: Value) -> Value {
    match &mut value {
        Value::Object(fields) => {
            for key in MEASURED {
                fields.remove(key);
            }
            for field in fields.values_mut() {
                *field = without_measured(field.take());
            }
        }
        Value::Array(items) => {
            for item in items {
                *item = without_measured(item.take());
            }
        }
        _ => {}
    }
    value
}

/// Every placeholder in the strings of `value` replaced by its value.
fn placed(value: Value, places: &[(&str, String)]) -> Value {
    match value {
        Value::String(text) => Value::String(places.iter().fold(text, |text, (placeholder, with)| text.replace(placeholder, with))),
        Value::Array(items) => Value::Array(items.into_iter().map(|item| placed(item, places)).collect()),
        Value::Object(fields) => Value::Object(fields.into_iter().map(|(key, field)| (key, placed(field, places))).collect()),
        other => other,
    }
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(SCRATCH_DIR).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

fn written_files(dir: &Path) -> Value {
    let mut files: Vec<(String, String)> = fs::read_dir(dir)
        .expect("scratch directory")
        .filter_map(Result::ok)
        .map(|entry| (entry.file_name().to_string_lossy().into_owned(), sha1_hex(&fs::read(entry.path()).expect("written file"))))
        .collect();
    files.sort();
    json!(files.into_iter().map(|(name, sha1)| json!({ "file": name, "sha1": sha1 })).collect::<Vec<_>>())
}

fn run(batch: &Batch) -> Option<Value> {
    let fixtures = testkit::fixture::dir();
    if batch.fixtures.iter().any(|name| testkit::fixture::bytes(name).is_none()) {
        return None;
    }
    let tmp = scratch(batch.name);
    let mut kit = toolkit::new();
    kit.check_rollbacks();
    let handle = if batch.hand_built {
        kit.insert(PathBuf::from(HAND_BUILT_PATH), testkit::raw::document()).expect("hand-built document").handle.to_string()
    } else {
        String::new()
    };
    let text = fs::read_to_string(Path::new(CALLS_DIR).join(format!("{}.calls.json", batch.name))).expect("calls file");
    let calls: Vec<Value> = serde_json::from_str(&text).expect("calls are a JSON array");
    // Placed after parsing: a Windows path's backslashes are not JSON escapes.
    let places = [(TMP, tmp.to_string_lossy().into_owned()), (FIXTURES, fixtures.to_string_lossy().into_owned()), (HANDLE, handle)];
    let calls: Vec<Value> = calls.into_iter().map(|call| placed(call, &places)).collect();

    let mut results = Vec::new();
    let mut recorded = Vec::new();
    for (index, entry) in calls.iter().enumerate() {
        let tool = entry["tool"].as_str().expect("tool name");
        let args = batch::substitute(&entry["args"], &results).unwrap_or_else(|error| panic!("call {index}: {error}"));
        let outcome = match toolkit::edge::call(&mut kit, tool, &args) {
            Ok(Output::Data(value)) => {
                let value = toolkit::edge::data::to_json(&value);
                results.push(value.clone());
                json!({ "ok": without_measured(value) })
            }
            Ok(Output::Image { mime, bytes, caption }) => {
                results.push(Value::Null);
                json!({ "image": { "mime": mime, "sha1": sha1_hex(&bytes), "caption": caption } })
            }
            Err(error) => {
                results.push(Value::Null);
                json!({ "error": format!("{error:?}") })
            }
        };
        recorded.push(json!({ "call": index, "tool": tool, "result": outcome }));
    }
    Some(json!({ "calls": recorded, "written": written_files(&tmp) }))
}

#[test]
fn every_result_and_every_written_file_is_as_recorded() {
    let results_dir = testkit::fixture::dir().join(RESULTS_DIR);
    let recording = std::env::var_os(RECORD).is_some();
    for batch in &BATCHES {
        let Some(actual) = run(batch) else { continue };
        let path = results_dir.join(format!("{}.results.json", batch.name));
        let actual = serde_json::to_string_pretty(&actual).expect("results serialise");
        if recording {
            fs::create_dir_all(&results_dir).expect("results directory");
            fs::write(&path, actual).expect("results written");
            continue;
        }
        let Ok(expected) = fs::read_to_string(&path) else { continue };
        for (line, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
            assert_eq!(got, want, "{}: line {} of {} changed", batch.name, line + 1, path.display());
        }
        assert_eq!(actual.len(), expected.len(), "{}: the batch changed length", batch.name);
    }
}

/// The tool list a model reads, name, description and schema, in order. It
/// depends on no font or fixture, so it is kept in the repository.
const TOOLS: &str = "tests/golden/tools.json";

#[test]
fn the_tool_list_is_as_recorded() {
    let list: Vec<Value> =
        tools::ALL.iter().map(|tool| json!({ "name": tool.name(), "description": tool.description(), "inputSchema": tool.input_schema() })).collect();
    let actual = serde_json::to_string_pretty(&list).expect("tool list serialises") + "\n";
    if std::env::var_os(RECORD).is_some() {
        fs::write(TOOLS, actual).expect("tool list written");
        return;
    }
    let expected = fs::read_to_string(TOOLS).expect("the recorded tool list");
    for (line, (want, got)) in expected.lines().zip(actual.lines()).enumerate() {
        assert_eq!(got, want, "line {} of {TOOLS} changed", line + 1);
    }
    assert_eq!(actual.lines().count(), expected.lines().count(), "the tool list changed length");
}
