//! End-to-end GSD workspace + `VoxDb` CAS lifecycle transitions.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use vox_db::VoxDb;
use vox_orchestrator_types::gsd::{
    GsdDocument, GsdDocumentKind, GsdMilestone, GsdMilestoneStatus, GsdPhase, GsdPhaseStatus,
    GsdPlanWave, GsdProgressSnapshot, GsdWaveTask,
};

use crate::orchestrator::OrchestratorError;
use crate::planning::PlanNode;
use crate::planning::gsd::parser::{
    infer_document_kind, parse_phase_plan_waves, parse_planning_workspace, render_phase_plan_md,
    render_state_json,
};
use crate::planning::gsd::scheduler::waves_to_plan_nodes;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GsdHealthReport {
    pub healthy: bool,
    pub planning_dir_exists: bool,
    pub present_files: Vec<String>,
    pub issues: Vec<String>,
}

fn now_unix_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(0)
}

fn io_err(err: std::io::Error) -> OrchestratorError {
    OrchestratorError::DatabaseError(err.to_string())
}

fn db_err(err: vox_db::StoreError) -> OrchestratorError {
    OrchestratorError::DatabaseError(err.to_string())
}

fn phase_dir(workspace_root: &Path, phase_number: u32) -> PathBuf {
    workspace_root
        .join(".planning")
        .join("phases")
        .join(format!("{phase_number:03}"))
}

/// Inspect workspace `.planning/` health and return diagnostics without mutating files.
#[must_use]
pub fn health_check(workspace_root: &Path) -> GsdHealthReport {
    let pdir = workspace_root.join(".planning");
    if !pdir.is_dir() {
        return GsdHealthReport {
            healthy: false,
            planning_dir_exists: false,
            present_files: vec![],
            issues: vec![".planning/ directory missing — run /gsd-new-milestone".to_string()],
        };
    }
    let required = ["PROJECT.md", "ROADMAP.md", "REQUIREMENTS.md", "STATE.md"];
    let mut present_files = Vec::new();
    let mut issues = Vec::new();
    for f in required {
        if pdir.join(f).is_file() {
            present_files.push(format!(".planning/{f}"));
        } else {
            issues.push(format!("missing required GSD file .planning/{f}"));
        }
    }
    if pdir.join("state.json").is_file() {
        present_files.push(".planning/state.json".to_string());
    }
    GsdHealthReport {
        healthy: issues.is_empty(),
        planning_dir_exists: true,
        present_files,
        issues,
    }
}

/// Sync `.planning/` markdown/state files into `VoxDb` (`gsd_*` tables + Arca CAS blobs).
pub async fn sync_planning_dir_to_db(
    workspace_root: &Path,
    db: &VoxDb,
) -> Result<GsdProgressSnapshot, OrchestratorError> {
    let now = now_unix_ms();
    let pdir = workspace_root.join(".planning");
    let parsed = parse_planning_workspace(&pdir, now);

    let milestone_id = if let Some(ref ms) = parsed.active_milestone {
        db.upsert_gsd_milestone(ms).await.map_err(db_err)?;
        ms.milestone_id.clone()
    } else {
        "v1.0".to_string()
    };

    for phase in &parsed.phases {
        db.upsert_gsd_phase(phase).await.map_err(db_err)?;
    }
    for req in &parsed.requirements {
        db.upsert_gsd_requirement(req).await.map_err(db_err)?;
    }

    let mut stack = vec![pdir.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|s| s.to_str()) != Some("milestones") {
                    stack.push(path);
                }
                continue;
            }
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext != "md" && ext != "json" {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let rel = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let kind = infer_document_kind(&rel);
            let doc_id = format!("{milestone_id}:{rel}");
            let phase_num = extract_phase_number_from_rel(&rel);
            db.store_gsd_document(&doc_id, &milestone_id, phase_num, kind, &rel, &content, now)
                .await
                .map_err(db_err)?;
        }
    }

    db.gsd_progress_snapshot(Some(&milestone_id))
        .await
        .map_err(db_err)
}

fn extract_phase_number_from_rel(rel: &str) -> Option<u32> {
    let idx = rel.find("phases/")?;
    let rest = &rel[idx + 7..];
    let seg = rest.split('/').next()?;
    let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse::<u32>().ok()
}

