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

/// Live VRAM budget from the Metal framework's own advisory accessor.
///
/// Replaces the old `vram_autodetect::query_apple_available_memory` shell-out
/// (`vm_stat` + `sysctl`, a heuristic estimate of live-reclaimable memory).
/// `recommendedMaxWorkingSetSize` is the driver's own measured working-set
/// budget for this process on this device — the same accessor `monitor_metal`
/// already uses for live telemetry — so this is a real value from the
/// framework, not a shell-parsed guess. Leaf call: must not route through
/// `get_system_vram_info()` (that function's own fallback recurses into
/// `hardware::probe()` → this module).
#[cfg(target_os = "macos")]
fn recommended_vram_mb() -> u64 {
    use objc2_metal::MTLDevice;
    objc2_metal::MTLCreateSystemDefaultDevice()
        .map(|device| bytes_to_mb_ceil(device.recommendedMaxWorkingSetSize()))
        .unwrap_or(0)
}

#[cfg(target_os = "macos")]
pub fn probe_metal() -> Option<HardwareSummary> {
    use crate::mens::hardware::types::{ComputeBackend, GpuVendor};
    let vram_mb = recommended_vram_mb();
    let probe_failures = if vram_mb == 0 {
        Some(vec![
            "MTLCreateSystemDefaultDevice/recommendedMaxWorkingSetSize returned None or zero budget"
                .into(),
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

/// Minimum safety reserve subtracted from live-reclaimable memory, in bytes,
/// even when the proportional margin would compute less — protects a
/// nearly-idle small Mac from a near-zero margin. Matches the constant this
/// logic used before the vram_autodetect ladder deletion (`MIN_LIVE_MEM_RESERVE_GIB`,
/// 2 GiB) and the plan's cited `max(15%, 2 GiB)` reserve.
const MIN_LIVE_PRESSURE_RESERVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Proportional margin taken off live-reclaimable memory before it's treated
/// as available budget. Matches the deleted `vram_autodetect` default.
const LIVE_PRESSURE_MARGIN_PCT: f64 = 0.15;

/// Parse `vm_stat` output into `(page_size_bytes, free_pages, inactive_pages, speculative_pages)`.
/// `None` if any required field is missing or unparseable.
///
/// Ported from the deleted `vram_autodetect::parse_vm_stat` (English `vm_stat`
/// output only, same class of limitation as the `nvidia-smi` CSV parser).
/// Purgeable pages are omitted from the reclaimable sum by design.
#[cfg(target_os = "macos")]
fn parse_vm_stat(stdout: &str) -> Option<(u64, u64, u64, u64)> {
    fn trailing_count(rest: &str) -> Option<u64> {
        rest.trim().trim_end_matches('.').parse::<u64>().ok()
    }

    let mut page_size = None;
    let mut free = None;
    let mut inactive = None;
    let mut speculative = None;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Mach Virtual Memory Statistics: (page size of ") {
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            page_size = digits.parse::<u64>().ok();
        } else if let Some(rest) = line.strip_prefix("Pages free:") {
            free = trailing_count(rest);
        } else if let Some(rest) = line.strip_prefix("Pages inactive:") {
            inactive = trailing_count(rest);
        } else if let Some(rest) = line.strip_prefix("Pages speculative:") {
            speculative = trailing_count(rest);
        }
    }

    Some((page_size?, free?, inactive?, speculative?))
}

/// Reduce a reclaimable-memory pool (bytes) by a proportional margin, floored
/// at a minimum absolute reserve so a small pool isn't left with almost no
/// margin. Pure function, independently testable without a real Mac.
fn apply_live_pressure_margin(
    reclaimable_bytes: u64,
    margin_pct: f64,
    min_reserve_bytes: u64,
) -> u64 {
    let margin = ((reclaimable_bytes as f64 * margin_pct) as u64).max(min_reserve_bytes);
    reclaimable_bytes.saturating_sub(margin)
}

/// Live memory-pressure-derived working-set estimate, in bytes: current
/// free+inactive+speculative pages (the standard macOS reclaimable-memory
/// definition, read via `vm_stat`) minus a reserve for the OS/GUI.
///
/// This is an ADDITIONAL, more conservative signal alongside
/// `recommendedMaxWorkingSetSize` — it reacts to what other processes are
/// currently using, which the driver's static advisory does not. It does not
/// replace the driver advisory (see `query_accel_budget`'s doc comment) and
/// it is not a resurrection of the deleted static VRAM ladder: it produces
/// one number, not a lookup table.
///
/// `None` if `vm_stat` fails to run or its output can't be parsed — callers
/// should fall back to the driver advisory alone in that case.
#[cfg(target_os = "macos")]
pub(crate) fn live_pressure_budget_bytes() -> Option<u64> {
    let out = std::process::Command::new("vm_stat").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let (page_size, free, inactive, speculative) =
        parse_vm_stat(&String::from_utf8_lossy(&out.stdout))?;
    let reclaimable_bytes = (free + inactive + speculative).saturating_mul(page_size);
    Some(apply_live_pressure_margin(
        reclaimable_bytes,
        LIVE_PRESSURE_MARGIN_PCT,
        MIN_LIVE_PRESSURE_RESERVE_BYTES,
    ))
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
    #[cfg(target_os = "macos")]
    fn probe_metal_matches_the_recommended_working_set_accessor() {
        // `probe_metal` must report the SAME value `recommended_vram_mb` reads
        // directly from Metal — no independent shell-based estimate anymore.
        let expected = recommended_vram_mb();
        let summary = probe_metal().expect("macOS always returns Some");
        assert_eq!(summary.vram_mb, expected);
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

    #[test]
    #[cfg(target_os = "macos")]
    fn parse_vm_stat_reads_page_size_and_reclaimable_pages() {
        // Real `vm_stat` output shape (page size of 16384 bytes).
        let sample = "Mach Virtual Memory Statistics: (page size of 16384 bytes)\n\
Pages free:                                    53189.\n\
Pages active:                                3598773.\n\
Pages inactive:                              2935813.\n\
Pages speculative:                            712573.\n\
Pages throttled:                                   0.\n\
Pages wired down:                             413750.\n\
Pages purgeable:                               17793.\n";
        let (page_size, free, inactive, speculative) = parse_vm_stat(sample).expect("parses");
        assert_eq!(page_size, 16384);
        assert_eq!(free, 53189);
        assert_eq!(inactive, 2935813);
        assert_eq!(speculative, 712573);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn parse_vm_stat_rejects_missing_fields() {
        assert!(parse_vm_stat("").is_none());
        assert!(
            parse_vm_stat("Mach Virtual Memory Statistics: (page size of 16384 bytes)\n").is_none()
        );
    }

    #[test]
    fn apply_live_pressure_margin_uses_proportional_reserve_above_the_floor() {
        // 40 GiB reclaimable, 15% margin -> 6 GiB margin (above the 2 GiB floor) -> 34 GiB.
        let gib = 1024u64 * 1024 * 1024;
        assert_eq!(
            apply_live_pressure_margin(40 * gib, 0.15, MIN_LIVE_PRESSURE_RESERVE_BYTES),
            34 * gib
        );
    }

    #[test]
    fn apply_live_pressure_margin_floors_the_reserve_on_small_pools() {
        // 5 GiB reclaimable, 15% would be 0.75 GiB -- the 2 GiB floor wins.
        let gib = 1024u64 * 1024 * 1024;
        assert_eq!(
            apply_live_pressure_margin(5 * gib, 0.15, MIN_LIVE_PRESSURE_RESERVE_BYTES),
            3 * gib
        );
    }

    #[test]
    fn apply_live_pressure_margin_never_underflows() {
        let gib = 1024u64 * 1024 * 1024;
        assert_eq!(
            apply_live_pressure_margin(gib, 0.15, MIN_LIVE_PRESSURE_RESERVE_BYTES),
            0
        );
    }
}
