//! Drawing hand-built scene graphs, checked pixel by pixel. No fixture is read
//!.

use figfile::FigFile;
use kiwi::{Schema, Value as Raw};
use model::{Effect, EffectType, Fill, FillType, GradientStop, Matrix, NodeType, SceneGraph, TextStyle};
use resvg::tiny_skia::Pixmap;
use testkit::scene::{guid, instance_with_override, rgb, solid, GraphBuilder, PAGE};

use crate::render;

/// A file with no node changes: every node is positioned by the graph alone.
fn empty_file() -> FigFile {
    FigFile {
        product: figfile::Product::Design,
        node_changes: Raw::Object(kiwi::Fields::from([("nodeChanges".to_string(), Raw::Array(Vec::new()))])),
        blobs: Vec::new(),
        images: Vec::new(),
        version: 101,
        schema: Schema { definitions: Vec::new() },
        schema_chunk: Vec::new(),
        canvas_name: "canvas.fig".into(),
        trailing_chunks: Vec::new(),
        other_entries: Vec::new(),
    }
}

/// The pixel at (x, y), as opaque 0–255 channels and alpha.
fn pixel(png: &[u8], x: u32, y: u32) -> [u8; 4] {
    let pixmap = Pixmap::decode_png(png).expect("a PNG");
    let p = pixmap.pixel(x, y).expect("inside the picture").demultiply();
    [p.red(), p.green(), p.blue(), p.alpha()]
}

/// A 100×60 frame `1:1` filled red on the page, holding `1:2`, a 20×20 blue
/// square at 10,10.
fn frame_with_square() -> GraphBuilder {
    GraphBuilder::with_page()
        .add("1:1", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 100.0;
            node.height = 60.0;
            node.fills = vec![solid(rgb(255, 0, 0))];
        })
        .add("1:2", "Square", NodeType::Rectangle, Some("1:1"), |node| {
            node.x = 10.0;
            node.y = 10.0;
            node.width = 20.0;
            node.height = 20.0;
            node.fills = vec![solid(rgb(0, 0, 255))];
        })
}

fn draw(graph: &SceneGraph, id: &str) -> crate::Rendered {
    render(&empty_file(), graph, guid(id), 1.0, 2048).expect("drawn")
}

#[test]
fn a_frame_and_its_child_are_drawn_where_they_are() {
    let graph = frame_with_square().build();
    let picture = draw(&graph, "1:1");
    assert_eq!((picture.width, picture.height), (100, 60));
    assert_eq!(pixel(&picture.png, 50, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 20, 20), [0, 0, 255, 255]);
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn a_hidden_child_is_not_drawn_and_a_hidden_root_is() {
    let graph = frame_with_square()
        .add("1:3", "Hidden", NodeType::Rectangle, Some("1:1"), |node| {
            node.x = 60.0;
            node.width = 40.0;
            node.height = 60.0;
            node.visible = false;
            node.fills = vec![solid(rgb(0, 255, 0))];
        })
        .build();
    assert_eq!(pixel(&draw(&graph, "1:1").png, 80, 30), [255, 0, 0, 255]);
    assert_eq!(pixel(&draw(&graph, "1:3").png, 20, 30), [0, 255, 0, 255]);
}

#[test]
fn a_clipping_frame_cuts_its_children_off() {
    let graph = GraphBuilder::with_page()
        .add("1:1", "Clip", NodeType::Frame, Some(PAGE), |node| {
            node.width = 40.0;
            node.height = 40.0;
            node.clips_content = true;
        })
        .add("1:2", "Wide", NodeType::Rectangle, Some("1:1"), |node| {
            node.width = 200.0;
            node.height = 40.0;
            node.fills = vec![solid(rgb(0, 0, 255))];
        })
        .add("1:0", "Outer", NodeType::Frame, Some(PAGE), |node| {
            node.width = 100.0;
            node.height = 40.0;
        })
        .build();
    // Draw the clip inside a wider parent, so the cut-off part would be visible.
    let mut graph = graph;
    graph.nodes.get_mut(&guid("1:1")).unwrap().parent_id = Some(guid("1:0"));
    graph.nodes.get_mut(&guid("1:0")).unwrap().children.push(guid("1:1"));
    let picture = draw(&graph, "1:0");
    assert_eq!(pixel(&picture.png, 20, 20)[3], 255);
    assert_eq!(pixel(&picture.png, 70, 20)[3], 0, "drawn past the clip");
}

#[test]
fn a_linear_gradient_runs_from_its_first_stop_to_its_last() {
    let graph = GraphBuilder::with_page()
        .add("1:1", "Gradient", NodeType::Rectangle, Some(PAGE), |node| {
            node.width = 100.0;
            node.height = 10.0;
            node.fills = vec![Fill {
                fill_type: FillType::GradientLinear,
                color: None,
                gradient_stops: vec![
                    GradientStop { color: rgb(255, 0, 0), position: 0.0 },
                    GradientStop { color: rgb(0, 0, 255), position: 1.0 },
                ],
                gradient_transform: Some(Matrix::default()),
                ..solid(rgb(0, 0, 0))
            }];
        })
        .build();
    let picture = draw(&graph, "1:1");
    let left = pixel(&picture.png, 1, 5);
    let right = pixel(&picture.png, 98, 5);
    assert!(left[0] > 240 && left[2] < 15, "{left:?}");
    assert!(right[2] > 240 && right[0] < 15, "{right:?}");
}

#[test]
fn an_instance_is_drawn_from_its_component_with_its_overrides() {
    let mut graph = instance_with_override();
    for (id, color) in [("2:1", rgb(0, 0, 255)), ("3:1", rgb(0, 0, 255))] {
        let node = graph.nodes.get_mut(&guid(id)).unwrap();
        node.width = 40.0;
        node.height = 20.0;
        node.fills = vec![solid(color)];
    }
    let picture = draw(&graph, "3:1");
    assert_eq!(pixel(&picture.png, 38, 18), [0, 0, 255, 255]);
    // The overridden label has no stored layout, so it is drawn with a font.
    assert!(picture.notes.iter().any(|n| n.contains("laid out")), "{:?}", picture.notes);
}

#[test]
fn text_without_figmas_layout_is_drawn_and_noted() {
    let graph = GraphBuilder::with_page()
        .add("1:1", "Title", NodeType::Text, Some(PAGE), |node| {
            node.width = 120.0;
            node.height = 30.0;
            node.text = Some("Hola".into());
            node.text_style = Some(TextStyle { font_size: 24.0, ..TextStyle::default() });
            node.fills = vec![solid(rgb(0, 0, 0))];
        })
        .build();
    let picture = draw(&graph, "1:1");
    assert!(picture.notes.iter().any(|n| n.contains("laid out")));
}

fn effect(effect_type: EffectType, color: model::Color, (dx, dy): (f32, f32), radius: f32, spread: f32) -> Effect {
    Effect { effect_type, color: Some(color), offset_x: dx, offset_y: dy, radius, spread, visible: true }
}

#[test]
fn an_inner_shadow_darkens_the_edge_it_falls_from_inside_the_shape() {
    let graph = GraphBuilder::with_page()
        .add("1:1", "Card", NodeType::Frame, Some(PAGE), |node| {
            node.width = 50.0;
            node.height = 50.0;
            node.fills = vec![solid(rgb(255, 255, 255))];
            node.effects = vec![effect(EffectType::InnerShadow, rgb(0, 0, 0), (0.0, 10.0), 0.0, 0.0)];
        })
        .build();
    let picture = draw(&graph, "1:1");
    assert_eq!(pixel(&picture.png, 25, 4), [0, 0, 0, 255], "the top band, uncovered by the shape moved down");
    assert_eq!(pixel(&picture.png, 25, 30), [255, 255, 255, 255]);
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn a_drop_shadows_spread_grows_it_beyond_the_shape() {
    let graph = GraphBuilder::with_page()
        .add("1:0", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 60.0;
            node.height = 60.0;
        })
        .add("1:1", "Square", NodeType::Rectangle, Some("1:0"), |node| {
            node.x = 20.0;
            node.y = 20.0;
            node.width = 20.0;
            node.height = 20.0;
            node.fills = vec![solid(rgb(0, 0, 255))];
            node.effects = vec![effect(EffectType::DropShadow, rgb(255, 0, 0), (0.0, 0.0), 0.0, 8.0)];
        })
        .build();
    let picture = draw(&graph, "1:0");
    assert_eq!(pixel(&picture.png, 30, 15), [255, 0, 0, 255], "5 px outside the square, inside the 8 px spread");
    assert_eq!(pixel(&picture.png, 30, 30), [0, 0, 255, 255], "the square over its shadow");
    assert_eq!(pixel(&picture.png, 30, 5)[3], 0, "beyond the spread");
}

#[test]
fn a_background_blur_blurs_the_layers_behind_it_within_its_shape() {
    let graph = GraphBuilder::with_page()
        .add("1:0", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 100.0;
            node.height = 60.0;
        })
        .add("1:1", "Red", NodeType::Rectangle, Some("1:0"), |node| {
            node.width = 50.0;
            node.height = 60.0;
            node.fills = vec![solid(rgb(255, 0, 0))];
        })
        .add("1:2", "Blue", NodeType::Rectangle, Some("1:0"), |node| {
            node.x = 50.0;
            node.width = 50.0;
            node.height = 60.0;
            node.fills = vec![solid(rgb(0, 0, 255))];
        })
        .add("1:3", "Glass", NodeType::Frame, Some("1:0"), |node| {
            node.x = 25.0;
            node.width = 50.0;
            node.height = 30.0;
            node.effects = vec![effect(EffectType::BackgroundBlur, rgb(0, 0, 0), (0.0, 0.0), 20.0, 0.0)];
        })
        .build();
    let picture = draw(&graph, "1:0");
    let [r, _, b, _] = pixel(&picture.png, 50, 15);
    assert!(r > 60 && b > 60, "red and blue mixed where the glass is: {r}, {b}");
    assert_eq!(pixel(&picture.png, 50, 45), [0, 0, 255, 255], "no blur outside the glass");
}

