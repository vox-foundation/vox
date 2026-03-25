//! Readiness checks for [`crate::publication::PublicationManifest`] before journal or repository submission.

use std::sync::OnceLock;

use regex::Regex;

use crate::publication::PublicationManifest;
use crate::scientific_metadata::{ScientificPublicationMetadata, METADATA_KEY_SCIENTIFIC};

/// Venue-sensitive strictness (`double_blind` adds anonymization checks on the body).
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightProfile {
    #[default]
    Default,
    DoubleBlind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PreflightSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PreflightFinding {
    pub code: &'static str,
    pub severity: PreflightSeverity,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PreflightReport {
    pub ok: bool,
    pub readiness_score: u8,
    pub findings: Vec<PreflightFinding>,
}

fn email_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
            .expect("email preflight regex")
    })
}

fn normalize_person_name(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Parse `metadata_json.scientific_publication` if present.
pub fn parse_scientific_from_metadata_json(
    metadata_json: Option<&str>,
) -> Result<Option<ScientificPublicationMetadata>, String> {
    let Some(raw) = metadata_json else {
        return Ok(None);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let root: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("metadata_json: {e}"))?;
    let Some(block) = root.get(METADATA_KEY_SCIENTIFIC) else {
        return Ok(None);
    };
    serde_json::from_value(block.clone()).map_err(|e| format!("scientific_publication: {e}"))
}

/// Run checks; `ok` is false when any finding has severity [`PreflightSeverity::Error`].
#[must_use]
pub fn run_preflight(manifest: &PublicationManifest, profile: PreflightProfile) -> PreflightReport {
    let mut findings: Vec<PreflightFinding> = Vec::new();

    if manifest.title.trim().is_empty() {
        findings.push(PreflightFinding {
            code: "title_empty",
            severity: PreflightSeverity::Error,
            message: "title must not be empty".to_string(),
        });
    }

    if manifest.author.trim().is_empty() {
        findings.push(PreflightFinding {
            code: "author_empty",
            severity: PreflightSeverity::Error,
            message: "author must not be empty".to_string(),
        });
    }

    if let Some(raw) = manifest.metadata_json.as_deref() {
        if !raw.trim().is_empty() {
            match serde_json::from_str::<serde_json::Value>(raw) {
                Ok(_) => {}
                Err(e) => findings.push(PreflightFinding {
                    code: "metadata_json_invalid",
                    severity: PreflightSeverity::Error,
                    message: format!("metadata_json is not valid JSON: {e}"),
                }),
            }
        }
    }

    match parse_scientific_from_metadata_json(manifest.metadata_json.as_deref()) {
        Ok(Some(sci)) => {
            for (i, a) in sci.authors.iter().enumerate() {
                if a.name.trim().is_empty() {
                    findings.push(PreflightFinding {
                        code: "scientific_author_name_empty",
                        severity: PreflightSeverity::Error,
                        message: format!("scientific_publication.authors[{i}].name is empty"),
                    });
                }
            }
            if !sci.authors.is_empty() {
                let primary = normalize_person_name(&sci.authors[0].name);
                let top = normalize_person_name(&manifest.author);
                if !primary.is_empty() && !top.is_empty() && primary != top {
                    findings.push(PreflightFinding {
                        code: "author_primary_mismatch",
                        severity: PreflightSeverity::Error,
                        message: format!(
                            "manifest.author {:?} does not match scientific_publication.authors[0].name {:?}",
                            manifest.author, sci.authors[0].name
                        ),
                    });
                }
            }
            if sci.license_spdx.as_ref().map_or(true, |s| s.trim().is_empty()) {
                findings.push(PreflightFinding {
                    code: "license_missing",
                    severity: PreflightSeverity::Warning,
                    message: "scientific_publication.license_spdx is unset (recommended for self-archiving and journals)".to_string(),
                });
            }
            let repro_empty = sci.reproducibility.as_ref().map_or(true, |r| {
                r.code_repository_url.as_ref().map_or(true, |s| s.trim().is_empty())
                    && r.data_repository_url.as_ref().map_or(true, |s| s.trim().is_empty())
                    && r.artifact_checksum_note.as_ref().map_or(true, |s| s.trim().is_empty())
            });
            if repro_empty {
                findings.push(PreflightFinding {
                    code: "reproducibility_sparse",
                    severity: PreflightSeverity::Warning,
                    message: "reproducibility block has no code_repository_url, data_repository_url, or artifact_checksum_note".to_string(),
                });
            }
        }
        Ok(None) => {
            findings.push(PreflightFinding {
                code: "scientific_metadata_absent",
                severity: PreflightSeverity::Warning,
                message: format!(
                    "no `{METADATA_KEY_SCIENTIFIC}` in metadata_json — add structured authors, license, and reproducibility for publication targets"
                ),
            });
        }
        Err(e) => findings.push(PreflightFinding {
            code: "scientific_metadata_invalid",
            severity: PreflightSeverity::Error,
            message: e,
        }),
    }

    if manifest
        .abstract_text
        .as_deref()
        .map_or(true, |s| s.trim().is_empty())
    {
        findings.push(PreflightFinding {
            code: "abstract_missing",
            severity: PreflightSeverity::Warning,
            message: "abstract_text is empty (journals and arXiv expect an abstract)".to_string(),
        });
    }

    if let Some(c) = manifest.citations_json.as_deref() {
        let t = c.trim();
        if !t.is_empty() && serde_json::from_str::<serde_json::Value>(t).is_err() {
            findings.push(PreflightFinding {
                code: "citations_json_invalid",
                severity: PreflightSeverity::Error,
                message: "citations_json is not valid JSON".to_string(),
            });
        }
    }

    if profile == PreflightProfile::DoubleBlind && email_pattern().is_match(&manifest.body_markdown) {
        findings.push(PreflightFinding {
            code: "double_blind_email_in_body",
            severity: PreflightSeverity::Error,
            message: "email-like pattern in body_markdown — remove for double-blind submission".to_string(),
        });
    }

    let err_n = findings
        .iter()
        .filter(|f| f.severity == PreflightSeverity::Error)
        .count();
    let warn_n = findings
        .iter()
        .filter(|f| f.severity == PreflightSeverity::Warning)
        .count();
    let mut score: i32 = 100 - (err_n as i32) * 25 - (warn_n as i32) * 10;
    score = score.clamp(0, 100);

    PreflightReport {
        ok: err_n == 0,
        readiness_score: score as u8,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scientific_metadata::{ScientificAuthor, ScientificPublicationMetadata};

    fn sample_manifest(f: impl FnOnce(&mut PublicationManifest)) -> PublicationManifest {
        let mut m = PublicationManifest {
            publication_id: "p".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "Title".to_string(),
            author: "Ada Lovelace".to_string(),
            abstract_text: Some("Abstract.".to_string()),
            body_markdown: "Hello.".to_string(),
            citations_json: None,
            metadata_json: None,
        };
        f(&mut m);
        m
    }

    #[test]
    fn ok_when_aligned_scientific_block() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Ada Lovelace".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta = crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci))
            .unwrap();
        let m = sample_manifest(|x| x.metadata_json = Some(meta));
        let r = run_preflight(&m, PreflightProfile::Default);
        assert!(r.ok, "{:?}", r.findings);
        assert!(r.readiness_score >= 80);
    }

    #[test]
    fn error_on_author_mismatch() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Someone Else".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta = crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci))
            .unwrap();
        let m = sample_manifest(|x| x.metadata_json = Some(meta));
        let r = run_preflight(&m, PreflightProfile::Default);
        assert!(!r.ok);
        assert!(r.findings.iter().any(|f| f.code == "author_primary_mismatch"));
    }

    #[test]
    fn double_blind_flags_email() {
        let m = sample_manifest(|x| {
            x.body_markdown = "Contact me at lee@example.com.".to_string();
        });
        let r = run_preflight(&m, PreflightProfile::DoubleBlind);
        assert!(!r.ok);
        assert!(
            r.findings
                .iter()
                .any(|f| f.code == "double_blind_email_in_body")
        );
    }
}
