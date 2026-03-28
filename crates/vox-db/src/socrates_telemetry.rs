//! Persist **Socrates** calibration signals into `research_metrics` / `eval_runs` for drift monitoring
//! and proxy “hallucination risk” tracking when gold labels are absent.

use crate::research_metrics_contract::{
    METRIC_TYPE_MEMORY_HYBRID_FUSION, METRIC_TYPE_SOCRATES_SURFACE, SESSION_ID_MEMORY_HYBRID_FUSION,
    TelemetryWriteOptions,
};
use crate::store::StoreError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vox_socrates_policy::RiskDecision;

use crate::VoxDb;

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
    /// Optional retrieval evidence envelope (tier, contradictions, modality flags).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval: Option<Value>,
}

/// Rollup over recent `socrates_surface` rows (parsed from `metadata_json`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocratesSurfaceAggregate {
    /// Rows returned from the store for this query (includes rows with missing/bad metadata).
    pub sample_size: usize,
    /// Subset of [`Self::sample_size`] where `metric_value` is SQL non-NULL (proxy present).
    #[serde(default)]
    pub rows_with_metric_value: usize,
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
    /// Record one Socrates tool turn under session `mcp:<repository_id>`, metric type `socrates_surface`.
    ///
    /// Low-level append to `research_metrics`.
    pub async fn record_socrates_surface_event(
        &self,
        repository_id: &str,
        surface: &str,
        decision: RiskDecision,
        confidence_estimate: f64,
        contradiction_ratio: f64,
        model_used: Option<&str>,
        retrieval: Option<Value>,
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
            retrieval,
        };
        let json =
            serde_json::to_string(&meta).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let tw = TelemetryWriteOptions::new(repository_id);
        self.append_research_metric(
            &tw.session_mcp(),
            METRIC_TYPE_SOCRATES_SURFACE,
            Some(proxy),
            Some(&json),
        )
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
            SESSION_ID_MEMORY_HYBRID_FUSION,
            METRIC_TYPE_MEMORY_HYBRID_FUSION,
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
    ) -> Result<Vec<(String, Option<f64>, Option<String>)>, StoreError> {
        let prefix = repository_id
            .map(TelemetryWriteOptions::new)
            .map(|tw| tw.session_mcp())
            .unwrap_or_default();
        self.list_research_metrics_by_type(METRIC_TYPE_SOCRATES_SURFACE, &prefix, limit)
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
        let mut n_proxy = 0_usize;
        let mut sum_conf = 0.0_f64;
        let mut sum_cr = 0.0_f64;
        let mut parsed_n = 0_usize;
        for (_session, metric_value, meta) in rows {
            agg.sample_size += 1;
            if let Some(v) = metric_value {
                sum_proxy += v;
                n_proxy += 1;
                agg.rows_with_metric_value += 1;
            }
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
        agg.mean_hallucination_risk_proxy = if n_proxy > 0 {
            sum_proxy / n_proxy as f64
        } else {
            0.0
        };
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
        let mut agg_for_meta = serde_json::to_value(&agg)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
        if let serde_json::Value::Object(ref mut m) = agg_for_meta {
            m.insert(
                "rate_denominator".into(),
                serde_json::json!("parsed_metadata_rows"),
            );
            m.insert(
                "abstain_rate_denominator_n".into(),
                serde_json::json!(agg.parsed_metadata_rows),
            );
            m.insert(
                "answer_rate_denominator_n".into(),
                serde_json::json!(agg.parsed_metadata_rows),
            );
            m.insert(
                "mean_proxy_denominator_n".into(),
                serde_json::json!(agg.rows_with_metric_value),
            );
            m.insert(
                "rows_total_n".into(),
                serde_json::json!(agg.sample_size),
            );
        }
        let meta = serde_json::to_string(&agg_for_meta)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;
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
        let quality = if agg.rows_with_metric_value > 0 {
            (1.0 - agg.mean_hallucination_risk_proxy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.record_eval_run(
            eval_id,
            repository_id,
            Some(answer_rate),
            Some(abstain_rate),
            Some(quality),
            None,
            None,
            Some(&meta),
        )
        .await
    }

    /// Inject a [`SocratesSurfaceAggregate`]-compatible JSON object into `metadata_json.scientia_evidence.socrates_aggregate`
    /// when missing or `sample_size == 0`, using the latest `socrates_surface` rows for `repository_id`.
    pub async fn merge_scientia_live_socrates_into_metadata_json(
        &self,
        metadata_json: Option<&str>,
        repository_id: &str,
    ) -> Result<String, StoreError> {
        const KEY: &str = "scientia_evidence";
        let mut root: Value = match metadata_json {
            Some(s) if !s.trim().is_empty() => {
                serde_json::from_str(s).map_err(|e| StoreError::Serialization(e.to_string()))?
            }
            _ => serde_json::json!({}),
        };
        let skip = root
            .get(KEY)
            .and_then(|ev| ev.get("socrates_aggregate"))
            .and_then(|a| a.get("sample_size"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            > 0;
        if skip {
            return serde_json::to_string(&root)
                .map_err(|e| StoreError::Serialization(e.to_string()));
        }

        let agg = self
            .aggregate_socrates_surface_metrics(Some(repository_id), 200)
            .await?;
        if agg.sample_size == 0 {
            return serde_json::to_string(&root)
                .map_err(|e| StoreError::Serialization(e.to_string()));
        }

        let snap = serde_json::json!({
            "sample_size": agg.sample_size,
            "parsed_metadata_rows": agg.parsed_metadata_rows,
            "mean_hallucination_risk_proxy": agg.mean_hallucination_risk_proxy,
            "mean_confidence_estimate": agg.mean_confidence_estimate,
            "mean_contradiction_ratio": agg.mean_contradiction_ratio,
            "answer_count": agg.answer_count,
            "ask_count": agg.ask_count,
            "abstain_count": agg.abstain_count,
        });

        let mut ev = root
            .get(KEY)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        match ev {
            Value::Object(ref mut m) => {
                m.insert("socrates_aggregate".to_string(), snap);
            }
            _ => {
                ev = serde_json::json!({
                    "socrates_aggregate": snap,
                });
            }
        }
        root[KEY] = ev;
        serde_json::to_string(&root).map_err(|e| StoreError::Serialization(e.to_string()))
    }
}

#[cfg(all(test, feature = "local"))]
mod db_tests {
    use crate::store::StoreError;
    use crate::{DbConfig, VoxDb};
    use serde_json::json;
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
            None,
        )
        .await
        .expect("record");
        db.record_socrates_surface_event(
            rid,
            "vox_plan",
            RiskDecision::Abstain,
            0.2,
            0.5,
            None,
            None,
        )
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
        match err {
            StoreError::Db(msg) => {
                assert!(
                    msg.contains("no socrates_surface"),
                    "unexpected message: {msg}"
                );
            }
            _ => panic!("expected StoreError::Db, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn socrates_surface_persists_optional_retrieval_metadata() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("memory vox-db");
        let rid = "retrieval-meta-repo";
        db.record_socrates_surface_event(
            rid,
            "vox_chat_message",
            RiskDecision::Ask,
            0.6,
            0.2,
            Some("test-model"),
            Some(json!({
                "retrieval_tier": "hybrid",
                "used_vector": true,
                "contradiction_count": 1
            })),
        )
        .await
        .expect("record retrieval metadata");

        let rows = db
            .list_socrates_surface_events(Some(rid), 5)
            .await
            .expect("list rows");
        assert_eq!(rows.len(), 1);
        let meta = rows[0].2.clone().expect("metadata json");
        assert!(meta.contains("\"retrieval_tier\":\"hybrid\""));
        assert!(meta.contains("\"used_vector\":true"));
    }

    #[tokio::test]
    async fn merge_scientia_injects_aggregate_into_metadata() {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .expect("memory vox-db");
        let rid = "merge-scientia-repo";
        db.record_socrates_surface_event(
            rid,
            "vox_chat_message",
            RiskDecision::Answer,
            0.88,
            0.04,
            Some("m"),
            None,
        )
        .await
        .expect("record");
        let base = serde_json::json!({ "repository_id": rid, "prepared_by": "t" });
        let base_str = base.to_string();
        let out = db
            .merge_scientia_live_socrates_into_metadata_json(Some(&base_str), rid)
            .await
            .expect("merge");
        let v: serde_json::Value = serde_json::from_str(&out).expect("parse out");
        assert!(
            v["scientia_evidence"]["socrates_aggregate"]["sample_size"]
                .as_u64()
                .unwrap_or(0)
                > 0
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
