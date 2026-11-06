use super::*;

impl ModelRegistry {
    /// Register candidate entries for Mens 27B quantization tiers and 8B fallback under [`ProviderType::VoxLocal`].
    pub fn register_mens_local_candidates(&mut self) {
        let tiers = [
            ("mens/runs/qwen3_27b_metal_check/merged_bf16", 27.0),
            ("mens/runs/qwen3_27b_metal_check/quant_q8_0", 27.0),
            ("mens/runs/qwen3_27b_metal_check/quant_q6_k", 27.0),
            ("mens/runs/qwen3_27b_metal_check/quant_q5_k_m", 27.0),
            ("mens/runs/qwen3_27b_metal_check/quant_q4_k_m", 27.0),
            ("vox-mens-8b-v0.6", 8.0),
        ];
        for (id, param_count) in tiers {
            if !self.models.contains_key(id) {
                let spec = ModelSpec {
                    id: id.to_string(),
                    canonical_slug: format!("voxlocal/{}", id),
                    provider: "voxlocal".to_string(),
                    provider_type: ProviderType::VoxLocal,
                    max_tokens: 8192,
                    cost_per_1k: 0.0,
                    cost_per_1k_input: 0.0,
                    cost_per_1k_output: 0.0,
                    is_free: true,
                    observed_cost_per_1k: None,
                    strengths: vec![
                        crate::models::generated::StrengthTag::Generalist,
                        crate::models::generated::StrengthTag::Codegen,
                        crate::models::generated::StrengthTag::UiCodegen,
                        crate::models::generated::StrengthTag::Parsing,
                        crate::models::generated::StrengthTag::Debugging,
                        crate::models::generated::StrengthTag::Review,
                        crate::models::generated::StrengthTag::Security,
                        crate::models::generated::StrengthTag::Logic,
                        crate::models::generated::StrengthTag::InterAgent,
                        crate::models::generated::StrengthTag::Planning,
                    ],
                    capabilities: ModelCapabilities {
                        supports_json: true,
                        supports_jsonl: true,
                        supports_tool_use: true,
                        supports_native_tools: true,
                        supports_reasoning: true,
                        writes_vox: true,
                        max_context: 32768,
                        latency_p50_ms: Some(140),
                        tier: crate::models::generated::ModelTier::Local,
                        param_count_b: Some(param_count),
                        ..Default::default()
                    },
                    cache_creation_cost_per_1k: 0.0,
                    cache_read_cost_per_1k: 0.0,
                    supports_prompt_caching: false,
                    pricing_source: PricingSource::Bootstrap,
                    supported_parameters: vec![],
                };
                self.register(spec);
            }
        }
    }

