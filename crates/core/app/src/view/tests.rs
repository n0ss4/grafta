use model::{Color, Effect, EffectType, Fill, NodeType, SceneGraph, SceneNode};


use super::*;

fn graph_with(node: SceneNode) -> SceneGraph {
    let mut graph = SceneGraph::default();
    graph.insert(node, None);
    graph
}

#[test]
fn colours_are_exact_hex_with_alpha_folded_into_opacity() {
    let mut node = SceneNode::new(testkit::scene::guid("1:1"), "Box", NodeType::Rectangle);
    node.fills = vec![Fill::solid(Color { r: 0.2, g: 0.4, b: 0.6, a: 0.5 })];
    let graph = graph_with(node.clone());
    let view = properties(&graph, &node, false);
    assert_eq!(view["fills"][0]["color"], "#336699");
    assert_eq!(view["fills"][0]["opacity"], 0.5);
    assert_eq!(view["type"], "RECTANGLE");
}

#[test]
fn every_non_default_property_is_present_and_every_default_absent() {
    let mut node = SceneNode::new(testkit::scene::guid("1:1"), "Box", NodeType::Frame);
    let defaults = properties(&graph_with(node.clone()), &node, false);
    let mut keys: Vec<&str> = defaults.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["height", "id", "name", "type", "width", "x", "y"]);

    node.rotation = 90.0;
    node.visible = false;
    node.opacity = 0.25;
    node.clips_content = true;
    node.effects = vec![Effect {
        effect_type: EffectType::DropShadow,
        color: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.25 }),
        offset_x: 0.0,
        offset_y: 4.0,
        radius: 8.0,
        spread: 0.0,
        visible: true,
    }];
    node.horizontal_constraint = "STRETCH".into();
    let view = properties(&graph_with(node.clone()), &node, false);
    // The model keeps the code targets' sign; the view shows Figma's panel's
    //.
    assert_eq!(view["rotation"], -90);
    assert_eq!(view["visible"], false);
    assert_eq!(view["opacity"], 0.25);
    assert_eq!(view["clipsContent"], true);
    assert_eq!(view["effects"][0], data!({ "type": "DROP_SHADOW", "radius": 8.0, "color": "#000000", "opacity": 0.25, "offset": { "x": 0.0, "y": 4.0 } }));
    assert_eq!(view["constraints"]["horizontal"], "STRETCH");
}

#[test]
fn floats_do_not_carry_f32_noise() {
    assert_eq!(num(0.1), data!(0.1));
    assert_eq!(num(12.0), data!(12));
    assert_eq!(num(-3.5), data!(-3.5));
}

#[test]
fn types_parse_in_the_plugin_apis_spelling() {
    assert_eq!(parse_type("page"), Some(NodeType::Canvas));
    assert_eq!(parse_type("CANVAS"), Some(NodeType::Canvas));
    assert_eq!(parse_type("COMPONENT_SET"), Some(NodeType::ComponentSet));
    assert_eq!(parse_type("widget"), Some(NodeType::Widget), "FigJam's, since 050");
    assert_eq!(parse_type("banana"), None);
}
