//! Session manager: persistence and lifecycle.

use std::collections::HashMap;
use std::sync::Arc;

use super::config::SessionConfig;

mod db_io;
mod lifecycle;
mod mutations;
mod persist_load;
#[cfg(test)]
mod tests;

/// Manages agent sessions: creation, persistence, lifecycle, cleanup.
///
/// When a `VoxDb` is attached via [`SessionManager::with_db`], session rows and
/// `agent_session_events` are the **durable SSOT**. JSONL under [`SessionConfig::sessions_dir`]
/// is an optional, non-authoritative export for debugging or tooling when `persist` is enabled.
pub struct SessionManager {
    pub(super) config: SessionConfig,
    pub(super) sessions: HashMap<String, super::state::Session>,
    /// Optional VoxDB backing store for SSOT persistence.
    pub(super) db: Option<Arc<vox_db::VoxDb>>,
}
