use crate::mens::hardware::probe::{HardwareProbe, ProbeError};
use crate::mens::hardware::types::HardwareSummary;
use crate::mens::tensor::vram_autodetect::VramInfo;
use async_trait::async_trait;

/// Convert a leaf Apple-memory query into the `HardwareSummary.vram_mb` unit.
///
/// `None` → 0 (caller should record `probe_failures`). Must not call
/// `get_system_vram_info` / `get_system_vram_gb` — those Priority-4-recurse
/// into `hardware::probe()` → this module.
fn vram_mb_from_apple_info(info: Option<VramInfo>) -> u64 {
    info.map(|i| (i.total_gb * 1024.0) as u64).unwrap_or(0)
}

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
    // Leaf call only — do not route through get_system_vram_info().
    let vram_mb = vram_mb_from_apple_info(
        crate::mens::tensor::vram_autodetect::query_apple_available_memory(),
    );
    let probe_failures = if vram_mb == 0 {
        Some(vec![
            "query_apple_available_memory returned None or zero budget".into(),
        ])
    } else {
        None
    };
    Some(HardwareSummary {
        model_name: "Apple Silicon GPU".into(),
        vram_mb,
        gpu_count: 1,
        vendor: GpuVendor::Apple,
        backend: ComputeBackend::Metal,
        driver_version: None,
        pci_bus_id: None,
        probe_failures,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn probe_metal() -> Option<HardwareSummary> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vram_mb_from_apple_info_converts_usable_gib() {
        let info = VramInfo {
            total_gb: 116.0,
            used_gb: 12.0,
            free_gb: 116.0,
        };
        assert_eq!(vram_mb_from_apple_info(Some(info)), 118784);
    }

    #[test]
    fn vram_mb_from_apple_info_none_is_zero() {
        assert_eq!(vram_mb_from_apple_info(None), 0);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn probe_metal_matches_live_query_when_available() {
        let Some(info) = crate::mens::tensor::vram_autodetect::query_apple_available_memory()
        else {
            return;
        };
        let summary = probe_metal().expect("macOS always returns Some");
        let expected = vram_mb_from_apple_info(Some(info));
        // Two sequential vm_stat reads can differ by a page; allow 1 GiB drift.
        assert!(summary.vram_mb > 0, "live probe must report real memory");
        assert!(
            summary.vram_mb.abs_diff(expected) <= 1024,
            "probe_metal {} vs conversion {} drifted more than 1 GiB",
            summary.vram_mb,
            expected
        );
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn probe_metal_is_none_off_macos() {
        assert!(probe_metal().is_none());
    }
}
