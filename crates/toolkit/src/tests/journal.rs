//! Journaling and recovery, on the hand-built document, with the journals in
//! memory. A restart is a toolkit dropped and a new one opening the same file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::edge::journal::{path_for, recover};
use crate::Base;
use crate::tests::{call, document};
use crate::port::disk::Render;
use crate::port::memory::{FixedFaces, Memory, MemoryJournal};
use crate::port::{Codec, DocumentStore, Journal, Ports};
use figfile::{Difference, FigFile};
use crate::{ToolError, Toolkit};

const FILE: &str = "/tmp/hand-built.fig";
const BASE: Base = Base { bytes: 1000, fnv: 0xfeed };

/// A save of the hand-built document, which has no schema to encode by. What a
/// save writes is not what these tests are about, only that it happened.
struct Saves;

impl Codec for Saves {
    fn decode(&self, _: &[u8]) -> Result<FigFile, String> {
        Ok(document())
    }
    fn encode(&self, _: &FigFile) -> Result<Vec<u8>, String> {
        Ok(b"saved".to_vec())
    }
    fn compare(&self, _: &FigFile, _: &FigFile) -> Option<Difference> {
        None
    }
}

/// One test's journals and files, in memory, kept from one server to the next.
struct Journals {
    dir: PathBuf,
    store: Arc<Memory>,
    kept: Arc<MemoryJournal>,
}

impl Journals {
    fn new() -> Self {
        Self { dir: PathBuf::from("/memory/journals"), store: Arc::default(), kept: Arc::default() }
    }

    fn ports(&self) -> Ports {
        Ports { raster: Arc::new(Render), codec: Arc::new(Saves), ..crate::port::memory::ports(&self.store, &self.kept, FixedFaces::default()) }
    }

    fn text(&self, path: &Path) -> Option<String> {
        self.kept.read(path)
    }

    fn write(&self, path: &Path, text: &str) {
        self.kept.write(path, text).unwrap();
    }

    fn exists(&self, path: &Path) -> bool {
        self.kept.read(path).is_some()
    }
}

/// A server that has just started and opened the document: what it
/// recovered, if anything.
fn start(dir: &Journals, base: Base, recovering: bool) -> (Toolkit, String, Option<Value>) {
    let mut kit = crate::with_ports(dir.ports(), Some(dir.dir.clone()));
    // With the two entries a save asks for.
    let mut fig = document();
    fig.other_entries = vec![("thumbnail.png".into(), b"a picture".to_vec()), ("meta.json".into(), b"{}".to_vec())];
    let handle = kit.insert_with_base(PathBuf::from(FILE), fig, base).unwrap().handle.to_string();
    let recovered = recover(&mut kit, &handle, recovering).unwrap().map(|r| crate::edge::data::to_json(&r));
    (kit, handle, recovered)
}

/// Three edits: a frame, its fill, and a text inside it.
fn edit(kit: &mut Toolkit, handle: &str) -> String {
    let frame = call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Kept", "x": 0, "y": 0, "width": 100, "height": 50 })).unwrap();
    let id = frame["id"].as_str().unwrap().to_string();
    call(kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "fills": [{ "type": "SOLID", "color": "#1E88E5" }] } })).unwrap();
    call(kit, "create_text", json!({ "handle": handle, "parent": id, "characters": "Hola" })).unwrap();
    id
}

fn journal(kit: &Toolkit, handle: &str, dir: &Journals) -> PathBuf {
    path_for(&dir.dir, kit.session(handle).unwrap())
}

#[test]
fn edits_come_back_after_a_restart_node_for_node() {
    let dir = Journals::new();
    let (mut kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, None, "nothing to recover on a first open");
    edit(&mut kit, &handle);
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 3 })));
    let session = kit.session(&handle).unwrap();
    assert_eq!(session.fig.node_changes, before);
    assert_eq!(session.edits, 3);
}

