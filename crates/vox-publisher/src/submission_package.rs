//! Venue-scoped **staging** layout for scholarly exports (sidecar files next to a submission bundle).
//!
//! This module plans which files to write and validates an on-disk tree before upload tooling runs.

use std::fs;
use std::path::Path;

use flate2::Compression;
use flate2::write::GzEncoder;

use crate::citation_cff::render_citation_cff;
use crate::crossref_metadata::crossref_work_export_json;
use crate::publication::PublicationManifest;
use crate::zenodo_metadata;

/// Target repository or assist workflow for a staging bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScholarlyVenue {
    Zenodo,
    OpenReview,
    ArxivAssist,
}

impl ScholarlyVenue {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "zenodo" => Some(Self::Zenodo),
            "openreview" => Some(Self::OpenReview),
            "arxiv" | "arxiv_assist" | "arxiv-assist" => Some(Self::ArxivAssist),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zenodo => "zenodo",
            Self::OpenReview => "openreview",
            Self::ArxivAssist => "arxiv_assist",
        }
    }
}

/// One file in a staging directory.
#[derive(Debug, Clone)]
pub struct StagingArtifact {
    pub relative_path: String,
    /// When false, the file is omitted if there is no source data (e.g. empty `citations_json`).
    pub require_non_empty_source: bool,
}

/// Files recommended for `venue` (relative to the staging root).
#[must_use]
pub fn staging_artifacts(venue: ScholarlyVenue) -> Vec<StagingArtifact> {
    let mut v = vec![
        StagingArtifact {
            relative_path: "body.md".to_string(),
            require_non_empty_source: true,
        },
        StagingArtifact {
            relative_path: "CITATION.cff".to_string(),
            require_non_empty_source: false,
        },
        StagingArtifact {
            relative_path: "crossref_work.json".to_string(),
            require_non_empty_source: false,
        },
        StagingArtifact {
            relative_path: "citations.json".to_string(),
            require_non_empty_source: false,
        },
    ];
    if matches!(venue, ScholarlyVenue::Zenodo) {
        v.push(StagingArtifact {
            relative_path: "zenodo.json".to_string(),
            require_non_empty_source: false,
        });
    }
    if matches!(venue, ScholarlyVenue::ArxivAssist) {
        v.push(StagingArtifact {
            relative_path: "arxiv_handoff.json".to_string(),
            require_non_empty_source: false,
        });
        v.push(StagingArtifact {
            relative_path: "arxiv_bundle.tar.gz".to_string(),
            require_non_empty_source: false,
        });
    }
    v
}

#[must_use]
pub fn arxiv_operator_handoff_value(manifest: &PublicationManifest) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "workflow": "arxiv_operator_assist",
        "publication_id": manifest.publication_id,
        "title": manifest.title,
        "primary_author": manifest.author,
        "content_sha3_256": manifest.content_sha3_256(),
        "staging_generated_by": "vox-publisher/submission_package",
        "arxiv_bundle_relpath": "arxiv_bundle.tar.gz",
        "note": "Operator-assisted arXiv submission; not an automated arXiv API deposit.",
    })
}

