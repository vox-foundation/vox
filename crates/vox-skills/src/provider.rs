//! Shared [`SkillRegistry`] construction for MCP, CLI, and ARS embedders.
//!
//! The canonical implementation is in `vox_plugin_host::skill_registry::new_registry_arc`.
//! This re-export exists so existing `vox_skills::new_registry_arc` call sites keep compiling.

pub use vox_plugin_host::skill_registry::new_registry_arc;
