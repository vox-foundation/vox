#![allow(unused_imports)]
use crate::params::ToolResult;
use crate::server_state::ServerState;

#[cfg(feature = "heavy-browser")]
use crate::browser_tools;
#[cfg(feature = "gui-visual-review")]
use crate::visus_tools;
use crate::{
    agent_tools, benchmark_tools, chat_tools, code_validator, codex_tools, compiler_tools,
    db_tools, exec_time_tools, feedback_tools, git_tools, grammar_tools, introspection_tools,
    openclaw_tools, persistence_tools, populi_tools, project_init_tools, questioning_tools,
    rag_tools, receipt_tools, repo_catalog_tools, repo_index, secrets_tools, task_tools,
    toestub_tools, tool_aliases, training_tools, trust_tools, vcs_tools,
};
#[cfg(feature = "news-publish")]
use crate::{news_tools, scientia_tools};

#[cfg(feature = "oratio-rerank")]
use crate::{oratio_tools, speech_pipeline_tools};

pub fn handles(name: &str) -> bool {
    match name {
        #[cfg(feature = "news-publish")]
        "vox_news_test_syndicate" => true,
        #[cfg(feature = "news-publish")]
        "vox_news_draft_research" => true,
        #[cfg(feature = "news-publish")]
        "vox_news_approve" => true,
        #[cfg(feature = "news-publish")]
        "vox_news_approval_status" => true,
        #[cfg(feature = "news-publish")]
        "vox_news_simulate_publish_gate" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_prepare" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_approve" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_submit_local" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_status" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status_sync_all" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status_sync_batch" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_arxiv_handoff_record" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_staging_export" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_due" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_dead_letter" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_replay" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_tick" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_pipeline_metrics" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_pipeline_run" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_upsert" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_list" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_delete" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_route_simulate" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_publish" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_retry_failed" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_preflight" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_worthiness_evaluate" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_scan" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_explain" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_refresh_evidence" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_novelty_fetch" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_decision_explain" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_novelty_happy_path" => true,
        #[cfg(feature = "news-publish")]
        "vox_scientia_assist_suggestions" => true,
        "vox_kb_search" => true,
        "vox_kb_get" => true,
        "vox_kb_cite" => true,
        "vox_kb_status" => true,
        "vox_kb_reindex" => true,
        _ => false,
    }
}

#[inline(never)]
pub async fn dispatch(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    match name {
        #[cfg(feature = "news-publish")]
        "vox_news_test_syndicate" => {
            Ok(news_tools::vox_news_test_syndicate(state, serde_json::from_value(args)?).await)
        }

        #[cfg(feature = "news-publish")]
        "vox_news_draft_research" => {
            Ok(news_tools::vox_news_draft_research(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "news-publish")]
        "vox_news_approve" => {
            Ok(news_tools::vox_news_approve(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "news-publish")]
        "vox_news_approval_status" => {
            Ok(news_tools::vox_news_approval_status(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "news-publish")]
        "vox_news_simulate_publish_gate" => Ok(news_tools::vox_news_simulate_publish_gate(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_prepare" => Ok(scientia_tools::vox_scientia_publication_prepare(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_approve" => Ok(scientia_tools::vox_scientia_publication_approve(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_submit_local" => {
            Ok(scientia_tools::vox_scientia_publication_submit_local(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_status" => Ok(scientia_tools::vox_scientia_publication_status(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status_sync_all" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status_sync_all(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status_sync_batch" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status_sync_batch(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_arxiv_handoff_record" => Ok(
            scientia_tools::vox_scientia_publication_arxiv_handoff_record(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_staging_export" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_staging_export(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_due" => {
            Ok(scientia_tools::vox_scientia_publication_external_jobs_due(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_dead_letter" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_dead_letter(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_replay" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_replay(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_tick" => {
            Ok(scientia_tools::vox_scientia_publication_external_jobs_tick(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_pipeline_metrics" => Ok(
            scientia_tools::vox_scientia_publication_external_pipeline_metrics(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_pipeline_run" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_pipeline_run(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_upsert" => {
            Ok(scientia_tools::vox_scientia_publication_media_upsert(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_list" => {
            Ok(scientia_tools::vox_scientia_publication_media_list(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_delete" => {
            Ok(scientia_tools::vox_scientia_publication_media_delete(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_route_simulate" => {
            Ok(scientia_tools::vox_scientia_publication_route_simulate(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_publish" => Ok(scientia_tools::vox_scientia_publication_publish(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_retry_failed" => {
            Ok(scientia_tools::vox_scientia_publication_retry_failed(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_preflight" => {
            Ok(scientia_tools::vox_scientia_publication_preflight(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_worthiness_evaluate" => Ok(scientia_tools::vox_scientia_worthiness_evaluate(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_scan" => {
            Ok(scientia_tools::vox_scientia_publication_discovery_scan(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_explain" => {
            Ok(scientia_tools::vox_scientia_publication_discovery_explain(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_refresh_evidence" => Ok(
            scientia_tools::vox_scientia_publication_discovery_refresh_evidence(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_novelty_fetch" => {
            Ok(scientia_tools::vox_scientia_publication_novelty_fetch(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_decision_explain" => {
            Ok(scientia_tools::vox_scientia_publication_decision_explain(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_novelty_happy_path" => {
            Ok(scientia_tools::vox_scientia_publication_novelty_happy_path(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_assist_suggestions" => Ok(scientia_tools::vox_scientia_assist_suggestions(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        // ── Knowledge Bases (VoxKB) ──────────────────────────────────────────
        "vox_kb_search" => {
            Ok(crate::kb::library::kb_search(state, serde_json::from_value(args)?).await)
        }
        "vox_kb_get" => Ok(crate::kb::library::kb_get(state, serde_json::from_value(args)?).await),
        "vox_kb_cite" => {
            Ok(crate::kb::library::kb_cite(state, serde_json::from_value(args)?).await)
        }
        "vox_kb_status" => Ok(crate::kb::library::kb_status(state).await),
        "vox_kb_reindex" => {
            crate::kb::library::reject_reindex_args(&args).map_err(anyhow::Error::msg)?;
            Ok(crate::kb::library::kb_reindex(state).await)
        }
        _ => Err(anyhow::anyhow!("unhandled tool in scientia_kb: {}", name)),
    }
}