#[test]
fn a_picture_larger_than_the_maximum_is_scaled_down_to_fit() {
    let graph = frame_with_square().build();
    let picture = render(&empty_file(), &graph, guid("1:1"), 2.0, 50).unwrap();
    assert_eq!((picture.width, picture.height), (50, 30));
    assert!((picture.scale - 0.5).abs() < 1e-6);
    assert_eq!(pixel(&picture.png, 10, 10), [0, 0, 255, 255]);
}

#[test]
fn a_node_without_area_or_that_does_not_exist_is_refused() {
    let graph = GraphBuilder::with_page()
        .add("1:1", "Line", NodeType::Frame, Some(PAGE), |node| node.width = 10.0)
        .build();
    assert!(render(&empty_file(), &graph, guid("1:1"), 1.0, 2048).unwrap_err().0.contains("no area"));
    assert!(render(&empty_file(), &graph, guid("9:9"), 1.0, 2048).unwrap_err().0.contains("unknown"));
}

#[test]
fn a_path_that_opens_with_a_close_command_is_still_drawn() {
    // Figma's glyph outlines start with a close command; SVG drops such a
    // path whole unless it is removed.
    let graph = GraphBuilder::with_page()
        .add("1:1", "Glyph", NodeType::Vector, Some(PAGE), |node| {
            node.width = 10.0;
            node.height = 10.0;
            node.fill_geometry = vec![model::VectorPath {
                d: "Z M 0 0 L 10 0 L 10 10 L 0 10 Z".to_string(),
                fill_rule: "nonzero".to_string(),
            }];
            node.fills = vec![solid(rgb(0, 0, 255))];
        })
        .build();
    assert_eq!(pixel(&draw(&graph, "1:1").png, 5, 5), [0, 0, 255, 255]);
}

#[test]
fn a_blank_picture_is_one_colour_at_its_size() {
    let png = crate::blank_png(40, 24, [1.0, 0.0, 0.0, 1.0]).unwrap();
    let pixmap = Pixmap::decode_png(&png).unwrap();
    assert_eq!((pixmap.width(), pixmap.height()), (40, 24));
    assert_eq!(pixel(&png, 39, 23), [255, 0, 0, 255]);
}

#[test]
fn text_of_a_fixed_width_wraps_into_lines_like_figmas() {
    if crate::text::fonts().is_empty() {
        return; // A machine with no fonts draws no text to measure.
    }
    let graph = GraphBuilder::with_page()
        .add("1:1", "Paragraph", NodeType::Text, Some(PAGE), |node| {
            node.width = 60.0;
            node.height = 80.0;
            node.text = Some("one two three four".into());
            node.text_style = Some(TextStyle { font_size: 14.0, text_auto_resize: "HEIGHT".into(), ..TextStyle::default() });
            node.fills = vec![solid(rgb(0, 0, 0))];
        })
        .build();
    let picture = draw(&graph, "1:1");
    let pixmap = Pixmap::decode_png(&picture.png).unwrap();
    // Ink in the band of a second line, which one unbroken line would not have.
    let inked = |y0: u32, y1: u32| (0..60).any(|x| (y0..y1).any(|y| pixmap.pixel(x, y).is_some_and(|p| p.alpha() > 0)));
    assert!(inked(0, 17), "a first line");
    assert!(inked(18, 34), "a second line");
    assert!(picture.notes.iter().any(|n| n.contains("laid out")), "{:?}", picture.notes);
}

/// Inner (`2:1`, a red 10×10 dot `2:2` keyed `7:2`), Green (`2:5`, the same
/// dot in green), Outer (`3:1`) holding an instance of Inner (`3:2`, keyed
/// `7:1`), and on the page an instance of Outer (`4:1`) with `overrides`.
fn nested(overrides: Vec<model::SymbolOverride>) -> SceneGraph {
    let dot = |color| move |node: &mut model::SceneNode| {
        node.width = 10.0;
        node.height = 10.0;
        node.fills = vec![solid(color)];
        node.override_key = Some(guid("7:2"));
    };
    let sized = |node: &mut model::SceneNode| {
        node.width = 10.0;
        node.height = 10.0;
    };
    GraphBuilder::with_page()
        .add("2:1", "Inner", NodeType::Component, None, sized)
        .add("2:2", "Dot", NodeType::Rectangle, Some("2:1"), dot(rgb(255, 0, 0)))
        .add("2:5", "Green", NodeType::Component, None, sized)
        .add("2:6", "Dot", NodeType::Rectangle, Some("2:5"), dot(rgb(0, 255, 0)))
        .add("3:1", "Outer", NodeType::Component, None, sized)
        .add("3:2", "Inner", NodeType::Instance, Some("3:1"), |node| {
            sized(node);
            node.component_id = Some(guid("2:1"));
            node.override_key = Some(guid("7:1"));
        })
        .add("4:1", "Outer", NodeType::Instance, Some(PAGE), move |node| {
            sized(node);
            node.component_id = Some(guid("3:1"));
            node.symbol_overrides = overrides;
        })
        .build()
}

#[test]
fn an_override_through_a_nested_instance_reaches_the_layer_it_names() {
    let plain = draw(&nested(Vec::new()), "4:1");
    assert_eq!(pixel(&plain.png, 5, 5), [255, 0, 0, 255], "the inner component as it is");
    let blue = model::SymbolOverride {
        path: vec![guid("7:1"), guid("7:2")],
        fills: Some(vec![solid(rgb(0, 0, 255))]),
        ..model::SymbolOverride::default()
    };
    let picture = draw(&nested(vec![blue]), "4:1");
    assert_eq!(pixel(&picture.png, 5, 5), [0, 0, 255, 255]);
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn a_nested_instance_swapped_for_another_component_draws_that_component() {
    let swap = model::SymbolOverride { path: vec![guid("7:1")], symbol_id: Some(guid("2:5")), ..model::SymbolOverride::default() };
    assert_eq!(pixel(&draw(&nested(vec![swap]), "4:1").png, 5, 5), [0, 255, 0, 255]);
}

#[test]
fn a_component_property_the_instance_assigns_reaches_the_layer_bound_to_it() {
    // Inner's dot `2:2` shows or hides by property `9:1`; Outer's instance of
    // Inner assigns it, by its own `property_assignments`.
    let mut graph = nested(Vec::new());
    graph.nodes.get_mut(&guid("3:2")).unwrap().property_assignments.insert(guid("9:1"), "false".into());
    let bound = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("2:2").to_value()),
        ("componentPropRefs", kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([
            ("defID", guid("9:1").to_value()),
            ("componentPropNodeField", kiwi::Value::Enum("VISIBLE")),
        ]))])),
    ]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![bound]))]));
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5)[3], 0, "hidden by the property");
    graph.nodes.get_mut(&guid("3:2")).unwrap().property_assignments.insert(guid("9:1"), "true".into());
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5), [255, 0, 0, 255], "shown by it");
}

fn sized_ten(node: &mut model::SceneNode) {
    node.width = 10.0;
    node.height = 10.0;
}


/// An entry on `path` assigning property `9:1` to `value`.
fn assigning(path: &[&str], value: &str) -> model::SymbolOverride {
    model::SymbolOverride {
        path: path.iter().map(|key| guid(key)).collect(),
        assignments: [(guid("9:1"), value.to_string())].into_iter().collect(),
        ..model::SymbolOverride::default()
    }
}

#[test]
fn an_assignment_in_an_override_swaps_the_layer_bound_to_it() {

    let mut graph = GraphBuilder::with_page()
        .add("2:1", "Inner", NodeType::Component, None, sized_ten)
        .add("2:2", "Slot", NodeType::Instance, Some("2:1"), |node| {
            sized_ten(node);
            node.component_id = Some(guid("3:1"));
            node.override_key = Some(guid("3:2"));
        })
        .add("3:1", "Red", NodeType::Component, None, sized_ten)
        .add("3:3", "Dot", NodeType::Rectangle, Some("3:1"), |node| {
            sized_ten(node);
            node.fills = vec![solid(rgb(255, 0, 0))];
        })
        .add("2:5", "Green", NodeType::Component, None, sized_ten)
        .add("2:6", "Dot", NodeType::Rectangle, Some("2:5"), |node| {
            sized_ten(node);
            node.fills = vec![solid(rgb(0, 255, 0))];
        })
        .add("5:1", "Outer", NodeType::Component, None, sized_ten)
        .add("5:2", "Inner", NodeType::Instance, Some("5:1"), |node| {
            sized_ten(node);
            node.component_id = Some(guid("2:1"));
            node.override_key = Some(guid("7:1"));
        })
        .add("4:1", "Outer", NodeType::Instance, Some(PAGE), |node| {
            sized_ten(node);
            node.component_id = Some(guid("5:1"));
        })
        .build();
    let mut fig = empty_file();
    let slot = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("2:2").to_value()),
        ("componentPropRefs", kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([
            ("defID", guid("9:1").to_value()),
            ("componentPropNodeField", kiwi::Value::Enum("OVERRIDDEN_SYMBOL_ID")),
        ]))])),
    ]));
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![slot]))]));
    assert_eq!(pixel(&render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap().png, 5, 5), [255, 0, 0, 255], "the default");
    graph.nodes.get_mut(&guid("4:1")).unwrap().symbol_overrides = vec![assigning(&["7:1"], "2:5")];
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5), [0, 255, 0, 255], "the component assigned");
}

#[test]
fn an_assignment_in_an_override_wins_over_the_instances_own() {
    let mut graph = nested(vec![assigning(&["7:1"], "true")]);
    graph.nodes.get_mut(&guid("3:2")).unwrap().property_assignments.insert(guid("9:1"), "false".into());
    let bound = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("2:2").to_value()),
        ("componentPropRefs", kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([
            ("defID", guid("9:1").to_value()),
            ("componentPropNodeField", kiwi::Value::Enum("VISIBLE")),
        ]))])),
    ]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![bound]))]));
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5), [255, 0, 0, 255], "shown by the outer value");
}

