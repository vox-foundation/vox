//! `vox scientia scout` — Phase F handler.
//!
//! Wraps `vox-scientia-producers`'s `ProducerRegistry` in a single command
//! that surveys the current workspace, persists new candidates to
//! `scientia_finding_candidates`, and renders a report.

use anyhow::{Context, Result};
use vox_cli_core::scientia::ScoutOutput;
use vox_db::VoxDb;
use vox_db::store::{FindingCandidateClass, FindingCandidateRow, InsertOutcome};
use vox_research_events::ResearchEvent;
use vox_research_events::schema_types::{
    DiscoverySignalFamily, FindingCandidateV1, WorthinessProfile,
};
use vox_scientia::inspect_bridge::novelty::NoveltyVerdict;
use vox_scientia::producers::{
    ProducerContext, ProducerRegistry, WorthinessSignalsV2, evaluate_worthiness_v2,
};

/// Entry point invoked from `commands::scientia`.
pub async fn run(
    commit_window: usize,
    days_window: u32,
    output: ScoutOutput,
    candidate_class: Option<String>,
) -> Result<()> {
    let codex = VoxDb::connect_default()
        .await
        .context("connect to default Codex / VoxDb")?;

    let repo_root = std::env::current_dir().context("current_dir")?;
    let now_ms = chrono::Utc::now().timestamp_millis();
    let session_id = format!("scout-{now_ms}");

    let ctx = ProducerContext {
        repo_root,
        commit_window,
        days_window,
        now_ms,
        session_id: session_id.clone(),
        repository_id: None,
    };

    let registry = ProducerRegistry::default_with_codex(codex.clone());
    let events = registry.run_all(&ctx).await;

    let inserted = persist_new_candidates(&codex, &events, now_ms).await?;

    if std::env::var("VOX_SCOUT_CODE_UNIQUENESS").ok().as_deref() == Some("1") {
        let hook = crate::db::publication::scout_code_uniqueness_hook(&codex, &ctx.repo_root).await;
        eprintln!(
            "scout code_uniqueness: {}",
            serde_json::to_string(&hook).unwrap_or_else(|_| "{}".into())
        );
    }

    let filtered: Vec<&ResearchEvent> = events
        .iter()
        .filter(|e| matches_class_filter(e, candidate_class.as_deref()))
        .collect();

    match output {
        ScoutOutput::Table => {
            print_table(&filtered);
            eprintln!(
                "scout: {} candidate(s) shown ({} producers ran; {} new persisted)",
                filtered.len(),
                registry.len(),
                inserted,
            );
        }
        ScoutOutput::Json => print_json(&filtered)?,
    }
    Ok(())
}

