//! Clap subcommands for [`super::db`] (`vox db …`).

mod subcommands;
mod types;

pub use subcommands::DbCli;
pub use types::{
    ArxivHandoffStageCli, DbPreflightProfileCli, PublicationPrepareBodyCli, ScholarlyVenueCli,
};

/// Dispatch `vox db` subcommands to `commands::db` implementations.
pub async fn run(cmd: DbCli) -> anyhow::Result<()> {
    use super::db;
    match cmd {
        DbCli::Status => db::status().await,
        DbCli::Audit { timestamps } => db::audit(timestamps).await,
        DbCli::Reset { file } => db::reset(file.as_ref()).await,
        DbCli::Schema { file } => db::schema(file.as_ref()).await,
        DbCli::Explain {
            file,
            query,
            compact,
            jsonl,
        } => db::explain(file.as_ref(), query.as_deref(), !compact, jsonl).await,
        DbCli::Sample { table, limit } => db::sample(&table, limit).await,
        DbCli::Migrate { file } => db::migrate(file.as_ref()).await,
        DbCli::Export { user_id, output } => db::export(&user_id, output.as_ref()).await,
        DbCli::Import { path } => db::import(path.as_path()).await,
        DbCli::Vacuum => db::vacuum().await,
        DbCli::Prune { user_id, days } => db::prune(&user_id, days).await,
        DbCli::PrunePlan { policy } => db::prune_plan(policy.as_deref()).await,
        DbCli::PruneApply {
            policy,
            i_understand,
        } => db::prune_apply(policy.as_deref(), i_understand).await,
        DbCli::PrefGet { user_id, key } => db::pref_get(&user_id, &key).await,
        DbCli::PrefSet {
            user_id,
            key,
            value,
        } => db::pref_set(&user_id, &key, &value).await,
        DbCli::PrefList { user_id, prefix } => db::pref_list(&user_id, prefix.as_deref()).await,
        DbCli::CapabilityList => db::capability_list().await,
        DbCli::SyncInvocables { path } => db::sync_invocables(&path).await,
        DbCli::RetrievalStatus => db::retrieval_status().await,
        DbCli::ResearchIngestUrl {
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
        DbCli::ResearchIngestFile {
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
        DbCli::ResearchRefresh { vendor, dry_run } => db::research_refresh(&vendor, dry_run).await,
        DbCli::ResearchList {
            vendor,
            topic,
            limit,
        } => db::research_list(vendor.as_deref(), topic.as_deref(), limit).await,
        DbCli::ResearchMapAdd {
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
        DbCli::ResearchMapList {
            vendor,
            topic,
            limit,
        } => db::research_map_list(vendor.as_deref(), topic.as_deref(), limit).await,
        DbCli::ResearchMetrics {
            session_id,
            metric_type,
        } => db::research_metrics(session_id, metric_type.as_deref()).await,
        DbCli::ReliabilityList { domain, limit } => db::reliability_list(&domain, limit).await,
        DbCli::ReliabilityAgents { limit, min_score } => {
            db::reliability_agents(limit, min_score).await
        }
        DbCli::PublicationPrepare {
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
        DbCli::PublicationPrepareValidated {
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
        DbCli::PublicationPreflight {
            publication_id,
            profile,
            with_worthiness,
        } => db::publication_preflight(&publication_id, profile.into(), with_worthiness).await,
        DbCli::PublicationZenodoMetadata { publication_id } => {
            db::publication_zenodo_metadata(&publication_id).await
        }
        DbCli::PublicationScholarlyStagingExport {
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
        DbCli::PublicationWorthinessEvaluate {
            contract_yaml,
            metrics_json,
        } => db::publication_worthiness_evaluate(contract_yaml.as_ref(), metrics_json).await,
        DbCli::PublicationApprove {
            publication_id,
            approver,
        } => db::publication_approve(&publication_id, &approver).await,
        DbCli::PublicationSubmitLocal {
            publication_id,
            adapter,
        } => {
            db::publication_submit_local(&publication_id, adapter.as_deref()).await
        }
        DbCli::PublicationStatus { publication_id } => {
            db::publication_status(&publication_id).await
        }
        DbCli::PublicationScholarlyRemoteStatus {
            publication_id,
            external_submission_id,
        } => {
            db::publication_scholarly_remote_status(
                &publication_id,
                external_submission_id.as_deref(),
            )
            .await
        }
        DbCli::PublicationScholarlyRemoteStatusSyncAll { publication_id } => {
            db::publication_scholarly_remote_status_sync_all(&publication_id).await
        }
        DbCli::PublicationScholarlyRemoteStatusSyncBatch {
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
        DbCli::PublicationArxivHandoffRecord {
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
        DbCli::PublicationExternalJobsDue { limit } => {
            db::publication_external_jobs_due(limit).await
        }
        DbCli::PublicationExternalJobsDeadLetter { limit } => {
            db::publication_external_jobs_dead_letter(limit).await
        }
        DbCli::PublicationExternalJobsReplay { job_id } => {
            db::publication_external_jobs_replay(job_id).await
        }
        DbCli::PublicationExternalJobsTick {
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
        DbCli::PublicationScholarlyPipelineRun {
            publication_id,
            preflight_profile,
            dry_run,
            staging_output_dir,
            venue,
            adapter,
        } => {
            db::publication_scholarly_pipeline_run(
                &publication_id,
                preflight_profile.into(),
                dry_run,
                staging_output_dir.as_deref(),
                venue,
                adapter.as_deref(),
            )
            .await
        }
        DbCli::PublicationExternalPipelineMetrics { since_hours } => {
            db::publication_external_pipeline_metrics(since_hours).await
        }
        DbCli::PublicationMediaUpsert {
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
        DbCli::PublicationMediaList { publication_id } => {
            db::publication_media_list(&publication_id).await
        }
        DbCli::PublicationMediaDelete {
            publication_id,
            asset_ref,
        } => db::publication_media_delete(&publication_id, &asset_ref).await,
        DbCli::PublicationRouteSimulate {
            publication_id,
            json,
        } => db::publication_route_simulate(&publication_id, json).await,
        DbCli::PublicationPublish {
            publication_id,
            channels,
            dry_run,
            json,
        } => db::publication_publish(&publication_id, channels.as_deref(), dry_run, json).await,
        DbCli::PublicationRetryFailed {
            publication_id,
            channel,
            dry_run,
            json,
        } => db::publication_retry_failed(&publication_id, channel.as_deref(), dry_run, json).await,
    }
}
