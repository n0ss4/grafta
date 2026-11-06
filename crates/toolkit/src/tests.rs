//! The tools, end to end, on a file built by hand. No fixture is read: every
//! behaviour here is provable without one.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use figfile::FigFile;
use indexmap::IndexMap;
use kiwi::Value as Raw;
use serde_json::{json, Value};

use testkit::raw::{guid, obj};
use testkit::scene::guid as node_id;
pub(crate) use testkit::raw::document;

use crate::port::disk::Render;
use crate::port::memory::{FixedFaces, Memory, MemoryJournal};
use crate::port::Ports;
use crate::tools::{self, ALL};
use crate::{Output, ToolError, Toolkit};

/// A toolkit on memory: files a test puts there, journals, and the machine's
/// fonts, so a tool's decisions are tested with no disk.
pub(crate) fn kit() -> (Toolkit, String) {
    let mut kit = memory_toolkit();
    let handle = kit.insert(PathBuf::from("/tmp/hand-built.fig"), document()).unwrap().handle.to_string();
    (kit, handle)
}

fn memory_toolkit() -> Toolkit {
    let ports = crate::port::memory::ports(&Arc::new(Memory::default()), &Arc::new(MemoryJournal::default()), FixedFaces::default());
    crate::with_ports(Ports { raster: Arc::new(Render), ..ports }, None)
}

/// `bytes` at `path` in the toolkit's store.
fn put(kit: &Toolkit, path: &str, bytes: impl AsRef<[u8]>) -> String {
    kit.ports().store.write(Path::new(path), bytes.as_ref()).unwrap();
    path.to_string()
}

/// The bytes at `path` in the toolkit's store.
fn stored(kit: &Toolkit, path: &str) -> Option<Vec<u8>> {
    kit.ports().store.read(Path::new(path)).ok()
}

pub(crate) fn call(kit: &mut Toolkit, name: &str, args: Value) -> Result<Value, ToolError> {
    kit.check_rollbacks();
    match crate::edge::call(kit, name, &args)? {
        Output::Data(value) => Ok(crate::edge::data::to_json(&value)),
        Output::Image { mime, bytes, caption } => {
            Ok(json!({ "mime": mime, "len": bytes.len(), "caption": caption }))
        }
    }
}

// ── The list ─────────────────────────────────────────────────────────────────

#[test]
fn every_tool_has_a_unique_name_a_description_and_an_object_schema() {
    let mut names: Vec<&str> = ALL.iter().map(|t| t.name()).collect();
    assert_eq!(names.len(), 43);
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), 43, "tool names must be unique");
    for tool in ALL {
        assert!(tool.description().len() > 40, "{} needs a real description", tool.name());
        assert_eq!(tool.input_schema()["type"], "object", "{}", tool.name());
    }
}

#[test]
fn a_missing_parameter_is_named() {
    let (mut kit, _) = kit();
    let error = call(&mut kit, "summarize", json!({})).unwrap_err();
    assert!(matches!(&error, ToolError::BadArgument(m) if m.contains("\"handle\"")), "{error}");
}

#[test]
fn an_unknown_tool_lists_the_real_ones() {
    let (mut kit, _) = kit();
    let error = call(&mut kit, "paint_everything", json!({})).unwrap_err();
    assert!(error.to_string().contains("read_node"));
}

// ── US1: open, summarise, search ────────────────────────────────────────────

#[test]
fn a_summary_counts_pages_nodes_and_components() {
    let (mut kit, handle) = kit();
    let out = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert_eq!(out["pages"].as_array().unwrap().len(), 2);
    assert_eq!(out["pages"][0]["name"], "Screens");
    assert_eq!(out["pages"][0]["top_level"], 3);
    assert_eq!(out["components"], 3);
    assert_eq!(out["instances"], 2);
    assert_eq!(out["images"], 1);
}

#[test]
fn search_finds_by_name_and_type_with_a_path() {
    let (mut kit, handle) = kit();
    let out = call(&mut kit, "search_nodes", json!({ "handle": handle, "name": "butt" })).unwrap();
    assert_eq!(out["total"], 2);
    let out = call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "INSTANCE" })).unwrap();
    assert_eq!(out["total"], 2);
    assert_eq!(out["results"][0]["path"], "Screens / Button");
    let out = call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "TEXT", "within": "1:2" })).unwrap();
    assert_eq!(out["results"][0]["path"], "Screens / Card / Title");
}

#[test]
fn a_search_with_no_match_says_zero_rather_than_nothing() {
    let (mut kit, handle) = kit();
    let out = call(&mut kit, "search_nodes", json!({ "handle": handle, "name": "nope" })).unwrap();
    assert_eq!(out["total"], 0);
}

#[test]
fn an_unknown_type_or_node_is_refused_with_its_reason() {
    let (mut kit, handle) = kit();
    let error = call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "BANANA" })).unwrap_err();
    assert!(error.to_string().contains("not a node type"));
    let error = call(&mut kit, "read_node", json!({ "handle": handle, "id": "9:9" })).unwrap_err();
    assert!(matches!(&error, ToolError::NotFound(m) if m.contains("9:9")));
}

#[test]
fn a_closed_handle_is_refused_and_says_so() {
    let (mut kit, handle) = kit();
    call(&mut kit, "close_file", json!({ "handle": handle })).unwrap();
    let error = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap_err();
    assert!(error.to_string().contains("closed"));
}

#[test]
fn opening_something_that_is_not_a_fig_says_which() {
    let mut kit = memory_toolkit();
    let error = call(&mut kit, "open_file", json!({ "path": "/definitely/not/here.fig" })).unwrap_err();
    assert!(matches!(error, ToolError::NotFound(_)));

    let text = put(&kit, "/memory/not-a-fig.fig", "hello");
    let error = call(&mut kit, "open_file", json!({ "path": text })).unwrap_err();
    assert!(error.to_string().contains("not a .fig"), "{error}");
}

// ── US2: read exactly ────────────────────────────────────────────────────────

#[test]
fn a_node_reads_back_with_exactly_its_values() {
    let (mut kit, handle) = kit();
    let card = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:2", "depth": 0 })).unwrap();
    assert_eq!(card["type"], "FRAME");
    assert_eq!((card["x"].clone(), card["y"].clone()), (json!(40), json!(60)));
    assert_eq!((card["width"].clone(), card["height"].clone()), (json!(320), json!(200)));
    assert_eq!(card["fills"][0]["color"], "#FF0000");
    assert_eq!(card["fills"][0]["opacity"], 0.5);
    assert_eq!(card["cornerRadius"], 8);
    assert_eq!(card["layoutMode"], "VERTICAL");
    assert_eq!(card["itemSpacing"], 12);
    assert_eq!(card["path"], "Screens / Card");
}

#[test]
fn defaults_are_left_out_and_children_past_the_depth_are_counted() {
    let (mut kit, handle) = kit();
    let card = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:2", "depth": 0 })).unwrap();
    for absent in ["visible", "opacity", "rotation", "strokes", "effects", "locked", "constraints"] {
        assert!(card.get(absent).is_none(), "{absent} is at its default and should be omitted");
    }
    assert_eq!(card["childCount"], 1);
    assert_eq!(card["childrenOmitted"], true);
    assert!(card.get("children").is_none());
}

#[test]
fn text_reads_as_characters_and_a_style_and_its_glyphs_as_undecoded() {
    let (mut kit, handle) = kit();
    let card = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:2", "depth": 1 })).unwrap();
    let title = &card["children"][0];
    assert_eq!(title["characters"], "Hola");
    assert_eq!(title["style"]["fontSize"], 18);
    assert_eq!(title["style"]["fontFamily"], "Inter");

    let title = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:3" })).unwrap();
    assert_eq!(title["undecoded"], json!(["textData (glyphs)"]));
}

#[test]
fn a_field_grafta_does_not_model_is_named_and_reachable_raw() {
    let (mut kit, handle) = kit();
    let card = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:2", "depth": 0 })).unwrap();
    assert_eq!(card["undecoded"], json!(["derivedSomething"]));
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": "1:2", "fields": ["derivedSomething"] })).unwrap();
    assert_eq!(raw, json!({ "derivedSomething": true }));
}

#[test]
fn an_instance_names_its_component() {
    let (mut kit, handle) = kit();
    let button = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:5" })).unwrap();
    assert_eq!(button["componentId"], "1:4");
    assert_eq!(button["componentName"], "Button");
}

#[test]
fn a_budget_cut_ends_at_a_node_and_says_where_to_resume() {
    let (kit, handle) = kit();
    let session = kit.session(&handle).unwrap();
    let page_node = session.graph.node(node_id("1:1")).unwrap();
    // Below the tools' 1 KB floor, so a document this small still gets cut.
    let limits = crate::view::Limits { depth: 3, budget: 300, geometry: false, children_from: 0 };
    let page = crate::view::node(&session.graph, None, page_node, &limits, &session.ports, session.products);

    let shown = page["children"].as_list().unwrap().len();
    assert!((1..3).contains(&shown), "the budget should have cut the page's children, showed {shown}");
    assert_eq!(page["omitted"]["reason"], "budget");
    assert_eq!(page["omitted"]["remaining_children"], 3 - shown);
    assert_eq!(page["omitted"]["resume"]["children_from"], shown);

    let resumed = crate::view::node(
        &session.graph,
        None,
        page_node,
        &crate::view::Limits { depth: 1, budget: 32 * 1024, geometry: false, children_from: shown },
        &session.ports,
        session.products,
    );
    assert_eq!(resumed["children"].as_list().unwrap().len(), 3 - shown);
    assert_eq!(resumed["childrenFrom"], shown);
}

// ── US3: catalog ─────────────────────────────────────────────────────────────

#[test]
fn components_are_listed_with_their_instance_counts() {
    let (mut kit, handle) = kit();
    let out = call(&mut kit, "list_components", json!({ "handle": handle })).unwrap();
    assert_eq!(out["total"], 3);
    assert_eq!(out["results"][0]["name"], "Button");
    assert_eq!(out["results"][0]["instances"], 1);
    assert_eq!(out["results"][0]["page"], "Screens");

    let out = call(&mut kit, "component_instances", json!({ "handle": handle, "id": "1:4" })).unwrap();
    assert_eq!(out["results"][0]["id"], "1:5");
}

#[test]
fn an_image_comes_back_as_an_image() {
    let (mut kit, handle) = kit();
    let out = call(&mut kit, "get_image", json!({ "handle": handle, "hash": "abc123" })).unwrap();
    assert_eq!(out["mime"], "image/png");
    let error = call(&mut kit, "get_image", json!({ "handle": handle, "hash": "zzz" })).unwrap_err();
    assert!(error.to_string().contains("1 images"));
}

// ── US4: create ──────────────────────────────────────────────────────────────

#[test]
fn a_created_frame_and_instance_read_back_at_once() {
    let (mut kit, handle) = kit();
    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "grafta" })).unwrap();
    let page_id = page["id"].as_str().unwrap().to_string();
    let frame = call(
        &mut kit,
        "create_frame",
        json!({ "handle": handle, "parent": page_id, "name": "Screen", "x": 0, "y": 0, "width": 1366, "height": 768 }),
    )
    .unwrap();
    let frame_id = frame["id"].as_str().unwrap().to_string();
    let instance = call(
        &mut kit,
        "create_instance",
        json!({ "handle": handle, "parent": frame_id, "component": "1:4", "x": 24, "y": 24 }),
    )
    .unwrap();
    assert_eq!((instance["width"].clone(), instance["height"].clone()), (json!(96), json!(32)));

    let read = call(&mut kit, "read_node", json!({ "handle": handle, "id": frame_id, "depth": 1 })).unwrap();
    assert_eq!(read["width"], 1366);
    assert_eq!(read["path"], "grafta / Screen");
    assert_eq!(read["children"][0]["componentName"], "Button");
    assert_eq!((read["children"][0]["x"].clone(), read["children"][0]["y"].clone()), (json!(24), json!(24)));

    // Created ids are grafta's.
    for id in [&page_id, &frame_id] {
        assert!(id.starts_with(&format!("{}:", model::author::GRAFTA_SESSION)), "{id}");
    }
    let summary = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert_eq!(summary["unsaved_edits"], 3);
    assert_eq!(summary["pages"].as_array().unwrap().len(), 3);
}

#[test]
fn an_edit_makes_an_earlier_cursor_stale() {
    let (mut kit, handle) = kit();
    let first = call(&mut kit, "search_nodes", json!({ "handle": handle, "limit": 1 })).unwrap();
    call(&mut kit, "create_page", json!({ "handle": handle, "name": "p" })).unwrap();
    let error = call(
        &mut kit,
        "search_nodes",
        json!({ "handle": handle, "limit": 1, "cursor": first["next_cursor"] }),
    )
    .unwrap_err();
    assert!(error.to_string().contains("stale"));
}

#[test]
fn instantiating_anything_but_a_component_is_refused() {
    let (mut kit, handle) = kit();
    let error = call(
        &mut kit,
        "create_instance",
        json!({ "handle": handle, "parent": "1:1", "component": "1:2" }),
    )
    .unwrap_err();
    assert!(matches!(&error, ToolError::Refused(m) if m.contains("not a component")), "{error}");
}

#[test]
fn a_frame_under_a_text_node_is_refused_as_unmeasured() {
    let (mut kit, handle) = kit();
    let error = call(
        &mut kit,
        "create_frame",
        json!({ "handle": handle, "parent": "1:3", "name": "x", "width": 1, "height": 1 }),
    )
    .unwrap_err();
    assert!(matches!(&error, ToolError::Refused(m) if m.contains("measured")), "{error}");
}

#[test]
fn an_argument_a_tool_does_not_take_is_refused_and_nothing_is_done() {
    let (mut kit, handle) = kit();
    let before = kit.session(&handle).unwrap().edits;
    let error = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "F", "width": 10, "height": 10, "fills": [] })).unwrap_err();
    assert!(matches!(&error, ToolError::BadArgument(m) if m.contains("create_frame has no argument \"fills\"") && m.contains("width")), "{error}");
    assert_eq!(kit.session(&handle).unwrap().edits, before);

    let error = call(&mut kit, "render_node", json!({ "handle": handle, "id": "1:1", "path": "/tmp/x.png" })).unwrap_err();
    assert!(error.to_string().contains("render_node has no argument \"path\""), "{error}");

    // What a tool's schema names is still taken.
    call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "F", "width": 10, "height": 10 })).unwrap();
}

#[test]
fn saving_over_an_existing_file_or_without_the_archive_entries_is_refused() {
    let (mut kit, handle) = kit();
    let existing = put(&kit, "/memory/exists.fig", "x");
    let error = call(&mut kit, "save_as", json!({ "handle": handle, "path": existing })).unwrap_err();
    assert!(matches!(&error, ToolError::Refused(m) if m.contains("never overwrites")), "{error}");

    // The hand-built document carries no meta.json or thumbnail.png.
    let fresh = "/memory/fresh-never-written.fig";
    let error = call(&mut kit, "save_as", json!({ "handle": handle, "path": fresh })).unwrap_err();
    assert!(matches!(&error, ToolError::Refused(m) if m.contains("meta.json")), "{error}");
    assert_eq!(stored(&kit, fresh), None);
}

// ── styling ─────────────────────────────────────────────────────────────

/// A frame grafta created on the first page, ready to style.
fn created_frame(kit: &mut Toolkit, handle: &str) -> String {
    let frame = call(
        kit,
        "create_frame",
        json!({ "handle": handle, "parent": "1:1", "name": "Styled", "x": 0, "y": 0, "width": 100, "height": 50 }),
    )
    .unwrap();
    frame["id"].as_str().unwrap().to_string()
}

fn set(kit: &mut Toolkit, handle: &str, id: &str, properties: Value) -> Result<Value, ToolError> {
    call(kit, "set_properties", json!({ "handle": handle, "id": id, "properties": properties }))
}

fn read(kit: &mut Toolkit, handle: &str, id: &str) -> Value {
    call(kit, "read_node", json!({ "handle": handle, "id": id, "depth": 0 })).unwrap()
}

#[test]
fn a_fill_reads_back_as_it_was_set_and_an_empty_list_clears_it() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "color": "#1E88E5", "opacity": 0.5 }] })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["fills"], json!([{ "type": "SOLID", "color": "#1E88E5", "opacity": 0.5 }]));

    set(&mut kit, &handle, &id, json!({ "fills": [] })).unwrap();
    assert!(read(&mut kit, &handle, &id).get("fills").is_none());
}

#[test]
fn an_alpha_in_the_hex_becomes_the_paints_opacity() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "color": "#FF000080" }] })).unwrap();
    let fill = &read(&mut kit, &handle, &id)["fills"][0];
    assert_eq!(fill["color"], "#FF0000");
    assert!((fill["opacity"].as_f64().unwrap() - 128.0 / 255.0).abs() < 1e-3);
}

#[test]
fn a_fill_binds_to_a_local_variable_and_reads_back_with_both_name_and_colour() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "variable": "primary/500" }] })).unwrap();
    let fill = &read(&mut kit, &handle, &id)["fills"][0];
    assert_eq!(fill["variable"], "primary/500");
    assert_eq!(fill["color"], "#1E88E5");

    // And it is written as Figma writes a local alias.
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["fillPaints"] })).unwrap();
    assert_eq!(raw["fillPaints"][0]["colorVar"]["value"]["alias"]["guid"], json!({ "sessionID": 1, "localID": 21 }));
    assert_eq!(raw["fillPaints"][0]["colorVar"]["resolvedDataType"], "COLOR");
}

#[test]
fn every_variable_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    let before = read(&mut kit, &handle, &id);
    let cases = [
        ("primary/50", "Closest: primary/500"),
        ("spacing/4", "not a colour"),
        ("library/blue", "imported from a library"),
    ];
    for (name, reason) in cases {
        let error = set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "variable": name }] })).unwrap_err();
        assert!(error.to_string().contains(reason), "{name}: {error}");
        assert_eq!(read(&mut kit, &handle, &id), before, "{name} changed the node");
    }
}

#[test]
fn strokes_radii_effects_and_opacity_read_back_as_set() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(
        &mut kit,
        &handle,
        &id,
        json!({
            "strokes": [{ "type": "SOLID", "color": "#E5E7EB" }],
            "strokeWeight": 2,
            "strokeAlign": "OUTSIDE",
            "cornerRadius": 8,
            "effects": [{ "type": "DROP_SHADOW", "color": "#000000", "opacity": 0.25, "offset": { "x": 0, "y": 4 }, "radius": 12, "spread": 0 }],
            "opacity": 0.4,
        }),
    )
    .unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["strokes"], json!([{ "type": "SOLID", "color": "#E5E7EB" }]));
    assert_eq!(node["strokeWeight"], 2);
    assert_eq!(node["strokeAlign"], "OUTSIDE");
    assert_eq!(node["cornerRadius"], 8);
    assert_eq!(node["opacity"], 0.4);
    let shadow = &node["effects"][0];
    assert_eq!(shadow["type"], "DROP_SHADOW");
    assert_eq!(shadow["color"], "#000000");
    assert_eq!(shadow["opacity"], 0.25);
    assert_eq!(shadow["offset"], json!({ "x": 0.0, "y": 4.0 }));
    assert_eq!(shadow["radius"], 12.0);
}

#[test]
fn per_corner_radii_read_back_per_corner_and_a_layer_blur_keeps_its_name() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(
        &mut kit,
        &handle,
        &id,
        json!({ "topLeftRadius": 12, "topRightRadius": 12, "effects": [{ "type": "LAYER_BLUR", "radius": 4 }] }),
    )
    .unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!((node["topLeftRadius"].clone(), node["bottomLeftRadius"].clone()), (json!(12), json!(0)));
    assert_eq!(node["effects"][0]["type"], "LAYER_BLUR");

    // Back to uniform: the per-corner fields go.
    set(&mut kit, &handle, &id, json!({ "cornerRadius": 6 })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["cornerRadius"], 6);
    assert!(node.get("topLeftRadius").is_none());
}

#[test]
fn only_the_named_properties_change() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }], "cornerRadius": 4 })).unwrap();
    set(&mut kit, &handle, &id, json!({ "opacity": 0.5 })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["fills"][0]["color"], "#FFFFFF");
    assert_eq!(node["cornerRadius"], 4);
    assert_eq!(node["opacity"], 0.5);
}

#[test]
fn every_value_refusal_names_what_is_allowed_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    let before = read(&mut kit, &handle, &id);
    let cases = [
        (json!({ "cornerradius": 4 }), "accepts"),
        (json!({ "fills": [{ "type": "PATTERN", "color": "#FFFFFF" }] }), "expected SOLID, GRADIENT_LINEAR, GRADIENT_RADIAL or IMAGE"),
        (json!({ "fills": [{ "type": "SOLID", "color": "blue" }] }), "#RRGGBB"),
        (json!({ "fills": [{ "type": "SOLID" }] }), "needs a color or a variable"),
        (json!({ "cornerRadius": -1 }), "at least 0"),
        (json!({ "opacity": 2 }), "from 0 to 1"),
        (json!({ "strokeWeight": 0 }), "greater than 0"),
        (json!({ "strokeAlign": "MIDDLE" }), "INSIDE, CENTER or OUTSIDE"),
        (json!({ "effects": [{ "type": "GLOW", "radius": 1 }] }), "DROP_SHADOW"),
        (json!({ "effects": [{ "type": "DROP_SHADOW" }] }), "radius is required"),
        // A valid property beside an invalid one: the valid one must not land.
        (json!({ "cornerRadius": 20, "opacity": 7 }), "from 0 to 1"),
    ];
    for (properties, reason) in cases {
        let error = set(&mut kit, &handle, &id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
        assert_eq!(read(&mut kit, &handle, &id), before, "{properties} changed the node");
    }
}

#[test]
fn the_files_own_frames_and_the_layers_of_its_components_can_be_styled() {
    let (mut kit, handle) = kit();
    // Card (1:2) is the file's own, on a page: it can be styled now.
    set(&mut kit, &handle, "1:2", json!({ "opacity": 0.5 })).unwrap();
    assert_eq!(read(&mut kit, &handle, "1:2")["opacity"], 0.5);

    // A layer inside a component can too, since 024: its instances' caches
    // are dropped for Figma to derive again.
    set(&mut kit, &handle, "1:31", json!({ "visible": false })).unwrap();

    let page = call(&mut kit, "create_page", json!({ "handle": handle, "name": "p" })).unwrap();
    let error = set(&mut kit, &handle, page["id"].as_str().unwrap(), json!({ "opacity": 0.5 })).unwrap_err();
    assert!(error.to_string().contains("only frames, components, instances, text layers and vectors"), "{error}");
}

// ── editing what you created ────────────────────────────────────────────

#[test]
fn geometry_name_visibility_and_clipping_read_back_as_set() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(
        &mut kit,
        &handle,
        &id,
        json!({ "x": 12, "y": 34, "width": 200, "height": 80, "name": "Card", "visible": false, "clipsContent": true }),
    )
    .unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!((node["x"].clone(), node["y"].clone()), (json!(12), json!(34)));
    assert_eq!((node["width"].clone(), node["height"].clone()), (json!(200), json!(80)));
    assert_eq!(node["name"], "Card");
    assert_eq!(node["visible"], false);
    assert_eq!(node["clipsContent"], true);

    set(&mut kit, &handle, &id, json!({ "clipsContent": false, "visible": true, "width": 150 })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert!(node.get("clipsContent").is_none());
    assert!(node.get("visible").is_none());
    assert_eq!((node["width"].clone(), node["height"].clone()), (json!(150), json!(80)));
}

#[test]
fn an_instance_moves_and_hides_but_takes_no_radius_or_effects_of_its_own() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": frame, "component": "1:4" })).unwrap();
    let id = instance["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &id, json!({ "x": 5, "y": 6, "name": "Play", "visible": false })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!((node["x"].clone(), node["y"].clone(), node["name"].clone()), (json!(5), json!(6), json!("Play")));

    // Its own paint and dashes are stored here; its own radius and effects are not
    // written yet.
    for refused in [json!({ "cornerRadius": 10 }), json!({ "effects": [] })] {
        let error = set(&mut kit, &handle, &id, refused.clone()).unwrap_err();
        assert!(error.to_string().contains("not accepted on an instance"), "{refused}: {error}");
    }
}

#[test]
fn a_refused_key_names_the_node_it_was_sent_to_and_not_an_instance() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let vector = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": frame, "path": "M0 0 L10 10" })).unwrap()["id"].as_str().unwrap().to_string();
    let shape = call(&mut kit, "create_shape", json!({ "handle": handle, "parent": frame, "type": "RECTANGLE", "width": 10, "height": 10 })).unwrap()["id"].as_str().unwrap().to_string();
    for (id, refused, is) in [
        (frame.as_str(), json!({ "characters": "x" }), "FRAME"),
        (vector.as_str(), json!({ "clipsContent": true }), "VECTOR"),
        (shape.as_str(), json!({ "characters": "x" }), "RECTANGLE"),
    ] {
        let error = set(&mut kit, &handle, id, refused.clone()).unwrap_err().to_string();
        assert!(error.contains(&format!("is not accepted on a {is} node. Here it accepts:")), "{refused}: {error}");
        assert!(!error.contains("instance"), "{refused}: {error}");
    }
}

#[test]
fn a_slide_refusing_a_texts_key_names_the_slide() {
    let Some((mut kit, handle)) = deck() else { return };
    let slide = slides(&mut kit, &handle)[0].clone();
    let error = call(&mut kit, "set_properties", json!({ "handle": handle, "id": slide, "properties": { "characters": "x" } })).unwrap_err().to_string();
    assert!(error.starts_with("`characters` is not accepted on a SLIDE node. Here it accepts:"), "{error}");
}

#[test]
fn a_zero_size_is_refused_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    let before = read(&mut kit, &handle, &id);
    let error = set(&mut kit, &handle, &id, json!({ "x": 99, "width": 0 })).unwrap_err();
    assert!(error.to_string().contains("greater than 0"), "{error}");
    assert_eq!(read(&mut kit, &handle, &id), before);
}

#[test]
fn deleting_answers_a_tree_breadth_first_in_the_files_order() {
    let (mut kit, handle) = kit();
    let frame = |kit: &mut Toolkit, parent: &str, name: &str| {
        let out = call(kit, "create_frame", json!({ "handle": handle, "parent": parent, "name": name, "width": 10, "height": 10 })).unwrap();
        out["id"].as_str().unwrap().to_string()
    };
    let top = created_frame(&mut kit, &handle);
    let a = frame(&mut kit, &top, "a");
    let b = frame(&mut kit, &top, "b");
    let under_a = frame(&mut kit, &a, "under a");
    let under_b = frame(&mut kit, &b, "under b");
    let out = call(&mut kit, "delete_node", json!({ "handle": handle, "id": top })).unwrap();
    assert_eq!(out["removed"], json!([top, a, b, under_a, under_b]));
}

#[test]
fn deleting_a_node_and_one_inside_it_removes_each_once() {
    let (mut kit, handle) = kit();
    let top = created_frame(&mut kit, &handle);
    let inner = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": top, "name": "inner", "width": 10, "height": 10 })).unwrap();
    let inner = inner["id"].as_str().unwrap().to_string();
    let out = call(&mut kit, "delete_node", json!({ "handle": handle, "id": top, "also": [inner] })).unwrap();
    assert_eq!(out["removed"], json!([top, inner]));
}

#[test]
fn deleting_removes_the_node_and_everything_inside_it() {
    let (mut kit, handle) = kit();
    let outer = created_frame(&mut kit, &handle);
    let inner = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": outer, "name": "inner", "width": 10, "height": 10 })).unwrap();
    let inner = inner["id"].as_str().unwrap().to_string();
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": inner, "component": "1:4" })).unwrap();
    let instance = instance["id"].as_str().unwrap().to_string();

    let out = call(&mut kit, "delete_node", json!({ "handle": handle, "id": outer })).unwrap();
    let mut removed: Vec<String> = out["removed"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    removed.sort();
    let mut expected = vec![outer.clone(), inner.clone(), instance.clone()];
    expected.sort();
    assert_eq!(removed, expected);
    for id in [&outer, &inner, &instance] {
        let error = call(&mut kit, "read_node", json!({ "handle": handle, "id": id })).unwrap_err();
        assert!(matches!(error, ToolError::NotFound(_)));
        assert!(call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).is_err(), "{id} is still in the file");
    }
    let page = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:1", "depth": 0 })).unwrap();
    assert_eq!(page["childCount"], 3, "the page is back to its own three children");
}

#[test]
fn the_files_own_nodes_can_be_deleted_unless_something_points_at_them() {
    let (mut kit, handle) = kit();
    // Card (1:2) is referenced by nothing: it goes, with its title.
    let out = call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:2" })).unwrap();
    assert_eq!(out["removed"], json!(["1:2", "1:3"]));

    // A component its instance points at is refused, naming the instance.
    let error = call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:4" })).unwrap_err();
    assert!(error.to_string().contains("1:5"), "{error}");

    // A page whose icon component an instance elsewhere points at is refused,
    // naming who points at it.
    let frame = created_frame(&mut kit, &handle);
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": frame, "component": "1:30" })).unwrap();
    let error = call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:6" })).unwrap_err();
    let message = error.to_string();
    assert!(message.contains(instance["id"].as_str().unwrap()) && message.contains("symbolData.symbolID"), "{message}");

    // The file's own node can be moved.
    call(&mut kit, "move_node", json!({ "handle": handle, "id": "1:5", "parent": "1:6", "x": 0, "y": 0 })).unwrap();
    assert_eq!(read(&mut kit, &handle, "1:5")["path"], "Notes / Button");
}

#[test]
fn where_a_node_used_to_live_does_not_keep_its_old_parent() {
    // Figma records, on a component moved out of a deleted frame, the path it
    // came from (`ancestorPathBeforeDeletion`): history for its undo, not a
    // link. It must not keep that old parent from being deleted.
    let mut file = document();
    let Raw::Object(message) = &mut file.node_changes else { unreachable!() };
    let Some((_, Raw::Array(changes))) = message.iter_mut().find(|(k, _)| *k == "nodeChanges") else { unreachable!() };
    changes.push(testkit::raw::node(
        50,
        Some((1, "~")),
        "SYMBOL",
        "Moved",
        vec![
            ("size", testkit::raw::size(24.0, 24.0)),
            ("ancestorPathBeforeDeletion", Raw::Array(vec![guid(1, 6), guid(1, 30)])),
        ],
    ));
    // A frame detached from a component there keeps its id, as Figma does.
    changes.push(testkit::raw::node(
        51,
        Some((1, "~~")),
        "FRAME",
        "Detached",
        vec![("size", testkit::raw::size(24.0, 24.0)), ("detachedSymbolId", obj(vec![("guid", guid(1, 30))]))],
    ));
    let mut kit = memory_toolkit();
    let handle = kit.insert(PathBuf::from("/tmp/history.fig"), file).unwrap().handle.to_string();

    let out = call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:6" })).unwrap();
    assert!(out["removed"].as_array().unwrap().contains(&json!("1:30")), "{out}");
    assert_eq!(read(&mut kit, &handle, "1:50")["name"], "Moved", "the moved component stays");
    assert_eq!(read(&mut kit, &handle, "1:51")["name"], "Detached", "the detached frame stays");
}

#[test]
fn nodes_that_point_at_each_other_go_together() {
    let (mut kit, handle) = kit();
    // The page Notes holds the icon an instance on another page points at:
    // alone it is refused, with that instance it goes.
    let frame = created_frame(&mut kit, &handle);
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": frame, "component": "1:30" })).unwrap();
    let instance = instance["id"].as_str().unwrap().to_string();
    assert!(call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:6" })).is_err());
    let out = call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:6", "also": [instance] })).unwrap();
    let removed = out["removed"].as_array().unwrap();
    assert!(removed.contains(&json!("1:30")) && removed.contains(&json!(instance)), "{out}");

    // Every page at once is refused: a file keeps one.
    let error = call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:1", "also": [] })).unwrap_err();
    assert!(error.to_string().contains("only page"), "{error}");
}

#[test]
fn a_child_of_an_auto_layout_frame_is_placed_by_it() {
    let (mut kit, handle) = kit();
    let parent = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &parent, json!({ "layoutMode": "VERTICAL" })).unwrap();
    let child = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": parent, "name": "c", "width": 10, "height": 10 })).unwrap();
    let child = child["id"].as_str().unwrap().to_string();
    let error = set(&mut kit, &handle, &child, json!({ "x": 50 })).unwrap_err();
    assert!(error.to_string().contains("placed by its auto-layout parent"), "{error}");
    // Once absolute, it can be placed by hand.
    set(&mut kit, &handle, &child, json!({ "layoutPositioning": "ABSOLUTE" })).unwrap();
    set(&mut kit, &handle, &child, json!({ "x": 50 })).unwrap();
}

