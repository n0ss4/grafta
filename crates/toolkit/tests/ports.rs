//! The toolkit reaches the world through its ports .
//! In no crate inside the adapters is a file read or written, a document
//! encoded or decoded or anything drawn but through them.

use std::path::{Path, PathBuf};

const CALLS: [&str; 17] = [
    "std::fs",
    "fs::",
    "File::",
    "OpenOptions",
    ".exists()",
    ".is_file()",
    ".canonicalize()",
    "figfile::parse_fig",
    "figfile::write_fig",
    "figfile::compare",
    "render::render(",
    "render::contrast::contrast(",
    "render::pdf::pdf(",
    "render::blank_png(",
    "render::measure_text(",
    "render::text::faces(",
    "render::text::fonts(",
];
/// The crates inside the adapters, by their directory under `crates/`.
const INSIDE: [&str; 7] = ["core/domain", "core/app", "products/design", "products/slides", "products/figjam", "products/sites", "toolkit"];
const TEST_FILE: &str = "tests.rs";
const TEST_DIR: &str = "tests";

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != TEST_DIR) {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != TEST_FILE) {
            out.push(path);
        }
    }
}

/// Whether `line` calls `call`, as a whole name: `FigFile::` is not `File::`.
fn calls(line: &str, call: &str) -> bool {
    line.match_indices(call).any(|(at, _)| {
        call.starts_with('.') || !line[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_')
    })
}

#[test]
fn the_toolkit_reaches_the_world_through_its_ports() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for dir in INSIDE {
        sources(&crates.join(dir).join("src"), &mut files);
    }
    assert!(files.len() > 50, "{} files read", files.len());
    let found: Vec<String> = files
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("readable source");
            text.lines()
                .enumerate()
                .filter(|(_, line)| !line.trim_start().starts_with("//"))
                .filter(|(_, line)| CALLS.iter().any(|call| calls(line, call)))
                .map(|(number, line)| format!("{}:{}: {}", path.display(), number + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(found.is_empty(), "I/O outside the ports' adapters:\n{}", found.join("\n"));
}

#[test]
fn a_call_is_matched_by_its_whole_name() {
    assert!(calls("let f = File::create(p)?;", "File::"));
    assert!(!calls("FigFile::blobs", "File::"));
    assert!(calls("std::fs::read(p)", "fs::"));
    assert!(calls("path.exists()", ".exists()"));
}
