//! Unit tests for the parts of the model that have no public entry point.
//! Everything reachable from outside the crate is tested in `../tests/`.

use super::*;
use types::style_to_weight;

#[test]
fn maps_font_styles_to_weights() {
    assert_eq!(style_to_weight("Regular"), 400);
    assert_eq!(style_to_weight("Semi Bold"), 600);
    assert_eq!(style_to_weight("SemiBold Italic"), 600);
    assert_eq!(style_to_weight("Extra Light"), 200);
    assert_eq!(style_to_weight("Bold"), 700);
    assert_eq!(style_to_weight("Black"), 900);
    assert_eq!(style_to_weight("Unknown"), 400);
}

#[test]
fn parses_variant_names() {
    let parsed = parse_variant_name("Size=Large, State=Hover");
    assert_eq!(parsed.get("Size").map(String::as_str), Some("Large"));
    assert_eq!(parsed.get("State").map(String::as_str), Some("Hover"));
    assert!(parse_variant_name("Button").is_empty());
}

#[test]
fn formats_corner_radii() {
    let uniform = CornerRadii { top_left: 4.0, top_right: 4.0, bottom_right: 4.0, bottom_left: 4.0 };
    assert_eq!(uniform.to_css(), "4px");
    let mixed = CornerRadii { top_left: 4.0, top_right: 0.0, bottom_right: 8.0, bottom_left: 0.0 };
    assert_eq!(mixed.to_css(), "4px 0px 8px 0px");
}
