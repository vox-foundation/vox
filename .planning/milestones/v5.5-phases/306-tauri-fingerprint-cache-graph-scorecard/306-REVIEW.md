---
phase: 306-tauri-fingerprint-cache-graph-scorecard
reviewed: 2026-10-26T01:30:00Z
depth: standard
scope: v5.4 phases 301-306 (milestone-wide; no phase arg → phase 306 artifact)
files_reviewed: 28
files_reviewed_list:
  - crates/vox-cli-ci/src/check_changed.rs
  - crates/vox-cli-ci/src/evidence_asset_ban.rs
  - crates/vox-cli-ci/src/transfer_budget_report.rs
  - crates/vox-cli-ci/src/hit_every_surface.rs
  - crates/vox-cli-ci/src/suppression_review_gate.rs
  - crates/vox-cli-ci/src/knip_gui.rs
  - crates/vox-cli-ci/src/cmd_enums.rs
  - crates/vox-cli-ci/src/lib.rs
  - crates/vox-cli/src/commands/ci/run_body.rs
  - crates/vox-code-audit/src/detectors/thin_transport_bypass.rs
  - crates/vox-code-audit/src/detectors/unreasoned_suppression.rs
  - crates/vox-code-audit/src/detectors/mod.rs
  - crates/vox-code-audit/src/diagnostics/catalog.rs
  - contracts/ci/changed-path-owners.v1.yaml
  - contracts/ci/evidence-asset-allowlist.v1.yaml
  - contracts/ci/shard-weights.v1.json
  - contracts/reports/transfer-budget-ci.v1.json
  - contracts/reports/t3-vox-ci-enforcement-scorecard.v1.json
  - contracts/reports/t3-oxlint-toestub-mapping-ledger.v1.json
  - .github/workflows/pr-size.yml
  - .github/workflows/thread-transfer-report.yml
  - .github/workflows/gui-sidecar-fingerprint.yml
  - crates/vox-gui/ui/knip.jsonc
  - docs/src/ci/gui-cache-layers.md
  - docs/agents/kill-safety-and-hit-every-surface.md
  - docs/agents/suppression-review.md
  - docs/agents/worktree-isolation.md
  - docs/src/architecture/t3-vox-ci-toestub-enforcement-closure-2026.md
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 306: Code Review Report (v5.4 milestone)

**Reviewed:** 2026-10-26T01:30:00Z (America/Phoenix: 2026-10-25 18:30)
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Milestone-wide review of v5.4 CI/TOESTUB enforcement (phases 301–306). Core gates (`check-changed`, evidence-asset-ban, unreasoned-suppression, knip wiring, PR-size labeling) are structurally sound. Two Critical issues: thin-transport allowlist substring bypass, and transfer-budget CI claiming a live test source while always emitting a hard-coded fixture. Warnings cover checkout pin drift, fragile fingerprint base SHA, swallowed mkdir errors, and loose `**/` glob matching.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Thin-transport allowlist uses substring `contains`

**File:** `crates/vox-code-audit/src/detectors/thin_transport_bypass.rs:21-26`
**Issue:** `is_allowlisted` returns true when `p.contains(s)` for any allowlist suffix. A path that merely embeds an allowlisted suffix (e.g. `…/remote/gate.rs.bak`, `…/rpc/mod.rs_extra.ts`, or a crafted directory name) skips the detector entirely — a security-policy bypass for an Error-severity rule.
**Fix:** Match with `ends_with` only (path-normalized). Add a regression test that substring-near-miss paths are **not** allowlisted.

### CR-02: Transfer-budget gate is fixture theater mislabeled as live test source

**File:** `crates/vox-cli-ci/src/transfer_budget_report.rs:141-142` (also `contracts/reports/transfer-budget-ci.v1.json`)
**Issue:** `run()` always calls `build_report(fixture_sample())`. The committed artifact sets `source` to `vox-orchestrator-mcp/tests/transfer_budget.rs`, so `--check` / PR comments read as a live measurement ratchet while they only re-serialize constants + a hard-coded under-ceiling sample. Ceiling loosening is caught; real transfer regressions are not.
**Fix:** Relabel `source` to an honest fixture provenance string; document in scorecard that this is a ceiling-constant / fixture artifact (live delta deferred, matching existing Partial gem); update the trusted PR comment copy accordingly.

## Warnings

### WR-01: New workflows pin `actions/checkout@v4` while repo SSOT is `@v7`

**File:** `.github/workflows/pr-size.yml:53`, `gui-sidecar-fingerprint.yml:26`, `thread-transfer-report.yml:29`
**Issue:** Rest of `.github/workflows` uses `actions/checkout@v7`. New v5.4 workflows introduce pin drift.
**Fix:** Bump all three to `actions/checkout@v7`.

### WR-02: Sidecar fingerprint base uses `HEAD^1` with shallow fetch

**File:** `.github/workflows/gui-sidecar-fingerprint.yml:29-34`
**Issue:** `fetch-depth: 2` + `git rev-parse HEAD^1` is fragile on non-merge SHAs / shallow history. PR events already expose `github.event.pull_request.base.sha`.
**Fix:** Checkout with adequate history (or fetch base SHA) and diff against `github.event.pull_request.base.sha`.

### WR-03: `write_github_output` swallows mkdir failures

**File:** `crates/vox-cli-ci/src/check_changed.rs:298-299`
**Issue:** `std::fs::create_dir_all(parent).ok()` drops errors; subsequent `write` fails with a less useful message (or GITHUB_OUTPUT path silently missing in CI).
**Fix:** Propagate `create_dir_all` with context.

### WR-04: Evidence-ban `**/…` globs use bare `contains` without path-segment bounds

**File:** `crates/vox-cli-ci/src/evidence_asset_ban.rs:43-54`
**Issue:** `path_s.contains(mid)` / `contains(prefix)` can match accidental substrings (e.g. `my-review-bundle-notes`). Ban false-positives are safer than allow false-positives, but segment-bounded matching matches the documented glob semantics.
**Fix:** Require `/mid/` or boundary-equivalent segment contains.

## Info

### IN-01: Duplicate `glob_match` helpers

**File:** `check_changed.rs` vs `evidence_asset_ban.rs`
**Issue:** Two divergent glob implementations; drift risk.
**Fix:** Deferred (defactor / shared helper later); not in `--fix` default scope.

### IN-02: Advisory gates never fail unless `--strict`

**File:** `hit_every_surface.rs`, `suppression_review_gate.rs`
**Issue:** By design for v5.4; scorecard already marks Partial where relevant.
**Fix:** None required.

### IN-03: `knip-gui` falls back to `pnpm dlx knip@5`

**File:** `crates/vox-cli-ci/src/knip_gui.rs:34-38`
**Issue:** Unpinned minor/patch when local binary missing; scorecard already notes lockfile pin deferred.
**Fix:** None required for this pass.
