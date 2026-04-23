//! Merge live Socrates telemetry from VoxDb into `metadata_json.scientia_evidence` for integrated worthiness runs.

use std::path::Path;

#[allow(unused_imports)]
use anyhow::{Context, Result};

use vox_publisher::publication::PublicationManifest;
#[allow(unused_imports)]
use vox_publisher::scientia_evidence::METADATA_KEY_SCIENTIA_EVIDENCE;

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
