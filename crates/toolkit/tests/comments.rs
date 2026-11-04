//! A comment says why, never what the code already says, and cites no
//! feature number: history lives in git. Every crate's non-test source is read.

use std::path::{Path, PathBuf};

const TEST_FILE: &str = "tests.rs";
const TEST_DIR: &str = "tests";

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable directory").flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != TEST_DIR && n != "target") {
                sources(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().is_some_and(|n| n != TEST_FILE) {
            out.push(path);
        }
    }
}

/// Whether `text` cites a feature number: `0` and two digits, the second
/// from 0 to 6, standing alone (`057`, `(059)`, `049's`), not inside a longer
/// number, a count, a decimal or a word.
fn cites_a_feature(text: &str) -> bool {
    let bytes = text.as_bytes();
    let apart = |at: Option<&u8>| at.is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'.' || *b == b'_' || *b == b'#'));
    (0..bytes.len().saturating_sub(2)).any(|at| {
        bytes[at] == b'0'
            && (b'0'..=b'6').contains(&bytes[at + 1])
            && bytes[at + 2].is_ascii_digit()
            && apart(at.checked_sub(1).and_then(|before| bytes.get(before)))
            && apart(bytes.get(at + 3))
            // `12,064` is a count, its comma a thousands separator.
            && !(at >= 2 && bytes[at - 1] == b',' && bytes[at - 2].is_ascii_digit())
    })
}

fn found() -> Vec<String> {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    sources(&crates, &mut files);
    files
        .iter()
        .filter(|path| path.components().any(|c| c.as_os_str() == "src"))
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("readable source");
            text.lines()
                .enumerate()
                .filter(|(_, line)| line.trim_start().starts_with("//") && cites_a_feature(line))
                .map(|(number, line)| format!("{}:{}: {}", path.display(), number + 1, line.trim()))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn no_comment_cites_a_feature_number() {
    let found = found();
    assert!(found.is_empty(), "{} comments cite a feature number:\n{}", found.len(), found.join("\n"));
}

#[test]
fn a_feature_number_is_told_from_other_numbers() {
    assert!(cites_a_feature("as 057 measured"));
    assert!(cites_a_feature("feature 024)"));
    assert!(cites_a_feature("(059)"));
    assert!(cites_a_feature("049's rung"));
    assert!(!cites_a_feature("0.5 opacity"));
    assert!(!cites_a_feature("#000000"));
    assert!(!cites_a_feature("12,064 measured"));
    assert!(!cites_a_feature("0x0100"));
    assert!(!cites_a_feature("version 1.0.150"));
    assert!(!cites_a_feature("rung u1"));
}
