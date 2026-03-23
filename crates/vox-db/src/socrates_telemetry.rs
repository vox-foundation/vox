//! Persist **Socrates** calibration signals into `research_metrics` / `eval_runs` for drift monitoring
//! and proxy “hallucination risk” tracking when gold labels are absent.

use serde::{Deserialize, Serialize};
use vox_pm::store::StoreError;
use vox_socrates_policy::RiskDecision;

use crate::{EvalRunParams, VoxDb};

/// Higher values ⇒ more conservative outputs (abstain / contradiction) — useful as a **proxy** when
/// ground-truth hallucination labels are not available.
#[must_use]
pub fn hallucination_risk_proxy(decision: RiskDecision, contradiction_ratio: f64) -> f64 {
    let base = match decision {
        RiskDecision::Answer => 0.0_f64,
        RiskDecision::Ask => 0.45_f64,
        RiskDecision::Abstain => 1.0_f64,
    };
    let cr = contradiction_ratio.clamp(0.0, 1.0);
    (base + 0.35 * cr).min(1.0)
}

/// One MCP / tool surface emission (mirrors the JSON `socrates` object plus routing context).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocratesSurfaceTelemetry {
    /// Logical tool id (e.g. `vox_chat_message`).
    pub surface: String,
    /// Repository id from `vox-repository` (stable hash).
    pub repository_id: String,
    /// Socrates routing outcome for this turn.
    pub risk_decision: RiskDecision,
    /// Model-reported or heuristic confidence in [0, 1].
    pub confidence_estimate: f64,
    /// Fraction of retrieved evidence that contradicted the draft answer [0, 1].
    pub contradiction_ratio: f64,
    /// Combined safety proxy stored alongside the metric row.
    pub hallucination_risk_proxy: f64,
    /// LLM id / label when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_used: Option<String>,
}

/// Rollup over recent `socrates_surface` rows (parsed from `metadata_json`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocratesSurfaceAggregate {
    /// Rows returned from the store for this query (includes rows with missing/bad metadata).
    pub sample_size: usize,
    /// Subset of [`Self::sample_size`] where `metadata_json` deserialized as [`SocratesSurfaceTelemetry`].
    #[serde(default)]
    pub parsed_metadata_rows: usize,
    /// Mean of the `metric_value` column (proxy score) over all sampled rows.
    pub mean_hallucination_risk_proxy: f64,
    /// Counts below apply only to **parsed** metadata rows (see [`Self::parsed_metadata_rows`]).
    pub answer_count: usize,
    /// Parsed rows with [`RiskDecision::Ask`].
    pub ask_count: usize,
    /// Parsed rows with [`RiskDecision::Abstain`].
    pub abstain_count: usize,
    /// Mean `confidence_estimate` over parsed metadata rows only.
    pub mean_confidence_estimate: f64,
    /// Mean `contradiction_ratio` over parsed metadata rows only.
    pub mean_contradiction_ratio: f64,
}

impl VoxDb {
    /// Low-level append to `research_metrics`.
    pub async fn append_research_metric(
        &self,
        session_id: &str,
        metric_type: &str,
        metric_value: Option<f64>,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.store
            .append_research_metric(session_id, metric_type, metric_value, metadata_json)
            .await
    }

    /// Record one Socrates tool turn under session `mcp:<repository_id>`, metric type `socrates_surface`.
    pub async fn record_socrates_surface_event(
        &self,
        repository_id: &str,
        surface: &str,
        decision: RiskDecision,
        confidence_estimate: f64,
        contradiction_ratio: f64,
        model_used: Option<&str>,
    ) -> Result<i64, StoreError> {
        let proxy = hallucination_risk_proxy(decision, contradiction_ratio);
        let meta = SocratesSurfaceTelemetry {
            surface: surface.to_string(),
            repository_id: repository_id.to_string(),
            risk_decision: decision,
            confidence_estimate,
            contradiction_ratio,
            hallucination_risk_proxy: proxy,
            model_used: model_used.map(std::string::ToString::to_string),
        };
        let json =
            serde_json::to_string(&meta).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let session = format!("mcp:{repository_id}");
        self.append_research_metric(&session, "socrates_surface", Some(proxy), Some(&json))
            .await
    }

    /// Best-effort telemetry for hybrid memory retrieval (BM25 + vector fusion via `fuse_hybrid_results`).
    pub async fn record_memory_hybrid_retrieval(
        &self,
        query: &str,
        bm25_candidates: usize,
        vector_hits: usize,
        fused_returned: usize,
        contradictions: usize,
        top_score: Option<f64>,
    ) -> Result<i64, StoreError> {
        let contradiction_rate = if fused_returned > 0 {
            contradictions as f64 / fused_returned as f64
        } else {
            0.0
        };
        let meta = serde_json::json!({
            "query_len": query.chars().count(),
            "bm25_candidates": bm25_candidates,
            "vector_hits": vector_hits,
            "fused_returned": fused_returned,
            "contradictions": contradictions,
            "top_score": top_score,
            "fusion_impl": "vox_db::fuse_hybrid_results",
        });
        let s = meta.to_string();
        self.append_research_metric(
            "socrates:retrieval",
            "memory_hybrid_fusion",
            Some(contradiction_rate),
            Some(&s),
        )
        .await
    }

