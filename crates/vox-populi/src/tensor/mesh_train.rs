//! GPU mesh training scaffolding for future multi-device support.
//! 
//! This module establishes the integration points for data-parallel training 
//! without adding runtime overhead to single-device runs.

/// Configuration for a distributed GPU mesh training worker.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeshTrainConfig {
    /// Total number of devices participating in the mesh.
    pub world_size: usize,
    /// The unique index of this worker (0..world_size-1).
    pub rank: usize,
    /// Whether to synchronize gradients after each accumulation step.
    pub gradient_reduce: bool,
}

/// Returns true if `VOX_MESH_TRAIN=1` is set in the environment.
pub fn is_mesh_mode() -> bool {
    // Check environment; default to false (single-node)
    std::env::var("VOX_MESH_TRAIN").map(|v| v == "1").unwrap_or(false)
}

/// Returns the calculated rank for this worker from environment variables.
pub fn get_mesh_rank() -> usize {
    std::env::var("VOX_MESH_RANK")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
