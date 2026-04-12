//! Publication manifest and syndication helpers for `vox db publication-*`.

mod helpers;
mod ingest;

pub use ingest::*;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::commands::db_cli::{ArxivHandoffStageCli, ScholarlyVenueCli};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;

use helpers::{
    build_scientia_evidence_context, read_scientific_metadata_json, repository_id_for_prepare,
    source_ref_string,
};
/// Prepare (upsert) a canonical publication manifest from markdown body content.
#[allow(clippy::too_many_arguments)]
pub async fn publication_prepare(
    publication_id: &str,
    content_type: &str,
    author: &str,
    title: Option<&str>,
    path: &Path,
    abstract_text: Option<&str>,
    citations_json_path: Option<&Path>,
    scholarly_metadata_json_path: Option<&Path>,
    eval_gate_report_json_path: Option<&Path>,
    benchmark_pair_report_json_path: Option<&Path>,
    human_meaningful_advance: bool,
    human_ai_disclosure_complete: bool,
    preflight: bool,
    preflight_profile: vox_publisher::publication_preflight::PreflightProfile,
    discovery_intake_gate: vox_publisher::scientia_discovery::DiscoveryIntakeGate,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let repository_id = repository_id_for_prepare(&repo_root);
    let body_markdown = read_utf8_path_capped(path)
        .with_context(|| format!("failed to read markdown body from {}", path.display()))?;
    let inferred_title = title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| vox_publisher::scientia_evidence::infer_markdown_title(&body_markdown));
    let citations_json = if let Some(p) = citations_json_path {
        Some(
            read_utf8_path_capped(p)
                .with_context(|| format!("failed to read citations JSON from {}", p.display()))?,
        )
    } else {
        None
    };
    let scientific = read_scientific_metadata_json(scholarly_metadata_json_path)?;
    let source_ref = source_ref_string(&repo_root, path);
    let scientia_evidence = build_scientia_evidence_context(
        &repo_root,
        &source_ref,
        abstract_text,
        citations_json.as_deref(),
        scientific.as_ref(),
        eval_gate_report_json_path,
        benchmark_pair_report_json_path,
        human_meaningful_advance,
        human_ai_disclosure_complete,
        body_markdown.as_str(),
    )?;
    if content_type == "scientia"
        && discovery_intake_gate != vox_publisher::scientia_discovery::DiscoveryIntakeGate::None
    {
        let empty_rank_evidence =
            vox_publisher::scientia_evidence::ScientiaEvidenceContext::default();
        let ev_for_rank = scientia_evidence.as_ref().unwrap_or(&empty_rank_evidence);
        let scientia_h =
            vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
        let rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
            publication_id,
            Some(source_ref.as_str()),
            ev_for_rank,
            &scientia_h,
            None,
        );
        if !vox_publisher::scientia_discovery::intake_gate_allows(discovery_intake_gate, &rank) {
            anyhow::bail!(
                "discovery intake gate blocked prepare: gate={discovery_intake_gate:?} rank_score={} intake_tier={:?} auto_draft_eligible={}; {}",
                rank.rank_score,
                rank.intake_tier,
                rank.auto_draft_eligible,
                rank.machine_explanation.join("; ")
            );
        }
    }
    let metadata_json = vox_publisher::scientific_metadata::build_scientia_metadata_json(
        "vox db publication-prepare",
        Some(repository_id.as_str()),
        scientific.as_ref(),
        scientia_evidence.as_ref(),
    )
    .context("build publication metadata_json")?;
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: publication_id.to_string(),
        content_type: content_type.to_string(),
        source_ref: Some(source_ref.clone()),
        title: inferred_title,
        author: author.to_string(),
        abstract_text: abstract_text.map(std::string::ToString::to_string),
        body_markdown,
        citations_json: citations_json.clone(),
        metadata_json: Some(metadata_json),
    };
    if preflight {
        let report =
            vox_publisher::publication_preflight::run_preflight(&manifest, preflight_profile);
        if !report.ok {
            anyhow::bail!(
                "publication preflight failed (readiness {}):\n{}",
                report.readiness_score,
                serde_json::to_string_pretty(&report)?
            );
        }
    }

    let digest = manifest.content_sha3_256();
    db.upsert_publication_manifest(vox_db::PublicationManifestParams {
        publication_id: &manifest.publication_id,
        content_type: &manifest.content_type,
        source_ref: manifest.source_ref.as_deref(),
        title: &manifest.title,
        author: &manifest.author,
        abstract_text: manifest.abstract_text.as_deref(),
        body_markdown: &manifest.body_markdown,
        citations_json: citations_json.as_deref(),
        metadata_json: manifest.metadata_json.as_deref(),
        revision_history_json: None,
        content_sha3_256: &digest,
        state: "draft",
    })
    .await?;
    if let Some(ref evidence) = scientia_evidence
        && !evidence.discovery_signals.is_empty()
    {
        let detail = serde_json::json!({
            "source_ref": source_ref,
            "candidate_note": evidence.candidate_note,
            "discovery_signals": evidence.discovery_signals,
            "draft_preparation": evidence.draft_preparation,
        });
        db.append_publication_status_event(
            publication_id,
            "discovery_candidate_prepared",
            Some(&serde_json::to_string(&detail)?),
        )
        .await?;
    }
    println!(
        "Prepared publication '{}' ({}) digest={}{}",
        publication_id,
        content_type,
        digest,
        scientia_evidence
            .as_ref()
            .and_then(|e| e.candidate_note.as_deref())
            .map(|note| format!(" note={note}"))
            .unwrap_or_default()
    );
    Ok(())
}

