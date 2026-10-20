---
phase: 77-persistent-multi-spoke-training-data-schema-hygiene-voxdb-ingestion
plan: 02
subsystem: corpus
tags: [mens, training, rust-spoke, shell-spoke, mcp-spoke, decontamination]
requires: [77-01]
provides:
  - verify_rust_item_admission, RsTrainingPair::to_training_record_v2
  - redact_trajectory_secrets, classify_shell_safety_floor, ToolTraceRecord::to_training_record_v2, WorkflowTraceRecord::to_training_record_v2
  - DecontaminationReport, deduplicate_records_v2, decontaminate_against_holdout
affects: [crates/vox-corpus]
key-files:
  modified:
    - crates/vox-corpus/src/corpus/extract_rs.rs
    - crates/vox-corpus/src/tool_workflow_corpus.rs
    - crates/vox-corpus/src/corpus/eval_split.rs
key-decisions:
  - "Enforced syn::parse_file AST check + todo!/unimplemented! stub exclusion for RustWorkspace spoke records."
  - "Enforced secret redaction and shell safety floor classification before admitting StructuredShell, McpToolChain, and AgentHarness trajectories."
  - "Implemented exact content_digest + 3-gram Jaccard decontamination guaranteeing 0% prompt leakage against held-out eval benchmarks."
requirements-completed: [CORPUS-03, CORPUS-04, CORPUS-05]
completed: 2026-10-16
---

# Phase 77 Plan 02: Rust Spoke Extraction, Shell/MCP/Agent Trajectory Hygiene & Holdout Decontamination Summary

## Accomplishments
- Added `verify_rust_item_admission` and `RsTrainingPair::to_training_record_v2` in `crates/vox-corpus/src/corpus/extract_rs.rs` (`CORPUS-03`).
- Added `redact_trajectory_secrets`, `classify_shell_safety_floor`, and `to_training_record_v2` conversions in `crates/vox-corpus/src/tool_workflow_corpus.rs` (`CORPUS-04`).
- Added `DecontaminationReport`, `deduplicate_records_v2`, and `decontaminate_against_holdout` in `crates/vox-corpus/src/corpus/eval_split.rs` (`CORPUS-05`).
