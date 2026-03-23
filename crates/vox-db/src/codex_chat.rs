//! Codex **user chat**, **tool calls**, **usage counters**, and **topics** (manifest slices `v11`–`v14`).
//!
//! Callers must use a store opened through [`crate::VoxDb::connect`] so the baseline DDL has been applied.

use turso::params;

use crate::VoxDb;
use crate::arca_store::StoreError;

impl VoxDb {
    /// Insert a `conversations` row (V11+). Returns SQLite `rowid` / `id`.
    pub async fn chat_create_conversation(
        &self,
        user_id: Option<&str>,
        title: &str,
    ) -> Result<i64, StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT INTO conversations (user_id, title) VALUES (?1, ?2)",
                params![user_id, title],
            )
            .await?;
        Ok(self.store().connection().last_insert_rowid())
    }

    /// Bump `conversations.updated_at` for listing recency (V11+).
    pub async fn chat_touch_conversation(&self, conversation_id: i64) -> Result<(), StoreError> {
        self.store()
            .connection()
            .execute(
                "UPDATE conversations SET updated_at = datetime('now') WHERE id = ?1",
                params![conversation_id],
            )
            .await?;
        Ok(())
    }

    /// Append a `conversation_messages` row (V11+). Returns message `id`.
    pub async fn chat_append_message(
        &self,
        conversation_id: i64,
        role: &str,
        content_text: &str,
        payload_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT INTO conversation_messages (conversation_id, role, content_text, payload_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![conversation_id, role, content_text, payload_json],
            )
            .await?;
        let id = self.store().connection().last_insert_rowid();
        self.chat_touch_conversation(conversation_id).await?;
        Ok(id)
    }

    /// Record a tool invocation for an assistant message (V12+). Returns tool-call row `id`.
    pub async fn chat_insert_tool_call(
        &self,
        conversation_message_id: i64,
        ordinal: i32,
        tool_name: &str,
        arguments_json: &str,
        status: &str,
    ) -> Result<i64, StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        self.store()
            .connection()
            .execute(
                "INSERT INTO conversation_tool_calls
                    (conversation_message_id, ordinal, tool_name, arguments_json, status, started_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    conversation_message_id,
                    ordinal,
                    tool_name,
                    arguments_json,
                    status,
                    now
                ],
            )
            .await?;
        Ok(self.store().connection().last_insert_rowid())
    }

    /// Update result / terminal state for a tool call (V12+).
    pub async fn chat_finish_tool_call(
        &self,
        tool_call_id: i64,
        status: &str,
        result_json: Option<&str>,
        error_text: Option<&str>,
    ) -> Result<(), StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        self.store()
            .connection()
            .execute(
                "UPDATE conversation_tool_calls
                 SET status = ?2, result_json = ?3, error_text = ?4, finished_at_ms = ?5
                 WHERE id = ?1",
                params![tool_call_id, status, result_json, error_text, now],
            )
            .await?;
        Ok(())
    }

    /// Upsert a usage limit policy row (V13+).
    pub async fn chat_upsert_usage_limit(
        &self,
        metric_key: &str,
        scope_kind: &str,
        scope_id: &str,
        period_kind: &str,
        limit_value: i64,
        enforcement: &str,
    ) -> Result<(), StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT INTO usage_limit_definitions
                    (metric_key, scope_kind, scope_id, period_kind, limit_value, enforcement, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, datetime('now'))
                 ON CONFLICT(metric_key, scope_kind, scope_id, period_kind) DO UPDATE SET
                    limit_value = excluded.limit_value,
                    enforcement = excluded.enforcement,
                    updated_at = datetime('now')",
                params![
                    metric_key,
                    scope_kind,
                    scope_id,
                    period_kind,
                    limit_value,
                    enforcement
                ],
            )
            .await?;
        Ok(())
    }

    /// Add `delta` to a usage counter for the given window (V13+). Returns the new total `amount`.
    pub async fn chat_add_usage_amount(
        &self,
        metric_key: &str,
        scope_kind: &str,
        scope_id: &str,
        period_start: &str,
        delta: i64,
    ) -> Result<i64, StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT INTO usage_counter_snapshots
                    (metric_key, scope_kind, scope_id, period_start, amount, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
                 ON CONFLICT(metric_key, scope_kind, scope_id, period_start) DO UPDATE SET
                    amount = usage_counter_snapshots.amount + excluded.amount,
                    updated_at = datetime('now')",
                params![metric_key, scope_kind, scope_id, period_start, delta],
            )
            .await?;
        let mut rows = self
            .store()
            .connection()
            .query(
                "SELECT amount FROM usage_counter_snapshots
                 WHERE metric_key = ?1 AND scope_kind = ?2 AND scope_id = ?3 AND period_start = ?4",
                params![metric_key, scope_kind, scope_id, period_start],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("usage_counter_snapshots readback".into()))?;
        let amount: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(amount)
    }

    /// Current counted usage for a window, or `0` if missing (V13+).
    pub async fn chat_usage_amount(
        &self,
        metric_key: &str,
        scope_kind: &str,
        scope_id: &str,
        period_start: &str,
    ) -> Result<i64, StoreError> {
        let mut rows = self
            .store()
            .connection()
            .query(
                "SELECT COALESCE(
                    (SELECT amount FROM usage_counter_snapshots
                     WHERE metric_key = ?1 AND scope_kind = ?2 AND scope_id = ?3 AND period_start = ?4),
                    0)",
                params![metric_key, scope_kind, scope_id, period_start],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("usage amount".into()))?;
        let v: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(v)
    }

    /// Resolved limit for an exact scope match, if defined (V13+).
    pub async fn chat_usage_limit_value(
        &self,
        metric_key: &str,
        scope_kind: &str,
        scope_id: &str,
        period_kind: &str,
    ) -> Result<Option<i64>, StoreError> {
        let mut rows = self
            .store()
            .connection()
            .query(
                "SELECT limit_value FROM usage_limit_definitions
                 WHERE metric_key = ?1 AND scope_kind = ?2 AND scope_id = ?3 AND period_kind = ?4
                 LIMIT 1",
                params![metric_key, scope_kind, scope_id, period_kind],
            )
            .await?;
        let row = match rows.next().await? {
            Some(r) => r,
            None => return Ok(None),
        };
        let v: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(Some(v))
    }

    /// `INSERT OR IGNORE` then return `topics.id` for `slug` (V14+).
    pub async fn chat_ensure_topic(&self, slug: &str, label: &str) -> Result<i64, StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT OR IGNORE INTO topics (slug, label) VALUES (?1, ?2)",
                params![slug, label],
            )
            .await?;
        let mut rows = self
            .store()
            .connection()
            .query(
                "SELECT id FROM topics WHERE slug = ?1 LIMIT 1",
                params![slug],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("topics slug missing after insert".into()))?;
        let id: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(id)
    }

    /// Link a conversation to a topic with optional weight (V14+).
    pub async fn chat_link_conversation_topic(
        &self,
        conversation_id: i64,
        topic_id: i64,
        weight: f64,
    ) -> Result<(), StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT INTO conversation_topics (conversation_id, topic_id, weight)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(conversation_id, topic_id) DO UPDATE SET weight = excluded.weight",
                params![conversation_id, topic_id, weight],
            )
            .await?;
        Ok(())
    }

    /// Link a single message to a topic (V14+).
    pub async fn chat_link_message_topic(
        &self,
        conversation_message_id: i64,
        topic_id: i64,
    ) -> Result<(), StoreError> {
        self.store()
            .connection()
            .execute(
                "INSERT OR IGNORE INTO conversation_message_topics (conversation_message_id, topic_id)
                 VALUES (?1, ?2)",
                params![conversation_message_id, topic_id],
            )
            .await?;
        Ok(())
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use crate::{DbConfig, VoxDb};

    #[tokio::test]
    async fn chat_tool_usage_topic_round_trip() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        assert_eq!(
            db.schema_version().await.expect("v"),
            crate::schema::BASELINE_VERSION
        );

        db.store()
            .connection()
            .execute(
                "INSERT OR IGNORE INTO users (id, display_name, role) VALUES ('u1', 'u1', 'user')",
                (),
            )
            .await
            .expect("seed user");

        let conv = db
            .chat_create_conversation(Some("u1"), "hi")
            .await
            .expect("conv");
        let msg = db
            .chat_append_message(conv, "assistant", "calling tool", None)
            .await
            .expect("msg");
        let tc = db
            .chat_insert_tool_call(msg, 0, "search", "{}", "running")
            .await
            .expect("tc");
        db.chat_finish_tool_call(tc, "succeeded", Some("{\"ok\":true}"), None)
            .await
            .expect("fin");

        db.chat_upsert_usage_limit("tokens", "user", "u1", "daily", 1000, "hard")
            .await
            .expect("lim");
        let amt = db
            .chat_add_usage_amount("tokens", "user", "u1", "2026-03-21", 42)
            .await
            .expect("add");
        assert_eq!(amt, 42);
        let lim = db
            .chat_usage_limit_value("tokens", "user", "u1", "daily")
            .await
            .expect("q");
        assert_eq!(lim, Some(1000));

        let tid = db.chat_ensure_topic("rust", "Rust").await.expect("topic");
        db.chat_link_conversation_topic(conv, tid, 1.0)
            .await
            .expect("ct");
        db.chat_link_message_topic(msg, tid).await.expect("mt");
    }
}
