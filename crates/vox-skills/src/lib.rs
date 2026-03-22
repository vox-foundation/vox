//! # vox-skills — Skill Marketplace and Plugin Architecture
//!
//! Provides a typed skill registry, skill bundle format parsing,
//! plugin lifecycle management, and an optional Vox Skills HTTP bridge.
//!
//! Submodules carry detailed docs; the crate root re-exports stable types for embedders.
#![allow(clippy::collapsible_if)]

/// Compile-time and first-run installation of embedded skills.
pub mod builtins;
/// In-memory skill bundle (`VoxSkillBundle`) and JSON (de)serialization.
pub mod bundle;
/// Lifecycle hook registry and event types.
pub mod hooks;
/// Skill manifest schema (`SkillManifest`, categories, permissions).
pub mod manifest;
/// Parse `SKILL.md` (YAML/TOML front matter + body) into a bundle.
pub mod parser;
/// Plugin trait, skill-backed plugins, and `PluginManager`.
pub mod plugin;
/// Shared `Arc<SkillRegistry>` factory for MCP and CLI surfaces.
pub mod provider;
/// Install, uninstall, search, and list skills (`SkillRegistry`).
pub mod registry;

#[cfg(feature = "skills-registry")]
/// HTTP client for the remote skills marketplace (`skills-registry` feature).
pub mod registry_api;

pub use builtins::install_builtins;
pub use bundle::{SkillBundle, VoxSkillBundle};
pub use hooks::{HookEvent, HookFn, HookRegistry};
pub use manifest::{SkillCategory, SkillManifest, SkillPermission};
pub use plugin::{Plugin, PluginKind, PluginManager};
pub use provider::new_registry_arc;
pub use registry::{InstallResult, SkillRegistry, UninstallResult};

/// The canonical Vox Skills marketplace registry URL.
pub const SKILLS_REGISTRY_BASE: &str =
    "https://raw.githubusercontent.com/brbrainerd/vox/main/skills";

/// Errors from the skill system.
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    /// Requested skill id is not present in the registry.
    #[error("Skill not found: {0}")]
    NotFound(String),
    /// Install attempted for an id that is already present (policy-dependent).
    #[error("Skill already installed: {0}")]
    AlreadyInstalled(String),
    /// Installed version does not match the requested version.
    #[error("Version conflict: installed={installed}, requested={requested}")]
    VersionConflict {
        /// Semver (or opaque string) currently installed.
        installed: String,
        /// Semver (or opaque string) that was requested.
        requested: String,
    },
    /// `SKILL.md` or manifest JSON failed validation.
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),
    /// Caller did not grant a required [`SkillPermission`].
    #[error("Permission denied: skill requires {0:?}")]
    PermissionDenied(SkillPermission),
    /// Underlying filesystem error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parse/serialize failure (bundle or API payload).
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// TOML parse failure (typically `SKILL.md` front matter).
    #[error("TOML error: {0}")]
    Toml(String),
    /// Registry HTTP client or non-success status.
    #[error("HTTP error: {0}")]
    Http(String),
    /// User-registered hook callback returned an error.
    #[error("Hook error: {0}")]
    Hook(String),
}
