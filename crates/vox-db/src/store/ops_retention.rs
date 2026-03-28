//! Time-based retention helpers for `vox db prune-plan` / `vox db prune-apply`.
//!
//! `table` / `time_column` are validated (`[A-Za-z0-9_]+`) then double-quoted for SQLite.
//!
//! - **days**: `time_column` is compared with SQLite `datetime('now', '-N day')` (TEXT/ISO-style).
//! - **ms_days** (via [`VoxDb::retention_count_older_than_ms_cutoff`] /
//!   [`VoxDb::retention_delete_all_ms_older_than_days`]): `time_column` is Unix millis (`INTEGER`);
//!   deletes use a Turso-safe row-by-row pattern (no `IN (SELECT …)`).

use turso::params;

use crate::store::types::StoreError;

fn validate_retention_ident(name: &str) -> Result<(), StoreError> {
    if name.is_empty() || name.len() > 128 {
        return Err(StoreError::Db(
            "retention table/column name empty or too long".into(),
        ));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(StoreError::Db(format!(
            "retention identifier must be [A-Za-z0-9_]+: {name:?}"
        )));
    }
    Ok(())
}

fn quote_sqlite_ident(name: &str) -> Result<String, StoreError> {
    validate_retention_ident(name)?;
    let mut s = String::with_capacity(name.len() + 2);
    s.push('"');
    for c in name.chars() {
        if c == '"' {
            s.push_str("\"\"");
        } else {
            s.push(c);
        }
    }
    s.push('"');
    Ok(s)
}

impl crate::VoxDb {
    /// Cutoff for “older than N days” when the compared column stores **Unix millis** (`INTEGER`).
    /// Rows match retention when `time_ms_column <` this value.
    pub fn retention_cutoff_ms_exclusive_for_days(days: u32) -> i64 {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        now_ms.saturating_sub((days as i64).saturating_mul(86_400_000))
    }

    /// `SELECT COUNT(*)` where `time_ms_column < cutoff_ms_exclusive` (INTEGER Unix millis).
    pub async fn retention_count_older_than_ms_cutoff(
        &self,
        table: &str,
        time_ms_column: &str,
        cutoff_ms_exclusive: i64,
    ) -> Result<i64, StoreError> {
        let tq = quote_sqlite_ident(table)?;
        let cq = quote_sqlite_ident(time_ms_column)?;
        let sql = format!("SELECT COUNT(*) FROM {tq} WHERE {cq} < ?1");
        let mut rows = self.conn.query(&sql, params![cutoff_ms_exclusive]).await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("retention ms count returned no rows".into()))?;
        let n: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(n)
    }

    /// Delete up to `max_rows` rows with `time_ms_column < cutoff_ms_exclusive`, one row per
    /// statement (Turso-compatible).
    pub async fn retention_delete_ms_older_than_chunk(
        &self,
        table: &str,
        time_ms_column: &str,
        cutoff_ms_exclusive: i64,
        max_rows: u64,
    ) -> Result<u64, StoreError> {
        let tq = quote_sqlite_ident(table)?;
        let cq = quote_sqlite_ident(time_ms_column)?;
        let select_sql =
            format!("SELECT rowid FROM {tq} WHERE {cq} < ?1 ORDER BY rowid ASC LIMIT 1");
        let delete_sql = format!("DELETE FROM {tq} WHERE rowid = ?1");
        let cap = max_rows.clamp(1, 100_000) as usize;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| {
                let select_sql = select_sql.clone();
                let delete_sql = delete_sql.clone();
                async move {
                    let mut deleted: u64 = 0;
                    for _ in 0..cap {
                        let mut rows = conn
                            .query(&select_sql, params![cutoff_ms_exclusive])
                            .await?;
                        let Some(r) = rows.next().await? else {
                            break;
                        };
                        let rowid: i64 = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
                        let n = conn.execute(&delete_sql, params![rowid]).await?;
                        deleted += n as u64;
                    }
                    Ok(deleted)
                }
            })
            .await
    }

    /// Delete **all** rows older than `days` (millis column), batched in chunks of up to 100k rows
    /// per inner pass. Stops after 10k outer iterations to avoid unbounded work in one CLI call.
    pub async fn retention_delete_all_ms_older_than_days(
        &self,
        table: &str,
        time_ms_column: &str,
        days: u32,
    ) -> Result<u64, StoreError> {
        let cutoff = Self::retention_cutoff_ms_exclusive_for_days(days);
        let mut total: u64 = 0;
        for _ in 0..10_000 {
            let n = self
                .retention_delete_ms_older_than_chunk(table, time_ms_column, cutoff, 100_000)
                .await?;
            total = total.saturating_add(n);
            if n == 0 {
                break;
            }
        }
        Ok(total)
    }

    /// `SELECT COUNT(*)` for rows where `time_column < datetime('now', '-days day')`.
    pub async fn retention_count_older_than_days(
        &self,
        table: &str,
        time_column: &str,
        days: u32,
    ) -> Result<i64, StoreError> {
        let tq = quote_sqlite_ident(table)?;
        let cq = quote_sqlite_ident(time_column)?;
        let sql = format!("SELECT COUNT(*) FROM {tq} WHERE {cq} < datetime('now', '-{days} day')");
        let mut rows = self.conn.query(&sql, ()).await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("retention count returned no rows".into()))?;
        let n: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(n)
    }

    /// `DELETE` rows where `time_column < datetime('now', '-days day')`; returns affected count.
    pub async fn retention_delete_older_than_days(
        &self,
        table: &str,
        time_column: &str,
        days: u32,
    ) -> Result<u64, StoreError> {
        let tq = quote_sqlite_ident(table)?;
        let cq = quote_sqlite_ident(time_column)?;
        let sql = format!("DELETE FROM {tq} WHERE {cq} < datetime('now', '-{days} day')");
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let n = conn.execute(&sql, ()).await?;
                Ok::<_, StoreError>(n)
            })
            .await
    }

    /// Rows where `expires_at_column` is non-NULL and lexicographically before `datetime('now')`
    /// (TEXT timestamps from SQLite `datetime('now')` / `datetime('now', '+N …')`).
    pub async fn retention_count_expires_lt_now(
        &self,
        table: &str,
        expires_at_column: &str,
    ) -> Result<i64, StoreError> {
        let tq = quote_sqlite_ident(table)?;
        let cq = quote_sqlite_ident(expires_at_column)?;
        let sql = format!(
            "SELECT COUNT(*) FROM {tq} WHERE {cq} IS NOT NULL AND {cq} < datetime('now')"
        );
        let mut rows = self.conn.query(&sql, ()).await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("retention expires count returned no rows".into()))?;
        let n: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(n)
    }

    /// Delete rows past logical expiry (`expires_at < datetime('now')`, non-NULL only).
    pub async fn retention_delete_expires_lt_now(
        &self,
        table: &str,
        expires_at_column: &str,
    ) -> Result<u64, StoreError> {
        let tq = quote_sqlite_ident(table)?;
        let cq = quote_sqlite_ident(expires_at_column)?;
        let sql = format!("DELETE FROM {tq} WHERE {cq} IS NOT NULL AND {cq} < datetime('now')");
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let n = conn.execute(&sql, ()).await?;
                Ok::<_, StoreError>(n)
            })
            .await
    }
}
