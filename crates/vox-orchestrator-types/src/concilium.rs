//! Pure L0 wire and domain contracts for **Vox Concilium** (Causal Swimlanes, Shared `#seq` Board,
//! `0 tok/s` Parked Actors, Pilot Interventions) and **Vox Populi** (Hybrid Compute Placement &
//! `FileAffinityMap` Lock Matrix).
//!
//! Pure data (`serde` only): zero async runtime, zero DB, zero HTTP dependencies.

use serde::{Deserialize, Serialize};

/// Hybrid compute execution tier where a swarm worker is placed (`CONC-01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeTier {
    /// Cloud frontier model routed through `vox_actor_runtime::llm` / Thompson bandit.
    #[default]
    CloudBandit,
    /// Local host MENS Candle/CUDA/Metal GPU worker preserving local KV-cache prefixes.
    LocalMensGpu,
    /// Remote Populi QUIC (`iroh`) peer worker reached via `RemoteTaskEnvelope`.
    PopuliQuicPeer,
    /// Lightweight local CPU / deterministic Rust worker (`0` GPU/LLM spend).
    InlineCpu,
}

impl ComputeTier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CloudBandit => "cloud_bandit",
            Self::LocalMensGpu => "local_mens_gpu",
            Self::PopuliQuicPeer => "populi_quic_peer",
            Self::InlineCpu => "inline_cpu",
        }
    }
}

/// Workspace isolation mode for a swarm worker (`CONC-01`, `CONC-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmWorkspaceMode {
    /// Shares the parent workspace directly (readers, verifiers, disjoint file shards).
    #[default]
    Inherit,
    /// Independent working copy sharing repository storage (`jj share` / git worktree).
    Share,
    /// Full isolated branch/clone workspace.
    Branch,
}

impl SwarmWorkspaceMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Share => "share",
            Self::Branch => "branch",
        }
    }
}

/// Canonical causal metadata stamped across `AgentEvent`, `BulletinBoard`, `DbA2AMessage`,
/// and `RemoteTaskEnvelope` (`CONC-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CausalSwarmStamp {
    /// Monotonic `#seq` sequence number on the Shared Sequence Board (`0` when unstamped).
    pub swarm_seq: u64,
    /// Logical `#channel` tag (e.g. `#main`, `#parser`, `#verify`, `#scientia`, `#ops`).
    pub channel: String,
    /// W3C Trace Context `traceparent` (`00-<32hex>-<16hex>-01`).
    pub traceparent: String,
    /// 16-hex span identifier for this causal activity bar.
    pub span_id: String,
    /// Active compute placement tier.
    pub compute_tier: ComputeTier,
    /// Exclusive or shared `FileAffinityMap` path prefix held during this event, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_lock_prefix: Option<String>,
    /// VCS workspace isolation mode (`inherit | share | branch`).
    #[serde(default)]
    pub workspace_mode: SwarmWorkspaceMode,
    /// When the worker is parked (`0 tok/s`), the `#seq` watermark it is sleeping on.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked_since_seq: Option<u64>,
}

/// Span classification for Perfetto-style causal swimlanes (`CONC-06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConciliumSpanKind {
    Thinking,
    ToolCall,
    Writing,
    Parked,
    LockWait,
    Trip,
}

/// One millisecond-resolved span bar on a worker's causal swimlane (`CONC-06`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConciliumSpanBar {
    pub span_id: String,
    pub traceparent: String,
    pub kind: ConciliumSpanKind,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub start_offset_ms: u64,
    pub duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_lock_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub woken_by_seq: Option<u64>,
}

/// Golden `@mention` wakeup arc linking a `#seq` board post to the span it woke (`CONC-06`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConciliumWakeupArc {
    pub from_seq: u64,
    pub target_agent: String,
    pub target_span_id: String,
    pub mention: String,
}

/// Worker lifecycle state in Concilium & Populi (`CONC-02`, `CONC-06`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmWorkerStatus {
    Working,
    #[default]
    Waiting,
    Parked,
    Tripped,
    Idle,
}

/// One horizontal causal swimlane row in Mode 1: Vox Concilium (`CONC-06`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConciliumAgentLane {
    pub agent_id: String,
    pub roster_name: String,
    pub role: String,
    pub status: SwarmWorkerStatus,
    pub compute_tier: ComputeTier,
    pub node_id: String,
    pub model_id: String,
    pub workspace_mode: SwarmWorkspaceMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_lock_prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked_since_seq: Option<u64>,
    pub tok_per_sec: f64,
    pub used_tokens: u64,
    pub max_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kv_cache_hit_pct: Option<f64>,
    pub current_action: String,
    pub spans: Vec<ConciliumSpanBar>,
}

