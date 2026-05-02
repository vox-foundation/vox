use crate::mens::hardware::probe::{HardwareProbe, ProbeError};
use crate::mens::hardware::types::HardwareSummary;
use async_trait::async_trait;

/// Hardware probe backend using the macOS Metal framework.
pub struct MacosMetalProbe;

#[async_trait]
impl HardwareProbe for MacosMetalProbe {
    fn name(&self) -> &'static str {
        "macos_metal"
    }
    fn applicable(&self) -> bool {
        cfg!(target_os = "macos")
    }
    async fn probe(&self) -> Result<Option<HardwareSummary>, ProbeError> {
        Ok(probe_metal())
    }
}

#[cfg(target_os = "macos")]
pub fn probe_metal() -> Option<HardwareSummary> {
    use crate::mens::hardware::types::{ComputeBackend, GpuVendor};
    Some(HardwareSummary {
        model_name: "Apple Silicon GPU".into(),
        vram_mb: 0,
        gpu_count: 1,
        vendor: GpuVendor::Apple,
        backend: ComputeBackend::Metal,
        driver_version: None,
        pci_bus_id: None,
        probe_failures: None,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn probe_metal() -> Option<HardwareSummary> {
    None
}
