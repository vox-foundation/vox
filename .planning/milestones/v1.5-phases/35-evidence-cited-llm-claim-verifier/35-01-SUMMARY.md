---
phase: 35-evidence-cited-llm-claim-verifier
plan: 01
subsystem: vox-doc-pipeline
tags: [llm-verifier, evidence-citations, golden-drift-eval]
provides:
  - Structured prose claim extraction (`VERIFY-01`)
  - Evidence-cited verdicts requiring `[path:Lstart-Lend]` spans (`VERIFY-02`)
  - Content-addressed `(claim_hash, symbol_span_hash)` cache and token/spend caps (`VERIFY-03`)
  - Golden drift benchmark evaluation (`>= 50` pairs, `FPR <= 0.10`, `precision >= 0.80`) (`VERIFY-04`)
key-files:
  created:
    - crates/vox-doc-pipeline/src/claims/llm_verifier.rs
  modified:
    - crates/vox-doc-pipeline/src/claims/mod.rs
requirements-completed: [VERIFY-01, VERIFY-02, VERIFY-03, VERIFY-04]
completed: 2026-10-15
---

# Phase 35 Plan 01: Evidence-Cited LLM Claim Verifier Summary

## Accomplishments
- Implemented `extract_claim_candidates`, `is_valid_line_range_citation`, and `enforce_citation_requirement` in [`crates/vox-doc-pipeline/src/claims/llm_verifier.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/llm_verifier.rs) (`VERIFY-01`, `VERIFY-02`).
- Implemented `LlmVerifyCache` keyed by `(claim_hash, symbol_span_hash)` and `DocVerifyBudget` (`VERIFY-03`).
- Implemented `evaluate_golden_drift_set` scoring `>= 50` pairs against `contracts/reports/docs-freshness/golden-drift-eval-set.v1.json` with `FPR <= 0.10` and `precision >= 0.80` (`VERIFY-04`).

## Verification
- `timeout 900s cargo --config 'build.rustc-wrapper=""' test -p vox-doc-pipeline --lib` — **84 passed, 0 failed**.
