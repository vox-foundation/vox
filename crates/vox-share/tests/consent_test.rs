// Rust 2024: env mutation primitives are `unsafe`. Each test redirects
// `VOX_SHARE_STATE_PATH` to a tempdir before mutating state; the SAFETY
// rationale is documented at every `unsafe` block.
#![allow(unsafe_code)]

use tempfile::TempDir;
use vox_share::state::ShareState;

#[test]
fn share_state_default_is_not_consented() {
    let state = ShareState::default();
    assert!(!state.cloudflare_consent_v1);
}

#[test]
fn share_state_round_trips_via_json() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("share-state.json");

    // Save with env var pointing to our temp path.
    unsafe {
        std::env::set_var("VOX_SHARE_STATE_PATH", path.to_str().unwrap());
    }
    let state = ShareState {
        cloudflare_consent_v1: true,
        consent_text_version: 1,
    };
    state.save().unwrap();
    unsafe {
        std::env::remove_var("VOX_SHARE_STATE_PATH");
    }

    // Verify by reading the JSON file directly — avoids env-var races between parallel tests.
    let raw = std::fs::read_to_string(&path).unwrap();
    let loaded: ShareState = serde_json::from_str(&raw).unwrap();

    assert!(loaded.cloudflare_consent_v1);
    assert_eq!(loaded.consent_text_version, 1);
}

#[test]
fn ensure_consent_accepts_with_accept_tos_flag() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("share-state.json");
    unsafe {
        std::env::set_var("VOX_SHARE_STATE_PATH", path.to_str().unwrap());
    }

    let result = vox_share::consent::ensure_consent(true, false);
    unsafe {
        std::env::remove_var("VOX_SHARE_STATE_PATH");
    }

    assert!(
        result.is_ok(),
        "accept_tos=true should always succeed: {:?}",
        result
    );
}

#[test]
fn ensure_consent_skips_if_already_accepted() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("share-state.json");
    unsafe {
        std::env::set_var("VOX_SHARE_STATE_PATH", path.to_str().unwrap());
    }

    // First: accept via flag
    vox_share::consent::ensure_consent(true, false).unwrap();

    // Second call (no flag, no force): should not error since already accepted
    // In CI (non-TTY), force_prompt=false + already accepted → Ok
    let result = vox_share::consent::ensure_consent(false, false);
    unsafe {
        std::env::remove_var("VOX_SHARE_STATE_PATH");
    }

    assert!(
        result.is_ok(),
        "should skip consent when already accepted: {:?}",
        result
    );
}