#[test]
fn a_moved_node_takes_its_children_to_its_new_parent() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let child = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": frame, "name": "child", "width": 10, "height": 10 })).unwrap();
    let child = child["id"].as_str().unwrap().to_string();

    call(&mut kit, "move_node", json!({ "handle": handle, "id": frame, "parent": "1:6", "x": 7, "y": 8 })).unwrap();
    let moved = call(&mut kit, "read_node", json!({ "handle": handle, "id": frame, "depth": 1 })).unwrap();
    assert_eq!(moved["path"], "Notes / Styled");
    assert_eq!((moved["x"].clone(), moved["y"].clone()), (json!(7), json!(8)));
    assert_eq!(moved["children"][0]["id"], child);
    let old_page = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:1", "depth": 0 })).unwrap();
    assert_eq!(old_page["childCount"], 3);
    let new_page = call(&mut kit, "read_node", json!({ "handle": handle, "id": "1:6", "depth": 1 })).unwrap();
    assert_eq!(new_page["children"].as_array().unwrap().last().unwrap()["id"], frame);
}

#[test]
fn a_shape_or_vector_moves_behind_in_front_of_or_into_another_frame() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let title = call(&mut kit, "create_text", json!({ "handle": handle, "parent": frame, "characters": "Hola", "fontSize": 12 })).unwrap()["id"].as_str().unwrap().to_string();
    // A glow drawn after the text lands on top of it, as in 068.
    let glow = call(&mut kit, "create_shape", json!({ "handle": handle, "parent": frame, "type": "ELLIPSE", "width": 60, "height": 40, "x": 5, "y": 5 })).unwrap()["id"].as_str().unwrap().to_string();
    let out = call(&mut kit, "move_node", json!({ "handle": handle, "id": glow, "behind": title })).unwrap();
    assert_eq!((out["index"].as_u64(), out["of"].as_u64()), (Some(0), Some(2)), "the glow first, the text drawn over it: {out}");
    assert_eq!(read(&mut kit, &handle, &glow)["width"], 60, "its size is its own");

    let vector = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": frame, "path": "M0 0 L10 10" })).unwrap()["id"].as_str().unwrap().to_string();
    for kind in ["RECTANGLE", "LINE", "POLYGON", "STAR"] {
        let mut args = json!({ "handle": handle, "parent": frame, "type": kind, "width": 10, "height": 10 });
        if kind == "LINE" {
            args.as_object_mut().unwrap().remove("height");
            args["strokes"] = json!([{ "type": "SOLID", "color": "#000000" }]);
        }
        let shape = call(&mut kit, "create_shape", args).unwrap()["id"].as_str().unwrap().to_string();
        call(&mut kit, "move_node", json!({ "handle": handle, "id": shape, "in_front_of": glow })).unwrap();
    }
    let other = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Other", "x": 300, "y": 0, "width": 50, "height": 50 })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "move_node", json!({ "handle": handle, "id": vector, "parent": other, "x": 3, "y": 4 })).unwrap();
    let moved = read(&mut kit, &handle, &vector);
    assert!(moved["path"].as_str().unwrap().contains("Other"), "{moved}");
    assert_eq!((moved["x"].as_f64(), moved["y"].as_f64()), (Some(3.0), Some(4.0)));
}

#[test]
fn a_component_set_moves_into_a_frame_but_not_into_a_component() {
    let (mut kit, handle) = kit();
    let (set, exact, _) = badge_set(&mut kit, &handle);
    let frame = created_frame(&mut kit, &handle);
    call(&mut kit, "move_node", json!({ "handle": handle, "id": set, "parent": frame })).unwrap();
    assert!(read(&mut kit, &handle, &exact)["path"].as_str().unwrap().contains("Styled / Badge"), "its variants go with it");

    let host = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Host", "x": 400, "y": 0, "width": 50, "height": 50 })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "create_component", json!({ "handle": handle, "id": host })).unwrap();
    let error = call(&mut kit, "move_node", json!({ "handle": handle, "id": set, "parent": host })).unwrap_err();
    assert!(error.to_string().contains("component"), "{error}");
}

#[test]
fn a_group_is_still_not_moved_and_the_refusal_says_what_is() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let a = call(&mut kit, "create_shape", json!({ "handle": handle, "parent": frame, "type": "RECTANGLE", "width": 10, "height": 10 })).unwrap()["id"].as_str().unwrap().to_string();
    let b = call(&mut kit, "create_shape", json!({ "handle": handle, "parent": frame, "type": "ELLIPSE", "width": 10, "height": 10 })).unwrap()["id"].as_str().unwrap().to_string();
    let group = call(&mut kit, "group_nodes", json!({ "handle": handle, "ids": [a, b] })).unwrap()["id"].as_str().unwrap().to_string();
    let error = call(&mut kit, "move_node", json!({ "handle": handle, "id": group, "parent": "1:1" })).unwrap_err();
    assert!(error.to_string().contains("shapes, vectors"), "{error}");
}

#[test]
fn a_node_cannot_be_moved_into_itself() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let child = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": frame, "name": "child", "width": 10, "height": 10 })).unwrap();
    let error = call(&mut kit, "move_node", json!({ "handle": handle, "id": frame, "parent": child["id"] })).unwrap_err();
    assert!(error.to_string().contains("is inside"), "{error}");
}

// ── text ────────────────────────────────────────────────────────────────

fn text(kit: &mut Toolkit, handle: &str, args: Value) -> Result<Value, ToolError> {
    let mut args = args;
    args["handle"] = json!(handle);
    call(kit, "create_text", args)
}

#[test]
fn a_text_reads_back_with_its_characters_font_colour_and_place() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let out = text(
        &mut kit,
        &handle,
        json!({ "parent": frame, "characters": "Recientes", "fontFamily": "Inter", "fontStyle": "Bold", "fontSize": 24,
                "fills": [{ "type": "SOLID", "color": "#FFFFFF" }], "x": 24, "y": 32, "lineHeight": 32 }),
    )
    .unwrap();
    assert!(out["size"]["provisional"].is_string(), "an estimated size must say so: {out}");
    let id = out["id"].as_str().unwrap().to_string();

    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["type"], "TEXT");
    assert_eq!(node["characters"], "Recientes");
    assert_eq!(node["name"], "Recientes");
    assert_eq!(node["style"]["fontFamily"], "Inter");
    assert_eq!(node["style"]["fontStyle"], "Bold");
    assert_eq!(node["style"]["fontSize"], 24);
    assert_eq!(node["style"]["lineHeight"], json!({ "value": 32.0, "unit": "PIXELS" }));
    assert_eq!(node["style"]["textAutoResize"], "WIDTH_AND_HEIGHT");
    assert_eq!(node["fills"][0]["color"], "#FFFFFF");
    assert_eq!((node["x"].clone(), node["y"].clone()), (json!(24), json!(32)));

    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    assert!(raw.get("derivedTextData").is_none(), "grafta never writes layout data");
    assert_eq!(raw["fontVersion"], "");
}

#[test]
fn each_paragraph_is_a_line_and_a_width_makes_the_text_wrap() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Uno\nDos\nTres", "width": 200 })).unwrap();
    let id = out["id"].as_str().unwrap().to_string();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData", "textAutoResize"] })).unwrap();
    assert_eq!(raw["textData"]["lines"].as_array().unwrap().len(), 3);
    assert_eq!(raw["textData"]["lines"][0]["lineType"], "PLAIN");
    assert_eq!(raw["textAutoResize"], "HEIGHT");
    assert_eq!(out["size"]["width"], 200);
}

#[test]
fn without_a_font_a_text_takes_the_files_own() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Hola" })).unwrap();
    // The hand-built file's only text is Inter Bold 18.
    assert_eq!(out["font"], json!({ "family": "Inter", "style": "Bold", "size": 18 }));
}

#[test]
fn a_text_colour_binds_to_a_local_variable() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Token", "fills": [{ "type": "SOLID", "variable": "primary/500" }] })).unwrap();
    let node = read(&mut kit, &handle, out["id"].as_str().unwrap());
    assert_eq!(node["fills"][0]["variable"], "primary/500");
}

#[test]
fn an_empty_or_badly_styled_text_is_refused() {
    let (mut kit, handle) = kit();
    for (args, reason) in [
        (json!({ "parent": "1:1", "characters": "" }), "non-empty"),
        (json!({ "parent": "1:1", "characters": "x", "fontSize": 0 }), "greater than 0"),
        (json!({ "parent": "1:1", "characters": "x", "textAlignHorizontal": "MIDDLE" }), "LEFT, CENTER"),
        (json!({ "parent": "1:1", "characters": "x", "lineHeight": { "value": 20, "unit": "EM" } }), "PIXELS or PERCENT"),
    ] {
        let error = text(&mut kit, &handle, args.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{args}: {error}");
    }
}

#[test]
fn a_created_text_can_be_edited_and_a_refused_edit_changes_nothing() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Antes", "fontSize": 16 })).unwrap();
    let id = out["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &id, json!({ "characters": "Después\ncon dos líneas", "fontSize": 20, "textAlignHorizontal": "CENTER", "x": 50 })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["characters"], "Después\ncon dos líneas");
    assert_eq!(node["style"]["fontSize"], 20);
    assert_eq!(node["style"]["textAlignHorizontal"], "CENTER");
    assert_eq!(node["x"], 50);

    let before = read(&mut kit, &handle, &id);
    for (properties, reason) in [
        (json!({ "fontSize": -2 }), "greater than 0"),
        (json!({ "characters": "" }), "non-empty"),
        (json!({ "clipsContent": true }), "not accepted on a text layer"),
    ] {
        let error = set(&mut kit, &handle, &id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
        assert_eq!(read(&mut kit, &handle, &id), before, "{properties} changed the text");
    }
}

#[test]
fn editing_a_text_drops_layout_data_that_would_be_stale() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Hola" })).unwrap();
    let id = out["id"].as_str().unwrap().to_string();
    // Pretend Figma laid it out once, as a saved and reopened file would show.
    {
        let session = kit.session_mut(&handle).unwrap();
        let position = session.raw_index[&node_id(&id)];
        let kiwi::Value::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(kiwi::Value::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let kiwi::Value::Object(fields) = &mut changes[position] else { unreachable!() };
        fields.insert("derivedTextData", kiwi::Value::Object(kiwi::Fields::new()));
    }
    set(&mut kit, &handle, &id, json!({ "characters": "Adiós" })).unwrap();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    assert!(raw.get("derivedTextData").is_none());
}

// ── gradients ───────────────────────────────────────────────────────────

fn two_stops() -> Value {
    json!([{ "position": 0, "color": "#8B5CF6" }, { "position": 1, "color": "#EC4899", "opacity": 0.5 }])
}

#[test]
fn a_linear_gradient_reads_back_with_its_stops_and_direction() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "GRADIENT_LINEAR", "angle": 180, "gradientStops": two_stops() }] })).unwrap();
    let fill = &read(&mut kit, &handle, &id)["fills"][0];
    assert_eq!(fill["type"], "GRADIENT_LINEAR");
    assert_eq!(fill["gradientStops"][0], json!({ "position": 0.0, "color": "#8B5CF6" }));
    assert_eq!(fill["gradientStops"][1]["color"], "#EC4899");
    assert_eq!(fill["gradientStops"][1]["opacity"], 0.5);
    // 180 degrees is the matrix measured on a real top-to-bottom fade.
    assert_eq!(fill["gradientTransform"], json!([[0.0, 1.0, 0.0], [-1.0, 0.0, 1.0]]));

    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["fillPaints"] })).unwrap();
    assert_eq!(raw["fillPaints"][0]["stopsVar"], json!([]));
}

#[test]
fn ninety_degrees_is_left_to_right_and_radial_is_centred() {
    use crate::tools::style::angle_matrix;
    assert_eq!(angle_matrix(90.0), [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
    assert_eq!(angle_matrix(180.0), [[0.0, 1.0, 0.0], [-1.0, 0.0, 1.0]]);
    assert_eq!(angle_matrix(270.0), [[-1.0, 0.0, 1.0], [0.0, -1.0, 1.0]]);

    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "GRADIENT_RADIAL", "gradientStops": two_stops() }] })).unwrap();
    let fill = &read(&mut kit, &handle, &id)["fills"][0];
    assert_eq!(fill["type"], "GRADIENT_RADIAL");
    assert_eq!(fill["gradientTransform"], json!([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]));
}

#[test]
fn a_gradient_read_back_can_be_written_back_unchanged() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "GRADIENT_LINEAR", "angle": 135, "gradientStops": two_stops() }] })).unwrap();
    let first = read(&mut kit, &handle, &id)["fills"].clone();
    set(&mut kit, &handle, &id, json!({ "fills": first.clone() })).unwrap();
    assert_eq!(read(&mut kit, &handle, &id)["fills"], first);
}

#[test]
fn a_text_can_be_painted_with_a_gradient() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Neon", "fills": [{ "type": "GRADIENT_LINEAR", "angle": 90, "gradientStops": two_stops() }] })).unwrap();
    let node = read(&mut kit, &handle, out["id"].as_str().unwrap());
    assert_eq!(node["fills"][0]["type"], "GRADIENT_LINEAR");
}

#[test]
fn every_gradient_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    let before = read(&mut kit, &handle, &id);
    for (paint, reason) in [
        (json!({ "type": "GRADIENT_LINEAR", "gradientStops": [{ "position": 0, "color": "#000000" }] }), "at least two stops"),
        (json!({ "type": "GRADIENT_LINEAR", "gradientStops": [{ "position": 0, "color": "#000000" }, { "position": 2, "color": "#FFFFFF" }] }), "from 0 to 1"),
        (json!({ "type": "GRADIENT_LINEAR", "angle": "down", "gradientStops": two_stops() }), "expected a number"),
        (json!({ "type": "GRADIENT_RADIAL", "angle": 90, "gradientStops": two_stops() }), "only a linear gradient has an angle"),
        (json!({ "type": "GRADIENT_LINEAR", "angle": 90, "gradientTransform": [[1, 0, 0], [0, 1, 0]], "gradientStops": two_stops() }), "not both"),
        (json!({ "type": "GRADIENT_LINEAR", "gradientTransform": [[1, 0]], "gradientStops": two_stops() }), "[[a, b, c], [d, e, f]]"),
        (json!({ "type": "GRADIENT_LINEAR", "color": "#000000", "gradientStops": two_stops() }), "is not accepted"),
    ] {
        let error = set(&mut kit, &handle, &id, json!({ "fills": [paint.clone()] })).unwrap_err();
        assert!(error.to_string().contains(reason), "{paint}: {error}");
        assert_eq!(read(&mut kit, &handle, &id), before, "{paint} changed the node");
    }
}

// ── instance overrides ──────────────────────────────────────────────────

fn instance_of(kit: &mut Toolkit, handle: &str, component: &str) -> String {
    let frame = created_frame(kit, handle);
    let out = call(kit, "create_instance", json!({ "handle": handle, "parent": frame, "component": component })).unwrap();
    out["id"].as_str().unwrap().to_string()
}

fn overrides(kit: &mut Toolkit, handle: &str, id: &str, layer: Value, properties: Value) -> Result<Value, ToolError> {
    let mut args = json!({ "handle": handle, "id": id, "properties": properties });
    if !layer.is_null() {
        args["layer"] = layer;
    }
    call(kit, "override_instance", args)
}

fn raw_overrides(kit: &mut Toolkit, handle: &str, id: &str) -> Value {
    call(kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["symbolData"] })).unwrap()["symbolData"]["symbolOverrides"].clone()
}

#[test]
fn recolouring_an_icon_overrides_every_vector_layer_and_nothing_else() {
    let (mut kit, handle) = kit();
    let id = instance_of(&mut kit, &handle, "1:30");
    let out = overrides(&mut kit, &handle, &id, Value::Null, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    assert_eq!(out["layers"], json!(["1:31", "1:32"]), "the background rectangle must not be painted");

    let entries = raw_overrides(&mut kit, &handle, &id);
    assert_eq!(entries.as_array().unwrap().len(), 2);
    // A layer's key is its overrideKey (2:31 for Shape) or its own guid (1:32 for Accent).
    assert_eq!(entries[0]["guidPath"]["guids"], json!([{ "sessionID": 2, "localID": 31 }]));
    assert_eq!(entries[1]["guidPath"]["guids"], json!([{ "sessionID": 1, "localID": 32 }]));
    // Shape had no style, so the entry is the minimal { guidPath, fillPaints }.
    assert!(entries[0].get("styleIdForFill").is_none());
    // Accent used a colour style, which the override detaches, as Figma does.
    assert_eq!(entries[1]["styleIdForFill"]["guid"], json!({ "sessionID": 4294967295u32, "localID": 4294967295u32 }));

    let node = call(&mut kit, "read_node", json!({ "handle": handle, "id": id, "depth": 0 })).unwrap();
    assert_eq!(node["overrides"][0]["layer"], "1:31");
    assert_eq!(node["overrides"][0]["layerName"], "Shape");
    assert_eq!(node["overrides"][0]["fills"][0]["color"], "#FFFFFF");
}

#[test]
fn one_layer_a_label_and_a_hidden_part_can_be_overridden() {
    let (mut kit, handle) = kit();
    let id = instance_of(&mut kit, &handle, "1:40");
    overrides(&mut kit, &handle, &id, json!("1:41"), json!({ "characters": "Nuevo", "fills": [{ "type": "SOLID", "color": "#22D3EE" }] })).unwrap();
    overrides(&mut kit, &handle, &id, json!("1:42"), json!({ "visible": false })).unwrap();
    let node = call(&mut kit, "read_node", json!({ "handle": handle, "id": id, "depth": 0 })).unwrap();
    let label = node["overrides"].as_array().unwrap().iter().find(|o| o["layer"] == "1:41").unwrap();
    assert_eq!(label["characters"], "Nuevo");
    assert_eq!(label["fills"][0]["color"], "#22D3EE");
    let icon = node["overrides"].as_array().unwrap().iter().find(|o| o["layer"] == "1:42").unwrap();
    assert_eq!(icon["visible"], false);
}

#[test]
fn a_layer_inside_a_nested_instance_is_addressed_by_a_path() {
    let (mut kit, handle) = kit();
    let id = instance_of(&mut kit, &handle, "1:40");
    overrides(&mut kit, &handle, &id, json!(["1:42", "1:31"]), json!({ "fills": [{ "type": "SOLID", "color": "#EC4899" }] })).unwrap();
    let entries = raw_overrides(&mut kit, &handle, &id);
    assert_eq!(entries[0]["guidPath"]["guids"], json!([{ "sessionID": 2, "localID": 42 }, { "sessionID": 2, "localID": 31 }]));

    // And recolouring the chip without a layer reaches through the nested icon.
    let out = overrides(&mut kit, &handle, &id, Value::Null, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    assert_eq!(out["layers"], json!(["1:42 › 1:31", "1:42 › 1:32"]));
}

#[test]
fn overriding_the_same_layer_twice_merges_into_one_entry() {
    let (mut kit, handle) = kit();
    let id = instance_of(&mut kit, &handle, "1:40");
    overrides(&mut kit, &handle, &id, json!("1:41"), json!({ "characters": "Uno" })).unwrap();
    overrides(&mut kit, &handle, &id, json!("1:41"), json!({ "characters": "Dos", "visible": true })).unwrap();
    let entries = raw_overrides(&mut kit, &handle, &id);
    assert_eq!(entries.as_array().unwrap().len(), 1);
    assert_eq!(entries[0]["textData"]["characters"], "Dos");
    assert_eq!(entries[0]["visible"], true);
}

#[test]
fn every_override_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = instance_of(&mut kit, &handle, "1:40");
    let before = raw_overrides(&mut kit, &handle, &id);
    for (layer, properties, reason) in [
        (json!("1:31"), json!({ "visible": false }), "not a layer of component Chip"),
        (json!("1:42"), json!({ "characters": "x" }), "not a text layer"),
        (json!(["1:41", "1:31"]), json!({ "visible": false }), "only a nested instance can continue a path"),
        (Value::Null, json!({ "characters": "x" }), "need a layer"),
        (json!("1:41"), json!({ "width": 10 }), "cannot be overridden"),
        (json!("1:41"), json!({ "fills": [{ "type": "SOLID", "color": "red" }] }), "#RRGGBB"),
    ] {
        let error = overrides(&mut kit, &handle, &id, layer.clone(), properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{layer} {properties}: {error}");
        assert_eq!(raw_overrides(&mut kit, &handle, &id), before, "{properties} changed the overrides");
    }
    // The file's own instance of a component with no vector layers: nothing to
    // recolour, and it says so.
    let error = overrides(&mut kit, &handle, "1:5", Value::Null, json!({ "fills": [] })).unwrap_err();
    assert!(error.to_string().contains("no vector layers"), "{error}");
}

#[test]
fn a_recolour_never_adds_paint_to_a_layer_that_had_none() {
    let (mut kit, handle) = kit();
    let id = instance_of(&mut kit, &handle, "1:30");
    // Only Shape and Accent have fills; Bounds has no paint at all.
    let out = overrides(&mut kit, &handle, &id, Value::Null, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    assert_eq!(out["layers"], json!(["1:31", "1:32"]));

    // No layer of this icon has strokes, so a stroke recolour has nothing to
    // change, and says where to look instead.
    let error = overrides(&mut kit, &handle, &id, Value::Null, json!({ "strokes": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap_err();
    assert!(error.to_string().contains("drawn with fills"), "{error}");
}

// ── auto-layout ─────────────────────────────────────────────────────────

#[test]
fn auto_layout_reads_back_as_set_and_defaults_are_left_out() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(
        &mut kit,
        &handle,
        &id,
        json!({
            "layoutMode": "HORIZONTAL", "itemSpacing": 12, "padding": 16, "paddingLeft": 24,
            "primaryAxisSizingMode": "FIXED", "counterAxisSizingMode": "AUTO",
            "primaryAxisAlignItems": "SPACE_BETWEEN", "counterAxisAlignItems": "CENTER",
        }),
    )
    .unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["layoutMode"], "HORIZONTAL");
    assert_eq!(node["itemSpacing"], 12);
    assert_eq!((node["paddingTop"].clone(), node["paddingRight"].clone(), node["paddingBottom"].clone(), node["paddingLeft"].clone()), (json!(16), json!(16), json!(16), json!(24)));
    assert_eq!(node["primaryAxisSizingMode"], "FIXED");
    assert_eq!(node["counterAxisSizingMode"], "AUTO");
    assert_eq!(node["primaryAxisAlignItems"], "SPACE_BETWEEN");
    assert_eq!(node["counterAxisAlignItems"], "CENTER");

    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    assert_eq!(raw["stackPrimaryAlignItems"], "SPACE_EVENLY", "space between is stored as Figma stores it");
    assert_eq!(raw["stackCounterSizing"], "RESIZE_TO_FIT_WITH_IMPLICIT_SIZE");

    // Back to the defaults: the fields go, as Figma omits them.
    set(&mut kit, &handle, &id, json!({ "primaryAxisSizingMode": "AUTO", "primaryAxisAlignItems": "MIN", "layoutWrap": "NO_WRAP" })).unwrap();
    let error = set(&mut kit, &handle, &id, json!({ "layoutWrap": "WRAP" })).unwrap_err();
    assert!(error.to_string().contains("not laid out by grafta yet"), "{error}");
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    for field in ["stackPrimarySizing", "stackPrimaryAlignItems", "stackWrap"] {
        assert!(raw.get(field).is_none(), "{field} is at its default and must be omitted");
    }
    assert_eq!(read(&mut kit, &handle, &id)["primaryAxisSizingMode"], "AUTO");

    set(&mut kit, &handle, &id, json!({ "layoutMode": "NONE" })).unwrap();
    assert!(read(&mut kit, &handle, &id).get("layoutMode").is_none());
}

#[test]
fn a_child_grows_stretches_and_floats_in_its_parents_layout() {
    let (mut kit, handle) = kit();
    let parent = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &parent, json!({ "layoutMode": "VERTICAL" })).unwrap();
    let child = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": parent, "name": "c", "width": 10, "height": 10 })).unwrap();
    let child = child["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &child, json!({ "layoutGrow": 1, "layoutAlign": "STRETCH", "layoutPositioning": "ABSOLUTE" })).unwrap();
    let node = read(&mut kit, &handle, &child);
    assert_eq!(node["layoutGrow"], 1);
    assert_eq!(node["layoutAlign"], "STRETCH");
    assert_eq!(node["layoutPositioning"], "ABSOLUTE");

    // A text and an instance take the child-side keys too.
    let text = call(&mut kit, "create_text", json!({ "handle": handle, "parent": parent, "characters": "t" })).unwrap();
    set(&mut kit, &handle, text["id"].as_str().unwrap(), json!({ "layoutAlign": "STRETCH" })).unwrap();
}

#[test]
fn a_layout_value_outside_its_set_is_refused_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    let before = read(&mut kit, &handle, &id);
    for (properties, reason) in [
        (json!({ "layoutMode": "GRID" }), "NONE, HORIZONTAL, VERTICAL"),
        (json!({ "itemSpacing": -4 }), "at least 0"),
        (json!({ "layoutGrow": 2 }), "0 or 1"),
        (json!({ "primaryAxisAlignItems": "BASELINE" }), "SPACE_BETWEEN"),
        (json!({ "layoutMode": "VERTICAL", "padding": -1 }), "at least 0"),
    ] {
        let error = set(&mut kit, &handle, &id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
        assert_eq!(read(&mut kit, &handle, &id), before, "{properties} changed the node");
    }
}

#[test]
fn resizing_the_files_own_frame_drops_geometry_figma_derived_for_the_old_size() {
    let (mut kit, handle) = kit();
    {
        let session = kit.session_mut(&handle).unwrap();
        let position = session.raw_index[&node_id("1:2")];
        let kiwi::Value::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(kiwi::Value::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let kiwi::Value::Object(fields) = &mut changes[position] else { unreachable!() };
        fields.insert("fillGeometry", kiwi::Value::Array(Vec::new()));
    }
    set(&mut kit, &handle, "1:2", json!({ "width": 500 })).unwrap();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": "1:2" })).unwrap();
    assert!(raw.get("fillGeometry").is_none());
    assert_eq!(raw["size"]["x"], 500.0);
}

// ── seeing ──────────────────────────────────────────────────────────────

/// Draw a node, returning its PNG and caption.
fn render(kit: &mut Toolkit, args: Value) -> Result<(Vec<u8>, Value), ToolError> {
    match crate::edge::call(kit, "render_node", &args)? {
        Output::Image { mime, bytes, caption } => {
            assert_eq!(mime, "image/png");
            Ok((bytes, crate::edge::data::to_json(&caption.expect("a drawn node has a caption"))))
        }
        Output::Data(value) => panic!("expected a picture, got {value}"),
    }
}

fn pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).unwrap();
    let p = pixmap.pixel(x, y).unwrap().demultiply();
    [p.red(), p.green(), p.blue(), p.alpha()]
}

#[test]
fn a_board_is_drawn_with_a_caption_naming_what_is_approximate() {
    let (mut kit, handle) = kit();
    let (png, caption) = render(&mut kit, json!({ "handle": handle, "id": "1:2" })).unwrap();
    assert_eq!((caption["width"].clone(), caption["height"].clone()), (json!(320), json!(200)));
    assert_eq!(caption["scale"], 1);
    assert_eq!(caption["name"], "Card");
    // The card is red at half opacity; its title has no layout Figma stored.
    assert_eq!(pixel(&png, 300, 190), [255, 0, 0, 128]);
    let notes = caption["notes"].as_array().unwrap();
    assert!(notes.iter().any(|n| n.as_str().unwrap().contains("laid out")), "{notes:?}");
}

#[test]
fn a_board_larger_than_max_size_is_scaled_down_and_says_so() {
    let (mut kit, handle) = kit();
    let (_, caption) = render(&mut kit, json!({ "handle": handle, "id": "1:2", "max_size": 64 })).unwrap();
    assert_eq!(caption["width"], 64);
    assert_eq!(caption["scale"], 0.2);
}

#[test]
fn drawing_an_unknown_node_a_node_without_area_or_a_bad_scale_is_refused() {
    let (mut kit, handle) = kit();
    let unknown = render(&mut kit, json!({ "handle": handle, "id": "9:9" })).unwrap_err();
    assert!(matches!(unknown, ToolError::NotFound(_)), "{unknown}");
    let flat = render(&mut kit, json!({ "handle": handle, "id": "1:1" })).unwrap_err();
    assert!(matches!(flat, ToolError::BadArgument(ref m) if m.contains("no area")), "{flat}");
    let scale = render(&mut kit, json!({ "handle": handle, "id": "1:2", "scale": 10 })).unwrap_err();
    assert!(matches!(scale, ToolError::BadArgument(_)), "{scale}");
}

#[test]
fn a_change_is_drawn_before_it_is_saved() {
    let (mut kit, handle) = kit();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "color": "#1E88E5" }] })).unwrap();
    let (png, caption) = render(&mut kit, json!({ "handle": handle, "id": id })).unwrap();
    assert_eq!(pixel(&png, 50, 25), [0x1E, 0x88, 0xE5, 255]);
    assert_eq!(caption["notes"], json!([]));

    let text = call(&mut kit, "create_text", json!({ "handle": handle, "parent": id, "characters": "Hola" })).unwrap();
    let (_, caption) = render(&mut kit, json!({ "handle": handle, "id": text["id"] })).unwrap();
    assert!(caption["notes"][0].as_str().unwrap().contains("laid out"), "{caption}");
}

// ── a new file ──────────────────────────────────────────────────────────

/// The hand-built document as a template: with the two entries Figma needs,
/// a blob and an image.
fn template() -> FigFile {
    let mut fig = document();
    let meta = json!({ "client_meta": { "background_color": { "r": 0.0, "g": 0.0, "b": 1.0, "a": 1.0 }, "thumbnail_size": { "width": 40, "height": 24 }, "render_coordinates": { "x": 5 } }, "file_name": "Someone's design" });
    fig.other_entries = vec![
        ("thumbnail.png".into(), b"the template's picture".to_vec()),
        ("meta.json".into(), meta.to_string().into_bytes()),
    ];
    fig.blobs = vec![vec![1, 2, 3]];
    if let Raw::Object(message) = &mut fig.node_changes {
        message.insert("blobs", Raw::Array(vec![obj(vec![("bytes", Raw::Str("blob".into()))])]));
    }
    fig
}

#[test]
fn a_new_file_keeps_the_templates_format_and_none_of_its_content() {
    let fig = tools::new_file::blank(&crate::port::on_disk(), crate::product::all(), template(), "spotify-nuevo", "Inicio").unwrap();
    let changes = model::author::node_changes(&fig);
    let types: Vec<&str> = changes.iter().filter_map(|c| match c.get("type") { Some(Raw::Enum(t)) => Some(*t), _ => None }).collect();
    assert_eq!(types, ["DOCUMENT", "CANVAS"]);
    assert_eq!(changes[1].get("name"), Some(&Raw::Str("Inicio".into())));
    assert!(fig.blobs.is_empty() && fig.images.is_empty());
    assert_eq!(fig.node_changes.get("blobs"), Some(&Raw::Array(Vec::new())), "blobs are in the message too");

    let entry = |name: &str| fig.other_entries.iter().find(|(n, _)| n == name).unwrap().1.clone();
    let meta: Value = serde_json::from_slice(&entry("meta.json")).unwrap();
    assert_eq!(meta["file_name"], "spotify-nuevo");
    assert!(meta["client_meta"].get("render_coordinates").is_none());
    let thumbnail = resvg::tiny_skia::Pixmap::decode_png(&entry("thumbnail.png")).unwrap();
    assert_eq!((thumbnail.width(), thumbnail.height()), (40, 24));
    assert_eq!(pixel(&entry("thumbnail.png"), 0, 0), [0, 0, 255, 255]);

    // It is a file like any other: grafta can build on it.
    let mut kit = crate::new();
    let handle = kit.insert(PathBuf::from("/tmp/new.fig"), fig).unwrap().handle.to_string();
    let summary = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert_eq!(summary["pages"][0]["name"], "Inicio");
    assert_eq!(summary["components"], 0);
    let page = summary["pages"][0]["id"].clone();
    call(&mut kit, "create_frame", json!({ "handle": handle, "parent": page, "name": "Home", "x": 0, "y": 0, "width": 390, "height": 844 })).unwrap();
}

#[test]
fn a_template_figma_would_refuse_and_a_path_that_exists_are_refused() {
    let mut fig = template();
    fig.other_entries.retain(|(name, _)| name != "thumbnail.png");
    let Err(error) = tools::new_file::blank(&crate::port::on_disk(), crate::product::all(), fig, "x", "Page 1") else { panic!("a template without a thumbnail was accepted") };
    assert!(matches!(error, ToolError::Refused(ref m) if m.contains("thumbnail.png")), "{error}");

    let mut kit = memory_toolkit();
    let existing = put(&kit, "/memory/new-exists.fig", b"x");
    let error = call(&mut kit, "new_file", json!({ "path": existing, "from": existing })).unwrap_err();
    assert!(matches!(error, ToolError::Refused(_)), "{error}");
    let error = call(&mut kit, "new_file", json!({ "path": "/tmp/new.txt", "from": existing })).unwrap_err();
    assert!(matches!(error, ToolError::BadArgument(_)), "{error}");
}

// ── Incremental edits ───────────────────────────────────────────────────

