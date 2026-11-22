//! The layout pass, against positions computed by hand for each case.

use serde_json::{json, Value};

use crate::tests::{call, kit};
use crate::Toolkit;

fn frame(kit: &mut Toolkit, handle: &str, parent: &str, w: f32, h: f32, props: Value) -> String {
    let out = call(kit, "create_frame", json!({ "handle": handle, "parent": parent, "name": "f", "width": w, "height": h })).unwrap();
    let id = out["id"].as_str().unwrap().to_string();
    if props.as_object().is_some_and(|p| !p.is_empty()) {
        call(kit, "set_properties", json!({ "handle": handle, "id": id, "properties": props })).unwrap();
    }
    id
}

fn geometry(kit: &mut Toolkit, handle: &str, id: &str) -> (f64, f64, f64, f64) {
    let n = call(kit, "read_node", json!({ "handle": handle, "id": id, "depth": 0 })).unwrap();
    let f = |k: &str| n[k].as_f64().unwrap();
    (f("x"), f("y"), f("width"), f("height"))
}

#[test]
fn a_vertical_stack_places_children_with_spacing_and_padding() {
    let (mut kit, h) = kit();
    let a1 = frame(&mut kit, &h, "1:1", 232.0, 300.0, json!({ "layoutMode": "VERTICAL", "itemSpacing": 12, "padding": 16, "primaryAxisSizingMode": "FIXED" }));
    let bars: Vec<String> = (0..3).map(|_| frame(&mut kit, &h, &a1, 200.0, 40.0, json!({}))).collect();
    assert_eq!(geometry(&mut kit, &h, &bars[0]), (16.0, 16.0, 200.0, 40.0));
    assert_eq!(geometry(&mut kit, &h, &bars[1]), (16.0, 68.0, 200.0, 40.0));
    assert_eq!(geometry(&mut kit, &h, &bars[2]), (16.0, 120.0, 200.0, 40.0));
    assert_eq!(geometry(&mut kit, &h, &a1).3, 300.0, "a fixed frame keeps its height");
}

#[test]
fn a_hugging_frame_takes_the_size_of_its_content() {
    let (mut kit, h) = kit();
    let a2 = frame(&mut kit, &h, "1:1", 232.0, 600.0, json!({ "layoutMode": "VERTICAL", "itemSpacing": 12, "padding": 16, "primaryAxisSizingMode": "AUTO" }));
    for _ in 0..3 {
        frame(&mut kit, &h, &a2, 200.0, 40.0, json!({}));
    }
    // 16 + 40 + 12 + 40 + 12 + 40 + 16
    assert_eq!(geometry(&mut kit, &h, &a2).3, 176.0);
}

#[test]
fn space_between_spreads_children_and_center_centres_them() {
    let (mut kit, h) = kit();
    let a3 = frame(&mut kit, &h, "1:1", 600.0, 120.0, json!({ "layoutMode": "HORIZONTAL", "padding": 16, "primaryAxisAlignItems": "SPACE_BETWEEN", "counterAxisAlignItems": "CENTER", "primaryAxisSizingMode": "FIXED" }));
    let a = frame(&mut kit, &h, &a3, 80.0, 40.0, json!({}));
    let b = frame(&mut kit, &h, &a3, 80.0, 80.0, json!({}));
    let c = frame(&mut kit, &h, &a3, 80.0, 60.0, json!({}));
    // Room 568, bars 240, so 164 between them. Counter room 88.
    assert_eq!(geometry(&mut kit, &h, &a), (16.0, 40.0, 80.0, 40.0));
    assert_eq!(geometry(&mut kit, &h, &b), (260.0, 20.0, 80.0, 80.0));
    assert_eq!(geometry(&mut kit, &h, &c), (504.0, 30.0, 80.0, 60.0));
}

