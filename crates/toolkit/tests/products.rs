//! A product is told apart by its profile alone, and a node type of one
//! product by its behaviours. Outside the
//! products' crates and tests, no code asks which product a file is from, names a node type
//! one product has, or reaches into a product's own code.

use std::path::{Path, PathBuf};

const PRODUCT: [&str; 2] = ["Product::", "figfile::Product"];
/// The node types one product has, and a product's own code.
const ONE_PRODUCT: [&str; 13] = [
    "NodeType::Slide",
    "NodeType::SlideGrid",
    "NodeType::SlideRow",
    "NodeType::InteractiveSlideElement",
    "NodeType::Sticky",
    "NodeType::ShapeWithText",
    "NodeType::Connector",
    "NodeType::Table",
    "NodeType::Stamp",
    "NodeType::Widget",
    "NodeType::ResponsiveSet",
    "slides::deck::",
    "figjam::board::",
];
/// The plugin API's name for every node type, and back: vocabulary, not a
/// branch.
const VOCABULARY: [&str; 2] = ["pub fn type_name(", "pub fn parse_type("];
/// The shared crates, by their directory under `crates/`: every one but the
/// products'.
const SHARED: [&str; 4] = ["core/domain", "core/app", "adapters/edge", "toolkit"];
/// The file in a shared crate that lists the products or defines their
/// traits.
const PRODUCTS: &str = "product";
const TEST_FILE: &str = "tests.rs";
const TEST_DIR: &str = "tests";

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != TEST_DIR && n != PRODUCTS) {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != TEST_FILE) {
            out.push(path);
        }
    }
}

/// Whether `line` names `name` whole: `FigProduct::` is not `Product::`, nor
/// `NodeType::SlideRow` `NodeType::Slide`.
fn names(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(at, _)| {
        let before = !line[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
        let after = name.ends_with("::") || !line[at + name.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
        before && after
    })
}

/// Every non-comment line of the shared source naming one of `names`, outside
/// the functions `VOCABULARY` opens, as (file, line).
fn found(names_one_of: &[&str]) -> Vec<(PathBuf, String)> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for dir in SHARED {
        sources(&crates.join(dir).join("src"), &mut files);
    }
    files
        .iter()
        .filter(|path| path.file_stem().is_some_and(|stem| stem != PRODUCTS))
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("readable source");
            let mut in_vocabulary = false;
            text.lines()
                .enumerate()
                .filter(|(_, line)| {
                    in_vocabulary = (in_vocabulary || VOCABULARY.iter().any(|f| line.starts_with(f))) && *line != "}";
                    !in_vocabulary
                })
                .filter(|(_, line)| !line.trim_start().starts_with("//"))
                .filter(|(_, line)| names_one_of.iter().any(|name| names(line, name)))
                .map(|(number, line)| (path.clone(), format!("{}:{}: {}", path.display(), number + 1, line.trim())))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn lines(found: &[(PathBuf, String)]) -> String {
    found.iter().map(|(_, line)| line.as_str()).collect::<Vec<_>>().join("\n")
}

#[test]
fn no_shared_code_asks_which_product_a_file_is_from() {
    let found = found(&PRODUCT);
    assert!(found.is_empty(), "a product branched on outside its profile:\n{}", lines(&found));
}

#[test]
fn no_shared_code_names_a_node_type_of_one_product() {
    let found = found(&ONE_PRODUCT);
    assert!(found.is_empty(), "a node type of one product branched on outside its behaviours:\n{}", lines(&found));
}

#[test]
fn a_name_is_matched_whole() {
    assert!(names("match figfile::Product::Slides {", "Product::"));
    assert!(!names("FigProduct::new()", "Product::"));
    assert!(names("n.node_type == NodeType::Slide)", "NodeType::Slide"));
    assert!(!names("NodeType::SlideRow =>", "NodeType::Slide"));
}
