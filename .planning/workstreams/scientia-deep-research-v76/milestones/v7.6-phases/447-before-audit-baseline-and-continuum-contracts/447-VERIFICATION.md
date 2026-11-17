---
phase: 447-before-audit-baseline-and-continuum-contracts
verified: 2026-11-15
verifier: rust-verifier
status: passed
requirements: [AUDT-01, DISC-01]
---

# Phase 447 Verification Matrix

## Requirement Coverage

| Requirement | Plan(s) | Verification Evidence | Status |
|---|---|---|---|
| **DISC-01** | `447-01`, `447-02`, `447-03` | `vox-research-events` (`37 passed`), `vox-scientia --lib embedder` (`4 passed`), `vox-publisher` (`195 unit + 44 integration + 0 doc` including `novelty_bundle_contract_parity` `TypeId` checks), thin re-export in `crates/vox-cli-research/src/db/publication/embedder.rs` | Passed |
| **AUDT-01** | `447-03` | `eval::tests` (`55 passed`), `scientia_v76_audit` (`2 passed`), `vox scientia v76-audit --stage before` (`exit 0`), `contracts/reports/scientia-v76-before-baseline.v1.json` validated against `contracts/reports/scientia-v76-audit.v1.schema.json` | Passed |

## Full Command Receipt Log

### Plan 447-01 Checks (Recorded in `447-01-SUMMARY.md`)
- `vox-orchestrator-types`: `51 + 3 + 5 + 2` tests passed
- `vox-research-events`: `37` tests passed
- `vox-scientia --lib inspect_bridge`: `18` tests passed
- `vox-arch-check`: `0` errors

### Plan 447-02 Checks (Executed & Recorded in `447-02-SUMMARY.md`)
- `timeout 900s cargo test -p vox-db-types`:
  - `src/lib.rs`: `43 passed; 0 failed`
  - `tests/serde_uniformity.rs`: `1 passed; 0 failed`
  - Doc-tests: `1 passed; 0 failed`
- `timeout 900s cargo test -p vox-db baseline_policy_matches_compiled_schema`: `1 passed; 0 failed; 312 filtered out`
- `timeout 900s cargo test -p vox-db research_session_pages`: `3 passed; 0 failed; 310 filtered out`
- `timeout 900s cargo test -p vox-scientia --lib embedder`: `4 passed; 0 failed; 347 filtered out`
- `timeout 900s cargo test -p vox-research-shim --lib types`: `4 passed; 0 failed; 153 filtered out`
- `cargo test -p vox-db legacy_export_covers_all_baseline_tables`: `"research_session_sources"` covered; 4 pre-existing `gsd_*` table omissions (`gsd_documents`, `gsd_milestones`, `gsd_phases`, `gsd_requirements`) exist on `HEAD` under `--features legacy-import` unrelated to `447-02`.

### Plan 447-03 Checks (Executed on Current Workspace State)
- `timeout 900s cargo test -p vox-publisher`:
  - Unit tests: `195 passed; 0 failed` (`finished in 62.79s`)
  - Integration suites (13 suites): `44 passed; 0 failed` (`dry_run_tests`: 1, `extracted_claims_overrides_citation_heuristic`: 4, `http_mock_publish`: 2, `novelty_bundle_contract_parity`: 3, `novelty_golden_harness`: 1, `provider_atlas`: 6, `route_simulation_golden`: 1, `routing_policy_tests`: 6, `scholarly_openreview_mock_test`: 1, `scholarly_zenodo_mock_test`: 3, `scientia_novelty_acceptance`: 8, `trust_snapshot`: 7, `zenodo_autofill_sandbox_fixture_test`: 1)
