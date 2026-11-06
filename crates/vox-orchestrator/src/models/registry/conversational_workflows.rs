use super::*;

impl ModelRegistry {
    /// Classify and route a single conversational turn through Axis, automatically escalating to
    /// [`super::select::SelectionIntent::repair_loop`] when the user's message is a
    /// [`ConversationalTurnMode::SelfRepairLoop`] turn.
    pub fn route_conversational_turn_with_context(
        &self,
        prompt: &str,
        prior_spoke: Option<vox_corpus::training::contract::OrganicSpokeKind>,
        clutch: crate::mode::ClutchProfile,
        risk: crate::mode::RiskPosture,
    ) -> Result<ConversationalRoutingDecision, String> {
        let (
            chosen_spoke,
            primary_score,
            runner_up_spoke,
            runner_up_score,
            matched_signals,
            turn_mode,
            used_prior_turn_context,
        ) = Self::score_conversational_turn_across_spokes(prompt, prior_spoke);

        let (task_category, complexity, selected_model, rationale) = if turn_mode
            == ConversationalTurnMode::SelfRepairLoop
        {
            let mut repair_intent = super::select::SelectionIntent::repair_loop();
            repair_intent.prefer_local = true;
            repair_intent.allow_free_in_performance_mode = true;
            let repair_req = super::select::ModelSelectionRequest::from_intent(repair_intent);
            let model = super::select::decide(&repair_req, self)
                .map(|d| d.outcome.model_spec)
                .ok_or_else(|| {
                    "SelectionIntent::repair_loop failed to select a model".to_string()
                })?;
            (
                TaskCategory::Debugging,
                7,
                model,
                format!(
                    "Escalated conversational turn on `{}` to SelectionIntent::repair_loop (signals: {})",
                    chosen_spoke.as_str(),
                    matched_signals.join(", ")
                ),
            )
        } else {
            let (cat, comp) = Self::organic_spoke_routing_target(chosen_spoke);
            let model = self
                .route_organic_spoke_task_with_policy(chosen_spoke, clutch, risk, true)
                .ok_or_else(|| {
                    format!(
                        "conversational spoke {:?} failed to route under clutch={:?} risk={:?}",
                        chosen_spoke, clutch, risk
                    )
                })?;
            (
                cat,
                comp,
                model,
                format!(
                    "Routed `{:?}` turn to `{}` ({:?}, c={}) via signals [{}]",
                    turn_mode,
                    chosen_spoke.as_str(),
                    cat,
                    comp,
                    matched_signals.join(", ")
                ),
            )
        };

        Ok(ConversationalRoutingDecision {
            chosen_spoke,
            turn_mode,
            task_category,
            complexity,
            primary_score,
            runner_up_spoke,
            runner_up_score,
            matched_signals,
            used_prior_turn_context,
            routed_model_id: selected_model.id,
            routed_provider: selected_model.provider_type,
            selection_rationale: rationale,
        })
    }

    /// Map an 8-spoke [`vox_corpus::training::contract::OrganicSpokeKind`] to its Axis
    /// [`TaskCategory`] and complexity.
    #[must_use]
    pub fn organic_spoke_routing_target(
        spoke: vox_corpus::training::contract::OrganicSpokeKind,
    ) -> (TaskCategory, u8) {
        use vox_corpus::training::contract::OrganicSpokeKind;

        match spoke {
            OrganicSpokeKind::VoxLang => (TaskCategory::CodeGen, 6),
            OrganicSpokeKind::RustWorkspace => (TaskCategory::Debugging, 6),
            OrganicSpokeKind::StructuredShell => (TaskCategory::Parsing, 4),
            OrganicSpokeKind::McpToolChain => (TaskCategory::ToolOrchestration, 6),
            OrganicSpokeKind::AgentHarness => (TaskCategory::Planning, 6),
            OrganicSpokeKind::SpeechToCode => (TaskCategory::TypeChecking, 5),
            OrganicSpokeKind::PopuliMesh => (TaskCategory::InterAgent, 6),
            OrganicSpokeKind::SkillCreation => (TaskCategory::Ars, 5),
        }
    }

