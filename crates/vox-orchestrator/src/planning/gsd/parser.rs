//! Deterministic parser and renderer for `.planning/` GSD artifacts.

use std::path::Path;
use vox_orchestrator_types::gsd::{
    GsdDocumentKind, GsdMilestone, GsdMilestoneStatus, GsdPhase, GsdPhaseStatus, GsdPlanWave,
    GsdRequirement, GsdWaveTask,
};

#[derive(Debug, Clone, Default)]
pub struct ParsedPlanningState {
    pub active_milestone: Option<GsdMilestone>,
    pub phases: Vec<GsdPhase>,
    pub requirements: Vec<GsdRequirement>,
}

/// Extract the active milestone, phases, and requirements from `.planning/` files.
pub fn parse_planning_workspace(planning_dir: &Path, now_ms: i64) -> ParsedPlanningState {
    let state_json = planning_dir.join("state.json");
    let state_md = planning_dir.join("STATE.md");
    let roadmap_md = planning_dir.join("ROADMAP.md");
    let reqs_md = planning_dir.join("REQUIREMENTS.md");

    let mut milestone_id = "v1.0".to_string();
    let mut milestone_title = "Active GSD Milestone".to_string();
    let mut active_phase_num: Option<u32> = None;

    if let Ok(raw) = std::fs::read_to_string(&state_json) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(m) = v.get("milestone").and_then(|x| x.as_str()) {
                if !m.trim().is_empty() {
                    milestone_id = m.trim().to_string();
                }
            }
            if let Some(p) = v.get("active_phase").and_then(|x| x.as_u64()) {
                active_phase_num = u32::try_from(p).ok();
            }
        }
    }

    if let Ok(raw) = std::fs::read_to_string(&state_md) {
        for line in raw.lines().take(30) {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("milestone:") {
                let val = rest.trim().trim_matches('"');
                if !val.is_empty() {
                    milestone_id = val.to_string();
                }
            } else if let Some(rest) = trimmed.strip_prefix("milestone_name:") {
                let val = rest.trim().trim_matches('"');
                if !val.is_empty() {
                    milestone_title = val.to_string();
                }
            }
        }
    }

    let mut phases = Vec::new();
    if let Ok(roadmap_raw) = std::fs::read_to_string(&roadmap_md) {
        phases = parse_roadmap_phases(&roadmap_raw, &milestone_id, now_ms);
        if let Some(title) = parse_roadmap_milestone_title(&roadmap_raw, &milestone_id) {
            milestone_title = title;
        }
    }

    if let Some(ap) = active_phase_num {
        for p in &mut phases {
            if p.phase_number == ap && p.status == GsdPhaseStatus::Pending {
                p.status = GsdPhaseStatus::Executing;
            }
        }
    }

    let requirements = std::fs::read_to_string(&reqs_md)
        .map(|raw| parse_requirements(&raw, &milestone_id, now_ms))
        .unwrap_or_default();

    let start_phase = phases.iter().map(|p| p.phase_number).min().unwrap_or(1);
    let end_phase = phases
        .iter()
        .map(|p| p.phase_number)
        .max()
        .unwrap_or(start_phase);

    let active_milestone = Some(GsdMilestone {
        milestone_id,
        title: milestone_title,
        status: GsdMilestoneStatus::Active,
        start_phase,
        end_phase,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
        completed_at_ms: None,
    });

    ParsedPlanningState {
        active_milestone,
        phases,
        requirements,
    }
}

fn parse_roadmap_milestone_title(roadmap: &str, milestone_id: &str) -> Option<String> {
    for line in roadmap.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# Roadmap:") && trimmed.contains(milestone_id) {
            return Some(trimmed.trim_start_matches("# Roadmap:").trim().to_string());
        }
    }
    None
}

