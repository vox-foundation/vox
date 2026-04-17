// ─── Top-level generator ──────────────────────────────────────────────────────

/// Generate all synthetic training pairs and write them to `output_path` as JSONL.
///
/// Returns the total number of pairs written.
pub fn generate_all(cfg: &SyntheticGenConfig, output_path: &Path) -> anyhow::Result<usize> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output directory: {}", parent.display()))?;
    }

    let mut file = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(output_path)
            .with_context(|| format!("create synthetic corpus: {}", output_path.display()))?,
    );

    let mut total = 0usize;

    if cfg.emit_tool_traces {
        let n = tool_pairs::generate_tool_pairs(&mut file, TOOL_REGISTRY_SLIM, cfg)?;
        eprintln!("  [synthetic] tool_trace: {n} pairs");
        total += n;
    }

    if cfg.emit_a2a_traces {
        let n = a2a_pairs::generate_a2a_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] a2a_trace: {n} pairs");
        total += n;
    }

    if cfg.emit_workflow_traces {
        let n = workflow_pairs::generate_workflow_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] workflow_trace: {n} pairs");
        total += n;
    }

    if cfg.emit_orchestrator_rows {
        let n = orchestrator_pairs::generate_orchestrator_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] orchestrator: {n} pairs");
        total += n;
    }

    if cfg.emit_skill_rows {
        let n = orchestrator_pairs::generate_skill_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] skill: {n} pairs");
        total += n;
    }

    if cfg.emit_agent_rows {
        let n = agent_pairs::generate_agent_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] agent_def: {n} pairs");
        total += n;
    }

    let n = web_pairs::generate_web_construct_pairs(&mut file, cfg)?;
    eprintln!("  [synthetic] web_construct: {n} pairs");
    total += n;

    let n = negative_pairs::generate_negative_preference_pairs(&mut file, cfg)?;
    eprintln!("  [synthetic] negative_routing: {n} pairs");
    total += n;

    if cfg.emit_cli_rows {
        let n = cli_pairs::generate_cli_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] cli_command: {n} pairs");
        total += n;
    }

    if cfg.emit_script_rows {
        let n = script_pairs::generate_script_pairs(&mut file, cfg)?;
        eprintln!("  [synthetic] script: {n} pairs");
        total += n;
    }

    // ── Organic Vox code generation ──────────────────────────────────────
    if cfg.emit_organic_vox {
        let organic = crate::codegen_vox::generate_organic_corpus(cfg.seed);
        let verified = organic.iter().filter(|p| p.verified).count();
        let mut compact_count = 0usize;
        let mut error_fix_count = 0usize;
        let mut multiturn_count = 0usize;
        for (i, pair) in organic.iter().enumerate() {
            writeln!(file, "{}", pair.to_jsonl())?;
            total += 1;

            // Compact variant: every 5th pair gets a compact form (20%)
            if i % 5 == 0 && !pair.response.is_empty() {
                let compact_line = crate::corpus::preflight::compact_variant(
                    &pair.prompt,
                    &pair.response,
                    &pair.category,
                );
                writeln!(file, "{}", compact_line)?;
                total += 1;
                compact_count += 1;
            }

            // Error → fix pairs: 50% of organic pairs, cycling through all 12 BrokenKind variants
            if i % 2 == 0 {
                use crate::corpus::preflight::{BrokenKind, break_vox, error_fix_to_jsonl};
                let kind = match i % 12 {
                    0 => BrokenKind::MissingReturnArrow,
                    1 => BrokenKind::UnclosedBrace,
                    2 => BrokenKind::KeywordTypo,
                    3 => BrokenKind::MissingRet,
                    4 => BrokenKind::MissingToUnit,
                    5 => BrokenKind::TypeMismatch,
                    6 => BrokenKind::OptionUnwrapMissing,
                    7 => BrokenKind::BadReturnType,
                    8 => BrokenKind::WrongType,
                    9 => BrokenKind::UnresolvedGenericArity,
                    10 => BrokenKind::InferenceAmbiguity,
                    _ => BrokenKind::UnreachableMatchArm,
                };
                let (broken, explanation) = break_vox(&pair.response, kind);
                let fix_line =
                    error_fix_to_jsonl(&broken, &explanation, &pair.response, &pair.category);
                writeln!(file, "{}", fix_line)?;
                total += 1;
                error_fix_count += 1;
            }

            // Multi-turn conversations
            if i % 10 == 0 {
                use crate::corpus::preflight::{gen_multiturn_vox, multiturn_to_jsonl};
                let decl_type = pair
                    .coverage_tags
                    .first()
                    .map(|s| s.as_str())
                    .unwrap_or("function");
                let name = pair.category.split('.').next().unwrap_or("handler");
                // The signature in preflight.rs is gen_multiturn_vox(construct, name, base_code, template_idx)
                let turns = gen_multiturn_vox(decl_type, name, &pair.response, i);
                let mt_line = multiturn_to_jsonl(&turns, &pair.category);
                writeln!(file, "{}", mt_line)?;
                total += 1;
                multiturn_count += 1;
            }
        }
        eprintln!(
            "  [synthetic] organic_vox: {} pairs ({} verified) +{} compact +{} error_fix +{} multiturn",
            organic.len(),
            verified,
            compact_count,
            error_fix_count,
            multiturn_count
        );

        // Architectural Q&A pairs (static, high-signal)
        let arch_n = crate::corpus::preflight::write_architectural_pairs(&mut file)?;
        total += arch_n;
        eprintln!("  [synthetic] architectural_qa: {arch_n} pairs");

        // TypeScript interop pairs
        let ts_n = crate::corpus::preflight::write_ts_interop_pairs(&mut file)?;
        total += ts_n;
        eprintln!("  [synthetic] ts_interop: {ts_n} pairs");

        // Explain, debug, refactor pairs derived from the organic corpus
        let organic_triples: Vec<(String, String, String)> = organic
            .iter()
            .map(|p| (p.prompt.clone(), p.response.clone(), p.category.clone()))
            .collect();
        let explain_lines = crate::corpus::preflight::gen_explain_pairs(&organic_triples, 5);
        let debug_lines = crate::corpus::preflight::gen_debug_pairs(&organic_triples, 7);
        let refactor_lines = crate::corpus::preflight::gen_refactor_pairs(&organic_triples, 7);
        for line in explain_lines
            .iter()
            .chain(debug_lines.iter())
            .chain(refactor_lines.iter())
        {
            writeln!(file, "{line}")?;
            total += 1;
        }
        eprintln!(
            "  [synthetic] explain+debug+refactor: {} pairs",
            explain_lines.len() + debug_lines.len() + refactor_lines.len()
        );
    }

    // Tool-chain orchestration pairs
    let tc_n = routing_pairs::generate_tool_chain_pairs(&mut file, cfg)?;
    total += tc_n;
    eprintln!("  [synthetic] tool_chain: {tc_n} pairs");

    // Agent lifecycle pairs
    let al_n = agent_pairs::generate_agent_lifecycle_pairs(&mut file, cfg)?;
    total += al_n;
    eprintln!("  [synthetic] agent_lifecycle: {al_n} pairs");

    // Routing decision pairs (Gap 1: orchestrator-as-model training)
    if cfg.emit_routing_decisions {
        let n = routing_pairs::generate_routing_decision_pairs(&mut file, cfg)?;
        total += n;
        eprintln!("  [synthetic] routing_decision: {n} pairs");
    }

    // Expanded negative preference pairs (Gap 2: 4 → 25+)
    if cfg.emit_negative_expanded {
        let n = routing_pairs::generate_negative_preference_pairs_expanded(&mut file, cfg)?;
        total += n;
        eprintln!("  [synthetic] negative_routing_expanded: {n} pairs");
    }

    // Error → recovery pairs (Gap 6)
    if cfg.emit_error_recovery {
        let n = error_recovery_pairs::generate_error_recovery_pairs(&mut file, cfg)?;
        total += n;
        eprintln!("  [synthetic] error_recovery: {n} pairs");
    }

    // Multi-agent conversation pairs (Gap 8)
    if cfg.emit_multi_agent_convos {
        let n = multi_agent_pairs::generate_multi_agent_conversation_pairs(&mut file, cfg)?;
        total += n;
        eprintln!("  [synthetic] multi_agent_flow: {n} pairs");
    }

    // Telemetry interpretation pairs (Gap 10)
    if cfg.emit_telemetry_pairs {
        let n = telemetry_pairs::generate_telemetry_interpretation_pairs(&mut file, cfg)?;
        total += n;
        eprintln!("  [synthetic] telemetry_trace: {n} pairs");
    }

    // KCH Anti-conflation negative DPO pairs (Wave 4)
    if cfg.emit_kch_anticonflation {
        let n = kch_anticonflation::generate_kch_anticonflation_pairs(&mut file, cfg)?;
        total += n;
        eprintln!("  [synthetic] kch_anticonflation: {n} pairs");
    }

    file.flush()?;
    eprintln!(
        "  [synthetic] total: {total} pairs → {}",
        output_path.display()
    );

    // ── Post-generation augmentation ────────────────────────────────────
    // Re-read the generated JSONL and apply the typo/synonym/case augmentation
    // engine to produce 3× more training variants with natural language noise.
    if cfg.augment_after_generate {
        let raw_lines: Vec<String> = vox_bounded_fs::read_utf8_path_capped(output_path)?
            .lines()
            .map(String::from)
            .collect();
        let aug_cfg = crate::corpus::augment::AugmentConfig {
            variants_per_prompt: 3,
            typo_char_rate: 0.05,
            synonym_swap_rate: 0.25,
            shuffle_words: true,
            case_variants: true,
        };
        let augmented = crate::corpus::augment::augment_jsonl_lines(&raw_lines, &aug_cfg, cfg.seed);
        let aug_count = augmented.len() - raw_lines.len();
        // Rewrite file with augmented lines
        let mut aug_file = std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(output_path)?,
        );
        for line in &augmented {
            writeln!(aug_file, "{}", line)?;
        }
        aug_file.flush()?;
        total += aug_count;
        eprintln!("  [augment] +{aug_count} augmented variants (total now {total})");
    }

    // Generate search traces into a companion file
    let search_output_path = output_path.with_file_name("synthetic_search.jsonl");
    let mut search_file = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&search_output_path)
            .with_context(|| {
                format!(
                    "create synthetic search corpus: {}",
                    search_output_path.display()
                )
            })?,
    );
    let n = crate::synthetic_search_gen::generate_search_traces(&mut search_file)?;
    search_file.flush()?;
    eprintln!(
        "  [synthetic_search] total: {n} pairs → {}",
        search_output_path.display()
    );
    total += n;

    // ── Coverage report ──────────────────────────────────────────────────
    // Re-generate the organic corpus to compute coverage metrics.
    // This is cheap (deterministic, in-memory) and surfaces exact gaps.
    if cfg.emit_organic_vox {
        let organic = crate::codegen_vox::generate_organic_corpus(cfg.seed);
        let report = crate::codegen_vox::compute_coverage_report(&organic);
        crate::codegen_vox::print_coverage_report(&report);
    }

    Ok(total)
}