/// Every kind of edit, one after another, kept up to date incrementally, must
/// leave the same graph and index a full rebuild of the file gives.
#[test]
fn edits_kept_up_to_date_incrementally_match_a_full_rebuild() {
    let (mut kit, handle) = kit();
    let frame = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "A", "x": 0, "y": 0, "width": 200, "height": 100 })).unwrap()["id"].clone();
    let second = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "B", "x": 300, "y": 0, "width": 50, "height": 50 })).unwrap()["id"].clone();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": frame, "properties": { "layoutMode": "VERTICAL", "itemSpacing": 8, "fills": [{ "type": "SOLID", "color": "#FF0000" }] } })).unwrap();
    call(&mut kit, "create_text", json!({ "handle": handle, "parent": frame, "characters": "one" })).unwrap();
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": frame, "component": "1:4" })).unwrap()["id"].clone();
    call(&mut kit, "create_page", json!({ "handle": handle, "name": "More" })).unwrap();
    call(&mut kit, "move_node", json!({ "handle": handle, "id": second, "parent": frame })).unwrap();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": "1:2", "properties": { "cornerRadius": 12 } })).unwrap();
    call(&mut kit, "delete_node", json!({ "handle": handle, "id": instance })).unwrap();
    call(&mut kit, "create_frame", json!({ "handle": handle, "parent": frame, "name": "C", "x": 0, "y": 0, "width": 10, "height": 10 })).unwrap();

    let session = kit.session(&handle).unwrap();
    let rebuilt = model::build_scene_graph(&session.fig).unwrap();
    for (id, node) in &rebuilt.nodes {
        assert_eq!(session.graph.nodes.get(id), Some(node), "{id} differs from a rebuild");
    }
    assert_eq!(session.graph.nodes.len(), rebuilt.nodes.len());
    assert_eq!(session.graph.pages, rebuilt.pages);
    let fresh: IndexMap<schema::Guid, usize> = model::author::node_changes(&session.fig)
        .iter()
        .enumerate()
        .filter_map(|(at, nc)| Some((crate::raw_id_of(nc)?, at)))
        .collect();
    assert_eq!(session.raw_index.len(), fresh.len());
    for (id, at) in &fresh {
        assert_eq!(session.raw_index.get(id), Some(at), "{id} indexed at the wrong position");
    }
}

// ── layer order ─────────────────────────────────────────────────────────

/// A frame holding three squares, "Back", "Middle" and "Front", in that order.
fn three_layers(kit: &mut Toolkit, handle: &str) -> (String, [String; 3]) {
    let frame = call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Stack", "x": 0, "y": 0, "width": 100, "height": 100 })).unwrap()["id"].as_str().unwrap().to_string();
    let square = |kit: &mut Toolkit, name: &str| {
        call(kit, "create_frame", json!({ "handle": handle, "parent": frame, "name": name, "x": 0, "y": 0, "width": 50, "height": 50 })).unwrap()["id"].as_str().unwrap().to_string()
    };
    let layers = [square(kit, "Back"), square(kit, "Middle"), square(kit, "Front")];
    (frame, layers)
}

fn names(kit: &Toolkit, handle: &str, frame: &str) -> Vec<String> {
    let session = kit.session(handle).unwrap();
    session.graph.children(node_id(frame)).map(|n| n.name.clone()).collect()
}

#[test]
fn a_layer_goes_behind_in_front_of_to_the_back_and_to_the_front() {
    let (mut kit, handle) = kit();
    let (frame, [back, middle, front]) = three_layers(&mut kit, &handle);

    let out = call(&mut kit, "move_node", json!({ "handle": handle, "id": front, "behind": back })).unwrap();
    assert_eq!(names(&kit, &handle, &frame), ["Front", "Back", "Middle"]);
    assert_eq!((out["index"].clone(), out["of"].clone()), (json!(0), json!(3)));

    call(&mut kit, "move_node", json!({ "handle": handle, "id": front, "in_front_of": back })).unwrap();
    assert_eq!(names(&kit, &handle, &frame), ["Back", "Front", "Middle"]);

    call(&mut kit, "move_node", json!({ "handle": handle, "id": middle, "parent": frame, "to": "back" })).unwrap();
    assert_eq!(names(&kit, &handle, &frame), ["Middle", "Back", "Front"]);

    call(&mut kit, "move_node", json!({ "handle": handle, "id": middle, "parent": frame })).unwrap();
    assert_eq!(names(&kit, &handle, &frame), ["Back", "Front", "Middle"]);

    // Again and again behind the same layer, as a model layering glows would.
    for _ in 0..20 {
        call(&mut kit, "move_node", json!({ "handle": handle, "id": middle, "behind": front })).unwrap();
        call(&mut kit, "move_node", json!({ "handle": handle, "id": back, "in_front_of": middle })).unwrap();
    }
    assert_eq!(names(&kit, &handle, &frame), ["Middle", "Back", "Front"]);

    // Only the moved nodes' keys changed, and the graph is what the file says.
    let session = kit.session(&handle).unwrap();
    let rebuilt = model::build_scene_graph(&session.fig).unwrap();
    assert_eq!(rebuilt.node(node_id(&frame)).unwrap().children, session.graph.node(node_id(&frame)).unwrap().children);
}

#[test]
fn a_layer_put_next_to_a_node_in_another_parent_moves_there_keeping_its_place_on_screen() {
    let (mut kit, handle) = kit();
    let (frame, [back, _, _]) = three_layers(&mut kit, &handle);
    let other = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Other", "x": 0, "y": 0, "width": 40, "height": 40 })).unwrap()["id"].as_str().unwrap().to_string();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": other, "properties": { "x": 7, "y": 9 } })).unwrap();
    call(&mut kit, "move_node", json!({ "handle": handle, "id": other, "in_front_of": back })).unwrap();
    assert_eq!(names(&kit, &handle, &frame), ["Back", "Other", "Middle", "Front"]);
    let node = kit.session(&handle).unwrap().graph.node(node_id(&other)).unwrap();
    assert_eq!((node.x, node.y), (7.0, 9.0));
}

#[test]
fn in_an_auto_layout_frame_the_order_is_the_flow() {
    let (mut kit, handle) = kit();
    let (frame, [back, _, front]) = three_layers(&mut kit, &handle);
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": frame, "properties": { "layoutMode": "VERTICAL", "itemSpacing": 10 } })).unwrap();
    let y = |kit: &Toolkit, id: &str| kit.session(&handle).unwrap().graph.node(node_id(id)).unwrap().y;
    assert_eq!(y(&kit, &front), 120.0);
    call(&mut kit, "move_node", json!({ "handle": handle, "id": front, "behind": back })).unwrap();
    assert_eq!((y(&kit, &front), y(&kit, &back)), (0.0, 60.0), "laid out again in the new order");
}

#[test]
fn a_move_that_names_no_place_or_itself_or_its_own_inside_is_refused() {
    let (mut kit, handle) = kit();
    let (frame, [back, middle, _]) = three_layers(&mut kit, &handle);
    let none = call(&mut kit, "move_node", json!({ "handle": handle, "id": back })).unwrap_err();
    assert!(matches!(none, ToolError::BadArgument(_)), "{none}");
    let two = call(&mut kit, "move_node", json!({ "handle": handle, "id": back, "behind": middle, "parent": frame })).unwrap_err();
    assert!(matches!(two, ToolError::BadArgument(_)), "{two}");
    let itself = call(&mut kit, "move_node", json!({ "handle": handle, "id": back, "behind": back })).unwrap_err();
    assert!(matches!(itself, ToolError::BadArgument(_)), "{itself}");
    let inside = call(&mut kit, "move_node", json!({ "handle": handle, "id": frame, "behind": back })).unwrap_err();
    assert!(matches!(inside, ToolError::Refused(_)), "{inside}");
    let to = call(&mut kit, "move_node", json!({ "handle": handle, "id": back, "parent": frame, "to": "top" })).unwrap_err();
    assert!(matches!(to, ToolError::BadArgument(_)), "{to}");
}

// ── vectors ─────────────────────────────────────────────────────────────

/// Lucide's "check", 24 units, drawn at 48 px with a 4 px round stroke.
fn lucide_check(kit: &mut Toolkit, handle: &str) -> Value {
    call(kit, "create_vector", json!({
        "handle": handle, "parent": "1:1", "name": "Check", "path": "M20 6 9 17l-5-5",
        "viewBox": [0, 0, 24, 24], "width": 48, "x": 10, "y": 20,
        "strokes": [{ "type": "SOLID", "color": "#1E88E5" }], "strokeWeight": 4, "strokeCap": "ROUND", "strokeJoin": "ROUND",
    }))
    .unwrap()
}

#[test]
fn a_line_icon_is_a_network_scaled_to_its_size_and_nothing_derived() {
    let (mut kit, handle) = kit();
    let out = lucide_check(&mut kit, &handle);
    assert_eq!(out["size"], json!({ "width": 48, "height": 48 }));
    let id = out["id"].as_str().unwrap();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    assert!(raw.get("fillGeometry").is_none() && raw.get("strokeGeometry").is_none(), "{raw}");
    assert_eq!(raw["strokeCap"], "ROUND");
    assert_eq!(raw["vectorData"]["normalizedSize"], json!({ "x": 48, "y": 48 }));

    let session = kit.session(&handle).unwrap();
    let blob = session.raw(node_id(id)).unwrap().get("vectorData").and_then(|v| v.get("vectorNetworkBlob")).and_then(Raw::as_u32).unwrap();
    let network = model::path::Network::decode(&session.fig.blobs[blob as usize]).unwrap();
    // 24 → 48: every point doubled.
    assert_eq!(network.vertices, [(40.0, 12.0), (18.0, 34.0), (8.0, 24.0)]);
    assert!(network.regions.is_empty(), "an open line has no region");
    assert_eq!(session.fig.node_changes.get("blobs").and_then(Raw::as_array).map(Vec::len), Some(session.fig.blobs.len()));
}

#[test]
fn a_vector_is_drawn_from_its_network_stroke_and_fill() {
    let (mut kit, handle) = kit();
    let id = lucide_check(&mut kit, &handle)["id"].clone();
    let (png, caption) = render(&mut kit, json!({ "handle": handle, "id": id })).unwrap();
    assert_eq!(caption["notes"], json!([]));
    // On the stroke from (18, 34) to (40, 12), and clear of it in a corner.
    assert_eq!(pixel(&png, 29, 23), [0x1E, 0x88, 0xE5, 255]);
    assert_eq!(pixel(&png, 44, 44)[3], 0);

    let square = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "path": "M0 0H10V10H0Z", "fills": [{ "type": "SOLID", "color": "#FF0000" }], "width": 20 })).unwrap();
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": square["id"] })).unwrap();
    assert_eq!(pixel(&png, 10, 10), [255, 0, 0, 255]);
}

#[test]
fn a_vector_is_restyled_and_resized_and_the_network_scales_with_it() {
    let (mut kit, handle) = kit();
    let id = lucide_check(&mut kit, &handle)["id"].clone();
    call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": {
        "width": 24, "height": 24, "strokes": [{ "type": "SOLID", "color": "#FF0000" }], "strokeWeight": 2, "strokeCap": "SQUARE",
    } })).unwrap();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    assert_eq!((raw["size"]["x"].clone(), raw["strokeCap"].clone()), (json!(24), json!("SQUARE")));
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": id })).unwrap();
    // The stroke's midpoint, (14.5, 11.5) at half the normalized size.
    assert_eq!(pixel(&png, 14, 11), [255, 0, 0, 255], "the network drawn at half its normalized size");
    let error = call(&mut kit, "set_properties", json!({ "handle": handle, "id": id, "properties": { "cornerRadius": 4 } })).unwrap_err();
    assert!(matches!(error, ToolError::BadArgument(_)), "{error}");
}

#[test]
fn path_data_that_cannot_be_drawn_and_a_box_with_no_area_are_refused() {
    let (mut kit, handle) = kit();
    let bad = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "path": "M0 0 X1 1" })).unwrap_err();
    assert!(matches!(bad, ToolError::BadArgument(ref m) if m.contains("path")), "{bad}");
    let flat = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "path": "M5 12h14" })).unwrap_err();
    assert!(matches!(flat, ToolError::BadArgument(ref m) if m.contains("viewBox")), "{flat}");
    let line = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "path": "M5 12h14", "viewBox": [0, 0, 24, 24] })).unwrap();
    assert_eq!(line["size"], json!({ "width": 24, "height": 24 }));
    let cap = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "path": "M0 0L1 1", "strokeCap": "BUTT" })).unwrap_err();
    assert!(matches!(cap, ToolError::BadArgument(_)), "{cap}");
}

#[test]
fn each_path_in_a_list_starts_afresh_as_an_svg_path_element_does() {
    let (mut kit, handle) = kit();
    // Lucide's search: a circle, then a handle drawn with a relative move.
    let out = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "viewBox": [0, 0, 24, 24], "path": ["M3 11a8 8 0 1 0 16 0a8 8 0 1 0-16 0", "m21 21-4.3-4.3"] })).unwrap();
    let session = kit.session(&handle).unwrap();
    let id = out["id"].as_str().unwrap();
    let blob = session.raw(node_id(id)).unwrap().get("vectorData").and_then(|v| v.get("vectorNetworkBlob")).and_then(Raw::as_u32).unwrap();
    let network = model::path::Network::decode(&session.fig.blobs[blob as usize]).unwrap();
    assert!(network.vertices.contains(&(21.0, 21.0)), "{:?}", network.vertices);
}

// ── components ──────────────────────────────────────────────────────────

/// A violet chip frame with a label, turned into a component.
fn chip_component(kit: &mut Toolkit, handle: &str) -> (String, String) {
    let chip = call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Chip", "x": 0, "y": 0, "width": 100, "height": 32 })).unwrap()["id"].as_str().unwrap().to_string();
    call(kit, "set_properties", json!({ "handle": handle, "id": chip, "properties": { "fills": [{ "type": "SOLID", "color": "#7C3AED" }], "cornerRadius": 16 } })).unwrap();
    let label = call(kit, "create_text", json!({ "handle": handle, "parent": chip, "characters": "Etiqueta", "x": 12, "y": 6 })).unwrap()["id"].as_str().unwrap().to_string();
    let out = call(kit, "create_component", json!({ "handle": handle, "id": chip, "name": "Chip/Default" })).unwrap();
    assert_eq!((out["type"].clone(), out["name"].clone(), out["children"].clone()), (json!("COMPONENT"), json!("Chip/Default"), json!(1)));
    (chip, label)
}

#[test]
fn a_frame_becomes_a_component_in_place_with_the_fields_every_local_one_has() {
    let (mut kit, handle) = kit();
    let (chip, label) = chip_component(&mut kit, &handle);
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": chip })).unwrap();
    assert_eq!(raw["type"], "SYMBOL");
    assert_eq!(raw["isSymbolPublishable"], true);
    assert!(raw["sharedSymbolVersion"].as_str().is_some_and(|v| v.split(':').count() == 2), "{raw}");
    let listed = call(&mut kit, "list_components", json!({ "handle": handle, "name": "Chip/Default" })).unwrap();
    assert_eq!(listed["results"][0]["id"], chip);
    // Its children are where they were.
    assert_eq!(kit.session(&handle).unwrap().graph.node(node_id(&label)).unwrap().parent_id, Some(node_id(&chip)));
}

#[test]
fn instances_of_a_grafta_component_are_placed_overridden_and_drawn_with_its_style() {
    let (mut kit, handle) = kit();
    let (chip, label) = chip_component(&mut kit, &handle);
    let copy = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": chip, "x": 0, "y": 100 })).unwrap()["id"].clone();
    call(&mut kit, "override_instance", json!({ "handle": handle, "id": copy, "layer": label, "properties": { "characters": "Nuevo" } })).unwrap();
    let (png, caption) = render(&mut kit, json!({ "handle": handle, "id": copy })).unwrap();
    // The component's violet and radius, though the instance stores neither.
    assert_eq!(pixel(&png, 50, 28), [0x7C, 0x3A, 0xED, 255]);
    assert_eq!(pixel(&png, 0, 0)[3], 0, "a rounded corner");
    assert!(caption["notes"].as_array().unwrap().iter().any(|n| n.as_str().unwrap().contains("laid out")));
    let count = call(&mut kit, "component_instances", json!({ "handle": handle, "id": chip })).unwrap();
    assert_eq!(count["total"], 1, "{count}");
}

#[test]
fn only_a_frame_outside_components_and_instances_becomes_a_component() {
    let (mut kit, handle) = kit();
    let (chip, label) = chip_component(&mut kit, &handle);
    let text = call(&mut kit, "create_component", json!({ "handle": handle, "id": label })).unwrap_err();
    assert!(matches!(text, ToolError::Refused(_)), "{text}");
    let again = call(&mut kit, "create_component", json!({ "handle": handle, "id": chip })).unwrap_err();
    assert!(matches!(again, ToolError::Refused(_)), "{again}");
    let page = call(&mut kit, "create_component", json!({ "handle": handle, "id": "1:1" })).unwrap_err();
    assert!(matches!(page, ToolError::Refused(_)), "{page}");
}

#[test]
fn an_instance_takes_its_components_opacity_stroke_and_clip_rather_than_defaults() {
    let (mut kit, handle) = kit();
    let (chip, _) = chip_component(&mut kit, &handle);
    // Changed on the frame before it became a component: through its raw fields.
    {
        let session = kit.session_mut(&handle).unwrap();
        let at = session.raw_index[&node_id(&chip)];
        let kiwi::Value::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(kiwi::Value::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let kiwi::Value::Object(fields) = &mut changes[at] else { unreachable!() };
        fields.insert("opacity", kiwi::Value::Float(0.5));
        fields.insert("strokeWeight", kiwi::Value::Float(3.0));
        fields.insert("frameMaskDisabled", kiwi::Value::Bool(false));
    }
    let copy = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": chip })).unwrap()["id"].clone();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": copy })).unwrap();
    assert_eq!((raw["opacity"].clone(), raw["strokeWeight"].clone(), raw["frameMaskDisabled"].clone()), (json!(0.5), json!(3), json!(false)), "{raw}");
}

// ── editing components ─────────────────────────────────────────────────

/// Plant a cached expansion on an instance, as Figma writes one.
fn cache(kit: &mut Toolkit, handle: &str, id: &str) {
    let session = kit.session_mut(handle).unwrap();
    let at = session.raw_index[&node_id(id)];
    let Raw::Object(message) = &mut session.fig.node_changes else { unreachable!() };
    let Some(Raw::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
    let Raw::Object(fields) = &mut changes[at] else { unreachable!() };
    fields.insert("derivedSymbolData", Raw::Array(Vec::new()));
}

fn cached(kit: &mut Toolkit, handle: &str, id: &str) -> bool {
    call(kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap().get("derivedSymbolData").is_some()
}

#[test]
fn a_change_in_a_component_drops_the_caches_of_every_instance_it_reaches() {
    let (mut kit, handle) = kit();
    // 1:42 is an instance of Icon/Play (1:30) inside the Chip component (1:40);
    // an instance of the Chip on the page caches Icon/Play inside it.
    let chip = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": "1:40" })).unwrap()["id"].as_str().unwrap().to_string();
    for id in ["1:5", "1:42", chip.as_str()] {
        cache(&mut kit, &handle, id);
    }
    let out = set(&mut kit, &handle, "1:31", json!({ "fills": [{ "type": "SOLID", "color": "#FF0000" }] })).unwrap();
    assert_eq!(out["instances_refreshed"], 2, "{out}");
    assert!(!cached(&mut kit, &handle, "1:42"), "the icon's own instance");
    assert!(!cached(&mut kit, &handle, &chip), "an instance of a component holding the icon");
    assert!(cached(&mut kit, &handle, "1:5"), "an instance of an unrelated component keeps its cache");
}

#[test]
fn a_component_is_restyled_moved_and_given_children_after_it_is_made() {
    let (mut kit, handle) = kit();
    let (chip, _) = chip_component(&mut kit, &handle);
    let copy = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": chip, "x": 0, "y": 100 })).unwrap()["id"].clone();
    set(&mut kit, &handle, &chip, json!({ "fills": [{ "type": "SOLID", "color": "#16A34A" }], "width": 120 })).unwrap();
    let dot = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": chip, "name": "Dot", "x": 100, "y": 12, "width": 8, "height": 8 })).unwrap();
    assert_eq!(dot["type"], "FRAME");
    call(&mut kit, "move_node", json!({ "handle": handle, "id": chip, "parent": "1:6" })).unwrap();
    // The instance shows the component as it is now.
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": copy })).unwrap();
    assert_eq!(pixel(&png, 50, 28), [0x16, 0xA3, 0x4A, 255]);
}

#[test]
fn a_component_does_not_go_inside_another() {
    let (mut kit, handle) = kit();
    let (chip, _) = chip_component(&mut kit, &handle);
    let moved = call(&mut kit, "move_node", json!({ "handle": handle, "id": chip, "parent": "1:40" })).unwrap_err();
    assert!(matches!(moved, ToolError::Refused(_)), "{moved}");
    let inner = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:40", "name": "Inner", "x": 0, "y": 0, "width": 10, "height": 10 })).unwrap()["id"].clone();
    let promoted = call(&mut kit, "create_component", json!({ "handle": handle, "id": inner })).unwrap_err();
    assert!(matches!(promoted, ToolError::Refused(_)), "{promoted}");
}

#[test]
fn instances_follow_their_components_new_size_unless_they_had_their_own() {
    let (mut kit, handle) = kit();
    let (chip, _) = chip_component(&mut kit, &handle);
    let follows = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": chip })).unwrap()["id"].as_str().unwrap().to_string();
    let own = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": chip })).unwrap()["id"].as_str().unwrap().to_string();
    // Given a size of its own, through its raw size, as Figma would store one.
    {
        let session = kit.session_mut(&handle).unwrap();
        let at = session.raw_index[&node_id(&own)];
        let Raw::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(Raw::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let Raw::Object(fields) = &mut changes[at] else { unreachable!() };
        fields.insert("size", obj(vec![("x", Raw::Float(150.0)), ("y", Raw::Float(32.0))]));
        session.refresh(&[node_id(&own)]);
    }
    let out = set(&mut kit, &handle, &chip, json!({ "width": 140 })).unwrap();
    assert_eq!(out["unsaved_edits"], 7, "one edit, however many instances follow: {out}");
    let width = |kit: &Toolkit, id: &str| kit.session(&handle).unwrap().graph.node(node_id(id)).unwrap().width;
    assert_eq!(width(&kit, &follows), 140.0);
    assert_eq!(width(&kit, &own), 150.0);
}

// ── variant sets ────────────────────────────────────────────────────────

/// Two chips made components, side by side: "Exacto" (100 wide) and
/// "Sustituto" (120 wide), grouped into a set by "Estado".
fn badge_set(kit: &mut Toolkit, handle: &str) -> (String, String, String) {
    let make = |kit: &mut Toolkit, name: &str, x: f32, width: f32| {
        let id = call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": name, "x": x, "y": 0, "width": width, "height": 30 })).unwrap()["id"].as_str().unwrap().to_string();
        call(kit, "create_component", json!({ "handle": handle, "id": id })).unwrap();
        id
    };
    let exact = make(kit, "Exacto", 0.0, 100.0);
    let substitute = make(kit, "Sustituto", 120.0, 120.0);
    let set = call(kit, "create_variant_set", json!({ "handle": handle, "name": "Badge", "variants": [
        { "id": exact, "values": { "Estado": "Exacto" } },
        { "id": substitute, "values": { "Estado": "Sustituto" } },
    ] }))
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    (set, exact, substitute)
}

#[test]
fn components_become_variants_of_a_set_shaped_as_figmas() {
    let (mut kit, handle) = kit();
    let (set, exact, substitute) = badge_set(&mut kit, &handle);
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": set })).unwrap();
    assert_eq!(raw["isStateGroup"], true);
    assert_eq!(raw["componentPropDefs"][0]["name"], "Estado");
    assert_eq!(raw["componentPropDefs"][0]["type"], "VARIANT");
    assert_eq!(raw["stateGroupPropertyValueOrders"][0]["values"], json!(["Exacto", "Sustituto"]));
    let variant = call(&mut kit, "read_raw", json!({ "handle": handle, "id": substitute })).unwrap();
    assert_eq!(variant["name"], "Estado=Sustituto");
    assert_eq!(variant["variantPropSpecs"][0]["value"], "Sustituto");
    assert_eq!(variant["variantPropSpecs"][0]["propDefId"], raw["componentPropDefs"][0]["id"]);
    let session = kit.session(&handle).unwrap();
    assert_eq!(session.graph.node(node_id(&set)).unwrap().node_type, model::NodeType::ComponentSet);
    assert_eq!(session.graph.node(node_id(&exact)).unwrap().parent_id, Some(node_id(&set)));
    // Placed around them: the first variant 20 in from the set's corner.
    let (s, e) = (session.graph.node(node_id(&set)).unwrap(), session.graph.node(node_id(&exact)).unwrap());
    assert_eq!((s.x, s.y, s.width, s.height, e.x, e.y), (-20.0, -20.0, 280.0, 70.0, 20.0, 20.0));
    let listed = call(&mut kit, "list_components", json!({ "handle": handle, "name": "Badge" })).unwrap();
    assert!(listed["results"].as_array().unwrap().iter().any(|c| c["id"] == set), "{listed}");
}

#[test]
fn an_instance_switches_variant_and_takes_its_size() {
    let (mut kit, handle) = kit();
    let (_, exact, substitute) = badge_set(&mut kit, &handle);
    let copy = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": exact, "x": 0, "y": 100 })).unwrap()["id"].as_str().unwrap().to_string();
    let out = call(&mut kit, "set_variant", json!({ "handle": handle, "id": copy, "values": { "Estado": "Sustituto" } })).unwrap();
    assert_eq!((out["variant"].clone(), out["variant_name"].clone()), (json!(substitute), json!("Estado=Sustituto")));
    let node = kit.session(&handle).unwrap().graph.node(node_id(&copy)).unwrap();
    assert_eq!((node.component_id, node.width), (Some(node_id(&substitute)), 120.0));
    let missing = call(&mut kit, "set_variant", json!({ "handle": handle, "id": copy, "values": { "Estado": "Pendiente" } })).unwrap_err();
    assert!(matches!(missing, ToolError::Refused(ref m) if m.contains("Estado=Exacto")), "{missing}");
    let unknown = call(&mut kit, "set_variant", json!({ "handle": handle, "id": copy, "values": { "Color": "Rojo" } })).unwrap_err();
    assert!(matches!(unknown, ToolError::BadArgument(_)), "{unknown}");
}

#[test]
fn a_set_refuses_what_figma_would_not_hold() {
    let (mut kit, handle) = kit();
    let (_, exact, substitute) = badge_set(&mut kit, &handle);
    let again = call(&mut kit, "create_variant_set", json!({ "handle": handle, "name": "X", "variants": [
        { "id": exact, "values": { "Estado": "A" } }, { "id": substitute, "values": { "Estado": "B" } },
    ] })).unwrap_err();
    assert!(matches!(again, ToolError::Refused(ref m) if m.contains("already a variant")), "{again}");
    let make = |kit: &mut Toolkit, x: f32| {
        let id = call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "C", "x": x, "y": 200, "width": 10, "height": 10 })).unwrap()["id"].as_str().unwrap().to_string();
        call(kit, "create_component", json!({ "handle": handle, "id": id })).unwrap();
        id
    };
    let (a, b) = (make(&mut kit, 0.0), make(&mut kit, 20.0));
    let same = call(&mut kit, "create_variant_set", json!({ "handle": handle, "name": "X", "variants": [
        { "id": a, "values": { "P": "1" } }, { "id": b, "values": { "P": "1" } },
    ] })).unwrap_err();
    assert!(matches!(same, ToolError::BadArgument(ref m) if m.contains("repeats")), "{same}");
    let other = call(&mut kit, "create_variant_set", json!({ "handle": handle, "name": "X", "variants": [
        { "id": a, "values": { "P": "1" } }, { "id": b, "values": { "Q": "1" } },
    ] })).unwrap_err();
    assert!(matches!(other, ToolError::BadArgument(ref m) if m.contains("other properties")), "{other}");
    let frame = created_frame(&mut kit, &handle);
    let not_component = call(&mut kit, "create_variant_set", json!({ "handle": handle, "name": "X", "variants": [
        { "id": a, "values": { "P": "1" } }, { "id": frame, "values": { "P": "2" } },
    ] })).unwrap_err();
    assert!(matches!(not_component, ToolError::Refused(_)), "{not_component}");
}

// ── Variables ─────────────────────────────────────────────────

fn collection(kit: &mut Toolkit, handle: &str, name: &str, modes: &[&str]) -> Result<Value, ToolError> {
    call(kit, "create_variable_collection", json!({ "handle": handle, "name": name, "modes": modes }))
}

fn new_variable(kit: &mut Toolkit, handle: &str, collection: &str, name: &str, kind: &str, values: Value) -> Result<Value, ToolError> {
    call(kit, "create_variable", json!({ "handle": handle, "collection": collection, "name": name, "type": kind, "values": values }))
}

fn listed(kit: &mut Toolkit, handle: &str, collection: &str) -> Value {
    call(kit, "list_variables", json!({ "handle": handle, "collection": collection })).unwrap()
}

/// The collection and variables read again from the node changes, by the
/// reader every opened file goes through, rather than from the session's
/// graph the tools kept in step.
fn reread(kit: &mut Toolkit, handle: &str, collection: &str) -> Value {
    let mut fig = document();
    fig.node_changes = kit.session(handle).unwrap().fig.node_changes.clone();
    let again = kit.insert(PathBuf::from("/tmp/hand-built-reread.fig"), fig).unwrap().handle.to_string();
    let out = listed(kit, &again, collection);
    kit.close(&again).unwrap();
    out
}

fn internal_canvases(kit: &Toolkit, handle: &str) -> Vec<Raw> {
    model::author::node_changes(&kit.session(handle).unwrap().fig)
        .iter()
        .filter(|nc| nc.get("internalOnly").and_then(Raw::as_bool) == Some(true))
        .cloned()
        .collect()
}

#[test]
fn a_collection_and_a_variable_of_each_type_read_back_as_written() {
    let (mut kit, handle) = kit();
    let made = collection(&mut kit, &handle, "Foundations", &["Light", "Dark"]).unwrap();
    assert_eq!(made["modes"].as_array().unwrap().len(), 2);
    let colour = new_variable(&mut kit, &handle, "Foundations", "Brand/Primary", "COLOR", json!({ "Light": "#7C3AED", "Dark": "#A78BFA80" })).unwrap();
    assert_eq!(colour["values"]["Light"], "#7C3AED");
    assert_eq!(colour["values"]["Dark"]["color"], "#A78BFA");
    assert!((colour["values"]["Dark"]["opacity"].as_f64().unwrap() - 128.0 / 255.0).abs() < 1e-6);
    new_variable(&mut kit, &handle, "Foundations", "Space/M", "FLOAT", json!({ "Light": 16, "Dark": 20 })).unwrap();
    new_variable(&mut kit, &handle, "Foundations", "Font/Family", "STRING", json!({ "Light": "Inter", "Dark": "Inter" })).unwrap();
    new_variable(&mut kit, &handle, "Foundations", "Flags/Rounded", "BOOLEAN", json!({ "Light": true, "Dark": false })).unwrap();

    let out = listed(&mut kit, &handle, "Foundations");
    assert_eq!(out["collections"][0]["modes"], json!(["Light", "Dark"]));
    assert_eq!(out["collections"][0]["variables"], 4);
    let by_name = |name: &str| out["results"].as_array().unwrap().iter().find(|v| v["name"] == name).unwrap().clone();
    assert_eq!(by_name("Space/M")["values"], json!({ "Light": 16, "Dark": 20 }));
    assert_eq!(by_name("Font/Family")["type"], "STRING");
    assert_eq!(by_name("Flags/Rounded")["values"], json!({ "Light": true, "Dark": false }));

    // What the tools kept in step is what the reader finds in the bytes.
    assert_eq!(reread(&mut kit, &handle, "Foundations")["results"], out["results"]);
}

#[test]
fn collections_and_variables_are_written_in_the_shape_figma_opened() {
    let (mut kit, handle) = kit();
    let set = collection(&mut kit, &handle, "Foundations", &["Light", "Dark"]).unwrap()["id"].as_str().unwrap().to_string();
    let var = new_variable(&mut kit, &handle, "Foundations", "Space/M", "FLOAT", json!({ "Light": 16, "Dark": 20 })).unwrap()["id"].as_str().unwrap().to_string();
    let session = kit.session(&handle).unwrap();
    let set_raw = session.raw(node_id(&set)).unwrap();
    for field in ["guid", "phase", "parentIndex", "type", "name", "isPublishable", "version", "variableSetModes"] {
        assert!(set_raw.get(field).is_some(), "the collection has no {field}");
    }
    for mode in set_raw.get("variableSetModes").and_then(Raw::as_array).unwrap() {
        assert!(mode.get("sortPosition").is_some(), "a mode has no sortPosition");
    }
    let var_raw = session.raw(node_id(&var)).unwrap();
    for field in ["isPublishable", "version", "sortPosition", "variableSetID", "variableResolvedType", "variableDataValues"] {
        assert!(var_raw.get(field).is_some(), "the variable has no {field}");
    }
    assert_eq!(var_raw.get("sortPosition"), var_raw.get("parentIndex").and_then(|p| p.get("position")));
    let data = &var_raw.get("variableDataValues").and_then(|v| v.get("entries")).and_then(Raw::as_array).unwrap()[0].get("variableData").unwrap();
    assert_eq!(data.get("dataType").and_then(Raw::as_str), Some("FLOAT"));
    assert_eq!(data.get("resolvedDataType").and_then(Raw::as_str), Some("FLOAT"));
}

