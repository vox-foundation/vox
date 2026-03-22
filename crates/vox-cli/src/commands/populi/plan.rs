//! `vox populi plan` — Generate, replan, and query Codex plans from the CLI.
//!
//! Planning surfaces: MCP exposes `vox_plan` (in-process LLM), `vox_replan`, and `vox_plan_status`
//! (both call `vox-dei-d` `ai.plan.replan` / `ai.plan.status`). This CLI uses the daemon for all
//! subcommands so terminal and DeI share one persistence layer.
//!
//! Default builds talk to the `vox-dei-d` JSON-RPC daemon (same contract as MCP plan tools).
//! Optional in-process Codex integration may return behind a `dashboard` feature later.

use anyhow::Result;
use clap::Parser;

/// `vox populi plan` — Subcommands for AI-assisted planning.
#[derive(Parser, serde::Serialize, serde::Deserialize)]
#[command(
    name = "plan",
    about = "AI-assisted planning: generate, replan, and query plan sessions",
    long_about = "Generate structured implementation plans via LLM, backed by Codex (SQLite).\n\
                  Plans are versioned and resumable.\n\
                  \n\
                  By default this CLI uses the `vox-dei-d` daemon; ensure it is installed and on PATH."
)]
pub enum PlanAction {
    /// Generate a new structured implementation plan for a goal.
    New {
        /// High-level goal or request to plan for.
        #[arg(required = true)]
        goal: String,
        /// Restrict analysis to these files (space-separated).
        #[arg(long, num_args = 1..)]
        scope_files: Vec<String>,
        /// Write the generated plan to PLAN.md in the workspace root.
        #[arg(long, default_value = "false")]
        write_to_disk: bool,
        /// Maximum number of tasks to include in the plan (default: 30).
        #[arg(long)]
        max_tasks: Option<usize>,
        /// Execution mode: efficient, fast, verbose, or precision. Affects planner token budgets.
        #[arg(long)]
        mode: Option<String>,
        /// Output the plan as JSON (default: Markdown).
        #[arg(long, default_value = "false")]
        json: bool,
    },

    /// Regenerate a plan for an existing session with a delta hint.
    Replan {
        /// Session ID from a previous `vox populi plan new` or `vox_plan` MCP call.
        #[arg(required = true)]
        session_id: String,
        /// Short description of what changed to drive the replan.
        #[arg(required = true)]
        delta_hint: String,
        /// Write the updated plan to PLAN.md.
        #[arg(long, default_value = "false")]
        write_to_disk: bool,
        /// Execution mode: efficient, fast, verbose, or precision.
        #[arg(long)]
        mode: Option<String>,
        /// Output as JSON.
        #[arg(long, default_value = "false")]
        json: bool,
    },

    /// Show the current status of a planning session.
    Status {
        /// Session ID to query.
        #[arg(required = true)]
        session_id: String,
        /// Output as machine-readable JSON.
        #[arg(long, default_value = "false")]
        json: bool,
    },
    /// Execute the steps of a structured interaction plan.
    Execute {
        /// Session ID to execute.
        #[arg(required = true)]
        session_id: String,
        /// Approve the plan even if the execution mode is set to 'requires_approval'.
        #[arg(long, default_value = "false")]
        approve: bool,
    },
}

