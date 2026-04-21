//! Telemetry and Scoreboard operations for [`crate::VoxDb`] (Arca / Turso).

use turso::params;
use crate::store::types::{StoreError, ModelScoreboardRow};

impl crate::VoxDb {
    /// Retrieve the current model scoreboard for a specific window.
    pub async fn get_model_scoreboard(
        &self,
        window_days: i64,
    ) -> Result<Vec<ModelScoreboardRow>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        
        breaker
            .call(move || {
                let conn = conn.clone();
                async move {
                    let mut rows = conn
                        .query(
                            "SELECT 
                                model_id, task_category, strength_tag, window_days, 
                                n_calls, success_rate, p50_latency_ms, p99_latency_ms, 
                                cost_per_success_usd, quality_score, updated_at_ms,
                                success_count, cumulative_cost_usd
                             FROM model_scoreboard
                             WHERE window_days = ?1",
                            params![window_days],
                        )
                        .await?;

                    let mut out = Vec::new();
                    while let Some(row) = rows.next().await? {
                        out.push(ModelScoreboardRow {
                            model_id: row.get(0)?,
                            task_category: row.get(1)?,
                            strength_tag: row.get(2)?,
                            window_days: row.get(3)?,
                            n_calls: row.get(4)?,
                            success_rate: row.get(5)?,
                            p50_latency_ms: row.get(6)?,
                            p99_latency_ms: row.get(7)?,
                            cost_per_success_usd: row.get(8)?,
                            quality_score: row.get(9)?,
                            updated_at_ms: row.get(10)?,
                            success_count: row.get(11)?,
                            cumulative_cost_usd: row.get(12)?,
                        });
                    }
                    Ok::<_, StoreError>(out)
                }
            })
            .await
    }

    /// Upsert a model scoreboard entry.
    pub async fn upsert_model_scoreboard(
        &self,
        row: ModelScoreboardRow,
    ) -> Result<(), StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        
        breaker
            .call(move || {
                let conn = conn.clone();
                async move {
                    conn.execute(
                        "INSERT INTO model_scoreboard (
                            model_id, task_category, strength_tag, window_days, 
                            n_calls, success_rate, p50_latency_ms, p99_latency_ms, 
                            cost_per_success_usd, quality_score, updated_at_ms,
                            success_count, cumulative_cost_usd
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                         ON CONFLICT(model_id, task_category, strength_tag, window_days) DO UPDATE SET
                            n_calls = excluded.n_calls,
                            success_rate = excluded.success_rate,
                            p50_latency_ms = excluded.p50_latency_ms,
                            p99_latency_ms = excluded.p99_latency_ms,
                            cost_per_success_usd = excluded.cost_per_success_usd,
                            quality_score = excluded.quality_score,
                            updated_at_ms = excluded.updated_at_ms,
                            success_count = excluded.success_count,
                            cumulative_cost_usd = excluded.cumulative_cost_usd",
                        params![
                            row.model_id.as_str(),
                            row.task_category.as_str(),
                            row.strength_tag.as_str(),
                            row.window_days,
                            row.n_calls,
                            row.success_rate,
                            row.p50_latency_ms,
                            row.p99_latency_ms,
                            row.cost_per_success_usd,
                            row.quality_score,
                            row.updated_at_ms,
                            row.success_count,
                            row.cumulative_cost_usd,
                        ],
                    )
                    .await?;
                    Ok(())
                }
            })
            .await
    }

    /// Perform a batch rollup of telemetry into the scoreboard for a specific window.
    pub async fn rollup_model_scoreboard(&self, window_days: i64) -> Result<usize, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        breaker
            .call(move || {
                let conn = conn.clone();
                async move {
                    // This query aggregates interactions and joins feedback (averaging ratings if present)
                    // rating is 0..1 (binary) or 0..5 (thumbs/stars), we normalize it to quality.
                    let sql = format!(
                        "INSERT INTO model_scoreboard (
                            model_id, task_category, strength_tag, window_days,
                            n_calls, success_rate, p50_latency_ms, p99_latency_ms,
                            cost_per_success_usd, quality_score, updated_at_ms,
                            success_count, cumulative_cost_usd
                        )
                        WITH interaction_stats AS (
                            SELECT 
                                id,
                                model_version, 
                                task_category, 
                                strength_tag,
                                success,
                                latency_ms,
                                cost_usd
                            FROM llm_interactions
                            WHERE created_at >= datetime('now', '-{} days')
                        ),
                        feedback_agg AS (
                            SELECT interaction_id, AVG(rating) as rating 
                            FROM llm_feedback 
                            GROUP BY interaction_id
                        )
                        SELECT 
                            s.model_version, 
                            s.task_category, 
                            s.strength_tag, 
                            ?1,
                            COUNT(*),
                            AVG(CAST(s.success AS REAL)),
                            AVG(CAST(s.latency_ms AS REAL)),
                            MAX(s.latency_ms),
                            SUM(s.cost_usd) / NULLIF(SUM(s.success), 0) as cost_per_success_usd,
                            COALESCE(AVG(CAST(f.rating AS REAL) / 5.0), 1.0),
                            ?2,
                            SUM(s.success),
                            COALESCE(SUM(s.cost_usd), 0.0)
                        FROM interaction_stats s
                        LEFT JOIN feedback_agg f ON s.id = f.interaction_id
                        GROUP BY s.model_version, s.task_category, s.strength_tag
                        ON CONFLICT(model_id, task_category, strength_tag, window_days) DO UPDATE SET
                            n_calls = excluded.n_calls,
                            success_rate = excluded.success_rate,
                            p50_latency_ms = excluded.p50_latency_ms,
                            p99_latency_ms = excluded.p99_latency_ms,
                            cost_per_success_usd = excluded.cost_per_success_usd,
                            quality_score = excluded.quality_score,
                            updated_at_ms = excluded.updated_at_ms,
                            success_count = excluded.success_count,
                            cumulative_cost_usd = excluded.cumulative_cost_usd",
                        window_days
                    );

                    let affected = conn.execute(&sql, params![window_days, now_ms]).await?;
                    Ok(affected as usize)
                }
            })
            .await
    }

    /// Retrieve the most recent trace_id for a given task category.
    pub async fn get_last_interaction_trace_id(
        &self,
        task_category: &str,
    ) -> Result<Option<String>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let category = task_category.to_string();

        breaker
            .call(move || {
                let conn = conn.clone();
                async move {
                    let mut rows = conn
                        .query(
                            "SELECT trace_id
                             FROM llm_interactions
                             WHERE task_category = ?1 AND trace_id IS NOT NULL
                             ORDER BY created_at DESC
                             LIMIT 1",
                            params![category],
                        )
                        .await?;

                    if let Some(row) = rows.next().await? {
                        let tid: Option<String> = row.get(0)?;
                        Ok::<_, StoreError>(tid)
                    } else {
                        Ok::<_, StoreError>(None)
                    }
                }
            })
            .await
    }

    /// Retrieve the current model pricing catalog (confident rows).
    pub async fn get_pricing_catalog(&self) -> Result<Vec<crate::store::types::ModelPricingCatalogRow>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        
        breaker
            .call(move || {
                let conn = conn.clone();
                async move {
                    let mut rows = conn
                        .query(
                            "SELECT 
                                model_id, provider, observed_blended_per_1k, observed_input_per_1k, 
                                observed_output_per_1k, catalog_input_per_1k, catalog_output_per_1k, 
                                n_provider_reported, n_estimated, n_free, confidence, 
                                last_observed_at_ms, updated_at_ms
                             FROM model_pricing_catalog",
                            (),
                        )
                        .await?;

                    let mut out = Vec::new();
                    while let Some(row) = rows.next().await? {
                        out.push(crate::store::types::ModelPricingCatalogRow {
                            model_id: row.get(0)?,
                            provider: row.get(1)?,
                            observed_blended_per_1k: row.get(2)?,
                            observed_input_per_1k: row.get(3)?,
                            observed_output_per_1k: row.get(4)?,
                            catalog_input_per_1k: row.get(5)?,
                            catalog_output_per_1k: row.get(6)?,
                            n_provider_reported: row.get(7)?,
                            n_estimated: row.get(8)?,
                            n_free: row.get(9)?,
                            confidence: row.get(10)?,
                            last_observed_at_ms: row.get(11)?,
                            updated_at_ms: row.get(12)?,
                        });
                    }
                    Ok::<_, StoreError>(out)
                }
            })
            .await
    }

    /// Perform a batch rollup of telemetry into the pricing catalog.
    pub async fn rollup_pricing_catalog(&self) -> Result<usize, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        // Ensure collection table exists before we try to select from it.
        if let Err(e) = self.collection("provider_usage").ensure_table().await {
            tracing::warn!(error = %e, "Failed to ensure provider_usage collection table exists");
        }

        breaker
            .call(move || {
                let conn = conn.clone();
                async move {
                    let sql = r#"
                        INSERT INTO model_pricing_catalog (
                            model_id, provider, observed_blended_per_1k, 
                            catalog_input_per_1k, catalog_output_per_1k,
                            n_provider_reported, n_estimated, n_free, confidence, 
                            last_observed_at_ms, updated_at_ms
                        )
                        WITH raw_usage AS (
                            SELECT 
                                json_extract(_data, '$.model') as model_id,
                                json_extract(_data, '$.provider') as provider,
                                CAST(json_extract(_data, '$.input_tokens') AS INTEGER) as input_tokens,
                                CAST(json_extract(_data, '$.output_tokens') AS INTEGER) as output_tokens,
                                CAST(json_extract(_data, '$.cost_usd') AS REAL) as cost_usd,
                                json_extract(_data, '$.cost_source') as cost_source,
                                CAST(json_extract(_data, '$.timestamp_ms') AS INTEGER) as timestamp_ms
                            FROM provider_usage
                        ),
                        agg_usage AS (
                            SELECT 
                                model_id,
                                provider,
                                SUM(CASE WHEN cost_source = 'provider_reported' AND cost_usd > 0.0 THEN cost_usd ELSE 0 END) as sum_reported_cost,
                                SUM(CASE WHEN cost_source = 'provider_reported' AND cost_usd > 0.0 THEN input_tokens + output_tokens ELSE 0 END) as sum_reported_tokens,
                                SUM(CASE WHEN cost_source = 'provider_reported' AND cost_usd > 0.0 THEN 1 ELSE 0 END) as n_provider_reported,
                                SUM(CASE WHEN cost_source = 'estimated' THEN 1 ELSE 0 END) as n_estimated,
                                SUM(CASE WHEN cost_source = 'provider_reported' AND cost_usd = 0.0 THEN 1 ELSE 0 END) as n_free,
                                MAX(timestamp_ms) as last_observed_at_ms
                            FROM raw_usage
                            WHERE model_id IS NOT NULL AND provider IS NOT NULL
                            GROUP BY model_id, provider
                        )
                        SELECT 
                            model_id,
                            provider,
                            CASE WHEN sum_reported_tokens > 0 THEN (sum_reported_cost / sum_reported_tokens) * 1000.0 ELSE NULL END as observed_blended_per_1k,
                            0.0 as catalog_input_per_1k,
                            0.0 as catalog_output_per_1k,
                            n_provider_reported,
                            n_estimated,
                            n_free,
                            CASE 
                                WHEN n_provider_reported >= 100 THEN 'high'
                                WHEN n_provider_reported >= 20 THEN 'medium'
                                ELSE 'low'
                            END as confidence,
                            last_observed_at_ms,
                            ?1 as updated_at_ms
                        FROM agg_usage
                        ON CONFLICT(model_id, provider) DO UPDATE SET
                            observed_blended_per_1k = excluded.observed_blended_per_1k,
                            n_provider_reported = excluded.n_provider_reported,
                            n_estimated = excluded.n_estimated,
                            n_free = excluded.n_free,
                            confidence = excluded.confidence,
                            last_observed_at_ms = excluded.last_observed_at_ms,
                            updated_at_ms = excluded.updated_at_ms
                    "#;
                    
                    let changes = conn.execute(sql, turso::params![now_ms]).await?;
                    Ok::<_, StoreError>(changes as usize)
                }
            })
            .await
    }
}
