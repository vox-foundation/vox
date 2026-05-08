//! Container isolation primitives for Vox actors.
//!
//! Provides types for Linux namespaces, cgroups, and Windows Job Objects.

use serde::{Deserialize, Serialize};

/// Container configuration for an actor process.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerConfig {
    pub cpu_limit: Option<f64>,
    pub memory_limit_bytes: Option<u64>,
    pub read_only_root: bool,
    pub bind_mounts: Vec<String>,
}

/// Statistics for a running container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStats {
    pub cpu_usage_ns: u64,
    pub memory_usage_bytes: u64,
    pub restart_count: u32,
}
