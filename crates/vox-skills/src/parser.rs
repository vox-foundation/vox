//! SKILL.md format parser — re-exported from `vox-plugin-host`.
//!
//! The parser has moved to `vox_plugin_host::skill_parser`.
//! New code should import from `vox_plugin_host::skill_parser` directly.

pub use vox_plugin_host::skill_parser::{ParseSkillError, parse_skill_md};
