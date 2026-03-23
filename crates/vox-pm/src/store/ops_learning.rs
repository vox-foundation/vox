//! Behavioral learning, pattern detection, command analytics, and training data export
//! for [`CodeStore`].
//!
//! Tables covered (all V3 schema):
//! - **`behavior_events`** — raw user/agent action log.
//! - **`learned_patterns`** — inferred habits and preferences with confidence scores.
//! - **`user_preferences`** — key-value preference store.
//! - **`llm_interactions`** + **`llm_feedback`** — read side for RLHF data export.

use turso::params;

use crate::store::CodeStore;
use crate::store::types::{
    BehaviorEventEntry, CommandFrequencyEntry, LearnedPatternEntry, SaveSnippetParams, SnippetEntry,
    StoreError, TrainingPair,
};

impl CodeStore {
    // ── Behavior Events (behavior_events) ────────────────────────────────────

    /// Append a row to `behavior_events`. Returns the inserted `rowid`.
    ///
    /// Called from `vox-db/src/learning.rs` `BehavioralLearner::observe`.
    pub async fn record_behavior_event(
        &self,
        user_id: &str,
        event_type: &str,
        context: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO behavior_events (user_id, event_type, context, metadata)
                 VALUES (?1, ?2, ?3, ?4)",
                params![user_id, event_type, context, metadata],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch `behavior_events` for `user_id`, newest first, optionally filtered by `event_type`.
    ///
    /// Called from `vox-db/src/learning.rs`.
    pub async fn get_behavior_events(
        &self,
        user_id: &str,
        event_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<BehaviorEventEntry>, StoreError> {
        let lim = limit.clamp(1, 50_000);
        let mut rows = match event_type {
            Some(t) => {
                self.conn
                    .query(
                        "SELECT id, user_id, event_type, context, metadata, created_at
                         FROM behavior_events
                         WHERE user_id = ?1 AND event_type = ?2
                         ORDER BY created_at DESC LIMIT ?3",
                        params![user_id, t, lim],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        "SELECT id, user_id, event_type, context, metadata, created_at
                         FROM behavior_events
                         WHERE user_id = ?1
                         ORDER BY created_at DESC LIMIT ?2",
                        params![user_id, lim],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(BehaviorEventEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                user_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                event_type: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                context: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                metadata: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// Aggregate event-type counts for `user_id` — used for frequency-based pattern detection.
    ///
    /// Returns `(event_type, count)` pairs sorted by count descending.
    pub async fn get_behavior_summary(
        &self,
        user_id: &str,
    ) -> Result<Vec<(String, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT event_type, COUNT(*) AS cnt
                 FROM behavior_events WHERE user_id = ?1
                 GROUP BY event_type ORDER BY cnt DESC",
                params![user_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let t: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let c: i64 = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((t, c));
        }
        Ok(out)
    }

    // ── Learned Patterns (learned_patterns) ──────────────────────────────────

    /// Insert a `learned_patterns` row. Returns the inserted `rowid`.
    pub async fn store_learned_pattern(
        &self,
        user_id: &str,
        pattern_type: &str,
        category: &str,
        description: &str,
        confidence: f64,
        vcs_snapshot_id: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO learned_patterns
                     (user_id, pattern_type, category, description, confidence, vcs_snapshot_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    user_id,
                    pattern_type,
                    category,
                    description,
                    confidence,
                    vcs_snapshot_id
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch all `learned_patterns` for `user_id`, sorted by confidence descending.
    pub async fn get_learned_patterns(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<LearnedPatternEntry>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT id, user_id, pattern_type, category, description, confidence, vcs_snapshot_id
                 FROM learned_patterns WHERE user_id = ?1
                 ORDER BY confidence DESC LIMIT ?2",
                params![user_id, lim],
            )
            .await?;
        Self::collect_pattern_rows(&mut rows).await
    }

    /// Fetch patterns for `user_id` filtered by `category`.
    pub async fn get_patterns_by_category(
        &self,
        user_id: &str,
        category: &str,
    ) -> Result<Vec<LearnedPatternEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, user_id, pattern_type, category, description, confidence, vcs_snapshot_id
                 FROM learned_patterns WHERE user_id = ?1 AND category = ?2
                 ORDER BY confidence DESC",
                params![user_id, category],
            )
            .await?;
        Self::collect_pattern_rows(&mut rows).await
    }

    /// Update the `confidence` column for a single `learned_patterns` row by `id`.
    pub async fn update_pattern_confidence(&self, id: i64, confidence: f64) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE learned_patterns SET confidence = ?2 WHERE id = ?1",
                params![id, confidence],
            )
            .await?;
        Ok(())
    }

    // ── Command Analytics ─────────────────────────────────────────────────────

    /// Aggregate CLI command frequency from `behavior_events` where `event_type = 'command'`
    /// and `context` holds the command name.
    ///
    /// Called from `vox-db/src/learning.rs` `BehavioralLearner::frequency_analysis`.
    pub async fn get_command_frequency(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<CommandFrequencyEntry>, StoreError> {
        let lim = limit.clamp(1, 1_000);
        let mut rows = self
            .conn
            .query(
                "SELECT context,
                        COUNT(*) AS total,
                        SUM(CASE WHEN json_extract(metadata, '$.success') = 1 THEN 1 ELSE 0 END),
                        AVG(CAST(json_extract(metadata, '$.duration_ms') AS REAL))
                 FROM behavior_events
                 WHERE user_id = ?1 AND event_type = 'command' AND context IS NOT NULL
                 GROUP BY context ORDER BY total DESC LIMIT ?2",
                params![user_id, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(CommandFrequencyEntry {
                command: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                count: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                success_count: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                avg_duration_ms: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    // ── User Preferences (user_preferences) ──────────────────────────────────

    /// Upsert a `user_preferences` key-value row.
    ///
    /// Called from `vox-db/src/learning.rs` `BehavioralLearner::preference_inference`.
    pub async fn set_user_preference(
        &self,
        user_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO user_preferences (user_id, key, value, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'))
                 ON CONFLICT(user_id, key)
                 DO UPDATE SET value = excluded.value, updated_at = datetime('now')",
                params![user_id, key, value],
            )
            .await?;
        Ok(())
    }

    /// Read a single `user_preferences` value by `(user_id, key)`, or `None` if absent.
    ///
    /// Called from `vox-mcp/src/memory.rs` preference handler.
    pub async fn get_user_preference(
        &self,
        user_id: &str,
        key: &str,
    ) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT value FROM user_preferences WHERE user_id = ?1 AND key = ?2 LIMIT 1",
                params![user_id, key],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let v: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    /// Return all `(key, value)` pairs in `user_preferences` for `user_id`, optionally filtered by
    /// `prefix` (matched as `key LIKE 'prefix%'`).
    ///
    /// Called from `vox-mcp/src/memory.rs` preference list handler.
    pub async fn list_user_preferences(
        &self,
        user_id: &str,
        prefix: Option<&str>,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = match prefix {
            Some(pfx) => {
                let pattern = format!("{pfx}%");
                self.conn
                    .query(
                        "SELECT key, value FROM user_preferences
                         WHERE user_id = ?1 AND key LIKE ?2
                         ORDER BY key ASC",
                        params![user_id, pattern],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        "SELECT key, value FROM user_preferences
                         WHERE user_id = ?1
                         ORDER BY key ASC",
                        params![user_id],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let k: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let v: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((k, v));
        }
        Ok(out)
    }



    /// Join `llm_interactions` with `llm_feedback` to produce RLHF training pairs.
    ///
    /// Called from `vox-db/src/learning.rs` and `vox-pm/src/feedback.rs`.
    pub async fn get_training_data(&self, limit: i64) -> Result<Vec<TrainingPair>, StoreError> {
        let lim = limit.clamp(1, 50_000);
        let mut rows = self
            .conn
            .query(
                "SELECT i.prompt, i.response, f.rating, f.correction_text, f.feedback_type
                 FROM llm_interactions i
                 LEFT JOIN llm_feedback f ON f.interaction_id = i.id
                 ORDER BY i.id DESC LIMIT ?1",
                params![lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrainingPair {
                prompt: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                response: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                rating: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                correction: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                feedback_type: row
                    .get::<Option<String>>(4)
                    .map_err(|e| StoreError::Db(e.to_string()))?
                    .unwrap_or_else(|| "none".to_string()),
            });
        }
        Ok(out)
    }

    // ── Snippets (snippets) ───────────────────────────────────────────────────

    /// Insert a row into `snippets`. Returns the inserted `rowid`.
    ///
    /// Called from `vox-cli/src/commands/extras/snippet/mod.rs`.
    pub async fn save_snippet(&self, p: SaveSnippetParams<'_>) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO snippets (language, title, code, description, tags, author_id, source_ref, embedding_ref)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    p.language,
                    p.title,
                    p.code,
                    p.description,
                    p.tags,
                    p.author_id,
                    p.source_ref,
                    p.embedding_ref
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Search `snippets` by keyword with an optional tag filter.
    ///
    /// `query` is matched as `LIKE '%query%'` against `title`, `code`, and `description`.
    /// `tag_filter` further narrows by `tags LIKE '%tag_filter%'`.
    /// Returns at most 500 rows, newest first.
    ///
    /// Called from `vox-cli/src/commands/extras/snippet/mod.rs`.
    pub async fn search_snippets(
        &self,
        query: &str,
        tag_filter: Option<&str>,
    ) -> Result<Vec<SnippetEntry>, StoreError> {
        let pattern = if query == "%" || query.is_empty() {
            "%".to_string()
        } else {
            format!("%{query}%")
        };
        let mut rows = match tag_filter {
            Some(tag) => {
                let tag_pat = format!("%{tag}%");
                self.conn
                    .query(
                        "SELECT id, language, title, code, description, tags
                         FROM snippets
                         WHERE (title LIKE ?1 OR code LIKE ?1 OR description LIKE ?1)
                           AND tags LIKE ?2
                         ORDER BY id DESC LIMIT 500",
                        params![pattern, tag_pat],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        "SELECT id, language, title, code, description, tags
                         FROM snippets
                         WHERE title LIKE ?1 OR code LIKE ?1 OR description LIKE ?1
                         ORDER BY id DESC LIMIT 500",
                        params![pattern],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SnippetEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                language: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                title: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                code: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                description: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                tags: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    // ── Private helpers ───────────────────────────────────────────────────────


    async fn collect_pattern_rows(
        rows: &mut turso::Rows,
    ) -> Result<Vec<LearnedPatternEntry>, StoreError> {
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(LearnedPatternEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                user_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                pattern_type: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                category: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                description: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                confidence: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                vcs_snapshot_id: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }
}
