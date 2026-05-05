//! [`MeshStore`] backed by a [`vox_db::VoxDb`] Turso connection.
//!
//! Tables live in the caller's Arca database (schema fragment `vox_mesh`, schema v60).
//! The three tables are:
//! - `mesh_a2a_messages`
//! - `mesh_exec_leases`
//! - `mesh_dispatch_results`
//!
//! All writes go through the Turso connection already held by `VoxDb`; WAL mode and
//! synchronous=NORMAL are applied by `VoxDb::apply_pragmas` on open.

use std::collections::HashMap;
use std::time::Instant;

use async_trait::async_trait;
use tracing::debug;

use super::{A2AAck, A2APage, IntegrityFinding, IntegrityReport, MeshStore, MeshStoreError};
use crate::transport::{A2AStoredMessage, DispatchResponse, RemoteExecLeaseRow};

/// VoxDb-backed implementation of [`MeshStore`].
///
/// Constructed via [`VoxDbMeshStore::new`]; wraps a `vox_db::VoxDb` handle
/// (which already owns a Turso connection).
#[derive(Clone)]
pub struct VoxDbMeshStore {
    db: vox_db::VoxDb,
}

impl VoxDbMeshStore {
    /// Wrap an existing `VoxDb` handle. The handle must have been opened with
    /// schema v60+ (the `vox_mesh` fragment includes the three mesh tables).
    #[must_use]
    pub fn new(db: vox_db::VoxDb) -> Self {
        Self { db }
    }
}

// convenience: map turso errors → MeshStoreError
fn turso_err(e: turso::Error) -> MeshStoreError {
    let s = e.to_string();
    if s.contains("locked") || s.contains("busy") || s.contains("SQLITE_BUSY") {
        MeshStoreError::LockContention
    } else {
        MeshStoreError::Io(s)
    }
}

macro_rules! store_span {
    ($op:expr, $ms:expr) => {
        debug!(
            "vox.mesh.store.op" = $op,
            "vox.mesh.store.duration_ms" = $ms
        );
    };
    ($op:expr, $ms:expr, rows = $n:expr) => {
        debug!(
            "vox.mesh.store.op" = $op,
            "vox.mesh.store.duration_ms" = $ms,
            "vox.mesh.store.row_count" = $n
        );
    };
    ($op:expr, $ms:expr, err = $e:expr) => {
        debug!(
            "vox.mesh.store.op" = $op,
            "vox.mesh.store.duration_ms" = $ms,
            "vox.mesh.store.error" = $e
        );
    };
}

// ── row → struct helpers ──────────────────────────────────────────────

fn a2a_from_row(row: &turso::Row) -> Result<A2AStoredMessage, MeshStoreError> {
    let col = |i: i32| -> Result<_, MeshStoreError> {
        row.get(i as usize)
            .map_err(|e| MeshStoreError::Io(e.to_string()))
    };
    let col_opt = |i: i32| -> Result<Option<String>, MeshStoreError> {
        row.get::<Option<String>>(i as usize)
            .map_err(|e| MeshStoreError::Io(e.to_string()))
    };
    let col_i64 = |i: i32| -> Result<i64, MeshStoreError> {
        row.get::<i64>(i as usize)
            .map_err(|e| MeshStoreError::Io(e.to_string()))
    };
    let col_opt_i64 = |i: i32| -> Result<Option<i64>, MeshStoreError> {
        row.get::<Option<i64>>(i as usize)
            .map_err(|e| MeshStoreError::Io(e.to_string()))
    };

    Ok(A2AStoredMessage {
        id: col_i64(0)? as u64,
        sender_agent_id: col(1)?,
        receiver_agent_id: col(2)?,
        message_type: col(3)?,
        payload: col(4)?,
        created_unix_ms: col_i64(5)? as u64,
        acknowledged: col_i64(6)? != 0,
        lease_holder_node_id: col_opt(7)?,
        lease_expires_unix_ms: col_opt_i64(8)?.map(|v| v as u64),
        privacy_class: col_opt(9)?,
        idempotency_dedupe_key: col_opt(10)?,
        payload_blake3_hex: col_opt(11)?,
        worker_ed25519_sig_b64: col_opt(12)?,
        jwe_payload: col_opt(13)?,
        priority: col_i64(14)? as u8,
        task_kind: col_opt(15)?,
        model_id: col_opt(16)?,
        sender_node_id: col_opt(17)?,
        traceparent: col_opt(18)?,
    })
}

