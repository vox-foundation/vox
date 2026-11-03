---
phase: "21-measure-and-prune"
plan: "02"
status: completed
date: "2026-10-13"
requirements_met:
  - MEASURE-03
  - MEASURE-04
---

# Plan 21-02 Summary: CLI Command `vox audit docs`, Evaluation Sample & Reality Audit Schema Bump

## Execution Summary
Implemented the `vox audit docs` subcommand, hand-labelled evaluation sample, and bumped the Docs Reality Audit schemas with fingerprint deduplication and metrics tracking.

## Delivered Components
1. **Schema Bump (`contracts/reports/docs-reality-audit/`)**:
   - `findings.v1.schema.json`: Added `source`, `fingerprint` (64-char SHA-256 hex), and `detector`.
   - `metrics.v1.schema.json`: Added `corpus_pages_scanned`, `total_mentions_extracted`, `resolved_mentions_count`, `dead_mentions_count`, and `health_status` (`"unpopulated"`, `"healthy"`, `"deficits_found"`).
2. **CI Synchronization (`crates/vox-cli-ci/src/docs_reality_audit.rs`)**:
   - Updated `FindingRow` with `source`, `fingerprint`, and `detector`.
   - Enforced deterministic fingerprint uniqueness across findings.
   - `vox ci docs-reality-audit verify` passed cleanly.
3. **Evaluation Dataset & Extractor Accuracy (`contracts/reports/docs-reality-audit/evaluation-sample.v1.json`, `crates/vox-doc-pipeline/src/claims/eval.rs`)**:
   - 25 hand-labelled mentions across 5 documentation files.
   - Measured accuracy: Precision 100.00%, Recall 100.00%, False-Positive Rate 0.00% (< 5% threshold).
4. **Findings & Metrics Generation (`crates/vox-doc-pipeline/src/claims/findings.rs`)**:
   - Deterministic SHA-256 fingerprint generation.
   - Idempotent deduplication by fingerprint.
   - Priority score and band calculation matching program formula.
5. **CLI Subcommand `vox audit docs` (`crates/vox-cli/src/commands/audit.rs`)**:
   - Subcommand under `vox audit` supporting `--format text`, `--format json`, `--format jsonl`, `--paths`, `--eval-sample`, and `--write-findings`.
   - Emits findings JSONL adhering to `schema_version: 1` umbrella audit contract.
   - Report-only mode (exit 0) in Phase 21.

## Verification
- Evaluation sample: `timeout 60s cargo test -p vox-doc-pipeline claims::eval` passed (1/1 ok).
- CI schema verification: `timeout 60s cargo run -p vox-cli -- ci docs-reality-audit verify` passed (exit 0).
- Subcommand execution: `timeout 60s cargo run -p vox-cli -- audit docs --format text` scanned 862 pages, extracted 27,835 mentions (exit 0).
- JSONL format: `timeout 60s cargo run -p vox-cli -- audit docs --format jsonl --paths docs/src/ref/env-vars.md` passed (exit 0).
- Architecture & layers check: `timeout 60s cargo run -p vox-arch-check` passed (exit 0).
