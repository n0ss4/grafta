//! A file opened, edited, journaled, replayed and saved with no disk.
//!
//! Saving needs the schema a file Figma wrote carries, so the round trip runs
//! on a fixture, held in memory, and is skipped without one.
//! The journal and the store's refusals need none: the hand-built document is
//! inserted as a file read from memory would be.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{json, Value};

use crate::port::memory::{FixedFaces, Memory, MemoryJournal};
use crate::edge::journal::recover;
use crate::Base;
use crate::port::{Creating, DocumentStore, ImageSource, Journal, Ports, Rasterizer};
use crate::tests::{call, document};
use crate::Toolkit;

const FIXTURE: &str = "gold-preview.fig";
const FILE: &str = "/memory/gold-preview.fig";
const HAND_BUILT: &str = "/memory/hand-built.fig";
const JOURNALS: &str = "/memory/journals";

fn ports(store: &Arc<Memory>, journal: &Arc<MemoryJournal>) -> Ports {
    crate::port::memory::ports(store, journal, FixedFaces::new([("Inter", &["Regular", "Bold"][..])]))
}

fn toolkit(store: &Arc<Memory>, journal: &Arc<MemoryJournal>) -> Toolkit {
    crate::with_ports(ports(store, journal), Some(PathBuf::from(JOURNALS)))
}

/// The fixture, held in memory, or `None` on a clone without it.
fn fixture_in_memory() -> Option<Arc<Memory>> {
    let bytes = testkit::fixture::bytes(FIXTURE)?;
    Some(Arc::new(Memory::with([(PathBuf::from(FILE), bytes)])))
}

/// A server just started on memory: the file opened, and what it recovered.
fn open(kit: &mut Toolkit, path: &str) -> (String, Value) {
    let opened = call(kit, "open_file", json!({ "path": path })).unwrap();
    (opened["handle"].as_str().unwrap().to_string(), opened)
}

/// A frame on the first page, and its fill.
fn edit(kit: &mut Toolkit, handle: &str) -> String {
    let summary = call(kit, "summarize", json!({ "handle": handle })).unwrap();
    let page = summary["pages"][0]["id"].as_str().unwrap().to_string();
    let frame = call(kit, "create_frame", json!({ "handle": handle, "parent": page, "name": "Kept", "x": 0, "y": 0, "width": 100, "height": 50 })).unwrap();
    let id = frame["id"].as_str().unwrap().to_string();
    call(kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "fills": [{ "type": "SOLID", "color": "#1E88E5" }] } })).unwrap();
    id
}

#[test]
fn a_file_is_edited_journaled_replayed_and_saved_with_no_disk() {
    let Some(store) = fixture_in_memory() else { return };
    let journal = Arc::new(MemoryJournal::default());

    // The same edits, saved in a session of their own, to compare the replay with.
    let mut kit = toolkit(&store, &Arc::new(MemoryJournal::default()));
    let (handle, _) = open(&mut kit, FILE);
    let id = edit(&mut kit, &handle);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/first.fig" })).unwrap();
    drop(kit);

    let mut kit = toolkit(&store, &journal);
    let (handle, _) = open(&mut kit, FILE);
    edit(&mut kit, &handle);
    assert_eq!(journal.all().len(), 1, "one journal, in memory");
    drop(kit);

    let mut kit = toolkit(&store, &journal);
    let (handle, opened) = open(&mut kit, FILE);
    assert_eq!(opened["recovered"]["edits"], 2, "{opened}");
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/replayed.fig" })).unwrap();

    let replayed = store.get(Path::new("/memory/replayed.fig")).unwrap();
    assert_eq!(store.get(Path::new("/memory/first.fig")).unwrap(), replayed, "the replay saves the bytes the edits saved");
    let graph = model::build_scene_graph(&figfile::parse_fig(&replayed).unwrap()).unwrap();
    assert_eq!(graph.node(id.parse().unwrap()).map(|n| n.name.as_str()), Some("Kept"));
    assert_eq!(store.paths().len(), 3, "the file and two saves, nothing else");
}

