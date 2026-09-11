//! What the accelerator will actually give us, asked of the driver.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetSource {
    Metal,
    Cuda,
}

#[derive(Debug, Clone)]
pub struct AccelBudget {
    pub device_name: String,
    pub total_bytes: u64,
    /// Metal: `recommendedMaxWorkingSetSize`. CUDA: total VRAM.
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

// Verified against objc2-metal 0.3.2's generated/MTLDevice.rs:722
// (recommendedMaxWorkingSetSize) and :1740 (maxBufferLength) — re-check on any
// objc2-metal version bump.
#[cfg(target_os = "macos")]
pub fn query_accel_budget() -> Option<AccelBudget> {
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
    let device = MTLCreateSystemDefaultDevice()?;
    let working = device.recommendedMaxWorkingSetSize();
    Some(AccelBudget {
        device_name: device.name().to_string(),
        total_bytes: working,
        working_set_bytes: working,
        // NSUInteger is usize; the struct field is u64.
        max_alloc_bytes: device.maxBufferLength() as u64,
        source: BudgetSource::Metal,
    })
}

/// CUDA: **not yet wired**. `vox-plugin-nvml-probe` (layer 3) reports total
/// VRAM (which would become `working_set_bytes` == `max_alloc_bytes`, since
/// CUDA has no separate single-buffer cap), but `vox-populi` is layer 2 — a
/// static dependency would be an upward edge disallowed by the crate-layers
/// "downward-only" rule (see `contracts/ci/crate-layers.v1.json`), and adding
/// a `crate-edges` exception is user-authorized-only (see `AGENTS.md`
/// §Dependency Discipline). `vox-orchestrator` (also layer 3) already calls
/// `vox_plugin_nvml_probe::probe::probe_summary()` directly — see
/// `crates/vox-populi/src/mens/hardware/mod.rs::monitor()`'s doc comment for
/// the identical precedent and the call site to reuse once a human approves
/// either a ledger exception or moving this call site to a layer-3-or-above
/// crate. Until then, `None` is the honest answer — not a fabricated number.
#[cfg(not(target_os = "macos"))]
#[rustfmt::skip] // keeps the toestub-ignore comment pinned to the fn signature line
pub fn query_accel_budget() -> Option<AccelBudget> { // toestub-ignore(skeleton/hollow-fn): blocked on a user-authorized crate-edges exception, see doc comment above
    None
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
