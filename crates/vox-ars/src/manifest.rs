//! ARS-facing manifest shapes (execution limits, skill kind).

use serde::{Deserialize, Serialize};

/// Resource envelope for sandboxed task execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max wall-clock milliseconds (advisory; executor may ignore if unset).
    pub max_wall_ms: Option<u64>,
    /// Max output bytes (advisory).
    pub max_output_bytes: Option<u64>,
}

/// High-level skill classification for the runtime harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillKind {
    /// Document-style skill (markdown instructions).
    #[default]
    Document,
    /// Executable / tool-backed skill.
    Tool,
}
