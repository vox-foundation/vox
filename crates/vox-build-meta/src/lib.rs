//! vox-build-meta is in deprecation. Its API is preserved as shims so existing
//! call sites compile, but every shim returns "no features available" and
//! `require` emits an error pointing at `vox plugin install <id>`.
//!
//! Direct callers should migrate to `vox-plugin-host`'s registry queries
//! (lands in SP2). Removal of this crate is scheduled for SP6.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

#[deprecated(note = "vox-build-meta is being removed; query vox-plugin-host instead. See plugin-system-redesign-2026.md")]
pub const FEATURES_JSON: &str = env!("VOX_BUILD_FEATURES");

#[allow(deprecated)]
pub fn active_features() -> Vec<&'static str> {
    serde_json::from_str(FEATURES_JSON).unwrap_or_default()
}

pub fn has(_feature: &str) -> bool {
    // Always false: feature stubs are retired. Capability presence now flows
    // through the plugin host registry.
    false
}

#[derive(Debug, thiserror::Error)]
#[error(
    "This Vox capability requires the '{feature}' plugin, which is not installed.\n\nTo install it, run:\n\n  {install_cmd}\n\nSee: docs/src/reference/plugins.md"
)]
pub struct FeatureMissingError {
    pub feature: &'static str,
    pub install_cmd: &'static str,
}

/// Deprecated. Always returns `Err`. The `install_cmd` argument is preserved
/// for backwards compatibility but should be the new `vox plugin install <id>`
/// invocation, not a `cargo build --features` invocation.
pub fn require(
    feature: &'static str,
    install_cmd: &'static str,
) -> Result<(), FeatureMissingError> {
    Err(FeatureMissingError {
        feature,
        install_cmd,
    })
}
