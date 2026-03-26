//! Readiness checks for [`crate::publication::PublicationManifest`] before journal or repository submission.

use std::sync::OnceLock;

use regex::Regex;

use crate::publication::PublicationManifest;
use crate::scientific_metadata::{METADATA_KEY_SCIENTIFIC, ScientificPublicationMetadata};

/// Venue-sensitive strictness (`double_blind` anonymization; `metadata_complete` errors on thin metadata).
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreflightProfile {
    #[default]
    Default,
    DoubleBlind,
    /// Errors when structured scholarly metadata is missing or insufficient for repository metadata exports.
    MetadataComplete,
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
    /// Conservative worthiness rubric output when requested (heuristic metrics; `meaningful_advance` is always false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worthiness: Option<crate::publication_worthiness::WorthinessEvaluation>,
}

fn email_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
            .expect("email preflight regex")
    })
}

/// ORCID id pattern (checksum digit may be `X`).
fn orcid_id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"\b",
            r"\d{4}-\d{4}-\d{4}-\d{3}",
            r"[0-9X]",
            r"\b"
        ))
        .expect("orcid id preflight regex")
    })
}

fn normalize_person_name(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
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

    if let Some(raw) = manifest.metadata_json.as_deref()
        && !raw.trim().is_empty()
    {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(_) => {}
            Err(e) => findings.push(PreflightFinding {
                code: "metadata_json_invalid",
                severity: PreflightSeverity::Error,
                message: format!("metadata_json is not valid JSON: {e}"),
            }),
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
            if sci
                .license_spdx
                .as_ref()
                .is_none_or(|s| s.trim().is_empty())
            {
                if profile == PreflightProfile::MetadataComplete {
                    findings.push(PreflightFinding {
                        code: "license_required",
                        severity: PreflightSeverity::Error,
                        message: "scientific_publication.license_spdx is required for metadata_complete preflight".to_string(),
                    });
                } else {
                    findings.push(PreflightFinding {
                        code: "license_missing",
                        severity: PreflightSeverity::Warning,
                        message: "scientific_publication.license_spdx is unset (recommended for self-archiving and journals)".to_string(),
                    });
                }
            }
            if profile == PreflightProfile::MetadataComplete && sci.authors.is_empty() {
                findings.push(PreflightFinding {
                    code: "scientific_authors_required",
                    severity: PreflightSeverity::Error,
                    message: "metadata_complete requires at least one scientific_publication.authors entry".to_string(),
                });
            }
            let repro_empty = sci.reproducibility.as_ref().is_none_or(|r| {
                r.code_repository_url
                    .as_ref()
                    .is_none_or(|s| s.trim().is_empty())
                    && r.data_repository_url
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())
                    && r.artifact_checksum_note
                        .as_ref()
                        .is_none_or(|s| s.trim().is_empty())
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
            if profile == PreflightProfile::MetadataComplete {
                findings.push(PreflightFinding {
                    code: "scientific_metadata_required",
                    severity: PreflightSeverity::Error,
                    message: format!(
                        "metadata_complete requires `{METADATA_KEY_SCIENTIFIC}` in metadata_json"
                    ),
                });
            } else {
                findings.push(PreflightFinding {
                    code: "scientific_metadata_absent",
                    severity: PreflightSeverity::Warning,
                    message: format!(
                        "no `{METADATA_KEY_SCIENTIFIC}` in metadata_json — add structured authors, license, and reproducibility for publication targets"
                    ),
                });
            }
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
        .is_none_or(|s| s.trim().is_empty())
    {
        if profile == PreflightProfile::MetadataComplete {
            findings.push(PreflightFinding {
                code: "abstract_required",
                severity: PreflightSeverity::Error,
                message: "abstract_text is required for metadata_complete preflight".to_string(),
            });
        } else {
            findings.push(PreflightFinding {
                code: "abstract_missing",
                severity: PreflightSeverity::Warning,
                message: "abstract_text is empty (journals and arXiv expect an abstract)".to_string(),
            });
        }
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

    if profile == PreflightProfile::DoubleBlind {
        let body = &manifest.body_markdown;
        if email_pattern().is_match(body) {
            findings.push(PreflightFinding {
                code: "double_blind_email_in_body",
                severity: PreflightSeverity::Error,
                message: "email-like pattern in body_markdown — remove for double-blind submission"
                    .to_string(),
            });
        }
        if body.to_ascii_lowercase().contains("orcid.org") {
            findings.push(PreflightFinding {
                code: "double_blind_orcid_url_in_body",
                severity: PreflightSeverity::Error,
                message: "`orcid.org` reference in body_markdown — remove for double-blind submission"
                    .to_string(),
            });
        }
        if orcid_id_pattern().is_match(body) {
            findings.push(PreflightFinding {
                code: "double_blind_orcid_id_in_body",
                severity: PreflightSeverity::Error,
                message: "ORCID identifier pattern in body_markdown — remove for double-blind submission"
                    .to_string(),
            });
        }
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
        worthiness: None,
    }
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Build [`crate::publication_worthiness::WorthinessInputs`] from manifest + preflight (automated proxy only).
///
/// This is intentionally conservative: benchmark-style fields are weakly informed, and
/// [`crate::publication_worthiness::WorthinessInputs::meaningful_advance`] is always `false`.
#[must_use]
pub fn worthiness_inputs_from_manifest_and_preflight(
    manifest: &PublicationManifest,
    report: &PreflightReport,
) -> crate::publication_worthiness::WorthinessInputs {
    let r = (report.readiness_score as f64 / 100.0).clamp(0.0, 1.0);

    let mut red_line_violation_ids: Vec<String> = Vec::new();
    for f in &report.findings {
        if f.severity != PreflightSeverity::Error {
            continue;
        }
        match f.code {
            "citations_json_invalid"
            | "metadata_json_invalid"
            | "scientific_metadata_invalid"
            | "author_primary_mismatch" => {
                if !red_line_violation_ids
                    .iter()
                    .any(|x| x == "claim_evidence_mismatch")
                {
                    red_line_violation_ids.push("claim_evidence_mismatch".to_string());
                }
            }
            _ => {}
        }
    }

    let citation_score = match manifest.citations_json.as_deref() {
        Some(raw) if !raw.trim().is_empty() => match serde_json::from_str::<serde_json::Value>(raw)
        {
            Ok(v) if v.as_array().is_some_and(|a| !a.is_empty()) => clamp01(0.55 + 0.45 * r),
            Ok(_) => clamp01(0.35 + 0.35 * r),
            Err(_) => clamp01(0.3 * r),
        },
        _ => clamp01(0.35 * r),
    };

    let sci = parse_scientific_from_metadata_json(manifest.metadata_json.as_deref())
        .ok()
        .flatten();

    let repro_score = match sci.as_ref().and_then(|s| s.reproducibility.as_ref()) {
        Some(rep)
            if rep
                .code_repository_url
                .as_ref()
                .is_some_and(|s| !s.trim().is_empty())
                || rep
                    .data_repository_url
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
                || rep
                    .artifact_checksum_note
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty()) =>
        {
            clamp01(0.62 + 0.33 * r)
        }
        _ => clamp01(0.42 * r),
    };

    let meta_score = match &sci {
        Some(s) => {
            let mut pts = 0u32;
            const MAX: u32 = 5;
            if !s.authors.is_empty() {
                pts += 1;
            }
            if s.license_spdx
                .as_ref()
                .is_some_and(|x| !x.trim().is_empty())
            {
                pts += 1;
            }
            if s.funding_statement
                .as_ref()
                .is_some_and(|x| !x.trim().is_empty())
            {
                pts += 1;
            }
            if s.competing_interests_statement
                .as_ref()
                .is_some_and(|x| !x.trim().is_empty())
            {
                pts += 1;
            }
            if s.reproducibility.as_ref().is_some_and(|rep| {
                rep.code_repository_url
                    .as_ref()
                    .is_some_and(|x| !x.trim().is_empty())
                    || rep
                        .data_repository_url
                        .as_ref()
                        .is_some_and(|x| !x.trim().is_empty())
                    || rep
                        .artifact_checksum_note
                        .as_ref()
                        .is_some_and(|x| !x.trim().is_empty())
            }) {
                pts += 1;
            }
            clamp01(0.15 + (f64::from(pts) / f64::from(MAX)) * 0.75 * (0.5 + 0.5 * r))
        }
        None => clamp01(0.2 * r),
    };

    let ai_disclosure = match sci.as_ref().and_then(|s| s.ethics_and_impact.as_ref()) {
        Some(e)
            if e.broader_impact_statement
                .as_ref()
                .is_some_and(|x| !x.trim().is_empty())
                || e.irb_or_human_subjects_note
                    .as_ref()
                    .is_some_and(|x| !x.trim().is_empty()) =>
        {
            1.0
        }
        _ => 0.85,
    };

    let before_after = clamp01(0.35 * r);

    let abstract_boost = manifest
        .abstract_text
        .as_deref()
        .map_or(0.0, |s| if s.trim().is_empty() { 0.0 } else { 0.06 });

    let epistemic = clamp01(0.42 + 0.5 * r + abstract_boost);
    let novelty = clamp01(0.35 + 0.38 * r);
    let reliability = clamp01(0.48 + 0.47 * r);

    let mut inputs = crate::publication_worthiness::WorthinessInputs {
        red_line_violation_ids,
        repeated_unresolved_contradiction: false,
        claim_evidence_coverage: citation_score,
        artifact_replayability: repro_score,
        before_after_pair_integrity: before_after,
        metadata_completeness: meta_score,
        ai_disclosure_compliance: ai_disclosure,
        epistemic,
        reproducibility: repro_score,
        novelty,
        reliability,
        metadata_policy: meta_score,
        meaningful_advance: false,
    };
    if let Some(evidence) =
        crate::scientia_evidence::parse_scientia_evidence(manifest.metadata_json.as_deref())
    {
        inputs = crate::scientia_evidence::apply_scientia_evidence(inputs, &evidence);
    }
    inputs
}

/// Same as [`run_preflight`], then attaches [`PreflightReport::worthiness`] using `contract`.
#[must_use]
pub fn run_preflight_with_worthiness(
    manifest: &PublicationManifest,
    profile: PreflightProfile,
    contract: &crate::publication_worthiness::PublicationWorthinessContract,
) -> PreflightReport {
    let mut report = run_preflight(manifest, profile);
    let inputs = worthiness_inputs_from_manifest_and_preflight(manifest, &report);
    let eval = crate::publication_worthiness::evaluate_worthiness(contract, &inputs);
    report.worthiness = Some(eval);
    report
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
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci), None)
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
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci), None)
                .unwrap();
        let m = sample_manifest(|x| x.metadata_json = Some(meta));
        let r = run_preflight(&m, PreflightProfile::Default);
        assert!(!r.ok);
        assert!(
            r.findings
                .iter()
                .any(|f| f.code == "author_primary_mismatch")
        );
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

    #[test]
    fn double_blind_flags_orcid_in_body() {
        let m = sample_manifest(|x| {
            x.body_markdown = "See also https://orcid.org/0000-0002-1825-0097".to_string();
        });
        let r = run_preflight(&m, PreflightProfile::DoubleBlind);
        assert!(!r.ok);
        assert!(
            r.findings
                .iter()
                .any(|f| f.code == "double_blind_orcid_url_in_body")
        );
    }

    #[test]
    fn metadata_complete_errors_without_scientific_block() {
        let m = sample_manifest(|_| {});
        let r = run_preflight(&m, PreflightProfile::MetadataComplete);
        assert!(!r.ok);
        assert!(
            r.findings
                .iter()
                .any(|f| f.code == "scientific_metadata_required")
        );
    }

    #[test]
    fn metadata_complete_ok_when_fully_populated() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Ada Lovelace".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci), None)
                .unwrap();
        let m = sample_manifest(|x| x.metadata_json = Some(meta));
        let r = run_preflight(&m, PreflightProfile::MetadataComplete);
        assert!(r.ok, "{:?}", r.findings);
    }

    #[test]
    fn worthiness_attached_when_contract_provided() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "Ada Lovelace".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ethics_and_impact: Some(crate::scientific_metadata::EthicsAndImpactAttestation {
                broader_impact_statement: Some("Low risk.".to_string()),
                irb_or_human_subjects_note: None,
            }),
            ..Default::default()
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci), None)
                .unwrap();
        let mut m = sample_manifest(|x| {
            x.metadata_json = Some(meta);
            x.citations_json = Some("[{}]".to_string());
        });
        let yaml = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../contracts/scientia/publication-worthiness.default.yaml"
        ));
        let contract =
            crate::publication_worthiness::load_contract_from_str(yaml).expect("contract");
        let r = run_preflight_with_worthiness(&m, PreflightProfile::Default, &contract);
        assert!(r.worthiness.is_some());
        let w = r.worthiness.as_ref().expect("worthiness");
        assert_ne!(
            w.decision,
            crate::publication_worthiness::WorthinessDecision::Publish,
            "heuristic never claims Publish without meaningful_advance: {w:?}"
        );
        m.body_markdown = "Contact me at x@y.zz.".to_string();
        let r2 = run_preflight_with_worthiness(&m, PreflightProfile::DoubleBlind, &contract);
        assert!(!r2.ok);
        assert!(r2.worthiness.is_some());
    }
}
