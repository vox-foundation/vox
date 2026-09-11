use crate::mens::hardware::types::HardwareSummary;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

pub mod linux_drm;
pub mod macos_metal;
#[cfg(test)]
mod mock;
// nvml.rs deleted: NVML probing is owned by vox-plugin-nvml-probe.
pub mod pipeline;
pub mod probe;
pub mod registry;
#[cfg(test)]
mod tests;
pub mod types;
pub mod windows_fallback;

/// Default probe cache TTL. Re-probes after 5 minutes by default.
const DEFAULT_CACHE_TTL: Duration = vox_config::timeouts::D_300S;

static REGISTRY_V2: OnceLock<registry::HardwareRegistryV2> = OnceLock::new();

fn global_registry() -> &'static registry::HardwareRegistryV2 {
    REGISTRY_V2.get_or_init(|| registry::HardwareRegistryV2::new(DEFAULT_CACHE_TTL))
}

/// Global hardware registry for the Mens subsystem.
pub struct HardwareRegistry;

impl HardwareRegistry {
    /// Returns a cached hardware summary, re-probing if the cache has expired (default TTL: 5 min).
    pub async fn probe() -> Arc<HardwareSummary> {
        global_registry().probe().await
    }

    /// Monitors real-time telemetry (not cached).
    ///
    /// Metal (macOS): live via `MTLDevice.currentAllocatedSize` /
    /// `recommendedMaxWorkingSetSize` (see [`macos_metal::monitor_metal`]).
    ///
    /// CUDA: **not yet wired**. `vox-plugin-nvml-probe` (layer 3) reports this
    /// telemetry, but `vox-populi` is layer 2 — a static dependency would be an
    /// upward edge disallowed by the crate-layers "downward-only" rule (see
    /// `contracts/ci/crate-layers.v1.json`), and adding a `crate-edges`
    /// exception is user-authorized-only (see `AGENTS.md` §Dependency
    /// Discipline). `vox-orchestrator` (also layer 3) already calls
    /// `vox_plugin_nvml_probe::probe::device_metrics()` directly — see
    /// `crates/vox-orchestrator/src/models/vram.rs` for the pattern to reuse
    /// once a human approves either a ledger exception or moving this call
    /// site to a layer-3-or-above crate.
    pub fn monitor() -> Option<types::GpuTelemetry> {
        #[cfg(target_os = "macos")]
        {
            macos_metal::monitor_metal()
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    /// Invalidates the cache, forcing the next [`Self::probe`] call to re-probe.
    pub fn invalidate_cache() {
        global_registry().invalidate();
    }
}

/// Compatibility wrapper — returns a cached hardware summary.
pub async fn probe() -> Arc<HardwareSummary> {
    HardwareRegistry::probe().await
}

/// Probes with the full attempt log, bypassing the cache.
pub async fn probe_with_report() -> crate::mens::hardware::probe::ProbeReport {
    global_registry().probe_with_report().await
}
