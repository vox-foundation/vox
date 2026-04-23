use crate::syndication_outcome::ChannelOutcome;
use crate::types::UnifiedNewsItem;
use anyhow::Result;

pub async fn post(_item: &UnifiedNewsItem, dry_run: bool) -> Result<ChannelOutcome> {
    if dry_run {
        return Ok(ChannelOutcome::DryRun { external_id: None });
    }

    // Since ResearchGate has no public Posting API as of 2026,
    // this adapter serves as a "Manual Action Required" bridge.
    // It captures the intent and returns a failure class that tells the orchestrator
    // to flag this for human intervention or to display instructions.

    Ok(ChannelOutcome::Failed {
        code: "manual_action_required".to_string(),
        message: "ResearchGate requires manual upload via their web interface. Use the DOI to import metadata.".to_string(),
        retryable: false,
        failure_class: Some(crate::syndication_outcome::FailureClass::ManualActionRequired),
    })
}
