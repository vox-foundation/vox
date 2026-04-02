//! Venue-scoped **staging** layout for scholarly exports (sidecar files next to a submission bundle).
//!
//! This module plans which files to write and validates an on-disk tree before upload tooling runs.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use sha3::Sha3_256;
use sha3::digest::Digest;
use tar::Archive;

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
            relative_path: "main.tex".to_string(),
            require_non_empty_source: true,
        });
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
        "main_tex_relpath": "main.tex",
        "body_markdown_relpath": "body.md",
        "staging_generated_by": "vox-publisher/submission_package",
        "arxiv_bundle_relpath": "arxiv_bundle.tar.gz",
        "staging_checksums_relpath": "staging_checksums.json",
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
        let main_tex_body = arxiv_assist_main_tex(manifest);
        fs::write(out_dir.join("main.tex"), main_tex_body)?;
        written.push("main.tex".to_string());
        let ah = serde_json::to_string_pretty(&arxiv_operator_handoff_value(manifest))?;
        fs::write(out_dir.join("arxiv_handoff.json"), ah)?;
        written.push("arxiv_handoff.json".to_string());
        let tar_path = out_dir.join("arxiv_bundle.tar.gz");
        pack_arxiv_staging_tar_gz(out_dir, &tar_path)?;
        written.push("arxiv_bundle.tar.gz".to_string());
    }

    write_staging_checksum_manifest(out_dir, &written)?;
    written.push("staging_checksums.json".to_string());

    Ok(written)
}

/// Writes `staging_checksums.json` (`schema_version` + hex SHA3-256 per relative path) for Zenodo upload verification.
pub fn write_staging_checksum_manifest(
    out_dir: &Path,
    relpaths: &[String],
) -> Result<(), StagingExportError> {
    let mut sha_map = serde_json::Map::new();
    for rel in relpaths {
        if rel == "staging_checksums.json" {
            continue;
        }
        let p = out_dir.join(rel);
        if !p.is_file() {
            continue;
        }
        let bytes = fs::read(&p)?;
        let digest = Sha3_256::digest(&bytes);
        let hex = format!("{digest:x}");
        sha_map.insert(rel.clone(), serde_json::json!(hex));
    }
    let doc = serde_json::json!({
        "schema_version": 1_i32,
        "sha3_256": sha_map,
    });
    fs::write(
        out_dir.join("staging_checksums.json"),
        serde_json::to_string_pretty(&doc)?,
    )?;
    Ok(())
}

