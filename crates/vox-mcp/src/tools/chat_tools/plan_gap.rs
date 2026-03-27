//! Deterministic gap / risk heuristics for MCP-generated plans (tier-1 refinement signals).

use super::params::PlanTask;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct TaskGapFinding {
    pub task_id: usize,
    pub reason_codes: Vec<String>,
    /// 0.0 (weak / risky) .. 1.0 (concrete / safe)
    pub task_confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanGapReport {
    pub per_task: Vec<TaskGapFinding>,
    pub aggregate_unresolved_risk: f32,
    pub critical_count: usize,
    pub suggested_clarifying_questions: Vec<String>,
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
    ]
}

/// Analyze tasks for vagueness, destructive cues, and dependency integrity.
pub fn analyze_plan_gaps(tasks: &[PlanTask]) -> PlanGapReport {
    let id_set: HashSet<usize> = tasks.iter().map(|t| t.id).collect();
    let mut per_task = Vec::with_capacity(tasks.len());
    let mut critical = 0usize;

    for t in tasks {
        let mut codes = Vec::new();
        let mut score: f32 = 1.0;
        let mut task_critical = false;
        let dl = t.description.to_ascii_lowercase();

        if t.description.trim().len() < 12 {
            codes.push("vague_short".to_string());
            score *= 0.65;
        }
        if dl.contains("tbd") || t.files.iter().any(|f| f.eq_ignore_ascii_case("tbd")) {
            codes.push("tbd_placeholder".to_string());
            score *= 0.7;
        }
        for ph in vague_phrases() {
            if dl.contains(ph) {
                codes.push("vague_phrase".to_string());
                score *= 0.75;
                break;
            }
        }
        for kw in danger_keywords() {
            if dl.contains(kw) {
                codes.push("risk_destructive".to_string());
                score *= 0.35;
                task_critical = true;
                break;
            }
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

    let n = per_task.len().max(1) as f32;
    let aggregate_unresolved_risk: f32 = per_task
        .iter()
        .map(|p| 1.0 - p.task_confidence)
        .sum::<f32>()
        / n;

    let suggested = build_suggested_questions(tasks, &per_task);

    PlanGapReport {
        per_task,
        aggregate_unresolved_risk,
        critical_count: critical,
        suggested_clarifying_questions: suggested,
    }
}

fn build_suggested_questions(tasks: &[PlanTask], findings: &[TaskGapFinding]) -> Vec<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task(id: usize, desc: &str) -> PlanTask {
        PlanTask {
            id,
            description: desc.to_string(),
            files: vec![],
            estimated_complexity: 3,
            depends_on: vec![],
        }
    }

    #[test]
    fn flags_destructive() {
        let tasks = vec![sample_task(1, "rm -rf /unused")];
        let r = analyze_plan_gaps(&tasks);
        assert!(
            r.per_task[0]
                .reason_codes
                .iter()
                .any(|c| c == "risk_destructive")
        );
        assert!(r.aggregate_unresolved_risk > 0.3);
    }

    #[test]
    fn flags_bad_deps() {
        let tasks = vec![PlanTask {
            id: 2,
            description: "Second".into(),
            files: vec![],
            estimated_complexity: 2,
            depends_on: vec![99],
        }];
        let r = analyze_plan_gaps(&tasks);
        assert!(
            r.per_task[0]
                .reason_codes
                .iter()
                .any(|c| c == "deps_incomplete_or_order")
        );
    }
}
