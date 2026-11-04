//! Figma Sites, on the maintainer's site: read, drawn, edited inside a
//! breakpoint, and saved as a `.site`.

use serde_json::{json, Value};

use crate::tests::call;
use crate::Toolkit;

/// The maintainer's site, open; `None` where it is not.
fn site() -> Option<(Toolkit, String)> {
    let path = testkit::fixture::dir().join("Read.cv.site");
    if !path.is_file() {
        return None;
    }
    let mut kit = crate::new();
    let handle = call(&mut kit, "open_file", json!({ "path": path })).unwrap()["handle"].as_str().unwrap().to_string();
    Some((kit, handle))
}

/// The site's first page and its breakpoints, by id.
fn page_and_breakpoints(kit: &mut Toolkit, handle: &str) -> (String, Vec<Value>) {
    let found = call(kit, "search_nodes", json!({ "handle": handle, "type": "RESPONSIVE_SET" })).unwrap();
    let page = found["results"][0]["id"].as_str().unwrap().to_string();
    let read = call(kit, "read_node", json!({ "handle": handle, "id": page, "depth": 1 })).unwrap();
    (page, read["children"].as_array().unwrap().clone())
}

#[test]
fn a_site_opens_as_sites_reads_its_pages_draws_and_round_trips() {
    let Some((mut kit, handle)) = site() else { return };
    let summary = call(&mut kit, "summarize", json!({ "handle": handle })).unwrap();
    assert_eq!((summary["product"].clone(), summary["site_pages"].clone()), (json!("site"), json!(1)), "{summary}");

    let (page, breakpoints) = page_and_breakpoints(&mut kit, &handle);
    let read = call(&mut kit, "read_node", json!({ "handle": handle, "id": page, "depth": 0 })).unwrap();
    assert_eq!((read["type"].clone(), read["name"].clone()), (json!("RESPONSIVE_SET"), json!("/")), "{read}");
    let names: Vec<&str> = breakpoints.iter().map(|b| b["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Desktop", "Mobile"]);
    let read_alone: Vec<Value> = breakpoints
        .iter()
        .map(|b| call(&mut kit, "read_node", json!({ "handle": handle, "id": b["id"], "depth": 0 })).unwrap())
        .collect();
    let primary: Vec<&Value> = read_alone.iter().filter(|b| b["isPrimaryBreakpoint"] == true).map(|b| &b["name"]).collect();
    assert_eq!(primary, [&json!("Desktop")], "one breakpoint is the page's primary");
    assert!(read_alone.iter().all(|b| !b["undecoded"].as_array().into_iter().flatten().any(|f| f == "isPrimaryBreakpoint")));
    let desktop = breakpoints[0]["id"].as_str().unwrap();

    let picture = call(&mut kit, "render_node", json!({ "handle": handle, "id": desktop, "scale": 0.5 })).unwrap();
    assert_eq!(picture["caption"]["width"], 365, "{picture}");
    assert!(call(&mut kit, "render_node", json!({ "handle": handle, "id": page, "scale": 0.25 })).is_ok(), "a site page draws as the frame it is");

    let fig = std::env::temp_dir().join(format!("grafta-062-{}.fig", std::process::id()));
    let error = call(&mut kit, "save_as", json!({ "handle": handle, "path": fig })).unwrap_err();
    assert!(error.to_string().contains("saved as .site"), "{error}");
    let out = std::env::temp_dir().join(format!("grafta-062-{}.site", std::process::id()));
    std::fs::remove_file(&out).ok();
    let saved = call(&mut kit, "save_as", json!({ "handle": handle, "path": out })).unwrap();
    assert_eq!(saved["reread"], "compared equal", "{saved}");
    let bytes = std::fs::read(&out).unwrap();
    std::fs::remove_file(&out).ok();
    assert_eq!(figfile::parse_fig(&bytes).unwrap().product, figfile::Product::Site);
}

#[test]
fn a_site_is_edited_inside_its_breakpoints_and_its_pages_hold_them_alone() {
    let Some((mut kit, handle)) = site() else { return };
    let (page, breakpoints) = page_and_breakpoints(&mut kit, &handle);
    let desktop = breakpoints[0]["id"].as_str().unwrap().to_string();

    let made = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": desktop, "name": "Banner", "width": 200, "height": 40 })).unwrap();
    let text = call(&mut kit, "create_text", json!({ "handle": handle, "parent": made["id"], "characters": "Hola" })).unwrap();
    assert_eq!(call(&mut kit, "read_node", json!({ "handle": handle, "id": text["id"], "depth": 0 })).unwrap()["characters"], "Hola");

    let error = call(&mut kit, "create_frame", json!({ "handle": handle, "parent": page, "name": "Tablet", "width": 600, "height": 900 })).unwrap_err();
    assert!(error.to_string().contains("put it inside a breakpoint"), "{error}");
    let error = call(&mut kit, "move_node", json!({ "handle": handle, "id": made["id"], "parent": page })).unwrap_err();
    assert!(error.to_string().contains("holds its breakpoints alone"), "{error}");
    let error = call(&mut kit, "duplicate_node", json!({ "handle": handle, "id": desktop })).unwrap_err();
    assert!(error.to_string().contains("holds its breakpoints alone"), "{error}");

    let error = call(&mut kit, "new_file", json!({ "path": "/tmp/grafta-062-new.site", "from": "/tmp/whatever.fig" })).unwrap_err();
    assert!(error.to_string().contains("a Figma Sites file from nothing is not made yet"), "{error}");
}
