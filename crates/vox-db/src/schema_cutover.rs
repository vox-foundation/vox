//! Idempotent alignment for databases created before a schema SSOT cutover.
//!
//! [`crate::store::open::VoxDb::migrate`] runs baseline DDL first (`CREATE IF NOT EXISTS`), which
//! does not add columns to existing tables. This module applies additive fixes and one-time table
//! rebuilds when detected via `PRAGMA table_info`.

use turso::Connection;

use crate::store::types::StoreError;

async fn table_column_names(
    conn: &Connection,
    pragma_sql: &'static str,
) -> Result<Vec<String>, StoreError> {
    let mut rows = conn.query(pragma_sql, ()).await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let name: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        out.push(name);
    }
    Ok(out)
}

fn has_col(cols: &[String], name: &str) -> bool {
    cols.iter().any(|c| c == name)
}

/// Apply additive migrations and renames that baseline `IF NOT EXISTS` cannot perform.
pub async fn apply_schema_cutover(conn: &Connection) -> Result<(), StoreError> {
    align_workflow_activity_log(conn).await?;
    align_workflow_run_log(conn).await?;
    align_question_sessions_belief(conn).await?;
    align_plan_sessions_iterative(conn).await?;
    align_a2a_claim_columns(conn).await?;
    align_agent_events(conn).await?;
    migrate_published_news_news_id(conn).await?;
    migrate_published_news_content_digest(conn).await?;
    align_conversations_workspace_journey(conn).await?;
    align_routing_decisions_table(conn).await?;
    apply_knowledge_fts_cutover(conn).await?;
    apply_search_document_chunks_fts_cutover(conn).await?;
    crate::ludus_schema_cutover::apply_ludus_gamify_cutover(conn).await?;
    Ok(())
}

/// Add workspace/journey identity columns to structured transcript tables (Unified Vox Request Journey).
async fn align_conversations_workspace_journey(conn: &Connection) -> Result<(), StoreError> {
    let ccols = table_column_names(conn, "PRAGMA table_info(conversations)").await?;
    if !ccols.is_empty() {
        if !has_col(&ccols, "repository_id") {
            conn.execute("ALTER TABLE conversations ADD COLUMN repository_id TEXT", ())
                .await?;
        }
        if !has_col(&ccols, "external_session_id") {
            conn.execute(
                "ALTER TABLE conversations ADD COLUMN external_session_id TEXT",
                (),
            )
            .await?;
        }
        if !has_col(&ccols, "thread_id") {
            conn.execute("ALTER TABLE conversations ADD COLUMN thread_id TEXT", ())
                .await?;
        }
        if !has_col(&ccols, "origin_surface") {
            conn.execute("ALTER TABLE conversations ADD COLUMN origin_surface TEXT", ())
                .await?;
        }
    }

    let mcols = table_column_names(conn, "PRAGMA table_info(conversation_messages)").await?;
    if !mcols.is_empty() {
        if !has_col(&mcols, "external_turn_id") {
            conn.execute(
                "ALTER TABLE conversation_messages ADD COLUMN external_turn_id TEXT",
                (),
            )
            .await?;
        }
        if !has_col(&mcols, "model_used") {
            conn.execute("ALTER TABLE conversation_messages ADD COLUMN model_used TEXT", ())
                .await?;
        }
        if !has_col(&mcols, "token_count") {
            conn.execute(
                "ALTER TABLE conversation_messages ADD COLUMN token_count INTEGER",
                (),
            )
            .await?;
        }
        if !has_col(&mcols, "context_files_json") {
            conn.execute(
                "ALTER TABLE conversation_messages ADD COLUMN context_files_json TEXT",
                (),
            )
            .await?;
        }
    }

    // Partial unique index (idempotent): one conversation row per (repository, MCP session).
    let _ = conn
        .execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_conversations_repo_ext_session
             ON conversations(repository_id, external_session_id)
             WHERE repository_id IS NOT NULL AND external_session_id IS NOT NULL",
            (),
        )
        .await;
    let _ = conn
        .execute(
            "CREATE INDEX IF NOT EXISTS idx_conversation_messages_external_turn
             ON conversation_messages(external_turn_id)",
            (),
        )
        .await;
    let _ = conn
        .execute(
            "CREATE INDEX IF NOT EXISTS idx_conversations_repository ON conversations(repository_id)",
            (),
        )
        .await;

    Ok(())
}

/// Bounded routing decision log for analysis (no prompt/body content).
async fn align_routing_decisions_table(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS routing_decisions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            journey_id TEXT,
            repository_id TEXT,
            session_id TEXT,
            surface TEXT NOT NULL DEFAULT '',
            model_id TEXT,
            reason_json TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_routing_decisions_created ON routing_decisions(created_at);
        CREATE INDEX IF NOT EXISTS idx_routing_decisions_journey ON routing_decisions(journey_id);
        ",
    )
    .await?;
    Ok(())
}