/// Reload live telemetry / sidecars, recompute `scientia_evidence`, and upsert the manifest (scientia only).
pub async fn publication_discovery_refresh_evidence(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let repository_id = repository_id_for_prepare(&repo_root);
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    if row.content_type != "scientia" {
        anyhow::bail!(
            "publication-discovery-refresh-evidence requires content_type `scientia` (got `{}`)",
            row.content_type
        );
    }
    let mut manifest = publication_manifest_from_row(&row);
    manifest =
        crate::commands::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
            manifest,
            &db,
            &repo_root,
            Some(repository_id.as_str()),
        )
        .await?;

    let scientific = vox_publisher::publication_preflight::parse_scientific_from_metadata_json(
        manifest.metadata_json.as_deref(),
    )
    .ok()
    .flatten();

    let new_meta = vox_publisher::scientia_evidence::rebuild_scientia_evidence_metadata_json(
        manifest.metadata_json.as_deref(),
        manifest.body_markdown.as_str(),
        manifest.abstract_text.as_deref(),
        manifest.citations_json.as_deref(),
        scientific.as_ref(),
        manifest
            .source_ref
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
        Some("vox db publication-discovery-refresh-evidence"),
    )
    .context("rebuild scientia_evidence metadata_json")?;

    manifest.metadata_json = Some(new_meta);
    let digest = manifest.content_sha3_256();

    db.upsert_publication_manifest(vox_db::PublicationManifestParams {
        publication_id: &manifest.publication_id,
        content_type: &manifest.content_type,
        source_ref: manifest.source_ref.as_deref(),
        title: &manifest.title,
        author: &manifest.author,
        abstract_text: manifest.abstract_text.as_deref(),
        body_markdown: &manifest.body_markdown,
        citations_json: manifest.citations_json.as_deref(),
        metadata_json: manifest.metadata_json.as_deref(),
        revision_history_json: row.revision_history_json.as_deref(),
        content_sha3_256: &digest,
        state: row.state.as_str(),
    })
    .await?;

    let evidence = vox_publisher::scientia_evidence::parse_scientia_evidence(
        manifest.metadata_json.as_deref(),
    )
    .unwrap_or_default();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        publication_id,
        manifest.source_ref.as_deref(),
        &evidence,
        &scientia_h,
        None,
    );
    let detail = serde_json::json!({ "digest": digest, "rank": rank });
    db.append_publication_status_event(
        publication_id,
        "discovery_evidence_refreshed",
        Some(&serde_json::to_string(&detail)?),
    )
    .await?;

    println!("Refreshed discovery evidence for '{publication_id}' digest={digest}");
    Ok(())
}

/// Print a JSON preflight report for a manifest already in Codex (no DB writes).
pub async fn publication_preflight(
    publication_id: &str,
    profile: vox_publisher::publication_preflight::PreflightProfile,
    with_worthiness: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let mut manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let item = publication_item_from_manifest(&row)?;
    let attention = publication_attention_inputs_for_row(&db, &row, &item).await?;
    let report = if with_worthiness {
        let root = vox_repository::resolve_repo_root_for_ci();
        manifest =
            crate::commands::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
                manifest, &db, &root, None,
            )
            .await?;
        let contract_path =
            root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH);
        let yaml = read_utf8_path_capped(&contract_path).with_context(|| {
            format!(
                "read worthiness contract {} (repo root discovery required)",
                contract_path.display()
            )
        })?;
        let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml)?;
        vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;
        vox_publisher::publication_preflight::run_preflight_with_worthiness_attention(
            &manifest,
            profile,
            &contract,
            Some(attention),
        )
    } else {
        vox_publisher::publication_preflight::run_preflight_with_attention(
            &manifest,
            profile,
            Some(attention),
        )
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Print Zenodo-oriented deposition metadata JSON (no network).
fn resolve_under_repo(root: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

/// Print worthiness evaluation JSON using the repo contract + metrics inputs (no DB writes).
pub async fn publication_worthiness_evaluate(
    contract_yaml: Option<&PathBuf>,
    metrics_json: PathBuf,
) -> Result<()> {
    let root = vox_repository::resolve_repo_root_for_ci();
    let contract_path = match contract_yaml {
        Some(p) => resolve_under_repo(&root, p),
        None => root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH),
    };
    let yaml = read_utf8_path_capped(&contract_path)
        .with_context(|| format!("read contract {}", contract_path.display()))?;
    let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml)?;
    vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;
    let metrics_path = resolve_under_repo(&root, &metrics_json);
    let m_src = read_utf8_path_capped(&metrics_path)
        .with_context(|| format!("read metrics {}", metrics_path.display()))?;
    let inputs: vox_publisher::publication_worthiness::WorthinessInputs =
        serde_json::from_str(&m_src).context("parse metrics JSON")?;
    let out = vox_publisher::publication_worthiness::evaluate_worthiness(&contract, &inputs);
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

pub async fn publication_zenodo_metadata(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let z = vox_publisher::zenodo_metadata::zenodo_deposition_metadata(&manifest);
    println!("{}", serde_json::to_string_pretty(&z)?);
    Ok(())
}

/// Print merged OpenReview submit profile JSON (invitation, signature, readers, API base; no HTTP).
pub async fn publication_openreview_profile(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let profile = vox_publisher::scholarly::export_openreview_submit_profile(&manifest)?;
    println!("{}", serde_json::to_string_pretty(&profile)?);
    Ok(())
}

/// Write [`vox_publisher::submission_package`] staging files for an existing manifest (by id).
pub async fn publication_scholarly_staging_export(
    publication_id: &str,
    output_dir: &std::path::Path,
    venue: vox_publisher::submission_package::ScholarlyVenue,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let written =
        vox_publisher::submission_package::write_scholarly_staging(&manifest, venue, output_dir)?;
    vox_publisher::submission_package::validate_scholarly_staging(output_dir, venue, &manifest)
        .map_err(|findings| {
            let msg: String = findings
                .iter()
                .map(|f| format!("{}: {}", f.code, f.message))
                .collect::<Vec<_>>()
                .join("; ");
            anyhow::anyhow!("staging validation failed: {msg}")
        })?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": publication_id,
            "output_dir": output_dir,
            "venue": venue.as_str(),
            "written": written,
        }))?
    );
    Ok(())
}