/// Persist `FindingCandidateProposed` events as `scientia_finding_candidates`
/// rows and promote only two-stage Novelty x Utility `strong_candidate` rows
/// into `scientia_discovery_inbox`. Returns newly inserted ledger count.
async fn persist_new_candidates(
    codex: &VoxDb,
    events: &[ResearchEvent],
    now_ms: i64,
) -> Result<usize> {
    let mut inserted = 0;
    for ev in events {
        if let ResearchEvent::FindingCandidateProposed {
            finding_id,
            worthiness_score,
            finding_candidate,
            ..
        } = ev
        {
            let class = class_from_finding_id(finding_id);
            let producer = producer_from_finding_id(finding_id);
            let candidate_parsed: Option<FindingCandidateV1> = finding_candidate
                .as_ref()
                .and_then(|v| serde_json::from_value::<FindingCandidateV1>(v.clone()).ok());
            let signals_v2 = derive_worthiness_signals_v2(
                finding_id,
                *worthiness_score,
                candidate_parsed.as_ref(),
                producer,
            );
            let eval = evaluate_worthiness_v2(&signals_v2);
            let internal_signals_json = candidate_parsed
                .as_ref()
                .and_then(|fc| serde_json::to_string(&fc.internal_signals).ok())
                .unwrap_or_else(|| internal_signals_json_for_producer(producer));

            let row = FindingCandidateRow {
                candidate_id: finding_id.clone(),
                candidate_class: class,
                publication_id: candidate_parsed
                    .as_ref()
                    .and_then(|fc| fc.publication_id.clone()),
                title_hint: candidate_parsed
                    .as_ref()
                    .and_then(|fc| fc.title_hint.clone()),
                internal_signals_json,
                novelty_evidence_bundle_id: candidate_parsed
                    .as_ref()
                    .and_then(|fc| fc.novelty_evidence_bundle_id.clone()),
                worthiness_decision_ref: Some(format!(
                    "worthiness-v2:{}",
                    eval.tier.as_intake_tier_str()
                )),
                confidence_json: Some(
                    serde_json::json!({
                        "signal_strength": sane_float(*worthiness_score),
                        "novelty_score": eval.novelty_score,
                        "utility_score": eval.utility_score,
                        "composite_score": eval.composite_score,
                        "novelty_passed": eval.novelty_passed,
                        "utility_passed": eval.utility_passed,
                    })
                    .to_string(),
                ),
                repository_id: None,
                producer_name: producer.to_string(),
                signal_fingerprint: finding_id.clone(),
                created_at_ms: now_ms,
                updated_at_ms: now_ms,
            };
            match codex.insert_finding_candidate(&row).await {
                Ok(InsertOutcome::Inserted) => {
                    inserted += 1;
                    if eval.should_surface_to_inbox() {
                        let signal_codes: Vec<String> = candidate_parsed
                            .as_ref()
                            .map(|fc| {
                                fc.internal_signals
                                    .iter()
                                    .map(|s| s.code.clone())
                                    .collect::<Vec<_>>()
                            })
                            .filter(|codes| !codes.is_empty())
                            .unwrap_or_else(|| vec!["WORTHINESS_V2_STRONG".to_string()]);
                        let codes_json = serde_json::to_string(&signal_codes)
                            .unwrap_or_else(|_| "[\"WORTHINESS_V2_STRONG\"]".to_string());
                        let target_id = row
                            .publication_id
                            .as_deref()
                            .unwrap_or(row.candidate_id.as_str());
                        if let Err(e) = codex
                            .insert_discovery_inbox(
                                target_id,
                                now_ms,
                                eval.tier.as_intake_tier_str(),
                                &codes_json,
                            )
                            .await
                        {
                            return Err(anyhow::anyhow!(
                                "scout: insert_discovery_inbox({}): {e}",
                                target_id
                            ));
                        }
                    }
                }
                Ok(InsertOutcome::AlreadySeen) => {}
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "scout: insert_finding_candidate({}): {e}",
                        finding_id
                    ));
                }
            }
        }
    }
    Ok(inserted)
}

fn derive_worthiness_signals_v2(
    finding_id: &str,
    worthiness_score: f64,
    candidate: Option<&FindingCandidateV1>,
    producer: &str,
) -> WorthinessSignalsV2 {
    let score = sane_float(worthiness_score).clamp(0.0, 1.0);
    let signals = candidate
        .map(|fc| fc.internal_signals.as_slice())
        .unwrap_or(&[]);

    let has_not_novel_signal = signals.iter().any(|s| {
        let lc = s.code.to_ascii_lowercase();
        let ls = s.summary.to_ascii_lowercase();
        lc.contains("not_novel") || ls.contains("prior_art_not_novel")
    });
    let has_possibly_novel_signal = signals.iter().any(|s| {
        let lc = s.code.to_ascii_lowercase();
        lc.contains("possibly_novel")
    });

    let (novelty_verdict, novelty_score) = if has_not_novel_signal {
        (
            NoveltyVerdict::NotNovel {
                closest_hit_uri: String::new(),
                similarity: 0.9,
            },
            0.0,
        )
    } else if has_possibly_novel_signal {
        (NoveltyVerdict::PossiblyNovel { closest_score: 0.6 }, 0.4)
    } else {
        (NoveltyVerdict::Novel, score)
    };

    let empirical_delta = matches!(producer, "bench_history" | "mens_training_run")
        || signals.iter().any(|s| {
            matches!(
                s.family,
                DiscoverySignalFamily::BenchmarkPair | DiscoverySignalFamily::MensScorecard
            ) || s.code.to_ascii_uppercase().contains("DELTA")
                || s.code.to_ascii_uppercase().contains("BENCH")
        });

    let ci_improvement = producer == "test_corpus"
        || signals
            .iter()
            .any(|s| s.family == DiscoverySignalFamily::EvalGate);

    let repro_from_conf = candidate
        .and_then(|fc| fc.confidence.as_ref())
        .and_then(|c| c.reproducibility_support)
        .is_some_and(|r| r >= 0.70);
    let reproducibility_proof = repro_from_conf
        || signals
            .iter()
            .any(|s| s.family == DiscoverySignalFamily::ReproducibilityArtifact);

    let regression_fix = signals.iter().any(|s| {
        let lc = s.code.to_ascii_lowercase();
        let ls = s.summary.to_ascii_lowercase();
        lc.contains("regression") || ls.contains("regression fix")
    });

    let claim_grounding = candidate
        .and_then(|fc| fc.confidence.as_ref())
        .and_then(|c| c.signal_strength)
        .map(|v| sane_float(v).clamp(0.0, 1.0))
        .unwrap_or_else(|| {
            if matches!(producer, "bench_history" | "mens_training_run")
                || finding_id.contains("-bench-")
            {
                score
            } else {
                0.0
            }
        });

    WorthinessSignalsV2 {
        novelty_verdict,
        novelty_score,
        empirical_delta,
        ci_improvement,
        reproducibility_proof,
        regression_fix,
        claim_grounding,
        profile: WorthinessProfile::Repository,
    }
}

