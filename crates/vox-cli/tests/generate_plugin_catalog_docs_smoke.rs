//! Smoke test that the new CLI command can be invoked and writes both
//! generated docs to disk.

use std::process::Command;

#[test]
fn generates_both_docs() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cat_path = tmp.path().join("plugin-catalog.generated.md");
    let bun_path = tmp.path().join("distribution-bundles.generated.md");

    let status = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args([
            "ci",
            "generate-plugin-catalog-docs",
            "--catalog-out",
            cat_path.to_str().unwrap(),
            "--bundles-out",
            bun_path.to_str().unwrap(),
        ])
        .status()
        .expect("vox should run");

    assert!(status.success(), "command should exit 0");
    let cat = std::fs::read_to_string(&cat_path).expect("catalog file should exist");
    let bun = std::fs::read_to_string(&bun_path).expect("bundles file should exist");
    assert!(cat.contains("Plugin Catalog (Generated)"));
    assert!(bun.contains("Distribution Bundles (Generated)"));
}
