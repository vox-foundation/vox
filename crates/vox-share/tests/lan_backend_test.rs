//! LAN backend integration test. Verifies the LAN backend returns a URL
//! that looks like a routable LAN or loopback address containing the given port.

use std::time::Duration;
use vox_share::backends::lan::LanBackend;
use vox_share::{BackendKind, TunnelBackend};

#[tokio::test]
async fn lan_backend_returns_lan_url_pointing_at_local_port() {
    let backend = LanBackend::new();

    backend
        .preflight()
        .await
        .expect("LAN preflight should always succeed");

    let port = 7860u16;
    let handle = backend
        .start(port, Duration::from_secs(1))
        .await
        .expect("LAN backend start should succeed unconditionally");

    assert_eq!(handle.backend, BackendKind::Lan);
    // URL should be http (not https) and contain the port.
    assert!(handle.public_url.starts_with("http://"));
    assert!(handle.public_url.contains(&format!(":{}", port)));
    // For LAN we expect either `0.0.0.0` or a real LAN IP (3 dots).
    assert!(
        handle.public_url.contains("0.0.0.0")
            || handle.public_url.chars().filter(|c| *c == '.').count() == 3,
        "LAN URL should contain 0.0.0.0 or a dotted IP, got: {}",
        handle.public_url
    );

    handle.shutdown();
}

#[tokio::test]
async fn lan_backend_kind_is_lan() {
    let backend = LanBackend::new();
    assert_eq!(backend.kind(), BackendKind::Lan);
}
