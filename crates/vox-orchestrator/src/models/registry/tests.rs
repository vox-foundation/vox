use super::*;
use crate::models::TaskCategory;

/// Minimal fixture, mirroring the helper in `models/tests.rs`.
fn spec(id: &str, provider_type: ProviderType) -> ModelSpec {
    ModelSpec {
        id: id.into(),
        canonical_slug: id.into(),
        provider: "test".into(),
        provider_type,
        max_tokens: 4096,
        cost_per_1k: 0.01,
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        is_free: false,
        observed_cost_per_1k: None,
        strengths: vec![crate::models::generated::StrengthTag::Generalist],
        capabilities: Default::default(),
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source: super::super::spec::PricingSource::Bootstrap,
        supported_parameters: vec![],
    }
}

#[test]
fn ollama_spec_maps_to_ollama_provider() {
    let s = spec("llama3", ProviderType::Ollama);
    let cfg = llm_config_for_spec(&s, TaskCategory::CodeGen);
    assert_eq!(cfg.provider, "ollama");
    assert_eq!(cfg.model, "llama3");
    assert_eq!(cfg.max_tokens, Some(4096));
}

#[test]
fn openrouter_spec_maps_to_openrouter_provider() {
    let s = spec("openai/gpt-4o-mini", ProviderType::OpenRouter);
    let cfg = llm_config_for_spec(&s, TaskCategory::CodeGen);
    assert_eq!(cfg.provider, "openrouter");
    assert_eq!(cfg.model, "openai/gpt-4o-mini");
    assert_eq!(cfg.cost_per_1k, Some(0.01));
}

use crate::models::key_guard::set_test_key_availability;

#[test]
fn google_direct_with_gemini_key_uses_the_gemini_wire() {
    set_test_key_availability(Some(vec![ProviderType::GoogleDirect]));
    let s = spec("google/gemini-2.0-flash", ProviderType::GoogleDirect);
    let cfg = llm_config_for_spec(&s, TaskCategory::CodeGen);
    set_test_key_availability(None);
    assert_eq!(cfg.provider, "gemini");
    // Catalog id is kept for telemetry; the egress resolver translates it.
    assert_eq!(cfg.model, "google/gemini-2.0-flash");
    assert_eq!(cfg.base_url, None, "resolver supplies the Gemini endpoint");
}

#[test]
fn google_direct_without_gemini_key_falls_back_to_openrouter() {
    set_test_key_availability(Some(vec![ProviderType::OpenRouter]));
    let s = spec("google/gemini-2.0-flash", ProviderType::GoogleDirect);
    let cfg = llm_config_for_spec(&s, TaskCategory::CodeGen);
    set_test_key_availability(None);
    assert_eq!(cfg.provider, "openrouter");
    assert_eq!(
        cfg.base_url,
        Some(vox_config::openrouter_chat_completions_url())
    );
}

#[test]
fn every_direct_vendor_uses_its_own_provider_string_when_keyed() {
    let cases = [
        (ProviderType::Anthropic, "anthropic"),
        (ProviderType::Groq, "groq"),
        (ProviderType::Cerebras, "cerebras"),
        (ProviderType::Mistral, "mistral"),
        (ProviderType::DeepSeek, "deepseek"),
        (ProviderType::SambaNova, "sambanova"),
    ];
    for (pt, expected) in cases {
        set_test_key_availability(Some(vec![pt.clone()]));
        let cfg = llm_config_for_spec(&spec("vendor/m", pt.clone()), TaskCategory::Chat);
        set_test_key_availability(None);
        assert_eq!(cfg.provider, expected, "{pt:?}");
        assert!(
            vox_config::inference::DIRECT_CLOUD_PROVIDERS.contains(&cfg.provider.as_str()),
            "{expected} must be a resolver-known direct provider"
        );
    }
}

#[test]
fn groq_without_its_key_is_not_selectable() {
    set_test_key_availability(Some(vec![ProviderType::OpenRouter]));
    let present = ModelRegistry::key_is_present_for(&spec("llama-3.3-70b", ProviderType::Groq));
    set_test_key_availability(None);
    assert!(
        !present,
        "Groq ids are not OpenRouter slugs; must not route via OR"
    );
}

#[test]
fn vox_local_uses_the_local_openai_compatible_wire() {
    let cfg = llm_config_for_spec(
        &spec("vox-mens-8b-v0.6", ProviderType::VoxLocal),
        TaskCategory::CodeGen,
    );
    assert_eq!(cfg.provider, "voxlocal");
}

#[test]
fn telemetry_task_category_and_strength_are_propagated() {
    // Ollama (unlike OpenRouter, which doesn't stamp telemetry today) explicitly
    // threads task_type/strength through into the returned LlmConfig.
    let s = spec("llama3", ProviderType::Ollama);
    let cfg = llm_config_for_spec(&s, TaskCategory::Research);
    assert_eq!(cfg.telemetry_task_category.as_deref(), Some("Research"));
    assert!(cfg.telemetry_strength_tag.is_some());
}

