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
    /// Status view of current queued plans and telemetry via CLI
    Status {
        /// Optional Plan Session ID to scope output
        session_id: Option<String>,
    },
}

pub async fn dispatch(cmd: PlanCmd) -> anyhow::Result<()> {
    match cmd {
        PlanCmd::Create { goal, approve } => {
            println!("Initializing PlanningOrchestrator for goal: {}", goal);
            if approve {
                println!("Auto-approve enabled: Bypassing RequiresApproval gate.");
            }
            // TODO: Wire direct PlanningOrchestrator dispatch bridging here.
        }
        PlanCmd::Replan { session_id } => {
            println!("Manually triggering Replan for Session: {}", session_id);
        }
        PlanCmd::Status { session_id } => {
            println!("Reading Planning DB bounds for SSE stream output...");
            if let Some(s) = session_id {
                println!("Filtering output for session: {}", s);
            }
        }
    }
    Ok(())
}