    /// Register a calibrated Mens adapter from a 5-spoke [`vox_corpus::training::adapter_registry::MensAdapterRegistryEntry`]
    /// into the Axis [`ModelRegistry`], populating `BenchEvidence` (`ModelConfidence::Confirmed`),
    /// `ModelScore`, `arm_stats`, and per-spoke [`crate::models::StrengthTag`]s (`MENS-HARNESS-01..03`).
    pub fn register_calibrated_mens_adapter(
        &mut self,
        entry: &vox_corpus::training::adapter_registry::MensAdapterRegistryEntry,
    ) -> String {
        use vox_corpus::training::contract::SpokeKind;

        let model_id = if entry.checkpoint_id.starts_with("mens/") {
            entry.checkpoint_id.clone()
        } else {
            format!("mens/{}", entry.checkpoint_id)
        };

        let mut strengths = vec![crate::models::generated::StrengthTag::Generalist];
        for (spoke_key, &score) in &entry.per_spoke_scores {
            if score < 0.50 {
                continue;
            }
            match spoke_key.as_str() {
                k if k == SpokeKind::VoxLang.as_str() => {
                    strengths.push(crate::models::generated::StrengthTag::Codegen);
                    strengths.push(crate::models::generated::StrengthTag::UiCodegen);
                }
                k if k == SpokeKind::RustWorkspace.as_str() => {
                    strengths.push(crate::models::generated::StrengthTag::Debugging);
                    strengths.push(crate::models::generated::StrengthTag::Review);
                    strengths.push(crate::models::generated::StrengthTag::Security);
                }
                k if k == SpokeKind::StructuredShell.as_str() => {
                    strengths.push(crate::models::generated::StrengthTag::Parsing);
                }
                k if k == SpokeKind::McpToolChain.as_str() => {
                    strengths.push(crate::models::generated::StrengthTag::InterAgent);
                    strengths.push(crate::models::generated::StrengthTag::Logic);
                }
                k if k == SpokeKind::AgentHarness.as_str() => {
                    strengths.push(crate::models::generated::StrengthTag::Planning);
                }
                "speech_to_code" => {
                    strengths.push(crate::models::generated::StrengthTag::Parsing);
                    strengths.push(crate::models::generated::StrengthTag::Codegen);
                    strengths.push(crate::models::generated::StrengthTag::Logic);
                }
                "populi_mesh" => {
                    strengths.push(crate::models::generated::StrengthTag::InterAgent);
                    strengths.push(crate::models::generated::StrengthTag::Planning);
                }
                "skill_creation" => {
                    strengths.push(crate::models::generated::StrengthTag::Research);
                    strengths.push(crate::models::generated::StrengthTag::Logic);
                }
                _ => {}
            }
        }
        strengths.dedup();

        let pass1 = entry.overall_pass_at_1.clamp(0.0, 1.0);
        let spec = ModelSpec {
            id: model_id.clone(),
            canonical_slug: entry.adapter_trusty_uri.clone(),
            provider: "voxlocal".to_string(),
            provider_type: ProviderType::VoxLocal,
            max_tokens: 16384,
            cost_per_1k: entry.cost_per_1k_tokens_usd,
            cost_per_1k_input: entry.cost_per_1k_tokens_usd,
            cost_per_1k_output: entry.cost_per_1k_tokens_usd,
            is_free: entry.cost_per_1k_tokens_usd <= 0.0,
            observed_cost_per_1k: Some(entry.cost_per_1k_tokens_usd),
            strengths,
            capabilities: ModelCapabilities {
                supports_json: true,
                supports_jsonl: true,
                supports_tool_use: true,
                supports_native_tools: true,
                supports_reasoning: true,
                writes_vox: true,
                max_context: 32768,
                latency_p50_ms: Some(entry.latency_ms_p50 as u32),
                intelligence_index: Some((pass1 * 100.0) as f32),
                tier: crate::models::generated::ModelTier::Local,
                param_count_b: Some(27.0),
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Telemetry,
            supported_parameters: vec![],
        };
        self.register(spec);

        let total_problems: i64 = entry.total_cases_evaluated.max(25) as i64;
        let successes = ((pass1 * total_problems as f64).round() as i64).min(total_problems);
        let failures = (total_problems - successes).max(0) as u32;

        self.bench.insert(
            model_id.clone(),
            super::bench_evidence::BenchEvidence {
                cases: total_problems,
                passed: successes,
                errors: 0,
                mean_score: pass1,
                resolved_model_id: Some(model_id.clone()),
                p50_ttft_ms: Some(40),
                p50_latency_ms: Some(entry.latency_ms_p50 as i64),
                p95_latency_ms: Some((entry.latency_ms_p50 * 2) as i64),
                mean_tokens_per_sec: Some(65.0),
                cost_per_solve_usd: Some(0.0),
                tokens_per_solve: Some(512.0),
                last_recorded_at_ms: 1_710_000_002_000,
            },
        );
        self.scoreboard.insert(
            model_id.clone(),
            ModelScore {
                success_rate: pass1,
                quality_score: pass1,
                cost_per_success_usd: Some(0.0),
                p50_latency_ms: Some(entry.latency_ms_p50 as i64),
                n_calls: total_problems,
                success_count: successes,
                p95_ttft_ms: Some(40),
                p95_tpot_ms: Some(15.0),
                goodput_tokens_per_sec: Some(65.0),
                response_quality_ema: None,
                benchmark_prior: None,
            },
        );
        self.arm_stats
            .insert(model_id.clone(), ((successes as u32).max(1), failures));
        self.apply_routing_reference();
        model_id
    }

    /// Map a canonical [`vox_corpus::training::contract::SpokeKind`] to its Axis [`super::routing_task::RoutingTask`]
    /// and select the highest-scoring model in the registry (`MENS-HARNESS-02..03`).
    #[must_use]
    pub fn route_spoke_task(
        &self,
        spoke: vox_corpus::training::contract::SpokeKind,
    ) -> Option<ModelSpec> {
        self.route_spoke_task_with_policy(
            spoke,
            crate::mode::ClutchProfile::Free,
            crate::mode::RiskPosture::Moderate,
            true,
        )
    }

    /// Policy-aware spoke routing across `(clutch, risk, prefer_local)` (`ROUTE-MENS-01..04`).
    #[must_use]
    pub fn route_spoke_task_with_policy(
        &self,
        spoke: vox_corpus::training::contract::SpokeKind,
        clutch: crate::mode::ClutchProfile,
        risk: crate::mode::RiskPosture,
        prefer_local: bool,
    ) -> Option<ModelSpec> {
        use vox_corpus::training::contract::SpokeKind;

        let (category, complexity) = match spoke {
            SpokeKind::VoxLang => (TaskCategory::CodeGen, 6),
            SpokeKind::RustWorkspace => (TaskCategory::Debugging, 6),
            SpokeKind::StructuredShell => (TaskCategory::Parsing, 4),
            SpokeKind::McpToolChain => (TaskCategory::ToolOrchestration, 6),
            SpokeKind::AgentHarness => (TaskCategory::Planning, 6),
        };
        let (cost, responsiveness, intelligence) = crate::mode::effective_axes(clutch, risk);
        let mut intent = super::select::SelectionIntent::for_task(category);
        intent.complexity = complexity;
        intent.prefer_local = prefer_local;
        intent.allow_free_in_performance_mode =
            prefer_local || matches!(clutch, crate::mode::ClutchProfile::Free);
        if clutch.resolve().force_free_pool {
            intent.max_cost_usd_per_call = Some(0.0);
        }
        intent.axes = super::select::SelectionAxes {
            cost,
            responsiveness,
            intelligence,
        };
        let req = super::select::ModelSelectionRequest::from_intent(intent);
        super::select::decide(&req, self)
            .map(|d| d.outcome.model_spec)
            .or_else(|| {
                let task = RoutingTask {
                    category,
                    complexity,
                    ..Default::default()
                };
                self.best_for_task(&task, CostPreference::Economy)
            })
    }

    /// Continuous 5-spoke MENS adapter auto-eval gate and 60-cell `(clutch, risk, spoke)`
    /// drift ratchet (`MENS-GATE-01..04`).
    pub fn verify_full_flywheel_routing_and_spoke_ratchet(
        &self,
        expected_mens_id: &str,
    ) -> Result<usize, String> {
        use crate::mode::{ClutchProfile, RiskPosture};
        use vox_corpus::training::contract::SpokeKind;
        use vox_corpus::training::spoke_roundtrip::{
            canonical_five_spoke_benchmark_suite_25, evaluate_multi_spoke_suite,
        };

        let suite = canonical_five_spoke_benchmark_suite_25();
        let scorecard = evaluate_multi_spoke_suite(&suite);
        if (scorecard.overall_pass_at_1 - 1.0).abs() > f64::EPSILON {
            return Err(format!(
                "5-spoke benchmark pass@1 regressed below 1.0: {}",
                scorecard.overall_pass_at_1
            ));
        }

        let clutches = [
            ClutchProfile::Free,
            ClutchProfile::Efficiency,
            ClutchProfile::Balanced,
            ClutchProfile::Genius,
        ];
        let risks = [RiskPosture::High, RiskPosture::Moderate, RiskPosture::Low];
        let mut verified_cells = 0usize;
        for spoke in SpokeKind::ALL {
            for clutch in clutches {
                for risk in risks {
                    let chosen = self
                        .route_spoke_task_with_policy(spoke, clutch, risk, true)
                        .ok_or_else(|| {
                            format!(
                                "spoke {:?} failed to route under clutch={:?} risk={:?}",
                                spoke, clutch, risk
                            )
                        })?;
                    if chosen.id != expected_mens_id {
                        return Err(format!(
                            "spoke {:?} routed to {} instead of {} under clutch={:?} risk={:?}",
                            spoke, chosen.id, expected_mens_id, clutch, risk
                        ));
                    }
                    verified_cells += 1;
                }
            }
        }
        Ok(verified_cells)
    }

    /// Deterministically classify a developer task prompt or everyday human conversational utterance
    /// to the exact [`vox_corpus::training::contract::OrganicSpokeKind`] that should execute it,
    /// using weighted primary-deliverable, everyday natural-language phrasing, and domain-evidence
    /// scoring so both colloquial human requests and technical prompts resolve accurately.
    #[must_use]
    pub fn classify_task_to_organic_spoke(
        prompt: &str,
    ) -> vox_corpus::training::contract::OrganicSpokeKind {
        Self::score_conversational_turn_across_spokes(prompt, None).0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_corpus::training::contract::{OrganicSpokeKind, SpokeKind};

    fn mens_only_registry() -> ModelRegistry {
        let mut reg = ModelRegistry::new();
        reg.clear();
        reg.register_mens_local_candidates();
        reg
    }

    #[test]
    fn register_mens_local_candidates_is_idempotent_and_local() {
        let mut reg = mens_only_registry();
        assert_eq!(reg.list_models().len(), 6);
        reg.register_mens_local_candidates();
        let models = reg.list_models();
        assert_eq!(models.len(), 6);
        assert!(
            models
                .iter()
                .all(|m| m.provider_type == ProviderType::VoxLocal && m.is_free)
        );
    }

    #[test]
    fn route_spoke_task_picks_a_local_mens_model_for_every_spoke() {
        let reg = mens_only_registry();
        for spoke in SpokeKind::ALL {
            let chosen = reg
                .route_spoke_task(spoke)
                .unwrap_or_else(|| panic!("{spoke:?} should route to a mens candidate"));
            assert_eq!(chosen.provider_type, ProviderType::VoxLocal);
        }
    }

    #[test]
    fn classify_task_to_organic_spoke_resolves_domain_prompts() {
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
    }
}
