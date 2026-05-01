use crate::mens::hardware::types::HardwareSummary;

#[cfg(target_os = "linux")]
pub fn probe_drm() -> Option<HardwareSummary> {
    use crate::mens::hardware::types::{ComputeBackend, GpuVendor};
    use std::fs;

    // Check for NVIDIA driver via procfs first
    if fs::metadata("/proc/driver/nvidia/version").is_ok() {
        let name = fs::read_to_string("/proc/driver/nvidia/gpus/0000:01:00.0/information")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|l| l.contains("Model:"))
                    .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            })
            .unwrap_or_else(|| "NVIDIA GPU".into());

        return Some(HardwareSummary {
            model_name: name,
            vram_mb: 0,
            gpu_count: 1,
            vendor: GpuVendor::Nvidia,
            backend: ComputeBackend::Cuda,
            driver_version: None,
            pci_bus_id: Some("0000:01:00.0".into()),
            probe_failures: None,
        });
    }

    let mut count = 0;
    let mut first_card: Option<HardwareSummary> = None;
    let paths = fs::read_dir("/sys/class/drm").ok()?;
    for entry in paths.filter_map(Result::ok) {
        let name = entry.file_name().into_string().ok()?;
        if name.starts_with("card") && !name.contains('-') {
            count += 1;
            if first_card.is_none() {
                first_card = Some(HardwareSummary {
                    model_name: format!("DRM Device ({})", name),
                    vram_mb: 0,
                    gpu_count: 0, // Placeholder
                    vendor: GpuVendor::Unknown,
                    backend: ComputeBackend::Wgpu,
                    driver_version: None,
                    pci_bus_id: None,
                    probe_failures: None,
                });
            }
        }
    }
    if let Some(mut info) = first_card {
        info.gpu_count = count;
        return Some(info);
    }
    None
}

#[cfg(not(target_os = "linux"))]
pub fn probe_drm() -> Option<HardwareSummary> {
    None
}
