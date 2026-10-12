use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

const EXPECTED_VERSIONS: [(&str, &str); 5] = [
    ("candle-core", "0.10.2"),
    ("candle-nn", "0.10.2"),
    ("candle-transformers", "0.10.2"),
    ("peft-rs", "1.0.3"),
    ("qlora-rs", "1.0.5"),
];

#[test]
fn resolved_ml_dependency_versions_are_unified_and_pinned() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("vox-cli manifest should be two levels below the workspace root");
    let output = Command::new("cargo")
        .current_dir(workspace_root)
        .args(["metadata", "--format-version", "1", "--locked"])
        .output()
        .expect("spawn brokered cargo metadata --locked");

    assert!(
        output.status.success(),
        "cargo metadata --format-version 1 --locked failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("parse cargo metadata JSON");
    let packages = metadata["packages"]
        .as_array()
        .expect("cargo metadata packages must be an array");
    let mut resolved = EXPECTED_VERSIONS
        .iter()
        .map(|(name, _)| ((*name).to_owned(), BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();

    for package in packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        let Some(versions) = resolved.get_mut(name) else {
            continue;
        };
        let version = package["version"]
            .as_str()
            .unwrap_or_else(|| panic!("cargo metadata package {name} has no version"));
        versions.insert(version.to_owned());
    }

    for (name, expected) in EXPECTED_VERSIONS {
        let versions = &resolved[name];
        assert_eq!(
            versions,
            &BTreeSet::from([expected.to_owned()]),
            "{name} must resolve exactly once at the ADR-034 verified version; update this contract and ADR evidence together for an intentional upgrade"
        );
    }
}