#[test]
fn a_swapped_instance_takes_its_box_from_the_component_swapped_to() {
    let mut graph = nested(vec![model::SymbolOverride {
        path: vec![guid("7:1")],
        symbol_id: Some(guid("2:5")),
        ..model::SymbolOverride::default()
    }]);
    let green = graph.nodes.get_mut(&guid("2:5")).unwrap();
    green.fills = vec![solid(rgb(0, 0, 255))];
    graph.nodes.get_mut(&guid("2:6")).unwrap().width = 2.0;
    graph.nodes.get_mut(&guid("2:6")).unwrap().height = 2.0;
    let bare = kiwi::Value::Object(kiwi::Fields::from([("guid", guid("3:2").to_value())]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![bare]))]));
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 1, 1), [0, 255, 0, 255], "its layer");
    assert_eq!(pixel(&picture.png, 7, 7), [0, 0, 255, 255], "its box, the fill of the component swapped to");
}

#[test]
fn every_entry_of_a_path_counts() {
    let name = model::SymbolOverride { path: vec![guid("7:1"), guid("7:2")], name: Some("Dot".into()), ..model::SymbolOverride::default() };
    let blue = model::SymbolOverride { path: vec![guid("7:1"), guid("7:2")], fills: Some(vec![solid(rgb(0, 0, 255))]), ..model::SymbolOverride::default() };
    let picture = draw(&nested(vec![name, blue]), "4:1");
    assert_eq!(pixel(&picture.png, 5, 5), [0, 0, 255, 255]);
}

#[test]
fn an_outer_entry_wins_field_by_field() {
    let outer = || model::SymbolOverride {
        path: vec![guid("7:1"), guid("7:2")],
        fills: Some(vec![solid(rgb(0, 0, 255))]),
        ..model::SymbolOverride::default()
    };
    let inner = |entry: model::SymbolOverride| model::SymbolOverride { path: vec![guid("7:2")], ..entry };
    let mut graph = nested(vec![outer()]);
    let green = model::SymbolOverride { fills: Some(vec![solid(rgb(0, 255, 0))]), ..model::SymbolOverride::default() };
    graph.nodes.get_mut(&guid("3:2")).unwrap().symbol_overrides = vec![inner(green)];
    assert_eq!(pixel(&draw(&graph, "4:1").png, 5, 5), [0, 0, 255, 255], "the outer fill");
    let hidden = model::SymbolOverride { visible: Some(false), ..model::SymbolOverride::default() };
    graph.nodes.get_mut(&guid("3:2")).unwrap().symbol_overrides = vec![inner(hidden)];
    assert_eq!(pixel(&draw(&graph, "4:1").png, 5, 5)[3], 0, "what the outer entry leaves unset comes from the inner");
}

/// A label `2:2` in a component, in an instance `3:2` that stores a layout for it
/// (a square at 100,100, 50 across); `4:1` an instance around it.
fn label_with_a_stored_layout(overrides: Vec<model::SymbolOverride>) -> (FigFile, SceneGraph) {
    let big = |node: &mut model::SceneNode| {
        node.width = 200.0;
        node.height = 200.0;
    };
    let graph = GraphBuilder::with_page()
        .add("2:1", "Inner", NodeType::Component, None, big)
        .add("2:2", "Label", NodeType::Text, Some("2:1"), move |node| {
            big(node);
            node.text = Some("Old".into());
            node.text_style = Some(TextStyle::default());
            node.fills = vec![solid(rgb(255, 0, 0))];
            node.override_key = Some(guid("7:2"));
        })
        .add("3:1", "Outer", NodeType::Component, None, big)
        .add("3:2", "Inner", NodeType::Instance, Some("3:1"), move |node| {
            big(node);
            node.component_id = Some(guid("2:1"));
            node.override_key = Some(guid("7:1"));
        })
        .add("4:1", "Outer", NodeType::Instance, Some(PAGE), move |node| {
            big(node);
            node.component_id = Some(guid("3:1"));
            node.symbol_overrides = overrides;
        })
        .build();
    let inner = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("3:2").to_value()),
        ("derivedSymbolData", kiwi::Value::Array(vec![stored_layout(&["7:2"])])),
    ]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![inner]))]));
    fig.blobs = vec![square_blob(10.0)];
    (fig, graph)
}

fn stored_layout(keys: &[&str]) -> kiwi::Value {
    let guids = keys.iter().map(|key| guid(key).to_value()).collect();
    let position = kiwi::Value::Object(kiwi::Fields::from([("x", kiwi::Value::Float(100.0)), ("y", kiwi::Value::Float(150.0))]));
    let glyph = kiwi::Value::Object(kiwi::Fields::from([
        ("commandsBlob", kiwi::Value::Uint(0)),
        ("fontSize", kiwi::Value::Float(5.0)),
        ("position", position),
    ]));
    kiwi::Value::Object(kiwi::Fields::from([
        ("guidPath", kiwi::Value::Object(kiwi::Fields::from([("guids", kiwi::Value::Array(guids))]))),
        ("derivedTextData", kiwi::Value::Object(kiwi::Fields::from([("glyphs", kiwi::Value::Array(vec![glyph]))]))),
    ]))
}

fn new_label() -> model::SymbolOverride {
    model::SymbolOverride { path: vec![guid("7:1"), guid("7:2")], text: Some("New".into()), ..model::SymbolOverride::default() }
}

#[test]
fn a_field_the_outer_entry_leaves_unset_comes_from_the_inner_entry() {
    // The outer entry holds the dot's size; the nested instance's, where it is.
    let graph = nested(Vec::new());
    let key = |keys: &[&str]| {
        let guids = keys.iter().map(|k| guid(k).to_value()).collect();
        ("guidPath", kiwi::Value::Object(kiwi::Fields::from([("guids", kiwi::Value::Array(guids))])))
    };
    let moved = kiwi::Value::Object(kiwi::Fields::from([("m00", kiwi::Value::Float(1.0)), ("m11", kiwi::Value::Float(1.0)), ("m02", kiwi::Value::Float(5.0))]));
    let inner = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("3:2").to_value()),
        ("derivedSymbolData", kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([key(&["7:2"]), ("transform", moved)]))])),
    ]));
    let outer = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("4:1").to_value()),
        ("derivedSymbolData", kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([key(&["7:1", "7:2"]), ("size", cached_size(10.0))]))])),
    ]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![inner, outer]))]));
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 2, 5)[3], 0, "moved by the inner entry");
    assert_eq!(pixel(&picture.png, 7, 5), [255, 0, 0, 255]);
}

const UNDONE: &str = "the instance around it keeps its size";

#[test]
fn an_outer_text_override_is_not_drawn_with_the_nested_layout() {
    let (fig, graph) = label_with_a_stored_layout(Vec::new());
    let plain = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&plain.png, 125, 125), [255, 0, 0, 255], "with no override, the layout stored");
    let (fig, graph) = label_with_a_stored_layout(vec![new_label()]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 125, 125)[3], 0, "not the old glyphs");
    assert!(picture.notes.iter().any(|n| n.contains(UNDONE)), "{:?}", picture.notes);
}

#[test]
fn an_outer_layout_for_an_overridden_text_is_drawn_with_no_note() {
    let (mut fig, graph) = label_with_a_stored_layout(vec![new_label()]);
    let kiwi::Value::Object(root) = &mut fig.node_changes else { panic!() };
    let outer = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("4:1").to_value()),
        ("derivedSymbolData", kiwi::Value::Array(vec![stored_layout(&["7:1", "7:2"])])),
    ]));
    let inner = root.get("nodeChanges").and_then(kiwi::Value::as_array).unwrap()[0].clone();
    *root = kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![inner, outer]))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 125, 125), [255, 0, 0, 255]);
    assert!(picture.notes.iter().all(|n| !n.contains(UNDONE)), "{:?}", picture.notes);
}

#[test]
fn an_instance_is_drawn_as_figmas_cached_expansion_says() {
    // Outer's instance `4:1` caches the dot two levels down at 20×20 and at
    // 5,5: Figma's resolved layout, which the component alone does not have.
    let mut graph = nested(Vec::new());
    for id in ["4:1", "3:1", "3:2"] {
        let node = graph.nodes.get_mut(&guid(id)).unwrap();
        node.width = 30.0;
        node.height = 30.0;
    }
    let path = |keys: &[&str]| kiwi::Value::Object(kiwi::Fields::from([("guids", kiwi::Value::Array(keys.iter().map(|key| guid(key).to_value()).collect()))]));
    let size = kiwi::Value::Object(kiwi::Fields::from([("x", kiwi::Value::Float(20.0)), ("y", kiwi::Value::Float(20.0))]));
    let at = kiwi::Value::Object(kiwi::Fields::from([
        ("m00", kiwi::Value::Float(1.0)), ("m01", kiwi::Value::Float(0.0)), ("m02", kiwi::Value::Float(5.0)),
        ("m10", kiwi::Value::Float(0.0)), ("m11", kiwi::Value::Float(1.0)), ("m12", kiwi::Value::Float(5.0)),
    ]));
    let instance = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("4:1").to_value()),
        ("derivedSymbolData", kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([
            ("guidPath", path(&["7:1", "7:2"])),
            ("size", size),
            ("transform", at),
        ]))])),
    ]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![instance]))]));
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 20, 20), [255, 0, 0, 255], "the dot where and as large as the cache says");
    assert_eq!(pixel(&picture.png, 2, 2)[3], 0, "not at the component's 0,0");
    assert!(picture.notes.iter().all(|n| !n.contains("resized")), "{:?}", picture.notes);
}

// ── 083: an instance's vectors at its size ──────────────────────────────────

/// A filled square from 0,0, as Figma stores an outline.
fn square_blob(side: f32) -> Vec<u8> {
    let mut blob = Vec::new();
    for (command, x, y) in [(1u8, 0.0, 0.0), (2, side, 0.0), (2, side, side), (2, 0.0, side)] {
        blob.push(command);
        blob.extend_from_slice(&f32::to_le_bytes(x));
        blob.extend_from_slice(&f32::to_le_bytes(y));
    }
    blob.push(0);
    blob
}