/// Parse `- [ ] **Phase N: Title**` lines from `ROADMAP.md` up to the first `---` section separator.
pub fn parse_roadmap_phases(roadmap: &str, milestone_id: &str, now_ms: i64) -> Vec<GsdPhase> {
    let mut out = Vec::new();
    let mut seen_active_heading = false;
    for line in roadmap.lines() {
        let trimmed = line.trim();
        if trimmed == "---" && !out.is_empty() {
            break;
        }
        if trimmed.starts_with("# Roadmap:") && trimmed.contains(milestone_id) {
            seen_active_heading = true;
            continue;
        }
        if !seen_active_heading && roadmap.contains("# Roadmap:") && !trimmed.contains("**Phase ") {
            continue;
        }
        let Some((done, rest)) = strip_checkbox(trimmed) else {
            continue;
        };
        if let Some(after_phase) = rest.strip_prefix("**Phase ") {
            if let Some((num_str, title_part)) = after_phase.split_once(':') {
                if let Ok(phase_number) = num_str.trim().parse::<u32>() {
                    let clean_title = title_part
                        .split("**")
                        .next()
                        .unwrap_or(title_part)
                        .trim()
                        .to_string();
                    out.push(GsdPhase {
                        milestone_id: milestone_id.to_string(),
                        phase_number,
                        title: clean_title,
                        status: if done {
                            GsdPhaseStatus::Completed
                        } else {
                            GsdPhaseStatus::Pending
                        },
                        wave_count: 1,
                        plan_session_id: None,
                        updated_at_ms: now_ms,
                    });
                }
            }
        }
    }
    out
}

/// Parse `- [ ] **REQ-ID**: Description` lines from `REQUIREMENTS.md` up to the first `---`.
pub fn parse_requirements(reqs: &str, milestone_id: &str, now_ms: i64) -> Vec<GsdRequirement> {
    let mut out = Vec::new();
    let mut current_phase = 1_u32;
    for line in reqs.lines() {
        let trimmed = line.trim();
        if trimmed == "---" && !out.is_empty() {
            break;
        }
        if trimmed.starts_with("### ") && trimmed.contains("Phase ") {
            if let Some(idx) = trimmed.find("Phase ") {
                let digits: String = trimmed[idx + 6..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(p) = digits.parse::<u32>() {
                    current_phase = p;
                }
            }
        }
        let Some((satisfied, rest)) = strip_checkbox(trimmed) else {
            continue;
        };
        if let Some(after_bold) = rest.strip_prefix("**") {
            if let Some((req_id, desc)) = after_bold.split_once("**:") {
                out.push(GsdRequirement {
                    req_id: req_id.trim().to_string(),
                    milestone_id: milestone_id.to_string(),
                    phase_number: current_phase,
                    description: desc.trim().to_string(),
                    satisfied,
                    verification_ref: None,
                    updated_at_ms: now_ms,
                });
            }
        }
    }
    out
}

fn strip_checkbox(line: &str) -> Option<(bool, &str)> {
    if let Some(r) = line.strip_prefix("- [x] ") {
        Some((true, r.trim()))
    } else if let Some(r) = line.strip_prefix("- [X] ") {
        Some((true, r.trim()))
    } else if let Some(r) = line.strip_prefix("- [ ] ") {
        Some((false, r.trim()))
    } else {
        None
    }
}

/// Parse wave headings (`## Wave N`) and task bullets (`- [ ] \`task_id\` Title | files: a, b | verify: cmd`) from `PLAN.md`.
pub fn parse_phase_plan_waves(plan_md: &str) -> Vec<GsdPlanWave> {
    let mut waves: Vec<GsdPlanWave> = Vec::new();
    let mut current_wave: u32 = 1;
    for line in plan_md.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("## Wave ") {
            if let Ok(n) = rest
                .split_whitespace()
                .next()
                .unwrap_or("1")
                .trim_matches(':')
                .parse::<u32>()
            {
                current_wave = n.max(1);
            }
            continue;
        }
        if let Some((_, rest)) = strip_checkbox(trimmed) {
            let task = parse_task_line(rest, current_wave, waves.len());
            if let Some(existing) = waves.iter_mut().find(|w| w.wave_number == current_wave) {
                existing.tasks.push(task);
            } else {
                waves.push(GsdPlanWave {
                    wave_number: current_wave,
                    tasks: vec![task],
                });
            }
        }
    }
    if waves.is_empty() {
        waves.push(GsdPlanWave {
            wave_number: 1,
            tasks: vec![GsdWaveTask {
                task_id: "wave1-task1".to_string(),
                title: "Implement phase deliverable and verify".to_string(),
                files: vec!["Cargo.toml".to_string()],
                verification_cmd: "cargo check --workspace".to_string(),
            }],
        });
    }
    waves.sort_by_key(|w| w.wave_number);
    waves
}