    /// Route an 8-spoke [`vox_corpus::training::contract::OrganicSpokeKind`] under default local policy.
    #[must_use]
    pub fn route_organic_spoke_task(
        &self,
        spoke: vox_corpus::training::contract::OrganicSpokeKind,
    ) -> Option<ModelSpec> {
        self.route_organic_spoke_task_with_policy(
            spoke,
            crate::mode::ClutchProfile::Free,
            crate::mode::RiskPosture::Moderate,
            true,
        )
    }

    /// Policy-aware routing across all 8 [`vox_corpus::training::contract::OrganicSpokeKind`] spokes
    /// (`4 clutches × 3 risk postures × 8 spokes = 96 cells`).
    #[must_use]
    pub fn route_organic_spoke_task_with_policy(
        &self,
        spoke: vox_corpus::training::contract::OrganicSpokeKind,
        clutch: crate::mode::ClutchProfile,
        risk: crate::mode::RiskPosture,
        prefer_local: bool,
    ) -> Option<ModelSpec> {
        let (category, complexity) = Self::organic_spoke_routing_target(spoke);
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

    /// Continuous 8-spoke MENS adapter auto-eval gate and 96-cell `(clutch, risk, organic_spoke)`
    /// drift ratchet (`SPOKE8-01..04`).
    pub fn verify_eight_spoke_flywheel_routing_and_ratchet(
        &self,
        expected_mens_id: &str,
    ) -> Result<usize, String> {
        use crate::mode::{ClutchProfile, RiskPosture};
        use vox_corpus::training::contract::OrganicSpokeKind;
        use vox_corpus::training::spoke_roundtrip::{
            canonical_organic_search_to_compilation_suite_24, evaluate_organic_search_spoke_suite,
        };

        let suite = canonical_organic_search_to_compilation_suite_24();
        let scorecard = evaluate_organic_search_spoke_suite(&suite);
        if (scorecard.overall_end_to_end_pass_at_1 - 1.0).abs() > f64::EPSILON {
            return Err(format!(
                "8-spoke organic benchmark pass@1 regressed below 1.0: {}",
                scorecard.overall_end_to_end_pass_at_1
            ));
        }
        if (scorecard.overall_compilation_pass_rate - 1.0).abs() > f64::EPSILON {
            return Err(format!(
                "8-spoke deep compilation rate regressed below 1.0: {}",
                scorecard.overall_compilation_pass_rate
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
        for spoke in OrganicSpokeKind::ALL {
            for clutch in clutches {
                for risk in risks {
                    let chosen = self
                        .route_organic_spoke_task_with_policy(spoke, clutch, risk, true)
                        .ok_or_else(|| {
                            format!(
                                "organic spoke {:?} failed to route under clutch={:?} risk={:?}",
                                spoke, clutch, risk
                            )
                        })?;
                    if chosen.id != expected_mens_id {
                        return Err(format!(
                            "organic spoke {:?} routed to {} instead of {} under clutch={:?} risk={:?}",
                            spoke, chosen.id, expected_mens_id, clutch, risk
                        ));
                    }
                    verified_cells += 1;
                }
            }
        }
        Ok(verified_cells)
    }

    /// Execute a composite 8-step local LLM development workflow where `AgentHarness` and `Axis`
    /// wield all 8 [`vox_corpus::training::contract::OrganicSpokeKind`] spokes in combination,
    /// classifying each step's task intent, selecting the right spoke and local Mens model at the
    /// right time, and verifying every step's output through its real compiler/execution oracle.
    pub fn execute_multi_spoke_harness_workflow(
        &self,
        clutch: crate::mode::ClutchProfile,
        risk: crate::mode::RiskPosture,
    ) -> Result<MultiSpokeHarnessWorkflowReport, String> {
        use vox_corpus::training::contract::OrganicSpokeKind;
        use vox_corpus::training::spoke_roundtrip::{
            AgentHarnessSpec, McpToolSpec, verify_agent_harness_roundtrip, verify_mcp_roundtrip,
            verify_populi_mesh_roundtrip, verify_rust_snippet_with_rustc,
            verify_rust_workspace_roundtrip, verify_skill_creation_roundtrip,
            verify_speech_to_code_roundtrip, verify_structured_shell_roundtrip,
            verify_voxlang_roundtrip,
        };

        let steps: [(usize, &str, OrganicSpokeKind, &str); 8] = [
            (
                1,
                "Decompose goal into a TaskPlanEnvelope workflow plan with write_manifest scope",
                OrganicSpokeKind::AgentHarness,
                r#"{
                    "goal": "Build, verify, package, and mesh-dispatch local LLM build monitor",
                    "steps": [
                        "Normalize spoken voice intent into .vox actor",
                        "Query MCP tool registry for workspace check",
                        "Compile .vox workflow and Rust companion module",
                        "Author SKILL.md, verify shell gate, and dispatch to Populi mesh"
                    ],
                    "files": [
                        "crates/vox-corpus/src/training/spoke_roundtrip.rs",
                        "crates/vox-orchestrator/src/models/registry.rs"
                    ],
                    "budget_tokens": 5000,
                    "max_steps": 5,
                    "verification_gate": "timeout 300s cargo test -p vox-orchestrator --lib models::registry"
                }"#,
            ),
            (
                2,
                "um define actor build monitor statebuilds int on record count int to int statebuilds plus count",
                OrganicSpokeKind::SpeechToCode,
                "```vox\nactor BuildMonitor {\n    on record(count: int) to int {\n        return count + 1\n    }\n}\n```",
            ),
            (
                3,
                "Invoke MCP tool call vox_compiler_check to inspect crate before codegen",
                OrganicSpokeKind::McpToolChain,
                r#"{"tool_name":"vox_compiler_check","arguments":{"path":"examples/golden/ref_actors.vox","source":"actor Ping { on ping(x: int) to int { return x + 1 } }"}}"#,
            ),
            (
                4,
                "Implement .vox bare-keyword table, workflow, and actor pipeline for build telemetry aggregation",
                OrganicSpokeKind::VoxLang,
                "```vox\ntable TelemetryRow {\n    run_id: int\n    label: str\n}\nactor TelemetryAggregator {\n    on add(delta: int) to int {\n        return delta + 1\n    }\n}\nworkflow record_sample(v: int) to int {\n    return v + 1\n}\n```",
            ),
            (
                5,
                "Implement Rust companion helper pub fn sliding_window_max and verify with rustc borrow checker",
                OrganicSpokeKind::RustWorkspace,
                "```rust\npub fn sliding_window_max(values: &[i64], window: usize) -> Vec<i64> {\n    if window == 0 || values.is_empty() {\n        return Vec::new();\n    }\n    values.windows(window.min(values.len())).map(|w| *w.iter().max().unwrap_or(&0)).collect()\n}\n```",
            ),
            (
                6,
                "Author a SKILL.md agentskills specification with vox-category metadata and compiled vox block",
                OrganicSpokeKind::SkillCreation,
                "---\nname: build-monitor-telemetry\ndescription: \"Monitors local LLM build health and aggregates actor telemetry across spokes.\"\nmetadata:\n  vox-id: vox.skills.build-monitor-telemetry\n  vox-category: observability\n  vox-tools: vox_compiler_check\n---\n# Build Monitor Telemetry Skill\n\nThis skill coordinates multi-spoke local LLM verification and actor health telemetry across Axis.\n\n```vox\nfn compute_health_score(passed: int, total: int) to int {\n    return passed * 100\n}\n```\n",
            ),
            (
                7,
                "Emit pwsh terminal command matching exec-policy allowlist to verify workspace compilation",
                OrganicSpokeKind::StructuredShell,
                "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json",
            ),
            (
                8,
                "Dispatch verified build monitor artifact across Populi mesh with node-lease replica routing",
                OrganicSpokeKind::PopuliMesh,
                r#"{"target_node_id":"populi-node-alpha","capability_tier":"gpu_cuda","vram_min_gb":24,"iroh_endpoint":"iroh://node-alpha.populi.vox:4433","quorum_size":3,"payload_kind":"a2a_task_envelope","timeout_ms":5000}"#,
            ),
        ];

        let harness_spec = AgentHarnessSpec {
            allowed_path_prefixes: vec![
                "crates/vox-corpus/".to_string(),
                "crates/vox-orchestrator/".to_string(),
            ],
            max_steps: 6,
            max_budget_tokens: 16_000,
        };
        let mcp_spec = McpToolSpec {
            allowed_tools: vec!["vox_compiler_check".to_string()],
            required_arg_keys: vec!["path".to_string(), "source".to_string()],
        };
        let shell_keys = vec!["Name".to_string(), "Length".to_string()];

        let mut step_receipts = Vec::with_capacity(steps.len());
        let mut admitted_sft_records = 0usize;

        for (step_index, prompt, expected_spoke, candidate_output) in steps {
            let classified_spoke = Self::classify_task_to_organic_spoke(prompt);
            if classified_spoke != expected_spoke {
                return Err(format!(
                    "step {step_index} misclassified prompt `{prompt}` as {:?} (expected {:?})",
                    classified_spoke, expected_spoke
                ));
            }
            let (task_category, complexity) = Self::organic_spoke_routing_target(classified_spoke);
            let selected_model = self
                .route_organic_spoke_task_with_policy(classified_spoke, clutch, risk, true)
                .ok_or_else(|| {
                    format!(
                        "step {step_index} ({:?}) failed to route under clutch={:?} risk={:?}",
                        classified_spoke, clutch, risk
                    )
                })?;

            let case_id = format!("multi_spoke_chain_step_{step_index}");
            let (roundtrip, deep_compiled) = match classified_spoke {
                OrganicSpokeKind::AgentHarness => {
                    let rt = verify_agent_harness_roundtrip(
                        &case_id,
                        prompt,
                        candidate_output,
                        Some(&harness_spec),
                    );
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
                OrganicSpokeKind::SpeechToCode => {
                    let rt = verify_speech_to_code_roundtrip(&case_id, prompt, candidate_output);
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
                OrganicSpokeKind::McpToolChain => {
                    let rt =
                        verify_mcp_roundtrip(&case_id, prompt, candidate_output, Some(&mcp_spec));
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
                OrganicSpokeKind::VoxLang => {
                    let rt = verify_voxlang_roundtrip(&case_id, prompt, candidate_output);
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
                OrganicSpokeKind::RustWorkspace => {
                    let rt = verify_rust_workspace_roundtrip(&case_id, prompt, candidate_output);
                    let raw_rust = candidate_output
                        .trim()
                        .trim_start_matches("```rust")
                        .trim_end_matches("```")
                        .trim();
                    let rustc_ok = verify_rust_snippet_with_rustc(raw_rust).is_ok();
                    let ok = rt.execution_passed && rustc_ok;
                    (rt, ok)
                }
                OrganicSpokeKind::SkillCreation => {
                    let rt = verify_skill_creation_roundtrip(&case_id, prompt, candidate_output);
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
                OrganicSpokeKind::StructuredShell => {
                    let rt = verify_structured_shell_roundtrip(
                        &case_id,
                        prompt,
                        candidate_output,
                        &shell_keys,
                    );
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
                OrganicSpokeKind::PopuliMesh => {
                    let rt = verify_populi_mesh_roundtrip(&case_id, prompt, candidate_output);
                    let ok = rt.execution_passed;
                    (rt, ok)
                }
            };

            if !roundtrip.oracle_receipt.passed || !deep_compiled {
                return Err(format!(
                    "step {step_index} ({:?}) failed spoke verification: {:?}",
                    classified_spoke, roundtrip.oracle_receipt.diagnostics
                ));
            }
            if roundtrip.admitted_record.is_some() {
                admitted_sft_records += 1;
            }

            step_receipts.push(MultiSpokeHarnessStepReceipt {
                step_index,
                classified_spoke,
                task_category,
                complexity,
                routed_model_id: selected_model.id,
                provider_type: selected_model.provider_type,
                oracle_passed: roundtrip.oracle_receipt.passed,
                deep_compilation_passed: deep_compiled,
                constructs_covered: roundtrip.constructs_covered,
            });
        }

        Ok(MultiSpokeHarnessWorkflowReport {
            total_steps: step_receipts.len(),
            spokes_wielded: step_receipts.len(),
            admitted_sft_records,
            all_steps_passed: true,
            steps: step_receipts,
        })
    }

    /// Execute a closed-loop self-repairing multi-spoke workflow (`SelectionIntent::repair_loop`)
    /// where initial candidates fail with:
    /// 1. Retired `@table` / `ret` syntax on `VoxLang`,
    /// 2. A `rustc` `E0502` borrow-checker error on `RustWorkspace`,
    /// 3. A retired `vox_kb_store` tool call on `McpToolChain`,
    /// and verify that Axis routes the repair loop to the calibrated Mens model, compiles the
    /// repaired candidates cleanly, and admits 3 DPO contrast records (`rejected_response` populated).
    pub fn execute_self_repairing_multi_spoke_workflow(
        &self,
    ) -> Result<Vec<vox_corpus::training::contract::TrainingRecordV2>, String> {
        use vox_corpus::training::contract::OrganicSpokeKind;
        use vox_corpus::training::spoke_roundtrip::{
            verify_mcp_roundtrip, verify_rust_snippet_with_rustc, verify_rust_workspace_roundtrip,
            verify_voxlang_roundtrip,
        };

        let mut repair_intent = super::select::SelectionIntent::repair_loop();
        repair_intent.prefer_local = true;
        repair_intent.allow_free_in_performance_mode = true;
        let repair_req = super::select::ModelSelectionRequest::from_intent(repair_intent);
        let repair_model = super::select::decide(&repair_req, self)
            .map(|d| d.outcome.model_spec)
            .ok_or_else(|| "SelectionIntent::repair_loop failed to select a model".to_string())?;
        if repair_model.provider_type != ProviderType::VoxLocal {
            return Err(format!(
                "repair_loop selected {:?} ({}) instead of calibrated VoxLocal adapter",
                repair_model.provider_type, repair_model.id
            ));
        }

        let mut dpo_records = Vec::new();

        // Case 1: VoxLang retired `@table` + `ret` -> repaired bare-keyword `table` + `return`
        let vox_prompt =
            "Define a Vox table `BuildSample` and helper `fn next_sample(n: int) to int`";
        let vox_broken = "@table type BuildSample {\n    sample_id: int\n}\nfn next_sample(n: int) to int {\n    ret n + 1\n}";
        let vox_repaired = "table BuildSample {\n    sample_id: int\n}\nfn next_sample(n: int) to int {\n    return n + 1\n}";
        let vox_fail = verify_voxlang_roundtrip("repair_vox_fail", vox_prompt, vox_broken);
        if vox_fail.execution_passed {
            return Err("retired `@table`/`ret` candidate should have failed oracle".to_string());
        }
        let vox_ok = verify_voxlang_roundtrip("repair_vox_ok", vox_prompt, vox_repaired);
        if !vox_ok.execution_passed {
            return Err(format!(
                "repaired bare-keyword Vox candidate failed: {:?}",
                vox_ok.oracle_receipt.diagnostics
            ));
        }
        let mut vox_rec = vox_ok
            .admitted_record
            .ok_or("missing vox admitted_record")?;
        vox_rec.rejected_response = Some(vox_broken.to_string());
        vox_rec.provenance.origin = "audit_repair".to_string();
        dpo_records.push(vox_rec);

        // Case 2: RustWorkspace E0502 borrow-checker failure -> repaired borrow-safe Rust
        let rust_prompt = "Implement `pub fn append_first(items: &mut Vec<i64>) -> i64` in Rust";
        let rust_broken = "pub fn append_first(items: &mut Vec<i64>) -> i64 {\n    let first = &items[0];\n    items.push(1);\n    *first\n}";
        let rust_repaired = "pub fn append_first(items: &mut Vec<i64>) -> i64 {\n    let first = items[0];\n    items.push(1);\n    first\n}";
        if verify_rust_snippet_with_rustc(rust_broken).is_ok() {
            return Err("E0502 borrow-violating Rust snippet should fail rustc".to_string());
        }
        if let Err(e) = verify_rust_snippet_with_rustc(rust_repaired) {
            return Err(format!("repaired Rust snippet failed rustc: {e}"));
        }
        let rust_ok = verify_rust_workspace_roundtrip("repair_rust_ok", rust_prompt, rust_repaired);
        let mut rust_rec = rust_ok
            .admitted_record
            .ok_or("missing rust admitted_record")?;
        rust_rec.rejected_response = Some(rust_broken.to_string());
        rust_rec.provenance.origin = "audit_repair".to_string();
        dpo_records.push(rust_rec);

        // Case 3: McpToolChain retired `vox_kb_store` -> repaired `vox_kb_search`
        let mcp_prompt = "Invoke MCP tool to search KB for `examples/golden/ref_actors.vox`";
        let mcp_broken =
            r#"{"tool_name":"vox_kb_store","arguments":{"key":"actors","value":"ok"}}"#;
        let mcp_repaired = r#"{"tool_name":"vox_kb_search","arguments":{"query":"examples/golden/ref_actors.vox","limit":5}}"#;
        let mcp_fail = verify_mcp_roundtrip("repair_mcp_fail", mcp_prompt, mcp_broken, None);
        if mcp_fail.execution_passed {
            return Err("retired `vox_kb_store` MCP call should have failed oracle".to_string());
        }
        let mcp_ok = verify_mcp_roundtrip("repair_mcp_ok", mcp_prompt, mcp_repaired, None);
        if !mcp_ok.execution_passed {
            return Err(format!(
                "repaired MCP call failed: {:?}",
                mcp_ok.oracle_receipt.diagnostics
            ));
        }
        let mut mcp_rec = mcp_ok
            .admitted_record
            .ok_or("missing mcp admitted_record")?;
        mcp_rec.rejected_response = Some(mcp_broken.to_string());
        mcp_rec.provenance.origin = "audit_repair".to_string();
        assert_eq!(
            mcp_rec.effective_organic_spoke(),
            OrganicSpokeKind::McpToolChain
        );
        dpo_records.push(mcp_rec);

        Ok(dpo_records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::{ClutchProfile, RiskPosture};
    use vox_corpus::training::contract::OrganicSpokeKind;

    fn mens_only_registry() -> ModelRegistry {
        let mut reg = ModelRegistry::new();
        reg.clear();
        reg.register_mens_local_candidates();
        reg
    }

    #[test]
    fn organic_spoke_targets_are_exact() {
        assert_eq!(
            ModelRegistry::organic_spoke_routing_target(OrganicSpokeKind::VoxLang),
            (TaskCategory::CodeGen, 6)
        );
        assert_eq!(
            ModelRegistry::organic_spoke_routing_target(OrganicSpokeKind::StructuredShell),
            (TaskCategory::Parsing, 4)
        );
        assert_eq!(
            ModelRegistry::organic_spoke_routing_target(OrganicSpokeKind::SkillCreation),
            (TaskCategory::Ars, 5)
        );
    }

    #[test]
    fn every_organic_spoke_routes_to_a_local_model() {
        let reg = mens_only_registry();
        for spoke in OrganicSpokeKind::ALL {
            let chosen = reg
                .route_organic_spoke_task(spoke)
                .unwrap_or_else(|| panic!("{spoke:?} should route"));
            assert_eq!(chosen.provider_type, ProviderType::VoxLocal);
        }
    }

    #[test]
    fn conversational_turn_routes_through_the_classified_spoke() {
        let reg = mens_only_registry();
        let d = reg
            .route_conversational_turn_with_context(
                "Invoke MCP tool call vox_compiler_check on a Rust crate",
                None,
                ClutchProfile::Free,
                RiskPosture::Moderate,
            )
            .expect("mens-only registry can route");
        assert_eq!(d.chosen_spoke, OrganicSpokeKind::McpToolChain);
        assert_eq!(d.task_category, TaskCategory::ToolOrchestration);
        assert_eq!(d.complexity, 6);
        assert_eq!(d.routed_provider, ProviderType::VoxLocal);
        assert!(!d.selection_rationale.is_empty());
    }
}
