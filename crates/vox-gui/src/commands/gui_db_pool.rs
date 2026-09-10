//! Shared workspace database handle for GUI commands that would otherwise open
//! a fresh `VoxDb` connection per invoke (causing SQLITE_BUSY / os error 33).

use std::sync::Arc;

use std::path::Path;
#[cfg(test)]
use vox_db::DbConfig;
use vox_db::{
    DbConnectSurface, StoreError, VoxDb, connect_workspace_journey_optional,
    open_project_db_at_root,
};

#[derive(Clone)]
pub struct GuiDbPool {
    db: Option<Arc<VoxDb>>,
}

impl GuiDbPool {
    /// Best-effort workspace connect; pool may be empty when Turso is unavailable.
    pub async fn connect_workspace() -> Self {
        if let Ok(root) = std::env::var("VOX_GUI_DRIVE_STORE_ROOT")
            && !root.is_empty()
        {
            let root_path = Path::new(&root);
            let db = match open_project_db_at_root(root_path).await {
                Ok(db) => Some(Arc::new(db)),
                Err(StoreError::LegacySchemaChain { max_version }) => {
                    // Never wipe interactive/canonical DBs. Drive stores only,
                    // and only when explicitly opted in.
                    if !drive_store_reset_allowed() {
                        tracing::warn!(
                            max_version,
                            root = %root,
                            "Drive store legacy schema (max_version={max_version}); \
                             set VOX_GUI_DRIVE_ALLOW_STORE_RESET=1 to wipe and recreate \
                             under VOX_GUI_DRIVE_STORE_ROOT only"
                        );
                        None
                    } else {
                        wipe_drive_store(root_path);
                        open_project_db_at_root(root_path).await.ok().map(Arc::new)
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Drive store open failed");
                    None
                }
            };
            return Self { db };
        }
        let db = connect_workspace_journey_optional(DbConnectSurface::Runtime, true)
            .await
            .map(Arc::new);
        Self { db }
    }

    #[cfg(test)]
    pub async fn connect_memory() -> Result<Self, String> {
        let db = VoxDb::connect(DbConfig::Memory)
            .await
            .map_err(|e| e.to_string())?;
        Ok(Self {
            db: Some(Arc::new(db)),
        })
    }

    pub fn handle(&self) -> Result<Arc<VoxDb>, String> {
        self.db
            .clone()
            .ok_or_else(|| "workspace database unavailable".to_string())
    }
}

/// Wipe is allowed only for Axis Drive profiles that opted in. Requires both
/// the Drive store root (caller already gated) and the explicit reset flag —
/// never wipe on substring heuristics alone.
fn drive_store_reset_allowed() -> bool {
    std::env::var("VOX_GUI_DRIVE").ok().as_deref() == Some("1")
        && std::env::var("VOX_GUI_DRIVE_ALLOW_STORE_RESET")
            .ok()
            .as_deref()
            == Some("1")
}

fn wipe_drive_store(root_path: &Path) {
    let store = root_path.join(".vox").join("store.db");
    let _ = std::fs::remove_file(&store);
    let _ = std::fs::remove_file(store.with_extension("db-wal"));
    let _ = std::fs::remove_file(store.with_extension("db-shm"));
}

pub fn map_db_err(e: impl std::fmt::Display) -> String {
    let s = e.to_string();
    if s.contains("Locking error")
        || s.contains("SQLITE_BUSY")
        || s.contains("os error 33")
        || s.contains("database is locked")
    {
        "Database busy — another process is writing. Retry in a moment.".into()
    } else if s.contains("concurrent use forbidden") {
        // Defense-in-depth: the primary fix serializes access inside `vox_db::GuardedConnection`
        // so this should no longer be reachable from `VoxDb` methods. Keep this mapping in case
        // some other unguarded connection path still surfaces the raw Turso error, so a future
        // regression degrades to a retry hint instead of a scary raw Misuse string in the toast.
        "Database busy — another process is writing. Retry in a moment.".into()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pool_reuses_same_connection() {
        let pool = GuiDbPool::connect_memory().await.unwrap();
        let a = pool.handle().unwrap();
        let b = pool.handle().unwrap();
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn map_db_err_gives_friendly_message_for_concurrent_use_forbidden() {
        let raw = "Misuse(\"concurrent use forbidden\")";
        let mapped = map_db_err(raw);
        assert_eq!(
            mapped,
            "Database busy — another process is writing. Retry in a moment."
        );
        assert!(
            !mapped.contains("concurrent use forbidden"),
            "raw Turso Misuse string must not leak into the user-facing toast"
        );
    }

    #[test]
    fn drive_store_reset_requires_drive_and_explicit_flag() {
        // Isolate from ambient Drive env in agent shells.
        let prev_drive = std::env::var_os("VOX_GUI_DRIVE");
        let prev_reset = std::env::var_os("VOX_GUI_DRIVE_ALLOW_STORE_RESET");
        unsafe {
            std::env::remove_var("VOX_GUI_DRIVE");
            std::env::remove_var("VOX_GUI_DRIVE_ALLOW_STORE_RESET");
        }
        assert!(!drive_store_reset_allowed());
        unsafe {
            std::env::set_var("VOX_GUI_DRIVE", "1");
        }
        assert!(!drive_store_reset_allowed());
        unsafe {
            std::env::set_var("VOX_GUI_DRIVE_ALLOW_STORE_RESET", "1");
        }
        assert!(drive_store_reset_allowed());
        unsafe {
            match prev_drive {
                Some(v) => std::env::set_var("VOX_GUI_DRIVE", v),
                None => std::env::remove_var("VOX_GUI_DRIVE"),
            }
            match prev_reset {
                Some(v) => std::env::set_var("VOX_GUI_DRIVE_ALLOW_STORE_RESET", v),
                None => std::env::remove_var("VOX_GUI_DRIVE_ALLOW_STORE_RESET"),
            }
        }
    }
}
