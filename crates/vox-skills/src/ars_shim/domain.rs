//! Domain model for skills inside the ARS harness.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ars_shim::manifest::{ResourceLimits, SkillKind, TrustLevel};

/// Skill payload used by [`crate::ars_shim::runtime::ArsRuntime`] (distinct from OpenClaw list/import DTOs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArsSkill {
    /// Stable skill id.
    pub id: String,
    /// Logical namespace (e.g. `vox`, `local`, `openclaw`).
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
    /// Advisory resource limits (used by the container sandbox runner).
    pub resource_limits: ResourceLimits,
    /// Trust classification — drives isolation tier and approval gate.
    ///
    /// Defaults to [`TrustLevel::Community`] for `namespace == "openclaw"` skills;
    /// callers constructing internal builtins should set [`TrustLevel::Trusted`].
    #[serde(default)]
    pub trust: TrustLevel,
}

impl ArsSkill {
    /// Returns `true` if this skill requires an explicit operator approval
    /// before execution is permitted.
    pub fn requires_approval(&self) -> bool {
        self.trust.requires_approval()
    }

    /// Returns `true` if this skill must execute inside the container sandbox.
    pub fn requires_container(&self) -> bool {
        matches!(self.trust, TrustLevel::Community | TrustLevel::Untrusted)
            || matches!(self.kind, SkillKind::Shell)
    }
}
