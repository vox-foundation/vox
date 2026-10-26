---
phase: 191-live-cli-llm-claim-verifier-golden-drift-benchmark-wiring
plan: 01
subsystem: docs-llm-verifier
tags: [llm-verifier, golden-drift, evidence-citations, caching]
status: completed
---

# Phase 191 Summary: Live CLI LLM Claim Verifier & Golden Drift Benchmark Wiring

## Requirements Completed (`OPENWIKI-VERIFY-01..04`)
- **`OPENWIKI-VERIFY-01`**: Added `--verify-llm` and `--eval-golden-drift` flags to `DocsAuditArgs` (`crates/vox-cli/src/commands/audit.rs`) and wired them to `run_advisory_claim_verification` and `evaluate_golden_drift_set`.
- **`OPENWIKI-VERIFY-02`**: Persisted content-addressed `(claim_hash, symbol_span_hash)` verdicts to `contracts/reports/docs-freshness/llm-verify-cache.v1.json` with token/claim budget enforcement (`DocVerifyBudget`).
- **`OPENWIKI-VERIFY-03`**: Added `is_valid_line_range_citation_on_disk` in `crates/vox-doc-pipeline/src/claims/llm_verifier.rs` to verify both `[path:Lstart-Lend]` syntax and on-disk file existence.
- **`OPENWIKI-VERIFY-04`**: Verified `evaluate_golden_drift_set` scores `50/50` pairs (`100.0%` precision, `0.0%` false-positive rate, `eligible_for_bot_promotion = true`).

## Verification
- `cargo test -p vox-doc-pipeline --test openwiki_e2e_integration` (`phase_191_llm_claim_verifier_and_golden_drift_benchmark ... ok`).
