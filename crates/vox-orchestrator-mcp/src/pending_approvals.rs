//! B3 HITL: in-memory pending-approval registry shared across in-process MCP
//! dispatch (`tasks_core.rs`) and daemon RPC (`daemon_extra.rs`), supporting
//! restart visibility (`reregister_after_restart`), rich terminal resolution
//! (`PendingResolution`), and boundary-safe session `Ask`-tier allowlists.

use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::oneshot;
use vox_orchestrator::ApprovalOutcome;

/// Opaque id for one pending approval (e.g. `"AP-000001"`).
pub type ApprovalId = String;

/// Optional terminal-specific metadata attached to a parked HITL approval.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingApprovalMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_prompt: Option<bool>,
}

/// Operator-facing description of a pending approval (returned by `list`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingApprovalInfo {
    /// Stable id used to resolve this approval.
    pub approval_id: ApprovalId,
    /// Canonical MCP tool name awaiting approval.
    pub tool: String,
    /// Short human-readable summary of what will run.
    pub summary: String,
    /// When the approval was requested (unix-ms).
    pub requested_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_prompt: Option<bool>,
}

/// Detailed operator resolution payload for `Edit & Run`, `Allow Pattern`, and `Reject + Steer`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PendingResolution {
    pub outcome: ApprovalOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steering_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_pattern: Option<String>,
}

impl PendingResolution {
    #[must_use]
    pub fn simple(outcome: ApprovalOutcome) -> Self {
        Self {
            outcome,
            modified_command: None,
            steering_note: None,
            allow_pattern: None,
        }
    }
}

/// Parse optional `modified_command`, `steering_note`, and `allow_pattern` from JSON params,
/// rejecting `ApprovalOutcome::Modified` when `modified_command` is missing or blank.
pub fn parse_resolution_from_params(
    outcome: ApprovalOutcome,
    params: &serde_json::Value,
) -> Result<PendingResolution, &'static str> {
    let opt_str = |key: &str| {
        params
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
    };
    let modified_command = opt_str("modified_command");
    if outcome == ApprovalOutcome::Modified && modified_command.is_none() {
        return Err("outcome 'modified' requires a non-empty 'modified_command'");
    }
    Ok(PendingResolution {
        outcome,
        modified_command,
        steering_note: opt_str("steering_note"),
        allow_pattern: opt_str("allow_pattern"),
    })
}

enum PendingWaiter {
    Simple(oneshot::Sender<ApprovalOutcome>),
    Detailed(oneshot::Sender<PendingResolution>),
}

const MAX_SESSION_PATTERNS: usize = 64;
const BAD_PATTERN_CHARS: [char; 10] = ['*', ';', '&', '|', '`', '$', '>', '<', '\n', '\r'];

#[derive(Default)]
struct Inner {
    next: u64,
    waiters: HashMap<ApprovalId, PendingWaiter>,
    pending: Vec<PendingApprovalInfo>,
    session_allowlist: HashMap<String, Vec<String>>,
}

/// Shared registry of approvals awaiting a human decision.
#[derive(Default)]
pub struct PendingApprovals {
    inner: Mutex<Inner>,
}

impl PendingApprovals {
    fn insert_session_pattern(
        allowlist: &mut HashMap<String, Vec<String>>,
        session_id: &str,
        raw_pattern: &str,
    ) -> bool {
        let sid = session_id.trim();
        let pat = raw_pattern.trim();
        if sid.is_empty() || pat.chars().count() < 2 || pat.contains(BAD_PATTERN_CHARS) {
            return false;
        }
        let entries = allowlist.entry(sid.to_owned()).or_default();
        if entries.iter().any(|existing| existing == pat) {
            return true;
        }
        if entries.len() >= MAX_SESSION_PATTERNS {
            return false;
        }
        entries.push(pat.to_owned());
        true
    }

