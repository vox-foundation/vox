//! Smoke tests: `vox ci pre-push --dry-run` profiles and `--report-json`.

use std::process::Command;

use tempfile::tempdir;

#[test]
fn pre_push_dry_run_quick_lists_fast_steps() {
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
    assert!(
        stdout.contains("vox-doc-pipeline"),
        "expected scoped doc lint step in:\n{stdout}"
    );
    assert!(
        stdout.contains("doctest-md"),
        "expected scoped doctest step in:\n{stdout}"
    );
    assert!(
        stdout.contains("vox-drift-check"),
        "missing drift-check in:\n{stdout}"
    );
    assert!(
        !stdout.contains("cargo clippy"),
        "fast profile must not run workspace clippy; got:\n{stdout}"
    );
}

#[test]
fn pre_push_dry_run_default_is_fast_profile() {
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "pre-push", "--dry-run"])
        .output()
        .expect("spawn vox");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("cargo clippy"),
        "default must be fast profile without clippy; got:\n{stdout}"
    );
}

#[test]
fn pre_push_dry_run_complete_includes_clippy_not_nextest() {
    let out = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(["ci", "pre-push", "--dry-run", "--complete"])
        .output()
        .expect("spawn vox");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("cargo clippy"),
        "complete profile must list clippy; got:\n{stdout}"
    );
    assert!(
        stdout.contains("doc-inventory"),
        "complete profile must list doc-inventory; got:\n{stdout}"
    );
    assert!(
        !stdout.contains("cargo nextest"),
        "`--complete` alone must not run nextest; got:\n{stdout}"
    );
}

#[test]
fn pre_push_dry_run_quick_writes_report_json_v2() {
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
        raw.contains("\"schema_version\": 2"),
        "report missing schema_version 2:\n{raw}"
    );
    assert!(
        raw.contains("\"profile\": \"fast\""),
        "report missing profile fast:\n{raw}"
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
    assert!(
        stdout.contains("cargo clippy"),
        "`--full` implies complete static checks; missing clippy in:\n{stdout}"
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