/// Validate gzip-compressed tar expected to contain an arXiv-style source bundle (≥1 `.tex`, no risky extensions).
#[must_use]
pub fn validate_arxiv_submission_tar_gz(bytes: &[u8]) -> Vec<ValidationFinding> {
    let mut findings = Vec::new();
    if bytes.len() < 2 || bytes[0] != 0x1f || bytes[1] != 0x8b {
        findings.push(ValidationFinding {
            code: "arxiv_bundle_not_gzip",
            message: "bundle must be gzip (0x1f 0x8b)".into(),
        });
        return findings;
    }
    let dec = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(dec);
    let mut tex_count = 0_usize;
    let mut main_tex_present = false;
    let mut entries = match archive.entries() {
        Ok(e) => e,
        Err(e) => {
            findings.push(ValidationFinding {
                code: "arxiv_bundle_tar_unreadable",
                message: format!("tar entries: {e}"),
            });
            return findings;
        }
    };
    for ent in entries.by_ref() {
        let ent = match ent {
            Ok(e) => e,
            Err(e) => {
                findings.push(ValidationFinding {
                    code: "arxiv_bundle_tar_entry",
                    message: format!("{e}"),
                });
                continue;
            }
        };
        let path = match ent.path() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let path_s = path.to_string_lossy();
        if path_s.contains("..") {
            findings.push(ValidationFinding {
                code: "arxiv_path_traversal",
                message: format!("unsafe path {path_s:?}"),
            });
            continue;
        }
        let lower = path_s.to_ascii_lowercase();
        if lower.ends_with(".tex") {
            tex_count += 1;
        }
        if lower == "main.tex" {
            main_tex_present = true;
        }
        for bad in [".exe", ".dll", ".dmg", ".pkg", ".deb", ".msi", ".app/"] {
            if lower.contains(bad) {
                findings.push(ValidationFinding {
                    code: "arxiv_disallowed_binary",
                    message: format!("path {path_s:?} matches disallowed pattern {bad:?}"),
                });
            }
        }
        for bad in [".zip", ".rar", ".7z", ".tar"] {
            if lower.ends_with(bad) {
                findings.push(ValidationFinding {
                    code: "arxiv_disallowed_nested_archive",
                    message: format!("path {path_s:?} uses nested archive suffix {bad:?}"),
                });
            }
        }
        if !arxiv_allowed_archive_path(&lower) {
            findings.push(ValidationFinding {
                code: "arxiv_unrecognized_file_family",
                message: format!(
                    "path {path_s:?} is not in the supported arXiv assist file families"
                ),
            });
        }
    }
    if tex_count == 0 {
        findings.push(ValidationFinding {
            code: "arxiv_missing_tex",
            message: "arXiv bundle must contain at least one .tex source".into(),
        });
    }
    if tex_count > 0 && !main_tex_present {
        findings.push(ValidationFinding {
            code: "arxiv_missing_main_tex",
            message: "arXiv assist bundle must include main.tex as the primary handoff entrypoint"
                .into(),
        });
    }
    if tex_count > 1 {
        findings.push(ValidationFinding {
            code: "arxiv_multiple_tex_sources",
            message: format!(
                "bundle contains {tex_count} `.tex` files — ensure `main.tex` is the unique entrypoint or split submissions deliberately"
            ),
        });
    }
    findings
}

fn latex_escape_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_mul(2));
    for c in s.chars() {
        match c {
            '\\' | '{' | '}' | '#' | '$' | '%' | '^' | '_' | '&' | '~' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("\n\n"),
            _ => out.push(c),
        }
    }
    out
}

