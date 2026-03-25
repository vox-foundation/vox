//! # vox-orchestrator
//!
//! Multi-agent file-affinity queue system for the Vox programming language.
//!
//! Routes tasks to agents based on **file ownership** — ensuring only one agent
//! writes to any given file at a time. Prevents race conditions and lost updates
//! when multiple AI agents work concurrently across a Vox workspace.
//!
//! ## Architecture
//!
//! ```text
//!   User Request
//!       │
//!       ▼
//!   Orchestrator ──► FileAffinityMap ──► route to Agent
//!       │                                    │
//!       ▼                                    ▼
//!   BulletinBoard ◄──── AgentQueue ──► FileLockManager
//! ```
//!
//! ## Features
//!
//! - `runtime` — Actor-based agents using `vox-runtime` Scheduler/Supervisor
//! - `toestub-gate` — Post-task quality validation using TOESTUB (on by default)
//! - `lsp` — LSP diagnostic integration for file ownership info
//!
//! Module-level behavior is documented in each submodule; the crate root is a large re-export surface
//! for the orchestrator binary and integration tests.
//!
//! **Embedding:** the usual MCP host is the `vox-mcp` crate (stdio server), which
//! holds `Orchestrator` plus optional Turso `VoxDb` for Codex/Arca. Training and model SSOT for
//! Populi live in mdBook [`populi-training-ssot.md`](../../../docs/src/architecture/populi-training-ssot.md)
//! (three levels up from `src/` to repo root).
#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::let_underscore_future)]

pub mod sync_lock;

/// Agent-to-agent messaging types and helpers.
pub mod a2a;
/// File and task affinity groups for routing work to the right agent.
pub mod affinity;
/// Token and cost budgets per agent and orchestrator-wide tracking.
pub mod budget;
/// Shared bulletin board for cross-agent notices.
pub mod bulletin;
/// Host capability probing and merge with `OrchestratorConfig::default_agent_capabilities`.
pub mod capability_probe;
/// Context window compaction for long-running agent sessions.
pub mod compaction;
/// Orchestrator configuration load, merge, and validation.
pub mod config;
/// File conflict detection and resolution hooks.
pub mod conflicts;
/// Ephemeral context store for orchestrator-visible state.
pub mod context;
/// Continuation strategies when tasks pause or hand off.
pub mod continuation;
/// Canonical orchestration contract types (v2 payloads, plan surface alignment).
pub mod contract;
/// Agent activity events and pub/sub bus.
pub mod events;
/// Pre/post task gates (including TOESTUB quality checks).
pub mod gate;
/// Affinity group registry built from repository layout.
pub mod groups;
/// Structured handoff payloads between agents.
pub mod handoff;
/// Agent liveness heartbeats and staleness policy.
pub mod heartbeat;
/// Jujutsu (jj) merge DAG and backend helpers.
pub mod jj_backend;
/// Per-file lock manager for exclusive writer access.
pub mod locks;
/// Long-term and daily agent memory backed by Codex when enabled.
pub mod memory;
/// Hybrid search over orchestrator memory (lexical + embeddings).
pub mod memory_search;
/// Read-only mesh HTTP federation snapshot types (filled by MCP / embedders).
pub mod mesh_federation;
/// LLM model registry and provider configuration.
pub mod models;
/// Dynamic model catalogs.
pub mod catalog;
/// Lightweight AI usage / behavior monitor hooks.
pub mod monitor;
/// Append-only operation log for durable orchestration history.
pub mod oplog;
/// Core multi-agent orchestrator implementation.
pub mod orchestrator;
/// Question/answer routing between agents.
pub mod qa;
/// Priority task queues and overflow handling.
pub mod queue;
/// Load-based agent scale-up/down suggestions.
pub mod rebalance;
/// JSON schemas for persisted orchestrator artifacts.
pub mod schema;
/// Task path scopes and enforcement guards.
pub mod scope;
/// Security policies, audit log, and guard checks.
pub mod security;
/// Embeddings, routing, scaling, policy, and gateway services.
pub mod services;
/// Durable agent session records and managers.
pub mod session;
/// Workspace snapshots and content hashing for diffs.
pub mod snapshot;
/// Socrates evidence gate and shared task context types.
pub mod socrates;
/// Serializable orchestrator state snapshots for UI and persistence.
pub mod state;
/// Rolling summarization of agent interactions.
pub mod summary;
/// Core identifiers, tasks, messages, and shared value types.
pub mod types;
/// Aggregated LLM usage, quotas, and cost accounting.
pub mod usage;
/// Per-agent workspace views and pending change tracking.
pub mod workspace;



