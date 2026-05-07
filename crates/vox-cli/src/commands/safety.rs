use clap::Subcommand;
use miette::Result;
use owo_colors::OwoColorize;
use vox_orchestrator::{
    AgentId, Orchestrator, OrchestratorConfig, build_repo_scoped_orchestrator_for_repository,
};
use vox_repository::discover_repository_or_fallback;

/// Manage and inspect the Vox safety and coherence systems.
#[derive(Debug, Subcommand)]
pub enum SafetyCommand {
    /// Show current safety, drift, and budget status for all agents.
    Status,
    /// Inspect the cryptographic tool receipt ledger.
    Ledger {
        /// Optional: filter by agent.
        #[arg(long)]
        agent_id: Option<u64>,
    },
    /// Inspect active generic resource locks.
    Locks,
}

pub async fn handle_safety_command(
    cmd: SafetyCommand,
    workspace_root: &std::path::Path,
) -> Result<()> {
    match cmd {
        SafetyCommand::Status => status_cmd(workspace_root).await,
        SafetyCommand::Ledger { agent_id } => ledger_cmd(agent_id, workspace_root).await,
        SafetyCommand::Locks => locks_cmd(workspace_root).await,
    }
}

async fn status_cmd(workspace_root: &std::path::Path) -> Result<()> {
    let repo = discover_repository_or_fallback(workspace_root);
    let config =
        OrchestratorConfig::load_from_toml(&repo.root).map_err(|e| miette::miette!("{}", e))?;

    let build = build_repo_scoped_orchestrator_for_repository(config, &repo);
    let orch = build.orchestrator;

    println!("{}", "╔══════════════════════════════════════╗".cyan());
    println!("{}", "║   Vox Safety & Coherence Status      ║".cyan());
    println!("{}", "╚══════════════════════════════════════╝".cyan());
    println!();

    let budget_manager = orch.budget_manager_handle();
    let bm = vox_orchestrator::sync_lock::rw_read(&*budget_manager);

    println!("{}", "Agent Budgets & Drift:".bold().underline());
    let statuses = orch.status();
    for agent_status in statuses.agents {
        let signal = bm.agent_budget_signal(agent_status.id);
        let signal_str = match signal {
            vox_orchestrator::budget::BudgetSignal::Normal { usage_ratio } => {
                format!("Normal ({:.1}%)", usage_ratio * 100.0)
            }
            vox_orchestrator::budget::BudgetSignal::HighLoad { usage_ratio, .. } => {
                format!("High Load ({:.1}%)", usage_ratio * 100.0)
            }
            vox_orchestrator::budget::BudgetSignal::Critical { .. } => "CRITICAL".to_string(),
            vox_orchestrator::budget::BudgetSignal::CostExceeded { .. } => {
                "COST EXCEEDED".to_string()
            }
            vox_orchestrator::budget::BudgetSignal::HaltAgent { reason } => {
                format!("HALTED: {}", reason)
            }
            vox_orchestrator::budget::BudgetSignal::DoomLoopSuspect { consecutive_calls } => {
                format!("DOOM LOOP SUSPECT ({} calls)", consecutive_calls)
            }
            _ => "Unknown".to_string(),
        };

        println!(
            "  Agent {} ({}): {}",
            agent_status.id.to_string().bold(),
            agent_status.name,
            signal_str
        );
    }

    println!();
    println!("{}", "Active Locks:".bold().underline());
    println!(
        "  Tool Receipts:  {}",
        orch.tool_ledger_handle().read().unwrap().len()
    );

    let locks = orch.resource_locks();
    println!("  Resource Locks: {}", locks.len());

    Ok(())
}

async fn ledger_cmd(agent_id_opt: Option<u64>, workspace_root: &std::path::Path) -> Result<()> {
    let repo = discover_repository_or_fallback(workspace_root);
    let config =
        OrchestratorConfig::load_from_toml(&repo.root).map_err(|e| miette::miette!("{}", e))?;

    let build = build_repo_scoped_orchestrator_for_repository(config, &repo);
    let ledger_handle = build.orchestrator.tool_ledger_handle();
    let ledger = ledger_handle.read().unwrap();

    println!("{}", "Tool Receipt Ledger".bold().underline());
    let snapshot = ledger.snapshot();
    if snapshot.is_empty() {
        println!("  (No receipts issued in this session)");
    } else {
        for (id, (aid, tool)) in snapshot.iter() {
            if let Some(target) = agent_id_opt {
                if aid.0 != target {
                    continue;
                }
            }
            println!("  [{}] Agent {} -> {}", id.dimmed(), aid, tool.cyan());
        }
    }
    Ok(())
}

async fn locks_cmd(workspace_root: &std::path::Path) -> Result<()> {
    let repo = discover_repository_or_fallback(workspace_root);
    let config =
        OrchestratorConfig::load_from_toml(&repo.root).map_err(|e| miette::miette!("{}", e))?;

    let build = build_repo_scoped_orchestrator_for_repository(config, &repo);
    let locks = build.orchestrator.resource_locks();

    println!("{}", "Active Resource Locks".bold().underline());
    let snapshot = locks.snapshot();
    if snapshot.is_empty() {
        println!("  (No active resource locks)");
    } else {
        for lock in snapshot {
            println!(
                "  {:30} held by Agent {}",
                lock.resource_id.cyan(),
                lock.holder
            );
        }
    }
    Ok(())
}
