//! Built-in skills that are always available in the Vox skill registry.
//!
//! All builtin skills have been extracted to standalone plugin crates
//! (SP6). The `BUILTIN_SKILLS` array is now empty; skill content lives in:
//!
//!   crates/vox-plugin-skill-git/
//!   crates/vox-plugin-skill-memory/
//!   crates/vox-plugin-skill-orchestrator/
//!   crates/vox-plugin-skill-rag/
//!   crates/vox-plugin-skill-testing/
//!   crates/vox-plugin-skill-testing-validate/
//!   crates/vox-plugin-skill-v0/
//!   crates/vox-plugin-skill-compiler/        (extracted in SP4)
//!   crates/vox-plugin-populi-mesh/           (composite plugin; includes populi skill)
//!
//! Skills are installed at runtime via `vox bundle apply <bundle>`.

use crate::SkillError;
use crate::bundle::VoxSkillBundle;
use crate::registry::SkillRegistry;

/// All built-in skill SKILL.md contents.
///
/// Empty: all skills are now standalone plugins installed at runtime.
#[allow(dead_code)]
const BUILTIN_SKILLS: &[(&str, &str)] = &[];

/// Install all built-in skills into the registry if they are not already present.
///
/// This is now a no-op (returns 0). Skills are discovered and installed from
/// the plugin data directory by the orchestrator bridge at startup.
pub async fn install_builtins(_registry: &SkillRegistry) -> Result<usize, SkillError> {
    Ok(0)
}

/// Return all built-in skill bundles without installing them.
pub fn builtin_bundles() -> Result<Vec<VoxSkillBundle>, SkillError> {
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_builtins_remain() {
        let bundles = builtin_bundles().expect("parse all built-ins");
        assert_eq!(bundles.len(), 0, "all skills are now standalone plugins");
    }

    #[tokio::test]
    async fn install_builtins_is_noop() {
        let reg = SkillRegistry::new();
        let count = install_builtins(&reg).await.expect("install");
        assert_eq!(count, 0, "no builtins to install; all are plugin-installed");
    }

    #[tokio::test]
    async fn install_builtins_is_idempotent() {
        let reg = SkillRegistry::new();
        install_builtins(&reg).await.expect("first install");
        let count = install_builtins(&reg).await.expect("second install");
        assert_eq!(count, 0, "idempotent no-op");
    }
}
