//! Registers vox-plugin-nvml-probe's `HardwareProbe` as vox-orchestrator's GPU
//! probe (Phase 3 D-09 follow-up). vox-orchestrator (L3) no longer links the L4
//! plugin; the host loads it through vox-plugin-host instead. When the plugin is
//! not installed or exposes no `HardwareProbe`, nothing is registered and VRAM
//! scoring keeps its no-signal fallback.

use std::sync::OnceLock;
use vox_orchestrator::models::{VramProbe, register_vram_probe};

/// Plugin id from `crates/vox-plugin-nvml-probe/Plugin.toml`.
const PLUGIN_ID: &str = "nvml-probe";

/// Calls one `HardwareProbe` method on the (cached) loaded plugin.
macro_rules! call_hardware_probe {
    ($method:ident) => {{
        let plugin = vox_plugin_host::cached_code_plugin(PLUGIN_ID)
            .map_err(|e| format!("{PLUGIN_ID} plugin load: {e}"))?;
        let probe = plugin
            .plugin
            .as_hardware_probe()
            .into_option()
            .ok_or_else(|| format!("{PLUGIN_ID} plugin has no HardwareProbe"))?;
        probe
            .$method()
            .into_result()
            .map(|json| json.into_string())
            .map_err(|e| e.to_string())
    }};
}

fn probe_summary_json() -> Result<String, String> {
    call_hardware_probe!(probe_summary_json)
}

fn device_metrics_json() -> Result<String, String> {
    call_hardware_probe!(device_metrics_json)
}

/// Loads the nvml-probe plugin once per process and, if it exposes a
/// `HardwareProbe`, registers it with vox-orchestrator. Later calls are no-ops,
/// so every host (MCP server, CLI, GUI) can call this before model selection.
pub fn register_nvml_vram_probe() {
    static DONE: OnceLock<bool> = OnceLock::new();
    DONE.get_or_init(|| {
        register_with(|| {
            vox_plugin_host::cached_code_plugin(PLUGIN_ID)
                .map(|p| p.plugin.as_hardware_probe().is_rsome())
                .map_err(|e| e.to_string())
        })
    });
}

/// `load` reports whether the plugin loaded (`Err` = it did not) and whether it
/// has a `HardwareProbe`. Registers only on `Ok(true)`; logs the reason otherwise.
fn register_with(load: impl FnOnce() -> Result<bool, String>) -> bool {
    match load() {
        Ok(true) => {
            register_vram_probe(VramProbe {
                probe_summary_json,
                device_metrics_json,
            });
            true
        }
        Ok(false) => {
            tracing::warn!(
                target: "vox.mcp.vram_probe",
                "{PLUGIN_ID} plugin exposes no HardwareProbe; VRAM-fit signal stays off"
            );
            false
        }
        Err(e) => {
            tracing::debug!(
                target: "vox.mcp.vram_probe",
                error = %e,
                "{PLUGIN_ID} plugin not loaded; VRAM-fit signal stays off"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_only_when_the_plugin_loads_with_a_hardware_probe() {
        assert!(
            !register_with(|| Err("not installed".into())),
            "a load failure must leave the probe unregistered"
        );
        assert!(
            !register_with(|| Ok(false)),
            "a plugin without HardwareProbe must leave the probe unregistered"
        );
        assert!(register_with(|| Ok(true)));
        let probe = vox_orchestrator::models::registered_vram_probe().expect("registered");
        type ProbeFn = fn() -> Result<String, String>;
        assert!(std::ptr::fn_addr_eq(
            probe.probe_summary_json,
            probe_summary_json as ProbeFn
        ));
        assert!(std::ptr::fn_addr_eq(
            probe.device_metrics_json,
            device_metrics_json as ProbeFn
        ));
    }
}
