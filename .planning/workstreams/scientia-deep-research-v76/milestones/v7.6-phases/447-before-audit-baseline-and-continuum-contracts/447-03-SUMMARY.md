---
phase: 447-before-audit-baseline-and-continuum-contracts
plan: 03
subsystem: testing
tags: [rust, scientia, audit, frozen-cassette, novelty, socrates, vox-publisher, vox-cli-research, json-schema]

# Dependency graph
requires:
  - phase: 447-before-audit-baseline-and-continuum-contracts
    provides: Canonical L1 contracts (`vox-research-events`) from Plan 447-01 and shared L3 `vox_scientia::embedder` + `VoxDb` v99 schema from Plan 447-02
provides:
  - Canonical L1 `NoveltyEvidenceBundle` & L3 `vox_scientia::embedder` unification in `vox-publisher` and thin re-export in `crates/vox-cli-research/src/db/publication/embedder.rs`
  - Honest multi-hop stage/span/bridge scoring module (`crates/vox-cli-research/src/scientia_v76_audit/scoring.rs`, 111 LoC) replacing pre-v7.6 hardcoded `1.0` stub in `eval.rs`
  - Deterministic frozen-cassette 6-axis + live-bug-reproduction audit harness (`crates/vox-cli-research/src/scientia_v76_audit/`, `vox scientia v76-audit --stage before`)
  - Validated pre-remediation baseline report at `contracts/reports/scientia-v76-before-baseline.v1.json` (`x-vox-version: 1`, `deterministic_seed: 20260447`) and schema `contracts/reports/scientia-v76-audit.v1.schema.json`
affects:
  - Phase 448–452 (`CONT`, `STRM`, `VERI`, `DISC`, `PUBL`, `AUDT-02` paired `--stage after` diff against `scientia-v76-before-baseline.v1.json`)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Canonical L1/L3 unification checked at test time via `std::any::TypeId::of` parity (`novelty_bundle_contract_parity.rs`)"
    - "Deterministic seeded frozen-cassette audit (`deterministic_seed: 20260447`) validating output JSON against Draft 2020-12 contract schema with `additionalProperties: false`"
    - "Pre-remediation authenticity invariant: audit probes intentionally execute and measure pre-v7.6 bugs in place without fixing them prematurely in Phase 447"

key-files:
  created:
    - contracts/reports/scientia-v76-audit.v1.schema.json
    - contracts/reports/scientia-v76-before-baseline.v1.json
    - crates/vox-cli-research/src/scientia_v76_audit/mod.rs
    - crates/vox-cli-research/src/scientia_v76_audit/scoring.rs
    - crates/vox-cli-research/src/scientia_v76_audit/cassettes.rs
    - crates/vox-cli-research/src/scientia_v76_audit/bug_probes.rs
    - crates/vox-cli-research/tests/fixtures/scientia_v76_cassettes.json
  modified:
    - crates/vox-cli-research/src/db/publication/embedder.rs
    - crates/vox-cli-research/src/eval.rs
    - crates/vox-publisher/tests/novelty_bundle_contract_parity.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs
    - contracts/index.yaml

key-decisions:
  - "Preserved all pre-v7.6 runtime bugs intentionally for Phase 447 so `vox scientia v76-audit --stage before` captures an authentic, non-zero pre-remediation baseline."
  - "Executed compiled test binaries (`target/debug/deps/*`) directly with `RUST_MIN_STACK=8388608` after `--no-run` batched builds with `VOX_BROKER_COALESCE=0` to avoid repeated cargo-broker lock contention."
  - "Applied coordinator-authorized mechanical build-unblock edits in `crates/vox-orchestrator-mcp` (`terminal_tools.rs`, `term_mentions.rs`) so the workspace binary and test binaries compile cleanly."

patterns-established:
  - "Deterministic before-baseline JSON persisted in `contracts/reports/scientia-v76-before-baseline.v1.json` as the immutable reference for Phase 452 `--stage after` comparison."

requirements-completed: [AUDT-01, DISC-01]

# Metrics
duration: 45min
completed: 2026-11-15
---

# Phase 447 Plan 03: Canonical L1/L3 Unification & Deterministic Pre-Remediation Audit Baseline Summary

**Phase 447 Plan 03 unified `vox-publisher` and `vox-cli-research` onto the canonical `L1`/`L3` novelty/embedder types and captured the deterministic pre-v7.6 baseline (`contracts/reports/scientia-v76-before-baseline.v1.json`) across all 6 evaluation axes and 9 live bug reproductions.**

## Performance

- **Completed:** 2026-11-15
- **Key Outputs:** Canonical `TypeId` parity in `vox-publisher`, honest multi-hop scoring in `scoring.rs`, frozen-cassette `scientia_v76_audit` harness, validated `scientia-v76-before-baseline.v1.json`, and all Phase 447 Wave A closure gates verified.

