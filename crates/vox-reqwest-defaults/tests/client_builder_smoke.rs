//! Offline checks for shared `reqwest` presets (`vox-reqwest-defaults`).

use vox_reqwest_defaults::client_builder;

#[test]
fn client_builder_preset_builds_offline() {
    // Default headers (including user-agent) are applied when the request is sent,
    // not on `Request::headers()` for a freshly built request.
    let _client = client_builder()
        .build()
        .expect("ClientBuilder from vox_reqwest_defaults should build offline");
}

#[test]
fn fallback_client_constructor_does_not_panic() {
    let _c = vox_reqwest_defaults::client();
}
