//! Populate [`AgentTask::approval_tier`] and [`AgentTask::attention_weight`] at enqueue time.

use crate::attention::{
    ActionDescriptor, AgentTrustScore, ApprovalTier, classify_tier, compute_attention_cost_ms,
    decision_entropy_bits,
};
use crate::orchestrator::Orchestrator;
use crate::types::{AccessKind, AgentId, AgentTask, FileAffinity};

fn strip_description_hints(description: &str) -> String {
    let mut out = String::with_capacity(description.len());
    let mut start = 0;
    while let Some(open) = description[start..].find("[[") {
        let open_pos = start + open;
        out.push_str(&description[start..open_pos]);
        if let Some(close) = description[open_pos..].find("]]") {
            start = open_pos + close + 2;
        } else {
            start = open_pos;
            break;
        }
    }
    out.push_str(&description[start..]);
    out
}

#[must_use]
pub(super) fn task_description_suggests_external(description: &str) -> bool {
    let clean = strip_description_hints(description);
    let d = clean.to_ascii_lowercase();
    d.contains("deploy")
        || d.contains("production")
        || d.contains("publish ")
        || d.contains("http://")
        || d.contains("https://")
        || d.contains("curl ")
        || d.contains("terraform")
        || d.contains("kubectl")
        || d.contains("api key")
        || d.contains("secret ")
}

/// Effective approval tier after risk-policy escalation: strictest of task hint,
/// category/source policy, and the classified tier (FLOOR-03). Extracted from
/// `populate_task_attention_fields` so it's unit-testable without a live `Orchestrator`.
#[must_use]
fn effective_approval_tier(
    task: &AgentTask,
    overrides: &crate::config::TaskPolicyOverrides,
    classified: ApprovalTier,
) -> ApprovalTier {
    match crate::mode::strictest_risk_for_task(task, overrides) {
        Some(risk) => classified.max_strictness(risk.resolve().approval.to_approval_tier()),
        None => classified,
    }
}

/// Set Phase-15 attention metadata from trust, manifest, and orchestrator attention config.
/// Background floored actions are lifted to Review, and Phase 13 composes the level on top.
pub(super) fn populate_task_attention_fields(
    orch: &Orchestrator,
    task: &mut AgentTask,
    agent_id: AgentId,
    file_manifest: &[FileAffinity],
) {
    let config = crate::sync_lock::rw_read(&*orch.config);
    let bm = crate::sync_lock::rw_read(&*orch.budget_manager);
    let trust = bm
        .trust_snapshot()
        .get(&agent_id)
        .cloned()
        .unwrap_or_else(|| AgentTrustScore::new(agent_id));

    let write_count = file_manifest
        .iter()
        .filter(|f| f.access == AccessKind::Write)
        .count();
    let external = task_description_suggests_external(&task.description)
        || file_manifest.iter().any(|f| {
            f.path
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("deploy")
        });
    let concurrent_tasks = {
        let agents = crate::sync_lock::rw_read(&*orch.agents);
        agents
            .values()
            .map(|q| {
                let qq = crate::sync_lock::rw_read(q);
                qq.len() + qq.in_progress_count()
            })
            .sum::<usize>()
    };
    // T5.5: use the rolling window (last `DEFAULT_TRUST_WINDOW_MS`), not the lifetime
    // `successful_outcomes`/`total_outcomes` accumulators, so an agent's approve-rate and
    // repeated-approve count age out old behavior instead of weighing it equally forever.
    let now_ms = crate::types::now_unix_ms();
    let approve_rate = trust
        .windowed_approve_rate(now_ms, crate::attention::DEFAULT_TRUST_WINDOW_MS)
        .unwrap_or(0.5);
    let repeated = trust
        .windowed_repeated_approve_count(now_ms, crate::attention::DEFAULT_TRUST_WINDOW_MS)
        .min(50);
    let action = ActionDescriptor {
        estimated_complexity: task.estimated_complexity,
        tokens_output: 0,
        priority: task.priority,
        write_file_count: write_count,
        external,
        repeated_approve_count: repeated,
        concurrent_tasks: concurrent_tasks.max(1),
    };
    let entropy = decision_entropy_bits(approve_rate);
    let tier = classify_tier(&trust, &action, entropy, &config.tier_gate);
    // Task E/2.4: RiskPosture may *escalate* the trust-classified tier (e.g. Low
    // risk forces Review) but must never demote below what trust warranted. See
    // `effective_approval_tier` for the full precedence chain (explicit hint >
    // category policy > source policy).
    let effective_tier = effective_approval_tier(task, &config.task_policy, tier);
    let write_paths: Vec<std::path::PathBuf> = file_manifest
        .iter()
        .filter(|f| f.access == AccessKind::Write)
        .map(|f| f.path.clone())
        .collect();
    let floor = crate::oversight::floor::classify_floor(
        &crate::oversight::floor::FloorInput::task(&write_paths, &task.tool_hints),
    );
    let effective_tier = effective_tier.max_strictness(crate::oversight::floor::floor_tier(floor));
    task.approval_tier = Some(effective_tier);
    let base = config.attention_interrupt_cost_ms.max(1);
    let cost = compute_attention_cost_ms(
        &action,
        trust.trust_score,
        base,
        &config.attention_tlx_weights,
    );
    task.attention_weight = cost as f64 / base as f64;
}

