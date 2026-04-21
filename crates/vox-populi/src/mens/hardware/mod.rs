use crate::mens::hardware::types::{ComputeBackend, GpuVendor, HardwareSummary, vendor_from_model};
use std::sync::Arc;
use tokio::sync::OnceCell;

pub mod linux_drm;
pub mod macos_metal;
#[cfg(feature = "nvml-gpu-probe")]
pub mod nvml;
pub mod types;
#[cfg(feature = "mens-gpu")]
pub mod wgpu_probe;
#[cfg(all(target_os = "windows", feature = "mens-gpu"))]
pub mod win_dxgi;

static REGISTRY: OnceCell<Arc<HardwareSummary>> = OnceCell::const_new();

/// Global hardware registry for the Mens subsystem.
pub struct HardwareRegistry;

impl HardwareRegistry {
    /// Probes the hardware and caches the result for the lifetime of the process.
    pub async fn probe() -> Arc<HardwareSummary> {
        REGISTRY
            .get_or_init(|| async { Arc::new(probe_internal().await) })
            .await
            .clone()
    }

    /// Monitors real-time telemetry (not cached).
    pub fn monitor() -> Option<types::GpuTelemetry> {
        #[cfg(feature = "nvml-gpu-probe")]
        {
            nvml::monitor_nvml()
        }
        #[cfg(not(feature = "nvml-gpu-probe"))]
        {
            None
        }
    }
}

/// Compatibility wrapper for the new registry.
pub async fn probe() -> Arc<HardwareSummary> {
    HardwareRegistry::probe().await
}

async fn probe_internal() -> HardwareSummary {
    // 1. Check for overrides in vox-clavis
    if let (Some(model), Some(vram_s)) = (
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxGpuModel).expose(),
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxGpuVramMb).expose(),
    ) {
        if let Ok(vram_mb) = vram_s.parse::<u64>() {
            return HardwareSummary {
                vendor: vendor_from_model(&model),
                model_name: model.to_string(),
                vram_mb,
                gpu_count: 1, // Assume 1 for override
                backend: ComputeBackend::Unknown,
                driver_version: None,
                pci_bus_id: None,
            };
        }
    }

    // 2. Platform-specific native probes
    #[cfg(all(target_os = "windows", feature = "mens-gpu"))]
    if let Some(info) = win_dxgi::probe_dxgi() {
        return info;
    }
    if let Some(info) = linux_drm::probe_drm() {
        return info;
    }
    if let Some(info) = macos_metal::probe_metal() {
        return info;
    }

    // 3. Generic Probe: wgpu (cross-platform fallback)
    #[cfg(feature = "mens-gpu")]
    if let Some(info) = wgpu_probe::probe_wgpu().await {
        return info;
    }

    // 4. Ultimate Fallback: NVML (if available) or Host CPU
    #[cfg(feature = "nvml-gpu-probe")]
    if let Some(telemetry) = nvml::monitor_nvml() {
        return HardwareSummary {
            model_name: "NVIDIA GPU (via NVML)".into(),
            vram_mb: telemetry.memory_used_mb + telemetry.memory_free_mb,
            gpu_count: 1,
            vendor: GpuVendor::Nvidia,
            backend: ComputeBackend::Cuda,
            driver_version: None,
            pci_bus_id: None,
        };
    }

    HardwareSummary {
        model_name: "Host CPU".into(),
        vram_mb: 0,
        gpu_count: 0,
        vendor: GpuVendor::Cpu,
        backend: ComputeBackend::Cpu,
        driver_version: None,
        pci_bus_id: None,
    }
}
