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
#[rustfmt::skip] // keeps the toestub-ignore comment pinned to the fn signature line
pub fn probe_metal() -> Option<HardwareSummary> { // toestub-ignore(skeleton/hollow-fn): off-platform stub, this file builds Metal only
    None
}

/// Rounds a byte count up to the nearest whole megabyte.
///
/// `MTLDevice.currentAllocatedSize` on a freshly-created device context is
/// typically tens of KB of driver bookkeeping (well under 1 MB): floor
/// division would report that as `0`, indistinguishable from "no telemetry" —
/// the exact ambiguity this feature exists to eliminate. Ceiling division
/// keeps any genuine nonzero usage visibly nonzero at MB granularity.
#[cfg(target_os = "macos")]
fn bytes_to_mb_ceil(bytes: u64) -> u64 {
    use crate::mens::hardware::types::BYTES_TO_MB;
    bytes.div_ceil(BYTES_TO_MB)
}

/// Live (uncached) GPU memory telemetry via Metal's `MTLDevice` accessors.
///
/// `currentAllocatedSize` is the device's live allocated-memory counter;
/// `recommendedMaxWorkingSetSize` is the driver's advisory memory budget.
/// Metal exposes no temperature/power/fan/utilization counters, so those
/// `GpuTelemetry` fields are reported as `0.0` (unknown) rather than guessed.
#[cfg(target_os = "macos")]
pub fn monitor_metal() -> Option<crate::mens::hardware::types::GpuTelemetry> {
    use crate::mens::hardware::types::GpuTelemetry;
    use objc2_metal::MTLDevice;

    let device = objc2_metal::MTLCreateSystemDefaultDevice()?;
    let used_mb = bytes_to_mb_ceil(device.currentAllocatedSize() as u64);
    let budget_mb = bytes_to_mb_ceil(device.recommendedMaxWorkingSetSize());
    Some(GpuTelemetry {
        temperature_c: 0.0,
        power_usage_w: 0.0,
        fan_speed_pct: 0.0,
        memory_used_mb: used_mb,
        memory_free_mb: budget_mb.saturating_sub(used_mb),
        utilization_pct: 0.0,
    })
}

#[cfg(not(target_os = "macos"))]
#[rustfmt::skip] // keeps the toestub-ignore comment pinned to the fn signature line
pub fn monitor_metal() -> Option<crate::mens::hardware::types::GpuTelemetry> { // toestub-ignore(skeleton/hollow-fn): off-platform stub, this file builds Metal only
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

    #[test]
    #[cfg(target_os = "macos")]
    fn monitor_metal_reports_nonzero_memory_on_a_real_device() {
        let telemetry = monitor_metal().expect("macOS always has a default Metal device");
        assert!(
            telemetry.memory_used_mb > 0,
            "a stub returning a zeroed struct is the same bug wearing a different shape"
        );
    }

    #[test]
    #[cfg(not(target_os = "macos"))]
    fn monitor_metal_is_none_off_macos() {
        assert!(monitor_metal().is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn bytes_to_mb_ceil_rounds_up_sub_megabyte_remainders() {
        assert_eq!(bytes_to_mb_ceil(0), 0);
        assert_eq!(bytes_to_mb_ceil(65_536), 1);
        assert_eq!(bytes_to_mb_ceil(1024 * 1024), 1);
        assert_eq!(bytes_to_mb_ceil(1024 * 1024 + 1), 2);
    }
}
