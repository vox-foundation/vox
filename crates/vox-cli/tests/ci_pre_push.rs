//! Smoke test: `vox ci pre-push --dry-run --quick` enumerates the expected steps
//! without executing them.

use std::process::Command;

use tempfile::tempdir;

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
    for needle in ["vox-doc-pipeline", "doctest-md", "vox-drift-check"] {
        assert!(
            stdout.contains(needle),
            "quick mode must still list doc/drift steps; missing `{needle}` in:\n{stdout}"
        );
    }
}

#[test]
fn pre_push_dry_run_quick_writes_report_json() {
    let dir = tempdir().expect("tempdir");
    let report = dir.path().join("pre-push-report.json");
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args([
            "ci",
            "pre-push",
            "--dry-run",
            "--quick",
            "--report-json",
            report.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("spawn vox");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let raw = std::fs::read_to_string(&report).expect("read report");
    assert!(
        raw.contains("\"schema_version\": 1"),
        "report missing schema_version:\n{raw}"
    );
    assert!(
        raw.contains("\"dry_run\": true"),
        "report missing dry_run:\n{raw}"
    );
    assert!(
        raw.contains("\"elapsed_ms\": null") || raw.contains("\"elapsed_ms\":null"),
        "dry-run steps should have null elapsed_ms:\n{raw}"
    );
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
        stdout.contains("DRY-RUN: cargo nextest run --workspace --profile ci --no-fail-fast"),
        "expected nextest label in:\n{stdout}"
    );
}

#[test]
fn pre_push_dry_run_act_lists_workflows() {
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "pre-push", "--dry-run", "--quick", "--act"])
        .output()
        .expect("spawn vox");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    for workflow in [
        ".github/workflows/docs-quality.yml",
        ".github/workflows/link_checker.yml",
        ".github/workflows/ts-emit-noemit.yml",
    ] {
        assert!(
            stdout.contains(&format!("==> act: {workflow}")),
            "missing act workflow label `{workflow}` in:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("DRY-RUN:") && stdout.contains("push --workflows"),
        "expected dry-run act command output in:\n{stdout}"
    );
}