fn arxiv_assist_main_tex(manifest: &PublicationManifest) -> String {
    let title = latex_escape_minimal(&manifest.title);
    let author = latex_escape_minimal(&manifest.author);
    let abs = manifest
        .abstract_text
        .as_deref()
        .map(latex_escape_minimal)
        .unwrap_or_else(|| "Abstract pending.".to_string());
    format!(
        "% Auto-generated main.tex for arXiv operator-assist staging (vox-publisher).\n\
\\documentclass{{article}}\n\
\\usepackage{{hyperref}}\n\
\\title{{{title}}}\n\
\\author{{{author}}}\n\
\\begin{{document}}\n\
\\maketitle\n\
\\begin{{abstract}}\n\
{abs}\n\
\\end{{abstract}}\n\
\\noindent\\textit{{Companion manuscript:}} \\texttt{{body.md}} in this bundle.\n\
\\end{{document}}\n"
    )
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

fn arxiv_allowed_archive_path(lower: &str) -> bool {
    [
        ".tex", ".sty", ".cls", ".bst", ".bib", ".bbl", ".bbx", ".cbx", ".cfg", ".clo", ".def",
        ".fd", ".ist", ".txt", ".md", ".json", ".yaml", ".yml", ".cff", ".png", ".jpg", ".jpeg",
        ".pdf", ".eps", ".ps", ".svg", ".csv", ".tsv",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
}

fn validate_arxiv_handoff_value(v: &serde_json::Value) -> bool {
    v.get("schema_version").and_then(|n| n.as_i64()) == Some(1)
        && v.get("workflow").and_then(|s| s.as_str()) == Some("arxiv_operator_assist")
        && v.get("publication_id")
            .and_then(|s| s.as_str())
            .is_some_and(|s| !s.is_empty())
        && v.get("title")
            .and_then(|s| s.as_str())
            .is_some_and(|s| !s.is_empty())
        && v.get("primary_author")
            .and_then(|s| s.as_str())
            .is_some_and(|s| !s.is_empty())
        && v.get("content_sha3_256")
            .and_then(|s| s.as_str())
            .is_some_and(|s| !s.is_empty())
        && v.get("main_tex_relpath").and_then(|s| s.as_str()) == Some("main.tex")
        && v.get("body_markdown_relpath").and_then(|s| s.as_str()) == Some("body.md")
        && v.get("arxiv_bundle_relpath").and_then(|s| s.as_str()) == Some("arxiv_bundle.tar.gz")
        && v.get("staging_checksums_relpath").and_then(|s| s.as_str())
            == Some("staging_checksums.json")
}

/// Validate staging directory contents against the venue plan.
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
            let src = manifest
                .citations_json
                .as_deref()
                .map(str::trim)
                .unwrap_or("");
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

    if matches!(venue, ScholarlyVenue::ArxivAssist) {
        findings.extend(arxiv_staging_handoff_quality_notes(out_dir, manifest));
    }

    if findings.is_empty() {
        Ok(())
    } else {
        Err(findings)
    }
}

/// Advisory completeness checks for operator-assisted arXiv staging (non-blocking collection).
fn arxiv_staging_handoff_quality_notes(
    out_dir: &Path,
    manifest: &PublicationManifest,
) -> Vec<ValidationFinding> {
    let mut v = Vec::new();
    let handoff_p = out_dir.join("arxiv_handoff.json");
    if handoff_p.is_file()
        && let Ok(raw) = fs::read_to_string(&handoff_p)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        if !validate_arxiv_handoff_value(&val) {
            v.push(ValidationFinding {
                code: "arxiv_handoff_contract_mismatch",
                message: "arxiv_handoff.json failed contract validation against expected arXiv assist operator envelope".into(),
            });
        }
        let digest_ok = val.get("content_sha3_256").and_then(|x| x.as_str())
            == Some(manifest.content_sha3_256().as_str());
        if !digest_ok {
            v.push(ValidationFinding {
                code: "arxiv_handoff_digest_mismatch",
                message: "arxiv_handoff.json content_sha3_256 does not match current manifest digest — regenerate staging".into(),
            });
        }
    }
    let checksums = out_dir.join("staging_checksums.json");
    if checksums.is_file()
        && let Ok(raw) = fs::read_to_string(&checksums)
        && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
    {
        let has_bundle = val
            .get("sha3_256")
            .and_then(|m| m.as_object())
            .is_some_and(|m| m.contains_key("arxiv_bundle.tar.gz"));
        if !has_bundle {
            v.push(ValidationFinding {
                code: "staging_checksums_missing_arxiv_bundle",
                message: "staging_checksums.json should record arxiv_bundle.tar.gz for custody"
                    .into(),
            });
        }
    }
    v
}

