//! VCS-style snapshot, oplog, undo/redo, and database snapshot operations.
//!
//! Provides a JJ-inspired durable versioning surface on top of `SnapshotStore`,
//! `OpLog`, and the optional Codex `db_snapshots` table.

use std::path::PathBuf;
use std::time::Duration;

use crate::oplog::{OperationId, OperationKind};
use crate::orchestrator::OrchestratorError;
use crate::snapshot::SnapshotId;
use crate::types::AgentId;

/// Wall-clock bound for Turso / sqlite snapshot and CAS writes so hung DB layers
/// cannot block the orchestrator forever inside an async poll.
const DB_IO_TIMEOUT: Duration = Duration::from_secs(60);

impl crate::orchestrator::Orchestrator {
    /// Take a filesystem snapshot of `paths` and optionally persist their bytes to
    /// the CAS (`VoxDb::store`). Returns the new [`SnapshotId`].
    pub async fn capture_snapshot(
        &self,
        agent_id: AgentId,
        paths: &[PathBuf],
        description: impl Into<String>,
    ) -> SnapshotId {
        let desc = description.into();
        // Fast path: no paths to snapshot (zero-cost in tests and read-only routes).
        if paths.is_empty() {
            return crate::sync_lock::rw_write(&*self.snapshot_store).take_snapshot(
                agent_id,
                &[],
                &desc,
            );
        }

        // Unit tests (`cargo test -p vox-orchestrator`) compile this crate with `cfg(test)`.
        // Avoid real filesystem reads there: they can stall on CI/Windows (AV, locks) and are
        // not needed to validate routing/queue behavior. Integration tests link the library
        // without `cfg(test)` and still exercise real `std::fs::read` paths.
        #[cfg(test)]
        {
            // Synthetic snapshot only: keeps unit tests hermetic and fast (no `std::fs::read`).
            let prefetched: Vec<(PathBuf, Option<Vec<u8>>)> =
                paths.iter().map(|p| (p.clone(), None)).collect();
            let mut store = crate::sync_lock::rw_write(&*self.snapshot_store);
            return store.take_snapshot_prefetched(agent_id, &prefetched, desc);
        }

        #[cfg(not(test))]
        {
            // Prefetch outside the `snapshot_store` write guard.
            let prefetched: Vec<(PathBuf, Option<Vec<u8>>)> = paths
                .iter()
                .map(|p| match std::fs::read(p) {
                    Ok(data) => (p.clone(), Some(data)),
                    Err(_) => (p.clone(), None),
                })
                .collect();

            let snap_id = {
                let mut store = crate::sync_lock::rw_write(&*self.snapshot_store);
                store.take_snapshot_prefetched(agent_id, &prefetched, &desc)
            };

            let db_opt = crate::sync_lock::rw_read(&*self.db).clone();
            if let Some(db) = db_opt {
                for (_path, maybe_data) in &prefetched {
                    if let Some(data) = maybe_data {
                        match tokio::time::timeout(DB_IO_TIMEOUT, db.store("file", data.as_slice()))
                            .await
                        {
                            Ok(Ok(_)) | Ok(Err(_)) => {}
                            Err(_) => tracing::warn!("capture_snapshot: db.store timed out"),
                        }
                    }
                }
            }

            snap_id
        }
    }

    /// Record a generic operation in the oplog, capturing a pre-op DB snapshot when
    /// `db_snapshot_before` is `None` and a VoxDb is attached.
    pub async fn record_operation(
        &self,
        agent_id: AgentId,
        kind: OperationKind,
        description: impl Into<String>,
        snapshot_before: Option<SnapshotId>,
        snapshot_after: Option<SnapshotId>,
        db_snapshot_before: Option<u64>,
        db_snapshot_after: Option<u64>,
    ) -> OperationId {
        let desc = description.into();
        let db_snap_before = match db_snapshot_before {
            Some(id) => Some(id),
            None => {
                self.take_db_snapshot(agent_id, format!("pre-op: {}", desc))
                    .await
            }
        };

        crate::sync_lock::rw_write(&*self.oplog).record(
            agent_id,
            kind,
            desc,
            snapshot_before,
            snapshot_after,
            db_snap_before,
            db_snapshot_after,
            None,
            None,
        )
    }

