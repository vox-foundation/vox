//! Merge live Socrates telemetry from VoxDb into `metadata_json.scientia_evidence` for integrated worthiness runs.

use std::path::Path;

use anyhow::Context;
use anyhow::Result;

use vox_publisher::publication::PublicationManifest;
use vox_publisher::scientia_evidence::METADATA_KEY_SCIENTIA_EVIDENCE;

fn extract_repository_id(manifest: &PublicationManifest) -> Option<String> {
    let raw = manifest.metadata_json.as_deref()?;
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    v.get("repository_id")
        .and_then(|x| x.as_str())
        .map(std::string::ToString::to_string)
}

/// When `metadata_json.scientia_evidence.socrates_aggregate` is missing or empty, fill from `socrates_surface` rows.
pub async fn merge_live_socrates_aggregate(
    manifest: PublicationManifest,
    db: &vox_db::VoxDb,
    repository_id_fallback: Option<&str>,
) -> Result<PublicationManifest> {
    let rid = extract_repository_id(&manifest)
        .or_else(|| repository_id_fallback.map(std::string::ToString::to_string));
    let Some(repository_id) = rid else {
        return Ok(manifest);
    };
    let merged = db
        .merge_scientia_live_socrates_into_metadata_json(
            manifest.metadata_json.as_deref(),
            repository_id.as_str(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut out = manifest;
    out.metadata_json = Some(merged);
    Ok(out)
}

#[cfg(any(feature = "mens-base", feature = "gpu"))]
fn merge_eval_gate_from_run_dir(
    mut manifest: PublicationManifest,
    repo_root: &Path,
) -> Result<PublicationManifest> {
    let Some(ref meta_raw) = manifest.metadata_json else {
        return Ok(manifest);
    };
    let trimmed = meta_raw.trim();
    if trimmed.is_empty() {
        return Ok(manifest);
    }
    let mut root: serde_json::Value = serde_json::from_str(trimmed)
        .with_context(|| "parse metadata_json for eval_gate run_dir merge")?;

    let mut ev: vox_publisher::scientia_evidence::ScientiaEvidenceContext = root
        .get(METADATA_KEY_SCIENTIA_EVIDENCE)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    if ev.eval_gate.is_some() {
        return Ok(manifest);
    }

    let Some(ref run_rel) = ev.eval_gate_run_dir_repo_relative else {
        return Ok(manifest);
    };
    let run_part = run_rel
        .trim()
        .trim_start_matches('/')
        .trim_start_matches('\\');
    if run_part.is_empty() {
        return Ok(manifest);
    }

    let run_dir = repo_root.join(run_part);
    let policy_path = repo_root.join("mens/config/eval-gates.yaml");
    if !run_dir.is_dir() || !policy_path.is_file() {
        return Ok(manifest);
    }

    let results = crate::commands::mens::eval_gate::check_run(&run_dir, &policy_path)?;
    let gates_total = results.len();
    let gates_failed = results.iter().filter(|r| !r.passed).count();
    let passed = !results.iter().any(|r| !r.passed && r.block);
    ev.eval_gate = Some(vox_publisher::scientia_evidence::EvalGateSnapshot {
        passed,
        gates_failed,
        gates_total,
    });

    root[METADATA_KEY_SCIENTIA_EVIDENCE] = serde_json::to_value(&ev)?;
    manifest.metadata_json = Some(serde_json::to_string(&root)?);
    Ok(manifest)
}

#[cfg(not(any(feature = "mens-base", feature = "gpu")))]
fn merge_eval_gate_from_run_dir(
    manifest: PublicationManifest,
    _repo_root: &Path,
) -> Result<PublicationManifest> {
    Ok(manifest)
}

/// Live Socrates rollup, eval-gate directory checks (`eval_gate_run_dir_repo_relative`), then JSON sidecar hydration.
pub async fn enrich_manifest_for_worthiness_preflight(
    manifest: PublicationManifest,
    db: &vox_db::VoxDb,
    repo_root: &Path,
    repository_id_fallback: Option<&str>,
) -> Result<PublicationManifest> {
    let mut m = merge_live_socrates_aggregate(manifest, db, repository_id_fallback).await?;
    m = merge_eval_gate_from_run_dir(m, repo_root)?;
    if let Some(updated) = vox_publisher::scientia_evidence::enrich_metadata_json_with_repo_files(
        m.metadata_json.as_deref(),
        repo_root,
    )? {
        m.metadata_json = Some(updated);
    }
    Ok(m)
}