fn parse_task_line(rest: &str, wave: u32, idx_hint: usize) -> GsdWaveTask {
    let segments: Vec<&str> = rest.split('|').map(str::trim).collect();
    let head = segments.first().copied().unwrap_or(rest);
    let (task_id, title) = if let Some(stripped) = head.strip_prefix('`') {
        if let Some((id, tail)) = stripped.split_once('`') {
            (id.trim().to_string(), tail.trim().to_string())
        } else {
            (format!("w{wave}-t{}", idx_hint + 1), head.to_string())
        }
    } else {
        (format!("w{wave}-t{}", idx_hint + 1), head.to_string())
    };

    let mut files = vec!["Cargo.toml".to_string()];
    let mut verification_cmd = "cargo check".to_string();
    for seg in segments.iter().skip(1) {
        if let Some(f_list) = seg.strip_prefix("files:") {
            let parsed: Vec<String> = f_list
                .split(',')
                .map(|s| s.trim().trim_matches('`').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !parsed.is_empty() {
                files = parsed;
            }
        } else if let Some(v_cmd) = seg.strip_prefix("verify:") {
            let clean = v_cmd.trim().trim_matches('`');
            if !clean.is_empty() {
                verification_cmd = clean.to_string();
            }
        }
    }
    GsdWaveTask {
        task_id,
        title: if title.is_empty() {
            "Execute wave step".to_string()
        } else {
            title
        },
        files,
        verification_cmd,
    }
}

/// Render wave-structured `PLAN.md` markdown compatible with `parse_phase_plan_waves`.
#[must_use]
pub fn render_phase_plan_md(phase_number: u32, title: &str, waves: &[GsdPlanWave]) -> String {
    let mut out = format!("# Phase {phase_number}: {title} — Execution Plan\n\n");
    for wave in waves {
        out.push_str(&format!("## Wave {}\n\n", wave.wave_number));
        for t in &wave.tasks {
            out.push_str(&format!(
                "- [ ] `{}` {} | files: {} | verify: `{}`\n",
                t.task_id,
                t.title,
                t.files.join(", "),
                t.verification_cmd
            ));
        }
        out.push('\n');
    }
    out
}

/// Render `.planning/state.json`.
#[must_use]
pub fn render_state_json(
    milestone_id: &str,
    milestone_name: &str,
    active_phase: u32,
    status: &str,
    next_action: &str,
) -> String {
    serde_json::json!({
        "milestone": milestone_id,
        "milestone_name": milestone_name,
        "active_phase": active_phase,
        "status": status,
        "next_action": next_action,
    })
    .to_string()
        + "\n"
}

/// Infer [`GsdDocumentKind`] from a `.planning/` relative path.
#[must_use]
pub fn infer_document_kind(rel_path: &str) -> GsdDocumentKind {
    let upper = rel_path.to_ascii_uppercase();
    if upper.ends_with("PROJECT.MD") {
        GsdDocumentKind::Project
    } else if upper.ends_with("ROADMAP.MD") {
        GsdDocumentKind::Roadmap
    } else if upper.ends_with("REQUIREMENTS.MD") {
        GsdDocumentKind::Requirements
    } else if upper.ends_with("STATE.MD") || upper.ends_with("STATE.JSON") {
        GsdDocumentKind::State
    } else if upper.contains("SPEC.MD") {
        GsdDocumentKind::Spec
    } else if upper.contains("CONTEXT.MD") {
        GsdDocumentKind::Context
    } else if upper.contains("PLAN.MD") {
        GsdDocumentKind::Plan
    } else if upper.contains("SUMMARY.MD") {
        GsdDocumentKind::Summary
    } else if upper.contains("VERIFICATION.MD") {
        GsdDocumentKind::Verification
    } else if upper.contains("REVIEW.MD") {
        GsdDocumentKind::Review
    } else if upper.contains("DEBUG.MD") {
        GsdDocumentKind::Debug
    } else {
        GsdDocumentKind::Codebase
    }
}
