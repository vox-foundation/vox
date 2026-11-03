//! CLI route simulation matches the same golden `SyndicationResult` as `vox-publisher` / MCP tests.

use vox_cli::commands::db::publication_route_simulate_with_db;
use vox_db::{DbConfig, PublicationManifestParams, VoxDb};

#[tokio::test]
#[serial_test::serial]
async fn publication_route_simulate_with_memory_db_matches_golden_fixture() {
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

    let result = publication_route_simulate_with_db(&db, "golden-route-001")
        .await
        .expect("route simulate");

    let expected: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../vox-publisher/tests/fixtures/route_simulation_golden.json"
    )))
    .expect("golden fixture");
    let actual = serde_json::to_value(&result).expect("serialize");
    assert_eq!(actual, expected);
}
