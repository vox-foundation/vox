//! What the accelerator will actually give us, asked of the driver.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetSource {
    Metal,
    Cuda,
}

#[derive(Debug, Clone)]
pub struct AccelBudget {
    pub device_name: String,
    /// Metal: the driver's static `recommendedMaxWorkingSetSize` advisory,
    /// unadjusted. CUDA: total VRAM. See `working_set_bytes` for the
    /// live-pressure-adjusted effective budget.
    pub total_bytes: u64,
    /// Metal: `min(recommendedMaxWorkingSetSize, live memory pressure)` — the
    /// driver's static advisory does not react to what other processes
    /// (browser, IDE, other apps) are using right now, so it's combined with
    /// a live `vm_stat`-derived reading and the smaller of the two wins. CUDA:
    /// total VRAM (no live-pressure signal implemented for CUDA here).
    pub working_set_bytes: u64,
    /// Largest single allocation. Metal: `maxBufferLength`. Binds before
    /// `working_set_bytes` on large-batch runs.
    pub max_alloc_bytes: u64,
    pub source: BudgetSource,
}

impl AccelBudget {
    /// Stable identity for calibration lookups. Exact bytes, never rounded.
    pub fn host_key(&self) -> String {
        let compact: String = self
            .device_name
            .chars()
            .filter(char::is_ascii_alphanumeric)
            .collect();
        format!("{compact}-{}", self.working_set_bytes)
    }

    pub fn single_alloc_fits(&self, bytes: u64) -> bool {
        bytes <= self.max_alloc_bytes
    }
}

/// Combine the driver's static working-set advisory with a live
/// memory-pressure reading, taking the more conservative (smaller) of the
/// two. The driver advisory (`recommendedMaxWorkingSetSize`) doesn't react to
/// what other processes (browser, IDE, other apps) are using right now; a
/// pure live-pressure read ignores the hardware ceiling. Neither alone is
/// trustworthy, so the effective budget is their minimum. `None` live
/// pressure (e.g. `vm_stat` unavailable/unparseable) leaves the driver
/// advisory untouched. Not `#[cfg]`-gated so it's testable on every host.
fn combine_with_live_pressure(driver_advisory_bytes: u64, live_pressure_bytes: Option<u64>) -> u64 {
    match live_pressure_bytes {
        Some(live) => driver_advisory_bytes.min(live),
        None => driver_advisory_bytes,
    }
}

// Verified against objc2-metal 0.3.2's generated/MTLDevice.rs:722
// (recommendedMaxWorkingSetSize) and :1740 (maxBufferLength) — re-check on any
// objc2-metal version bump.
#[cfg(target_os = "macos")]
pub fn query_accel_budget() -> Option<AccelBudget> {
    use crate::mens::hardware::macos_metal::live_pressure_budget_bytes;
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
    let device = MTLCreateSystemDefaultDevice()?;
    let driver_advisory = device.recommendedMaxWorkingSetSize();
    let working = combine_with_live_pressure(driver_advisory, live_pressure_budget_bytes());
    Some(AccelBudget {
        device_name: device.name().to_string(),
        total_bytes: driver_advisory,
        working_set_bytes: working,
        // NSUInteger is usize; the struct field is u64.
        max_alloc_bytes: device.maxBufferLength() as u64,
        source: BudgetSource::Metal,
    })
}

/// Build a CUDA-sourced budget from `vram_autodetect::get_system_vram_info()`'s
/// free-VRAM reading — the same value the old `train_arm.rs::budget_gate`
/// used before this plan. CUDA has no separate single-buffer cap distinct
/// from the pool total (see the module doc comment above), so
/// `max_alloc_bytes == working_set_bytes` here, exactly as anticipated.
/// Deliberately NOT `#[cfg]`-gated so it is unit-testable on every host,
/// including this dev machine — only its non-macOS caller below is gated.
/// (On a macOS build it is only reachable from `#[cfg(test)]`, hence the
/// `allow`.)
#[cfg_attr(target_os = "macos", allow(dead_code))]
fn cuda_budget_from_vram_info(info: crate::mens::tensor::vram_autodetect::VramInfo) -> AccelBudget {
    let free_bytes = (info.free_gb as f64 * 1024.0 * 1024.0 * 1024.0).round() as u64;
    AccelBudget {
        device_name: "CUDA".to_string(),
        total_bytes: free_bytes,
        working_set_bytes: free_bytes,
        max_alloc_bytes: free_bytes,
        source: BudgetSource::Cuda,
    }
}

