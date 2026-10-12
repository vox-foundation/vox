use std::process::Command;

#[test]
fn row_serde_lint_passes_on_workspace() {
    let vox = env!("CARGO_BIN_EXE_vox");
    let status = Command::new(vox)
        .args(["ci", "row-serde-lint"])
        .status()
        .expect("failed to spawn vox");
    assert!(status.success(), "row-serde-lint should exit 0");
}
