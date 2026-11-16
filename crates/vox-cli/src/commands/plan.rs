use clap::Subcommand;

#[derive(Subcommand)]
pub enum PlanCmd {
    /// Generate a new V2 plan via the PlanningOrchestrator.
    Create {
        /// Human readable goal the agent should plan out
        goal: String,

        /// Auto-approve execution bypassing the `RequiresApproval` block.
        #[arg(long)]
        approve: bool,
    },
    /// Re-evaluate an existing PlanId to unblock exceptions.
    Replan {
        /// Active Plan Session ID
        session_id: String,
    },
    /// Show current GSD milestone/phase status or a specific Plan Session ID.
    Status {
        /// Optional Plan Session ID to scope output
        session_id: Option<String>,
        /// Emit machine-readable JSON output
        #[arg(long)]
        json: bool,
    },
    /// Diagnose `.planning/` health, required GSD files, and VoxDb CAS readiness.
    Health {
        #[arg(long)]
        json: bool,
    },
    /// Show detailed GSD milestone progress snapshot and next recommended skill.
    Progress {
        #[arg(long)]
        json: bool,
    },
    /// Initialize a new GSD milestone (`PROJECT.md`, `ROADMAP.md`, `REQUIREMENTS.md`, `STATE.md`).
    NewMilestone {
        version: String,
        title: String,
        #[arg(long, default_value_t = 1)]
        start_phase: u32,
        #[arg(long, default_value_t = 3)]
        phases: u32,
    },
    /// Record discussion context (`CONTEXT.md`) before planning a phase.
    DiscussPhase {
        phase: u32,
        #[arg(
            long,
            default_value = "Locked implementation boundaries and acceptance checks."
        )]
        summary: String,
    },
    /// Generate or load a wave-ordered `PLAN.md` and validate `PlanNode` DAG tasks.
    PlanPhase {
        phase: u32,
        #[arg(long)]
        mvp: bool,
    },
    /// Prepare a phase for execution and display its wave-ordered DAG steps.
    ExecutePhase { phase: u32 },
    /// Record verification evidence (`VERIFICATION.md`) for a phase.
    VerifyWork {
        phase: u32,
        #[arg(long)]
        passed: bool,
        #[arg(long, default_value = "Verification checks recorded via CLI")]
        evidence: String,
    },
    /// Archive and complete a verified GSD milestone.
    CompleteMilestone { version: String },
    /// List GSD documents tracked in `.planning/` and VoxDb Arca CAS.
    Docs {
        #[arg(long)]
        phase: Option<u32>,
        #[arg(long)]
        json: bool,
    },
}