#[test]
fn a_file_without_an_internal_only_canvas_gets_exactly_one() {
    let (mut kit, handle) = kit();
    assert!(internal_canvases(&kit, &handle).is_empty());
    let first = collection(&mut kit, &handle, "Colors", &["Light"]).unwrap();
    assert_eq!(first["internal_only_canvas"], "written, as the file had none");
    let second = collection(&mut kit, &handle, "Spacing", &["Default"]).unwrap();
    assert_eq!(second["internal_only_canvas"], "the file's own");

    let canvases = internal_canvases(&kit, &handle);
    assert_eq!(canvases.len(), 1);
    let canvas = &canvases[0];
    assert_eq!(canvas.get("visible").and_then(Raw::as_bool), Some(false));
    assert_eq!(canvas.get("name").and_then(Raw::as_str), Some("Internal Only Canvas"));
    let canvas_id = canvas.get("guid").cloned();
    let session = kit.session(&handle).unwrap();
    for set in [&first, &second] {
        let parent = session.raw(node_id(set["id"].as_str().unwrap())).unwrap().get("parentIndex").and_then(|p| p.get("guid")).cloned();
        assert_eq!(parent, canvas_id);
    }
}

#[test]
fn a_fill_binds_by_name_to_a_variable_made_the_call_before() {
    let (mut kit, handle) = kit();
    collection(&mut kit, &handle, "Foundations", &["Light", "Dark"]).unwrap();
    new_variable(&mut kit, &handle, "Foundations", "Brand/Primary", "COLOR", json!({ "Light": "#7C3AED", "Dark": "#A78BFA" })).unwrap();
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "variable": "Brand/Primary" }] })).unwrap();
    let fill = &read(&mut kit, &handle, &id)["fills"][0];
    assert_eq!(fill["variable"], "Brand/Primary");
    assert_eq!(fill["color"], "#7C3AED");
}

#[test]
fn every_variable_writing_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    collection(&mut kit, &handle, "Foundations", &["Light", "Dark"]).unwrap();
    new_variable(&mut kit, &handle, "Foundations", "Brand/Primary", "COLOR", json!({ "Light": "#7C3AED", "Dark": "#A78BFA" })).unwrap();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let cases = [
        ("Brand/Secondary", "COLOR", json!({ "Light": "#000000" }), "no value for mode `Dark`"),
        ("Brand/Secondary", "COLOR", json!({ "Light": "#000000", "Dark": "#000000", "Dim": "#000000" }), "has no mode `Dim`"),
        ("Brand/Primary", "COLOR", json!({ "Light": "#000000", "Dark": "#000000" }), "already has a variable named"),
        ("Space/M", "FLOAT", json!({ "Light": "16", "Dark": 20 }), "expected a number"),
        ("Brand/Secondary", "COLOR", json!({ "Light": 3, "Dark": "#000000" }), "expected #RRGGBB"),
        ("Brand/Secondary", "COLOUR", json!({ "Light": "#000000", "Dark": "#000000" }), "expected COLOR, FLOAT"),
        ("Brand/Secondary", "COLOR", json!({ "Light": { "alias": "library/blue" }, "Dark": "#000000" }), "imported from a library"),
        ("Space/M", "FLOAT", json!({ "Light": { "alias": "Brand/Primary" }, "Dark": 1 }), "must have its variable's type"),
        ("Brand/Secondary", "COLOR", json!({ "Light": { "alias": "Brand/Primry" }, "Dark": "#000000" }), "Closest: Brand/Primary"),
    ];
    for (name, kind, values, reason) in cases {
        let error = new_variable(&mut kit, &handle, "Foundations", name, kind, values).unwrap_err();
        assert!(error.to_string().contains(reason), "{name} {kind}: {error}");
    }
    let error = collection(&mut kit, &handle, "Foundations", &["Light"]).unwrap_err();
    assert!(error.to_string().contains("already named"), "{error}");
    let error = collection(&mut kit, &handle, "Empty", &[]).unwrap_err();
    assert!(error.to_string().contains("at least one mode"), "{error}");
    let error = new_variable(&mut kit, &handle, "Tokens", "Mine", "FLOAT", json!({ "Light": 1 })).unwrap_err();
    assert!(error.to_string().contains("the file's own"), "{error}");
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn a_semantic_token_aliases_a_primitive_one_in_its_collection_or_another() {
    let (mut kit, handle) = kit();
    collection(&mut kit, &handle, "Primitives", &["Default"]).unwrap();
    new_variable(&mut kit, &handle, "Primitives", "Violet/600", "COLOR", json!({ "Default": "#7C3AED" })).unwrap();
    collection(&mut kit, &handle, "Semantic", &["Light", "Dark"]).unwrap();
    new_variable(&mut kit, &handle, "Semantic", "Brand/Primary", "COLOR", json!({ "Light": { "alias": "Violet/600" }, "Dark": { "alias": "primary/500" } })).unwrap();
    new_variable(&mut kit, &handle, "Semantic", "Button/Background", "COLOR", json!({ "Light": { "alias": "Brand/Primary" }, "Dark": { "alias": "Brand/Primary" } })).unwrap();

    let out = listed(&mut kit, &handle, "Semantic");
    let primary = out["results"].as_array().unwrap().iter().find(|v| v["name"] == "Brand/Primary").unwrap();
    assert_eq!(primary["values"], json!({ "Light": { "alias": "Violet/600" }, "Dark": { "alias": "primary/500" } }));
    assert_eq!(reread(&mut kit, &handle, "Semantic")["results"], out["results"]);

    // Bound through two aliases, a fill shows the primitive's colour.
    let id = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &id, json!({ "fills": [{ "type": "SOLID", "variable": "Button/Background" }] })).unwrap();
    assert_eq!(read(&mut kit, &handle, &id)["fills"][0]["color"], "#7C3AED");
    let raw = kit.session(&handle).unwrap().raw(node_id(primary["id"].as_str().unwrap())).unwrap().clone();
    let data = &raw.get("variableDataValues").and_then(|v| v.get("entries")).and_then(Raw::as_array).unwrap()[0].get("variableData").unwrap();
    assert_eq!(data.get("dataType").and_then(Raw::as_str), Some("ALIAS"));
    assert_eq!(data.get("resolvedDataType").and_then(Raw::as_str), Some("COLOR"));
}

#[test]
fn a_variable_grafta_made_is_renamed_revalued_and_given_a_mode() {
    let (mut kit, handle) = kit();
    let set_id = collection(&mut kit, &handle, "Foundations", &["Light", "Dark"]).unwrap()["id"].as_str().unwrap().to_string();
    new_variable(&mut kit, &handle, "Foundations", "Brand/Primary", "COLOR", json!({ "Light": "#7C3AED", "Dark": "#A78BFA" })).unwrap();
    new_variable(&mut kit, &handle, "Foundations", "Button/Background", "COLOR", json!({ "Light": { "alias": "Brand/Primary" }, "Dark": "#000000" })).unwrap();
    let frame = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &frame, json!({ "fills": [{ "type": "SOLID", "variable": "Brand/Primary" }] })).unwrap();

    let changed = call(&mut kit, "set_variable", json!({ "handle": handle, "id": "Brand/Primary", "name": "Brand/Main", "values": { "Dark": "#C4B5FD" } })).unwrap();
    assert_eq!(changed["name"], "Brand/Main");
    assert_eq!(changed["values"], json!({ "Light": "#7C3AED", "Dark": "#C4B5FD" }));
    assert_eq!(read(&mut kit, &handle, &frame)["fills"][0]["variable"], "Brand/Main");

    let error = call(&mut kit, "set_variable", json!({ "handle": handle, "id": "Brand/Main", "values": { "Light": { "alias": "Button/Background" } } })).unwrap_err();
    assert!(error.to_string().contains("cycle"), "{error}");

    let grown = call(&mut kit, "set_variable_collection", json!({ "handle": handle, "id": set_id, "name": "Theme", "add_mode": "High contrast" })).unwrap();
    assert_eq!(grown["name"], "Theme");
    assert_eq!(grown["modes"].as_array().unwrap().len(), 3);
    let out = listed(&mut kit, &handle, "Theme");
    let main = out["results"].as_array().unwrap().iter().find(|v| v["name"] == "Brand/Main").unwrap();
    assert_eq!(main["values"]["High contrast"], "#7C3AED", "a new mode takes the first mode's value");
    assert_eq!(reread(&mut kit, &handle, "Theme")["results"], out["results"]);

    for (tool, id) in [("set_variable", "primary/500"), ("set_variable_collection", "Tokens")] {
        let error = call(&mut kit, tool, json!({ "handle": handle, "id": id, "name": "Renamed" })).unwrap_err();
        assert!(error.to_string().contains("the file's own"), "{tool}: {error}");
    }
    let error = call(&mut kit, "set_variable", json!({ "handle": handle, "id": "library/blue", "name": "Renamed" })).unwrap_err();
    assert!(error.to_string().contains("imported from a library"), "{error}");
}

// ── Bindings and modes ────────────────────────────────────────

/// Theme (Light, Dark) and Dimensions (Default).
fn foundations(kit: &mut Toolkit, handle: &str) {
    collection(kit, handle, "Theme", &["Light", "Dark"]).unwrap();
    collection(kit, handle, "Dimensions", &["Default"]).unwrap();
    for (c, name, kind, values) in [
        ("Theme", "Surface", "COLOR", json!({ "Light": "#FFFFFF", "Dark": "#1E1B2E" })),
        ("Theme", "Text", "COLOR", json!({ "Light": "#0B0B12", "Dark": "#F8FAFC" })),
        ("Theme", "Radius", "FLOAT", json!({ "Light": 12, "Dark": 24 })),
        ("Theme", "Pad", "FLOAT", json!({ "Light": 16, "Dark": 24 })),
        ("Theme", "Background", "COLOR", json!({ "Light": { "alias": "Surface" }, "Dark": { "alias": "Surface" } })),
        ("Dimensions", "Gap", "FLOAT", json!({ "Default": 8 })),
        ("Dimensions", "Width", "FLOAT", json!({ "Default": 240 })),
        ("Dimensions", "Title", "FLOAT", json!({ "Default": 22 })),
        ("Dimensions", "Half", "FLOAT", json!({ "Default": 0.5 })),
        ("Dimensions", "Flag", "BOOLEAN", json!({ "Default": true })),
    ] {
        new_variable(kit, handle, c, name, kind, values).unwrap();
    }
}

fn bound_card(kit: &mut Toolkit, handle: &str) -> (String, String) {
    foundations(kit, handle);
    let card = created_frame(kit, handle);
    set(kit, handle, &card, json!({
        "layoutMode": "VERTICAL",
        "padding": { "variable": "Pad" },
        "itemSpacing": { "variable": "Gap" },
        "cornerRadius": { "variable": "Radius" },
        "width": { "variable": "Width" },
        "opacity": { "variable": "Half" },
        "fills": [{ "type": "SOLID", "variable": "Surface" }],
    }))
    .unwrap();
    let title = call(kit, "create_text", json!({ "handle": handle, "parent": card, "characters": "Card", "fills": [{ "type": "SOLID", "variable": "Text" }] })).unwrap()["id"].as_str().unwrap().to_string();
    set(kit, handle, &title, json!({ "fontSize": { "variable": "Title" } })).unwrap();
    (card, title)
}

fn raw_of(kit: &Toolkit, handle: &str, id: &str) -> Raw {
    kit.session(handle).unwrap().raw(node_id(id)).unwrap().clone()
}

#[test]
fn a_bound_number_stores_its_value_and_shows_its_variable() {
    let (mut kit, handle) = kit();
    let (card, title) = bound_card(&mut kit, &handle);
    let node = read(&mut kit, &handle, &card);
    for (key, value) in [("paddingTop", 16.0), ("paddingLeft", 16.0), ("itemSpacing", 8.0), ("cornerRadius", 12.0), ("width", 240.0), ("opacity", 0.5)] {
        assert_eq!(node[key].as_f64(), Some(value), "{key}: {node}");
    }
    let bound = &node["boundVariables"];
    for (key, name) in [
        ("paddingTop", "Pad"), ("paddingRight", "Pad"), ("paddingBottom", "Pad"), ("paddingLeft", "Pad"),
        ("itemSpacing", "Gap"), ("topLeftRadius", "Radius"), ("bottomRightRadius", "Radius"),
        ("width", "Width"), ("opacity", "Half"),
    ] {
        assert_eq!(bound[key], name, "{key}: {bound}");
    }
    // A radius is bound as four corners beside one radius, as b1 wrote it.
    let raw = raw_of(&kit, &handle, &card);
    assert_eq!(raw.get("rectangleCornerRadiiIndependent").and_then(Raw::as_bool), Some(false));
    assert_eq!(raw.get("rectangleBottomLeftCornerRadius").and_then(Raw::as_f32), Some(12.0));
    let entries = raw.get("variableConsumptionMap").and_then(|m| m.get("entries")).and_then(Raw::as_array).unwrap();
    assert_eq!(entries.len(), 11);
    assert_eq!(entries[0].get("variableData").and_then(|d| d.get("resolvedDataType")).and_then(Raw::as_str), Some("FLOAT"));

    let text = read(&mut kit, &handle, &title);
    assert_eq!(text["style"]["fontSize"].as_f64(), Some(22.0));
    assert_eq!(text["boundVariables"]["fontSize"], "Title");
}

#[test]
fn a_plain_number_unbinds_only_its_own_field() {
    let (mut kit, handle) = kit();
    let (card, _) = bound_card(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "itemSpacing": 4, "topLeftRadius": 2 })).unwrap();
    let node = read(&mut kit, &handle, &card);
    assert_eq!(node["itemSpacing"].as_f64(), Some(4.0));
    let bound = node["boundVariables"].as_object().unwrap();
    assert!(!bound.contains_key("itemSpacing") && !bound.contains_key("topLeftRadius"), "{bound:?}");
    assert_eq!(bound["topRightRadius"], "Radius");
    assert_eq!(bound["paddingTop"], "Pad");
    assert_eq!(raw_of(&kit, &handle, &card).get("rectangleCornerRadiiIndependent").and_then(Raw::as_bool), Some(true));
}

#[test]
fn every_binding_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let (card, _) = bound_card(&mut kit, &handle);
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": "1:4", "x": 0, "y": 300 })).unwrap()["id"].as_str().unwrap().to_string();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let cases = [
        (&card, json!({ "padding": { "variable": "Surface" } }), "is a Color variable"),
        (&card, json!({ "itemSpacing": { "variable": "Gapp" } }), "Closest: Gap"),
        (&card, json!({ "itemSpacing": { "value": 3 } }), "expected a number or"),
        (&card, json!({ "visible": { "variable": "Flag" } }), "did nothing"),
        (&card, json!({ "explicitVariableModes": { "Colours": "Dark" } }), "no collection `Colours`"),
        (&card, json!({ "explicitVariableModes": { "Theme": "Dim" } }), "has no mode"),
        (&instance, json!({ "explicitVariableModes": { "Theme": "Dark" } }), "draws its component's stored values"),
    ];
    for (id, properties, reason) in cases {
        let error = set(&mut kit, &handle, id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn a_frame_in_dark_stores_dark_values_on_itself_and_everything_in_it() {
    let (mut kit, handle) = kit();
    let (card, title) = bound_card(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "explicitVariableModes": { "Theme": "Dark" } })).unwrap();
    let node = read(&mut kit, &handle, &card);
    assert_eq!(node["explicitVariableModes"], json!({ "Theme": "Dark" }));
    assert_eq!(node["fills"][0]["color"], "#1E1B2E");
    assert_eq!(node["paddingTop"].as_f64(), Some(24.0));
    assert_eq!(node["cornerRadius"].as_f64(), Some(24.0));
    assert_eq!(node["itemSpacing"].as_f64(), Some(8.0), "Dimensions has no Dark");
    assert_eq!(read(&mut kit, &handle, &title)["fills"][0]["color"], "#F8FAFC");

    // Something bound inside it afterwards resolves in Dark too.
    let inner = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": card, "name": "Inner", "x": 0, "y": 0, "width": 10, "height": 10 })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &inner, json!({ "cornerRadius": { "variable": "Radius" }, "fills": [{ "type": "SOLID", "variable": "Background" }] })).unwrap();
    let inner_node = read(&mut kit, &handle, &inner);
    assert_eq!(inner_node["cornerRadius"].as_f64(), Some(24.0));
    assert_eq!(inner_node["fills"][0]["color"], "#1E1B2E");

    set(&mut kit, &handle, &card, json!({ "explicitVariableModes": { "Theme": null } })).unwrap();
    let node = read(&mut kit, &handle, &card);
    assert!(node.get("explicitVariableModes").is_none(), "{node}");
    assert_eq!(node["fills"][0]["color"], "#FFFFFF");
    assert_eq!(node["paddingTop"].as_f64(), Some(16.0));
    assert_eq!(read(&mut kit, &handle, &title)["fills"][0]["color"], "#0B0B12");
}

#[test]
fn a_revalued_variable_is_stored_again_wherever_it_reaches() {
    let (mut kit, handle) = kit();
    foundations(&mut kit, &handle);
    let light = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &light, json!({ "fills": [{ "type": "SOLID", "variable": "Background" }], "cornerRadius": { "variable": "Radius" } })).unwrap();
    let dark = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &dark, json!({ "fills": [{ "type": "SOLID", "variable": "Background" }], "explicitVariableModes": { "Theme": "Dark" } })).unwrap();

    call(&mut kit, "set_variable", json!({ "handle": handle, "id": "Surface", "values": { "Light": "#EEEEEE" } })).unwrap();
    call(&mut kit, "set_variable", json!({ "handle": handle, "id": "Radius", "values": { "Light": 6 } })).unwrap();
    let node = read(&mut kit, &handle, &light);
    assert_eq!(node["fills"][0]["color"], "#EEEEEE", "through the alias Background");
    assert_eq!(node["cornerRadius"].as_f64(), Some(6.0));
    assert_eq!(read(&mut kit, &handle, &dark)["fills"][0]["color"], "#1E1B2E", "Dark is unchanged");
}

// ── Styles ────────────────────────────────────────────────────

fn styles(kit: &mut Toolkit, handle: &str) {
    for args in [
        json!({ "name": "Brand/Primary", "type": "FILL", "fills": [{ "type": "SOLID", "color": "#7C3AED" }] }),
        json!({ "name": "Heading/L", "type": "TEXT", "fontFamily": "Inter", "fontStyle": "Bold", "fontSize": 28, "lineHeight": 34 }),
        json!({ "name": "Shadow/Card", "type": "EFFECT", "effects": [{ "type": "DROP_SHADOW", "color": "#000000", "opacity": 0.25, "offset": { "x": 0, "y": 8 }, "radius": 24 }] }),
    ] {
        let mut args = args;
        args["handle"] = json!(handle);
        call(kit, "create_style", args).unwrap();
    }
}

#[test]
fn styles_are_made_and_listed_with_their_values() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    let out = call(&mut kit, "list_styles", json!({ "handle": handle })).unwrap();
    assert_eq!(out["total"], 3);
    let by = |name: &str| out["styles"].as_array().unwrap().iter().find(|s| s["name"] == name).unwrap().clone();
    assert_eq!(by("Brand/Primary")["fills"][0]["color"], "#7C3AED");
    let heading = by("Heading/L");
    assert_eq!((heading["type"].as_str(), heading["fontFamily"].as_str(), heading["fontStyle"].as_str(), heading["fontSize"].as_f64()), (Some("TEXT"), Some("Inter"), Some("Bold"), Some(28.0)));
    assert_eq!(by("Shadow/Card")["effects"][0]["type"], "DROP_SHADOW");
    let only = call(&mut kit, "list_styles", json!({ "handle": handle, "type": "TEXT" })).unwrap();
    assert_eq!(only["total"], 1);
    // One Internal Only Canvas, however many styles.
    assert_eq!(internal_canvases(&kit, &handle).len(), 1);

    // In the shape Figma listed.
    let id = by("Brand/Primary")["id"].as_str().unwrap().to_string();
    let raw = raw_of(&kit, &handle, &id);
    for field in ["styleType", "isPublishable", "version", "sortPosition", "fillPaints"] {
        assert!(raw.get(field).is_some(), "no {field}");
    }
    assert_eq!(raw.get("type").and_then(Raw::as_str), Some("ROUNDED_RECTANGLE"));
    assert_eq!(raw.get("visible").and_then(Raw::as_bool), Some(false));
    assert!(raw.get("fillGeometry").is_none() && raw.get("key").is_none());
}

#[test]
fn a_style_applied_points_at_it_and_stores_its_values() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    let card = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "fillStyle": "Brand/Primary", "strokeStyle": "Brand/Primary", "effectStyle": "Shadow/Card" })).unwrap();
    let node = read(&mut kit, &handle, &card);
    assert_eq!((node["fillStyle"].as_str(), node["strokeStyle"].as_str(), node["effectStyle"].as_str()), (Some("Brand/Primary"), Some("Brand/Primary"), Some("Shadow/Card")));
    assert_eq!(node["fills"][0]["color"], "#7C3AED");
    assert_eq!(node["strokes"][0]["color"], "#7C3AED");
    assert_eq!(node["effects"][0]["type"], "DROP_SHADOW");
    let raw = raw_of(&kit, &handle, &card);
    assert!(raw.get("styleIdForFill").and_then(|p| p.get("guid")).is_some());
    assert!(raw.get("strokePaints").is_some() && raw.get("effects").is_some());

    let title = call(&mut kit, "create_text", json!({ "handle": handle, "parent": "1:1", "characters": "Hola", "fontSize": 12 })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &title, json!({ "textStyle": "Heading/L" })).unwrap();
    let text = read(&mut kit, &handle, &title);
    assert_eq!(text["textStyle"], "Heading/L");
    assert_eq!(text["style"]["fontSize"].as_f64(), Some(28.0));
    let raw = raw_of(&kit, &handle, &title);
    assert_eq!(raw.get("fontSize").and_then(Raw::as_f32), Some(28.0));
    assert!(raw.get("derivedTextData").is_none());
}

#[test]
fn a_direct_value_or_null_detaches_a_style() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    let card = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "fillStyle": "Brand/Primary", "effectStyle": "Shadow/Card" })).unwrap();
    set(&mut kit, &handle, &card, json!({ "fills": [{ "type": "SOLID", "color": "#000000" }] })).unwrap();
    let node = read(&mut kit, &handle, &card);
    assert!(node.get("fillStyle").is_none(), "{node}");
    assert_eq!(node["fills"][0]["color"], "#000000");
    assert_eq!(node["effectStyle"], "Shadow/Card", "only the fill's style is detached");

    set(&mut kit, &handle, &card, json!({ "effectStyle": null })).unwrap();
    let node = read(&mut kit, &handle, &card);
    assert!(node.get("effectStyle").is_none());
    assert_eq!(node["effects"][0]["type"], "DROP_SHADOW", "a detached node keeps the values");
}

#[test]
fn a_style_detached_keeps_the_runs_and_one_applied_replaces_their_fills() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    call(&mut kit, "create_style", json!({ "handle": handle, "name": "Ink", "type": "FILL", "fills": [{ "type": "SOLID", "color": "#111111" }] })).unwrap();
    let runs = json!([{ "characters": "Plain " }, { "characters": "red", "fills": [{ "type": "SOLID", "color": "#FF0000" }] }]);
    let title = text(&mut kit, &handle, json!({ "parent": "1:1", "runs": runs })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &title, json!({ "textStyle": "Heading/L", "fillStyle": "Ink" })).unwrap();
    // Applying a fill style is set on the whole layer, so it takes the runs' fills.
    assert!(read(&mut kit, &handle, &title).get("runs").is_none());

    set(&mut kit, &handle, &title, json!({ "runs": runs })).unwrap();
    for detach in [json!({ "textStyle": null }), json!({ "fillStyle": null })] {
        set(&mut kit, &handle, &title, detach.clone()).unwrap();
        let node = read(&mut kit, &handle, &title);
        assert_eq!(node["runs"][1]["fills"][0]["color"], "#FF0000", "after {detach}: {node}");
        assert_eq!(node["fills"][0]["color"], "#111111", "after {detach}: a detached node keeps the values");
    }
    assert!(read(&mut kit, &handle, &title).get("fillStyle").is_none());

    // Together, as a kit's styles are cleared.
    set(&mut kit, &handle, &title, json!({ "textStyle": "Heading/L", "fillStyle": "Ink" })).unwrap();
    set(&mut kit, &handle, &title, json!({ "runs": runs })).unwrap();
    set(&mut kit, &handle, &title, json!({ "textStyle": null, "fillStyle": null })).unwrap();
    assert_eq!(read(&mut kit, &handle, &title)["runs"][1]["fills"][0]["color"], "#FF0000");
}

#[test]
fn a_changed_style_shows_on_every_node_using_it() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    let card = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "fillStyle": "Brand/Primary" })).unwrap();
    let before = raw_of(&kit, &handle, &card);
    let out = call(&mut kit, "set_style", json!({ "handle": handle, "id": "Brand/Primary", "name": "Brand/Main", "fills": [{ "type": "SOLID", "color": "#16A34A" }] })).unwrap();
    assert_eq!(out["nodes_using_it"], 1);
    let node = read(&mut kit, &handle, &card);
    assert_eq!(node["fillStyle"], "Brand/Main");
    assert_eq!(node["fills"][0]["color"], "#16A34A");
    assert_eq!(raw_of(&kit, &handle, &card), before, "Figma draws the style's values (s2): the node is not written");
}

/// The hand-built file with styles of its own, as Figma writes them: a
/// version, a key, `authoredColor` on the paint; the fill style is the one
/// `Icon/Play`'s Accent layer points at, and two instances cache expansions
/// that hold it, one through its component, one through an override.
fn kit_with_own_styles() -> (Toolkit, String) {
    use testkit::raw::{node, size};
    let mut file = document();
    let Raw::Object(message) = &mut file.node_changes else { unreachable!() };
    let Some((_, Raw::Array(changes))) = message.iter_mut().find(|(k, _)| *k == "nodeChanges") else { unreachable!() };
    let version = || Raw::Str("7524:66".into());
    changes.push(node(97, Some((0, "~")), "CANVAS", "Internal Only Canvas", vec![("internalOnly", Raw::Bool(true))]));
    let mut purple = testkit::raw::solid(0.5, 0.34, 0.85, 1.0);
    if let Raw::Object(paint) = &mut purple {
        paint.insert("authoredColor", obj(vec![("c0", Raw::Float(0.5)), ("c1", Raw::Float(0.34)), ("c2", Raw::Float(0.85)), ("c3", Raw::Float(1.0))]));
    }
    changes.push(node(
        98,
        Some((97, "!")),
        "ROUNDED_RECTANGLE",
        "Brand/600",
        vec![
            ("styleType", Raw::Enum("FILL")),
            ("visible", Raw::Bool(false)),
            ("size", size(100.0, 100.0)),
            ("fillPaints", Raw::Array(vec![purple])),
            ("key", Raw::Str("abc98".into())),
            ("version", version()),
            ("userFacingVersion", version()),
        ],
    ));
    changes.push(node(
        96,
        Some((97, "\"")),
        "TEXT",
        "Text md/Regular",
        vec![
            ("styleType", Raw::Enum("TEXT")),
            ("visible", Raw::Bool(false)),
            ("size", size(100.0, 20.0)),
            ("fontName", obj(vec![("family", Raw::Str("Inter".into())), ("style", Raw::Str("Regular".into()))])),
            ("fontSize", Raw::Float(16.0)),
            ("textData", obj(vec![("characters", Raw::Str("Rag 123".into()))])),
            ("derivedTextData", obj(vec![("layoutSize", size(56.0, 24.0))])),
            ("version", version()),
            ("userFacingVersion", version()),
        ],
    ));
    let derived = Raw::Array(vec![]);
    changes.push(node(
        52,
        Some((1, "~~~")),
        "INSTANCE",
        "Play",
        vec![("size", size(24.0, 24.0)), ("symbolData", obj(vec![("symbolID", guid(1, 30))])), ("derivedSymbolData", derived.clone())],
    ));
    changes.push(node(
        53,
        Some((1, "~~~~")),
        "INSTANCE",
        "Purple button",
        vec![
            ("size", size(96.0, 32.0)),
            (
                "symbolData",
                obj(vec![
                    ("symbolID", guid(1, 4)),
                    (
                        "symbolOverrides",
                        Raw::Array(vec![obj(vec![
                            ("guidPath", obj(vec![("guids", Raw::Array(vec![guid(2, 4)]))])),
                            ("styleIdForFill", obj(vec![("guid", guid(1, 98))])),
                        ])]),
                    ),
                ]),
            ),
            ("derivedSymbolData", derived),
        ],
    ));
    let mut kit = memory_toolkit();
    let handle = kit.insert(PathBuf::from("/tmp/own-styles.fig"), file).unwrap().handle.to_string();
    (kit, handle)
}

#[test]
fn a_files_own_colour_style_changes_once_for_every_node_and_instance() {
    let (mut kit, handle) = kit_with_own_styles();
    let out = call(&mut kit, "set_style", json!({ "handle": handle, "id": "Brand/600", "fills": [{ "type": "SOLID", "color": "#16A34A" }] })).unwrap();
    assert_eq!(out["nodes_using_it"], 1, "{out}");
    assert_eq!(read(&mut kit, &handle, "1:32")["fills"][0]["color"], "#16A34A", "the layer in the component reads the new colour");

    let style = raw_of(&kit, &handle, "1:98");
    let version = style.get("version").and_then(Raw::as_str).unwrap().to_string();
    assert_ne!(version, "7524:66", "a new version, as Figma's own edit gives");
    assert_eq!(style.get("userFacingVersion").and_then(Raw::as_str), Some(version.as_str()));
    assert_eq!(style.get("key").and_then(Raw::as_str), Some("abc98"), "its key stays");
    let paint = style.get("fillPaints").and_then(Raw::as_array).and_then(|p| p.first()).unwrap();
    assert!(paint.get("authoredColor").is_none(), "no old authoredColor to contradict the new colour");

    // An instance's cache holds layout, not paints: a new colour keeps it.
    for instance in ["1:52", "1:53"] {
        assert!(raw_of(&kit, &handle, instance).get("derivedSymbolData").is_some(), "{instance} keeps Figma's layout");
    }
}

#[test]
fn a_files_own_text_style_drops_its_samples_layout() {
    let (mut kit, handle) = kit_with_own_styles();
    call(&mut kit, "set_style", json!({ "handle": handle, "id": "Text md/Regular", "fontSize": 18 })).unwrap();
    let style = raw_of(&kit, &handle, "1:96");
    assert_eq!(style.get("fontSize").and_then(Raw::as_f32), Some(18.0));
    assert!(style.get("derivedTextData").is_none(), "its sample's layout was for 16 px");
    assert_ne!(style.get("version").and_then(Raw::as_str), Some("7524:66"));
}

#[test]
fn a_text_style_given_a_new_size_has_its_instances_derived_again() {
    let (mut kit, handle) = kit_with_own_styles();
    // Chip's Label (1:41) inside a component, with an instance that caches it.
    set(&mut kit, &handle, "1:41", json!({ "textStyle": "Text md/Regular" })).unwrap();
    call(&mut kit, "set_style", json!({ "handle": handle, "id": "Text md/Regular", "fontSize": 20 })).unwrap();
    assert_eq!(read(&mut kit, &handle, "1:41")["style"]["fontSize"].as_f64(), Some(20.0));
}

