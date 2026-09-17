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

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub fn probe_metal() -> Option<HardwareSummary> {
    use crate::mens::hardware::types::{ComputeBackend, GpuVendor};
    let total_ram_mb = get_macos_unified_memory_mb();
    if total_ram_mb == 0 {
        return None;
    }
    // Dynamic query: sysctl iogpu.wired_limit_percent expands working set limit up to 90%
    // Defaults safely to 75% (Apple's recommendedMaxWorkingSetSize standard)
    let wired_percent = get_macos_wired_limit_percent().unwrap_or(75).clamp(65, 90);
    let vram_mb = (total_ram_mb * wired_percent) / 100;
    Some(HardwareSummary {
        model_name: "Apple Silicon GPU".into(),
        vram_mb,
        gpu_count: 1,
        vendor: GpuVendor::Apple,
        backend: ComputeBackend::Metal,
        driver_version: None,
        pci_bus_id: None,
        probe_failures: None,
    })
}

#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
pub fn probe_metal() -> Option<HardwareSummary> {
    None
}

#[cfg(target_os = "macos")]
fn get_macos_unified_memory_mb() -> u64 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let mem = sys.total_memory() / (1024 * 1024);
    if mem > 0 {
        return mem;
    }

    if let Ok(output) = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        && output.status.success()
        && let Ok(s) = std::str::from_utf8(&output.stdout)
        && let Ok(bytes) = s.trim().parse::<u64>()
    {
        return bytes / (1024 * 1024);
    }
    0
}

#[cfg(target_os = "macos")]
fn get_macos_wired_limit_percent() -> Option<u64> {
    if let Ok(output) = std::process::Command::new("sysctl")
        .args(["-n", "iogpu.wired_limit_percent"])
        .output()
        && output.status.success()
        && let Ok(s) = std::str::from_utf8(&output.stdout)
        && let Ok(pct) = s.trim().parse::<u64>()
    {
        return Some(pct);
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn probe_metal() -> Option<HardwareSummary> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_metal_probe_dynamic_wired_limit() {
        if cfg!(target_arch = "aarch64") {
            let summary =
                probe_metal().expect("probe_metal should return Some on Apple Silicon macOS");
            assert_eq!(summary.model_name, "Apple Silicon GPU");
            let total = get_macos_unified_memory_mb();
            let pct = get_macos_wired_limit_percent().unwrap_or(75).clamp(65, 90);
            assert_eq!(
                summary.vram_mb,
                (total * pct) / 100,
                "Expected VRAM to match dynamic wired limit calculation"
            );
            assert_eq!(
                summary.vendor,
                crate::mens::hardware::types::GpuVendor::Apple
            );
            assert_eq!(
                summary.backend,
                crate::mens::hardware::types::ComputeBackend::Metal
            );
        } else {
            assert!(
                probe_metal().is_none(),
                "probe_metal should return None on non-aarch64 macOS"
            );
        }
    }
}
