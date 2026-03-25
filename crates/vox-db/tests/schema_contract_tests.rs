//! Contract checks after SSOT cutover: critical columns exist on a migrated in-memory DB.

use vox_db::{DbConfig, VoxDb};

async fn pragma_columns(db: &VoxDb, table: &str) -> Vec<String> {
    let sql = format!("PRAGMA table_info({table})");
    let rows = db.query_all(&sql, ()).await.expect("pragma");
    let mut out = Vec::new();
    for r in rows {
        let name: String = r.get(1).expect("col name");
        out.push(name);
    }
    out
}

#[tokio::test]
async fn agent_events_has_payload_json_and_cli_version() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let cols = pragma_columns(&db, "agent_events").await;
    assert!(
        cols.iter().any(|c| c == "payload_json"),
        "agent_events.payload_json missing: {cols:?}"
    );
    assert!(
        cols.iter().any(|c| c == "cli_version"),
        "agent_events.cli_version missing: {cols:?}"
    );
}

#[tokio::test]
async fn published_news_uses_news_id_primary_key() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let cols = pragma_columns(&db, "published_news").await;
    assert!(
        cols.iter().any(|c| c == "news_id"),
        "published_news.news_id missing: {cols:?}"
    );
    assert!(
        !cols.iter().any(|c| c == "id"),
        "published_news should not use legacy `id` column: {cols:?}"
    );
}

#[tokio::test]
async fn agent_session_events_table_exists() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let cols = pragma_columns(&db, "agent_session_events").await;
    assert!(
        cols.iter().any(|c| c == "session_id"),
        "agent_session_events.session_id missing: {cols:?}"
    );
    assert!(
        cols.iter().any(|c| c == "payload_json"),
        "agent_session_events.payload_json missing: {cols:?}"
    );
}

#[tokio::test]
async fn record_agent_event_round_trip_matches_list_gamify_events() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    db.record_agent_event("agent-a", "test_evt", r#"{"k":1}"#, "9.9.9")
        .await
        .expect("insert");
    let rows = db
        .list_gamify_events("agent-a", 10)
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].event_type, "test_evt");
    assert_eq!(rows[0].payload_json.as_deref(), Some(r#"{"k":1}"#));
    assert_eq!(rows[0].cli_version.as_deref(), Some("9.9.9"));
}
