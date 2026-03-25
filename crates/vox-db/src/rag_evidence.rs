//! Trusted evidence bundle façade over `trusted_evidence_bundles` (Arca V20).
//!
//! Only RAG results that pass the Socrates contradiction gate should be persisted here.
//! Callers must verify `contradiction_hints.is_empty()` for every [`RetrievalResult`] before
//! calling [`VoxDb::store_trusted_evidence`].
//!
//! Bundle keys should be stable: `format!("{session_key}:{query_hash}")` where `query_hash` is
//! a short hex digest of the normalised query string (whitespace-collapsed, lowercased).

use serde::{Deserialize, Serialize};

use crate::RetrievalResult;
use crate::VoxDb;
use crate::store::StoreError;

// ── Domain type ───────────────────────────────────────────────────────────────

/// Minimal serialisable copy of a vetted [`RetrievalResult`] stored inside a trusted bundle.
///
/// Fields are kept to a compact subset to avoid bloating the JSON payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedHit {
    /// Stable chunk ID from the source retrieval system.
    pub chunk_id: String,
    /// Document URI or file path attribution.
    pub source: String,
    /// Fused relevance score from [`crate::retrieval::fuse_hybrid_results`].
    pub score: f32,
    /// Short text excerpt for LLM context injection (≤ 512 chars, best-effort).
    pub snippet: String,
}

impl From<&RetrievalResult> for TrustedHit {
    fn from(r: &RetrievalResult) -> Self {
        let snippet = if r.snippet.len() > 512 {
            r.snippet[..512].to_string()
        } else {
            r.snippet.clone()
        };
        Self {
            chunk_id: r.chunk_id.clone(),
            source: r.source.clone(),
            score: r.score,
            snippet,
        }
    }
}

// ── VoxDb methods ─────────────────────────────────────────────────────────────

impl VoxDb {
    /// Persist a `Vec<RetrievalResult>` as a trusted bundle.
    ///
    /// **Callers must only call this when the evidence is contradiction-free** (i.e. all items
    /// passed the Socrates gate). The function counts `contradiction_hints` across all hits and
    /// records the count in the row for auditing — it does *not* block the write even when
    /// `contradiction_count > 0`, but callers should treat a non-zero count as a logic bug.
    ///
    /// # Returns
    /// `Ok(Some(row_id))` on a successful upsert, or `Ok(None)` if `hits` is empty.
    pub async fn store_trusted_evidence(
        &self,
        bundle_key: &str,
        repository_id: &str,
        session_key: &str,
        hits: &[RetrievalResult],
        ttl_minutes: Option<u32>,
    ) -> Result<Option<i64>, StoreError> {
        if hits.is_empty() {
            return Ok(None);
        }

        let contradiction_count: i64 = hits
            .iter()
            .map(|h| h.contradiction_hints.len() as i64)
            .sum();

        let trusted: Vec<TrustedHit> = hits.iter().map(TrustedHit::from).collect();
        let json = serde_json::to_string(&trusted)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;

        // Compute optional SQLite datetime expression for expiry.
        let expires_at_expr: Option<String> = ttl_minutes.map(|m| {
            format!("datetime('now', '+{} minutes')", m)
        });

        let id = self
            .store()
            .upsert_trusted_evidence_bundle(
                bundle_key,
                repository_id,
                session_key,
                &json,
                contradiction_count,
                expires_at_expr.as_deref(),
            )
            .await?;
        Ok(Some(id))
    }

    /// Fetch and deserialise a trusted bundle by key.
    ///
    /// Returns `None` when the key is absent or the row's `expires_at` has passed.
    pub async fn get_trusted_evidence(
        &self,
        bundle_key: &str,
    ) -> Result<Option<Vec<TrustedHit>>, StoreError> {
        match self.get_trusted_evidence_bundle(bundle_key).await? {
            Some(json) => {
                let hits: Vec<TrustedHit> = serde_json::from_str(&json)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                Ok(Some(hits))
            }
            None => Ok(None),
        }
    }