/// One-shot scholarly pipeline: local preflight, dual-approval gate, optional staging export, then digest-bound submit.
pub async fn publication_scholarly_pipeline_run(
    publication_id: &str,
    preflight_profile: vox_publisher::publication_preflight::PreflightProfile,
    dry_run: bool,
    staging_output_dir: Option<&std::path::Path>,
    venue: Option<ScholarlyVenueCli>,
    adapter: Option<&str>,
    json_output: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let report =
        publication_preflight_report_for_row(&db, &row, &manifest, preflight_profile, false)
            .await?;
    if !report.ok {
        anyhow::bail!(
            "scholarly pipeline preflight failed (readiness {}):\n{}",
            report.readiness_score,
            serde_json::to_string_pretty(&report)?
        );
    }
    let digest = row.content_sha3_256.clone();
    let dual = db
        .has_dual_publication_approval_for_digest(publication_id, &digest)
        .await?;
    if !dual {
        anyhow::bail!(
            "scholarly pipeline requires two distinct digest-bound approvers before staging export / submit"
        );
    }
    let mut stages: Vec<String> = vec!["preflight_ok".into(), "dual_approval_ok".into()];

    match (venue, staging_output_dir) {
        (Some(v), Some(out)) => {
            if dry_run {
                stages.push(format!(
                    "staging_skipped_dry_run venue={} dir={}",
                    v.to_venue().as_str(),
                    out.display()
                ));
            } else {
                publication_scholarly_staging_export(publication_id, out, v.to_venue()).await?;
                stages.push("staging_exported".into());
            }
        }
        (None, Some(_)) => {
            anyhow::bail!("--staging-output-dir requires --venue");
        }
        (Some(_), None) => {
            anyhow::bail!("--venue requires --staging-output-dir (or omit both)");
        }
        (None, None) => {}
    }

    if dry_run {
        let doc = serde_json::json!({
            "dry_run": true,
            "publication_id": publication_id,
            "digest": digest,
            "stages": stages,
            "preflight_report": report,
        });
        if json_output {
            println!("{}", serde_json::to_string(&doc)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        return Ok(());
    }

    let receipt = vox_publisher::scholarly_external_jobs::publication_scholarly_submit_with_ledger(
        &db,
        publication_id,
        adapter,
    )
    .await?;
    let doc = serde_json::json!({
        "pipeline_completed": true,
        "publication_id": publication_id,
        "digest": digest,
        "stages": stages,
        "submission": {
            "adapter": receipt.adapter,
            "external_submission_id": receipt.external_submission_id,
            "status": receipt.status,
        }
    });
    if json_output {
        println!("{}", serde_json::to_string(&doc)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&doc)?);
    }
    Ok(())
}

/// Record one digest-bound publication approval.
pub async fn publication_approve(publication_id: &str, approver: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(manifest) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let approver = approver.trim();
    if approver.is_empty() {
        anyhow::bail!("approver must not be empty");
    }
    db.record_publication_approval_for_digest(publication_id, &manifest.content_sha3_256, approver)
        .await?;
    let count = db
        .count_publication_approvers_for_digest(publication_id, &manifest.content_sha3_256)
        .await?;
    if count >= 2 {
        db.set_publication_state(publication_id, "approved", None)
            .await?;
    }
    println!(
        "Recorded approval for '{}' digest={} distinct_approvers={}",
        publication_id, manifest.content_sha3_256, count
    );
    Ok(())
}

/// Submit to the scholarly adapter (`--adapter` or `VOX_SCHOLARLY_ADAPTER`; default `local_ledger`).
pub async fn publication_submit_local(publication_id: &str, adapter: Option<&str>) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let receipt = vox_publisher::scholarly_external_jobs::publication_scholarly_submit_with_ledger(
        &db,
        publication_id,
        adapter,
    )
    .await?;
    println!(
        "Submitted '{}' via {} as {} ({})",
        publication_id, receipt.adapter, receipt.external_submission_id, receipt.status
    );
    Ok(())
}

