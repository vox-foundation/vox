# Phase 312 / v5.5 Milestone Code Review Report

**Scope:** Whole milestone v5.5 (Phases 307–312) — no phase arg; reviewed implementation sources since `3651662cf`.
**Depth:** standard
**Date:** 2026-10-25 (America/Phoenix)
**Branch:** `cursor/t3-deeper-extraction-v55-510f`

## Summary

| Severity | Count | Fixed |
|---|---:|---:|
| Critical | 2 | 2 |
| Warning | 3 | 3 |
| Info | 3 | 0 (deferred) |

## Critical Issues

### CR-01: Sidecar fingerprint hashed path names / `rg`, not blob contents
**File:** `.github/workflows/gui-sidecar-fingerprint.yml`
**Issue:** `hash_tree` piped `git ls-tree --name-only` and used `rg` (not on ubuntu-latest by default), so the "content hash" was largely path-list theater and brittle on runners.
**Fix:** Hash real blob contents for key manifests + Cargo.lock package slices + `git ls-tree` blob IDs; use `grep -E` instead of `rg`.

### CR-02: `vox/test/wall-clock` left `#[cfg(test)]` sticky for the rest of the file
**File:** `crates/vox-code-audit/src/detectors/test_wall_clock.rs`
**Issue:** After seeing `#[cfg(test)]`, production `Instant::now` later in the same file could false-positive.
**Fix:** Brace-depth-bounded cfg(test) scope + regression test `does_not_flag_production_after_cfg_test_module`.

## Warnings

### WR-01: `ci-changes` swallowed git failures
**File:** `crates/vox-cli-ci/src/ci_changes.rs`
**Fix:** Fail closed when `git diff` is non-zero; stop `unwrap_or_default()`.

### WR-02: Preview allowlist entries lacked required owner/reason
**File:** `crates/vox-cli-ci/src/preview_trust_audit.rs`
**Fix:** Reject allowlist rows with empty `owner` or `reason`.

### WR-03: Vouch action did not pin `vouched-file`
**File:** `.github/workflows/pr-vouch.yml`
**Fix:** Explicit `vouched-file: .github/VOUCHED.td` (matches action default; documents intent).

## Info (not fixed)

### IN-01: Live transfer upload still not a required `ci.yml` step (documented Partial).
### IN-02: `dom-test-env-ban` remains advisory (~230 legacy pragmas).
### IN-03: Surface-graph `inventory.vox` regen still deferred.

## Files reviewed

- `.github/workflows/{pr-vouch,gui-sidecar-fingerprint,thread-transfer-report}.yml`
- `.github/VOUCHED.td`
- `crates/vox-cli-ci/src/{vouched_parse,preview_trust_audit,ci_changes,dom_test_env_ban,transfer_budget_report,cmd_enums,lib}.rs`
- `crates/vox-cli/src/commands/ci/run_body.rs`
- `crates/vox-code-audit/src/detectors/{test_wall_clock,test_in_loop,host_runtime_probe,inline_schema_compile,mod}.rs`
- `crates/vox-code-audit/src/diagnostics/catalog.rs`