#[test]
fn every_style_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    let card = created_frame(&mut kit, &handle);
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let cases = [
        (json!({ "fillStyle": "Brand/Primry" }), "Closest: Brand/Primary"),
        (json!({ "fillStyle": "Heading/L" }), "is a TEXT style"),
        (json!({ "fillStyle": "Brand/Primary", "fills": [] }), "pass one"),
        (json!({ "fillStyle": 3 }), "expected a style name"),
    ];
    for (properties, reason) in cases {
        let error = set(&mut kit, &handle, &card, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    for (args, reason) in [
        (json!({ "name": "Brand/Primary", "type": "FILL", "fills": [] }), "already named"),
        (json!({ "name": "Grid", "type": "GRID" }), "grid styles"),
        (json!({ "name": "Heading/M", "type": "TEXT", "fontFamily": "Inter" }), "needs its size"),
        (json!({ "name": "Wrong", "type": "FILL", "effects": [] }), "is not a value of this style"),
    ] {
        let mut args = args;
        args["handle"] = json!(handle);
        let error = call(&mut kit, "create_style", args.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{args}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

// ── Component properties ──────────────────────────────────────

/// Two icon components and a Button component holding an instance of the
/// first and a label; and one instance of the
/// Button. Returns (plus, check, button, icon instance, label, instance).
fn button(kit: &mut Toolkit, handle: &str) -> [String; 6] {
    let id = |v: Value| v["id"].as_str().unwrap().to_string();
    let mut icons = Vec::new();
    for name in ["Icon/Plus", "Icon/Check"] {
        let icon = id(call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": name, "x": 0, "y": 0, "width": 20, "height": 20 })).unwrap());
        call(kit, "create_component", json!({ "handle": handle, "id": icon })).unwrap();
        icons.push(icon);
    }
    let button = id(call(kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Button", "x": 0, "y": 60, "width": 160, "height": 44 })).unwrap());
    let icon = id(call(kit, "create_instance", json!({ "handle": handle, "parent": button, "component": icons[0], "x": 14, "y": 12 })).unwrap());
    let label = id(call(kit, "create_text", json!({ "handle": handle, "parent": button, "characters": "Button", "x": 44, "y": 12 })).unwrap());
    call(kit, "create_component", json!({ "handle": handle, "id": button })).unwrap();
    let instance = id(call(kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": button, "x": 0, "y": 140 })).unwrap());
    [icons[0].clone(), icons[1].clone(), button, icon, label, instance]
}

fn property(kit: &mut Toolkit, handle: &str, component: &str, name: &str, kind: &str, default: Value) -> Result<Value, ToolError> {
    call(kit, "add_component_property", json!({ "handle": handle, "component": component, "name": name, "type": kind, "default": default }))
}

fn with_properties(kit: &mut Toolkit, handle: &str) -> [String; 6] {
    let ids = button(kit, handle);
    let [plus, _, button, icon, label, _] = ids.clone();
    property(kit, handle, &button, "Label", "TEXT", json!("Button")).unwrap();
    property(kit, handle, &button, "Show icon", "BOOLEAN", json!(true)).unwrap();
    property(kit, handle, &button, "Icon", "INSTANCE_SWAP", json!(plus)).unwrap();
    set(kit, handle, &label, json!({ "componentPropertyReferences": { "characters": "Label" } })).unwrap();
    set(kit, handle, &icon, json!({ "componentPropertyReferences": { "visible": "Show icon", "mainComponent": "Icon" } })).unwrap();
    ids
}

#[test]
fn a_component_property_is_defined_bound_and_set_per_instance() {
    let (mut kit, handle) = kit();
    let [_, check, button, icon, label, instance] = with_properties(&mut kit, &handle);

    let component = read(&mut kit, &handle, &button);
    let definitions = component["componentPropertyDefinitions"].as_array().unwrap();
    let names: Vec<&str> = definitions.iter().map(|d| d["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Label", "Show icon", "Icon"]);
    // Ids far from the ones grafta's nodes take.
    for d in definitions {
        let local: u32 = d["id"].as_str().unwrap().split(':').nth(1).unwrap().parse().unwrap();
        assert!(local >= model::author::PROPERTY_IDS_FROM, "{d}");
    }
    assert_eq!(read(&mut kit, &handle, &label)["componentPropertyReferences"], json!({ "characters": "Label" }));
    assert_eq!(read(&mut kit, &handle, &icon)["componentPropertyReferences"], json!({ "visible": "Show icon", "mainComponent": "Icon" }));

    // Both forms of a reference, as all 203,522 measured.
    let raw = raw_of(&kit, &handle, &icon);
    assert_eq!(raw.get("componentPropRefs").and_then(Raw::as_array).map(Vec::len), Some(2));
    let consumed = raw.get("parameterConsumptionMap").and_then(|m| m.get("entries")).and_then(Raw::as_array).unwrap();
    assert_eq!(consumed.len(), 2);
    assert_eq!(consumed[0].get("variableData").and_then(|d| d.get("dataType")).and_then(Raw::as_str), Some("PROP_REF"));
    // Spelled apart from `componentPropRefs`' `defID`, in every schema measured: this
    // document has no schema to refuse the wrong one, and a save on a real file does.
    let prop_ref = consumed[0].get("variableData").and_then(|d| d.get("value")).and_then(|v| v.get("propRefValue")).unwrap();
    assert!(prop_ref.get("defId").is_some() && prop_ref.get("defID").is_none());

    set(&mut kit, &handle, &instance, json!({ "componentProperties": { "Label": "Guardar", "Show icon": false, "Icon": "Icon/Check" } })).unwrap();
    let node = read(&mut kit, &handle, &instance);
    assert_eq!(node["componentProperties"], json!({ "Label": "Guardar", "Show icon": "false", "Icon": "Icon/Check" }));
    let raw = raw_of(&kit, &handle, &instance);
    let assigned = raw.get("componentPropAssignments").and_then(Raw::as_array).unwrap();
    assert_eq!(assigned.len(), 3);
    assert!(assigned.iter().all(|a| a.get("varValue").is_some()));
    let _ = check;

    // Setting one again replaces it.
    set(&mut kit, &handle, &instance, json!({ "componentProperties": { "Label": "Salir" } })).unwrap();
    assert_eq!(read(&mut kit, &handle, &instance)["componentProperties"]["Label"], "Salir");
    assert_eq!(raw_of(&kit, &handle, &instance).get("componentPropAssignments").and_then(Raw::as_array).map(Vec::len), Some(3));
}

#[test]
fn a_bound_layer_shows_the_default_and_null_unbinds_it() {
    let (mut kit, handle) = kit();
    let [_, _, button, _, label, _] = button(&mut kit, &handle);
    property(&mut kit, &handle, &button, "Label", "TEXT", json!("Continuar")).unwrap();
    set(&mut kit, &handle, &label, json!({ "componentPropertyReferences": { "characters": "Label" } })).unwrap();
    assert_eq!(read(&mut kit, &handle, &label)["characters"], "Continuar");
    set(&mut kit, &handle, &label, json!({ "componentPropertyReferences": { "characters": null } })).unwrap();
    let raw = raw_of(&kit, &handle, &label);
    assert!(raw.get("componentPropRefs").is_none() && raw.get("parameterConsumptionMap").is_none());
}

#[test]
fn every_component_property_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let [plus, check, button, icon, label, instance] = with_properties(&mut kit, &handle);
    let card = created_frame(&mut kit, &handle);
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let add_cases = [
        (&card, "X", "TEXT", json!("x"), "properties belong to a component"),
        (&button, "Label", "TEXT", json!("x"), "already has a property"),
        (&button, "Flag", "BOOLEAN", json!("yes"), "takes true or false"),
        (&button, "Swap", "INSTANCE_SWAP", json!(card), "no component"),
        (&button, "Size", "NUMBER", json!(1), "expected TEXT, BOOLEAN"),
    ];
    for (component, name, kind, default, reason) in add_cases {
        let error = property(&mut kit, &handle, component, name, kind, default).unwrap_err();
        assert!(error.to_string().contains(reason), "{name}: {error}");
    }
    let set_cases = [
        (&label, json!({ "componentPropertyReferences": { "characters": "Show icon" } }), "is a BOOLEAN property"),
        (&icon, json!({ "componentPropertyReferences": { "characters": "Label" } }), "is bound on a text"),
        (&label, json!({ "componentPropertyReferences": { "visible": "Nope" } }), "has no property `Nope`"),
        (&card, json!({ "componentPropertyReferences": { "visible": "Show icon" } }), "not inside a component"),
        (&instance, json!({ "componentProperties": { "Label": [3] } }), "takes a string"),
        (&instance, json!({ "componentProperties": { "Icon": card } }), "no component"),
        (&instance, json!({ "componentProperties": { "Size": "L" } }), "has no property `Size`"),
    ];
    for (id, properties, reason) in set_cases {
        let error = set(&mut kit, &handle, id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    let _ = (plus, check);
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn a_variant_is_refused_properties_until_a_rung_writes_a_sets() {
    let (mut kit, handle) = kit();
    let [plus, check, ..] = button(&mut kit, &handle);
    call(&mut kit, "create_variant_set", json!({ "handle": handle, "name": "Icon", "variants": [
        { "id": plus, "values": { "Kind": "Plus" } }, { "id": check, "values": { "Kind": "Check" } },
    ] })).unwrap();
    let error = property(&mut kit, &handle, &plus, "Label", "TEXT", json!("x")).unwrap_err();
    assert!(error.to_string().contains("is a variant"), "{error}");
}

// ── Images ────────────────────────────────────────────────────

/// A 1×1 PNG, and the SHA-1 of its bytes.
const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01,
    0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41,
    0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn image_file(kit: &Toolkit, name: &str, bytes: &[u8]) -> String {
    put(kit, &format!("/memory/images/{name}"), bytes)
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    Sha1::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn images_in(kit: &Toolkit, handle: &str) -> usize {
    kit.session(handle).unwrap().fig.images.len()
}

#[test]
fn an_image_from_disk_fills_a_frame_and_is_embedded_once() {
    let (mut kit, handle) = kit();
    let png = PNG;
    let path = image_file(&kit, "cover.png", png);
    let hash = sha1_hex(png);
    let before = images_in(&kit, &handle);
    let first = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &first, json!({ "fills": [{ "type": "IMAGE", "image": path }] })).unwrap();
    let fill = &read(&mut kit, &handle, &first)["fills"][0];
    assert_eq!((fill["type"].as_str(), fill["imageHash"].as_str(), fill["scaleMode"].as_str()), (Some("IMAGE"), Some(hash.as_str()), Some("FILL")));
    assert_eq!(images_in(&kit, &handle), before + 1);
    assert!(kit.session(&handle).unwrap().fig.images.iter().any(|(name, bytes)| *name == hash && bytes == png));
    // And in the graph's copy, which render_node draws from.
    assert!(kit.session(&handle).unwrap().graph.images.iter().any(|(name, _)| *name == hash));

    // The 16 fields all measured image paints carry, the size the image's own.
    let raw = raw_of(&kit, &handle, &first);
    let paint = &raw.get("fillPaints").and_then(Raw::as_array).unwrap()[0];
    let Raw::Object(fields) = paint else { panic!() };
    assert_eq!(fields.keys().count(), 16);
    assert_eq!(paint.get("originalImageWidth").and_then(Raw::as_u32), Some(1));
    assert_eq!(paint.get("scale").and_then(Raw::as_f32), Some(1.0));
    assert_eq!(paint.get("image"), paint.get("imageThumbnail"));

    // Again, by path and by hash: still once.
    let second = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &second, json!({ "fills": [{ "type": "IMAGE", "imageHash": hash, "scaleMode": "FIT" }] })).unwrap();
    set(&mut kit, &handle, &first, json!({ "strokes": [{ "type": "IMAGE", "image": path, "scaleMode": "STRETCH" }] })).unwrap();
    assert_eq!(images_in(&kit, &handle), before + 1);
    assert_eq!(read(&mut kit, &handle, &second)["fills"][0]["scaleMode"], "FIT");
}

#[test]
fn a_jpeg_is_sized_from_its_own_header() {
    let (mut kit, handle) = kit();
    // SOI, then a baseline start-of-frame: 32 rows of 64 columns.
    let jpeg = [0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x20, 0x00, 0x40, 0x03, 0x01, 0x22, 0x00, 0xFF, 0xD9];
    let path = image_file(&kit, "photo.jpg", &jpeg);
    let frame = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &frame, json!({ "fills": [{ "type": "IMAGE", "image": path }] })).unwrap();
    let raw = raw_of(&kit, &handle, &frame);
    let paint = &raw.get("fillPaints").and_then(Raw::as_array).unwrap()[0];
    assert_eq!((paint.get("originalImageWidth").and_then(Raw::as_u32), paint.get("originalImageHeight").and_then(Raw::as_u32)), (Some(64), Some(32)));
}

#[test]
fn every_image_refusal_names_its_reason_and_embeds_nothing() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    let text = image_file(&kit, "notes.txt", b"not an image");
    let png = image_file(&kit, "ok.png", PNG);
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let images = images_in(&kit, &handle);
    let cases = [
        (json!({ "type": "IMAGE", "image": "/nonexistent/cover.png" }), "cannot read"),
        (json!({ "type": "IMAGE", "image": text }), "not a PNG or JPEG"),
        (json!({ "type": "IMAGE", "imageHash": "00".repeat(20) }), "embeds no image"),
        (json!({ "type": "IMAGE", "image": png, "scaleMode": "TILE" }), "expected FILL, FIT or STRETCH"),
        (json!({ "type": "IMAGE" }), "needs image"),
        (json!({ "type": "IMAGE", "image": png, "rotation": 90 }), "not accepted"),
    ];
    for (paint, reason) in cases {
        let error = set(&mut kit, &handle, &frame, json!({ "fills": [paint.clone()] })).unwrap_err();
        assert!(error.to_string().contains(reason), "{paint}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
    assert_eq!(images_in(&kit, &handle), images, "a refused call embeds nothing");
}

#[test]
fn a_number_given_to_a_text_property_is_its_text() {
    let (mut kit, handle) = kit();
    let [.., instance] = with_properties(&mut kit, &handle);
    set(&mut kit, &handle, &instance, json!({ "componentProperties": { "Label": 32 } })).unwrap();
    assert_eq!(read(&mut kit, &handle, &instance)["componentProperties"]["Label"], "32");
    set(&mut kit, &handle, &instance, json!({ "componentProperties": { "Label": 2.5 } })).unwrap();
    assert_eq!(read(&mut kit, &handle, &instance)["componentProperties"]["Label"], "2.5");
}

// ── Shapes ────────────────────────────────────────────────────

fn shape(kit: &mut Toolkit, handle: &str, args: Value) -> Result<Value, ToolError> {
    let mut args = args;
    args["handle"] = json!(handle);
    args["parent"] = json!("1:1");
    call(kit, "create_shape", args)
}

#[test]
fn each_shape_is_written_as_figmas_own_type_with_no_geometry() {
    let (mut kit, handle) = kit();
    let violet = json!([{ "type": "SOLID", "color": "#7C3AED" }]);
    let cases = [
        (json!({ "type": "RECTANGLE", "width": 160, "height": 100, "cornerRadius": 16, "fills": violet }), "RECTANGLE", "ROUNDED_RECTANGLE"),
        (json!({ "type": "ELLIPSE", "width": 100, "height": 100, "fills": violet }), "ELLIPSE", "ELLIPSE"),
        (json!({ "type": "LINE", "width": 300, "strokes": [{ "type": "SOLID", "color": "#000000" }], "strokeWeight": 4 }), "LINE", "LINE"),
        (json!({ "type": "POLYGON", "width": 100, "height": 100, "pointCount": 6, "fills": violet }), "POLYGON", "REGULAR_POLYGON"),
        (json!({ "type": "STAR", "width": 100, "height": 100, "pointCount": 5, "innerRadius": 0.4, "fills": violet }), "STAR", "STAR"),
    ];
    for (args, shown, stored) in cases {
        let id = shape(&mut kit, &handle, args.clone()).unwrap()["id"].as_str().unwrap().to_string();
        assert_eq!(read(&mut kit, &handle, &id)["type"], shown, "{args}");
        let raw = raw_of(&kit, &handle, &id);
        assert_eq!(raw.get("type").and_then(Raw::as_str), Some(stored));
        assert!(raw.get("fillGeometry").is_none() && raw.get("strokeGeometry").is_none(), "{args}");
        for field in ["guid", "phase", "parentIndex", "type", "name", "visible", "opacity", "size", "transform", "strokeWeight", "strokeAlign", "strokeJoin"] {
            assert!(raw.get(field).is_some(), "{shown} has no {field}");
        }
        // Drawn from its own values, not skipped.
        let picture = call(&mut kit, "render_node", json!({ "handle": handle, "id": id })).unwrap();
        let notes = picture["caption"]["notes"].to_string();
        assert!(!notes.contains("not drawn"), "{shown}: {notes}");
    }
}

#[test]
fn a_shapes_own_values_are_set_and_read_in_the_plugin_apis_names() {
    let (mut kit, handle) = kit();
    let ring = shape(&mut kit, &handle, json!({ "type": "ELLIPSE", "width": 80, "height": 80, "arcData": { "innerRadius": 0.6 }, "fills": [{ "type": "SOLID", "color": "#16A34A" }] })).unwrap()["id"].as_str().unwrap().to_string();
    assert!((read(&mut kit, &handle, &ring)["arcData"]["innerRadius"].as_f64().unwrap() - 0.6).abs() < 1e-4);
    set(&mut kit, &handle, &ring, json!({ "arcData": { "startingAngle": 0, "endingAngle": std::f64::consts::PI, "innerRadius": 0 } })).unwrap();
    assert!((read(&mut kit, &handle, &ring)["arcData"]["endingAngle"].as_f64().unwrap() - std::f64::consts::PI).abs() < 1e-5);

    let star = shape(&mut kit, &handle, json!({ "type": "STAR", "width": 60, "height": 60 })).unwrap()["id"].as_str().unwrap().to_string();
    let node = read(&mut kit, &handle, &star);
    assert_eq!(node["pointCount"].as_u64(), Some(5));
    assert!((node["innerRadius"].as_f64().unwrap() - 0.382).abs() < 1e-4);
    set(&mut kit, &handle, &star, json!({ "pointCount": 8, "innerRadius": 0.5, "fills": [{ "type": "SOLID", "color": "#FACC15" }], "width": 90 })).unwrap();
    let node = read(&mut kit, &handle, &star);
    assert_eq!((node["pointCount"].as_u64(), node["innerRadius"].as_f64(), node["width"].as_f64()), (Some(8), Some(0.5), Some(90.0)));
    assert_eq!(node["fills"][0]["color"], "#FACC15");
}

#[test]
fn every_shape_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let ring = shape(&mut kit, &handle, json!({ "type": "ELLIPSE", "width": 80, "height": 80 })).unwrap()["id"].as_str().unwrap().to_string();
    let line = shape(&mut kit, &handle, json!({ "type": "LINE", "width": 80, "strokes": [{ "type": "SOLID", "color": "#000000" }] })).unwrap()["id"].as_str().unwrap().to_string();
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let creates = [
        (json!({ "type": "HEART", "width": 10, "height": 10 }), "expected RECTANGLE"),
        (json!({ "type": "LINE", "width": 100 }), "drawn by its strokes"),
        (json!({ "type": "LINE", "width": 100, "height": 4, "strokes": [{ "type": "SOLID", "color": "#000000" }] }), "a line has none"),
        (json!({ "type": "POLYGON", "width": 10, "height": 10, "pointCount": 2 }), "from 3 to 60"),
        (json!({ "type": "STAR", "width": 10, "height": 10, "innerRadius": 1.5 }), "between 0 and 1"),
        (json!({ "type": "RECTANGLE", "width": 10, "height": 10, "arcData": {} }), "only an ellipse"),
        (json!({ "type": "ELLIPSE", "width": 10, "height": 10, "cornerRadius": 4 }), "no corners"),
        (json!({ "type": "RECTANGLE", "width": 10 }), "\"height\" is required"),
    ];
    for (args, reason) in creates {
        let error = shape(&mut kit, &handle, args.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{args}: {error}");
    }
    for (id, properties, reason) in [
        (&ring, json!({ "pointCount": 5 }), "only a polygon or a star"),
        (&ring, json!({ "cornerRadius": 4 }), "no corners"),
        (&line, json!({ "height": 3 }), "a line has none"),
        (&ring, json!({ "arcData": { "innerRadius": 1 } }), "from 0 up to 1"),
    ] {
        let error = set(&mut kit, &handle, id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

// ── Groups, sections and duplicates ───────────────────────────

fn frame_at(kit: &mut Toolkit, handle: &str, parent: &str, name: &str, (x, y, w, h): (f32, f32, f32, f32)) -> String {
    call(kit, "create_frame", json!({ "handle": handle, "parent": parent, "name": name, "x": x, "y": y, "width": w, "height": h })).unwrap()["id"].as_str().unwrap().to_string()
}

fn at_of(kit: &mut Toolkit, handle: &str, id: &str) -> (f64, f64, f64, f64) {
    let n = read(kit, handle, id);
    (n["x"].as_f64().unwrap(), n["y"].as_f64().unwrap(), n["width"].as_f64().unwrap(), n["height"].as_f64().unwrap())
}

#[test]
fn a_group_sits_at_its_layers_bounds_and_keeps_each_in_place() {
    let (mut kit, handle) = kit();
    let a = frame_at(&mut kit, &handle, "1:1", "A", (40.0, 40.0, 100.0, 100.0));
    let b = frame_at(&mut kit, &handle, "1:1", "B", (120.0, 110.0, 50.0, 50.0));
    let group = call(&mut kit, "group_nodes", json!({ "handle": handle, "ids": [a, b], "name": "Pair" })).unwrap()["id"].as_str().unwrap().to_string();
    let node = read(&mut kit, &handle, &group);
    assert_eq!(node["type"], "GROUP");
    assert_eq!(at_of(&mut kit, &handle, &group), (40.0, 40.0, 130.0, 120.0));
    assert_eq!(at_of(&mut kit, &handle, &a), (0.0, 0.0, 100.0, 100.0));
    assert_eq!(at_of(&mut kit, &handle, &b), (80.0, 70.0, 50.0, 50.0));
    let raw = raw_of(&kit, &handle, &group);
    assert_eq!(raw.get("resizeToFit").and_then(Raw::as_bool), Some(true));
    assert_eq!(raw.get("frameMaskDisabled").and_then(Raw::as_bool), Some(false));
    assert!(raw.get("fillPaints").is_none());

    // Moving a child fits the group again, the other child staying put.
    set(&mut kit, &handle, &b, json!({ "x": 150, "y": 90 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &group), (40.0, 40.0, 200.0, 140.0));
    set(&mut kit, &handle, &a, json!({ "x": 20 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &group), (60.0, 40.0, 180.0, 140.0));
    assert_eq!(at_of(&mut kit, &handle, &a), (0.0, 0.0, 100.0, 100.0));
    assert_eq!(at_of(&mut kit, &handle, &b), (130.0, 90.0, 50.0, 50.0));

    // Ungrouping puts them back in the page where they are.
    call(&mut kit, "ungroup", json!({ "handle": handle, "id": group })).unwrap();
    assert!(call(&mut kit, "read_node", json!({ "handle": handle, "id": group })).is_err());
    assert_eq!(at_of(&mut kit, &handle, &a), (60.0, 40.0, 100.0, 100.0));
    assert_eq!(at_of(&mut kit, &handle, &b), (190.0, 130.0, 50.0, 50.0));
    assert_eq!(read(&mut kit, &handle, &a)["path"], "Screens / A");
}

#[test]
fn a_section_holds_the_frames_moved_into_it() {
    let (mut kit, handle) = kit();
    let section = call(&mut kit, "create_section", json!({ "handle": handle, "parent": "1:1", "name": "Onboarding", "x": 600, "y": 0, "width": 400, "height": 300 })).unwrap()["id"].as_str().unwrap().to_string();
    assert_eq!(read(&mut kit, &handle, &section)["type"], "SECTION");
    let raw = raw_of(&kit, &handle, &section);
    assert!(raw.get("editInfo").is_none() && raw.get("fillGeometry").is_none() && raw.get("fillPaints").is_some());
    let screen = created_frame(&mut kit, &handle);
    call(&mut kit, "move_node", json!({ "handle": handle, "id": screen, "parent": section, "x": 20, "y": 20 })).unwrap();
    assert_eq!(read(&mut kit, &handle, &screen)["path"], "Screens / Onboarding / Styled");
    let inside = frame_at(&mut kit, &handle, &section, "Made inside", (40.0, 120.0, 80.0, 80.0));
    assert_eq!(read(&mut kit, &handle, &inside)["path"], "Screens / Onboarding / Made inside");
}

#[test]
fn a_duplicate_copies_every_node_with_new_ids_and_leaves_the_original() {
    let (mut kit, handle) = kit();
    let card = frame_at(&mut kit, &handle, "1:1", "Card", (0.0, 0.0, 200.0, 120.0));
    set(&mut kit, &handle, &card, json!({ "fills": [{ "type": "SOLID", "color": "#0B0B12" }] })).unwrap();
    let label = call(&mut kit, "create_text", json!({ "handle": handle, "parent": card, "characters": "Original", "x": 16, "y": 16 })).unwrap()["id"].as_str().unwrap().to_string();
    let chip = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": card, "component": "1:4", "x": 16, "y": 60 })).unwrap()["id"].as_str().unwrap().to_string();
    let before = (read(&mut kit, &handle, &card), read(&mut kit, &handle, &label));

    let out = call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": card, "x": 240, "y": 0, "name": "Card 2" })).unwrap();
    assert_eq!(out["nodes"], 3);
    assert_eq!(out["copies"][0]["from"], card);
    assert!(out["copies"].as_array().unwrap().iter().any(|c| c["from"] == label));
    let copy = out["id"].as_str().unwrap().to_string();
    let node = call(&mut kit, "read_node", json!({ "handle": handle, "id": copy, "depth": 1 })).unwrap();
    assert_eq!((node["name"].as_str(), node["x"].as_f64(), node["fills"][0]["color"].as_str()), (Some("Card 2"), Some(240.0), Some("#0B0B12")));
    let kids = node["children"].as_array().unwrap();
    assert_eq!(kids.len(), 2);
    assert!(kids.iter().all(|k| k["id"] != label && k["id"] != chip));
    let copied_chip = kids.iter().find(|k| k["type"] == "INSTANCE").unwrap();
    assert_eq!(copied_chip["componentId"], "1:4");

    // Changing the copy leaves the original.
    let copied_label = kids.iter().find(|k| k["type"] == "TEXT").unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &copied_label, json!({ "characters": "Copy" })).unwrap();
    assert_eq!((read(&mut kit, &handle, &card), read(&mut kit, &handle, &label)), before);
}

#[test]
fn every_structure_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let a = frame_at(&mut kit, &handle, "1:1", "A", (0.0, 0.0, 10.0, 10.0));
    let b = frame_at(&mut kit, &handle, "1:6", "B", (0.0, 0.0, 10.0, 10.0));
    let c = frame_at(&mut kit, &handle, "1:1", "C", (20.0, 0.0, 10.0, 10.0));
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    let cases = [
        ("group_nodes", json!({ "ids": [a, b] }), "share one"),
        ("group_nodes", json!({ "ids": ["1:1"] }), "is a page"),
        ("group_nodes", json!({ "ids": [] }), "one or more node ids"),
        ("ungroup", json!({ "id": c }), "not a group"),
        ("create_section", json!({ "parent": a, "width": 10, "height": 10 }), "not a page"),
        ("duplicate_node", json!({ "id": "1:4" }), "is a component"),
        ("duplicate_node", json!({ "id": "1:6" }), "is a page"),
    ];
    for (tool, args, reason) in cases {
        let mut args = args;
        args["handle"] = json!(handle);
        let error = call(&mut kit, tool, args.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{tool} {args}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

// ── Sizing and constraints ────────────────────────────────────

#[test]
fn a_rotation_is_written_and_read_as_figmas_panel_shows_it() {
    let (mut kit, handle) = kit();
    let id = frame_at(&mut kit, &handle, "1:1", "Tag", (60.0, 40.0, 120.0, 60.0));
    set(&mut kit, &handle, &id, json!({ "rotation": 30 })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert!((node["rotation"].as_f64().unwrap() - 30.0).abs() < 1e-3, "{node}");
    assert!((node["x"].as_f64().unwrap() - 60.0).abs() < 1e-3 && (node["y"].as_f64().unwrap() - 40.0).abs() < 1e-3, "{node}");
    // A matrix Figma reads as 30°.
    let t = raw_of(&kit, &handle, &id).get("transform").cloned().unwrap();
    assert!((t.get("m01").and_then(Raw::as_f32).unwrap() - 0.5).abs() < 1e-4);
    assert!((t.get("m10").and_then(Raw::as_f32).unwrap() + 0.5).abs() < 1e-4);
    // Moving it keeps the rotation, and x and y read back as set.
    set(&mut kit, &handle, &id, json!({ "x": 10, "y": 20 })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert!((node["x"].as_f64().unwrap() - 10.0).abs() < 1e-3 && (node["rotation"].as_f64().unwrap() - 30.0).abs() < 1e-3);
}

#[test]
fn children_follow_their_constraints_when_grafta_resizes_their_frame() {
    let (mut kit, handle) = kit();
    let frame = frame_at(&mut kit, &handle, "1:1", "Frame", (0.0, 0.0, 300.0, 120.0));
    let right = frame_at(&mut kit, &handle, &frame, "Right", (240.0, 10.0, 50.0, 30.0));
    let wide = frame_at(&mut kit, &handle, &frame, "Wide", (10.0, 50.0, 280.0, 20.0));
    let centre = frame_at(&mut kit, &handle, &frame, "Centre", (125.0, 80.0, 50.0, 30.0));
    let scaled = frame_at(&mut kit, &handle, &frame, "Scaled", (30.0, 0.0, 60.0, 10.0));
    let inner = frame_at(&mut kit, &handle, &wide, "Inner right", (250.0, 0.0, 30.0, 20.0));
    set(&mut kit, &handle, &right, json!({ "constraints": { "horizontal": "MAX" } })).unwrap();
    set(&mut kit, &handle, &wide, json!({ "constraints": { "horizontal": "STRETCH" } })).unwrap();
    set(&mut kit, &handle, &centre, json!({ "constraints": { "horizontal": "CENTER", "vertical": "CENTER" } })).unwrap();
    set(&mut kit, &handle, &scaled, json!({ "constraints": { "horizontal": "SCALE" } })).unwrap();
    set(&mut kit, &handle, &inner, json!({ "constraints": { "horizontal": "MAX" } })).unwrap();
    assert_eq!(read(&mut kit, &handle, &centre)["constraints"], json!({ "horizontal": "CENTER", "vertical": "CENTER" }));

    set(&mut kit, &handle, &frame, json!({ "width": 400, "height": 160 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &right), (340.0, 10.0, 50.0, 30.0));
    assert_eq!(at_of(&mut kit, &handle, &wide), (10.0, 50.0, 380.0, 20.0));
    assert_eq!(at_of(&mut kit, &handle, &centre), (175.0, 100.0, 50.0, 30.0));
    assert_eq!(at_of(&mut kit, &handle, &scaled), (40.0, 0.0, 80.0, 10.0));
    // And a stretched child's own children follow it in turn.
    assert_eq!(at_of(&mut kit, &handle, &inner), (350.0, 0.0, 30.0, 20.0));

    // MIN is the default, and is written by leaving the field out.
    set(&mut kit, &handle, &right, json!({ "constraints": { "horizontal": "MIN" } })).unwrap();
    assert!(raw_of(&kit, &handle, &right).get("horizontalConstraint").is_none());
}

#[test]
fn fill_hug_and_fixed_and_minimum_and_maximum_sizes() {
    let (mut kit, handle) = kit();
    let row = frame_at(&mut kit, &handle, "1:1", "Row", (0.0, 0.0, 400.0, 60.0));
    set(&mut kit, &handle, &row, json!({ "layoutMode": "HORIZONTAL", "padding": 10, "itemSpacing": 10, "layoutSizingHorizontal": "FIXED" })).unwrap();
    let fixed = frame_at(&mut kit, &handle, &row, "Fixed", (0.0, 0.0, 80.0, 40.0));
    let fill = frame_at(&mut kit, &handle, &row, "Fill", (0.0, 0.0, 40.0, 40.0));
    set(&mut kit, &handle, &fill, json!({ "layoutSizingHorizontal": "FILL", "layoutSizingVertical": "FILL" })).unwrap();
    let node = read(&mut kit, &handle, &fill);
    assert_eq!((node["layoutSizingHorizontal"].as_str(), node["layoutSizingVertical"].as_str()), (Some("FILL"), Some("FILL")));
    assert_eq!(at_of(&mut kit, &handle, &fill), (100.0, 10.0, 290.0, 40.0));
    assert_eq!(read(&mut kit, &handle, &fixed)["layoutSizingHorizontal"], "FIXED");

    // A maximum holds a filling child, a minimum a fixed one.
    set(&mut kit, &handle, &fill, json!({ "maxWidth": 200 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &fill).2, 200.0);
    set(&mut kit, &handle, &fixed, json!({ "minWidth": 120 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &fixed).2, 120.0);
    let node = read(&mut kit, &handle, &fill);
    assert_eq!(node["maxWidth"].as_f64(), Some(200.0));
    // Written as the pair the files store: infinity for no maximum.
    let raw = raw_of(&kit, &handle, &fill);
    let max = raw.get("maxSize").and_then(|m| m.get("value")).cloned().unwrap();
    assert_eq!(max.get("x").and_then(Raw::as_f32), Some(200.0));
    assert!(max.get("y").and_then(Raw::as_f32).unwrap().is_infinite());
    set(&mut kit, &handle, &fill, json!({ "maxWidth": null })).unwrap();
    assert!(raw_of(&kit, &handle, &fill).get("maxSize").is_none());

    // HUG wraps the row around its children.
    set(&mut kit, &handle, &fill, json!({ "layoutSizingHorizontal": "FIXED" })).unwrap();
    set(&mut kit, &handle, &row, json!({ "layoutSizingHorizontal": "HUG" })).unwrap();
    assert_eq!(read(&mut kit, &handle, &row)["layoutSizingHorizontal"], "HUG");
    let width = at_of(&mut kit, &handle, &row).2;
    assert_eq!(width, 10.0 + 120.0 + 10.0 + at_of(&mut kit, &handle, &fill).2 + 10.0);
}

#[test]
fn an_instance_is_resized_and_its_cache_dropped() {
    let (mut kit, handle) = kit();
    let instance = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": "1:4", "x": 0, "y": 0 })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &instance, json!({ "width": 200, "height": 48, "rotation": 15 })).unwrap();
    let node = read(&mut kit, &handle, &instance);
    assert_eq!((node["width"].as_f64(), node["height"].as_f64()), (Some(200.0), Some(48.0)));
    assert!((node["rotation"].as_f64().unwrap() - 15.0).abs() < 1e-3);
}

#[test]
fn every_sizing_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let loose = frame_at(&mut kit, &handle, "1:1", "Loose", (0.0, 0.0, 50.0, 50.0));
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    for (properties, reason) in [
        (json!({ "layoutSizingHorizontal": "FILL" }), "has none"),
        (json!({ "layoutSizingVertical": "HUG" }), "no auto-layout"),
        (json!({ "layoutSizingHorizontal": "GROW" }), "expected FIXED, HUG or FILL"),
        (json!({ "constraints": { "horizontal": "LEFT" } }), "expected MIN, CENTER"),
        (json!({ "constraints": { "depth": "MIN" } }), "expected horizontal or vertical"),
        (json!({ "minWidth": -1 }), "0 or more"),
        (json!({ "minWidth": 300, "maxWidth": 200 }), "larger than the maximum"),
        (json!({ "rotation": "a lot" }), "expected degrees"),
    ] {
        let error = set(&mut kit, &handle, &loose, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

// ── rich text ───────────────────────────────────────────────────────────

fn rich(kit: &mut Toolkit, handle: &str, extra: Value) -> String {
    let mut args = json!({ "parent": "1:1", "fontFamily": "Inter", "fontStyle": "Regular", "fontSize": 16 });
    args.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    text(kit, handle, args).unwrap()["id"].as_str().unwrap().to_string()
}

#[test]
fn runs_are_ids_and_a_table_holding_only_what_differs() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "runs": [
        { "characters": "Your order " },
        { "characters": "ships today", "fontStyle": "Bold" },
        { "characters": ". " },
        { "characters": "Free", "fills": [{ "type": "SOLID", "color": "#16A34A" }] },
        { "characters": " and " },
        { "characters": "fast", "fontStyle": "Bold", "fontSize": 16 },
        { "characters": " too." },
    ] }));
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    let data = &raw["textData"];
    assert_eq!(data["characters"], "Your order ships today. Free and fast too.");
    // The trailing zeros are left out, as Figma writes them.
    let ids: Vec<u64> = data["characterStyleIDs"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
    assert_eq!(ids.len(), "Your order ships today. Free and fast".len());
    assert_eq!(&ids[11..22], &[1; 11]);
    assert_eq!(&ids[24..28], &[2; 4]);
    // A run styled as an earlier one shares its id, and a size equal to the
    // layer's is not written.
    assert_eq!(&ids[33..37], &[1; 4]);
    let table = data["styleOverrideTable"].as_array().unwrap();
    assert_eq!(table.len(), 2);
    // Inter Bold as the file already stores it, and its version beside it.
    assert_eq!(table[0].as_object().unwrap().keys().collect::<Vec<_>>(), ["fontName", "fontVersion", "styleID"]);
    assert_eq!(table[0]["fontName"]["style"], "Bold");
    assert_eq!(table[1].as_object().unwrap().keys().collect::<Vec<_>>(), ["fillPaints", "styleID"]);

    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["runs"][0], json!({ "characters": "Your order " }));
    assert_eq!(node["runs"][1], json!({ "characters": "ships today", "fontFamily": "Inter", "fontStyle": "Bold" }));
    assert_eq!(node["runs"][3]["fills"][0]["color"], "#16A34A");
    assert_eq!(node["runs"][6], json!({ "characters": " too." }));
}

#[test]
fn a_character_outside_the_basic_plane_takes_two_places() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "runs": [{ "characters": "🎉 " }, { "characters": "Done", "fontStyle": "Bold" }] }));
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    assert_eq!(raw["textData"]["characterStyleIDs"], json!([0, 0, 0, 1, 1, 1, 1]));
    assert_eq!(read(&mut kit, &handle, &id)["runs"][1]["characters"], "Done");
}

#[test]
fn a_link_is_underlined_and_must_be_a_web_address() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "runs": [
        { "characters": "Read the " },
        { "characters": "terms", "hyperlink": { "type": "URL", "value": "https://example.com/terms" } },
    ] }));
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["runs"][1]["hyperlink"], json!({ "type": "URL", "value": "https://example.com/terms" }));
    assert_eq!(node["runs"][1]["textDecoration"], "UNDERLINE");

    for (link, reason) in [
        (json!({ "type": "URL", "value": "javascript:alert(1)" }), "not a web address"),
        (json!({ "type": "URL", "value": "https://" }), "not a web address"),
        (json!({ "type": "NODE", "value": "1:2" }), "expected"),
        (json!("https://example.com"), "expected"),
    ] {
        let error = text(&mut kit, &handle, json!({ "parent": "1:1", "runs": [{ "characters": "x", "hyperlink": link }] })).unwrap_err();
        assert!(error.to_string().contains(reason), "{link}: {error}");
    }
}

#[test]
fn lists_are_lines_and_an_item_starts_a_list_after_another_kind() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({
        "characters": "Shopping\nApples\nPears\nNested\nKiwis\nSteps\nOne\nTwo",
        "paragraphs": [
            {}, { "listType": "UNORDERED" }, { "listType": "UNORDERED" }, { "listType": "UNORDERED", "indentation": 2 },
            { "listType": "UNORDERED" }, { "listType": "NONE" }, { "listType": "ORDERED" }, { "listType": "ORDERED" },
        ],
    }));
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    let lines = raw["textData"]["lines"].as_array().unwrap();
    let shape: Vec<(String, i64, bool)> = lines
        .iter()
        .map(|l| (l["lineType"].as_str().unwrap().to_string(), l["indentationLevel"].as_i64().unwrap(), l["isFirstLineOfList"].as_bool().unwrap()))
        .collect();
    let expected = [
        ("PLAIN", 0, false), ("UNORDERED_LIST", 1, true), ("UNORDERED_LIST", 1, false), ("UNORDERED_LIST", 2, true),
        ("UNORDERED_LIST", 1, false), ("PLAIN", 0, false), ("ORDERED_LIST", 1, true), ("ORDERED_LIST", 1, false),
    ];
    assert_eq!(shape, expected.map(|(k, l, f)| (k.to_string(), l, f)));
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["paragraphs"][3], json!({ "listType": "UNORDERED", "indentation": 2 }));
    assert_eq!(node["paragraphs"][0], json!({ "listType": "NONE" }));

    for (paragraphs, reason) in [
        (json!([{}]), "has 2 paragraphs"),
        (json!([{ "listType": "BULLET" }, {}]), "expected NONE, UNORDERED or ORDERED"),
        (json!([{ "listType": "ORDERED", "indentation": 3 }, {}]), "expected 1 or 2"),
        (json!([{ "indentation": 1 }, {}]), "not a list"),
    ] {
        let error = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "a\nb", "paragraphs": paragraphs })).unwrap_err();
        assert!(error.to_string().contains(reason), "{paragraphs}: {error}");
    }
}

#[test]
fn the_layer_fields_are_written_and_their_defaults_left_out() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({
        "characters": "new arrivals", "textCase": "UPPER", "textDecoration": "UNDERLINE", "paragraphSpacing": 16,
        "paragraphIndent": 24, "textAlignVertical": "CENTER", "width": 240, "height": 120,
    }));
    let node = read(&mut kit, &handle, &id);
    let style = &node["style"];
    assert_eq!((style["textCase"].clone(), style["textDecoration"].clone()), (json!("UPPER"), json!("UNDERLINE")));
    assert_eq!((style["paragraphSpacing"].clone(), style["paragraphIndent"].clone()), (json!(16), json!(24)));
    assert_eq!(style["textAlignVertical"], "CENTER");
    // A fixed box is NONE, which is no field at all.
    assert!(style.get("textAutoResize").is_none(), "{style}");
    assert_eq!((node["width"].clone(), node["height"].clone()), (json!(240), json!(120)));
    // The characters are stored as written.
    assert_eq!(node["characters"], "new arrivals");

    set(&mut kit, &handle, &id, json!({ "textCase": "ORIGINAL", "textDecoration": "NONE", "paragraphSpacing": 0, "textAutoResize": "HEIGHT" })).unwrap();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    for field in ["textCase", "textDecoration", "paragraphSpacing"] {
        assert!(raw.get(field).is_none(), "{field} left in {raw}");
    }
    assert_eq!(raw["textAutoResize"], "HEIGHT");
    assert_eq!(raw["paragraphIndent"], 24);
}

#[test]
fn a_style_set_on_the_layer_reaches_every_run() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "runs": [
        { "characters": "Old " },
        { "characters": "249 €", "fontSize": 28, "fills": [{ "type": "SOLID", "color": "#FF0000" }] },
        { "characters": " new" , "fontSize": 20 },
    ] }));
    // The size now applies to every character: the run that only changed its
    // size has nothing left, and the other keeps its colour.
    set(&mut kit, &handle, &id, json!({ "fontSize": 18 })).unwrap();
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    let table = raw["textData"]["styleOverrideTable"].as_array().unwrap();
    assert_eq!(table.len(), 1);
    assert!(table[0].get("fontSize").is_none() && table[0].get("fillPaints").is_some());
    assert_eq!(raw["textData"]["characterStyleIDs"].as_array().unwrap().len(), "Old 249 €".encode_utf16().count());

    // New characters give the text one style, and the result says so.
    let out = set(&mut kit, &handle, &id, json!({ "characters": "Plain now" })).unwrap();
    assert!(out["notes"][0].as_str().unwrap().contains("layer's style"), "{out}");
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    assert!(raw["textData"].get("styleOverrideTable").is_none() && raw["textData"].get("characterStyleIDs").is_none());
}