/// Show publication state and scholarly submission rows.
pub async fn publication_status(publication_id: &str, with_worthiness: bool) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = publication_manifest_from_row(&row);
    let preflight_profile = vox_publisher::publication_preflight::PreflightProfile::Default;
    let preflight_report = publication_preflight_report_for_row(
        &db,
        &row,
        &manifest,
        preflight_profile,
        with_worthiness,
    )
    .await?;
    let operator_status_surface_v1 =
        vox_publisher::publication_preflight::operator_status_surface_v1(
            publication_id,
            preflight_profile,
            &preflight_report,
        );
    let approvals = db
        .count_publication_approvers_for_digest(publication_id, &row.content_sha3_256)
        .await?;
    let submissions = db.list_scholarly_submissions(publication_id).await?;
    let media_assets = db.list_publication_media_assets(publication_id).await?;
    let attempts = db.list_publication_attempts(publication_id).await?;
    let status_events = db.list_publication_status_events(publication_id).await?;
    let evidence =
        vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref());
    let evidence_fallback = vox_publisher::scientia_evidence::ScientiaEvidenceContext::default();
    let evidence_ref = evidence.as_ref().unwrap_or(&evidence_fallback);
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let discovery_rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        publication_id,
        row.source_ref.as_deref(),
        evidence_ref,
        &scientia_h,
        None,
    );
    let manifest_completion =
        vox_publisher::scientia_discovery::manifest_completion_report(&manifest);
    let evidence_complete = Some(
        vox_publisher::scientia_discovery::evidence_completeness_score(evidence_ref, &scientia_h),
    );
    let transform_preview = vox_publisher::scientia_discovery::destination_transform_previews(
        &manifest,
        evidence.as_ref(),
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": row.publication_id,
            "content_type": row.content_type,
            "state": row.state,
            "digest": row.content_sha3_256,
            "version": row.version,
            "approvals_for_digest": approvals,
            "preflight_report": preflight_report,
            "operator_status_surface_v1": operator_status_surface_v1,
            "discovery_rank": discovery_rank,
            "manifest_completion": manifest_completion,
            "evidence_completeness_0_100": evidence_complete,
            "transform_preview": transform_preview,
            "scholarly_submissions": submissions,
            "media_assets": media_assets,
            "publication_attempts": attempts,
            "publication_status_events": status_events,
        }))?
    );
    Ok(())
}

/// Rank publication manifests for SCIENTIA discovery (deterministic; no LLM).
pub async fn publication_discovery_scan(
    content_type: Option<&str>,
    state: Option<&str>,
    limit: i64,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let rows = db
        .list_publication_manifests(content_type, state, limit)
        .await?;
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let mut candidates: Vec<serde_json::Value> = Vec::new();
    for row in rows {
        let evidence =
            vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref())
                .unwrap_or_default();
        let rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
            row.publication_id.as_str(),
            row.source_ref.as_deref(),
            &evidence,
            &scientia_h,
            None,
        );
        candidates.push(serde_json::json!({
            "publication_id": row.publication_id,
            "content_type": row.content_type,
            "state": row.state,
            "updated_at_ms": row.updated_at_ms,
            "rank": rank,
        }));
    }
    candidates.sort_by(|a, b| {
        let sa = a["rank"]["rank_score"].as_u64().unwrap_or(0);
        let sb = b["rank"]["rank_score"].as_u64().unwrap_or(0);
        sb.cmp(&sa)
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_kind": "scientia_discovery_scan",
            "candidates": candidates,
        }))?
    );
    Ok(())
}

/// Machine explanation + completion + previews for one publication id.
pub async fn publication_discovery_explain(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = publication_manifest_from_row(&row);
    let evidence =
        vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref())
            .unwrap_or_default();
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let novelty_bundle = vox_publisher::scientia_prior_art::parse_novelty_bundle_from_metadata_json(
        row.metadata_json.as_deref(),
    );
    let overlap_for_rank = novelty_bundle.as_ref().map(|b| {
        vox_publisher::scientia_finding_ledger::novelty_overlap_blend_01(b, &scientia_h) as f32
    });
    let mut rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        publication_id,
        row.source_ref.as_deref(),
        &evidence,
        &scientia_h,
        overlap_for_rank,
    );
    if let Some(ref b) = novelty_bundle {
        vox_publisher::scientia_discovery::merge_novelty_overlap_into_rank(
            &mut rank,
            b,
            &scientia_h,
        );
    }
    let completion = vox_publisher::scientia_discovery::manifest_completion_report(&manifest);
    let preview = vox_publisher::scientia_discovery::destination_transform_previews(
        &manifest,
        Some(&evidence),
    );
    let impact_readership_projection = novelty_bundle.as_ref().map(|b| {
        vox_publisher::scientia_finding_ledger::impact_readership_projection_v1(b, &scientia_h)
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": publication_id,
            "discovery_rank": rank,
            "novelty_evidence_bundle": novelty_bundle,
            "manifest_completion": completion,
            "evidence_completeness_0_100": vox_publisher::scientia_discovery::evidence_completeness_score(&evidence, &scientia_h),
            "transform_preview": preview,
            "impact_readership_projection": impact_readership_projection,
        }))?
    );
    Ok(())
}

/// Destination transform preview JSON only (no DB writes).
pub async fn publication_transform_preview(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = publication_manifest_from_row(&row);
    let evidence =
        vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref());
    let preview = vox_publisher::scientia_discovery::destination_transform_previews(
        &manifest,
        evidence.as_ref(),
    );
    println!("{}", serde_json::to_string_pretty(&preview)?);
    Ok(())
}

