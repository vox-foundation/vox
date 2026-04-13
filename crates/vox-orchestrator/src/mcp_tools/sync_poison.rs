//! RwLock poison handling for MCP tool surfaces (shared with `dei_tools::orchestrator_snapshot`).
//!
//! `PoisonError<RwLock*Guard<_>>` does not satisfy `anyhow::Context`’s `Send + Sync` error bounds,
//! so we map to `anyhow::Error` with an explicit message (same spirit as `orchestrator_snapshot`).

use std::sync::{LockResult, RwLockReadGuard, RwLockWriteGuard};

pub fn poison_rw_read<'a, T>(
    res: LockResult<RwLockReadGuard<'a, T>>,
    context: &'static str,
) -> anyhow::Result<RwLockReadGuard<'a, T>> {
    res.map_err(|e| anyhow::anyhow!("{context}: {e}"))
}

pub fn poison_rw_write<'a, T>(
    res: LockResult<RwLockWriteGuard<'a, T>>,
    context: &'static str,
) -> anyhow::Result<RwLockWriteGuard<'a, T>> {
    res.map_err(|e| anyhow::anyhow!("{context}: {e}"))
}
