//! Core orchestrator value types: ids, tasks, file affinity, and bulletin messages.
//!
//! These structs serialize cleanly for dashboards and Codex snapshots; prefer them over
//! ad-hoc tuples when crossing crate boundaries.

mod ids;
mod messages;
mod tasks;

pub use ids::{
    AgentId, AgentIdGenerator, BatchId, CorrelationId, CorrelationIdGenerator, IdParseError,
    LockToken, TaskId, TaskIdGenerator, is_zero_f64, now_unix_ms,
};
pub use messages::{
    A2AMessage, A2AMessageType, AgentMessage, MessageEnvelope, MessageId, MessagePriority,
    ThreadId, VcsContext,
};
pub use tasks::{
    AccessKind, AgentTask, FileAffinity, TaskCategory, TaskDescriptor, TaskPriority, TaskStatus,
};