fn content_checks(
    path: &Path,
    relative_path: &str,
    venue: ScholarlyVenue,
) -> Vec<ValidationFinding> {
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
        "main.tex" => {
            if let Ok(text) = std::str::from_utf8(&bytes) {
                for (needle, code) in [
                    ("\\documentclass", "staging_arxiv_main_tex_documentclass"),
                    ("\\title{", "staging_arxiv_main_tex_title"),
                    ("\\author{", "staging_arxiv_main_tex_author"),
                    ("\\begin{abstract}", "staging_arxiv_main_tex_abstract"),
                ] {
                    if !text.contains(needle) {
                        out.push(ValidationFinding {
                            code,
                            message: format!("{} must contain {}", path.display(), needle),
                        });
                    }
                }
                let lower_tex = text.to_ascii_lowercase();
                if lower_tex.contains("\\usepackage{minted}") {
                    out.push(ValidationFinding {
                        code: "staging_arxiv_minted_package",
                        message: format!(
                            "{} uses `minted` — arXiv may require `-shell-escape` / custom packaging; verify venue policy",
                            path.display()
                        ),
                    });
                }
            }
        }
        _ => {}
    }
    if matches!(venue, ScholarlyVenue::Zenodo)
        && relative_path == "zenodo.json"
        && let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && val.get("metadata").and_then(|m| m.as_object()).is_none()
    {
        out.push(ValidationFinding {
            code: "staging_zenodo_json_shape",
            message: format!(
                "{} must be an object with a `metadata` object (Zenodo deposit envelope)",
                path.display()
            ),
        });
    }
    if matches!(venue, ScholarlyVenue::ArxivAssist)
        && relative_path == "arxiv_handoff.json"
        && let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes)
        && !validate_arxiv_handoff_value(&val)
    {
        out.push(ValidationFinding {
            code: "staging_arxiv_handoff_shape",
            message: format!(
                "{} must include schema_version=1, workflow=arxiv_operator_assist, publication_id, title, primary_author, content_sha3_256, main_tex_relpath, body_markdown_relpath, arxiv_bundle_relpath, staging_checksums_relpath",
                path.display()
            ),
        });
    }
    if matches!(venue, ScholarlyVenue::ArxivAssist) && relative_path == "arxiv_bundle.tar.gz" {
        for f in validate_arxiv_submission_tar_gz(&bytes) {
            if f.code == "arxiv_bundle_not_gzip" {
                out.push(ValidationFinding {
                    code: "staging_arxiv_bundle_not_gzip",
                    message: format!(
                        "{} must be a gzip stream (starts with 1f 8b)",
                        path.display()
                    ),
                });
            } else {
                out.push(f);
            }
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
        || lower.ends_with(".tex")
    {
        return Ok(());
    }
    Err(ValidationFinding {
        code: "staging_unexpected_extension",
        message: format!(
            "{} (allowed: .md .cff .json .yaml .yml .tar.gz .tex)",
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
        let files =
            write_scholarly_staging(&manifest, ScholarlyVenue::ArxivAssist, tmp.path()).unwrap();
        assert!(!files.iter().any(|f| f == "zenodo.json"));
        assert!(files.iter().any(|f| f == "main.tex"));
        assert!(files.iter().any(|f| f == "arxiv_handoff.json"));
        assert!(files.iter().any(|f| f == "arxiv_bundle.tar.gz"));
        let handoff = fs::read_to_string(tmp.path().join("arxiv_handoff.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&handoff).unwrap();
        assert_eq!(v["workflow"], "arxiv_operator_assist");
        assert_eq!(v["publication_id"], "pub-arxiv");
        assert_eq!(v["main_tex_relpath"], "main.tex");
        assert_eq!(v["body_markdown_relpath"], "body.md");
        assert_eq!(v["arxiv_bundle_relpath"], "arxiv_bundle.tar.gz");
        assert_eq!(v["staging_checksums_relpath"], "staging_checksums.json");
        let bundle = fs::read(tmp.path().join("arxiv_bundle.tar.gz")).unwrap();
        assert!(bundle.len() >= 2 && bundle[0] == 0x1f && bundle[1] == 0x8b);
        validate_scholarly_staging(tmp.path(), ScholarlyVenue::ArxivAssist, &manifest).unwrap();
    }

    #[test]
    fn arxiv_bundle_validation_requires_main_tex() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("body.md"), "body").unwrap();
        fs::write(dir.path().join("appendix.tex"), "\\documentclass{article}").unwrap();
        fs::write(
            dir.path().join("staging_checksums.json"),
            "{\"schema_version\":1}",
        )
        .unwrap();
        let bundle = dir.path().join("arxiv_bundle.tar.gz");
        pack_arxiv_staging_tar_gz(dir.path(), &bundle).unwrap();
        let findings = validate_arxiv_submission_tar_gz(&fs::read(bundle).unwrap());
        assert!(findings.iter().any(|f| f.code == "arxiv_missing_main_tex"));
    }
}
