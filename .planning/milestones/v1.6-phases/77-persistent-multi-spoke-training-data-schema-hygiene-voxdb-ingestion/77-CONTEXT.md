# Phase 77: Persistent Multi-Spoke Training Data Schema, Hygiene & `VoxDb` Ingestion — Context

**Gathered:** 2026-10-16
**Status:** Ready for execution
**Milestone:** `v1.6 Mens Continuous Training, Data Flywheel & End-to-End Evaluation`

<domain>
## Phase Boundary

Establish a strict, schema-validated, decontaminated, and database-persisted training corpus spanning all 5 Vox spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`) in `crates/vox-corpus/` so every training example is deterministically verified before entering Mens training splits and persisted as immutable Trusty-URI snapshots in `VoxDb`.

Requirements covered: `CORPUS-01`, `CORPUS-02`, `CORPUS-03`, `CORPUS-04`, `CORPUS-05`, `CORPUS-06`.
</domain>

<decisions>
## Implementation Decisions

### Schema & Digest (`CORPUS-01`)
- Define `SpokeKind`, `OracleReceipt`, and `TrainingRecordV2` in `crates/vox-corpus/src/training/contract.rs` and re-export them from `crates/vox-corpus/src/training/mod.rs` and `crates/vox-corpus/src/lib.rs`.
- Comply with `AGENTS.md §Cryptography Policy`: do not add a direct `blake3` crate import to `vox-corpus`; compute canonical 128-bit content digests (`xxh3_128` hex) in pure `vox-corpus` and pair with `vox_db::hash::content_hash` (SHA3-512 Base32Hex Trusty URI CAS key) when persisting to `VoxDb`.

### Spoke Admission Gates (`CORPUS-02`, `CORPUS-03`, `CORPUS-04`)
- `.vox` admission (`CORPUS-02`): run `vox_compiler::pipeline::run_frontend_str` (`lex -> parse -> HIR -> typeck`) and smoke-check interpreter readiness on positive `.vox` examples in `corpus_readiness.rs`.
- Rust admission (`CORPUS-03`): validate extracted Rust items in `extract_rs.rs` with `syn::parse_file` and anti-stub quality checks (rejecting `todo!()`, `unimplemented!()`, or syntax errors) before converting to `TrainingRecordV2`.
- Shell / MCP / Harness trajectory admission (`CORPUS-04`): enforce secret redaction (`[REDACTED]`) and shell safety-floor checks (`classify_shell_safety_floor`) on `ToolTraceRecord`, `WorkflowTraceRecord`, and `A2ATraceRecord` in `tool_workflow_corpus.rs`.

### Deduplication & Holdout Decontamination (`CORPUS-05`)
- Extend `crates/vox-corpus/src/corpus/eval_split.rs` with `deduplicate_records_v2` (exact content-digest + 3-gram Jaccard near-duplicate removal) and `decontaminate_against_holdout` (filtering any training record whose prompt or normalized code shares exact digest or `>= threshold` n-gram Jaccard similarity with held-out `vox-eval` / `vox-audit` prompts).

### Persistent `VoxDb` Trusty-URI Dataset Snapshots (`CORPUS-06`)
- Extend `crates/vox-corpus/src/dataset_snapshot.rs` and `crates/vox-corpus/src/arca_replay.rs` to persist and reload immutable `TrustyDatasetSnapshot` bundles and `TrainingRecordV2` rows via `VoxDb::store` (`objects` CAS table), `VoxDb::bind_name` (`names` table under `"mens_dataset_snapshot"`), and `VoxDb::upsert_corpus_quality`.
</decisions>
