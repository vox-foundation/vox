use crate::VoxDb;
use crate::store::StoreError;
use crate::store::types::{VisusAuditLogRow, VisusBaselineRow};

impl VoxDb {
    /// Save a visual baseline (golden state) for a target.
    pub async fn upsert_visus_baseline(&self, row: VisusBaselineRow) -> Result<(), StoreError> {
        let sql = r#"
            INSERT OR REPLACE INTO visus_baselines (
                id, target_url, viewport, theme, screenshot_cas, ax_tree_cas, metadata_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        self.conn
            .execute(
                sql,
                (
                    row.id,
                    row.target_url,
                    row.viewport,
                    row.theme,
                    row.screenshot_cas,
                    row.ax_tree_cas,
                    row.metadata_json,
                ),
            )
            .await?;

        Ok(())
    }

    /// Get the latest visual baseline for a target and configuration.
    pub async fn get_visus_baseline(
        &self,
        target_url: &str,
        viewport: &str,
        theme: &str,
    ) -> Result<Option<VisusBaselineRow>, StoreError> {
        let sql = r#"
            SELECT id, target_url, viewport, theme, screenshot_cas, ax_tree_cas, metadata_json, created_at
            FROM visus_baselines
            WHERE target_url = ? AND viewport = ? AND theme = ?
        "#;

        let mut rows = self.conn.query(sql, (target_url, viewport, theme)).await?;

        if let Some(row) = rows.next().await? {
            Ok(Some(VisusBaselineRow {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                target_url: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                viewport: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                theme: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                screenshot_cas: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                ax_tree_cas: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                metadata_json: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at: row.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Log a visual audit outcome.
    pub async fn log_visus_audit(&self, row: VisusAuditLogRow) -> Result<(), StoreError> {
        let sql = r#"
            INSERT INTO visus_audit_log (
                id, baseline_id, target_url, outcome, findings_json, screenshot_cas
            ) VALUES (?, ?, ?, ?, ?, ?)
        "#;

        self.conn
            .execute(
                sql,
                (
                    row.id,
                    row.baseline_id,
                    row.target_url,
                    row.outcome,
                    row.findings_json,
                    row.screenshot_cas,
                ),
            )
            .await?;

        Ok(())
    }

    /// List visual audit logs with an optional limit.
    pub async fn list_visus_audit_logs(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<VisusAuditLogRow>, StoreError> {
        let limit = limit.unwrap_or(100);
        let sql = format!(
            "SELECT id, baseline_id, target_url, outcome, findings_json, screenshot_cas, created_at FROM visus_audit_log ORDER BY created_at DESC LIMIT {limit}"
        );

        let mut rows = self.conn.query(&sql, ()).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(VisusAuditLogRow {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                baseline_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                target_url: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                outcome: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                findings_json: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                screenshot_cas: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }

        Ok(results)
    }
}
