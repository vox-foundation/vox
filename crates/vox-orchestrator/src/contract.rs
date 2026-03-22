//! Canonical orchestration contract (v2-compatible payloads) for MCP, CLI, and DeI surfaces.
//!
//! Use these types when crossing subsystem boundaries so task requirements, sessions, and
//! migration flags stay aligned. See also `PLAN_TOOL_DAEMON_ALIGNMENT`.

use serde::{Deserialize, Serialize};

#[cfg(feature = "json-schema")]
use schemars::JsonSchema;

/// Version label for orchestration wire format (bump when breaking compatibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationContractVersion {
    /// Legacy task/session shapes (default).
    #[default]
    V1,
    /// Opt-in unified contract (adapters may translate to V1).
    V2,
}

/// Hardware hints for a **task** requirement or an **agent** queue capability profile.
///
/// **CPU-first mesh:** `cpu_cores`, `arch`, `hostname`, and `labels` describe the host; GPU / NPU
/// fields remain optional extensions. Deserialization fills missing fields from defaults so older
/// JSON/TOML remains valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
pub struct TaskCapabilityHints {
    /// Task requires CUDA-capable execution; agent provides CUDA when true.
    #[serde(default)]
    pub gpu_cuda: bool,
    /// Task requires Metal; agent provides Metal when true.
    #[serde(default)]
    pub gpu_metal: bool,
    /// Agent advertises Vulkan-class GPU (typical Android / Linux).
    #[serde(default)]
    pub gpu_vulkan: bool,
    /// Agent advertises WebGPU-capable browser or host (soft; policy may disable WebGPU).
    #[serde(default)]
    pub gpu_webgpu: bool,
    /// Agent advertises an on-device NPU / neural accelerator.
    #[serde(default)]
    pub npu: bool,
    /// Optional host class label (`server`, `desktop`, `mobile`, `browser`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,
    /// Minimum VRAM in MiB when GPU is required (soft hint for routing).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_vram_mb: Option<u32>,
    /// Logical CPU count observed on the host (or operator override via config).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    /// Target architecture string (e.g. `x86_64`, `aarch64`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    /// Host name when known (mesh / placement visibility).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    /// Optional scheduler labels (mesh, region, pool, …).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    /// Task requires at least this many logical cores (soft routing penalty when unmet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_cpu_cores: Option<u32>,
    /// Soft routing hint: deprioritize agents without any GPU capability (Populi-style training intent).
    #[serde(default)]
    pub prefer_gpu_compute: bool,
}

/// Session envelope for tools and Codex dual-write (repository-scoped).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionContractEnvelope {
    /// Stable repository id from `vox-repository`.
    pub repository_id: String,
    /// Opaque session id (JSONL or Codex).
    pub session_id: String,
    /// Optional MCP / IDE metadata (JSON-serializable).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Staged migration toggles (defaults preserve legacy behavior).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrchestrationMigrationFlags {
    /// When true, prefer v2 contract paths where implemented.
    #[serde(default)]
    pub orchestration_v2_enabled: bool,
    /// When true, allow falling back to legacy handlers if v2 fails.
    #[serde(default = "default_true")]
    pub legacy_orchestration_fallback: bool,
}

fn default_true() -> bool {
    true
}

impl Default for OrchestrationMigrationFlags {
    fn default() -> Self {
        Self {
            orchestration_v2_enabled: false,
            legacy_orchestration_fallback: true,
        }
    }
}

/// MCP planning tool names aligned index-wise with DeI `ai.plan.*` methods (new / replan / status).
pub const MCP_PLAN_TOOL_NAMES: &[&str] = &["vox_plan", "vox_replan", "vox_plan_status"];

/// DeI daemon JSON-RPC methods for the same planning operations (execute is separate).
pub const DEI_PLAN_METHODS_NEW_REPLAN_STATUS: &[&str] =
    &["ai.plan.new", "ai.plan.replan", "ai.plan.status"];

/// Documented mapping: `MCP_PLAN_TOOL_NAMES[i]` ↔ `DEI_PLAN_METHODS_NEW_REPLAN_STATUS[i]`.
#[must_use]
pub fn plan_tool_daemon_alignment_valid() -> bool {
    MCP_PLAN_TOOL_NAMES.len() == DEI_PLAN_METHODS_NEW_REPLAN_STATUS.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_alignment_len_matches() {
        assert!(plan_tool_daemon_alignment_valid());
        assert_eq!(MCP_PLAN_TOOL_NAMES.len(), 3);
    }

    #[test]
    fn task_capability_hints_deserialize_omitted_fields() {
        let j = r#"{"gpu_cuda":true}"#;
        let h: TaskCapabilityHints = serde_json::from_str(j).unwrap();
        assert!(h.gpu_cuda);
        assert!(!h.gpu_metal);
        assert!(!h.gpu_vulkan);
        assert!(!h.gpu_webgpu);
        assert!(!h.npu);
        assert!(h.device_class.is_none());
        assert!(h.cpu_cores.is_none());
        assert!(h.labels.is_empty());
    }
}
