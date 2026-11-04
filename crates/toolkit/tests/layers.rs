//! The layers point inward. Every workspace
//! crate is read from its manifest: the domain and the application name no
//! I/O, JSON or transport crate and no adapter; a product names no other
//! product and no adapter; nothing inside names what is outside it.

use std::path::{Path, PathBuf};

/// Each crate's layer, by the name the workspace depends on it by.
const LAYERS: [(&str, Layer); 18] = [
    ("kiwi", Layer::ReadSide),
    ("schema", Layer::ReadSide),
    ("figfile", Layer::ReadSide),
    ("model", Layer::ReadSide),
    ("domain", Layer::Domain),
    ("app", Layer::App),
    ("design", Layer::Product),
    ("slides", Layer::Product),
    ("figjam", Layer::Product),
    ("sites", Layer::Product),
    ("edge", Layer::Adapter),
    ("disk", Layer::Adapter),
    ("memory", Layer::Adapter),
    ("render", Layer::Adapter),
    ("toolkit", Layer::Assembly),
    ("mcp", Layer::Driver),
    ("grafta", Layer::Driver),
    ("testkit", Layer::Test),
];
/// Crates that reach the world or speak JSON: no crate inside the adapters may
/// name one.
const OUTSIDE: [&str; 3] = ["serde_json", "rmcp", "tokio"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Layer {
    ReadSide,
    Domain,
    App,
    Product,
    Adapter,
    Assembly,
    Driver,
    Test,
}

fn layer(name: &str) -> Option<Layer> {
    LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

/// Why `from` may not depend on `on`, if it may not.
fn refused(from: Layer, on: &str) -> Option<String> {
    let inner = matches!(from, Layer::ReadSide | Layer::Domain | Layer::App | Layer::Product);
    if inner && OUTSIDE.contains(&on) {
        return Some(format!("{on} is I/O, JSON or transport"));
    }
    let to = layer(on)?;
    match (from, to) {
        (Layer::Product, Layer::Product) => Some("a product names no other product".into()),
        (_, Layer::Test) => None,
        (from, to) if inner && to > Layer::Product => Some(format!("{to:?} is outside {from:?}")),
        (from, to) if to > from && from != Layer::Test => Some(format!("{to:?} is outside {from:?}")),
        _ => None,
    }
}

/// The names in a manifest's `[dependencies]`: `name.workspace = true`,
/// `name = …` or `name = { … }`.
fn dependencies(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[dependencies]";
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            if let Some(name) = line.split(['.', '=', ' ']).next().filter(|n| !n.is_empty()) {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// The `[lib]` name, or the package name without `grafta-`.
fn lib_name(manifest: &str) -> String {
    let value = |section: &str, key: &str| {
        let mut inside = false;
        manifest.lines().map(str::trim).find_map(|line| {
            if line.starts_with('[') {
                inside = line == section;
                return None;
            }
            let (k, v) = line.split_once('=')?;
            (inside && k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    value("[lib]", "name").unwrap_or_else(|| value("[package]", "name").unwrap_or_default().trim_start_matches("grafta-").to_string())
}

fn manifests(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable crates directory").flatten() {
        let path = entry.path();
        if path.join("Cargo.toml").is_file() {
            out.push(path.join("Cargo.toml"));
        } else if path.is_dir() {
            manifests(&path, out);
        }
    }
}

#[test]
fn every_crate_points_inward() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    manifests(&crates, &mut found);
    let mut wrong = Vec::new();
    for path in &found {
        let text = std::fs::read_to_string(path).expect("readable manifest");
        let name = lib_name(&text);
        let from = layer(&name).unwrap_or_else(|| panic!("{name} ({}) has no layer in LAYERS", path.display()));
        for on in dependencies(&text) {
            if let Some(why) = refused(from, &on) {
                wrong.push(format!("{name} depends on {on}: {why}"));
            }
        }
    }
    assert!(found.len() >= 9, "{} manifests read", found.len());
    assert!(wrong.is_empty(), "dependencies pointing outward:\n{}", wrong.join("\n"));
}

#[test]
fn an_outward_dependency_is_refused() {
    let planted = "[package]\nname = \"grafta-domain\"\n\n[lib]\nname = \"domain\"\n\n[dependencies]\nmodel.workspace = true\nserde_json.workspace = true\nedge = { path = \"x\" }\n";
    let from = layer(&lib_name(planted)).unwrap();
    let refusals: Vec<String> = dependencies(planted).iter().filter_map(|on| refused(from, on)).collect();
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    assert!(refused(Layer::Product, "slides").is_some(), "a product on another");
    assert!(refused(Layer::App, "design").is_some(), "the application on a product");
    assert!(refused(Layer::Product, "app").is_none(), "a product on the application");
    assert!(refused(Layer::Assembly, "edge").is_none(), "the assembly on an adapter");
    assert!(refused(Layer::Driver, "toolkit").is_none(), "MCP on the assembly");
}
