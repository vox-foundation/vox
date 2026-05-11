//! Smoke test: `vox ci pre-push --dry-run --quick` enumerates the expected steps
//! without executing them.

use std::process::Command;

#[test]
fn pre_push_dry_run_quick_lists_steps() {
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "pre-push", "--dry-run", "--quick"])
        .output()
        .expect("spawn vox");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for needle in ["cargo fmt", "ci line-endings", "ci ssot-drift"] {
        assert!(stdout.contains(needle), "missing `{needle}` in:\n{stdout}");
    }
}

#[test]
fn pre_push_dry_run_full_includes_nextest_ci_profile() {
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "pre-push", "--dry-run", "--full"])
        .output()
        .expect("spawn vox");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("nextest") && stdout.contains("profile ci"),
        "expected nextest + ci profile in:\n{stdout}"
    );
}