fn square_path(side: f32) -> model::VectorPath {
    model::VectorPath { d: format!("M0 0 L{side} 0 L{side} {side} L0 {side} Z"), fill_rule: "nonzero".into() }
}

/// Component `2:1` holding a red vector `2:2` (keyed `7:2`) `size` wide, drawn
/// by `own`, and on the page an instance of it (`4:1`) `shown` wide.
fn instance_of_vector(size: f32, own: model::VectorPath, shown: f32) -> SceneGraph {
    GraphBuilder::with_page()
        .add("2:1", "Component", NodeType::Component, None, |node| {
            node.width = size;
            node.height = size;
        })
        .add("2:2", "Vector", NodeType::Vector, Some("2:1"), |node| {
            node.width = size;
            node.height = size;
            node.fills = vec![solid(rgb(255, 0, 0))];
            node.fill_geometry = vec![own];
            node.override_key = Some(guid("7:2"));
        })
        .add("4:1", "Instance", NodeType::Instance, Some(PAGE), |node| {
            node.width = shown;
            node.height = shown;
            node.component_id = Some(guid("2:1"));
        })
        .build()
}

/// A file where instance `4:1` caches `fields` for the layer at `keys`, and
/// whose blob 0 is a square of `side`.
fn file_caching(keys: &[&str], side: f32, fields: Vec<(&str, kiwi::Value)>) -> FigFile {
    let guids = keys.iter().map(|key| guid(key).to_value()).collect();
    let mut entry = vec![("guidPath", kiwi::Value::Object(kiwi::Fields::from([("guids", kiwi::Value::Array(guids))])))];
    entry.extend(fields);
    let instance = kiwi::Value::Object(kiwi::Fields::from([
        ("guid", guid("4:1").to_value()),
        ("derivedSymbolData", kiwi::Value::Array(vec![kiwi::Value::Object(entry.into_iter().collect())])),
    ]));
    let mut fig = empty_file();
    fig.node_changes = kiwi::Value::Object(kiwi::Fields::from([("nodeChanges", kiwi::Value::Array(vec![instance]))]));
    fig.blobs = vec![square_blob(side)];
    fig
}

fn stored_outline(blob: u32) -> kiwi::Value {
    kiwi::Value::Array(vec![kiwi::Value::Object(kiwi::Fields::from([("commandsBlob", kiwi::Value::Uint(blob))]))])
}

fn cached_size(side: f32) -> kiwi::Value {
    kiwi::Value::Object(kiwi::Fields::from([("x", kiwi::Value::Float(side)), ("y", kiwi::Value::Float(side))]))
}

#[test]
fn a_resized_instance_draws_its_vector_from_its_cached_outlines() {
    let graph = instance_of_vector(300.0, square_path(300.0), 30.0);
    let fig = file_caching(&["7:2"], 10.0, vec![("size", cached_size(30.0)), ("fillGeometry", stored_outline(0))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5), [255, 0, 0, 255], "the outline the instance stores");
    assert_eq!(pixel(&picture.png, 20, 20)[3], 0, "not the component's, which covers all of it");
    assert!(picture.notes.iter().all(|n| !n.contains("scaled")), "{:?}", picture.notes);
}

#[test]
fn a_cached_fill_leaves_the_components_stroke() {
    let graph = {
        let mut graph = instance_of_vector(30.0, square_path(30.0), 30.0);
        let vector = graph.nodes.get_mut(&guid("2:2")).unwrap();
        vector.strokes = vec![solid(rgb(0, 0, 255))];
        vector.stroke_geometry = vec![model::VectorPath { d: "M20 20 L30 20 L30 30 L20 30 Z".into(), fill_rule: "nonzero".into() }];
        graph
    };
    let fig = file_caching(&["7:2"], 10.0, vec![("fillGeometry", stored_outline(0))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5), [255, 0, 0, 255], "the fill from the entry");
    assert_eq!(pixel(&picture.png, 15, 15)[3], 0, "and not the component's fill");
    assert_eq!(pixel(&picture.png, 25, 25), [0, 0, 255, 255], "the stroke from the component");
}

#[test]
fn a_nested_instance_draws_the_outlines_its_outer_entry_names() {
    let mut graph = nested(Vec::new());
    let dot = graph.nodes.get_mut(&guid("2:2")).unwrap();
    dot.node_type = NodeType::Vector;
    dot.fill_geometry = vec![square_path(10.0)];
    let fig = file_caching(&["7:1", "7:2"], 4.0, vec![("fillGeometry", stored_outline(0))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 2, 2), [255, 0, 0, 255], "the outline two levels down");
    assert_eq!(pixel(&picture.png, 8, 8)[3], 0, "not the component's");
}

#[test]
fn an_outline_that_cannot_be_read_is_named_and_not_taken_from_the_component() {
    let graph = instance_of_vector(30.0, square_path(30.0), 30.0);
    let fig = file_caching(&["7:2"], 10.0, vec![("fillGeometry", stored_outline(9))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5)[3], 0, "nothing drawn for it");
    assert!(picture.notes.iter().any(|n| n.contains("outline")), "{:?}", picture.notes);
}

#[test]
fn an_empty_outline_draws_nothing_and_says_nothing() {
    let graph = instance_of_vector(30.0, square_path(30.0), 30.0);
    let mut fig = file_caching(&["7:2"], 10.0, vec![("fillGeometry", stored_outline(0))]);
    fig.blobs = vec![Vec::new()];
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 5, 5)[3], 0, "the instance stores no outline here");
    assert!(picture.notes.iter().all(|n| !n.contains("outline")), "{:?}", picture.notes);
}

#[test]
fn a_new_size_with_no_outlines_is_scaled_and_named() {
    let graph = instance_of_vector(300.0, square_path(150.0), 30.0);
    let fig = file_caching(&["7:2"], 10.0, vec![("size", cached_size(30.0))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 10, 10), [255, 0, 0, 255], "half of the instance, as half of the component");
    assert_eq!(pixel(&picture.png, 20, 20)[3], 0, "and no more");
    assert!(picture.notes.iter().any(|n| n.contains("scaled")), "{:?}", picture.notes);
}

#[test]
fn the_same_size_with_no_outlines_draws_as_before() {
    let graph = instance_of_vector(30.0, square_path(15.0), 30.0);
    let fig = file_caching(&["7:2"], 10.0, vec![("size", cached_size(30.0))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 10, 10), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 20, 20)[3], 0);
    assert!(picture.notes.iter().all(|n| !n.contains("scaled")), "{:?}", picture.notes);
}

#[test]
fn a_paragraph_breaks_with_its_first_line_narrower_by_its_indent() {
    let chars: Vec<char> = "aa bb cc".chars().collect();
    let advances = vec![10.0; chars.len()];
    // 50 wide: "aa bb" (50) fits; indented by 20, the first line takes "aa".
    let plain = crate::text::break_paragraph(&chars, &advances, Some(50.0), 0.0);
    assert_eq!(plain.iter().map(|l| (l.start, l.end)).collect::<Vec<_>>(), vec![(0, 6), (6, 8)]);
    let indented = crate::text::break_paragraph(&chars, &advances, Some(50.0), 20.0);
    assert_eq!(indented.iter().map(|l| (l.start, l.end)).collect::<Vec<_>>(), vec![(0, 3), (3, 8)]);
    assert_eq!(crate::text::break_paragraph(&chars, &advances, None, 20.0).len(), 1);
}

// ── 045: PDF ─────────────────────────────────────────────────────────────────

/// Each page's `MediaBox`, as krilla writes it uncompressed.
fn media_boxes(pdf: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(pdf);
    text.match_indices("MediaBox[").map(|(at, _)| text[at + 9..].split(']').next().unwrap().to_string()).collect()
}

#[test]
fn nodes_are_written_a_page_each_at_their_size_in_order() {
    let graph = frame_with_square()
        .add("2:1", "Other", NodeType::Frame, Some(PAGE), |node| {
            node.width = 40.0;
            node.height = 30.0;
            node.fills = vec![solid(rgb(0, 255, 0))];
        })
        .build();
    let pdf = crate::pdf::pdf(&empty_file(), &graph, &[guid("2:1"), guid("1:1")]).unwrap();
    assert!(pdf.bytes.starts_with(b"%PDF-"));
    assert_eq!(media_boxes(&pdf.bytes), ["0 0 40 30", "0 0 100 60"]);
    assert_eq!(pdf.pages.iter().map(|(id, _)| *id).collect::<Vec<_>>(), [guid("2:1"), guid("1:1")]);
    assert!(pdf.pages.iter().all(|(_, notes)| !notes.iter().any(|n| n == crate::pdf::RASTERISED)));
}

#[test]
fn a_page_with_a_shadow_says_it_is_partly_an_image() {
    let mut graph = frame_with_square().build();
    graph.nodes.get_mut(&guid("1:2")).unwrap().effects = vec![effect(EffectType::DropShadow, rgb(0, 0, 0), (0.0, 4.0), 8.0, 0.0)];
    let pdf = crate::pdf::pdf(&empty_file(), &graph, &[guid("1:1")]).unwrap();
    assert!(pdf.pages[0].1.iter().any(|n| n == crate::pdf::RASTERISED), "{:?}", pdf.pages);
}

#[test]
fn a_pdf_of_nothing_or_of_an_unknown_node_is_refused() {
    let graph = frame_with_square().build();
    assert!(crate::pdf::pdf(&empty_file(), &graph, &[]).is_err());
    let error = crate::pdf::pdf(&empty_file(), &graph, &[guid("9:9")]).unwrap_err();
    assert!(error.0.contains("9:9"), "{error}");
}

// ── 046: contrast ────────────────────────────────────────────────────────────

/// A white 100×60 frame `1:1` holding a 60×20 text `1:3` at 10,10 in `colour`,
/// at `size` px.
fn text_on_white(colour: model::Color, size: f32) -> GraphBuilder {
    GraphBuilder::with_page()
        .add("1:1", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 100.0;
            node.height = 60.0;
            node.fills = vec![solid(rgb(255, 255, 255))];
        })
        .add("1:3", "Label", NodeType::Text, Some("1:1"), move |node| {
            node.x = 10.0;
            node.y = 10.0;
            node.width = 60.0;
            node.height = 20.0;
            node.text = Some("Label".into());
            node.text_style = Some(TextStyle { font_size: size, ..TextStyle::default() });
            node.fills = vec![solid(colour)];
        })
}

