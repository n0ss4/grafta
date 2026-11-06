//! The toolkit against real files. Every test skips when its fixture is absent,
//! which it is in a fresh clone.

use std::path::PathBuf;
use std::time::Instant;

use serde_json::{json, Value};
use toolkit::bound::DEFAULT_BUDGET;
use toolkit::tools::ALL;
use toolkit::{Output, Toolkit};

fn fixture(name: &str) -> Option<PathBuf> {
    let path = testkit::fixture::dir().join(name);
    testkit::fixture::bytes(name).map(|_| path)
}

fn call(kit: &mut Toolkit, name: &str, args: Value) -> Value {
    match toolkit::edge::call(kit, name, &args).unwrap_or_else(|e| panic!("{name}: {e}")) {
        Output::Data(value) => toolkit::edge::data::to_json(&value),
        Output::Image { mime, bytes, caption } => json!({ "mime": mime, "len": bytes.len(), "caption": caption }),
    }
}

fn open(kit: &mut Toolkit, path: &PathBuf) -> String {
    call(kit, "open_file", json!({ "path": path }))["handle"].as_str().unwrap().to_string()
}

/// Open once, then find a screen and read it two levels deep, each
/// call after the open in under a second.
#[test]
fn after_opening_a_large_file_every_call_answers_in_under_a_second() {
    let Some(path) = fixture("material3.fig") else { return };
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &path);

    let timed = |kit: &mut Toolkit, name: &str, args: Value| {
        let started = Instant::now();
        let out = call(kit, name, args);
        let seconds = started.elapsed().as_secs_f64();
        assert!(seconds < 1.0, "{name} took {seconds:.2} s");
        out
    };
    let found = timed(&mut kit, "search_nodes", json!({ "handle": handle, "name": "Introduction", "type": "FRAME" }));
    assert!(found["total"].as_u64().unwrap() >= 1, "{found}");
    let id = found["results"][0]["id"].clone();
    let screen = timed(&mut kit, "read_node", json!({ "handle": handle, "id": id, "depth": 2 }));
    assert!(screen["childCount"].as_u64().unwrap() > 0);
    timed(&mut kit, "summarize", json!({ "handle": handle }));
    timed(&mut kit, "list_components", json!({ "handle": handle }));
    timed(&mut kit, "list_variables", json!({ "handle": handle }));

    // Reopening is the same session, not a second read.
    let started = Instant::now();
    assert_eq!(open(&mut kit, &path), handle);
    assert!(started.elapsed().as_secs_f64() < 1.0);
}

/// With default settings, no result on any fixture exceeds the budget,
/// and a result that left something out says so.
#[test]
fn no_default_result_on_any_fixture_exceeds_the_budget() {
    for path in testkit::fixture::all() {
        let mut kit = toolkit::new();
        let handle = open(&mut kit, &path);
        let summary = call(&mut kit, "summarize", json!({ "handle": handle }));
        let first_page = summary["pages"][0]["id"].clone();

        let results = [
            ("summarize", call(&mut kit, "summarize", json!({ "handle": handle }))),
            ("search_nodes", call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "FRAME" }))),
            ("read_node", call(&mut kit, "read_node", json!({ "handle": handle, "id": first_page, "depth": 3 }))),
            ("list_components", call(&mut kit, "list_components", json!({ "handle": handle }))),
            ("list_variables", call(&mut kit, "list_variables", json!({ "handle": handle }))),
        ];
        for (name, out) in results {
            let size = out.to_string().len();
            assert!(
                size <= DEFAULT_BUDGET,
                "{}: {name} returned {size} bytes",
                path.display()
            );
        }

        // Every tool that lists ALL of something a big file has more of than a
        // page holds must say so.
        let frames = call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "FRAME" }));
        if frames["total"].as_u64().unwrap() > 50 {
            assert!(frames.get("next_cursor").is_some(), "{}", path.display());
            assert_eq!(frames["omitted"]["reason"], "page");
        }
    }
}

