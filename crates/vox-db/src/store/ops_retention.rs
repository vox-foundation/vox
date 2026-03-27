//! Time-based retention helpers for `vox db prune-plan` / `prune-apply`.
//!
//! `table` / `time_column` are validated (`[A-Za-z0-9_]+`) then double-quoted for SQLite.

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
        let n = self.conn.execute(&sql, ()).await?;
        Ok(n)
    }
}