fn contrast_of(graph: &SceneGraph) -> crate::contrast::Contrast {
    let mut all = crate::contrast::contrast(&empty_file(), graph, guid("1:1")).unwrap();
    assert_eq!(all.len(), 1, "{all:?}");
    all.remove(0)
}

#[test]
fn wcag_ratios_come_out_to_two_decimals() {
    assert!((crate::contrast::ratio([0.0; 3], [1.0; 3]) - 21.0).abs() < 0.01);
    let grey = |v: u8| [f32::from(v) / 255.0; 3];
    assert!((crate::contrast::ratio(grey(0x77), [1.0; 3]) - 4.48).abs() < 0.01);
    assert!((crate::contrast::ratio(grey(0x76), [1.0; 3]) - 4.54).abs() < 0.01);
}

#[test]
fn grey_text_on_white_fails_aa_by_a_shade_and_passes_one_darker() {
    let c = contrast_of(&text_on_white(rgb(0x77, 0x77, 0x77), 14.0).build());
    assert!((c.ratio.unwrap() - 4.48).abs() < 0.01, "{c:?}");
    assert_eq!(c.required, 4.5);
    assert_eq!(c.background.as_deref(), Some("#FFFFFF"));
    assert_eq!(c.foreground.as_deref(), Some("#777777"));
    assert!(c.varies.is_none());
    let c = contrast_of(&text_on_white(rgb(0x76, 0x76, 0x76), 14.0).build());
    assert!(c.ratio.unwrap() >= 4.5, "{c:?}");
}

#[test]
fn large_text_needs_three_to_one() {
    assert_eq!(contrast_of(&text_on_white(rgb(0x77, 0x77, 0x77), 24.0).build()).required, 3.0);
    assert_eq!(contrast_of(&text_on_white(rgb(0x77, 0x77, 0x77), 20.0).build()).required, 4.5);
}

#[test]
fn a_sibling_drawn_below_is_the_background_not_the_parent() {
    let graph = text_on_white(rgb(255, 255, 255), 14.0)
        .add("1:2", "Chip", NodeType::Rectangle, Some("1:1"), |node| {
            node.x = 0.0;
            node.y = 0.0;
            node.width = 100.0;
            node.height = 40.0;
            node.fills = vec![solid(rgb(0, 0, 0))];
        })
        .build();
    let mut graph = graph;
    // The chip is drawn first, below the text.
    let children = &mut graph.nodes.get_mut(&guid("1:1")).unwrap().children;
    children.retain(|c| *c != guid("1:2"));
    children.insert(0, guid("1:2"));
    let c = contrast_of(&graph);
    assert_eq!(c.background.as_deref(), Some("#000000"), "{c:?}");
    assert!((c.ratio.unwrap() - 21.0).abs() < 0.01);
}

#[test]
fn opacity_lightens_the_text_over_what_is_behind_it() {
    let mut graph = text_on_white(rgb(0, 0, 0), 14.0).build();
    graph.nodes.get_mut(&guid("1:3")).unwrap().opacity = 0.5;
    let c = contrast_of(&graph);
    // Black at half opacity over white is mid grey.
    assert!((c.ratio.unwrap() - crate::contrast::ratio([0.5; 3], [1.0; 3])).abs() < 0.02, "{c:?}");
}

#[test]
fn a_gradient_text_fill_or_nothing_behind_is_unmeasured_not_guessed() {
    let mut graph = text_on_white(rgb(0, 0, 0), 14.0).build();
    graph.nodes.get_mut(&guid("1:3")).unwrap().fills = vec![Fill { fill_type: FillType::GradientLinear, ..solid(rgb(0, 0, 0)) }];
    let c = contrast_of(&graph);
    assert!(c.ratio.is_none() && c.sample.unmeasurable.as_deref().unwrap().contains("solid"), "{c:?}");

    let mut graph = text_on_white(rgb(0, 0, 0), 14.0).build();
    graph.nodes.get_mut(&guid("1:1")).unwrap().fills = Vec::new();
    let c = contrast_of(&graph);
    assert!(c.ratio.is_none() && c.sample.unmeasurable.as_deref().unwrap().contains("nothing is painted"), "{c:?}");
}

#[test]
fn a_background_of_two_colours_gives_the_lower_ratio_and_says_it_varies() {
    let graph = text_on_white(rgb(0x55, 0x55, 0x55), 14.0)
        .add("1:2", "Half", NodeType::Rectangle, Some("1:1"), |node| {
            node.x = 40.0;
            node.width = 60.0;
            node.height = 60.0;
            node.fills = vec![solid(rgb(0x99, 0x99, 0x99))];
        })
        .build();
    let mut graph = graph;
    let children = &mut graph.nodes.get_mut(&guid("1:1")).unwrap().children;
    children.retain(|c| *c != guid("1:2"));
    children.insert(0, guid("1:2"));
    let c = contrast_of(&graph);
    let grey = |v: u8| [f32::from(v) / 255.0; 3];
    assert!((c.ratio.unwrap() - crate::contrast::ratio(grey(0x55), grey(0x99))).abs() < 0.02, "{c:?}");
    assert_eq!(c.background.as_deref(), Some("#999999"));
    assert!(c.varies.is_some());
}

#[test]
fn a_text_in_an_instance_is_measured_with_its_override_and_named_with_it() {
    let mut graph = instance_with_override();
    for (id, w, h) in [("2:1", 80.0, 30.0), ("3:1", 80.0, 30.0), ("2:2", 60.0, 20.0)] {
        let node = graph.nodes.get_mut(&guid(id)).unwrap();
        node.width = w;
        node.height = h;
    }
    graph.nodes.get_mut(&guid("2:1")).unwrap().fills = vec![solid(rgb(0, 0, 255))];
    graph.nodes.get_mut(&guid("3:1")).unwrap().fills = vec![solid(rgb(0, 0, 255))];
    graph.nodes.get_mut(&guid("2:2")).unwrap().fills = vec![solid(rgb(0, 0, 0))];
    graph.nodes.get_mut(&guid("3:1")).unwrap().symbol_overrides[0].fills = Some(vec![solid(rgb(255, 255, 255))]);
    let all = crate::contrast::contrast(&empty_file(), &graph, guid("3:1")).unwrap();
    assert_eq!(all.len(), 1, "{all:?}");
    assert_eq!(all[0].sample.instance, Some(guid("3:1")));
    assert_eq!(all[0].foreground.as_deref(), Some("#FFFFFF"), "the override's colour: {:?}", all[0]);
    assert_eq!(all[0].background.as_deref(), Some("#0000FF"));
}

/// `depth` frames, each inside the last, on the page, from `3:0`; the deepest a
/// red square.
fn nested_frames(depth: u32) -> SceneGraph {
    let sized = |node: &mut model::SceneNode| {
        node.width = 10.0;
        node.height = 10.0;
    };
    let mut graph = GraphBuilder::with_page().add("3:0", "Frame", NodeType::Frame, Some(PAGE), sized);
    for level in 1..depth {
        let (id, parent) = (format!("3:{level}"), format!("3:{}", level - 1));
        graph = graph.add(&id, "Frame", NodeType::Frame, Some(&parent), sized);
    }
    let last = format!("3:{}", depth - 1);
    graph
        .add("4:0", "Square", NodeType::Rectangle, Some(&last), |node| {
            sized(node);
            node.fills = vec![solid(rgb(255, 0, 0))];
        })
        .build()
}

/// `work` from a stack far smaller than any grafta runs on.
fn from_a_small_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| std::thread::Builder::new().stack_size(256 << 10).spawn_scoped(scope, work).unwrap().join().unwrap())
}

const TOO_DEEP: &str = "layers nested deeper than 256";

#[test]
fn layers_past_the_limit_are_left_out_and_named() {
    let graph = nested_frames(10_000);
    let picture = from_a_small_stack(|| draw(&graph, "3:0"));
    assert!(picture.notes.iter().any(|n| n.contains(TOO_DEEP)), "{:?}", picture.notes);
    assert_eq!(pixel(&picture.png, 5, 5)[3], 0, "the square, below the limit, is not drawn");
    let pdf = from_a_small_stack(|| crate::pdf::pdf(&empty_file(), &graph, &[guid("3:0")])).unwrap();
    assert!(pdf.pages[0].1.iter().any(|n| n.contains(TOO_DEEP)), "{:?}", pdf.pages);
}

#[test]
fn layers_within_the_limit_are_drawn_with_no_note() {
    let graph = nested_frames(200);
    let picture = from_a_small_stack(|| draw(&graph, "3:0"));
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
    assert_eq!(pixel(&picture.png, 5, 5), [255, 0, 0, 255]);
}

#[test]
fn a_chain_of_components_counts_through_its_instances() {
    let sized = |node: &mut model::SceneNode| {
        node.width = 10.0;
        node.height = 10.0;
    };
    let mut graph = GraphBuilder::with_page();
    for level in 0..1000 {
        let (component, instance) = (format!("5:{level}"), format!("6:{level}"));
        graph = graph.add(&component, "C", NodeType::Component, None, sized);
        graph = graph.add(&instance, "I", NodeType::Instance, Some(&component), move |node| {
            sized(node);
            node.component_id = Some(guid(&format!("5:{}", level + 1)));
        });
    }
    let graph = graph
        .add("5:1000", "C", NodeType::Component, None, sized)
        .add("7:1", "I", NodeType::Instance, Some(PAGE), move |node| {
            sized(node);
            node.component_id = Some(guid("5:0"));
        })
        .build();
    let picture = from_a_small_stack(|| draw(&graph, "7:1"));
    assert!(picture.notes.iter().any(|n| n.contains(TOO_DEEP)), "{:?}", picture.notes);
}

