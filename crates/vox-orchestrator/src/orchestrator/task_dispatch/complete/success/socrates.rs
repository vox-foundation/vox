use crate::orchestrator::task_dispatch::complete::success::gates::GateOutcome;
use crate::orchestrator::{Orchestrator, OrchestratorError};
use crate::types::{AgentId, AgentTask, CompletionAttestation, TaskId, TaskStatus};
use tracing;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Enforce {
    configured: bool,
    hinted: bool,
}

impl Enforce {
    fn required(self, trust_relax_gates: bool) -> bool {
        self.configured || (self.hinted && !trust_relax_gates)
    }
}

fn configured_flag(global: bool, configured: Option<bool>) -> bool {
    global || configured.unwrap_or(false)
}

/// Compute the effective `(grounding, socrates)` [`Enforce`] pair for one task.
///
/// Precedence is strictest-wins (FLOOR-03):
/// - `configured` is the global flag OR the configured (category/source) policy flag.
///   An agent-selected `task_category` or `trigger_source` cannot lower enforcement below global.
/// - `hinted` is the task's self-assigned `task.risk_posture` hint flag.
///   A hint can only add/raise enforcement, never lower configured/global enforcement.
/// - Trust-based gate relaxation (`trust_relax_gates`) waives ONLY enforcement
///   that the task's own hint added; it cannot waive configured or global enforcement.
#[must_use]
fn resolve_grounding_socrates_enforce(
    task: &AgentTask,
    overrides: &crate::config::TaskPolicyOverrides,
    global_grounding_enforce: bool,
    global_socrates_enforce: bool,
) -> (Enforce, Enforce) {
    let configured_risk =
        crate::mode::configured_risk_for_task(task, overrides).map(|r| r.resolve());
    let configured_grounding = configured_risk.map(|r| r.grounding_enforce);
    let configured_socrates = configured_risk.map(|r| r.socrates_enforce);

    let hint_risk = task.risk_posture.map(|r| r.resolve());
    let hint_grounding = hint_risk.map(|r| r.grounding_enforce).unwrap_or(false);
    let hint_socrates = hint_risk.map(|r| r.socrates_enforce).unwrap_or(false);

    let grounding = Enforce {
        configured: configured_flag(global_grounding_enforce, configured_grounding),
        hinted: hint_grounding,
    };
    let socrates = Enforce {
        configured: configured_flag(global_socrates_enforce, configured_socrates),
        hinted: hint_socrates,
    };
    (grounding, socrates)
}