fn matches_class_filter(ev: &ResearchEvent, class_filter: Option<&str>) -> bool {
    let Some(filter) = class_filter else {
        return true;
    };
    match ev {
        ResearchEvent::FindingCandidateProposed { finding_id, .. } => {
            class_from_finding_id(finding_id).as_sql() == filter
        }
        _ => false,
    }
}

/// Map a finding_id prefix back to its `FindingCandidateClass`. Prefixes are
/// minted by producers in `vox-scientia-producers` (see the per-producer
/// `id_prefix()` / format strings).
fn class_from_finding_id(id: &str) -> FindingCandidateClass {
    if id.starts_with("algimp-") {
        FindingCandidateClass::AlgorithmicImprovement
    } else if id.starts_with("repinf-") {
        FindingCandidateClass::ReproducibilityInfra
    } else if id.starts_with("teltr-") {
        FindingCandidateClass::TelemetryTrust
    } else if id.starts_with("polgov-") {
        FindingCandidateClass::PolicyGovernance
    } else {
        FindingCandidateClass::Other
    }
}

/// Map a finding_id to its producer name. Used for the
/// `(producer_name, signal_fingerprint)` UNIQUE index in
/// `scientia_finding_candidates`.
fn producer_from_finding_id(id: &str) -> &'static str {
    // The middle token before the final sha7 distinguishes producers; for
    // now we infer from the prefix. (algimp- can come from commit_graph or
    // bench_history; the second token "bench" distinguishes the latter.)
    if id.contains("-bench-") {
        "bench_history"
    } else if id.contains("-trust-") || id.contains("-socrates-") {
        "socrates_telemetry"
    } else if id.contains("-mens-") {
        "mens_training_run"
    } else if id.contains("-test-") {
        "test_corpus"
    } else {
        "commit_graph"
    }
}

/// Scout hook: commit_graph findings should be corroborated via code_uniqueness.
fn internal_signals_json_for_producer(producer: &str) -> String {
    if producer == "commit_graph" {
        return r#"[{"code":"code_uniqueness","strength":"supporting","summary":"corroborate via discovery-watch embedding kNN"}]"#.into();
    }
    "[]".into()
}

fn sane_float(f: f64) -> f64 {
    if f.is_finite() { f } else { 0.0 }
}

fn print_table(events: &[&ResearchEvent]) {
    println!("{:42} {:26} {:>5}", "candidate-id", "class", "score");
    println!("{}", "-".repeat(42 + 1 + 26 + 1 + 5));
    for ev in events {
        if let ResearchEvent::FindingCandidateProposed {
            finding_id,
            worthiness_score,
            ..
        } = ev
        {
            let class = class_from_finding_id(finding_id);
            println!(
                "{:42} {:26} {:>5.2}",
                truncate(finding_id, 42),
                class.as_sql(),
                sane_float(*worthiness_score)
            );
        }
    }
}