fn merge_novelty_bundle_into_metadata_json_str(
    metadata_json: Option<&str>,
    bundle: &vox_publisher::scientia_finding_ledger::NoveltyEvidenceBundleV1,
) -> Result<String> {
    let mut root: serde_json::Value =
        if let Some(raw) = metadata_json.map(str::trim).filter(|s| !s.is_empty()) {
            serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}))
        } else {
            serde_json::json!({})
        };
    root[vox_publisher::scientia_evidence::METADATA_KEY_SCIENTIA_NOVELTY_BUNDLE] =
        serde_json::to_value(bundle).context("novelty bundle serde")?;
    Ok(serde_json::to_string(&root)?)
}

/// Fetch OpenAlex / Crossref / Semantic Scholar prior art for a stored manifest; optional `--persist-metadata` merges `scientia_novelty_bundle` and recomputes digest.
pub async fn publication_novelty_fetch(
    publication_id: &str,
    offline: bool,
    persist_metadata: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    if row.content_type != "scientia" {
        anyhow::bail!(
            "publication-novelty-fetch is intended for content_type `scientia` (got `{}`)",
            row.content_type
        );
    }
    let candidate_id = vox_publisher::scientia_finding_ledger::default_candidate_id(publication_id);
    let query = vox_publisher::scientia_prior_art::PriorArtQuery {
        title: row.title.clone(),
        abstract_text: row.abstract_text.clone(),
    };
    let client = vox_reqwest_defaults::client();
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let bundle = vox_publisher::scientia_prior_art::fetch_prior_art_federated(
        &client,
        &candidate_id,
        &query,
        vec![],
        vox_publisher::scientia_prior_art::PriorArtFetchOptions::default(),
        offline,
        &scientia_h,
    )
    .await
    .context("prior-art federated fetch")?;

    if persist_metadata {
        let mut manifest = publication_manifest_from_row(&row);
        manifest.metadata_json = Some(merge_novelty_bundle_into_metadata_json_str(
            manifest.metadata_json.as_deref(),
            &bundle,
        )?);
        let digest = manifest.content_sha3_256();
        db.upsert_publication_manifest(vox_db::PublicationManifestParams {
            publication_id: &manifest.publication_id,
            content_type: &manifest.content_type,
            source_ref: manifest.source_ref.as_deref(),
            title: &manifest.title,
            author: &manifest.author,
            abstract_text: manifest.abstract_text.as_deref(),
            body_markdown: &manifest.body_markdown,
            citations_json: manifest.citations_json.as_deref(),
            metadata_json: manifest.metadata_json.as_deref(),
            revision_history_json: row.revision_history_json.as_deref(),
            content_sha3_256: &digest,
            state: row.state.as_str(),
        })
        .await?;
        db.append_publication_status_event(
            publication_id,
            "scientia_novelty_bundle_updated",
            Some(
                &serde_json::json!({ "bundle_id": bundle.bundle_id, "digest": digest }).to_string(),
            ),
        )
        .await?;
    }

    println!("{}", serde_json::to_string_pretty(&bundle)?);
    Ok(())
}

/// Preflight + worthiness + discovery rank with optional live prior-art refresh (stdout JSON).
pub async fn publication_decision_explain(
    publication_id: &str,
    live_prior_art: bool,
    offline: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let mut manifest = publication_manifest_from_row(&row);
    if live_prior_art {
        if manifest.content_type != "scientia" {
            anyhow::bail!("--live-prior-art requires content_type `scientia`");
        }
        let candidate_id =
            vox_publisher::scientia_finding_ledger::default_candidate_id(publication_id);
        let query = vox_publisher::scientia_prior_art::PriorArtQuery {
            title: manifest.title.clone(),
            abstract_text: manifest.abstract_text.clone(),
        };
        let client = vox_reqwest_defaults::client();
        let bundle = vox_publisher::scientia_prior_art::fetch_prior_art_federated(
            &client,
            &candidate_id,
            &query,
            vec![],
            vox_publisher::scientia_prior_art::PriorArtFetchOptions::default(),
            offline,
            &scientia_h,
        )
        .await?;
        manifest.metadata_json = Some(merge_novelty_bundle_into_metadata_json_str(
            manifest.metadata_json.as_deref(),
            &bundle,
        )?);
    }
    manifest =
        crate::commands::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
            manifest, &db, &repo_root, None,
        )
        .await?;

    let contract_yaml = read_utf8_path_capped(
        &repo_root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH),
    )?;
    let contract = vox_publisher::publication_worthiness::load_contract_from_str(&contract_yaml)
        .context("worthiness yaml")?;
    vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;

    let report = vox_publisher::publication_preflight::run_preflight_with_worthiness_heuristics(
        &manifest,
        vox_publisher::publication_preflight::PreflightProfile::Default,
        &contract,
        &scientia_h,
    );
    let evidence = vox_publisher::scientia_evidence::parse_scientia_evidence(
        manifest.metadata_json.as_deref(),
    )
    .unwrap_or_default();
    let novelty_bundle = vox_publisher::scientia_prior_art::parse_novelty_bundle_from_metadata_json(
        manifest.metadata_json.as_deref(),
    );
    let overlap_for_rank = novelty_bundle.as_ref().map(|b| {
        vox_publisher::scientia_finding_ledger::novelty_overlap_blend_01(b, &scientia_h) as f32
    });
    let mut rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        publication_id,
        manifest.source_ref.as_deref(),
        &evidence,
        &scientia_h,
        overlap_for_rank,
    );
    if let Some(ref b) = novelty_bundle {
        vox_publisher::scientia_discovery::merge_novelty_overlap_into_rank(
            &mut rank,
            b,
            &scientia_h,
        );
    }
    let impact_readership_projection =
        vox_publisher::scientia_prior_art::parse_novelty_bundle_from_metadata_json(
            manifest.metadata_json.as_deref(),
        )
        .map(|b| {
            vox_publisher::scientia_finding_ledger::impact_readership_projection_v1(&b, &scientia_h)
        });

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": publication_id,
            "preflight_readiness_score": report.readiness_score,
            "worthiness": report.worthiness,
            "discovery_rank": rank,
            "preflight_findings": report.findings,
            "impact_readership_projection": impact_readership_projection,
        }))?
    );
    Ok(())
}

