use crate::mens::hardware::types::HardwareSummary;

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
    })
}

#[cfg(not(target_os = "macos"))]
pub fn probe_metal() -> Option<HardwareSummary> {
    None
}
