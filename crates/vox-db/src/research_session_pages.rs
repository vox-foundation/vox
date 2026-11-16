//! Cursor-paginated session claims & sources queries and continuum metadata upserts.

use crate::VoxDb;
use crate::store::StoreError;
use turso::params;
use vox_db_types::{
    NewResearchSessionSource, ResearchSessionSourceRecord, SessionClaimPageItem, SessionClaimsPage,
    SessionSourcesPage,
};

fn trim_page<T>(items: &mut Vec<T>, clamped: usize, id_of: impl Fn(&T) -> i64) -> Option<i64> {
    if items.len() > clamped {
        items.truncate(clamped);
        items.last().map(id_of)
    } else {
        None
    }
}

impl VoxDb {
    async fn count_session_rows(&self, sql: &str, session_id: i64) -> Result<i64, StoreError> {
        let mut rows = self.conn.query(sql, params![session_id]).await?;
        Ok(match rows.next().await? {
            Some(r) => r.get(0)?,
            None => 0,
        })
    }

    /// Insert or update a research session source deduplicated on `(session_id, url)`.
    pub async fn upsert_session_source(
        &self,
        src: &NewResearchSessionSource,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO research_session_sources \
                 (session_id, wave, url, title, snippet, provider, domain, relevance_score, created_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                 ON CONFLICT(session_id, url) DO UPDATE SET \
                   wave = CASE WHEN excluded.wave > research_session_sources.wave THEN excluded.wave ELSE research_session_sources.wave END, \
                   title = CASE WHEN TRIM(excluded.title) <> '' THEN excluded.title ELSE research_session_sources.title END, \
                   snippet = CASE WHEN TRIM(excluded.snippet) <> '' THEN excluded.snippet ELSE research_session_sources.snippet END, \
                   provider = CASE WHEN TRIM(excluded.provider) <> '' THEN excluded.provider ELSE research_session_sources.provider END, \
                   domain = CASE WHEN TRIM(excluded.domain) <> '' THEN excluded.domain ELSE research_session_sources.domain END, \
                   relevance_score = CASE WHEN excluded.relevance_score > research_session_sources.relevance_score THEN excluded.relevance_score ELSE research_session_sources.relevance_score END",
                params![
                    src.session_id,
                    src.wave,
                    src.url.as_str(),
                    src.title.as_str(),
                    src.snippet.as_str(),
                    src.provider.as_str(),
                    src.domain.as_str(),
                    src.relevance_score,
                    src.created_at_ms,
                ],
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT id FROM research_session_sources WHERE session_id = ?1 AND url = ?2",
                params![src.session_id, src.url.as_str()],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(format!("research_session_sources {}", src.url)))?;
        Ok(row.get(0)?)
    }

