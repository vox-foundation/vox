use super::db_util::get_db;
use anyhow::Result;
use owo_colors::OwoColorize;
use vox_gamify::db::{
    appeal_dispute as db_appeal_dispute, cast_vote, file_dispute as db_file_dispute,
};
use vox_gamify::util::now_unix;

pub async fn dispute_file(
    target_user: &str,
    event_id: Option<&str>,
    rationale: &str,
) -> Result<()> {
    let db = get_db().await?;
    let accuser_id = vox_gamify::db::canonical_user_id();

    // Simple dispute ID generation for CLI testing
    let dispute_id = format!("dsp-{}", now_unix());

    db_file_dispute(
        &db,
        &dispute_id,
        target_user,
        &accuser_id,
        event_id,
        None,
        rationale, // Using rationale directly as evidence_json for simplicity in MVP
        0.5,       // Default malice score
    )
    .await?;

    println!(
        "{} {}",
        "✓".green().bold(),
        "Dispute filed successfully.".bold()
    );
    println!("  Dispute ID: {}", dispute_id.cyan());
    println!("  Target: {}", target_user.yellow());
    Ok(())
}

pub async fn dispute_vote(dispute_id: &str, verdict: &str, rationale: Option<&str>) -> Result<()> {
    let db = get_db().await?;
    let juror_id = vox_gamify::db::canonical_user_id();

    cast_vote(&db, dispute_id, &juror_id, verdict, rationale).await?;

    println!(
        "{} {}",
        "✓".green().bold(),
        "Vote cast successfully.".bold()
    );
    println!("  Dispute ID: {}", dispute_id.cyan());
    println!("  Verdict: {}", verdict.yellow());
    Ok(())
}

pub async fn dispute_status(dispute_id: &str) -> Result<()> {
    // Basic status print for MVP since getting single dispute isn't explicitly implemented
    // in vox_gamify DB layer yet outside of get_gamify_disputes_by_status.
    println!("{} {}", "ℹ".cyan().bold(), "Dispute Status Check".bold());
    println!("  Dispute ID: {}", dispute_id.cyan());
    println!("  (Detailed status fetching requires further VoxDb integration)");
    Ok(())
}

pub async fn dispute_appeal(dispute_id: &str) -> Result<()> {
    let db = get_db().await?;
    db_appeal_dispute(&db, dispute_id).await?;

    println!(
        "{} {}",
        "✓".green().bold(),
        "Dispute appealed successfully.".bold()
    );
    println!("  Dispute ID: {}", dispute_id.cyan());
    println!("  Status: {}", "appealed".yellow());
    Ok(())
}