fn print_json(events: &[&ResearchEvent]) -> Result<()> {
    // Serialize a stripped record for stability instead of the raw enum.
    #[derive(serde::Serialize)]
    struct Row<'a> {
        candidate_id: &'a str,
        candidate_class: &'a str,
        worthiness_score: f64,
        session_id: &'a str,
    }
    let rows: Vec<Row<'_>> = events
        .iter()
        .filter_map(|e| match e {
            ResearchEvent::FindingCandidateProposed {
                finding_id,
                worthiness_score,
                session_id,
                ..
            } => Some(Row {
                candidate_id: finding_id,
                candidate_class: class_from_finding_id(finding_id).as_sql(),
                worthiness_score: sane_float(*worthiness_score),
                session_id,
            }),
            _ => None,
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max - 1).collect();
        format!("{cut}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finding_id_prefix_maps_back_to_class() {
        assert_eq!(
            class_from_finding_id("algimp-2026-05-15-deadbeef"),
            FindingCandidateClass::AlgorithmicImprovement
        );
        assert_eq!(
            class_from_finding_id("repinf-2026-05-15-deadbeef"),
            FindingCandidateClass::ReproducibilityInfra
        );
        assert_eq!(
            class_from_finding_id("teltr-2026-05-15-trust-abcd"),
            FindingCandidateClass::TelemetryTrust
        );
        assert_eq!(
            class_from_finding_id("polgov-2026-05-15-deadbeef"),
            FindingCandidateClass::PolicyGovernance
        );
        assert_eq!(
            class_from_finding_id("unknown"),
            FindingCandidateClass::Other
        );
    }

    #[test]
    fn producer_name_inferred_from_finding_id_subtoken() {
        assert_eq!(
            producer_from_finding_id("algimp-2026-05-15-bench-1234"),
            "bench_history"
        );
        assert_eq!(
            producer_from_finding_id("teltr-2026-05-15-trust-1234"),
            "socrates_telemetry"
        );
        assert_eq!(
            producer_from_finding_id("algimp-2026-05-15-abcdef0"),
            "commit_graph"
        );
    }

    #[test]
    fn class_filter_matches_only_requested_class() {
        let ev = ResearchEvent::FindingCandidateProposed {
            finding_id: "algimp-x".into(),
            claim_ids: vec![],
            worthiness_score: 0.5,
            session_id: "s".into(),
            finding_candidate: None,
        };
        assert!(matches_class_filter(&ev, None));
        assert!(matches_class_filter(&ev, Some("algorithmic_improvement")));
        assert!(!matches_class_filter(&ev, Some("telemetry_trust")));
    }

    #[test]
    fn truncate_handles_long_strings() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("0123456789abc", 10), "012345678…");
    }

    #[test]
    fn commit_graph_scout_rows_hint_code_uniqueness_corroboration() {
        let json = internal_signals_json_for_producer("commit_graph");
        assert!(json.contains("code_uniqueness"));
        assert_eq!(internal_signals_json_for_producer("bench_history"), "[]");
    }

    #[test]
    fn worthiness_v2_gating_holds_low_utility_noise_and_surfaces_verified_delta() {
        // Low-utility commit_graph churn stays in ledger (not surfaced to discovery_inbox)
        let low_util =
            derive_worthiness_signals_v2("algimp-2026-05-15-abcdef01", 0.90, None, "commit_graph");
        let low_eval = evaluate_worthiness_v2(&low_util);
        assert!(!low_eval.should_surface_to_inbox());

        // Verified benchmark delta passes two-stage Novelty x Utility gate
        let strong_bench = derive_worthiness_signals_v2(
            "algimp-2026-05-15-bench-12345678",
            0.85,
            None,
            "bench_history",
        );
        let strong_eval = evaluate_worthiness_v2(&strong_bench);
        assert!(strong_eval.should_surface_to_inbox());
        assert_eq!(strong_eval.tier.as_intake_tier_str(), "strong_candidate");
    }
}
