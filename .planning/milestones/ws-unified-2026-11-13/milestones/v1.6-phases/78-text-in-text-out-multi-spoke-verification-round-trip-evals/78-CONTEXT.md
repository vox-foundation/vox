# Phase 78: Text-In / Text-Out Multi-Spoke Verification & Round-Trip Evals — Context

**Gathered:** 2026-10-16
**Status:** Ready for planning
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Deliver a deterministic, in-process 5-spoke text-in / text-out round-trip verification harness (`SPOKE-01..06`) in `crates/vox-corpus` (`training::spoke_roundtrip`) that evaluates model completions (`prompt -> text response -> oracle verification -> OracleReceipt`) across all 5 training/evaluation spokes and emits a machine-readable `MensSpokeScorecardV1` (`contracts/reports/mens-spoke-scorecard.v1.json` + Markdown summary).

</domain>

<decisions>
## Implementation Decisions

### Crate Placement (`crates/vox-corpus`)
- **D-01 (`SPOKE-01..06`)**: `crates/vox-corpus` already depends on `vox-compiler`, `syn`, `serde_json`, `regex`, and `xxhash-rust`, and defines `SpokeKind`, `OracleReceipt`, and `TrainingRecordV2`. Placing the 5-spoke round-trip harness in `crates/vox-corpus/src/training/spoke_roundtrip.rs` avoids adding any new crate dependencies or modifying `Cargo.toml` / `layers.toml`.

### Five-Spoke Round-Trip Oracles (`SPOKE-01..05`)
- **D-02 (`SPOKE-01` — VoxLang Spoke Round-Trip)**: Strip optional markdown code fences, run `vox_compiler::pipeline::run_frontend_str` (`lex -> parse -> HIR -> typeck`), and run `vox_compiler::eval::Interpreter` to load the module and execute zero-arg `main()` when present, recording deterministic output and AST construct coverage in `SpokeRoundtripReceipt`.
- **D-03 (`SPOKE-02` — Rust Workspace Spoke Round-Trip)**: Strip optional markdown code fences, parse via `syn::parse_file`, enforce stub-free quality (`todo!`/`unimplemented!`), and enforce workspace source-policy detectors (banned crypto crate imports `blake3`/`sha2`/`sha3`/`md5`/`sha1`, hardcoded LLM vendor hostnames `api.anthropic.com`/`api.openai.com`/`generativelanguage.googleapis.com`, and direct `std::env::var` secret reads).
- **D-04 (`SPOKE-03` — Structured Shell `pwsh` Spoke Round-Trip)**: Verify structured PowerShell (`pwsh`) pipelines against `classify_shell_safety_floor`, check pipeline structure (`Get-ChildItem`, `Select-Object`, `Where-Object`, `ConvertTo-Json`, etc.), and validate any expected JSON output schema sample when provided.
- **D-05 (`SPOKE-04` — MCP Tool-Calling Spoke Round-Trip)**: Parse single or multi-step MCP tool call trajectories (JSON object or array of `{ "tool" / "tool_name", "arguments" / "args" }`), validate against declared `McpToolSpec` schemas (required keys + known tool names), simulate mock/sandbox execution, and verify goal completion.
- **D-06 (`SPOKE-05` — Agent Harness `vox-dei` Spoke Round-Trip)**: Parse `TaskPlan` JSON (`goal`, `steps`, `files`, `budget_tokens` / `max_steps`, `verification_gate`), validate file-scope adherence against allowed paths, enforce step/token budget bounds, and verify presence of a concrete verification command/gate.

### Unified Multi-Spoke Scorecard (`SPOKE-06`)
- **D-07 (`SPOKE-06`)**: Implement `MensSpokeScorecardV1` (`schema_version = "mens_spoke_scorecard_v1"`) computing `pass_at_1`, `pass_at_k`, `syntax_validity_rate`, `semantic_validity_rate`, and `execution_success_rate` per spoke and overall, with JSON serialization matching `contracts/reports/mens-spoke-scorecard.v1.json` and a human-readable Markdown table renderer.

</decisions>
