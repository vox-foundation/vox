//! [`NodeRecord`] and on-disk registry file layout for Populi.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use vox_repository::TaskCapabilityHints;

use crate::now_ms;

/// One participant in the populi view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeRecord {
    /// Stable node id (operator- or env-assigned).
    pub id: String,
    /// Host capabilities (CPU + optional GPU hints).
    pub capabilities: TaskCapabilityHints,
    /// Optional listen address for control or data plane (phase 3+).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listen_addr: Option<String>,
    /// `CARGO_PKG_VERSION` of `vox-populi` / embedding crate at registration time.
    pub version: String,
    /// Wall-clock last update (epoch ms).
    pub last_seen_unix_ms: u64,
    /// Populi tenancy / cluster id; must match server [`crate::transport::PopuliTransportState::required_scope`] when the server enforces scope.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    /// Worker visibility for scheduling policy (`private` or `public` when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    /// Logical pool id (`pool=…` mesh label normalization).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<String>,
    /// Trust tier for public mesh policy (`new`, `probation`, `trusted`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_tier: Option<String>,
    /// Declared workload classes (`infer`, `train`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workload_classes: Option<Vec<String>>,
    /// Privacy class advertised by this node (`public_ok`, `trusted_only`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_class: Option<String>,
    /// Advertised models loaded into VRAM on this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_llm_models: Option<Vec<String>>,
    /// When true, scheduler should not place new work here (drain-only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance: Option<bool>,
    /// When set and `maintenance` is true, maintenance is treated as cleared at this Unix ms (lazy sweep + gate checks).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maintenance_until_unix_ms: Option<u64>,
    /// Optional cloud / bridge provider tag (`runpod`, `vast`, …) for hybrid workers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    /// Total number of GPU devices visible on this node (when probed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_total_count: Option<u32>,
    /// Number of currently healthy GPUs on this node (when probed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_healthy_count: Option<u32>,
    /// Number of currently allocatable GPUs after local reservations (Layer B).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_allocatable_count: Option<u32>,
    /// Source of GPU inventory values (`probed`, `advertised`, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_inventory_source: Option<String>,
    /// Truth-layer marker (`layer_a_verified`, `layer_b_allocatable`, `layer_c_advertised`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_truth_layer: Option<String>,
    /// NVIDIA kernel driver version (NVML `sys_driver_version`), when probe-backed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nvidia_driver_version: Option<String>,
    /// CUDA driver version (`major.minor` from NVML), when probe-backed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cuda_driver_version: Option<String>,
    /// Worker-reported GPU readiness for scheduling (NVML probe or pilot self-check).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_readiness_ok: Option<bool>,
    /// Short machine-readable reason when [`Self::gpu_readiness_ok`] is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_readiness_reason: Option<String>,
    /// Unix ms when readiness was last evaluated on the worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_readiness_checked_unix_ms: Option<u64>,
    /// When true, server rejects new A2A claims for this node (set via admin quarantine API only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quarantined: Option<bool>,
    /// Host architecture triple (e.g. `x86_64-pc-windows-msvc`) for cross-compilation (Wave 4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_triple: Option<String>,
    /// Real-time CPU usage percentage (0.0 - 100.0) for Wave 5 load balancing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_usage_pct: Option<f32>,
    /// Available system memory in bytes for Wave 5 resource-aware scheduling.
    pub memory_free_bytes: Option<u64>,
    /// The user ID of the node owner (assigned securely from the join token).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_vox_user_id: Option<String>,
    /// Models advertised by this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advertised_models: Option<Vec<vox_mesh_types::ModelAdvertisement>>,
    /// Donation policy for GPU compute.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub donation_policy: Option<vox_mesh_types::WorkerDonationPolicy>,
    /// Ed25519 public key used to verify attestation signatures.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ed25519_pub_key_b64: Option<String>,
    /// Names of hardware probes that returned an error during the last probe pipeline run.
    /// `None` means no failures occurred, or the summary was not produced by a pipeline run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probe_failures: Option<Vec<String>>,
}

/// Serializable registry file (`.vox/cache/populi/local-registry.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PopuliRegistryFile {
    /// Schema version for forward compatibility.
    pub schema_version: u32,
    /// Known nodes (typically one for local-only mode).
    pub nodes: Vec<NodeRecord>,
    /// Wave 5: Global pending job count across all receiver agent inboxes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_depth: Option<usize>,
}