    fn build_info(
        approval_id: ApprovalId,
        tool: String,
        summary: String,
        requested_at_ms: u64,
        m: PendingApprovalMeta,
    ) -> PendingApprovalInfo {
        PendingApprovalInfo {
            approval_id,
            tool,
            summary,
            requested_at_ms,
            floor_category: m.floor_category,
            session_id: m.session_id,
            command: m.command,
            control_mode: m.control_mode,
            credential_prompt: m.credential_prompt,
        }
    }

    /// Register a new pending approval returning its id and outcome receiver.
    pub fn register(
        &self,
        tool: String,
        summary: String,
        requested_at_ms: u64,
    ) -> (ApprovalId, oneshot::Receiver<ApprovalOutcome>) {
        let (tx, rx) = oneshot::channel();
        let mut g = self.inner.lock().expect("pending-approvals lock");
        g.next += 1;
        let id = format!("AP-{:06}", g.next);
        g.waiters.insert(id.clone(), PendingWaiter::Simple(tx));
        let info = Self::build_info(
            id.clone(),
            tool,
            summary,
            requested_at_ms,
            Default::default(),
        );
        g.pending.push(info);
        (id, rx)
    }

    /// Register a pending approval enriched with terminal context and a detailed resolution receiver.
    pub fn register_with_meta(
        &self,
        tool: String,
        summary: String,
        requested_at_ms: u64,
        meta: PendingApprovalMeta,
    ) -> (ApprovalId, oneshot::Receiver<PendingResolution>) {
        let (tx, rx) = oneshot::channel();
        let mut g = self.inner.lock().expect("pending-approvals lock");
        g.next += 1;
        let id = format!("AP-{:06}", g.next);
        g.waiters.insert(id.clone(), PendingWaiter::Detailed(tx));
        g.pending.push(Self::build_info(
            id.clone(),
            tool,
            summary,
            requested_at_ms,
            meta,
        ));
        (id, rx)
    }

    /// T1.4: re-park an unresolved approval from the durable op-log after daemon restart.
    pub fn reregister_after_restart(
        &self,
        approval_id: ApprovalId,
        tool: String,
        summary: String,
        requested_at_ms: u64,
    ) {
        let mut g = self.inner.lock().expect("pending-approvals lock");
        if !g.pending.iter().any(|p| p.approval_id == approval_id) {
            let info = Self::build_info(
                approval_id,
                tool,
                summary,
                requested_at_ms,
                Default::default(),
            );
            g.pending.push(info);
        }
    }

    /// Resolve a pending approval, waking its awaiter with `outcome` if one is still parked.
    pub fn resolve(&self, id: &str, outcome: ApprovalOutcome) -> bool {
        self.resolve_with_detail(id, PendingResolution::simple(outcome))
    }

    /// Resolve a pending approval with optional `modified_command`, `steering_note`, and `allow_pattern`.
    pub fn resolve_with_detail(&self, id: &str, resolution: PendingResolution) -> bool {
        let mut g = self.inner.lock().expect("pending-approvals lock");
        let matched = g
            .pending
            .iter()
            .position(|p| p.approval_id == id)
            .map(|idx| g.pending.remove(idx));
        if let Some(ref entry) = matched
            && matches!(
                resolution.outcome,
                ApprovalOutcome::Approved | ApprovalOutcome::Modified
            )
            && entry.floor_category.is_none()
            && !entry.summary.starts_with("[floor:")
            && entry.credential_prompt != Some(true)
            && let (Some(sid), Some(pat)) = (&entry.session_id, &resolution.allow_pattern)
        {
            Self::insert_session_pattern(&mut g.session_allowlist, sid, pat);
        }
        match g.waiters.remove(id) {
            Some(PendingWaiter::Simple(tx)) => {
                let _ = tx.send(resolution.outcome);
                true
            }
            Some(PendingWaiter::Detailed(tx)) => {
                let _ = tx.send(resolution);
                true
            }
            None => matched.is_some(),
        }
    }

