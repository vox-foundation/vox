//! Unified **plan refinement** signals: MCP-style per-task gap heuristics plus plan-level **adequacy**
//! (detecting structurally thin plans for a goal's estimated complexity).
//!
//! Used by MCP `vox_plan` and orchestrator-native `PlanNode` DAGs so behavior stays aligned.

use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Minimal task shape for analysis (MCP [`PlanTask`] maps 1:1).
#[derive(Debug, Clone, Serialize)]
pub struct PlanAdequacyTask {
    /// Monotonic task index (1-based for MCP).
    pub id: usize,
    pub description: String,
    pub files: Vec<String>,
    pub estimated_complexity: u8,
    pub depends_on: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskGapFinding {
    pub task_id: usize,
    pub reason_codes: Vec<String>,
    /// 0.0 (weak / risky) .. 1.0 (concrete / safe)
    pub task_confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanAdequacySummary {
    /// 0.0 = structurally thin .. 1.0 = adequate for estimated complexity
    pub score: f32,
    pub is_too_thin: bool,
    pub reason_codes: Vec<String>,
    pub detail_target_min_tasks: usize,
    pub estimated_goal_complexity: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanRefinementReport {
    pub per_task: Vec<TaskGapFinding>,
    pub aggregate_unresolved_risk: f32,
    pub critical_count: usize,
    pub suggested_clarifying_questions: Vec<String>,
    pub adequacy: PlanAdequacySummary,
}

/// Word-count heuristic aligned with [`super::intake_router::complexity_heuristic`].
pub fn estimate_goal_word_complexity(goal: &str) -> u8 {
    let words = goal.split_whitespace().count() as u8;
    if words <= 6 {
        2
    } else if words <= 16 {
        5
    } else if words <= 30 {
        7
    } else {
        9
    }
}

/// Optional bump when the router already classified the goal (search intent, etc.).
pub fn effective_goal_complexity(goal: &str, router_hint: Option<u8>) -> u8 {
    let base = estimate_goal_word_complexity(goal);
    router_hint.map(|h| h.max(base)).unwrap_or(base)
}

fn danger_keywords() -> &'static [&'static str] {
    &[
        "rm -rf",
        "delete all",
        "drop database",
        "truncate table",
        "force push",
        "chmod 777",
        "format c:",
        "mkfs",
        "dd if=",
    ]
}

fn vague_phrases() -> &'static [&'static str] {
    &[
        "fix stuff",
        "clean up",
        "misc",
        "various",
        "todo",
        "something",
        "somehow",
        "refactor everything",
        "placeholder",
        "tbd",
        "stub",
    ]
}

/// Single-source text checks for orchestrator [`super::quality_gate`] (no dependency graph / complexity).
pub fn orchestrator_node_text_findings(description: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let dl = description.to_ascii_lowercase();
    if description.trim().len() < 12 {
        out.push("vague_short");
    }
    for ph in vague_phrases() {
        if dl.contains(ph) {
            out.push("vague_phrase");
            break;
        }
    }
    for kw in danger_keywords() {
        if dl.contains(kw) {
            out.push("risk_destructive");
            break;
        }
    }
    out
}

fn tasks_with_nonempty_files(tasks: &[PlanAdequacyTask]) -> usize {
    tasks
        .iter()
        .filter(|t| {
            t.files
                .iter()
                .any(|f| !f.is_empty() && !f.eq_ignore_ascii_case("tbd"))
        })
        .count()
}

/// Many tasks share the same opening phrase — common failure mode for “detailed” rewrites.
fn repeated_task_opening(tasks: &[PlanAdequacyTask]) -> bool {
    if tasks.len() < 4 {
        return false;
    }
    let threshold = ((tasks.len() as f32) * 0.4).ceil() as usize;
    let threshold = threshold.max(3);
    let mut prefix_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for t in tasks {
        let p = t
            .description
            .to_ascii_lowercase()
            .split_whitespace()
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        if p.len() < 10 {
            continue;
        }
        *prefix_counts.entry(p).or_insert(0) += 1;
    }
    prefix_counts.values().any(|&c| c >= threshold)
}

fn structural_noise_penalties(
    tasks: &[PlanAdequacyTask],
    estimated: u8,
    detail_target: usize,
) -> (Vec<String>, f32) {
    let mut codes = Vec::new();
    let mut mul = 1.0_f32;
    if estimated >= 6 && tasks.len() >= detail_target.max(3) {
        let n = tasks.len().max(1);
        let sum_len: usize = tasks.iter().map(|t| t.description.len()).sum();
        let avg = sum_len / n;
        let linked = tasks_with_nonempty_files(tasks);
        if avg < 28 && linked * 3 < n {
            codes.push("verbose_but_low_surface".to_string());
            mul *= 0.86;
        }
    }
    if repeated_task_opening(tasks) {
        codes.push("repeated_task_phrasing".to_string());
        mul *= 0.88;
    }
    (codes, mul)
}

fn refinement_regression_penalties(
    prev: &[PlanAdequacyTask],
    cur: &[PlanAdequacyTask],
    detail_target: usize,
) -> (Vec<String>, f32) {
    if prev.is_empty() || cur.is_empty() {
        return (Vec::new(), 1.0);
    }
    let mut codes = Vec::new();
    let mut mul = 1.0_f32;
    if cur.len() < prev.len() && prev.len() >= detail_target.saturating_sub(1).max(2) {
        codes.push("possible_rewrite_compression".to_string());
        mul *= 0.87;
    }
    let lp = tasks_with_nonempty_files(prev);
    let lc = tasks_with_nonempty_files(cur);
    if lp > 0 && lc < lp {
        codes.push("file_linkage_reduced".to_string());
        mul *= 0.85;
    }
    let sp: usize = prev.iter().map(|t| t.description.len()).sum();
    let sc: usize = cur.iter().map(|t| t.description.len()).sum();
    if sc + 50 < (sp * 9 / 10).max(1) && cur.len() <= prev.len() {
        codes.push("description_mass_reduced".to_string());
        mul *= 0.9;
    }
    (codes, mul)
}

fn detail_target_min_tasks(complexity: u8, depth_bonus: i8) -> usize {
    let mut t = match complexity {
        0..=3 => 1,
        4 | 5 => 2,
        6 | 7 => 4,
        8 => 6,
        _ => 8,
    };
    let b = depth_bonus.clamp(-2, 4);
    t = (t as i32 + b as i32).max(1) as usize;
    t
}

fn goal_implies_verification_needed(goal: &str) -> bool {
    let g = goal.to_ascii_lowercase();
    let coding = [
        "implement",
        "refactor",
        "migrate",
        "fix bug",
        "fix the",
        "add feature",
        "change api",
        "deprecat",
        "schema",
        "database",
    ];
    coding.iter().any(|k| g.contains(k))
}

fn goal_has_path_like_token(goal: &str) -> bool {
    let exts = [
        ".rs", ".toml", ".md", ".yaml", ".yml", ".json", ".vox", ".tsx", ".ts", ".js",
    ];
    for w in goal.split_whitespace() {
        let w = w.trim_matches(|c: char| {
            c == '`' || c == '"' || c == '\'' || c == '(' || c == ')' || c == ',' || c == ';'
        });
        let n = w.replace('\\', "/");
        if n.contains('/')
            && (exts.iter().any(|e| n.ends_with(e))
                || n.starts_with("crates/")
                || n.starts_with("src/")
                || n.starts_with("docs/"))
        {
            return true;
        }
    }
    false
}

fn plan_has_verification_hint(tasks: &[PlanAdequacyTask]) -> bool {
    tasks.iter().any(|t| {
        let d = t.description.to_ascii_lowercase();
        d.contains("test")
            || d.contains("verify")
            || d.contains("validation")
            || d.contains("assert")
            || d.contains("regression")
            || d.contains("cargo test")
            || d.contains("lint")
            || d.contains("clippy")
    })
}

fn apply_shared_text_gap_codes(
    codes: &mut Vec<String>,
    score: &mut f32,
    task_critical: &mut bool,
    findings: Vec<&'static str>,
) {
    for finding in findings {
        match finding {
            "vague_short" => {
                codes.push("vague_short".to_string());
                *score *= 0.65;
            }
            "vague_phrase" => {
                codes.push("vague_phrase".to_string());
                *score *= 0.75;
            }
            "risk_destructive" => {
                codes.push("risk_destructive".to_string());
                *score *= 0.35;
                *task_critical = true;
            }
            _ => {}
        }
    }
}

fn per_task_findings(tasks: &[PlanAdequacyTask]) -> (Vec<TaskGapFinding>, usize) {
    let id_set: HashSet<usize> = tasks.iter().map(|t| t.id).collect();
    let mut per_task = Vec::with_capacity(tasks.len());
    let mut critical = 0usize;

    for t in tasks {
        let mut codes = Vec::new();
        let mut score: f32 = 1.0;
        let mut task_critical = false;
        let dl = t.description.to_ascii_lowercase();

        apply_shared_text_gap_codes(
            &mut codes,
            &mut score,
            &mut task_critical,
            orchestrator_node_text_findings(&t.description),
        );

        if dl.contains("tbd") || t.files.iter().any(|f| f.eq_ignore_ascii_case("tbd")) {
            codes.push("tbd_placeholder".to_string());
            score *= 0.7;
        }
        if t.estimated_complexity >= 7 && !dl.contains("test") && !dl.contains("verify") {
            codes.push("heavy_without_test_hint".to_string());
            score *= 0.82;
        }
        for &d in &t.depends_on {
            if !id_set.contains(&d) || d >= t.id {
                codes.push("deps_incomplete_or_order".to_string());
                score *= 0.55;
                break;
            }
        }

        score = score.clamp(0.0, 1.0);
        if task_critical {
            critical += 1;
        }

        per_task.push(TaskGapFinding {
            task_id: t.id,
            reason_codes: codes,
            task_confidence: score,
        });
    }

    (per_task, critical)
}

fn aggregate_risk(per_task: &[TaskGapFinding]) -> f32 {
    let n = per_task.len().max(1) as f32;
    per_task
        .iter()
        .map(|p| 1.0 - p.task_confidence)
        .sum::<f32>()
        / n
}

fn build_suggested_questions(
    tasks: &[PlanAdequacyTask],
    findings: &[TaskGapFinding],
) -> Vec<String> {
    let mut out = Vec::new();
    for (task, gap) in tasks.iter().zip(findings.iter()) {
        if gap.reason_codes.is_empty() {
            continue;
        }
        if gap.reason_codes.iter().any(|c| c == "risk_destructive") {
            out.push(format!(
                "Task {} proposes potentially destructive actions — confirm scope and safeguards?",
                task.id
            ));
        } else if gap
            .reason_codes
            .iter()
            .any(|c| c == "deps_incomplete_or_order")
        {
            out.push(format!(
                "Task {} has dependency issues — which prerequisites must finish first?",
                task.id
            ));
        } else if gap
            .reason_codes
            .iter()
            .any(|c| c == "vague_short" || c == "vague_phrase" || c == "tbd_placeholder")
        {
            out.push(format!(
                "Task {} is underspecified — what files, interfaces, or acceptance criteria apply?",
                task.id
            ));
        }
        if out.len() >= 8 {
            break;
        }
    }
    out
}

fn compute_adequacy_summary(
    goal: &str,
    tasks: &[PlanAdequacyTask],
    router_complexity_hint: Option<u8>,
    depth_bonus: i8,
    aggregate_unresolved_risk: f32,
    fatigued: bool,
) -> PlanAdequacySummary {
    let mut estimated = effective_goal_complexity(goal, router_complexity_hint);
    if fatigued {
        // Escalate complexity expectation when human is burnt out (forcing more precise plan decomposition).
        estimated = estimated.max(8);
    }
    let mut detail_target = detail_target_min_tasks(estimated, depth_bonus);
    if fatigued {
        detail_target = (detail_target * 2).max(4); // Force much higher task decomposition when fatigued.
    }

    let mut thin_codes = Vec::new();
    let mut score: f32 = 1.0;

    if estimated >= 5 && tasks.len() < detail_target {
        thin_codes.push("too_few_tasks".to_string());
        let gap = (detail_target - tasks.len()) as f32 / detail_target as f32;
        score *= (1.0 - gap * 0.45).clamp(0.35, 1.0);
    }

    let mega = tasks.iter().filter(|t| t.estimated_complexity >= 9).count();
    if mega >= 2 && tasks.len() < detail_target + 2 {
        thin_codes.push("mega_task_cluster".to_string());
        score *= 0.78;
    }

    if goal_implies_verification_needed(goal)
        && !plan_has_verification_hint(tasks)
        && !tasks.is_empty()
    {
        thin_codes.push("missing_plan_verification".to_string());
        score *= 0.72;
    }

    if tasks.len() >= 4 && tasks.iter().all(|t| t.depends_on.is_empty()) {
        thin_codes.push("flat_dependency_graph".to_string());
        score *= 0.85;
    }

    if goal_has_path_like_token(goal)
        && tasks.iter().all(|t| {
            t.files.is_empty()
                || t.files
                    .iter()
                    .all(|f| f.is_empty() || f.eq_ignore_ascii_case("tbd"))
        })
    {
        thin_codes.push("goal_paths_unlinked".to_string());
        score *= 0.8;
    }

    score = score.clamp(0.0, 1.0);

    // Thin if structural signals fire or score is low or overall gap risk is high
    let is_too_thin =
        !thin_codes.is_empty() && score < 0.82 || score < 0.58 || aggregate_unresolved_risk > 0.32;

    PlanAdequacySummary {
        score,
        is_too_thin,
        reason_codes: thin_codes,
        detail_target_min_tasks: detail_target,
        estimated_goal_complexity: estimated,
    }
}

fn adequacy_thinness_gate(
    reason_codes: &[String],
    score: f32,
    aggregate_unresolved_risk: f32,
) -> bool {
    !reason_codes.is_empty() && score < 0.82 || score < 0.58 || aggregate_unresolved_risk > 0.32
}

/// Full report: per-task gaps + plan adequacy. `scope_file_count` nudges complexity when the user
/// pinned files (optional).
///
/// `prior_tasks_after_refine`: when set, compares the plan **before** a refinement LLM pass to
/// `tasks` (**after**) to flag rewrite compression and lost file linkage.
pub fn analyze_plan_refinement_report(
    goal: &str,
    scope_file_count: usize,
    router_complexity_hint: Option<u8>,
    plan_depth_bonus: i8,
    tasks: &[PlanAdequacyTask],
    fatigued: bool,
) -> PlanRefinementReport {
    analyze_plan_refinement_report_with_prior(
        goal,
        scope_file_count,
        router_complexity_hint,
        plan_depth_bonus,
        tasks,
        None,
        fatigued,
    )
}

pub fn analyze_plan_refinement_report_with_prior(
    goal: &str,
    scope_file_count: usize,
    router_complexity_hint: Option<u8>,
    plan_depth_bonus: i8,
    tasks: &[PlanAdequacyTask],
    prior_tasks_after_refine: Option<&[PlanAdequacyTask]>,
    fatigued: bool,
) -> PlanRefinementReport {
    let mut router = router_complexity_hint;
    if scope_file_count >= 4 {
        router = Some(router.unwrap_or(5).max(7));
    } else if scope_file_count >= 1 {
        router = Some(router.unwrap_or(5).max(6));
    }

    let (per_task, critical_count) = per_task_findings(tasks);
    let aggregate_unresolved_risk = aggregate_risk(&per_task);
    let suggested = build_suggested_questions(tasks, &per_task);

    let mut adequacy = compute_adequacy_summary(
        goal,
        tasks,
        router,
        plan_depth_bonus,
        aggregate_unresolved_risk,
        fatigued,
    );

    let (sn_codes, sn_mul) = structural_noise_penalties(
        tasks,
        adequacy.estimated_goal_complexity,
        adequacy.detail_target_min_tasks,
    );
    adequacy.reason_codes.extend(sn_codes);
    adequacy.score = (adequacy.score * sn_mul).clamp(0.0, 1.0);

    if let Some(prior) = prior_tasks_after_refine {
        let (rg_codes, rg_mul) =
            refinement_regression_penalties(prior, tasks, adequacy.detail_target_min_tasks);
        adequacy.reason_codes.extend(rg_codes);
        adequacy.score = (adequacy.score * rg_mul).clamp(0.0, 1.0);
    }

    adequacy.is_too_thin = adequacy_thinness_gate(
        &adequacy.reason_codes,
        adequacy.score,
        aggregate_unresolved_risk,
    );

    PlanRefinementReport {
        per_task,
        aggregate_unresolved_risk,
        critical_count,
        suggested_clarifying_questions: suggested,
        adequacy,
    }
}

/// Map orchestrator [`super::PlanNode`]s to adequacy tasks. Complexity is inferred from description
/// length when not present on the node type.
pub fn plan_nodes_to_adequacy_tasks(nodes: &[super::PlanNode]) -> Vec<PlanAdequacyTask> {
    let id_by_node: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.node_id.clone(), i + 1))
        .collect();

    nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let deps: Vec<usize> = n
                .depends_on
                .iter()
                .filter_map(|d| id_by_node.get(d).copied())
                .collect();
            let files: Vec<String> = n
                .execution_policy
                .file_manifest
                .iter()
                .map(|f| f.path.to_string_lossy().into_owned())
                .collect();
            let inferred_cx = (crate::compaction::CompactionEngine::estimate_tokens(&n.description)
                / 10)
                .clamp(1, 10) as u8;
            PlanAdequacyTask {
                id: i + 1,
                description: n.description.clone(),
                files,
                estimated_complexity: inferred_cx.max(3),
                depends_on: deps,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task(id: usize, desc: &str, cx: u8) -> PlanAdequacyTask {
        PlanAdequacyTask {
            id,
            description: desc.to_string(),
            files: vec![],
            estimated_complexity: cx,
            depends_on: vec![],
        }
    }

    #[test]
    fn flags_too_few_tasks_for_complex_goal() {
        let goal = "migrate authentication across crates/vox-auth, crates/vox-mcp, and update docs; add regression tests";
        let tasks = vec![sample_task(1, "do the migration work", 8)];
        let r = analyze_plan_refinement_report(goal, 0, None, 0, &tasks, false);
        assert!(r.adequacy.is_too_thin);
        assert!(r.adequacy.reason_codes.iter().any(|c| c == "too_few_tasks"));
    }

    #[test]
    fn per_task_destructive_still_critical() {
        let goal = "small cleanup";
        let tasks = vec![sample_task(1, "rm -rf /unused", 3)];
        let r = analyze_plan_refinement_report(goal, 0, None, 0, &tasks, false);
        assert!(r.critical_count >= 1);
        assert!(r.aggregate_unresolved_risk > 0.2);
    }

    #[test]
    fn prior_snapshot_flags_task_compression() {
        let goal = "migrate authentication across crates/vox-auth and crates/vox-mcp with regression tests";
        let prior = vec![
            sample_task(1, "audit current auth flows in crates/vox-auth", 5),
            sample_task(2, "port token handling in crates/vox-mcp", 6),
            sample_task(3, "add integration tests for auth edge cases", 5),
            sample_task(4, "update docs for new auth contract", 4),
        ];
        let shrunk = vec![
            sample_task(1, "audit current auth flows in crates/vox-auth", 5),
            sample_task(2, "finish migration and testing", 8),
        ];
        let r = analyze_plan_refinement_report_with_prior(goal, 0, None, 0, &shrunk, Some(&prior), false);
        assert!(
            r.adequacy
                .reason_codes
                .iter()
                .any(|c| c == "possible_rewrite_compression"),
            "{:?}",
            r.adequacy.reason_codes
        );
    }

    #[test]
    fn repeated_openings_reduce_adequacy_score() {
        let goal = "implement several related parser fixes across the compiler frontend";
        let open =
            "Add detailed error recovery for unexpected tokens in the grammar pipeline with tests";
        let tasks: Vec<PlanAdequacyTask> =
            (1..=5usize).map(|id| sample_task(id, open, 5)).collect();
        let r = analyze_plan_refinement_report(goal, 0, None, 0, &tasks, false);
        assert!(
            r.adequacy
                .reason_codes
                .iter()
                .any(|c| c == "repeated_task_phrasing"),
            "{:?}",
            r.adequacy.reason_codes
        );
    }
}
