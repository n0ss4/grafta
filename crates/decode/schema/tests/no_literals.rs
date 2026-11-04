//! What `clippy::disallowed_methods` cannot see: a field name or an enum value
//! of the format spelt as a string, in an object built or a value read.
//! Outside `schema`, `kiwi` and tests, there is none.

use std::path::{Path, PathBuf};

const PATTERNS: [&str; 6] =
    ["Fields::from([(\"", ".to_string(), Raw::", ".to_string(), Value::", "\"sessionID\"", "Raw::Enum(\"", "Value::Enum(\""];

/// A value read from the format, then compared with a literal on the same line.
const READS: [&str; 3] = [".str(Field::", "Raw::as_str)", "Value::as_str)"];

/// Whether `line` compares a read with an enum value spelt out, as `== Some("SOLID")` does.
fn compares_a_read(line: &str) -> bool {
    READS.iter().any(|read| line.contains(read))
        && line.match_indices('"').step_by(2).any(|(at, _)| {
            let literal: String = line[at + 1..].chars().take_while(|c| *c != '"').collect();
            literal.len() > 1 && literal.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        })
}
const EXEMPT: [&[&str]; 3] = [&["decode", "schema"], &["decode", "kiwi"], &["testkit"]];
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

#[test]
fn nothing_of_the_format_is_spelt_outside_schema() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("the crates directory");
    let mut files = Vec::new();
    sources(&crates, &mut files);
    let found: Vec<String> = files
        .iter()
        .filter(|path| {
            let within = path.strip_prefix(&crates).expect("found under the crates directory");
            !EXEMPT.iter().any(|exempt| within.starts_with(exempt.iter().collect::<PathBuf>()))
        })
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("readable source");
            text.lines()
                .enumerate()
                .filter(|(_, line)| PATTERNS.iter().any(|pattern| line.contains(pattern)) || compares_a_read(line))
                .map(|(number, line)| format!("{}:{}: {}", path.display(), number + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(found.is_empty(), "the format spelt as strings:\n{}", found.join("\n"));
}
