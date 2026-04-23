use crate::{VoxDb, store::StoreError};
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSourceRow {
    pub id: String,
    pub url: String,
    pub source_kind: String,
    pub crawl_interval_ms: i64,
    pub enabled: bool,
    pub last_crawled_at_ms: i64,
    pub last_error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalIntelligenceRow {
    pub id: String,
    pub source_url: String,
    pub source_kind: String,
    pub title: String,
    pub abstract_text: Option<String>,
    pub embedding_id: Option<String>,
    pub provenance_json: String,
    pub ingest_status: String,
    pub preflight_score: Option<f64>,
    pub ingested_at_ms: i64,
    pub reviewed_at_ms: Option<i64>,
    pub socrates_risk_band: Option<String>,
    pub socrates_confidence: Option<f64>,
    pub worthiness_score: Option<f64>,
    pub claim_evidence_coverage: Option<f64>,
}

impl VoxDb {
    pub async fn upsert_feed_source(
        &self,
        id: &str,
        url: &str,
        source_kind: &str,
        crawl_interval_ms: i64,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let url = url.to_string();
        let source_kind = source_kind.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| async move {
            conn.execute(
                "INSERT INTO scientia_feed_sources (id, url, source_kind, crawl_interval_ms, enabled, last_crawled_at_ms)
                 VALUES (?1, ?2, ?3, ?4, 1, 0)
                 ON CONFLICT(id) DO UPDATE SET
                    url = excluded.url,
                    source_kind = excluded.source_kind,
                    crawl_interval_ms = excluded.crawl_interval_ms",
                turso::params![id, url, source_kind, crawl_interval_ms],
            ).await?;
            Ok::<(), StoreError>(())
        }).await
    }

    pub async fn upsert_external_intelligence_pending(
        &self,
        source_url: &str,
        source_kind: &str,
        title: &str,
        abstract_text: Option<&str>,
    ) -> Result<(), StoreError> {
        let hash = blake3::hash(source_url.as_bytes());
        let id = hash.to_string();

        let provenance = serde_json::json!({
            "source_url": source_url,
            "source_kind": source_kind,
            "ingested_via": "vox-scientia-ingest-tick"
        })
        .to_string();

        self.upsert_external_intelligence(
            &id,
            source_url,
            source_kind,
            title,
            abstract_text,
            &provenance,
        )
        .await
    }

    pub async fn upsert_external_intelligence(
        &self,
        id: &str,
        source_url: &str,
        source_kind: &str,
        title: &str,
        abstract_text: Option<&str>,
        provenance_json: &str,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let source_url = source_url.to_string();
        let source_kind = source_kind.to_string();
        let title = title.to_string();
        let abstract_text = abstract_text.map(str::to_string);
        let provenance_json = provenance_json.to_string();

        let ingested_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| async move {
            conn.execute(
                "INSERT INTO scientia_external_intelligence 
                 (id, source_url, source_kind, title, abstract_text, provenance_json, ingest_status, ingested_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)
                 ON CONFLICT(id) DO NOTHING",
                turso::params![id, source_url, source_kind, title, abstract_text.as_deref(), provenance_json, ingested_at_ms],
            ).await?;
            Ok::<(), StoreError>(())
        }).await
    }

    pub async fn socrates_record_abstain_event(
        &self,
        event_type: &str,
        query: &str,
        reason: &str,
    ) -> Result<(), StoreError> {
        let event_type = event_type.to_string();
        let query = query.to_string();
        let reason = reason.to_string();

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let metadata = serde_json::json!({
            "query": query,
            "reason": reason
        })
        .to_string();

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| async move {
            conn.execute(
                "INSERT INTO agent_events (agent_id, session_id, event_type, metadata_json, created_at_ms)
                 VALUES ('socrates', 'system', ?1, ?2, ?3)",
                turso::params![event_type, metadata, now],
            ).await?;
            Ok::<(), StoreError>(())
        }).await
    }

    pub async fn list_feed_sources(&self) -> Result<Vec<FeedSourceRow>, StoreError> {
        let sql = "SELECT id, url, source_kind, crawl_interval_ms, enabled, last_crawled_at_ms, last_error FROM scientia_feed_sources WHERE enabled = 1";
        let mut rows = self.conn.query(sql, ()).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(FeedSourceRow {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                url: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                source_kind: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                crawl_interval_ms: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                enabled: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                last_crawled_at_ms: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                last_error: row.get(6).unwrap_or_default(),
            });
        }
        Ok(results)
    }

    pub async fn list_pending_external_intelligence(
        &self,
        limit: i64,
    ) -> Result<Vec<ExternalIntelligenceRow>, StoreError> {
        let sql = "SELECT id, source_url, source_kind, title, abstract_text, embedding_id, provenance_json, ingest_status, preflight_score, ingested_at_ms, reviewed_at_ms,
                          socrates_risk_band, socrates_confidence, worthiness_score, claim_evidence_coverage
                   FROM scientia_external_intelligence 
                   WHERE ingest_status = 'pending' 
                   ORDER BY ingested_at_ms ASC LIMIT ?1";
        let mut rows = self.conn.query(sql, turso::params![limit]).await?;
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(ExternalIntelligenceRow {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                source_url: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                source_kind: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                title: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                abstract_text: row.get(4).unwrap_or_default(),
                embedding_id: row.get(5).unwrap_or_default(),
                provenance_json: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                ingest_status: row.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                preflight_score: row.get(8).unwrap_or_default(),
                ingested_at_ms: row.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                reviewed_at_ms: row.get(10).unwrap_or_default(),
                socrates_risk_band: row.get(11).unwrap_or_default(),
                socrates_confidence: row.get(12).unwrap_or_default(),
                worthiness_score: row.get(13).unwrap_or_default(),
                claim_evidence_coverage: row.get(14).unwrap_or_default(),
            });
        }
        Ok(results)
    }

    pub async fn update_external_intelligence_enriched_scores(
        &self,
        id: &str,
        socrates_risk_band: &str,
        socrates_confidence: f64,
        worthiness_score: f64,
        claim_evidence_coverage: f64,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let socrates_risk_band = socrates_risk_band.to_string();

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE scientia_external_intelligence SET
                    socrates_risk_band = ?1,
                    socrates_confidence = ?2,
                    worthiness_score = ?3,
                    claim_evidence_coverage = ?4,
                    ingest_status = 'enriched'
                 WHERE id = ?5",
                    turso::params![
                        socrates_risk_band,
                        socrates_confidence,
                        worthiness_score,
                        claim_evidence_coverage,
                        id
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }
}
