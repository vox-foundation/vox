//! End-to-end replay round-trip on a trivial `MainEntity`.

use std::fs;
use std::time::Duration;
use vox_replay_runner::{run_replay, ReplayOutcome};
use vox_scientia::ro_crate::MainEntity;

/// Build a trivial `MainEntity` whose entry-point writes "ok" to `out.txt`
/// using each platform's `echo` builtin. The expected hash hardcodes the
/// platform-specific byte sequence (POSIX adds `\n`, Windows adds `\r\n`).
fn trivial_main_entity_writing_ok() -> MainEntity {
    let expected_bytes: &[u8] = if cfg!(windows) { b"ok \r\n" } else { b"ok\n" };
    let expected_hex = hex::encode(vox_crypto::compliance_hash(expected_bytes));
    // `cmd /C echo ok > out.txt` writes `ok \r\n` (cmd echo trims one
    // space, leaving `ok ` plus CRLF). `sh -c 'echo ok > out.txt'` writes
    // `ok\n`. We match each platform's exact output.
    let entry_point = "echo ok > out.txt".to_string();
    MainEntity {
        entry_point,
        expected_output_paths: vec!["out.txt".into()],
        expected_output_hashes_hex: vec![expected_hex],
        env_pin: "test".into(),
        timeout_seconds: 30,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        figures: vec![],
    }
}

#[tokio::test]
async fn replay_passes_when_output_matches() {
    let dir = tempfile::tempdir().unwrap();
    let me = trivial_main_entity_writing_ok();
    let report = run_replay(dir.path(), &me).await.expect("run replay");
    // If this fails on Windows due to a different `echo` shell, inspect
    // `dir.path().join("out.txt")` bytes and adjust the platform-specific
    // hash in `trivial_main_entity_writing_ok`.
    assert_eq!(
        report.outcome,
        ReplayOutcome::Pass,
        "expected Pass; diagnostics={:?}; stdout={:?}; stderr={:?}",
        report.diagnostics,
        report.stdout,
        report.stderr
    );
    assert_eq!(report.measured_score, 1.0);
}

#[tokio::test]
async fn replay_reports_hash_mismatch_when_expected_hash_is_wrong() {
    let dir = tempfile::tempdir().unwrap();
    let mut me = trivial_main_entity_writing_ok();
    me.expected_output_hashes_hex = vec!["deadbeef".into()];
    let report = run_replay(dir.path(), &me).await.expect("run replay");
    assert_eq!(report.outcome, ReplayOutcome::HashMismatch);
    assert_eq!(report.measured_score, 0.0);
    assert!(!report.diagnostics.is_empty());
}

#[tokio::test]
async fn replay_reports_nonzero_exit() {
    let dir = tempfile::tempdir().unwrap();
    let me = MainEntity {
        entry_point: "exit 3".into(),
        expected_output_paths: vec![],
        expected_output_hashes_hex: vec![],
        env_pin: "test".into(),
        timeout_seconds: 30,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        figures: vec![],
    };
    let report = run_replay(dir.path(), &me).await.expect("run replay");
    assert_eq!(report.outcome, ReplayOutcome::NonZeroExit { exit_code: 3 });
    assert_eq!(report.measured_score, 0.0);
}

#[tokio::test]
async fn replay_reports_timeout() {
    let dir = tempfile::tempdir().unwrap();
    let entry_point = if cfg!(windows) {
        "ping -n 30 127.0.0.1 > nul".to_string()
    } else {
        "sleep 30".to_string()
    };
    let me = MainEntity {
        entry_point,
        expected_output_paths: vec![],
        expected_output_hashes_hex: vec![],
        env_pin: "test".into(),
        // Sub-second timeout so the test completes quickly.
        timeout_seconds: 1,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        figures: vec![],
    };
    let report = run_replay(dir.path(), &me).await.expect("run replay");
    assert_eq!(report.outcome, ReplayOutcome::TimedOut);
    assert_eq!(report.measured_score, 0.0);
}

#[tokio::test]
async fn contract_malformed_when_paths_and_hashes_differ_in_length() {
    let dir = tempfile::tempdir().unwrap();
    let me = MainEntity {
        entry_point: "echo".into(),
        expected_output_paths: vec!["a.txt".into(), "b.txt".into()],
        expected_output_hashes_hex: vec!["abcd".into()],
        env_pin: "test".into(),
        timeout_seconds: 5,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        figures: vec![],
    };
    let result = run_replay(dir.path(), &me).await;
    assert!(matches!(
        result,
        Err(vox_replay_runner::ReplayError::ContractMalformed(_))
    ));
    // tempfile dir survives despite ContractMalformed (consumed below).
    let _ = fs::read_dir(dir.path());
    let _: Duration = Duration::from_secs(1); // silence unused-import noise
}
