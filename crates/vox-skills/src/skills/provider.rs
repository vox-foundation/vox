//! Shared [`SkillRegistry`] construction for MCP, CLI, and ARS embedders.

use std::sync::Arc;

use crate::SkillRegistry;

/// Returns a **new** empty registry in an [`Arc`]; not a process singleton (each call allocates).
#[must_use]
pub fn new_registry_arc() -> Arc<SkillRegistry> {
    Arc::new(SkillRegistry::new())
}
