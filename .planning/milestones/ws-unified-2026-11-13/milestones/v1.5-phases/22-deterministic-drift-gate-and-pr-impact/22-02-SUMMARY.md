---
phase: "22-deterministic-drift-gate-and-pr-impact"
plan: "02"
status: completed
date: "2026-10-13"
requirements_met:
  - DRIFT-04
  - DRIFT-05
  - DRIFT-06
---

# Plan 22-02 Summary: Tighten-Only Drift Ratchet Gate, Suppressions & Re-Attestation

## Execution Summary
Implemented the tighten-only drift ratchet gate, expiring suppressions validator, and human re-attestation CLI command. Generated the grandfathered reader-facing deficit baseline (`baseline.v1.json`) and verified with CLI runs and integration tests.

## Delivered Components
1. **Grandfathered Baseline (`contracts/reports/docs-freshness/baseline.v1.json`)**:
   - Captured 920 grandfathered reader-facing deficits across reference, tutorials, how-to, explanation, and concepts pages.
   - Tighten-only ratchet engine (`crates/vox-doc-pipeline/src/claims/gate.rs`) that only permits shrinking the baseline via `--tighten-baseline` when deficits are fixed, never expanding.
   - Added to `.gitattributes` under `linguist-generated=true -diff` to prevent PR noise.
2. **Expiring Suppressions System (`contracts/reports/docs-freshness/suppressions.v1.json`, `crates/vox-doc-pipeline/src/claims/suppressions.rs`)**:
   - Strict requirement for `owner`, `reason`, and RFC 3339 / ISO 8601 date `expires_after`.
   - Gate validator fails immediately if any suppression is expired or lacks required fields.
3. **Drift Gate Evaluator (`crates/vox-doc-pipeline/src/claims/gate.rs`)**:
   - `evaluate_drift_gate` isolates reader-facing `status: current` documents.
   - Blocks any new unsuppressed missing or retired code mentions.
   - Reports baselined active deficits, resolved deficits, and expired suppressions.
4. **CLI Integration (`crates/vox-cli/src/commands/audit.rs`)**:
   - `vox audit docs --check-drift`: Evaluates drift gate, returns exit 0 on clean/baselined, exit 1 on gate failure.
   - `vox audit docs --generate-baseline`: Generates initial grandfathered baseline.
   - `vox audit docs --tighten-baseline`: Shrinks baseline by pruning resolved deficits.
   - `vox audit docs --attest <path>`: Re-stamps page in `ledger.v1.json` with git HEAD commit and verified state.
5. **Integration Test (`crates/vox-cli/tests/docs_drift_gate.rs`)**:
   - Verified gate accepts baselined deficits, blocks new unsuppressed deficits, unblocks with valid suppressions, rejects expired suppressions, and correctly tightens baseline when fixed.

## Verification
- CLI test: `cargo test -p vox-cli --test docs_drift_gate` passed (5/5 tests ok).
- Pipeline test: `cargo test -p vox-doc-pipeline` passed (37/37 tests ok).
- Gate check: `cargo run -p vox-cli -- audit docs --check-drift` passed cleanly.
- Attestation check: `cargo run -p vox-cli -- audit docs --attest docs/src/contributors/contributor-hub.md` re-stamped page to `verified` with HEAD commit.
- Architecture check: `cargo run -p vox-arch-check` passed with 0 errors.
