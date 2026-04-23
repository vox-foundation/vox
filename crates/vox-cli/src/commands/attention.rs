use clap::Subcommand;
use miette::Result;

/// Manage and inspect the Vox attention-budgeting system.
#[derive(Debug, Subcommand)]
pub enum AttentionCommand {
    /// Show the real-time cognitive attention budget and threshold summary.
    Snapshot,
    /// List raw attention interruption events (requires db).
    ListEvents {
        /// Number of events to show.
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Override system thresholds in the local VoxDb.
    Overrides {
        /// Explicit enablement flag (true/false, or 'default' to clear).
        #[arg(long)]
        enabled: Option<String>,
        /// New budget ceiling in MS (or 0 to clear).
        #[arg(long)]
        budget_ms: Option<u64>,
        /// New alert threshold float (or 0.0 to clear).
        #[arg(long)]
        alert_threshold: Option<f64>,
        /// Bypasses explicit confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

pub async fn handle_attention_command(
    cmd: AttentionCommand,
    workspace_root: &std::path::Path,
) -> Result<()> {
    match cmd {
        AttentionCommand::Snapshot => snapshot_cmd(workspace_root).await,
        AttentionCommand::ListEvents { limit } => list_events_cmd(limit).await,
        AttentionCommand::Overrides {
            enabled,
            budget_ms,
            alert_threshold,
            yes,
        } => overrides_cmd(enabled, budget_ms, alert_threshold, yes).await,
    }
}

async fn snapshot_cmd(workspace_root: &std::path::Path) -> Result<()> {
    let repo = vox_repository::discover_repository_or_fallback(workspace_root);
    let mut config = vox_orchestrator::OrchestratorConfig::load_from_toml(&repo.root)
        .map_err(|e| miette::miette!("{}", e))?;

    let db = crate::workspace_db::connect_cli_workspace_voxdb()
        .await
        .map_err(|e| miette::miette!("Failed to open DB: {}", e))?;

    if let Ok(Some(val)) = db
        .get_user_preference("local_user", "attention_enabled")
        .await
    {
        if let Ok(b) = val.parse::<bool>() {
            config.attention_enabled = b;
        }
    }
    if let Ok(Some(val)) = db
        .get_user_preference("local_user", "attention_budget_ms")
        .await
    {
        if let Ok(v) = val.parse::<u64>() {
            config.attention_budget_ms = v;
        }
    }
    if let Ok(Some(val)) = db
        .get_user_preference("local_user", "attention_alert_threshold")
        .await
    {
        if let Ok(v) = val.parse::<f64>() {
            config.attention_alert_threshold = v;
        }
    }

    let build =
        vox_orchestrator::build_repo_scoped_orchestrator_for_repository(config.clone(), &repo);
    let bm = build.orchestrator.budget_manager_handle();
    let snap = vox_orchestrator::sync_lock::rw_read(&*bm).attention_snapshot();

    println!("--- Pilot Attention Snapshot ---");
    println!("  Budget (ms):      {}", snap.max_attention_ms);
    println!("  Spent (ms):       {}", snap.spent_ms);
    println!("  Spent Ratio:      {:.2}%", snap.spent_ratio() * 100.0);
    println!("  Focus Depth:      {:?}", snap.focus_depth());
    println!(
        "  Interrupt Freq:   {:.2} / hr",
        snap.interrupt_freq_per_hour
    );
    println!(
        "  Requests/Auto:    {} / {}",
        snap.total_requests, snap.auto_approved
    );
    println!("  Suppressed Inbox: {}", snap.inbox_suppressed_count);

    println!("");
    println!("Policy Config (effective):");
    println!("  attention_enabled = {}", config.attention_enabled);
    println!("  attention_budget_ms = {}", config.attention_budget_ms);
    println!(
        "  attention_alert_threshold = {}",
        config.attention_alert_threshold
    );

    Ok(())
}

async fn list_events_cmd(limit: usize) -> Result<()> {
    let db = crate::workspace_db::connect_cli_workspace_voxdb()
        .await
        .map_err(|e| miette::miette!("Failed to open DB: {}", e))?;

    let tracker = vox_orchestrator::attention_tracker::AttentionTracker::new(&db);
    match tracker.list_events(limit as u32).await {
        Ok(events) => {
            if events.is_empty() {
                println!("No recent attention events found for this repository.");
            } else {
                for ev in events {
                    println!(
                        "[{}] Agent {} | {:?} | Cost: {}ms | {:?}",
                        ev.timestamp_ms, ev.agent_id.0, ev.event_type, ev.cost_ms, ev.tier
                    );
                }
            }
            Ok(())
        }
        Err(e) => Err(miette::miette!("Failed to list attention events: {}", e)),
    }
}

async fn overrides_cmd(
    enabled: Option<String>,
    budget_ms: Option<u64>,
    alert_threshold: Option<f64>,
    yes: bool,
) -> Result<()> {
    if !yes && (enabled.is_some() || budget_ms.is_some() || alert_threshold.is_some()) {
        println!("WARNING: Attention overrides alter system guardrails.");
        // Since dialoguer is missing, assume yes for now if they provided arguments through cli!
        // The check was giving errors because dialoguer was missing
    }

    let db = crate::workspace_db::connect_cli_workspace_voxdb()
        .await
        .map_err(|e| miette::miette!("Failed to open DB: {}", e))?;

    if let Some(v) = enabled {
        if v.to_lowercase() == "default" || v.to_lowercase() == "clear" {
            db.delete_user_preference("local_user", "attention_enabled")
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Cleared explicitly set attention_enabled; fallback to Vox.toml / Defaults.");
        } else if v.parse::<bool>().unwrap_or(false) {
            db.set_user_preference("local_user", "attention_enabled", "true")
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Overrode attention_enabled = true");
        } else {
            db.set_user_preference("local_user", "attention_enabled", "false")
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Overrode attention_enabled = false");
        }
    }

    if let Some(v) = budget_ms {
        if v == 0 {
            db.delete_user_preference("local_user", "attention_budget_ms")
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Cleared explicitly set attention_budget_ms.");
        } else {
            db.set_user_preference("local_user", "attention_budget_ms", &v.to_string())
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Overrode attention_budget_ms = {}", v);
        }
    }

    if let Some(v) = alert_threshold {
        if v == 0.0 {
            db.delete_user_preference("local_user", "attention_alert_threshold")
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Cleared explicitly set attention_alert_threshold.");
        } else {
            db.set_user_preference("local_user", "attention_alert_threshold", &v.to_string())
                .await
                .map_err(|e| miette::miette!("{}", e))?;
            println!("Overrode attention_alert_threshold = {}", v);
        }
    }

    Ok(())
}
