//! `VoxConfig` — Single Source of Truth for all Vox toolchain settings.
//!
//! Precedence (highest → lowest):
//!   ENV VARS > Vox.toml (workspace) > ~/.vox/config.toml (global) > compiled defaults
//!
//! CLI flags must be applied by the caller *after* calling `VoxConfig::load()`.
//! See: `docs/agents/config-hierarchy.md`

mod gamify_web;
mod impl_ops;
mod persist;
mod toml_schema;
mod vox_config;

pub use gamify_web::{GamifyMode, WebRunMode};
pub use vox_config::VoxConfig;