#[test]
fn edits_made_after_a_recovery_come_back_with_it() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    drop(kit);
    let (mut kit, handle, _) = start(&dir, BASE, true);
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "name": "Renamed" } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 4 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn a_journal_made_on_other_bytes_is_set_aside_not_replayed() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    let path = journal(&kit, &handle, &dir);
    drop(kit);

    let (kit, handle, recovered) = start(&dir, Base { bytes: 1001, fnv: 0xbeef }, true);
    let recovered = recovered.unwrap();
    assert_eq!(recovered["edits"], 0);
    assert_eq!(recovered["set_aside"]["entries"], 3);
    assert!(recovered["set_aside"]["reason"].as_str().unwrap().contains("changed"));
    assert!(dir.exists(Path::new(recovered["set_aside"]["path"].as_str().unwrap())));
    assert!(!dir.exists(&path));
    assert_eq!(kit.session(&handle).unwrap().edits, 0);
}

#[test]
fn an_edit_that_comes_out_differently_is_kept_said_and_stops_the_replay() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    let path = journal(&kit, &handle, &dir);
    drop(kit);
    // The second edit claims to have produced something it does not.
    let text = dir.text(&path).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut second: Value = serde_json::from_str(&lines[2]).unwrap();
    second["check"]["nodes"] = json!(1);
    lines[2] = second.to_string();
    dir.write(&path, &(lines.join("\n") + "\n"));

    let (kit, handle, recovered) = start(&dir, BASE, true);
    let recovered = recovered.unwrap();
    assert_eq!(recovered["edits"], 1, "{recovered}");
    // The second edit ran, so it is in the file and stays in the journal.
    assert_eq!(recovered["diverged"]["at"], 1);
    assert!(recovered["diverged"]["reason"].as_str().unwrap().contains("set_properties"), "{recovered}");
    assert_eq!(recovered["unreplayed"]["entries"], 1);
    assert_eq!(recovered["unreplayed"]["from"], 2);
    let aside = dir.text(Path::new(recovered["unreplayed"]["path"].as_str().unwrap())).unwrap();
    assert_eq!(aside.lines().count(), 2, "the header and the entry set aside");
    assert_eq!(dir.text(&path).unwrap().lines().count(), 3, "the header and the two that ran");
    assert_eq!(kit.session(&handle).unwrap().edits, 2);

    // What the journal now holds is exactly what the file holds.
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);
    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 2 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn an_edit_torn_by_a_stop_is_dropped_and_said() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    let path = journal(&kit, &handle, &dir);
    drop(kit);
    let mut text = dir.text(&path).unwrap();
    text.push_str("{\"tool\":\"set_prop");
    dir.write(&path, &text);

    let (_, _, recovered) = start(&dir, BASE, true);
    let recovered = recovered.unwrap();
    assert_eq!(recovered["edits"], 3);
    assert_eq!(recovered["dropped"]["entries"], 1);
}

#[test]
fn declining_to_recover_and_closing_both_discard_the_edits() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    drop(kit);
    let (kit, handle, recovered) = start(&dir, BASE, false);
    assert_eq!(recovered, Some(json!({ "edits": 0, "discarded": 3 })));
    assert!(!dir.exists(&journal(&kit, &handle, &dir)));
    drop(kit);

    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    let path = journal(&kit, &handle, &dir);
    assert!(dir.exists(&path));
    call(&mut kit, "close_file", json!({ "handle": handle })).unwrap();
    assert!(!dir.exists(&path));
    let (_, _, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, None);
}

#[test]
fn edits_saved_elsewhere_are_not_recovered_and_the_source_opens_as_it_is() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/elsewhere.fig" })).unwrap();
    let path = journal(&kit, &handle, &dir);
    drop(kit);

    let (mut kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 0, "already_saved": { "edits": 3, "path": "/memory/elsewhere.fig" } })));
    assert!(!dir.exists(&path), "nothing is left to recover");
    assert_eq!(kit.session(&handle).unwrap().edits, 0);
    assert!(call(&mut kit, "read_node", json!({ "handle": handle, "id": id })).is_err(), "the source is as it is on disk");
}

