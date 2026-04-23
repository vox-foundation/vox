use crate::commands::ci::bounded_read::read_utf8_path_capped;
use anyhow::{Context, Result};
use std::path::Path;
pub(super) fn repo_relative_string(repo_root: &Path, path: &Path) -> Result<String> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    };
    let canon = std::fs::canonicalize(&abs)
        .with_context(|| format!("resolve path under repo root: {}", abs.display()))?;
    let rel = canon.strip_prefix(repo_root).with_context(|| {
        format!(
            "path must live under repo root {}: {}",
            repo_root.display(),
            canon.display()
        )
    })?;
    Ok(rel.to_string_lossy().replace('\\', "/"))
}

pub(super) fn source_ref_string(repo_root: &Path, path: &Path) -> String {
    repo_relative_string(repo_root, path).unwrap_or_else(|_| path.display().to_string())
}

pub(super) fn repository_id_for_prepare(repo_root: &Path) -> String {
    vox_repository::compute_repository_id(repo_root, None)
}

pub(super) fn read_scientific_metadata_json(
    scholarly_metadata_json_path: Option<&Path>,
) -> Result<Option<vox_publisher::scientific_metadata::ScientificPublicationMetadata>> {
    if let Some(p) = scholarly_metadata_json_path {
        let raw = read_utf8_path_capped(p).with_context(|| {
            format!(
                "failed to read scholarly metadata JSON from {}",
                p.display()
            )
        })?;
        Ok(Some(
            serde_json::from_str::<vox_publisher::scientific_metadata::ScientificPublicationMetadata>(
                raw.trim(),
            )
            .with_context(|| {
                format!(
                    "invalid scholarly metadata JSON (see scientific_publication schema in vox-publisher): {}",
                    p.display()
                )
            })?,
        ))
    } else {
        Ok(None)
    }
}

pub(super) fn build_scientia_evidence_context(
    repo_root: &Path,
    source_ref: &str,
    abstract_text: Option<&str>,
    citations_json: Option<&str>,
    scientific: Option<&vox_publisher::scientific_metadata::ScientificPublicationMetadata>,
    eval_gate_report_json_path: Option<&Path>,
    benchmark_pair_report_json_path: Option<&Path>,
    human_meaningful_advance: bool,
    human_ai_disclosure_complete: bool,
    body_markdown: &str,
) -> Result<Option<vox_publisher::scientia_evidence::ScientiaEvidenceContext>> {
    let mut evidence = vox_publisher::scientia_evidence::ScientiaEvidenceContext {
        eval_gate_report_repo_relative: match eval_gate_report_json_path {
            Some(p) => Some(repo_relative_string(repo_root, p)?),
            None => None,
        },
        benchmark_pair_report_repo_relative: match benchmark_pair_report_json_path {
            Some(p) => Some(repo_relative_string(repo_root, p)?),
            None => None,
        },
        human_meaningful_advance,
        human_ai_disclosure_complete,
        ..Default::default()
    };
    vox_publisher::scientia_evidence::populate_candidate_context_defaults(
        Some(source_ref),
        abstract_text,
        citations_json,
        scientific,
        &mut evidence,
    );
    vox_publisher::scientia_evidence::attach_autofill_and_doc_hints(body_markdown, &mut evidence);
    if evidence.discovery_signals.is_empty()
        && evidence.eval_gate_report_repo_relative.is_none()
        && evidence.benchmark_pair_report_repo_relative.is_none()
        && !human_meaningful_advance
        && !human_ai_disclosure_complete
        && evidence.doc_section_hints.is_empty()
    {
        return Ok(None);
    }
    Ok(Some(evidence))
}