#[test]
fn contrast_past_the_limit_says_so_rather_than_measuring_part() {
    let graph = nested_frames(10_000);
    let error = from_a_small_stack(|| crate::contrast::contrast(&empty_file(), &graph, guid("3:0"))).expect_err("refused");
    assert!(error.to_string().contains(TOO_DEEP), "{error}");
}

#[test]
fn components_doubling_at_each_level_stop_at_the_limit_of_layers() {
    // 40 levels would be 2^40 layers, from these 120 nodes.
    let sized = |node: &mut model::SceneNode| {
        node.width = 10.0;
        node.height = 10.0;
    };
    let mut graph = GraphBuilder::with_page();
    for level in 0..40 {
        let component = format!("5:{level}");
        graph = graph.add(&component, "C", NodeType::Component, None, sized);
        for side in ["a", "b"] {
            let id = format!("{}:{level}", if side == "a" { 6 } else { 8 });
            graph = graph.add(&id, "I", NodeType::Instance, Some(&component), move |node| {
                sized(node);
                node.component_id = Some(guid(&format!("5:{}", level + 1)));
            });
        }
    }
    let graph = graph
        .add("5:40", "C", NodeType::Component, None, sized)
        .add("7:1", "I", NodeType::Instance, Some(PAGE), move |node| {
            sized(node);
            node.component_id = Some(guid("5:0"));
        })
        .build();
    let picture = draw(&graph, "7:1");
    assert!(picture.notes.iter().any(|n| n.contains("limit of layers")), "{:?}", picture.notes);
    let error = crate::contrast::contrast(&empty_file(), &graph, guid("7:1")).expect_err("refused");
    assert!(error.to_string().contains("limit of layers"), "{error}");
}

/// Boolean `2:2`, keyed `7:2`, 20 wide and red, inside frame `2:1`, holding an
/// operand `2:3` that is blue. `outline` is what the boolean stores.
fn boolean_with_operand(outline: Option<model::VectorPath>) -> GraphBuilder {
    GraphBuilder::with_page()
        .add("2:1", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 20.0;
            node.height = 20.0;
        })
        .add("2:2", "Union", NodeType::BooleanOperation, Some("2:1"), |node| {
            node.width = 20.0;
            node.height = 20.0;
            node.fills = vec![solid(rgb(255, 0, 0))];
            node.fill_geometry = outline.into_iter().collect();
            node.override_key = Some(guid("7:2"));
        })
        .add("2:3", "Vector", NodeType::Vector, Some("2:2"), |node| {
            node.width = 20.0;
            node.height = 20.0;
            node.fills = vec![solid(rgb(0, 0, 255))];
            node.fill_geometry = vec![square_path(20.0)];
        })
}

