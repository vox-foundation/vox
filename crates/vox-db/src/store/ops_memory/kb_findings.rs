//! KB findings inbox (baseline 96): every captured web/research/GUI finding lands here as
//! `pending` with its provenance, and can never carry a `Trusted` or `SystemVerified` tier.
//! Validity lives in the Rust types (Turso rejects CHECK and triggers): [`FindingTier`] cannot
//! express a trusted tier, and text fields are [`SanitizedText`], so an unsanitized string
//! cannot reach a row through this writer. The legacy VoxKB readers live in `kb_legacy.rs`.

use crate::kb_text::{ContentHash, FindingOrigin, FindingStatus, SanitizedText};
use crate::store::types::StoreError;
use std::collections::BTreeMap;
use turso::params;
use vox_orchestrator_types::trust::{ContextTrustTier, FindingTier};

/// One finding to capture. No `Default` on purpose: every writer chooses a tier and an origin.
///
/// A finding cannot be `Trusted`:
///
/// ```compile_fail
/// use vox_db::kb_text::{ContentHash, FindingOrigin, SanitizedText};
/// use vox_db::{FindingTier, NewKbFinding};
/// let _ = NewKbFinding {
///     dedupe_key: "k".into(),
///     tier: FindingTier::Trusted,
///     origin: FindingOrigin::WebSearch,
///     title: SanitizedText::new("t"),
///     summary: SanitizedText::new("s"),
///     quotes: vec![],
///     source_url: None,
///     fetched_at_ms: 0,
///     content_hash: ContentHash::from_blake3_hex(&"a".repeat(64)).unwrap(),
///     snapshot_relpath: None,
///     provenance: serde_json::json!({}),
/// };
/// ```
///
/// The same value with a permitted tier compiles:
///
/// ```no_run
/// use vox_db::kb_text::{ContentHash, FindingOrigin, SanitizedText};
/// use vox_db::{FindingTier, NewKbFinding};
/// let _ = NewKbFinding {
///     dedupe_key: "k".into(),
///     tier: FindingTier::Untrusted,
///     origin: FindingOrigin::WebSearch,
///     title: SanitizedText::new("t"),
///     summary: SanitizedText::new("s"),
///     quotes: vec![],
///     source_url: None,
///     fetched_at_ms: 0,
///     content_hash: ContentHash::from_blake3_hex(&"a".repeat(64)).unwrap(),
///     snapshot_relpath: None,
///     provenance: serde_json::json!({}),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct NewKbFinding {
    pub dedupe_key: String,
    pub tier: FindingTier,
    pub origin: FindingOrigin,
    pub title: SanitizedText,
    pub summary: SanitizedText,
    pub quotes: Vec<SanitizedText>,
    pub source_url: Option<SanitizedText>,
    pub fetched_at_ms: i64,
    pub content_hash: ContentHash,
    pub snapshot_relpath: Option<String>,
    pub provenance: serde_json::Value,
}

/// A stored finding.
#[derive(Debug, Clone, PartialEq)]
pub struct KbFindingRow {
    pub id: String,
    pub dedupe_key: String,
    pub status: FindingStatus,
    pub trust_tier: ContextTrustTier,
    pub origin: FindingOrigin,
    pub title: String,
    pub summary: String,
    pub quotes: Vec<String>,
    pub source_url: Option<String>,
    pub fetched_at_ms: i64,
    pub content_hash: String,
    pub snapshot_relpath: Option<String>,
    pub provenance: serde_json::Value,
    pub review_note: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_at_ms: Option<i64>,
    pub mens_queued: bool,
    pub created_at_ms: i64,
}

/// Inbox totals for `vox kb status`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KbFindingCounts {
    pub total: i64,
    pub pending: i64,
    pub by_origin: BTreeMap<String, i64>,
}

const FINDING_SELECT: &str = "SELECT id, dedupe_key, status, trust_tier, origin, title, summary, \
     quotes_json, source_url, fetched_at_ms, content_hash, snapshot_relpath, provenance_json, \
     review_note, reviewed_by, reviewed_at_ms, mens_queued, created_at_ms FROM kb_findings";

fn decode_err(what: &str, e: impl std::fmt::Display) -> StoreError {
    StoreError::Db(format!("kb_findings: {what}: {e}"))
}

/// Decode one row. An unknown `status` or `origin` is an error; an unknown tier reads as
/// `Untrusted` (fail closed).
fn finding_row(r: &turso::Row) -> Result<KbFindingRow, StoreError> {
    Ok(KbFindingRow {
        id: r.get::<String>(0)?,
        dedupe_key: r.get::<String>(1)?,
        status: FindingStatus::from_db_str(&r.get::<String>(2)?)
            .map_err(|e| decode_err("status", e))?,
        trust_tier: ContextTrustTier::from_db_str(&r.get::<String>(3)?),
        origin: FindingOrigin::from_db_str(&r.get::<String>(4)?)
            .map_err(|e| decode_err("origin", e))?,
        title: r.get::<String>(5)?,
        summary: r.get::<String>(6)?,
        quotes: serde_json::from_str(&r.get::<String>(7)?)
            .map_err(|e| decode_err("quotes_json", e))?,
        source_url: r.get::<Option<String>>(8)?,
        fetched_at_ms: r.get::<i64>(9)?,
        content_hash: r.get::<String>(10)?,
        snapshot_relpath: r.get::<Option<String>>(11)?,
        provenance: serde_json::from_str(&r.get::<String>(12)?)
            .map_err(|e| decode_err("provenance_json", e))?,
        review_note: r.get::<Option<String>>(13)?,
        reviewed_by: r.get::<Option<String>>(14)?,
        reviewed_at_ms: r.get::<Option<i64>>(15)?,
        mens_queued: r.get::<i64>(16)? != 0,
        created_at_ms: r.get::<i64>(17)?,
    })
}

