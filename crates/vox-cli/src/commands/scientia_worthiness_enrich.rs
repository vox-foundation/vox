//! Merge live Socrates telemetry from VoxDb into `metadata_json.scientia_evidence` for integrated worthiness runs.

use std::path::Path;

#[allow(unused_imports)]
use anyhow::{Context, Result};

use vox_publisher::publication::PublicationManifest;
#[allow(unused_imports)]
use vox_publisher::scientia_evidence::METADATA_KEY_SCIENTIA_EVIDENCE;

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

    use clap::Parser;
    use std::process::Command;
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
    let mut m = vox_publisher::scientia_worthiness_enrich::enrich_manifest_socrates_and_sidecars(
        manifest,
        db,
        repo_root,
        repository_id_fallback,
    )
    .await?;
    m = merge_eval_gate_from_run_dir(m, repo_root)?;
    Ok(m)
}
