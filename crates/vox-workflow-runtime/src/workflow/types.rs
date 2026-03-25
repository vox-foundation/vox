//! Workflow planning types: mens control ops and planned activity descriptors.

/// Control-plane sub-step for a [`PopuliActivity`] (URL always comes from env / `Vox.toml`, not source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopuliHttpOp {
    /// `POST` heartbeat with the current node record.
    Heartbeat,
    /// Log only; still runs local registry publish when mens is enabled.
    Noop,
    /// `POST /v1/populi/join` for this process record.
    Join,
    /// `GET /v1/populi/nodes` (counts in journal only; no arbitrary URLs).
    Snapshot,
}

/// One planned activity invocation extracted from workflow HIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedActivity {
    /// Activity name as referenced in the workflow body.
    pub name: String,
    /// When true, run the mens / Populi HTTP step (`execute_populi_step` when feature `mens` is on).
    pub mens: bool,
    /// Idempotency / journal key from `with { activity_id: "…" }` when set.
    pub activity_id: Option<String>,
    /// Wall-clock timeout for mens HTTP sub-steps from `with { timeout: … }` (milliseconds).
    pub timeout_ms: Option<u64>,
    /// Populi control-plane operation when [`Self::mens`] is true.
    pub populi_op: PopuliHttpOp,
}

/// Mens-tagged activity (name convention: `mesh_*`, plus [`PopuliHttpOp`]).
#[derive(Debug, Clone)]
pub struct PopuliActivity {
    /// Activity name from source.
    pub name: String,
    /// Resolved mens HTTP operation.
    pub populi_op: PopuliHttpOp,
    /// Timeout for populi HTTP client (defaults inside `execute_populi_step` when unset).
    pub timeout_ms: Option<u64>,
    /// Stable id for journal / idempotency (`with { activity_id }` or generated).
    pub activity_id: String,
}
