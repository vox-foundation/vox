use crate::sync_poison::poison_rw_read;
use crate::{AgentInfo, ServerState, StatusResponse, ToolResult};

use vox_ludus::companion::Companion;
use vox_ludus::db::list_companions;

/// Get a full snapshot of the orchestrator's state.
pub async fn orchestrator_status(state: &ServerState) -> anyhow::Result<String> {
    let (
        status,
        scaling_profile,
        effective_scale_up_threshold,
        vcs_snapshot_count,
        vcs_oplog_count,
        vcs_active_conflicts,
        vcs_active_workspaces,
        vcs_active_changes,
        populi_control_url,
        populi_http_timeout_ms,
    ) = {
        let orch = &state.orchestrator;
        let handle = orch.config_handle();
        let cfg = poison_rw_read(
            handle.read(),
            "read orchestrator config for vox_orchestrator_status",
        )?;
        let effective = cfg.scaling_threshold as f64 * cfg.scaling_profile.threshold_multiplier();
        (
            orch.status(),
            Some(format!("{:?}", cfg.scaling_profile).to_lowercase()),
            Some(effective),
            poison_rw_read(
                orch.snapshot_store_handle().read(),
                "read VCS snapshot store for vox_orchestrator_status",
            )?
            .count(),
            poison_rw_read(
                orch.oplog_handle().read(),
                "read VCS oplog for vox_orchestrator_status",
            )?
            .count(),
            poison_rw_read(
                orch.conflict_manager_handle().read(),
                "read VCS conflict manager for vox_orchestrator_status",
            )?
            .active_count(),
            poison_rw_read(
                orch.workspace_manager_handle().read(),
                "read VCS workspace manager (list_workspaces) for vox_orchestrator_status",
            )?
            .list_workspaces()
            .len(),
            poison_rw_read(
                orch.workspace_manager_handle().read(),
                "read VCS workspace manager (list_changes) for vox_orchestrator_status",
            )?
            .list_changes(None, usize::MAX)
            .len(),
            cfg.populi_control_url.clone(),
            cfg.populi_http_timeout_ms,
        )
    };

    let populi_federation_cache = serde_json::to_value(
        poison_rw_read(
            state.populi_remote_snapshot.read(),
            "read Populi remote snapshot cache for vox_orchestrator_status",
        )?
        .clone(),
    )
    .ok();

    let max_stale_ms: Option<u64> = std::env::var("VOX_MESH_MAX_STALE_MS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|n| *n > 0);

    let mesh_snapshot = if let Some(url) = populi_control_url
        .as_ref()
        .filter(|s: &&String| !s.trim().is_empty())
    {
        let timeout = std::time::Duration::from_millis(populi_http_timeout_ms.max(500_u64));
        let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(url, timeout)
            .with_env_token();
        match client.list_nodes().await {
            Ok(f) => {
                let f = vox_populi::filter_registry_by_max_stale_ms(f, max_stale_ms);
                Some(serde_json::json!({
                    "ok": true,
                    "schema_version": f.schema_version,
                    "node_count": f.nodes.len(),
                    "nodes": f.nodes,
                }))
            }
            Err(e) => Some(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        }
    } else {
        None
    };

    if let Some(ref snap) = mesh_snapshot {
        persist_mesh_snapshot_codex_opt(state, snap).await;
    }

    let agents: Vec<AgentInfo> = status
        .agents
        .iter()
        .map(|a| AgentInfo {
            id: a.id.0,
            name: a.name.clone(),
            queued: a.queued,
            completed: a.completed,
            paused: a.paused,
        })
        .collect();

    let companion = {
        // Try to load from DB for persistence
        let id = "vox-dei";
        let mut comp = if let Some(db) = &state.db {
            match list_companions(db, "user").await {
                Ok(comps) => comps.into_iter().find(|c: &Companion| c.id == id),
                Err(_) => None,
            }
        } else {
            None
        }
        .unwrap_or_else(|| vox_ludus::companion::Companion::new(id, "user", "Vox DEI", "vox"));

        comp.ascii_sprite = Some("🧑‍💻".to_string());
        Some(comp)
    };

    let scaling_line = match (scaling_profile.as_deref(), effective_scale_up_threshold) {
        (Some(prof), Some(eff)) => format!(
            "**Scaling:** profile={}, effective scale-up threshold={:.1}\n\n",
            prof, eff
        ),
        _ => String::new(),
    };
    let mut markdown = format!(
        "### 🤖 Vox DEI Status\n\n**Agents Active:** {}\n**Tasks In Progress:** {}\n**Tasks Completed:** {}\n\n{}",
        status.agents.len(),
        status.agents.iter().map(|a| a.queued).sum::<usize>(),
        status.total_completed,
        scaling_line
    );

    if let Some(ref c) = companion {
        markdown.push_str("#### 🧬 Code Companion\n\n");
        markdown.push_str(&format!(
            "```\n{}\n```\n",
            c.ascii_sprite.as_deref().unwrap_or("")
        ));
        markdown.push_str(&format!("**{}** {}\n", c.name, c.mood.emoji()));
        markdown.push_str(&format!(
            "HP: `{}`\n",
            c.render_status_bar(15).split("HP: ").last().unwrap_or("")
        ));
    }
    let planning = if let Some(db) = &state.db {
        let mut active = 0_i64;
        let mut total = 0_i64;
        if let Ok(rows) = db
            .query_all(
                "SELECT
                    SUM(CASE WHEN status IN ('pending','queued','in_progress') THEN 1 ELSE 0 END) AS active,
                    COUNT(*) AS total
                 FROM plan_sessions",
                (),
            )
            .await
            && let Some(row) = rows.first()
        {
            active = row.get(0).unwrap_or(0);
            total = row.get(1).unwrap_or(0);
        }
        Some(serde_json::json!({
            "active_sessions": active,
            "total_sessions": total,
        }))
    } else {
        None
    };

    markdown.push_str("\n#### 📋 Agent Queue\n\n");
    for a in &agents {
        let status_icon = if a.paused {
            "⏸️"
        } else if a.queued > 0 {
            "⚙️"
        } else {
            "💤"
        };
        markdown.push_str(&format!(
            "- {} **{}** (Queued: {}, Done: {})\n",
            status_icon, a.name, a.queued, a.completed
        ));
    }

    let response = StatusResponse {
        agent_count: status.agents.len(),
        in_progress: status.agents.iter().map(|a| a.queued).sum(),
        completed: status.total_completed,
        agents,
        scaling_profile,
        effective_scale_up_threshold,
        companion,
        markdown_summary: Some(markdown),
        snapshot_count: vcs_snapshot_count,
        oplog_count: vcs_oplog_count,
        active_conflicts: vcs_active_conflicts,
        active_workspaces: vcs_active_workspaces,
        active_changes: vcs_active_changes,
        mesh_snapshot,
        populi_federation_cache,
        planning,
    };

    Ok(ToolResult::ok(response).to_json())
}

fn mesh_codex_telemetry_enabled() -> bool {
    std::env::var("VOX_MESH_CODEX_TELEMETRY")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

async fn persist_mesh_snapshot_codex_opt(state: &ServerState, snap: &serde_json::Value) {
    if !mesh_codex_telemetry_enabled() {
        return;
    }
    let Some(db) = state.db.as_ref() else {
        return;
    };
    let rid = state.repository.repository_id.clone();
    let details = serde_json::json!({
        "event": "orchestrator_status_mesh_snapshot",
        "ok": snap.get("ok"),
        "node_count": snap.get("node_count"),
        "schema_version": snap.get("schema_version"),
        "error": snap.get("error"),
    });
    if let Err(e) = db
        .record_populi_control_event(&rid, "orchestrator_status_mesh_snapshot", Some(details))
        .await
    {
        tracing::debug!(
            target: "vox.mesh_codex",
            error = %e,
            "record_populi_control_event failed (best-effort)"
        );
    }
}
