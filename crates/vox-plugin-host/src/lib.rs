//! Vox plugin host: discovery, loading, registry.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

pub mod discover;
pub mod errors;
pub mod host_impl;
pub mod loader;
pub mod registry;
pub mod skill_registry;
pub mod telemetry;

pub use discover::discover;
pub use errors::{AbiMismatchError, LoadError, PluginMissingError, SkillNotInstalledError};
pub use host_impl::DefaultVoxHost;
pub use loader::{LoadedCodePlugin, Loader};
pub use registry::{PluginEntry, Registry};
pub use skill_registry::SkillRegistry;

/// Return the target-triple key used in `[plugin.payload.artifacts]` for the current build.
///
/// The format is `"<os>-<arch>"` where `os` is `"windows"`, `"linux"`, or `"macos"` and
/// `arch` is `"x86_64"` or `"aarch64"`.  This matches the keys emitted by the Plugin.toml
/// generator and by `vox plugin install`.
pub fn current_target_triple_key() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "windows-x86_64";
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    return "windows-aarch64";
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x86_64";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "linux-aarch64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "macos-x86_64";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "macos-aarch64";
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    return "unknown";
}

/// Discover the given plugin in `registry`, resolve the dylib path for the current target
/// triple, and load it via [`Loader`].
///
/// This is the preferred one-shot entry point for code-payload plugins.  Callers can then
/// call `.plugin.as_ml_backend()` (or the relevant extension point accessor) on the
/// returned [`LoadedCodePlugin`].
pub fn load_code_plugin(
    registry: &Registry,
    plugin_id: &str,
) -> Result<LoadedCodePlugin, errors::LoadError> {
    use vox_plugin_api::manifest::PluginPayload;

    let entry = registry
        .get_full_entry(plugin_id)
        .ok_or_else(|| errors::LoadError::InitFailed(format!(
            "plugin '{plugin_id}' is not installed — run `vox plugin install {plugin_id}`"
        )))?;

    let triple = current_target_triple_key();
    let artifacts = match &entry.payload {
        PluginPayload::Code(c) => &c.artifacts,
        PluginPayload::Composite(c) => &c.code.artifacts,
        PluginPayload::Skill(_) => {
            return Err(errors::LoadError::InitFailed(format!(
                "plugin '{plugin_id}' is a skill-only plugin and cannot be loaded as a code plugin"
            )));
        }
    };

    let filename = artifacts.get(triple).ok_or_else(|| {
        errors::LoadError::InitFailed(format!(
            "plugin '{plugin_id}' has no artifact for target triple '{triple}' \
             (available: {:?})",
            artifacts.keys().collect::<Vec<_>>()
        ))
    })?;

    let dylib_path = entry.install_dir.join(filename);
    Loader::load(&entry.id, &entry.version, &dylib_path)
}