/// The catalog agrees with the graph it was read from.
#[test]
fn component_and_variable_counts_agree_with_the_graph() {
    let Some(path) = fixture("material3.fig") else { return };
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &path);
    let summary = call(&mut kit, "summarize", json!({ "handle": handle }));
    let variables = call(&mut kit, "list_variables", json!({ "handle": handle, "limit": 1000 }));
    assert_eq!(variables["total"], summary["variables"]);

    let components = call(&mut kit, "list_components", json!({ "handle": handle, "limit": 1000 }));
    let listed = components["total"].as_u64().unwrap();
    assert!(listed > 0);
    assert!(listed <= summary["components"].as_u64().unwrap() + summary["component_sets"].as_u64().unwrap());
}

/// Create on a real file, save, and the saved file opens
/// with the created nodes where they were put. Figma opening it is checked by a person,
/// taken by a person.
#[test]
fn a_file_edited_through_the_tools_saves_and_reads_back() {
    let Some(path) = fixture("gold-preview.fig") else { return };
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &path);

    let component = call(&mut kit, "list_components", json!({ "handle": handle, "limit": 1 }));
    let component = &component["results"][0];
    let component_id = if component["type"] == "COMPONENT_SET" {
        let set = call(&mut kit, "read_node", json!({ "handle": handle, "id": component["id"], "depth": 1 }));
        set["children"][0]["id"].clone()
    } else {
        component["id"].clone()
    };

    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "grafta" }))["id"].clone();
    let frame = call(
        &mut kit,
        "create_frame",
        json!({ "handle": handle, "parent": page, "name": "Screen", "x": 100, "y": 50, "width": 800, "height": 600 }),
    )["id"]
        .clone();
    call(
        &mut kit,
        "create_instance",
        json!({ "handle": handle, "parent": frame, "component": component_id, "x": 24, "y": 24 }),
    );

    let out = std::env::temp_dir().join(format!("grafta-toolkit-{}.fig", std::process::id()));
    std::fs::remove_file(&out).ok();
    let saved = call(&mut kit, "save_as", json!({ "handle": handle, "path": out }));
    assert_eq!(saved["nodes_added"], 3);
    assert_eq!(saved["reread"], "compared equal");

    // A second toolkit reads the saved file and finds the nodes.
    let mut fresh = toolkit::new();
    let reopened = open(&mut fresh, &out);
    let frame_view = call(&mut fresh, "read_node", json!({ "handle": reopened, "id": frame, "depth": 1 }));
    assert_eq!((frame_view["x"].clone(), frame_view["y"].clone()), (json!(100), json!(50)));
    assert_eq!(frame_view["path"], "grafta / Screen");
    assert_eq!(frame_view["children"][0]["componentId"], component_id);
    std::fs::remove_file(&out).ok();
}

/// Every tool is reachable through the one list, with the same name the
/// server will advertise.
#[test]
fn the_list_is_the_only_registry() {
    for tool in ALL {
        assert!(toolkit::edge::find(tool.name()).is_some());
    }
}