/// Upper bound for any single staging text/binary file (100 MiB).
pub const MAX_STAGING_FILE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug)]
pub enum StagingExportError {
    Io(std::io::Error),
    Cff(serde_yaml::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for StagingExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StagingExportError::Io(e) => write!(f, "{e}"),
            StagingExportError::Cff(e) => write!(f, "{e}"),
            StagingExportError::Json(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StagingExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StagingExportError::Io(e) => Some(e),
            StagingExportError::Cff(e) => Some(e),
            StagingExportError::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for StagingExportError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for StagingExportError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Cff(value)
    }
}

impl From<serde_json::Error> for StagingExportError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

/// Create `out_dir` (if needed) and write staging files for `manifest`.
pub fn write_scholarly_staging(
    manifest: &PublicationManifest,
    venue: ScholarlyVenue,
    out_dir: &Path,
) -> Result<Vec<String>, StagingExportError> {
    fs::create_dir_all(out_dir)?;
    let mut written: Vec<String> = Vec::new();

    let body_path = out_dir.join("body.md");
    fs::write(&body_path, manifest.body_markdown.as_bytes())?;
    written.push("body.md".to_string());

    let cff = render_citation_cff(manifest)?;
    fs::write(out_dir.join("CITATION.cff"), cff)?;
    written.push("CITATION.cff".to_string());

    let crossref = crossref_work_export_json(manifest);
    let crossref_s = serde_json::to_string_pretty(&crossref)?;
    fs::write(out_dir.join("crossref_work.json"), crossref_s)?;
    written.push("crossref_work.json".to_string());

    if let Some(raw) = manifest.citations_json.as_deref() {
        let t = raw.trim();
        if !t.is_empty() {
            fs::write(out_dir.join("citations.json"), raw.as_bytes())?;
            written.push("citations.json".to_string());
        }
    }

    if matches!(venue, ScholarlyVenue::Zenodo) {
        let zj = zenodo_metadata::zenodo_json_pretty(manifest)?;
        fs::write(out_dir.join("zenodo.json"), zj)?;
        written.push("zenodo.json".to_string());
    }

    if matches!(venue, ScholarlyVenue::ArxivAssist) {
        let ah = serde_json::to_string_pretty(&arxiv_operator_handoff_value(manifest))?;
        fs::write(out_dir.join("arxiv_handoff.json"), ah)?;
        written.push("arxiv_handoff.json".to_string());
        let tar_path = out_dir.join("arxiv_bundle.tar.gz");
        pack_arxiv_staging_tar_gz(out_dir, &tar_path)?;
        written.push("arxiv_bundle.tar.gz".to_string());
    }

    Ok(written)
}

/// Gzip-compressed tar of every regular file in `staging_dir` (sorted by name), except `arxiv_bundle.tar.gz`.
fn pack_arxiv_staging_tar_gz(staging_dir: &Path, dest: &Path) -> Result<(), StagingExportError> {
    let _ = fs::remove_file(dest);
    let out = fs::File::create(dest)?;
    let enc = GzEncoder::new(out, Compression::default());
    let mut builder = tar::Builder::new(enc);
    builder.mode(tar::HeaderMode::Deterministic);

    let mut names: Vec<String> = fs::read_dir(staging_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| n != "arxiv_bundle.tar.gz")
        .collect();
    names.sort_unstable();

    for name in names {
        let path = staging_dir.join(&name);
        let mut file = fs::File::open(&path)?;
        builder
            .append_file(&name, &mut file)
            .map_err(StagingExportError::Io)?;
    }

    let enc = builder.into_inner().map_err(StagingExportError::Io)?;
    enc.finish().map_err(StagingExportError::Io)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationFinding {
    pub code: &'static str,
    pub message: String,
}

/// Validate staging directory contents against the venue plan.
#[must_use]
pub fn validate_scholarly_staging(
    out_dir: &Path,
    venue: ScholarlyVenue,
    manifest: &PublicationManifest,
) -> Result<(), Vec<ValidationFinding>> {
    let mut findings: Vec<ValidationFinding> = Vec::new();
    let plan = staging_artifacts(venue);

    for art in &plan {
        let p = out_dir.join(&art.relative_path);
        if art.relative_path == "citations.json" {
            let src = manifest.citations_json.as_deref().map(str::trim).unwrap_or("");
            if src.is_empty() {
                continue;
            }
        }
        if !p.is_file() {
            findings.push(ValidationFinding {
                code: "staging_file_missing",
                message: format!("expected file {}", p.display()),
            });
            continue;
        }
        match fs::metadata(&p) {
            Ok(m) => {
                if m.len() > MAX_STAGING_FILE_BYTES {
                    findings.push(ValidationFinding {
                        code: "staging_file_too_large",
                        message: format!(
                            "{} exceeds max {} bytes",
                            p.display(),
                            MAX_STAGING_FILE_BYTES
                        ),
                    });
                }
                if m.len() == 0 && art.require_non_empty_source {
                    findings.push(ValidationFinding {
                        code: "staging_file_empty",
                        message: format!("{} must not be empty", p.display()),
                    });
                }
            }
            Err(e) => findings.push(ValidationFinding {
                code: "staging_metadata",
                message: format!("{}: {e}", p.display()),
            }),
        }
        if let Err(e) = file_kind_check(&p) {
            findings.push(e);
        }
        findings.extend(content_checks(&p, art.relative_path.as_str(), venue));
    }

    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings)
    }
}

