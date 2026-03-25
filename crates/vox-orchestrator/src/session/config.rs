//! Session manager configuration.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the session manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Directory where JSONL session files are stored. Default: `.sessions/`.
    pub sessions_dir: PathBuf,
    /// Optional stable repo id (e.g. MCP embeds this in session paths / payloads).
    #[serde(default)]
    pub repository_id: Option<String>,
    /// Seconds of inactivity before a session is considered idle. Default: 1800 (30 min).
    pub idle_timeout_secs: u64,
    /// Seconds of idle before archiving. Default: 86_400 (24 h).
    pub archive_timeout_secs: u64,
    /// Maximum number of active sessions. Default: 16.
    pub max_sessions: usize,
    /// Whether to enable JSONL persistence. Default: true.
    pub persist: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            sessions_dir: PathBuf::from(vox_config::MCP_SESSIONS_DIR_BASENAME),
            repository_id: None,
            idle_timeout_secs: 1_800,
            archive_timeout_secs: 86_400,
            max_sessions: 16,
            persist: true,
        }
    }
}