async fn align_workflow_run_log(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(workflow_run_log)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "lease_owner") {
        conn.execute(
            "ALTER TABLE workflow_run_log ADD COLUMN lease_owner TEXT",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "lease_until_ms") {
        conn.execute(
            "ALTER TABLE workflow_run_log ADD COLUMN lease_until_ms INTEGER",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "plan_session_id") {
        conn.execute(
            "ALTER TABLE workflow_run_log ADD COLUMN plan_session_id TEXT",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "plan_node_id") {
        conn.execute(
            "ALTER TABLE workflow_run_log ADD COLUMN plan_node_id TEXT",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "plan_version") {
        conn.execute(
            "ALTER TABLE workflow_run_log ADD COLUMN plan_version INTEGER",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_workflow_activity_log(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(workflow_activity_log)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "result_json") {
        conn.execute(
            "ALTER TABLE workflow_activity_log ADD COLUMN result_json TEXT",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_a2a_claim_columns(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(a2a_messages)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "claim_owner") {
        conn.execute("ALTER TABLE a2a_messages ADD COLUMN claim_owner TEXT", ())
            .await?;
    }
    if !has_col(&cols, "claim_until_ms") {
        conn.execute(
            "ALTER TABLE a2a_messages ADD COLUMN claim_until_ms INTEGER",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "delivery_attempts") {
        conn.execute(
            "ALTER TABLE a2a_messages ADD COLUMN delivery_attempts INTEGER NOT NULL DEFAULT 0",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "last_claim_error") {
        conn.execute(
            "ALTER TABLE a2a_messages ADD COLUMN last_claim_error TEXT",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "processed_at_ms") {
        conn.execute(
            "ALTER TABLE a2a_messages ADD COLUMN processed_at_ms INTEGER",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_plan_sessions_iterative(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(plan_sessions)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "question_session_id") {
        conn.execute(
            "ALTER TABLE plan_sessions ADD COLUMN question_session_id TEXT",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "iterative_loop_round") {
        conn.execute(
            "ALTER TABLE plan_sessions ADD COLUMN iterative_loop_round INTEGER NOT NULL DEFAULT 0",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "iterative_stop_reason") {
        conn.execute(
            "ALTER TABLE plan_sessions ADD COLUMN iterative_stop_reason TEXT",
            (),
        )
        .await?;
    }
    if !has_col(&cols, "iterative_loop_metadata_json") {
        conn.execute(
            "ALTER TABLE plan_sessions ADD COLUMN iterative_loop_metadata_json TEXT",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_question_sessions_belief(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(question_sessions)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "belief_state_json") {
        conn.execute(
            "ALTER TABLE question_sessions ADD COLUMN belief_state_json TEXT",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn align_agent_events(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(agent_events)").await?;
    if cols.is_empty() {
        return Ok(());
    }

    if !has_col(&cols, "payload_json") {
        conn.execute("ALTER TABLE agent_events ADD COLUMN payload_json TEXT", ())
            .await?;
    }
    if !has_col(&cols, "cli_version") {
        conn.execute("ALTER TABLE agent_events ADD COLUMN cli_version TEXT", ())
            .await?;
    }

    if has_col(&cols, "payload") {
        conn.execute(
            "UPDATE agent_events SET payload_json = payload WHERE payload_json IS NULL AND payload IS NOT NULL",
            (),
        )
        .await?;
    }
    Ok(())
}

async fn migrate_published_news_news_id(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(published_news)").await?;
    if cols.is_empty() {
        return Ok(());
    }

    if has_col(&cols, "news_id") {
        return Ok(());
    }
    if !has_col(&cols, "id") {
        return Ok(());
    }

    let batch = r#"
CREATE TABLE published_news__ssot (
    news_id TEXT PRIMARY KEY,
    published_at_ms INTEGER NOT NULL,
    github_release_id TEXT,
    twitter_tweet_id TEXT,
    opencollective_update_id TEXT,
    content_sha3_256 TEXT
);
INSERT INTO published_news__ssot (news_id, published_at_ms, github_release_id, twitter_tweet_id, opencollective_update_id, content_sha3_256)
SELECT id, published_at_ms, github_release_id, twitter_tweet_id, opencollective_update_id, NULL FROM published_news;
DROP TABLE published_news;
ALTER TABLE published_news__ssot RENAME TO published_news;
"#;
    conn.execute_batch(batch).await?;
    Ok(())
}

async fn migrate_published_news_content_digest(conn: &Connection) -> Result<(), StoreError> {
    let cols = table_column_names(conn, "PRAGMA table_info(published_news)").await?;
    if cols.is_empty() {
        return Ok(());
    }
    if !has_col(&cols, "content_sha3_256") {
        conn.execute(
            "ALTER TABLE published_news ADD COLUMN content_sha3_256 TEXT",
            (),
        )
        .await?;
    }
    // libSQL/Turso: avoid correlated subquery in UPDATE (not supported in all builds).
    let mut rows = conn
        .query(
            "SELECT publication_id, content_sha3_256 FROM publication_manifests WHERE content_type = 'news' AND content_sha3_256 IS NOT NULL",
            (),
        )
        .await?;
    while let Some(r) = rows.next().await? {
        let pid: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        let dig: String = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        conn.execute(
            "UPDATE published_news SET content_sha3_256 = ?1 WHERE news_id = ?2 AND content_sha3_256 IS NULL",
            (dig, pid),
        )
        .await?;
    }
    Ok(())
}

async fn apply_knowledge_fts_cutover(conn: &Connection) -> Result<(), StoreError> {
    let mut has_fts5 = false;
    if let Ok(mut rows) = conn.query("PRAGMA compile_options", ()).await {
        while let Some(row) = rows.next().await? {
            let opt: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            if opt.contains("FTS5") || opt == "ENABLE_FTS5" {
                has_fts5 = true;
                break;
            }
        }
    }
    if !has_fts5 {
        return Ok(());
    }

    let batch = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_nodes_fts
USING fts5(
    id UNINDEXED,
    label,
    content,
    content='knowledge_nodes',
    content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS knowledge_nodes_fts_ai
AFTER INSERT ON knowledge_nodes BEGIN
    INSERT INTO knowledge_nodes_fts(rowid, id, label, content)
    VALUES (new.rowid, new.id, new.label, COALESCE(new.content, ''));
END;
CREATE TRIGGER IF NOT EXISTS knowledge_nodes_fts_ad
AFTER DELETE ON knowledge_nodes BEGIN
    INSERT INTO knowledge_nodes_fts(knowledge_nodes_fts, rowid, id, label, content)
    VALUES ('delete', old.rowid, old.id, old.label, COALESCE(old.content, ''));
END;
CREATE TRIGGER IF NOT EXISTS knowledge_nodes_fts_au
AFTER UPDATE ON knowledge_nodes BEGIN
    INSERT INTO knowledge_nodes_fts(knowledge_nodes_fts, rowid, id, label, content)
    VALUES ('delete', old.rowid, old.id, old.label, COALESCE(old.content, ''));
    INSERT INTO knowledge_nodes_fts(rowid, id, label, content)
    VALUES (new.rowid, new.id, new.label, COALESCE(new.content, ''));
END;
INSERT INTO knowledge_nodes_fts(rowid, id, label, content)
SELECT rowid, id, label, COALESCE(content, '')
FROM knowledge_nodes
WHERE rowid NOT IN (SELECT rowid FROM knowledge_nodes_fts);
"#;
    let _ = conn.execute_batch(batch).await;
    Ok(())
}

/// FTS5 over `search_document_chunks.body_text` for RAG-style chunk retrieval (mirrors knowledge_nodes_fts).
async fn apply_search_document_chunks_fts_cutover(conn: &Connection) -> Result<(), StoreError> {
    let mut has_fts5 = false;
    if let Ok(mut rows) = conn.query("PRAGMA compile_options", ()).await {
        while let Some(row) = rows.next().await? {
            let opt: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            if opt.contains("FTS5") || opt == "ENABLE_FTS5" {
                has_fts5 = true;
                break;
            }
        }
    }
    if !has_fts5 {
        return Ok(());
    }

    let batch = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS search_document_chunks_fts
USING fts5(
    body_text,
    content='search_document_chunks',
    content_rowid='rowid'
);
CREATE TRIGGER IF NOT EXISTS search_document_chunks_fts_ai
AFTER INSERT ON search_document_chunks BEGIN
    INSERT INTO search_document_chunks_fts(rowid, body_text)
    VALUES (new.rowid, new.body_text);
END;
CREATE TRIGGER IF NOT EXISTS search_document_chunks_fts_ad
AFTER DELETE ON search_document_chunks BEGIN
    INSERT INTO search_document_chunks_fts(search_document_chunks_fts, rowid, body_text)
    VALUES ('delete', old.rowid, old.body_text);
END;
CREATE TRIGGER IF NOT EXISTS search_document_chunks_fts_au
AFTER UPDATE ON search_document_chunks BEGIN
    INSERT INTO search_document_chunks_fts(search_document_chunks_fts, rowid, body_text)
    VALUES ('delete', old.rowid, old.body_text);
    INSERT INTO search_document_chunks_fts(rowid, body_text)
    VALUES (new.rowid, new.body_text);
END;
INSERT INTO search_document_chunks_fts(rowid, body_text)
SELECT rowid, body_text FROM search_document_chunks
WHERE rowid NOT IN (SELECT rowid FROM search_document_chunks_fts);
"#;
    let _ = conn.execute_batch(batch).await;
    Ok(())
}
