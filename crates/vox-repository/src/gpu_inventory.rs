//! Optional NVIDIA GPU inventory via NVML for Populi **ADR 018** Layer A (GPU truth layering).
//!
//! NVML probing has been extracted to the `vox-plugin-nvml-probe` cdylib plugin.
//! This module retains the [`GpuInventorySnapshot`] type and a stub best-effort
//! function that always returns [`None`], preserving the public API so callers
//! do not need to be updated until they migrate to plugin-dispatch.

/// Driver-visible NVIDIA inventory snapshot (Layer A — probe-backed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInventorySnapshot {
    /// Device count from NVML.
    pub gpu_total_count: u32,
    /// Devices that enumerate and report memory (best-effort "healthy").
    pub gpu_healthy_count: u32,
    /// Offered allocatable devices (v1: same as healthy; refine with reservations later).
    pub gpu_allocatable_count: u32,
    /// Minimum **total** VRAM in MiB across enumerated devices (scheduling hint).
    pub min_vram_mb: Option<u32>,
}

/// Probe NVIDIA GPUs via NVML.
///
/// NVML probing has moved to `vox-plugin-nvml-probe`. This stub always returns
/// `None`. Callers should migrate to plugin-host dispatch via
/// `VoxPlugin::as_hardware_probe()` to get live GPU inventory.
#[must_use]
pub fn probe_nvidia_gpu_inventory_best_effort() -> Option<GpuInventorySnapshot> {
    let _ = std::hint::black_box(());
    None
}

#[cfg(test)]
mod tests {
    use super::GpuInventorySnapshot;

    #[test]
    fn snapshot_zero_devices_roundtrip_semantics() {
        let z = GpuInventorySnapshot {
            gpu_total_count: 0,
            gpu_healthy_count: 0,
            gpu_allocatable_count: 0,
            min_vram_mb: None,
        };
        assert_eq!(z.gpu_total_count, 0);
    }

    #[test]
    fn stub_returns_none() {
        assert!(super::probe_nvidia_gpu_inventory_best_effort().is_none());
    }
}
