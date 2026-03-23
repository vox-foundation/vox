//! Orchestrator CRUD for [`crate::arca_store::VoxDb`] (Arca / Turso).
//! Contains methods for distributed locks and heartbeats.

use turso::params;


use crate::arca_store::types::StoreError;

impl crate::VoxDb {
    // ── Distributed Locks (distributed_locks) ────────────────────────────────

    /// Check if a lock is held by another node, and if new/ours, insert or refresh.
    /// Returns Ok(fence_token) if acquired, Err(holder_node) if locked by someone else.
    pub async fn acquire_distributed_lock(
        &self,
        lock_key: &str,
        node_id: &str,
        agent_id: &str,
        ttl_secs: i64,
        repository_id: &str,
    ) -> Result<Result<i64, String>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT holder_node, fence_token FROM distributed_locks
             WHERE lock_key = ?1 AND repository_id = ?2 AND expires_at > datetime('now')",
            params![lock_key, repository_id],
        ).await?;

        if let Some(row) = rows.next().await? {
            let holder: String = row.get(0)?;
            if holder != node_id {
                return Ok(Err(holder));
            }
        }

        let mut rows = self.conn.query(
            "SELECT COALESCE(MAX(fence_token), 0) + 1 FROM distributed_locks WHERE lock_key = ?1",
            params![lock_key],
        ).await?;
        
        let next_fence: i64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            1
        };

        self.conn.execute(
            "INSERT INTO distributed_locks (lock_key, holder_node, holder_agent, fence_token, expires_at, repository_id)
             VALUES (?1, ?2, ?3, ?4, datetime('now', '+' || ?5 || ' seconds'), ?6)
             ON CONFLICT(lock_key, repository_id) DO UPDATE SET
                holder_node = excluded.holder_node,
                holder_agent = excluded.holder_agent,
                fence_token = excluded.fence_token,
                acquired_at = datetime('now'),
                expires_at = excluded.expires_at",
            params![lock_key, node_id, agent_id, next_fence, ttl_secs, repository_id],
        ).await?;

        Ok(Ok(next_fence))
    }

    /// Release a distributed lock.
    pub async fn release_distributed_lock(
        &self,
        lock_key: &str,
        node_id: &str,
        repository_id: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "DELETE FROM distributed_locks WHERE lock_key = ?1 AND holder_node = ?2 AND repository_id = ?3",
            params![lock_key, node_id, repository_id],
        ).await?;
        Ok(())
    }

    /// Prune all expired distributed locks.
    pub async fn prune_stale_distributed_locks(&self) -> Result<u64, StoreError> {
        let rows_affected = self.conn.execute(
            "DELETE FROM distributed_locks WHERE expires_at <= datetime('now')",
            (),
        ).await?;
        Ok(rows_affected as u64)
    }

    // ── Heartbeats (mesh_heartbeats) ─────────────────────────────────────────

    /// Upsert a node heartbeat.
    pub async fn upsert_mesh_heartbeat(
        &self,
        node_id: &str,
        agent_id: &str,
        activity: &str,
        now_ms: i64,
        repository_id: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO mesh_heartbeats (node_id, agent_id, last_seen_ms, activity, repository_id)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(node_id, repository_id) DO UPDATE SET
                agent_id = excluded.agent_id,
                last_seen_ms = excluded.last_seen_ms,
                activity = excluded.activity",
            params![node_id, agent_id, now_ms, activity, repository_id],
        ).await?;
        Ok(())
    }

    /// Get live nodes (heartbeats after min_seen).
    pub async fn list_live_nodes(
        &self,
        min_seen_ms: i64,
        repository_id: &str,
    ) -> Result<Vec<Vec<String>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT node_id, agent_id, activity, CAST(last_seen_ms AS TEXT)
             FROM mesh_heartbeats
             WHERE last_seen_ms >= ?1 AND repository_id = ?2",
            params![min_seen_ms, repository_id],
        ).await?;

        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let mut cols = Vec::new();
            cols.push(row.get::<String>(0)?);
            cols.push(row.get::<String>(1)?);
            cols.push(row.get::<String>(2)?);
            cols.push(row.get::<String>(3)?);
            out.push(cols);
        }
        Ok(out)
    }

    /// Remove heartbeats older than threshold.
    pub async fn evict_dead_heartbeats(&self, min_seen_ms: i64) -> Result<u64, StoreError> {
        let affected = self.conn.execute(
            "DELETE FROM mesh_heartbeats WHERE last_seen_ms < ?1",
            params![min_seen_ms],
        ).await?;
        Ok(affected as u64)
    }
}