/// Prior-art fetch + finding-candidate ledger row + decision snapshot (stdout JSON; does not persist unless `publication-novelty-fetch --persist-metadata` is used separately).
pub async fn publication_novelty_happy_path(publication_id: &str, offline: bool) -> Result<()> {
    let t0 = Instant::now();
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    if row.content_type != "scientia" {
        anyhow::bail!("publication-novelty-happy-path requires content_type `scientia`");
    }
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    let scientia_h =
        vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&repo_root);
    let candidate_id = vox_publisher::scientia_finding_ledger::default_candidate_id(publication_id);
    let query = vox_publisher::scientia_prior_art::PriorArtQuery {
        title: row.title.clone(),
        abstract_text: row.abstract_text.clone(),
    };
    let client = vox_reqwest_defaults::client();
    let bundle = vox_publisher::scientia_prior_art::fetch_prior_art_federated(
        &client,
        &candidate_id,
        &query,
        vec![],
        vox_publisher::scientia_prior_art::PriorArtFetchOptions::default(),
        offline,
        &scientia_h,
    )
    .await?;

    let mut manifest = publication_manifest_from_row(&row);
    manifest.metadata_json = Some(merge_novelty_bundle_into_metadata_json_str(
        manifest.metadata_json.as_deref(),
        &bundle,
    )?);
    manifest =
        crate::commands::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
            manifest, &db, &repo_root, None,
        )
        .await?;

    let evidence = vox_publisher::scientia_evidence::parse_scientia_evidence(
        manifest.metadata_json.as_deref(),
    )
    .unwrap_or_default();
    let signals = if evidence.discovery_signals.is_empty() {
        vox_publisher::scientia_evidence::infer_discovery_signals(
            manifest.source_ref.as_deref(),
            &evidence,
        )
    } else {
        evidence.discovery_signals.clone()
    };
    let overlap_for_rank =
        vox_publisher::scientia_finding_ledger::novelty_overlap_blend_01(&bundle, &scientia_h)
            as f32;
    let mut rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        publication_id,
        manifest.source_ref.as_deref(),
        &evidence,
        &scientia_h,
        Some(overlap_for_rank),
    );
    vox_publisher::scientia_discovery::merge_novelty_overlap_into_rank(
        &mut rank,
        &bundle,
        &scientia_h,
    );
    let now = vox_publisher::scientia_prior_art::now_unix_ms_strict();
    let mut candidate = vox_publisher::scientia_finding_ledger::build_finding_candidate(
        Some(publication_id),
        Some(row.title.clone()),
        signals,
        publication_id,
        rank.strong_signal_count,
        rank.supporting_signal_count,
        rank.informational_signal_count,
        rank.rank_score,
        rank.intake_tier == vox_publisher::scientia_discovery::DiscoveryIntakeTier::LowSignal,
        !rank.conflicts.is_empty(),
        now,
        &scientia_h,
    );
    candidate.novelty_evidence_bundle_id = Some(bundle.bundle_id.clone());

    let contract_yaml = read_utf8_path_capped(
        &repo_root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH),
    )?;
    let contract = vox_publisher::publication_worthiness::load_contract_from_str(&contract_yaml)?;
    vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;
    let report = vox_publisher::publication_preflight::run_preflight_with_worthiness_heuristics(
        &manifest,
        vox_publisher::publication_preflight::PreflightProfile::Default,
        &contract,
        &scientia_h,
    );

    let decision_latency_ms = t0.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    let (worthiness_decision, worthiness_score, hard_metrics_ok) = match report.worthiness.as_ref()
    {
        Some(w) => (
            serde_json::to_value(&w.decision)
                .ok()
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s),
                    _ => None,
                })
                .unwrap_or_else(|| "unknown".to_string()),
            w.worthiness_score,
            w.hard_metrics_ok,
        ),
        None => ("unknown".to_string(), 0.0, false),
    };
    let calibration = vox_publisher::scientia_finding_ledger::novelty_decision_calibration_v1(
        publication_id,
        &candidate_id,
        &bundle,
        decision_latency_ms,
        offline,
        &worthiness_decision,
        worthiness_score,
        hard_metrics_ok,
        rank.prior_art_max_lexical_overlap,
    );
    let impact_readership_projection =
        vox_publisher::scientia_finding_ledger::impact_readership_projection_v1(
            &bundle,
            &scientia_h,
        );

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_kind": "scientia_novelty_happy_path",
            "finding_candidate": candidate,
            "novelty_evidence_bundle": bundle,
            "discovery_rank": rank,
            "worthiness": report.worthiness,
            "preflight_readiness_score": report.readiness_score,
            "calibration_telemetry": calibration,
            "impact_readership_projection": impact_readership_projection,
        }))?
    );
    Ok(())
}

