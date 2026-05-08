//! Queue/lock/oplog primitives for vox-orchestrator.
//!
//! Extracted from `vox-orchestrator/src/{locks,oplog,affinity,sync_lock}.rs`
//! in 2026-05-08 reorg Phase 5. Depends only on `vox-orchestrator-types`
//! so it lives below the orchestrator core in the layer model.

pub mod affinity;
pub mod locks;
pub mod oplog;
pub mod sync_lock;
