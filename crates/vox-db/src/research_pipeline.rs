//! Codex methods for the SCIENTIA research pipeline (Phase 0d).
//!
//! These implement the DB half of the stubs in `vox-research-shim/src/research/orchestrator/pipeline.rs`.

use crate::VoxDb;
use crate::store::StoreError;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::OnceLock;
use turso::params;
use vox_db_types::{
    CachedClaimVerdict, ClaimsPendingCounts, MisguidanceReporter, RecordMisguidanceParams,
    ResearchArtifactRecord, ResearchDefectClass, ResearchMisguidanceRecord, ResearchSearchResult,
    ResearchSessionRecord, ResearchSessionSummary, ScientiaClaimWithVerdict,
};

impl VoxDb {
    /// Create a new research session and return its row id.
    pub async fn create_research_session(
        &self,
        session_key: &str,
        query_text: &str,
    ) -> Result<i64, StoreError> {
        let now = now_ms();
        let key = session_key.to_string();
        let q = query_text.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR IGNORE INTO scientia_research_sessions \
                     (session_key, status, started_at_ms, query_text) \
                     VALUES (?1, 'queued', ?2, ?3)",
                    params![key.as_str(), now, q.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Update the status of a research session.
    pub async fn update_research_session_status(
        &self,
        session_id: i64,
        status: &str,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let s = status.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE scientia_research_sessions \
                     SET status = ?1, finished_at_ms = ?2 \
                     WHERE id = ?3",
                    params![s.as_str(), now, session_id],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Fetch one SCIENTIA research session by row id.
    pub async fn get_research_session(
        &self,
        session_id: i64,
    ) -> Result<Option<ResearchSessionRecord>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT id, session_key, status, started_at_ms, finished_at_ms, query_text \
                         FROM scientia_research_sessions WHERE id = ?1",
                        params![session_id],
                    )
                    .await?;
                let Some(row) = rows.next().await? else {
                    return Ok::<Option<ResearchSessionRecord>, StoreError>(None);
                };
                Ok(Some(ResearchSessionRecord {
                    id: row.get::<i64>(0)?,
                    session_key: row.get::<String>(1)?,
                    status: row.get::<String>(2)?,
                    started_at_ms: row.get::<i64>(3)?,
                    finished_at_ms: row.get::<Option<i64>>(4)?,
                    query_text: row.get::<String>(5)?,
                }))
            })
            .await
    }

    /// List recent SCIENTIA research sessions newest-first.
    pub async fn list_recent_research_sessions(
        &self,
        limit: u32,
    ) -> Result<Vec<ResearchSessionSummary>, StoreError> {
        let lim = limit.clamp(1, 200) as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT id, session_key, status, started_at_ms, finished_at_ms, query_text \
                         FROM scientia_research_sessions ORDER BY started_at_ms DESC, id DESC LIMIT ?1",
                        params![lim],
                    )
                    .await?;
                let mut out = Vec::new();
                while let Some(row) = rows.next().await? {
                    out.push(ResearchSessionSummary {
                        id: row.get::<i64>(0)?,
                        session_key: row.get::<String>(1)?,
                        status: row.get::<String>(2)?,
                        started_at_ms: row.get::<i64>(3)?,
                        finished_at_ms: row.get::<Option<i64>>(4)?,
                        query_text: row.get::<String>(5)?,
                    });
                }
                Ok::<Vec<ResearchSessionSummary>, StoreError>(out)
            })
            .await
    }

    /// Record a single research pipeline metric.
    ///
    /// Maps onto the existing `research_metrics` table (column `metric_value`, `created_at`).
    pub async fn record_research_metric(
        &self,
        session_id: i64,
        metric_type: &str,
        value: f64,
        metadata_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let sid = session_id.to_string();
        let mt = metric_type.to_string();
        let meta = metadata_json.map(|s| s.to_string());
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO research_metrics \
                     (session_id, metric_type, metric_value, metadata_json) \
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        sid.as_str(),
                        mt.as_str(),
                        value,
                        meta.as_deref().unwrap_or("{}")
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Store an extracted atomic claim.
    pub async fn store_claim(
        &self,
        session_id: i64,
        claim_id: u64,
        text: &str,
        is_numeric: bool,
        is_recent: bool,
        is_named_event: bool,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let t = text.to_string();
        let cid = claim_id as i64;
        let num = is_numeric as i64;
        let rec = is_recent as i64;
        let named = is_named_event as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR IGNORE INTO scientia_claims \
                     (claim_id, session_id, text, is_numeric, is_recent, is_named_event, created_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![cid, session_id, t.as_str(), num, rec, named, now],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Set the verifiability score for a stored claim (best-effort; a no-op if
    /// the claim row does not exist). Kept separate from `store_claim` so its
    /// signature stays stable for existing callers.
    pub async fn update_claim_verifiability_score(
        &self,
        claim_id: u64,
        score: f64,
    ) -> Result<(), StoreError> {
        let cid = claim_id as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE scientia_claims SET verifiability_score = ?2 WHERE claim_id = ?1",
                    params![cid, score],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Store a claim verification verdict.
    pub async fn store_claim_verdict(
        &self,
        claim_id: u64,
        verdict: &str,
        confidence: f64,
        verifier_model: &str,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let cid = claim_id as i64;
        let v = verdict.to_string();
        let vm = verifier_model.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scientia_claim_verdicts \
                     (claim_id, verdict, confidence, verifier_model, created_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![cid, v.as_str(), confidence, vm.as_str(), now],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Store an evidence span for a claim verdict.
    pub async fn store_evidence_span(
        &self,
        claim_id: u64,
        span_start: usize,
        span_end: usize,
        span_text: &str,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let cid = claim_id as i64;
        let st = span_text.to_string();
        let ss = span_start as i64;
        let se = span_end as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scientia_claim_verdicts \
                     (claim_id, verdict, confidence, span_start, span_end, span_text, created_at_ms) \
                     VALUES (?1, 'Unverified', 0.0, ?2, ?3, ?4, ?5)",
                    params![cid, ss, se, st.as_str(), now],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List a publication's extracted claims, each joined to its latest
    /// non-span verdict. `session_id` is derived from the publication id by the
    /// caller (vox-cli). Newest claims first.
    pub async fn list_publication_claims(
        &self,
        session_id: i64,
    ) -> Result<Vec<ScientiaClaimWithVerdict>, StoreError> {
        let rows = self
            .query_all(
                "SELECT c.claim_id, c.text, c.is_numeric, c.verifiability_score, \
                   (SELECT v.verdict FROM scientia_claim_verdicts v \
                    WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                    ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS verdict, \
                   (SELECT v.confidence FROM scientia_claim_verdicts v \
                    WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                    ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS confidence, \
                   (SELECT v.verifier_model FROM scientia_claim_verdicts v \
                    WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                    ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS verifier_model, \
                   c.created_at_ms \
                 FROM scientia_claims c \
                 WHERE c.session_id = ?1 \
                 ORDER BY c.created_at_ms DESC, c.claim_id DESC",
                (session_id,),
            )
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let is_numeric: i64 = r.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push(ScientiaClaimWithVerdict {
                claim_id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                text: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                is_numeric: is_numeric != 0,
                verifiability_score: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                verdict: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                confidence: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                verifier_model: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at_ms: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// List claims that are **awaiting human review** for a publication session.
    ///
    /// A claim is awaiting review when BOTH conditions hold:
    /// 1. It has an extracted (non-`Unverified`) verdict — extraction ran.
    /// 2. Its latest `scientia_review_decisions` row is absent, or its `decision`
    ///    is NOT a terminal value (`approved` or `rejected`). `deferred` and
    ///    `edited` are recoverable/non-terminal and leave the claim in the queue.
    ///
    /// Returns newest claims first (`created_at_ms DESC, claim_id DESC`).
    ///
    /// `publication_id` scopes the review-decision lookup: `claim_id` is an
    /// FNV-1a hash of the claim text, so the same text in another publication
    /// shares an id — without this scope, a terminal decision elsewhere would
    /// wrongly drop the claim from this publication's queue.
    pub async fn list_claims_awaiting_review(
        &self,
        session_id: i64,
        publication_id: &str,
    ) -> Result<Vec<ScientiaClaimWithVerdict>, StoreError> {
        let rows = self
            .query_all(
                "SELECT c.claim_id, c.text, c.is_numeric, c.verifiability_score, \
                   (SELECT v.verdict FROM scientia_claim_verdicts v \
                    WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                    ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS verdict, \
                   (SELECT v.confidence FROM scientia_claim_verdicts v \
                    WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                    ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS confidence, \
                   (SELECT v.verifier_model FROM scientia_claim_verdicts v \
                    WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                    ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS verifier_model, \
                   c.created_at_ms \
                 FROM scientia_claims c \
                 WHERE c.session_id = ?1 \
                   AND (SELECT v.verdict FROM scientia_claim_verdicts v \
                        WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                        ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) IS NOT NULL \
                   AND COALESCE(\
                         (SELECT d.decision FROM scientia_review_decisions d \
                          WHERE d.claim_id = c.claim_id AND d.publication_id = ?2 \
                          ORDER BY d.decided_at_ms DESC, d.id DESC LIMIT 1), \
                         '') NOT IN ('approved', 'rejected') \
                 ORDER BY c.created_at_ms DESC, c.claim_id DESC",
                (session_id, publication_id),
            )
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let is_numeric: i64 = r.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push(ScientiaClaimWithVerdict {
                claim_id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                text: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                is_numeric: is_numeric != 0,
                verifiability_score: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                verdict: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                confidence: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                verifier_model: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at_ms: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// Global claims-pending counts for the SCIENTIA dashboard: each claim
    /// bucketed by its latest non-span verdict (`Supported` → verifiable,
    /// `Abstain` → abstained, none yet → extraction_running).
    pub async fn scientia_claims_pending_summary(&self) -> Result<ClaimsPendingCounts, StoreError> {
        let rows = self
            .query_all(
                "SELECT \
                   COALESCE(SUM(CASE WHEN verdict = 'Supported' THEN 1 ELSE 0 END), 0), \
                   COALESCE(SUM(CASE WHEN verdict = 'Abstain' THEN 1 ELSE 0 END), 0), \
                   COALESCE(SUM(CASE WHEN verdict IS NULL THEN 1 ELSE 0 END), 0) \
                 FROM ( \
                   SELECT (SELECT v.verdict FROM scientia_claim_verdicts v \
                           WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' \
                           ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1) AS verdict \
                   FROM scientia_claims c \
                 )",
                (),
            )
            .await?;
        match rows.into_iter().next() {
            Some(r) => Ok(ClaimsPendingCounts {
                verifiable: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                abstained: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                extraction_running: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }),
            None => Ok(ClaimsPendingCounts {
                verifiable: 0,
                abstained: 0,
                extraction_running: 0,
            }),
        }
    }

    /// Store a training pair (query + answer + quality score).
    pub async fn store_training_pair(
        &self,
        session_id: i64,
        query: &str,
        answer: &str,
        quality_score: i32,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let q = query.to_string();
        let a = answer.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scientia_training_pairs \
                     (session_id, query_text, answer_text, quality_score, created_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![session_id, q.as_str(), a.as_str(), quality_score, now],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Upsert the durable artifact for a completed research session.
    pub async fn store_research_artifact(
        &self,
        session_id: i64,
        artifact_json: &str,
        report_markdown: &str,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let artifact = artifact_json.to_string();
        let report = report_markdown.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scientia_research_artifacts \
                     (session_id, artifact_json, report_markdown, created_at_ms, updated_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?4) \
                     ON CONFLICT(session_id) DO UPDATE SET \
                       artifact_json = excluded.artifact_json, \
                       report_markdown = excluded.report_markdown, \
                       updated_at_ms = excluded.updated_at_ms",
                    params![session_id, artifact.as_str(), report.as_str(), now],
                )
                .await?;

                // Populate / update scientia_research_fts if available
                let mut query_text = String::new();
                if let Ok(mut rows) = conn
                    .query(
                        "SELECT query_text FROM scientia_research_sessions WHERE id = ?1",
                        params![session_id],
                    )
                    .await
                {
                    if let Ok(Some(row)) = rows.next().await {
                        if let Ok(q) = row.get::<String>(0) {
                            query_text = q;
                        }
                    }
                }

                let mut claims_text = String::new();
                if let Ok(mut rows) = conn
                    .query(
                        "SELECT text FROM scientia_claims WHERE session_id = ?1",
                        params![session_id],
                    )
                    .await
                {
                    let mut claims = Vec::new();
                    while let Ok(Some(row)) = rows.next().await {
                        if let Ok(t) = row.get::<String>(0) {
                            claims.push(t);
                        }
                    }
                    claims_text = claims.join("\n");
                }

                let _ = conn
                    .execute(
                        "DELETE FROM scientia_research_fts WHERE session_id = ?1",
                        params![session_id],
                    )
                    .await;
                let _ = conn
                    .execute(
                        "INSERT INTO scientia_research_fts (session_id, query_text, report_markdown, claims_text) \
                         VALUES (?1, ?2, ?3, ?4)",
                        params![session_id, query_text.as_str(), report.as_str(), claims_text.as_str()],
                    )
                    .await;

                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Full-text search across historical research artifacts and claims.
    ///
    /// Tries BM25 ranking on `scientia_research_fts` with `snippet()`; falls back to
    /// LIKE substring matching if FTS5 is not available or query has no FTS hits.
    pub async fn search_research_artifacts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ResearchSearchResult>, StoreError> {
        let lim = limit.clamp(1, 200) as i64;
        let q_trimmed = query.trim();
        if q_trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let q_fts = sanitize_fts_query(q_trimmed);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let q_owned = q_trimmed.to_string();

        breaker
            .call(|| async move {
                if !q_fts.is_empty() {
                    let fts_res = conn
                        .query(
                            "SELECT f.session_id, f.query_text, \
                                    snippet(scientia_research_fts, -1, '', '', '...', 32) AS snip, \
                                    COALESCE(a.created_at_ms, s.started_at_ms, 0) AS created_at_ms, \
                                    COALESCE(SUBSTR(f.report_markdown, 1, 200), '') AS fallback_snip \
                             FROM scientia_research_fts f \
                             LEFT JOIN scientia_research_artifacts a ON a.session_id = f.session_id \
                             LEFT JOIN scientia_research_sessions s ON s.id = f.session_id \
                             WHERE scientia_research_fts MATCH ?1 \
                             ORDER BY bm25(scientia_research_fts) ASC \
                             LIMIT ?2",
                            params![q_fts.as_str(), lim],
                        )
                        .await;

                    if let Ok(mut rows) = fts_res {
                        let mut results = Vec::new();
                        while let Ok(Some(row)) = rows.next().await {
                            let sid: i64 = row.get(0)?;
                            let qtext: String = row.get(1)?;
                            let mut snip: String = row.get(2)?;
                            let cat: i64 = row.get(3)?;
                            if snip.trim().is_empty() {
                                let fallback: String = row.get(4)?;
                                snip = fallback;
                            }
                            results.push(ResearchSearchResult {
                                session_id: sid,
                                query_text: qtext,
                                snippet: snip,
                                created_at_ms: cat,
                            });
                        }
                        if !results.is_empty() {
                            return Ok::<Vec<ResearchSearchResult>, StoreError>(results);
                        }
                    }
                }

                // Fallback: LIKE query
                let pat = format!("%{q_owned}%");
                let mut rows = conn
                    .query(
                        "SELECT a.session_id, s.query_text, \
                                COALESCE(SUBSTR(a.report_markdown, 1, 200), '') AS snip, \
                                a.created_at_ms \
                         FROM scientia_research_artifacts a \
                         JOIN scientia_research_sessions s ON s.id = a.session_id \
                         WHERE s.query_text LIKE ?1 OR a.report_markdown LIKE ?1 \
                            OR EXISTS (SELECT 1 FROM scientia_claims c WHERE c.session_id = a.session_id AND c.text LIKE ?1) \
                         ORDER BY a.created_at_ms DESC \
                         LIMIT ?2",
                        params![pat.as_str(), lim],
                    )
                    .await?;

                let mut results = Vec::new();
                while let Some(row) = rows.next().await? {
                    let sid: i64 = row.get(0)?;
                    let qtext: String = row.get(1)?;
                    let snip: String = row.get(2)?;
                    let cat: i64 = row.get(3)?;
                    results.push(ResearchSearchResult {
                        session_id: sid,
                        query_text: qtext,
                        snippet: snip,
                        created_at_ms: cat,
                    });
                }
                Ok::<Vec<ResearchSearchResult>, StoreError>(results)
            })
            .await
    }

    /// Look up a previously verified claim verdict from historical research sessions with optional minimum confidence filtering.
    ///
    /// If `max_age_ms > 0`, only verdicts recorded within the last `max_age_ms` are returned.
    /// If `min_confidence` is `Some(min_conf)`, only verdicts with `confidence >= min_conf` are returned.
    /// Spans stored as 'Unverified' are filtered out.
    /// If `min_confidence` is provided, results are ordered by `confidence DESC, created_at_ms DESC` so the highest confidence verdict wins.
    pub async fn get_cached_claim_verdict_filtered(
        &self,
        claim_id: u64,
        max_age_ms: i64,
        min_confidence: Option<f64>,
    ) -> Result<Option<CachedClaimVerdict>, StoreError> {
        let cid = claim_id as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = match (max_age_ms > 0, min_confidence) {
                    (true, Some(min_c)) => {
                        let cutoff = now_ms().saturating_sub(max_age_ms);
                        conn.query(
                            "SELECT claim_id, verdict, confidence, verifier_model, created_at_ms \
                             FROM scientia_claim_verdicts \
                             WHERE claim_id = ?1 AND verdict <> 'Unverified' AND created_at_ms >= ?2 AND confidence >= ?3 \
                             ORDER BY confidence DESC, created_at_ms DESC, id DESC LIMIT 1",
                            params![cid, cutoff, min_c],
                        )
                        .await?
                    }
                    (true, None) => {
                        let cutoff = now_ms().saturating_sub(max_age_ms);
                        conn.query(
                            "SELECT claim_id, verdict, confidence, verifier_model, created_at_ms \
                             FROM scientia_claim_verdicts \
                             WHERE claim_id = ?1 AND verdict <> 'Unverified' AND created_at_ms >= ?2 \
                             ORDER BY created_at_ms DESC, id DESC LIMIT 1",
                            params![cid, cutoff],
                        )
                        .await?
                    }
                    (false, Some(min_c)) => {
                        conn.query(
                            "SELECT claim_id, verdict, confidence, verifier_model, created_at_ms \
                             FROM scientia_claim_verdicts \
                             WHERE claim_id = ?1 AND verdict <> 'Unverified' AND confidence >= ?2 \
                             ORDER BY confidence DESC, created_at_ms DESC, id DESC LIMIT 1",
                            params![cid, min_c],
                        )
                        .await?
                    }
                    (false, None) => {
                        conn.query(
                            "SELECT claim_id, verdict, confidence, verifier_model, created_at_ms \
                             FROM scientia_claim_verdicts \
                             WHERE claim_id = ?1 AND verdict <> 'Unverified' \
                             ORDER BY created_at_ms DESC, id DESC LIMIT 1",
                            params![cid],
                        )
                        .await?
                    }
                };
                let Some(row) = rows.next().await? else {
                    return Ok::<Option<CachedClaimVerdict>, StoreError>(None);
                };
                let cid_val: i64 = row.get(0)?;
                let verdict: String = row.get(1)?;
                let confidence: f64 = row.get(2)?;
                let verifier_model: Option<String> = row.get(3)?;
                let created_at_ms: i64 = row.get(4)?;
                Ok(Some(CachedClaimVerdict {
                    claim_id: cid_val as u64,
                    verdict,
                    confidence,
                    verifier_model,
                    created_at_ms,
                }))
            })
            .await
    }

    /// Look up a previously verified claim verdict from historical research sessions.
    ///
    /// If `max_age_ms > 0`, only verdicts recorded within the last `max_age_ms` are returned.
    /// Spans stored as 'Unverified' are filtered out. Returns the newest non-Unverified verdict.
    pub async fn get_cached_claim_verdict(
        &self,
        claim_id: u64,
        max_age_ms: i64,
    ) -> Result<Option<CachedClaimVerdict>, StoreError> {
        self.get_cached_claim_verdict_filtered(claim_id, max_age_ms, None)
            .await
    }

    /// Fetch the latest durable artifact for a research session.
    pub async fn get_research_artifact(
        &self,
        session_id: i64,
    ) -> Result<Option<ResearchArtifactRecord>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT session_id, artifact_json, report_markdown, created_at_ms, updated_at_ms \
                         FROM scientia_research_artifacts WHERE session_id = ?1",
                        params![session_id],
                    )
                    .await?;
                let Some(row) = rows.next().await? else {
                    return Ok::<Option<ResearchArtifactRecord>, StoreError>(None);
                };
                Ok(Some(ResearchArtifactRecord {
                    session_id: row.get::<i64>(0)?,
                    artifact_json: row.get::<String>(1)?,
                    report_markdown: row.get::<String>(2)?,
                    created_at_ms: row.get::<i64>(3)?,
                    updated_at_ms: row.get::<i64>(4)?,
                }))
            })
            .await
    }

    /// List memory entries by type (uses `knowledge_nodes` table). Returns content strings.
    pub async fn list_memories_by_type(
        &self,
        memory_type: &str,
        limit: u32,
    ) -> Result<Vec<String>, StoreError> {
        let mt = memory_type.to_string();
        let lim = limit as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        let rows = breaker
            .call(|| async move {
                let mut stmt = conn
                    .query(
                        "SELECT content FROM knowledge_nodes \
                         WHERE node_type = ?1 \
                         ORDER BY created_at DESC LIMIT ?2",
                        params![mt.as_str(), lim],
                    )
                    .await?;
                let mut results = Vec::new();
                while let Some(row) = stmt.next().await? {
                    if let Ok(content) = row.get::<String>(0) {
                        results.push(content);
                    }
                }
                Ok::<Vec<String>, StoreError>(results)
            })
            .await?;
        Ok(rows)
    }

    /// Get the retrieval configuration from the DB (returns defaults if not configured).
    ///
    /// Phase 1 will persist this to a config table; for now a sensible static default is returned.
    pub async fn get_retrieval_config(&self) -> Result<serde_json::Value, StoreError> {
        Ok(serde_json::json!({
            "max_sources": 10,
            "min_score": 0.3,
            "timeout_ms": 30000
        }))
    }

    /// Start a provider search run within a session. Returns the new run row id.
    pub async fn start_provider_run(
        &self,
        session_id: i64,
        provider_name: &str,
    ) -> Result<i64, StoreError> {
        let now = now_ms();
        let pn = provider_name.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scientia_provider_runs \
                     (session_id, provider_name, started_at_ms) \
                     VALUES (?1, ?2, ?3)",
                    params![session_id, pn.as_str(), now],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record a research source URL found during a provider run.
    ///
    /// Uses `knowledge_nodes` with `node_type = 'research_source'`. Returns the last insert rowid.
    pub async fn create_research_source(
        &self,
        session_id: i64,
        url: &str,
        title: Option<&str>,
    ) -> Result<i64, StoreError> {
        let u = url.to_string();
        let label = title.unwrap_or(url).to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR IGNORE INTO knowledge_nodes \
                     (id, label, content, node_type, metadata, created_at) \
                     VALUES (?1, ?2, '', 'research_source', '{}', datetime('now'))",
                    params![u.as_str(), label.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await?;
        // session_id correlation handled at the provider_runs level
        let _ = session_id;
        Ok(self.conn.last_insert_rowid())
    }

    /// Upsert a model profile metric using a running average (Mesh §5.7 / Phase 6).
    ///
    /// On first insert, `profile_value` and `sample_count = 1` are stored.
    /// On subsequent calls, the running mean is updated:
    ///   `new_mean = (old_mean * n + new_value) / (n + 1)`.
    pub async fn rollup_model_scoreboard_with_scientia(
        &self,
        provider: &str,
        model_id: &str,
        profile_key: &str,
        new_value: f64,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let p = provider.to_string();
        let m = model_id.to_string();
        let k = profile_key.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scientia_model_profile_learning \
                     (provider, model_id, profile_key, profile_value, sample_count, window_start_ms, window_end_ms, updated_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5, ?5) \
                     ON CONFLICT(provider, model_id, profile_key) DO UPDATE SET \
                       profile_value = (profile_value * sample_count + excluded.profile_value) / (sample_count + 1), \
                       sample_count = sample_count + 1, \
                       window_end_ms = excluded.window_end_ms, \
                       updated_at_ms = excluded.updated_at_ms",
                    params![p.as_str(), m.as_str(), k.as_str(), new_value, now],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Mark a provider search run as complete.
    pub async fn finish_provider_run(
        &self,
        run_id: i64,
        hit_count: u32,
        elapsed_ms: u64,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let hc = hit_count as i64;
        let em = elapsed_ms as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE scientia_provider_runs \
                     SET hit_count = ?1, elapsed_ms = ?2, finished_at_ms = ?3 \
                     WHERE id = ?4",
                    params![hc, em, now, run_id],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Record a research misguidance event and update the culprit domain's reputation with exponential decay.
    pub async fn record_research_misguidance(
        &self,
        params: &RecordMisguidanceParams,
    ) -> Result<i64, StoreError> {
        let now = now_ms();
        let params = params.clone();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let culprit_url = params
                    .culprit_url
                    .as_deref()
                    .map(|s| turso::Value::Text(s.to_string()))
                    .unwrap_or(turso::Value::Null);
                let excerpt = params
                    .misleading_excerpt
                    .as_deref()
                    .map(|s| turso::Value::Text(sanitize_telemetry_string(s)))
                    .unwrap_or(turso::Value::Null);
                let code = params
                    .generated_code_snippet
                    .as_deref()
                    .map(|s| turso::Value::Text(s.to_string()))
                    .unwrap_or(turso::Value::Null);
                let diag = params
                    .failure_diagnostic
                    .as_deref()
                    .map(|s| turso::Value::Text(sanitize_telemetry_string(s)))
                    .unwrap_or(turso::Value::Null);
                let diff = params
                    .correction_diff
                    .as_deref()
                    .map(|s| turso::Value::Text(s.to_string()))
                    .unwrap_or(turso::Value::Null);
                let session_id_val = params
                    .session_id
                    .map(turso::Value::Integer)
                    .unwrap_or(turso::Value::Null);
                let claim_id_val = params
                    .claim_id
                    .map(turso::Value::Integer)
                    .unwrap_or(turso::Value::Null);

                let insert_params = vec![
                    session_id_val,
                    turso::Value::Text(params.defect_class.as_str().to_string()),
                    culprit_url,
                    turso::Value::Text(params.culprit_domain.clone()),
                    claim_id_val,
                    turso::Value::Text(params.research_query.clone()),
                    excerpt,
                    code,
                    diag,
                    diff,
                    turso::Value::Text(params.reporter.as_str().to_string()),
                    turso::Value::Real(params.domain_penalty),
                    turso::Value::Integer(now),
                ];

                conn.execute(
                    "INSERT INTO research_misguidance_events \
                     (session_id, defect_class, culprit_url, culprit_domain, \
                      claim_id, research_query, misleading_excerpt, generated_code_snippet, \
                      failure_diagnostic, correction_diff, reporter, domain_penalty, created_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    insert_params,
                )
                .await?;

                let event_id = conn.last_insert_rowid();

                let mut rep_rows = conn
                    .query(
                        "SELECT incident_count, penalty_score, last_incident_at_ms \
                         FROM research_domain_reputation WHERE domain = ?1",
                        params![params.culprit_domain.as_str()],
                    )
                    .await?;

                let (new_count, new_penalty) = if let Some(row) = rep_rows.next().await? {
                    let current_count: i64 = row.get(0)?;
                    let current_penalty: f64 = row.get(1)?;
                    let last_incident_at_ms: i64 = row.get(2)?;

                    let elapsed_days = ((now - last_incident_at_ms) as f64 / 86_400_000.0).max(0.0);
                    let decayed = current_penalty * (-elapsed_days * std::f64::consts::LN_2 / 30.0).exp();
                    let new_pen = (decayed + params.domain_penalty).max(0.0).min(2.0);
                    (current_count + 1, new_pen)
                } else {
                    (1i64, params.domain_penalty.max(0.0).min(2.0))
                };

                let is_blacklisted: i64 = if new_count >= 5 && new_penalty >= 1.0 { 1 } else { 0 };

                conn.execute(
                    "INSERT INTO research_domain_reputation \
                     (domain, incident_count, penalty_score, last_incident_at_ms, is_blacklisted, updated_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
                     ON CONFLICT(domain) DO UPDATE SET \
                       incident_count = excluded.incident_count, \
                       penalty_score = excluded.penalty_score, \
                       last_incident_at_ms = excluded.last_incident_at_ms, \
                       is_blacklisted = excluded.is_blacklisted, \
                       updated_at_ms = excluded.updated_at_ms",
                    params![
                        params.culprit_domain.as_str(),
                        new_count,
                        new_penalty,
                        now,
                        is_blacklisted,
                        now,
                    ],
                )
                .await?;

                Ok::<i64, StoreError>(event_id)
            })
            .await
    }

    /// Mark a research misguidance event as resolved, saving the correction diff.
    pub async fn resolve_research_misguidance(
        &self,
        id: i64,
        correction_diff: &str,
    ) -> Result<(), StoreError> {
        let now = now_ms();
        let diff = correction_diff.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE research_misguidance_events \
                     SET resolved_at_ms = ?1, correction_diff = ?2, status = 'resolved' \
                     WHERE id = ?3",
                    params![now, diff.as_str(), id],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List recorded research misguidance events, optionally filtered by session id.
    pub async fn list_research_misguidance(
        &self,
        session_id: Option<i64>,
        limit: u32,
    ) -> Result<Vec<ResearchMisguidanceRecord>, StoreError> {
        let lim = limit.clamp(1, 1000) as i64;
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = match session_id {
                    Some(sid) => {
                        conn.query(
                            "SELECT id, session_id, defect_class, culprit_url, culprit_domain, \
                             claim_id, research_query, misleading_excerpt, generated_code_snippet, \
                             failure_diagnostic, correction_diff, reporter, domain_penalty, \
                             status, resolved_at_ms, created_at_ms \
                             FROM research_misguidance_events \
                             WHERE session_id = ?1 \
                             ORDER BY created_at_ms DESC, id DESC LIMIT ?2",
                            params![sid, lim],
                        )
                        .await?
                    }
                    None => {
                        conn.query(
                            "SELECT id, session_id, defect_class, culprit_url, culprit_domain, \
                             claim_id, research_query, misleading_excerpt, generated_code_snippet, \
                             failure_diagnostic, correction_diff, reporter, domain_penalty, \
                             status, resolved_at_ms, created_at_ms \
                             FROM research_misguidance_events \
                             ORDER BY created_at_ms DESC, id DESC LIMIT ?1",
                            params![lim],
                        )
                        .await?
                    }
                };

                let mut out = Vec::new();
                while let Some(row) = rows.next().await? {
                    let defect_class_str: String = row.get(2)?;
                    let reporter_str: String = row.get(11)?;
                    let defect_class =
                        ResearchDefectClass::from_str(&defect_class_str).map_err(StoreError::Db)?;
                    let reporter =
                        MisguidanceReporter::from_str(&reporter_str).map_err(StoreError::Db)?;

                    out.push(ResearchMisguidanceRecord {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        defect_class,
                        culprit_url: row.get(3)?,
                        culprit_domain: row.get(4)?,
                        claim_id: row.get(5)?,
                        research_query: row.get(6)?,
                        misleading_excerpt: row.get(7)?,
                        generated_code_snippet: row.get(8)?,
                        failure_diagnostic: row.get(9)?,
                        correction_diff: row.get(10)?,
                        reporter,
                        domain_penalty: row.get(12)?,
                        status: row.get(13)?,
                        resolved_at_ms: row.get(14)?,
                        created_at_ms: row.get(15)?,
                    });
                }
                Ok::<Vec<ResearchMisguidanceRecord>, StoreError>(out)
            })
            .await
    }

    /// Retrieve all domain penalty scores where penalty > 0.0.
    pub async fn get_domain_penalties(&self) -> Result<HashMap<String, f64>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT domain, penalty_score FROM research_domain_reputation WHERE penalty_score > 0.0",
                        params![],
                    )
                    .await?;
                let mut map = HashMap::new();
                while let Some(row) = rows.next().await? {
                    let domain: String = row.get(0)?;
                    let penalty: f64 = row.get(1)?;
                    map.insert(domain, penalty);
                }
                Ok::<HashMap<String, f64>, StoreError>(map)
            })
            .await
    }

    /// Retrieve the set of all blacklisted domains.
    pub async fn get_blacklisted_domains(&self) -> Result<HashSet<String>, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT domain FROM research_domain_reputation WHERE is_blacklisted = 1",
                        params![],
                    )
                    .await?;
                let mut set = HashSet::new();
                while let Some(row) = rows.next().await? {
                    let domain: String = row.get(0)?;
                    set.insert(domain);
                }
                Ok::<HashSet<String>, StoreError>(set)
            })
            .await
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn sanitize_fts_query(input: &str) -> String {
    let tokens: Vec<&str> = input
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return String::new();
    }
    tokens
        .into_iter()
        .map(|t| format!("\"{t}\""))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Sanitize sensitive tokens, credentials, and env variable values from telemetry and diagnostic strings.
pub fn sanitize_telemetry_string(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut result = s.to_string();

    // 1. Sensitive environment variables: KEY=val, TOKEN=val, SECRET=val, PASSWORD=val, etc.
    static ENV_VAR_RE: OnceLock<Regex> = OnceLock::new();
    let env_var_re = ENV_VAR_RE.get_or_init(|| {
        Regex::new(
            r#"(?i)\b([A-Za-z0-9_]*(?:KEY|SECRET|TOKEN|PASSWORD|PASSWD|CREDENTIAL)[A-Za-z0-9_]*\s*=\s*)(?:"[^"\r\n]*"|'[^'\r\n]*'|[^\s"'\r\n]+)"#,
        )
        .unwrap()
    });
    result = env_var_re
        .replace_all(&result, "${1}[REDACTED]")
        .to_string();

    // 2. Bearer tokens: Bearer [a-zA-Z0-9_\-\.]+
    static BEARER_RE: OnceLock<Regex> = OnceLock::new();
    let bearer_re =
        BEARER_RE.get_or_init(|| Regex::new(r"(?i)\bBearer\s+[a-zA-Z0-9_\-\.]+").unwrap());
    result = bearer_re
        .replace_all(&result, "Bearer [REDACTED]")
        .to_string();

    // 3. API key patterns: (sk|tvly|ghp)[-_][a-zA-Z0-9_\-\.]+
    static API_KEY_PAT_RE: OnceLock<Regex> = OnceLock::new();
    let api_key_pat_re = API_KEY_PAT_RE
        .get_or_init(|| Regex::new(r"(?i)\b(?:sk|tvly|ghp)[-_][a-zA-Z0-9_\-\.]+").unwrap());
    result = api_key_pat_re
        .replace_all(&result, "[REDACTED]")
        .to_string();

    // 4. Pass through standard pattern redact (covers AWS keys AKIA..., generic sk-..., ghp_..., etc.)
    let (redacted, _) = crate::redact::redact(&result);
    redacted
}

#[cfg(test)]
mod tests {
    use crate::store::ReviewDecisionRow;
    use crate::{DbConfig, VoxDb};

    /// Seed helper: store a claim row under the given session.
    async fn seed_claim(db: &VoxDb, session_id: i64, claim_id: u64) {
        db.store_claim(
            session_id,
            claim_id,
            &format!("claim text {claim_id}"),
            false,
            false,
            false,
        )
        .await
        .expect("store_claim");
    }

    /// Seed helper: store a non-Unverified verdict for a claim.
    async fn seed_verdict(db: &VoxDb, claim_id: u64, verdict: &str) {
        db.store_claim_verdict(claim_id, verdict, 0.8, "test-model")
            .await
            .expect("store_claim_verdict");
    }

    /// Seed helper: record a review decision for a claim.
    async fn seed_decision(db: &VoxDb, claim_id: i64, decision: &str, decided_at_ms: i64) {
        db.record_review_decision(&ReviewDecisionRow {
            claim_id,
            publication_id: "pub-test".into(),
            bound_digest: "digest-abc".into(),
            decision: decision.to_string(),
            actor: "tester".into(),
            reason: None,
            model_fingerprints_json: None,
            decided_at_ms,
        })
        .await
        .expect("record_review_decision");
    }

    /// claim A: verdict present, no decision → MUST appear.
    /// claim B: verdict present, latest decision `approved` → MUST NOT appear.
    /// claim C: verdict present, latest decision `rejected` → MUST NOT appear.
    /// claim D: verdict present, decisions [approved@t1, deferred@t2] → deferred is latest → MUST appear.
    /// claim E: no verdict (only stored, no verdict row) → MUST NOT appear.
    #[tokio::test]
    async fn list_claims_awaiting_review_excludes_terminal_and_unverdicted() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let session_id: i64 = 9_001;

        // Claim A: verdict only, no decision.
        let claim_a: u64 = 101;
        seed_claim(&db, session_id, claim_a).await;
        seed_verdict(&db, claim_a, "Supported").await;

        // Claim B: verdict + approved → terminal.
        let claim_b: u64 = 102;
        seed_claim(&db, session_id, claim_b).await;
        seed_verdict(&db, claim_b, "Supported").await;
        seed_decision(&db, claim_b as i64, "approved", 1_000_000).await;

        // Claim C: verdict + rejected → terminal.
        let claim_c: u64 = 103;
        seed_claim(&db, session_id, claim_c).await;
        seed_verdict(&db, claim_c, "Contested").await;
        seed_decision(&db, claim_c as i64, "rejected", 1_000_000).await;

        // Claim D: verdict + approved@t1 then deferred@t2 → deferred is latest, non-terminal.
        let claim_d: u64 = 104;
        seed_claim(&db, session_id, claim_d).await;
        seed_verdict(&db, claim_d, "Abstain").await;
        seed_decision(&db, claim_d as i64, "approved", 1_000_001).await;
        seed_decision(&db, claim_d as i64, "deferred", 1_000_002).await;

        // Claim E: no verdict row at all (only the claim row).
        let claim_e: u64 = 105;
        seed_claim(&db, session_id, claim_e).await;

        let awaiting = db
            .list_claims_awaiting_review(session_id, "pub-test")
            .await
            .expect("list_claims_awaiting_review");

        let ids: std::collections::BTreeSet<i64> = awaiting.iter().map(|c| c.claim_id).collect();
        let expected: std::collections::BTreeSet<i64> =
            [claim_a as i64, claim_d as i64].into_iter().collect();

        assert_eq!(
            ids, expected,
            "awaiting set must be {{A, D}}; got claim_ids: {:?}",
            ids
        );

        // Spot-check: the returned rows must have verdicts filled in.
        for row in &awaiting {
            assert!(
                row.verdict.is_some(),
                "awaiting claim {} must have a non-null verdict",
                row.claim_id
            );
        }
    }

    /// A terminal decision in publication A must NOT drop the same-text claim
    /// (same FNV-1a `claim_id`) from publication B's review queue.
    #[tokio::test]
    async fn list_claims_awaiting_review_is_scoped_per_publication() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let session_b: i64 = 9_002;
        let shared_claim: u64 = 4242; // same id would arise from identical claim text

        // The claim + verdict live in publication B's session.
        seed_claim(&db, session_b, shared_claim).await;
        seed_verdict(&db, shared_claim, "Supported").await;

        // An `approved` decision exists, but for publication A only.
        db.record_review_decision(&ReviewDecisionRow {
            claim_id: shared_claim as i64,
            publication_id: "pub-A".into(),
            bound_digest: "dig-A".into(),
            decision: "approved".into(),
            actor: "tester".into(),
            reason: None,
            model_fingerprints_json: None,
            decided_at_ms: 1,
        })
        .await
        .expect("record A decision");

        // Publication B never reviewed it → it MUST still be awaiting in B.
        let awaiting_b = db
            .list_claims_awaiting_review(session_b, "pub-B")
            .await
            .expect("list awaiting B");
        assert!(
            awaiting_b.iter().any(|c| c.claim_id == shared_claim as i64),
            "claim approved only in pub-A must remain awaiting in pub-B"
        );
    }
}