/// Load current GSD progress snapshot from `VoxDb` (if present and populated) or disk.
pub async fn load_workspace_status(
    workspace_root: &Path,
    db: Option<&VoxDb>,
) -> Result<GsdProgressSnapshot, OrchestratorError> {
    if let Some(database) = db {
        if workspace_root.join(".planning").is_dir() {
            return sync_planning_dir_to_db(workspace_root, database).await;
        }
        return database.gsd_progress_snapshot(None).await.map_err(db_err);
    }
    let parsed = parse_planning_workspace(&workspace_root.join(".planning"), now_unix_ms());
    let next = if parsed.active_milestone.is_none() {
        "gsd-new-milestone"
    } else if parsed
        .phases
        .iter()
        .any(|p| p.status != GsdPhaseStatus::Completed)
    {
        "gsd-execute-phase"
    } else {
        "gsd-complete-milestone"
    };
    Ok(GsdProgressSnapshot {
        active_milestone: parsed.active_milestone,
        phases: parsed.phases,
        requirements: parsed.requirements,
        documents: vec![],
        next_recommended_skill: next.to_string(),
    })
}

/// Initialize a new GSD milestone cleanly under `.planning/` and persist to `VoxDb`.
pub async fn init_new_milestone(
    workspace_root: &Path,
    db: Option<&VoxDb>,
    milestone_id: &str,
    title: &str,
    start_phase: u32,
    phase_count: u32,
) -> Result<GsdProgressSnapshot, OrchestratorError> {
    let pdir = workspace_root.join(".planning");
    std::fs::create_dir_all(pdir.join("phases")).map_err(io_err)?;
    let count = phase_count.max(1);
    let end_phase = start_phase + count - 1;
    let now = now_unix_ms();

    let project_md = format!(
        "# Project\n\n## Active Milestone\n\n**{milestone_id}**: {title} (`Phases {start_phase}–{end_phase}`)\n"
    );
    let mut roadmap_md = format!("# Roadmap: {milestone_id} — {title}\n\n");
    let mut reqs_md = format!("# Requirements: {milestone_id} — {title}\n\n");
    for idx in 0..count {
        let p = start_phase + idx;
        roadmap_md.push_str(&format!(
            "- [ ] **Phase {p}: Deliverable Slice {}** (`GSD-{p:02}`)\n",
            idx + 1
        ));
        reqs_md.push_str(&format!(
            "### Phase {p}\n\n- [ ] **GSD-{p:02}**: Complete and verify milestone slice {}.\n\n",
            idx + 1
        ));
    }
    let state_md = format!(
        "---\nmilestone: \"{milestone_id}\"\nmilestone_name: \"{title}\"\n---\n\n# State\n\nActive milestone `{milestone_id}`, phase `{start_phase}` ready.\n"
    );
    let state_json = render_state_json(
        milestone_id,
        title,
        start_phase,
        "ready",
        &format!("/gsd-plan-phase {start_phase}"),
    );

    if !pdir.join("PROJECT.md").is_file() {
        std::fs::write(pdir.join("PROJECT.md"), project_md).map_err(io_err)?;
    }
    std::fs::write(pdir.join("ROADMAP.md"), roadmap_md).map_err(io_err)?;
    std::fs::write(pdir.join("REQUIREMENTS.md"), reqs_md).map_err(io_err)?;
    std::fs::write(pdir.join("STATE.md"), state_md).map_err(io_err)?;
    std::fs::write(pdir.join("state.json"), state_json).map_err(io_err)?;

    let milestone = GsdMilestone {
        milestone_id: milestone_id.to_string(),
        title: title.to_string(),
        status: GsdMilestoneStatus::Active,
        start_phase,
        end_phase,
        created_at_ms: now,
        updated_at_ms: now,
        completed_at_ms: None,
    };
    if let Some(database) = db {
        database
            .upsert_gsd_milestone(&milestone)
            .await
            .map_err(db_err)?;
    }
    load_workspace_status(workspace_root, db).await
}

