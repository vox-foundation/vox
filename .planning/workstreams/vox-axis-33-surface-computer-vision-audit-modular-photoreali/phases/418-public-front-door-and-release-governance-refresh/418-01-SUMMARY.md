---
phase: 418
plan: 01
subsystem: release-governance
tags: [release-notes, changelog, contributing, doc-pipeline, readiness-scorecard]
requires:
  - HYGIENE-TEST-03
provides:
  - HYGIENE-DOCS-01
  - HYGIENE-DOCS-02
  - HYGIENE-DOCS-03
  - HYGIENE-DOCS-04
affects:
  - RELEASE_NOTES.md
  - CHANGELOG.md
  - CONTRIBUTING.md
  - contracts/reports/v71-public-readiness-scorecard.v1.json
tech-stack:
  added: []
  patterns:
    - "Front-door release governance aligned with live workspace crates and `vox-doc-pipeline` frontmatter rules"
    - "Machine-readable public readiness scorecard (`v71-public-readiness-scorecard.v1.json`) capturing exit-0 receipts across all CI/audit gates"
key-files:
  created:
    - contracts/reports/v71-public-readiness-scorecard.v1.json
    - .planning/phases/418-public-front-door-and-release-governance-refresh/418-CONTEXT.md
    - .planning/phases/418-public-front-door-and-release-governance-refresh/418-01-PLAN.md
    - .planning/phases/418-public-front-door-and-release-governance-refresh/418-01-SUMMARY.md
    - .planning/phases/418-public-front-door-and-release-governance-refresh/418-REVIEW.md
    - .planning/phases/418-public-front-door-and-release-governance-refresh/418-VERIFICATION.md
    - .planning/v7.1-MILESTONE-AUDIT.md
  modified:
    - RELEASE_NOTES.md
    - CHANGELOG.md
    - CONTRIBUTING.md
key-decisions:
  - "Replaced obsolete Feb 2026 deleted-crate notes in `RELEASE_NOTES.md` with verified `0.6.0` (`v7.1`) capabilities and explicit maturity tiers."
  - "Removed forbidden manual `last_updated` frontmatter from `CHANGELOG.md` and recorded `v6.7`–`v7.1` milestones under `[Unreleased]`."
  - "Updated `CONTRIBUTING.md` to guide contributors onto scoped `cargo check -p <crate>`, `.vox` script automation (`vox run`), `vox ci pre-push`, and companion audit/doc binaries."
requirements-completed:
  - HYGIENE-DOCS-01
  - HYGIENE-DOCS-02
  - HYGIENE-DOCS-03
  - HYGIENE-DOCS-04
completed: 2026-11-13
---

# Phase 418 Plan 01 Summary: Public Front-Door & Release Governance Refresh

## Delivered Outcomes
1. **`RELEASE_NOTES.md` Rewritten for `0.6.0` (`HYGIENE-DOCS-01`):** Purged all Feb 2026 deleted-crate references (`vox-parser`, `vox-typeck`, `vox-codegen-rust`, `vox-codegen-ts`, `vox-hir`, `vox-integration-tests`) and documented the unified `vox-compiler`/`vox-codegen` pipeline, `--interp` capability sandbox, `vox-db` + `vox kb` Library index, `vox-orchestrator` + 32-surface Vox Axis workbench, `vox mens` Candle QLoRA plugins, 33 standalone `crates.io` packages, and maturity tiers.
2. **`CHANGELOG.md` & `CONTRIBUTING.md` Refreshed (`HYGIENE-DOCS-02`):** Removed forbidden `last_updated` frontmatter from `CHANGELOG.md`, added `v6.7`–`v7.1` entries under `[Unreleased]`, and updated `CONTRIBUTING.md` with scoped `-p <crate>` Cargo commands, `.vox` automation, and companion audit/doc binaries (`vox-doc-pipeline`, `toestub`, `vox-drift-check`, `vox-arch-check`).
3. **Doc Lint & 33-Crate Distribution Parity Verified (`HYGIENE-DOCS-03`):** `vox-doc-pipeline --lint-only` exited `0` (`no hard errors`), deleted-crate/frontmatter grep returned `0` matches, and `cargo test --locked -p voxup` passed `111/111` tests verifying all `33` standalone crates in `crates/_public.toml`.
4. **End-to-End Gate Sweep & Scorecard Emitted (`HYGIENE-DOCS-04`):** Verified `exit 0` across `vox ci pre-push`, `vox-arch-check`, `vox ci fan-in-budget`, `vox ci secret-env-guard`, `vox ci crate-edges`, `vox ci ssot-drift`, `vox ci cross-repo-parity`, `vox ci federated-symbol-graph`, `vox ci ignored-test-age --mode enforce`, `vox-drift-check`, and `toestub` (`arch/workspace_drift`), recorded in `contracts/reports/v71-public-readiness-scorecard.v1.json`.