#[test]
fn an_edit_after_a_save_brings_back_all_it_depends_on() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/first.fig" })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "name": "After" } })).unwrap();
    drop(kit);

    let (mut kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered.unwrap()["edits"], 4);
    assert_eq!(call(&mut kit, "read_node", json!({ "handle": handle, "id": id })).unwrap()["name"], "After");

    // Saved again, there is nothing left to recover.
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/second.fig" })).unwrap();
    drop(kit);
    let again = start(&dir, BASE, true).2.unwrap();
    assert_eq!(again["edits"], 0);
    assert_eq!(again["already_saved"], json!({ "edits": 4, "path": "/memory/second.fig" }));
}

#[test]
fn an_edit_torn_after_a_save_is_dropped_and_said_and_the_rest_is_saved() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/saved.fig" })).unwrap();
    let path = journal(&kit, &handle, &dir);
    drop(kit);
    let whole = dir.text(&path).unwrap();
    dir.write(&path, &format!("{whole}{{\"tool\":\"crea"));

    let recovered = start(&dir, BASE, true).2.unwrap();
    assert_eq!(recovered["edits"], 0);
    assert_eq!(recovered["already_saved"]["edits"], 3);
    assert_eq!(recovered["dropped"]["entries"], 1);
    assert!(!dir.exists(&path));
}

#[test]
fn edits_all_saved_are_not_set_aside_when_the_source_has_changed() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/saved.fig" })).unwrap();
    drop(kit);
    let recovered = start(&dir, Base { bytes: 2000, fnv: 0xbeef }, true).2.unwrap();
    assert_eq!(recovered["already_saved"]["path"], "/memory/saved.fig");
    assert!(recovered.get("set_aside").is_none(), "{recovered}");
}

#[test]
fn declining_to_recover_counts_only_the_edits_it_loses() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/saved.fig" })).unwrap();
    drop(kit);
    assert_eq!(start(&dir, BASE, false).2.unwrap()["discarded"], 0);

    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id2 = edit(&mut kit, &handle);
    assert_eq!(id, id2);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/again.fig" })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "name": "After" } })).unwrap();
    drop(kit);
    assert_eq!(start(&dir, BASE, false).2.unwrap()["discarded"], 4);
}

#[test]
fn a_save_that_fails_leaves_the_edits_to_recover() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    edit(&mut kit, &handle);
    assert!(call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/not-a-figma-name.txt" })).is_err());
    drop(kit);
    assert_eq!(start(&dir, BASE, true).2.unwrap()["edits"], 3);
}

#[test]
fn a_toolkit_without_a_journal_writes_none() {
    let dir = Journals::new();
    let mut kit = crate::with_ports(dir.ports(), None);
    let handle = kit.insert_with_base(PathBuf::from(FILE), document(), BASE).unwrap().handle.to_string();
    edit(&mut kit, &handle);
    assert!(dir.kept.all().is_empty());
    assert_eq!(recover(&mut kit, &handle, true).unwrap(), None);
}

#[test]
fn an_edit_that_cannot_be_journaled_fails_saying_it_is_only_in_memory() {
    // The disk's own failure: a file where the journal directory should be.
    let dir = std::env::temp_dir().join(format!("grafta-journal-unwritable-{}", std::process::id()));
    std::fs::write(&dir, b"not a directory").unwrap();
    let mut kit = crate::with_journal(dir.clone());
    let handle = kit.insert_with_base(PathBuf::from(FILE), document(), BASE).unwrap().handle.to_string();
    let error = call(&mut kit, "create_page", json!({ "handle": handle, "name": "Lost" })).unwrap_err();
    assert!(matches!(error, ToolError::Failed(ref m) if m.contains("only in memory")), "{error}");
    let _ = std::fs::remove_file(&dir);
}