impl Orchestrator {
    /// Check completion gates (grounding and Socrates verification) for `task`.
    ///
    /// Trust relax narrows only hint-added gates; it never waives configured or global
    /// enforcement or the bypass block (FLOOR-03, research Pitfall 2).
    pub async fn check_socrates_gate(
        &self,
        task_id: TaskId,
        agent_id: AgentId,
        task: &AgentTask,
        attestation: Option<&CompletionAttestation>,
        max_socrates_debug_iterations: u8,
        trust_relax_gates: bool,
    ) -> Result<GateOutcome, OrchestratorError> {
        let Some(ref ctx) = task.socrates else {
            return Ok(GateOutcome {
                requeue: None,
                needs_review_approval: false,
            });
        };

        let envelope_raw = task.session_id.as_ref().and_then(|sid| {
            let key = crate::socrates::session_context_envelope_key(sid);
            crate::sync_lock::rw_read(&*self.context_store).get(&key)
        });

        let (
            grounding_shadow,
            grounding_enforce,
            socrates_shadow,
            socrates_enforce,
            socrates_policy,
            bypass_blocked,
            force_research,
        ) = {
            let config = crate::sync_lock::rw_read(&*self.config);
            let (bb, fr) = if let Some(q_lock) = self.agent_queue(agent_id) {
                let q = crate::sync_lock::rw_read(&*q_lock);
                (
                    q.capabilities.is_low_confidence_bypass_blocked,
                    q.capabilities.force_socrates_research,
                )
            } else {
                (false, false)
            };
            let (grounding, socrates) = resolve_grounding_socrates_enforce(
                task,
                &config.task_policy,
                config.completion_grounding_enforce,
                config.socrates_gate_enforce,
            );
            let grounding_enforce = grounding.required(trust_relax_gates);
            let socrates_enforce = socrates.required(trust_relax_gates);
            (
                config.completion_grounding_shadow,
                grounding_enforce,
                config.socrates_gate_shadow,
                socrates_enforce,
                config.effective_socrates_policy(),
                bb,
                fr,
            )
        };

        if grounding_shadow || grounding_enforce {
            let declared = crate::grounding::declared_evidence_citations(attestation);
            let grounding_msg = if !declared.is_empty() {
                crate::grounding::grounding_violation_declared_not_in_envelope(
                    attestation,
                    envelope_raw.as_deref(),
                )
            } else {
                crate::grounding::grounding_violation_factual_mode_without_declarations(
                    attestation,
                    ctx,
                )
            };

            if let Some(msg) = grounding_msg {
                let violation_kind = if !declared.is_empty() {
                    "declared_not_in_envelope"
                } else {
                    "factual_without_declarations"
                };
                if grounding_shadow {
                    tracing::info!(
                        target: "vox_orchestrator::grounding",
                        task_id = task_id.0,
                        agent_id = agent_id.0,
                        violation_kind,
                        "{msg}"
                    );
                }
                if grounding_enforce && task.debug_iterations < max_socrates_debug_iterations {
                    tracing::warn!(
                        target: "vox_orchestrator::grounding",
                        task_id = task_id.0,
                        agent_id = agent_id.0,
                        violation_kind,
                        requeued = true,
                        "completion grounding enforce: task re-queued for more evidence",
                    );
                    let mut t = task.clone();
                    t.debug_iterations += 1;
                    t.description
                        .push_str(&format!("\n\n[GROUNDING GATE]\n{msg}\n",));
                    t.status = TaskStatus::Queued;
                    return Ok(GateOutcome {
                        requeue: Some((t, "grounding gate policy violation".into(), 1, 0)),
                        needs_review_approval: false,
                    });
                }
            }
        }

        let mut augmented = crate::grounding::merge_attestation_into_socrates_context(
            (*ctx).clone(),
            attestation,
            envelope_raw.as_deref(),
        );

        if crate::sync_lock::rw_read(&*self.budget_manager).is_fatigued() {
            augmented.fatigue_active = true;
        }

        let mut outcome = crate::socrates::evaluate_socrates_gate(
            &augmented,
            &socrates_policy,
            task.description.as_str(),
        );

        if force_research && !outcome.research_decision.should_research {
            outcome.research_decision.should_research = true;
            outcome.research_decision.trigger =
                "Policy: force_socrates_research enabled".to_string();
        }

        if socrates_shadow {
            tracing::info!(
                target: "vox_orchestrator::socrates",
                task_id = task_id.0,
                agent_id = agent_id.0,
                decision = ?outcome.decision,
                confidence = outcome.confidence,
                contradiction = outcome.contradiction_ratio,
                "socrates gate (shadow)"
            );
        }

        let is_low_confidence = outcome.confidence < 0.7 || outcome.contradiction_ratio > 0.3;
        let bypass_disallowed = bypass_blocked && is_low_confidence;

        // `evaluate_socrates_gate` may recommend web/autonomous research even when no gate is
        // enabled to *consume* those results (shadow/off configs). Running `perform_autonomous_research`
        // unconditionally dominated integration tests and stress drains — skip unless a policy
        // surface actually needs retrieval telemetry or enforcement inputs.
        let mut research_results = Vec::new();
        if outcome.research_decision.should_research {
            let autonomous_research_needed = force_research
                || socrates_enforce
                || socrates_shadow
                || grounding_enforce
                || grounding_shadow
                || bypass_disallowed;
            if autonomous_research_needed {
                let queries = outcome
                    .research_decision
                    .suggested_query
                    .clone()
                    .map(|q| vec![q])
                    .unwrap_or_else(|| vec![task.description.clone()]);
                let trigger = outcome.research_decision.trigger.clone();

                let results = self
                    .perform_autonomous_research(Some(agent_id), Some(task_id), queries, &trigger)
                    .await
                    .unwrap_or_default();
                research_results = results;
            }
        }

        if (socrates_enforce || bypass_disallowed)
            && (outcome.decision != vox_orchestrator_types::socrates_policy::RiskDecision::Answer
                || !research_results.is_empty()
                || bypass_disallowed)
            && task.debug_iterations < max_socrates_debug_iterations
        {
            let mut t = task.clone();
            if let Some(ref sid) = t.session_id {
                let context_key = crate::socrates::session_context_envelope_key(sid);
                let store = crate::sync_lock::rw_read(&*self.context_store);
                let context_raw = store.get(&context_key);
                drop(store);
                let parsed = context_raw.as_ref().and_then(|raw| {
                    serde_json::from_str::<crate::ContextEnvelope>(raw)
                        .ok()
                        .and_then(|env| {
                            crate::socrates::SessionRetrievalEnvelope::from_context_envelope(&env)
                        })
                });
                if let Some(env) = parsed {
                    t.socrates = Some(env.merge_into(t.socrates.clone()));
                }
            }

            if !research_results.is_empty() {
                let mut s_ctx = t.socrates.clone().unwrap_or(augmented.clone());
                let old_quality = s_ctx.evidence_quality;
                self.inject_research_results(&mut s_ctx, research_results);
                t.socrates = Some(s_ctx.clone());

                tracing::info!(
                    target: "vox_orchestrator::socrates",
                    task_id = task_id.0,
                    agent_id = agent_id.0,
                    quality_improvement = s_ctx.evidence_quality - old_quality,
                    "autonomous research injected; evidence quality boosted"
                );
            }

            t.debug_iterations += 1;
            let next_action = t
                .socrates
                .as_ref()
                .and_then(|ctx| ctx.recommended_next_action.as_deref())
                .unwrap_or("gather_more_grounding");
            let mut reason = format!(
                "Risk decision {:?} (confidence {:.2}, contradiction {:.2}). Improve grounding (citations, evidence) or resolve contradictions before completing.",
                outcome.decision, outcome.confidence, outcome.contradiction_ratio
            );
            if bypass_disallowed {
                reason.push_str(" Bypass blocked by security policy due to low confidence.");
            }
            reason.push_str(&format!(" Suggested next action: {}.", next_action));
            t.description
                .push_str(&format!("\n\n[SOCRATES GATE]\n{}\n", reason));
            t.status = TaskStatus::Queued;
            Ok(GateOutcome {
                requeue: Some((t, "Socrates risk gate blocked completion".into(), 1, 0)),
                needs_review_approval: false,
            })
        } else {
            Ok(GateOutcome {
                requeue: None,
                needs_review_approval: false,
            })
        }
    }
}