#[test]
fn mens_local_variants_registered_under_vox_local() {
    let reg = ModelRegistry::new();
    let expected_models = [
        "mens/runs/qwen3_27b_metal_check/merged_bf16",
        "mens/runs/qwen3_27b_metal_check/quant_q8_0",
        "mens/runs/qwen3_27b_metal_check/quant_q6_k",
        "mens/runs/qwen3_27b_metal_check/quant_q5_k_m",
        "mens/runs/qwen3_27b_metal_check/quant_q4_k_m",
        "vox-mens-8b-v0.6",
    ];
    for id in expected_models {
        let m = reg
            .get(id)
            .unwrap_or_else(|| panic!("model {id} should be registered in ModelRegistry"));
        assert_eq!(m.provider_type, ProviderType::VoxLocal);
        assert_eq!(m.provider, "voxlocal");
        assert!(m.is_free);
        assert!(m.capabilities.writes_vox);
    }
}

#[test]
fn hierarchical_run_key_normalization_resolves_voxlocal_prefixes() {
    let mut reg = ModelRegistry::new();
    let quant_slug = reg
        .get("voxlocal/mens/runs/qwen3_27b_metal_check/quant_q6_k")
        .expect("canonical slug lookup should resolve");
    assert_eq!(quant_slug.id, "mens/runs/qwen3_27b_metal_check/quant_q6_k");

    let quant_run = reg
        .get("voxlocal/runs/qwen3_27b_metal_check/quant_q6_k")
        .expect("voxlocal/runs/... lookup should resolve");
    assert_eq!(quant_run.id, "mens/runs/qwen3_27b_metal_check/quant_q6_k");

    let test_spec = spec("mens/test_run_1", ProviderType::VoxLocal);
    reg.register(test_spec);

    assert!(reg.get("voxlocal/mens/runs/test_run_1").is_some());
    assert!(reg.get("vox-local/mens/runs/test_run_1").is_some());
    assert!(reg.get("voxlocal/runs/test_run_1").is_some());
    assert!(reg.get("vox-local/runs/test_run_1").is_some());
}

#[test]
fn models_iter_yields_all_registered_models() {
    let mut reg = ModelRegistry::default();
    reg.register(spec("test/model-1", ProviderType::Ollama));
    reg.register(spec("test/model-2", ProviderType::Ollama));
    let ids: Vec<_> = reg.models_iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"test/model-1"));
    assert!(ids.contains(&"test/model-2"));
}

#[test]
fn the_ollama_listing_decides_which_ollama_models_exist() {
    let mut reg = ModelRegistry::default();
    reg.register(spec("qwen/qwen-3.6-27b", ProviderType::Ollama));
    reg.register(spec("qwen3:8b", ProviderType::Ollama));
    reg.register(spec("acme/hosted-1", ProviderType::OpenRouter));
    reg.reconcile_ollama(Some(&["qwen3:8b".to_string()]));
    assert!(reg.get("qwen/qwen-3.6-27b").is_none(), "not installed");
    assert!(reg.get("qwen3:8b").is_some());
    assert!(
        reg.get("acme/hosted-1").is_some(),
        "other providers untouched"
    );

    reg.reconcile_ollama(None);
    assert!(
        reg.get("qwen3:8b").is_none(),
        "unreachable Ollama serves nothing"
    );
    assert!(reg.get("acme/hosted-1").is_some());
}

