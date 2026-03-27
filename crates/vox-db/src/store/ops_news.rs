use crate::{VoxDb, store::StoreError};

impl VoxDb {
    /// Mark a news item as published. `github_release_id`, `twitter_tweet_id`, and
    /// `opencollective_update_id` align with `published_news` column names (GitHub URL or id string).
    pub async fn mark_news_published(
        &self,
        id: &str,
        content_sha3_256: &str,
        github_release_id: Option<&str>,
        twitter_tweet_id: Option<&str>,
        opencollective_update_id: Option<&str>,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO published_news (news_id, published_at_ms, github_release_id, twitter_tweet_id, opencollective_update_id, content_sha3_256) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    id.to_string(),
                    now,
                    github_release_id,
                    twitter_tweet_id,
                    opencollective_update_id,
                    content_sha3_256.to_string(),
                ),
            )
            .await?;
        Ok(())
    }

    /// `true` when this news id was marked published **for this exact content digest** (or legacy row with unknown digest — see below).
    ///
    /// Legacy rows may have `content_sha3_256` NULL after migration; those are treated as published
    /// for any digest until operators backfill or re-publish (avoids mass duplicate syndication on upgrade).
    pub async fn is_news_published_for_content(
        &self,
        id: &str,
        content_sha3_256: &str,
    ) -> Result<bool, StoreError> {
        let rows = self
            .query_all(
                "SELECT content_sha3_256 FROM published_news WHERE news_id = ?1",
                (id.to_string(),),
            )
            .await?;
        let Some(row) = rows.first() else {
            return Ok(false);
        };
        let stored: Option<String> = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        match stored.as_deref() {
            None | Some("") => Ok(true),
            Some(s) => Ok(s == content_sha3_256),
        }
    }

    /// Record a single approver for a news item id (idempotent per `(news_id, approver)`).
    pub async fn record_news_approval(
        &self,
        news_id: &str,
        approver: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO news_publish_approvals (news_id, approver, approved_at_ms) VALUES (?1, ?2, ?3)",
                (news_id.to_string(), approver.to_string(), now),
            )
            .await?;
        Ok(())
    }

    /// Count distinct approvers recorded for this news id.
    pub async fn count_news_approvers(&self, news_id: &str) -> Result<i64, StoreError> {
        let rows = self
            .query_all(
                "SELECT COUNT(DISTINCT approver) AS c FROM news_publish_approvals WHERE news_id = ?1",
                (news_id.to_string(),),
            )
            .await?;
        let row = rows
            .first()
            .ok_or_else(|| StoreError::Db("approval count: no row".into()))?;
        let v: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(v)
    }

    /// True when at least two **distinct** approver strings exist for `news_id`.
    pub async fn has_dual_news_approval(&self, news_id: &str) -> Result<bool, StoreError> {
        Ok(self.count_news_approvers(news_id).await? >= 2)
    }

    /// Record an approval bound to immutable content digest.
    pub async fn record_news_approval_for_digest(
        &self,
        news_id: &str,
        content_sha3_256: &str,
        approver: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO news_publish_approvals_v2 (news_id, content_sha3_256, approver, approved_at_ms) VALUES (?1, ?2, ?3, ?4)",
                (
                    news_id.to_string(),
                    content_sha3_256.to_string(),
                    approver.to_string(),
                    now,
                ),
            )
            .await?;
        Ok(())
    }

    /// Count distinct approvers recorded for this id+digest pair.
    pub async fn count_news_approvers_for_digest(
        &self,
        news_id: &str,
        content_sha3_256: &str,
    ) -> Result<i64, StoreError> {
        let rows = self
            .query_all(
                "SELECT COUNT(DISTINCT approver) AS c FROM news_publish_approvals_v2 WHERE news_id = ?1 AND content_sha3_256 = ?2",
                (news_id.to_string(), content_sha3_256.to_string()),
            )
            .await?;
        let row = rows
            .first()
            .ok_or_else(|| StoreError::Db("approval count v2: no row".into()))?;
        let v: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(v)
    }

    /// True when at least two distinct approvers exist for id+digest.
    pub async fn has_dual_news_approval_for_digest(
        &self,
        news_id: &str,
        content_sha3_256: &str,
    ) -> Result<bool, StoreError> {
        Ok(self
            .count_news_approvers_for_digest(news_id, content_sha3_256)
            .await?
            >= 2)
    }

    /// Migration-aware approval check: prefer digest-bound approvals, fallback to legacy id-only approvals.
    pub async fn has_dual_news_approval_with_fallback(
        &self,
        news_id: &str,
        content_sha3_256: &str,
    ) -> Result<bool, StoreError> {
        if self
            .has_dual_news_approval_for_digest(news_id, content_sha3_256)
            .await?
        {
            return Ok(true);
        }
        self.has_dual_news_approval(news_id).await
    }

    /// Persist one publish attempt for postmortem and retry-policy inspection.
    pub async fn record_news_publish_attempt(
        &self,
        news_id: &str,
        content_sha3_256: &str,
        result_json: &str,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        self.conn
            .execute(
                "INSERT INTO news_publish_attempts (news_id, content_sha3_256, attempted_at_ms, result_json) VALUES (?1, ?2, ?3, ?4)",
                (
                    news_id.to_string(),
                    content_sha3_256.to_string(),
                    now,
                    result_json.to_string(),
                ),
            )
            .await?;
        Ok(())
    }
}