#[test]
fn a_boolean_shows_its_own_paint_and_not_its_operands() {
    let graph = boolean_with_operand(Some(square_path(20.0))).build();
    let picture = draw(&graph, "2:1");
    assert_eq!(pixel(&picture.png, 10, 10), [255, 0, 0, 255], "the boolean's fill, not the operand's");
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn a_boolean_with_no_paint_shows_nothing_of_its_operands() {
    let mut graph = boolean_with_operand(Some(square_path(20.0))).build();
    graph.nodes.get_mut(&guid("2:2")).unwrap().fills = Vec::new();
    assert_eq!(pixel(&draw(&graph, "2:1").png, 10, 10)[3], 0);
}

#[test]
fn a_boolean_in_an_instance_shows_the_override_and_not_its_operands() {
    let mut graph = boolean_with_operand(Some(square_path(20.0)))
        .add("4:1", "Instance", NodeType::Instance, Some(PAGE), |node| {
            node.width = 20.0;
            node.height = 20.0;
            node.component_id = Some(guid("2:1"));
            node.symbol_overrides = vec![model::SymbolOverride {
                path: vec![guid("7:2")],
                fills: Some(vec![solid(rgb(0, 255, 0))]),
                ..model::SymbolOverride::default()
            }];
        })
        .build();
    graph.nodes.get_mut(&guid("2:1")).unwrap().node_type = NodeType::Component;
    assert_eq!(pixel(&draw(&graph, "4:1").png, 10, 10), [0, 255, 0, 255]);
}

#[test]
fn a_boolean_stored_only_by_its_instance_does_not_draw_its_operands() {
    let mut graph = boolean_with_operand(None)
        .add("4:1", "Instance", NodeType::Instance, Some(PAGE), |node| {
            node.width = 20.0;
            node.height = 20.0;
            node.component_id = Some(guid("2:1"));
        })
        .build();
    graph.nodes.get_mut(&guid("2:1")).unwrap().node_type = NodeType::Component;
    let fig = file_caching(&["7:2"], 20.0, vec![("fillGeometry", stored_outline(0))]);
    let picture = render(&fig, &graph, guid("4:1"), 1.0, 2048).unwrap();
    assert_eq!(pixel(&picture.png, 10, 10), [255, 0, 0, 255]);
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn an_operand_rendered_by_its_own_id_is_drawn() {
    let graph = boolean_with_operand(Some(square_path(20.0))).build();
    assert_eq!(pixel(&draw(&graph, "2:3").png, 10, 10), [0, 0, 255, 255]);
}

#[test]
fn a_boolean_with_no_outline_draws_its_operands_and_says_so() {
    let graph = boolean_with_operand(None).build();
    let picture = draw(&graph, "2:1");
    assert_eq!(pixel(&picture.png, 10, 10), [0, 0, 255, 255]);
    assert!(picture.notes.iter().any(|n| n.contains("not combined")), "{:?}", picture.notes);
}

#[test]
fn a_boolean_with_no_outline_and_no_operands_says_nothing_of_combining() {
    let mut graph = boolean_with_operand(None).build();
    graph.nodes.shift_remove(&guid("2:3"));
    let picture = draw(&graph, "2:1");
    assert!(picture.notes.iter().all(|n| !n.contains("not combined")), "{:?}", picture.notes);
}

/// Frame `5:1`, 40 × 20 and clear, holding blue `5:0` across its top, a mask
/// `5:2` over its left half, filled `paint`, and red `5:3` filling it.
fn masked(kind: &str, paint: Fill) -> GraphBuilder {
    let kind = kind.to_string();
    GraphBuilder::with_page()
        .add("5:1", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 40.0;
            node.height = 20.0;
        })
        .add("5:0", "Below", NodeType::Rectangle, Some("5:1"), |node| {
            node.width = 40.0;
            node.height = 4.0;
            node.fills = vec![solid(rgb(0, 0, 255))];
        })
        .add("5:2", "Mask", NodeType::Rectangle, Some("5:1"), move |node| {
            node.y = 4.0;
            node.width = 20.0;
            node.height = 16.0;
            node.fills = vec![paint];
            node.is_mask = true;
            node.mask_type = kind;
            node.override_key = Some(guid("7:2"));
        })
        .add("5:3", "Above", NodeType::Rectangle, Some("5:1"), |node| {
            node.y = 4.0;
            node.width = 40.0;
            node.height = 16.0;
            node.fills = vec![solid(rgb(255, 0, 0))];
        })
}

fn green() -> Fill {
    solid(rgb(0, 255, 0))
}

fn half_clear() -> Fill {
    solid(testkit::scene::rgba(0, 255, 0, 0.5))
}

#[test]
fn a_mask_shows_what_is_above_it_only_where_it_is_and_never_itself() {
    let picture = draw(&masked("ALPHA", green()).build(), "5:1");
    assert_eq!(pixel(&picture.png, 10, 12), [255, 0, 0, 255], "the layer above, over the mask, not the mask's green");
    assert_eq!(pixel(&picture.png, 30, 12)[3], 0, "the layer above, past the mask");
    assert_eq!(pixel(&picture.png, 30, 2), [0, 0, 255, 255], "the layer below, untouched");
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn an_alpha_mask_lets_through_what_its_alpha_says() {
    let [r, _, _, a] = pixel(&draw(&masked("ALPHA", half_clear()).build(), "5:1").png, 10, 12);
    assert_eq!(r, 255);
    assert!((120..=135).contains(&a), "half shown: {a}");
}

#[test]
fn an_outline_mask_shows_whole_inside_its_shape_whatever_its_paint() {
    let picture = draw(&masked("OUTLINE", half_clear()).build(), "5:1");
    assert_eq!(pixel(&picture.png, 10, 12), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 30, 12)[3], 0);
}

#[test]
fn a_luminance_mask_shows_over_white_and_not_over_black() {
    let white = solid(rgb(255, 255, 255));
    assert_eq!(pixel(&draw(&masked("LUMINANCE", white).build(), "5:1").png, 10, 12), [255, 0, 0, 255]);
    let black = solid(rgb(0, 0, 0));
    assert_eq!(pixel(&draw(&masked("LUMINANCE", black).build(), "5:1").png, 10, 12)[3], 0);
}

#[test]
fn a_hidden_mask_masks_nothing() {
    let mut graph = masked("ALPHA", green()).build();
    graph.nodes.get_mut(&guid("5:2")).unwrap().visible = false;
    assert_eq!(pixel(&draw(&graph, "5:1").png, 30, 12), [255, 0, 0, 255]);
}

#[test]
fn a_mask_hidden_by_an_instance_masks_nothing() {
    let mut graph = masked("ALPHA", green())
        .add("6:1", "Instance", NodeType::Instance, Some(PAGE), |node| {
            node.width = 40.0;
            node.height = 20.0;
            node.component_id = Some(guid("5:1"));
            node.symbol_overrides = vec![model::SymbolOverride {
                path: vec![guid("7:2")],
                visible: Some(false),
                ..model::SymbolOverride::default()
            }];
        })
        .build();
    graph.nodes.get_mut(&guid("5:1")).unwrap().node_type = NodeType::Component;
    assert_eq!(pixel(&draw(&graph, "6:1").png, 30, 12), [255, 0, 0, 255]);
}

#[test]
fn a_mask_asked_for_by_its_id_is_drawn() {
    assert_eq!(pixel(&draw(&masked("ALPHA", green()).build(), "5:2").png, 5, 5), [0, 255, 0, 255]);
}

#[test]
fn a_mask_larger_than_what_it_masks_is_not_cut_to_it() {
    let mut graph = masked("ALPHA", green()).build();
    let above = graph.nodes.get_mut(&guid("5:3")).unwrap();
    above.width = 2.0;
    above.height = 2.0;
    above.y = 18.0;
    above.x = 18.0;
    graph.nodes.get_mut(&guid("5:2")).unwrap().effects = vec![effect(EffectType::LayerBlur, rgb(0, 0, 0), (0.0, 0.0), 2.0, 0.0)];
    assert!(pixel(&draw(&graph, "5:1").png, 18, 18)[3] > 0, "a layer far from the mask's box's centre still shows");
}

#[test]
fn what_a_background_blur_rebuilds_does_not_paint_a_mask() {
    let mut graph = masked("ALPHA", green())
        .add("5:5", "Glass", NodeType::Rectangle, Some("5:1"), |node| {
            node.width = 40.0;
            node.height = 20.0;
            node.effects = vec![effect(EffectType::BackgroundBlur, rgb(0, 0, 0), (0.0, 0.0), 2.0, 0.0)];
        })
        .build();
    graph.nodes.shift_remove(&guid("5:3"));
    assert_eq!(pixel(&draw(&graph, "5:1").png, 10, 12)[1], 0, "no green from the mask");
}

#[test]
fn two_masks_in_one_parent_each_mask_up_to_the_next_and_say_it_was_not_measured() {
    let graph = masked("ALPHA", green())
        .add("5:6", "Second mask", NodeType::Rectangle, Some("5:1"), |node| {
            node.x = 20.0;
            node.width = 20.0;
            node.height = 20.0;
            node.fills = vec![solid(rgb(0, 255, 0))];
            node.is_mask = true;
        })
        .add("5:7", "Above both", NodeType::Rectangle, Some("5:1"), |node| {
            node.width = 40.0;
            node.height = 20.0;
            node.fills = vec![solid(rgb(255, 255, 255))];
        })
        .build();
    let picture = draw(&graph, "5:1");
    assert_eq!(pixel(&picture.png, 10, 12), [255, 0, 0, 255], "the first masks what is before the second");
    assert_eq!(pixel(&picture.png, 30, 12), [255, 255, 255, 255], "the second masks what is after it");
    assert!(picture.notes.iter().any(|n| n.contains("two masks")), "{:?}", picture.notes);
}

#[test]
fn a_text_that_is_a_mask_is_not_measured_for_contrast() {
    let graph = masked("ALPHA", green())
        .add("5:8", "Mask text", NodeType::Text, Some("5:1"), |node| {
            node.width = 20.0;
            node.height = 20.0;
            node.text = Some("Mask".to_string());
            node.text_style = Some(TextStyle::default());
            node.fills = vec![solid(rgb(0, 0, 0))];
            node.is_mask = true;
        })
        .build();
    let all = crate::contrast::contrast(&empty_file(), &graph, guid("5:1")).unwrap();
    assert!(all.is_empty(), "{all:?}");
}

#[test]
fn a_file_says_how_its_mask_masks_and_saying_nothing_means_alpha() {
    let layer = |id: &str, fields: Vec<(&str, Raw)>| {
        let mut all = vec![("guid", guid(id).to_value()), ("type", Raw::Enum("ROUNDED_RECTANGLE")), ("mask", Raw::Bool(true))];
        all.extend(fields);
        Raw::Object(all.into_iter().collect())
    };
    let mut fig = empty_file();
    fig.node_changes = Raw::Object(kiwi::Fields::from([(
        "nodeChanges",
        Raw::Array(vec![layer("1:1", vec![("maskType", Raw::Enum("OUTLINE"))]), layer("1:2", Vec::new())]),
    )]));
    let graph = model::build_scene_graph(&fig).unwrap();
    assert_eq!(graph.node(guid("1:1")).unwrap().mask_type, "OUTLINE");
    assert_eq!(graph.node(guid("1:2")).unwrap().mask_type, "ALPHA");
}

/// Frame `8:1`, 20 wide and yellow, holding `8:2`, a frame with no paint, which
/// holds cyan `8:3`.
fn blended(setup: impl FnOnce(&mut SceneGraph)) -> SceneGraph {
    let sized = |node: &mut model::SceneNode| {
        node.width = 20.0;
        node.height = 20.0;
    };
    let mut graph = GraphBuilder::with_page()
        .add("8:1", "Outer", NodeType::Frame, Some(PAGE), move |node| {
            sized(node);
            node.fills = vec![solid(rgb(255, 255, 0))];
        })
        .add("8:2", "Inner", NodeType::Frame, Some("8:1"), sized)
        .add("8:3", "Layer", NodeType::Rectangle, Some("8:2"), move |node| {
            sized(node);
            node.fills = vec![solid(rgb(0, 255, 255))];
            node.blend_mode = Some("MULTIPLY".into());
        })
        .build();
    setup(&mut graph);
    graph
}

const CYAN: [u8; 4] = [0, 255, 255, 255];
const PRODUCT: [u8; 4] = [0, 255, 0, 255];

#[test]
fn a_layer_blends_with_what_is_behind_it_through_a_frame_with_nothing_stored() {
    assert_eq!(pixel(&draw(&blended(|_| {}), "8:1").png, 10, 10), PRODUCT);
}

#[test]
fn a_frame_stored_normal_keeps_what_it_holds_from_what_is_behind_it() {
    let graph = blended(|graph| graph.nodes.get_mut(&guid("8:2")).unwrap().blend_mode = Some("NORMAL".into()));
    assert_eq!(pixel(&draw(&graph, "8:1").png, 10, 10), CYAN);
}

#[test]
fn a_blend_inside_a_clipped_frame_reaches_its_fill() {
    let graph = blended(|graph| {
        let inner = graph.nodes.get_mut(&guid("8:2")).unwrap();
        inner.clips_content = true;
        inner.fills = vec![solid(rgb(255, 255, 0))];
    });
    let picture = draw(&graph, "8:1");
    assert_eq!(pixel(&picture.png, 10, 10), PRODUCT);
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn a_blend_inside_a_clipped_frame_with_no_fill_says_it_stops_there() {
    let graph = blended(|graph| graph.nodes.get_mut(&guid("8:2")).unwrap().clips_content = true);
    let picture = draw(&graph, "8:1");
    assert_eq!(pixel(&picture.png, 10, 10), CYAN);
    assert!(picture.notes.iter().any(|n| n.contains("clipped")), "{:?}", picture.notes);
}

#[test]
fn a_paint_blends_with_the_paints_before_it() {
    let graph = blended(|graph| {
        let layer = graph.nodes.get_mut(&guid("8:3")).unwrap();
        layer.blend_mode = None;
        let mut over = solid(rgb(255, 255, 255));
        over.blend_mode = Some("DIFFERENCE".into());
        layer.fills = vec![solid(rgb(255, 0, 0)), over];
    });
    assert_eq!(pixel(&draw(&graph, "8:3").png, 10, 10), CYAN);
}

#[test]
fn a_paint_passing_through_is_a_plain_paint() {
    let graph = blended(|graph| {
        let layer = graph.nodes.get_mut(&guid("8:3")).unwrap();
        layer.blend_mode = None;
        layer.fills[0].blend_mode = Some("PASS_THROUGH".into());
    });
    let picture = draw(&graph, "8:1");
    assert_eq!(pixel(&picture.png, 10, 10), CYAN);
    assert!(picture.notes.is_empty(), "{:?}", picture.notes);
}

#[test]
fn the_linear_modes_are_drawn_as_normal_and_named() {
    for mode in ["LINEAR_BURN", "LINEAR_DODGE"] {
        let on_the_layer = blended(|graph| graph.nodes.get_mut(&guid("8:3")).unwrap().blend_mode = Some(mode.into()));
        let on_the_paint = blended(|graph| {
            let layer = graph.nodes.get_mut(&guid("8:3")).unwrap();
            layer.blend_mode = None;
            layer.fills[0].blend_mode = Some(mode.into());
        });
        for graph in [on_the_layer, on_the_paint] {
            let picture = draw(&graph, "8:1");
            assert_eq!(pixel(&picture.png, 10, 10), CYAN, "{mode}");
            assert!(picture.notes.iter().any(|n| n.contains(mode)), "{mode}: {:?}", picture.notes);
        }
    }
}

/// A 2 × 2 red PNG, and frame `9:1`, 20 wide, holding layer `9:2`, filled by it.
fn image_layer(kind: NodeType) -> SceneGraph {
    let mut red = Pixmap::new(2, 2).unwrap();
    red.fill(resvg::tiny_skia::Color::from_rgba8(255, 0, 0, 255));
    let mut graph = GraphBuilder::with_page()
        .add("9:1", "Frame", NodeType::Frame, Some(PAGE), |node| {
            node.width = 20.0;
            node.height = 20.0;
        })
        .add("9:2", "Layer", kind, Some("9:1"), |node| {
            node.width = 10.0;
            node.height = 10.0;
            let mut image = Fill::solid(rgb(0, 0, 0));
            image.fill_type = FillType::Image;
            image.color = None;
            image.image_hash = Some("red".into());
            node.fills = vec![image];
        })
        .build();
    graph.images = vec![("red".to_string(), red.encode_png().unwrap())];
    graph
}

#[test]
fn an_image_on_a_box_holds_its_shape_without_a_pattern() {
    let graph = image_layer(NodeType::Rectangle);
    let picture = draw(&graph, "9:1");
    assert_eq!(pixel(&picture.png, 5, 5), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 15, 15)[3], 0);
    let svg = crate::draw(&empty_file(), &graph, graph.node(guid("9:1")).unwrap(), 0.0, 20.0, 20, 20).svg;
    assert!(svg.contains("<image") && !svg.contains("<pattern"), "{}", &svg[..svg.len().min(300)]);
}

#[test]
fn an_image_on_a_vector_holds_each_of_its_outlines_without_a_pattern() {
    let mut graph = image_layer(NodeType::Vector);
    let layer = graph.nodes.get_mut(&guid("9:2")).unwrap();
    layer.fill_geometry = vec![
        model::VectorPath { d: "M0 0 L4 0 L4 4 L0 4 Z".into(), fill_rule: "nonzero".into() },
        model::VectorPath { d: "M6 6 L10 6 L10 10 L6 10 Z".into(), fill_rule: "nonzero".into() },
    ];
    let picture = draw(&graph, "9:1");
    assert_eq!(pixel(&picture.png, 2, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 8, 8), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 5, 5)[3], 0, "between the outlines");
    let svg = crate::draw(&empty_file(), &graph, graph.node(guid("9:1")).unwrap(), 0.0, 20.0, 20, 20).svg;
    assert!(!svg.contains("<pattern"));
    assert_eq!(svg.matches("<image").count(), 1, "one image for the layer");
}

