//! KB findings inbox (baseline 96): every captured web/research/GUI finding lands here as
//! `pending` with its provenance, and can never carry a `Trusted` or `SystemVerified` tier.
//! Validity lives in the Rust types (Turso rejects CHECK and triggers): [`FindingTier`] cannot
//! express a trusted tier, and text fields are [`SanitizedText`], so an unsanitized string
//! cannot reach a row through this writer.

use crate::kb_text::{ContentHash, FindingOrigin, FindingStatus, SanitizedText};
use crate::store::types::StoreError;
use std::collections::BTreeMap;
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

/// Row counts of the three legacy VoxKB tables (zero when a table is absent).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacyKbCounts {
    pub knowledge_bases: i64,
    pub kb_entries: i64,
    pub kb_routing_rules: i64,
}

/// One legacy `kb_entries` row joined with its knowledge-base name.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyKbEntry {
    pub id: String,
    pub kb_name: String,
    pub content: String,
    pub source_signal: String,
    pub source_ref: Option<String>,
    pub routing_confidence: f64,
    pub accepted: bool,
    pub mens_queued: bool,
    pub created_at_ms: i64,
}

impl crate::VoxDb {
    /// Insert a `pending` finding. `Some(id)` when a row was written, `None` when `dedupe_key`
    /// already exists.
    pub async fn insert_pending_finding(
        &self,
        _f: &NewKbFinding,
    ) -> Result<Option<String>, StoreError> {
        Ok(None)
    }

    /// Findings newest first (`created_at_ms DESC, id ASC`), optionally of one status.
    pub async fn kb_findings_list(
        &self,
        _status: Option<FindingStatus>,
        _limit: i64,
    ) -> Result<Vec<KbFindingRow>, StoreError> {
        Err(StoreError::Db("kb_findings: not implemented".into()))
    }

    pub async fn kb_findings_counts(&self) -> Result<KbFindingCounts, StoreError> {
        Ok(KbFindingCounts::default())
    }

    /// Human-reviewed (approved or rejected) findings not yet handed to MENS.
    pub async fn kb_findings_unqueued_reviewed(
        &self,
        _limit: i64,
    ) -> Result<Vec<KbFindingRow>, StoreError> {
        Err(StoreError::Db("kb_findings: not implemented".into()))
    }

    pub async fn kb_findings_mark_mens_queued(&self, _ids: &[String]) -> Result<(), StoreError> {
        Err(StoreError::Db("kb_findings: not implemented".into()))
    }

    // vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="kb-voxkb-retirement" canonical="kb_findings"
    pub async fn legacy_kb_counts(&self) -> Result<LegacyKbCounts, StoreError> {
        Ok(LegacyKbCounts::default())
    }

    // vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="kb-voxkb-retirement" canonical="kb_findings"
    pub async fn legacy_kb_entries_page(
        &self,
        _after_id: Option<&str>,
        _limit: i64,
    ) -> Result<Vec<LegacyKbEntry>, StoreError> {
        Err(StoreError::Db("kb_findings: not implemented".into()))
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
