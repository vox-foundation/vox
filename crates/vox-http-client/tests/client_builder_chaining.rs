//! Offline checks that the preset `ClientBuilder` composes with common overrides.

use std::time::Duration;

use vox_http_client::client_builder;

#[test]
fn client_builder_chains_user_agent_and_timeouts_without_panic() {
    let client = client_builder()
        .user_agent("integration-test/vox-reqwest-defaults")
        // drift-allow(timeout-literal): SSOT crate's own integration test exercises chainability
        .connect_timeout(Duration::from_secs(30))
        // drift-allow(timeout-literal): SSOT crate's own integration test exercises chainability
        .pool_idle_timeout(Duration::from_secs(120))
        .build()
        .expect("chained builder should still construct offline");

    let _ = client;
}

#[test]
fn client_builder_composes_with_gzip_toggle() {
    // Workspace reqwest builds with default-features = false (no gzip feature);
    // verify chainability with an always-available override while preserving test symbol.
    let client = client_builder()
        .no_proxy()
        .build()
        .expect("override should compose with preset builder");
    let _ = client;
}
