//! Access to real `.fig` files.
//!
//! Fixtures are large binaries, so the repository does not carry them. A test
//! that needs one asks for it and skips itself when it is missing, which keeps
//! `cargo test` green on a fresh clone while still exercising real files
//! wherever they are available.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// Directory fixtures are read from: `$GRAFTA_FIXTURES` if set, otherwise
/// `fixtures/` at the workspace root.
#[must_use]
pub fn dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GRAFTA_FIXTURES") {
        return PathBuf::from(dir);
    }
    // CARGO_MANIFEST_DIR is `<workspace>/crates/testkit`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// Raw bytes of a fixture, or `None` when it is absent or is an unfetched Git
/// LFS pointer rather than the file itself.
#[must_use]
pub fn bytes(name: &str) -> Option<Vec<u8>> {
    let data = std::fs::read(dir().join(name)).ok()?;
    if data.starts_with(b"version https://git-lfs") {
        return None;
    }
    Some(data)
}

/// A parsed fixture, or `None` when the fixture is unavailable.
///
/// ```ignore
/// let Some(fig) = testkit::fixture::parsed("design.fig") else { return };
/// ```
#[must_use]
pub fn parsed(name: &str) -> Option<figfile::FigFile> {
    let data = bytes(name)?;
    Some(figfile::parse_fig(&data).expect("fixture is a readable .fig"))
}

/// A fixture's scene graph, or `None` when the fixture is unavailable.
#[must_use]
pub fn graph(name: &str) -> Option<(figfile::FigFile, model::SceneGraph)> {
    let fig = parsed(name)?;
    let graph = model::build_scene_graph(&fig).expect("fixture builds a scene graph");
    Some((fig, graph))
}

/// A fixture's scene graph, parsed once and shared for the life of the test
/// binary.
///
/// [`graph`] parses on every call, which is the honest thing for a function
/// that hands back ownership and the wrong thing for a suite. `nuxtui.fig` is
/// 313,083 nodes and takes about a minute and a half to build a scene graph
/// from, so sixteen tests each asking for their own copy is twenty minutes
/// spent doing one thing sixteen times.
///
/// Tests that need to own or mutate a graph want [`graph`]. Tests that only
/// read one — which is nearly all of them — want this.
#[must_use]
pub fn shared(name: &str) -> Option<&'static (figfile::FigFile, model::SceneGraph)> {
    type Cache = HashMap<String, Option<&'static (figfile::FigFile, model::SceneGraph)>>;
    static PARSED: OnceLock<Mutex<Cache>> = OnceLock::new();

    // Held across the parse, so two tests asking for the same file at once do
    // the work once rather than twice.
    let mut cache = PARSED
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    *cache.entry(name.to_string()).or_insert_with(|| Some(&*Box::leak(Box::new(graph(name)?))))
}

/// Every `.fig` under [`dir`], for tests that should run against whatever the
/// developer has to hand.
#[must_use]
pub fn all() -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir()) else { return Vec::new() };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "fig"))
        .collect();
    paths.sort();
    paths
}
