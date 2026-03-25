//! Idempotent alignment for databases created before a schema SSOT cutover.
//!
//! [`crate::store::open::VoxDb::migrate`] runs baseline DDL first (`CREATE IF NOT EXISTS`), which
//! does not add columns to existing tables. This module applies additive fixes and one-time table
//! rebuilds when detected via `PRAGMA table_info`.

use turso::Connection;

use crate::store::types::StoreError;

async fn table_column_names(conn: &Connection, pragma_sql: &'static str) -> Result<Vec<String>, StoreError> {
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
    align_agent_events(conn).await?;
    migrate_published_news_news_id(conn).await?;
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
    opencollective_update_id TEXT
);
INSERT INTO published_news__ssot (news_id, published_at_ms, github_release_id, twitter_tweet_id, opencollective_update_id)
SELECT id, published_at_ms, github_release_id, twitter_tweet_id, opencollective_update_id FROM published_news;
DROP TABLE published_news;
ALTER TABLE published_news__ssot RENAME TO published_news;
"#;
    conn.execute_batch(batch).await?;
    Ok(())
}
