//! TOESTUB persistence on top of Codex (idempotent auxiliary tables).

use crate::VoxDb;
use crate::store::StoreError;
use turso::params;

async fn ensure_tables(db: &VoxDb) -> Result<(), StoreError> {
    db.connection()
        .execute_batch(crate::schema::domains::toestub_build::SCHEMA_TOESTUB_BUILD)
        .await?;
    Ok(())
}

/// Import one suppression row.
pub async fn add_suppression(
    db: &VoxDb,
    path: &str,
    line: i64,
    rule_id: &str,
    reason: Option<&str>,
) -> Result<(), StoreError> {
    ensure_tables(db).await?;
    db
        .connection()
        .execute(
            "INSERT INTO toestub_suppressions (path, line, rule_id, reason) VALUES (?1, ?2, ?3, ?4)",
            params![path, line, rule_id, reason],
        )
        .await?;
    Ok(())
}

/// Load baseline JSON by logical name (any `run_scope`).
pub async fn load_baseline(db: &VoxDb, name: &str) -> Result<Option<(String, String)>, StoreError> {
    ensure_tables(db).await?;
    let mut rows = db.connection()
        .query(
            "SELECT run_scope, findings_json FROM toestub_baselines WHERE name = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![name],
        )
        .await?;
    let row = rows.next().await?;
    match row {
        Some(r) => Ok(Some((r.get::<String>(0)?, r.get::<String>(1)?))),
        None => Ok(None),
    }
}

/// Upsert a TOESTUB baseline row (`toestub_baselines`).
pub async fn save_baseline(
    db: &VoxDb,
    name: &str,
    run_scope: &str,
    findings_json: &str,
) -> Result<(), StoreError> {
    ensure_tables(db).await?;
    db.connection()
        .execute(
            "INSERT OR REPLACE INTO toestub_baselines (name, run_scope, findings_json, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![name, run_scope, findings_json],
        )
        .await?;
    Ok(())
}

/// Latest `toestub_task_queue` row for `user_id`: `(total_findings, fix_suggestions_json)`.
pub async fn load_latest_task_queue(
    db: &VoxDb,
    user_id: &str,
) -> Result<Option<(i64, String)>, StoreError> {
    ensure_tables(db).await?;
    let mut rows = db
        .connection()
        .query(
            "SELECT total_findings, fix_suggestions_json FROM toestub_task_queue
             WHERE user_id = ?1 ORDER BY updated_at DESC LIMIT 1",
            params![user_id],
        )
        .await?;
    let row = rows.next().await?;
    match row {
        Some(r) => Ok(Some((r.get::<i64>(0)?, r.get::<String>(1)?))),
        None => Ok(None),
    }
}

/// Upsert a TOESTUB task queue snapshot (`toestub_task_queue`).
pub async fn save_task_queue(
    db: &VoxDb,
    user_id: &str,
    run_scope: &str,
    total_findings: i64,
    fix_suggestions_json: &str,
) -> Result<(), StoreError> {
    ensure_tables(db).await?;
    db
        .connection()
        .execute(
            "INSERT OR REPLACE INTO toestub_task_queue (user_id, run_scope, total_findings, fix_suggestions_json, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![user_id, run_scope, total_findings, fix_suggestions_json],
        )
        .await?;
    Ok(())
}

/// Read cached TOESTUB findings JSON for a `(path, content_hash, rules_version)` triple.
pub fn get_file_cache_blocking(
    db: &VoxDb,
    path: &str,
    content_hash: &str,
    rules_version: &str,
) -> Result<Option<String>, StoreError> {
    db.block_on(async {
        ensure_tables(db).await?;
        let mut rows = db.connection()
            .query(
                "SELECT findings_json FROM toestub_file_cache WHERE path = ?1 AND content_hash = ?2 AND rules_version = ?3",
                params![path, content_hash, rules_version],
            )
            .await?;
        let row = rows.next().await?;
        match row {
            Some(r) => Ok(Some(r.get::<String>(0)?)),
            None => Ok(None),
        }
    })
}

/// Upsert TOESTUB per-file cache (`toestub_file_cache`).
pub fn set_file_cache_blocking(
    db: &VoxDb,
    path: &str,
    content_hash: &str,
    rules_version: &str,
    findings_json: &str,
) -> Result<(), StoreError> {
    db.block_on(async {
        ensure_tables(db).await?;
        db
            .connection()
            .execute(
                "INSERT OR REPLACE INTO toestub_file_cache (path, content_hash, rules_version, findings_json, updated_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                params![path, content_hash, rules_version, findings_json],
            )
            .await?;
        Ok(())
    })
}

/// List suppressions for `path` as `(line, rule_id, reason)`.
pub fn list_suppressions_blocking(
    db: &VoxDb,
    path: &str,
) -> Result<Vec<(i64, String, Option<String>)>, StoreError> {
    db.block_on(async {
        ensure_tables(db).await?;
        let mut rows = db.connection()
            .query(
                "SELECT line, rule_id, reason FROM toestub_suppressions WHERE path = ?1 ORDER BY line ASC, id ASC",
                params![path],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<i64>(0)?,
                row.get::<String>(1)?,
                row.get::<Option<String>>(2)?,
            ));
        }
        Ok(out)
    })
}