/// Poll the remote scholarly repository for the latest stored submission (or one matching `external_submission_id`).
pub async fn publication_scholarly_remote_status(
    publication_id: &str,
    external_submission_id: Option<&str>,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let submissions = db.list_scholarly_submissions(publication_id).await?;
    let sub_row: &vox_db::ScholarlySubmissionRow = match external_submission_id {
        Some(e) => {
            let e = e.trim();
            if e.is_empty() {
                anyhow::bail!("--external-submission-id must not be empty when provided");
            }
            submissions
                .iter()
                .find(|r| r.external_submission_id == e)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no scholarly submission for publication {publication_id} with external_submission_id {e}"
                    )
                })?
        }
        None => submissions.first().ok_or_else(|| {
            anyhow::anyhow!("no scholarly submissions for publication {publication_id}")
        })?,
    };
    let v = vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_persist(
        &db,
        publication_id,
        sub_row,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

/// Poll remote status for **every** `scholarly_submissions` row for this publication (continues on per-row errors).
pub async fn publication_scholarly_remote_status_sync_all(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let v = vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_all_submissions_for_publication(
        &db,
        publication_id,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

/// Batch remote status poll across publications (distinct ids by recent `scholarly_submissions` activity). For cron/operators.
pub async fn publication_scholarly_remote_status_sync_batch(
    limit: i64,
    iterations: u32,
    interval_secs: u64,
    max_runtime_secs: Option<u64>,
    jitter_secs: u64,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let v = if iterations <= 1
        && interval_secs == 0
        && max_runtime_secs.is_none()
        && jitter_secs == 0
    {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch(&db, limit).await
    } else {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch_loop(
            &db,
            limit,
            iterations,
            interval_secs,
            max_runtime_secs,
            jitter_secs,
        )
        .await
    }
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

/// Record an operator milestone for the arXiv-assist workflow (append-only audit in `publication_status_events`).
pub async fn publication_arxiv_handoff_record(
    publication_id: &str,
    stage: ArxivHandoffStageCli,
    operator: Option<&str>,
    note: Option<&str>,
    arxiv_id: Option<&str>,
) -> Result<()> {
    let publication_id = publication_id.trim();
    if publication_id.is_empty() {
        anyhow::bail!("publication_id must not be empty");
    }
    if matches!(stage, ArxivHandoffStageCli::Published)
        && arxiv_id.map(str::trim).filter(|s| !s.is_empty()).is_none()
    {
        anyhow::bail!("--arxiv-id is required when --stage published");
    }
    let db = vox_db::VoxDb::connect_default().await?;
    if db.get_publication_manifest(publication_id).await?.is_none() {
        anyhow::bail!("publication not found: {publication_id}");
    }
    let status = format!("arxiv_handoff:{}", stage.slug());
    let op_trim = operator.map(str::trim).filter(|s| !s.is_empty());
    let note_trim = note.map(str::trim).filter(|s| !s.is_empty());
    let arxiv_trim = arxiv_id.map(str::trim).filter(|s| !s.is_empty());
    let detail = serde_json::json!({
        "schema_version": 1_u32,
        "workflow": "arxiv_operator_assist",
        "stage": stage.slug(),
        "operator": op_trim,
        "note": note_trim,
        "arxiv_id": arxiv_trim,
    });
    db.append_publication_status_event(publication_id, &status, Some(&detail.to_string()))
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "recorded": true,
            "publication_id": publication_id,
            "status": status,
            "detail": detail,
        }))?
    );
    Ok(())
}

/// Read-only metrics rollup for the scholarly external pipeline and related publication attempt channels.
pub async fn publication_external_pipeline_metrics(since_hours: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let hours = since_hours.clamp(0, 8_760);
    let since_ms = if hours == 0 {
        0_i64
    } else {
        now_ms.saturating_sub(hours.saturating_mul(3_600_000))
    };
    let v = db
        .summarize_scholarly_external_pipeline_metrics(since_ms)
        .await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

/// Operator view of scholarly outbound jobs eligible for a retry worker (`queued` / due `retryable_failed`).
pub async fn publication_external_jobs_due(limit: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let before_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let jobs = db
        .list_external_submission_jobs_due(before_ms, limit)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "due_before_ms_inclusive": before_ms,
            "jobs": jobs,
        }))?
    );
    Ok(())
}

/// List `external_submission_jobs` in terminal **`failed`** state (not scheduled for retry).
pub async fn publication_external_jobs_dead_letter(limit: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let jobs = db.list_external_submission_jobs_failed(limit).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "jobs": jobs }))?
    );
    Ok(())
}

/// Requeue one dead-letter job (`status = failed`) to `queued` for the next `publication-external-jobs-tick`.
pub async fn publication_external_jobs_replay(job_id: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let row = db
        .replay_failed_external_submission_job_to_queued(job_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "replayed": true,
            "job": row,
        }))?
    );
    Ok(())
}