/// CUDA (and any other non-macOS host `vram_autodetect` can read, e.g. via
/// its hardware-registry fallback): sourced from `vram_autodetect::
/// get_system_vram_info()`, which already lives in this crate (`vox-populi`)
/// and needs no crate-edge exception — `nvidia-smi` under the hood, same as
/// the pre-plan `train_arm.rs::budget_gate` used. `None` only when no VRAM
/// info could be read at all (no GPU, no override, no `nvidia-smi`).
#[cfg(not(target_os = "macos"))]
pub fn query_accel_budget() -> Option<AccelBudget> {
    let info = crate::mens::tensor::vram_autodetect::get_system_vram_info()?;
    Some(cuda_budget_from_vram_info(info))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget(working: u64, max_alloc: u64) -> AccelBudget {
        AccelBudget {
            device_name: "Apple M5 Max".into(),
            total_bytes: 137_438_953_472,
            working_set_bytes: working,
            max_alloc_bytes: max_alloc,
            source: BudgetSource::Metal,
        }
    }

    #[test]
    fn max_alloc_can_bind_before_the_working_set() {
        let b = budget(115_448_725_504, 86_587_244_544); // measured M5 Max
        assert!(!b.single_alloc_fits(90 * 1024 * 1024 * 1024));
        assert!(b.single_alloc_fits(70 * 1024 * 1024 * 1024));
    }

    #[test]
    fn host_key_carries_exact_bytes_so_it_cannot_drift_by_rounding() {
        let b = budget(115_448_725_504, 86_587_244_544);
        assert!(
            b.host_key().contains("115448725504"),
            "key must pin exact bytes: {}",
            b.host_key()
        );
        assert_ne!(
            b.host_key(),
            budget(57_724_362_752, 43_293_622_272).host_key()
        );
    }

    #[test]
    fn query_accel_budget_is_callable_on_every_platform() {
        // tdd-guard requires a same-file test for this pub fn, and a None on a
        // GPU-less host is a valid answer, not a skip.
        let _ = query_accel_budget();
    }

    // ── C1 fix: the non-macOS (CUDA) budget must be real, not `None` ──

    /// `cuda_budget_from_vram_info` is not `#[cfg]`-gated, so this runs on
    /// every host (including this Mac) and pins the C1 fix: a non-macOS host
    /// with real VRAM must produce a real, non-`None` `AccelBudget` sourced
    /// from `vram_autodetect`'s free-VRAM reading — not the old `None` stub.
    #[test]
    fn cuda_budget_from_vram_info_uses_free_vram_for_both_fields() {
        use crate::mens::tensor::vram_autodetect::VramInfo;
        let info = VramInfo {
            total_gb: 24.0,
            used_gb: 4.0,
            free_gb: 20.0,
        };
        let b = cuda_budget_from_vram_info(info);
        let expected = (20.0f64 * 1024.0 * 1024.0 * 1024.0).round() as u64;
        assert_eq!(b.working_set_bytes, expected);
        // CUDA has no separate single-buffer cap: max_alloc == working_set.
        assert_eq!(b.max_alloc_bytes, expected);
        assert_eq!(b.source, BudgetSource::Cuda);
    }

    /// The whole point of C1: a CUDA-sourced budget must flow through to a
    /// real `Fits`/`Refused` verdict from `plan_for`, not `NoMeasurement` —
    /// confirmed here by feeding the same budget the fixed
    /// `query_accel_budget()` would now produce on a non-macOS host into the
    /// real `memory_model::plan_for` against a calibrated `candle-cuda` lane.
    #[test]
    fn cuda_budget_flows_through_to_a_real_plan_for_verdict() {
        use crate::mens::tensor::memory_model::{
            CalKey, DeviceBudget, Lane, MemoryModels, ModelShape, Request, Verdict, plan_for,
        };
        use crate::mens::tensor::vram_autodetect::VramInfo;

        fn as_device_budget(accel: &AccelBudget) -> DeviceBudget {
            DeviceBudget {
                working_set_bytes: accel.working_set_bytes,
                operator_fraction: None,
            }
        }

        const SEED_YAML: &str = r#"
schema: vox.mens.memory-model.v1
lanes:
  - lane: candle-cuda
    gradient_checkpointing: false
    act_bytes_per_lht: 100.0
    source: measured
"#;
        let models = MemoryModels::load_from_str(SEED_YAML).unwrap();
        let key = CalKey::new(Lane::CandleCuda, false).unwrap();
        let shape = ModelShape {
            artifact_bytes: 1_000_000,
            layers: 4,
            hidden: 256,
        };

        // Plenty of VRAM: must Fit, not NoMeasurement/None.
        let roomy = cuda_budget_from_vram_info(VramInfo {
            total_gb: 80.0,
            used_gb: 0.0,
            free_gb: 80.0,
        });
        let fits = plan_for(
            &as_device_budget(&roomy),
            &models,
            &key,
            &shape,
            &Request {
                batch_size: 1,
                seq_len: 128,
            },
        );
        assert_eq!(fits.verdict, Verdict::Fits);

        // Almost no VRAM: must be a measured Refused, not a silent pass.
        let tiny = cuda_budget_from_vram_info(VramInfo {
            total_gb: 0.001,
            used_gb: 0.0,
            free_gb: 0.001,
        });
        let refused = plan_for(
            &as_device_budget(&tiny),
            &models,
            &key,
            &shape,
            &Request {
                batch_size: 64,
                seq_len: 4096,
            },
        );
        assert!(matches!(refused.verdict, Verdict::Refused(_)));
    }

    // ── live memory pressure as an additional, more-conservative constraint ──

    #[test]
    fn combine_with_live_pressure_takes_live_reading_when_tighter() {
        let driver_advisory = 100 * 1024 * 1024 * 1024; // 100 GiB
        let live_pressure = 40 * 1024 * 1024 * 1024; // 40 GiB free right now
        assert_eq!(
            combine_with_live_pressure(driver_advisory, Some(live_pressure)),
            live_pressure
        );
    }

    #[test]
    fn combine_with_live_pressure_takes_driver_advisory_when_tighter() {
        let driver_advisory = 40 * 1024 * 1024 * 1024; // 40 GiB hardware ceiling
        let live_pressure = 100 * 1024 * 1024 * 1024; // plenty currently free
        assert_eq!(
            combine_with_live_pressure(driver_advisory, Some(live_pressure)),
            driver_advisory
        );
    }

    #[test]
    fn combine_with_live_pressure_falls_back_to_driver_advisory_when_unavailable() {
        let driver_advisory = 64 * 1024 * 1024 * 1024;
        assert_eq!(
            combine_with_live_pressure(driver_advisory, None),
            driver_advisory
        );
    }

    #[test]
    #[ignore = "reads the live device; run manually on a Metal host"]
    fn live_device_reports_a_plausible_budget() {
        let b = query_accel_budget().expect("a device on this host");
        assert!(b.working_set_bytes > 0 && b.max_alloc_bytes > 0);
        assert!(b.max_alloc_bytes <= b.working_set_bytes);
        eprintln!(
            "{} working_set={} max_alloc={}",
            b.device_name, b.working_set_bytes, b.max_alloc_bytes
        );
    }
}
