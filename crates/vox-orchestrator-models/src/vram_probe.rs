/// A host-supplied GPU probe. vox-orchestrator (L3) must not link the L4
/// `vox-plugin-nvml-probe` cdylib, so hosts that load plugins through
/// vox-plugin-host (vox-orchestrator-mcp, vox-cli, vox-gui) register one here.
/// Both functions return the plugin's `HardwareProbe` JSON:
/// `probe_summary_json` -> `{"devices":[{"vram_total_mb":..},..]}`,
/// `device_metrics_json` -> `{"metrics":[{"memory_free_mb":..},..]}`.
#[derive(Debug, Clone, Copy)]
pub struct VramProbe {
    pub probe_summary_json: fn() -> Result<String, String>,
    pub device_metrics_json: fn() -> Result<String, String>,
}
