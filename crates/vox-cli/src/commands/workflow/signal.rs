//! `vox workflow signal` — append a signal for a durable workflow run.

use anyhow::Context;
use clap::Args;

/// Arguments for the `signal` subcommand.
#[derive(Debug, Args)]
pub struct SignalArgs {
    /// Durable workflow run identifier.
    #[arg(long)]
    pub run_id: String,
    /// Signal key awaited by the workflow.
    #[arg(long)]
    pub key: String,
    /// Optional JSON payload delivered to the workflow.
    #[arg(long)]
    pub payload: Option<String>,
}

pub(super) async fn run(args: &SignalArgs) -> anyhow::Result<()> {
    let payload = args.payload.as_deref().unwrap_or("null");
    validate_payload(payload)?;
    let db = vox_db::VoxDb::connect_default().await?;
    db.record_workflow_signal(&args.run_id, &args.key, payload)
        .await?;
    println!(
        "recorded signal `{}` for workflow run `{}`",
        args.key, args.run_id
    );
    Ok(())
}

fn validate_payload(payload: &str) -> anyhow::Result<()> {
    serde_json::from_str::<serde_json::Value>(payload)
        .context("--payload must be valid JSON")
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::validate_payload;

    #[test]
    fn signal_payload_accepts_json_and_rejects_plain_text() {
        assert!(validate_payload(r#"{"ok":true}"#).is_ok());
        assert!(validate_payload("not-json").is_err());
    }
}
