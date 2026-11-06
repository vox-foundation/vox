use super::*;

/// Canonical 5-spoke golden round-trip benchmark suite (`SPOKE-01..06`).
#[must_use]
pub fn canonical_five_spoke_golden_suite() -> Vec<SpokeRoundtripCase> {
    vec![
        SpokeRoundtripCase {
            case_id: "spoke_vox_01_fn_add".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `add(a: int, b: int) to int` that returns `a + b`.".to_string(),
            candidates: vec![
                "```vox\nfn add(a: int, b: int) to int {\n    return a + b\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "spoke_rust_01_clamp".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement a pure Rust helper `pub fn clamp_score(v: f64) -> f64` in `[0.0, 1.0]`.".to_string(),
            candidates: vec![
                "```rust\npub fn clamp_score(v: f64) -> f64 {\n    v.clamp(0.0, 1.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "spoke_shell_01_pwsh_json".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List `.vox` files in `examples/golden` as JSON with `Name` and `Length` via PowerShell.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "spoke_mcp_01_validate_mermaid".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call the MCP tool `vox_validate_mermaid` to validate a flowchart diagram.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n  A[\"Start\"] --> B[\"End\"]"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_validate_mermaid".to_string()],
                required_arg_keys: vec!["diagram".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "spoke_harness_01_task_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to add a unit test in `crates/vox-corpus`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Add unit test for multi-spoke round-trip verification",
                    "steps": [
                        "Add test case in crates/vox-corpus/src/training/spoke_roundtrip.rs",
                        "Run cargo test -p vox-corpus --lib spoke_roundtrip"
                    ],
                    "files": ["crates/vox-corpus/src/training/spoke_roundtrip.rs"],
                    "budget_tokens": 4000,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-corpus --lib spoke_roundtrip"
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
    ]
}

/// Expanded 25-problem (5 problems per spoke) multi-spoke benchmark suite (`MENS-PERF-01`).
///
/// Covers all 5 canonical Mens spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`,
/// `McpToolChain`, `AgentHarness`) with realistic codegen, type/schema, safety, and
/// verification-gate tasks.
#[must_use]
pub fn canonical_five_spoke_benchmark_suite_25() -> Vec<SpokeRoundtripCase> {
    vec![
        // ── Spoke 1: VoxLang (5 problems) ─────────────────────────────────────
        SpokeRoundtripCase {
            case_id: "vox_01_fn_add".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `add(a: int, b: int) to int` that returns `a + b`.".to_string(),
            candidates: vec![
                "```vox\nfn add(a: int, b: int) to int {\n    return a + b\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_02_fn_clamp_int".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `clamp_low(x: int, floor: int) to int` that returns `floor` when `x < floor`, else `x`.".to_string(),
            candidates: vec![
                "```vox\nfn clamp_low(x: int, floor: int) to int {\n    if x < floor {\n        return floor\n    }\n    return x\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_03_type_metric_row".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Define a Vox record `type MetricRow` with `spoke: str` and `passes: int`, plus `fn total_passes(a: int, b: int) to int`.".to_string(),
            candidates: vec![
                "```vox\ntype MetricRow {\n    spoke: str\n    passes: int\n}\n\nfn total_passes(a: int, b: int) to int {\n    return a + b\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_04_fn_weighted_delta".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `weighted_delta(before: int, after: int, weight: int) to int` returning `(after - before) * weight`.".to_string(),
            candidates: vec![
                "```vox\nfn weighted_delta(before: int, after: int, weight: int) to int {\n    return (after - before) * weight\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_05_fn_is_promoted".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `is_promoted(pass_pct: int, floor_pct: int) to bool` returning `pass_pct >= floor_pct`.".to_string(),
            candidates: vec![
                "```vox\nfn is_promoted(pass_pct: int, floor_pct: int) to bool {\n    return pass_pct >= floor_pct\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        // ── Spoke 2: RustWorkspace (5 problems) ───────────────────────────────
        SpokeRoundtripCase {
            case_id: "rust_01_clamp_score".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement a pure Rust helper `pub fn clamp_score(v: f64) -> f64` in `[0.0, 1.0]`.".to_string(),
            candidates: vec![
                "```rust\npub fn clamp_score(v: f64) -> f64 {\n    v.clamp(0.0, 1.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_02_inverse_pass_weight".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn raw_curriculum_weight(pass_at_1: f64, eps: f64) -> f64` returning `1.0 - pass_at_1.clamp(0.0, 1.0) + eps.max(0.0)`.".to_string(),
            candidates: vec![
                "```rust\npub fn raw_curriculum_weight(pass_at_1: f64, eps: f64) -> f64 {\n    1.0 - pass_at_1.clamp(0.0, 1.0) + eps.max(0.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_03_parse_spoke_floor".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn parse_spoke_floor(input: &str) -> Result<f64, String>` without `.unwrap()` or `.expect()`.".to_string(),
            candidates: vec![
                "```rust\npub fn parse_spoke_floor(input: &str) -> Result<f64, String> {\n    let v: f64 = input.trim().parse().map_err(|e| format!(\"invalid floor: {e}\"))?;\n    Ok(v.clamp(0.0, 1.0))\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_04_ratchet_non_decreasing".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn meets_ratchet(observed: f64, floor: f64) -> bool` with `1e-9` tolerance.".to_string(),
            candidates: vec![
                "```rust\npub fn meets_ratchet(observed: f64, floor: f64) -> bool {\n    observed + 1e-9 >= floor\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_05_wilson_lower_bound".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn empirical_rate(successes: u32, total: u32) -> f64` safely handling `total == 0`.".to_string(),
            candidates: vec![
                "```rust\npub fn empirical_rate(successes: u32, total: u32) -> f64 {\n    if total == 0 {\n        return 0.0;\n    }\n    (f64::from(successes) / f64::from(total)).clamp(0.0, 1.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        // ── Spoke 3: StructuredShell (5 problems) ─────────────────────────────
        SpokeRoundtripCase {
            case_id: "shell_01_pwsh_golden_vox".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List `.vox` files in `examples/golden` as JSON with `Name` and `Length` via PowerShell.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_02_pwsh_contracts_reports".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "Inspect `.json` contract reports under `contracts/reports` with `Name` and `LastWriteTime` as JSON.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path contracts/reports -Filter *.json | Select-Object Name, LastWriteTime | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "LastWriteTime".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_03_pwsh_crates_dirs".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List workspace crate directories under `crates` selecting `Name` and `FullName` as JSON.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path crates -Directory | Select-Object Name, FullName | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "FullName".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_04_nushell_structured_ls".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "Query `contracts/eval` files in Nushell selecting `name` and `size` and serializing `to json`.".to_string(),
            candidates: vec![
                "ls contracts/eval | select name size | to json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["name".to_string(), "size".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_05_pwsh_mens_runs".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List checkpoint artifacts in `mens/runs` selecting `Name` and `Length` as JSON.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path mens/runs -Recurse | Select-Object Name, Length | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
        },
        // ── Spoke 4: McpToolChain (5 problems) ────────────────────────────────
        SpokeRoundtripCase {
            case_id: "mcp_01_validate_mermaid".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call the MCP tool `vox_validate_mermaid` to validate a flowchart diagram.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n  A[\"Start\"] --> B[\"End\"]"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_validate_mermaid".to_string()],
                required_arg_keys: vec!["diagram".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_02_skill_search".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Invoke `vox_skill_search` with `query` and `limit` to locate Mens evaluation skills.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_skill_search","arguments":{"query":"mens spoke evaluation","limit":5}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_skill_search".to_string(), "vox_skill_list".to_string()],
                required_arg_keys: vec!["query".to_string(), "limit".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_03_search_query".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call `vox_search_query` with `query` and `scope` to search architecture SSOT docs.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_search_query","arguments":{"query":"mens continuous training flywheel","scope":"docs/src/architecture"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_search_query".to_string()],
                required_arg_keys: vec!["query".to_string(), "scope".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_04_compiler_check".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call `vox_compiler_check` with `path` and `source` to typecheck a Vox snippet.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_compiler_check","arguments":{"path":"examples/golden/hello.vox","source":"fn main() to int { return 0 }"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_compiler_check".to_string()],
                required_arg_keys: vec!["path".to_string(), "source".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_05_mens_scorecard_status".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call `vox_mens_scorecard` with `run_dir` and `include_deltas` to inspect the 5-spoke benchmark.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_mens_scorecard","arguments":{"run_dir":"workspace","include_deltas":true}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_mens_scorecard".to_string()],
                required_arg_keys: vec!["run_dir".to_string(), "include_deltas".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        // ── Spoke 5: AgentHarness (5 problems) ────────────────────────────────
        SpokeRoundtripCase {
            case_id: "harness_01_corpus_test_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to add a unit test in `crates/vox-corpus`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Add unit test for multi-spoke round-trip verification",
                    "steps": [
                        "Add test case in crates/vox-corpus/src/training/spoke_roundtrip.rs",
                        "Run cargo test -p vox-corpus --lib spoke_roundtrip"
                    ],
                    "files": ["crates/vox-corpus/src/training/spoke_roundtrip.rs"],
                    "budget_tokens": 4000,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-corpus --lib spoke_roundtrip"
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
        SpokeRoundtripCase {
            case_id: "harness_02_orchestrator_mens_routing".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to wire calibrated Mens adapter routing in `crates/vox-orchestrator`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Wire calibrated Mens adapter into ModelRegistry spoke routing",
                    "steps": [
                        "Add register_calibrated_mens_adapter and route_spoke_task in crates/vox-orchestrator/src/models/registry.rs",
                        "Update latency_score in crates/vox-orchestrator/src/models/scoring.rs",
                        "Verify with cargo test -p vox-orchestrator --lib"
                    ],
                    "files": [
                        "crates/vox-orchestrator/src/models/registry.rs",
                        "crates/vox-orchestrator/src/models/scoring.rs"
                    ],
                    "budget_tokens": 6000,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-orchestrator --lib"
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
        SpokeRoundtripCase {
            case_id: "harness_03_audit_spoke_ratchet".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to verify the per-spoke pass@1 regression ratchet in `crates/vox-audit`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Enforce spoke-regression-ratchet.v1.json in vox-audit regression_budget",
                    "steps": [
                        "Load contracts/eval/spoke-regression-ratchet.v1.json in crates/vox-audit/src/subcommands/regression_budget.rs",
                        "Add unit test verifying committed scorecard satisfies ratchet floors"
                    ],
                    "files": ["crates/vox-audit/src/subcommands/regression_budget.rs"],
                    "budget_tokens": 3500,
                    "max_steps": 3,
                    "verification_gate": "timeout 900s cargo test -p vox-audit"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-audit/".to_string()],
                max_steps: 4,
                max_budget_tokens: 12_000,
            }),
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "harness_04_cli_model_eval_five_spokes".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to add `--mens-five-spokes` to `vox model eval` in `crates/vox-cli`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Expose 5-spoke Mens evaluation and before/after comparison on vox model eval",
                    "steps": [
                        "Add --mens-five-spokes and --flywheel-burnin flags in crates/vox-cli/src/commands/model/eval.rs",
                        "Run cargo test -p vox-cli --lib"
                    ],
                    "files": ["crates/vox-cli/src/commands/model/eval.rs"],
                    "budget_tokens": 4500,
                    "max_steps": 3,
                    "verification_gate": "timeout 900s cargo test -p vox-cli --lib"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-cli/".to_string()],
                max_steps: 4,
                max_budget_tokens: 12_000,
            }),
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "harness_05_gui_mens_scorecard_panel".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to surface the 5-spoke scorecard and before/after deltas in `crates/vox-gui`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Render live 5-spoke scorecard and before/after deltas in Axis GUI MensTrainingView",
                    "steps": [
                        "Extend MensRunReportsDto in crates/vox-gui/src/commands/mens_run_reports.rs",
                        "Render mens-spoke-scorecard table in crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx",
                        "Verify with cargo test -p vox-gui and pnpm test MensTrainingView"
                    ],
                    "files": [
                        "crates/vox-gui/src/commands/mens_run_reports.rs",
                        "crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx"
                    ],
                    "budget_tokens": 5500,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-gui"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-gui/".to_string()],
                max_steps: 5,
                max_budget_tokens: 16_000,
            }),
            expected_json_keys: Vec::new(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const CANONICAL: [SpokeKind; 5] = [
        SpokeKind::VoxLang,
        SpokeKind::RustWorkspace,
        SpokeKind::StructuredShell,
        SpokeKind::McpToolChain,
        SpokeKind::AgentHarness,
    ];

    #[test]
    fn golden_suite_has_unique_ids_and_covers_all_five_spokes() {
        let cases = canonical_five_spoke_golden_suite();
        let ids: HashSet<&str> = cases.iter().map(|c| c.case_id.as_str()).collect();
        assert_eq!(ids.len(), cases.len());
        for spoke in CANONICAL {
            assert!(cases.iter().any(|c| c.spoke == spoke), "{spoke:?} missing");
        }
        assert!(cases.iter().all(|c| !c.candidates.is_empty()));
    }

    #[test]
    fn benchmark_suite_25_has_five_unique_problems_per_spoke() {
        let cases = canonical_five_spoke_benchmark_suite_25();
        assert_eq!(cases.len(), 25);
        let ids: HashSet<&str> = cases.iter().map(|c| c.case_id.as_str()).collect();
        assert_eq!(ids.len(), 25);
        for spoke in CANONICAL {
            assert_eq!(cases.iter().filter(|c| c.spoke == spoke).count(), 5);
        }
    }
}