fn content_checks(path: &Path, relative_path: &str, venue: ScholarlyVenue) -> Vec<ValidationFinding> {
    let mut out = Vec::new();
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            out.push(ValidationFinding {
                code: "staging_read",
                message: format!("{}: {e}", path.display()),
            });
            return out;
        }
    };
    let is_binary_staging = relative_path == "arxiv_bundle.tar.gz";
    if !is_binary_staging && std::str::from_utf8(&bytes).is_err() {
        out.push(ValidationFinding {
            code: "staging_invalid_utf8",
            message: format!("{} must be valid UTF-8", path.display()),
        });
        return out;
    }
    match relative_path {
        "citations.json" | "crossref_work.json" | "zenodo.json" | "arxiv_handoff.json" => {
            if let Err(e) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                out.push(ValidationFinding {
                    code: "staging_json_invalid",
                    message: format!("{}: {e}", path.display()),
                });
            }
        }
        "CITATION.cff" => {
            if let Err(e) = serde_yaml::from_slice::<serde_yaml::Value>(&bytes) {
                out.push(ValidationFinding {
                    code: "staging_cff_invalid",
                    message: format!("{}: {e}", path.display()),
                });
            }
        }
        _ => {}
    }
    if matches!(venue, ScholarlyVenue::Zenodo) && relative_path == "zenodo.json" {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if val.get("metadata").and_then(|m| m.as_object()).is_none() {
                out.push(ValidationFinding {
                    code: "staging_zenodo_json_shape",
                    message: format!(
                        "{} must be an object with a `metadata` object (Zenodo deposit envelope)",
                        path.display()
                    ),
                });
            }
        }
    }
    if matches!(venue, ScholarlyVenue::ArxivAssist) && relative_path == "arxiv_handoff.json" {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            let ok = val.get("schema_version").is_some()
                && val
                    .get("workflow")
                    .and_then(|s| s.as_str())
                    .is_some_and(|w| w == "arxiv_operator_assist")
                && val
                    .get("publication_id")
                    .and_then(|s| s.as_str())
                    .is_some_and(|s| !s.is_empty())
                && val
                    .get("content_sha3_256")
                    .and_then(|s| s.as_str())
                    .is_some_and(|s| !s.is_empty())
                && val
                    .get("arxiv_bundle_relpath")
                    .and_then(|s| s.as_str())
                    == Some("arxiv_bundle.tar.gz");
            if !ok {
                out.push(ValidationFinding {
                    code: "staging_arxiv_handoff_shape",
                    message: format!(
                        "{} must include schema_version, workflow=arxiv_operator_assist, publication_id, content_sha3_256, arxiv_bundle_relpath",
                        path.display()
                    ),
                });
            }
        }
    }
    if matches!(venue, ScholarlyVenue::ArxivAssist) && relative_path == "arxiv_bundle.tar.gz" {
        if bytes.len() < 2 || bytes[0] != 0x1f || bytes[1] != 0x8b {
            out.push(ValidationFinding {
                code: "staging_arxiv_bundle_not_gzip",
                message: format!("{} must be a gzip stream (starts with 1f 8b)", path.display()),
            });
        }
    }
    out
}

