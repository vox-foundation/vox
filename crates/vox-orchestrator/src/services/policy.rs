//! Policy engine: scope and lock checks before queueing tasks.
//!
//! Validates that an agent can acquire required locks and (optionally)
//! that writes fall within the agent's scope. Call before enqueueing
//! to fail fast and emit scope violations.

use crate::events::EventBus;
use crate::locks::{FileLockManager, LockConflict, LockKind};
use crate::scope::{ScopeCheckResult, ScopeGuard};
use crate::types::{AccessKind, AgentId, AgentTask, CompletionAttestation, FileAffinity};

/// Result of a policy check before queueing a task.
#[derive(Debug, Clone)]
pub enum PolicyCheckResult {
    /// All checks passed; safe to enqueue.
    Allowed,
    /// A required lock could not be acquired.
    LockConflict(LockConflict),
    /// Scope guard denied write to a path (with reason).
    ScopeDenied(String),
}

impl PolicyCheckResult {
    /// Returns true if the operation is allowed to proceed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed)
    }
}

/// Stateless policy engine for pre-queue validation.
pub struct PolicyEngine;

impl PolicyEngine {
    fn has_placeholder_marker(text: &str) -> bool {
        let lower = text.to_ascii_lowercase();
        [
            "todo",
            "tbd",
            "placeholder",
            "stub",
            "not implemented",
            "coming soon",
        ]
        .iter()
        .any(|m| lower.contains(m))
    }

    /// Completion policy for no-write tasks: require a concrete attestation and reject
    /// obvious placeholder markers unless `force_risky` is explicitly set with a reason.
    pub fn check_completion_before_complete(
        task: Option<&AgentTask>,
        attestation: Option<&CompletionAttestation>,
    ) -> PolicyCheckResult {
        let Some(task) = task else {
            return PolicyCheckResult::Allowed;
        };
        if !task.write_files().is_empty() {
            return PolicyCheckResult::Allowed;
        }

        let Some(att) = attestation else {
            return PolicyCheckResult::ScopeDenied(
                "Completion policy denied: no-write task requires completion attestation"
                    .to_string(),
            );
        };
        if att.force_risky {
            let reason_ok = match &att.force_risky_reason {
                Some(r) => !r.trim().is_empty(),
                None => false,
            };
            if !reason_ok {
                return PolicyCheckResult::ScopeDenied(
                    "Completion policy denied: force_risky requires non-empty force_risky_reason"
                        .to_string(),
                );
            }
            return PolicyCheckResult::Allowed;
        }
        if !att.declared_non_placeholder {
            return PolicyCheckResult::ScopeDenied(
                "Completion policy denied: declared_non_placeholder must be true for no-write tasks"
                    .to_string(),
            );
        }
        let summary_ok = match &att.completion_summary {
            Some(s) => s.trim().len() >= 24 && !Self::has_placeholder_marker(s),
            None => false,
        };
        if !summary_ok {
            return PolicyCheckResult::ScopeDenied(
                "Completion policy denied: completion_summary is missing/too short or includes placeholder markers".to_string(),
            );
        }
        let has_evidence = !att.artifact_paths.is_empty() || !att.checks_passed.is_empty();
        if !has_evidence {
            return PolicyCheckResult::ScopeDenied(
                "Completion policy denied: no-write task requires artifact_paths or checks_passed evidence".to_string(),
            );
        }
        PolicyCheckResult::Allowed
    }

    /// Check whether the agent can acquire locks for all write files.
    /// Does not actually acquire locks; use for dry-run or pre-check.
    pub fn check_locks(
        lock_manager: &FileLockManager,
        manifest: &[FileAffinity],
        agent_id: AgentId,
    ) -> PolicyCheckResult {
        for fa in manifest {
            if fa.access != AccessKind::Write {
                continue;
            }
            if let Err(e) = lock_manager.try_acquire(&fa.path, agent_id, LockKind::Exclusive) {
                return PolicyCheckResult::LockConflict(e);
            }
        }
        PolicyCheckResult::Allowed
    }

    /// Run both lock and scope checks. Pass None for scope_guard to skip scope checks.
    /// When scope_guard is Some, pass event_bus so that ScopeViolation events are emitted.
    pub fn check_before_queue(
        lock_manager: &FileLockManager,
        scope_guard: Option<&ScopeGuard>,
        event_bus: &EventBus,
        manifest: &[FileAffinity],
        agent_id: AgentId,
    ) -> PolicyCheckResult {
        if let PolicyCheckResult::LockConflict(e) =
            Self::check_locks(lock_manager, manifest, agent_id)
        {
            return PolicyCheckResult::LockConflict(e);
        }
        if let Some(guard) = scope_guard {
            for fa in manifest {
                if fa.access != AccessKind::Write {
                    continue;
                }
                let result = guard.check_write(agent_id, &fa.path, event_bus);
                if !result.is_allowed() {
                    let reason = match &result {
                        ScopeCheckResult::Warned(s) => s.clone(),
                        ScopeCheckResult::Denied(s) => s.clone(),
                        _ => String::new(),
                    };
                    return PolicyCheckResult::ScopeDenied(reason);
                }
            }
        }
        PolicyCheckResult::Allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentTask, CompletionAttestation, TaskId, TaskPriority};
    use std::path::PathBuf;

    #[test]
    fn check_before_queue_lock_conflict_when_other_agent_holds() {
        let lock_manager = FileLockManager::new();
        let event_bus = EventBus::new(16);
        let path = PathBuf::from("src/foo.rs");
        let manifest = vec![FileAffinity::write(&path)];
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        let _ = lock_manager.try_acquire(&path, a1, LockKind::Exclusive);
        let r = PolicyEngine::check_before_queue(&lock_manager, None, &event_bus, &manifest, a2);
        assert!(!r.is_allowed());
        assert!(matches!(r, PolicyCheckResult::LockConflict(_)));
    }

    #[test]
    fn check_before_queue_allowed_when_same_agent_reentrant() {
        let lock_manager = FileLockManager::new();
        let event_bus = EventBus::new(16);
        let path = PathBuf::from("src/bar.rs");
        let manifest = vec![FileAffinity::write(&path)];
        let a1 = AgentId(1);
        let _ = lock_manager.try_acquire(&path, a1, LockKind::Exclusive);
        let r = PolicyEngine::check_before_queue(&lock_manager, None, &event_bus, &manifest, a1);
        assert!(r.is_allowed());
    }

    #[test]
    fn no_write_completion_requires_attestation() {
        let task = AgentTask::new(TaskId(1), "no write task", TaskPriority::Normal, vec![]);
        let r = PolicyEngine::check_completion_before_complete(Some(&task), None);
        assert!(!r.is_allowed());
    }

    #[test]
    fn no_write_completion_allows_valid_attestation() {
        let task = AgentTask::new(TaskId(2), "no write task", TaskPriority::Normal, vec![]);
        let att = CompletionAttestation {
            completion_summary: Some(
                "Validated documentation output and emitted artifacts.".into(),
            ),
            checks_passed: vec!["schema-verify".into()],
            artifact_paths: vec![],
            declared_non_placeholder: true,
            force_risky: false,
            force_risky_reason: None,
        };
        let r = PolicyEngine::check_completion_before_complete(Some(&task), Some(&att));
        assert!(r.is_allowed());
    }
}
