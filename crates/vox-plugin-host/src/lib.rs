//! Vox plugin host: discovery, loading, registry.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

pub mod discover;
pub mod errors;
pub mod host_impl;
pub mod loader;
pub mod registry;
pub mod skill_bundle;
pub mod skill_manifest;
pub mod skill_parser;
pub mod skill_registry;
pub mod telemetry;

pub use discover::discover;
pub use errors::{AbiMismatchError, LoadError, PluginMissingError, SkillNotInstalledError};
pub use host_impl::DefaultVoxHost;
pub use loader::{LoadedCodePlugin, Loader};
pub use registry::{PluginEntry, Registry};
pub use skill_bundle::{SkillBundle, SkillBundleError, VoxSkillBundle};
pub use skill_manifest::{SkillCategory, SkillManifest, SkillPermission};
pub use skill_parser::{ParseSkillError, parse_skill_md};
pub use skill_registry::SkillRegistry;
pub use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;

/// Resolve the plugin install root, respecting `$VOX_PLUGINS_DIR` if set.
/// Falls back to the platform's local data directory under `vox/plugins`.
pub fn resolve_plugins_root() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("VOX_PLUGINS_DIR") {
        return std::path::PathBuf::from(p);
    }
    dirs::data_local_dir()
        .map(|p| p.join("vox").join("plugins"))
        .unwrap_or_else(|| std::path::PathBuf::from("./vox-plugins"))
}

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

/// Convenience wrapper: discover the plugin install root, build the registry, and load a
/// code plugin by id in a single call.
///
/// For one-off dispatches from async contexts, wrap in `tokio::task::spawn_blocking`.
pub fn load_code_plugin_by_id(plugin_id: &str) -> Result<LoadedCodePlugin, errors::LoadError> {
    let install_root = resolve_plugins_root();
    let registry = discover(&install_root)?;
    load_code_plugin(&registry, plugin_id)
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

/// Cached singleton: load a code plugin once and reuse the handle process-wide.
/// First call: discover + dlopen (tens of ms). Subsequent calls: O(1) HashMap lookup.
///
/// The plugin is leaked for the process lifetime (`Box::leak`) — code plugins are
/// designed to never unload while the host is running. Designed for plugins called
/// repeatedly (browser, mesh, ml backends).
pub fn cached_code_plugin(
    plugin_id: &'static str,
) -> Result<&'static LoadedCodePlugin, errors::LoadError> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type Cache = Mutex<HashMap<&'static str, &'static LoadedCodePlugin>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap();
    if let Some(p) = guard.get(plugin_id) {
        return Ok(p);
    }
    let loaded = load_code_plugin_by_id(plugin_id)?;
    let leaked: &'static LoadedCodePlugin = Box::leak(Box::new(loaded));
    guard.insert(plugin_id, leaked);
    Ok(leaked)
}
