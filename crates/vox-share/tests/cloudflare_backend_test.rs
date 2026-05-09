//! Cloudflare backend test using a mock cloudflared binary.
//!
//! The mock binary prints a fake trycloudflare.com URL to stderr after a short
//! delay, mimicking real cloudflared behavior. Set VOX_CLOUDFLARED_PATH to the
//! mock binary path for the test.

use std::time::Duration;
use vox_share::backends::cloudflare::CloudflareBackend;
use vox_share::{BackendKind, TunnelBackend};

/// Build the mock binary path. We use a shell/batch script as the mock.
fn mock_cloudflared_path() -> std::path::PathBuf {
    // We'll create a temp script that mimics cloudflared's URL output.
    let tmp = std::env::temp_dir().join("vox_share_mock_cloudflared");
    std::fs::create_dir_all(&tmp).unwrap();

    #[cfg(unix)]
    {
        let script = tmp.join("cloudflared");
        std::fs::write(
            &script,
            b"#!/bin/sh\nsleep 0.1\necho 'INF https://test-mock.trycloudflare.com' >&2\nsleep 30\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }
    #[cfg(windows)]
    {
        let script = tmp.join("cloudflared.bat");
        std::fs::write(&script, b"@echo off\r\nping -n 1 127.0.0.1 > nul\r\necho INF https://test-mock.trycloudflare.com 1>&2\r\nping -n 30 127.0.0.1 > nul\r\n").unwrap();
        script
    }
}

#[tokio::test]
async fn cloudflare_backend_parses_url_from_mock() {
    let mock_path = mock_cloudflared_path();

    // Point the binary cache at our mock.
    unsafe {
        std::env::set_var("VOX_CLOUDFLARED_PATH", mock_path.to_str().unwrap());
    }

    let backend = CloudflareBackend::new();
    assert_eq!(backend.kind(), BackendKind::Cloudflare);

    let handle = backend
        .start(7860, Duration::from_secs(15))
        .await
        .expect("Cloudflare backend should parse URL from mock cloudflared");

    unsafe {
        std::env::remove_var("VOX_CLOUDFLARED_PATH");
    }

    assert_eq!(handle.backend, BackendKind::Cloudflare);
    assert!(
        handle.public_url.contains("trycloudflare.com"),
        "URL should be a trycloudflare.com URL, got: {}",
        handle.public_url
    );

    handle.shutdown();
}

#[tokio::test]
async fn cloudflare_backend_preflight_fails_without_binary() {
    // Point at a nonexistent path so preflight fails.
    unsafe {
        std::env::set_var("VOX_CLOUDFLARED_PATH", "/nonexistent/cloudflared");
    }
    let backend = CloudflareBackend::new();
    let result = backend.preflight().await;
    unsafe {
        std::env::remove_var("VOX_CLOUDFLARED_PATH");
    }
    assert!(
        result.is_err(),
        "preflight should fail when binary is missing"
    );
}