/// TOESTUB-based output validation gate integration.
#[cfg(feature = "toestub-gate")]
pub mod validation;

/// Tokio scheduler bridge for running tasks against a live `crate::Orchestrator`.
#[cfg(feature = "runtime")]
pub mod runtime;

/// LSP-facing helpers for ownership and diagnostics surfacing.
#[cfg(feature = "lsp")]
pub mod lsp;

// Re-export key public types for ergonomic access.
pub use a2a::{
    send_to_db, poll_inbox_from_db, acknowledge_db_message,
    prune_old_a2a_messages, DbA2AMessage, A2ARoute,
};
pub use budget::{AgentBudgetAllocation, BudgetManager, ContextBudget};
pub use compaction::{
    CompactionConfig, CompactionEngine, CompactionResult, CompactionStrategy, Turn,
};
pub use config::{OrchestratorConfig, ScalingProfile};
pub use conflicts::{ConflictId, ConflictManager, ConflictResolution, FileConflict};
pub use context::ContextStore;
pub use continuation::{ContinuationEngine, ContinuationStrategy};
pub use contract::{
    DEI_PLAN_METHODS_NEW_REPLAN_STATUS, MCP_PLAN_TOOL_NAMES, OrchestrationContractVersion,
    OrchestrationMigrationFlags, SessionContractEnvelope, TaskCapabilityHints,
    plan_tool_daemon_alignment_valid,
};
pub use events::{AgentActivity, AgentEvent, AgentEventKind, EventBus};
pub use gate::{BudgetGate, Gate, GateResult};
pub use groups::{AffinityGroup, AffinityGroupRegistry, load_from_config};
pub use handoff::{
    HandoffInvariantError, HandoffPayload, execute_handoff, validate_handoff_invariants,
};
pub use heartbeat::{persist_heartbeat, live_nodes_from_db, evict_dead_heartbeats, AgentHeartbeat, HeartbeatMonitor, HeartbeatPolicy, StalenessLevel};
pub use jj_backend::{ContentMerge, DagNodeId, MergeSide, OperationDag};
pub use memory::{DailyLog, LongTermMemory, MemoryConfig, MemoryManager, SearchHit};
pub use memory_search::{HybridSearchHit, MemorySearchEngine};
pub use mesh_federation::{MeshNodeBrief, RemoteMeshRoutingHint, RemoteMeshSnapshot};
pub use monitor::AiMonitor;
pub use oplog::{OpLog, OperationEntry, OperationId, OperationKind};
pub use orchestrator::{Orchestrator, TaskTraceStep};
pub use scope::{ScopeCheckResult, ScopeEnforcement, ScopeGuard};
pub use security::{
    AuditEntry, AuditLog, AuditResult, PolicyRule, SecurityAction, SecurityGuard, SecurityPolicy,
};
pub use services::{
    MessageGateway, PolicyCheckResult, PolicyEngine, RouteResult, RoutingService, ScalingAction,
    ScalingService,
};
pub use session::{Session, SessionConfig, SessionManager, SessionState};
pub use snapshot::{SnapshotId, SnapshotStore};
pub use socrates::{SocratesGateOutcome, SocratesTaskContext, evaluate_socrates_gate};
pub use summary::SummaryManager;
pub use types::{
    A2AMessage, A2AMessageType, AccessKind, AgentId, AgentIdGenerator, AgentMessage, AgentTask,
    BatchId, CorrelationId, CorrelationIdGenerator, MessageEnvelope, FileAffinity, MessageId,
    MessagePriority, TaskCategory, TaskDescriptor, TaskId, TaskIdGenerator, TaskPriority,
    TaskStatus, ThreadId, VcsContext, now_unix_ms,
};

pub use usage::LlmUsageKey;
pub use workspace::{AgentWorkspace, ChangeId, ChangeStatus, WorkspaceManager};