- `timeout 900s cargo test -p vox-publisher --doc`: exit `0` (`0 passed; 0 failed`)
- `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=2400 timeout 3600s cargo test -p vox-cli-research --lib --tests --features runtime --no-run`: exit `0` (`4m 10s`)
- `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=2400 timeout 3600s cargo test -p vox-cli --test command_catalog_paths_baseline --no-run`: exit `0` (`18m 24s`)
- `timeout 180s ./target/debug/vox scientia v76-audit --stage before`: exit `0`, wrote `contracts/reports/scientia-v76-before-baseline.v1.json` (`"x-vox-version": 1`, `"schema_version": 1`, `"stage": "before"`, `"deterministic_seed": 20260447`)
- `RUST_MIN_STACK=8388608 timeout 180s ./target/debug/deps/vox_cli_research-5d086db9c4c7b12e eval::tests`: `55 passed; 0 failed; 0 ignored; 0 measured; 607 filtered out; finished in 41.53s`
- `RUST_MIN_STACK=8388608 timeout 180s ./target/debug/deps/vox_cli_research-5d086db9c4c7b12e scientia_v76_audit`: `2 passed; 0 failed; 0 ignored; 0 measured; 660 filtered out; finished in 32.28s`
- `RUST_MIN_STACK=8388608 timeout 180s ./target/debug/deps/eval_e2e_test-d3e969e5eb5ccc6d`: `0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s` (`test_eval_executes_real_research_pipeline_not_web_lines` is `#[ignore]` requiring live network + API key)
- `RUST_MIN_STACK=8388608 timeout 180s ./target/debug/deps/command_catalog_paths_baseline-c3a28365d1149f73`: `1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s`
- `timeout 180s ./target/debug/vox ci contracts-index`: exit `0` (`contracts-index OK (schema v1, 211 entries)`)
- `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=600 timeout 600s cargo run -p vox-jsonschema-util --example emit_agent_harness -- --verify`: exit `0` (`Verification passed for /Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/generated/agent_harness.rs.`)
- `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=300 timeout 300s ./target/debug/vox ci data-storage-guard`: exited `1` with `531` pre-existing repo-wide violations (`ignore-tracked-parity`, pre-existing `version-header-parity`, pre-existing `env-parity`); `0` Phase 447 violations found when searching full step output
- `VOX_BROKER_COALESCE=0 VOX_BROKER_RUN_TIMEOUT_SECS=600 timeout 900s cargo run -p vox-arch-check`: exit `0` (zero errors; 12 pre-existing LoC release-delta warnings)

## Baseline Metrics Snapshot (`contracts/reports/scientia-v76-before-baseline.v1.json`)

- **Cassettes:** `query_cases: 4`, `socrates_cases: 6`, `claim_pairs: 5`, `novelty_bundles: 3`
- **6 Axes Measured:**
  - `performance`: `simulated_fast_p50_ms=610`, `simulated_deep_p95_ms=5390`, `kv_cache_prefix_stability_ratio=0.544548`, `session_detail_ipc_payload_bytes=140517`, `gui_cli_db_spawn_count=3`
  - `quality`: `groundedness_mean=0.25`, `citation_precision_mean=1.0`, `multi_hop_bridge_recall_mean=0.833333`, `domain_coverage_modes=["general", "shopping", "codegen", "scholarly"]`
  - `verification`: `claim_precision=0.5`, `contradiction_detection_recall=0.333333`, `contradiction_false_support_rate=0.666667`, `mock_verifier_fallback_rate=1.0`
  - `integration`: `composer_continuum_fidelity_rate=0.0`, `doc_publish_artifact_retention_rate=0.0`, `sequenced_event_replay_supported=true`
  - `socrates_calibration`: `ece=0.198333`, `brier_score=0.159417`, `selective_abstention_f1=0.8`
  - `novelty_detection`: `prior_art_recall_at_5=0.5`, `false_novelty_rate=1.0`, `embedder_l3_shared=true`
- **9/9 Pre-v7.6 Bug Reproductions Active:** `composer_research_scope_overwritten=true`, `research_turn_waves_hardcoded_to_one=true`, `none_to_novel_false_positive_rate=1.0`, `artifact_json_wiped_on_doc_publish=true`, `mock_verifier_fallback_rate=1.0`, `contradiction_false_support_rate=0.666667`, `kv_cache_prefix_stability_ratio=0.544548`, `session_detail_ipc_payload_bytes=140517`, `gui_cli_db_spawn_count=3`
