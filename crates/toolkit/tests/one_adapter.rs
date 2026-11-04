//! A tool fails with a `DomainError`; the words a model reads are written by
//! its `From` alone. Outside `error` and tests, no `ToolError` is built.

use std::path::{Path, PathBuf};

const PATTERNS: [&str; 4] =
    ["ToolError::BadArgument(", "ToolError::NotFound(", "ToolError::Refused(", "ToolError::Failed("];
const ADAPTER: &str = "error";
const TEST_FILE: &str = "tests.rs";
const TEST_DIR: &str = "tests";

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != TEST_DIR && n != ADAPTER) {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != TEST_FILE) {
            out.push(path);
        }
    }
}

#[test]
fn no_tool_error_is_written_outside_the_adapter() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&src, &mut files);
    let found: Vec<String> = files
        .iter()
        .filter(|path| path.file_stem().is_some_and(|stem| stem != ADAPTER))
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("readable source");
            text.lines()
                .enumerate()
                .filter(|(_, line)| PATTERNS.iter().any(|pattern| line.contains(pattern)))
                .map(|(number, line)| format!("{}:{}: {}", path.display(), number + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(found.is_empty(), "error text written outside the adapter:\n{}", found.join("\n"));
}
