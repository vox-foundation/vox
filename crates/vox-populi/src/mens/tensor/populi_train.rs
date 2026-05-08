//! GPU mens training scaffolding for future multi-device support.
//!
//! This module establishes the integration points for data-parallel training
//! without adding runtime overhead to single-device runs.

/// Configuration for a distributed GPU mens training worker.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeshTrainConfig {
    /// Total number of devices participating in the mens.
    pub world_size: usize,
    /// The unique index of this worker (0..world_size-1).
    pub rank: usize,
    /// Whether to synchronize gradients after each accumulation step.
    pub gradient_reduce: bool,
}

pub fn is_mesh_mode() -> bool {
    // Check environment; default to false (single-node)
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshTrain)
        .expose()
        .map(|v: &str| v == "1")
        .unwrap_or(false)
}

pub fn get_mesh_rank() -> usize {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshRank)
        .expose()
        .and_then(|v: &str| v.parse().ok())
        .unwrap_or(0)
}
