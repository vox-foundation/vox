//! Smoke test: `vox ci pre-push --dry-run --quick` enumerates the expected steps
//! without executing them.

use std::process::Command;

#[test]
fn pre_push_dry_run_quick_lists_steps() {
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "pre-push", "--dry-run", "--quick"])
        .output()
        .expect("spawn vox");
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    for needle in ["cargo fmt", "ci line-endings", "ci ssot-drift"] {
        assert!(stdout.contains(needle), "missing `{needle}` in:\n{stdout}");
    }
}
