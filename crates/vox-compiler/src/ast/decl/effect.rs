//! Effect annotation types for function declarations (`uses net, db, mcp(...)`).

use crate::ast::span::Span;

/// A single effect capability that a function may declare.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EffectKind {
    /// Outbound HTTP / WebSocket calls.
    Net,
    /// Database reads or writes.
    Db,
    /// Filesystem reads or writes.
    Fs,
    /// Environment variable reads.
    Env,
    /// Reads current system time.
    Clock,
    /// Consumes entropy (RNG).
    Random,
    /// Spawns subprocesses or background tasks.
    Spawn,
    /// Calls a specific MCP tool — parameterized by tool name.
    Mcp(String),
}

impl EffectKind {
    /// Canonical display label (used in diagnostics).
    pub fn label(&self) -> String {
        match self {
            EffectKind::Net => "net".into(),
            EffectKind::Db => "db".into(),
            EffectKind::Fs => "fs".into(),
            EffectKind::Env => "env".into(),
            EffectKind::Clock => "clock".into(),
            EffectKind::Random => "random".into(),
            EffectKind::Spawn => "spawn".into(),
            EffectKind::Mcp(tool) => format!("mcp({tool})"),
        }
    }
}

/// A `uses` clause on a function declaration: an ordered, deduplicated list of effects.
///
/// `Vec` rather than `HashSet` so spans are preserved and ordering is deterministic.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EffectAnnotation {
    pub kind: EffectKind,
    pub span: Span,
}
