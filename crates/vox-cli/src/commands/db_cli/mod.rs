//! Clap subcommands for [`super::db`] (`vox db …`).

mod core_subcommands;
mod publication_subcommands;
mod subcommands;
mod types;

pub use core_subcommands::DbCliCore;
pub use publication_subcommands::DbCliPublication;
pub use subcommands::DbCli;
pub use types::{
    ArxivHandoffStageCli, DbPreflightProfileCli, PublicationPrepareBodyCli, ScholarlyVenueCli,
};

/// Dispatch `vox db` subcommands to `commands::db` implementations.
pub async fn run(cmd: DbCli) -> anyhow::Result<()> {
    use super::db;
    match cmd {
        DbCli::Core(cmd) => match cmd {
            DbCliCore::Status => db::status().await,
            DbCliCore::Audit { timestamps } => db::audit(timestamps).await,
            DbCliCore::Reset { file } => db::reset(file.as_ref()).await,
            DbCliCore::Schema { file } => db::schema(file.as_ref()).await,
            DbCliCore::Explain {
                file,
                query,
                compact,
                jsonl,
            } => db::explain(file.as_ref(), query.as_deref(), !compact, jsonl).await,
            DbCliCore::Sample { table, limit } => db::sample(&table, limit).await,
            DbCliCore::Migrate { file } => db::migrate(file.as_ref()).await,
            DbCliCore::Export { user_id, output } => db::export(&user_id, output.as_ref()).await,
            DbCliCore::Import { path } => db::import(path.as_path()).await,
            DbCliCore::Vacuum => db::vacuum().await,
            DbCliCore::Prune { user_id, days } => db::prune(&user_id, days).await,
            DbCliCore::PrunePlan { policy } => db::prune_plan(policy.as_deref()).await,
            DbCliCore::PruneApply {
                policy,
                i_understand,
            } => db::prune_apply(policy.as_deref(), i_understand).await,
            DbCliCore::PrefGet { user_id, key } => db::pref_get(&user_id, &key).await,
            DbCliCore::PrefSet {
                user_id,
                key,
                value,
            } => db::pref_set(&user_id, &key, &value).await,
            DbCliCore::PrefList { user_id, prefix } => {
                db::pref_list(&user_id, prefix.as_deref()).await
            }
            DbCliCore::CapabilityList => db::capability_list().await,
            DbCliCore::SyncInvocables { path } => db::sync_invocables(&path).await,
            DbCliCore::RetrievalStatus => db::retrieval_status().await,
            DbCliCore::ResearchIngestUrl {
                vendor,
                topic,
                url,
                title,
                summary,
                source_type,
                area,
                kb_id,
                tags,
                confidence,
            } => {
                db::research_ingest_url(
                    &vendor,
                    &topic,
                    &url,
                    title.as_deref(),
                    summary.as_deref(),
                    &source_type,
                    area.as_deref(),
                    kb_id.as_deref(),
                    tags.as_deref(),
                    confidence,
                )
                .await
            }
            DbCliCore::ResearchIngestFile {
                vendor,
                topic,
                path,
                area,
                kb_id,
                tags,
                confidence,
            } => {
                db::research_ingest_file(
                    &vendor,
                    &topic,
                    &path,
                    area.as_deref(),
                    kb_id.as_deref(),
                    tags.as_deref(),
                    confidence,
                )
                .await
            }
            DbCliCore::ResearchRefresh { vendor, dry_run } => {
                db::research_refresh(&vendor, dry_run).await
            }
            DbCliCore::ResearchList {
                vendor,
                topic,
                limit,
            } => db::research_list(vendor.as_deref(), topic.as_deref(), limit).await,
            DbCliCore::ResearchMapAdd {
                vendor,
                topic,
                area,
                openclaw_capability,
                vox_evidence,
                status,
                advantage_direction,
                recommended_action,
                linked_paths,
            } => {
                db::research_map_add(
                    &vendor,
                    &topic,
                    &area,
                    &openclaw_capability,
                    &vox_evidence,
                    &status,
                    &advantage_direction,
                    &recommended_action,
                    linked_paths.as_deref(),
                )
                .await
            }
            DbCliCore::ResearchMapList {
                vendor,
                topic,
                limit,
            } => db::research_map_list(vendor.as_deref(), topic.as_deref(), limit).await,
            DbCliCore::ResearchMetrics {
                session_id,
                metric_type,
            } => db::research_metrics(session_id.as_str(), metric_type.as_deref()).await,
            DbCliCore::ReliabilityList { domain, limit } => {
                db::reliability_list(&domain, limit).await
            }
            DbCliCore::ReliabilityAgents { limit, min_score } => {
                db::reliability_agents(limit, min_score).await
            }
        },
        DbCli::Publication(cmd) => match cmd {
            DbCliPublication::PublicationPrepare {
                content_type,
                body,
                preflight,
                preflight_profile,
            } => {
                db::publication_prepare(
                    &body.publication_id,
                    &content_type,
                    &body.author,
                    &body.title,
                    body.path.as_path(),
                    body.abstract_text.as_deref(),
                    body.citations_json.as_deref(),
                    body.scholarly_metadata_json.as_deref(),
                    preflight,
                    preflight_profile.into(),
                )
                .await
            }
            DbCliPublication::PublicationPrepareValidated {
                content_type,
                body,
                preflight_profile,
            } => {
                db::publication_prepare(
                    &body.publication_id,
                    &content_type,
                    &body.author,
                    &body.title,
                    body.path.as_path(),
                    body.abstract_text.as_deref(),
                    body.citations_json.as_deref(),
                    body.scholarly_metadata_json.as_deref(),
                    true,
                    preflight_profile.into(),
                )
                .await
            }
            DbCliPublication::PublicationPreflight {
                publication_id,
                profile,
                with_worthiness,
            } => db::publication_preflight(&publication_id, profile.into(), with_worthiness).await,
            DbCliPublication::PublicationZenodoMetadata { publication_id } => {
                db::publication_zenodo_metadata(&publication_id).await
            }
            DbCliPublication::PublicationOpenreviewProfile { publication_id } => {
                db::publication_openreview_profile(&publication_id).await
            }
            DbCliPublication::PublicationScholarlyStagingExport {
                publication_id,
                output_dir,
                venue,
            } => {
                db::publication_scholarly_staging_export(
                    &publication_id,
                    output_dir.as_path(),
                    venue.to_venue(),
                )
                .await
            }
            DbCliPublication::PublicationWorthinessEvaluate {
                contract_yaml,
                metrics_json,
            } => db::publication_worthiness_evaluate(contract_yaml.as_ref(), metrics_json).await,
            DbCliPublication::PublicationApprove {
                publication_id,
                approver,
            } => db::publication_approve(&publication_id, &approver).await,
            DbCliPublication::PublicationSubmitLocal {
                publication_id,
                adapter,
            } => db::publication_submit_local(&publication_id, adapter.as_deref()).await,
            DbCliPublication::PublicationStatus { publication_id } => {
                db::publication_status(&publication_id).await
            }
            DbCliPublication::PublicationScholarlyRemoteStatus {
                publication_id,
                external_submission_id,
            } => {
                db::publication_scholarly_remote_status(
                    &publication_id,
                    external_submission_id.as_deref(),
                )
                .await
            }
            DbCliPublication::PublicationScholarlyRemoteStatusSyncAll { publication_id } => {
                db::publication_scholarly_remote_status_sync_all(&publication_id).await
            }
            DbCliPublication::PublicationScholarlyRemoteStatusSyncBatch {
                limit,
                iterations,
                interval_secs,
                max_runtime_secs,
                jitter_secs,
            } => {
                db::publication_scholarly_remote_status_sync_batch(
                    limit,
                    iterations,
                    interval_secs,
                    max_runtime_secs,
                    jitter_secs,
                )
                .await
            }
            DbCliPublication::PublicationArxivHandoffRecord {
                publication_id,
                stage,
                operator,
                note,
                arxiv_id,
            } => {
                db::publication_arxiv_handoff_record(
                    &publication_id,
                    stage,
                    operator.as_deref(),
                    note.as_deref(),
                    arxiv_id.as_deref(),
                )
                .await
            }
            DbCliPublication::PublicationExternalJobsDue { limit } => {
                db::publication_external_jobs_due(limit).await
            }
            DbCliPublication::PublicationExternalJobsDeadLetter { limit } => {
                db::publication_external_jobs_dead_letter(limit).await
            }
            DbCliPublication::PublicationExternalJobsReplay { job_id } => {
                db::publication_external_jobs_replay(job_id).await
            }
            DbCliPublication::PublicationExternalJobsTick {
                limit,
                lock_ttl_ms,
                lock_owner,
                iterations,
                interval_secs,
                max_runtime_secs,
                jitter_secs,
            } => {
                db::publication_external_jobs_tick(
                    limit,
                    lock_ttl_ms,
                    lock_owner.as_deref(),
                    iterations,
                    interval_secs,
                    max_runtime_secs,
                    jitter_secs,
                )
                .await
            }
            DbCliPublication::PublicationScholarlyPipelineRun {
                publication_id,
                preflight_profile,
                dry_run,
                staging_output_dir,
                venue,
                adapter,
                json,
            } => {
                db::publication_scholarly_pipeline_run(
                    &publication_id,
                    preflight_profile.into(),
                    dry_run,
                    staging_output_dir.as_deref(),
                    venue,
                    adapter.as_deref(),
                    json,
                )
                .await
            }
            DbCliPublication::PublicationExternalPipelineMetrics { since_hours } => {
                db::publication_external_pipeline_metrics(since_hours).await
            }
            DbCliPublication::PublicationMediaUpsert {
                publication_id,
                asset_ref,
                media_type,
                storage_uri,
                status,
                metadata_json_path,
            } => {
                db::publication_media_upsert(
                    &publication_id,
                    &asset_ref,
                    &media_type,
                    storage_uri.as_deref(),
                    &status,
                    metadata_json_path.as_ref(),
                )
                .await
            }
            DbCliPublication::PublicationMediaList { publication_id } => {
                db::publication_media_list(&publication_id).await
            }
            DbCliPublication::PublicationMediaDelete {
                publication_id,
                asset_ref,
            } => db::publication_media_delete(&publication_id, &asset_ref).await,
            DbCliPublication::PublicationRouteSimulate {
                publication_id,
                json,
            } => db::publication_route_simulate(&publication_id, json).await,
            DbCliPublication::PublicationPublish {
                publication_id,
                channels,
                dry_run,
                json,
            } => db::publication_publish(&publication_id, channels.as_deref(), dry_run, json).await,
            DbCliPublication::PublicationRetryFailed {
                publication_id,
                channel,
                dry_run,
                json,
            } => {
                db::publication_retry_failed(&publication_id, channel.as_deref(), dry_run, json)
                    .await
            }
        },
    }
}
