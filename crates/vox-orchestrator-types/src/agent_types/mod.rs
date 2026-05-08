//! Core orchestrator agent/task identity types.
//!
//! These types are a leaf dependency — no orchestrator internals required.

pub mod ids;
pub mod switch;

pub use ids::{
    AgentId, AgentIdGenerator, BatchId, CorrelationId, CorrelationIdGenerator, IdParseError,
    LockToken, TaskId, TaskIdGenerator, is_zero_f64, now_unix_ms,
};
pub use switch::{SwitchAccessMode, SwitchAction, SwitchActionType};
