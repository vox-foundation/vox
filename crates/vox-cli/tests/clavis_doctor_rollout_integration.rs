use std::path::Path;
use std::process::Command;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root")
}

fn run_clavis_status_with_env(envs: &[(&str, &str)]) -> String {
    let bin = env!("CARGO_BIN_EXE_vox");
    let mut cmd = Command::new(bin);
    cmd.current_dir(workspace_root()).args([
        "clavis",
        "status",
        "--workflow",
        "chat",
        "--profile",
        "dev",
    ]);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output = cmd.output().expect("spawn vox clavis status");
    assert!(
        output.status.success(),
        "clavis status should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn clavis_status_prints_rollout_flags_line() {
    let stdout = run_clavis_status_with_env(&[]);
    assert!(
        stdout.contains("rollout_flags: lineage_persist="),
        "rollout flags line should be present"
    );
    assert!(
        stdout.contains("workflow_journal_codex_persist="),
        "rollout flags should include workflow journal codex field"
    );
    assert!(
        stdout.contains("db_circuit_breaker_env="),
        "rollout flags should include db circuit breaker field"
    );
}

#[test]
fn clavis_status_warns_when_workflow_journal_codex_is_off() {
    let stdout = run_clavis_status_with_env(&[("VOX_WORKFLOW_JOURNAL_CODEX_OFF", "1")]);
    assert!(
        stdout.contains("workflow_journal_codex_persist=false"),
        "rollout flags should reflect disabled codex journal persistence"
    );
    assert!(
        stdout.contains(
            "warning: VOX_WORKFLOW_JOURNAL_CODEX_OFF disables Codex workflow journal append"
        ),
        "expected durability warning when codex journal append is disabled"
    );
}

#[test]
fn clavis_status_warns_when_db_circuit_breaker_is_enabled() {
    let stdout = run_clavis_status_with_env(&[("VOX_DB_CIRCUIT_BREAKER", "1")]);
    assert!(
        stdout.contains("db_circuit_breaker_env=true"),
        "rollout flags should reflect db circuit breaker env toggle"
    );
    assert!(
        stdout.contains(
            "warning: VOX_DB_CIRCUIT_BREAKER may gate workflow durability writes under DB stress"
        ),
        "expected durability warning when db circuit breaker env is enabled"
    );
}
