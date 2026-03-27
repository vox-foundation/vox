//! Bounded iterative refinement for `vox_plan` (draft → gap → LLM refine).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::Deserialize;

use super::build_system_prompt;
use super::params::{PlanLoopMode, PlanParams, PlanTask};
use super::plan_gap::{self, PlanGapReport};
use crate::llm_bridge::{McpChatModelResolution, McpInferRouting, mcp_infer_completion};
use crate::server::ServerState;
use crate::tools::chat_model_resolve::resolve_chat_llm_model;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanRefinementState {
    pub refinement_rounds: u32,
    pub loop_status: String,
    pub stop_reason: Option<String>,
    pub last_gap_report: Option<PlanGapReport>,
}

#[derive(Deserialize)]
struct PlanResponseSchema {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    tasks: Vec<PlanTask>,
}

fn hash_fingerprint(tasks: &[PlanTask]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for t in tasks {
        t.id.hash(&mut hasher);
        t.description.hash(&mut hasher);
        t.estimated_complexity.hash(&mut hasher);
        for d in &t.depends_on {
            d.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn strip_json_fence(block: &str) -> &str {
    let block = block.trim();
    if block.starts_with("```json") {
        block
            .strip_prefix("```json")
            .unwrap_or(block)
            .strip_suffix("```")
            .unwrap_or(block)
            .trim()
    } else if block.starts_with("```") {
        block
            .strip_prefix("```")
            .unwrap_or(block)
            .strip_suffix("```")
            .unwrap_or(block)
            .trim()
    } else {
        block
    }
}

fn refinement_prompt(
    goal: &str,
    scope_note: &str,
    max_tasks: usize,
    prior_summary: &str,
    prior_tasks_json: &str,
    gap_json: &str,
) -> String {
    format!(
        r#"You are revising a software engineering task plan.

GOAL: {goal}{scope_note}

Previous summary:
{prior_summary}

Previous tasks JSON:
{prior_tasks_json}

Gap / risk report from automated review (address these concretely):
{gap_json}

Output a VALID JSON object in a ```json fenced block with the same schema as before:
{{ "summary": "...", "tasks": [ {{ "id", "description", "files", "estimated_complexity", "depends_on" }} ] }}

Rules:
- Fix vague or dangerous steps; add missing dependencies and explicit verification/test tasks where needed.
- Maximum {max_tasks} tasks.
- Keep task ids starting at 1 and consistent with depends_on.
- Do NOT include filler tasks."#,
    )
}

/// Run up to `max_rounds` refinement passes when gaps exceed `risk_threshold`.
pub async fn maybe_refine_plan(
    state: &ServerState,
    params: &PlanParams,
    mut tasks: Vec<PlanTask>,
    mut summary: String,
    initial_model_complexity: u8,
    session_label: &str,
) -> (Vec<PlanTask>, String, PlanRefinementState) {
    let mode = params.loop_mode.unwrap_or(PlanLoopMode::Off);
    let off_state = PlanRefinementState {
        refinement_rounds: 0,
        loop_status: "off".to_string(),
        stop_reason: None,
        last_gap_report: None,
    };

    if matches!(mode, PlanLoopMode::Off) {
        return (tasks, summary, off_state);
    }

    let max_rounds = params.max_refine_rounds.unwrap_or(2).min(8);
    let risk_threshold = params
        .gap_risk_threshold
        .unwrap_or(0.28_f32)
        .clamp(0.05, 0.95);
    let token_budget = params.refine_budget_tokens.unwrap_or(18_000_u32);

    let mut rounds: u32 = 0;
    let mut tokens_spent: u32 = 0;
    let mut prev_fingerprint: Option<u64> = None;
    let mut oscillation_strikes: u8 = 0;
    let mut last_gap: Option<PlanGapReport> = None;

    loop {
        let gap_report = plan_gap::analyze_plan_gaps(&tasks);
        last_gap = Some(gap_report.clone());

        let should_refine = match mode {
            PlanLoopMode::Off => false,
            PlanLoopMode::Force => rounds < max_rounds,
            PlanLoopMode::Auto => {
                rounds < max_rounds && gap_report.aggregate_unresolved_risk > risk_threshold
            }
        };

        if !should_refine {
            let stop = if rounds == 0 {
                None
            } else if rounds >= max_rounds {
                Some("max_refine_rounds".to_string())
            } else if gap_report.aggregate_unresolved_risk <= risk_threshold {
                Some("ready_below_threshold".to_string())
            } else {
                Some("ready".to_string())
            };
            return (
                tasks,
                summary,
                PlanRefinementState {
                    refinement_rounds: rounds,
                    loop_status: "ready".to_string(),
                    stop_reason: stop,
                    last_gap_report: last_gap,
                },
            );
        }

        if token_budget > 0 && tokens_spent >= token_budget {
            return (
                tasks,
                summary,
                PlanRefinementState {
                    refinement_rounds: rounds,
                    loop_status: "stopped_budget".to_string(),
                    stop_reason: Some("token_budget".to_string()),
                    last_gap_report: last_gap,
                },
            );
        }

        let scope_note = if params.scope_files.is_empty() {
            String::new()
        } else {
            format!(
                "\n\nScope this plan to these files:\n{}",
                params.scope_files.join("\n")
            )
        };
        let max_tasks = params.max_tasks.unwrap_or(30);
        let prior_tasks_json = serde_json::to_string(&tasks).unwrap_or_else(|_| "[]".into());
        let gap_json = serde_json::to_string(&gap_report).unwrap_or_else(|_| "{}".into());
        let user_prompt = refinement_prompt(
            &params.goal,
            &scope_note,
            max_tasks,
            &summary,
            &prior_tasks_json,
            &gap_json,
        );

        let resolution_template = McpChatModelResolution {
            complexity: initial_model_complexity.max(8),
            ..Default::default()
        };

        let (model, free_only) = match resolve_chat_llm_model(
            state,
            &user_prompt,
            resolution_template.clone(),
            params.session_id.as_deref(),
        )
        .await
        {
            Ok(pair) => pair,
            Err(_) => {
                return (
                    tasks,
                    summary,
                    PlanRefinementState {
                        refinement_rounds: rounds,
                        loop_status: "stopped_model_error".to_string(),
                        stop_reason: Some("model_resolve_failed".to_string()),
                        last_gap_report: last_gap,
                    },
                );
            }
        };

        let pref = match crate::sync_poison::poison_rw_read(
            state.mcp_chat_model_override.read(),
            "mcp_chat_model_override",
        ) {
            Ok(g) => g.clone(),
            Err(_) => {
                return (
                    tasks,
                    summary,
                    PlanRefinementState {
                        refinement_rounds: rounds,
                        loop_status: "stopped_model_error".to_string(),
                        stop_reason: Some("model_lock".to_string()),
                        last_gap_report: last_gap,
                    },
                );
            }
        };

        let routing = McpInferRouting {
            user_prompt: &user_prompt,
            sticky_model_pref: pref.as_deref(),
            resolution_template,
            free_only,
            allow_cloud_ollama_fallback: true,
            user_id: params.session_id.as_deref(),
        };

        let system_prompt = build_system_prompt(state).await;
        let reserve_tokens = 4096_u32;
        let max_out = token_budget
            .saturating_sub(tokens_spent)
            .clamp(512, 8192)
            .max(reserve_tokens);

        let Ok((response_json, _model_used, used_tokens)) = mcp_infer_completion(
            state,
            model,
            "vox_plan_refine",
            &system_prompt,
            &routing,
            u64::from(max_out),
            0.25,
            true,
        )
        .await
        else {
            return (
                tasks,
                summary,
                PlanRefinementState {
                    refinement_rounds: rounds,
                    loop_status: "stopped_llm_error".to_string(),
                    stop_reason: Some("completion_failed".to_string()),
                    last_gap_report: last_gap,
                },
            );
        };

        tokens_spent = tokens_spent.saturating_add(u32::try_from(used_tokens).unwrap_or(u32::MAX));

        tracing::debug!(
            target = "vox_mcp::plan_loop",
            label = %session_label,
            round = rounds,
            tokens = used_tokens,
            "iterative plan refinement pass"
        );

        let cleaned = strip_json_fence(&response_json);
        let parsed: PlanResponseSchema = match serde_json::from_str(cleaned) {
            Ok(p) => p,
            Err(_) => {
                return (
                    tasks,
                    summary,
                    PlanRefinementState {
                        refinement_rounds: rounds,
                        loop_status: "stopped_parse_error".to_string(),
                        stop_reason: Some("invalid_refine_json".to_string()),
                        last_gap_report: last_gap,
                    },
                );
            }
        };

        if !parsed.tasks.is_empty() {
            tasks = parsed.tasks;
        }
        if !parsed.summary.is_empty() {
            summary = parsed.summary;
        }

        let fp = hash_fingerprint(&tasks);
        if let Some(prev) = prev_fingerprint {
            if prev == fp {
                oscillation_strikes = oscillation_strikes.saturating_add(1);
            } else {
                oscillation_strikes = 0;
            }
            if oscillation_strikes >= 1 {
                return (
                    tasks,
                    summary,
                    PlanRefinementState {
                        refinement_rounds: rounds.saturating_add(1),
                        loop_status: "stopped_plateau".to_string(),
                        stop_reason: Some("oscillation".to_string()),
                        last_gap_report: last_gap,
                    },
                );
            }
        }
        prev_fingerprint = Some(fp);

        rounds = rounds.saturating_add(1);
    }
}
