//! The scene graph, built the way the rest of the workspace builds it.

use model::{NodeType, SceneGraph, SceneNode};
use testkit::scene::guid;

#[test]
fn links_children_to_their_parent() {
    let mut graph = SceneGraph::default();
    graph.insert(SceneNode::new(guid("0:1"), "Page", NodeType::Frame), None);
    graph.insert(SceneNode::new(guid("1:1"), "Card", NodeType::Frame), Some(guid("0:1")));

    let page = graph.node(guid("0:1")).expect("page");
    assert_eq!(page.children, vec![guid("1:1")]);
    assert_eq!(graph.node(guid("1:1")).and_then(|n| n.parent_id), Some(guid("0:1")));
    assert_eq!(graph.children(guid("0:1")).count(), 1);
}

#[test]
fn walks_a_whole_subtree() {
    let graph = testkit::scene::card();
    let mut visited = Vec::new();
    graph.walk(guid("1:1"), |node, _| visited.push(node.id));
    assert_eq!(visited, vec![guid("1:1"), guid("1:2")]);
}

/// Every field the generators read comes off a real file, so a fixture is the
/// only thing that proves the decode chain end to end. Absent one, skip.
#[test]
fn builds_a_graph_from_every_available_fixture() {
    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let Some((fig, graph)) = testkit::fixture::shared(name) else { continue };

        assert!(!graph.nodes.is_empty(), "{name}: no nodes");
        assert!(!graph.pages.is_empty(), "{name}: no pages");
        assert!(
            graph.nodes.values().all(|node| node.node_type != NodeType::Unknown),
            "{name}: variable and style node changes must stay out of the scene tree"
        );
        assert_eq!(graph.images.len(), fig.images.len(), "{name}: images not carried over");
    }
}

/// A text styled apart reads as runs that lie inside its
/// characters, in order, and its lists as one paragraph per line.
#[test]
fn every_styled_text_reads_as_runs_inside_its_characters() {
    let (mut styled, mut listed) = (0, 0);
    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let Some((_, graph)) = testkit::fixture::shared(name) else { continue };
        for node in graph.nodes.values().filter(|n| n.node_type == NodeType::Text) {
            let text = node.text.as_deref().unwrap_or_default();
            let units = text.encode_utf16().count();
            let mut at = 0;
            for run in &node.text_runs {
                assert!(at <= run.start && run.start < run.end && run.end <= units, "{name}: {} has a run {}..{} out of place", node.id, run.start, run.end);
                at = run.end;
            }
            styled += usize::from(!node.text_runs.is_empty());
            if !node.paragraphs.is_empty() {
                listed += 1;
                assert_eq!(node.paragraphs.len(), text.split('\n').count(), "{name}: {} has a paragraph per line", node.id);
            }
        }
    }
    if !testkit::fixture::all().is_empty() {
        assert!(styled > 0 && listed > 0, "the fixtures hold styled texts and lists: {styled}, {listed}");
    }
}