    /// Take a lightweight schema-level snapshot of the Codex database state.
    ///
    /// Returns the `snap_id` on success, or `None` if no DB is attached or the write fails.
    pub async fn take_db_snapshot(
        &self,
        agent_id: AgentId,
        description: impl Into<String>,
    ) -> Option<u64> {
        let db_opt = crate::sync_lock::rw_read(&*self.db).clone();
        if let Some(db) = db_opt {
            let snap_id = crate::sync_lock::rw_write(&*self.oplog).next_db_snapshot_id();
            let desc = description.into();
            match tokio::time::timeout(
                DB_IO_TIMEOUT,
                db.take_db_snapshot(snap_id, &agent_id.to_string(), &desc),
            )
            .await
            {
                Ok(Ok(())) => return Some(snap_id),
                Ok(Err(_)) => {}
                Err(_) => tracing::warn!("take_db_snapshot: timed out after {:?}", DB_IO_TIMEOUT),
            }
        }
        None
    }

    /// Undo the operation identified by `op_id`: restores the DB snapshot then the FS snapshot.
    pub async fn undo_operation(&self, op_id: OperationId) -> Result<(), OrchestratorError> {
        let (fs_snap, db_snap) = crate::sync_lock::rw_write(&*self.oplog)
            .undo(op_id)
            .ok_or(OrchestratorError::OperationNotFound)?;

        if let Some(db_id) = db_snap {
            let db_opt = crate::sync_lock::rw_read(&self.db).clone();
            if let Some(db) = db_opt {
                db.restore_db_snapshot(db_id).await.map_err(|e| {
                    OrchestratorError::DatabaseError(format!("Undo: DB restore failed: {}", e))
                })?;
            }
        }

        if let Some(fs_id) = fs_snap {
            self.restore_fs_snapshot(fs_id).await?;
        }

        self.event_bus
            .emit(crate::events::AgentEventKind::OperationUndone {
                agent_id: crate::types::AgentId(0),
                operation_id: op_id.to_string(),
            });

        Ok(())
    }

    /// Re-apply the state after a previously undone operation.
    pub async fn redo_operation(&self, op_id: OperationId) -> Result<(), OrchestratorError> {
        let (fs_snap, db_snap) = crate::sync_lock::rw_write(&*self.oplog)
            .redo(op_id)
            .ok_or(OrchestratorError::OperationNotFound)?;

        if let Some(db_id) = db_snap {
            let db_opt = crate::sync_lock::rw_read(&self.db).clone();
            if let Some(db) = db_opt {
                db.restore_db_snapshot(db_id).await.map_err(|e| {
                    OrchestratorError::DatabaseError(format!("Redo: DB restore failed: {}", e))
                })?;
            }
        }

        if let Some(fs_id) = fs_snap {
            self.restore_fs_snapshot(fs_id).await?;
        }

        self.event_bus
            .emit(crate::events::AgentEventKind::OperationRedone {
                agent_id: crate::types::AgentId(0),
                operation_id: op_id.to_string(),
            });

        Ok(())
    }

    /// Internal helper: restore files from a snapshot by reading their CAS blobs.
    pub async fn restore_fs_snapshot(
        &self,
        snapshot_id: SnapshotId,
    ) -> Result<(), OrchestratorError> {
        let snap = {
            let snap_store = crate::sync_lock::rw_read(&*self.snapshot_store);
            snap_store
                .get(snapshot_id)
                .ok_or(OrchestratorError::OperationNotFound)?
                .clone()
        };

        let db_opt = crate::sync_lock::rw_read(&self.db).clone();
        let db = db_opt.ok_or_else(|| {
            OrchestratorError::DatabaseError("Database not initialized for restore".into())
        })?;

        for entry in snap.files.values() {
            if entry.content_hash.is_empty() {
                if entry.path.exists() {
                    let _ = std::fs::remove_file(&entry.path);
                }
            } else {
                let data = db.get(&entry.content_hash).await.map_err(|e| {
                    OrchestratorError::DatabaseError(format!(
                        "Restore: object {} missing: {}",
                        entry.content_hash, e
                    ))
                })?;
                if let Some(parent) = entry.path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&entry.path, data).map_err(|e| {
                    OrchestratorError::DatabaseError(format!(
                        "Restore: write {} failed: {}",
                        entry.path.display(),
                        e
                    ))
                })?;
            }
        }
        Ok(())
    }
}
