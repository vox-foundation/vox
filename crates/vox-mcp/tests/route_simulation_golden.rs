//! MCP route simulation returns the same `SyndicationResult` snapshot as `vox-publisher` golden fixtures.

use serde_json::json;
use vox_db::{DbConfig, PublicationManifestParams, VoxDb};
use vox_mcp::{ServerState, tools};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scientia_route_simulate_matches_vox_publisher_golden_fixture() {
    vox_publisher::PublisherConfig::clear_route_simulation_env_overrides();
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let metadata = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-publisher/tests/fixtures/golden_route_metadata.json"
    ));
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "golden-route-001",
        content_type: "news",
        source_ref: None,
        title: "Golden route item",
        author: "Vox",
        abstract_text: None,
        body_markdown: "Hello world for golden route.",
        citations_json: None,
        metadata_json: Some(metadata),
        revision_history_json: None,
        content_sha3_256: "0000000000000000000000000000000000000000000000000000000000000000",
        state: "draft",
    })
    .await
    .expect("upsert manifest");

    let state = ServerState::new_test().await.with_db(db);
    let result = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_route_simulate",
        json!({ "publication_id": "golden-route-001" }),
    )
    .await
    .expect("tool call");
    let val: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(val["success"], true);
    let expected: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-publisher/tests/fixtures/route_simulation_golden.json"
    )))
    .expect("golden fixture");
    assert_eq!(val["data"], expected);
}
