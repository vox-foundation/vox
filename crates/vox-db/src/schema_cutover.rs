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
    align_question_sessions_belief(conn).await?;
    align_plan_sessions_iterative(conn).await?;
    align_agent_events(conn).await?;
    migrate_published_news_news_id(conn).await?;
    migrate_published_news_content_digest(conn).await?;
    apply_performance_indexes(conn).await?;
    apply_knowledge_fts_cutover(conn).await?;
    apply_search_document_chunks_fts_cutover(conn).await?;
    crate::ludus_schema_cutover::apply_ludus_gamify_cutover(conn).await?;
    Ok(())
}

/// Idempotent composite / reporting indexes (safe on legacy DBs; `IF NOT EXISTS`).
async fn apply_performance_indexes(conn: &Connection) -> Result<(), StoreError> {
    let batch = r#"
CREATE INDEX IF NOT EXISTS idx_memories_agent_created ON memories(agent_id, created_at);
CREATE INDEX IF NOT EXISTS idx_behavior_user_created ON behavior_events(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_codex_change_log_topic_id ON codex_change_log(topic, id);
CREATE INDEX IF NOT EXISTS idx_embeddings_source_created ON embeddings(source_type, created_at);
CREATE INDEX IF NOT EXISTS idx_a2a_ack_created ON a2a_messages(acknowledged, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_oplog_repo_ts ON agent_oplog(repository_id, timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_news_publish_attempts_news ON news_publish_attempts(news_id);
CREATE INDEX IF NOT EXISTS idx_publication_status_events_pub_id ON publication_status_events(publication_id, id);
"#;
    conn.execute_batch(batch).await?;
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
