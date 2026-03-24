//! Domain model for skills inside the ARS harness.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::manifest::{ResourceLimits, SkillKind};

/// Skill payload used by [`crate::runtime::ArsRuntime`] (distinct from OpenClaw list/import DTOs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArsSkill {
    /// Stable skill id.
    pub id: String,
    /// Logical namespace (e.g. `vox`, `local`).
    pub namespace: String,
    /// Display name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Content-addressable hash when known.
    pub content_hash: String,
    /// Short description.
    pub description: Option<String>,
    /// Author label.
    pub author: Option<String>,
    /// Opaque metadata JSON.
    pub metadata: Value,
    /// Skill kind discriminator.
    pub kind: SkillKind,
    /// Optional markdown / instruction body.
    pub body: Option<String>,
    /// Advisory resource limits.
    pub resource_limits: ResourceLimits,
}
