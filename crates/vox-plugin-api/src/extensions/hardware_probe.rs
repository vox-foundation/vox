//! HardwareProbe extension point — for hardware introspection plugins like
//! NVML / DRM / Metal / DXGI. Implementations live in plugins like
//! `vox-plugin-nvml-probe`.

use abi_stable::{sabi_trait, std_types::*};

pub const HARDWARE_PROBE_REVISION: u32 = 1;

#[sabi_trait]
pub trait HardwareProbe: Send + Sync {
    fn revision(&self) -> u32 {
        HARDWARE_PROBE_REVISION
    }

    /// Return a JSON-encoded summary of probed hardware. Plugin-defined
    /// schema; common fields include device count, names, memory, driver
    /// version, etc. The host doesn't interpret the JSON — it just passes
    /// it back to consumers.
    fn probe_summary_json(&self) -> RResult<RString, RBoxError>;

    /// Return JSON-encoded per-device metrics for monitoring (utilization,
    /// memory used, temperature, power). Called repeatedly during ops.
    fn device_metrics_json(&self) -> RResult<RString, RBoxError>;
}