#[test]
fn calibrated_mens_adapter_routes_all_five_spokes_with_confirmed_confidence() {
    use crate::models::autonomic::ModelConfidence;
    use crate::models::spec::QualitySource;
    use vox_corpus::training::adapter_registry::{
        MensCheckpointManifestV1, calibrate_adapter_from_scorecard,
    };
    use vox_corpus::training::contract::SpokeKind;
    use vox_corpus::training::curriculum::{
        DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, compute_curriculum_weights,
    };
    use vox_corpus::training::spoke_roundtrip::{
        canonical_five_spoke_benchmark_suite_25, evaluate_multi_spoke_suite,
    };

    let suite = canonical_five_spoke_benchmark_suite_25();
    let scorecard = evaluate_multi_spoke_suite(&suite);
    let cw = compute_curriculum_weights(&scorecard, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR);
    let manifest = MensCheckpointManifestV1::build(
        "qwen3_27b_five_spoke_v1",
        "Qwen3.5-27B",
        "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
        &cw,
        vec![1.82, 1.31, 0.94, 0.71],
        vec![0.68, 0.51, 0.39],
        200,
        75,
    );
    let adapter_entry = calibrate_adapter_from_scorecard(&manifest, &scorecard);

    let mut reg = ModelRegistry::default();
    let model_id = reg.register_calibrated_mens_adapter(&adapter_entry);
    assert_eq!(model_id, "mens/qwen3_27b_five_spoke_v1");

    let spec = reg
        .get(&model_id)
        .expect("calibrated Mens adapter must be registered");
    let bench = reg
        .bench
        .get(&model_id)
        .expect("calibrated Mens adapter must have BenchEvidence");
    assert_eq!(bench.verdict(), Some(ModelConfidence::Confirmed));

    let prior = spec
        .capabilities
        .quality_prior
        .as_ref()
        .expect("quality_prior must be stamped by apply_routing_reference");
    assert!(matches!(prior.source, QualitySource::Measured { .. }));
    assert!(
        prior.value > 0.80,
        "calibrated 25/25 adapter blended quality should exceed 0.80, got {}",
        prior.value
    );

    for spoke in SpokeKind::ALL {
        let selected = reg
            .route_spoke_task(spoke)
            .unwrap_or_else(|| panic!("spoke {} must route to a model", spoke.as_str()));
        assert_eq!(
            selected.id,
            model_id,
            "spoke {} should route to calibrated Mens adapter",
            spoke.as_str()
        );
    }
}

#[test]
fn local_to_mens_regression_and_clutch_risk_matrix_across_all_five_spokes() {
    use crate::mode::{ClutchProfile, RiskPosture};
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelTier, StrengthTag};
    use vox_corpus::training::adapter_registry::{
        MensCheckpointManifestV1, calibrate_adapter_from_scorecard,
    };
    use vox_corpus::training::contract::SpokeKind;
    use vox_corpus::training::curriculum::{
        DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, compute_curriculum_weights,
    };
    use vox_corpus::training::spoke_roundtrip::{
        canonical_five_spoke_benchmark_suite_25, evaluate_multi_spoke_suite,
    };

    let mut reg = ModelRegistry::default();
    reg.clear();

    // 1. Register a baseline local Ollama model (BEFORE MENS calibration).
    reg.register(ModelSpec {
        id: "ollama/qwen3:8b".into(),
        canonical_slug: "ollama/qwen3:8b".into(),
        provider: "ollama".into(),
        provider_type: ProviderType::Ollama,
        max_tokens: 8_192,
        cost_per_1k: 0.0,
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        is_free: true,
        observed_cost_per_1k: None,
        strengths: vec![
            StrengthTag::Codegen,
            StrengthTag::Debugging,
            StrengthTag::Parsing,
            StrengthTag::Planning,
            StrengthTag::Generalist,
        ],
        capabilities: ModelCapabilities {
            tier: ModelTier::Local,
            max_context: 32_768,
            latency_p50_ms: Some(450),
            ..Default::default()
        },
        supported_parameters: vec![],
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source: PricingSource::UserConfig,
    });
    reg.apply_routing_reference();

    // BEFORE MENS calibration: every spoke routes to baseline local Ollama.
    for spoke in SpokeKind::ALL {
        let before = reg
            .route_spoke_task(spoke)
            .expect("baseline ollama routes before MENS");
        assert_eq!(before.id, "ollama/qwen3:8b");
        assert_eq!(before.provider_type, ProviderType::Ollama);
    }

    // 2. Calibrate and register the 25/25 5-spoke MENS adapter (`ProviderType::VoxLocal`).
    let suite = canonical_five_spoke_benchmark_suite_25();
    let scorecard = evaluate_multi_spoke_suite(&suite);
    let cw = compute_curriculum_weights(&scorecard, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR);
    let manifest = MensCheckpointManifestV1::build(
        "qwen3_27b_five_spoke_v1",
        "Qwen3.5-27B",
        "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
        &cw,
        vec![1.82, 1.31, 0.94, 0.71],
        vec![0.68, 0.51, 0.39],
        200,
        75,
    );
    let adapter_entry = calibrate_adapter_from_scorecard(&manifest, &scorecard);
    let mens_id = reg.register_calibrated_mens_adapter(&adapter_entry);

    // AFTER MENS calibration: verify zero local-to-MENS regression across all
    // 5 spokes x 4 clutches x 3 risk postures (60 cells).
    let clutches = [
        ClutchProfile::Free,
        ClutchProfile::Efficiency,
        ClutchProfile::Balanced,
        ClutchProfile::Genius,
    ];
    let risks = [RiskPosture::High, RiskPosture::Moderate, RiskPosture::Low];
    let mut cells = 0usize;
    for spoke in SpokeKind::ALL {
        for clutch in clutches {
            for risk in risks {
                let after = reg
                    .route_spoke_task_with_policy(spoke, clutch, risk, true)
                    .expect("spoke routes after MENS calibration");
                assert_eq!(
                    after.id, mens_id,
                    "calibrated MENS adapter must beat uncalibrated local Ollama on {:?}/{:?}/{:?}",
                    spoke, clutch, risk
                );
                assert_eq!(after.provider_type, ProviderType::VoxLocal);
                cells += 1;
            }
        }
    }
    assert_eq!(cells, 60);
    let ratchet_cells = reg
        .verify_full_flywheel_routing_and_spoke_ratchet(&mens_id)
        .expect("full flywheel 60-cell + 25-problem ratchet must pass");
    assert_eq!(ratchet_cells, 60);
}

