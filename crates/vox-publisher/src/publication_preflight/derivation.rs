use super::*;
pub(super) fn collect_destination_readiness(manifest: &PublicationManifest) -> Vec<DestinationReadinessEntry> {
    let mut out = Vec::new();
    let zenodo_tok = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxZenodoAccessToken)
        .expose()
        .is_some_and(|s| !s.trim().is_empty());
    out.push(DestinationReadinessEntry {
        destination: "zenodo",
        ready: zenodo_tok,
        remediation: if zenodo_tok {
            String::new()
        } else {
            "Set Zenodo API token (Clavis `VoxZenodoAccessToken` / env alias per doctor)."
                .to_string()
        },
        credential_present: Some(zenodo_tok),
    });

    let or_token = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOpenReviewAccessToken)
        .expose()
        .is_some_and(|s| !s.trim().is_empty());
    let or_email = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOpenReviewEmail)
        .expose()
        .is_some_and(|s| !s.trim().is_empty());
    let or_pass = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOpenReviewPassword)
        .expose()
        .is_some_and(|s| !s.trim().is_empty());
    let openreview_ready = or_token || (or_email && or_pass);
    out.push(DestinationReadinessEntry {
        destination: "openreview",
        ready: openreview_ready,
        remediation: if openreview_ready {
            String::new()
        } else {
            "Provide OpenReview credentials (access token or email+password via Clavis)."
                .to_string()
        },
        credential_present: Some(openreview_ready),
    });

    let scholarly_adapter_configured = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxScholarlyAdapter)
        .expose()
        .is_some_and(|s| !s.trim().is_empty());
    out.push(DestinationReadinessEntry {
        destination: "scholarly_adapter",
        ready: scholarly_adapter_configured,
        remediation: if scholarly_adapter_configured {
            String::new()
        } else {
            "Set `VOX_SCHOLARLY_ADAPTER` when exercising scholarly submission adapters.".to_string()
        },
        credential_present: None,
    });

    let cred = operator_credential_presence();
    let social_ready =
        cred.twitter || cred.github || cred.open_collective || cred.reddit || cred.youtube;
    let social_detail = if social_ready {
        String::new()
    } else {
        "No social syndication credentials resolved — channels will stay dry-run or manual-assist."
            .to_string()
    };
    out.push(DestinationReadinessEntry {
        destination: "social_syndication",
        ready: social_ready,
        remediation: social_detail,
        credential_present: Some(social_ready),
    });

    // arXiv assist is always “package-ready” but never auto-submit.
    let arxiv_stub_ok =
        !manifest.title.trim().is_empty() && !manifest.body_markdown.trim().is_empty();
    out.push(DestinationReadinessEntry {
        destination: "arxiv_operator_assist",
        ready: arxiv_stub_ok,
        remediation: if arxiv_stub_ok {
            "Human operator must compile, verify, and upload to arXiv — tooling only stages handoff files."
                .to_string()
        } else {
            "Fill title/body before generating arXiv assist staging.".to_string()
        },
        credential_present: None,
    });

    out
}
pub(super) fn collect_manual_required(
    manifest: &PublicationManifest,
    attention: Option<&PreflightAttentionInputs>,
) -> Vec<ManualRequiredEntry> {
    let mut out = Vec::new();
    if let Some(raw) = manifest.metadata_json.as_deref()
        && !raw.trim().is_empty()
        && let Ok(root) = serde_json::from_str::<serde_json::Value>(raw)
        && root
            .get(crate::switching::LEGACY_METADATA_SYNDICATION_KEY)
            .is_some()
    {
        out.push(ManualRequiredEntry {
            code: "legacy_syndication_metadata_key",
            reason: format!(
                "metadata_json uses deprecated root key `{}`",
                crate::switching::LEGACY_METADATA_SYNDICATION_KEY
            ),
            severity: PreflightSeverity::Warning,
            next_action:
                "Prefer `metadata_json.syndication` as the canonical distribution envelope (legacy keys still merge at hydrate time)."
                    .to_string(),
            command_hint: None,
        });
    }

    if let Ok(item) = crate::switching::unified_news_item_from_manifest_parts(
        manifest.publication_id.as_str(),
        manifest.title.as_str(),
        manifest.author.as_str(),
        manifest.body_markdown.as_str(),
        manifest.metadata_json.as_deref(),
    ) {
        if item.syndication.hacker_news {
            out.push(ManualRequiredEntry {
                code: "hacker_news_manual_assist",
                reason: "Hacker News syndication uses manual-assist handoff (no posting API)."
                    .to_string(),
                severity: PreflightSeverity::Warning,
                next_action: "Complete the operator assist URL from the syndication outcome; keep an audit trail."
                    .to_string(),
                command_hint: Some(
                    "vox db publication-route-simulate --publication-id <id>".to_string(),
                ),
            });
        }
        if item.syndication.crates_io.is_some() {
            out.push(ManualRequiredEntry {
                code: "crates_io_not_automated",
                reason: "crates.io channel is modeled in policy but has no live publisher adapter."
                    .to_string(),
                severity: PreflightSeverity::Warning,
                next_action:
                    "Treat outcomes as explicit dry-run / not-implemented; use normal crate release tooling."
                        .to_string(),
                command_hint: None,
            });
        }
        if item.syndication.distribution_policy.approval_required == Some(true) {
            out.push(ManualRequiredEntry {
                code: "distribution_policy_approval_required",
                reason: "Manifest flags `distribution_policy.approval_required`.".to_string(),
                severity: PreflightSeverity::Warning,
                next_action:
                    "Record digest-bound approvals before any live fan-out (per publication policy)."
                        .to_string(),
                command_hint: Some(
                    "vox scientia publication-approve --publication-id <id> --approver <name>"
                        .to_string(),
                ),
            });
        }
        let cred = operator_credential_presence();
        if item.syndication.social.contains(&crate::types::SocialChannel::Twitter) && !cred.twitter {
            out.push(ManualRequiredEntry {
                code: "credential_twitter",
                reason: "Twitter is enabled in syndication but no operator bearer token resolved."
                    .to_string(),
                severity: PreflightSeverity::Warning,
                next_action: "Configure the Twitter bearer token / Clavis mapping for this shell."
                    .to_string(),
                command_hint: Some("vox clavis doctor".to_string()),
            });
        }
        if item.syndication.forge.is_some() && !cred.github {
            out.push(ManualRequiredEntry {
                code: "credential_github",
                reason: "GitHub syndication is enabled but no operator token resolved.".to_string(),
                severity: PreflightSeverity::Warning,
                next_action: "Configure `VOX_NEWS_GITHUB_TOKEN` / GitHub token via Clavis."
                    .to_string(),
                command_hint: Some("vox clavis doctor".to_string()),
            });
        }
        if item.syndication.open_collective.is_some() && !cred.open_collective {
            out.push(ManualRequiredEntry {
                code: "credential_open_collective",
                reason: "Open Collective syndication is enabled but no operator token resolved."
                    .to_string(),
                severity: PreflightSeverity::Warning,
                next_action: "Configure `VOX_NEWS_OPENCOLLECTIVE_TOKEN` / Clavis mapping."
                    .to_string(),
                command_hint: Some("vox clavis doctor".to_string()),
            });
        }
        if item.syndication.reddit.is_some() && !cred.reddit {
            out.push(ManualRequiredEntry {
                code: "credential_reddit",
                reason:
                    "Reddit syndication is enabled but OAuth client credentials are incomplete."
                        .to_string(),
                severity: PreflightSeverity::Warning,
                next_action: "Set `VOX_SOCIAL_REDDIT_*` secrets per Clavis SSOT.".to_string(),
                command_hint: Some("vox clavis doctor".to_string()),
            });
        }
        if item.syndication.youtube.is_some() && !cred.youtube {
            out.push(ManualRequiredEntry {
                code: "credential_youtube",
                reason:
                    "YouTube syndication is enabled but OAuth refresh credentials are incomplete."
                        .to_string(),
                severity: PreflightSeverity::Warning,
                next_action: "Set `VOX_SOCIAL_YOUTUBE_*` secrets per Clavis SSOT.".to_string(),
                command_hint: Some("vox clavis doctor".to_string()),
            });
        }
    }

    if let Some(att) = attention
        && let Some(ref gate) = att.gate
        && gate.would_be_live_without_dry_run
        && !gate.live_publish_allowed
    {
        for br in &gate.blocking_reasons {
            let (code, hint): (&'static str, Option<String>) = match br.code.as_str() {
                "missing_dual_approval" => (
                    "live_publish_dual_approval",
                    Some(
                        "vox scientia publication-approve --publication-id <id> --approver <name>"
                            .to_string(),
                    ),
                ),
                "publish_not_armed" => (
                    "live_publish_not_armed",
                    Some(
                        "export VOX_NEWS_PUBLISH_ARMED=1 (and/or orchestrator [news].publish_armed)"
                            .to_string(),
                    ),
                ),
                "missing_db" => (
                    "live_publish_db",
                    Some("Attach Turso/VoxDb for this shell or MCP server.".to_string()),
                ),
                _ => ("live_publish_blocked", None),
            };
            out.push(ManualRequiredEntry {
                code,
                reason: br.message.clone(),
                severity: PreflightSeverity::Error,
                next_action: "Resolve this gate before attempting a live syndication fan-out."
                    .to_string(),
                command_hint: hint,
            });
        }
    }
    out
}
pub(super) fn manifest_has_explicit_distribution_intent(manifest: &PublicationManifest) -> bool {
    let Some(raw) = manifest.metadata_json.as_deref() else {
        return false;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let Ok(root) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return false;
    };
    root.get("syndication").is_some()
        || root
            .get(crate::switching::LEGACY_METADATA_SYNDICATION_KEY)
            .is_some()
        || root.get("topic_pack").is_some()
}
pub(super) fn derive_next_actions(
    manifest: &PublicationManifest,
    findings: &[PreflightFinding],
    manual_required: &[ManualRequiredEntry],
) -> Vec<NextActionEntry> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    let mut push =
        |code: &'static str, summary: String, reason: String, command_hint: Option<String>| {
            if seen.insert(code.to_string()) {
                out.push(NextActionEntry {
                    code,
                    summary,
                    reason,
                    command_hint,
                });
            }
        };

    let error_count = findings
        .iter()
        .filter(|f| f.severity == PreflightSeverity::Error)
        .count();
    let warning_count = findings
        .iter()
        .filter(|f| f.severity == PreflightSeverity::Warning)
        .count();

    if error_count > 0 {
        push(
            "fix_preflight_errors",
            format!("Resolve {error_count} blocking preflight error(s) first."),
            "Readiness errors will block the shortest safe path to scholarly submission and increase operator churn.".to_string(),
            Some(
                "vox scientia publication-preflight --publication-id <id> --profile default"
                    .to_string(),
            ),
        );
    } else if warning_count > 0 {
        push(
            "review_preflight_warnings",
            format!("Review {warning_count} non-blocking preflight warning(s)."),
            "Warnings are often fixable boilerplate gaps that improve publication metadata quality before submit.".to_string(),
            Some(
                "vox scientia publication-preflight --publication-id <id> --with-worthiness"
                    .to_string(),
            ),
        );
    }

    let mut manual_sorted: Vec<&ManualRequiredEntry> = manual_required.iter().collect();
    manual_sorted.sort_by_key(|m| match m.severity {
        PreflightSeverity::Error => 0_u8,
        PreflightSeverity::Warning => 1_u8,
    });
    for manual in manual_sorted {
        push(
            manual.code,
            manual.next_action.clone(),
            manual.reason.clone(),
            manual.command_hint.clone(),
        );
    }

    if error_count == 0 {
        push(
            "run_default_scholarly_pipeline",
            "Use `publication-scholarly-pipeline-run` as the default scholarly happy path.".to_string(),
            "That command reuses preflight, approval gating, optional staging, and submit so the operator does not have to hand-orchestrate each step.".to_string(),
            Some(
                "vox scientia publication-scholarly-pipeline-run --publication-id <id> --dry-run"
                    .to_string(),
            ),
        );
    }

    if let Ok(item) = crate::switching::unified_news_item_from_manifest_parts(
        manifest.publication_id.as_str(),
        manifest.title.as_str(),
        manifest.author.as_str(),
        manifest.body_markdown.as_str(),
        manifest.metadata_json.as_deref(),
    ) {
        let has_non_rss_social_targets = item.syndication.is_active(crate::types::SocialChannel::Twitter)
            || item.syndication.is_active(crate::types::SocialChannel::Bluesky)
            || item.syndication.is_active(crate::types::SocialChannel::Mastodon)
            || item.syndication.is_active(crate::types::SocialChannel::Discord)
            || item.syndication.forge.is_some()
            || item.syndication.open_collective.is_some()
            || item.syndication.reddit.is_some()
            || item.syndication.hacker_news
            || item.syndication.youtube.is_some()
            || item.syndication.crates_io.is_some();
        if manifest_has_explicit_distribution_intent(manifest) || has_non_rss_social_targets {
            push(
                "simulate_social_routing",
                "Run route simulation before social fan-out.".to_string(),
                "Simulation shows channel policy, retries, and disabled-path reasons without spending approvals or posting live content.".to_string(),
                Some("vox db publication-route-simulate --publication-id <id>".to_string()),
            );
        }
        if error_count == 0 && has_non_rss_social_targets {
            push(
                "dry_run_social_publish",
                "Dry-run the configured social channels before any live publish.".to_string(),
                "A dry-run verifies effective routing and payload generation while keeping irreversible platform actions manual and explicit.".to_string(),
                Some(
                    "vox db publication-publish --publication-id <id> --dry-run true"
                        .to_string(),
                ),
            );
        }
    }

    out
}
pub(super) fn derive_confidence(
    findings: &[PreflightFinding],
    manual: &[ManualRequiredEntry],
) -> PreflightConfidence {
    let finding_err = findings
        .iter()
        .any(|f| f.severity == PreflightSeverity::Error);
    let manual_err = manual
        .iter()
        .any(|m| m.severity == PreflightSeverity::Error);
    if finding_err || manual_err {
        return PreflightConfidence::ManualRequired;
    }
    if !manual.is_empty()
        || findings
            .iter()
            .any(|f| f.severity == PreflightSeverity::Warning)
    {
        return PreflightConfidence::AutoWithReview;
    }
    PreflightConfidence::AutoSafe
}
pub(super) fn profile_label(profile: PreflightProfile) -> &'static str {
    match profile {
        PreflightProfile::Default => "default",
        PreflightProfile::DoubleBlind => "double_blind",
        PreflightProfile::MetadataComplete => "metadata_complete",
        PreflightProfile::ArxivAssist => "arxiv_assist",
        PreflightProfile::NewsInbound => "news_inbound",
    }
}
pub(super) fn email_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}")
            .expect("email preflight regex")
    })
}
/// ORCID id pattern (checksum digit may be `X`).
pub(super) fn orcid_id_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(r"\b", r"\d{4}-\d{4}-\d{4}-\d{3}", r"[0-9X]", r"\b"))
            .expect("orcid id preflight regex")
    })
}
pub(super) fn normalize_person_name(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