    /// Record a validated, token-boundary session `Ask`-tier command prefix pattern.
    pub fn allow_session_pattern(&self, session_id: &str, raw_pattern: &str) -> bool {
        let mut g = self.inner.lock().expect("pending-approvals lock");
        Self::insert_session_pattern(&mut g.session_allowlist, session_id, raw_pattern)
    }

    /// Return currently registered `Ask`-tier command prefix patterns for `session_id`.
    #[must_use]
    pub fn session_patterns(&self, session_id: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("pending-approvals lock")
            .session_allowlist
            .get(session_id.trim())
            .cloned()
            .unwrap_or_default()
    }

    /// Check whether a single leaf command matches an allowed prefix pattern for `session_id` on a token boundary.
    #[must_use]
    pub fn matches_leaf_command(&self, session_id: &str, leaf_cmd: &str) -> bool {
        let trimmed = leaf_cmd.trim();
        if trimmed.is_empty() {
            return false;
        }
        let g = self.inner.lock().expect("pending-approvals lock");
        let Some(patterns) = g.session_allowlist.get(session_id.trim()) else {
            return false;
        };
        patterns.iter().any(|p| {
            trimmed == p
                || trimmed
                    .strip_prefix(p.as_str())
                    .is_some_and(|rest| rest.starts_with(char::is_whitespace))
        })
    }

    /// Clear all session-scoped `Ask`-tier patterns for `session_id` (e.g. on `Autopilot -> Manual` preemption).
    pub fn clear_session_patterns(&self, session_id: &str) {
        self.inner
            .lock()
            .expect("pending-approvals lock")
            .session_allowlist
            .remove(session_id.trim());
    }

    /// Drop a pending approval without an outcome (e.g. the awaiter timed out).
    pub fn cancel(&self, id: &str) {
        let mut g = self.inner.lock().expect("pending-approvals lock");
        g.pending.retain(|p| p.approval_id != id);
        g.waiters.remove(id);
    }

    /// Return a snapshot of one pending approval by id without resolving or removing it.
    #[must_use]
    pub fn peek(&self, id: &str) -> Option<PendingApprovalInfo> {
        self.inner
            .lock()
            .expect("pending-approvals lock")
            .pending
            .iter()
            .find(|p| p.approval_id == id)
            .cloned()
    }

    /// Snapshot of approvals currently awaiting a decision.
    pub fn list(&self) -> Vec<PendingApprovalInfo> {
        self.inner
            .lock()
            .expect("pending-approvals lock")
            .pending
            .clone()
    }
}