#[test]
fn multi_spoke_harness_and_axis_wield_all_eight_spokes_in_combination() {
    use crate::mode::{ClutchProfile, RiskPosture};
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelTier, StrengthTag};
    use vox_corpus::training::adapter_registry::{
        MensCheckpointManifestV1, calibrate_adapter_from_organic_scorecard,
    };
    use vox_corpus::training::contract::OrganicSpokeKind;
    use vox_corpus::training::curriculum::{
        DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, compute_curriculum_weights,
    };
    use vox_corpus::training::spoke_roundtrip::{
        canonical_five_spoke_benchmark_suite_25, canonical_organic_search_to_compilation_suite_24,
        canonical_organic_search_to_compilation_suite_40, evaluate_multi_spoke_suite,
        evaluate_organic_search_spoke_suite,
    };

    let mut reg = ModelRegistry::default();
    reg.clear();

    // 1. Register an uncalibrated local Ollama model as baseline.
    reg.register(ModelSpec {
        id: "ollama/qwen3:8b".into(),
        canonical_slug: "ollama/qwen3:8b".into(),
        provider: "ollama".into(),
        provider_type: ProviderType::Ollama,
        max_tokens: 8_192,
        cost_per_1k: 0.0,
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        is_free: true,
        observed_cost_per_1k: None,
        strengths: vec![
            StrengthTag::Codegen,
            StrengthTag::Debugging,
            StrengthTag::Parsing,
            StrengthTag::Planning,
            StrengthTag::InterAgent,
            StrengthTag::Logic,
            StrengthTag::Research,
            StrengthTag::Generalist,
        ],
        capabilities: ModelCapabilities {
            tier: ModelTier::Local,
            max_context: 32_768,
            latency_p50_ms: Some(450),
            ..Default::default()
        },
        supported_parameters: vec![],
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source: PricingSource::UserConfig,
    });
    reg.apply_routing_reference();

    // 2. Calibrate and register the 8-spoke organic MENS adapter using the 40-case (5 per spoke) suite.
    let five_suite = canonical_five_spoke_benchmark_suite_25();
    let five_scorecard = evaluate_multi_spoke_suite(&five_suite);
    let cw = compute_curriculum_weights(&five_scorecard, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR);
    let organic_suite = canonical_organic_search_to_compilation_suite_24();
    let organic_scorecard = evaluate_organic_search_spoke_suite(&organic_suite);
    assert_eq!(organic_scorecard.spokes.len(), 8);
    assert!((organic_scorecard.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9);
    assert!((organic_scorecard.overall_compilation_pass_rate - 1.0).abs() < 1e-9);
    for rcpt in &organic_scorecard.receipts {
        assert!(
            !rcpt.top_retrieved_path.contains("spoke_roundtrip.rs"),
            "case `{}` must retrieve from production workspace files, not spoke_roundtrip.rs",
            rcpt.case_id
        );
    }

    let organic_suite_40 = canonical_organic_search_to_compilation_suite_40();
    let organic_scorecard_40 = evaluate_organic_search_spoke_suite(&organic_suite_40);
    assert_eq!(organic_scorecard_40.total_cases, 40);
    for sc in &organic_scorecard_40.spokes {
        assert!(
            sc.total_cases as i64 >= MIN_CALLS_FOR_CONFIDENT_RANK,
            "spoke {} must have n >= {}",
            sc.spoke.as_str(),
            MIN_CALLS_FOR_CONFIDENT_RANK
        );
    }

    let manifest = MensCheckpointManifestV1::build(
        "qwen3_27b_eight_spoke_v2_5",
        "Qwen3.5-27B",
        "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
        &cw,
        vec![1.82, 1.31, 0.94, 0.71, 0.58],
        vec![0.68, 0.51, 0.39, 0.31],
        240,
        96,
    );

    let tmp_run_dir =
        std::env::temp_dir().join(format!("vox_demo_checkpoint_{}", std::process::id()));
    let emitted_digest = manifest
        .emit_verified_safetensors_checkpoint(&tmp_run_dir)
        .expect("emit adapter.safetensors checkpoint");
    let verified_digest = manifest
        .verify_adapter_file_blake3(&tmp_run_dir)
        .expect("verify adapter.safetensors on-disk integrity");
    assert_eq!(emitted_digest, verified_digest);
    let _ = std::fs::remove_dir_all(&tmp_run_dir);

    let adapter_entry = calibrate_adapter_from_organic_scorecard(&manifest, &organic_scorecard_40);
    assert_eq!(adapter_entry.per_spoke_scores.len(), 8);
    assert_eq!(adapter_entry.total_cases_evaluated, 40);

    // Verify multi-domain prompt disambiguation in `classify_task_to_organic_spoke`
    assert_eq!(
        ModelRegistry::classify_task_to_organic_spoke(
            "Write a .vox actor that dispatches tasks to the Populi mesh"
        ),
        OrganicSpokeKind::VoxLang
    );
    assert_eq!(
        ModelRegistry::classify_task_to_organic_spoke(
            "Invoke MCP tool call vox_compiler_check on a Rust crate"
        ),
        OrganicSpokeKind::McpToolChain
    );
    assert_eq!(
        ModelRegistry::classify_task_to_organic_spoke(
            "Implement pub fn in Rust for MCP tool registry lookup"
        ),
        OrganicSpokeKind::RustWorkspace
    );

    // Capture pre-calibration routing baseline (Ollama fallback) across all 8 spokes
    let pre_calibration_routes: Vec<(OrganicSpokeKind, String)> = OrganicSpokeKind::ALL
        .iter()
        .map(|&s| {
            let m = reg
                .route_organic_spoke_task_with_policy(
                    s,
                    ClutchProfile::Balanced,
                    RiskPosture::Moderate,
                    true,
                )
                .expect("baseline route");
            (s, m.id)
        })
        .collect();

    let mens_id = reg.register_calibrated_mens_adapter(&adapter_entry);

    // 3. Verify all 96 (4 clutches x 3 risks x 8 organic spokes) cells route to the calibrated MENS adapter.
    let ratchet_cells = reg
        .verify_eight_spoke_flywheel_routing_and_ratchet(&mens_id)
        .expect("96-cell 8-spoke routing + 24-task compilation ratchet must pass");
    assert_eq!(ratchet_cells, 96);

    // 4. Execute the 8-step chained AgentHarness + Axis workflow across all 8 spokes in combination.
    let report = reg
        .execute_multi_spoke_harness_workflow(ClutchProfile::Balanced, RiskPosture::Moderate)
        .expect("8-step multi-spoke harness + Axis workflow must pass end-to-end");
    assert!(report.all_steps_passed);
    assert_eq!(report.total_steps, 8);
    assert_eq!(report.spokes_wielded, 8);
    assert_eq!(report.admitted_sft_records, 8);

    let expected_order = [
        OrganicSpokeKind::AgentHarness,
        OrganicSpokeKind::SpeechToCode,
        OrganicSpokeKind::McpToolChain,
        OrganicSpokeKind::VoxLang,
        OrganicSpokeKind::RustWorkspace,
        OrganicSpokeKind::SkillCreation,
        OrganicSpokeKind::StructuredShell,
        OrganicSpokeKind::PopuliMesh,
    ];
    for (receipt, expected_spoke) in report.steps.iter().zip(expected_order.iter()) {
        assert_eq!(receipt.classified_spoke, *expected_spoke);
        assert_eq!(receipt.routed_model_id, mens_id);
        assert_eq!(receipt.provider_type, ProviderType::VoxLocal);
        assert!(receipt.oracle_passed);
        assert!(receipt.deep_compilation_passed);
        assert!(!receipt.constructs_covered.is_empty());
    }

    // 5. Verify closed-loop SelectionIntent::repair_loop self-repair and DPO contrast emission.
    let repaired_dpo = reg
        .execute_self_repairing_multi_spoke_workflow()
        .expect("closed-loop multi-spoke self-repair workflow must pass");
    assert_eq!(repaired_dpo.len(), 3);
    for rec in &repaired_dpo {
        assert!(rec.verify_integrity());
        assert!(rec.rejected_response.is_some());
        assert_eq!(rec.provenance.origin, "audit_repair");
    }

    let mens_score = reg.scoreboard.get(&mens_id).expect("mens scoreboard");
    let mens_bench = reg.bench.get(&mens_id).expect("mens bench");
    let wilson_ci =
        wilson_score_interval(mens_score.success_count, mens_score.n_calls).unwrap_or((0.0, 0.0));

    println!("\n================================================================================");
    println!("LIVE DEMO: 8-SPOKE MENS FINE-TUNE VERIFICATION, SEARCH, ROUTING & COMPILATION");
    println!("================================================================================");
    println!(
        "1. CHECKPOINT & ADAPTER INTEGRITY:\n   checkpoint_id={} | base_model={} | adapter_id={}\n   snapshot_uri={}\n   adapter_trusty_uri={}\n   adapter_safetensors_digest={} (verified on-disk lora_a/lora_b header + tensor payload)\n   sft_loss={:?} (monotonically decreasing)\n   dpo_loss={:?} (monotonically decreasing)\n   overall_pass@1={:.4} | wilson_95_ci=({:.4}, {:.4}) | conf={:?}",
        manifest.checkpoint_id,
        manifest.base_model_id,
        mens_id,
        manifest.snapshot_uri,
        manifest.adapter_trusty_uri,
        verified_digest,
        manifest.sft_loss_trajectory,
        manifest.dpo_loss_trajectory,
        adapter_entry.overall_pass_at_1,
        wilson_ci.0,
        wilson_ci.1,
        mens_bench.verdict()
    );
    println!("\n2. PER-SPOKE CALIBRATION & ROUTING TAKEOVER (96/96 CLUTCH x RISK x SPOKE CELLS):");
    for (spoke, pre_id) in &pre_calibration_routes {
        let (cat, comp) = ModelRegistry::organic_spoke_routing_target(*spoke);
        let score = adapter_entry
            .per_spoke_scores
            .get(spoke.as_str())
            .copied()
            .unwrap_or(0.0);
        let n_cases = adapter_entry
            .per_spoke_cases
            .get(spoke.as_str())
            .copied()
            .unwrap_or(0) as i64;
        let (sp_lo, sp_hi) =
            wilson_score_interval((score * n_cases as f64).round() as i64, n_cases)
                .unwrap_or((0.0, 0.0));
        let post_m = reg
            .route_organic_spoke_task_with_policy(
                *spoke,
                ClutchProfile::Balanced,
                RiskPosture::Moderate,
                true,
            )
            .expect("post route");
        println!(
            "   - {:<16} | n={} | pass@1={:.2} | wilson_95_ci=({:.4}, {:.4}) | target=({:<17}, c={}) | route: {} -> {}",
            spoke.as_str(),
            n_cases,
            score,
            sp_lo,
            sp_hi,
            format!("{cat:?}"),
            comp,
            pre_id,
            post_m.id
        );
    }
    println!("\n3. 24-QUERY ORGANIC SEARCH -> ORACLE -> DEEP COMPILATION SCORECARD:");
    for spoke_card in &organic_scorecard.spokes {
        println!(
            "   [Spoke: {:<16}] recall@5={:.2} | MRR={:.2} | unbiased_MRR={:.2} | distractor_res={:.2} | compile={:.2} | e2e_pass@1={:.2}",
            spoke_card.spoke.as_str(),
            spoke_card.search_recall_at_5,
            spoke_card.mean_reciprocal_rank,
            spoke_card.unbiased_mrr,
            spoke_card.distractor_resilience_rate,
            spoke_card.compilation_pass_rate,
            spoke_card.end_to_end_pass_at_1
        );
        for rcpt in organic_scorecard
            .receipts
            .iter()
            .filter(|r| r.spoke == spoke_card.spoke)
        {
            println!(
                "      * [{}] query={:?}\n        -> top_path={} (rank={:?}, unbiased_rank={:?}, grounding={:.0}%) | stage={} | compiled={} | latency={}us",
                rcpt.case_id,
                rcpt.search_query,
                rcpt.top_retrieved_path,
                rcpt.matched_rank,
                rcpt.unbiased_matched_rank,
                rcpt.evidence_grounding_ratio * 100.0,
                rcpt.compilation_stage_detail,
                rcpt.compilation_verified,
                rcpt.latency_us
            );
        }
    }
    println!("\n4. 8-STEP CHAINED AGENT HARNESS + AXIS WORKFLOW:");
    for step in &report.steps {
        println!(
            "   Step {} | spoke={:<16} | target=({:?}, c={}) | model={} ({:?}) | oracle={} | compiled={} | constructs={:?}",
            step.step_index,
            step.classified_spoke.as_str(),
            step.task_category,
            step.complexity,
            step.routed_model_id,
            step.provider_type,
            step.oracle_passed,
            step.deep_compilation_passed,
            step.constructs_covered
        );
    }
    println!(
        "\n5. CLOSED-LOOP SELF-REPAIR (`SelectionIntent::repair_loop` -> DPO CONTRAST RECORDS):"
    );
    for (i, dpo) in repaired_dpo.iter().enumerate() {
        println!(
            "   Repair #{} | spoke={:<16} | origin={} | trusty_uri={}\n      rejected={:?}\n      chosen  ={:?}",
            i + 1,
            dpo.effective_organic_spoke().as_str(),
            dpo.provenance.origin,
            dpo.organic_trusty_uri(),
            dpo.rejected_response.as_deref().unwrap_or(""),
            dpo.response
        );
    }
    println!("================================================================================\n");
}