    /// List up to `limit` bundles for a `(repository_id, session_key)` pair, newest first.
    ///
    /// Returns `(bundle_key, Vec<TrustedHit>)` tuples. Rows with unparseable `evidence_json`
    /// are silently skipped.
    pub async fn list_trusted_evidence(
        &self,
        repository_id: &str,
        session_key: &str,
        limit: i64,
    ) -> Result<Vec<(String, Vec<TrustedHit>)>, StoreError> {
        let rows = self
            .store()
            .list_trusted_evidence_bundles(repository_id, session_key, limit)
            .await?;
        let mut out = Vec::new();
        for (key, json) in rows {
            if let Ok(hits) = serde_json::from_str::<Vec<TrustedHit>>(&json) {
                out.push((key, hits));
            }
        }
        Ok(out)
    }

    /// Update `endpoint_reliability` with an infrastructure-level failure (rate-limit or timeout).
    ///
    /// This is the **only** call path for network-level (infra) failures; Socrates-derived
    /// hallucination scores flow through [`VoxDb::record_socrates_surface_event`] which also
    /// calls [`crate::store::VoxDb::record_endpoint_observation`].
    pub async fn record_endpoint_infra_failure(
        &self,
        endpoint_url: &str,
        model_id: &str,
        is_rate_limit: bool,
        is_timeout: bool,
    ) -> Result<(), StoreError> {
        let infra = if is_rate_limit || is_timeout { 1.0_f64 } else { 0.0_f64 };
        self
            .record_endpoint_observation(
                endpoint_url,
                model_id,
                0.0,           // no hallucination signal — infra path only
                0.0,           // no contradiction signal
                infra,
                is_rate_limit,
                is_timeout,
            )
            .await
    }

    /// Return all `endpoint_reliability` rows sorted by composite degradation score (worst first).
    ///
    /// Composite = `0.5 * hallucination_proxy_ewma + 0.3 * contradiction_ratio_ewma + 0.2 * infra_failure_ewma`.
    pub async fn aggregate_endpoint_reliability(
        &self,
        limit: i64,
    ) -> Result<Vec<crate::store::EndpointReliabilityEntry>, StoreError> {
        self.list_endpoint_reliability(limit).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "local"))]
mod tests {
    use super::*;
    use crate::{DbConfig, RetrievalEvidenceSource};

    fn make_hit(chunk_id: &str, contradictions: usize) -> RetrievalResult {
        RetrievalResult {
            chunk_id: chunk_id.to_string(),
            source: "test.rs".to_string(),
            score: 0.9,
            snippet: "fn foo() {}".to_string(),
            evidence_source: RetrievalEvidenceSource::Hybrid,
            contradiction_hints: vec!["bogus".to_string(); contradictions],
            retrieved_at_ms: None,
            query_id: None,
            supporting_claim_ids: vec![],
        }
    }

    #[tokio::test]
    async fn trusted_evidence_round_trip() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        let hits = vec![make_hit("c1", 0), make_hit("c2", 0)];
        let id = db
            .store_trusted_evidence("key1", "repo-x", "sess-a", &hits, None)
            .await
            .expect("store");
        assert!(id.is_some());

        let cached = db
            .get_trusted_evidence("key1")
            .await
            .expect("get")
            .expect("present");
        assert_eq!(cached.len(), 2);
        assert_eq!(cached[0].chunk_id, "c1");
    }

    #[tokio::test]
    async fn trusted_evidence_empty_returns_none() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        let id = db
            .store_trusted_evidence("key2", "repo-x", "sess-b", &[], None)
            .await
            .expect("store");
        assert!(id.is_none());
    }

    #[tokio::test]
    async fn endpoint_infra_failure_recorded() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        db.record_endpoint_infra_failure(
            "https://openrouter.ai/api/v1",
            "test/model",
            true,
            false,
        )
        .await
        .expect("record");

        let rows = db.aggregate_endpoint_reliability(10).await.expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].rate_limit_hits, 1);
        // infra_failure_ewma = 0.0 * 0.95 + 1.0 * 0.05 = 0.05
        assert!((rows[0].infra_failure_ewma - 0.05).abs() < 1e-6);
    }
}
