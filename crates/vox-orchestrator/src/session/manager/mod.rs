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
/// # Persistence Architecture
///
/// `agent_session_events` are the **durable SSOT**.
///
/// # Thread Safety
pub struct SessionManager {
    pub(super) config: SessionConfig,
    pub(super) sessions: HashMap<String, super::state::Session>,
    /// Optional VoxDB backing store for SSOT persistence.
    pub(super) db: Option<Arc<vox_db::VoxDb>>,
}