fn file_kind_check(path: &Path) -> Result<(), ValidationFinding> {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return Err(ValidationFinding {
            code: "staging_bad_filename",
            message: path.display().to_string(),
        });
    };
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".md")
        || lower.ends_with(".cff")
        || lower.ends_with(".json")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with(".tar.gz")
    {
        return Ok(());
    }
    Err(ValidationFinding {
        code: "staging_unexpected_extension",
        message: format!(
            "{} (allowed: .md .cff .json .yaml .yml .tar.gz)",
            path.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::scientific_metadata::{ScientificAuthor, ScientificPublicationMetadata};

    #[test]
    fn write_and_validate_round_trip_zenodo() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Author".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("p", None, Some(&sci), None)
                .unwrap();
        let manifest = PublicationManifest {
            publication_id: "pub".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "T".to_string(),
            author: "Author".to_string(),
            abstract_text: Some("A".to_string()),
            body_markdown: "hello".to_string(),
            citations_json: None,
            metadata_json: Some(meta),
        };
        let tmp = tempfile::tempdir().unwrap();
        let files = write_scholarly_staging(&manifest, ScholarlyVenue::Zenodo, tmp.path()).unwrap();
        assert!(files.iter().any(|f| f == "zenodo.json"));
        validate_scholarly_staging(tmp.path(), ScholarlyVenue::Zenodo, &manifest).unwrap();
    }

    #[test]
    fn validation_rejects_malformed_citations_json() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Author".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("p", None, Some(&sci), None)
                .unwrap();
        let manifest = PublicationManifest {
            publication_id: "pub".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "T".to_string(),
            author: "Author".to_string(),
            abstract_text: Some("A".to_string()),
            body_markdown: "hello".to_string(),
            citations_json: Some("[1]".to_string()),
            metadata_json: Some(meta),
        };
        let tmp = tempfile::tempdir().unwrap();
        write_scholarly_staging(&manifest, ScholarlyVenue::OpenReview, tmp.path()).unwrap();
        fs::write(tmp.path().join("citations.json"), "not-json").unwrap();
        let err = validate_scholarly_staging(tmp.path(), ScholarlyVenue::OpenReview, &manifest)
            .unwrap_err();
        assert!(err.iter().any(|f| f.code == "staging_json_invalid"));
    }

    #[test]
    fn write_and_validate_round_trip_arxiv_assist() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Author".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("Apache-2.0".to_string()),
            ..Default::default()
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("p", None, Some(&sci), None)
                .unwrap();
        let manifest = PublicationManifest {
            publication_id: "pub-arxiv".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "T".to_string(),
            author: "Author".to_string(),
            abstract_text: Some("A".to_string()),
            body_markdown: "hello".to_string(),
            citations_json: None,
            metadata_json: Some(meta),
        };
        let tmp = tempfile::tempdir().unwrap();
        let files = write_scholarly_staging(
            &manifest,
            ScholarlyVenue::ArxivAssist,
            tmp.path(),
        )
        .unwrap();
        assert!(!files.iter().any(|f| f == "zenodo.json"));
        assert!(files.iter().any(|f| f == "arxiv_handoff.json"));
        assert!(files.iter().any(|f| f == "arxiv_bundle.tar.gz"));
        let handoff = fs::read_to_string(tmp.path().join("arxiv_handoff.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&handoff).unwrap();
        assert_eq!(v["workflow"], "arxiv_operator_assist");
        assert_eq!(v["publication_id"], "pub-arxiv");
        assert_eq!(v["arxiv_bundle_relpath"], "arxiv_bundle.tar.gz");
        let bundle = fs::read(tmp.path().join("arxiv_bundle.tar.gz")).unwrap();
        assert!(bundle.len() >= 2 && bundle[0] == 0x1f && bundle[1] == 0x8b);
        validate_scholarly_staging(tmp.path(), ScholarlyVenue::ArxivAssist, &manifest).unwrap();
    }
}
