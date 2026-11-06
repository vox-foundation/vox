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
async fn developer_journey_seed_present() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let steps = db
        .list_developer_journey_steps("canonical_journey.v1.greenfield_vox_mens_devloop")
        .await
        .expect("list_developer_journey_steps");
    assert_eq!(steps.len(), 8, "expected seeded canonical journey steps");
}

#[tokio::test]
async fn plan_sessions_has_iterative_telemetry_columns() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let cols = pragma_columns(&db, "plan_sessions").await;
    for required in [
        "question_session_id",
        "iterative_loop_round",
        "iterative_stop_reason",
        "iterative_loop_metadata_json",
    ] {
        assert!(
            cols.iter().any(|c| c == required),
            "plan_sessions.{required} missing: {cols:?}"
        );
    }
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
    assert!(
        cols.iter().any(|c| c == "content_sha3_256"),
        "published_news.content_sha3_256 missing: {cols:?}"
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
async fn record_agent_event_round_trip_matches_agent_events_table() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    db.record_agent_event("agent-a", "test_evt", r#"{"k":1}"#, "9.9.9")
        .await
        .expect("insert");
    let rows = db
        .query_all(
            "SELECT event_type, payload_json, cli_version FROM agent_events WHERE agent_id = 'agent-a' ORDER BY timestamp DESC LIMIT 10",
            (),
        )
        .await
        .expect("list");
    assert_eq!(rows.len(), 1);
    let event_type: String = rows[0].get(0).expect("event_type");
    let payload_json: Option<String> = rows[0].get(1).unwrap_or(None);
    let cli_version: Option<String> = rows[0].get(2).unwrap_or(None);
    assert_eq!(event_type, "test_evt");
    assert_eq!(payload_json.as_deref(), Some(r#"{"k":1}"#));
    assert_eq!(cli_version.as_deref(), Some("9.9.9"));
}

#[tokio::test]
async fn provider_quota_usage_table_exists() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let cols = pragma_columns(&db, "provider_quota_usage").await;
    for col in [
        "provider",
        "period_key",
        "units_spent",
        "units_limit",
        "last_synced_at",
    ] {
        assert!(
            cols.iter().any(|c| c == col),
            "provider_quota_usage.{col} missing: {cols:?}"
        );
    }
}

#[tokio::test]
async fn provider_quota_usage_spend_and_get_roundtrip() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
    let period = "2026-09";
    db.record_quota_spend("tavily", period, 50)
        .await
        .expect("record spend");
    let usage = db
        .get_quota_usage("tavily", period)
        .await
        .expect("get usage")
        .expect("some usage");
    assert_eq!(usage.provider, "tavily");
    assert_eq!(usage.period_key, period);
    assert_eq!(usage.units_spent, 50);
    assert_eq!(usage.units_limit, 1000);

    // Spend additional 25
    db.record_quota_spend("tavily", period, 25)
        .await
        .expect("record spend 2");
    let usage2 = db
        .get_quota_usage("tavily", period)
        .await
        .expect("get usage 2")
        .expect("some usage 2");
    assert_eq!(usage2.units_spent, 75);
}

#[test]
fn baseline_sql_partial_indexes_are_strictly_allowlisted() {
    let sql = vox_db::schema::baseline_sql();
    let index_re = regex::Regex::new(
        r"(?is)CREATE\s+(?:UNIQUE\s+)?INDEX\s+(?:IF\s+NOT\s+EXISTS\s+)?([a-zA-Z0-9_]+)\s+ON\s+[a-zA-Z0-9_]+\s*\([^)]+\)\s*WHERE\s+([^;]+);",
    )
    .expect("regex");

    // Only insert-time immutable uniqueness/dedupe partial indexes are allowed in Turso 0.6.x
    // (mutable nullable partial indexes trip Turso's IdxDelete bug on UPDATE).
    let allowlisted = [
        "idx_conversations_repo_ext_session",
        "idx_scientia_harness_issues_pending_session_category",
        "idx_mesh_a2a_idempotency",
        "convergence_op_log_change_id",
    ];

    for caps in index_re.captures_iter(&sql) {
        let idx_name = caps.get(1).expect("idx name").as_str();
        assert!(
            allowlisted.contains(&idx_name),
            "Disallowed partial index `{idx_name}` in baseline_sql(). Use a composite covering B-tree index instead of a partial WHERE index on mutable columns."
        );
    }
}

#[test]
fn baseline_sql_forbids_removed_redundant_prefix_indexes() {
    let sql = vox_db::schema::baseline_sql();
    let retired_redundant = [
        "idx_memories_agent",
        "idx_behavior_user",
        "idx_learned_patterns_user",
        "idx_knowledge_edges_src",
        "idx_knowledge_edges_dst",
        "idx_search_chunks_doc",
        "idx_user_preferences_user",
        "idx_account_config_account",
    ];
    for idx in retired_redundant {
        let pattern = format!("INDEX IF NOT EXISTS {idx} ");
        assert!(
            !sql.contains(&pattern),
            "Redundant left-prefix index `{idx}` must not be reintroduced in baseline_sql()"
        );
    }
}

#[tokio::test]
async fn turso_consolidated_sidecar_tables_and_covering_indexes_round_trip() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("db");

    // 1. Verify covering & FK indexes exist in sqlite_master
    let index_rows = db
        .query_all(
            "SELECT name, sql FROM sqlite_master WHERE type='index' AND name IN (
                'idx_llm_interactions_trial',
                'idx_llm_interactions_cat_created',
                'idx_llm_attempts_trace',
                'idx_scientia_ext_intel_status',
                'idx_corpus_pairs_label_origin',
                'idx_scientia_harness_decisions_issue',
                'idx_scientia_harness_fix_proposals_issue',
                'idx_telemetry_upload_spool_created',
                'idx_mesh_replay_nonces_recorded',
                'idx_mesh_local_nodes_updated'
            )",
            (),
        )
        .await
        .expect("query indexes");
    assert_eq!(
        index_rows.len(),
        10,
        "all 10 new/upgraded Turso covering and FK indexes must be present"
    );

    // 2. Verify telemetry_upload_spool round-trip
    db.enqueue_telemetry_upload_spool("spool-1", r#"{"event":"test"}"#, 1000)
        .await
        .expect("enqueue spool");
    let spool_items = db.list_telemetry_upload_spool().await.expect("list spool");
    assert_eq!(spool_items.len(), 1);
    assert_eq!(spool_items[0].0, "spool-1");
    assert!(
        db.ack_telemetry_upload_spool("spool-1")
            .await
            .expect("ack spool")
    );
    assert!(
        db.list_telemetry_upload_spool()
            .await
            .expect("list spool empty")
            .is_empty()
    );

    // 3. Verify mesh_replay_nonces round-trip
    db.upsert_mesh_replay_nonce("jwt_jti", "jti-abc", 5000)
        .await
        .expect("upsert nonce");
    let nonces = db
        .list_mesh_replay_nonces("jwt_jti")
        .await
        .expect("list nonces");
    assert_eq!(nonces, vec![("jti-abc".to_string(), 5000)]);

    // 4. Verify mesh_local_nodes round-trip
    db.upsert_mesh_local_node("node-1", r#"{"id":"node-1"}"#, 9000)
        .await
        .expect("upsert local node");
    let nodes = db.list_mesh_local_nodes().await.expect("list local nodes");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].0, "node-1");
}
