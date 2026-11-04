//! Authoring into files Figma wrote.
//!
//! Everything here is the half of the feature grafta can check on its own: that
//! an id collides with nothing real, and that a document grafta assembled comes
//! back out of a file as the document that went in. What none of it can answer
//! is whether Figma opens the result, which is the ladder the
//! `synth` command writes.
//!
//! These skip when `fixtures/` is empty, which it is in a fresh clone.

// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![allow(clippy::disallowed_methods)]

use model::author::{Graft, Guid, Provenance, GRAFTA_SESSION};

/// The smallest fixture available, because everything below is a property of
/// the authoring rather than of any one file, and the small file makes the
/// suite quick. The collision scan runs over all of them.
fn smallest_fixture() -> Option<(String, figfile::FigFile)> {
    let mut paths = testkit::fixture::all();
    paths.sort_by_key(|path| std::fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX));

    for path in paths {
        let name = path.file_name()?.to_str()?.to_string();
        if let Some(fig) = testkit::fixture::parsed(&name) {
            return Some((name, fig));
        }
    }
    None
}

fn ids(fig: &figfile::FigFile) -> Vec<Guid> {
    model::author::node_changes(fig)
        .iter()
        .filter_map(|nc| nc.get("guid"))
        .filter_map(|guid| {
            Some(Guid::new(guid.get("sessionID")?.as_u32()?, guid.get("localID")?.as_u32()?))
        })
        .collect()
}

/// Against every node change in every file available — not a sample.
#[test]
fn graftas_session_collides_with_nothing_in_any_real_file() {
    let mut checked = 0;
    let mut nodes = 0;

    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        let Some(fig) = testkit::fixture::parsed(&name) else { continue };
        checked += 1;

        let existing = ids(&fig);
        nodes += existing.len();
        assert!(
            !existing.iter().any(|id| id.is_graftas()),
            "{name}: a real file already uses grafta's session {GRAFTA_SESSION}, \
             so the reserved id has to move"
        );

        // And the allocator agrees, which is the path the code actually takes.
        let mut fig = fig;
        assert!(Graft::new(&mut fig).is_ok(), "{name}: grafta could not claim its own session");
    }

    if checked == 0 {
        eprintln!("no fixtures present — skipped");
        return;
    }
    println!("checked {nodes} node changes across {checked} files");
}

/// Assemble a page, a frame and an instance into a real document, write
/// it, read it back, and find what was authored.
#[test]
fn a_grafted_document_survives_a_write_and_a_read() {
    let Some((name, original)) = smallest_fixture() else {
        eprintln!("no fixtures present — skipped");
        return;
    };

    let mut grafted = testkit::fixture::parsed(&name).expect("the fixture parses twice");
    let before = model::author::node_changes(&grafted).len();

    let (page, frame, instance) = {
        let mut graft = Graft::new(&mut grafted).expect("claims grafta's session");
        let symbol = graft.first_symbol().expect("the fixture holds a component to instantiate");
        let page = graft.add_page("grafta").expect("adds a page");
        let frame = graft.add_frame(page, "Authored frame", (320.0, 240.0)).expect("adds a frame");
        let instance = graft.add_instance(frame, "Authored instance", symbol).expect("adds an instance");
        (page, frame, instance)
    };

    let written = figfile::write_fig(&grafted).expect("writes");
    let reread = figfile::parse_fig(&written).expect("reads back");

    assert_eq!(figfile::compare(&grafted, &reread), None, "{name}: the graft changed in transit");
    assert_eq!(model::author::node_changes(&reread).len(), before + 3);

    // The authored nodes are there, and they are still grafta's.
    let read_ids = ids(&reread);
    for id in [page, frame, instance] {
        assert!(read_ids.contains(&id), "{name}: {id} did not come back");
        assert!(id.is_graftas(), "{name}: {id} lost its provenance");
    }

    // And the document reads as a document: the scene graph has the page, and
    // the tree hangs together the way it was assembled.
    let graph = model::build_scene_graph(&reread).expect("a scene graph");
    assert!(graph.pages.contains(&page), "{name}: the authored page is not a page");

    let frame_node = graph.node(frame).expect("the frame is in the graph");
    assert_eq!(frame_node.parent_id, Some(page));
    assert_eq!(frame_node.node_type, model::NodeType::Frame);
    assert_eq!(frame_node.children, vec![instance]);

    let instance_node = graph.node(instance).expect("the instance is in the graph");
    assert_eq!(instance_node.node_type, model::NodeType::Instance);
    assert!(instance_node.component_id.is_some(), "{name}: the instance lost its symbol");

    // Everything that was already there is still there, unchanged.
    assert_eq!(
        &model::author::node_changes(&reread)[..before],
        model::author::node_changes(&original),
        "{name}: the graft modified nodes it did not author"
    );
}

/// The same graft written twice is the same bytes, and a grafted file
/// round-trips like any other.
#[test]
fn a_graft_is_deterministic_and_survives_a_further_round_trip() {
    let Some((name, _)) = smallest_fixture() else {
        eprintln!("no fixtures present — skipped");
        return;
    };

    let write_once = || {
        let mut fig = testkit::fixture::parsed(&name).expect("the fixture parses");
        {
            let mut graft = Graft::new(&mut fig).expect("claims grafta's session");
            let symbol = graft.first_symbol().expect("a component to instantiate");
            let page = graft.add_page("grafta").expect("adds a page");
            let frame = graft.add_frame(page, "Authored frame", (320.0, 240.0)).expect("adds a frame");
            graft.add_instance(frame, "Authored instance", symbol).expect("adds an instance");
        }
        figfile::write_fig(&fig).expect("writes")
    };

    let first = write_once();
    let second = write_once();
    assert_eq!(first, second, "{name}: two identical grafts wrote different bytes");

    let reread = figfile::parse_fig(&first).expect("reads back");
    let again = figfile::write_fig(&reread).expect("writes again");
    assert_eq!(first, again, "{name}: a grafted file does not round-trip to itself");
}

/// A disagreement has to name where it is, because investigating this
/// format by hand is the expensive thing the whole feature exists to avoid.
#[test]
fn a_graft_shows_up_as_a_located_difference() {
    let Some((name, original)) = smallest_fixture() else {
        eprintln!("no fixtures present — skipped");
        return;
    };

    let mut grafted = testkit::fixture::parsed(&name).expect("the fixture parses twice");
    {
        let mut graft = Graft::new(&mut grafted).expect("claims grafta's session");
        graft.add_page("grafta").expect("adds a page");
    }

    let difference = figfile::compare(&original, &grafted).expect("the documents differ");
    assert!(
        difference.path.contains("nodeChanges"),
        "{name}: a grafted node was reported at {}, which is not where it is",
        difference.path
    );
}
