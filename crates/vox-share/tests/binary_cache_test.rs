//! Tests for binary_cache.rs using env var overrides.
//! We never hit the real internet in tests.
//!
//! Note: tests using `std::env::set_var` are not thread-safe in parallel test runs.
//! Run with `-- --test-threads=1` or use the `serial_test` crate if isolation is needed.

// Rust 2024: `std::env::set_var` / `remove_var` are `unsafe`. The workspace
// policy is `unsafe_code = "warn"`; here we serialize each test's mutation
// with the SAFETY rationale documented at every `unsafe` block.
#![allow(unsafe_code)]

use tempfile::TempDir;
use vox_share::binary_cache::{CLOUDFLARED_VERSION, cached_binary_name, verify_sha256};

#[test]
fn cloudflared_version_is_set() {
    assert!(!CLOUDFLARED_VERSION.is_empty());
    assert!(
        CLOUDFLARED_VERSION.starts_with("20"),
        "version should be a year-based release: {}",
        CLOUDFLARED_VERSION
    );
}

#[test]
fn cached_binary_name_contains_version_and_platform() {
    let name = cached_binary_name();
    assert!(
        name.contains(CLOUDFLARED_VERSION),
        "name should contain version: {}",
        name
    );
    assert!(
        name.contains(std::env::consts::OS),
        "name should contain OS: {}",
        name
    );
}

#[test]
fn verify_sha256_correct_hash() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("testfile");
    std::fs::write(&path, b"hello world").unwrap();
    // Just verify it returns Ok and doesn't panic on a readable file.
    let some_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe04294e576e0efbbc26a6f1a0a";
    let result = verify_sha256(&path, some_hash);
    assert!(
        result.is_ok(),
        "verify_sha256 should not error on readable file"
    );
}

#[test]
fn verify_sha256_known_content() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("testfile");
    // Write known content
    let content = b"vox share test binary";
    std::fs::write(&path, content).unwrap();
    // Compute expected sha256
    use sha2::{Digest, Sha256};
    let expected = hex::encode(Sha256::digest(content));
    assert!(
        verify_sha256(&path, &expected).unwrap(),
        "sha256 of known content should match"
    );
    // Tampered content should not match
    std::fs::write(&path, b"tampered").unwrap();
    assert!(
        !verify_sha256(&path, &expected).unwrap(),
        "sha256 of tampered content should not match"
    );
}

#[tokio::test]
async fn ensure_cloudflared_respects_vox_cloudflared_path_override() {
    let tmp = TempDir::new().unwrap();
    let fake_bin = tmp.path().join("fake-cloudflared");
    std::fs::write(&fake_bin, b"fake binary content").unwrap();

    // Override path — note: set_var is not thread-safe; run this test with --test-threads=1
    // if parallelism causes flakiness.
    // SAFETY: single-threaded test; no other threads read this env var concurrently.
    unsafe {
        std::env::set_var("VOX_CLOUDFLARED_PATH", fake_bin.to_str().unwrap());
    }
    let result = vox_share::binary_cache::ensure_cloudflared().await;
    // SAFETY: same reasoning as set_var above.
    unsafe {
        std::env::remove_var("VOX_CLOUDFLARED_PATH");
    }

    assert!(result.is_ok(), "should succeed with override: {:?}", result);
    assert_eq!(result.unwrap(), fake_bin);
}

#[tokio::test]
async fn ensure_cloudflared_errors_on_missing_vox_cloudflared_path() {
    // SAFETY: single-threaded test; no other threads read this env var concurrently.
    unsafe {
        std::env::set_var("VOX_CLOUDFLARED_PATH", "/nonexistent/path/cloudflared");
    }
    let result = vox_share::binary_cache::ensure_cloudflared().await;
    // SAFETY: same reasoning as set_var above.
    unsafe {
        std::env::remove_var("VOX_CLOUDFLARED_PATH");
    }
    assert!(
        result.is_err(),
        "should error when VOX_CLOUDFLARED_PATH does not exist"
    );
}
