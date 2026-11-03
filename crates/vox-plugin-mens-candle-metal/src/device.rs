//! Device selection for Candle training.
//!
//! Ported from `vox-populi/src/mens/tensor/device.rs` (SP3 sub-batch C).
//! `probe_gpu` is stubbed to avoid pulling in vox-populi hardware registry.
//!
//! `DeviceKind` / `GpuInfo` moved to `vox-plugin-mens-candle-core` — they were
//! identical to the CUDA plugin's copies. `probe_gpu` below stays: it is a
//! genuine per-plugin stub (CUDA's real version reads NVML/driver memory
//! stats; this one always reports "unknown").
pub use vox_plugin_mens_candle_core::device::{DeviceKind, GpuInfo};

/// Minimal GPU probe — returns unknown vendor when no probe is possible.
/// SP3-C stub: hardware registry is a vox-populi concern; reconnect via host capability in sub-batch D.
#[must_use]
pub fn probe_gpu() -> GpuInfo {
    GpuInfo {
        model_name: "unknown".to_string(),
        vram_mb: 0,
        vendor: "unknown".to_string(),
    }
}
