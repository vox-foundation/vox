---
phase: 343-docs-system-ssot-and-automated-drift-control
plan: 07
subsystem: ci-docs-drift
tags: [docs-freshness, ci-workflow, draft-pr, ledger, idempotency]
requires:
  - phase: 343-06
    provides: docs_orphan_pages and clean inbound link graph
provides:
  - Idempotent docs-reality ledger (`sample_ledger` fixed commit) and sorted findings (`write_json_if_changed`)
  - Non-blocking weekly `.github/workflows/docs-drift-refresh.yml` workflow opening draft PR on `bot/docs-drift-refresh`
  - `ci_workflow_contract::docs_drift_refresh_opens_draft_pr_only` guard (plus nightly-report update)
affects: [343-08, 343-09]
completed: 2026-11-13
---

# Phase 343 Plan 07: Idempotent Ledger Refresh & Draft-PR Workflow Summary

Implemented idempotent regeneration for docs-freshness/reality artifacts and wired the weekly non-blocking `Docs Drift Refresh` workflow (decision: `option-a` — opens `--draft` PR on `bot/docs-drift-refresh`, never auto-merged, strict 5-path allowlist).

## Changes Made
- **Idempotency (`crates/vox-doc-pipeline/src/claims/ledger.rs`, `crates/vox-cli-research/src/audit.rs`, `crates/vox-cli/src/commands/audit.rs`):** Fixed `sample_ledger` `last_verified_commit` determinism, sorted reality findings before write, and guarded writes with `write_json_if_changed`.
- **Weekly workflow (`.github/workflows/docs-drift-refresh.yml`, `.github/workflows/nightly-report.yml`):** Added top-level read-only permissions, pinned `actions/checkout` SHA (`persist-credentials: false`), ran `audit docs-reality --update-ledger`, `audit docs-reality`, non-blocking `lychee-action@v2` (`v0.24.2`, `fail: false`), `scripts/lychee-triage.vox`, strict 5-path staging allowlist, and `gh pr create --draft` on `bot/docs-drift-refresh`. Updated `nightly-report.yml` workflow list (`Docs Link Checker` -> `Docs Drift Refresh`).
- **Contract test (`crates/vox-cli/tests/ci_workflow_contract.rs`):** Added `docs_drift_refresh_opens_draft_pr_only`.

## Verification & Mutation Proof
- `timeout 900s cargo test -p vox-doc-pipeline --lib claims::ledger` -> `5 passed; 0 failed`.
- `timeout 900s cargo test -p vox-cli-research --lib audit::tests` -> `10 passed; 0 failed`.
- `timeout 900s cargo test -p vox-cli --test ci_workflow_contract` -> `35 passed; 0 failed`.
- Mutation proof: temporarily removing `--draft` in `docs-drift-refresh.yml` caused `docs_drift_refresh_opens_draft_pr_only` to fail (`0 passed; 1 failed`), and restoring `--draft` returned to `1 passed; 0 failed`.
