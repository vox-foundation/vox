//! `vox workflow` — workflow introspection commands (P1-T8).

pub mod preview;

#[derive(clap::Subcommand, Debug)]
pub enum WorkflowCmd {
    /// Project the schedule of activities a workflow would dispatch without running it.
    Preview(preview::WorkflowPreviewArgs),
}

pub async fn run(cmd: WorkflowCmd) -> anyhow::Result<()> {
    match cmd {
        WorkflowCmd::Preview(args) => preview::run(&args).await,
    }
}