#[test]
fn variables_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    call(&mut kit, "create_variable_collection", json!({ "handle": handle, "name": "Foundations", "modes": ["Light", "Dark"] })).unwrap();
    call(&mut kit, "create_variable", json!({ "handle": handle, "collection": "Foundations", "name": "Brand/Primary", "type": "COLOR", "values": { "Light": "#7C3AED", "Dark": "#A78BFA" } })).unwrap();
    call(&mut kit, "set_variable", json!({ "handle": handle, "id": "Brand/Primary", "values": { "Dark": "#C4B5FD" } })).unwrap();
    call(&mut kit, "set_variable_collection", json!({ "handle": handle, "id": "Foundations", "add_mode": "High contrast" })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let listed = call(&mut kit, "list_variables", json!({ "handle": handle, "collection": "Foundations" })).unwrap();
    drop(kit);

    let (mut kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 4 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
    let again = call(&mut kit, "list_variables", json!({ "handle": handle, "collection": "Foundations" })).unwrap();
    assert_eq!(again["results"], listed["results"]);
}

#[test]
fn bindings_and_modes_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    call(&mut kit, "create_variable_collection", json!({ "handle": handle, "name": "Theme", "modes": ["Light", "Dark"] })).unwrap();
    call(&mut kit, "create_variable", json!({ "handle": handle, "collection": "Theme", "name": "Pad", "type": "FLOAT", "values": { "Light": 16, "Dark": 24 } })).unwrap();
    let id = edit(&mut kit, &handle);
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "layoutMode": "VERTICAL", "padding": { "variable": "Pad" } } })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "explicitVariableModes": { "Theme": "Dark" } } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 7 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn styles_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    call(&mut kit, "create_style", json!({ "handle": handle, "name": "Brand/Primary", "type": "FILL", "fills": [{ "type": "SOLID", "color": "#7C3AED" }] })).unwrap();
    let id = edit(&mut kit, &handle);
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "fillStyle": "Brand/Primary" } })).unwrap();
    call(&mut kit, "set_style", json!({ "handle": handle, "id": "Brand/Primary", "fills": [{ "type": "SOLID", "color": "#16A34A" }] })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 6 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn component_properties_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = |v: Value| v["id"].as_str().unwrap().to_string();
    let frame = id(call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Button", "x": 0, "y": 0, "width": 120, "height": 40 })).unwrap());
    let label = id(call(&mut kit, "create_text", json!({ "handle": handle, "parent": frame, "characters": "Button" })).unwrap());
    call(&mut kit, "create_component", json!({ "handle": handle, "id": frame })).unwrap();
    call(&mut kit, "add_component_property", json!({ "handle": handle, "component": frame, "name": "Label", "type": "TEXT", "default": "Button" })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": label, "properties": { "componentPropertyReferences": { "characters": "Label" } } })).unwrap();
    let copy = id(call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": frame, "x": 0, "y": 80 })).unwrap());
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": copy, "properties": { "componentProperties": { "Label": "Guardar" } } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 7 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn an_image_fill_comes_back_after_a_restart() {
    let dir = Journals::new();
    let png = Path::new("/memory/image.png");
    // A 1×1 PNG's header is enough: grafta sizes it and embeds the bytes.
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend([0, 0, 0, 2, 0, 0, 0, 3, 8, 6, 0, 0, 0]);
    dir.store.write(png, &bytes).unwrap();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "fills": [{ "type": "IMAGE", "image": png.display().to_string() }] } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let images = kit.session(&handle).unwrap().fig.images.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 4 })));
    let session = kit.session(&handle).unwrap();
    assert_eq!(session.fig.node_changes, before);
    assert_eq!(session.fig.images, images);
}