#[cfg(test)]
mod resolve_grounding_socrates_enforce_tests {
    use super::*;
    use crate::SocratesTaskContext;
    use crate::config::{TaskPolicyEntry, TaskPolicyOverrides};
    use crate::types::{AgentTask, TaskCategory, TaskId, TaskPriority};
    use std::collections::HashMap;

    // Task 2.4 regression test: a task with NO explicit risk_posture, but a
    // configured category-policy risk override, must still enable
    // grounding/socrates enforcement -- reproduces the same
    // "policy has zero effect without an explicit hint" bug fixed in
    // runtime.rs for cost/model routing.
    #[test]
    fn category_risk_policy_enforces_without_explicit_risk_posture() {
        let mut category = HashMap::new();
        category.insert(
            format!("{:?}", TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("low".to_string()),
            },
        );
        let overrides = TaskPolicyOverrides {
            category,
            source: HashMap::new(),
        };

        let mut task = AgentTask::new(TaskId(1), "generate code", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;
        assert_eq!(task.risk_posture, None, "no explicit risk hint set");

        // Global defaults are both OFF -- if the category policy had zero
        // effect, both enforce flags would stay false.
        let (grounding, socrates) =
            resolve_grounding_socrates_enforce(&task, &overrides, false, false);

        assert!(
            grounding.required(false),
            "Low-risk category policy must enable grounding enforcement even with no explicit risk_posture hint"
        );
        assert!(
            socrates.required(false),
            "Low-risk category policy must enable socrates enforcement even with no explicit risk_posture hint"
        );
    }

    #[test]
    fn source_risk_policy_applies_when_no_category_policy() {
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

        let mut task = AgentTask::new(TaskId(2), "generate code", TaskPriority::Normal, vec![]);
        task.trigger_source = Some(crate::mode::TriggerSource::Automated);

        let (grounding, socrates) =
            resolve_grounding_socrates_enforce(&task, &overrides, false, false);

        assert!(grounding.required(false));
        assert!(socrates.required(false));
    }

    #[test]
    fn nothing_configured_preserves_global_defaults() {
        let overrides = TaskPolicyOverrides {
            category: HashMap::new(),
            source: HashMap::new(),
        };
        let task = AgentTask::new(TaskId(3), "generate code", TaskPriority::Normal, vec![]);

        let (g_off, s_off) = resolve_grounding_socrates_enforce(&task, &overrides, false, false);
        assert!(!g_off.required(false));
        assert!(!s_off.required(false));

        let (g_on, s_on) = resolve_grounding_socrates_enforce(&task, &overrides, true, true);
        assert!(g_on.required(false));
        assert!(s_on.required(false));
    }

    #[test]
    fn explicit_risk_posture_cannot_lower_category_policy() {
        let mut category = HashMap::new();
        category.insert(
            format!("{:?}", TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("low".to_string()),
            },
        );
        let overrides = TaskPolicyOverrides {
            category,
            source: HashMap::new(),
        };

        let mut task = AgentTask::new(TaskId(4), "generate code", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;
        task.risk_posture = Some(crate::mode::RiskPosture::High);

        // High-risk hint cannot lower the category's Low-risk policy (Review, both enforce flags true).
        let (grounding, socrates) =
            resolve_grounding_socrates_enforce(&task, &overrides, false, false);
        assert!(grounding.required(false));
        assert!(socrates.required(false));
    }

    #[test]
    fn explicit_risk_posture_cannot_lower_global_enforce() {
        let overrides = TaskPolicyOverrides {
            category: HashMap::new(),
            source: HashMap::new(),
        };
        let mut task = AgentTask::new(TaskId(5), "generate code", TaskPriority::Normal, vec![]);
        task.risk_posture = Some(crate::mode::RiskPosture::High);

        let (grounding, socrates) =
            resolve_grounding_socrates_enforce(&task, &overrides, true, true);
        assert!(grounding.required(false));
        assert!(socrates.required(false));
    }

    #[test]
    fn explicit_risk_posture_can_raise_enforcement() {
        let overrides = TaskPolicyOverrides {
            category: HashMap::new(),
            source: HashMap::new(),
        };
        let mut task = AgentTask::new(TaskId(6), "generate code", TaskPriority::Normal, vec![]);
        task.risk_posture = Some(crate::mode::RiskPosture::Low);

        let (grounding, socrates) =
            resolve_grounding_socrates_enforce(&task, &overrides, false, false);
        assert!(grounding.required(false));
        assert!(socrates.required(false));
    }

    #[test]
    fn lax_category_policy_cannot_lower_global_enforce() {
        let mut category = HashMap::new();
        category.insert(
            format!("{:?}", TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("high".to_string()),
            },
        );
        let overrides = TaskPolicyOverrides {
            category,
            source: HashMap::new(),
        };
        let mut task = AgentTask::new(TaskId(7), "generate code", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;

        let (grounding, socrates) =
            resolve_grounding_socrates_enforce(&task, &overrides, true, true);
        assert!(grounding.required(false));
        assert!(socrates.required(false));
        assert!(grounding.required(true));
        assert!(socrates.required(true));
    }

    fn test_violating_context_and_attestation() -> (SocratesTaskContext, CompletionAttestation) {
        let soc = SocratesTaskContext {
            risk_budget: String::new(),
            factual_mode: true,
            required_citations: 1,
            ..Default::default()
        };
        let att = CompletionAttestation {
            completion_summary: Some(
                "The handler returns HTTP 403 for anonymous users; the gateway removes cookies on that path."
                    .into(),
            ),
            ..Default::default()
        };
        (soc, att)
    }

    #[tokio::test]
    async fn check_gate_hint_cannot_lower_category_policy() {
        let mut cfg = crate::config::OrchestratorConfig::for_testing();
        cfg.completion_grounding_enforce = false;
        cfg.socrates_gate_enforce = false;
        cfg.task_policy.category.insert(
            format!("{:?}", TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("low".to_string()),
            },
        );
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let (soc, att) = test_violating_context_and_attestation();
        let mut task = AgentTask::new(TaskId(10), "generate code", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;
        task.risk_posture = Some(crate::mode::RiskPosture::High);
        task.socrates = Some(soc);

        let outcome = orch
            .check_socrates_gate(task.id, aid, &task, Some(&att), 3, false)
            .await
            .expect("check gate ok");
        assert!(
            outcome.requeue.is_some(),
            "hint High must not lower category Low policy"
        );
    }

    #[tokio::test]
    async fn check_gate_lax_category_cannot_lower_global_enforce() {
        let mut cfg = crate::config::OrchestratorConfig::for_testing();
        cfg.completion_grounding_enforce = true;
        cfg.socrates_gate_enforce = false;
        cfg.task_policy.category.insert(
            format!("{:?}", TaskCategory::CodeGen),
            TaskPolicyEntry {
                clutch: None,
                risk: Some("high".to_string()),
            },
        );
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let (soc, att) = test_violating_context_and_attestation();
        let mut task = AgentTask::new(TaskId(11), "generate code", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;
        task.socrates = Some(soc);

        let outcome = orch
            .check_socrates_gate(task.id, aid, &task, Some(&att), 3, false)
            .await
            .expect("check gate ok");
        assert!(
            outcome.requeue.is_some(),
            "lax category must not lower global enforce"
        );
    }

    #[tokio::test]
    async fn trust_relax_cannot_waive_configured_grounding() {
        let mut cfg = crate::config::OrchestratorConfig::for_testing();
        cfg.completion_grounding_enforce = true;
        cfg.socrates_gate_enforce = false;
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let (soc, att) = test_violating_context_and_attestation();
        let mut task = AgentTask::new(TaskId(12), "generate code", TaskPriority::Normal, vec![]);
        task.socrates = Some(soc);

        let outcome = orch
            .check_socrates_gate(task.id, aid, &task, Some(&att), 3, true)
            .await
            .expect("check gate ok");
        assert!(
            outcome.requeue.is_some(),
            "trust relax must not waive configured grounding enforce"
        );
    }

    #[tokio::test]
    async fn trust_relax_cannot_waive_configured_socrates() {
        let mut cfg = crate::config::OrchestratorConfig::for_testing();
        cfg.completion_grounding_enforce = false;
        cfg.socrates_gate_enforce = true;
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let soc = SocratesTaskContext {
            fabricated_tool_claims: Some(vec!["fake_tool".into()]),
            ..Default::default()
        };
        let mut task = AgentTask::new(TaskId(13), "generate code", TaskPriority::Normal, vec![]);
        task.socrates = Some(soc);

        let outcome = orch
            .check_socrates_gate(task.id, aid, &task, None, 3, true)
            .await
            .expect("check gate ok");
        assert!(
            outcome.requeue.is_some(),
            "trust relax must not waive configured socrates enforce"
        );
    }

    #[tokio::test]
    async fn trust_relax_cannot_waive_configured_bypass_disallowed() {
        let mut cfg = crate::config::OrchestratorConfig::for_testing();
        cfg.completion_grounding_enforce = false;
        cfg.socrates_gate_enforce = false;
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        {
            let q_lock = orch.agent_queue(aid).expect("queue");
            let mut q = crate::sync_lock::rw_write(&*q_lock);
            q.capabilities.is_low_confidence_bypass_blocked = true;
        }

        let soc = SocratesTaskContext {
            fabricated_tool_claims: Some(vec!["fake_tool".into()]),
            ..Default::default()
        };
        let mut task = AgentTask::new(TaskId(14), "generate code", TaskPriority::Normal, vec![]);
        task.socrates = Some(soc);

        let outcome = orch
            .check_socrates_gate(task.id, aid, &task, None, 3, true)
            .await
            .expect("check gate ok");
        assert!(
            outcome.requeue.is_some(),
            "trust relax must not waive bypass_disallowed"
        );
    }

    #[tokio::test]
    async fn trust_relax_still_waives_hint_only_enforcement() {
        let mut cfg = crate::config::OrchestratorConfig::for_testing();
        cfg.completion_grounding_enforce = false;
        cfg.socrates_gate_enforce = false;
        let orch = Orchestrator::new(cfg);
        let aid = orch.spawn_agent("worker").expect("spawn");

        let (soc, att) = test_violating_context_and_attestation();
        let mut task = AgentTask::new(TaskId(15), "generate code", TaskPriority::Normal, vec![]);
        task.risk_posture = Some(crate::mode::RiskPosture::Low);
        task.socrates = Some(soc);

        let outcome = orch
            .check_socrates_gate(task.id, aid, &task, Some(&att), 3, true)
            .await
            .expect("check gate ok");
        assert!(
            outcome.requeue.is_none(),
            "trust relax should waive hint-added enforcement"
        );
    }
}

#[cfg(test)]
mod autonomous_research_short_circuit_tests {
    use super::*;
    use crate::config::OrchestratorConfig;
    use crate::sync_lock;
    use crate::types::{FileAffinity, TaskPriority};
    use std::time::Duration;

    #[tokio::test]
    async fn completion_does_not_call_autonomous_research_when_gates_disabled() {
        let mut cfg = OrchestratorConfig::for_testing();
        cfg.socrates_gate_enforce = false;
        cfg.socrates_gate_shadow = false;
        cfg.completion_grounding_enforce = false;
        cfg.completion_grounding_shadow = false;
        cfg.max_agents = 4;

        let orch = Orchestrator::new(cfg);
        orch.submit_task(
            "explore factual claims about rust async runtimes",
            vec![FileAffinity::write("src/a.rs")],
            Some(TaskPriority::Normal),
            None,
            None,
        )
        .await
        .unwrap();

        let aid = orch.agent_ids()[0];
        let task_id = {
            let q = orch.get_agent_queue_mut(aid).unwrap();
            let mut w = sync_lock::rw_write(&*q);
            w.dequeue().map(|t| t.id).expect("task dequeued")
        };

        tokio::time::timeout(
            Duration::from_secs(8),
            orch.complete_task_with_attestation(task_id, Some(CompletionAttestation::default())),
        )
        .await
        .expect("completion must not block on autonomous research when gates are off")
        .expect("complete ok");
    }
}
