//! Step bodies for [`super::DAEMON_STEPS`].

use std::sync::{Arc, Mutex};

use vox_orchestrator::{
    SessionConfig, SessionManager, a2a, clarification_db_inbox_poll, mesh_federation_poll, runtime,
};
use vox_orchestrator_mcp::profiles::Subsystem;
use vox_orchestrator_mcp::server_state::ServerState;

use super::DaemonBootCtx;
use crate::boot_plan::StepFuture;

pub(super) fn step_db(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let db = vox_db::connect_workspace_journey_optional_at(
            vox_db::DbConnectSurface::Runtime,
            ctx.start_dir.as_deref(),
            false,
        )
        .await
        .ok_or_else(|| "workspace journey store unavailable".to_string())?;
        let db = Arc::new(db);
        if let Err(e) = ctx.orch.init_db(db.clone()).await {
            tracing::warn!(error = %e, "orchestrator init_db failed; continuing without persisted Codex");
            return Err(format!("orchestrator init_db failed: {e}"));
        }
        tracing::info!("Codex attached and orchestrator schema synced");
        ctx.db = Some(db);
        Ok(())
    })
}

pub(super) fn step_model_registry(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let handle = ctx.orch.models_handle();
        let n = vox_orchestrator::sync_lock::rw_read(&*handle)
            .list_models()
            .len();
        tracing::info!(models = n, "model registry ready");
        Ok(())
    })
}

pub(super) fn step_routing_preferences(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        // Apply any GUI-persisted model-emphasis (routing priority) as this daemon's
        // global scorer default. Per-call SelectionAxes still override it via the
        // scorer's thread-local. Done before serving so all scoring sees it.
        let Some(db) = ctx.db.as_ref() else {
            return Ok(());
        };
        if let Ok(Some(csv)) = db
            .get_user_preference("local_user", "routing_priority")
            .await
        {
            let csv = csv.trim();
            if !csv.is_empty() {
                // Install as a thread-safe process-global rather than mutating the process
                // environment (set_var under the multi-threaded runtime is UB). See
                // `install_base_routing_priority`.
                match vox_config::AutoRoutingPriority::try_parse_csv(csv) {
                    Some(axes) => {
                        vox_orchestrator::models::install_base_routing_priority(Some(axes));
                        tracing::info!(priority = %csv, "applied persisted routing-priority emphasis");
                    }
                    None => tracing::warn!(
                        priority = %csv,
                        "routing_priority preference parsed no axes; leaving scorer default unchanged"
                    ),
                }
            }
        }
        // Apply any persisted ordered selection-policy chain; empty / absent leaves the
        // pre-existing selection cascade unchanged.
        if let Ok(Some(json)) = db
            .get_user_preference("local_user", "selection_policy")
            .await
        {
            let json = json.trim();
            if !json.is_empty() {
                match vox_orchestrator::models::SelectionPolicy::from_json(json) {
                    Ok(policy) => {
                        let n = policy.steps.len();
                        vox_orchestrator::models::install_active_policy(Some(policy));
                        tracing::info!(steps = n, "applied persisted selection-policy chain");
                    }
                    Err(e) => tracing::warn!(
                        error = %e,
                        "selection_policy preference is not valid JSON; ignoring"
                    ),
                }
            }
        }
        Ok(())
    })
}

/// Subsystems initialised lazily per call; booting them is declaring them available.
pub(super) fn step_lazy(_ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async { Ok(()) })
}

pub(super) fn step_sessions(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let cfg = SessionConfig {
            repository_id: Some(ctx.repository_id.clone()),
            sessions_dir: ctx
                .repository
                .root
                .join(vox_config::mcp_sessions_dir(&ctx.repository_id)),
            ..SessionConfig::default()
        };
        let sm = SessionManager::new(cfg)
            .map_err(|e| format!("session manager initialization failed: {e}"))?;
        ctx.session_manager = Some(sm);
        Ok(())
    })
}

