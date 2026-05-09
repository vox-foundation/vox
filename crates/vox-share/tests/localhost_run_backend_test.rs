//! localhost.run backend tests.

use vox_share::backends::localhost_run::LocalhostRunBackend;
use vox_share::{BackendKind, TunnelBackend};

#[tokio::test]
async fn localhost_run_backend_kind_is_correct() {
    let backend = LocalhostRunBackend::new();
    assert_eq!(backend.kind(), BackendKind::LocalhostRun);
}

#[test]
fn localhost_run_backend_constructs() {
    let _ = LocalhostRunBackend::new();
}

#[tokio::test]
async fn preflight_fails_gracefully_when_ssh_unavailable() {
    // We cannot reliably hide SSH in the test environment, so we just check
    // that preflight() returns a result (Ok or Err) without panicking.
    let backend = LocalhostRunBackend::new();
    let result = backend.preflight().await;
    // Either ssh is present (Ok) or not (Err with BackendUnavailable).
    match result {
        Ok(()) => {
            // ssh is on PATH — preflight passes
        }
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("ssh") || msg.contains("localhost-run"),
                "error should mention ssh or localhost-run, got: {}",
                msg
            );
        }
    }
}
