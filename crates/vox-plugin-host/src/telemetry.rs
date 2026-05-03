//! Plugin lifecycle telemetry events. Per
//! docs/src/architecture/telemetry-trust-ssot.md, emitted via tracing.

use tracing::info;

pub fn discovered(id: &str, version: &str, payload_kind: &str, abi_or_format_version: u32) {
    info!(
        event = "plugin.discovered",
        id, version, payload_kind, abi_or_format_version,
    );
}

pub fn loaded(id: &str, version: &str, payload_kind: &str, load_ms: u128) {
    info!(event = "plugin.loaded", id, version, payload_kind, load_ms = %load_ms);
}

pub fn load_failed(id: &str, version: &str, error_kind: &str) {
    info!(event = "plugin.load_failed", id, version, error_kind);
}

pub fn abi_mismatch(id: &str, plugin_abi: u32, host_abi: u32) {
    info!(event = "plugin.abi_mismatch", id, plugin_abi, host_abi);
}