impl crate::VoxDb {
    /// Insert a `pending` finding. `Some(id)` when a row was written, `None` when `dedupe_key`
    /// already exists. Status is always `pending` and the tier is the [`FindingTier`] wire
    /// string, so a finding never starts reviewed or trusted.
    pub async fn insert_pending_finding(
        &self,
        f: &NewKbFinding,
    ) -> Result<Option<String>, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        let quotes: Vec<&str> = f.quotes.iter().map(SanitizedText::as_str).collect();
        let quotes_json = serde_json::to_string(&quotes).map_err(|e| decode_err("quotes", e))?;
        let provenance_json = f.provenance.to_string();
        let inserted = self
            .conn
            .execute(
                "INSERT INTO kb_findings \
                 (id, dedupe_key, status, trust_tier, origin, title, summary, quotes_json, \
                  source_url, fetched_at_ms, content_hash, snapshot_relpath, provenance_json, \
                  mens_queued, created_at_ms) \
                 VALUES (?1, ?2, 'pending', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 0, ?13) \
                 ON CONFLICT(dedupe_key) DO NOTHING",
                params![
                    id.as_str(),
                    f.dedupe_key.as_str(),
                    f.tier.as_db_str(),
                    f.origin.as_db_str(),
                    f.title.as_str(),
                    f.summary.as_str(),
                    quotes_json.as_str(),
                    f.source_url.as_ref().map(SanitizedText::as_str),
                    f.fetched_at_ms,
                    f.content_hash.as_str(),
                    f.snapshot_relpath.as_deref(),
                    provenance_json.as_str(),
                    crate::now_unix_ms() as i64
                ],
            )
            .await?;
        Ok((inserted > 0).then_some(id))
    }

    /// Findings newest first (`created_at_ms DESC, id ASC`, so equal timestamps have a stable
    /// order), optionally of one status.
    pub async fn kb_findings_list(
        &self,
        status: Option<FindingStatus>,
        limit: i64,
    ) -> Result<Vec<KbFindingRow>, StoreError> {
        let order = "ORDER BY created_at_ms DESC, id ASC";
        let mut rows = match status {
            Some(s) => {
                self.conn
                    .query(
                        &format!("{FINDING_SELECT} WHERE status = ?1 {order} LIMIT ?2"),
                        params![s.as_db_str(), limit],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        &format!("{FINDING_SELECT} {order} LIMIT ?1"),
                        params![limit],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(finding_row(&r)?);
        }
        Ok(out)
    }

    pub async fn kb_findings_counts(&self) -> Result<KbFindingCounts, StoreError> {
        let mut counts = KbFindingCounts::default();
        let mut rows = self
            .conn
            .query(
                "SELECT origin, COUNT(*) FROM kb_findings GROUP BY origin",
                (),
            )
            .await?;
        while let Some(r) = rows.next().await? {
            let n = r.get::<i64>(1)?;
            counts.total += n;
            counts.by_origin.insert(r.get::<String>(0)?, n);
        }
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM kb_findings WHERE status = 'pending'",
                (),
            )
            .await?;
        counts.pending = rows.next().await?.map_or(Ok(0), |r| r.get::<i64>(0))?;
        Ok(counts)
    }

    /// Human-reviewed (approved or rejected, with a reviewer) findings not yet handed to MENS,
    /// oldest first.
    pub async fn kb_findings_unqueued_reviewed(
        &self,
        limit: i64,
    ) -> Result<Vec<KbFindingRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                &format!(
                    "{FINDING_SELECT} WHERE reviewed_by IS NOT NULL \
                     AND status IN ('approved', 'rejected') AND mens_queued = 0 \
                     ORDER BY created_at_ms ASC, id ASC LIMIT ?1"
                ),
                params![limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(finding_row(&r)?);
        }
        Ok(out)
    }

    pub async fn kb_findings_mark_mens_queued(&self, ids: &[String]) -> Result<(), StoreError> {
        for id in ids {
            self.conn
                .execute(
                    "UPDATE kb_findings SET mens_queued = 1 WHERE id = ?1",
                    params![id.as_str()],
                )
                .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VoxDb;

    fn sample(key: &str) -> NewKbFinding {
        NewKbFinding {
            dedupe_key: key.into(),
            tier: FindingTier::Untrusted,
            origin: FindingOrigin::WebSearch,
            title: SanitizedText::new("t"),
            summary: SanitizedText::new("s"),
            quotes: vec![SanitizedText::new("q")],
            source_url: None,
            fetched_at_ms: 1,
            content_hash: ContentHash::from_blake3_hex(&"c".repeat(64)).unwrap(),
            snapshot_relpath: None,
            provenance: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn kb_findings_insert_returns_an_id_once_per_dedupe_key() {
        let db = VoxDb::open_memory().await.unwrap();
        assert!(
            db.insert_pending_finding(&sample("k"))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            db.insert_pending_finding(&sample("k"))
                .await
                .unwrap()
                .is_none()
        );
        let rows = db.kb_findings_list(None, 10).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].status, FindingStatus::Pending);
    }
}