#[must_use]
pub(super) fn submission_approval_block_reason(task: &AgentTask) -> Option<String> {
    if matches!(task.status, crate::types::TaskStatus::BlockedOnApproval) {
        return Some(format!(
            "task {} requires explicit approval before execution (plan.execution_mode = RequiresApproval)",
            task.id
        ));
    }
    match task.approval_tier {
        Some(ApprovalTier::Blocked) => Some(format!(
            "task {} was classified as Blocked by approval policy (attention_weight={:.2})",
            task.id, task.attention_weight
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attention::ApprovalTier;
    use crate::types::{AgentTask, TaskId, TaskPriority};

    #[test]
    fn blocked_tier_returns_reason() {
        let mut t = AgentTask::new(TaskId(42), "deploy", TaskPriority::Urgent, vec![]);
        t.approval_tier = Some(ApprovalTier::Blocked);
        t.attention_weight = 2.4;
        let reason = submission_approval_block_reason(&t);
        assert!(reason.is_some());
        assert!(reason.unwrap_or_default().contains("Blocked"));
    }

    #[test]
    fn blocked_on_approval_returns_reason() {
        let mut t = AgentTask::new(TaskId(43), "deploy", TaskPriority::Urgent, vec![]);
        t.status = crate::types::TaskStatus::BlockedOnApproval;
        let reason = submission_approval_block_reason(&t);
        assert!(reason.is_some());
        assert!(
            reason
                .unwrap_or_default()
                .contains("requires explicit approval")
        );
    }

    #[test]
    fn non_blocked_tier_is_allowed() {
        let mut t = AgentTask::new(TaskId(1), "normal", TaskPriority::Normal, vec![]);
        t.approval_tier = Some(ApprovalTier::Confirm);
        assert!(submission_approval_block_reason(&t).is_none());
    }

    // Task E: replicate the escalation decision in attention_fields without
    // constructing a whole Orchestrator. `effective = classified.max_strictness(risk)`.
    fn effective(
        classified: ApprovalTier,
        posture: Option<crate::mode::RiskPosture>,
    ) -> ApprovalTier {
        match posture {
            Some(p) => classified.max_strictness(p.resolve().approval.to_approval_tier()),
            None => classified,
        }
    }

    fn empty_overrides() -> crate::config::TaskPolicyOverrides {
        crate::config::TaskPolicyOverrides {
            category: std::collections::HashMap::new(),
            source: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn low_risk_escalates_confirm_to_review() {
        // Low risk resolves to ApprovalLean::Review.
        assert_eq!(
            effective(ApprovalTier::Confirm, Some(crate::mode::RiskPosture::Low)),
            ApprovalTier::Review
        );
    }

    #[test]
    fn high_risk_does_not_demote_trust_required_review() {
        // High risk resolves to AutoApprove, but must NOT pull a classified Review down.
        assert_eq!(
            effective(ApprovalTier::Review, Some(crate::mode::RiskPosture::High)),
            ApprovalTier::Review
        );
    }

    #[test]
    fn high_risk_does_not_demote_blocked() {
        assert_eq!(
            effective(ApprovalTier::Blocked, Some(crate::mode::RiskPosture::High)),
            ApprovalTier::Blocked
        );
    }

    // Task 2.4: a task with NO explicit risk_posture, but a configured
    // category-policy risk override, must still get its approval tier
    // escalated -- reproduces the same "policy has zero effect without an
    // explicit hint" bug fixed in runtime.rs for cost/model routing.
    #[test]
    fn category_risk_policy_escalates_tier_without_explicit_risk_posture() {
        use crate::config::{TaskPolicyEntry, TaskPolicyOverrides};
        use std::collections::HashMap;

        let mut category = HashMap::new();
        category.insert(
            format!("{:?}", crate::types::TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("low".to_string()),
            },
        );
        let overrides = TaskPolicyOverrides {
            category,
            source: HashMap::new(),
        };

        let mut t = AgentTask::new(TaskId(7), "generate code", TaskPriority::Normal, vec![]);
        t.task_category = crate::types::TaskCategory::CodeGen;
        assert_eq!(t.risk_posture, None, "no explicit risk hint set");

        // A brand-new/low classified tier (AutoApprove) with no trust history:
        // if the category policy had zero effect the tier would stay unchanged
        // -- this assertion fails on the pre-fix code.
        let effective_tier = effective_approval_tier(&t, &overrides, ApprovalTier::AutoApprove);

        // Low risk resolves to ApprovalLean::Review.
        assert_eq!(
            effective_tier,
            ApprovalTier::Review,
            "category risk policy (Low) must escalate the tier even with no explicit risk_posture hint"
        );
    }

    #[test]
    fn source_only_risk_policy_escalates_tier() {
        use crate::config::{TaskPolicyEntry, TaskPolicyOverrides};
        use std::collections::HashMap;

        let mut source = HashMap::new();
        source.insert(
            format!("{:?}", crate::mode::TriggerSource::Automated),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("low".to_string()),
            },
        );
        let overrides = TaskPolicyOverrides {
            category: HashMap::new(),
            source,
        };

        let mut t = AgentTask::new(TaskId(8), "generate code", TaskPriority::Normal, vec![]);
        t.trigger_source = Some(crate::mode::TriggerSource::Automated);
        assert_eq!(t.risk_posture, None, "no explicit risk hint set");

        let effective_tier = effective_approval_tier(&t, &overrides, ApprovalTier::AutoApprove);

        assert_eq!(
            effective_tier,
            ApprovalTier::Review,
            "source risk policy (Low) must escalate the tier even with no category policy or explicit hint"
        );
    }

    #[test]
    fn risk_hint_cannot_lower_category_tier() {
        use crate::config::{OrchestratorConfig, TaskPolicyEntry};

        let mut cfg = OrchestratorConfig::for_testing();
        let orch_baseline = Orchestrator::new(cfg.clone());
        let agent_id = orch_baseline.spawn_agent("worker").expect("spawn");

        // Baseline: no category policy, hint High -> AutoApprove with empty manifest
        let mut t_baseline =
            AgentTask::new(TaskId(90), "generate code", TaskPriority::Normal, vec![]);
        t_baseline.task_category = crate::types::TaskCategory::CodeGen;
        t_baseline.risk_posture = Some(crate::mode::RiskPosture::High);
        populate_task_attention_fields(&orch_baseline, &mut t_baseline, agent_id, &[]);
        assert_eq!(t_baseline.approval_tier, Some(ApprovalTier::AutoApprove));

        // Category policy Low (Review) + task hint High -> Review
        cfg.task_policy.category.insert(
            format!("{:?}", crate::types::TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("low".to_string()),
            },
        );
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let mut t = AgentTask::new(TaskId(91), "generate code", TaskPriority::Normal, vec![]);
        t.task_category = crate::types::TaskCategory::CodeGen;
        t.risk_posture = Some(crate::mode::RiskPosture::High);

        populate_task_attention_fields(&orch, &mut t, aid, &[]);
        assert_eq!(
            t.approval_tier,
            Some(ApprovalTier::Review),
            "category policy Low (Review) must not be lowered by hint High"
        );
    }

    #[test]
    fn risk_hint_can_raise_tier() {
        use crate::config::OrchestratorConfig;

        let cfg = OrchestratorConfig::for_testing();
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let mut t = AgentTask::new(TaskId(92), "generate code", TaskPriority::Normal, vec![]);
        t.task_category = crate::types::TaskCategory::CodeGen;
        t.risk_posture = Some(crate::mode::RiskPosture::Low);

        populate_task_attention_fields(&orch, &mut t, aid, &[]);
        assert_eq!(
            t.approval_tier,
            Some(ApprovalTier::Review),
            "hint Low must raise tier to Review"
        );
    }

    #[test]
    fn nothing_configured_preserves_classified_tier() {
        let overrides = empty_overrides();
        let t = AgentTask::new(TaskId(10), "generate code", TaskPriority::Normal, vec![]);

        assert_eq!(
            effective_approval_tier(&t, &overrides, ApprovalTier::Confirm),
            ApprovalTier::Confirm
        );
    }

    #[test]
    fn none_posture_leaves_classified_tier_unchanged() {
        assert_eq!(
            effective(ApprovalTier::Confirm, None),
            ApprovalTier::Confirm
        );
        assert_eq!(
            effective(ApprovalTier::AutoApprove, None),
            ApprovalTier::AutoApprove
        );
    }

    #[test]
    fn floor_secrets_manifest_write_lifts_task_to_review() {
        use crate::attention::TrustTier;
        use crate::config::OrchestratorConfig;

        let cfg = OrchestratorConfig::for_testing();
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        {
            let bm = crate::sync_lock::rw_read(&*orch.budget_manager);
            let mut scores = crate::sync_lock::rw_write(&*bm.trust_scores);
            let mut ts = AgentTrustScore::new(aid);
            ts.tier = TrustTier::System;
            ts.trust_score = 0.95;
            scores.insert(aid, ts);
        }

        // 1. Baseline: target/floor-test/notes.md -> AutoApprove
        let manifest_baseline = vec![FileAffinity::write("target/floor-test/notes.md")];
        let mut task = AgentTask::new(
            TaskId(101),
            "tidy config",
            TaskPriority::Normal,
            manifest_baseline.clone(),
        );
        populate_task_attention_fields(&orch, &mut task, aid, &manifest_baseline);
        assert_eq!(task.approval_tier, Some(ApprovalTier::AutoApprove));

        // 2. Floored: .env -> Review
        let manifest_floored = vec![FileAffinity::write(".env")];
        let mut task_floored = AgentTask::new(
            TaskId(102),
            "tidy config",
            TaskPriority::Normal,
            manifest_floored.clone(),
        );
        populate_task_attention_fields(&orch, &mut task_floored, aid, &manifest_floored);
        assert_eq!(task_floored.approval_tier, Some(ApprovalTier::Review));

        // 3. Floored with risk_posture = High -> still Review
        let mut task_high = AgentTask::new(
            TaskId(103),
            "tidy config",
            TaskPriority::Normal,
            manifest_floored.clone(),
        );
        task_high.risk_posture = Some(crate::mode::RiskPosture::High);
        populate_task_attention_fields(&orch, &mut task_high, aid, &manifest_floored);
        assert_eq!(task_high.approval_tier, Some(ApprovalTier::Review));

        // 4. Repeat with TrustTier::Trusted (one write): baseline Confirm, floored Review
        {
            let bm = crate::sync_lock::rw_read(&*orch.budget_manager);
            let mut scores = crate::sync_lock::rw_write(&*bm.trust_scores);
            let mut ts = AgentTrustScore::new(aid);
            ts.tier = TrustTier::Trusted;
            ts.trust_score = 0.95;
            scores.insert(aid, ts);
        }
        let mut task_trusted_baseline = AgentTask::new(
            TaskId(104),
            "tidy config",
            TaskPriority::Normal,
            manifest_baseline.clone(),
        );
        populate_task_attention_fields(&orch, &mut task_trusted_baseline, aid, &manifest_baseline);
        assert_eq!(
            task_trusted_baseline.approval_tier,
            Some(ApprovalTier::Confirm)
        );

        let mut task_trusted_floored = AgentTask::new(
            TaskId(105),
            "tidy config",
            TaskPriority::Normal,
            manifest_floored.clone(),
        );
        populate_task_attention_fields(&orch, &mut task_trusted_floored, aid, &manifest_floored);
        assert_eq!(
            task_trusted_floored.approval_tier,
            Some(ApprovalTier::Review)
        );
    }

    #[tokio::test]
    async fn floor_tool_hint_lifts_task_to_review() {
        use crate::attention::TrustTier;
        use crate::config::OrchestratorConfig;

        let cfg = OrchestratorConfig::for_testing();
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        {
            let bm = crate::sync_lock::rw_read(&*orch.budget_manager);
            let mut scores = crate::sync_lock::rw_write(&*bm.trust_scores);
            let mut ts = AgentTrustScore::new(aid);
            ts.tier = TrustTier::System;
            ts.trust_score = 0.95;
            scores.insert(aid, ts);
        }

        // Baseline with no hints -> AutoApprove
        let task_id_baseline = orch
            .submit_task("inspect repository", vec![], None, None, None)
            .await
            .expect("submit baseline");
        let baseline_task = {
            let q = orch.get_agent_queue_mut(aid).expect("queue");
            let mut w = crate::sync_lock::rw_write(&*q);
            w.dequeue().expect("task dequeued")
        };
        assert_eq!(baseline_task.id, task_id_baseline);
        assert_eq!(baseline_task.approval_tier, Some(ApprovalTier::AutoApprove));

        // Floored with tool_hints = ["vox_deploy"] -> Review
        let task_id_floored = orch
            .submit_task(
                "inspect repository [[tool:vox_deploy]]",
                vec![],
                None,
                None,
                None,
            )
            .await
            .expect("submit floored");
        let floored_task = {
            let q = orch.get_agent_queue_mut(aid).expect("queue");
            let mut w = crate::sync_lock::rw_write(&*q);
            w.dequeue().expect("task dequeued")
        };
        assert_eq!(floored_task.id, task_id_floored);
        assert_eq!(floored_task.approval_tier, Some(ApprovalTier::Review));
    }

    #[test]
    fn floor_unfloored_task_tier_unchanged() {
        use crate::attention::TrustTier;
        use crate::config::OrchestratorConfig;

        let cfg = OrchestratorConfig::for_testing();
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        {
            let bm = crate::sync_lock::rw_read(&*orch.budget_manager);
            let mut scores = crate::sync_lock::rw_write(&*bm.trust_scores);
            let mut ts = AgentTrustScore::new(aid);
            ts.tier = TrustTier::System;
            ts.trust_score = 0.95;
            scores.insert(aid, ts);
        }

        let manifest = vec![FileAffinity::write("src/lib.rs")];
        let mut task = AgentTask::new(
            TaskId(201),
            "modify library",
            TaskPriority::Normal,
            manifest.clone(),
        );
        populate_task_attention_fields(&orch, &mut task, aid, &manifest);
        assert_eq!(task.approval_tier, Some(ApprovalTier::AutoApprove));
    }
}
