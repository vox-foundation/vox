//! Read-only view of a remote mens HTTP control plane (orchestrator / MCP federation).
//!
//! Populated by embedders (e.g. `vox-mcp` background poll). Does **not** perform HTTP itself.

use serde::{Deserialize, Serialize};
use vox_repository::TaskCapabilityHints;

/// Capability hints copied from a remote mens `NodeRecord` JSON for experimental routing logs.
///
/// Routing remains **in-process only**; this struct informs `RoutingService` when
/// `OrchestratorConfig::populi_routing_experimental` is enabled.
#[derive(Debug, Clone, PartialEq)]
pub struct RemotePopuliRoutingHint {
    /// [`vox_populi::NodeRecord::id`].
    pub node_id: String,
    /// Full remote capability snapshot copied from `NodeRecord.capabilities`.
    pub capabilities: TaskCapabilityHints,
    /// Labels from the remote node's [`vox_orchestrator::TaskCapabilityHints::labels`].
    pub labels: Vec<String>,
    /// Remote node advertises CUDA.
    pub gpu_cuda: bool,
    /// Remote node advertises Metal.
    pub gpu_metal: bool,
    /// Remote node advertises minimum VRAM (MiB), if known.
    pub min_vram_mb: Option<u32>,
    /// Labels related to training workloads (`workload=mens-train`, pool tags, etc.).
    pub training_labels: Vec<String>,
}

/// One line per remote node for status payloads (no full capability struct).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PopuliNodeBrief {
    /// Node id from control plane JSON.
    pub id: String,
    /// Epoch ms from `NodeRecord::last_seen_unix_ms`.
    pub last_seen_unix_ms: u64,
}

/// Cached result of `GET /v1/populi/nodes` (read-only federation).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemotePopuliSnapshot {
    /// Wall clock when the snapshot was taken (Unix ms).
    pub fetched_at_unix_ms: u64,
    /// Whether the last fetch parsed successfully.
    pub ok: bool,
    /// Number of nodes in the registry file.
    pub node_count: usize,
    /// `PopuliRegistryFile::schema_version` when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<u32>,
    /// Error message when `ok` is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Short node list for dashboards.
    #[serde(default)]
    pub nodes_brief: Vec<PopuliNodeBrief>,
}

impl RemotePopuliSnapshot {
    /// Successful parse.
    #[must_use]
    pub fn success(
        fetched_at_unix_ms: u64,
        schema_version: u32,
        nodes_brief: Vec<PopuliNodeBrief>,
    ) -> Self {
        let node_count = nodes_brief.len();
        Self {
            fetched_at_unix_ms,
            ok: true,
            node_count,
            schema_version: Some(schema_version),
            error: None,
            nodes_brief,
        }
    }

    /// Failed fetch (HTTP / JSON).
    #[must_use]
    pub fn failure(fetched_at_unix_ms: u64, message: impl Into<String>) -> Self {
        Self {
            fetched_at_unix_ms,
            ok: false,
            node_count: 0,
            schema_version: None,
            error: Some(message.into()),
            nodes_brief: Vec::new(),
        }
    }
}
