use super::*;
use anyhow::Result;

/// Simulate per-channel routing/policy outcomes using an existing DB handle (tests and in-process callers).
pub async fn publication_route_simulate_with_db(
    db: &vox_db::VoxDb,
    publication_id: &str,
) -> Result<vox_publisher::SyndicationResult> {
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let item = publication_item_from_manifest(&row)?;
    let manifest = publication_manifest_from_row(&row);
    let root = vox_repository::resolve_repo_root_for_ci();
    let worthiness =
        vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(
            &manifest, &root,
        )
        .ok();
    let publisher = vox_publisher::Publisher::new(publisher_config_from_env(true, worthiness));
    publisher.publish_all(&item).await
}
/// Publish one prepared publication to selected channels (default: all configured channels).
pub async fn publication_publish(
    publication_id: &str,
    channels_csv: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let allowed = channels_csv
        .map(vox_publisher::switching::parse_channels_csv)
        .filter(|v| !v.is_empty());
    let mut item = publication_item_from_manifest(&row)?;
    if let Some(allowlist) = allowed.as_deref() {
        vox_publisher::switching::apply_channel_allowlist(&mut item, allowlist);
    }
    let digest = row.content_sha3_256.as_str();
    let dual = db
        .has_dual_publication_approval_for_digest(publication_id, digest)
        .await?;
    let gate = vox_publisher::gate::evaluate_publish_gate(
        vox_publisher::gate::publish_gate_inputs_for_cli(dry_run, true, dual, &item),
    );
    if gate.has_blockers() {
        let detail = serde_json::json!({ "blocking_reasons": gate.blocking_reasons });
        anyhow::bail!(
            "live publish blocked by gate: {}",
            serde_json::to_string(&detail)?
        );
    }
    let manifest = publication_manifest_from_row(&row);
    let root = vox_repository::resolve_repo_root_for_ci();
    let worthiness =
        vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(
            &manifest, &root,
        )
        .ok();
    if cli_social_worthiness_enforce()
        && !dry_run
        && !item.syndication.dry_run
        && gate.live_publish_allowed
        && let Some(score) = worthiness
    {
        let floor = cli_social_worthiness_score_min();
        if score < floor {
            let detail = serde_json::json!({
                "error": "live publish blocked by worthiness floor",
                "worthiness_score": score,
                "floor": floor,
            });
            anyhow::bail!(
                "live publish blocked by worthiness: {}",
                serde_json::to_string(&detail)?
            );
        }
    }
    let publisher = vox_publisher::Publisher::new(publisher_config_from_env(dry_run, worthiness));
    let result = publisher.publish_all(&item).await?;
    let result_json = serde_json::to_string(&result)?;
    db.record_publication_attempt(publication_id, digest, "manual_cli", &result_json)
        .await?;
    if gate.live_publish_allowed {
        if result.all_enabled_channels_succeeded(&item) {
            let _ = db
                .set_publication_state(
                    publication_id,
                    "published",
                    Some(&serde_json::json!({ "channel_group": "manual_cli" }).to_string()),
                )
                .await;
        } else if result.has_failures() {
            let _ = db
                .set_publication_state(
                    publication_id,
                    "publish_failed",
                    Some(&serde_json::json!({ "channel_group": "manual_cli" }).to_string()),
                )
                .await;
        }
    }
    if json {
        println!("{}", result_json);
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}
/// Retry failed channels from the latest publication attempt.
pub async fn publication_retry_failed(
    publication_id: &str,
    channel: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let digest = row.content_sha3_256.as_str();
    let attempts = db.list_publication_attempts(publication_id).await?;
    let attempt_refs: Vec<vox_publisher::switching::AttemptOutcome<'_>> = attempts
        .iter()
        .map(|a| vox_publisher::switching::AttemptOutcome {
            content_sha3_256: a.content_sha3_256.as_str(),
            outcome_json: a.outcome_json.as_str(),
        })
        .collect();

    let explicit: Option<Vec<String>> = channel.map(vox_publisher::switching::parse_channels_csv);
    let plan = match vox_publisher::switching::plan_publication_retry_channels(
        attempt_refs.as_slice(),
        digest,
        explicit.as_deref(),
    )? {
        None => {
            anyhow::bail!(
                "no syndication attempt outcome for current manifest digest; run `vox db publication-publish` first"
            );
        }
        Some(p) => p,
    };

    if !plan.skipped_success_channels.is_empty() && plan.will_retry_channels.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "publication_id": publication_id,
                "retried": false,
                "reason": "channels_already_succeeded_for_digest",
                "skipped_success_channels": plan.skipped_success_channels,
                "blocked_channels": plan.blocked_channels,
            }))?
        );
        return Ok(());
    }

    if plan.will_retry_channels.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "publication_id": publication_id,
                "retried": false,
                "reason": if channel.is_some() { "no_channels_eligible_for_retry" } else { "no_failed_channels" },
                "skipped_success_channels": plan.skipped_success_channels,
                "blocked_channels": plan.blocked_channels,
            }))?
        );
        return Ok(());
    }

    let csv = plan.will_retry_channels.join(",");
    if !json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "publication_id": publication_id,
                "will_retry_channels": plan.will_retry_channels,
                "skipped_success_channels": plan.skipped_success_channels,
                "blocked_channels": plan.blocked_channels,
            }))?
        );
    }
    publication_publish(publication_id, Some(csv.as_str()), dry_run, json).await
}
