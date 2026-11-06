---
phase: 172-monolithic-god-object-decomposition-and-500-line-module-ergonomics-gate
reviewed: "2026-10-24T15:25:00Z"
depth: standard
files_reviewed: 49
files_reviewed_list:
  - crates/vox-code-audit/src/rules.rs
  - crates/vox-code-audit/src/analysis/rust_context.rs
  - crates/vox-code-audit/src/analysis/token_map.rs
  - crates/vox-code-audit/src/detectors/god_object.rs
  - crates/vox-code-audit/src/detectors/untested_pub_api.rs
  - crates/vox-code-audit/src/detectors/env_secret_shape.rs
  - crates/vox-code-audit/src/detectors/secrets.rs
  - crates/vox-code-audit/src/detectors/stub.rs
  - crates/vox-code-audit/src/detectors/magic_value.rs
  - crates/vox-code-audit/src/detectors/ai_laziness.rs
  - crates/vox-arch-check/src/main.rs
  - crates/vox-arch-check/src/config_types.rs
  - crates/vox-arch-check/src/report_model.rs
  - crates/vox-arch-check/src/git_and_walk.rs
  - crates/vox-arch-check/src/rule_runners.rs
  - crates/vox-corpus/src/training/spoke_roundtrip.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/verifiers_core.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/verifiers_shell_mcp.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/verifiers_harness.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/verifiers_ext_spokes.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/suites_five_spoke.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/organic_types.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/organic_oracles.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/organic_eval.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/suites_organic_part1.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/suites_organic_part2.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/suites_organic_adversarial.rs
  - crates/vox-corpus/src/training/spoke_roundtrip/tests.rs
  - crates/vox-orchestrator/src/models/registry.rs
  - crates/vox-orchestrator/src/models/registry/catalog_seed.rs
  - crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs
  - crates/vox-orchestrator/src/models/registry/conversational_routing.rs
  - crates/vox-orchestrator/src/models/registry/conversational_workflows.rs
  - crates/vox-orchestrator/src/models/registry/sota_harness_eval.rs
  - crates/vox-orchestrator/src/models/registry/selection_queries.rs
  - crates/vox-orchestrator/src/models/registry/tests.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/turn_events.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/context_and_memo.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/batch_and_eval_gate.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/tests.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/posture_and_routing.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/try_agent_turn.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/tests.rs
  - crates/vox-orchestrator-mcp/tests/autonomous_chat_research_test.rs
findings:
  critical: 0
  warning: 4
  info: 0
  total: 4
status: all_fixed
---

# Phase 172: Code Review Report (`--fix` Applied)

**Reviewed:** 2026-10-24T15:25:00Z
**Depth:** Standard + `cargo check --tests` + `toestub --min-severity error` + Unit/Integration Test Suites
**Files Reviewed:** 49
**Status:** All 4 Warning Findings Remediated Inline (`0` Remaining)

## Summary

Reviewed all 49 files across the TOESTUB precision engine (`vox-code-audit`), the 5 decomposed God Object facade modules (`spoke_roundtrip.rs`, `registry.rs`, `agent_loop.rs`, `message.rs`, `vox-arch-check/src/main.rs`), their 34 child submodules, and integration tests. Identified and immediately fixed 4 issues (`WR-01..04`).

## Warnings (All Fixed via `--fix`)

### WR-01: Submodule directory expansion needed in `discover_workspace_evidence_with_mode` after splitting `registry.rs` into `registry/*.rs`

**File:** `crates/vox-corpus/src/training/spoke_roundtrip/organic_types.rs:351-368`
**Issue:** When `crates/vox-orchestrator/src/models/registry.rs` was decomposed into `registry/*.rs`, `organic_harness_02_model_registry_plan` (whose `search_scope_paths` targets `"crates/vox-orchestrator/src/models/registry.rs"`) lostBM25/grounding token density for `register_calibrated_mens_adapter` and `route_spoke_task` because those `impl` blocks moved into `registry/mens_spoke_routing.rs`.
**Fix Applied:** Updated `discover_workspace_evidence_with_mode` so that when a candidate file `<dir>/<stem>.rs` has a companion submodule directory `<dir>/<stem>/`, its child `.rs` files are concatenated into the indexed document content. Verified `100.0%` Recall@5, Grounding, and E2E `pass@1` across all 24/40/48/64 organic cases and all 17 `models::registry::tests`.

### WR-02: Extracted `<dir>/tests.rs` submodule files were not recognized as test files by `EnvSecretShapeDetector`, `SecretDetector`, `StubDetector`, `MagicValueDetector`, and `AiLazinessDetector`

**File:** `crates/vox-code-audit/src/detectors/{env_secret_shape.rs, secrets.rs, stub.rs, magic_value.rs, ai_laziness.rs}`
**Issue:** Extracting `#[cfg(test)] mod tests { ... }` into out-of-line `#[cfg(test)] mod tests;` (`<dir>/tests.rs`) caused `toestub` to scan `agent_loop/tests.rs` and `message/tests.rs` as production code because the path filters only checked `path_str.contains("/tests/") || path_str.ends_with("_test.rs") || path_str.ends_with("_tests.rs")`, missing `path_str.ends_with("/tests.rs")`.
**Fix Applied:** Added `path_str.ends_with("/tests.rs")` to `env_secret_shape.rs`, `secrets.rs`, `stub.rs`, `magic_value.rs`, and `ai_laziness.rs`. Verified `toestub --min-severity error` reports `0 critical, 0 error` across all 49 files.

### WR-03: `crates/vox-orchestrator-mcp/tests/autonomous_chat_research_test.rs` struct literal missing `oversight_level`, `kb_scope`, and `grounding_check_enabled`

**File:** `crates/vox-orchestrator-mcp/tests/autonomous_chat_research_test.rs:41-46`
**Issue:** `cargo check -p vox-orchestrator-mcp --tests` failed with `E0063` in `autonomous_chat_research_test.rs` because `ChatMessageParams` had 3 fields (`oversight_level`, `kb_scope`, `grounding_check_enabled`) omitted from the explicit struct initializer.
**Fix Applied:** Added `oversight_level: None, kb_scope: None, grounding_check_enabled: None` to `autonomous_chat_research_test.rs`. Verified `cargo check -p vox-orchestrator-mcp --test autonomous_chat_research_test` exits `0`.

### WR-04: Unused `extract_symbol_span` warning in `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs:145`
**Issue:** Compiler emitted `#[warn(dead_code)]` on `pub fn extract_symbol_span` during `lib` compilation.
**Fix Applied:** Added `#[allow(dead_code)]` to `extract_symbol_span`. Verified `0` warnings in `vox-orchestrator-mcp`.

---

_Reviewed: 2026-10-24T15:25:00Z_
_Reviewer: Antigravity (`gsd-code-review --fix`)_