/// One append-only post on the Shared `#seq` Sequence Board (`CONC-02`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBoardEntry {
    pub seq: u64,
    pub channel: String,
    pub sender: String,
    pub body: String,
    pub mentions: Vec<String>,
    pub timestamp_ms: u64,
    pub traceparent: String,
    pub span_id: String,
    pub compute_tier: ComputeTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_lock_prefix: Option<String>,
    #[serde(default)]
    pub workspace_mode: SwarmWorkspaceMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parked_since_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_badge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_watchdog_alert: Option<bool>,
}

/// Cursor and filter query for `read_board` (`CONC-02`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceBoardFilter {
    #[serde(default)]
    pub since_seq: Option<u64>,
    #[serde(default)]
    pub seqs: Option<Vec<u64>>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Full Mode 1 snapshot served at `GET /api/v2/concilium/snapshot` (`CONC-03`, `CONC-06`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConciliumSnapshotDto {
    pub head_seq: u64,
    pub window_duration_ms: u64,
    pub active_workers: usize,
    pub parked_workers: usize,
    pub active_locks: usize,
    pub now_playing_ticker: String,
    pub lanes: Vec<ConciliumAgentLane>,
    pub wakeup_arcs: Vec<ConciliumWakeupArc>,
    pub board_tail: Vec<SequenceBoardEntry>,
}

/// Compact `@agent` pill docked onto a compute node card in Mode 2 (`CONC-07`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockedSwarmWorkerPill {
    pub agent_id: String,
    pub roster_name: String,
    pub status: SwarmWorkerStatus,
    pub tok_per_sec: f64,
    pub workspace_mode: SwarmWorkspaceMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_lock_prefix: Option<String>,
}

/// Node health status on Mode 2: Vox Populi Placement Map (`CONC-07`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeNodeStatus {
    #[default]
    Online,
    Maintenance,
    Quarantined,
    Degraded,
}

/// Hybrid compute node card (Local MENS GPU, Populi QUIC Peer, or Cloud Bandit Router) (`CONC-07`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridComputeNodeCardDto {
    pub node_id: String,
    pub display_name: String,
    pub tier: ComputeTier,
    pub status: ComputeNodeStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_triple: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gpu_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram_used_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vram_total_mb: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kv_cache_warm_prefixes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rtt_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bandit_weight: Option<f64>,
    #[serde(default)]
    pub advertised_models: Vec<String>,
    #[serde(default)]
    pub docked_workers: Vec<DockedSwarmWorkerPill>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_unix_ms: Option<u64>,
}

/// One row/cell in the `FileAffinityMap` Lock Matrix (`CONC-07`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileLockCellDto {
    pub prefix: String,
    pub holder_agent: String,
    pub exclusive: bool,
    pub compute_tier: ComputeTier,
    pub workspace_mode: SwarmWorkspaceMode,
    #[serde(default)]
    pub waiters: Vec<String>,
    pub contended: bool,
    pub acquired_seq: u64,
}

/// Full Mode 2 snapshot served at `GET /api/v2/concilium/placement` (`CONC-07`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PopuliPlacementDto {
    pub generated_at_ms: u64,
    pub head_seq: u64,
    pub gossip_cadence_ms: u64,
    pub source: String,
    pub queue_depth: usize,
    pub nodes: Vec<HybridComputeNodeCardDto>,
    pub lock_matrix: Vec<FileLockCellDto>,
}

/// Operator pilot intervention kinds (`CONC-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConciliumPilotActionKind {
    ParkAgent,
    SteerAgent,
    ForkAgent,
    SwapAgentCompute,
}

/// Input payload for `concilium_pilot_action` / `/api/v2/concilium/action` (`CONC-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConciliumPilotActionInput {
    pub action: ConciliumPilotActionKind,
    pub agent_id: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub directive: Option<String>,
    #[serde(default)]
    pub fork_name: Option<String>,
    #[serde(default)]
    pub workspace_mode: Option<SwarmWorkspaceMode>,
    #[serde(default)]
    pub target_tier: Option<ComputeTier>,
    #[serde(default)]
    pub target_node_id: Option<String>,
    #[serde(default)]
    pub target_model_id: Option<String>,
}

/// Outcome returned by a pilot steering action (`CONC-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConciliumPilotActionResult {
    pub ok: bool,
    pub seq: u64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forked_agent_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_tier_and_workspace_mode_labels_round_trip() {
        assert_eq!(ComputeTier::CloudBandit.as_str(), "cloud_bandit");
        assert_eq!(ComputeTier::LocalMensGpu.as_str(), "local_mens_gpu");
        assert_eq!(ComputeTier::PopuliQuicPeer.as_str(), "populi_quic_peer");
        assert_eq!(ComputeTier::InlineCpu.as_str(), "inline_cpu");
        assert_eq!(SwarmWorkspaceMode::Inherit.as_str(), "inherit");
        assert_eq!(SwarmWorkspaceMode::Share.as_str(), "share");
        assert_eq!(SwarmWorkspaceMode::Branch.as_str(), "branch");
    }
}
