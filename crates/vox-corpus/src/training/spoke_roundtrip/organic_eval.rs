use super::*;

/// Evaluate a single [`OrganicSearchSpokeCase`] end-to-end from workspace search (both standard and
/// unbiased filename-stripped BM25+RRF) through RAG distractor verification and deep compilation (`ORG-SPOKE-01..05`).
#[must_use]
pub fn evaluate_organic_search_spoke_case(
    case: &OrganicSearchSpokeCase,
) -> OrganicSearchSpokeReceipt {
    use crate::training::contract::OrganicSpokeKind;

    let start = std::time::Instant::now();
    let repo_root = workspace_root_dir();

    let hits =
        discover_workspace_evidence(&repo_root, &case.search_query, &case.search_scope_paths, 5);
    let top_retrieved_path = hits
        .first()
        .map(|h| h.0.clone())
        .unwrap_or_else(|| "<none>".to_string());

    let matched_rank = hits
        .iter()
        .position(|(p, _, _)| p.contains(&case.expected_top_path_hint))
        .map(|idx| idx + 1);
    let reciprocal_rank = matched_rank.map_or(0.0, |r| 1.0 / (r as f64));

    // Unbiased filename-stripped search (allow_path_boost = false)
    let stripped_query = strip_filename_hints(&case.search_query);
    let unbiased_hits = discover_workspace_evidence_with_mode(
        &repo_root,
        &stripped_query,
        &case.search_scope_paths,
        5,
        false,
    );
    let unbiased_matched_rank = unbiased_hits
        .iter()
        .position(|(p, _, _)| p.contains(&case.expected_top_path_hint))
        .map(|idx| idx + 1);
    let unbiased_reciprocal_rank = unbiased_matched_rank.map_or(0.0, |r| 1.0 / (r as f64));

    let combined_evidence: String = hits
        .iter()
        .map(|(p, _, c)| format!("{p}\n{c}"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    let matched_tokens = case
        .expected_grounding_tokens
        .iter()
        .filter(|tok| combined_evidence.contains(&tok.to_ascii_lowercase()))
        .count();
    let evidence_grounding_ratio = if case.expected_grounding_tokens.is_empty() {
        1.0
    } else {
        matched_tokens as f64 / (case.expected_grounding_tokens.len() as f64)
    };
    let search_recalled = matched_rank.is_some() && evidence_grounding_ratio >= 0.80;

    let first_cand = case
        .roundtrip_case
        .candidates
        .first()
        .map(String::as_str)
        .unwrap_or("");
    let (rag_context_verified, distractor_resilient, _rag_detail) =
        verify_rag_context_grounding_and_distractor_resilience(
            &hits,
            &case.expected_top_path_hint,
            &case.expected_grounding_tokens,
            first_cand,
        );

    let roundtrip_receipt = match case.spoke {
        OrganicSpokeKind::SpeechToCode => {
            verify_speech_to_code_roundtrip(&case.case_id, &case.roundtrip_case.prompt, first_cand)
        }
        OrganicSpokeKind::PopuliMesh => {
            verify_populi_mesh_roundtrip(&case.case_id, &case.roundtrip_case.prompt, first_cand)
        }
        OrganicSpokeKind::SkillCreation => {
            verify_skill_creation_roundtrip(&case.case_id, &case.roundtrip_case.prompt, first_cand)
        }
        _ => evaluate_spoke_case(&case.roundtrip_case),
    };

    let (compilation_verified, compilation_stage_detail) = verify_deep_compilation_for_spoke(
        &repo_root,
        case.spoke,
        &case.roundtrip_case,
        &roundtrip_receipt,
    );

    let latency_us = start.elapsed().as_micros().max(1) as u64;

    OrganicSearchSpokeReceipt {
        case_id: case.case_id.clone(),
        spoke: case.spoke,
        search_query: case.search_query.clone(),
        top_retrieved_path,
        matched_rank,
        reciprocal_rank,
        unbiased_matched_rank,
        unbiased_reciprocal_rank,
        evidence_grounding_ratio,
        search_recalled,
        rag_context_verified,
        distractor_resilient,
        compilation_verified,
        compilation_stage_detail,
        latency_us,
        roundtrip_receipt,
    }
}

/// Evaluate a full suite of [`OrganicSearchSpokeCase`]s and aggregate [`OrganicSearchSpokeScorecardV1`] across all 8 spokes (`ORG-SPOKE-06`, `SPOKE8-03`).
#[must_use]
pub fn evaluate_organic_search_spoke_suite(
    cases: &[OrganicSearchSpokeCase],
) -> OrganicSearchSpokeScorecardV1 {
    use crate::training::contract::OrganicSpokeKind;

    let receipts: Vec<OrganicSearchSpokeReceipt> = cases
        .iter()
        .map(evaluate_organic_search_spoke_case)
        .collect();

    let mut spokes = Vec::with_capacity(OrganicSpokeKind::ALL.len());
    for spoke in OrganicSpokeKind::ALL {
        let sr: Vec<&OrganicSearchSpokeReceipt> =
            receipts.iter().filter(|r| r.spoke == spoke).collect();
        let total = sr.len();
        if total == 0 {
            continue;
        }
        let denom = total as f64;
        let search_recall_at_5 = sr.iter().filter(|r| r.search_recalled).count() as f64 / denom;
        let mean_reciprocal_rank = sr.iter().map(|r| r.reciprocal_rank).sum::<f64>() / denom;
        let unbiased_search_recall_at_5 = sr
            .iter()
            .filter(|r| r.unbiased_matched_rank.is_some())
            .count() as f64
            / denom;
        let unbiased_mrr = sr.iter().map(|r| r.unbiased_reciprocal_rank).sum::<f64>() / denom;
        let mean_grounding_ratio =
            sr.iter().map(|r| r.evidence_grounding_ratio).sum::<f64>() / denom;
        let distractor_resilience_rate =
            sr.iter().filter(|r| r.distractor_resilient).count() as f64 / denom;
        let syntax_validity_rate = sr
            .iter()
            .filter(|r| r.roundtrip_receipt.syntax_valid)
            .count() as f64
            / denom;
        let semantic_validity_rate = sr
            .iter()
            .filter(|r| r.roundtrip_receipt.semantic_valid)
            .count() as f64
            / denom;
        let compilation_pass_rate =
            sr.iter().filter(|r| r.compilation_verified).count() as f64 / denom;
        let end_to_end_pass_at_1 = sr
            .iter()
            .filter(|r| {
                r.search_recalled
                    && r.compilation_verified
                    && r.roundtrip_receipt.pass_at_1
                    && r.rag_context_verified
                    && r.distractor_resilient
            })
            .count() as f64
            / denom;
        let mean_latency_us = sr.iter().map(|r| r.latency_us).sum::<u64>() / (total as u64);

        spokes.push(OrganicSpokeScorecardEntry {
            spoke,
            total_cases: total,
            search_recall_at_5,
            mean_reciprocal_rank,
            unbiased_search_recall_at_5,
            unbiased_mrr,
            mean_grounding_ratio,
            distractor_resilience_rate,
            syntax_validity_rate,
            semantic_validity_rate,
            compilation_pass_rate,
            end_to_end_pass_at_1,
            mean_latency_us,
        });
    }

    let total_cases = receipts.len();
    let denom = total_cases.max(1) as f64;
    let overall_search_recall_at_5 =
        receipts.iter().filter(|r| r.search_recalled).count() as f64 / denom;
    let overall_mrr = receipts.iter().map(|r| r.reciprocal_rank).sum::<f64>() / denom;
    let overall_unbiased_search_recall_at_5 = receipts
        .iter()
        .filter(|r| r.unbiased_matched_rank.is_some())
        .count() as f64
        / denom;
    let overall_unbiased_mrr = receipts
        .iter()
        .map(|r| r.unbiased_reciprocal_rank)
        .sum::<f64>()
        / denom;
    let overall_grounding_ratio = receipts
        .iter()
        .map(|r| r.evidence_grounding_ratio)
        .sum::<f64>()
        / denom;
    let overall_distractor_resilience_rate =
        receipts.iter().filter(|r| r.distractor_resilient).count() as f64 / denom;
    let overall_compilation_pass_rate =
        receipts.iter().filter(|r| r.compilation_verified).count() as f64 / denom;
    let overall_end_to_end_pass_at_1 = receipts
        .iter()
        .filter(|r| {
            r.search_recalled
                && r.compilation_verified
                && r.roundtrip_receipt.pass_at_1
                && r.rag_context_verified
                && r.distractor_resilient
        })
        .count() as f64
        / denom;

    OrganicSearchSpokeScorecardV1 {
        schema_version: OrganicSearchSpokeScorecardV1::SCHEMA_VERSION.to_string(),
        total_cases,
        overall_search_recall_at_5,
        overall_mrr,
        overall_unbiased_search_recall_at_5,
        overall_unbiased_mrr,
        overall_grounding_ratio,
        overall_distractor_resilience_rate,
        overall_compilation_pass_rate,
        overall_end_to_end_pass_at_1,
        spokes,
        receipts,
    }
}

/// Persist an [`OrganicSearchSpokeScorecardV1`] into `VoxDb` CAS (`ORG-SPOKE-06`).
#[cfg(feature = "database")]
pub async fn persist_organic_scorecard_to_db(
    db: &vox_db::VoxDb,
    run_id: &str,
    scorecard: &OrganicSearchSpokeScorecardV1,
) -> anyhow::Result<String> {
    let payload = serde_json::to_vec(scorecard)?;
    let hash = db.store("mens_organic_search_scorecard", &payload).await?;
    db.bind_name("mens_organic_search_scorecard", run_id, &hash)
        .await?;
    Ok(format!("trusty://mens-organic-scorecard/{hash}"))
}

/// Load an [`OrganicSearchSpokeScorecardV1`] from `VoxDb` CAS by `run_id` (`ORG-SPOKE-06`).
#[cfg(feature = "database")]
pub async fn load_organic_scorecard_from_db(
    db: &vox_db::VoxDb,
    run_id: &str,
) -> anyhow::Result<OrganicSearchSpokeScorecardV1> {
    let names = db
        .list_names_in_namespace("mens_organic_search_scorecard")
        .await?;
    let hash = names
        .into_iter()
        .find(|(name, _)| name == run_id)
        .map(|(_, h)| h)
        .ok_or_else(|| anyhow::anyhow!("organic scorecard `{run_id}` not found in VoxDb"))?;
    let bytes = db.get(&hash).await?;
    let scorecard: OrganicSearchSpokeScorecardV1 = serde_json::from_slice(&bytes)?;
    Ok(scorecard)
}

/// Enumerate all persisted [`OrganicSearchSpokeScorecardV1`] runs in `VoxDb` CAS (`PERSIST-BENCH-02`).
#[cfg(feature = "database")]
pub async fn list_organic_scorecard_history_from_db(
    db: &vox_db::VoxDb,
) -> anyhow::Result<Vec<(String, String, OrganicSearchSpokeScorecardV1)>> {
    let mut names = db
        .list_names_in_namespace("mens_organic_search_scorecard")
        .await?;
    names.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::with_capacity(names.len());
    for (run_id, hash) in names {
        let bytes = db.get(&hash).await?;
        let scorecard: OrganicSearchSpokeScorecardV1 = serde_json::from_slice(&bytes)?;
        out.push((
            run_id,
            format!("trusty://mens-organic-scorecard/{hash}"),
            scorecard,
        ));
    }
    Ok(out)
}

/// Verify that a sequence of [`OrganicSearchSpokeScorecardV1`] runs exhibits non-decreasing
/// `overall_end_to_end_pass_at_1`, `overall_search_recall_at_5`, and `overall_mrr` (`PERSIST-BENCH-02`).
pub fn verify_organic_scorecard_non_decreasing_trend(
    history: &[OrganicSearchSpokeScorecardV1],
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    for pair in history.windows(2) {
        let prev = &pair[0];
        let next = &pair[1];
        if next.overall_end_to_end_pass_at_1 + 1e-9 < prev.overall_end_to_end_pass_at_1 {
            violations.push(format!(
                "overall_end_to_end_pass_at_1 regressed from {:.4} to {:.4}",
                prev.overall_end_to_end_pass_at_1, next.overall_end_to_end_pass_at_1
            ));
        }
        if next.overall_search_recall_at_5 + 1e-9 < prev.overall_search_recall_at_5 {
            violations.push(format!(
                "overall_search_recall_at_5 regressed from {:.4} to {:.4}",
                prev.overall_search_recall_at_5, next.overall_search_recall_at_5
            ));
        }
        if next.overall_mrr + 1e-9 < prev.overall_mrr {
            violations.push(format!(
                "overall_mrr regressed from {:.4} to {:.4}",
                prev.overall_mrr, next.overall_mrr
            ));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Emit `ModelBenchCase` telemetry rows (`vox.research.model_bench_case`) into `VoxDb` (`research_metrics`)
/// for each receipt in an [`OrganicSearchSpokeScorecardV1`] (`PERSIST-BENCH-04`).
#[cfg(feature = "database")]
pub async fn record_organic_scorecard_telemetry_to_db(
    db: &vox_db::VoxDb,
    model_id: &str,
    scorecard: &OrganicSearchSpokeScorecardV1,
) -> anyhow::Result<usize> {
    use crate::training::contract::OrganicSpokeKind;

    let mut count = 0usize;
    for r in &scorecard.receipts {
        let task_category = match r.spoke {
            OrganicSpokeKind::VoxLang
            | OrganicSpokeKind::RustWorkspace
            | OrganicSpokeKind::SpeechToCode => "codegen",
            OrganicSpokeKind::StructuredShell => "refactoring",
            OrganicSpokeKind::McpToolChain | OrganicSpokeKind::SkillCreation => "research",
            OrganicSpokeKind::AgentHarness | OrganicSpokeKind::PopuliMesh => "planning",
        };
        let latency_ms = (r.latency_us / 1_000).max(1);
        let passed = r.search_recalled
            && r.compilation_verified
            && r.roundtrip_receipt.pass_at_1
            && r.rag_context_verified
            && r.distractor_resilient;
        let payload = serde_json::json!({
            "model_id": model_id,
            "fixture_id": r.case_id,
            "task_category": task_category,
            "passed": passed,
            "latency_ms": latency_ms,
        });
        let metadata_json = serde_json::to_string(&payload)?;
        db.append_research_metric(
            "bench:model_bench_case",
            "vox.model.bench_case",
            Some(latency_ms as f64),
            Some(&metadata_json),
        )
        .await?;
        count += 1;
    }
    Ok(count)
}

/// Verify zero XXH3-128 prompt or response digest collisions between held-out organic search-to-compilation
/// benchmark tasks and a slice of training records (`ORG-SEARCH-03`).
pub fn verify_organic_benchmark_decontamination(
    training_records: &[TrainingRecordV2],
) -> Result<(), Vec<String>> {
    use xxhash_rust::xxh3::xxh3_128;

    let suite = canonical_organic_search_to_compilation_suite_24();
    let mut heldout_prompt_hashes = std::collections::BTreeSet::new();
    let mut heldout_pair_hashes = std::collections::BTreeSet::new();
    for case in &suite {
        let prompt_clean = case.roundtrip_case.prompt.trim();
        heldout_prompt_hashes.insert(xxh3_128(prompt_clean.as_bytes()));
        heldout_prompt_hashes.insert(xxh3_128(case.search_query.trim().as_bytes()));
        for cand in &case.roundtrip_case.candidates {
            let clean = strip_code_fences(cand);
            let pair_str = format!("{}\0{}", prompt_clean, clean.trim());
            heldout_pair_hashes.insert(xxh3_128(pair_str.as_bytes()));
        }
    }

    let mut collisions = Vec::new();
    for rec in training_records {
        let prompt_clean = rec.prompt.trim();
        let resp_clean = strip_code_fences(&rec.response);
        let ph = xxh3_128(prompt_clean.as_bytes());
        let pair_h = xxh3_128(format!("{}\0{}", prompt_clean, resp_clean.trim()).as_bytes());
        if heldout_prompt_hashes.contains(&ph) {
            collisions.push(format!(
                "training record `{}` collides with held-out organic benchmark prompt",
                rec.trusty_uri()
            ));
        }
        if heldout_pair_hashes.contains(&pair_h) {
            collisions.push(format!(
                "training record `{}` collides with held-out organic benchmark (prompt, response) pair",
                rec.trusty_uri()
            ));
        }
    }

    if collisions.is_empty() {
        Ok(())
    } else {
        Err(collisions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scorecard(e2e: f64, recall: f64, mrr: f64) -> OrganicSearchSpokeScorecardV1 {
        OrganicSearchSpokeScorecardV1 {
            schema_version: OrganicSearchSpokeScorecardV1::SCHEMA_VERSION.to_string(),
            total_cases: 0,
            overall_search_recall_at_5: recall,
            overall_mrr: mrr,
            overall_unbiased_search_recall_at_5: 0.0,
            overall_unbiased_mrr: 0.0,
            overall_grounding_ratio: 1.0,
            overall_distractor_resilience_rate: 1.0,
            overall_compilation_pass_rate: 1.0,
            overall_end_to_end_pass_at_1: e2e,
            spokes: Vec::new(),
            receipts: Vec::new(),
        }
    }

    #[test]
    fn trend_accepts_flat_or_rising_history_and_reports_each_regression() {
        assert!(verify_organic_scorecard_non_decreasing_trend(&[]).is_ok());
        let rising = [scorecard(0.5, 0.5, 0.5), scorecard(0.5, 0.7, 0.9)];
        assert!(verify_organic_scorecard_non_decreasing_trend(&rising).is_ok());

        let falling = [scorecard(0.9, 0.9, 0.9), scorecard(0.5, 0.9, 0.4)];
        let errs = verify_organic_scorecard_non_decreasing_trend(&falling).unwrap_err();
        assert_eq!(errs.len(), 2);
        assert!(errs[0].starts_with("overall_end_to_end_pass_at_1 regressed"));
        assert!(errs[1].starts_with("overall_mrr regressed"));
    }

    #[test]
    fn decontamination_flags_records_matching_heldout_prompts() {
        assert!(verify_organic_benchmark_decontamination(&[]).is_ok());

        let case = &canonical_organic_search_to_compilation_suite_24()[0];
        let receipt = OracleReceipt {
            spoke: case.roundtrip_case.spoke,
            passed: true,
            stage: "test".to_string(),
            diagnostics: Vec::new(),
            exit_code: Some(0),
        };
        let prov = |id: &str| RecordProvenance {
            origin: "test".to_string(),
            source_id: id.to_string(),
            task_family: "test".to_string(),
            difficulty: 1,
        };
        let leaked = TrainingRecordV2::new(
            case.roundtrip_case.spoke,
            &case.roundtrip_case.prompt,
            "sys",
            &case.roundtrip_case.candidates[0],
            receipt.clone(),
            prov("leak"),
        );
        let clean = TrainingRecordV2::new(
            case.roundtrip_case.spoke,
            "an entirely unrelated training prompt zzz",
            "sys",
            "fn unrelated() {}",
            receipt,
            prov("clean"),
        );
        assert!(verify_organic_benchmark_decontamination(std::slice::from_ref(&clean)).is_ok());
        let errs = verify_organic_benchmark_decontamination(&[clean, leaked]).unwrap_err();
        assert!(errs.len() >= 2, "prompt and pair collisions: {errs:?}");
        assert!(
            errs.iter()
                .all(|e| e.contains("held-out organic benchmark"))
        );
    }
}
