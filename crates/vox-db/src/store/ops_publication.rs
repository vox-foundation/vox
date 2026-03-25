use crate::{VoxDb, store::StoreError, store::types::*};

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

impl VoxDb {
    /// Insert or update the canonical publication manifest.
    pub async fn upsert_publication_manifest(
        &self,
        params: PublicationManifestParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_manifests (
                    publication_id, content_type, source_ref, title, author, abstract_text,
                    body_markdown, citations_json, metadata_json, content_sha3_256, state,
                    created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)
                ON CONFLICT(publication_id) DO UPDATE SET
                    content_type=excluded.content_type,
                    source_ref=excluded.source_ref,
                    title=excluded.title,
                    author=excluded.author,
                    abstract_text=excluded.abstract_text,
                    body_markdown=excluded.body_markdown,
                    citations_json=excluded.citations_json,
                    metadata_json=excluded.metadata_json,
                    content_sha3_256=excluded.content_sha3_256,
                    state=excluded.state,
                    version=CASE
                        WHEN publication_manifests.content_sha3_256 = excluded.content_sha3_256
                            THEN publication_manifests.version
                        ELSE publication_manifests.version + 1
                    END,
                    updated_at_ms=excluded.updated_at_ms",
                (
                    params.publication_id.to_string(),
                    params.content_type.to_string(),
                    params.source_ref.map(std::string::ToString::to_string),
                    params.title.to_string(),
                    params.author.to_string(),
                    params.abstract_text.map(std::string::ToString::to_string),
                    params.body_markdown.to_string(),
                    params.citations_json.map(std::string::ToString::to_string),
                    params.metadata_json.map(std::string::ToString::to_string),
                    params.content_sha3_256.to_string(),
                    params.state.to_string(),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// Fetch one manifest by id.
    pub async fn get_publication_manifest(
        &self,
        publication_id: &str,
    ) -> Result<Option<PublicationManifestRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT publication_id, content_type, source_ref, title, author, abstract_text, body_markdown, citations_json, metadata_json, content_sha3_256, version, state, created_at_ms, updated_at_ms FROM publication_manifests WHERE publication_id = ?1",
                (publication_id.to_string(),),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(PublicationManifestRow {
            publication_id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            content_type: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            source_ref: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            title: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            author: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            abstract_text: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            body_markdown: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            citations_json: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            metadata_json: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
            version: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
            state: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
            created_at_ms: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
            updated_at_ms: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Digest-bound approval for any publication type.
    pub async fn record_publication_approval_for_digest(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
        approver: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO publication_approvals (publication_id, content_sha3_256, approver, approved_at_ms) VALUES (?1, ?2, ?3, ?4)",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                    approver.to_string(),
                    now_ms(),
                ),
            )
            .await?;
        Ok(())
    }

    /// Count distinct approvers for id+digest.
    pub async fn count_publication_approvers_for_digest(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
    ) -> Result<i64, StoreError> {
        let rows = self
            .query_all(
                "SELECT COUNT(DISTINCT approver) AS c FROM publication_approvals WHERE publication_id = ?1 AND content_sha3_256 = ?2",
                (publication_id.to_string(), content_sha3_256.to_string()),
            )
            .await?;
        let row = rows
            .first()
            .ok_or_else(|| StoreError::Db("publication approval count: no row".into()))?;
        row.get(0).map_err(|e| StoreError::Db(e.to_string()))
    }

    /// True when at least two distinct approvers exist for id+digest.
    pub async fn has_dual_publication_approval_for_digest(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
    ) -> Result<bool, StoreError> {
        Ok(self
            .count_publication_approvers_for_digest(publication_id, content_sha3_256)
            .await?
            >= 2)
    }

    /// Record one publication attempt outcome for a delivery channel.
    pub async fn record_publication_attempt(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
        channel: &str,
        outcome_json: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO publication_attempts (publication_id, content_sha3_256, channel, attempted_at_ms, outcome_json) VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                    channel.to_string(),
                    now_ms(),
                    outcome_json.to_string(),
                ),
            )
            .await?;
        Ok(())
    }