#[test]
fn shapes_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let star = call(&mut kit, "create_shape", json!({ "handle": handle, "parent": "1:1", "type": "STAR", "width": 40, "height": 40, "fills": [{ "type": "SOLID", "color": "#FACC15" }] })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": star["id"], "properties": { "pointCount": 6 } })).unwrap();
    call(&mut kit, "create_shape", json!({ "handle": handle, "parent": "1:1", "type": "LINE", "width": 200, "strokes": [{ "type": "SOLID", "color": "#000000" }] })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 3 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn groups_sections_and_duplicates_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    let other = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Other", "x": 200, "y": 0, "width": 40, "height": 40 })).unwrap()["id"].as_str().unwrap().to_string();
    let group = call(&mut kit, "group_nodes", json!({ "handle": handle, "ids": [id, other] })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": group, "y": 300 })).unwrap();
    call(&mut kit, "create_section", json!({ "handle": handle, "parent": "1:1", "width": 100, "height": 100 })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 7 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn sizing_and_constraints_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    let child = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": id, "name": "Child", "x": 60, "y": 10, "width": 30, "height": 20 })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": child, "properties": { "constraints": { "horizontal": "MAX" }, "rotation": 10, "minWidth": 20 } })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "width": 180 } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 6 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn rich_text_comes_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    let text = call(&mut kit, "create_text", json!({ "handle": handle, "parent": id, "fontSize": 16, "paragraphs": [{}, { "listType": "UNORDERED" }],
        "runs": [{ "characters": "Read the " }, { "characters": "terms", "hyperlink": { "type": "URL", "value": "https://example.com" } }, { "characters": "\nItem" }] })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": text, "properties": { "textCase": "UPPER", "fontSize": 18 } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 5 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn texts_estimated_again_come_back_at_the_same_size_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "layoutMode": "VERTICAL", "layoutSizingVertical": "HUG" } })).unwrap();
    call(&mut kit, "create_style", json!({ "handle": handle, "name": "Body", "type": "TEXT", "fontFamily": "Inter", "fontStyle": "Regular", "fontSize": 14, "lineHeight": 20 })).unwrap();
    let text = call(&mut kit, "create_text", json!({ "handle": handle, "parent": id, "characters": "Total" })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": text, "properties": { "textStyle": "Body" } })).unwrap();
    call(&mut kit, "set_style", json!({ "handle": handle, "id": "Body", "fontSize": 22, "lineHeight": 28 })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 8 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn an_instances_own_paint_and_dashes_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "create_component", json!({ "handle": handle, "id": id })).unwrap();
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": id, "x": 200, "y": 0 })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": instance, "properties": { "fills": [{ "type": "SOLID", "color": "#7C3AED" }], "dashPattern": [4, 4] } })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "dashPattern": [2, 2] } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 7 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn instance_names_come_back_after_a_restart() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "create_component", json!({ "handle": handle, "id": id })).unwrap();
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": id, "name": "Plan · Free" })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": instance, "properties": { "name": "Plan · Pro" } })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, recovered) = start(&dir, BASE, true);
    assert_eq!(recovered, Some(json!({ "edits": 6 })));
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn a_fill_is_one_entry_and_comes_back_whole() {
    let dir = Journals::new();
    let (mut kit, handle, _) = start(&dir, BASE, true);
    let card = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Card", "x": 0, "y": 0, "width": 100, "height": 40 })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "create_text", json!({ "handle": handle, "parent": card, "characters": "Title" })).unwrap();
    call(&mut kit, "create_component", json!({ "handle": handle, "id": card })).unwrap();
    let lines = |kit: &Toolkit| dir.text(&journal(kit, &handle, &dir)).unwrap().lines().count();
    let before = lines(&kit);
    call(&mut kit, "fill_template", json!({ "handle": handle, "component": card, "parent": "1:1", "rows": [{ "name": "A" }, { "name": "B" }, { "name": "C" }] })).unwrap();
    assert_eq!(lines(&kit), before + 1, "one fill, one entry");
    let changes = kit.session(&handle).unwrap().fig.node_changes.clone();
    drop(kit);

    let (kit, handle, _) = start(&dir, BASE, true);
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, changes);
}
