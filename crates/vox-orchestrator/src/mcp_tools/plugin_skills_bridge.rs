//! Bridges newly-discovered plugin-host skills into the existing
//! vox-skills SkillRegistry. Transitional — SP6 will retire vox-skills
//! entirely and route all skill lookups through vox-plugin-host directly.

use std::path::Path;
use std::sync::Arc;

/// Discover all skill plugins under `install_dir` and register each into
/// `registry`. Logs and ignores discover errors so a missing/empty install
/// dir doesn't crash the orchestrator.
pub async fn install_discovered_skills(
    registry: &Arc<vox_skills::SkillRegistry>,
    install_dir: &Path,
) {
    let plugin_registry = match vox_plugin_host::discover(install_dir) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("plugin-host discover failed at {install_dir:?}: {e}");
            return;
        }
    };
    for skill_id in plugin_registry.skills.list_ids() {
        let loaded = match plugin_registry.skills.lookup(&skill_id) {
            Ok(l) => l,
            Err(_) => continue,
        };
        // `loaded.body` contains the full SKILL.md (frontmatter + body) as written
        // to disk; parse_skill_md reconstructs a typed VoxSkillBundle from it.
        let bundle = match vox_skills::parser::parse_skill_md(&loaded.body) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    "plugin-host skill '{skill_id}': failed to parse SKILL.md: {e}"
                );
                continue;
            }
        };
        match registry.install(&bundle).await {
            Ok(result) if result.already_installed => {
                tracing::debug!(
                    skill = %skill_id,
                    "plugin-host skill already installed at same version, skipping"
                );
            }
            Ok(_) => {
                tracing::info!(skill = %skill_id, "Registered plugin-host skill into vox-skills registry");
            }
            Err(e) => {
                tracing::warn!("failed to install plugin-host skill '{skill_id}': {e}");
            }
        }
    }
}
