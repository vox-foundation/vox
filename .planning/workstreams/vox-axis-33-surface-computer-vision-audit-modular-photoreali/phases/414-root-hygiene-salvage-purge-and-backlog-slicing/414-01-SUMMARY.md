---
phase: 414-root-hygiene-salvage-purge-and-backlog-slicing
plan: 01
subsystem: repo-hygiene
tags: [hygiene, voxignore, vox-ml-cli, dependabot, commit-slices]
provides:
  - Clean root and sub-crate working tree free of scratch/salvage/CWD pollution
  - Workspace-anchored pipeline and hub dataset path resolution in vox-ml-cli
  - Verified link_checker.yml consolidation, 12 Dependabot PR dispositions, and 7-slice atomic commit manifest
affects:
  - .voxignore
  - .gitignore
  - .cursorignore
  - .aiignore
  - .aiexclude
  - crates/vox-ml-cli/src/commands/mens/pipeline.rs
  - crates/vox-ml-cli/src/commands/mens/hub.rs
  - contracts/reports/v71-atomic-commit-slices.v1.json
requirements-completed:
  - HYGIENE-ROOT-01
  - HYGIENE-ROOT-02
  - HYGIENE-ROOT-03
  - HYGIENE-ROOT-04
completed: 2026-11-13
---

# Phase 414 Plan 01: Root Hygiene, Salvage Purge, CWD Bug Fix & Backlog Slicing Summary

Purged all untracked scratch/salvage/CWD-polluted artifacts, fixed `vox-ml-cli`'s CWD-relative `mens/` path bug, documented `link_checker.yml` consolidation and all 12 open Dependabot PR dispositions, and generated `contracts/reports/v71-atomic-commit-slices.v1.json`.

## Key Changes

1. **Root, Salvage & CWD Clutter Purge (`HYGIENE-ROOT-01`):**
   - Removed `/Users/brbrainerd/dev/vox/file`, `test_syntax.vox`, `scratch_extract.vox`, `fix-harness-report.md`, `.remediation-notes/`, literal `~/` directory at repo root, `.vox/salvage/vox-mens-compare/`, and `crates/vox-ml-cli/mens/`.
   - Added `.vox/salvage/` and `.remediation-notes/` to `.voxignore` (and `.remediation-notes/` to `.gitignore`) and synchronized `.cursorignore`, `.aiignore`, and `.aiexclude` via `vox ci sync-ignore-files`.
2. **`vox-ml-cli` Workspace-Anchored Path Resolution (`HYGIENE-ROOT-02`):**
   - Added `PipelineDataPaths::resolve` in `crates/vox-ml-cli/src/commands/mens/pipeline.rs` so all generated/mix outputs (`synthetic.jsonl`, `research-lane-sft.jsonl`, `mix_sources/*`) anchor to `data_dir` (and relative `data_dir`/`output_dir` normalize to workspace root rather than sub-crate CWD).
   - Normalized `HubAction::Dataset` `stage_dir` in `crates/vox-ml-cli/src/commands/mens/hub.rs` via `resolve_from_workspace`.
   - Verified with `cargo test -p vox-ml-cli --lib pipeline` (`12 passed; 0 failed`) and confirmed `crates/vox-ml-cli/mens` is not recreated.
3. **`link_checker.yml` & 12 Dependabot PR Dispositions + Atomic Commit Slices (`HYGIENE-ROOT-03`, `HYGIENE-ROOT-04`):**
   - Verified `.github/workflows/link_checker.yml` is superseded without coverage loss by `.github/workflows/docs-quality.yml` (blocking internal link gate + self-test) and `.github/workflows/docs-drift-refresh.yml` (external link triage via `scripts/lychee-triage.vox`).
   - Triaged all 12 open Dependabot PRs (`6` ready-to-merge minor/patch grouped PRs `#721, #720, #703, #698, #693, #692`; `6` major-bump holds `#702, #701, #699, #697, #695, #694`) and emitted the 7-slice topological commit manifest in `contracts/reports/v71-atomic-commit-slices.v1.json`.
