use crate::{
    VoxDb,
    exec_time_telemetry::{ExecOutcome, ExecTimeRecord, ToolLatencyProfile},
    store::StoreError,
};
use std::time::{SystemTime, UNIX_EPOCH};

impl VoxDb {
    pub async fn record_exec_time(&self, record: &ExecTimeRecord<'_>) -> Result<(), StoreError> {
        if let Some(writer) = &self.writer {
            return writer
                .insert_exec_history(
                    record.tool_key.to_string(),
                    record.repository_id.to_string(),
                    None, // session_id not in ExecTimeRecord
                    record.duration_ms as i64,
                    record
                        .vendor_cost_usd_micros
                        .map(|v| v as f64 / 1_000_000.0),
                    record.compute_tokens_used.map(|v| v as i64),
                    None, // output_tokens not separate in ExecTimeRecord
                )
                .await;
        }

        let tool_key = record.tool_key.to_string();
        let repository_id = record.repository_id.to_string();
        let outcome = record.outcome.as_str().to_string();
        let duration_ms = record.duration_ms as i64;
        let timeout_budget_ms = record.timeout_budget_ms.map(|v| v as i64);
        let compute_tokens_used = record.compute_tokens_used.map(|v| v as i64);
        let vendor_cost_usd_micros = record.vendor_cost_usd_micros;
        let attention_cost_ms = record.attention_cost_ms.map(|v| v as i64);

        self.conn.execute(
            "INSERT INTO agent_exec_history 
                (tool_key, repository_id, outcome, duration_ms, timeout_budget_ms, compute_tokens_used, vendor_cost_usd_micros, attention_cost_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            turso::params![
                tool_key, repository_id, outcome, duration_ms, timeout_budget_ms,
                compute_tokens_used, vendor_cost_usd_micros, attention_cost_ms
            ],
        ).await?;
        Ok(())
    }

    pub async fn query_historical_exec_time(
        &self,
        tool_key: Option<&str>,
        repository_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, StoreError> {
        let mut sql = String::from(
            "SELECT tool_key, repository_id, outcome, duration_ms, timeout_budget_ms, compute_tokens_used, vendor_cost_usd_micros, attention_cost_ms, recorded_at \
             FROM agent_exec_history WHERE 1=1",
        );
        let mut params = Vec::<turso::Value>::new();

        if let Some(tk) = tool_key {
            sql.push_str(" AND tool_key = ?");
            params.push(tk.to_string().into());
        }
        if let Some(repo) = repository_id {
            sql.push_str(" AND repository_id = ?");
            params.push(repo.to_string().into());
        }

        sql.push_str(" ORDER BY recorded_at DESC LIMIT ?");
        params.push(limit.into());

        let mut rows = self.conn.query(&sql, params).await?;
        let mut out = Vec::new();

        while let Some(row) = rows.next().await? {
            let rs_tool_key: String = row.get(0).unwrap_or_default();
            let rs_repository_id: String = row.get(1).unwrap_or_default();
            let rs_outcome: String = row.get(2).unwrap_or_default();
            let rs_duration_ms: i64 = row.get(3).unwrap_or(0);
            let rs_timeout_ms: Option<i64> = row.get(4).ok();
            let rs_compute: Option<i64> = row.get(5).ok();
            let rs_vendor: Option<i64> = row.get(6).ok();
            let rs_attention: Option<i64> = row.get(7).ok();
            let rs_recorded_at: i64 = row.get(8).unwrap_or(0);

            out.push(serde_json::json!({
                "tool_key": rs_tool_key,
                "repository_id": rs_repository_id,
                "outcome": rs_outcome,
                "duration_ms": rs_duration_ms,
                "timeout_budget_ms": rs_timeout_ms,
                "compute_tokens_used": rs_compute,
                "vendor_cost_usd_micros": rs_vendor,
                "attention_cost_ms": rs_attention,
                "recorded_at": rs_recorded_at,
            }));
        }
        Ok(out)
    }

    pub async fn query_tool_latency(
        &self,
        tool_key: &str,
        repository_id: &str,
        window_days: u32,
        safety_multiplier: f64,
    ) -> Result<Option<ToolLatencyProfile>, StoreError> {
        let tool_key = tool_key.to_string();
        let repository_id = repository_id.to_string();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let min_recorded_at = now.saturating_sub(window_days as i64 * 86_400_000);

        // Query A: aggregate stats for successful runs only
        let sql_a = "SELECT COUNT(*) AS cnt, IFNULL(AVG(duration_ms), 0.0) AS avg_ms, IFNULL(MAX(duration_ms), 0) AS max_ms
             FROM agent_exec_history
             WHERE tool_key = ?1 AND repository_id = ?2 AND outcome = 'success' AND recorded_at >= ?3";
        let mut rows_a = self
            .conn
            .query(
                &sql_a,
                turso::params![tool_key.clone(), repository_id.clone(), min_recorded_at],
            )
            .await?;
        let Some(row_a) = rows_a.next().await? else {
            return Ok(None);
        };
        let cnt: i64 = row_a.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        let avg_ms: f64 = row_a.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        let max_ms: i64 = row_a.get(2).map_err(|e| StoreError::Db(e.to_string()))?;

        if cnt == 0 {
            return Ok(None);
        }

        // Query B: P90 via OFFSET
        let offset = std::cmp::max(0, (cnt as f64 * 0.9).ceil() as i64 - 1);
        let sql_b = "SELECT duration_ms
             FROM agent_exec_history
             WHERE tool_key = ?1 AND repository_id = ?2 AND outcome = 'success' AND recorded_at >= ?3
             ORDER BY duration_ms ASC
             LIMIT 1 OFFSET ?4";
        let mut rows_b = self
            .conn
            .query(
                &sql_b,
                turso::params![
                    tool_key.clone(),
                    repository_id.clone(),
                    min_recorded_at,
                    offset
                ],
            )
            .await?;
        let Some(row_b) = rows_b.next().await? else {
            return Ok(None);
        };
        let p90_val: i64 = row_b.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        let p90_ms = p90_val as f64;

        // Query C: timeout_rate over all outcomes
        let sql_c = "SELECT COUNT(*) AS total, SUM(CASE WHEN outcome = 'timeout' THEN 1 ELSE 0 END) AS timed_out
             FROM agent_exec_history
             WHERE tool_key = ?1 AND repository_id = ?2 AND recorded_at >= ?3";
        let mut rows_c = self
            .conn
            .query(
                &sql_c,
                turso::params![tool_key.clone(), repository_id.clone(), min_recorded_at],
            )
            .await?;
        let Some(row_c) = rows_c.next().await? else {
            return Ok(None);
        };
        let total: i64 = row_c.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        let timed_out: Option<i64> = row_c.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        let timed_out_val = timed_out.unwrap_or(0);
        let timeout_rate = if total > 0 {
            timed_out_val as f64 / total as f64
        } else {
            0.0
        };
        let recommended_budget_ms = (p90_ms * safety_multiplier).ceil() as u64;

        Ok(Some(ToolLatencyProfile {
            tool_key,
            sample_count: cnt,
            avg_ms,
            p90_ms,
            max_ms,
            timeout_rate,
            recommended_budget_ms,
        }))
    }

    /// Insert a timeout observation (agent exceeded its planned wait window).
    /// `attempted_budget_ms` is stored as `duration_ms` (the time we waited).
    pub async fn record_exec_timeout(
        &self,
        tool_key: &str,
        repository_id: &str,
        attempted_budget_ms: u64,
    ) -> Result<(), StoreError> {
        let record = ExecTimeRecord {
            tool_key,
            repository_id,
            outcome: ExecOutcome::Timeout,
            duration_ms: attempted_budget_ms,
            timeout_budget_ms: Some(attempted_budget_ms),
            compute_tokens_used: None,
            vendor_cost_usd_micros: None,
            attention_cost_ms: None,
        };
        self.record_exec_time(&record).await
    }
}