## Accomplishments

### Task 1: Canonical L1/L3 Type & Embedder Unification (`DISC-01`)
- Unified `vox-publisher` and `crates/vox-cli-research/src/db/publication/embedder.rs` (`3` LoC re-exporting `vox_scientia::embedder::{CachedLlmEmbedder, Embedder, EmbedderBackend, LocalHashEmbedder}`) onto the canonical `L1` `vox_research_events::schema_types::NoveltyEvidenceBundle` and `L3` `vox_scientia::embedder::Embedder`.
- Confirmed identity parity in `crates/vox-publisher/tests/novelty_bundle_contract_parity.rs` (`std::any::TypeId::of` equality checks for `NoveltyEvidenceBundleV1 == NoveltyEvidenceBundle` and `dyn vox_publisher::scientia_semantic::Embedder == dyn vox_scientia::embedder::Embedder`).
- Verified `vox-publisher`: `195 passed; 0 failed` (unit tests), `44 passed; 0 failed` across 13 integration suites (including `novelty_bundle_contract_parity`: `3` passed), and `0 passed; 0 failed` (doc-tests).

### Task 2: Honest Scoring, Frozen-Cassette Audit & Pre-Remediation Baseline (`AUDT-01`)
- Replaced pre-v7.6 hardcoded `1.0` stub scoring in `crates/vox-cli-research/src/eval.rs` (`729` LoC total; `538` LoC pre-`#[cfg(test)]`) via `crates/vox-cli-research/src/scientia_v76_audit/scoring.rs` (`111` LoC, `<150` budget) exposing `is_multi_hop_query`, `bridge_entity_recall`, `multi_hop_pipeline_score`, and `multi_hop_completion_score`.
- Built the deterministic frozen-cassette audit module under `crates/vox-cli-research/src/scientia_v76_audit/` (`mod.rs` `205` LoC, `cassettes.rs` `388` LoC, `bug_probes.rs` `391` LoC; all `<400` LoC) backed by `crates/vox-cli-research/tests/fixtures/scientia_v76_cassettes.json` (`258` LoC; `4` query cases, `6` Socrates cases, `5` claim pairs, `3` novelty bundles).
- Generated and schema-validated `contracts/reports/scientia-v76-before-baseline.v1.json` (`"x-vox-version": 1`, `"schema_version": 1`, `"stage": "before"`, `"deterministic_seed": 20260447`) against `contracts/reports/scientia-v76-audit.v1.schema.json` via `vox scientia v76-audit --stage before`.

### Captured Pre-Remediation Baseline (`scientia-v76-before-baseline.v1.json`)
- **Performance:** `simulated_fast_p50_ms: 610`, `simulated_deep_p95_ms: 5390`, `kv_cache_prefix_stability_ratio: 0.544548`, `session_detail_ipc_payload_bytes: 140517`, `gui_cli_db_spawn_count: 3`
- **Quality:** `groundedness_mean: 0.25`, `citation_precision_mean: 1.0`, `multi_hop_bridge_recall_mean: 0.833333`, `domain_coverage_modes: ["general", "shopping", "codegen", "scholarly"]`
- **Verification:** `claim_precision: 0.5`, `contradiction_detection_recall: 0.333333`, `contradiction_false_support_rate: 0.666667`, `mock_verifier_fallback_rate: 1.0`
- **Integration:** `composer_continuum_fidelity_rate: 0.0`, `doc_publish_artifact_retention_rate: 0.0`, `sequenced_event_replay_supported: true`
- **Socrates Calibration:** `ece: 0.198333`, `brier_score: 0.159417`, `selective_abstention_f1: 0.8`
- **Novelty Detection:** `prior_art_recall_at_5: 0.5`, `false_novelty_rate: 1.0`, `embedder_l3_shared: true`
- **Live Bug Reproductions (`9/9` reproduced cleanly before remediation):**
  - `composer_research_scope_overwritten: true`
  - `research_turn_waves_hardcoded_to_one: true`
  - `none_to_novel_false_positive_rate: 1.0`
  - `artifact_json_wiped_on_doc_publish: true`
  - `mock_verifier_fallback_rate: 1.0`
  - `contradiction_false_support_rate: 0.666667`
  - `kv_cache_prefix_stability_ratio: 0.544548`
  - `session_detail_ipc_payload_bytes: 140517`
  - `gui_cli_db_spawn_count: 3`

## Verification Receipts (Observed Outputs)

