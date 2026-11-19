//! Tier-2 verified documentation heal proposal staging store (`kb_heal_proposals`, `WIKI-04`).

use serde::{Deserialize, Serialize};
use turso::params;

use crate::schema_extensions::apply_kb_heal_proposals_table;
use crate::store::types::StoreError;

const VALID_PROPOSAL_STATUSES: &[&str] = &["pending", "applied", "rejected", "demoted_to_legacy"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewKbHealProposal {
    pub id: String,
    pub repository_id: String,
    pub doc_path: String,
    pub tier: i64,
    pub summary: String,
    pub unified_diff: String,
    pub original_hash: String,
    pub patched_markdown: String,
    pub evidence_citations: Vec<String>,
    pub gate_report_json: serde_json::Value,
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KbHealProposalRow {
    pub id: String,
    pub repository_id: String,
    pub doc_path: String,
    pub tier: i64,
    pub status: String,
    pub summary: String,
    pub unified_diff: String,
    pub original_hash: String,
    pub patched_markdown: String,
    pub evidence_citations: Vec<String>,
    pub gate_report_json: serde_json::Value,
    pub model_id: Option<String>,
    pub reviewed_by: Option<String>,
    pub review_note: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

const SELECT_PROPOSAL: &str = "SELECT id, repository_id, doc_path, tier, status, summary, \
    unified_diff, original_hash, patched_markdown, evidence_citations_json, gate_report_json, \
    model_id, reviewed_by, review_note, created_at_ms, updated_at_ms FROM kb_heal_proposals";

fn decode_row(r: &turso::Row) -> Result<KbHealProposalRow, StoreError> {
    let citations_raw = r.get::<String>(9)?;
    let gate_raw = r.get::<String>(10)?;
    Ok(KbHealProposalRow {
        id: r.get(0)?,
        repository_id: r.get(1)?,
        doc_path: r.get(2)?,
        tier: r.get(3)?,
        status: r.get(4)?,
        summary: r.get(5)?,
        unified_diff: r.get(6)?,
        original_hash: r.get(7)?,
        patched_markdown: r.get(8)?,
        evidence_citations: serde_json::from_str(&citations_raw).unwrap_or_default(),
        gate_report_json: serde_json::from_str(&gate_raw).unwrap_or_else(|_| serde_json::json!({})),
        model_id: r.get(11)?,
        reviewed_by: r.get(12)?,
        review_note: r.get(13)?,
        created_at_ms: r.get(14)?,
        updated_at_ms: r.get(15)?,
    })
}

impl crate::VoxDb {
    /// Insert or refresh a `pending` `kb_heal_proposals` record (`WIKI-04`).
    pub async fn kb_heal_proposal_upsert(
        &self,
        p: &NewKbHealProposal,
    ) -> Result<String, StoreError> {
        apply_kb_heal_proposals_table(&self.conn).await?;
        let now = crate::now_unix_ms() as i64;
        let citations_json = serde_json::to_string(&p.evidence_citations)
            .map_err(|e| StoreError::Db(format!("kb_heal_proposals citations: {e}")))?;
        let gate_json = p.gate_report_json.to_string();
        self.conn
            .execute(
                "INSERT INTO kb_heal_proposals \
                 (id, repository_id, doc_path, tier, status, summary, unified_diff, \
                  original_hash, patched_markdown, evidence_citations_json, gate_report_json, \
                  model_id, created_at_ms, updated_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12) \
                 ON CONFLICT(id) DO UPDATE SET \
                   repository_id = excluded.repository_id, \
                   doc_path = excluded.doc_path, \
                   tier = excluded.tier, \
                   status = 'pending', \
                   summary = excluded.summary, \
                   unified_diff = excluded.unified_diff, \
                   original_hash = excluded.original_hash, \
                   patched_markdown = excluded.patched_markdown, \
                   evidence_citations_json = excluded.evidence_citations_json, \
                   gate_report_json = excluded.gate_report_json, \
                   model_id = excluded.model_id, \
                   updated_at_ms = excluded.updated_at_ms",
                params![
                    p.id.as_str(),
                    p.repository_id.as_str(),
                    p.doc_path.as_str(),
                    p.tier,
                    p.summary.as_str(),
                    p.unified_diff.as_str(),
                    p.original_hash.as_str(),
                    p.patched_markdown.as_str(),
                    citations_json.as_str(),
                    gate_json.as_str(),
                    p.model_id.as_deref(),
                    now
                ],
            )
            .await?;
        Ok(p.id.clone())
    }

    /// List staged heal proposals filtered by optional `repository_id` and `status`.
    pub async fn kb_heal_proposals_list(
        &self,
        repository_id: Option<&str>,
        status: Option<&str>,
        limit: i64,
    ) -> Result<Vec<KbHealProposalRow>, StoreError> {
        apply_kb_heal_proposals_table(&self.conn).await?;
        let mut rows = match (repository_id, status) {
            (Some(repo), Some(st)) => {
                self.conn
                    .query(
                        &format!(
                            "{SELECT_PROPOSAL} WHERE repository_id = ?1 AND status = ?2 \
                             ORDER BY updated_at_ms DESC, id ASC LIMIT ?3"
                        ),
                        params![repo, st, limit],
                    )
                    .await?
            }
            (Some(repo), None) => {
                self.conn
                    .query(
                        &format!(
                            "{SELECT_PROPOSAL} WHERE repository_id = ?1 \
                             ORDER BY updated_at_ms DESC, id ASC LIMIT ?2"
                        ),
                        params![repo, limit],
                    )
                    .await?
            }
            (None, Some(st)) => {
                self.conn
                    .query(
                        &format!(
                            "{SELECT_PROPOSAL} WHERE status = ?1 \
                             ORDER BY updated_at_ms DESC, id ASC LIMIT ?2"
                        ),
                        params![st, limit],
                    )
                    .await?
            }
            (None, None) => {
                self.conn
                    .query(
                        &format!("{SELECT_PROPOSAL} ORDER BY updated_at_ms DESC, id ASC LIMIT ?1"),
                        params![limit],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(decode_row(&r)?);
        }
        Ok(out)
    }

    /// Fetch a single staged heal proposal by `id`.
    pub async fn kb_heal_proposal_get(
        &self,
        id: &str,
    ) -> Result<Option<KbHealProposalRow>, StoreError> {
        apply_kb_heal_proposals_table(&self.conn).await?;
        let mut rows = self
            .conn
            .query(
                &format!("{SELECT_PROPOSAL} WHERE id = ?1 LIMIT 1"),
                params![id],
            )
            .await?;
        match rows.next().await? {
            Some(r) => Ok(Some(decode_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Update proposal status (`pending` | `applied` | `rejected` | `demoted_to_legacy`).
    pub async fn kb_heal_proposal_update_status(
        &self,
        id: &str,
        status: &str,
        reviewed_by: Option<&str>,
        review_note: Option<&str>,
    ) -> Result<bool, StoreError> {
        if !VALID_PROPOSAL_STATUSES.contains(&status) {
            return Err(StoreError::Db(format!(
                "invalid kb_heal_proposals status '{status}'"
            )));
        }
        apply_kb_heal_proposals_table(&self.conn).await?;
        let now = crate::now_unix_ms() as i64;
        let updated = self
            .conn
            .execute(
                "UPDATE kb_heal_proposals \
                 SET status = ?1, reviewed_by = ?2, review_note = ?3, updated_at_ms = ?4 \
                 WHERE id = ?5",
                params![status, reviewed_by, review_note, now, id],
            )
            .await?;
        Ok(updated > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VoxDb;

    #[tokio::test]
    async fn kb_heal_proposals_upsert_filter_and_status_transitions() {
        let db = VoxDb::open_memory().await.unwrap();
        let id = db
            .kb_heal_proposal_upsert(&NewKbHealProposal {
                id: "heal-repoA-doc1".into(),
                repository_id: "repoA".into(),
                doc_path: "docs/wiki/interfaces-and-apis.md".into(),
                tier: 2,
                summary: "Update endpoint description".into(),
                unified_diff: "--- a/doc\n+++ b/doc\n@@ -1 +1 @@\n-old\n+new\n".into(),
                original_hash: "h0".into(),
                patched_markdown: "# Updated".into(),
                evidence_citations: vec!["[src/lib.rs:L1-L5]".into()],
                gate_report_json: serde_json::json!({"passed": true}),
                model_id: Some("vox-openwiki-v7.9".into()),
            })
            .await
            .unwrap();
        assert_eq!(id, "heal-repoA-doc1");

        let list = db
            .kb_heal_proposals_list(Some("repoA"), Some("pending"), 20)
            .await
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].evidence_citations, vec!["[src/lib.rs:L1-L5]"]);

        assert!(
            db.kb_heal_proposal_update_status(&id, "bogus", None, None)
                .await
                .is_err()
        );
        assert!(
            db.kb_heal_proposal_update_status(
                &id,
                "applied",
                Some("operator"),
                Some("Verified diff")
            )
            .await
            .unwrap()
        );

        let fetched = db.kb_heal_proposal_get(&id).await.unwrap().unwrap();
        assert_eq!(fetched.status, "applied");
        assert_eq!(fetched.reviewed_by.as_deref(), Some("operator"));
    }
}
