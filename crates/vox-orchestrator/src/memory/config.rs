use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Configuration for the persistent memory system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Directory for daily log files. Default: `./memory`.
    pub log_dir: PathBuf,
    /// Path to the long-term memory file. Default: `./memory/MEMORY.md`.
    pub memory_md_path: PathBuf,
    /// Maximum number of days to retain daily logs. 0 = keep forever.
    pub log_retention_days: u64,
    /// Whether the memory system is enabled. Default: true.
    pub enabled: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("memory"),
            memory_md_path: PathBuf::from("memory/MEMORY.md"),
            log_retention_days: 30,
            enabled: true,
        }
    }
}