1. **`vox-publisher` test suite (`cargo test -p vox-publisher` & `--doc`)**
   - Unit tests: `195 passed; 0 failed` (`finished in 62.79s`)
   - Integration tests across 13 suites: `44 passed; 0 failed` (`dry_run_tests`: 1, `extracted_claims_overrides_citation_heuristic`: 4, `http_mock_publish`: 2, `novelty_bundle_contract_parity`: 3, `novelty_golden_harness`: 1, `provider_atlas`: 6, `route_simulation_golden`: 1, `routing_policy_tests`: 6, `scholarly_openreview_mock_test`: 1, `scholarly_zenodo_mock_test`: 3, `scientia_novelty_acceptance`: 8, `trust_snapshot`: 7, `zenodo_autofill_sandbox_fixture_test`: 1)
   - Doc-tests (`--doc`): exit `0` (`0 passed; 0 failed`)
2. **Compiled `--no-run` builds (`VOX_BROKER_COALESCE=0`)**
   - `cargo test -p vox-cli-research --lib --tests --features runtime --no-run`: exit `0` (`4m 10s`); produced `vox_cli_research-5d086db9c4c7b12e`, `clap_arg_order_test-51e2c74bc4878e05`, `eval_e2e_test-d3e969e5eb5ccc6d` (1 non-blocking unused-imports warning at `crates/vox-cli-research/src/db/publication/embedder.rs:3`).
   - `cargo test -p vox-cli --test command_catalog_paths_baseline --no-run`: exit `0` (`18m 24s`); built `target/debug/vox` and `command_catalog_paths_baseline-c3a28365d1149f73`.
3. **Direct compiled test runs (`RUST_MIN_STACK=8388608`)**
   - `vox_cli_research-5d086db9c4c7b12e eval::tests`: `test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 607 filtered out; finished in 41.53s`
   - `vox_cli_research-5d086db9c4c7b12e scientia_v76_audit`: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 660 filtered out; finished in 32.28s` (`audit_report_is_byte_for_byte_deterministic_and_schema_valid`, `before_stage_captures_authentic_pre_v76_live_bug_reproductions`)
   - `eval_e2e_test-d3e969e5eb5ccc6d`: `test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s` (`test_eval_executes_real_research_pipeline_not_web_lines` is `#[ignore]`, requiring live network + API key)
   - `command_catalog_paths_baseline-c3a28365d1149f73`: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s`
4. **Baseline CLI generation**
   - `./target/debug/vox scientia v76-audit --stage before`: exit `0`, wrote `contracts/reports/scientia-v76-before-baseline.v1.json`.
5. **CI & Architectural Gates**
   - `./target/debug/vox ci contracts-index`: exit `0` (`contracts-index OK (schema v1, 211 entries)`).
   - `cargo run -p vox-jsonschema-util --example emit_agent_harness -- --verify`: exit `0` (`Verification passed for /Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/generated/agent_harness.rs.`).
   - `./target/debug/vox ci data-storage-guard`: exited `1` with `531` pre-existing repo-wide violations (`ignore-tracked-parity`, pre-existing `version-header-parity`, pre-existing `env-parity`); `rg` over full command output confirmed `0` violations for Phase 447 files/checks (`scientia-v76-*`, `research-continuum-events*`, `turso-only-persistence`, `schema-codegen-drift`, `continuum`, `v76`).
   - `cargo run -p vox-arch-check`: exit `0` (zero errors; 12 pre-existing `[warn] LoC delta >15% since last release` notices across workspace crates).
   - Banned hardcoded OS path check (`"/tmp|"/var|"/home` in modified crates): `0` matches.

## Deviations from Plan

- **Authorized Mechanical Build Unblock (Step A0):** Two unrelated pre-existing compile blockers in `crates/vox-orchestrator-mcp` (`dispatch_domains/terminal_tools.rs` lines ~57 & ~386 `scrub_terminal_mcp_str` `Option<String>` mismatch, and `chat_tools/chat/term_mentions.rs` lines ~99 & ~184 match arm / borrow split) were fixed under `@coordinator` authorization (`#62`) and formatted with `rustfmt --edition 2024`.
- **Non-blocking Cleanup Note:** `crates/vox-cli-research/src/db/publication/embedder.rs:3` emits a single compiler unused-imports warning during `vox-cli-research` compilation; functionality and re-exports are unaffected.

## Next Phase Readiness

- Phase 447 is complete (`447-01`, `447-02`, `447-03` verified).
- Wave B (Phase 448: Unified Light ↔ Deep Continuum Router, Socrates Escalation & KV-Cache-Safe Chat Harness) can proceed immediately against the immutable baseline `contracts/reports/scientia-v76-before-baseline.v1.json`.

---
*Phase: 447-before-audit-baseline-and-continuum-contracts*
*Completed: 2026-11-15*
