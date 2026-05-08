//! Built-in skills that are always available in the Vox skill registry.
//!
//! All builtin skills have been extracted to standalone plugin crates.
//! Skills are installed at runtime via `vox bundle apply <bundle>`.
//! This module is a no-op shim retained for API compatibility.

use crate::SkillError;
use crate::registry::SkillRegistry;

/// Install all built-in skills into the registry if they are not already present.
///
/// This is now a no-op (returns 0). Skills are discovered and installed from
/// the plugin data directory by the orchestrator bridge at startup.
pub async fn install_builtins(_registry: &SkillRegistry) -> Result<usize, SkillError> {
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn install_builtins_is_noop() {
        let reg = SkillRegistry::new();
        let count = install_builtins(&reg).await.expect("install");
        assert_eq!(count, 0, "no builtins to install; all are plugin-installed");
    }
}
