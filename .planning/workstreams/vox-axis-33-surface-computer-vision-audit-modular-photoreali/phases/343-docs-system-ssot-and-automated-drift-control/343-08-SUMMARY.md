---
phase: 343-docs-system-ssot-and-automated-drift-control
plan: 08
subsystem: ci-docs-links
tags: [lychee-triage, link-checker-retirement, docs-self-heal, pre-push]
requires:
  - phase: 343-07
    provides: weekly docs-drift-refresh.yml workflow
provides:
  - Sorted non-blocking `scripts/lychee-triage.vox` triage script with `--self-test`
  - Retired blocking `.github/workflows/link_checker.yml` and removed write job `stage-draft-prs` from `.github/workflows/docs-self-heal.yml`
  - Updated 7 operator/CI docs and `ACT_WORKFLOWS` (`pre_push.rs`, `ci_pre_push.rs`) to match reality
affects: [343-09]
completed: 2026-11-13
---

# Phase 343 Plan 08: Lychee Triage, Link Checker Retirement & Docs Sync Summary

Applied the `option-a` decision across `lychee-triage.vox`, `docs-self-heal.yml`, `link_checker.yml`, and all downstream CI/docs references.

## Changes Made
- **Lychee triage (`scripts/lychee-triage.vox`):** Fixed `Option` handling (`.is_none()`/`.is_some()`/`.unwrap_or(...)`), depth-1 JSON `object_keys`, and deterministic `str_lt` sorting for `external-links.v1.json` and 403/429/5xx `.lycheeignore` proposals.
- **Workflow retirement & read-only hardening:** Deleted `.github/workflows/link_checker.yml`, removed `stage-draft-prs` from `.github/workflows/docs-self-heal.yml`, updated `ACT_WORKFLOWS` in `crates/vox-cli/src/commands/ci/pre_push.rs` + `crates/vox-cli/tests/ci_pre_push.rs`, and added `link_checker_retired` and `self_heal_has_no_write_job` in `crates/vox-cli/tests/ci_workflow_contract.rs`.
- **Docs alignment (7 files + `scripts/docs-fix-drafts.vox`):** Updated `docs-reality-audit-program.md`, `openwiki-self-healing-docs-ssot-2026.md`, `workflow-enumeration.md`, `rcicd-coverage-cost-matrix-2026.md`, `alternatives-and-local-mirroring.md`, `detector-coverage-ledger.md`, and `docs-system-ssot.md`.

## Verification & Mutation Proof
- `./target/debug/vox check scripts/lychee-triage.vox` -> `Check passed with 0 warning(s)`; `./target/debug/vox run --interp scripts/lychee-triage.vox -- --self-test` -> `lychee-triage: self-test OK`.
- `./target/debug/vox check scripts/docs-fix-drafts.vox` -> `Check passed` (21 pre-existing warnings).
- `timeout 900s cargo test -p vox-cli --test ci_workflow_contract` -> `35 passed; 0 failed`.
- `timeout 900s cargo test -p vox-cli --test ci_pre_push pre_push_dry_run_act_lists_workflows` -> `1 passed; 0 failed`.
- `timeout 900s cargo test -p vox-cli-ci --lib workflow_` -> `33 passed; 0 failed`.
- Mutation proof: recreating empty `.github/workflows/link_checker.yml` caused `link_checker_retired` to fail (`0 passed; 1 failed`); deleting it restored `1 passed; 0 failed`.
- `vox-doc-pipeline --lint-only` passed across all 7 modified docs with zero hard errors.