/// Drop nodes whose `last_seen_unix_ms` is older than `now - max_stale_ms` when `max_stale_ms` is Some and > 0.
#[must_use]
pub fn filter_registry_by_max_stale_ms(
    mut file: PopuliRegistryFile,
    max_stale_ms: Option<u64>,
) -> PopuliRegistryFile {
    let Some(threshold) = max_stale_ms.filter(|n| *n > 0) else {
        return file;
    };
    let now = now_ms();
    file.nodes
        .retain(|n| now.saturating_sub(n.last_seen_unix_ms) <= threshold);
    file
}

/// Upper bound for operator `maintenance_for_ms` (`POST /v1/populi/admin/maintenance`).
pub const MAX_MAINTENANCE_FOR_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Whether this node should block **new** claims / exec lease grant+renew (drain semantics).
#[must_use]
pub fn node_maintenance_blocks_new_work(now_ms: u64, n: &NodeRecord) -> bool {
    if n.maintenance != Some(true) {
        return false;
    }
    if let Some(until) = n.maintenance_until_unix_ms
        && now_ms >= until
    {
        return false;
    }
    true
}

/// Clear [`NodeRecord::maintenance`] / deadline when the deadline has passed (mutates in place).
pub fn sweep_expired_maintenance_on_nodes(nodes: &mut [NodeRecord], now_ms: u64) {
    for n in nodes.iter_mut() {
        if n.maintenance == Some(true) && n.maintenance_until_unix_ms.is_some_and(|u| now_ms >= u) {
            n.maintenance = None;
            n.maintenance_until_unix_ms = None;
        }
    }
}

/// Local file-backed registry (single-writer; suitable for shared Docker volume in dev).
#[derive(Debug)]
pub struct LocalRegistry {
    path: PathBuf,
}

impl LocalRegistry {
    /// Default path under the user home: `~/.vox/cache/populi/local-registry.json`.
    #[must_use]
    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        base.join(".vox")
            .join("cache")
            .join("populi")
            .join("local-registry.json")
    }

    /// Open registry at `path` (file may not exist yet).
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Prefer `VOX_MESH_REGISTRY_PATH`, else [`LocalRegistry::default_path`].
    #[must_use]
    pub fn resolved_default_path() -> PathBuf {
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshRegistryPath)
            .expose()
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(Self::default_path)
    }

    /// Path on disk.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load or return empty registry.
    pub fn load(&self) -> Result<PopuliRegistryFile, PopuliRegistryError> {
        if !self.path.is_file() {
            return Ok(PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
                queue_depth: None,
            });
        }
        let raw = std::fs::read_to_string(&self.path).map_err(PopuliRegistryError::Io)?;
        let parsed: PopuliRegistryFile =
            serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
        Ok(parsed)
    }

    /// Replace registry contents atomically (write temp + rename).
    pub fn save(&self, reg: &PopuliRegistryFile) -> Result<(), PopuliRegistryError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
        }
        let json = serde_json::to_string_pretty(reg)
            .map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes()).map_err(PopuliRegistryError::Io)?;
        std::fs::rename(&tmp, &self.path).map_err(PopuliRegistryError::Io)?;
        Ok(())
    }

    /// Upsert a node by `id` and persist.
    pub fn upsert_node(&self, mut record: NodeRecord) -> Result<(), PopuliRegistryError> {
        record.last_seen_unix_ms = now_ms();
        let mut reg = self.load()?;
        reg.schema_version = 1;
        if let Some(i) = reg.nodes.iter().position(|n| n.id == record.id) {
            reg.nodes[i] = record;
        } else {
            reg.nodes.push(record);
        }
        self.save(&reg)
    }
}

/// Registry I/O errors.
#[derive(Debug, thiserror::Error)]
pub enum PopuliRegistryError {
    /// Filesystem error.
    #[error("populi registry I/O: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parse/serialize error.
    #[error("populi registry JSON: {0}")]
    Json(String),
    /// HTTP control plane error.
    #[error("populi HTTP: {0}")]
    Http(String),
    /// HTTP control plane status error with structured code/context.
    #[error("populi HTTP {status} ({context}){body_suffix}")]
    HttpStatus {
        /// HTTP status code (`404`, `409`, ...).
        status: u16,
        /// Short operation context (`exec_lease_renew`, `a2a_inbox`, ...).
        context: String,
        /// Optional response body snippet.
        body_suffix: String,
    },
}

impl PopuliRegistryError {
    /// Status code when this error came from an HTTP status failure.
    #[must_use]
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Convenience predicate for status-code branching.
    #[must_use]
    pub fn is_http_status(&self, code: u16) -> bool {
        self.status_code() == Some(code)
    }
}
