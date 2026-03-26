//! Publication manifest and syndication helpers for `vox db publication-*`.

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::commands::db_cli::{ArxivHandoffStageCli, ScholarlyVenueCli};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Prepare (upsert) a canonical publication manifest from markdown body content.
pub async fn publication_prepare(
    publication_id: &str,
    content_type: &str,
    author: &str,
    title: &str,
    path: &Path,
    abstract_text: Option<&str>,
    citations_json_path: Option<&Path>,
    scholarly_metadata_json_path: Option<&Path>,
    preflight: bool,
    preflight_profile: vox_publisher::publication_preflight::PreflightProfile,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let body_markdown = read_utf8_path_capped(path)
        .with_context(|| format!("failed to read markdown body from {}", path.display()))?;
    let citations_json = if let Some(p) = citations_json_path {
        Some(
            read_utf8_path_capped(p)
                .with_context(|| format!("failed to read citations JSON from {}", p.display()))?,
        )
    } else {
        None
    };
    let scientific = if let Some(p) = scholarly_metadata_json_path {
        let raw = read_utf8_path_capped(p).with_context(|| {
            format!(
                "failed to read scholarly metadata JSON from {}",
                p.display()
            )
        })?;
        Some(
            serde_json::from_str::<vox_publisher::scientific_metadata::ScientificPublicationMetadata>(
                raw.trim(),
            )
            .with_context(|| {
                format!(
                    "invalid scholarly metadata JSON (see scientific_publication schema in vox-publisher): {}",
                    p.display()
                )
            })?,
        )
    } else {
        None
    };
    let metadata_json = vox_publisher::scientific_metadata::build_scientia_metadata_json(
        "vox db publication-prepare",
        None,
        scientific.as_ref(),
        None,
    )
    .context("build publication metadata_json")?;
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: publication_id.to_string(),
        content_type: content_type.to_string(),
        source_ref: Some(path.display().to_string()),
        title: title.to_string(),
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
        content_sha3_256: &digest,
        state: "draft",
    })
    .await?;
    println!(
        "Prepared publication '{}' ({}) digest={}",
        publication_id, content_type, digest
    );
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
    let report = if with_worthiness {
        let root = vox_repository::resolve_repo_root_for_ci();
        manifest = crate::commands::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
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
        vox_publisher::publication_preflight::run_preflight_with_worthiness(
            &manifest, profile, &contract,
        )
    } else {
        vox_publisher::publication_preflight::run_preflight(&manifest, profile)
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
    let written = vox_publisher::submission_package::write_scholarly_staging(
        &manifest,
        venue,
        output_dir,
    )?;
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
    let report = vox_publisher::publication_preflight::run_preflight(&manifest, preflight_profile);
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
pub async fn publication_submit_local(
    publication_id: &str,
    adapter: Option<&str>,
) -> Result<()> {
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
pub async fn publication_status(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let approvals = db
        .count_publication_approvers_for_digest(publication_id, &row.content_sha3_256)
        .await?;
    let submissions = db.list_scholarly_submissions(publication_id).await?;
    let media_assets = db.list_publication_media_assets(publication_id).await?;
    let attempts = db.list_publication_attempts(publication_id).await?;
    let status_events = db.list_publication_status_events(publication_id).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": row.publication_id,
            "content_type": row.content_type,
            "state": row.state,
            "digest": row.content_sha3_256,
            "version": row.version,
            "approvals_for_digest": approvals,
            "scholarly_submissions": submissions,
            "media_assets": media_assets,
            "publication_attempts": attempts,
            "publication_status_events": status_events,
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
    let v = if iterations <= 1 && interval_secs == 0 && max_runtime_secs.is_none() && jitter_secs == 0
    {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch(&db, limit)
            .await
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
    if db
        .get_publication_manifest(publication_id)
        .await?
        .is_none()
    {
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
    db.append_publication_status_event(
        publication_id,
        &status,
        Some(&detail.to_string()),
    )
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

fn publication_manifest_from_row(row: &vox_db::PublicationManifestRow) -> vox_publisher::publication::PublicationManifest {
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

fn cli_social_worthiness_enforce() -> bool {
    std::env::var("VOX_SOCIAL_WORTHINESS_ENFORCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn cli_social_worthiness_score_min() -> f64 {
    std::env::var("VOX_SOCIAL_WORTHINESS_SCORE_MIN")
        .ok()
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
                    Some(
                        &serde_json::json!({ "channel_group": "manual_cli" }).to_string(),
                    ),
                )
                .await;
        } else if result.has_failures() {
            let _ = db
                .set_publication_state(
                    publication_id,
                    "publish_failed",
                    Some(
                        &serde_json::json!({ "channel_group": "manual_cli" }).to_string(),
                    ),
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
    if let Some(ch) = channel {
        return publication_publish(publication_id, Some(ch), dry_run, json).await;
    }
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
    let Some(failed) = vox_publisher::switching::failed_channels_from_latest_digest_attempt(
        attempt_refs.as_slice(),
        digest,
    )?
    else {
        anyhow::bail!(
            "no syndication attempt outcome for current manifest digest; run `vox db publication-publish` first"
        );
    };
    if failed.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"publication_id": publication_id, "retried": false, "reason": "no_failed_channels"})
            )?
        );
        return Ok(());
    }
    let csv = failed.join(",");
    publication_publish(publication_id, Some(csv.as_str()), dry_run, json).await
}