#[test]
fn runs_and_lists_set_on_an_existing_text_drop_its_layout_data() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "characters": "One\nTwo" }));
    set(&mut kit, &handle, &id, json!({ "runs": [{ "characters": "Pay " }, { "characters": "now", "fontStyle": "Bold" }, { "characters": "\nLater" }],
                                        "paragraphs": [{ "listType": "ORDERED" }, { "listType": "ORDERED" }] })).unwrap();
    let node = read(&mut kit, &handle, &id);
    assert_eq!(node["characters"], "Pay now\nLater");
    assert_eq!(node["runs"][1]["characters"], "now");
    assert_eq!(node["paragraphs"][1], json!({ "listType": "ORDERED", "indentation": 1 }));
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id })).unwrap();
    assert!(raw.get("derivedTextData").is_none());
}

#[test]
fn a_run_at_a_larger_size_makes_the_provisional_box_taller() {
    let (mut kit, handle) = kit();
    let plain = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Price 249 €", "fontSize": 16, "lineHeight": { "value": 120, "unit": "PERCENT" } })).unwrap();
    let mixed = text(&mut kit, &handle, json!({ "parent": "1:1", "fontSize": 16, "lineHeight": { "value": 120, "unit": "PERCENT" },
        "runs": [{ "characters": "Price " }, { "characters": "249 €", "fontSize": 28 }] })).unwrap();
    assert!(mixed["size"]["height"].as_f64() > plain["size"]["height"].as_f64(), "{mixed} vs {plain}");
}

#[test]
fn every_rich_text_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "characters": "Hola" }));
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    for (properties, reason) in [
        (json!({ "characters": "a", "runs": [{ "characters": "b" }] }), "not both"),
        (json!({ "runs": [] }), "non-empty array"),
        (json!({ "runs": [{ "characters": "" }] }), "non-empty text"),
        (json!({ "runs": [{ "characters": "a", "color": "#000000" }] }), "not a run key"),
        (json!({ "runs": [{ "characters": "a", "textDecoration": "OVERLINE" }] }), "expected NONE, UNDERLINE"),
        (json!({ "runs": [{ "characters": "a", "fontSize": 0 }] }), "greater than 0"),
        (json!({ "textCase": "SHOUT" }), "expected ORIGINAL"),
        (json!({ "textAlignVertical": "MIDDLE" }), "expected TOP"),
        (json!({ "textAutoResize": "TRUNCATE" }), "expected NONE"),
        (json!({ "paragraphSpacing": -4 }), "0 or more"),
        (json!({ "paragraphs": [{}, {}] }), "has 1 paragraph"),
    ] {
        let error = set(&mut kit, &handle, &id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

#[test]
fn runs_read_back_can_be_written_back_unchanged() {
    let (mut kit, handle) = kit();
    let id = rich(&mut kit, &handle, json!({ "runs": [
        { "characters": "Price " },
        { "characters": "249 €", "textDecoration": "STRIKETHROUGH", "fills": [{ "type": "SOLID", "color": "#6B7280" }] },
        { "characters": " " },
        { "characters": "199 €", "fontStyle": "Bold", "fontSize": 28 },
        { "characters": " · " },
        { "characters": "terms", "hyperlink": { "type": "URL", "value": "https://example.com/terms" }, "letterSpacing": { "value": 2, "unit": "PERCENT" } },
    ] }));
    let before = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    let runs = read(&mut kit, &handle, &id)["runs"].clone();
    set(&mut kit, &handle, &id, json!({ "runs": runs })).unwrap();
    let after = call(&mut kit, "read_raw", json!({ "handle": handle, "id": id, "fields": ["textData"] })).unwrap();
    assert_eq!(after, before);
}

// ── text sized when it changes ─────────────────────────────────────────

/// A button that hugs its label, as the ninth session drew them.
fn hugging_button(kit: &mut Toolkit, handle: &str) -> (String, String) {
    let button = frame_at(kit, handle, "1:1", "Button", (0.0, 0.0, 10.0, 10.0));
    set(kit, handle, &button, json!({ "layoutMode": "HORIZONTAL", "padding": 12, "counterAxisAlignItems": "CENTER",
                                       "layoutSizingHorizontal": "HUG", "layoutSizingVertical": "HUG" })).unwrap();
    let label = text(kit, handle, json!({ "parent": button, "characters": "Buy", "fontFamily": "Inter", "fontStyle": "Regular",
                                          "fontSize": 14, "lineHeight": 20 })).unwrap()["id"].as_str().unwrap().to_string();
    (button, label)
}

/// The label sits in the middle of the button, padding apart from each side.
fn assert_centred(kit: &mut Toolkit, handle: &str, button: &str, label: &str) {
    let (_, _, bw, bh) = at_of(kit, handle, button);
    let (x, y, w, h) = at_of(kit, handle, label);
    assert_eq!((x, y), (12.0, 12.0), "the label's place");
    // Measured sizes have two decimals, which sums keep only nearly.
    assert!((bw - (w + 24.0)).abs() < 0.01 && (bh - (h + 24.0)).abs() < 0.01, "the button hugs the label: {bw}×{bh} around {w}×{h}");
}

#[test]
fn a_restyled_label_is_estimated_again_and_its_button_follows() {
    let (mut kit, handle) = kit();
    let (button, label) = hugging_button(&mut kit, &handle);
    let first = at_of(&mut kit, &handle, &label);
    assert_centred(&mut kit, &handle, &button, &label);

    let out = set(&mut kit, &handle, &label, json!({ "fontSize": 20 })).unwrap();
    assert!(out["size"]["provisional"].is_string(), "{out}");
    let larger = at_of(&mut kit, &handle, &label);
    assert!(larger.2 > first.2, "wider at 20: {larger:?} vs {first:?}");
    assert_eq!(out["size"]["width"].as_f64(), Some(larger.2));
    assert_centred(&mut kit, &handle, &button, &label);

    set(&mut kit, &handle, &label, json!({ "characters": "Continue to payment" })).unwrap();
    let longer = at_of(&mut kit, &handle, &label);
    assert!(longer.2 > larger.2);
    assert_centred(&mut kit, &handle, &button, &label);

    set(&mut kit, &handle, &label, json!({ "runs": [{ "characters": "Pay " }, { "characters": "now", "fontSize": 32 }] })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &label).3, 20.0, "a line height of 20 px holds at 32");
    assert_centred(&mut kit, &handle, &button, &label);
    assert!(call(&mut kit, "read_raw", json!({ "handle": handle, "id": label })).unwrap().get("derivedTextData").is_none());
}

#[test]
fn a_card_places_what_follows_a_title_at_its_new_height() {
    let (mut kit, handle) = kit();
    let card = frame_at(&mut kit, &handle, "1:1", "Card", (0.0, 0.0, 240.0, 10.0));
    set(&mut kit, &handle, &card, json!({ "layoutMode": "VERTICAL", "itemSpacing": 8, "layoutSizingVertical": "HUG" })).unwrap();
    let title = text(&mut kit, &handle, json!({ "parent": card, "characters": "Order summary", "fontSize": 16,
                                                "lineHeight": { "value": 125, "unit": "PERCENT" } })).unwrap()["id"].as_str().unwrap().to_string();
    let body = frame_at(&mut kit, &handle, &card, "Body", (0.0, 0.0, 240.0, 40.0));
    assert_eq!(at_of(&mut kit, &handle, &body).1, 20.0 + 8.0);
    set(&mut kit, &handle, &title, json!({ "fontSize": 24 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &title).3, 30.0);
    assert_eq!(at_of(&mut kit, &handle, &body).1, 30.0 + 8.0);
    assert_eq!(at_of(&mut kit, &handle, &card).3, 30.0 + 8.0 + 40.0);
}

#[test]
fn a_wrapping_text_keeps_its_width_a_fixed_box_its_size_and_a_colour_its_box() {
    let (mut kit, handle) = kit();
    let wrapping = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Free returns on every item", "fontSize": 16, "lineHeight": 20, "width": 120 })).unwrap()["id"].as_str().unwrap().to_string();
    let before = at_of(&mut kit, &handle, &wrapping);
    set(&mut kit, &handle, &wrapping, json!({ "fontSize": 24 })).unwrap();
    let after = at_of(&mut kit, &handle, &wrapping);
    assert_eq!(after.2, 120.0, "the width is kept");
    assert!(after.3 > before.3, "more lines: {after:?} vs {before:?}");

    let fixed = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Note", "fontSize": 16, "width": 200, "height": 80 })).unwrap()["id"].as_str().unwrap().to_string();
    let out = set(&mut kit, &handle, &fixed, json!({ "fontSize": 28 })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &fixed).2, 200.0);
    assert_eq!(at_of(&mut kit, &handle, &fixed).3, 80.0);
    assert!(out.get("size").is_none(), "{out}");

    let out = set(&mut kit, &handle, &wrapping, json!({ "fills": [{ "type": "SOLID", "color": "#16A34A" }] })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &wrapping), after);
    assert!(out.get("size").is_none(), "{out}");
}

#[test]
fn a_files_own_auto_layout_parent_is_taken_over_and_a_wrapping_one_named() {
    let (mut kit, handle) = kit();
    // The file's Card lays out its Title, a fixed box: given one that fits
    // its text, the title is estimated again and grafta lays the card out.
    let out = set(&mut kit, &handle, "1:3", json!({ "textAutoResize": "WIDTH_AND_HEIGHT" })).unwrap();
    assert!(out["size"].is_object(), "{out}");
    assert!(kit.session(&handle).unwrap().laid_out.iter().any(|f| *f == node_id("1:2")));

    let (mut wrapped, handle) = self::kit();
    let kit = &mut wrapped;
    {
        let session = kit.session_mut(&handle).unwrap();
        let at = session.raw_index[&node_id("1:2")];
        let Raw::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(Raw::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let Some(Raw::Object(card)) = changes.get_mut(at) else { unreachable!() };
        card.insert("stackMode", Raw::Enum("HORIZONTAL"));
        card.insert("stackWrap", Raw::Enum("WRAP"));
        session.refresh(&[node_id("1:2")]);
    }
    let out = set(kit, &handle, "1:3", json!({ "textAutoResize": "WIDTH_AND_HEIGHT" })).unwrap();
    assert_eq!(out["left_to_figma"]["frames"], json!(["1:2"]), "{out}");
    assert!(!kit.session(&handle).unwrap().laid_out.iter().any(|f| *f == node_id("1:2")));
}

#[test]
fn a_text_style_changed_reaches_every_text_using_it() {
    let (mut kit, handle) = kit();
    styles(&mut kit, &handle);
    let (button, label) = hugging_button(&mut kit, &handle);
    set(&mut kit, &handle, &label, json!({ "textStyle": "Heading/L" })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &label).3, 34.0, "estimated at the style's line height");
    assert_centred(&mut kit, &handle, &button, &label);

    let other = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "Heading" })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &other, json!({ "textStyle": "Heading/L" })).unwrap();
    let before = at_of(&mut kit, &handle, &label);
    let out = call(&mut kit, "set_style", json!({ "handle": handle, "id": "Heading/L", "fontSize": 40, "lineHeight": 48 })).unwrap();
    assert_eq!(out["texts_resized"], 2, "{out}");
    let after = at_of(&mut kit, &handle, &label);
    assert!(after.2 > before.2 && after.3 == 48.0, "{after:?}");
    assert_centred(&mut kit, &handle, &button, &label);
    for id in [&label, &other] {
        assert!(raw_of(&kit, &handle, id).get("derivedTextData").is_none());
    }
}

#[test]
fn a_bound_size_revalued_or_shown_in_another_mode_is_estimated_again() {
    let (mut kit, handle) = kit();
    let (card, title) = bound_card(&mut kit, &handle);
    new_variable(&mut kit, &handle, "Theme", "Type/L", "FLOAT", json!({ "Light": 16, "Dark": 32 })).unwrap();
    set(&mut kit, &handle, &title, json!({ "fontSize": { "variable": "Type/L" }, "lineHeight": { "value": 100, "unit": "PERCENT" } })).unwrap();
    assert_eq!(at_of(&mut kit, &handle, &title).3, 16.0);

    let out = set(&mut kit, &handle, &card, json!({ "explicitVariableModes": { "Theme": "Dark" } })).unwrap();
    assert_eq!(out["texts_resized"], 1, "{out}");
    assert_eq!(at_of(&mut kit, &handle, &title).3, 32.0);
    assert!(raw_of(&kit, &handle, &title).get("derivedTextData").is_none());

    let out = call(&mut kit, "set_variable", json!({ "handle": handle, "id": "Type/L", "values": { "Dark": 40 } })).unwrap();
    assert_eq!(out["texts_resized"], 1, "{out}");
    assert_eq!(at_of(&mut kit, &handle, &title).3, 40.0);
}

// ── instance fills and dashed strokes ──────────────────────────────────

/// A card component: white, a grey stroke inside, radius 12. Optionally
/// filled from a colour style.
fn card_component(kit: &mut Toolkit, handle: &str, styled: bool) -> String {
    let card = frame_at(kit, handle, "1:1", "Card", (0.0, 0.0, 200.0, 120.0));
    let fill = if styled {
        call(kit, "create_style", json!({ "handle": handle, "name": "Brand", "type": "FILL", "fills": [{ "type": "SOLID", "color": "#2563EB" }] })).unwrap();
        json!({ "fillStyle": "Brand" })
    } else {
        json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })
    };
    set(kit, handle, &card, fill).unwrap();
    set(kit, handle, &card, json!({ "strokes": [{ "type": "SOLID", "color": "#94A3B8" }], "strokeWeight": 1, "strokeAlign": "INSIDE", "cornerRadius": 12 })).unwrap();
    call(kit, "create_component", json!({ "handle": handle, "id": card })).unwrap();
    card
}

fn card_instance(kit: &mut Toolkit, handle: &str, component: &str) -> String {
    call(kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": component, "x": 300, "y": 0 })).unwrap()["id"].as_str().unwrap().to_string()
}

/// The entry of an instance's overrides addressed to its component itself.
fn own_entry(kit: &Toolkit, handle: &str, instance: &str, component: &str) -> Raw {
    let raw = raw_of(kit, handle, instance);
    let key = raw_of(kit, handle, component).get("guid").cloned().unwrap();
    raw.get("symbolData").and_then(|s| s.get("symbolOverrides")).and_then(Raw::as_array).unwrap()
        .iter().find(|e| e.get("guidPath").and_then(|p| p.get("guids")).and_then(Raw::as_array).is_some_and(|p| p == &vec![key.clone()]))
        .cloned().expect("an entry addressed to the component")
}

#[test]
fn an_instances_own_paint_is_on_it_and_in_the_entry_addressed_to_its_component() {
    let (mut kit, handle) = kit();
    let card = card_component(&mut kit, &handle, false);
    let instance = card_instance(&mut kit, &handle, &card);
    set(&mut kit, &handle, &instance, json!({ "fills": [{ "type": "SOLID", "color": "#7C3AED" }] })).unwrap();
    set(&mut kit, &handle, &instance, json!({ "strokes": [{ "type": "SOLID", "color": "#DC2626" }], "strokeWeight": 2, "strokeAlign": "OUTSIDE", "dashPattern": [4, 4] })).unwrap();

    let entry = own_entry(&kit, &handle, &instance, &card);
    let raw = raw_of(&kit, &handle, &instance);
    for field in ["fillPaints", "strokePaints", "strokeWeight", "strokeAlign", "dashPattern"] {
        assert!(entry.get(field).is_some(), "{field} in the entry");
        assert_eq!(entry.get(field), raw.get(field), "{field} on the instance too");
    }
    // Set again, one entry, merged; no sentinel without a style.
    let overrides = raw.get("symbolData").and_then(|s| s.get("symbolOverrides")).and_then(Raw::as_array).unwrap();
    assert_eq!(overrides.len(), 1);
    assert!(entry.get("styleIdForFill").is_none());

    let node = read(&mut kit, &handle, &instance);
    assert_eq!(node["fills"][0]["color"], "#7C3AED");
    assert_eq!(node["strokes"][0]["color"], "#DC2626");
    assert_eq!(node["dashPattern"], json!([4.0, 4.0]));
    assert!(node.get("overrides").is_none(), "the own entry is shown as the instance's values: {node}");

    // [] clears the component's dashes in the entry.
    set(&mut kit, &handle, &instance, json!({ "dashPattern": [] })).unwrap();
    assert_eq!(own_entry(&kit, &handle, &instance, &card).get("dashPattern"), Some(&Raw::Array(Vec::new())));
}

#[test]
fn an_instance_of_a_styled_component_carries_the_no_style_sentinel() {
    let (mut kit, handle) = kit();
    let badge = card_component(&mut kit, &handle, true);
    let instance = card_instance(&mut kit, &handle, &badge);
    set(&mut kit, &handle, &instance, json!({ "fills": [{ "type": "SOLID", "color": "#16A34A" }] })).unwrap();
    let sentinel = |raw: &Raw| raw.get("styleIdForFill").and_then(|p| p.get("guid")).and_then(|g| g.get("sessionID")).and_then(Raw::as_u32);
    assert_eq!(sentinel(&own_entry(&kit, &handle, &instance, &badge)), Some(u32::MAX));
    assert_eq!(sentinel(&raw_of(&kit, &handle, &instance)), Some(u32::MAX));
    assert!(own_entry(&kit, &handle, &instance, &badge).get("styleIdForStrokeFill").is_none());
}

#[test]
fn an_instance_with_its_own_fill_is_drawn_with_its_components_corners() {
    let (mut kit, handle) = kit();
    let card = card_component(&mut kit, &handle, false);
    let instance = card_instance(&mut kit, &handle, &card);
    set(&mut kit, &handle, &instance, json!({ "fills": [{ "type": "SOLID", "color": "#7C3AED" }] })).unwrap();
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": instance })).unwrap();
    assert_eq!(pixel(&png, 100, 60), [0x7C, 0x3A, 0xED, 255], "its own fill");
    assert_eq!(pixel(&png, 0, 0)[3], 0, "the component's radius leaves the corner empty");
    let edge = pixel(&png, 100, 0);
    assert!(edge[0] < 0x7C + 40 && edge[2] > 0x90, "the component's grey stroke: {edge:?}");
}

#[test]
fn dashes_are_written_on_frames_shapes_and_vectors_and_drawn() {
    let (mut kit, handle) = kit();
    let area = frame_at(&mut kit, &handle, "1:1", "Upload", (0.0, 0.0, 200.0, 100.0));
    set(&mut kit, &handle, &area, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }], "strokes": [{ "type": "SOLID", "color": "#000000" }], "strokeWeight": 2, "strokeAlign": "INSIDE", "dashPattern": [10, 10] })).unwrap();
    assert_eq!(read(&mut kit, &handle, &area)["dashPattern"], json!([10.0, 10.0]));
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": area })).unwrap();
    assert_eq!(pixel(&png, 5, 1)[0], 0, "a dash");
    assert_eq!(pixel(&png, 15, 1)[0], 255, "a gap");

    let rect = shape(&mut kit, &handle, json!({ "type": "RECTANGLE", "width": 40, "height": 40, "strokes": [{ "type": "SOLID", "color": "#000000" }] })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &rect, json!({ "dashPattern": [2, 2] })).unwrap();
    assert_eq!(read(&mut kit, &handle, &rect)["dashPattern"], json!([2.0, 2.0]));
    set(&mut kit, &handle, &rect, json!({ "dashPattern": [] })).unwrap();
    assert!(raw_of(&kit, &handle, &rect).get("dashPattern").is_none(), "[] is solid");
    let vector = call(&mut kit, "create_vector", json!({ "handle": handle, "parent": "1:1", "path": "M0 0L10 10", "viewBox": [0, 0, 10, 10], "strokes": [{ "type": "SOLID", "color": "#000000" }] })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &vector, json!({ "dashPattern": [1, 2, 3, 4] })).unwrap();
    assert_eq!(read(&mut kit, &handle, &vector)["dashPattern"], json!([1.0, 2.0, 3.0, 4.0]));
}

