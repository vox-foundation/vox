//! Workspace + snapshot identifiers extracted from `vox-orchestrator` in 2026-05-08 reorg Phase 5.
//!
//! Pure-types: no async runtime, no DB. Used by `vox-orchestrator-queue`
//! (oplog, locks) so the queue subsystem doesn't need to depend on the
//! full orchestrator.

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// Unique snapshot identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnapshotId(pub u64);

impl fmt::Display for SnapshotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "S-{:06}", self.0)
    }
}

/// Monotonic snapshot id generator.
#[derive(Debug)]
pub struct SnapshotIdGenerator(AtomicU64);

impl SnapshotIdGenerator {
    pub const fn new() -> Self {
        Self(AtomicU64::new(1))
    }

    pub fn next(&self) -> SnapshotId {
        SnapshotId(self.0.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for SnapshotIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Identifier for a workspace change (file mutation set).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId(pub u64);

impl fmt::Display for ChangeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CH-{:06}", self.0)
    }
}
