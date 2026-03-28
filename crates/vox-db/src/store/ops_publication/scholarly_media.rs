use super::common::now_ms;
use crate::{VoxDb, store::StoreError, store::types::*};

impl VoxDb {
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
        let rows = self
            .query_all(
                "SELECT publication_id, content_sha3_256 FROM scholarly_submissions
                 WHERE adapter = ?1 AND external_submission_id = ?2",
                (adapter.to_string(), external_submission_id.to_string()),
            )
            .await?;
        if let Some(r) = rows.first() {
            let ex_pub: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let ex_dig: String = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            if ex_pub != publication_id || ex_dig != content_sha3_256 {
                return Err(StoreError::UpsertIdentityMismatch(format!(
                    "scholarly_submissions (adapter={adapter}, external_submission_id={external_submission_id}) \
                     is bound to publication_id={ex_pub} digest={ex_dig}; refused publication_id={publication_id} digest={content_sha3_256}"
                )));
            }
        }
        let ts = now_ms();
        let pub_id = publication_id.to_string();
        let digest = content_sha3_256.to_string();
        let adapter_s = adapter.to_string();
        let ext_id = external_submission_id.to_string();
        let status_s = status.to_string();
        let rf = response_fingerprint.map(std::string::ToString::to_string);
        let mj = metadata_json.map(std::string::ToString::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO scholarly_submissions (
                    publication_id, content_sha3_256, adapter, external_submission_id, status,
                    submitted_at_ms, updated_at_ms, response_fingerprint, metadata_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, ?7, ?8)
                ON CONFLICT(adapter, external_submission_id) DO UPDATE SET
                    status = excluded.status,
                    updated_at_ms = excluded.updated_at_ms,
                    response_fingerprint = excluded.response_fingerprint,
                    metadata_json = excluded.metadata_json",
                    (pub_id, digest, adapter_s, ext_id, status_s, ts, rf, mj),
                )
                .await?;
                Ok::<(), StoreError>(())
            })
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
        let status_s = status.to_string();
        let meta = metadata_json.map(std::string::ToString::to_string);
        let pub_id = publication_id.to_string();
        let adapter_s = adapter.to_string();
        let ext_id = external_submission_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let n = conn
                    .execute(
                        "UPDATE scholarly_submissions SET
                    status = ?1,
                    updated_at_ms = ?2,
                    metadata_json = COALESCE(?3, metadata_json)
                 WHERE publication_id = ?4 AND adapter = ?5 AND external_submission_id = ?6",
                        (status_s, ts, meta, pub_id, adapter_s, ext_id),
                    )
                    .await?;
                Ok::<_, StoreError>(n)
            })
            .await
    }

    /// Insert or update one publication media asset row.
    pub async fn upsert_publication_media_asset(
        &self,
        params: PublicationMediaAssetParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        let publication_id = params.publication_id.to_string();
        let asset_ref = params.asset_ref.to_string();
        let media_type = params.media_type.to_string();
        let storage_uri = params.storage_uri.map(std::string::ToString::to_string);
        let status = params.status.to_string();
        let metadata_json = params.metadata_json.map(std::string::ToString::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
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
                        publication_id,
                        asset_ref,
                        media_type,
                        storage_uri,
                        status,
                        metadata_json,
                        ts,
                    ),
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
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
        let publication_id = publication_id.to_string();
        let asset_ref = asset_ref.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "DELETE FROM publication_media_assets WHERE publication_id = ?1 AND asset_ref = ?2",
                    (publication_id, asset_ref),
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
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
        let lim = limit.clamp(1, 500);
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
}
