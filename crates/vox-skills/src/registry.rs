//! Skill registry — thin re-export shim.
//!
//! The authoritative implementation has moved to `vox_plugin_host::SkillRegistry`.
//! This module re-exports everything so existing `vox_skills::SkillRegistry` callers
//! keep compiling without any changes.

pub use vox_plugin_host::skill_registry::{
    BundleInstallError as InstallError,
    InstallResult,
    SkillRegistry,
    UninstallResult,
    new_registry_arc,
};

// SkillError-compatible conversion: map BundleInstallError -> SkillError::Http
impl From<vox_plugin_host::skill_registry::BundleInstallError> for crate::SkillError {
    fn from(e: vox_plugin_host::skill_registry::BundleInstallError) -> Self {
        crate::SkillError::Http(e.0)
    }
}
impl From<vox_plugin_host::skill_registry::UninstallError> for crate::SkillError {
    fn from(e: vox_plugin_host::skill_registry::UninstallError) -> Self {
        crate::SkillError::Http(e.0)
    }
}
impl From<vox_plugin_host::skill_registry::HydrateError> for crate::SkillError {
    fn from(e: vox_plugin_host::skill_registry::HydrateError) -> Self {
        crate::SkillError::Http(e.0)
    }
}