/// Process one batch of due `external_submission_jobs`: preflight, lease, scholarly `submit` using the job's adapter.
pub async fn publication_external_jobs_tick(
    limit: i64,
    lock_ttl_ms: i64,
    lock_owner: Option<&str>,
    iterations: u32,
    interval_secs: u64,
    max_runtime_secs: Option<u64>,
    jitter_secs: u64,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    if iterations <= 1 && interval_secs == 0 && max_runtime_secs.is_none() && jitter_secs == 0 {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let out = vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick(
            &db,
            limit,
            lock_ttl_ms,
            lock_owner,
            now_ms,
        )
        .await?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "now_ms": now_ms,
                "lock_owner": out.lock_owner,
                "lock_ttl_ms": out.lock_ttl_ms,
                "results": out.results,
            }))?
        );
        return Ok(());
    }
    let v = vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick_loop(
        &db,
        limit,
        lock_ttl_ms,
        lock_owner,
        iterations,
        interval_secs,
        max_runtime_secs,
        jitter_secs,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

/// Upsert one publication media asset row.
pub async fn publication_media_upsert(
    publication_id: &str,
    asset_ref: &str,
    media_type: &str,
    storage_uri: Option<&str>,
    status: &str,
    metadata_json_path: Option<&PathBuf>,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let metadata_json = if let Some(path) = metadata_json_path {
        Some(
            read_utf8_path_capped(path)
                .with_context(|| format!("failed to read metadata JSON from {}", path.display()))?,
        )
    } else {
        None
    };
    db.upsert_publication_media_asset(vox_db::PublicationMediaAssetParams {
        publication_id,
        asset_ref,
        media_type,
        storage_uri,
        status,
        metadata_json: metadata_json.as_deref(),
    })
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": publication_id,
            "asset_ref": asset_ref,
            "media_type": media_type,
            "storage_uri": storage_uri,
            "status": status,
            "metadata_json_present": metadata_json.is_some()
        }))?
    );
    Ok(())
}

/// List publication media assets for one publication id.
pub async fn publication_media_list(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let rows = db.list_publication_media_assets(publication_id).await?;
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

/// Delete one publication media asset by `publication_id + asset_ref`.
pub async fn publication_media_delete(publication_id: &str, asset_ref: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    db.delete_publication_media_asset(publication_id, asset_ref)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "deleted": true,
            "publication_id": publication_id,
            "asset_ref": asset_ref
        }))?
    );
    Ok(())
}

pub(crate) fn publication_item_from_manifest(
    row: &vox_db::PublicationManifestRow,
) -> Result<vox_publisher::types::UnifiedNewsItem> {
    vox_publisher::switching::unified_news_item_from_manifest_parts(
        &row.publication_id,
        &row.title,
        &row.author,
        &row.body_markdown,
        row.metadata_json.as_deref(),
    )
}

fn publication_manifest_from_row(
    row: &vox_db::PublicationManifestRow,
) -> vox_publisher::publication::PublicationManifest {
    vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    }
}

async fn publication_attention_inputs_for_row(
    db: &vox_db::VoxDb,
    row: &vox_db::PublicationManifestRow,
    item: &vox_publisher::types::UnifiedNewsItem,
) -> Result<vox_publisher::publication_preflight::PreflightAttentionInputs> {
    let dual = db
        .has_dual_publication_approval_for_digest(
            row.publication_id.as_str(),
            row.content_sha3_256.as_str(),
        )
        .await?;
    let gate = vox_publisher::gate::evaluate_publish_gate(
        vox_publisher::gate::publish_gate_inputs_for_cli(false, true, dual, item),
    );
    Ok(vox_publisher::publication_preflight::PreflightAttentionInputs { gate: Some(gate) })
}

async fn publication_preflight_report_for_row(
    db: &vox_db::VoxDb,
    row: &vox_db::PublicationManifestRow,
    manifest: &vox_publisher::publication::PublicationManifest,
    profile: vox_publisher::publication_preflight::PreflightProfile,
    with_worthiness: bool,
) -> Result<vox_publisher::publication_preflight::PreflightReport> {
    let item = publication_item_from_manifest(row)?;
    let attention = publication_attention_inputs_for_row(db, row, &item).await?;
    if with_worthiness {
        let root = vox_repository::resolve_repo_root_for_ci();
        let manifest =
            crate::commands::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
                manifest.clone(),
                db,
                &root,
                None,
            )
            .await?;
        let contract_path =
            root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH);
        let yaml = read_utf8_path_capped(&contract_path).with_context(|| {
            format!(
                "read worthiness contract {} (repo root discovery required)",
                contract_path.display()
            )
        })?;
        let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml)?;
        vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;
        let scientia_h =
            vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(&root);
        Ok(
            vox_publisher::publication_preflight::run_preflight_with_worthiness_attention_heuristics(
                &manifest,
                profile,
                &contract,
                Some(attention),
                &scientia_h,
            ),
        )
    } else {
        Ok(
            vox_publisher::publication_preflight::run_preflight_with_attention(
                manifest,
                profile,
                Some(attention),
            ),
        )
    }
}

fn cli_social_worthiness_enforce() -> bool {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSocialWorthinessEnforce).expose()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn cli_social_worthiness_score_min() -> f64 {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSocialWorthinessScoreMin).expose()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.85)
}

fn publisher_config_from_env(
    dry_run: bool,
    worthiness_score: Option<f64>,
) -> vox_publisher::PublisherConfig {
    let mut cfg = vox_publisher::PublisherConfig::from_operator_environment(
        dry_run,
        Some(vox_repository::resolve_repo_root_for_ci()),
        vox_publisher::NewsSiteConfig::from_default_with_operator_env(),
    );
    cfg.worthiness_score = worthiness_score;
    cfg
}

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

/// Simulate per-channel routing/policy outcomes for one prepared publication id.
///
/// When `json` is true, prints one line of compact JSON (stable key order from `serde_json`).
pub async fn publication_route_simulate(publication_id: &str, json: bool) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let result = publication_route_simulate_with_db(&db, publication_id).await?;
    if json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
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
