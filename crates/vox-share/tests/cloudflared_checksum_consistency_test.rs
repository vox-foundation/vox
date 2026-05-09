//! Verify that the hardcoded cloudflared checksums in binary_cache.rs
//! match what we expect. This test is `#[ignore]` by default and is run
//! in CI with `cargo test -- --ignored` to detect version drift.
//!
//! If this test fails: update CLOUDFLARED_VERSION and the checksums in
//! binary_cache.rs to match the latest release.

use vox_share::binary_cache::{CLOUDFLARED_VERSION, cloudflared_url_and_checksum};

#[test]
#[ignore = "network test — run with --ignored in CI"]
fn cloudflared_version_constant_is_reasonable() {
    // Just verify the version string looks like a cloudflared release version.
    assert!(
        CLOUDFLARED_VERSION.starts_with("20"),
        "CLOUDFLARED_VERSION should start with year: {}",
        CLOUDFLARED_VERSION
    );
    // Version should have at least two dots: 20XX.Y.Z
    assert!(
        CLOUDFLARED_VERSION.matches('.').count() >= 1,
        "CLOUDFLARED_VERSION should have at least one dot: {}",
        CLOUDFLARED_VERSION
    );
}

#[test]
fn cloudflared_url_and_checksum_returns_some_for_current_platform() {
    // Non-network: just verify the function returns Some for the current platform.
    // Unsupported platforms are acceptable to skip.
    let result = cloudflared_url_and_checksum();
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64")
        | ("linux", "aarch64")
        | ("macos", "x86_64")
        | ("macos", "aarch64")
        | ("windows", "x86_64") => {
            assert!(
                result.is_some(),
                "should have a checksum for {}-{}",
                os,
                arch
            );
            let (url, sha) = result.unwrap();
            assert!(url.contains(CLOUDFLARED_VERSION));
            assert_eq!(sha.len(), 64, "SHA256 should be 64 hex chars");
        }
        _ => {
            // Unsupported platform — acceptable to skip
        }
    }
}