/// Map a human's `vox_resolve_approval` decision string to an [`ApprovalOutcome`].
pub fn outcome_from_decision(decision: &str) -> ApprovalOutcome {
    match decision {
        "approve" | "approved" => ApprovalOutcome::Approved,
        "modify" | "modified" => ApprovalOutcome::Modified,
        _ => ApprovalOutcome::Rejected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_from_decision_maps_known_and_unknown() {
        for (s, exp) in [
            ("approve", ApprovalOutcome::Approved),
            ("approved", ApprovalOutcome::Approved),
            ("modify", ApprovalOutcome::Modified),
            ("modified", ApprovalOutcome::Modified),
            ("reject", ApprovalOutcome::Rejected),
            ("", ApprovalOutcome::Rejected),
            ("garbage", ApprovalOutcome::Rejected),
        ] {
            assert_eq!(outcome_from_decision(s), exp);
        }
    }

    #[test]
    fn sync_register_resolve_backward_compat_and_register_with_meta() {
        let reg = PendingApprovals::default();
        let (id1, mut rx1) = reg.register("vox_write_file".into(), "write a.rs".into(), 100);
        let j1 = serde_json::to_value(&reg.list()[0]).expect("serialize basic info");
        assert!(j1.get("floor_category").is_none() && j1.get("session_id").is_none());
        assert!(reg.resolve(&id1, ApprovalOutcome::Approved));
        assert_eq!(rx1.try_recv().unwrap(), ApprovalOutcome::Approved);

        let meta = PendingApprovalMeta {
            floor_category: None,
            session_id: Some("s-1".into()),
            command: Some("cargo test".into()),
            control_mode: Some("co_pilot".into()),
            credential_prompt: Some(false),
        };
        let (id2, mut rx2) =
            reg.register_with_meta("vox_terminal_exec".into(), "t".into(), 200, meta);
        assert_eq!(reg.peek(&id2).unwrap().approval_id, id2);
        assert!(reg.peek("AP-999999").is_none());
        let j2 = serde_json::to_value(&reg.list()[0]).expect("serialize meta info");
        assert_eq!(
            (&j2["session_id"], &j2["command"]),
            (&serde_json::json!("s-1"), &serde_json::json!("cargo test"))
        );
        assert!(j2.get("floor_category").is_none());

        let detail = PendingResolution {
            outcome: ApprovalOutcome::Modified,
            modified_command: Some("cargo test -p vox-cli".into()),
            steering_note: None,
            allow_pattern: Some("cargo test".into()),
        };
        assert!(reg.resolve_with_detail(&id2, detail.clone()));
        assert_eq!(rx2.try_recv().unwrap(), detail);
        assert_eq!(reg.session_patterns("s-1"), vec!["cargo test".to_string()]);

        let fl_meta = PendingApprovalMeta {
            floor_category: Some("history_rewrite".into()),
            session_id: Some("s-1".into()),
            ..Default::default()
        };
        let (id_fl, _) =
            reg.register_with_meta("vox_terminal_exec".into(), "f".into(), 300, fl_meta);
        let fl_res = PendingResolution {
            allow_pattern: Some("git push".into()),
            ..PendingResolution::simple(ApprovalOutcome::Approved)
        };
        assert!(reg.resolve_with_detail(&id_fl, fl_res));
        assert_eq!(reg.session_patterns("s-1"), vec!["cargo test".to_string()]);

        let sum_fl_meta = PendingApprovalMeta {
            session_id: Some("s-1".into()),
            ..Default::default()
        };
        let (id_sum_fl, _) = reg.register_with_meta(
            "vox_terminal_exec".into(),
            "[floor:history_rewrite] git push --force".into(),
            301,
            sum_fl_meta,
        );
        let sum_fl_res = PendingResolution {
            allow_pattern: Some("git push".into()),
            ..PendingResolution::simple(ApprovalOutcome::Approved)
        };
        assert!(reg.resolve_with_detail(&id_sum_fl, sum_fl_res));
        assert_eq!(reg.session_patterns("s-1"), vec!["cargo test".to_string()]);
    }

    #[test]
    fn session_allowlist_boundary_and_lifecycle() {
        let reg = PendingApprovals::default();
        assert!(reg.allow_session_pattern("s-1", "  cargo test  "));
        assert!(reg.allow_session_pattern("s-1", "cargo test"));
        assert_eq!(reg.session_patterns("s-1"), vec!["cargo test".to_string()]);
        assert!(reg.matches_leaf_command("s-1", "cargo test"));
        assert!(reg.matches_leaf_command("s-1", "cargo test -p vox-cli"));
        assert!(!reg.matches_leaf_command("s-1", "cargo test-evil"));
        assert!(!reg.matches_leaf_command("s-2", "cargo test -p vox-cli"));

        for bad in "*~~~~  ~~a~~cargo *~~cargo test; rm~~cargo test && id~~cargo | sh~~cargo `id`~~cargo $HOME~~cargo > o~~cargo < i~~cargo\nrm".split("~~") {
            assert!(!reg.allow_session_pattern("s-1", bad), "accepted bad pattern {bad:?}");
        }

        reg.clear_session_patterns("s-1");
        assert!(reg.session_patterns("s-1").is_empty());
        assert!(!reg.matches_leaf_command("s-1", "cargo test"));
    }
}
