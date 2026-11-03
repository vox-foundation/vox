//! Oversight composition. Phase 12: the action-only safety floor. Phases 13–17 add `resolve()`,
//! session registry, background/chat consumers, and `AttentionInbox`; effective tier = max(floor, level, trust).

pub mod floor;
pub mod resolver;
pub use resolver::*;
