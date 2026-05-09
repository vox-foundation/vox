//! Tests for the rename registry loader and alias resolution.
//!
//! Implementation lives in `crates/vox-compiler/src/parser/renames.rs` (added in Task 3).
//! This test file is written first (TDD) and will fail to compile until that module exists.

use vox_compiler::parser::renames::RenameRegistry;

#[test]
fn registry_loads_from_canonical_path() {
    let registry = RenameRegistry::load_canonical()
        .expect("should load contracts/naming/renames.v1.json");
    // Empty registry is valid (VUV-9 ships an empty registry; entries are added in later phases).
    assert_eq!(registry.entries().count(), 0);
}

#[test]
fn registry_rejects_duplicate_from_keys() {
    let json = r#"{
        "version": 1,
        "entries": [
          { "from": "Box", "to": "panel", "kind": "primitive", "since": "0.5.0" },
          { "from": "Box", "to": "container", "kind": "primitive", "since": "0.5.0" }
        ]
    }"#;
    let result = RenameRegistry::from_str(json);
    assert!(result.is_err(), "duplicate `from` keys must be rejected");
}

#[test]
fn registry_rejects_alias_chain() {
    // `from` cannot itself be a `to` of another entry — no chains, only direct mappings.
    let json = r#"{
        "version": 1,
        "entries": [
          { "from": "Box",       "to": "container", "kind": "primitive", "since": "0.5.0" },
          { "from": "container", "to": "panel",     "kind": "primitive", "since": "0.5.0" }
        ]
    }"#;
    let result = RenameRegistry::from_str(json);
    assert!(result.is_err(), "alias chains must be rejected");
}

#[test]
fn registry_accepts_empty_entries() {
    let json = r#"{ "version": 1, "entries": [] }"#;
    let result = RenameRegistry::from_str(json);
    assert!(result.is_ok(), "empty registry must be valid");
}

#[test]
fn registry_rejects_unsupported_version() {
    let json = r#"{ "version": 99, "entries": [] }"#;
    let result = RenameRegistry::from_str(json);
    assert!(result.is_err(), "version != 1 must be rejected");
}
