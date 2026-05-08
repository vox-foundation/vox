//! Identity types and monotonic ID generators.
//!
//! Re-exported from `vox-orchestrator-types::agent_types::ids`.

pub use vox_orchestrator_types::agent_types::ids::{
    AgentId, AgentIdGenerator, BatchId, CorrelationId, CorrelationIdGenerator, IdParseError,
    LockToken, TaskId, TaskIdGenerator, is_zero_f64, now_unix_ms,
};
