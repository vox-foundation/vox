//! Plugin-to-skills bridge stub.
//!
//! Full implementation planned for SP6 (plugin system redesign Phase 2).
//! The `install_discovered_skills` entry-point is generic over the registry type
//! so it can compile without a production dep on `vox-skills`.

use std::path::Path;
use std::sync::Arc;

/// Discover plugins under `plugins_root` and register them into `registry`.
///
/// # PHASE_0a_STUB
/// Body is a no-op. Phase SP6 will walk `plugins_root` for `Plugin.toml` manifests,
/// parse their `SKILL.md` frontmatter via `vox_plugin_host::skill_parser`, and call
/// `registry.register(manifest)` for each discovered skill.
pub async fn install_discovered_skills<R>(_registry: &Arc<R>, _plugins_root: &Path) {
    // PHASE_0a_STUB: no-op until SP6 bridge is implemented.
}