    /// Set the current manifest state and append an immutable status event.
    pub async fn set_publication_state(
        &self,
        publication_id: &str,
        state: &str,
        detail_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "UPDATE publication_manifests SET state = ?2, updated_at_ms = ?3 WHERE publication_id = ?1",
                (publication_id.to_string(), state.to_string(), ts),
            )
            .await?;
        self.conn
            .execute(
                "INSERT INTO publication_status_events (publication_id, status, detail_json, recorded_at_ms) VALUES (?1, ?2, ?3, ?4)",
                (
                    publication_id.to_string(),
                    state.to_string(),
                    detail_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// Insert or update one scholarly submission record and mirror lifecycle state.
    pub async fn upsert_scholarly_submission(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
        adapter: &str,
        external_submission_id: &str,
        status: &str,
        response_fingerprint: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO scholarly_submissions (
                    publication_id, content_sha3_256, adapter, external_submission_id, status,
                    submitted_at_ms, updated_at_ms, response_fingerprint, metadata_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)
                ON CONFLICT(adapter, external_submission_id) DO UPDATE SET
                    status = excluded.status,
                    updated_at_ms = excluded.updated_at_ms,
                    response_fingerprint = excluded.response_fingerprint,
                    metadata_json = excluded.metadata_json",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                    adapter.to_string(),
                    external_submission_id.to_string(),
                    status.to_string(),
                    ts,
                    response_fingerprint.map(std::string::ToString::to_string),
                    metadata_json.map(std::string::ToString::to_string),
                ),
            )
            .await?;
        self.set_publication_state(
            publication_id,
            status,
            Some(
                &serde_json::json!({
                    "adapter": adapter,
                    "external_submission_id": external_submission_id
                })
                .to_string(),
            ),
        )
        .await?;
        Ok(())
    }

    /// Insert or update one publication media asset row.
    pub async fn upsert_publication_media_asset(
        &self,
        params: PublicationMediaAssetParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_media_assets (
                    publication_id, asset_ref, media_type, storage_uri, status, metadata_json, created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
                ON CONFLICT(publication_id, asset_ref) DO UPDATE SET
                    media_type = excluded.media_type,
                    storage_uri = excluded.storage_uri,
                    status = excluded.status,
                    metadata_json = excluded.metadata_json,
                    updated_at_ms = excluded.updated_at_ms",
                (
                    params.publication_id.to_string(),
                    params.asset_ref.to_string(),
                    params.media_type.to_string(),
                    params.storage_uri.map(std::string::ToString::to_string),
                    params.status.to_string(),
                    params.metadata_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// List media assets for one publication.
    pub async fn list_publication_media_assets(
        &self,
        publication_id: &str,
    ) -> Result<Vec<PublicationMediaAssetRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, asset_ref, media_type, storage_uri, status, metadata_json, created_at_ms, updated_at_ms
                 FROM publication_media_assets
                 WHERE publication_id = ?1
                 ORDER BY id DESC",
                (publication_id.to_string(),),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(PublicationMediaAssetRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    asset_ref: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    media_type: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    storage_uri: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    status: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    created_at_ms: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Delete a publication media asset by `publication_id + asset_ref`.
    pub async fn delete_publication_media_asset(
        &self,
        publication_id: &str,
        asset_ref: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "DELETE FROM publication_media_assets WHERE publication_id = ?1 AND asset_ref = ?2",
                (publication_id.to_string(), asset_ref.to_string()),
            )
            .await?;
        Ok(())
    }

    /// List scholarly submissions for one publication.
    pub async fn list_scholarly_submissions(
        &self,
        publication_id: &str,
    ) -> Result<Vec<ScholarlySubmissionRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, external_submission_id, status, submitted_at_ms, updated_at_ms, response_fingerprint, metadata_json FROM scholarly_submissions WHERE publication_id = ?1 ORDER BY id DESC",
                (publication_id.to_string(),),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ScholarlySubmissionRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    external_submission_id: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    status: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    submitted_at_ms: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    response_fingerprint: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// List publication attempt rows for one publication (newest first: `ORDER BY id DESC`).
    pub async fn list_publication_attempts(
        &self,
        publication_id: &str,
    ) -> Result<Vec<PublicationAttemptRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, channel, attempted_at_ms, outcome_json
                 FROM publication_attempts
                 WHERE publication_id = ?1
                 ORDER BY id DESC",
                (publication_id.to_string(),),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(PublicationAttemptRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    channel: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    attempted_at_ms: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    outcome_json: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// List immutable publication status events for one publication.
    pub async fn list_publication_status_events(
        &self,
        publication_id: &str,
    ) -> Result<Vec<PublicationStatusEventRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, status, detail_json, recorded_at_ms
                 FROM publication_status_events
                 WHERE publication_id = ?1
                 ORDER BY id DESC",
                (publication_id.to_string(),),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(PublicationStatusEventRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    status: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    detail_json: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    recorded_at_ms: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }
}
