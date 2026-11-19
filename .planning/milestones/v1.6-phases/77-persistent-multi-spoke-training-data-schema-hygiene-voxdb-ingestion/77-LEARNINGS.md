---
phase: 77
phase_name: "persistent-multi-spoke-training-data-schema-hygiene-voxdb-ingestion"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 13
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 77 Learnings: persistent-multi-spoke-training-data-schema-hygiene-voxdb-ingestion

## Decisions

### Used xxhash_rust::xxh3::xxh3_128 for deterministic 32-hex content digest...
Used xxhash_rust::xxh3::xxh3_128 for deterministic 32-hex content digests and Trusty URIs to respect AGENTS.md crypto boundaries.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 77-01-SUMMARY.md

---

### Verified VoxLang admission in-process via vox_compiler::pipeline::run_fr...
Verified VoxLang admission in-process via vox_compiler::pipeline::run_frontend_str and vox_compiler::eval::Interpreter.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 77-01-SUMMARY.md

---

### Implemented SpokeKind (VoxLang, RustWorkspace, StructuredShell, McpToolC...
Implemented `SpokeKind` (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), `OracleReceipt`, `RecordProvenance`, and `TrainingRecordV2` with deterministic `xxh3_128` content digests (`trusty://vox-record/<spoke>/<digest>`) and integrity verification (`CORPUS-01`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-01-SUMMARY.md

---

### Implemented verify_vox_admission and admit_vox_record in crates/vox-corp...
Implemented `verify_vox_admission` and `admit_vox_record` in `crates/vox-corpus/src/corpus/corpus_readiness.rs` gating `.vox` training records through `run_frontend_str` (`lex -> parse -> HIR -> typeck`) and in-process `Interpreter` module load + zero-arg `main()` execution (`CORPUS-02`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-01-SUMMARY.md

---

### Enforced syn::parse_file AST check + todo!/unimplemented! stub exclusion...
Enforced syn::parse_file AST check + todo!/unimplemented! stub exclusion for RustWorkspace spoke records.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 77-02-SUMMARY.md

---

### Enforced secret redaction and shell safety floor classification before a...
Enforced secret redaction and shell safety floor classification before admitting StructuredShell, McpToolChain, and AgentHarness trajectories.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 77-02-SUMMARY.md

---

### Implemented exact content_digest + 3-gram Jaccard decontamination guaran...
Implemented exact content_digest + 3-gram Jaccard decontamination guaranteeing 0% prompt leakage against held-out eval benchmarks.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 77-02-SUMMARY.md

---

### Added verify_rust_item_admission and RsTrainingPair::to_training_record_...
Added `verify_rust_item_admission` and `RsTrainingPair::to_training_record_v2` in `crates/vox-corpus/src/corpus/extract_rs.rs` (`CORPUS-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-02-SUMMARY.md

---

### Added redact_trajectory_secrets, classify_shell_safety_floor, and to_tra...
Added `redact_trajectory_secrets`, `classify_shell_safety_floor`, and `to_training_record_v2` conversions in `crates/vox-corpus/src/tool_workflow_corpus.rs` (`CORPUS-04`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-02-SUMMARY.md

---

### Added DecontaminationReport, deduplicate_records_v2, and decontaminate_a...
Added `DecontaminationReport`, `deduplicate_records_v2`, and `decontaminate_against_holdout` in `crates/vox-corpus/src/corpus/eval_split.rs` (`CORPUS-05`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-02-SUMMARY.md

---

### Stored Trusty-URI dataset snapshots in VoxDb's existing content-addresse...
Stored Trusty-URI dataset snapshots in VoxDb's existing content-addressed store (objects + names tables via VoxDb::store and VoxDb::bind_name) without changing VoxDb baseline SQL DDL.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 77-03-SUMMARY.md

---

### Implemented SpokeMixtureSummary, TrustyDatasetSnapshot, build_trusty_sna...
Implemented `SpokeMixtureSummary`, `TrustyDatasetSnapshot`, `build_trusty_snapshot`, `persist_snapshot_to_db`, and `load_snapshot_from_db` in `crates/vox-corpus/src/dataset_snapshot.rs` (`CORPUS-06`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-03-SUMMARY.md

---

### Implemented ReplayRow::to_training_record_v2 and replay_snapshot_for_spo...
Implemented `ReplayRow::to_training_record_v2` and `replay_snapshot_for_spoke` in `crates/vox-corpus/src/arca_replay.rs` (`CORPUS-06`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 77-03-SUMMARY.md

## Lessons

## Patterns

## Surprises

