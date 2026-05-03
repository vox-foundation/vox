use vox_build_meta::{active_features, has, require};

#[test]
fn active_features_is_empty() {
    assert_eq!(active_features(), Vec::<&'static str>::new());
}

#[test]
fn has_always_returns_false() {
    assert!(!has("gpu"));
    assert!(!has("mens-candle-cuda"));
    assert!(!has("any-feature-name"));
}

#[test]
fn require_returns_error_with_install_command_in_message() {
    let err = require("mens-candle-cuda", "vox plugin install mens-candle-cuda")
        .expect_err("require should fail when feature is absent");
    let msg = format!("{err}");
    assert!(msg.contains("vox plugin install mens-candle-cuda"), "msg was: {msg}");
}
