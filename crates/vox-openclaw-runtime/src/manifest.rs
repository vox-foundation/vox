//! ARS-facing manifest shapes (execution limits, trust classification, skill kind).

use serde::{Deserialize, Serialize};

/// Advisory resource envelope for sandboxed task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max wall-clock milliseconds (advisory; executor may ignore if unset).
    pub max_wall_ms: Option<u64>,
    /// Max captured output bytes (advisory; executor may ignore if unset).
    pub max_output_bytes: Option<u64>,
    /// Memory limit in MiB for container sandbox. Default: 256 MiB.
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u64,
    /// CPU quota (fractional cores) for container sandbox. Default: 0.5.
    #[serde(default = "default_cpu_quota")]
    pub cpu_quota: f32,
    /// Network access policy inside the container sandbox.
    #[serde(default)]
    pub network: NetworkPolicy,
}

fn default_memory_mb() -> u64 {
    256
}

fn default_cpu_quota() -> f32 {
    0.5
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_wall_ms: None,
            max_output_bytes: None,
            memory_mb: default_memory_mb(),
            cpu_quota: default_cpu_quota(),
            network: NetworkPolicy::None,
        }
    }
}

/// Network access policy for sandboxed skill execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPolicy {
    /// No network access (default for community skills).
    #[default]
    None,
    /// Loopback only (`127.0.0.1`).
    Loopback,
    /// Unrestricted — only for operator-pinned trusted skills.
    Unrestricted,
}

impl NetworkPolicy {
    /// Return the `--network` flag value for Docker/Podman.
    pub fn docker_flag(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Loopback => "host",
            Self::Unrestricted => "bridge",
        }
    }
}

/// High-level skill classification for the runtime harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillKind {
    /// Document-style skill (markdown instructions).
    #[default]
    Document,
    /// Executable / tool-backed skill (calls Vox MCP tools).
    Tool,
    /// Shell-execution skill — always requires `Container` isolation tier.
    Shell,
}

/// Trust classification for a skill.
///
/// Determines the minimum required isolation tier and whether an explicit
/// operator approval is needed before the skill may execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// Internal Vox builtins — runs with `Permissive` isolation.
    Trusted,
    /// Community skills (namespace `openclaw` or unverified) — requires
    /// explicit operator approval AND `Container` isolation.
    #[default]
    Community,
    /// Imported but not yet reviewed — execution blocked until promoted to `Community`
    /// via `vox openclaw approve`.
    Untrusted,
}

impl TrustLevel {
    /// Returns `true` if this trust level requires a pre-execution approval check.
    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::Community | Self::Untrusted)
    }

    /// Returns the minimum required isolation tier for this trust level.
    pub fn minimum_isolation(&self) -> &'static str {
        match self {
            Self::Trusted => "permissive",
            Self::Community | Self::Untrusted => "container",
        }
    }
}
