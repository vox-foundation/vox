use std::path::{Path, PathBuf};

use super::{nested_cargo_target_dir, resolve_mens_gate_manifest_path};
use crate::commands::ci::constants::FEATURE_SETS;

#[test]
fn feature_sets_include_script_execution_lane() {
    assert!(
        FEATURE_SETS.contains(&"script-execution"),
        "CI feature matrix must compile the script-execution lane"
    );
    assert!(
        FEATURE_SETS.contains(&"script-execution,stub-check"),
        "CI feature matrix must include a mixed script-execution + stub-check build"
    );
}

#[test]
fn feature_sets_include_populi_oratio_lane() {
    assert!(
        FEATURE_SETS.contains(&"oratio"),
        "CI feature matrix must compile the oratio (Oratio STT) lane"
    );
}

#[test]
fn canonical_mens_gate_manifest_exists_in_repo() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let resolved = resolve_mens_gate_manifest_path(&root);
    assert!(
        resolved.ends_with(PathBuf::from("scripts/populi/gates.yaml")),
        "expected canonical mens gate manifest path to resolve first, got {}",
        resolved.display()
    );
    assert!(
        resolved.is_file(),
        "missing gate manifest: {}",
        resolved.display()
    );
}

#[test]
fn mens_gate_manifest_resolution_uses_legacy_fallback() {
    let td = tempfile::tempdir().expect("tempdir");
    let root = td.path();
    std::fs::create_dir_all(root.join("scripts/mens")).expect("mkdir scripts/mens");
    std::fs::write(root.join("scripts/mens/gates.yaml"), "profiles: {}\n")
        .expect("write legacy gates");
    let resolved = resolve_mens_gate_manifest_path(root);
    assert!(
        resolved.ends_with(PathBuf::from("scripts/mens/gates.yaml")),
        "expected legacy fallback, got {}",
        resolved.display()
    );
}

#[test]
fn nested_cargo_target_uses_nested_ci_suffix() {
    let td = tempfile::tempdir().expect("tempdir");
    let root = td.path();
    let nested = nested_cargo_target_dir(root);
    assert!(
        nested.ends_with(PathBuf::from("target/nested-ci")),
        "unexpected nested target path: {}",
        nested.display()
    );
}