#[test]
fn a_growing_child_fills_what_the_others_leave() {
    let (mut kit, h) = kit();
    let a4 = frame(&mut kit, &h, "1:1", 600.0, 80.0, json!({ "layoutMode": "HORIZONTAL", "itemSpacing": 12, "padding": 16, "primaryAxisSizingMode": "FIXED" }));
    frame(&mut kit, &h, &a4, 80.0, 48.0, json!({}));
    let grows = frame(&mut kit, &h, &a4, 10.0, 48.0, json!({ "layoutGrow": 1 }));
    let last = frame(&mut kit, &h, &a4, 80.0, 48.0, json!({}));
    // Room 568 − 160 − 24 = 384.
    assert_eq!(geometry(&mut kit, &h, &grows), (108.0, 16.0, 384.0, 48.0));
    assert_eq!(geometry(&mut kit, &h, &last), (504.0, 16.0, 80.0, 48.0));
}

#[test]
fn a_stretching_child_fills_the_cross_axis_and_hidden_ones_are_skipped() {
    let (mut kit, h) = kit();
    let col = frame(&mut kit, &h, "1:1", 300.0, 100.0, json!({ "layoutMode": "VERTICAL", "padding": 20, "itemSpacing": 10 }));
    let hidden = frame(&mut kit, &h, &col, 50.0, 50.0, json!({ "visible": false }));
    let stretched = frame(&mut kit, &h, &col, 10.0, 30.0, json!({ "layoutAlign": "STRETCH" }));
    assert_eq!(geometry(&mut kit, &h, &stretched), (20.0, 20.0, 260.0, 30.0));
    assert_eq!(geometry(&mut kit, &h, &col).3, 70.0, "the hidden child takes no room");
    // It was laid out while visible, then hidden: it stays where that put it.
    assert_eq!(geometry(&mut kit, &h, &hidden).0, 20.0, "a hidden child is left where it was");
}

#[test]
fn a_nested_hugging_frame_is_sized_before_its_parent_uses_it() {
    let (mut kit, h) = kit();
    let outer = frame(&mut kit, &h, "1:1", 10.0, 10.0, json!({ "layoutMode": "VERTICAL", "padding": 10, "counterAxisSizingMode": "AUTO" }));
    let inner = frame(&mut kit, &h, &outer, 10.0, 10.0, json!({ "layoutMode": "HORIZONTAL", "itemSpacing": 5, "counterAxisSizingMode": "AUTO" }));
    frame(&mut kit, &h, &inner, 40.0, 20.0, json!({}));
    frame(&mut kit, &h, &inner, 40.0, 30.0, json!({}));
    assert_eq!(geometry(&mut kit, &h, &inner), (10.0, 10.0, 85.0, 30.0));
    assert_eq!(geometry(&mut kit, &h, &outer).2, 105.0);
    assert_eq!(geometry(&mut kit, &h, &outer).3, 50.0);
}

/// A text that grows in a file's own hugging frames grows each of them, up to
/// the outermost, as Figma lays them out: not only the frame holding it (row
/// 116: `order.fig`'s Card kept its height when its texts grew).
#[test]
fn a_grown_text_grows_every_hugging_frame_around_it() {
    let (mut kit, h) = kit();
    let hug = json!({ "layoutMode": "VERTICAL", "padding": 10, "primaryAxisSizingMode": "AUTO" });
    let outer = frame(&mut kit, &h, "1:1", 200.0, 10.0, hug.clone());
    let inner = frame(&mut kit, &h, &outer, 180.0, 10.0, hug);
    let text = call(&mut kit, "create_text", json!({ "handle": h, "parent": inner, "characters": "Grows", "fontSize": 12 })).unwrap()["id"].as_str().unwrap().to_string();
    // As in a file opened again: nothing in it is grafta's own to lay out.
    let session = kit.session_mut(&h).unwrap();
    session.created.clear();
    session.laid_out.clear();
    call(&mut kit, "set_properties", json!({ "handle": h, "id": text, "properties": { "fontSize": 40 } })).unwrap();
    let (_, _, _, text_h) = geometry(&mut kit, &h, &text);
    let (_, _, _, inner_h) = geometry(&mut kit, &h, &inner);
    let (_, _, _, outer_h) = geometry(&mut kit, &h, &outer);
    assert!(text_h > 40.0, "the text grew: {text_h}");
    assert!((inner_h - (text_h + 20.0)).abs() < 0.01, "inner hugs the text: {inner_h} for {text_h}");
    assert!((outer_h - (inner_h + 20.0)).abs() < 0.01, "outer hugs inner: {outer_h} for {inner_h}");
}