/// Record discussion context (`CONTEXT.md`) for `phase_number`.
pub async fn record_discuss_phase(
    workspace_root: &Path,
    db: Option<&VoxDb>,
    milestone_id: &str,
    phase_number: u32,
    summary: &str,
) -> Result<GsdDocument, OrchestratorError> {
    let pdir = phase_dir(workspace_root, phase_number);
    std::fs::create_dir_all(&pdir).map_err(io_err)?;
    let path = pdir.join("CONTEXT.md");
    let body = format!(
        "# Phase {phase_number}: Context & Design Decisions\n\n## Locked Decisions\n\n{summary}\n"
    );
    std::fs::write(&path, &body).map_err(io_err)?;
    let rel = path
        .strip_prefix(workspace_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    let now = now_unix_ms();
    if let Some(database) = db {
        let doc = database
            .store_gsd_document(
                &format!("{milestone_id}:{rel}"),
                milestone_id,
                Some(phase_number),
                GsdDocumentKind::Context,
                &rel,
                &body,
                now,
            )
            .await
            .map_err(db_err)?;
        database
            .upsert_gsd_phase(&GsdPhase {
                milestone_id: milestone_id.to_string(),
                phase_number,
                title: format!("Phase {phase_number}"),
                status: GsdPhaseStatus::Discussing,
                wave_count: 1,
                plan_session_id: None,
                updated_at_ms: now,
            })
            .await
            .map_err(db_err)?;
        return Ok(doc);
    }
    Ok(GsdDocument {
        doc_id: format!("{milestone_id}:{rel}"),
        milestone_id: milestone_id.to_string(),
        phase_number: Some(phase_number),
        kind: GsdDocumentKind::Context,
        rel_path: rel,
        cas_hash: String::new(),
        byte_len: u64::try_from(body.len()).unwrap_or(0),
        updated_at_ms: now,
    })
}

/// Generate or load `PLAN.md` and return validated orchestrator `PlanNode` DAG tasks.
pub async fn generate_or_load_phase_plan(
    workspace_root: &Path,
    db: Option<&VoxDb>,
    milestone_id: &str,
    phase_number: u32,
    mvp: bool,
) -> Result<Vec<PlanNode>, OrchestratorError> {
    let pdir = phase_dir(workspace_root, phase_number);
    std::fs::create_dir_all(&pdir).map_err(io_err)?;
    let plan_path = pdir.join(format!("{phase_number:03}-01-PLAN.md"));
    let content = if plan_path.is_file() {
        std::fs::read_to_string(&plan_path).map_err(io_err)?
    } else {
        let waves = if mvp {
            vec![GsdPlanWave {
                wave_number: 1,
                tasks: vec![GsdWaveTask {
                    task_id: "mvp-slice".to_string(),
                    title: format!("Deliver MVP slice for phase {phase_number}"),
                    files: vec![format!(
                        ".planning/phases/{phase_number:03}/VERIFICATION.md"
                    )],
                    verification_cmd: "cargo check".to_string(),
                }],
            }]
        } else {
            vec![
                GsdPlanWave {
                    wave_number: 1,
                    tasks: vec![GsdWaveTask {
                        task_id: "impl".to_string(),
                        title: format!("Implement core deliverable for phase {phase_number}"),
                        files: vec!["Cargo.toml".to_string()],
                        verification_cmd: "cargo check".to_string(),
                    }],
                },
                GsdPlanWave {
                    wave_number: 2,
                    tasks: vec![GsdWaveTask {
                        task_id: "verify".to_string(),
                        title: format!("Verify phase {phase_number} acceptance checks"),
                        files: vec![format!(
                            ".planning/phases/{phase_number:03}/VERIFICATION.md"
                        )],
                        verification_cmd: format!("vox gsd verify-work {phase_number}"),
                    }],
                },
            ]
        };
        let rendered = render_phase_plan_md(phase_number, &format!("Phase {phase_number}"), &waves);
        std::fs::write(&plan_path, &rendered).map_err(io_err)?;
        rendered
    };

    let waves = parse_phase_plan_waves(&content);
    let nodes = waves_to_plan_nodes(phase_number, &waves)?;
    let now = now_unix_ms();
    if let Some(database) = db {
        let rel = plan_path
            .strip_prefix(workspace_root)
            .unwrap_or(&plan_path)
            .to_string_lossy()
            .replace('\\', "/");
        database
            .store_gsd_document(
                &format!("{milestone_id}:{rel}"),
                milestone_id,
                Some(phase_number),
                GsdDocumentKind::Plan,
                &rel,
                &content,
                now,
            )
            .await
            .map_err(db_err)?;
        database
            .upsert_gsd_phase(&GsdPhase {
                milestone_id: milestone_id.to_string(),
                phase_number,
                title: format!("Phase {phase_number}"),
                status: GsdPhaseStatus::Planned,
                wave_count: u32::try_from(waves.len()).unwrap_or(1),
                plan_session_id: Some(format!("gsd-{milestone_id}-p{phase_number}")),
                updated_at_ms: now,
            })
            .await
            .map_err(db_err)?;
    }
    Ok(nodes)
}

/// Mark a phase executing and return its wave DAG nodes.
pub async fn prepare_execute_phase(
    workspace_root: &Path,
    db: Option<&VoxDb>,
    milestone_id: &str,
    phase_number: u32,
) -> Result<Vec<PlanNode>, OrchestratorError> {
    let nodes =
        generate_or_load_phase_plan(workspace_root, db, milestone_id, phase_number, false).await?;
    let now = now_unix_ms();
    if let Some(database) = db {
        database
            .upsert_gsd_phase(&GsdPhase {
                milestone_id: milestone_id.to_string(),
                phase_number,
                title: format!("Phase {phase_number}"),
                status: GsdPhaseStatus::Executing,
                wave_count: u32::try_from(
                    nodes.iter().map(|n| n.depends_on.len()).max().unwrap_or(0) + 1,
                )
                .unwrap_or(1),
                plan_session_id: Some(format!("gsd-{milestone_id}-p{phase_number}")),
                updated_at_ms: now,
            })
            .await
            .map_err(db_err)?;
    }
    Ok(nodes)
}

/// Record verification status in `VERIFICATION.md` and update `VoxDb` phase state.
pub async fn record_verify_work(
    workspace_root: &Path,
    db: Option<&VoxDb>,
    milestone_id: &str,
    phase_number: u32,
    passed: bool,
    evidence: &str,
) -> Result<GsdDocument, OrchestratorError> {
    let pdir = phase_dir(workspace_root, phase_number);
    std::fs::create_dir_all(&pdir).map_err(io_err)?;
    let vpath = pdir.join("VERIFICATION.md");
    let verdict = if passed { "passed" } else { "failed" };
    let body = format!(
        "# Phase {phase_number}: Verification\n\n- **Verdict:** {verdict}\n- **Evidence:** {evidence}\n"
    );
    std::fs::write(&vpath, &body).map_err(io_err)?;
    let rel = vpath
        .strip_prefix(workspace_root)
        .unwrap_or(&vpath)
        .to_string_lossy()
        .replace('\\', "/");
    let now = now_unix_ms();
    if let Some(database) = db {
        let doc = database
            .store_gsd_document(
                &format!("{milestone_id}:{rel}"),
                milestone_id,
                Some(phase_number),
                GsdDocumentKind::Verification,
                &rel,
                &body,
                now,
            )
            .await
            .map_err(db_err)?;
        database
            .upsert_gsd_phase(&GsdPhase {
                milestone_id: milestone_id.to_string(),
                phase_number,
                title: format!("Phase {phase_number}"),
                status: if passed {
                    GsdPhaseStatus::Completed
                } else {
                    GsdPhaseStatus::Verifying
                },
                wave_count: 1,
                plan_session_id: None,
                updated_at_ms: now,
            })
            .await
            .map_err(db_err)?;
        return Ok(doc);
    }
    Ok(GsdDocument {
        doc_id: format!("{milestone_id}:{rel}"),
        milestone_id: milestone_id.to_string(),
        phase_number: Some(phase_number),
        kind: GsdDocumentKind::Verification,
        rel_path: rel,
        cas_hash: String::new(),
        byte_len: u64::try_from(body.len()).unwrap_or(0),
        updated_at_ms: now,
    })
}

/// Complete a GSD milestone, archive summary under `.planning/milestones/`, and persist to `VoxDb`.
pub async fn complete_milestone(
    workspace_root: &Path,
    db: Option<&VoxDb>,
    milestone_id: &str,
) -> Result<GsdProgressSnapshot, OrchestratorError> {
    let now = now_unix_ms();
    let archive_dir = workspace_root.join(".planning").join("milestones");
    std::fs::create_dir_all(&archive_dir).map_err(io_err)?;
    std::fs::write(
        archive_dir.join(format!("{milestone_id}-summary.md")),
        format!("# Milestone {milestone_id} Archive\n\nCompleted at unix_ms={now}.\n"),
    )
    .map_err(io_err)?;

    if let Some(database) = db {
        let existing = database
            .get_gsd_milestone(milestone_id)
            .await
            .map_err(db_err)?;
        let updated = if let Some(mut ms) = existing {
            ms.status = GsdMilestoneStatus::Completed;
            ms.updated_at_ms = now;
            ms.completed_at_ms = Some(now);
            ms
        } else {
            GsdMilestone {
                milestone_id: milestone_id.to_string(),
                title: format!("Milestone {milestone_id}"),
                status: GsdMilestoneStatus::Completed,
                start_phase: 1,
                end_phase: 1,
                created_at_ms: now,
                updated_at_ms: now,
                completed_at_ms: Some(now),
            }
        };
        database
            .upsert_gsd_milestone(&updated)
            .await
            .map_err(db_err)?;
        return database
            .gsd_progress_snapshot(Some(milestone_id))
            .await
            .map_err(db_err);
    }
    load_workspace_status(workspace_root, None).await
}
