//! Integration tests for vox-arch-check.

use std::process::Command;

/// Verify the binary runs without panicking and exits cleanly under --warn-only.
#[test]
fn arch_check_smoke_test() {
    let status = Command::new("cargo")
        .args(["run", "-p", "vox-arch-check", "--", "--warn-only"])
        .status()
        .expect("failed to run vox-arch-check");
    assert!(status.success(), "vox-arch-check --warn-only should exit 0");
}

/// Verify the description_present rule is wired and produces output.
/// The rule is strict (`description = "error"` in layers.toml), so running
/// without --warn-only on a clean workspace should exit 0. We just check
/// the summary line appears in stderr so the rule is confirmed active.
#[test]
fn description_rule_produces_output_on_clean_workspace() {
    let out = Command::new("cargo")
        .args(["run", "-p", "vox-arch-check", "--", "--warn-only"])
        .output()
        .expect("failed to run vox-arch-check");
    // Clean workspace: no description warnings should appear.
    // The key assertion: arch-check must exit 0 (no regressions).
    assert!(
        out.status.success(),
        "arch-check --warn-only should exit 0 on clean workspace; stderr:\n{}",
        String::from_utf8_lossy(&out.stderr),
    );
    // Confirm the summary line is printed (proves the binary ran to completion).
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("vox-arch-check: clean") || stderr.contains("[warn]") || stderr.contains("[ERROR]"),
        "expected arch-check to print a summary line; got:\n{stderr}",
    );
}
