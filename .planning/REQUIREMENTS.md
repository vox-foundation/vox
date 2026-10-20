# Requirements: Vox Milestone `v2.0` — Persistent Organic Multi-Spoke Search-to-Compilation Benchmark, Live Execution Oracles & Longitudinal Evaluation Flywheel (`Phases 108–111`)

**Defined:** 2026-10-19
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`), while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.
**Architecture SSOT:** [`docs/src/architecture/organic-multi-spoke-search-to-compilation-benchmark-2026.md`](../docs/src/architecture/organic-multi-spoke-search-to-compilation-benchmark-2026.md)
**Contract SSOT:** [`contracts/reports/mens-organic-search-spoke-scorecard.v1.json`](../contracts/reports/mens-organic-search-spoke-scorecard.v1.json)

---

## v2.0 Requirements

### Phase 108: Organic Search-to-Spoke Retrieval & Contamination-Free Holdout Expansion (`ORG-SEARCH-01..04`)

- [x] **ORG-SEARCH-01 (15-Task 5-Spoke Organic Search-to-Compilation Benchmark Suite):** `canonical_organic_search_to_compilation_suite_15` in `crates/vox-corpus/src/training/spoke_roundtrip.rs` defines 15 organic developer search-to-compilation tasks (`3` per spoke across `VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), each pairing a real workspace search query (`search_query`, `search_scope_paths`, `expected_top_path_hint`, `expected_grounding_tokens`) with a full round-trip compilation task.
- [x] **ORG-SEARCH-02 (Workspace Evidence Retrieval & Grounding Scorer):** `discover_workspace_evidence` and `evaluate_organic_search_spoke_case` rank workspace files by path + content token overlap, compute `matched_rank`, `reciprocal_rank` (`MRR`), and `evidence_grounding_ratio`, and require `matched_rank.is_some()` and `evidence_grounding_ratio >= 0.80` (`100.0%` achieved across all 15 tasks).
- [x] **ORG-SEARCH-03 (Decontamination Guard Between Organic Benchmark & Training Snapshots):** `crates/vox-corpus/src/training/spoke_roundtrip.rs` provides `verify_organic_benchmark_decontamination(&[TrainingRecordV2])` ensuring zero XXH3-128 prompt/response collisions between held-out organic search-to-compilation tasks and SFT/DPO training snapshots.
- [x] **ORG-SEARCH-04 (Committed Organic Search-to-Compilation Contract Artifact):** `contracts/reports/mens-organic-search-spoke-scorecard.v1.json` records the deterministic 15-task organic search-to-compilation scorecard (`100.0%` Recall@5, `1.000` MRR, `100.0%` Grounding, `100.0%` Compilation, `100.0%` E2E `pass@1`) and is verified for byte-for-byte parity in `vox-corpus` unit tests.

### Phase 109: Deep Compilation & Functional Runtime Oracles Across All 5 Spokes (`COMP-ORACLE-01..04`)

- [x] **COMP-ORACLE-01 (`VoxLang` Full Compiler + Formatter + Interpreter Oracle):** `verify_deep_compilation_for_spoke` formats `.vox` code via `vox_compiler::format`, validates `ast_eval`, runs `vox_compiler::pipeline::run_frontend_str` (`lex -> parse -> HIR -> typeck` with zero errors), and executes `vox_compiler::eval::Interpreter::run_module(&res.hir)`.
- [x] **COMP-ORACLE-02 (`RustWorkspace` AST + Token Stream + Policy Oracle):** `verify_deep_compilation_for_spoke` parses Rust items via `syn::parse_file`, emits canonical token streams via `quote::ToTokens`, verifies non-empty item definitions, and enforces `vox-code-audit` detectors (`banned-crate-import`, `llm_provider_call`, `secret-env-guard`).
- [x] **COMP-ORACLE-03 (`StructuredShell` & `McpToolChain` Live Disk & Inline Compiler Oracles):** `verify_deep_compilation_for_spoke` verifies on-disk target directories (`examples/golden`, `contracts/reports`, `contracts/eval`) for `StructuredShell`, and compiles inline `.vox` payloads via `vox_compiler::pipeline::check_file` for `McpToolChain` (`vox_compiler_check`).
- [x] **COMP-ORACLE-04 (`AgentHarness` Live Target File & `ModelRegistry::route_spoke_task` Verification):** `verify_deep_compilation_for_spoke` verifies that every file in `TaskPlanEnvelope::files` exists on disk without path traversal and verifies `verification_gate` against `classify_shell_safety_floor`.

