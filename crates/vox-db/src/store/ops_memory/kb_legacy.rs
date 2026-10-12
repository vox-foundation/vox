//! Read-only access to the legacy VoxKB tables (`knowledge_bases`, `kb_entries`,
//! `kb_routing_rules`) so their rows can be counted and migrated into `kb_findings` before the
//! tables are quarantined (RET-03, 03-08, 03-12).

use crate::store::types::StoreError;
use turso::params;

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
    async fn legacy_table_exists(&self, name: &str) -> Result<bool, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
                params![name],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// Row counts of the legacy VoxKB tables; zero for a table that no longer exists.
    // vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="kb-voxkb-retirement" canonical="kb_findings"
    pub async fn legacy_kb_counts(&self) -> Result<LegacyKbCounts, StoreError> {
        let mut out = [0_i64; 3];
        for (slot, table) in
            out.iter_mut()
                .zip(["knowledge_bases", "kb_entries", "kb_routing_rules"])
        {
            if self.legacy_table_exists(table).await? {
                let mut rows = self
                    .conn
                    .query(&format!("SELECT COUNT(*) FROM {table}"), ())
                    .await?;
                *slot = rows.next().await?.map_or(Ok(0), |r| r.get::<i64>(0))?;
            }
        }
        Ok(LegacyKbCounts {
            knowledge_bases: out[0],
            kb_entries: out[1],
            kb_routing_rules: out[2],
        })
    }

    /// Legacy entries in id order after the `after_id` cursor, joined with their KB name.
    /// Empty when the legacy tables are gone.
    // vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="kb-voxkb-retirement" canonical="kb_findings"
    pub async fn legacy_kb_entries_page(
        &self,
        after_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LegacyKbEntry>, StoreError> {
        if !self.legacy_table_exists("kb_entries").await?
            || !self.legacy_table_exists("knowledge_bases").await?
        {
            return Ok(Vec::new());
        }
        let mut rows = self
            .conn
            .query(
                "SELECT e.id, b.name, e.content, e.source_signal, e.source_ref, \
                        e.routing_confidence, e.accepted, e.mens_queued, e.created_at_ms \
                 FROM kb_entries e JOIN knowledge_bases b ON b.id = e.kb_id \
                 WHERE e.id > ?1 ORDER BY e.id ASC LIMIT ?2",
                params![after_id.unwrap_or(""), limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(LegacyKbEntry {
                id: r.get::<String>(0)?,
                kb_name: r.get::<String>(1)?,
                content: r.get::<String>(2)?,
                source_signal: r.get::<String>(3)?,
                source_ref: r.get::<Option<String>>(4)?,
                routing_confidence: r.get::<f64>(5)?,
                accepted: r.get::<i64>(6)? != 0,
                mens_queued: r.get::<i64>(7)? != 0,
                created_at_ms: r.get::<i64>(8)?,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::VoxDb;

    #[tokio::test]
    async fn kb_findings_legacy_readers_are_empty_on_a_fresh_database() {
        let db = VoxDb::open_memory().await.unwrap();
        assert_eq!(db.legacy_kb_counts().await.unwrap().kb_entries, 0);
        assert!(
            db.legacy_kb_entries_page(None, 10)
                .await
                .unwrap()
                .is_empty()
        );
    }
}