    /// Newest `socrates_surface` rows. Pass `repository_id` to filter `mcp:<id>` sessions only.
    pub async fn list_socrates_surface_events(
        &self,
        repository_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(String, f64, Option<String>)>, StoreError> {
        let prefix = repository_id
            .map(|r| format!("mcp:{r}"))
            .unwrap_or_default();
        self.store
            .list_research_metrics_by_type("socrates_surface", &prefix, limit)
            .await
    }

    /// Aggregate recent surface events for dashboards / batch eval.
    pub async fn aggregate_socrates_surface_metrics(
        &self,
        repository_id: Option<&str>,
        limit: i64,
    ) -> Result<SocratesSurfaceAggregate, StoreError> {
        let rows = self
            .list_socrates_surface_events(repository_id, limit)
            .await?;
        let mut agg = SocratesSurfaceAggregate::default();
        if rows.is_empty() {
            return Ok(agg);
        }
        let mut sum_proxy = 0.0_f64;
        let mut sum_conf = 0.0_f64;
        let mut sum_cr = 0.0_f64;
        let mut parsed_n = 0_usize;
        for (_session, metric_value, meta) in rows {
            agg.sample_size += 1;
            sum_proxy += metric_value;
            if let Some(ref m) = meta {
                if let Ok(t) = serde_json::from_str::<SocratesSurfaceTelemetry>(m) {
                    parsed_n += 1;
                    sum_conf += t.confidence_estimate;
                    sum_cr += t.contradiction_ratio;
                    match t.risk_decision {
                        RiskDecision::Answer => agg.answer_count += 1,
                        RiskDecision::Ask => agg.ask_count += 1,
                        RiskDecision::Abstain => agg.abstain_count += 1,
                    }
                }
            }
        }
        let n = agg.sample_size as f64;
        agg.mean_hallucination_risk_proxy = if n > 0.0 { sum_proxy / n } else { 0.0 };
        agg.mean_confidence_estimate = if parsed_n > 0 {
            sum_conf / parsed_n as f64
        } else {
            0.0
        };
        agg.mean_contradiction_ratio = if parsed_n > 0 {
            sum_cr / parsed_n as f64
        } else {
            0.0
        };
        agg.parsed_metadata_rows = parsed_n;
        Ok(agg)
    }

    /// Write one `eval_runs` row summarizing recent Socrates surface traffic (proxy “quality” / safety).
    ///
    /// Returns [`StoreError::Db`] when there are no `socrates_surface` rows in the scanned window
    /// (avoids writing a misleading `eval_runs` row with artificial “perfect” quality).
    pub async fn record_socrates_eval_summary(
        &self,
        eval_id: &str,
        repository_id: Option<&str>,
        sample_limit: i64,
    ) -> Result<i64, StoreError> {
        let agg = self
            .aggregate_socrates_surface_metrics(repository_id, sample_limit)
            .await?;
        if agg.sample_size == 0 {
            return Err(StoreError::Db(
                "record_socrates_eval_summary: no socrates_surface rows in range — \
                 run MCP tools with Codex attached or widen --limit"
                    .into(),
            ));
        }
        let meta =
            serde_json::to_string(&agg).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let p = agg.parsed_metadata_rows as f64;
        let abstain_rate = if p > 0.0 {
            agg.abstain_count as f64 / p
        } else {
            0.0
        };
        let answer_rate = if p > 0.0 {
            agg.answer_count as f64 / p
        } else {
            0.0
        };
        let quality = (1.0 - agg.mean_hallucination_risk_proxy).clamp(0.0, 1.0);
        self.record_eval_run(EvalRunParams {
            eval_id,
            model_path: repository_id,
            format_validity: Some(answer_rate),
            safety_rejection_rate: Some(abstain_rate),
            quality_proxy: Some(quality),
            skills_discovered: None,
            workflows_discovered: None,
            metadata_json: Some(&meta),
        })
        .await
    }
}

#[cfg(all(test, feature = "local"))]
mod db_tests {
    use crate::{DbConfig, VoxDb};
    use vox_pm::store::StoreError;
    use vox_socrates_policy::RiskDecision;

    #[tokio::test]
    async fn socrates_surface_round_trip_and_aggregate() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("memory vox-db");
        let rid = "telemetry-test-repo";
        db.record_socrates_surface_event(
            rid,
            "vox_chat_message",
            RiskDecision::Answer,
            0.9,
            0.0,
            Some("test-model"),
        )
        .await
        .expect("record");
        db.record_socrates_surface_event(rid, "vox_plan", RiskDecision::Abstain, 0.2, 0.5, None)
            .await
            .expect("record2");
        let agg = db
            .aggregate_socrates_surface_metrics(Some(rid), 10)
            .await
            .expect("agg");
        assert_eq!(agg.sample_size, 2);
        assert_eq!(agg.parsed_metadata_rows, 2);
        assert_eq!(agg.answer_count, 1);
        assert_eq!(agg.abstain_count, 1);
        assert!(agg.mean_hallucination_risk_proxy > 0.0);
    }

    #[tokio::test]
    async fn eval_summary_errors_when_no_surface_rows() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("memory vox-db");
        let err = db
            .record_socrates_eval_summary("empty-snapshot", None, 50)
            .await
            .expect_err("expected empty window");
        let StoreError::Db(msg) = err else {
            panic!("expected Db error, got {err:?}");
        };
        assert!(
            msg.contains("no socrates_surface"),
            "unexpected message: {msg}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_orders_decisions() {
        let p_answer = hallucination_risk_proxy(RiskDecision::Answer, 0.0);
        let p_ask = hallucination_risk_proxy(RiskDecision::Ask, 0.0);
        let p_abs = hallucination_risk_proxy(RiskDecision::Abstain, 0.0);
        assert!(p_answer < p_ask && p_ask < p_abs);
    }

    #[test]
    fn contradiction_increases_proxy() {
        let a = hallucination_risk_proxy(RiskDecision::Answer, 0.0);
        let b = hallucination_risk_proxy(RiskDecision::Answer, 0.5);
        assert!(b > a);
    }
}