/// Run a `vox populi plan` subcommand (via daemon).
pub async fn run(action: PlanAction) -> Result<()> {
    match action {
        PlanAction::New {
            goal,
            scope_files: _,
            write_to_disk,
            max_tasks,
            mode,
            json,
        } => {
            let resp = crate::dei_daemon::call(
                crate::dei_daemon::method::AI_PLAN_NEW,
                serde_json::json!({
                    "goal": goal,
                    "write_to_disk": write_to_disk,
                    "max_tasks": max_tasks,
                    "mode": mode,
                }),
                false,
            )
            .await?;
            print_plan_daemon_response(&resp, json, write_to_disk);
            Ok(())
        }
        PlanAction::Replan {
            session_id,
            delta_hint,
            write_to_disk,
            mode: _,
            json,
        } => {
            let resp = crate::dei_daemon::call(
                crate::dei_daemon::method::AI_PLAN_REPLAN,
                serde_json::json!({
                    "session_id": session_id,
                    "delta_hint": delta_hint,
                    "write_to_disk": write_to_disk,
                }),
                false,
            )
            .await?;
            print_plan_daemon_response(&resp, json, write_to_disk);
            Ok(())
        }
        PlanAction::Status { session_id, json } => {
            let resp = crate::dei_daemon::call(
                crate::dei_daemon::method::AI_PLAN_STATUS,
                serde_json::json!({
                    "session_id": session_id,
                }),
                false,
            )
            .await?;
            print_plan_summary_json(&resp, json);
            Ok(())
        }
        PlanAction::Execute {
            session_id,
            approve,
        } => {
            let resp = crate::dei_daemon::call(
                crate::dei_daemon::method::AI_PLAN_EXECUTE,
                serde_json::json!({
                    "session_id": session_id,
                    "approve": approve,
                }),
                false,
            )
            .await?;
            if let Some(ids) = resp.get("task_ids").and_then(|v| v.as_array()) {
                println!("✓ Plan execution started. Spawned {} tasks.", ids.len());
            } else {
                println!("✓ Plan execution finished (nothing to do).");
            }
            Ok(())
        }
    }
}

fn print_plan_daemon_response(resp: &serde_json::Value, json: bool, write: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(resp).unwrap_or_else(|_| "{}".to_string())
        );
        return;
    }
    let session_id = resp
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let goal = resp.get("goal").and_then(|v| v.as_str()).unwrap_or("");
    let summary = resp.get("summary").and_then(|v| v.as_str()).unwrap_or("");
    let steps = resp
        .get("versions")
        .and_then(|v| v.as_array())
        .and_then(|a| a.last())
        .and_then(|v| v.get("steps"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    println!("\n  \x1b[1;36m✓ Plan response\x1b[0m");
    println!("  Session : \x1b[33m{session_id}\x1b[0m");
    println!("  Goal    : {goal}");
    println!("  Summary : {summary}");
    println!("  Steps   : {steps}");
    if write {
        println!("  Written : PLAN.md (when supported by daemon)");
    }
    println!();
    if let Some(md) = resp.get("markdown").and_then(|v| v.as_str()) {
        println!("{md}");
    }
}

/// Print the visual summary of a plan status result (JSON from daemon).
pub fn print_plan_summary_json(resp: &serde_json::Value, json_out: bool) {
    if json_out {
        println!(
            "{}",
            serde_json::to_string_pretty(resp).unwrap_or_else(|_| "{}".into())
        );
    } else {
        let session_id = resp
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let goal = resp.get("goal").and_then(|v| v.as_str()).unwrap_or("");
        let mode = resp
            .get("execution_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let version = resp
            .get("latest_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        println!("\n  \x1b[1;36mPlan Status\x1b[0m");
        println!("  Session : \x1b[33m{session_id}\x1b[0m");
        println!("  Goal    : {goal}");
        println!("  Mode    : {mode}");
        println!("  Version : {version}");
        println!();

        if let Some(counts) = resp.get("step_counts").and_then(|v| v.as_object()) {
            let pending = counts.get("pending").and_then(|v| v.as_u64()).unwrap_or(0);
            let done = counts
                .get("completed")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let failed = counts.get("failed").and_then(|v| v.as_u64()).unwrap_or(0);
            let running = counts
                .get("in_progress")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let skipped = counts.get("skipped").and_then(|v| v.as_u64()).unwrap_or(0);

            println!("    \x1b[32m✓ completed  : {done}\x1b[0m");
            println!("    \x1b[36m⟳ in_progress: {running}\x1b[0m");
            println!("    \x1b[33m○ pending    : {pending}\x1b[0m");
            println!("    \x1b[31m✗ failed     : {failed}\x1b[0m");
            println!("    \x1b[2m⊘ skipped    : {skipped}\x1b[0m");
        }

        if let Some(events) = resp.get("recent_events").and_then(|v| v.as_array())
            && !events.is_empty()
        {
            println!();
            println!("  Recent events:");
            for e in events.iter().rev().take(5) {
                let at = e.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                let etype = e.get("event_type").and_then(|v| v.as_str()).unwrap_or("");
                let payload = e
                    .get("event_payload")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                println!("    [{at}] {etype}: {payload}");
            }
        }
        println!();
    }
}
