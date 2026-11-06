use super::*;

/// Canonical 24-problem (3 per spoke across all 8 spokes) organic search-to-compilation benchmark suite (`ORG-SPOKE-01..06`, `SPOKE8-01..04`).
#[must_use]
pub fn canonical_organic_search_to_compilation_suite_15() -> Vec<OrganicSearchSpokeCase> {
    canonical_organic_search_to_compilation_suite_24()
}

pub(super) fn organic_suite_24_part1() -> Vec<OrganicSearchSpokeCase> {
    use crate::training::contract::OrganicSpokeKind;
    vec![
        // ── Spoke 1: VoxLang (3 organic search-to-compilation tasks) ──────────
        OrganicSearchSpokeCase {
            case_id: "organic_vox_01_actor_counter".to_string(),
            spoke: OrganicSpokeKind::VoxLang,
            search_query: "actor Counter on increment return n counter_actor.vox pipeline.rs"
                .to_string(),
            search_scope_paths: vec![
                "crates/vox-compiler/src/pipeline.rs".to_string(),
                "examples/golden".to_string(),
            ],
            expected_top_path_hint: "examples/golden/counter_actor.vox".to_string(),
            expected_grounding_tokens: vec![
                "actor".to_string(),
                "Counter".to_string(),
                "increment".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_vox_01_actor_counter".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "Grounded in `examples/golden/counter_actor.vox` and `crates/vox-compiler/src/pipeline.rs` ADR-041 actor syntax, define `actor Counter` with `on increment(n: int) to int` and helper `fn next_count(v: int) to int`.".to_string(),
                candidates: vec![
                    "```vox\nactor Counter {\n    on increment(n: int) to int {\n        return n + 1\n    }\n}\n\nfn next_count(v: int) to int {\n    return v + 1\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_vox_02_workflow_checkout".to_string(),
            spoke: OrganicSpokeKind::VoxLang,
            search_query:
                "activity charge workflow checkout amount int checkout_workflow.vox pipeline.rs"
                    .to_string(),
            search_scope_paths: vec![
                "crates/vox-compiler/src/pipeline.rs".to_string(),
                "examples/golden".to_string(),
            ],
            expected_top_path_hint: "examples/golden/checkout_workflow.vox".to_string(),
            expected_grounding_tokens: vec![
                "activity".to_string(),
                "charge".to_string(),
                "workflow".to_string(),
                "checkout".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_vox_02_workflow_checkout".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "Grounded in ADR-041 workflow+activity syntax, implement `activity charge(amount: int) to int` and `workflow checkout(amount: int, fee: int) to int`.".to_string(),
                candidates: vec![
                    "```vox\nactivity charge(amount: int) to int {\n    return amount\n}\n\nworkflow checkout(amount: int, fee: int) to int {\n    return charge(amount + fee)\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_vox_03_record_metric_delta".to_string(),
            spoke: OrganicSpokeKind::VoxLang,
            search_query: "ChosenRejectedPairV2 SpokeCurriculumWeights TrainingBatchManifestV1 curriculum.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
                "crates/vox-corpus/src/training/contract.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            expected_grounding_tokens: vec![
                "ChosenRejectedPairV2".to_string(),
                "SpokeCurriculumWeights".to_string(),
                "TrainingBatchManifestV1".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_vox_03_record_metric_delta".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "Define `type SpokeMetricRow` (`spoke: str`, `passes: int`, `total: int`) and `fn net_gain(before: int, after: int, weight: int) to int`.".to_string(),
                candidates: vec![
                    "```vox\ntype SpokeMetricRow {\n    spoke: str\n    passes: int\n    total: int\n}\n\nfn net_gain(before: int, after: int, weight: int) to int {\n    if after < before {\n        return 0\n    }\n    return (after - before) * weight\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 2: RustWorkspace (3 organic search-to-compilation tasks) ────
        OrganicSearchSpokeCase {
            case_id: "organic_rust_01_wilson_interval".to_string(),
            spoke: OrganicSpokeKind::RustWorkspace,
            search_query: "wilson_score_interval successes MIN_CALLS_FOR_CONFIDENT_RANK registry.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-orchestrator/src/models/registry.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-orchestrator/src/models/registry.rs".to_string(),
            expected_grounding_tokens: vec![
                "wilson_score_interval".to_string(),
                "MIN_CALLS_FOR_CONFIDENT_RANK".to_string(),
                "successes".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_rust_01_wilson_interval".to_string(),
                spoke: SpokeKind::RustWorkspace,
                prompt: "Grounded in `crates/vox-orchestrator/src/models/registry.rs`, implement `pub fn wilson_lower_bound_95(successes: u64, n: u64) -> Option<f64>`.".to_string(),
                candidates: vec![
                    "```rust\npub fn wilson_lower_bound_95(successes: u64, n: u64) -> Option<f64> {\n    if n == 0 {\n        return None;\n    }\n    let z = 1.96_f64;\n    let nf = n as f64;\n    let p = (successes as f64 / nf).clamp(0.0, 1.0);\n    let z2 = z * z;\n    let denom = 1.0 + z2 / nf;\n    let center = (p + z2 / (2.0 * nf)) / denom;\n    let margin = (z * ((p * (1.0 - p) / nf) + z2 / (4.0 * nf * nf)).sqrt()) / denom;\n    Some((center - margin).clamp(0.0, 1.0))\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_rust_02_curriculum_floor".to_string(),
            spoke: OrganicSpokeKind::RustWorkspace,
            search_query: "compute_curriculum_weights DEFAULT_MIN_SPOKE_WEIGHT_FLOOR SpokeCurriculumWeights curriculum.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
                "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            expected_grounding_tokens: vec![
                "compute_curriculum_weights".to_string(),
                "DEFAULT_MIN_SPOKE_WEIGHT_FLOOR".to_string(),
                "SpokeCurriculumWeights".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_rust_02_curriculum_floor".to_string(),
                spoke: SpokeKind::RustWorkspace,
                prompt: "Grounded in `crates/vox-corpus/src/training/curriculum.rs`, implement `pub fn compute_spoke_share(pass_at_1: f64, floor: f64, remaining_budget: f64, total_deficit: f64) -> f64`.".to_string(),
                candidates: vec![
                    "```rust\npub fn compute_spoke_share(\n    pass_at_1: f64,\n    floor: f64,\n    remaining_budget: f64,\n    total_deficit: f64,\n) -> f64 {\n    let clamped_floor = floor.clamp(0.05, 0.20);\n    if total_deficit <= 0.0 {\n        return 0.20;\n    }\n    let deficit = (1.0 - pass_at_1.clamp(0.0, 1.0)) + 0.05;\n    clamped_floor + remaining_budget.max(0.0) * (deficit / total_deficit)\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_rust_03_safetensors_header_check".to_string(),
            spoke: OrganicSpokeKind::RustWorkspace,
            search_query: "verify_adapter_file_blake3 lora_a.weight lora_b.weight adapter_registry.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
            expected_grounding_tokens: vec![
                "verify_adapter_file_blake3".to_string(),
                "lora_a.weight".to_string(),
                "lora_b.weight".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_rust_03_safetensors_header_check".to_string(),
                spoke: SpokeKind::RustWorkspace,
                prompt: "Grounded in `crates/vox-corpus/src/training/adapter_registry.rs`, implement `pub fn parse_safetensors_header_bounds(bytes: &[u8]) -> Result<(usize, usize), String>` with checked arithmetic.".to_string(),
                candidates: vec![
                    "```rust\npub fn parse_safetensors_header_bounds(bytes: &[u8]) -> Result<(usize, usize), String> {\n    if bytes.len() < 8 {\n        return Err(\"safetensors buffer shorter than 8-byte header prefix\".to_string());\n    }\n    let mut prefix = [0u8; 8];\n    prefix.copy_from_slice(&bytes[..8]);\n    let header_len = usize::try_from(u64::from_le_bytes(prefix))\n        .map_err(|_| \"header length overflows usize\".to_string())?;\n    let header_end = 8usize\n        .checked_add(header_len)\n        .ok_or_else(|| \"header end overflows usize\".to_string())?;\n    if bytes.len() <= header_end {\n        return Err(\"safetensors tensor payload is empty or truncated\".to_string());\n    }\n    Ok((8, header_end))\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 3: StructuredShell (3 organic search-to-compilation tasks) ──
        OrganicSearchSpokeCase {
            case_id: "organic_shell_01_golden_vox_inventory".to_string(),
            spoke: OrganicSpokeKind::StructuredShell,
            search_query: "allowed_cmdlets Get-ChildItem Select-Object ConvertTo-Json exec-policy.v1.yaml".to_string(),
            search_scope_paths: vec![
                "contracts/terminal/exec-policy.v1.yaml".to_string(),
                "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
            ],
            expected_top_path_hint: "contracts/terminal/exec-policy.v1.yaml".to_string(),
            expected_grounding_tokens: vec![
                "allowed_cmdlets".to_string(),
                "Get-ChildItem".to_string(),
                "ConvertTo-Json".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_shell_01_golden_vox_inventory".to_string(),
                spoke: SpokeKind::StructuredShell,
                prompt: "Emit a single structured PowerShell pipeline listing `.vox` files in `examples/golden` with `Name` and `Length` serialized to JSON.".to_string(),
                candidates: vec![
                    "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_shell_02_contract_reports_json".to_string(),
            spoke: OrganicSpokeKind::StructuredShell,
            search_query: "mens_before_after_comparison_v1 promoted_adapter_uri mens-before-after-comparison.v1.json".to_string(),
            search_scope_paths: vec![
                "contracts/reports/mens-before-after-comparison.v1.json".to_string(),
                "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
            ],
            expected_top_path_hint: "contracts/reports/mens-before-after-comparison.v1.json".to_string(),
            expected_grounding_tokens: vec![
                "mens_before_after_comparison_v1".to_string(),
                "promoted_adapter_uri".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_shell_02_contract_reports_json".to_string(),
                spoke: SpokeKind::StructuredShell,
                prompt: "Emit a structured PowerShell pipeline inspecting `.json` reports in `contracts/reports` with `Name` and `LastWriteTime` as JSON.".to_string(),
                candidates: vec![
                    "Get-ChildItem -Path contracts/reports -Filter *.json | Select-Object Name, LastWriteTime | ConvertTo-Json".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: vec!["Name".to_string(), "LastWriteTime".to_string()],
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_shell_03_nushell_eval_ratchet".to_string(),
            spoke: OrganicSpokeKind::StructuredShell,
            search_query: "vox.spoke_regression_ratchet.v1 spoke_pass_at_1_floors spoke-regression-ratchet.v1.json".to_string(),
            search_scope_paths: vec![
                "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
                "contracts/reports/mens-before-after-comparison.v1.json".to_string(),
            ],
            expected_top_path_hint: "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
            expected_grounding_tokens: vec![
                "vox.spoke_regression_ratchet.v1".to_string(),
                "spoke_pass_at_1_floors".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_shell_03_nushell_eval_ratchet".to_string(),
                spoke: SpokeKind::StructuredShell,
                prompt: "Query `contracts/eval` in Nushell selecting `name` and `size` and converting `to json`.".to_string(),
                candidates: vec![
                    "ls contracts/eval | select name size | to json".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: vec!["name".to_string(), "size".to_string()],
            },
        },
        // ── Spoke 4: McpToolChain (3 organic search-to-compilation tasks) ─────
        OrganicSearchSpokeCase {
            case_id: "organic_mcp_01_compiler_check_pipeline".to_string(),
            spoke: OrganicSpokeKind::McpToolChain,
            search_query: "vox_compiler_check check_file VoxCompilerDiagnosticPayload pipeline.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-compiler/src/pipeline.rs".to_string(),
                "crates/vox-corpus/src/flywheel.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-compiler/src/pipeline.rs".to_string(),
            expected_grounding_tokens: vec![
                "check_file".to_string(),
                "VoxCompilerDiagnosticPayload".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_mcp_01_compiler_check_pipeline".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Invoke `vox_compiler_check` with a valid ADR-041 `actor Ping` `.vox` source snippet.".to_string(),
                candidates: vec![
                    r#"{"tool_name":"vox_compiler_check","arguments":{"path":"examples/golden/ref_actors.vox","source":"actor Ping { on ping(x: int) to int { return x + 1 } }"}}"#.to_string(),
                ],
                mcp_spec: Some(McpToolSpec {
                    allowed_tools: vec!["vox_compiler_check".to_string()],
                    required_arg_keys: vec!["path".to_string(), "source".to_string()],
                }),
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_mcp_02_search_query_architecture".to_string(),
            spoke: OrganicSpokeKind::McpToolChain,
            search_query: "vox_kb_search requires must close over db registry_entry profile.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-mcp-registry/src/profile.rs".to_string(),
                "crates/vox-corpus/src/flywheel.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-mcp-registry/src/profile.rs".to_string(),
            expected_grounding_tokens: vec![
                "vox_kb_search".to_string(),
                "registry_entry".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_mcp_02_search_query_architecture".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Call `vox_search_query` scoped to `docs/src/architecture` to retrieve Mens 5-spoke flywheel documentation.".to_string(),
                candidates: vec![
                    r#"{"tool_name":"vox_search_query","arguments":{"query":"mens 5-spoke continuous training flywheel","scope":"docs/src/architecture"}}"#.to_string(),
                ],
                mcp_spec: Some(McpToolSpec {
                    allowed_tools: vec!["vox_search_query".to_string()],
                    required_arg_keys: vec!["query".to_string(), "scope".to_string()],
                }),
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_mcp_03_validate_mermaid_graph".to_string(),
            spoke: OrganicSpokeKind::McpToolChain,
            search_query: "tool_name_exists TOOL_REGISTRY SKILL_TOOLS ORCHESTRATOR_TOOLS eval_agentic_metrics.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/corpus/eval_agentic_metrics.rs".to_string(),
                "crates/vox-corpus/src/flywheel.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/corpus/eval_agentic_metrics.rs".to_string(),
            expected_grounding_tokens: vec![
                "tool_name_exists".to_string(),
                "TOOL_REGISTRY".to_string(),
                "SKILL_TOOLS".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_mcp_03_validate_mermaid_graph".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Call `vox_validate_mermaid` to validate the organic search-to-compilation flowchart.".to_string(),
                candidates: vec![
                    r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n  Search[\"Organic Search\"] --> Compile[\"Vox Compiler\"]\n  Compile --> Verify[\"Oracle Receipt\"]"}}"#.to_string(),
                ],
                mcp_spec: Some(McpToolSpec {
                    allowed_tools: vec!["vox_validate_mermaid".to_string()],
                    required_arg_keys: vec!["diagram".to_string()],
                }),
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_15_alias_returns_the_same_cases_as_suite_24() {
        let a = canonical_organic_search_to_compilation_suite_15();
        let b = canonical_organic_search_to_compilation_suite_24();
        assert_eq!(a, b);
    }

    #[test]
    fn part1_cases_are_well_formed() {
        let cases = organic_suite_24_part1();
        assert!(!cases.is_empty());
        for c in &cases {
            assert_eq!(c.case_id, c.roundtrip_case.case_id);
            assert!(!c.search_query.is_empty());
            assert!(!c.expected_grounding_tokens.is_empty());
            assert!(!c.roundtrip_case.candidates.is_empty());
        }
    }
}
