use crate::mens::hardware::types::{ComputeBackend, HardwareSummary, vendor_from_model};
use std::sync::Arc;
use tokio::sync::OnceCell;

pub mod linux_drm;
pub mod macos_metal;
#[cfg(feature = "nvml-gpu-probe")]
pub mod nvml;
pub mod pipeline;
pub mod probe;
pub mod types;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
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

/// Probes with the full attempt log. Used by `vox doctor mesh` and diagnostics.
pub async fn probe_with_report() -> crate::mens::hardware::probe::ProbeReport {
    crate::mens::hardware::pipeline::ProbePipeline::default_for_platform()
        .run()
        .await
}

async fn probe_internal() -> HardwareSummary {
    // 1. Check for operator overrides in vox-clavis (preempts probing entirely).
    if let (Some(model), Some(vram_s)) = (
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxGpuModel).expose(),
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxGpuVramMb).expose(),
    ) {
        if let Ok(vram_mb) = vram_s.parse::<u64>() {
            return HardwareSummary {
                vendor: vendor_from_model(&model),
                model_name: model.to_string(),
                vram_mb,
                gpu_count: 1,
                backend: ComputeBackend::Unknown,
                driver_version: None,
                pci_bus_id: None,
                probe_failures: None,
            };
        }
    }

    // 2. Run the platform-default pipeline.
    crate::mens::hardware::pipeline::ProbePipeline::default_for_platform()
        .run()
        .await
        .summary
}