#[test]
fn every_paint_and_dash_refusal_names_its_reason_and_changes_nothing() {
    let (mut kit, handle) = kit();
    let area = frame_at(&mut kit, &handle, "1:1", "Area", (0.0, 0.0, 50.0, 50.0));
    // The document's own instance is of a component in the file; one of a
    // component it does not have cannot keep values of its own.
    {
        let session = kit.session_mut(&handle).unwrap();
        let at = session.raw_index[&node_id("1:5")];
        let Raw::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(Raw::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let Some(Raw::Object(instance)) = changes.get_mut(at) else { unreachable!() };
        instance.insert("symbolData", obj(vec![("symbolID", guid(9, 9))]));
        session.refresh(&[node_id("1:5")]);
    }
    let before = kit.session(&handle).unwrap().fig.node_changes.clone();
    for (id, properties, reason) in [
        (area.as_str(), json!({ "dashPattern": [4] }), "even count"),
        (area.as_str(), json!({ "dashPattern": [4, -1] }), "0 or more"),
        (area.as_str(), json!({ "dashPattern": "dashed" }), "dash then gap"),
        ("1:5", json!({ "fills": [{ "type": "SOLID", "color": "#FF0000" }] }), "not in this file"),
        ("1:5", json!({ "cornerRadius": 4 }), "not accepted on an instance"),
    ] {
        let error = set(&mut kit, &handle, id, properties.clone()).unwrap_err();
        assert!(error.to_string().contains(reason), "{properties}: {error}");
    }
    assert_eq!(kit.session(&handle).unwrap().fig.node_changes, before);
}

// ── modes in the picture ───────────────────────────────────────────────

/// Theme (Light, Dark) with a surface colour and a radius, and a card
/// component bound to both. Returns the card.
fn themed_card(kit: &mut Toolkit, handle: &str) -> String {
    collection(kit, handle, "Theme", &["Light", "Dark"]).unwrap();
    new_variable(kit, handle, "Theme", "Surface", "COLOR", json!({ "Light": "#FFFFFF", "Dark": "#1E1B2E" })).unwrap();
    new_variable(kit, handle, "Theme", "Radius", "FLOAT", json!({ "Light": 0, "Dark": 20 })).unwrap();
    new_variable(kit, handle, "Theme", "Wide", "FLOAT", json!({ "Light": 100, "Dark": 140 })).unwrap();
    let card = frame_at(kit, handle, "1:1", "Card", (0.0, 0.0, 100.0, 60.0));
    set(kit, handle, &card, json!({ "fills": [{ "type": "SOLID", "variable": "Surface" }], "cornerRadius": { "variable": "Radius" } })).unwrap();
    call(kit, "create_component", json!({ "handle": handle, "id": card })).unwrap();
    card
}

/// A frame set to `mode` (or none), filled red so the card stands out, with
/// an instance of `card` at 20, 20.
fn screen_with(kit: &mut Toolkit, handle: &str, card: &str, mode: Option<&str>) -> (String, String) {
    let screen = frame_at(kit, handle, "1:1", "Screen", (300.0, 0.0, 140.0, 100.0));
    set(kit, handle, &screen, json!({ "fills": [{ "type": "SOLID", "color": "#FF0000" }] })).unwrap();
    let instance = call(kit, "create_instance", json!({ "handle": handle, "parent": screen, "component": card, "x": 20, "y": 20 })).unwrap()["id"].as_str().unwrap().to_string();
    if let Some(mode) = mode {
        set(kit, handle, &screen, json!({ "explicitVariableModes": { "Theme": mode } })).unwrap();
    }
    (screen, instance)
}

const DARK: [u8; 4] = [0x1E, 0x1B, 0x2E, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

#[test]
fn an_instance_in_a_dark_frame_is_drawn_dark_and_in_a_light_one_light() {
    let (mut kit, handle) = kit();
    let card = themed_card(&mut kit, &handle);
    let (dark, _) = screen_with(&mut kit, &handle, &card, Some("Dark"));
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": dark })).unwrap();
    assert_eq!(pixel(&png, 70, 50), DARK, "the card's surface, in Dark");
    assert_eq!(pixel(&png, 21, 21), [255, 0, 0, 255], "Dark's radius 20 leaves the card's corner showing the screen");

    let (light, _) = screen_with(&mut kit, &handle, &card, Some("Light"));
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": light })).unwrap();
    assert_eq!(pixel(&png, 70, 50), WHITE);
    assert_eq!(pixel(&png, 21, 21), WHITE, "Light's radius is 0");
}

#[test]
fn the_nearest_mode_wins_an_instances_own_then_its_components_then_the_frames() {
    let (mut kit, handle) = kit();
    let card = themed_card(&mut kit, &handle);
    // The instance's own Light, in a Dark frame.
    let (dark, instance) = screen_with(&mut kit, &handle, &card, Some("Dark"));
    set(&mut kit, &handle, &instance, json!({ "explicitVariableModes": { "Theme": "Light" } })).unwrap_err();
    // An instance takes no mode through set_properties, so it is
    // written as Figma's own instances store it.
    {
        let modes = raw_of(&kit, &handle, &dark).get("variableModeBySetMap").cloned().unwrap();
        let light = kit.session(&handle).unwrap().graph.collections.values().find(|c| c.name == "Theme").unwrap().modes[0].id.declared().unwrap();
        let mut modes = modes;
        if let Raw::Object(m) = &mut modes {
            if let Some(Raw::Array(entries)) = m.get_mut("entries") {
                if let Raw::Object(e) = &mut entries[0] {
                    e.insert("variableModeID", light.to_value());
                }
            }
        }
        let session = kit.session_mut(&handle).unwrap();
        let at = session.raw_index[&node_id(&instance)];
        let Raw::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(Raw::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let Some(Raw::Object(fields)) = changes.get_mut(at) else { unreachable!() };
        fields.insert("variableModeBySetMap", modes);
        session.refresh(&[node_id(&instance)]);
    }
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": dark })).unwrap();
    assert_eq!(pixel(&png, 70, 50), WHITE, "the instance's own Light wins over the frame's Dark");

    // A component set to Dark, in a frame set to Light.
    let (mut kit, handle) = self::kit();
    let card = themed_card(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "explicitVariableModes": { "Theme": "Dark" } })).unwrap();
    let (light, _) = screen_with(&mut kit, &handle, &card, Some("Light"));
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": light })).unwrap();
    assert_eq!(pixel(&png, 70, 50), DARK, "the component's own Dark is nearer than the frame's Light");
}

#[test]
fn an_instance_rendered_alone_inside_a_dark_frame_is_dark() {
    let (mut kit, handle) = kit();
    let card = themed_card(&mut kit, &handle);
    let (_, instance) = screen_with(&mut kit, &handle, &card, Some("Dark"));
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": instance })).unwrap();
    assert_eq!(pixel(&png, 50, 30), DARK);
}

#[test]
fn a_bound_size_drawn_as_stored_is_named() {
    let (mut kit, handle) = kit();
    let card = themed_card(&mut kit, &handle);
    set(&mut kit, &handle, &card, json!({ "width": { "variable": "Wide" } })).unwrap();
    let (dark, _) = screen_with(&mut kit, &handle, &card, Some("Dark"));
    let (_, caption) = render(&mut kit, json!({ "handle": handle, "id": dark })).unwrap();
    let notes = caption["notes"].to_string();
    assert!(notes.contains("width bound to a variable is drawn as the component stores it, 100"), "{notes}");
}

// ── rich text in the picture ───────────────────────────────────────────

/// The columns and rows where a picture has ink matching `is`, as
/// (first x, last x, first y, last y) within the rows `rows`.
fn ink(png: &[u8], rows: std::ops::Range<u32>, is: impl Fn([u8; 4]) -> bool) -> Option<(u32, u32, u32, u32)> {
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).unwrap();
    let mut found: Option<(u32, u32, u32, u32)> = None;
    for y in rows.start..rows.end.min(pixmap.height()) {
        for x in 0..pixmap.width() {
            let p = pixmap.pixel(x, y).unwrap().demultiply();
            if p.alpha() > 128 && is([p.red(), p.green(), p.blue(), p.alpha()]) {
                found = Some(found.map_or((x, x, y, y), |(a, b, c, d)| (a.min(x), b.max(x), c.min(y), d.max(y))));
            }
        }
    }
    found
}

fn red(p: [u8; 4]) -> bool {
    p[0] > 180 && p[1] < 90 && p[2] < 90
}

fn dark(p: [u8; 4]) -> bool {
    p[0] < 90 && p[1] < 90 && p[2] < 90
}

/// A text on a white frame, rendered.
fn rendered_text(kit: &mut Toolkit, handle: &str, args: Value) -> Vec<u8> {
    let board = frame_at(kit, handle, "1:1", "Board", (0.0, 0.0, 400.0, 300.0));
    set(kit, handle, &board, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    let mut args = args;
    args["parent"] = json!(board);
    args["fontFamily"] = json!("Inter");
    args["fontStyle"] = json!("Regular");
    text(kit, handle, args).unwrap();
    render(kit, json!({ "handle": handle, "id": board })).unwrap().0
}

#[test]
fn each_run_is_drawn_in_its_own_colour_and_a_larger_run_makes_its_line_taller() {
    let (mut kit, handle) = kit();
    let png = rendered_text(&mut kit, &handle, json!({ "fontSize": 20, "lineHeight": { "value": 100, "unit": "PERCENT" },
        "runs": [{ "characters": "Total " }, { "characters": "99", "fills": [{ "type": "SOLID", "color": "#FF0000" }] }] }));
    let (red_from, ..) = ink(&png, 0..60, red).expect("the red run is drawn");
    let (_, dark_to, ..) = ink(&png, 0..60, dark).expect("the layer's run is drawn");
    assert!(dark_to < red_from, "the red run follows the black one: {dark_to} then {red_from}");

    // The second paragraph sits lower when the first line holds a 40 px run.
    let second = |kit: &mut Toolkit, first: Value| {
        let png = rendered_text(kit, &handle, json!({ "fontSize": 20, "lineHeight": { "value": 100, "unit": "PERCENT" },
            "runs": [first, { "characters": "\nNext", "fills": [{ "type": "SOLID", "color": "#FF0000" }] }] }));
        ink(&png, 0..200, red).unwrap().2
    };
    let small = second(&mut kit, json!({ "characters": "Title" }));
    let large = second(&mut kit, json!({ "characters": "Title", "fontSize": 40 }));
    assert_eq!(large - small, 20, "a 40 px line is 20 px taller than a 20 px one");
}

#[test]
fn list_markers_and_text_sit_where_figma_puts_them() {
    let (mut kit, handle) = kit();
    // 20 px text: a level's slot is 1.5 em, 30 px.
    let png = rendered_text(&mut kit, &handle, json!({ "fontSize": 20, "lineHeight": 30, "characters": "One\nTwo",
        "paragraphs": [{ "listType": "UNORDERED" }, { "listType": "UNORDERED", "indentation": 2 }] }));
    let row = |png: &[u8], rows: std::ops::Range<u32>, from: u32, to: u32| {
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(png).unwrap();
        (from..to).filter(|&x| rows.clone().any(|y| { let p = pixmap.pixel(x, y).unwrap().demultiply(); p.alpha() > 128 && p.red() < 90 })).collect::<Vec<u32>>()
    };
    let first = row(&png, 0..30, 0, 400);
    let bullet: Vec<u32> = first.iter().copied().filter(|&x| x < 30).collect();
    let text: Vec<u32> = first.iter().copied().filter(|&x| x >= 30).collect();
    assert!(!bullet.is_empty() && bullet[0] >= 8 && *bullet.last().unwrap() <= 22, "a bullet centred in 0–30: {bullet:?}");
    assert!(text[0] >= 30 && text[0] <= 34, "the text 1.5 em in: {}", text[0]);
    let second = row(&png, 30..60, 0, 400);
    assert!(second.iter().all(|&x| x >= 30), "nothing in the first slot at level 2: {:?}", &second[..3.min(second.len())]);
    assert!(second.iter().any(|&x| x < 60) && second.iter().any(|&x| (60..64).contains(&x)), "a bullet in 30–60, the text 3 em in");

    // Numbers end 0.33 em before the text.
    let (mut kit, handle) = self::kit();
    let png = rendered_text(&mut kit, &handle, json!({ "fontSize": 20, "lineHeight": 30, "characters": "One\nTwo",
        "paragraphs": [{ "listType": "ORDERED" }, { "listType": "ORDERED" }] }));
    let first = row(&png, 0..30, 0, 400);
    let number: Vec<u32> = first.iter().copied().filter(|&x| x < 30).collect();
    assert!(!number.is_empty() && *number.last().unwrap() <= 24, "\"1.\" ends before 30 − 6.6: {number:?}");
}

#[test]
fn paragraph_spacing_moves_the_next_paragraph_down() {
    let (mut kit, handle) = kit();
    let top = |kit: &mut Toolkit, spacing: f64| {
        let png = rendered_text(kit, &handle, json!({ "fontSize": 20, "lineHeight": 30, "paragraphSpacing": spacing,
            "runs": [{ "characters": "First\n" }, { "characters": "Second", "fills": [{ "type": "SOLID", "color": "#FF0000" }] }] }));
        ink(&png, 0..200, red).unwrap().2
    };
    let tight = top(&mut kit, 0.0);
    let spaced = top(&mut kit, 16.0);
    assert_eq!(spaced - tight, 16);
}

// ── text measured when estimated ───────────────────────────────────────

/// The distances from a button's edges to its label's ink, left and right.
fn ink_gaps(kit: &mut Toolkit, handle: &str, button: &str) -> (u32, u32) {
    let (png, _) = render(kit, json!({ "handle": handle, "id": button })).unwrap();
    let width = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap().width();
    let white = |p: [u8; 4]| p[0] > 200 && p[1] > 200 && p[2] > 200;
    let (from, to, ..) = ink(&png, 0..200, white).expect("the label's ink");
    (from, width - 1 - to)
}

#[test]
fn a_hugging_buttons_label_is_measured_and_renders_centred() {
    let (mut kit, handle) = kit();
    let (button, label) = hugging_button(&mut kit, &handle);
    set(&mut kit, &handle, &button, json!({ "fills": [{ "type": "SOLID", "color": "#4F46E5" }] })).unwrap();
    set(&mut kit, &handle, &label, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    for change in [json!({ "characters": "Continue to payment" }), json!({ "fontSize": 20, "fontStyle": "Bold" })] {
        set(&mut kit, &handle, &label, change.clone()).unwrap();
        let (left, right) = ink_gaps(&mut kit, &handle, &button);
        assert!(left.abs_diff(right) <= 2, "{change}: ink {left} px from the left, {right} from the right");
    }
}

#[test]
fn a_wrapping_texts_height_is_its_rendered_lines() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "fontFamily": "Inter", "fontStyle": "Regular", "fontSize": 16, "lineHeight": 20,
        "width": 120, "characters": "Two items, delivered tomorrow, with free returns" })).unwrap();
    let height = out["size"]["height"].as_f64().unwrap();
    assert_eq!(height % 20.0, 0.0, "whole lines: {height}");
    let id = out["id"].as_str().unwrap().to_string();
    let (png, _) = render(&mut kit, json!({ "handle": handle, "id": id })).unwrap();
    let (.., last) = ink(&png, 0..400, dark).unwrap();
    assert!(f64::from(last) < height && f64::from(last) > height - 20.0, "the last line's ink, {last}, is in the box's last line, {height}");
}

#[test]
fn a_font_not_installed_is_measured_in_its_stand_in_and_named() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "fontFamily": "No Such Font", "characters": "Hello" })).unwrap();
    assert!(out["measured_in_stand_in"].as_str().is_some_and(|n| n.contains("No Such Font in")), "{out}");
}

// ── instance names that stick ──────────────────────────────────────────

#[test]
fn a_named_instance_carries_its_name_in_the_entry_addressed_to_its_component() {
    let (mut kit, handle) = kit();
    let card = card_component(&mut kit, &handle, false);
    let named = call(&mut kit, "create_instance", json!({ "handle": handle, "parent": "1:1", "component": card, "name": "Plan · Free" })).unwrap()["id"].as_str().unwrap().to_string();
    assert_eq!(own_entry(&kit, &handle, &named, &card).get("name").and_then(Raw::as_str), Some("Plan · Free"));
    assert_eq!(read(&mut kit, &handle, &named)["name"], "Plan · Free");

    // Its component's name, or none, writes no entry.
    for args in [json!({ "name": "Card" }), json!({})] {
        let mut args = args;
        args["handle"] = json!(handle);
        args["parent"] = json!("1:1");
        args["component"] = json!(card);
        let plain = call(&mut kit, "create_instance", args).unwrap()["id"].as_str().unwrap().to_string();
        assert!(raw_of(&kit, &handle, &plain).get("symbolData").and_then(|s| s.get("symbolOverrides")).is_none());
    }

    // Renamed beside its own fill, in one entry.
    let instance = card_instance(&mut kit, &handle, &card);
    set(&mut kit, &handle, &instance, json!({ "fills": [{ "type": "SOLID", "color": "#7C3AED" }] })).unwrap();
    set(&mut kit, &handle, &instance, json!({ "name": "Plan · Pro" })).unwrap();
    let entry = own_entry(&kit, &handle, &instance, &card);
    assert_eq!(entry.get("name").and_then(Raw::as_str), Some("Plan · Pro"));
    assert!(entry.get("fillPaints").is_some(), "merged with its own fill");
    assert_eq!(raw_of(&kit, &handle, &instance).get("symbolData").and_then(|s| s.get("symbolOverrides")).and_then(Raw::as_array).map(Vec::len), Some(1));
    assert_eq!(read(&mut kit, &handle, &instance)["name"], "Plan · Pro");
}

#[test]
fn an_instance_of_a_component_from_another_file_is_named_on_itself_and_says_so() {
    let (mut kit, handle) = kit();
    {
        let session = kit.session_mut(&handle).unwrap();
        let at = session.raw_index[&node_id("1:5")];
        let Raw::Object(message) = &mut session.fig.node_changes else { unreachable!() };
        let Some(Raw::Array(changes)) = message.get_mut("nodeChanges") else { unreachable!() };
        let Some(Raw::Object(instance)) = changes.get_mut(at) else { unreachable!() };
        instance.insert("symbolData", obj(vec![("symbolID", guid(9, 9))]));
        session.refresh(&[node_id("1:5")]);
    }
    let out = set(&mut kit, &handle, "1:5", json!({ "name": "Elsewhere" })).unwrap();
    assert!(out["name_kept"].as_str().is_some_and(|n| n.contains("not in this file")), "{out}");
    assert_eq!(read(&mut kit, &handle, "1:5")["name"], "Elsewhere");
}

// ── font faces that exist ───────────────────────────────────────────────

/// A family installed on this machine, with its faces; `None` on a machine
/// with no fonts, where there is nothing to check against.
fn an_installed_family() -> Option<(String, Vec<String>)> {
    let db = render::text::fonts();
    let found = db.faces().find_map(|face| {
        let family = face.families.first()?.0.clone();
        let faces = render::text::faces(&db, &family)?;
        (!faces.is_empty()).then_some((family, faces))
    });
    found
}

fn missing_faces(out: &Value) -> Vec<(String, String)> {
    out["font_faces"]["missing"]
        .as_array()
        .map(|missing| missing.iter().map(|m| (m["family"].as_str().unwrap().to_string(), m["style"].as_str().unwrap().to_string())).collect())
        .unwrap_or_default()
}

#[test]
fn a_style_no_installed_face_has_is_said_with_the_faces_there_are() {
    let Some((family, faces)) = an_installed_family() else { return };
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "A", "fontFamily": family, "fontStyle": "No Such Face" })).unwrap();
    assert_eq!(missing_faces(&out), [(family.clone(), "No Such Face".to_string())], "{out}");
    assert_eq!(out["font_faces"]["missing"][0]["installed_here"], json!(faces));
    assert!(out["font_faces"]["why"].as_str().unwrap().contains("this machine"));
    assert!(out["id"].is_string(), "the text is still made");

    // A face, spaces and case aside, says nothing.
    let spaced = faces[0].to_uppercase().replace("", " ").trim().to_string();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "B", "fontFamily": family, "fontStyle": spaced })).unwrap();
    assert!(out.get("font_faces").is_none(), "{out}");
}

#[test]
fn a_family_not_installed_is_not_judged() {
    let (mut kit, handle) = kit();
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "A", "fontFamily": "No Such Family 044", "fontStyle": "Bold" })).unwrap();
    assert!(out.get("font_faces").is_none(), "{out}");
}

#[test]
fn a_run_a_changed_style_and_a_text_style_are_checked_too() {
    let Some((family, faces)) = an_installed_family() else { return };
    let (mut kit, handle) = kit();
    let out = text(
        &mut kit,
        &handle,
        json!({ "parent": "1:1", "fontFamily": family, "fontStyle": faces[0], "runs": [{ "characters": "ok " }, { "characters": "no", "fontStyle": "Run Face 044" }] }),
    )
    .unwrap();
    assert_eq!(missing_faces(&out), [(family.clone(), "Run Face 044".to_string())], "{out}");

    let id = out["id"].as_str().unwrap().to_string();
    let out = set(&mut kit, &handle, &id, json!({ "characters": "plain", "fontStyle": "Set Face 044" })).unwrap();
    assert_eq!(missing_faces(&out), [(family.clone(), "Set Face 044".to_string())], "{out}");

    let out = call(&mut kit, "create_style", json!({ "handle": handle, "name": "H 044", "type": "TEXT", "fontFamily": family, "fontStyle": "Style Face 044", "fontSize": 20 })).unwrap();
    assert_eq!(missing_faces(&out), [(family.clone(), "Style Face 044".to_string())], "{out}");
    let out = call(&mut kit, "set_style", json!({ "handle": handle, "id": "H 044", "fontStyle": faces[0] })).unwrap();
    assert!(out.get("font_faces").is_none(), "{out}");
}

/// Over every text in the fixtures whose family is installed here, the styles
/// Figma itself wrote are faces: a rule that misread Figma's names (its
/// `Semi Bold` for a file's `SemiBold`) would report thousands. The census of
/// 2026-10-24 found 2 of 29,307 on the maintainer's Mac, where this machine's
/// Helvetica has an Oblique and the designer's had an Italic.
#[test]
fn figmas_own_styles_are_faces_wherever_their_family_is_installed() {
    let db = render::text::fonts();
    let mut faces_of = std::collections::HashMap::new();
    let (mut checked, mut reported) = (0usize, 0usize);
    for path in testkit::fixture::all() {
        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let Some((_, graph)) = testkit::fixture::graph(&name) else { continue };
        for node in graph.nodes.values() {
            let Some(style) = &node.text_style else { continue };
            let faces = faces_of.entry(style.font_family.clone()).or_insert_with(|| render::text::faces(&db, &style.font_family));
            let Some(faces) = faces else { continue };
            checked += 1;
            if !render::text::is_face(faces, &style.font_style) {
                reported += 1;
            }
        }
    }
    eprintln!("044: {reported} of {checked} installed-family texts reported");
    assert!(reported * 100 <= checked, "{reported} of {checked} of Figma's own styles read as missing faces");
}

#[test]
fn a_face_the_file_uses_is_written_as_the_file_spells_it() {
    let Some((family, faces)) = an_installed_family() else { return };
    let (mut kit, handle) = kit();
    let figmas = format!("{} 044", faces[0]); // as if Figma spelt a face its own way
    text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "A", "fontFamily": family, "fontStyle": figmas })).unwrap();

    let asked = figmas.to_lowercase().replace(' ', "");
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "B", "fontFamily": family, "fontStyle": asked })).unwrap();
    assert_eq!(out["font"]["style"], json!(figmas), "{out}");
    assert_eq!(out["font_respelled"]["asked"], json!(asked));
    let id = out["id"].as_str().unwrap().to_string();
    assert_eq!(raw_of(&kit, &handle, &id).get("fontName").and_then(|f| f.get("style")).and_then(Raw::as_str), Some(figmas.as_str()));

    let other = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "C", "fontFamily": family, "fontStyle": faces[0] })).unwrap();
    let id = other["id"].as_str().unwrap().to_string();
    let out = set(&mut kit, &handle, &id, json!({ "fontStyle": asked })).unwrap();
    assert!(out["notes"].to_string().contains("written as"), "{out}");

    // A style missing everywhere lists the file's spellings first.
    let out = text(&mut kit, &handle, json!({ "parent": "1:1", "characters": "D", "fontFamily": family, "fontStyle": "Nowhere 044" })).unwrap();
    assert!(out["font_faces"]["missing"][0]["in_this_file"].as_array().unwrap().contains(&json!(figmas)), "{out}");
}

// ── PDF export ──────────────────────────────────────────────────────────

#[test]
fn frames_export_to_a_new_pdf_a_page_each_and_never_over_a_file() {
    let (mut kit, handle) = kit();
    let a = created_frame(&mut kit, &handle);
    let b = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "B", "width": 40, "height": 30 })).unwrap()["id"].as_str().unwrap().to_string();
    let path = "/memory/045.pdf";

    let out = call(&mut kit, "export_pdf", json!({ "handle": handle, "ids": [a, b], "path": path })).unwrap();
    assert_eq!(out["pages"].as_array().unwrap().len(), 2, "{out}");
    assert_eq!(out["pages"][1]["id"], json!(b));
    assert_eq!(out["pages"][1]["width"], json!(40));
    let bytes = stored(&kit, path).unwrap();
    assert_eq!(out["bytes"], json!(bytes.len()));
    assert!(bytes.starts_with(b"%PDF-"));

    let error = call(&mut kit, "export_pdf", json!({ "handle": handle, "ids": [a], "path": path })).unwrap_err();
    assert!(matches!(error, ToolError::Refused(_)) && error.to_string().contains("never overwrites"), "{error}");
    assert_eq!(stored(&kit, path).unwrap(), bytes, "the file is untouched");

    for (args, says) in [
        (json!({ "ids": [a], "path": "out.png" }), ".pdf"),
        (json!({ "ids": [], "path": "out.pdf" }), "at least one"),
        (json!({ "ids": ["9:9"], "path": "out-045.pdf" }), "9:9"),
    ] {
        let mut args = args;
        args["handle"] = json!(handle);
        let error = call(&mut kit, "export_pdf", args).unwrap_err();
        assert!(error.to_string().contains(says), "{error}");
    }
}

// ── contrast ────────────────────────────────────────────────────────────

#[test]
fn check_contrast_lists_what_fails_counts_the_rest_and_lists_all_on_request() {
    let (mut kit, handle) = kit();
    let frame = created_frame(&mut kit, &handle);
    set(&mut kit, &handle, &frame, json!({ "fills": [{ "type": "SOLID", "color": "#FFFFFF" }] })).unwrap();
    for (characters, colour) in [("faint", "#AAAAAA"), ("clear", "#111111")] {
        text(&mut kit, &handle, json!({ "parent": frame, "characters": characters, "fontSize": 14, "fills": [{ "type": "SOLID", "color": colour }] })).unwrap();
    }
    let out = call(&mut kit, "check_contrast", json!({ "handle": handle, "id": frame })).unwrap();
    assert_eq!((out["checked"].clone(), out["passed"].clone(), out["failed"].clone(), out["unmeasured"].clone()), (json!(2), json!(1), json!(1), json!(0)), "{out}");
    let listed = out["results"].as_array().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["characters"], "faint");
    assert_eq!(listed[0]["colour"], "#AAAAAA");
    assert_eq!(listed[0]["background"], "#FFFFFF");
    assert_eq!(listed[0]["ratio"], json!(2.32));
    assert_eq!(listed[0]["required"], json!(4.5));

    let out = call(&mut kit, "check_contrast", json!({ "handle": handle, "id": frame, "all": true })).unwrap();
    assert_eq!(out["results"].as_array().unwrap().len(), 2, "{out}");
    assert_eq!(out["results"][1]["verdict"], "passes", "the worst first");
}

// ── templates filled from data ──────────────────────────────────────────

/// A row container and the button of 029 with its three properties.
fn template_kit() -> (Toolkit, String, String, [String; 6]) {
    let (mut kit, handle) = kit();
    let ids = with_properties(&mut kit, &handle);
    let list = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "List", "x": 400, "y": 0, "width": 200, "height": 10 })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &list, json!({ "layoutMode": "VERTICAL", "itemSpacing": 8, "layoutSizingVertical": "HUG" })).unwrap();
    (kit, handle, list, ids)
}

fn children_of(kit: &Toolkit, handle: &str, id: &str) -> Vec<String> {
    kit.session(handle).unwrap().graph.node(node_id(id)).unwrap().children.iter().map(ToString::to_string).collect()
}

#[test]
fn rows_place_one_instance_each_with_their_properties_and_names() {
    let (mut kit, handle, list, [_, _, button, ..]) = template_kit();
    let out = call(&mut kit, "fill_template", json!({ "handle": handle, "component": button, "parent": list, "rows": [
        { "name": "Save", "Label": "Guardar", "Show icon": false },
        { "name": "Send", "Label": "Enviar", "Icon": "Icon/Check" },
        { "Label": 42 },
    ] }))
    .unwrap();
    assert_eq!(out["created"], 3, "{out}");
    let instances: Vec<String> = out["instances"].as_array().unwrap().iter().map(|i| i.as_str().unwrap().to_string()).collect();
    assert_eq!(children_of(&kit, &handle, &list), instances, "in row order, in the parent");
    let first = read(&mut kit, &handle, &instances[0]);
    assert_eq!(first["name"], "Save");
    assert_eq!(first["componentProperties"]["Label"], "Guardar");
    assert_eq!(first["componentProperties"]["Show icon"], "false");
    assert_eq!(read(&mut kit, &handle, &instances[1])["componentProperties"]["Icon"], "Icon/Check");
    assert_eq!(read(&mut kit, &handle, &instances[2])["componentProperties"]["Label"], "42");
}

#[test]
fn a_csv_fills_what_the_same_rows_in_json_fill() {
    let (mut kit, handle, list, [_, _, button, ..]) = template_kit();
    let csv = put(&kit, "/memory/047.csv", "name,Label,Show icon\nSave,\"Guardar, ya\",false\n\"Two\nlines\",\"Say \"\"hi\"\"\",true\n");
    let out = call(&mut kit, "fill_template", json!({ "handle": handle, "component": button, "parent": list, "csv": csv })).unwrap();
    let ids: Vec<String> = out["instances"].as_array().unwrap().iter().map(|i| i.as_str().unwrap().to_string()).collect();
    let first = read(&mut kit, &handle, &ids[0]);
    assert_eq!((first["name"].clone(), first["componentProperties"]["Label"].clone(), first["componentProperties"]["Show icon"].clone()), (json!("Save"), json!("Guardar, ya"), json!("false")));
    let second = read(&mut kit, &handle, &ids[1]);
    assert_eq!((second["name"].clone(), second["componentProperties"]["Label"].clone()), (json!("Two\nlines"), json!("Say \"hi\"")));
}

#[test]
fn a_variant_property_chooses_the_variant() {
    let (mut kit, handle) = kit();
    let (set, exact, substitute) = badge_set(&mut kit, &handle);
    let out = call(&mut kit, "fill_template", json!({ "handle": handle, "component": set, "parent": "1:1", "rows": [{ "Estado": "Sustituto" }, {}] })).unwrap();
    let ids: Vec<String> = out["instances"].as_array().unwrap().iter().map(|i| i.as_str().unwrap().to_string()).collect();
    let session = kit.session(&handle).unwrap();
    assert_eq!(session.graph.node(node_id(&ids[0])).unwrap().component_id, Some(node_id(&substitute)));
    assert_eq!(session.graph.node(node_id(&ids[1])).unwrap().component_id, Some(node_id(&exact)), "the first variant by default");
}

#[test]
fn a_layer_named_in_a_row_gets_its_text_or_its_image() {
    let (mut kit, handle) = kit();
    let card = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": "1:1", "name": "Card", "x": 0, "y": 0, "width": 200, "height": 120 })).unwrap()["id"].as_str().unwrap().to_string();
    let photo = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": card, "name": "Photo", "x": 0, "y": 0, "width": 200, "height": 80 })).unwrap()["id"].as_str().unwrap().to_string();
    let title = text(&mut kit, &handle, json!({ "parent": card, "characters": "Title", "x": 8, "y": 90 })).unwrap()["id"].as_str().unwrap().to_string();
    set(&mut kit, &handle, &title, json!({ "name": "Title" })).unwrap();
    call(&mut kit, "create_component", json!({ "handle": handle, "id": card })).unwrap();
    let png = put(&kit, "/memory/047.png", render::blank_png(4, 4, [1.0, 0.0, 0.0, 1.0]).unwrap());

    let out = call(&mut kit, "fill_template", json!({ "handle": handle, "component": card, "parent": "1:1", "rows": [{ "Title": "Lisboa", "Photo": png }] })).unwrap();
    let instance = out["instances"][0].as_str().unwrap().to_string();
    let overrides = raw_of(&kit, &handle, &instance).get("symbolData").and_then(|d| d.get("symbolOverrides")).and_then(Raw::as_array).cloned().unwrap_or_default();
    let has = |field: &str| overrides.iter().any(|o| o.get(field).is_some());
    assert!(has("textData"), "the title's text: {overrides:?}");
    assert!(has("fillPaints"), "the photo's image: {overrides:?}");
    let _ = photo;
}

#[test]
fn a_bad_row_refuses_the_whole_fill_before_anything_is_written() {
    let (mut kit, handle, list, [_, _, button, ..]) = template_kit();
    let edits = kit.session(&handle).unwrap().edits;
    let error = call(&mut kit, "fill_template", json!({ "handle": handle, "component": button, "parent": list, "rows": [
        { "Label": "fine" },
        { "Colour": "red" },
    ] }))
    .unwrap_err();
    let said = error.to_string();
    assert!(said.contains("row 1") && said.contains("Colour") && said.contains("Label") && said.contains("Show icon"), "{said}");
    assert!(children_of(&kit, &handle, &list).is_empty(), "nothing placed");
    assert_eq!(kit.session(&handle).unwrap().edits, edits);

    for (args, says) in [
        (json!({ "rows": [] }), "1 to 1000"),
        (json!({ "rows": [{ "Show icon": "maybe" }] }), "true or false"),
        (json!({}), "rows"),
    ] {
        let mut args = args;
        args["handle"] = json!(handle);
        args["component"] = json!(button);
        args["parent"] = json!(list);
        let error = call(&mut kit, "fill_template", args).unwrap_err();
        assert!(error.to_string().contains(says), "{error}");
    }
    let frame = created_frame(&mut kit, &handle);
    let error = call(&mut kit, "fill_template", json!({ "handle": handle, "component": frame, "parent": list, "rows": [{}] })).unwrap_err();
    assert!(error.to_string().contains("create_component"), "{error}");
}

#[test]
fn csv_reads_quotes_commas_line_breaks_and_leaves_empty_fields_out() {
    let rows = crate::tools::template::csv_rows("a,b\r\n1,\"x, \"\"y\"\"\"\n,z\n").unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["b"], crate::app::data::Data::from("x, \"y\""));
    assert!(!rows[1].contains_key("a"), "an empty field changes nothing");
    assert!(crate::tools::template::csv_rows("a,b\n1\n").unwrap_err().contains("line 2"));
}

// ── HTML into design layers ─────────────────────────────────────────────

