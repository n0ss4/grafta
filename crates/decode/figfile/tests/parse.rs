//! Parsing real `.fig` files. Skipped when no fixture is available — see
//! `fixtures/README.md`.

// A test spells the wire apart from `schema`, so a misspelt name fails it.
#![allow(clippy::disallowed_methods)]

use kiwi::Value;

#[test]
fn decodes_node_changes_from_every_available_fixture() {
    for path in testkit::fixture::all() {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let Some(fig) = testkit::fixture::parsed(name) else { continue };

        let changes = fig
            .node_changes
            .get("nodeChanges")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("{name}: no nodeChanges array"));

        assert!(!changes.is_empty(), "{name}: nodeChanges is empty");
        assert!(fig.version > 0, "{name}: no container version");
    }
}

#[test]
fn rejects_data_that_is_not_an_archive() {
    assert!(figfile::parse_fig(b"definitely not a zip").is_err());
}