pub(super) fn step_skills(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        // Skills are discovered from plugins; bridge plugin-host skills into vox-skills.
        let registry = vox_skills::new_registry_arc();
        vox_orchestrator_mcp::plugin_skills_bridge::install_discovered_skills(
            &registry,
            &ctx.plugins_dir,
        )
        .await;
        ctx.skill_registry = Some(registry);
        Ok(())
    })
}

pub(super) fn step_server_state(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        // `sessions` outside the closure (or failed under `dev`): an in-memory manager keeps
        // ServerState total; the gate denies `vox_session_*` via `requires: [sessions]`.
        let session_manager = match ctx.session_manager.take() {
            Some(sm) => sm,
            None => SessionManager::new(SessionConfig {
                repository_id: Some(ctx.repository_id.clone()),
                persist: false,
                ..SessionConfig::default()
            })
            .map_err(|e| format!("in-memory session manager: {e}"))?,
        };
        let registry = ctx
            .skill_registry
            .clone()
            .unwrap_or_else(vox_skills::new_registry_arc);
        let mut state = ServerState::new_for_daemon_gated(
            ctx.orch.clone(),
            ctx.orch_config.clone(),
            ctx.repository.clone(),
            Arc::new(tokio::sync::Mutex::new(session_manager)),
            registry,
            ctx.profile.subsystems.contains(&Subsystem::Research),
        );
        state.active_profile = Arc::new(ctx.profile.clone());
        if let Some(db) = ctx.db.clone() {
            state = state.with_db_initialized(db).await;
        }
        ctx.state = Some(state);
        Ok(())
    })
}

pub(super) fn step_agent_fleet(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let state = ctx
            .state
            .as_ref()
            .ok_or_else(|| "server state not built".to_string())?;
        // Wire the AgentFleet's AiTaskProcessor with a real ToolDispatcher backed by this
        // same ServerState, so autonomous `@tool` intents go through the MCP approval gate.
        runtime::spawn_agent_fleet_if_enabled_with_dispatcher(
            ctx.orch.clone(),
            Some(
                vox_orchestrator_mcp::autonomous_tool_dispatch::McpToolDispatcher::new_arc(
                    state.clone(),
                ),
            ),
        );
        Ok(())
    })
}

pub(super) fn step_populi_federation(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        mesh_federation_poll::spawn_populi_federation_poller(
            &ctx.orch_config,
            ctx.repository_id.clone(),
            ctx.db.clone(),
            ctx.orch.clone(),
            Arc::clone(&ctx.populi_remote_snapshot),
            Arc::clone(&ctx.populi_poll_join),
        );
        Ok(())
    })
}

pub(super) fn step_a2a_result(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        a2a::spawn_populi_remote_result_poller(ctx.orch.clone(), Arc::new(Mutex::new(None)));
        Ok(())
    })
}

pub(super) fn step_a2a_worker(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        a2a::spawn_populi_remote_worker_poller(ctx.orch.clone(), Arc::new(Mutex::new(None)));
        Ok(())
    })
}

pub(super) fn step_clarification(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let db = ctx
            .db
            .clone()
            .ok_or_else(|| "workspace journey store not attached".to_string())?;
        clarification_db_inbox_poll::spawn_clarification_db_inbox_poller(
            db,
            ctx.repository_id.clone(),
            Arc::new(Mutex::new(None)),
        );
        Ok(())
    })
}

pub(super) fn step_socrates(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        vox_orchestrator::socrates::spawn_socrates_research_poller(ctx.orch.clone());
        Ok(())
    })
}

pub(super) fn step_flywheel(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        vox_orchestrator::services::flywheel::FlywheelMonitor::new(ctx.orch.clone())
            .spawn()
            .await;
        Ok(())
    })
}

pub(super) fn step_attention(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        vox_orchestrator::services::attention_calibration::spawn_attention_calibration(
            ctx.orch.clone(),
        );
        Ok(())
    })
}
