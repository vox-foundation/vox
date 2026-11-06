use super::*;

pub(super) fn organic_suite_24_part2() -> Vec<OrganicSearchSpokeCase> {
    use crate::training::contract::OrganicSpokeKind;
    vec![
        // ── Spoke 5: AgentHarness (3 organic search-to-compilation tasks) ─────
        OrganicSearchSpokeCase {
            case_id: "organic_harness_01_spoke_roundtrip_plan".to_string(),
            spoke: OrganicSpokeKind::AgentHarness,
            search_query: "verify_vox_admission check_vox_retired_and_enforcement_rules ReadinessReport corpus_readiness.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/corpus/corpus_readiness.rs".to_string(),
                "crates/vox-corpus/src/flywheel.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/corpus/corpus_readiness.rs".to_string(),
            expected_grounding_tokens: vec![
                "verify_vox_admission".to_string(),
                "check_vox_retired_and_enforcement_rules".to_string(),
                "ReadinessReport".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_harness_01_spoke_roundtrip_plan".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a `vox-dei` TaskPlanEnvelope JSON grounded in `crates/vox-corpus/src/corpus/corpus_readiness.rs`.".to_string(),
                candidates: vec![
                    r#"{
                        "goal": "Verify Vox grammar unification and retired-surface admission rules in vox-corpus",
                        "steps": [
                            "Inspect check_vox_retired_and_enforcement_rules in crates/vox-corpus/src/corpus/corpus_readiness.rs",
                            "Run timeout 300s cargo test -p vox-corpus --lib corpus_readiness"
                        ],
                        "files": ["crates/vox-corpus/src/corpus/corpus_readiness.rs"],
                        "budget_tokens": 4500,
                        "max_steps": 4,
                        "verification_gate": "timeout 300s cargo test -p vox-corpus --lib corpus_readiness"
                    }"#
                    .to_string(),
                ],
                mcp_spec: None,
                harness_spec: Some(AgentHarnessSpec {
                    allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
                    max_steps: 5,
                    max_budget_tokens: 16_000,
                }),
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_harness_02_model_registry_plan".to_string(),
            spoke: OrganicSpokeKind::AgentHarness,
            search_query: "register_calibrated_mens_adapter route_spoke_task calibrated_mens_adapter_routes_all_five_spokes registry.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-orchestrator/src/models/registry.rs".to_string(),
                "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-orchestrator/src/models/registry.rs".to_string(),
            expected_grounding_tokens: vec![
                "register_calibrated_mens_adapter".to_string(),
                "route_spoke_task".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_harness_02_model_registry_plan".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a `vox-dei` TaskPlanEnvelope JSON grounded in `crates/vox-orchestrator/src/models/registry.rs`.".to_string(),
                candidates: vec![
                    r#"{
                        "goal": "Verify calibrated Mens adapter routing across all 5 spokes in vox-orchestrator",
                        "steps": [
                            "Inspect register_calibrated_mens_adapter and route_spoke_task in crates/vox-orchestrator/src/models/registry.rs",
                            "Run timeout 300s cargo test -p vox-orchestrator --lib calibrated_mens_adapter"
                        ],
                        "files": ["crates/vox-orchestrator/src/models/registry.rs"],
                        "budget_tokens": 5000,
                        "max_steps": 4,
                        "verification_gate": "timeout 300s cargo test -p vox-orchestrator --lib calibrated_mens_adapter"
                    }"#
                    .to_string(),
                ],
                mcp_spec: None,
                harness_spec: Some(AgentHarnessSpec {
                    allowed_path_prefixes: vec!["crates/vox-orchestrator/".to_string()],
                    max_steps: 5,
                    max_budget_tokens: 16_000,
                }),
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_harness_03_flywheel_burnin_plan".to_string(),
            spoke: OrganicSpokeKind::AgentHarness,
            search_query: "run_before_after_five_spoke_evaluation MensBeforeAfterEvaluationV1 flywheel.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/flywheel.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/flywheel.rs".to_string(),
            expected_grounding_tokens: vec![
                "run_before_after_five_spoke_evaluation".to_string(),
                "MensBeforeAfterEvaluationV1".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_harness_03_flywheel_burnin_plan".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a `vox-dei` TaskPlanEnvelope JSON grounded in `crates/vox-corpus/src/flywheel.rs`.".to_string(),
                candidates: vec![
                    r#"{
                        "goal": "Execute 25-problem before/after 5-spoke burn-in evaluation and persist to VoxDb",
                        "steps": [
                            "Verify run_before_after_five_spoke_evaluation in crates/vox-corpus/src/flywheel.rs",
                            "Run timeout 300s cargo test -p vox-corpus --lib burnin_tests"
                        ],
                        "files": ["crates/vox-corpus/src/flywheel.rs"],
                        "budget_tokens": 4000,
                        "max_steps": 4,
                        "verification_gate": "timeout 300s cargo test -p vox-corpus --lib burnin_tests"
                    }"#
                    .to_string(),
                ],
                mcp_spec: None,
                harness_spec: Some(AgentHarnessSpec {
                    allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
                    max_steps: 5,
                    max_budget_tokens: 16_000,
                }),
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 6: SpeechToCode (3 organic voice-to-Vox compilation tasks) ──
        OrganicSearchSpokeCase {
            case_id: "organic_speech_01_voice_actor_counter".to_string(),
            spoke: OrganicSpokeKind::SpeechToCode,
            search_query: "actor Counter on increment return n counter_actor.vox".to_string(),
            search_scope_paths: vec![
                "examples/golden/counter_actor.vox".to_string(),
                "examples/golden/checkout_workflow.vox".to_string(),
            ],
            expected_top_path_hint: "examples/golden/counter_actor.vox".to_string(),
            expected_grounding_tokens: vec![
                "actor".to_string(),
                "Counter".to_string(),
                "increment".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_speech_01_voice_actor_counter".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "um transcribe voice command to vox actor Counter with on increment(n: int) to int returning n + 1".to_string(),
                candidates: vec![
                    "```vox\nactor Counter {\n    on increment(n: int) to int {\n        return n + 1\n    }\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_speech_02_voice_workflow_charge".to_string(),
            spoke: OrganicSpokeKind::SpeechToCode,
            search_query: "activity charge workflow checkout amount int checkout_workflow.vox".to_string(),
            search_scope_paths: vec![
                "examples/golden/checkout_workflow.vox".to_string(),
                "crates/vox-compiler/src/pipeline.rs".to_string(),
            ],
            expected_top_path_hint: "examples/golden/checkout_workflow.vox".to_string(),
            expected_grounding_tokens: vec![
                "activity".to_string(),
                "charge".to_string(),
                "workflow".to_string(),
                "checkout".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_speech_02_voice_workflow_charge".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "uh dictated voice snippet for activity charge(amount: int) to int and workflow checkout(amount: int) to int".to_string(),
                candidates: vec![
                    "```vox\nactivity charge(amount: int) to int {\n    return amount\n}\n\nworkflow checkout(amount: int) to int {\n    return charge(amount)\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_speech_03_voice_spoke_bonus".to_string(),
            spoke: OrganicSpokeKind::SpeechToCode,
            search_query: "OrganicSpokeKind SpeechToCode PopuliMesh SkillCreation to_core_spoke contract.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/contract.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/contract.rs".to_string(),
            expected_grounding_tokens: vec![
                "OrganicSpokeKind".to_string(),
                "SpeechToCode".to_string(),
                "to_core_spoke".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_speech_03_voice_spoke_bonus".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "er voice command define fn spoke_bonus(passes: int, weight: int) to int".to_string(),
                candidates: vec![
                    "```vox\nfn spoke_bonus(passes: int, weight: int) to int {\n    return passes * weight\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 7: PopuliMesh (3 organic A2A mesh dispatch & quorum tasks) ──
        OrganicSearchSpokeCase {
            case_id: "organic_populi_01_mesh_dispatch_envelope".to_string(),
            spoke: OrganicSpokeKind::PopuliMesh,
            search_query: "A2ADeliverRequest sender_agent_id payload_blake3_hex worker_ed25519_sig_b64 a2a.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-mesh-types/src/a2a.rs".to_string(),
                "crates/vox-mesh-types/src/quorum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-mesh-types/src/a2a.rs".to_string(),
            expected_grounding_tokens: vec![
                "A2ADeliverRequest".to_string(),
                "sender_agent_id".to_string(),
                "payload_blake3_hex".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_populi_01_mesh_dispatch_envelope".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a PopuliMeshDispatchEnvelope JSON for dispatching an a2a_task_envelope to populi-node-alpha with gpu_cuda and quorum_size 3.".to_string(),
                candidates: vec![
                    r#"{"target_node_id":"populi-node-alpha","capability_tier":"gpu_cuda","vram_min_gb":24,"iroh_endpoint":"iroh://node-alpha.populi.vox:4433","quorum_size":3,"payload_kind":"a2a_task_envelope","timeout_ms":5000}"#.to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_populi_02_tensor_shard_sync".to_string(),
            spoke: OrganicSpokeKind::PopuliMesh,
            search_query: "QuorumPolicy min_agreeing_workers ComparisonMode SemanticSample quorum.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-mesh-types/src/quorum.rs".to_string(),
                "crates/vox-mesh-types/src/a2a.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-mesh-types/src/quorum.rs".to_string(),
            expected_grounding_tokens: vec![
                "QuorumPolicy".to_string(),
                "min_agreeing_workers".to_string(),
                "ComparisonMode".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_populi_02_tensor_shard_sync".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a PopuliMeshDispatchEnvelope JSON for synchronizing tensor shards across a 5-node Populi metal_unified mesh.".to_string(),
                candidates: vec![
                    r#"{"target_node_id":"populi-mesh-mac-studio-01","capability_tier":"metal_unified","vram_min_gb":64,"iroh_endpoint":"iroh://mac-studio-01.populi.vox:4433","quorum_size":5,"payload_kind":"tensor_shard_sync","timeout_ms":12000}"#.to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_populi_03_gradient_all_reduce".to_string(),
            spoke: OrganicSpokeKind::PopuliMesh,
            search_query: "PopuliMesh OrganicSpokeKind populi_mesh TrainingRecordV2 contract.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/contract.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/contract.rs".to_string(),
            expected_grounding_tokens: vec![
                "PopuliMesh".to_string(),
                "populi_mesh".to_string(),
                "TrainingRecordV2".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_populi_03_gradient_all_reduce".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a PopuliMeshDispatchEnvelope JSON for gradient_all_reduce on gpu_rocm cluster.".to_string(),
                candidates: vec![
                    r#"{"target_node_id":"populi-rocm-worker-07","capability_tier":"gpu_rocm","vram_min_gb":48,"iroh_endpoint":"iroh://rocm-worker-07.populi.vox:4433","quorum_size":3,"payload_kind":"gradient_all_reduce","timeout_ms":15000}"#.to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 8: SkillCreation (3 organic SKILL.md authoring & compilation tasks) ──
        OrganicSearchSpokeCase {
            case_id: "organic_skill_01_compiler_audit_skill".to_string(),
            spoke: OrganicSpokeKind::SkillCreation,
            search_query: "parse_skill_md VoxSkillBundle ParseSkillError vox-category vox-tools skill_parser.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-plugin-host/src/skill_parser.rs".to_string(),
                "crates/vox-skills/src/lib.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-plugin-host/src/skill_parser.rs".to_string(),
            expected_grounding_tokens: vec![
                "parse_skill_md".to_string(),
                "VoxSkillBundle".to_string(),
                "vox-category".to_string(),
                "vox-tools".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_skill_01_compiler_audit_skill".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Author an agentskills.io-compliant SKILL.md for `vox-compiler-audit` with YAML frontmatter, metadata.vox-category, metadata.vox-tools, and executable Vox verification code.".to_string(),
                candidates: vec![
                    "---\nname: vox-compiler-audit\ndescription: Audits Vox source modules through the compiler pipeline and verifies zero typeck errors before corpus admission.\nmetadata:\n  vox-category: verification\n  vox-tools: vox_compiler_check\n---\n# Vox Compiler Audit Skill\n\nRun the compiler pipeline on candidate `.vox` snippets and verify zero diagnostics before admitting records to the Mens training corpus.\n\n```vox\nfn check_admission_budget(errors: int, limit: int) to int {\n    if errors > limit {\n        return 0\n    }\n    return 1\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_skill_02_populi_quorum_router_skill".to_string(),
            spoke: OrganicSpokeKind::SkillCreation,
            search_query: "SkillCreation skill_creation OrganicSpokeKind ALL contract.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/contract.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/contract.rs".to_string(),
            expected_grounding_tokens: vec![
                "SkillCreation".to_string(),
                "skill_creation".to_string(),
                "OrganicSpokeKind".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_skill_02_populi_quorum_router_skill".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Author an agentskills.io-compliant SKILL.md for `populi-quorum-router` with YAML frontmatter and a valid Vox quorum helper block.".to_string(),
                candidates: vec![
                    "---\nname: populi-quorum-router\ndescription: Routes distributed A2A tasks across Populi mesh nodes with quorum verification and Iroh endpoint validation.\nmetadata:\n  vox-category: orchestration\n  vox-tools: vox_search_query\n---\n# Populi Quorum Router Skill\n\nValidate node VRAM capacity and quorum size before dispatching mesh workloads over Iroh transport.\n\n```vox\nfn has_quorum(online_nodes: int, required_quorum: int) to int {\n    if online_nodes < required_quorum {\n        return 0\n    }\n    return online_nodes\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_skill_03_rag_distractor_guard_skill".to_string(),
            spoke: OrganicSpokeKind::SkillCreation,
            search_query: "SKILLS_REGISTRY_BASE SkillError InvalidManifest PermissionDenied vox-skills lib.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-skills/src/lib.rs".to_string(),
                "crates/vox-plugin-host/src/skill_parser.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-skills/src/lib.rs".to_string(),
            expected_grounding_tokens: vec![
                "SKILLS_REGISTRY_BASE".to_string(),
                "SkillError".to_string(),
                "InvalidManifest".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_skill_03_rag_distractor_guard_skill".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Author an agentskills.io-compliant SKILL.md for `rag-distractor-guard` with YAML frontmatter and a compilable Vox grounding score helper.".to_string(),
                candidates: vec![
                    "---\nname: rag-distractor-guard\ndescription: Verifies retrieval-augmented synthesis grounding against workspace evidence and rejects Rank-1 distractor poisoning.\nmetadata:\n  vox-category: evaluation\n  vox-tools: vox_search_query\n---\n# RAG Distractor Guard Skill\n\nInspect retrieved workspace chunks, strip filename bias hints, and reject synthesis candidates containing distractor sentinel markers.\n\n```vox\nfn grounding_percentage(matched: int, required: int) to int {\n    if required <= 0 {\n        return 100\n    }\n    return (matched * 100) / required\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
    ]
}

/// Canonical 24-problem (3 per spoke across all 8 spokes) organic search-to-compilation benchmark suite ().
#[must_use]
pub fn canonical_organic_search_to_compilation_suite_24() -> Vec<OrganicSearchSpokeCase> {
    let mut cases = organic_suite_24_part1();
    cases.extend(organic_suite_24_part2());
    cases
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn suite_24_is_part1_plus_part2_with_unique_ids_and_matching_roundtrip_ids() {
        let cases = canonical_organic_search_to_compilation_suite_24();
        assert_eq!(
            cases.len(),
            organic_suite_24_part1().len() + organic_suite_24_part2().len()
        );
        let ids: HashSet<&str> = cases.iter().map(|c| c.case_id.as_str()).collect();
        assert_eq!(ids.len(), cases.len());
        assert!(cases.iter().all(|c| c.case_id == c.roundtrip_case.case_id));
    }

    #[test]
    fn suite_24_has_three_cases_per_spoke() {
        use crate::training::contract::OrganicSpokeKind;
        let cases = canonical_organic_search_to_compilation_suite_24();
        assert_eq!(cases.len(), 3 * OrganicSpokeKind::ALL.len());
        for spoke in OrganicSpokeKind::ALL {
            assert_eq!(cases.iter().filter(|c| c.spoke == spoke).count(), 3);
        }
    }
}
