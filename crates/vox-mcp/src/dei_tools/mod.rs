//! Live DEI orchestrator inspection and lightweight control surfaces exposed as MCP tools.
//!
//! Covers agent queues, lock/budget summaries, Gamify-backed history, affinity graphs,
//! JSON config patch, task submission shims, session heartbeats, and cost recording.

pub mod params;

mod control;
mod orchestrator_snapshot;
mod vcs_runtime;

pub use control::{
    agent_events, budget_status, cancel_task, config_get, config_set, cost_history, drain_agent,
    file_graph, lock_status, map_agent_session, orchestrator_start, queue_status, rebalance,
    reorder_task,
};
pub use orchestrator_snapshot::orchestrator_status;
pub use params::*;
pub use vcs_runtime::{
    check_file_owner, heartbeat, poll_events, record_cost, submit_task, vcs_status,
};
