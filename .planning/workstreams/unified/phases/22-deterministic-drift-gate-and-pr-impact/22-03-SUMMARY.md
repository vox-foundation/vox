---
phase: "22-deterministic-drift-gate-and-pr-impact"
plan: "03"
status: completed
date: "2026-10-13"
requirements_met:
  - DRIFT-01
  - DRIFT-07
---

# Plan 22-03 Summary: Reverse Index, PR Impact Comment & Definition of Done

## Execution Summary
Implemented the reverse code-to-docs index, PR impact comment generator with idempotency marker, and Docs Definition-of-Done (DoD) contract checker. Wired the drift gate and PR impact / DoD analysis into `.github/workflows/docs-quality.yml`.

## Delivered Components
1. **Reverse Code-to-Docs Index (`crates/vox-doc-pipeline/src/claims/reverse_index.rs`)**:
   - `ReverseIndex` structure indexing code targets, files, crates, CLI commands, and env vars against reader-facing `status: current` doc pages.
   - `query_impact` matches PR-changed files against anchors and code mentions.
   - Generates formatted Markdown report with idempotency marker `<!-- vox-docs-freshness-pr-impact -->`, table of affected pages and categories, and actionable re-attestation instructions.
2. **Docs Definition-of-Done Checker (`crates/vox-doc-pipeline/src/claims/dod.rs`)**:
   - `check_definition_of_done` performs deterministic checks on PR diffs:
     - `DOD-CLI-01`: Changes to `contracts/cli/command-registry.yaml` require CLI doc or generator updates.
     - `DOD-ENV-01`: Changes to `contracts/config/env-vars.v1.yaml` require environment variable reference updates.
     - `DOD-CONTRACT-01`: Changes to public contracts under `contracts/` require corresponding documentation updates under `docs/`.
3. **CLI Integration (`crates/vox-cli/src/commands/audit.rs`)**:
   - Supported both flag and subcommand action syntax:
     - `vox audit docs pr-impact` or `vox audit docs --pr-impact` with `--diff-base`, `--diff-head`, `--diff-out`, `--dry-run`.
     - `vox audit docs dod` or `vox audit docs --dod`.
   - Optimized path: runs PR impact / DoD checks in milliseconds without requiring a full corpus scan when only PR analysis is requested.
4. **CI Workflow Integration (`.github/workflows/docs-quality.yml`)**:
   - Added `pull-requests: write` permissions.
   - Added `Documentation Drift Gate` step running `vox audit docs --check-drift` to block PRs introducing new broken mentions.
   - Added `Docs PR Impact and Definition-of-Done` step on pull requests:
     - Analyzes diff against PR base SHA.
     - Creates or updates PR comment in place using the idempotency marker.
     - Emits deterministic DoD warnings on missing contract docs.

## Verification
- Unit & integration tests:
  - `cargo test -p vox-doc-pipeline` passed (41/41 tests ok).
  - `cargo test -p vox-cli --test docs_drift_gate` passed (5/5 tests ok).
- Architecture check:
  - `cargo run -p vox-arch-check` passed with 0 errors.
- CLI verification:
  - `cargo run -p vox-cli -- audit docs pr-impact --dry-run` generated valid Markdown table with marker.
  - `cargo run -p vox-cli -- audit docs dod --dry-run` surfaced expected `DOD-CLI-01` warning on modified command registry.
- Workflow syntax:
  - `git diff .github/workflows/docs-quality.yml` verified clean and compliant.