/// A snapshot as scripts/snapshot.js writes one: a 300-wide column with 20
/// padding holding a heading and a row that spreads two labels.
fn snapshot(kit: &Toolkit, extra: Value) -> String {
    let text = |t: &str, x: f64, y: f64, w: f64| json!({ "tag": "#text", "text": t, "box": { "x": x, "y": y + 2.0, "width": w, "height": 16 },
        "lines": [{ "x": x, "y": y + 2.0, "width": w, "height": 16 }],
        "font": { "family": "Inter", "size": 14, "weight": 400, "italic": false, "lineHeight": 20, "letterSpacing": 0, "align": "start", "colour": "rgb(17, 17, 17)", "decoration": "none", "transform": "none" } });
    let mut root = json!({ "tag": "div", "name": "card", "box": { "x": 0, "y": 0, "width": 300, "height": 100 },
        "background": "rgb(255, 255, 255)", "radius": [12, 12, 12, 12], "border": { "width": 1, "colour": "rgb(230, 226, 216)" },
        "shadow": "rgba(0, 0, 0, 0.1) 0px 4px 12px 0px", "opacity": 1, "position": "static",
        "layout": { "direction": "column", "wrap": false, "gap": 8, "columnGap": 0, "justify": "normal", "align": "normal", "padding": [20, 20, 20, 20] },
        "unsupported": [], "children": [
            { "tag": "h2", "name": "title", "box": { "x": 21, "y": 21, "width": 258, "height": 20 }, "background": null, "radius": [0, 0, 0, 0], "border": null, "shadow": null, "opacity": 1, "position": "static", "layout": null, "unsupported": [], "children": [text("Hello", 21.0, 21.0, 40.0)] },
            { "tag": "div", "name": "row", "box": { "x": 21, "y": 49, "width": 258, "height": 20 }, "background": null, "radius": [0, 0, 0, 0], "border": null, "shadow": null, "opacity": 1, "position": "static",
              "layout": { "direction": "row", "wrap": false, "gap": 0, "columnGap": 0, "justify": "space-between", "align": "normal", "padding": [0, 0, 0, 0] },
              "unsupported": ["transform"], "children": [text("Left", 21.0, 49.0, 30.0), text("Right", 241.0, 49.0, 38.0)] }
        ] });
    if let Value::Object(extra) = extra {
        for (k, v) in extra {
            root[k] = v;
        }
    }
    let snapshot = json!({ "grafta": "layout-snapshot/1", "url": "file:///card.html", "nodes": 7, "root": root });
    put(kit, "/memory/048.json", snapshot.to_string())
}

#[test]
fn a_snapshot_becomes_frames_text_and_auto_layout_sized_as_measured() {
    let (mut kit, handle) = kit();
    let path = snapshot(&kit, json!({}));
    let out = call(&mut kit, "import_layout", json!({ "handle": handle, "parent": "1:1", "snapshot": path })).unwrap();
    assert_eq!((out["frames"].clone(), out["texts"].clone(), out["layout"].clone()), (json!(3), json!(3), json!("auto")), "{out}");
    assert_eq!(out["not_imported"], json!([{ "element": "card / row", "what": "transform" }]));
    let card = read(&mut kit, &handle, out["id"].as_str().unwrap());
    assert_eq!(card["name"], "card");
    assert_eq!(card["layoutMode"], "VERTICAL");
    assert_eq!(card["itemSpacing"], 8);
    assert_eq!(card["cornerRadius"], 12);
    assert_eq!(card["fills"][0]["color"], "#FFFFFF");
    assert_eq!(card["strokes"][0]["color"], "#E6E2D8");
    assert_eq!(card["effects"][0]["type"], "DROP_SHADOW");
    let children = children_of(&kit, &handle, out["id"].as_str().unwrap());
    let row = read(&mut kit, &handle, &children[1]);
    assert_eq!(row["layoutMode"], "HORIZONTAL");
    assert_eq!(row["primaryAxisAlignItems"], "SPACE_BETWEEN");
    assert_eq!(row["layoutSizingHorizontal"], "FILL", "it spans the card's content box, the border inside it: {row}");
}

#[test]
fn absolute_layout_places_every_box_where_the_browser_put_it() {
    let (mut kit, handle) = kit();
    let path = snapshot(&kit, json!({}));
    let out = call(&mut kit, "import_layout", json!({ "handle": handle, "parent": "1:1", "snapshot": path, "layout": "absolute", "x": 500, "y": 40 })).unwrap();
    let card = read(&mut kit, &handle, out["id"].as_str().unwrap());
    assert!(card.get("layoutMode").is_none_or(|m| m == "NONE"), "{card}");
    assert_eq!((card["x"].clone(), card["y"].clone()), (json!(500), json!(40)));
    let row_id = children_of(&kit, &handle, out["id"].as_str().unwrap())[1].clone();
    let row = read(&mut kit, &handle, &row_id);
    assert_eq!((row["x"].clone(), row["y"].clone()), (json!(21), json!(49)));
}

#[test]
fn a_data_image_is_a_fill_a_remote_one_is_listed_and_another_version_is_refused() {
    let (mut kit, handle) = kit();
    let png = render::blank_png(2, 2, [0.0, 0.5, 1.0, 1.0]).unwrap();
    let data = format!("data:image/png;base64,{}", render::base64(&png));
    let path = snapshot(&kit, json!({ "tag": "img", "image": data }));
    let out = call(&mut kit, "import_layout", json!({ "handle": handle, "parent": "1:1", "snapshot": path })).unwrap();
    assert_eq!(out["images"], 1, "{out}");
    let card = read(&mut kit, &handle, out["id"].as_str().unwrap());
    assert!(card["fills"].as_array().unwrap().iter().any(|f| f["type"] == "IMAGE"), "{card}");

    let path = snapshot(&kit, json!({ "tag": "img", "image": "https://example.com/a.png" }));
    let out = call(&mut kit, "import_layout", json!({ "handle": handle, "parent": "1:1", "snapshot": path })).unwrap();
    assert_eq!(out["images"], 0);
    assert!(out["not_imported"].to_string().contains("https://example.com/a.png"), "{out}");

    let other = put(&kit, "/memory/048-v0.json", r#"{ "grafta": "layout-snapshot/0", "root": {} }"#);
    let error = call(&mut kit, "import_layout", json!({ "handle": handle, "parent": "1:1", "snapshot": other })).unwrap_err();
    assert!(error.to_string().contains("layout-snapshot/1"), "{error}");
}

#[test]
fn css_colours_and_shadow_lists_read_as_chrome_writes_them() {
    use crate::tools::import::{rgba, shadows};
    assert_eq!(rgba("rgb(31, 157, 107)"), Some(("#1F9D6B".to_string(), 1.0)));
    assert_eq!(rgba("rgba(21, 23, 28, 0.06)"), Some(("#15171C".to_string(), 0.06)));
    let effects = shadows("rgba(0, 0, 0, 0.1) 0px 4px 12px 0px, rgb(255, 255, 255) 0px 1px 0px 0px inset");
    assert_eq!(effects.len(), 2);
    assert_eq!((effects[0]["type"].clone(), effects[0]["radius"].clone()), (crate::app::data::data!("DROP_SHADOW"), crate::app::data::data!(12.0)));
    assert_eq!(effects[1]["type"], "INNER_SHADOW");
}

// ── Figma Slides files ──────────────────────────────────────────────────

#[test]
fn a_design_file_is_saved_as_fig_and_a_new_file_is_never_a_deck() {
    let (mut kit, handle) = kit();
    let path = "/memory/049.deck";
    let error = call(&mut kit, "save_as", json!({ "handle": handle, "path": path })).unwrap_err();
    assert!(error.to_string().contains("Figma Design file, so it is saved as .fig"), "{error}");
    let error = call(&mut kit, "new_file", json!({ "path": path, "from": "/tmp/whatever.fig" })).unwrap_err();
    assert!(error.to_string().contains("a Figma Slides file from nothing is not made yet, so open one and save_as a copy"), "{error}");
    assert!(!error.to_string().contains("(0"), "no feature number: {error}");
    let board = call(&mut kit, "new_file", json!({ "path": "/memory/065.jam", "from": "/tmp/whatever.fig" })).unwrap_err();
    assert!(board.to_string().contains("a FigJam file from nothing"), "{board}");
}

/// The maintainer's deck, open; `None` where it is not.
fn deck() -> Option<(Toolkit, String)> {
    let path = testkit::fixture::dir().join("Figma Deck Test.deck");
    if !path.is_file() {
        return None;
    }
    let mut kit = crate::new();
    let handle = call(&mut kit, "open_file", json!({ "path": path })).unwrap()["handle"].as_str().unwrap().to_string();
    Some((kit, handle))
}

/// The deck's slides in its order: its row's children.
fn slides(kit: &mut Toolkit, handle: &str) -> Vec<String> {
    let found = call(kit, "search_nodes", json!({ "handle": handle, "type": "SLIDE_ROW" })).unwrap();
    let row = found["results"][0]["id"].as_str().unwrap().to_string();
    let graph = &kit.session(handle).unwrap().graph;
    graph.children(node_id(&row)).filter(|n| n.node_type == model::NodeType::Slide).map(|n| n.id.to_string()).collect()
}

/// Whether each slide links to the next, and the last to none, with no
/// interaction id twice.
fn chain_follows_order(kit: &mut Toolkit, handle: &str) -> bool {
    let order = slides(kit, handle);
    let mut ids = std::collections::HashSet::new();
    let mut next = Vec::new();
    for slide in &order {
        let raw = call(kit, "read_raw", json!({ "handle": handle, "id": slide })).unwrap();
        let links: Vec<String> = raw["prototypeInteractions"].as_array().into_iter().flatten().flat_map(|p| {
            assert!(ids.insert(p["id"].to_string()), "interaction {} twice", p["id"]);
            p["actions"].as_array().unwrap().iter().filter(|a| a["navigationType"] == "NAVIGATE").map(|a| format!("{}:{}", a["transitionNodeID"]["sessionID"], a["transitionNodeID"]["localID"])).collect::<Vec<_>>()
        }).collect();
        next.push(links);
    }
    (0..order.len()).all(|i| next[i] == order.get(i + 1).cloned().into_iter().collect::<Vec<_>>())
}

#[test]
fn a_deck_opens_as_slides_draws_and_round_trips() {
    let Some((mut kit, handle)) = deck() else { return };
    let summary = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert_eq!((summary["product"].clone(), summary["slides"].clone()), (json!("slides"), json!(18)), "{summary}");
    let order = slides(&mut kit, &handle);
    assert_eq!(read(&mut kit, &handle, &order[0])["type"], "SLIDE");
    let picture = call(&mut kit, "render_node", json!({ "handle": handle, "id": order[0], "scale": 0.25 })).unwrap();
    assert_eq!((picture["caption"]["width"].clone(), picture["caption"]["height"].clone()), (json!(480), json!(270)));
    assert!(chain_follows_order(&mut kit, &handle), "as Figma Slides saved it");
    let fig = std::env::temp_dir().join(format!("grafta-049-{}.fig", std::process::id()));
    assert!(call(&mut kit, "save_as", json!({ "handle": handle, "path": fig })).unwrap_err().to_string().contains("saved as .deck"));
    let out = std::env::temp_dir().join(format!("grafta-049-{}.deck", std::process::id()));
    std::fs::remove_file(&out).ok();
    let saved = call(&mut kit, "save_as", json!({ "handle": handle, "path": out })).unwrap();
    assert_eq!(saved["reread"], "compared equal", "{saved}");
    let bytes = std::fs::read(&out).unwrap();
    std::fs::remove_file(&out).ok();
    assert_eq!(figfile::parse_fig(&bytes).unwrap().product, figfile::Product::Slides);
}

#[test]
fn slides_duplicated_moved_and_deleted_keep_the_decks_next_links_in_order() {
    let Some((mut kit, handle)) = deck() else { return };
    let order = slides(&mut kit, &handle);
    let copy = call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": order[0] })).unwrap()["id"].as_str().unwrap().to_string();
    assert_eq!(slides(&mut kit, &handle)[1], copy, "the copy is the next slide");
    assert!(chain_follows_order(&mut kit, &handle));

    call(&mut kit, "move_node", json!({ "handle": handle, "id": order[17], "behind": order[0] })).unwrap();
    assert_eq!(slides(&mut kit, &handle)[0], order[17]);
    assert!(chain_follows_order(&mut kit, &handle));

    call(&mut kit, "delete_node", json!({ "handle": handle, "id": order[1] })).unwrap();
    assert_eq!(slides(&mut kit, &handle).len(), 18);
    assert!(chain_follows_order(&mut kit, &handle));

    let row = kit.session(&handle).unwrap().graph.node(node_id(&order[0])).unwrap().parent_id.unwrap().to_string();
    let error = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": row, "name": "x", "width": 10, "height": 10 })).unwrap_err();
    assert!(error.to_string().contains("holds slides alone"), "{error}");
    let error = call(&mut kit, "move_node", json!({ "handle": handle, "id": order[0], "parent": order[1] })).unwrap_err();
    assert!(error.to_string().contains("a slide goes in a row of slides"), "{error}");
}

// ── FigJam boards ───────────────────────────────────────────────────────

/// The maintainer's board, open; `None` where it is not.
fn board() -> Option<(Toolkit, String)> {
    let path = testkit::fixture::dir().join("Roadmap review template (Copy).jam");
    if !path.is_file() {
        return None;
    }
    let mut kit = crate::new();
    let handle = call(&mut kit, "open_file", json!({ "path": path })).unwrap()["handle"].as_str().unwrap().to_string();
    Some((kit, handle))
}

#[test]
fn a_board_reads_its_stickies_shapes_connectors_and_tables() {
    let Some((mut kit, handle)) = board() else { return };
    let summary = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert_eq!(summary["product"], "whiteboard");
    assert_eq!(summary["board"], json!({ "stickies": 26, "shapes_with_text": 56, "connectors": 5, "sections": 23, "tables": 5, "stamps": 8, "widgets": 11 }));
    let sticky = read(&mut kit, &handle, "13709:3272");
    assert_eq!((sticky["type"].clone(), sticky["fills"][0]["color"].clone()), (json!("STICKY"), json!("#93E396")));
    assert_eq!((sticky["authorName"].clone(), sticky["authorVisible"].clone()), (json!("Yuhki"), json!(false)), "the author is generated hidden: {sticky}");
    let shape = read(&mut kit, &handle, "13709:3270");
    assert_eq!((shape["text"]["characters"].clone(), shape["shapeType"].clone()), (json!("NOT STARTED"), json!("SQUARE")));
    let table = read(&mut kit, &handle, "14610:1953");
    assert_eq!((table["numRows"].clone(), table["numColumns"].clone()), (json!(4), json!(6)));
    assert_eq!(table["cells"][0][0], "Project Name");
    let connector = read(&mut kit, &handle, "13709:3649");
    assert_eq!((connector["connectorStart"]["endpointNodeId"].clone(), connector["connectorLineType"].clone()), (json!("13709:3602"), json!("STRAIGHT")));
    let found = call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "STICKY" })).unwrap();
    assert_eq!(found["total"], 26);
}

#[test]
fn a_boards_generated_layers_are_drawn_and_a_stamp_is_named() {
    let Some((kit, handle)) = board() else { return };
    let session = kit.session(&handle).unwrap();
    // The Highlights section: nine green stickies on a pale section.
    let picture = render::render(&session.fig, &session.graph, node_id("13709:3271"), 0.5, 2048).unwrap();
    let pixmap = resvg::tiny_skia::Pixmap::decode_png(&picture.png).unwrap();
    let green = pixmap.pixels().iter().filter(|p| { let c = p.demultiply(); c.green() > 200 && c.red() < 170 && c.blue() < 170 }).count();
    assert!(green > 10_000, "the stickies show: {green} green pixels");
    let team = render::render(&session.fig, &session.graph, node_id("13709:3293"), 0.4, 2048).unwrap();
    assert!(team.notes.iter().any(|n| n.contains("stamp")), "{:?}", team.notes);
}

#[test]
fn a_stickys_text_and_colour_are_written_where_figjam_generates_them() {
    let Some((mut kit, handle)) = board() else { return };
    let out = set(&mut kit, &handle, "13709:3272", json!({ "text": { "characters": "Written by grafta" }, "fills": [{ "type": "SOLID", "color": "#FFC943" }] })).unwrap();
    assert_eq!(out["set"], json!(["fills", "characters"]), "{out}");
    let sticky = read(&mut kit, &handle, "13709:3272");
    assert_eq!((sticky["text"]["characters"].clone(), sticky["fills"][0]["color"].clone()), (json!("Written by grafta"), json!("#FFC943")));
    assert!(raw_of(&kit, &handle, "13709:3272").get("derivedImmutableFrameData").is_none(), "FigJam's old result is dropped");
    let error = set(&mut kit, &handle, "13709:3272", json!({ "cornerRadius": 4 })).unwrap_err();
    assert!(error.to_string().contains("text, characters, fills"), "{error}");
}

// ── slide transitions kept with their slides ────────────────────────────

#[test]
fn speaker_notes_are_written_as_the_deck_holds_them_and_read_back_as_text() {
    let text = crate::edge::measure::Text;
    let json = crate::product::slides::deck::notes_json(&text, "Hola me llamo Nossair!\n\nDos");
    let value: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["root"]["type"], "root");
    assert_eq!(value["root"]["children"][0]["children"][0]["text"], "Hola me llamo Nossair!");
    assert_eq!(crate::product::slides::deck::notes_text(&text, &json).as_deref(), Some("Hola me llamo Nossair!\n\nDos"));
}

/// The maintainer's deck with transitions, a skipped slide, notes and object
/// animations (2026-10-25); `None` where it is not.
fn notes_deck() -> Option<(Toolkit, String)> {
    let path = testkit::fixture::dir().join("Figma Deck Notes.deck");
    if !path.is_file() {
        return None;
    }
    let mut kit = crate::new();
    let handle = call(&mut kit, "open_file", json!({ "path": path })).unwrap()["handle"].as_str().unwrap().to_string();
    Some((kit, handle))
}

/// Each slide's link target and transition type, in deck order.
fn links(kit: &mut Toolkit, handle: &str) -> Vec<(String, Option<String>, Option<String>, bool)> {
    let order = slides(kit, handle);
    order
        .iter()
        .map(|slide| {
            let raw = call(kit, "read_raw", json!({ "handle": handle, "id": slide })).unwrap();
            let action = raw["prototypeInteractions"].as_array().and_then(|p| p.iter().find(|p| p["actions"][0]["navigationType"] == "NAVIGATE")).map(|p| p["actions"][0].clone());
            let target = action.as_ref().map(|a| format!("{}:{}", a["transitionNodeID"]["sessionID"], a["transitionNodeID"]["localID"]));
            let kind = action.as_ref().and_then(|a| a["transitionType"].as_str()).map(str::to_string);
            (slide.clone(), target, kind, raw["isSkippedSlide"] == true)
        })
        .collect()
}

/// Each slide links to the next not skipped, the last not skipped to itself.
fn links_follow_order(links: &[(String, Option<String>, Option<String>, bool)]) -> bool {
    links.iter().enumerate().all(|(i, (slide, target, _, _))| {
        let next = links[i + 1..].iter().find(|l| !l.3).map(|l| l.0.clone()).unwrap_or_else(|| slide.clone());
        target.as_ref().is_none_or(|t| *t == next) && (target.is_some() || links[i + 1..].iter().all(|l| l.3))
    })
}

#[test]
fn slides_keep_their_own_transitions_through_a_move_a_duplicate_a_delete_and_a_skip() {
    let Some((mut kit, handle)) = notes_deck() else { return };
    let before = links(&mut kit, &handle);
    assert!(links_follow_order(&before), "as Figma Slides saved it: {before:?}");
    let transition_of = |links: &[(String, Option<String>, Option<String>, bool)], id: &str| links.iter().find(|l| l.0 == id).and_then(|l| l.2.clone());
    let last = before.last().unwrap().0.clone();
    let first = before[0].0.clone();

    call(&mut kit, "move_node", json!({ "handle": handle, "id": last, "behind": first })).unwrap();
    let after = links(&mut kit, &handle);
    assert!(links_follow_order(&after), "{after:?}");
    for (slide, _, kind, skipped) in &before {
        if !skipped {
            assert_eq!(&transition_of(&after, slide), kind, "{slide} keeps its transition");
        }
    }

    let animated = "1:341";
    let copy = call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": animated })).unwrap()["id"].as_str().unwrap().to_string();
    assert!(links_follow_order(&links(&mut kit, &handle)));
    let view = read(&mut kit, &handle, &copy);
    let inside: Vec<String> = kit.session(&handle).unwrap().graph.nodes.values().filter(|n| n.parent_id == Some(node_id(&copy))).map(|n| n.id.to_string()).collect();
    for animation in view["objectAnimations"].as_array().unwrap() {
        assert!(inside.contains(&animation["node"].as_str().unwrap().to_string()), "the copy animates its own layers: {view}");
    }
    assert_eq!(view["speakerNotes"], "Hola me llamo Nossair!");

    call(&mut kit, "delete_node", json!({ "handle": handle, "id": "1:346" })).unwrap();
    assert!(links_follow_order(&links(&mut kit, &handle)));

    let out = set(&mut kit, &handle, "1:351", json!({ "isSkippedSlide": true, "speakerNotes": "Saltada" })).unwrap();
    assert_eq!(out["set"], json!(["isSkippedSlide", "speakerNotes"]), "{out}");
    let skipped = links(&mut kit, &handle);
    assert!(links_follow_order(&skipped), "{skipped:?}");
    assert_eq!(read(&mut kit, &handle, "1:351")["isSkippedSlide"], true);

    let out = set(&mut kit, &handle, "1:355", json!({ "fills": [{ "type": "SOLID", "color": "#FFEECC" }], "speakerNotes": "Fondo nuevo" })).unwrap();
    assert!(out["set"].as_array().unwrap().iter().any(|k| k == "fills") && out["set"].as_array().unwrap().iter().any(|k| k == "speakerNotes"), "{out}");
}

// ── slide transitions and object animations written ─────────────────────

/// A refused call leaves the file as it was. Before the unit of work, the
/// slide's own keys stayed written when the frame's refused one failed it.
#[test]
fn a_refused_call_on_a_slide_leaves_its_transition_as_it_was() {
    let Some((mut kit, handle)) = notes_deck() else { return };
    let first = slides(&mut kit, &handle)[0].clone();
    let before = read(&mut kit, &handle, &first);
    let error = set(&mut kit, &handle, &first, json!({ "transition": { "style": "DISSOLVE", "duration": 0.7 }, "characters": "x" })).unwrap_err();
    assert!(error.to_string().contains("characters"), "{error}");
    assert_eq!(read(&mut kit, &handle, &first), before);
    assert_eq!(kit.session(&handle).unwrap().edits, 0);
}

#[test]
fn a_slide_takes_a_frames_keys_and_its_decks_in_one_call_the_frames_reported_first() {
    let Some((mut kit, handle)) = notes_deck() else { return };
    let first = slides(&mut kit, &handle)[0].clone();
    let edits = kit.session(&handle).unwrap().edits;
    let out = set(&mut kit, &handle, &first, json!({
        "isSkippedSlide": true,
        "fills": [{ "type": "SOLID", "color": "#336699" }],
        "transition": { "style": "DISSOLVE" },
        "cornerRadius": 12,
    }))
    .unwrap();
    assert_eq!(out["set"], json!(["cornerRadius", "fills", "isSkippedSlide", "transition"]), "{out}");
    assert_eq!(out["unsaved_edits"], json!(edits + 1), "one call, one edit: {out}");
    let only_deck = set(&mut kit, &handle, &first, json!({ "speakerNotes": "Hi" })).unwrap();
    assert_eq!(only_deck, json!({ "id": first, "set": ["speakerNotes"], "unsaved_edits": edits + 2 }));
    let error = set(&mut kit, &handle, &first, json!({ "transition": { "style": "DISSOLVE" }, "characters": "x" })).unwrap_err();
    assert!(error.to_string().contains("characters"), "{error}");
}

#[test]
fn a_slide_takes_a_transition_and_object_animations_and_reads_them_back() {
    let Some((mut kit, handle)) = notes_deck() else { return };
    let order = slides(&mut kit, &handle);
    let (first, last) = (order[0].clone(), order.last().unwrap().clone());
    let layer = kit.session(&handle).unwrap().graph.nodes.values().find(|n| n.parent_id == Some(node_id(&first))).unwrap().id.to_string();

    let out = set(&mut kit, &handle, &first, json!({
        "transition": { "style": "SMART_ANIMATE", "duration": 0.8, "curve": "EASE_IN_AND_OUT", "timing": { "type": "ON_CLICK" } },
        "objectAnimations": [{ "node": layer, "type": "SLIDE_FROM_BOTTOM", "duration": 0.5 }, { "node": layer, "type": "FADE", "phase": "OUT" }],
    }))
    .unwrap();
    assert_eq!(out["set"], json!(["transition", "objectAnimations"]), "{out}");
    let view = read(&mut kit, &handle, &first);
    assert_eq!(view["transition"], json!({ "style": "SMART_ANIMATE", "duration": 0.8, "curve": "EASE_IN_AND_OUT", "timing": { "type": "ON_CLICK" } }), "{view}");
    assert_eq!(view["objectAnimations"][0], json!({ "node": layer, "type": "SLIDE_FROM_BOTTOM", "phase": "IN", "duration": 0.5 }));
    assert_eq!(view["objectAnimations"][1]["phase"], "OUT");
    assert!(links_follow_order(&links(&mut kit, &handle)));

    // The last slide's transition is a link to itself; an instant one is none.
    set(&mut kit, &handle, &last, json!({ "transition": { "style": "DISSOLVE" } })).unwrap();
    let after = links(&mut kit, &handle);
    assert_eq!(after.last().unwrap().1.as_deref(), Some(last.as_str()), "{after:?}");
    assert_eq!(read(&mut kit, &handle, &last)["transition"]["style"], "DISSOLVE");
    set(&mut kit, &handle, &last, json!({ "transition": { "style": "NONE" } })).unwrap();
    assert_eq!(links(&mut kit, &handle).last().unwrap().1, None);

    // Refused, and nothing written: a style the API does not list, a layer of
    // another slide.
    let other = kit.session(&handle).unwrap().graph.nodes.values().find(|n| n.parent_id == Some(node_id(&order[1]))).unwrap().id.to_string();
    assert!(set(&mut kit, &handle, &first, json!({ "transition": { "style": "SPIN" } })).is_err());
    assert!(set(&mut kit, &handle, &first, json!({ "objectAnimations": [{ "node": other }] })).is_err());
    assert_eq!(read(&mut kit, &handle, &first)["objectAnimations"].as_array().unwrap().len(), 2);

    // A move keeps what was written.
    call(&mut kit, "move_node", json!({ "handle": handle, "id": first, "behind": last })).unwrap();
    assert_eq!(read(&mut kit, &handle, &first)["transition"]["style"], "SMART_ANIMATE");
    assert!(links_follow_order(&links(&mut kit, &handle)));
}

#[test]
fn slides_duplicated_again_and_again_are_searched_linked_and_placed_in_the_decks_order() {
    let Some((mut kit, handle)) = deck() else { return };
    let first = slides(&mut kit, &handle)[0].clone();
    // Each copy goes right after the original, so the last made is second.
    let copies: Vec<String> = (0..3)
        .map(|_| call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": first })).unwrap()["id"].as_str().unwrap().to_string())
        .collect();
    let order = slides(&mut kit, &handle);
    assert_eq!(order[1..4], [copies[2].clone(), copies[1].clone(), copies[0].clone()]);
    // search_nodes lists them in that order, not in the order they were made.
    let found = call(&mut kit, "search_nodes", json!({ "handle": handle, "type": "SLIDE", "limit": 1000 })).unwrap();
    let searched: Vec<String> = found["results"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_string()).collect();
    assert_eq!(searched, order);
    assert!(links_follow_order(&links(&mut kit, &handle)));
    // Side by side, as Figma Slides lays a row out: x 0, 2160, 4320…
    let xs: Vec<f32> = order.iter().map(|s| kit.session(&handle).unwrap().graph.node(node_id(s)).unwrap().x).collect();
    for (i, x) in xs.iter().enumerate() {
        assert_eq!(*x, i as f32 * 2160.0, "slide {i} of {xs:?}");
    }
}

// ── FigJam boards written ───────────────────────────────────────────────

#[test]
fn a_sticky_and_a_shape_move_resize_and_take_their_texts_colour() {
    let Some((mut kit, handle)) = board() else { return };
    let (sticky, shape) = ("13709:3272", "13709:3296");
    let out = set(&mut kit, &handle, sticky, json!({ "text": { "characters": "Moved", "fills": [{ "type": "SOLID", "color": "#13704B" }] }, "x": 10, "y": 20 })).unwrap();
    assert_eq!(out["set"], json!(["x", "y", "characters", "text.fills"]), "{out}");
    let view = read(&mut kit, &handle, sticky);
    assert_eq!((view["x"].as_f64(), view["y"].as_f64()), (Some(10.0), Some(20.0)));
    assert_eq!(view["text"], json!({ "characters": "Moved", "fills": [{ "type": "SOLID", "color": "#13704B" }] }));
    // FigJam sizes and aligns a sticky itself.
    assert!(set(&mut kit, &handle, sticky, json!({ "width": 400 })).is_err());
    assert!(set(&mut kit, &handle, sticky, json!({ "textAlignHorizontal": "CENTER" })).is_err());

    set(&mut kit, &handle, shape, json!({ "characters": "Toolkit", "width": 320, "height": 96, "fontSize": 32, "textAlignHorizontal": "CENTER" })).unwrap();
    let view = read(&mut kit, &handle, shape);
    assert_eq!((view["width"].as_f64(), view["height"].as_f64()), (Some(320.0), Some(96.0)));
    let raw = call(&mut kit, "read_raw", json!({ "handle": handle, "id": shape, "fields": ["nodeGenerationData"] })).unwrap();
    let text = raw["nodeGenerationData"]["overrides"].as_array().unwrap().iter().find(|o| o["guidPath"]["guids"][0]["localID"] == 1).unwrap().clone();
    assert_eq!(text["fontSize"].as_f64(), Some(32.0));
    assert_eq!(text["textAlignHorizontal"], "CENTER");
    assert_eq!((text["lineHeight"]["value"].as_f64(), text["lineHeight"]["units"].as_str()), (Some(43.0), Some("PIXELS")));
}

#[test]
fn a_table_takes_its_cells_and_grows_rows_for_more() {
    let Some((mut kit, handle)) = board() else { return };
    let table = "14610:2007";
    let before = read(&mut kit, &handle, table);
    assert_eq!(before["numRows"], 4);
    let cells = json!([["#", "Feature"], ["049", "Slides"], ["050", "FigJam"], ["051", "Transitions"], ["052", "Animations"], ["053", "Boards"]]);
    set(&mut kit, &handle, table, json!({ "cells": cells })).unwrap();
    let after = read(&mut kit, &handle, table);
    assert_eq!(after["numRows"], 6);
    assert_eq!(after["numColumns"], 6);
    for (r, row) in cells.as_array().unwrap().iter().enumerate() {
        assert_eq!(after["cells"][r][0], row[0], "{after}");
        assert_eq!(after["cells"][r][1], row[1]);
    }
    // Two rows of 96 more.
    assert_eq!(after["height"].as_f64().unwrap() - before["height"].as_f64().unwrap(), 192.0);
    // More texts in a row than columns is refused, and nothing written.
    assert!(set(&mut kit, &handle, table, json!({ "cells": [["1", "2", "3", "4", "5", "6", "7"]] })).is_err());
    assert_eq!(read(&mut kit, &handle, table)["cells"][0][0], "#");
}

#[test]
fn a_connector_runs_between_two_nodes_with_its_caps_and_line() {
    let Some((mut kit, handle)) = board() else { return };
    let (a, b) = ("13709:3272", "13709:3273");
    let copy = call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": "13709:3649" })).unwrap()["id"].as_str().unwrap().to_string();
    let out = set(&mut kit, &handle, &copy, json!({
        "connectorStart": { "endpointNodeId": a, "magnet": "RIGHT" },
        "connectorEnd": { "endpointNodeId": b, "magnet": "LEFT" },
        "connectorEndStrokeCap": "ARROW_LINES",
        "connectorLineType": "CURVED",
    }))
    .unwrap();
    assert_eq!(out["set"], json!(["connectorEndStrokeCap", "connectorLineType", "connectorStart", "connectorEnd"]), "{out}");
    let view = read(&mut kit, &handle, &copy);
    assert_eq!(view["connectorStart"], json!({ "endpointNodeId": a, "magnet": "RIGHT" }));
    assert_eq!(view["connectorEnd"], json!({ "endpointNodeId": b, "magnet": "LEFT" }));
    assert_eq!(view["connectorEndStrokeCap"], "ARROW_LINES");
    assert_eq!(view["connectorLineType"], "CURVED");
    // Refused: a magnet, a cap or a node the file does not have.
    assert!(set(&mut kit, &handle, &copy, json!({ "connectorStart": { "endpointNodeId": a, "magnet": "UP" } })).is_err());
    assert!(set(&mut kit, &handle, &copy, json!({ "connectorEndStrokeCap": "WASHI_TAPE_1" })).is_err());
    assert!(set(&mut kit, &handle, &copy, json!({ "connectorEnd": { "endpointNodeId": "9:9" } })).is_err());
    // Drawn as a line from one to the other, until FigJam routes it.
    let rendered = call(&mut kit, "render_node", json!({ "handle": handle, "id": "13709:3602" }));
    assert!(rendered.is_ok());
}

mod journal;
mod layout;
mod measure;
mod memory;
mod sites;
