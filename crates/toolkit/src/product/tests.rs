use super::PRODUCTS;

fn of(product: figfile::Product) -> &'static dyn super::ProductProfile {
    crate::product::all().of(product)
}

fn for_extension(extension: &str) -> Option<&'static dyn super::ProductProfile> {
    crate::product::all().for_extension(extension)
}

#[test]
fn every_product_has_one_profile() {
    for product in figfile::Product::ALL {
        assert_eq!(PRODUCTS.iter().filter(|p| p.product() == product).count(), 1, "{product:?}");
        assert_eq!(of(product).product(), product);
    }
    assert_eq!(crate::product::all().first().product(), figfile::Product::Design, "Design first");
}

#[test]
fn names_and_keys_are_as_a_person_and_summarize_read_them() {
    let named: Vec<(&str, &str)> = PRODUCTS.iter().map(|p| (p.name(), p.key())).collect();
    assert_eq!(named, [("Figma Design", "design"), ("Figma Slides", "slides"), ("FigJam", "whiteboard"), ("Figma Sites", "site")]);
}

#[test]
fn a_product_is_found_by_its_extension() {
    assert_eq!(for_extension("deck").map(|p| p.name()), Some("Figma Slides"));
    assert_eq!(for_extension("JAM").map(|p| p.name()), Some("FigJam"));
    assert_eq!(for_extension("fig").map(|p| p.name()), Some("Figma Design"));
    assert!(for_extension("png").is_none());
}

#[test]
fn only_a_deck_a_board_and_a_site_add_to_a_summary() {
    let graph = model::SceneGraph::default();
    let added: Vec<Option<&str>> = PRODUCTS.iter().map(|p| p.summary(&graph).map(|(key, _)| key)).collect();
    assert_eq!(added, [None, Some("slides"), Some("board"), Some("site_pages")]);
}
