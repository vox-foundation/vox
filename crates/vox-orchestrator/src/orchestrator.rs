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

mod core;
mod agent_lifecycle;
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
use crate::types::{
    AgentId, AgentIdGenerator, TaskId, TaskIdGenerator,
};

/// Error type for orchestrator operations.

pub mod types;
pub mod task_dispatch;
pub mod accessors;
pub mod comms;

#[cfg(test)]
mod tests;

pub use types::{AgentSummary, OrchestratorError, OrchestratorStatus, TaskTraceStep, MAX_TASK_TRACES};

pub struct Orchestrator {
    config: OrchestratorConfig,
    affinity_map: FileAffinityMap,
    lock_manager: FileLockManager,
    context_store: crate::context::ContextStore,
    budget_manager: crate::budget::BudgetManager,
    summary_manager: crate::summary::SummaryManager,
    models: crate::models::ModelRegistry,
    bulletin: BulletinBoard,
    agents: HashMap<AgentId, AgentQueue>,
    groups: AffinityGroupRegistry,
    task_id_gen: TaskIdGenerator,
    agent_id_gen: AgentIdGenerator,
    /// Maps task IDs to the agent they were assigned to.
    task_assignments: HashMap<TaskId, AgentId>,
    qa_router: crate::qa::QARouter,
    monitor: crate::monitor::AiMonitor,
    event_bus: crate::events::EventBus,
    message_bus: crate::a2a::MessageBus,
    /// IDs of agents that were dynamically spawned (transient).
    dynamic_agents: std::collections::HashSet<AgentId>,
    /// Handles to the running agent processes.
    agent_handles: HashMap<AgentId, vox_runtime::ProcessHandle>,
    heartbeat_monitor: crate::heartbeat::HeartbeatMonitor,
    /// System resource monitor.
    #[cfg(feature = "system-metrics")]
    sys: sysinfo::System,
    /// Historical system load for predictive scaling.
    load_history: std::collections::VecDeque<f64>,
    /// Scope guard for write boundaries (synced with affinity on assign/retire).
    scope_guard: ScopeGuard,
    /// Per-task timeline (ingress → route → outcome), capped at MAX_TASK_TRACES.
    task_traces: HashMap<TaskId, Vec<TaskTraceStep>>,
    /// **Codex** database handle (Turso/libSQL).
    db: Option<std::sync::Arc<vox_db::VoxDb>>,
    // -- JJ-inspired subsystems --
    /// Auto-snapshot store for tracking file state changes.
    snapshot_store: crate::snapshot::SnapshotStore,
    /// Operation log for universal undo/redo.
    oplog: crate::oplog::OpLog,
    /// First-class conflict tracking.
    conflict_manager: crate::conflicts::ConflictManager,
    /// Per-agent virtual workspaces and change tracking.
    workspace_manager: crate::workspace::WorkspaceManager,
    /// Timestamp of the last rebalance (for cooldown enforcement).
    last_rebalance_at: Option<std::time::Instant>,
    /// Last global activity timestamp (ms) for idle detection.
    last_activity_ms: std::sync::atomic::AtomicU64,
    /// Last remote mesh snapshot hints (from MCP federation poller); read-only placement signals.
    remote_mesh_routing_hints: Vec<crate::mesh_federation::RemoteMeshRoutingHint>,
}