    /// Update `continuum_tier` and `escalated_from` on `scientia_research_sessions`
    /// and mirror to `research_sessions` when a matching `session_key` row exists.
    pub async fn update_session_continuum_metadata(
        &self,
        session_id: i64,
        continuum_tier: &str,
        escalated_from: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE scientia_research_sessions SET continuum_tier = ?1, escalated_from = ?2 WHERE id = ?3",
                params![continuum_tier, escalated_from, session_id],
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT session_key FROM scientia_research_sessions WHERE id = ?1",
                params![session_id],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            let key: String = row.get(0)?;
            self.conn
                .execute(
                    "UPDATE research_sessions SET continuum_tier = ?1, escalated_from = ?2, updated_at = datetime('now') WHERE session_key = ?3",
                    params![continuum_tier, escalated_from, key.as_str()],
                )
                .await?;
        }
        Ok(())
    }

    /// Read `(continuum_tier, escalated_from)` from `scientia_research_sessions` by row id.
    pub async fn get_session_continuum_metadata(
        &self,
        session_id: i64,
    ) -> Result<Option<(Option<String>, Option<String>)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT continuum_tier, escalated_from FROM scientia_research_sessions WHERE id = ?1",
                params![session_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(None);
        };
        Ok(Some((row.get(0)?, row.get(1)?)))
    }

    /// Deterministically paginate `scientia_claims` for `session_id` ordered by `c.id ASC`.
    pub async fn list_session_claims_page(
        &self,
        session_id: i64,
        after_row_id: Option<i64>,
        limit: u32,
    ) -> Result<SessionClaimsPage, StoreError> {
        let clamped = limit.clamp(1, 200) as usize;
        let total_count = self
            .count_session_rows(
                "SELECT COUNT(*) FROM scientia_claims WHERE session_id = ?1",
                session_id,
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT c.id, c.claim_id, c.session_id, c.text, \
                   (SELECT v.verdict FROM scientia_claim_verdicts v WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1), \
                   (SELECT v.confidence FROM scientia_claim_verdicts v WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1), \
                   (SELECT v.verifier_model FROM scientia_claim_verdicts v WHERE v.claim_id = c.claim_id AND v.verdict <> 'Unverified' ORDER BY v.created_at_ms DESC, v.id DESC LIMIT 1), \
                   c.created_at_ms \
                 FROM scientia_claims c WHERE c.session_id = ?1 AND c.id > ?2 ORDER BY c.id ASC LIMIT ?3",
                params![session_id, after_row_id.unwrap_or(0), (clamped + 1) as i64],
            )
            .await?;
        let mut items = Vec::with_capacity(clamped + 1);
        while let Some(r) = rows.next().await? {
            items.push(SessionClaimPageItem {
                row_id: r.get(0)?,
                claim_id: r.get(1)?,
                session_id: r.get(2)?,
                text: r.get(3)?,
                verdict: r.get(4)?,
                confidence: r.get(5)?,
                verifier_model: r.get(6)?,
                created_at_ms: r.get(7)?,
            });
        }
        let next_cursor = trim_page(&mut items, clamped, |i| i.row_id);
        Ok(SessionClaimsPage {
            items,
            next_cursor,
            total_count,
        })
    }

    /// Deterministically paginate `research_session_sources` for `session_id` ordered by `id ASC`.
    pub async fn list_session_sources_page(
        &self,
        session_id: i64,
        after_source_id: Option<i64>,
        limit: u32,
    ) -> Result<SessionSourcesPage, StoreError> {
        let clamped = limit.clamp(1, 200) as usize;
        let total_count = self
            .count_session_rows(
                "SELECT COUNT(*) FROM research_session_sources WHERE session_id = ?1",
                session_id,
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT id, session_id, wave, url, title, snippet, provider, domain, relevance_score, created_at_ms \
                 FROM research_session_sources WHERE session_id = ?1 AND id > ?2 ORDER BY id ASC LIMIT ?3",
                params![session_id, after_source_id.unwrap_or(0), (clamped + 1) as i64],
            )
            .await?;
        let mut items = Vec::with_capacity(clamped + 1);
        while let Some(r) = rows.next().await? {
            items.push(ResearchSessionSourceRecord {
                id: r.get(0)?,
                session_id: r.get(1)?,
                wave: r.get(2)?,
                url: r.get(3)?,
                title: r.get(4)?,
                snippet: r.get(5)?,
                provider: r.get(6)?,
                domain: r.get(7)?,
                relevance_score: r.get(8)?,
                created_at_ms: r.get(9)?,
            });
        }
        let next_cursor = trim_page(&mut items, clamped, |i| i.id);
        Ok(SessionSourcesPage {
            items,
            next_cursor,
            total_count,
        })
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use crate::{DbConfig, NewResearchSessionSource, VoxDb};
    use std::collections::HashSet;

    fn src_row(
        sid: i64,
        wave: i64,
        url: &str,
        title: &str,
        snip: &str,
        score: f64,
    ) -> NewResearchSessionSource {
        NewResearchSessionSource {
            session_id: sid,
            wave,
            url: url.into(),
            title: title.into(),
            snippet: snip.into(),
            provider: "openalex".into(),
            domain: "scholar.example.org".into(),
            relevance_score: score,
            created_at_ms: 1_700_000_000_000,
        }
    }

    #[tokio::test]
    async fn cursor_pagination_55_sources_and_45_claims() {
        let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
        let sid = db.create_research_session("sess-page", "q").await.unwrap();
        for i in 0..55i64 {
            let u = format!("https://ex.org/{i:03}");
            db.upsert_session_source(&src_row(sid, 1, &u, "P", "S", 0.5))
                .await
                .unwrap();
        }
        for i in 0..45u64 {
            let cid = 10_000 + i;
            db.store_claim(sid, cid, &format!("Claim {i}"), i % 2 == 0, false, false)
                .await
                .unwrap();
            db.store_evidence_span(cid, 0, 10, "span").await.unwrap();
            if i % 3 == 0 {
                db.store_claim_verdict(cid, "Supported", 0.91, "minicheck-v1")
                    .await
                    .unwrap();
            }
        }
        let s1 = db.list_session_sources_page(sid, None, 20).await.unwrap();
        let s2 = db
            .list_session_sources_page(sid, s1.next_cursor, 20)
            .await
            .unwrap();
        let s3 = db
            .list_session_sources_page(sid, s2.next_cursor, 20)
            .await
            .unwrap();
        assert_eq!(
            [s1.items.len(), s2.items.len(), s3.items.len()],
            [20, 20, 15]
        );
        assert_eq!(
            [s1.total_count, s2.total_count, s3.total_count],
            [55, 55, 55]
        );
        assert!(s1.next_cursor.is_some() && s2.next_cursor.is_some() && s3.next_cursor.is_none());
        let sids: Vec<i64> = [s1.items, s2.items, s3.items]
            .concat()
            .into_iter()
            .map(|r| r.id)
            .collect();
        assert!(
            sids.windows(2).all(|w| w[0] < w[1]) && sids.iter().collect::<HashSet<_>>().len() == 55
        );

        let c1 = db.list_session_claims_page(sid, None, 20).await.unwrap();
        let c2 = db
            .list_session_claims_page(sid, c1.next_cursor, 20)
            .await
            .unwrap();
        let c3 = db
            .list_session_claims_page(sid, c2.next_cursor, 20)
            .await
            .unwrap();
        assert_eq!(
            [c1.items.len(), c2.items.len(), c3.items.len()],
            [20, 20, 5]
        );
        assert_eq!(
            [c1.total_count, c2.total_count, c3.total_count],
            [45, 45, 45]
        );
        assert!(c1.next_cursor.is_some() && c2.next_cursor.is_some() && c3.next_cursor.is_none());
        assert_eq!(c1.items[0].verdict.as_deref(), Some("Supported"));
        assert_eq!(c1.items[1].verdict, None);
        let cids: Vec<i64> = [c1.items, c2.items, c3.items]
            .concat()
            .into_iter()
            .map(|r| r.row_id)
            .collect();
        assert!(cids.len() == 45 && cids.windows(2).all(|w| w[0] < w[1]));
    }

    #[tokio::test]
    async fn upsert_session_source_deduplicates_session_url() {
        let db = VoxDb::connect(DbConfig::Memory).await.unwrap();
        let sid = db
            .create_research_session("sess-dedup", "dedup")
            .await
            .unwrap();
        let u = "https://arxiv.org/abs/2602.12345";
        let id1 = db
            .upsert_session_source(&src_row(sid, 1, u, "Init", "InitSnip", 0.45))
            .await
            .unwrap();
        let id2 = db
            .upsert_session_source(&src_row(sid, 2, u, "", "RefinedSnip", 0.89))
            .await
            .unwrap();
        let id3 = db
            .upsert_session_source(&src_row(sid, 3, u, "FinalTitle", "", 0.30))
            .await
            .unwrap();
        assert_eq!([id1, id2], [id2, id3]);
        let p = db.list_session_sources_page(sid, None, 20).await.unwrap();
        assert_eq!((p.total_count, p.items.len()), (1, 1));
        assert!((p.items[0].relevance_score - 0.89).abs() < 1e-9);
        assert_eq!(p.items[0].title, "FinalTitle");
        assert_eq!(p.items[0].snippet, "RefinedSnip");
    }

    #[tokio::test]
    async fn upgrade_pre_v99_sqlite_adds_continuum_columns_without_data_loss() {
        let dir = tempfile::tempdir().unwrap();
        let path_str = dir.path().join("pre_v99.db").to_string_lossy().into_owned();
        {
            let raw = turso::Builder::new_local(&path_str).build().await.unwrap();
            raw.connect().unwrap().execute_batch(
                "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (datetime('now')));
                 INSERT INTO schema_version (version) VALUES (98);
                 CREATE TABLE scientia_research_sessions (id INTEGER PRIMARY KEY AUTOINCREMENT, session_key TEXT NOT NULL UNIQUE, status TEXT NOT NULL DEFAULT 'active', started_at_ms INTEGER NOT NULL, finished_at_ms INTEGER, query_text TEXT, hit_count INTEGER NOT NULL DEFAULT 0, claim_count INTEGER NOT NULL DEFAULT 0, quality_score INTEGER NOT NULL DEFAULT 0);
                 CREATE TABLE research_sessions (id INTEGER PRIMARY KEY AUTOINCREMENT, session_key TEXT NOT NULL UNIQUE, title TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'active', repository_id TEXT NOT NULL DEFAULT '', config_json TEXT, summary_json TEXT, created_at TEXT NOT NULL DEFAULT (datetime('now')), updated_at TEXT NOT NULL DEFAULT (datetime('now')));
                 INSERT INTO scientia_research_sessions (session_key, status, started_at_ms, query_text) VALUES ('sess-pre-v99', 'active', 1700000000000, 'q');
                 INSERT INTO research_sessions (session_key, title, status) VALUES ('sess-pre-v99', 'Legacy', 'active');",
            ).await.unwrap();
        }
        let db = VoxDb::connect(DbConfig::local(&path_str)).await.unwrap();
        assert_eq!(
            db.get_research_session(1)
                .await
                .unwrap()
                .unwrap()
                .session_key,
            "sess-pre-v99"
        );
        assert_eq!(
            db.get_session_continuum_metadata(1).await.unwrap(),
            Some((None, None))
        );
        db.update_session_continuum_metadata(1, "deep_research", Some("fast_search"))
            .await
            .unwrap();
        let expected = (
            Some("deep_research".to_string()),
            Some("fast_search".to_string()),
        );
        assert_eq!(
            db.get_session_continuum_metadata(1).await.unwrap(),
            Some(expected.clone())
        );
        let mut r = db.connection().query("SELECT continuum_tier, escalated_from FROM research_sessions WHERE session_key = 'sess-pre-v99'", ()).await.unwrap();
        let row = r.next().await.unwrap().unwrap();
        assert_eq!((row.get(0).unwrap(), row.get(1).unwrap()), expected);
    }
}