#[test]
fn an_image_keeps_its_paints_opacity_and_blend() {
    let mut graph = image_layer(NodeType::Rectangle);
    graph.nodes.get_mut(&guid("9:2")).unwrap().fills[0].opacity = 0.5;
    let [r, _, _, a] = pixel(&draw(&graph, "9:1").png, 5, 5);
    assert_eq!(r, 255);
    assert!((120..=135).contains(&a), "half shown: {a}");

    let mut graph = image_layer(NodeType::Rectangle);
    let frame = graph.nodes.get_mut(&guid("9:1")).unwrap();
    frame.fills = vec![solid(rgb(255, 255, 0))];
    graph.nodes.get_mut(&guid("9:2")).unwrap().fills[0].blend_mode = Some("MULTIPLY".into());
    assert_eq!(pixel(&draw(&graph, "9:1").png, 5, 5), [255, 0, 0, 255], "red times yellow");
}

/// Layer `9:2` of frame `9:1`, filled with a gradient.
fn gradient_layer(setup: impl FnOnce(&mut model::SceneNode)) -> SceneGraph {
    let mut graph = image_layer(NodeType::Rectangle);
    let layer = graph.nodes.get_mut(&guid("9:2")).unwrap();
    let mut paint = Fill::solid(rgb(0, 0, 0));
    paint.fill_type = FillType::GradientLinear;
    paint.color = None;
    paint.gradient_transform = Some(Matrix::default());
    paint.gradient_stops = vec![
        GradientStop { position: 0.0, color: rgb(255, 0, 0) },
        GradientStop { position: 1.0, color: rgb(0, 0, 255) },
    ];
    layer.fills = vec![paint];
    setup(layer);
    graph
}

fn pdf_notes(graph: &SceneGraph) -> Vec<String> {
    let pdf = crate::pdf::pdf(&empty_file(), graph, &[guid("9:1")]).unwrap();
    pdf.pages.into_iter().flat_map(|(_, notes)| notes).collect()
}

fn names_apple(notes: &[String]) -> bool {
    notes.iter().any(|n| n.contains("Apple"))
}

#[test]
fn a_gradient_inside_a_group_is_named_in_a_pdf() {
    assert!(names_apple(&pdf_notes(&gradient_layer(|node| node.opacity = 0.5))), "opacity");
    assert!(names_apple(&pdf_notes(&gradient_layer(|node| node.blend_mode = Some("MULTIPLY".into())))), "a blend");
    assert!(names_apple(&pdf_notes(&gradient_layer(|node| node.fills[0].blend_mode = Some("MULTIPLY".into())))), "a paint's blend");
    let mut held = gradient_layer(|_| {});
    held.nodes.get_mut(&guid("9:1")).unwrap().blend_mode = Some("NORMAL".into());
    assert!(names_apple(&pdf_notes(&held)), "isolation");
    let mut masked = gradient_layer(|_| {});
    let frame = guid("9:1");
    let mut mask = model::SceneNode::new(guid("9:3"), "Mask", NodeType::Rectangle);
    mask.width = 20.0;
    mask.height = 20.0;
    mask.fills = vec![solid(testkit::scene::rgba(0, 0, 0, 0.5))];
    mask.is_mask = true;
    masked.insert(mask, Some(frame));
    masked.nodes.get_mut(&frame).unwrap().children.rotate_right(1);
    assert!(names_apple(&pdf_notes(&masked)), "a mask");
}

#[test]
fn a_gradient_on_its_own_or_only_clipped_says_nothing_of_it() {
    assert!(!names_apple(&pdf_notes(&gradient_layer(|_| {}))));
    let mut clipped = gradient_layer(|_| {});
    clipped.nodes.get_mut(&guid("9:1")).unwrap().clips_content = true;
    assert!(!names_apple(&pdf_notes(&clipped)));
}

#[test]
fn a_gradient_inside_a_group_says_nothing_of_it_in_a_png() {
    assert!(!names_apple(&draw(&gradient_layer(|node| node.opacity = 0.5), "9:1").notes));
}

/// The drawing of `masked`'s frame, for what it holds rather than how it looks.
fn drawing_of(graph: &SceneGraph) -> String {
    crate::draw(&empty_file(), graph, graph.node(guid("5:1")).unwrap(), 0.0, 20.0, 40, 20).svg
}

fn is_clip(graph: &SceneGraph) -> bool {
    let svg = drawing_of(graph);
    svg.contains("<clipPath") && !svg.contains("<mask")
}

fn is_mask(graph: &SceneGraph) -> bool {
    drawing_of(graph).contains("<mask")
}

#[test]
fn an_opaque_shape_masks_as_a_clip_and_shows_the_same() {
    let graph = masked("ALPHA", green()).build();
    assert!(is_clip(&graph));
    let picture = draw(&graph, "5:1");
    assert_eq!(pixel(&picture.png, 10, 12), [255, 0, 0, 255]);
    assert_eq!(pixel(&picture.png, 30, 12)[3], 0);
    assert_eq!(pixel(&picture.png, 30, 2), [0, 0, 255, 255], "the layer below is outside the clip");
}

#[test]
fn an_outline_mask_is_a_clip_whatever_its_paint() {
    assert!(is_clip(&masked("OUTLINE", half_clear()).build()));
}

#[test]
fn a_clip_cuts_where_a_vectors_outlines_are() {
    let mut graph = masked("ALPHA", green()).build();
    let mask = graph.nodes.get_mut(&guid("5:2")).unwrap();
    mask.node_type = NodeType::Vector;
    mask.x = 10.0;
    mask.fill_geometry = vec![square_path(4.0)];
    assert!(is_clip(&graph));
    let picture = draw(&graph, "5:1");
    assert_eq!(pixel(&picture.png, 12, 6), [255, 0, 0, 255], "inside the moved outline");
    assert_eq!(pixel(&picture.png, 2, 6)[3], 0, "where it would be unmoved");
    assert_eq!(pixel(&picture.png, 16, 12)[3], 0, "inside its box, outside its outline");
}

#[test]
fn a_mask_whose_paint_says_how_much_stays_a_mask() {
    assert!(is_mask(&masked("ALPHA", half_clear()).build()), "translucent");
    assert!(is_mask(&masked("LUMINANCE", green()).build()), "luminance");
    let mut gradient = Fill::solid(rgb(0, 0, 0));
    gradient.fill_type = FillType::GradientLinear;
    gradient.color = None;
    gradient.gradient_transform = Some(Matrix::default());
    gradient.gradient_stops = vec![GradientStop { position: 0.0, color: rgb(0, 0, 0) }];
    assert!(is_mask(&masked("ALPHA", gradient).build()), "a gradient");
    let mut faint = green();
    faint.opacity = 0.5;
    assert!(is_mask(&masked("ALPHA", faint).build()), "a paint's opacity");
}

#[test]
fn a_mask_drawn_with_more_than_its_shape_stays_a_mask() {
    let with = |setup: fn(&mut model::SceneNode)| {
        let mut graph = masked("ALPHA", green()).build();
        setup(graph.nodes.get_mut(&guid("5:2")).unwrap());
        graph
    };
    assert!(is_mask(&with(|mask| mask.opacity = 0.5)), "opacity");
    assert!(is_mask(&with(|mask| mask.effects = vec![effect(EffectType::LayerBlur, rgb(0, 0, 0), (0.0, 0.0), 2.0, 0.0)])), "an effect");
    assert!(is_mask(&with(|mask| mask.strokes = vec![solid(rgb(0, 0, 0))])), "a stroke");
    let holding = masked("ALPHA", green())
        .add("5:9", "Inside", NodeType::Rectangle, Some("5:2"), |node| {
            node.width = 2.0;
            node.height = 2.0;
        })
        .build();
    assert!(is_mask(&holding), "children");
}

#[test]
fn a_mask_an_instance_overrides_stays_a_mask_and_shows_the_override() {
    let mut graph = masked("ALPHA", green())
        .add("6:1", "Instance", NodeType::Instance, Some(PAGE), |node| {
            node.width = 40.0;
            node.height = 20.0;
            node.component_id = Some(guid("5:1"));
            node.symbol_overrides = vec![model::SymbolOverride {
                path: vec![guid("7:2")],
                fills: Some(vec![half_clear()]),
                ..model::SymbolOverride::default()
            }];
        })
        .build();
    graph.nodes.get_mut(&guid("5:1")).unwrap().node_type = NodeType::Component;
    let [r, _, _, a] = pixel(&draw(&graph, "6:1").png, 10, 12);
    assert_eq!(r, 255);
    assert!((120..=135).contains(&a), "half shown, by the override: {a}");
}

#[test]
fn a_gradient_under_a_plain_mask_says_nothing_of_apple() {
    let mut graph = gradient_layer(|_| {});
    let frame = guid("9:1");
    let mut mask = model::SceneNode::new(guid("9:3"), "Mask", NodeType::Rectangle);
    mask.width = 20.0;
    mask.height = 20.0;
    mask.fills = vec![solid(rgb(0, 0, 0))];
    mask.is_mask = true;
    graph.insert(mask, Some(frame));
    graph.nodes.get_mut(&frame).unwrap().children.rotate_right(1);
    assert!(!names_apple(&pdf_notes(&graph)));
}
