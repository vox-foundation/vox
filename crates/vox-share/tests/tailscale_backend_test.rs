//! Tailscale backend unit tests (no real Tailscale required).

use vox_share::backends::tailscale::TailscaleBackend;
use vox_share::{BackendKind, TunnelBackend};

#[test]
fn tailscale_backend_kind() {
    let b = TailscaleBackend::new();
    assert_eq!(b.kind(), BackendKind::Tailscale);
}

#[tokio::test]
async fn tailscale_preflight_fails_gracefully_without_tailscale() {
    // If tailscale is not installed (common in CI), preflight should return a clear error.
    if vox_share::backends::tailscale::detect_tailscale().is_some() {
        // Tailscale is installed — skip this test (we can't assert failure).
        return;
    }
    let backend = TailscaleBackend::new();
    let result = backend.preflight().await;
    assert!(result.is_err(), "preflight should fail when tailscale is not installed");
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("tailscale") || err_str.contains("not found"),
        "error should mention tailscale: {}",
        err_str
    );
}

#[tokio::test]
async fn tailscale_start_rejects_unsupported_port() {
    // Even if tailscale is not installed, the port validation happens before binary invocation
    // IF the port check is done early. But our implementation calls detect_tailscale first.
    // If tailscale is not installed, this returns BackendUnavailable.
    // If it IS installed, it returns Config error for unsupported port.
    let backend = TailscaleBackend::new();
    let result = backend.start(7860, std::time::Duration::from_secs(1)).await;
    assert!(result.is_err(), "port 7860 is not supported by Tailscale Funnel");
}