#[test]
fn restarts_that_recover_leave_the_journal_holding_its_edits_alone() {
    let Some(store) = fixture_in_memory() else { return };
    let journal = Arc::new(MemoryJournal::default());
    let mut kit = toolkit(&store, &journal);
    let (handle, _) = open(&mut kit, FILE);
    edit(&mut kit, &handle);
    drop(kit);

    for restart in 1..=3 {
        let mut kit = toolkit(&store, &journal);
        let (_, opened) = open(&mut kit, FILE);
        assert_eq!(opened["recovered"]["edits"], 2, "restart {restart}: {opened}");
        let entries: Vec<String> = journal.all()[0].1.lines().skip(1).filter(|l| !l.is_empty()).map(str::to_string).collect();
        assert_eq!(entries.len(), 2, "restart {restart}: the header and the two edits, no open_file: {entries:?}");
        assert!(entries.iter().all(|e| !e.contains("\"open_file\"")), "{entries:?}");
    }
}

#[test]
fn saving_where_a_file_is_is_refused_by_what_the_store_holds() {
    let Some(store) = fixture_in_memory() else { return };
    store.write(Path::new("/memory/taken.fig"), b"x").unwrap();
    let mut kit = toolkit(&store, &Arc::new(MemoryJournal::default()));
    let (handle, _) = open(&mut kit, FILE);
    let taken = call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/taken.fig" })).unwrap_err();
    assert!(taken.to_string().contains("a file is already there"), "{taken}");
    let own = call(&mut kit, "save_as", json!({ "handle": handle, "path": FILE })).unwrap_err();
    assert!(own.to_string().contains("the file this session was opened from"), "{own}");
}

#[test]
fn a_new_file_is_saved_where_it_was_made_while_it_is_still_empty() {
    let Some(store) = fixture_in_memory() else { return };
    let journal = Arc::new(MemoryJournal::default());
    let mut kit = toolkit(&store, &journal);
    let made = call(&mut kit, "new_file", json!({ "path": "/memory/made.fig", "from": FILE })).unwrap();
    let handle = made["handle"].as_str().unwrap().to_string();
    call(&mut kit, "create_page", json!({ "handle": handle, "name": "Mine" })).unwrap();
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/made.fig" })).unwrap();
    let saved = crate::port::DocumentStore::read(&*store, Path::new("/memory/made.fig")).unwrap();
    let graph = model::build_scene_graph(&figfile::parse_fig(&saved).unwrap()).unwrap();
    assert_eq!(graph.nodes.values().filter(|n| n.name == "Mine").count(), 1, "the saved file holds the edit");

    // Saved, it holds someone's work: the rule against overwriting is back.
    let again = call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/made.fig" })).unwrap_err();
    assert!(again.to_string().contains("the file this session was opened from"), "{again}");

    // A server started again finds the saved file, with the edit once.
    let mut after = toolkit(&store, &journal);
    let (reopened, _) = open(&mut after, "/memory/made.fig");
    let found = call(&mut after, "search_nodes", json!({ "handle": reopened, "name": "Mine" })).unwrap();
    assert_eq!(found["total"], 1, "{found}");
}

#[test]
fn a_new_file_changed_by_anyone_else_is_not_saved_over() {
    let Some(store) = fixture_in_memory() else { return };
    let mut kit = toolkit(&store, &Arc::new(MemoryJournal::default()));
    let made = call(&mut kit, "new_file", json!({ "path": "/memory/theirs.fig", "from": FILE })).unwrap();
    let handle = made["handle"].as_str().unwrap().to_string();
    store.write(Path::new("/memory/theirs.fig"), b"someone else's work").unwrap();
    let error = call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/theirs.fig" })).unwrap_err();
    assert!(error.to_string().contains("will not write"), "{error}");
    assert_eq!(crate::port::DocumentStore::read(&*store, Path::new("/memory/theirs.fig")).unwrap(), b"someone else's work");
}

/// The hand-built document, opened from memory with a fingerprint.
fn start(journal: &Arc<MemoryJournal>, base: Base) -> (Toolkit, String, Option<Value>) {
    let mut kit = toolkit(&Arc::new(Memory::default()), journal);
    let handle = kit.insert_with_base(PathBuf::from(HAND_BUILT), document(), base).unwrap().handle.to_string();
    let recovered = recover(&mut kit, &handle, true).unwrap().map(|r| crate::edge::data::to_json(&r));
    (kit, handle, recovered)
}