/// 100 edits on a large file survive a restart,
/// the file reading the same node for node. Prints what journaling costs per
/// edit and what recovery adds to opening, for the record.
#[test]
fn a_hundred_edits_on_a_large_file_survive_a_restart() {
    let Some(path) = fixture("material3.fig") else { return };
    let dir = std::env::temp_dir().join(format!("grafta-journal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    // The same 100 edits, timed without a journal and then with one.
    let edits = |kit: &mut Toolkit, handle: &str| -> f64 {
        let started = Instant::now();
        for i in 0..50 {
            let frame = call(kit, "create_frame", json!({ "handle": handle, "parent": "54727:23604", "name": format!("J{i}"), "x": i * 10, "y": 0, "width": 8, "height": 8 }));
            call(kit, "set_properties", json!({ "handle": handle, "id": frame["id"], "properties": { "fills": [{ "type": "SOLID", "color": "#FF00AA" }] } }));
        }
        started.elapsed().as_secs_f64()
    };
    let mut plain = toolkit::new();
    let handle = open(&mut plain, &path);
    let without = edits(&mut plain, &handle);
    drop(plain);

    let mut kit = toolkit::with_journal(dir.clone());
    let handle = open(&mut kit, &path);
    let with = edits(&mut kit, &handle);
    let before = figfile::write_fig(&kit.session(&handle).unwrap().fig).unwrap();
    drop(kit);

    let mut kit = toolkit::with_journal(dir.clone());
    let started = Instant::now();
    let out = call(&mut kit, "open_file", json!({ "path": path }));
    let reopen = started.elapsed().as_secs_f64();
    assert_eq!(out["recovered"], json!({ "edits": 100 }), "{out}");
    // Compared as the bytes save_as would write: the file carries NaN sizes,
    // and NaN is not equal to itself, so comparing values would fail on
    // identical files.
    let session = kit.session(out["handle"].as_str().unwrap()).unwrap();
    assert!(figfile::write_fig(&session.fig).unwrap() == before, "the recovered file writes differently");

    let mut fresh = toolkit::new();
    let started = Instant::now();
    open(&mut fresh, &path);
    let open_alone = started.elapsed().as_secs_f64();
    println!(
        "013: 100 edits {without:.3} s without a journal, {with:.3} s with one ({:.2} ms more per edit); \
         reopening with recovery {reopen:.2} s, of which decoding {:.2} s; opening alone {open_alone:.2} s",
        (with - without) * 10.0,
        out["open_seconds"].as_f64().unwrap_or(0.0)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// What 100 edits cost on the largest fixture, for the record before and
/// after the unit of work keeps what each call touches.
#[test]
fn a_hundred_edits_on_nuxtui_cost_as_recorded() {
    let Some(path) = fixture("nuxtui.fig") else { return };
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &path);
    let page = call(&mut kit, "summarize", json!({ "handle": handle }))["pages"][0]["id"].clone();
    let started = Instant::now();
    for i in 0..50 {
        let frame = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": page, "name": format!("U{i}"), "x": i * 10, "y": 0, "width": 8, "height": 8 }));
        call(&mut kit, "set_properties", json!({ "handle": handle, "id": frame["id"], "properties": { "fills": [{ "type": "SOLID", "color": "#FF00AA" }] } }));
    }
    println!("059: 100 edits on nuxtui.fig {:.3} s", started.elapsed().as_secs_f64());
    // A read no longer passes the unit of work.
    let started = Instant::now();
    for _ in 0..100 {
        call(&mut kit, "read_node", json!({ "handle": handle, "id": page }));
    }
    println!("060: 100 reads on nuxtui.fig {:.3} s", started.elapsed().as_secs_f64());
}

/// A new file made from the largest fixture carries none of its
/// nodes and is under 1% of its size.
#[test]
fn a_new_file_from_a_large_file_is_empty_and_small() {
    let Some(from) = fixture("material3.fig") else { return };
    let path = std::env::temp_dir().join(format!("grafta-new-{}.fig", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut kit = toolkit::new();
    let out = call(&mut kit, "new_file", json!({ "path": path, "from": from }));
    assert_eq!(out["nodes"], 2, "{out}");
    assert_eq!(out["components"], 0);
    let template = std::fs::metadata(&from).unwrap().len();
    let written = std::fs::metadata(&path).unwrap().len();
    println!("014: new file {written} bytes from a {template}-byte template");
    assert!(written * 100 < template, "{written} of {template}");
    let _ = std::fs::remove_file(&path);
}

fn picture(kit: &mut Toolkit, handle: &str, id: &Value) -> Vec<u8> {
    match toolkit::edge::call(kit, "render_node", &json!({ "handle": handle, "id": id })).unwrap() {
        Output::Image { bytes, .. } => bytes,
        Output::Data(_) => panic!("render_node gave no picture"),
    }
}

/// What a delete leaves behind stays out of the saved file, and the blob after
/// it moving down an index draws the same layer. The first vector is made
/// first so that the second's blob has to move.
#[test]
fn what_a_delete_leaves_behind_is_not_saved_and_what_stays_draws_the_same() {
    let Some(from) = fixture("gold-preview.fig") else { return };
    let dir = std::env::temp_dir().join(format!("grafta-073-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &from);

    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "073" }))["id"].clone();
    let vector = |kit: &mut Toolkit, path: &str, color: &str, x: i32| {
        let args = json!({ "handle": handle, "parent": page, "path": path, "fills": [{ "type": "SOLID", "color": color }], "x": x, "width": 64, "height": 64 });
        call(kit, "create_vector", args)["id"].clone()
    };
    let gone = vector(&mut kit, "M0 0H10V10H0Z", "#FF0000", 0);
    let kept = vector(&mut kit, "M0 10L5 0L10 10Z", "#0000FF", 100);

    let framed = |kit: &mut Toolkit, source: &Value, name: &str, x: i32| {
        let png = dir.join(name);
        std::fs::write(&png, picture(kit, &handle, source)).unwrap();
        let frame = call(kit, "create_frame", json!({ "handle": handle, "parent": page, "name": name, "x": x, "y": 100, "width": 64, "height": 64 }))["id"].clone();
        call(kit, "set_properties", json!({ "handle": handle, "id": frame, "properties": { "fills": [{ "type": "IMAGE", "image": png }] } }));
        frame
    };
    let gone_image = framed(&mut kit, &gone, "gone.png", 0);
    let kept_image = framed(&mut kit, &kept, "kept.png", 100);

    call(&mut kit, "delete_node", json!({ "handle": handle, "id": gone }));
    call(&mut kit, "delete_node", json!({ "handle": handle, "id": gone_image }));
    let before = (picture(&mut kit, &handle, &kept), picture(&mut kit, &handle, &kept_image));

    let saved = dir.join("saved.fig");
    let out = call(&mut kit, "save_as", json!({ "handle": handle, "path": saved }));
    assert_eq!(out["left_out"]["images"], 1, "{out}");
    assert!(out["left_out"]["blobs"].as_u64().unwrap() >= 1, "{out}");

    let parsed = figfile::parse_fig(&std::fs::read(&saved).unwrap()).unwrap();
    assert!(figfile::without_unused(&parsed).is_none(), "the saved file holds nothing unnamed");

    let again = dir.join("again.fig");
    call(&mut kit, "save_as", json!({ "handle": handle, "path": again }));
    assert_eq!(std::fs::read(&saved).unwrap(), std::fs::read(&again).unwrap(), "the session is as it was");

    let mut reopened = toolkit::new();
    let handle = open(&mut reopened, &saved);
    let after = (picture(&mut reopened, &handle, &kept), picture(&mut reopened, &handle, &kept_image));
    assert!(before == after, "what stays draws the same");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A layer bound to each kind of property saves in the file's own schema, and
/// comes back bound, with an instance's values.
#[test]
fn a_layer_bound_to_a_property_saves_and_comes_back_bound() {
    let Some(from) = fixture("gold-preview.fig") else { return };
    let dir = std::env::temp_dir().join(format!("grafta-079-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &from);

    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "079" }))["id"].clone();
    let frame = |kit: &mut Toolkit, name: &str, x: i32| {
        call(kit, "create_frame", json!({ "handle": handle, "parent": page, "name": name, "x": x, "y": 0, "width": 100, "height": 40 }))["id"].clone()
    };
    let icon = frame(&mut kit, "Icon/A", 0);
    call(&mut kit, "create_component", json!({ "handle": handle, "id": icon }));
    let other = frame(&mut kit, "Icon/B", 120);
    call(&mut kit, "create_component", json!({ "handle": handle, "id": other }));

    let card = frame(&mut kit, "Card", 240);
    let label = call(&mut kit, "create_text", json!({ "handle": handle, "parent": card, "characters": "Label" }))["id"].clone();
    let slot = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": card, "component": icon }))["id"].clone();
    call(&mut kit, "create_component", json!({ "handle": handle, "id": card }));
    for (name, kind, default) in [("Label", "TEXT", json!("Label")), ("Show", "BOOLEAN", json!(true)), ("Icon", "INSTANCE_SWAP", json!("Icon/A"))] {
        call(&mut kit, "add_component_property", json!({ "handle": handle, "component": card, "name": name, "type": kind, "default": default }));
    }
    let bind = |kit: &mut Toolkit, id: &Value, references: Value| {
        call(kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "componentPropertyReferences": references } }));
    };
    bind(&mut kit, &label, json!({ "characters": "Label" }));
    bind(&mut kit, &slot, json!({ "visible": "Show", "mainComponent": "Icon" }));

    let copy = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": page, "component": card, "x": 400 }))["id"].clone();
    let values = json!({ "Label": "Guardar", "Show": false, "Icon": "Icon/B" });
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": copy, "properties": { "componentProperties": values } }));

    let saved = dir.join("saved.fig");
    call(&mut kit, "save_as", json!({ "handle": handle, "path": saved }));

    let mut reopened = toolkit::new();
    let handle = open(&mut reopened, &saved);
    let read = |kit: &mut Toolkit, id: &Value| call(kit, "read_node", json!({ "handle": handle, "id": id, "depth": 0 }));
    assert_eq!(read(&mut reopened, &label)["componentPropertyReferences"], json!({ "characters": "Label" }));
    assert_eq!(read(&mut reopened, &slot)["componentPropertyReferences"], json!({ "visible": "Show", "mainComponent": "Icon" }));
    assert_eq!(read(&mut reopened, &copy)["componentProperties"], json!({ "Label": "Guardar", "Show": "false", "Icon": "Icon/B" }));
    let _ = std::fs::remove_dir_all(&dir);
}

/// Thousands of identical layers pack past what the reader takes in zstd, and
/// have to save all the same.
#[test]
fn a_file_of_identical_layers_saves_and_opens() {
    let Some(from) = fixture("gold-preview.fig") else { return };
    let dir = std::env::temp_dir().join(format!("grafta-094-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &from);
    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "094" }))["id"].clone();
    let frame = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": page, "name": "f", "x": 0, "y": 0, "width": 10, "height": 10 }))["id"].clone();
    for _ in 0..4000 {
        call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": frame }));
    }
    let saved = dir.join("saved.fig");
    call(&mut kit, "save_as", json!({ "handle": handle, "path": saved }));
    let mut reopened = toolkit::new();
    let opened = call(&mut reopened, "open_file", json!({ "path": saved }));
    assert!(opened["nodes"].as_u64().unwrap() > 4000, "{opened}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// On a real file, edits kept up to date incrementally leave the graph a
/// full rebuild gives. Compared by `Debug`, since the file carries NaN sizes
/// and NaN is not equal to itself.
#[test]
fn incremental_edits_on_a_large_file_match_a_full_rebuild() {
    let Some(path) = fixture("material3.fig") else { return };
    let mut kit = toolkit::new();
    let handle = open(&mut kit, &path);
    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "Incremental" }))["id"].clone();
    for i in 0..20 {
        let frame = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": page, "name": format!("F{i}"), "x": i * 50, "y": 0, "width": 40, "height": 40 }))["id"].clone();
        call(&mut kit, "set_properties", json!({ "handle": handle, "id": frame, "properties": { "layoutMode": "VERTICAL", "itemSpacing": 4 } }));
        call(&mut kit, "create_text", json!({ "handle": handle, "parent": frame, "characters": format!("t{i}") }));
    }
    // The file's own screen, and one of its own frames moved.
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": "54727:23605", "properties": { "fills": [{ "type": "SOLID", "color": "#FAFAFA" }] } }));
    call(&mut kit, "move_node", json!({ "handle": handle, "id": "55100:14157", "parent": page }));

    let session = kit.session(&handle).unwrap();
    let rebuilt = model::build_scene_graph(&session.fig).unwrap();
    assert_eq!(session.graph.nodes.len(), rebuilt.nodes.len());
    assert_eq!(session.graph.pages, rebuilt.pages);
    let mut differing = Vec::new();
    for (id, node) in &rebuilt.nodes {
        let kept = session.graph.nodes.get(id);
        if kept != Some(node) && kept.map(|k| format!("{k:?}")) != Some(format!("{node:?}")) {
            differing.push(*id);
        }
    }
    assert!(differing.is_empty(), "{} nodes differ from a rebuild, e.g. {:?}", differing.len(), &differing[..differing.len().min(5)]);
}
