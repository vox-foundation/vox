//! Central coordinator for queues, affinity, locks, scope, and JJ-style undo metadata.
//!
//! [`Orchestrator`] is the integration point for routing services, Codex-backed stores,
//! and runtime agent processes when the `runtime` feature is enabled.
//!
//! ## Sub-module structure (decomposed from the original god-object)
//!
//! | Sub-module | Responsibility |
//! |---|---|
//! | [`core`] | `new`, `with_groups`, `init_db`, `record_ai_usage` |
//! | [`agent_lifecycle`] | `spawn_agent`, `retire_agent`, `pause/resume`, `heartbeat`, `handoff` |
//! | [`scaling`] | `rebalance`, `tick` |
//! | [`vcs_ops`] | `capture_snapshot`, `take_db_snapshot`, `undo/redo_operation` |

mod agent_lifecycle;
mod core;
mod scaling;
mod vcs_ops;

use std::collections::HashMap;

use crate::affinity::FileAffinityMap;
use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::groups::AffinityGroupRegistry;
use crate::locks::FileLockManager;
use crate::queue::AgentQueue;
use crate::scope::ScopeGuard;
use crate::types::{AgentId, AgentIdGenerator, TaskId, TaskIdGenerator};

pub mod accessors;
pub mod comms;
pub mod task_dispatch;
/// Error type for orchestrator operations.
pub mod types;
pub mod workflow_bridge;

#[cfg(test)]
mod tests;

pub use types::{
    AgentSummary, MAX_TASK_TRACES, OrchestratorError, OrchestratorStatus, TaskTraceStep,
};

pub struct Orchestrator {
    pub config: std::sync::Arc<std::sync::RwLock<OrchestratorConfig>>,
    pub affinity_map: FileAffinityMap,
    pub lock_manager: FileLockManager,
    pub context_store: std::sync::Arc<std::sync::RwLock<crate::context::ContextStore>>,
    pub budget_manager: std::sync::Arc<std::sync::RwLock<crate::budget::BudgetManager>>,
    pub summary_manager: std::sync::Arc<std::sync::RwLock<crate::summary::SummaryManager>>,
    pub models: std::sync::Arc<std::sync::RwLock<crate::models::ModelRegistry>>,
    pub bulletin: BulletinBoard,
    pub agents: std::sync::Arc<
        std::sync::RwLock<HashMap<AgentId, std::sync::Arc<std::sync::RwLock<AgentQueue>>>>,
    >,
    pub groups: std::sync::Arc<std::sync::RwLock<AffinityGroupRegistry>>,
    pub task_id_gen: TaskIdGenerator,
    pub agent_id_gen: AgentIdGenerator,
    /// Maps task IDs to the agent they were assigned to.
    pub task_assignments: std::sync::Arc<std::sync::RwLock<HashMap<TaskId, AgentId>>>,
    pub qa_router: std::sync::Arc<std::sync::RwLock<crate::qa::QARouter>>,
    pub monitor: std::sync::Arc<std::sync::RwLock<crate::monitor::AiMonitor>>,
    pub event_bus: crate::events::EventBus,
    pub message_bus: crate::a2a::MessageBus,
    /// IDs of agents that were dynamically spawned (transient).
    pub dynamic_agents: std::sync::Arc<std::sync::RwLock<std::collections::HashSet<AgentId>>>,
    /// Handles to the running agent processes.
    pub agent_handles:
        std::sync::Arc<std::sync::RwLock<HashMap<AgentId, vox_runtime::ProcessHandle>>>,
    pub heartbeat_monitor: std::sync::Arc<std::sync::RwLock<crate::heartbeat::HeartbeatMonitor>>,
    /// System resource monitor.
    #[cfg(feature = "system-metrics")]
    pub sys: std::sync::Arc<std::sync::RwLock<sysinfo::System>>,
    /// Historical system load for predictive scaling.
    pub load_history: std::sync::Arc<std::sync::RwLock<std::collections::VecDeque<f64>>>,
    /// Scope guard for write boundaries (synced with affinity on assign/retire).
    pub scope_guard: std::sync::Arc<std::sync::RwLock<ScopeGuard>>,
    /// Per-task timeline (ingress → route → outcome), capped at MAX_TASK_TRACES.
    pub task_traces: std::sync::Arc<std::sync::RwLock<HashMap<TaskId, Vec<TaskTraceStep>>>>,
    /// **Codex** database handle (Turso/libSQL).
    pub db: std::sync::Arc<std::sync::RwLock<Option<std::sync::Arc<vox_db::VoxDb>>>>,

    // -- JJ-inspired subsystems --
    /// Auto-snapshot store for tracking file state changes.
    pub snapshot_store: std::sync::Arc<std::sync::RwLock<crate::snapshot::SnapshotStore>>,
    /// Operation log for universal undo/redo.
    pub oplog: std::sync::Arc<std::sync::RwLock<crate::oplog::OpLog>>,
    /// First-class conflict tracking.
    pub conflict_manager: std::sync::Arc<std::sync::RwLock<crate::conflicts::ConflictManager>>,
    /// Per-agent virtual workspaces and change tracking.
    pub workspace_manager: std::sync::Arc<std::sync::RwLock<crate::workspace::WorkspaceManager>>,
    /// Timestamp of the last rebalance (for cooldown enforcement).
    pub last_rebalance_at: std::sync::Arc<std::sync::RwLock<Option<std::time::Instant>>>,
    /// Last global activity timestamp (ms) for idle detection.
    pub last_activity_ms: std::sync::atomic::AtomicU64,
    /// Last remote mens snapshot hints (from MCP federation poller); read-only placement signals.
    pub remote_mesh_routing_hints:
        std::sync::Arc<std::sync::RwLock<Vec<crate::populi_federation::RemoteMeshRoutingHint>>>,
}
