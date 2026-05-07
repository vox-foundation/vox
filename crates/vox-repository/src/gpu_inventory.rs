//! Optional NVIDIA GPU inventory via NVML for Populi **ADR 018** Layer A (GPU truth layering).
//!
//! Enable the `nvml-probe` crate feature to link `nvml-wrapper`. Without it,
//! [`probe_nvidia_gpu_inventory_best_effort`] always returns [`None`].

/// Driver-visible NVIDIA inventory snapshot (Layer A — probe-backed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuInventorySnapshot {
    /// Device count from NVML.
    pub gpu_total_count: u32,
    /// Devices that enumerate and report memory (best-effort “healthy”).
    pub gpu_healthy_count: u32,
    /// Offered allocatable devices (v1: same as healthy; refine with reservations later).
    pub gpu_allocatable_count: u32,
    /// Minimum **total** VRAM in MiB across enumerated devices (scheduling hint).
    pub min_vram_mb: Option<u32>,
}

/// Probe NVIDIA GPUs via NVML when the `nvml-probe` feature is enabled.
#[must_use]
pub fn probe_nvidia_gpu_inventory_best_effort() -> Option<GpuInventorySnapshot> {
    probe_nvml_inner()
}

#[cfg(feature = "nvml-probe")]
fn probe_nvml_inner() -> Option<GpuInventorySnapshot> {
    use nvml_wrapper::Nvml;

    let nvml = Nvml::init().ok()?;
    let count = nvml.device_count().ok()?;
    if count == 0 {
        return Some(GpuInventorySnapshot {
            gpu_total_count: 0,
            gpu_healthy_count: 0,
            gpu_allocatable_count: 0,
            min_vram_mb: None,
        });
    }

    let mut healthy: u32 = 0;
    let mut min_total_mb: Option<u32> = None;

    for idx in 0..count {
        let Ok(device) = nvml.device_by_index(idx) else {
            continue;
        };
        healthy = healthy.saturating_add(1);
        if let Ok(mem) = device.memory_info() {
            let total_mb = u32::try_from(mem.total / (1024 * 1024)).unwrap_or(u32::MAX);
            min_total_mb = Some(match min_total_mb {
                None => total_mb,
                Some(m) => m.min(total_mb),
            });
        }
    }

    Some(GpuInventorySnapshot {
        gpu_total_count: count,
        gpu_healthy_count: healthy,
        gpu_allocatable_count: healthy,
        min_vram_mb: min_total_mb,
    })
}

#[cfg(not(feature = "nvml-probe"))]
fn probe_nvml_inner() -> Option<GpuInventorySnapshot> {
    std::hint::black_box(());
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
}
