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

    /// Append one `publication_status_events` row without updating `publication_manifests.state`.
    pub async fn append_publication_status_event(
        &self,
        publication_id: &str,
        status: &str,
        detail_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_status_events (publication_id, status, detail_json, recorded_at_ms) VALUES (?1, ?2, ?3, ?4)",
                (
                    publication_id.to_string(),
                    status.to_string(),
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

    /// Update `scholarly_submissions.status` (and timestamps) for remote polling — does **not** change
    /// `publication_manifests.state`.
    pub async fn patch_scholarly_submission_status(
        &self,
        publication_id: &str,
        adapter: &str,
        external_submission_id: &str,
        status: &str,
        metadata_json: Option<&str>,
    ) -> Result<u64, StoreError> {
        let ts = now_ms();
        let n = self
            .conn
            .execute(
                "UPDATE scholarly_submissions SET
                    status = ?1,
                    updated_at_ms = ?2,
                    metadata_json = COALESCE(?3, metadata_json)
                 WHERE publication_id = ?4 AND adapter = ?5 AND external_submission_id = ?6",
                (
                    status.to_string(),
                    ts,
                    metadata_json.map(std::string::ToString::to_string),
                    publication_id.to_string(),
                    adapter.to_string(),
                    external_submission_id.to_string(),
                ),
            )
            .await?;
        Ok(n)
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

    /// Distinct publications with at least one `scholarly_submissions` row — most recently updated submission first.
    pub async fn list_publication_ids_with_scholarly_submissions(
        &self,
        limit: i64,
    ) -> Result<Vec<String>, StoreError> {
        let lim = limit.max(1).min(500);
        let rows = self
            .query_all(
                "SELECT publication_id FROM scholarly_submissions
                 GROUP BY publication_id
                 ORDER BY MAX(updated_at_ms) DESC
                 LIMIT ?1",
                (lim,),
            )
            .await?;
        rows.into_iter()
            .map(|r| r.get(0).map_err(|e| StoreError::Db(e.to_string())))
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

    /// Insert or update an external submission job row keyed by [`ExternalSubmissionJobUpsertParams::idempotency_key`].
    pub async fn upsert_external_submission_job(
        &self,
        p: ExternalSubmissionJobUpsertParams<'_>,
    ) -> Result<i64, StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO external_submission_jobs (
                    publication_id, content_sha3_256, adapter, operation, idempotency_key,
                    status, lock_owner, lock_expires_at_ms, next_retry_at_ms, attempt_count,
                    last_error_class, last_error_message, metadata_json, created_at_ms, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14)
                ON CONFLICT(idempotency_key) DO UPDATE SET
                    status = excluded.status,
                    lock_owner = excluded.lock_owner,
                    lock_expires_at_ms = excluded.lock_expires_at_ms,
                    next_retry_at_ms = excluded.next_retry_at_ms,
                    last_error_class = excluded.last_error_class,
                    last_error_message = excluded.last_error_message,
                    metadata_json = excluded.metadata_json,
                    updated_at_ms = excluded.updated_at_ms",
                (
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.adapter.to_string(),
                    p.operation.to_string(),
                    p.idempotency_key.to_string(),
                    p.status.to_string(),
                    p.lock_owner.map(std::string::ToString::to_string),
                    p.lock_expires_at_ms,
                    p.next_retry_at_ms,
                    p.attempt_count,
                    p.last_error_class.map(std::string::ToString::to_string),
                    p.last_error_message.map(std::string::ToString::to_string),
                    p.metadata_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        let rows = self
            .query_all(
                "SELECT id FROM external_submission_jobs WHERE idempotency_key = ?1",
                (p.idempotency_key.to_string(),),
            )
            .await?;
        let row = rows
            .first()
            .ok_or_else(|| StoreError::Db("external_submission_jobs id lookup missing row".into()))?;
        row.get(0).map_err(|e| StoreError::Db(e.to_string()))
    }

    /// Fetch one external submission job by idempotency key.
    pub async fn get_external_submission_job_by_idempotency_key(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<ExternalSubmissionJobRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, operation, idempotency_key,
                        status, lock_owner, lock_expires_at_ms, next_retry_at_ms, attempt_count,
                        last_error_class, last_error_message, metadata_json, created_at_ms, updated_at_ms
                 FROM external_submission_jobs WHERE idempotency_key = ?1",
                (idempotency_key.to_string(),),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(ExternalSubmissionJobRow {
            id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            operation: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            idempotency_key: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            status: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            lock_owner: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            lock_expires_at_ms: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
            next_retry_at_ms: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
            attempt_count: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
            last_error_class: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
            last_error_message: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
            metadata_json: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
            created_at_ms: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
            updated_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Fetch one external submission job by primary key.
    pub async fn get_external_submission_job_by_id(
        &self,
        job_id: i64,
    ) -> Result<Option<ExternalSubmissionJobRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, operation, idempotency_key,
                        status, lock_owner, lock_expires_at_ms, next_retry_at_ms, attempt_count,
                        last_error_class, last_error_message, metadata_json, created_at_ms, updated_at_ms
                 FROM external_submission_jobs WHERE id = ?1",
                (job_id,),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(ExternalSubmissionJobRow {
            id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            operation: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            idempotency_key: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            status: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            lock_owner: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            lock_expires_at_ms: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
            next_retry_at_ms: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
            attempt_count: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
            last_error_class: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
            last_error_message: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
            metadata_json: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
            created_at_ms: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
            updated_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Jobs ready for worker processing: due `queued` / `retryable_failed`, or stale `running` with expired lock.
    pub async fn list_external_submission_jobs_due(
        &self,
        before_ms_inclusive: i64,
        limit: i64,
    ) -> Result<Vec<ExternalSubmissionJobRow>, StoreError> {
        let lim = limit.max(1).min(500);
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, operation, idempotency_key,
                        status, lock_owner, lock_expires_at_ms, next_retry_at_ms, attempt_count,
                        last_error_class, last_error_message, metadata_json, created_at_ms, updated_at_ms
                 FROM external_submission_jobs
                 WHERE (
                    (status IN ('queued', 'retryable_failed')
                     AND (next_retry_at_ms IS NULL OR next_retry_at_ms <= ?1))
                    OR (status = 'running'
                        AND lock_expires_at_ms IS NOT NULL
                        AND lock_expires_at_ms < ?1)
                 )
                 ORDER BY COALESCE(next_retry_at_ms, 0) ASC, id ASC
                 LIMIT ?2",
                (before_ms_inclusive, lim),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ExternalSubmissionJobRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    operation: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    idempotency_key: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    status: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    lock_owner: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    lock_expires_at_ms: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                    next_retry_at_ms: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                    attempt_count: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_error_class: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_error_message: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
                    created_at_ms: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Permanently failed scholarly outbound jobs (`status = failed`), newest first (dead-letter review).
    pub async fn list_external_submission_jobs_failed(
        &self,
        limit: i64,
    ) -> Result<Vec<ExternalSubmissionJobRow>, StoreError> {
        let lim = limit.max(1).min(500);
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, operation, idempotency_key,
                        status, lock_owner, lock_expires_at_ms, next_retry_at_ms, attempt_count,
                        last_error_class, last_error_message, metadata_json, created_at_ms, updated_at_ms
                 FROM external_submission_jobs
                 WHERE status = 'failed'
                 ORDER BY updated_at_ms DESC, id DESC
                 LIMIT ?1",
                (lim,),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ExternalSubmissionJobRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    operation: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    idempotency_key: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    status: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    lock_owner: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    lock_expires_at_ms: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                    next_retry_at_ms: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                    attempt_count: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_error_class: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_error_message: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
                    created_at_ms: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Operator replay: move one **terminal** `failed` job back to `queued` (clears lease, retry schedule, last error).
    ///
    /// Fails if the row is missing or `status` is not `failed`. `attempt_count` is preserved for audit.
    pub async fn replay_failed_external_submission_job_to_queued(
        &self,
        job_id: i64,
    ) -> Result<ExternalSubmissionJobRow, StoreError> {
        let ts = now_ms();
        let n = self
            .conn
            .execute(
                "UPDATE external_submission_jobs SET
                    status = 'queued',
                    lock_owner = NULL,
                    lock_expires_at_ms = NULL,
                    next_retry_at_ms = NULL,
                    last_error_class = NULL,
                    last_error_message = NULL,
                    updated_at_ms = ?1
                 WHERE id = ?2 AND status = 'failed'",
                (ts, job_id),
            )
            .await?;
        if n == 0 {
            let Some(j) = self.get_external_submission_job_by_id(job_id).await? else {
                return Err(StoreError::NotFound(format!(
                    "external_submission_jobs id={job_id}"
                )));
            };
            return Err(StoreError::Db(format!(
                "replay only allowed when status is 'failed' (job {job_id} is {})",
                j.status
            )));
        }
        self.get_external_submission_job_by_id(job_id)
            .await?
            .ok_or_else(|| {
                StoreError::Db(format!(
                    "external_submission_jobs id={job_id} missing after replay update"
                ))
            })
    }

    /// Try to move a due job to `running` with a lease. Returns `true` if this worker won the race.
    pub async fn try_claim_external_submission_job(
        &self,
        job_id: i64,
        lock_owner: &str,
        lock_expires_at_ms: i64,
        now_ms_inclusive: i64,
    ) -> Result<bool, StoreError> {
        let ts = now_ms();
        let n = self
            .conn
            .execute(
                "UPDATE external_submission_jobs SET
                    status = 'running',
                    lock_owner = ?1,
                    lock_expires_at_ms = ?2,
                    updated_at_ms = ?3
                 WHERE id = ?4
                   AND (
                     (status IN ('queued', 'retryable_failed')
                      AND (next_retry_at_ms IS NULL OR next_retry_at_ms <= ?5))
                     OR (status = 'running'
                         AND lock_expires_at_ms IS NOT NULL
                         AND lock_expires_at_ms < ?5)
                   )",
                (
                    lock_owner.to_string(),
                    lock_expires_at_ms,
                    ts,
                    job_id,
                    now_ms_inclusive,
                ),
            )
            .await?;
        Ok(n > 0)
    }

    /// List external submission jobs for one publication id and content digest.
    pub async fn list_external_submission_jobs_for_publication_digest(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
    ) -> Result<Vec<ExternalSubmissionJobRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, operation, idempotency_key,
                        status, lock_owner, lock_expires_at_ms, next_retry_at_ms, attempt_count,
                        last_error_class, last_error_message, metadata_json, created_at_ms, updated_at_ms
                 FROM external_submission_jobs
                 WHERE publication_id = ?1 AND content_sha3_256 = ?2
                 ORDER BY id DESC",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                ),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(ExternalSubmissionJobRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    operation: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    idempotency_key: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    status: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    lock_owner: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    lock_expires_at_ms: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                    next_retry_at_ms: r.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                    attempt_count: r.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_error_class: r.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                    last_error_message: r.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
                    created_at_ms: r.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
                    updated_at_ms: r.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Append one adapter HTTP attempt row and bump `attempt_count` on the parent job.
    pub async fn record_external_submission_attempt(
        &self,
        p: ExternalSubmissionAttemptParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        let retryable_i: i64 = if p.retryable { 1 } else { 0 };
        self.conn
            .execute(
                "INSERT INTO external_submission_attempts (
                    job_id, attempted_at_ms, http_status, error_class, retryable,
                    request_fingerprint, response_fingerprint, detail_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    p.job_id,
                    ts,
                    p.http_status.map(i64::from),
                    p.error_class.map(std::string::ToString::to_string),
                    retryable_i,
                    p.request_fingerprint.map(std::string::ToString::to_string),
                    p.response_fingerprint.map(std::string::ToString::to_string),
                    p.detail_json.map(std::string::ToString::to_string),
                ),
            )
            .await?;
        self.conn
            .execute(
                "UPDATE external_submission_jobs SET attempt_count = attempt_count + 1, updated_at_ms = ?2 WHERE id = ?1",
                (p.job_id, ts),
            )
            .await?;
        Ok(())
    }

    /// List attempts for a job, oldest first.
    pub async fn list_external_submission_attempts_for_job(
        &self,
        job_id: i64,
    ) -> Result<Vec<ExternalSubmissionAttemptRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, job_id, attempted_at_ms, http_status, error_class, retryable,
                        request_fingerprint, response_fingerprint, detail_json
                 FROM external_submission_attempts WHERE job_id = ?1 ORDER BY id ASC",
                (job_id,),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                let retry_i: i64 = r.get(5).map_err(|e| StoreError::Db(e.to_string()))?;
                Ok(ExternalSubmissionAttemptRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    job_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    attempted_at_ms: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    http_status: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    error_class: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    retryable: retry_i != 0,
                    request_fingerprint: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    response_fingerprint: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                    detail_json: r.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Append one remote status snapshot.
    pub async fn insert_external_status_snapshot(
        &self,
        p: ExternalStatusSnapshotParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO external_status_snapshots (
                    adapter, external_submission_id, publication_id, content_sha3_256, snapshot_json, fetched_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    p.adapter.to_string(),
                    p.external_submission_id.to_string(),
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.snapshot_json.to_string(),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// Latest remote snapshot for adapter + external id.
    pub async fn get_latest_external_status_snapshot(
        &self,
        adapter: &str,
        external_submission_id: &str,
    ) -> Result<Option<ExternalStatusSnapshotRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, adapter, external_submission_id, publication_id, content_sha3_256, snapshot_json, fetched_at_ms
                 FROM external_status_snapshots
                 WHERE adapter = ?1 AND external_submission_id = ?2
                 ORDER BY fetched_at_ms DESC
                 LIMIT 1",
                (adapter.to_string(), external_submission_id.to_string()),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(ExternalStatusSnapshotRow {
            id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            adapter: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            external_submission_id: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            publication_id: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            snapshot_json: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            fetched_at_ms: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Upsert a publication external link (DOI, URL, deposition id, note id, ...).
    pub async fn upsert_publication_external_link(
        &self,
        p: PublicationExternalLinkUpsertParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_external_links (
                    publication_id, content_sha3_256, adapter, link_kind, link_value, metadata_json, created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(publication_id, content_sha3_256, adapter, link_kind) DO UPDATE SET
                    link_value = excluded.link_value,
                    metadata_json = excluded.metadata_json,
                    created_at_ms = excluded.created_at_ms",
                (
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.adapter.to_string(),
                    p.link_kind.to_string(),
                    p.link_value.to_string(),
                    p.metadata_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// List external links for publication id + digest.
    pub async fn list_publication_external_links(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
    ) -> Result<Vec<PublicationExternalLinkRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, link_kind, link_value, metadata_json, created_at_ms
                 FROM publication_external_links
                 WHERE publication_id = ?1 AND content_sha3_256 = ?2
                 ORDER BY id ASC",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                ),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(PublicationExternalLinkRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    link_kind: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    link_value: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    created_at_ms: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Upsert the adapter revision pointer for a publication + content digest.
    pub async fn upsert_publication_external_revision(
        &self,
        p: PublicationExternalRevisionUpsertParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_external_revisions (
                    publication_id, content_sha3_256, adapter, external_revision, metadata_json, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(publication_id, content_sha3_256, adapter) DO UPDATE SET
                    external_revision = excluded.external_revision,
                    metadata_json = excluded.metadata_json,
                    updated_at_ms = excluded.updated_at_ms",
                (
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.adapter.to_string(),
                    p.external_revision.to_string(),
                    p.metadata_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// Load revision pointer for publication id + digest + adapter, if present.
    pub async fn get_publication_external_revision(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
        adapter: &str,
    ) -> Result<Option<PublicationExternalRevisionRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, external_revision, metadata_json, updated_at_ms
                 FROM publication_external_revisions
                 WHERE publication_id = ?1 AND content_sha3_256 = ?2 AND adapter = ?3",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                    adapter.to_string(),
                ),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(PublicationExternalRevisionRow {
            id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            external_revision: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            metadata_json: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            updated_at_ms: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Read-only rollup over scholarly external tables: job queue snapshot, attempts and snapshots in a time window,
    /// approximate terminal latencies from job row timestamps, and multi-channel `publication_attempts` counts.
    pub async fn summarize_scholarly_external_pipeline_metrics(
        &self,
        since_ms: i64,
    ) -> Result<serde_json::Value, StoreError> {
        let generated_at_ms = now_ms();
        let jobs_by_status_rows = self
            .query_all(
                "SELECT status, COUNT(*) FROM external_submission_jobs GROUP BY status ORDER BY status",
                (),
            )
            .await?;
        let mut jobs_by_status = serde_json::Map::new();
        for r in jobs_by_status_rows {
            let k: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            jobs_by_status.insert(k, serde_json::json!(n));
        }

        let adapter_status_rows = self
            .query_all(
                "SELECT adapter, status, COUNT(*) FROM external_submission_jobs GROUP BY adapter, status ORDER BY adapter, status",
                (),
            )
            .await?;
        let mut by_adapter_status = Vec::new();
        for r in adapter_status_rows {
            by_adapter_status.push(serde_json::json!({
                "adapter": r.get::<String>(0).map_err(|e| StoreError::Db(e.to_string()))?,
                "status": r.get::<String>(1).map_err(|e| StoreError::Db(e.to_string()))?,
                "count": r.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }));
        }

        let lat_ok_row = self
            .query_all(
                "SELECT AVG(updated_at_ms - created_at_ms) FROM external_submission_jobs WHERE status = 'succeeded' AND updated_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let avg_latency_succeeded_ms: Option<f64> = lat_ok_row
            .first()
            .and_then(|r| r.get::<Option<f64>>(0).ok().flatten());

        let lat_fail_row = self
            .query_all(
                "SELECT AVG(updated_at_ms - created_at_ms) FROM external_submission_jobs WHERE status = 'failed' AND updated_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let avg_latency_failed_ms: Option<f64> = lat_fail_row
            .first()
            .and_then(|r| r.get::<Option<f64>>(0).ok().flatten());

        let attempt_total_rows = self
            .query_all(
                "SELECT COUNT(*), COALESCE(SUM(retryable), 0) FROM external_submission_attempts WHERE attempted_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let (attempts_total, attempts_retryable) = if let Some(r) = attempt_total_rows.first() {
            let c: i64 = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let s: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            (c, s)
        } else {
            (0_i64, 0_i64)
        };

        let err_class_rows = self
            .query_all(
                "SELECT COALESCE(error_class, ''), COUNT(*) FROM external_submission_attempts WHERE attempted_at_ms >= ?1 GROUP BY error_class ORDER BY COUNT(*) DESC",
                (since_ms,),
            )
            .await?;
        let mut by_error_class = serde_json::Map::new();
        for r in err_class_rows {
            let k: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let key = if k.is_empty() { "(null)".to_string() } else { k };
            by_error_class.insert(key, serde_json::json!(n));
        }

        let snap_rows = self
            .query_all(
                "SELECT COUNT(*) FROM external_status_snapshots WHERE fetched_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let snapshots_since: i64 = snap_rows
            .first()
            .map(|r| r.get(0).map_err(|e| StoreError::Db(e.to_string())))
            .transpose()?
            .unwrap_or(0);

        let sub_rows = self
            .query_all(
                "SELECT adapter, status, COUNT(*) FROM scholarly_submissions GROUP BY adapter, status ORDER BY adapter, status",
                (),
            )
            .await?;
        let mut scholarly_by_adapter_status = Vec::new();
        for r in sub_rows {
            scholarly_by_adapter_status.push(serde_json::json!({
                "adapter": r.get::<String>(0).map_err(|e| StoreError::Db(e.to_string()))?,
                "status": r.get::<String>(1).map_err(|e| StoreError::Db(e.to_string()))?,
                "count": r.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }));
        }

        let pub_attempt_rows = self
            .query_all(
                "SELECT channel, COUNT(*) FROM publication_attempts WHERE attempted_at_ms >= ?1 GROUP BY channel ORDER BY COUNT(*) DESC",
                (since_ms,),
            )
            .await?;
        let mut publication_attempts_by_channel = serde_json::Map::new();
        for r in pub_attempt_rows {
            let ch: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            publication_attempts_by_channel.insert(ch, serde_json::json!(n));
        }

        Ok(serde_json::json!({
            "generated_at_ms": generated_at_ms,
            "since_ms": since_ms,
            "external_submission_jobs": {
                "by_status": jobs_by_status,
                "by_adapter_status": by_adapter_status,
                "avg_terminal_latency_ms_in_window": {
                    "succeeded": avg_latency_succeeded_ms,
                    "failed": avg_latency_failed_ms,
                },
            },
            "external_submission_attempts": {
                "total_in_window": attempts_total,
                "retryable_in_window": attempts_retryable,
                "by_error_class": by_error_class,
            },
            "external_status_snapshots_in_window": snapshots_since,
            "scholarly_submissions_by_adapter_status": scholarly_by_adapter_status,
            "publication_attempts_in_window_by_channel": publication_attempts_by_channel,
        }))
    }
}