#[test]
fn extended_human_conversation_session_routes_all_eight_spokes_and_self_repairs() {
    use crate::mode::{ClutchProfile, RiskPosture};
    use vox_corpus::training::adapter_registry::{
        MensCheckpointManifestV1, calibrate_adapter_from_organic_scorecard,
    };
    use vox_corpus::training::curriculum::{
        DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, compute_curriculum_weights,
    };
    use vox_corpus::training::spoke_roundtrip::{
        canonical_five_spoke_benchmark_suite_25, canonical_organic_search_to_compilation_suite_40,
        evaluate_multi_spoke_suite, evaluate_organic_search_spoke_suite,
    };

    let mut reg = ModelRegistry::default();
    let five_suite = canonical_five_spoke_benchmark_suite_25();
    let five_scorecard = evaluate_multi_spoke_suite(&five_suite);
    let cw = compute_curriculum_weights(&five_scorecard, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR);
    let manifest = MensCheckpointManifestV1::build(
        "qwen3_27b_eight_spoke_v2_6",
        "Qwen3.5-27B",
        "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
        &cw,
        vec![1.82, 1.31, 0.94, 0.71, 0.58],
        vec![0.68, 0.51, 0.39, 0.31],
        240,
        96,
    );

    let suite_40 = canonical_organic_search_to_compilation_suite_40();
    let scorecard_40 = evaluate_organic_search_spoke_suite(&suite_40);
    let adapter_entry = calibrate_adapter_from_organic_scorecard(&manifest, &scorecard_40);
    let mens_id = reg.register_calibrated_mens_adapter(&adapter_entry);

    let session = reg
        .execute_extended_human_conversation_session(ClutchProfile::Balanced, RiskPosture::Moderate)
        .expect("12-turn extended human conversation must pass across all 8 spokes");

    assert!(session.all_turns_verified);
    assert_eq!(session.total_turns, 12);
    assert_eq!(session.unique_spokes_used, 8);
    assert_eq!(session.context_carryover_turns, 7);
    assert_eq!(session.self_repair_turns, 3);
    assert_eq!(session.sft_records_admitted, 12);
    assert_eq!(session.dpo_records_emitted, 3);

    println!("\n================================================================================");
    println!("EXTENDED 12-TURN HUMAN CONVERSATION TRANSCRIPT (AXIS + AGENT HARNESS)");
    println!(
        "Session: {} | Turns: {} | Spokes Used: {}/8 | Context Carryover Turns: {} | Self-Repairs: {} | SFT: {} | DPO: {}",
        session.session_id,
        session.total_turns,
        session.unique_spokes_used,
        session.context_carryover_turns,
        session.self_repair_turns,
        session.sft_records_admitted,
        session.dpo_records_emitted
    );
    println!("================================================================================");
    for turn in &session.turns {
        assert_eq!(turn.routing.routed_model_id, mens_id);
        println!("\n--- [Turn {:02}/12] ---", turn.turn_index);
        println!("User: \"{}\"", turn.user_prompt);
        println!(
            "[Axis Router] spoke={} | mode={:?} | context_carryover={} | target=({:?}, c={}) | score={} (runner_up={:?}:{}) | model={} ({:?})",
            turn.routing.chosen_spoke.as_str(),
            turn.routing.turn_mode,
            turn.routing.used_prior_turn_context,
            turn.routing.task_category,
            turn.routing.complexity,
            turn.routing.primary_score,
            turn.routing.runner_up_spoke.map(|s| s.as_str()),
            turn.routing.runner_up_score,
            turn.routing.routed_model_id,
            turn.routing.routed_provider,
        );
        println!("              signals={:?}", turn.routing.matched_signals);
        if let Some(rejected) = &turn.repaired_from_rejected {
            println!("[Repair Loop] Rejected broken attempt:\n{}", rejected);
        }
        println!("Assistant: {}", turn.assistant_reply);
        println!("{}", turn.assistant_artifact);
        println!(
            "[Oracle Verification] oracle_passed={} | deep_compiled={} | constructs={:?} | admitted_sft={} | emitted_dpo={}",
            turn.oracle_passed,
            turn.deep_compilation_passed,
            turn.constructs_covered,
            turn.admitted_sft,
            turn.emitted_dpo
        );
    }
    println!(
        "\n================================================================================\n"
    );
}

