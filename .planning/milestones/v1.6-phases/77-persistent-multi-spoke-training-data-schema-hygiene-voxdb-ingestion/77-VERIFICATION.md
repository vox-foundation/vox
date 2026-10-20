---
phase: 77-persistent-multi-spoke-training-data-schema-hygiene-voxdb-ingestion
status: passed
verified: "2026-10-16T13:43:12Z"
score: 6/6
requirements_verified:
  - CORPUS-01
  - CORPUS-02
  - CORPUS-03
  - CORPUS-04
  - CORPUS-05
  - CORPUS-06
---

# Phase 77 Verification Report: Persistent Multi-Spoke Training Data Schema, Hygiene & `VoxDb` Ingestion

## Goal Verification

| # | Success Criterion | Status | Evidence |
|---|-------------------|--------|----------|
| 1 | `crates/vox-corpus/` defines `TrainingRecordV2` and `SpokeKind` (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`) with required oracle receipt and content digest fields (`CORPUS-01`). | ✓ PASSED | `crates/vox-corpus/src/training/contract.rs` (`SpokeKind`, `OracleReceipt`, `RecordProvenance`, `TrainingRecordV2`), verified by `training::contract::tests::training_record_v2_computes_digest_and_verifies_integrity`. |
| 2 | Positive `.vox` examples pass `vox-compiler` (`lex -> parse -> HIR -> typeck`) and `--interp` smoke validation (`CORPUS-02`); positive Rust examples pass `syn::parse_file` + stub filter (`CORPUS-03`); Shell/MCP/Harness trajectories pass secret redaction and safety-floor checks (`CORPUS-04`). | ✓ PASSED | `corpus_readiness::verify_vox_admission` & `admit_vox_record`; `extract_rs::verify_rust_item_admission` & `RsTrainingPair::to_training_record_v2`; `tool_workflow_corpus::redact_trajectory_secrets`, `classify_shell_safety_floor`, and `to_training_record_v2` converters. |
| 3 | `eval_split.rs` enforces exact content-digest + 3-gram Jaccard near-duplicate deduplication and verifies `0%` overlap with held-out `vox-eval` and `vox-audit` benchmark prompts (`CORPUS-05`). | ✓ PASSED | `crates/vox-corpus/src/corpus/eval_split.rs` (`deduplicate_records_v2`, `decontaminate_against_holdout`, `DecontaminationReport`), verified by unit tests. |
| 4 | `dataset_snapshot.rs` and `arca_replay.rs` persist immutable, Trusty-URI-identified dataset snapshots and per-spoke mixtures into `VoxDb` and reload them deterministically (`CORPUS-06`). | ✓ PASSED | `crates/vox-corpus/src/dataset_snapshot.rs` (`TrustyDatasetSnapshot`, `build_trusty_snapshot`, `persist_snapshot_to_db`, `load_snapshot_from_db`) and `crates/vox-corpus/src/arca_replay.rs` (`ReplayRow::to_training_record_v2`, `replay_snapshot_for_spoke`). |

## Test Execution Receipt

- Command: `timeout 900s cargo test -p vox-corpus --features database --lib`
- Result: `test result: ok. 217 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.95s`
