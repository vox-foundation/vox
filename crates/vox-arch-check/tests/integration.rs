//! Smoke test: verify the arch-check binary exists and runs without panicking.

#[test]
fn arch_check_smoke_test() {
    let status = std::process::Command::new("cargo")
        .args(["run", "-p", "vox-arch-check", "--", "--warn-only"])
        .status()
        .expect("failed to run vox-arch-check");
    assert!(status.success(), "vox-arch-check --warn-only should exit 0");
}