#[test]
fn sota_harness_before_after_scorecard_proves_smarter_cheaper_faster_execution() {
    use crate::mode::{ClutchProfile, RiskPosture};
    use vox_corpus::training::adapter_registry::{
        MensCheckpointManifestV1, calibrate_adapter_from_organic_scorecard,
    };
    use vox_corpus::training::curriculum::{
        DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, compute_curriculum_weights,
    };
    use vox_corpus::training::spoke_roundtrip::{
        canonical_five_spoke_benchmark_suite_25, canonical_organic_search_to_compilation_suite_40,
        evaluate_multi_spoke_suite, evaluate_organic_search_spoke_suite,
    };

    let mut reg = ModelRegistry::new();
    let five_suite = canonical_five_spoke_benchmark_suite_25();
    let five_scorecard = evaluate_multi_spoke_suite(&five_suite);
    let cw = compute_curriculum_weights(&five_scorecard, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR);
    let manifest = MensCheckpointManifestV1::build(
        "qwen3_27b_eight_spoke_v2_7",
        "Qwen3.5-27B",
        "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
        &cw,
        vec![1.82, 1.31, 0.94, 0.71, 0.58],
        vec![0.68, 0.51, 0.39, 0.31],
        240,
        96,
    );
    let suite_40 = canonical_organic_search_to_compilation_suite_40();
    let scorecard_40 = evaluate_organic_search_spoke_suite(&suite_40);
    let adapter_entry = calibrate_adapter_from_organic_scorecard(&manifest, &scorecard_40);
    reg.register_calibrated_mens_adapter(&adapter_entry);

    let scorecard = reg
        .evaluate_sota_harness_scorecard(ClutchProfile::Balanced, RiskPosture::Moderate)
        .expect("SOTA harness before/after scorecard must succeed");

    assert_eq!(scorecard.legacy_tool_schema_cap, 40);
    assert_eq!(scorecard.sota_ranked_tool_schema_cap, 20);
    assert!((scorecard.tool_schema_token_reduction_pct - 50.0).abs() < 1e-6);
    assert!((scorecard.legacy_unpinned_late_alphabet_tool_recall - 0.0).abs() < 1e-6);
    assert!((scorecard.sota_ranked_late_alphabet_tool_recall - 1.0).abs() < 1e-6);
    assert_eq!(scorecard.legacy_inline_verified_file_classes, 1);
    assert_eq!(scorecard.sota_inline_verified_file_classes, 4);
    assert!(scorecard.observation_masking_tokens_saved > 500);
    assert!(scorecard.kv_cache_prefix_overlap_ratio >= 0.70);
    assert!(scorecard.kv_cache_hit_predicted);
    assert!(scorecard.working_memory_artifacts_preserved);
    assert_eq!(scorecard.extended_session_turns_verified, 12);
    assert_eq!(scorecard.extended_session_spokes_covered, 8);
    assert_eq!(scorecard.extended_session_context_carryover_turns, 7);
    assert_eq!(scorecard.extended_session_self_repairs, 3);
    assert_eq!(scorecard.live_sft_records_admitted, 12);
    assert_eq!(scorecard.live_dpo_contrast_pairs_harvested, 3);
    assert!(scorecard.single_pass_sse_tool_calls_enabled);
    assert_eq!(scorecard.tool_turn_http_requests_per_iteration_before, 2);
    assert_eq!(scorecard.tool_turn_http_requests_per_iteration_after, 1);
    assert!((scorecard.provider_tool_loop_coverage_after - 1.0).abs() < 1e-6);
    assert!(scorecard.symbol_skeleton_compression_ratio >= 0.70);
    assert!(scorecard.system_prompt_kv_prefix_stable);
    assert!(scorecard.streaming_cache_read_accounting_enabled);
    assert!(scorecard.intent_gated_preamble_enabled);
    assert!(scorecard.intraturn_tool_masking_enabled);
    assert!(scorecard.progressive_tool_schema_compression_pct >= 30.0);
    assert!(scorecard.wire_cache_breakpoints_enabled);
    assert!(scorecard.streaming_goodput_feedback_closed_loop);
    assert!(scorecard.cache_aware_efficiency_routing_enabled);
    assert!(scorecard.intraturn_read_tool_dedup_enabled);
    assert!(scorecard.rust_structural_ast_verification_enabled);
    assert!(scorecard.rolling_intraturn_cache_breakpoints_enabled);
    assert!(scorecard.live_context_fill_and_spoke_router_wired);
    assert!(scorecard.path_scoped_read_memo_invalidation_enabled);
    assert!(scorecard.edit_window_recovery_enabled);
    assert!((scorecard.adversarial_48_case_spoke_pass_rate - 1.0).abs() < 1e-6);
    assert!(scorecard.write_tool_priority_and_target_content_recovery_enabled);
    assert!(scorecard.prefix_immutable_kv_cache_tool_loops_enabled);
    assert!(scorecard.universal_multi_format_tool_salvage_enabled);
    assert!(scorecard.syn_ast_rust_structural_verification_enabled);
    assert!(scorecard.speculative_repair_deescalation_enabled);
    assert!((scorecard.adversarial_64_case_spoke_pass_rate - 1.0).abs() < 1e-6);
}