### Phase 110: Persistent Longitudinal Benchmark Time-Series in `VoxDb` & Multi-Metric Regression Ratchet (`PERSIST-BENCH-01..04`)

- [x] **PERSIST-BENCH-01 (`VoxDb` Content-Addressed Persistence for Organic Scorecards):** `persist_organic_scorecard_to_db` and `load_organic_scorecard_from_db` in `crates/vox-corpus/src/training/spoke_roundtrip.rs` store and rehydrate `OrganicSearchSpokeScorecardV1` in `VoxDb` (`mens_organic_search_scorecard` namespace) with `trusty://mens-organic-scorecard/<hash>` URIs.
- [x] **PERSIST-BENCH-02 (Longitudinal Benchmark History & Non-Decreasing Trend Verifier):** `list_organic_scorecard_history_from_db` in `crates/vox-corpus/src/training/spoke_roundtrip.rs` enumerates persisted organic benchmark runs in `VoxDb` and verifies non-decreasing `overall_end_to_end_pass_at_1` and `overall_mrr` across runs.
- [x] **PERSIST-BENCH-03 (Multi-Metric Regression Ratchet Contract Expansion):** `contracts/eval/spoke-regression-ratchet.v1.json` and `crates/vox-audit/src/subcommands/regression_budget.rs` enforce both per-spoke `pass@1` floors (`1.0`) and organic search-to-compilation floors (`search_recall_at_5 >= 1.0`, `overall_mrr >= 0.95`, `overall_compilation_pass_rate >= 1.0`).
- [x] **PERSIST-BENCH-04 (`ModelBenchCaseEvent` Telemetry Emission for Organic Benchmark Runs):** Evaluating the organic search-to-compilation suite emits strongly-typed `ModelBenchCaseEvent` telemetry records into `VoxDb` (`research_metrics`).

### Phase 111: CLI (`vox model eval --organic-search-spokes`), MCP & Axis GUI Longitudinal Scorecard Surface (`SURFACE-BENCH-01..04`)

- [x] **SURFACE-BENCH-01 (`vox model eval --organic-search-spokes` CLI Flag):** `crates/vox-cli/src/commands/model/eval.rs` supports `--organic-search-spokes`, executing both the 25-problem before/after suite and the 15-problem organic search-to-compilation suite and printing the unified Markdown report.
- [x] **SURFACE-BENCH-02 (`OrganicSearchSpokeScorecardDto` in `vox-gui` Backend):** `crates/vox-gui/src/commands/mens_run_reports.rs` exposes `OrganicSearchSpokeScorecardDto` loaded from `contracts/reports/mens-organic-search-spoke-scorecard.v1.json`.
- [x] **SURFACE-BENCH-03 (`MensTrainingView.tsx` Organic Search-to-Compilation Scorecard Panel):** `crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx` renders the organic search-to-compilation metrics (`Recall@5`, `MRR`, `Grounding`, `Compilation`, `E2E pass@1`) alongside the 25-problem before/after table.
- [x] **SURFACE-BENCH-04 (End-to-End Rust, Vitest & Doc-Pipeline Verification):** All `vox-corpus`, `vox-audit`, `vox-cli`, `vox-gui`, Vitest, and `vox-doc-pipeline --lint-only` checks pass (`0` errors).
