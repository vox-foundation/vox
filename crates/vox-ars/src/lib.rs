//! # vox-ars — Agent Runtime Shell (ARS)
//!
//! OpenClaw gateway client, in-process skill runtime harness, and retrieval context bundles.
//! Skill install/list reuses [`vox_skills::SkillRegistry`]; SKILL.md parsing reuses [`vox_skills`].

pub mod context;
pub mod domain;
pub mod executor;
pub mod hooks;
pub mod manifest;
pub mod openclaw;
pub mod runtime;

/// SKILL.md parsing — delegates to [`vox_skills::parser`].
pub mod parser {
    pub use vox_skills::parser::parse_skill_md;
}

pub use domain::ArsSkill;
pub use openclaw::{OpenClawClient, OpenClawRemoteConfig, OpenClawSkillSpec};
pub use vox_skills::manifest::{SkillCategory, SkillManifest, SkillPermission};
pub use vox_skills::{SkillRegistry, install_builtins};
