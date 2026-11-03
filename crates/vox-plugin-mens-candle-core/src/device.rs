//! Device-kind vocabulary shared by both Candle backend plugins.
//!
//! `DeviceKind` and `GpuInfo` were byte-identical (modulo one doc-comment word)
//! between `vox-plugin-mens-candle-metal::device` and
//! `vox-plugin-mens-candle-cuda::device`. Everything else in those two
//! `device.rs` files — `probe_gpu`'s CUDA NVML/driver integration, the CUDA
//! memory-pool FFI bindings — is genuinely device-specific and stays in each
//! plugin crate.

/// CLI / env device intent for the Candle backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// Prefer host CPU / software paths.
    Cpu,
    /// Let the stack pick: prefers CUDA, then Metal, then CPU, depending on
    /// which accelerator feature(s) this build enables.
    #[default]
    Best,
    /// Prefer NVIDIA CUDA.
    Cuda,
    /// Prefer Apple Metal (macOS).
    Metal,
}

/// Best-effort local GPU description.
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub model_name: String,
    pub vram_mb: u64,
    pub vendor: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_kind_default_is_best() {
        assert_eq!(DeviceKind::default(), DeviceKind::Best);
    }

    #[test]
    fn device_kind_round_trips_through_serde() {
        for kind in [
            DeviceKind::Cpu,
            DeviceKind::Best,
            DeviceKind::Cuda,
            DeviceKind::Metal,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: DeviceKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }
}
