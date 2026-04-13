use super::*;
use anyhow::{Context, Result};
use std::path::Path;

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
