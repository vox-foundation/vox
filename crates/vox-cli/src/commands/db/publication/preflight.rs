use super::*;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

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
pub(super) fn resolve_under_repo(root: &Path, p: &Path) -> PathBuf {
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
