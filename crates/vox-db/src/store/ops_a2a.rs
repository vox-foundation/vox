//! A2A messaging, agent OpLog, actor state, file locks, and heartbeats for [`crate::VoxDb`].
//!
//! These coordination primitives were previously mixed into `ops_ludus/gamify_extended.rs`.
//! They are not gamification-domain ops; they belong to the coordination / orchestration layer.

use std::time::{SystemTime, UNIX_EPOCH};

use turso::params;

use crate::store::types::{A2AMessageRow, StoreError};

fn a2a_now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn default_a2a_consumer_id() -> String {
    std::env::var("VOX_A2A_CONSUMER_ID").unwrap_or_else(|_| format!("pid:{}", std::process::id()))
}

impl crate::VoxDb {
    // ── A2A Messages (a2a_messages) ───────────────────────────────────────────

    /// Acknowledge an A2A message by row id.
    pub async fn acknowledge_a2a_message_by_id(&self, id: i64) -> Result<(), StoreError> {
        let now_ms = a2a_now_ms();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE a2a_messages SET acknowledged=1, claim_owner=NULL, claim_until_ms=NULL,
                 processed_at_ms=?1 WHERE id=?2",
                    params![now_ms, id],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Send an A2A message.
    #[allow(clippy::too_many_arguments)]
    pub async fn send_a2a_message(
        &self,
        message_uuid: &str,
        sender_agent: &str,
        receiver_agent: &str,
        msg_type: &str,
        payload: &str,
        priority: i64,
        thread_id: Option<&str>,
        repository_id: &str,
    ) -> Result<(), StoreError> {
        let message_uuid = message_uuid.to_string();
        let sender_agent = sender_agent.to_string();
        let receiver_agent = receiver_agent.to_string();
        let msg_type = msg_type.to_string();
        let payload = payload.to_string();
        let thread_id = thread_id.map(str::to_string);
        let repository_id = repository_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO a2a_messages
             (message_uuid, sender_agent, receiver_agent, msg_type, payload, priority, thread_id, repository_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        message_uuid.as_str(),
                        sender_agent.as_str(),
                        receiver_agent.as_str(),
                        msg_type.as_str(),
                        payload.as_str(),
                        priority,
                        thread_id.as_deref(),
                        repository_id.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Poll unacknowledged messages for an agent in a repository.
    ///
    /// Claims a bounded batch for `default_a2a_consumer_id` so concurrent poll workers do not
    /// receive duplicate rows until the claim lease expires. Override the consumer via
    /// `VOX_A2A_CONSUMER_ID` or call [`Self::poll_a2a_inbox_claimed`].
    pub async fn poll_a2a_inbox(
        &self,
        agent_id: &str,
        repository_id: &str,
    ) -> Result<Vec<A2AMessageRow>, StoreError> {
        self.poll_a2a_inbox_claimed(
            agent_id,
            repository_id,
            &default_a2a_consumer_id(),
            256,
            120_000,
        )
        .await
    }

    /// Claim up to `limit` inbox rows for `consumer_id`, leasing each for `lease_ms` milliseconds.
    pub async fn poll_a2a_inbox_claimed(
        &self,
        receiver_agent: &str,
        repository_id: &str,
        consumer_id: &str,
        limit: i64,
        lease_ms: i64,
    ) -> Result<Vec<A2AMessageRow>, StoreError> {
        let limit = limit.clamp(1, 2048);
        let now_ms = a2a_now_ms();
        let claim_deadline = now_ms.saturating_add(lease_ms);
        let receiver_agent = receiver_agent.to_string();
        let repository_id = repository_id.to_string();
        let consumer_id = consumer_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker
            .call(|| async move {
                conn.execute("BEGIN IMMEDIATE", ())
                    .await
                    .map_err(StoreError::from)?;

                let result = async {
                    let mut id_rows = conn
                        .query(
                            "SELECT id FROM a2a_messages
                     WHERE receiver_agent = ?1 AND repository_id = ?2 AND acknowledged = 0
                       AND (claim_until_ms IS NULL OR claim_until_ms < ?3 OR claim_owner = ?4)
                     ORDER BY priority DESC, id ASC
                     LIMIT ?5",
                            params![
                                receiver_agent.as_str(),
                                repository_id.as_str(),
                                now_ms,
                                consumer_id.as_str(),
                                limit
                            ],
                        )
                        .await?;

                    let mut ids: Vec<i64> = Vec::new();
                    while let Some(row) = id_rows.next().await? {
                        ids.push(row.get(0)?);
                    }

                    for id in &ids {
                        conn.execute(
                            "UPDATE a2a_messages SET
                           claim_owner = ?1,
                           claim_until_ms = ?2,
                           delivery_attempts = delivery_attempts + IIF(
                             COALESCE(claim_owner, '') = ?1 AND COALESCE(claim_until_ms, 0) >= ?3,
                             0,
                             1
                           ),
                           last_claim_error = NULL
                         WHERE id = ?4",
                            params![consumer_id.as_str(), claim_deadline, now_ms, id],
                        )
                        .await?;
                    }

                    let mut out = Vec::new();
                    for id in ids {
                        let mut rows = conn
                            .query(
                                "SELECT id, message_uuid, sender_agent, receiver_agent, msg_type, payload,
                            priority, thread_id, acknowledged, created_at, repository_id,
                            claim_owner, claim_until_ms, COALESCE(delivery_attempts, 0),
                            last_claim_error, processed_at_ms
                     FROM a2a_messages WHERE id = ?1",
                                params![id],
                            )
                            .await?;
                        if let Some(row) = rows.next().await? {
                            let ack: i64 = row.get(8).unwrap_or(0);
                            out.push(A2AMessageRow {
                                id: row.get(0)?,
                                message_uuid: row.get(1)?,
                                sender_agent: row.get(2)?,
                                receiver_agent: row.get(3)?,
                                msg_type: row.get(4)?,
                                payload: row.get(5)?,
                                priority: row.get(6)?,
                                thread_id: row.get(7)?,
                                acknowledged: ack != 0,
                                created_at: row.get(9)?,
                                repository_id: row.get(10)?,
                                claim_owner: row.get(11)?,
                                claim_until_ms: row.get(12)?,
                                delivery_attempts: row.get(13)?,
                                last_claim_error: row.get(14)?,
                                processed_at_ms: row.get(15)?,
                            });
                        }
                    }

                    conn.execute("COMMIT", ()).await?;
                    Ok::<Vec<A2AMessageRow>, StoreError>(out)
                }
                .await;

                if result.is_err() {
                    let _ = conn.execute("ROLLBACK", ()).await;
                }
                result
            })
            .await
    }

    /// Acknowledge an A2A message by UUID.
    pub async fn acknowledge_a2a_message_by_uuid(&self, uuid: &str) -> Result<(), StoreError> {
        let now_ms = a2a_now_ms();
        let uuid = uuid.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE a2a_messages SET acknowledged=1, claim_owner=NULL, claim_until_ms=NULL,
                 processed_at_ms=?1 WHERE message_uuid=?2",
                    params![now_ms, uuid.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Prune old acknowledged A2A messages older than N days.
    pub async fn prune_a2a_messages(&self, older_than_days: u32) -> Result<u64, StoreError> {
        let sql = format!(
            "DELETE FROM a2a_messages WHERE acknowledged=1 AND created_at < datetime('now', '-{} days')",
            older_than_days
        );
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let affected = conn.execute(&sql, ()).await?;
                Ok::<_, StoreError>(affected)
            })
            .await
    }

    // ── OpLog (agent_oplog) ───────────────────────────────────────────────────

    /// Append an operation log entry.
    #[allow(clippy::too_many_arguments)]
    pub async fn append_oplog_entry(
        &self,
        agent_id: &str,
        operation_id: &str,
        kind_json: &str,
        description: &str,
        predecessor_hash: Option<&str>,
        model_id: Option<&str>,
        change_id: Option<i64>,
        timestamp_ms: i64,
        repository_id: &str,
    ) -> Result<(), StoreError> {
        let agent_id = agent_id.to_string();
        let operation_id = operation_id.to_string();
        let kind_json = kind_json.to_string();
        let description = description.to_string();
        let predecessor_hash = predecessor_hash.map(str::to_string);
        let model_id = model_id.map(str::to_string);
        let repository_id = repository_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO agent_oplog
             (agent_id, operation_id, kind, description, predecessor_hash, model_id, change_id, timestamp_ms, repository_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        agent_id.as_str(),
                        operation_id.as_str(),
                        kind_json.as_str(),
                        description.as_str(),
                        predecessor_hash.as_deref(),
                        model_id.as_deref(),
                        change_id,
                        timestamp_ms,
                        repository_id.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List oplog entries for a repository (optionally filtered by agent), newest first.
    pub async fn list_oplog_entries(
        &self,
        agent_id: Option<&str>,
        repository_id: &str,
        limit: u32,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut cursor = if let Some(aid) = agent_id {
            self.conn
                .query(
                    "SELECT operation_id, agent_id, kind, description, predecessor_hash, model_id,
                              CAST(change_id AS TEXT), CAST(timestamp_ms AS TEXT), CAST(undone AS TEXT)
                       FROM agent_oplog WHERE repository_id=?1 AND agent_id=?2
                       ORDER BY timestamp_ms DESC LIMIT ?3",
                    params![repository_id, aid, limit as i64],
                )
                .await?
        } else {
            self.conn
                .query(
                    "SELECT operation_id, agent_id, kind, description, predecessor_hash, model_id,
                              CAST(change_id AS TEXT), CAST(timestamp_ms AS TEXT), CAST(undone AS TEXT)
                       FROM agent_oplog WHERE repository_id=?1
                       ORDER BY timestamp_ms DESC LIMIT ?2",
                    params![repository_id, limit as i64],
                )
                .await?
        };
        let mut out = Vec::new();
        while let Some(row) = cursor.next().await? {
            let cols: Vec<Option<String>> = (0..9)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    /// Highest numeric `operation_id` suffix (`OP-000123` → `123`) currently
    /// persisted in `agent_oplog` for `repository_id`, or `None` if the table
    /// has no rows for that repo yet. This is the table `orch.subscribe`/
    /// `orch.subscribe_events`'s replay-from-offset actually reads via
    /// [`Self::list_oplog_entries_since`] — reseeding an `OperationId`
    /// generator (T1.3 restart-durability) must query *this* table, not the
    /// unrelated mesh-replication `convergence_op_log` table (see
    /// [`Self::max_convergence_op_id`], which tracks a different sequence
    /// entirely).
    pub async fn max_agent_oplog_id(&self, repository_id: &str) -> Result<Option<u64>, StoreError> {
        let repository_id = repository_id.to_string();
        let mut rows = self
            .conn
            .query(
                "SELECT MAX(CAST(SUBSTR(operation_id, 4) AS INTEGER)) \
                   FROM agent_oplog WHERE repository_id=?1",
                params![repository_id.as_str()],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            let max_id: Option<i64> = row.get(0).ok().flatten();
            Ok(max_id.map(|v| v as u64))
        } else {
            Ok(None)
        }
    }

    /// List oplog entries for a repository whose numeric `operation_id`
    /// suffix (`OP-000123` → `123`) is strictly greater than `since_op_id`,
    /// oldest-first, unbounded (T1.3 replay-from-offset). `operation_id` is
    /// stored as the zero-padded `OP-NNNNNN` text form (see
    /// `OperationId::fmt::Display`), so the numeric comparison is done via
    /// `CAST(SUBSTR(operation_id, 4) AS INTEGER)` rather than a lexicographic
    /// string compare (which would misorder past 6 digits or with a
    /// non-`OP-` prefixed id).
    pub async fn list_oplog_entries_since(
        &self,
        repository_id: &str,
        since_op_id: u64,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut cursor = self
            .conn
            .query(
                "SELECT operation_id, agent_id, kind, description, predecessor_hash, model_id,
                          CAST(change_id AS TEXT), CAST(timestamp_ms AS TEXT), CAST(undone AS TEXT)
                   FROM agent_oplog
                   WHERE repository_id=?1
                     AND CAST(SUBSTR(operation_id, 4) AS INTEGER) > ?2
                   ORDER BY CAST(SUBSTR(operation_id, 4) AS INTEGER) ASC",
                params![repository_id, since_op_id as i64],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = cursor.next().await? {
            let cols: Vec<Option<String>> = (0..9)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    /// Mark an oplog entry as undone (or re-done).
    pub async fn set_oplog_undone(
        &self,
        operation_id: &str,
        undone: bool,
    ) -> Result<(), StoreError> {
        let operation_id = operation_id.to_string();
        let undone_flag = if undone { 1i64 } else { 0i64 };
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE agent_oplog SET undone=?1 WHERE operation_id=?2",
                    params![undone_flag, operation_id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    // ── Checkpoints (checkpoint_blobs) — T1.6 cold-tier compaction ────────────

    /// Insert a checkpoint blob (concatenated `Projection::snapshot()` bytes for
    /// ops in `(op_id_lo..=op_id_hi]`) and return the new row's `id`, which is
    /// what `OperationKind::Checkpoint.payload_blob_id` stores.
    pub async fn insert_checkpoint_blob(
        &self,
        repository_id: &str,
        op_id_lo: u64,
        op_id_hi: u64,
        blake3_hex: &str,
        payload: Vec<u8>,
        created_at_ms: i64,
    ) -> Result<u64, StoreError> {
        let repository_id = repository_id.to_string();
        let blake3_hex = blake3_hex.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO checkpoint_blobs \
                     (repository_id, op_id_lo, op_id_hi, blake3, payload, created_at_ms) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        repository_id.as_str(),
                        op_id_lo as i64,
                        op_id_hi as i64,
                        blake3_hex.as_str(),
                        payload.as_slice(),
                        created_at_ms,
                    ],
                )
                .await?;
                let mut rows = conn.query("SELECT last_insert_rowid()", ()).await?;
                let id: i64 = rows
                    .next()
                    .await?
                    .ok_or_else(|| StoreError::Db("last_insert_rowid returned no row".into()))?
                    .get(0)
                    .map_err(|e| StoreError::Db(e.to_string()))?;
                Ok::<u64, StoreError>(id as u64)
            })
            .await
    }

    /// Fetch the checkpoint blob row with the highest `op_id_hi` for `repository_id`,
    /// or `None` if no checkpoint has been recorded yet. Used at boot to find the
    /// most recent cold-tier snapshot to restore before replaying the tail.
    pub async fn latest_checkpoint_blob(
        &self,
        repository_id: &str,
    ) -> Result<Option<(u64, u64, u64, String, Vec<u8>)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, op_id_lo, op_id_hi, blake3, payload \
                   FROM checkpoint_blobs WHERE repository_id=?1 \
                   ORDER BY op_id_hi DESC LIMIT 1",
                params![repository_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(None);
        };
        let id: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        let op_id_lo: i64 = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
        let op_id_hi: i64 = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
        let blake3_hex: String = row.get(3).map_err(|e| StoreError::Db(e.to_string()))?;
        let payload: Vec<u8> = row.get(4).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(Some((
            id as u64,
            op_id_lo as u64,
            op_id_hi as u64,
            blake3_hex,
            payload,
        )))
    }

    /// Delete `agent_oplog` rows for `repository_id` whose numeric `operation_id`
    /// suffix is `<= op_id_lo` (T1.6 warm-tier pruning after a checkpoint covers
    /// them). Returns the number of rows deleted.
    ///
    /// Prefer [`Self::prune_agent_oplog_up_to_excluding`] from any caller that
    /// must preserve specific rows within the pruned range (e.g. unresolved
    /// HITL approval/feedback/doubt entries — T1.6 follow-up Bug 2). This
    /// unconditional variant is kept for callers (and existing tests) that
    /// intentionally want a hard prune with no exclusions.
    pub async fn prune_agent_oplog_up_to(
        &self,
        repository_id: &str,
        op_id_lo: u64,
    ) -> Result<u64, StoreError> {
        self.prune_agent_oplog_up_to_excluding(repository_id, op_id_lo, &[])
            .await
    }

    /// [`Self::prune_agent_oplog_up_to`], but never deletes a row whose
    /// numeric `operation_id` suffix appears in `exclude_op_ids` — used to
    /// keep unresolved `ApprovalRequested`/`FeedbackRequested`/`TaskDoubted`
    /// rows alive across a checkpoint compaction even when they fall at or
    /// below `op_id_lo` (T1.6 follow-up, Bug 2: pruning was previously
    /// unconditional on `operation_id <= up_to`, with no awareness of
    /// whether a HITL `*Requested` entry in that range had a matching
    /// `*Resolved` entry — an unresolved approval parked before a restart
    /// could be silently, permanently deleted by the next compaction, with no
    /// trace anywhere and no error). Returns the number of rows actually
    /// deleted (excluded rows are not counted).
    pub async fn prune_agent_oplog_up_to_excluding(
        &self,
        repository_id: &str,
        op_id_lo: u64,
        exclude_op_ids: &[u64],
    ) -> Result<u64, StoreError> {
        let repository_id = repository_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        if exclude_op_ids.is_empty() {
            return breaker
                .call(|| async move {
                    let affected = conn
                        .execute(
                            "DELETE FROM agent_oplog \
                               WHERE repository_id=?1 \
                                 AND CAST(SUBSTR(operation_id, 4) AS INTEGER) <= ?2",
                            params![repository_id.as_str(), op_id_lo as i64],
                        )
                        .await?;
                    Ok::<u64, StoreError>(affected)
                })
                .await;
        }

        // Build a NOT IN (...) clause. `exclude_op_ids` comes from a bounded
        // scan of one checkpoint interval's worth of open HITL entries, so
        // this list is never large enough to threaten SQLite's parameter-count
        // limit in practice.
        let placeholders = (0..exclude_op_ids.len())
            .map(|i| format!("?{}", i + 3))
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "DELETE FROM agent_oplog \
               WHERE repository_id=?1 \
                 AND CAST(SUBSTR(operation_id, 4) AS INTEGER) <= ?2 \
                 AND CAST(SUBSTR(operation_id, 4) AS INTEGER) NOT IN ({placeholders})"
        );
        let exclude_op_ids = exclude_op_ids.to_vec();
        breaker
            .call(|| async move {
                let mut bound: Vec<turso::Value> =
                    vec![repository_id.as_str().into(), (op_id_lo as i64).into()];
                bound.extend(
                    exclude_op_ids
                        .iter()
                        .map(|id| turso::Value::from(*id as i64)),
                );
                let affected = conn.execute(&sql, bound).await?;
                Ok::<u64, StoreError>(affected)
            })
            .await
    }

    /// List `agent_oplog` rows for `repository_id` whose numeric
    /// `operation_id` suffix is `<= up_to`, oldest-first. Used by T1.6's
    /// `compact_now` to scan the range about to be pruned for unresolved HITL
    /// entries (Bug 2 follow-up) before deleting anything — a full-history
    /// scan bounded by `up_to` rather than `list_oplog_entries_since`'s
    /// tail-only view, since an approval requested in an earlier checkpoint
    /// interval and still unresolved would otherwise be invisible to a
    /// tail-only scan (it already survived the earlier compaction specifically
    /// *because* it was excluded from that prune).
    pub async fn list_oplog_entries_up_to(
        &self,
        repository_id: &str,
        up_to: u64,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut cursor = self
            .conn
            .query(
                "SELECT operation_id, agent_id, kind, description, predecessor_hash, model_id,
                          CAST(change_id AS TEXT), CAST(timestamp_ms AS TEXT), CAST(undone AS TEXT)
                   FROM agent_oplog
                   WHERE repository_id=?1
                     AND CAST(SUBSTR(operation_id, 4) AS INTEGER) <= ?2
                   ORDER BY CAST(SUBSTR(operation_id, 4) AS INTEGER) ASC",
                params![repository_id, up_to as i64],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = cursor.next().await? {
            let cols: Vec<Option<String>> = (0..9)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    // ── Actor State (actor_state) ─────────────────────────────────────────────

    /// Save a JSON-serialized actor state value under a key (upsert).
    pub async fn save_actor_state(&self, key: &str, value_json: &str) -> Result<(), StoreError> {
        let key = key.to_string();
        let value_json = value_json.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO actor_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')",
                    params![key.as_str(), value_json.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Load a JSON-serialized actor state value by key.
    pub async fn load_actor_state(&self, key: &str) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query("SELECT value FROM actor_state WHERE key=?1", params![key])
            .await?;
        Ok(rows
            .next()
            .await?
            .and_then(|r| r.get::<Option<String>>(0).unwrap_or(None)))
    }

    /// Delete an actor state entry.
    pub async fn delete_actor_state(&self, key: &str) -> Result<(), StoreError> {
        let key = key.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "DELETE FROM actor_state WHERE key=?1",
                    params![key.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    // ── Agent File Locks (agent_locks) ────────────────────────────────────────

    /// Try to acquire a file lock for an agent. Returns true if acquired.
    pub async fn acquire_file_lock(
        &self,
        path: &str,
        agent_id: &str,
        repository_id: &str,
    ) -> Result<bool, StoreError> {
        let path = path.to_string();
        let agent_id = agent_id.to_string();
        let repository_id = repository_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let affected = conn
                    .execute(
                        "INSERT OR IGNORE INTO agent_locks (path, agent_id, repository_id, acquired_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
                        params![path.as_str(), agent_id.as_str(), repository_id.as_str()],
                    )
                    .await?;
                Ok::<_, StoreError>(affected > 0)
            })
            .await
    }

    /// Release a file lock held by an agent.
    pub async fn release_file_lock(&self, path: &str, agent_id: &str) -> Result<(), StoreError> {
        let path = path.to_string();
        let agent_id = agent_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "DELETE FROM agent_locks WHERE path=?1 AND agent_id=?2",
                    params![path.as_str(), agent_id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List current file locks for a repository.
    pub async fn list_file_locks(
        &self,
        repository_id: &str,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT path, agent_id, acquired_at FROM agent_locks WHERE repository_id=?1",
                params![repository_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<String>(2)?,
            ));
        }
        Ok(out)
    }

    // ── Agent Heartbeats (agent_heartbeats) ───────────────────────────────────

    /// Upsert a heartbeat record for an agent.
    pub async fn upsert_heartbeat(
        &self,
        agent_id: &str,
        repository_id: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        let agent_id = agent_id.to_string();
        let repository_id = repository_id.to_string();
        let status = status.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO agent_heartbeats (agent_id, repository_id, status, last_seen)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(agent_id, repository_id) DO UPDATE SET
               status=excluded.status, last_seen=datetime('now')",
                    params![agent_id.as_str(), repository_id.as_str(), status.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// List heartbeats for a repository.
    pub async fn list_heartbeats(
        &self,
        repository_id: &str,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT agent_id, status, last_seen FROM agent_heartbeats WHERE repository_id=?1",
                params![repository_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<String>(2)?,
            ));
        }
        Ok(out)
    }

    /// Delete heartbeats not updated within `timeout_secs` seconds.
    pub async fn prune_stale_heartbeats(&self, timeout_secs: i64) -> Result<u64, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let affected = conn
                    .execute(
                        "DELETE FROM agent_heartbeats
             WHERE last_seen < datetime('now', '-' || ?1 || ' seconds')",
                        params![timeout_secs],
                    )
                    .await?;
                Ok::<_, StoreError>(affected)
            })
            .await
    }

    // ── Vox Concilium Shared `#seq` Sequence Board (`CONC-02`) ────────────────

    /// Persist a sequenced `#seq` board entry into `a2a_messages` (`msg_type = "concilium_seq_board"`).
    pub async fn append_concilium_seq_entry(
        &self,
        repository_id: &str,
        entry: &vox_orchestrator_types::SequenceBoardEntry,
    ) -> Result<(), StoreError> {
        let payload =
            serde_json::to_string(entry).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let message_uuid = format!(
            "concilium-seq-{}-{}-{}",
            repository_id, entry.seq, entry.span_id
        );
        let priority_seq = entry.seq.min(i64::MAX as u64) as i64;
        self.send_a2a_message(
            &message_uuid,
            &entry.sender,
            &entry.channel,
            "concilium_seq_board",
            &payload,
            priority_seq,
            Some(entry.traceparent.as_str()),
            repository_id,
        )
        .await
    }

    /// Read persisted `#seq` board entries from `a2a_messages` ordered by `seq ASC`.
    pub async fn read_concilium_seq_board(
        &self,
        repository_id: &str,
        filter: &vox_orchestrator_types::SequenceBoardFilter,
    ) -> Result<Vec<vox_orchestrator_types::SequenceBoardEntry>, StoreError> {
        let is_tail = filter.since_seq.is_none() && filter.seqs.as_ref().is_none_or(Vec::is_empty);
        let since_seq = filter.since_seq.unwrap_or(0).min(i64::MAX as u64) as i64;
        let limit = filter.limit.unwrap_or(50).clamp(1, 512);
        let channel_filter = filter
            .channel
            .as_deref()
            .map(str::trim)
            .filter(|s| {
                !s.is_empty() && !s.eq_ignore_ascii_case("#all") && !s.eq_ignore_ascii_case("all")
            })
            .map(|ch| {
                let lower = ch.to_ascii_lowercase();
                if lower.starts_with('#') {
                    lower
                } else {
                    format!("#{lower}")
                }
            });

        let sql = if is_tail {
            "SELECT payload FROM a2a_messages
             WHERE repository_id = ?1 AND msg_type = 'concilium_seq_board' AND priority >= 0
             ORDER BY priority DESC, id DESC"
        } else if filter.since_seq.is_some() {
            "SELECT payload FROM a2a_messages
             WHERE repository_id = ?1 AND msg_type = 'concilium_seq_board' AND priority > ?2
             ORDER BY priority ASC, id ASC"
        } else {
            "SELECT payload FROM a2a_messages
             WHERE repository_id = ?1 AND msg_type = 'concilium_seq_board' AND priority >= ?2
             ORDER BY priority ASC, id ASC"
        };
        let mut rows = if is_tail {
            self.conn.query(sql, params![repository_id]).await?
        } else {
            self.conn
                .query(sql, params![repository_id, since_seq])
                .await?
        };

        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let payload: String = row.get(0)?;
            let Ok(entry) =
                serde_json::from_str::<vox_orchestrator_types::SequenceBoardEntry>(&payload)
            else {
                continue;
            };
            if let Some(ref ch) = channel_filter
                && !entry.channel.eq_ignore_ascii_case(ch)
            {
                continue;
            }
            if let Some(ref seqs) = filter.seqs
                && !seqs.is_empty()
                && !seqs.contains(&entry.seq)
            {
                continue;
            }
            if let Some(since) = filter.since_seq
                && entry.seq <= since
            {
                continue;
            }
            out.push(entry);
            if out.len() >= limit {
                break;
            }
        }
        if is_tail {
            out.reverse();
        }
        Ok(out)
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use vox_orchestrator_types::{
        ComputeTier, SequenceBoardEntry, SequenceBoardFilter, SwarmWorkspaceMode,
    };

    use crate::{DbConfig, VoxDb};

    fn entry(seq: u64, channel: &str) -> SequenceBoardEntry {
        SequenceBoardEntry {
            seq,
            channel: channel.to_string(),
            sender: "author".to_string(),
            body: format!("entry {seq}"),
            mentions: Vec::new(),
            timestamp_ms: 1_700_000_000_000 + seq,
            traceparent: format!("00-4bf92f3577b34da6a3ce929d0e0e4736-{seq:016x}-01"),
            span_id: format!("{seq:016x}"),
            compute_tier: ComputeTier::LocalMensGpu,
            file_lock_prefix: None,
            workspace_mode: SwarmWorkspaceMode::Share,
            parked_since_seq: None,
            receipt_badge: None,
            is_watchdog_alert: None,
        }
    }

    #[tokio::test]
    async fn concilium_seq_board_persists_tail_and_cursor_filters() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
        for seq in 1..=60 {
            let ch = if seq % 2 == 0 { "#verify" } else { "#parser" };
            db.append_concilium_seq_entry("repo-conc", &entry(seq, ch))
                .await
                .expect("append");
        }
        let tail = db
            .read_concilium_seq_board("repo-conc", &SequenceBoardFilter::default())
            .await
            .expect("tail");
        assert_eq!(tail.len(), 50);
        assert_eq!(tail.first().expect("first").seq, 11);
        assert_eq!(tail.last().expect("last").seq, 60);

        let cursor = db
            .read_concilium_seq_board(
                "repo-conc",
                &SequenceBoardFilter {
                    since_seq: Some(50),
                    channel: Some("verify".into()),
                    limit: Some(3),
                    ..Default::default()
                },
            )
            .await
            .expect("cursor");
        assert_eq!(
            cursor.iter().map(|e| e.seq).collect::<Vec<_>>(),
            vec![52, 54, 56]
        );
    }
}
