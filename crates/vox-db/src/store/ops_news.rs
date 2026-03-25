use crate::{VoxDb, store::StoreError};

impl VoxDb {
    /// Mark a news item as published. `github_release_id`, `twitter_tweet_id`, and
    /// `opencollective_update_id` align with `published_news` column names (GitHub URL or id string).
    pub async fn mark_news_published(
        &self,
        id: &str,
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
                "INSERT OR REPLACE INTO published_news (id, published_at_ms, github_release_id, twitter_tweet_id, opencollective_update_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                (
                    id.to_string(),
                    now,
                    github_release_id,
                    twitter_tweet_id,
                    opencollective_update_id,
                ),
            )
            .await?;
        Ok(())
    }

    pub async fn is_news_published(&self, id: &str) -> Result<bool, StoreError> {
        let rows = self
            .query_all(
                "SELECT 1 FROM published_news WHERE id = ?1",
                (id.to_string(),),
            )
            .await?;
        Ok(!rows.is_empty())
    }

    /// Record a single approver for a news item id (idempotent per `(news_id, approver)`).
    pub async fn record_news_approval(&self, news_id: &str, approver: &str) -> Result<(), StoreError> {
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
        let v: i64 = row
            .get(0)
            .map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(v)
    }

    /// True when at least two **distinct** approver strings exist for `news_id`.
    pub async fn has_dual_news_approval(&self, news_id: &str) -> Result<bool, StoreError> {
        Ok(self.count_news_approvers(news_id).await? >= 2)
    }
}