fn lease_from_row(row: &turso::Row) -> Result<RemoteExecLeaseRow, MeshStoreError> {
    Ok(RemoteExecLeaseRow {
        lease_id: row
            .get::<String>(0)
            .map_err(|e| MeshStoreError::Io(e.to_string()))?,
        scope_key: row
            .get::<String>(1)
            .map_err(|e| MeshStoreError::Io(e.to_string()))?,
        holder_node_id: row
            .get::<String>(2)
            .map_err(|e| MeshStoreError::Io(e.to_string()))?,
        expires_unix_ms: row
            .get::<i64>(3)
            .map_err(|e| MeshStoreError::Io(e.to_string()))? as u64,
    })
}

// ────────────────────────────────────────── MeshStore impl ───────────

#[async_trait]
impl MeshStore for VoxDbMeshStore {
    // ── A2A inbox ──────────────────────────────────────────────────

    async fn put_a2a(&self, msg: &A2AStoredMessage) -> Result<(), MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        conn.execute(
            r#"INSERT INTO mesh_a2a_messages (
                id, sender_agent_id, receiver_agent_id, message_type, payload,
                idempotency_key, idempotency_dedupe_key, privacy_class, payload_blake3_hex,
                worker_ed25519_sig_b64, jwe_payload, priority, task_kind, model_id,
                sender_node_id, traceparent, created_at, acknowledged,
                lease_holder_node_id, lease_expires_unix_ms
            ) VALUES (
                ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20
            ) ON CONFLICT(id) DO UPDATE SET
                acknowledged          = excluded.acknowledged,
                lease_holder_node_id  = excluded.lease_holder_node_id,
                lease_expires_unix_ms = excluded.lease_expires_unix_ms,
                traceparent           = excluded.traceparent,
                acked_at              = CASE WHEN excluded.acknowledged = 1
                                             THEN COALESCE(acked_at, excluded.created_at)
                                             ELSE NULL END"#,
            turso::params![
                msg.id as i64,
                msg.sender_agent_id.as_str(),
                msg.receiver_agent_id.as_str(),
                msg.message_type.as_str(),
                msg.payload.as_str(),
                msg.idempotency_dedupe_key.as_deref(),
                msg.idempotency_dedupe_key.as_deref(),
                msg.privacy_class.as_deref(),
                msg.payload_blake3_hex.as_deref(),
                msg.worker_ed25519_sig_b64.as_deref(),
                msg.jwe_payload.as_deref(),
                msg.priority as i64,
                msg.task_kind.as_deref(),
                msg.model_id.as_deref(),
                msg.sender_node_id.as_deref(),
                msg.traceparent.as_deref(),
                msg.created_unix_ms as i64,
                if msg.acknowledged { 1i64 } else { 0i64 },
                msg.lease_holder_node_id.as_deref(),
                msg.lease_expires_unix_ms.map(|v| v as i64),
            ],
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            store_span!("put_a2a", t.elapsed().as_millis(), err = msg.as_str());
            turso_err(e)
        })?;
        store_span!("put_a2a", t.elapsed().as_millis());
        Ok(())
    }

    async fn list_a2a(&self, page: A2APage) -> Result<Vec<A2AStoredMessage>, MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();

        // Build dynamic query.
        let mut sql = String::from(
            r#"SELECT id, sender_agent_id, receiver_agent_id, message_type, payload,
                      created_at, acknowledged, lease_holder_node_id, lease_expires_unix_ms,
                      privacy_class, idempotency_dedupe_key, payload_blake3_hex,
                      worker_ed25519_sig_b64, jwe_payload, priority, task_kind, model_id,
                      sender_node_id, traceparent
               FROM mesh_a2a_messages WHERE 1=1"#,
        );
        let mut args: Vec<turso::Value> = Vec::new();

        if !page.include_acked {
            sql.push_str(" AND acknowledged = 0");
        }
        if let Some(ref rcv) = page.receiver_agent_id {
            args.push(turso::Value::Text(rcv.clone()));
            sql.push_str(&format!(" AND receiver_agent_id = ?{}", args.len()));
        }
        if let Some(since) = page.since_id {
            args.push(turso::Value::Integer(since as i64));
            sql.push_str(&format!(" AND id > ?{}", args.len()));
        }
        sql.push_str(" ORDER BY id ASC");
        if let Some(lim) = page.limit {
            args.push(turso::Value::Integer(lim as i64));
            sql.push_str(&format!(" LIMIT ?{}", args.len()));
        }

        let mut rows = conn
            .query(&sql, turso::params_from_iter(args))
            .await
            .map_err(turso_err)?;

        let mut result = Vec::new();
        while let Some(row) = rows.next().await.map_err(turso_err)? {
            result.push(a2a_from_row(&row)?);
        }
        store_span!("list_a2a", t.elapsed().as_millis(), rows = result.len());
        Ok(result)
    }

    async fn ack_a2a(&self, message_id: u64, ack: A2AAck) -> Result<(), MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let acked_at: Option<i64> = if ack.acknowledged {
            Some(ack.acked_unix_ms as i64)
        } else {
            None
        };
        conn.execute(
            "UPDATE mesh_a2a_messages SET acknowledged = ?1, acked_at = ?2 WHERE id = ?3",
            turso::params![
                if ack.acknowledged { 1i64 } else { 0i64 },
                acked_at,
                message_id as i64,
            ],
        )
        .await
        .map_err(|e| {
            store_span!(
                "ack_a2a",
                t.elapsed().as_millis(),
                err = e.to_string().as_str()
            );
            turso_err(e)
        })?;
        store_span!("ack_a2a", t.elapsed().as_millis());
        Ok(())
    }

    async fn load_all_a2a(&self) -> Result<Vec<A2AStoredMessage>, MeshStoreError> {
        self.list_a2a(A2APage {
            include_acked: true,
            ..Default::default()
        })
        .await
    }

    // ── exec leases ────────────────────────────────────────────────

    async fn put_exec_lease(&self, row: &RemoteExecLeaseRow) -> Result<(), MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let now_ms = crate::now_ms() as i64;
        conn.execute(
            r#"INSERT INTO mesh_exec_leases
               (lease_id, task_id, scope_key, holder_node_id, granted_at, expires_at, state)
               VALUES (?1,?2,?3,?4,?5,?6,'granted')
               ON CONFLICT(lease_id) DO UPDATE SET
                   expires_at     = excluded.expires_at,
                   holder_node_id = excluded.holder_node_id,
                   state          = 'renewed'"#,
            turso::params![
                row.lease_id.as_str(),
                row.lease_id.as_str(),
                row.scope_key.as_str(),
                row.holder_node_id.as_str(),
                now_ms,
                row.expires_unix_ms as i64,
            ],
        )
        .await
        .map_err(|e| {
            store_span!(
                "put_exec_lease",
                t.elapsed().as_millis(),
                err = e.to_string().as_str()
            );
            turso_err(e)
        })?;
        store_span!("put_exec_lease", t.elapsed().as_millis());
        Ok(())
    }

    async fn list_exec_leases(&self) -> Result<Vec<RemoteExecLeaseRow>, MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let mut rows = conn
            .query(
                "SELECT lease_id, scope_key, holder_node_id, expires_at
                 FROM mesh_exec_leases
                 WHERE state NOT IN ('revoked','completed')
                 ORDER BY granted_at ASC",
                (),
            )
            .await
            .map_err(turso_err)?;

        let mut result = Vec::new();
        while let Some(row) = rows.next().await.map_err(turso_err)? {
            result.push(lease_from_row(&row)?);
        }
        store_span!(
            "list_exec_leases",
            t.elapsed().as_millis(),
            rows = result.len()
        );
        Ok(result)
    }

    async fn revoke_exec_lease(&self, lease_id: &str) -> Result<(), MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        conn.execute(
            "UPDATE mesh_exec_leases SET state = 'revoked' WHERE lease_id = ?1",
            [lease_id],
        )
        .await
        .map_err(|e| {
            store_span!(
                "revoke_exec_lease",
                t.elapsed().as_millis(),
                err = e.to_string().as_str()
            );
            turso_err(e)
        })?;
        store_span!("revoke_exec_lease", t.elapsed().as_millis());
        Ok(())
    }

    async fn delete_exec_lease(&self, lease_id: &str) -> Result<(), MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        conn.execute(
            "DELETE FROM mesh_exec_leases WHERE lease_id = ?1",
            [lease_id],
        )
        .await
        .map_err(|e| {
            store_span!(
                "delete_exec_lease",
                t.elapsed().as_millis(),
                err = e.to_string().as_str()
            );
            turso_err(e)
        })?;
        store_span!("delete_exec_lease", t.elapsed().as_millis());
        Ok(())
    }

    // ── dispatch results ───────────────────────────────────────────

    async fn put_dispatch_result(
        &self,
        key: &str,
        value: &DispatchResponse,
    ) -> Result<(), MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let json = serde_json::to_string(value)
            .map_err(|e| MeshStoreError::Serialization(e.to_string()))?;
        let now_ms = crate::now_ms() as i64;
        conn.execute(
            r#"INSERT INTO mesh_dispatch_results (key, value_json, created_at)
               VALUES (?1, ?2, ?3)
               ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json"#,
            turso::params![key, json.as_str(), now_ms],
        )
        .await
        .map_err(|e| {
            store_span!(
                "put_dispatch_result",
                t.elapsed().as_millis(),
                err = e.to_string().as_str()
            );
            turso_err(e)
        })?;
        store_span!("put_dispatch_result", t.elapsed().as_millis());
        Ok(())
    }

    async fn get_dispatch_result(
        &self,
        key: &str,
    ) -> Result<Option<DispatchResponse>, MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let mut rows = conn
            .query(
                "SELECT value_json FROM mesh_dispatch_results WHERE key = ?1",
                [key],
            )
            .await
            .map_err(turso_err)?;

        let result = if let Some(row) = rows.next().await.map_err(turso_err)? {
            let json: String = row.get(0).map_err(|e| MeshStoreError::Io(e.to_string()))?;
            Some(
                serde_json::from_str(&json)
                    .map_err(|e| MeshStoreError::Serialization(e.to_string()))?,
            )
        } else {
            None
        };
        store_span!("get_dispatch_result", t.elapsed().as_millis());
        Ok(result)
    }

    async fn load_all_dispatch_results(
        &self,
    ) -> Result<HashMap<String, DispatchResponse>, MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let mut rows = conn
            .query("SELECT key, value_json FROM mesh_dispatch_results", ())
            .await
            .map_err(turso_err)?;

        let mut map = HashMap::new();
        while let Some(row) = rows.next().await.map_err(turso_err)? {
            let k: String = row.get(0).map_err(|e| MeshStoreError::Io(e.to_string()))?;
            let json: String = row.get(1).map_err(|e| MeshStoreError::Io(e.to_string()))?;
            let v: DispatchResponse = serde_json::from_str(&json)
                .map_err(|e| MeshStoreError::Serialization(e.to_string()))?;
            map.insert(k, v);
        }
        store_span!(
            "load_all_dispatch_results",
            t.elapsed().as_millis(),
            rows = map.len()
        );
        Ok(map)
    }

    // ── meta ───────────────────────────────────────────────────────

    fn schema_version(&self) -> u32 {
        1
    }

    async fn integrity_check(&self) -> Result<IntegrityReport, MeshStoreError> {
        let t = Instant::now();
        let conn = self.db.connection();
        let mut findings = Vec::new();

        // 1. Built-in SQLite integrity check.
        let mut rows = conn
            .query("PRAGMA integrity_check", ())
            .await
            .map_err(turso_err)?;
        while let Some(row) = rows.next().await.map_err(turso_err)? {
            let s: String = row.get(0).map_err(|e| MeshStoreError::Io(e.to_string()))?;
            if s != "ok" {
                findings.push(IntegrityFinding {
                    code: "sqlite_integrity".into(),
                    detail: s,
                });
            }
        }

        // 2. Dedupe uniqueness: at most one unacked row per non-null idempotency_dedupe_key.
        let mut rows2 = conn
            .query(
                r#"SELECT idempotency_dedupe_key, COUNT(*) AS cnt
                   FROM mesh_a2a_messages
                   WHERE idempotency_dedupe_key IS NOT NULL AND acknowledged = 0
                   GROUP BY idempotency_dedupe_key
                   HAVING cnt > 1"#,
                (),
            )
            .await
            .map_err(turso_err)?;
        while let Some(row) = rows2.next().await.map_err(turso_err)? {
            let key: String = row.get(0).map_err(|e| MeshStoreError::Io(e.to_string()))?;
            let cnt: i64 = row.get(1).map_err(|e| MeshStoreError::Io(e.to_string()))?;
            findings.push(IntegrityFinding {
                code: "dedupe_violation".into(),
                detail: format!(
                    "idempotency_dedupe_key '{key}' has {cnt} unacked rows (expected ≤1)"
                ),
            });
        }

        store_span!("integrity_check", t.elapsed().as_millis());
        Ok(IntegrityReport {
            ok: findings.is_empty(),
            findings,
            schema_version: 1,
        })
    }
}