pub async fn dispatch(cmd: PlanCmd) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let db = vox_db::VoxDb::connect_default().await.ok();
    let db_ref = db.as_ref();

    match cmd {
        PlanCmd::Create { goal, approve } => {
            tracing::info!("Synthesizing plan for goal: {}", goal);
            let nodes = vox_orchestrator::planning::synthesizer::synthesize_plan_nodes(&goal);
            let json = serde_json::to_string_pretty(&nodes)?;
            println!("Plan synthesized:\n{}", json);
            if approve {
                tracing::info!("Auto-approve enabled.");
            }
        }
        PlanCmd::Replan { session_id } => {
            tracing::info!("Re-planning session: {}", session_id);
            let config = vox_orchestrator::session::SessionConfig {
                persist: true,
                ..Default::default()
            };
            let mut manager = vox_orchestrator::session::SessionManager::new(config)?;
            if let Ok(conn) = vox_db::VoxDb::connect_default().await {
                manager.set_db(std::sync::Arc::new(conn));
            }
            if let Err(e) = manager.load(&session_id).await {
                anyhow::bail!("Failed to load session {}: {}", session_id, e);
            }
            if let Some(session) = manager.get(&session_id) {
                println!(
                    "Session {} loaded from VoxDb (turns: {}, tokens: {}).",
                    session_id, session.turn_count, session.total_tokens
                );
            }
        }
        PlanCmd::Status { session_id, json } => {
            if let Some(sid) = session_id {
                tracing::info!("Status requested for session: {}", sid);
                if json {
                    println!(
                        "{}",
                        serde_json::json!({ "session_id": sid, "status": "ok" })
                    );
                } else {
                    println!("Plan session: {sid}");
                }
            } else {
                let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&snap)?);
                } else if let Some(ms) = &snap.active_milestone {
                    println!(
                        "GSD Milestone {} ({:?}): Phases {}-{} ({} tracked phases, {} reqs, {} docs) | Next: {}",
                        ms.milestone_id,
                        ms.status,
                        ms.start_phase,
                        ms.end_phase,
                        snap.phases.len(),
                        snap.requirements.len(),
                        snap.documents.len(),
                        snap.next_recommended_skill
                    );
                } else {
                    println!(
                        "No active GSD milestone. Next: {}",
                        snap.next_recommended_skill
                    );
                }
            }
        }
        PlanCmd::Health { json } => {
            let report = vox_orchestrator::planning::gsd::health_check(&cwd);
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "GSD health: {} (files={}, issues={})",
                    if report.healthy {
                        "healthy"
                    } else {
                        "needs_attention"
                    },
                    report.present_files.len(),
                    report.issues.len()
                );
                for issue in &report.issues {
                    println!("  - {issue}");
                }
            }
        }
        PlanCmd::Progress { json } => {
            let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if json {
                println!("{}", serde_json::to_string_pretty(&snap)?);
            } else {
                println!(
                    "Next recommended GSD skill: {}",
                    snap.next_recommended_skill
                );
                for p in &snap.phases {
                    println!("  Phase {}: {} [{:?}]", p.phase_number, p.title, p.status);
                }
            }
        }
        PlanCmd::NewMilestone {
            version,
            title,
            start_phase,
            phases,
        } => {
            let snap = vox_orchestrator::planning::gsd::init_new_milestone(
                &cwd,
                db_ref,
                &version,
                &title,
                start_phase,
                phases,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&snap)?);
        }
        PlanCmd::DiscussPhase { phase, summary } => {
            let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let ms_id = snap
                .active_milestone
                .map(|m| m.milestone_id)
                .unwrap_or_else(|| "v1.0".to_string());
            let doc = vox_orchestrator::planning::gsd::record_discuss_phase(
                &cwd, db_ref, &ms_id, phase, &summary,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        PlanCmd::PlanPhase { phase, mvp } => {
            let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let ms_id = snap
                .active_milestone
                .map(|m| m.milestone_id)
                .unwrap_or_else(|| "v1.0".to_string());
            let nodes = vox_orchestrator::planning::gsd::generate_or_load_phase_plan(
                &cwd, db_ref, &ms_id, phase, mvp,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&nodes)?);
        }
        PlanCmd::ExecutePhase { phase } => {
            let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let ms_id = snap
                .active_milestone
                .map(|m| m.milestone_id)
                .unwrap_or_else(|| "v1.0".to_string());
            let nodes =
                vox_orchestrator::planning::gsd::prepare_execute_phase(&cwd, db_ref, &ms_id, phase)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&nodes)?);
        }
        PlanCmd::VerifyWork {
            phase,
            passed,
            evidence,
        } => {
            let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let ms_id = snap
                .active_milestone
                .map(|m| m.milestone_id)
                .unwrap_or_else(|| "v1.0".to_string());
            let doc = vox_orchestrator::planning::gsd::record_verify_work(
                &cwd, db_ref, &ms_id, phase, passed, &evidence,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        PlanCmd::CompleteMilestone { version } => {
            let snap = vox_orchestrator::planning::gsd::complete_milestone(&cwd, db_ref, &version)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("{}", serde_json::to_string_pretty(&snap)?);
        }
        PlanCmd::Docs { phase, json } => {
            let snap = vox_orchestrator::planning::gsd::load_workspace_status(&cwd, db_ref)
                .await
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            let docs: Vec<_> = snap
                .documents
                .into_iter()
                .filter(|d| phase.is_none_or(|p| d.phase_number == Some(p)))
                .collect();
            if json {
                println!("{}", serde_json::to_string_pretty(&docs)?);
            } else {
                for d in &docs {
                    println!("{} [{:?}] {}", d.rel_path, d.kind, d.cas_hash);
                }
            }
        }
    }
    Ok(())
}
