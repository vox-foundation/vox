//! Skill execution policy resolution — maps trust level and kind to isolation tier.
//!
//! Two gates are enforced:
//! 1. **Approval gate** (`ApprovalGuard`): community/untrusted skills need explicit
//!    operator approval stored in Arca before they may execute.
//! 2. **Isolation gate** (`resolve_policy`): determines whether to run under the
//!    container sandbox or the permissive (host-process) runtime.

use crate::ars_shim::manifest::{SkillKind, TrustLevel};

/// Error returned when a pre-execution policy gate blocks execution.
#[derive(Debug, thiserror::Error)]
pub enum PolicyError {
    /// Skill has not been approved by an operator.
    #[error("Skill '{id}' requires explicit operator approval. Run `vox openclaw approve {id}`")]
    NotApproved { id: String },
    /// Skill is in Untrusted state and cannot be promoted automatically.
    #[error("Skill '{id}' is in Untrusted state. Review and approve via `vox openclaw approve {id}`")]
    Untrusted { id: String },
}

/// Which isolation tier to apply for a skill execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxPolicy {
    /// Execute inside the OCI container sandbox (`vox-skill-sandbox:latest`).
    Container,
    /// Execute directly in the host process (trusted builtins only).
    Permissive,
}

impl SandboxPolicy {
    /// Returns `true` if this policy requires a container runtime.
    pub fn requires_container(self) -> bool {
        matches!(self, Self::Container)
    }
}

/// Checks whether an operator approval entry exists for the skill id.
///
/// In the current implementation this probes the `skill_approvals` key in the
/// in-process skill registry metadata.  When a full Arca integration is wired,
/// the check will query the `ars_approvals` Arca table.
pub struct ApprovalGuard;

impl ApprovalGuard {
    /// Check whether `skill_id` is approved for execution.
    ///
    /// `approved` must be resolved by the caller from the Arca approval store
    /// (e.g. `ArsRuntime` queries `SELECT 1 FROM ars_approvals WHERE skill_id = ?`).
    ///
    /// Returns `Ok(())` when the gate passes, `Err(PolicyError::NotApproved)` otherwise.
    pub fn check(skill_id: &str, trust: TrustLevel, approved: bool) -> Result<(), PolicyError> {
        match trust {
            TrustLevel::Trusted => Ok(()),
            TrustLevel::Untrusted => Err(PolicyError::Untrusted {
                id: skill_id.to_string(),
            }),
            TrustLevel::Community => {
                if approved {
                    Ok(())
                } else {
                    Err(PolicyError::NotApproved {
                        id: skill_id.to_string(),
                    })
                }
            }
        }
    }
}

/// Resolve the [`SandboxPolicy`] for a skill based on its trust level and kind.
///
/// Rules:
/// - `TrustLevel::Trusted` → `Permissive` (builtins run on host)
/// - `TrustLevel::Community` or `TrustLevel::Untrusted` → `Container`
/// - `SkillKind::Shell` always → `Container` regardless of trust
pub fn resolve_policy(kind: SkillKind, trust: TrustLevel) -> SandboxPolicy {
    if matches!(trust, TrustLevel::Trusted) && !matches!(kind, SkillKind::Shell) {
        SandboxPolicy::Permissive
    } else {
        SandboxPolicy::Container
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_document_skill_is_permissive() {
        assert_eq!(
            resolve_policy(SkillKind::Document, TrustLevel::Trusted),
            SandboxPolicy::Permissive
        );
    }

    #[test]
    fn trusted_shell_skill_still_requires_container() {
        assert_eq!(
            resolve_policy(SkillKind::Shell, TrustLevel::Trusted),
            SandboxPolicy::Container
        );
    }

    #[test]
    fn community_tool_skill_requires_container() {
        assert_eq!(
            resolve_policy(SkillKind::Tool, TrustLevel::Community),
            SandboxPolicy::Container
        );
    }

    #[test]
    fn approval_gate_blocks_community_without_approval() {
        let result = ApprovalGuard::check("my-skill", TrustLevel::Community, false);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("my-skill"));
    }

    #[test]
    fn approval_gate_passes_community_with_approval() {
        assert!(ApprovalGuard::check("my-skill", TrustLevel::Community, true).is_ok());
    }

    #[test]
    fn approval_gate_always_passes_for_trusted() {
        assert!(ApprovalGuard::check("builtin", TrustLevel::Trusted, false).is_ok());
    }

    #[test]
    fn approval_gate_blocks_untrusted_regardless_of_flag() {
        assert!(ApprovalGuard::check("new-skill", TrustLevel::Untrusted, true).is_err());
    }
}
