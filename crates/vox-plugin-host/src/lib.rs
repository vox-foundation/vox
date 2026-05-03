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