#[test]
fn a_journal_in_memory_replays_and_a_stale_one_is_set_aside() {
    let base = Base { bytes: 1000, fnv: 0xfeed };
    let journal = Arc::new(MemoryJournal::default());
    let (mut kit, handle, _) = start(&journal, base);
    call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Kept", "x": 0, "y": 0, "width": 100, "height": 50 })).unwrap();
    drop(kit);

    let (_, _, recovered) = start(&journal, base);
    assert_eq!(recovered.unwrap()["edits"], 1);

    let (_, _, recovered) = start(&journal, Base { bytes: 1001, fnv: 0xbeef });
    let recovered = recovered.unwrap();
    assert_eq!(recovered["edits"], 0, "{recovered}");
    assert!(journal.all().iter().any(|(path, _)| path.to_string_lossy().contains(".stale-")), "{:?}", journal.all());
}

#[test]
fn the_store_creates_once_and_serves_its_images() {
    let store = Memory::default();
    let path = Path::new("/memory/a.png");
    store.create(path, b"one").unwrap();
    assert!(matches!(store.create(path, b"two"), Err(Creating::Open(_))));
    assert_eq!(ImageSource::read(&store, path).unwrap(), b"one");
    assert!(store.is_file(path));
    store.remove(path).unwrap();
    assert!(!store.exists(path));
    assert!(store.canonical(path).is_err());
}

#[test]
fn the_journal_writes_its_header_once() {
    let journal = MemoryJournal::default();
    let path = Path::new("/memory/j.jsonl");
    journal.append(path, "head", "one").unwrap();
    journal.append(path, "head", "two").unwrap();
    assert_eq!(journal.read(path).as_deref(), Some("head\none\ntwo\n"));
    journal.remove(path);
    assert_eq!(journal.read(path), None);
}

#[test]
fn installed_faces_are_the_table_given() {
    let faces = FixedFaces::new([("Inter", &["Regular", "Bold"][..])]);
    assert_eq!(faces.faces("inter"), Some(vec!["Regular".to_string(), "Bold".to_string()]));
    assert_eq!(faces.faces("Roboto"), None);
}

/// The same edits, journaled, replayed after a restart and saved, on
/// the disk's adapters and on memory's, give the same bytes.
#[test]
fn a_replay_saves_the_same_bytes_through_memory_as_from_disk() {
    let Some(store) = fixture_in_memory() else { return };
    let scratch = std::env::temp_dir().join(format!("grafta-057-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    let on_disk = scratch.join(FIXTURE);
    std::fs::copy(testkit::fixture::dir().join(FIXTURE), &on_disk).unwrap();
    let journals = scratch.join("journals");

    let mut kit = crate::with_journal(journals.clone());
    let (handle, _) = open(&mut kit, on_disk.to_str().unwrap());
    edit(&mut kit, &handle);
    drop(kit);
    let mut kit = crate::with_journal(journals);
    let (handle, opened) = open(&mut kit, on_disk.to_str().unwrap());
    assert_eq!(opened["recovered"]["edits"], 2, "{opened}");
    let saved = scratch.join("replayed.fig");
    call(&mut kit, "save_as", json!({ "handle": handle, "path": saved })).unwrap();
    let from_disk = std::fs::read(&saved).unwrap();

    let journal = Arc::new(MemoryJournal::default());
    let mut kit = toolkit(&store, &journal);
    let (handle, _) = open(&mut kit, FILE);
    edit(&mut kit, &handle);
    drop(kit);
    let mut kit = toolkit(&store, &journal);
    let (handle, _) = open(&mut kit, FILE);
    call(&mut kit, "save_as", json!({ "handle": handle, "path": "/memory/replayed.fig" })).unwrap();

    std::fs::remove_dir_all(&scratch).ok();
    assert_eq!(store.get(Path::new("/memory/replayed.fig")).unwrap(), from_disk);
}
