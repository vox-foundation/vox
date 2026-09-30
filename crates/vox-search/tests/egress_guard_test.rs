//! Task 9c: the test egress guard (`tests/common/mod.rs`) really blocks
//! non-loopback hosts and really lets wiremock through. Mutation guard:
//! without the guard, the real-host request below is not refused by the local
//! dead proxy, and this test fails.

// Task 9c: no real web host from any test (see tests/common/mod.rs).
mod common;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_loopback_requests_are_refused_and_loopback_is_allowed() {
    let client = vox_http_client::client_builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("client");

    // A real host: routed to the dead local proxy and refused before any DNS.
    let err = client
        .get("https://en.wikipedia.org/wiki/Main_Page")
        .send()
        .await
        .expect_err("a real web host must be unreachable from tests");
    let chain = format!("{err:?}");
    assert!(
        chain.contains("127.0.0.1:9") || chain.to_ascii_lowercase().contains("tunnel"),
        "must fail at the local dead proxy, not on the network: {chain}"
    );

    // Loopback (wiremock) is exempt.
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ok"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock)
        .await;
    let ok = client
        .get(format!("{}/ok", mock.uri()))
        .send()
        .await
        .expect("loopback must bypass the guard");
    assert!(ok.status().is_success());
}
