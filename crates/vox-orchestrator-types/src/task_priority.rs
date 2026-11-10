use serde::{Deserialize, Serialize};
use std::fmt;

/// Priority level for a task. Higher priority tasks are dequeued first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Background work — lowest priority.
    Background = 0,
    /// Normal priority — default.
    Normal = 1,
    /// Urgent — preempts normal tasks.
    Urgent = 2,
}

impl TaskPriority {
    /// Map a wire `u8` (e.g. the `priority` field of a `HopperSync` op) to a
    /// `TaskPriority`. Matches the enum's `repr` (0/1/2); any out-of-range value
    /// falls back to `Normal` rather than failing — replication should degrade
    /// gracefully, not drop an admission.
    #[must_use]
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Background,
            2 => Self::Urgent,
            _ => Self::Normal,
        }
    }
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Normal => write!(f, "normal"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}
