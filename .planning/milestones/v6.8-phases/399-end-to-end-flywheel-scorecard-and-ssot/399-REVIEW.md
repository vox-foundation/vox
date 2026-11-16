---
phase: 399-end-to-end-flywheel-scorecard-and-ssot
reviewed: 2026-11-10T08:38:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - crates/vox-repository/src/repo_catalog/federation.rs
  - crates/vox-repository/tests/federation_tests.rs
  - crates/vox-drift-check/src/extractors/vox.rs
  - crates/vox-orchestrator-mcp/src/introspection_tools.rs
  - crates/vox-orchestrator-mcp/src/attention_policy.rs
  - crates/vox-cli/src/commands/mod.rs
  - crates/vox-cli/src/commands/ci/run_body_helpers/mod.rs
  - crates/vox-cli/src/commands/ci/run_body_helpers/hash.rs
  - crates/vox-cli/src/commands/db_research/mod.rs
  - crates/vox-cli/src/commands/daemon_tool.rs
  - crates/vox-cli/src/commands/db_retention.rs
  - crates/vox-cli/src/commands/diagnostics/doctor/common.rs
  - /Users/brbrainerd/dev/vox-ops/crates/vox-ops-telemetry-ingest/src/validator.rs
  - /Users/brbrainerd/dev/vox-ops/crates/vox-plugin-cloud/src/lib.rs
findings:
  critical: 1
  warning: 4
  info: 1
  total: 6
status: resolved_via_auto_fix
---

# Milestone v6.7 & v6.8 (`Phases 387–399`): Code Review Report (`--fix`)

**Reviewed:** 2026-11-10T08:38:00Z
**Depth:** Standard + Auto-Fix (`--fix`)
**Files Reviewed:** 14
**Status:** All Critical & Warning findings resolved and verified (`status: resolved_via_auto_fix`)

## Summary

Reviewed the Milestone `v6.7` & `v6.8` defactoring, multi-repo federation (`vox-repository`), standalone `crates.io` extractor (`vox-drift-check`, `vox-code-audit`), MCP dispatch sharding (`vox-orchestrator-mcp`), CLI sub-crate split (`vox-cli`), and physical `vox-ops` repository (`vox-ops-telemetry-ingest`, `vox-plugin-cloud`). Found 1 Critical test/federation regression, 4 Warning issues (UTF-8 literal corruption, inconsistent `declared_repository_id` resolution, saturating arithmetic in telemetry clock-skew check, and 9 compiler warnings across `vox-orchestrator-mcp` & `vox-cli`), and 1 Info observation. All Critical and Warning issues were fixed in-place and verified with `cargo test`.

## Critical Issues (Fixed)

### CR-01: `federation_tests.rs` assumed `../vox-ops` was always absent (`skipped_optional_repositories`)

**File:** `crates/vox-repository/tests/federation_tests.rs:21, 57`
**Issue:** `test_verify_cross_repo_contract_parity_real_workspace` and `test_refresh_federated_symbol_graph` hardcoded `assert!(report.skipped_optional_repositories.contains(&"vox-ops".to_string()))` from Phase 391 (before `/Users/brbrainerd/dev/vox-ops` was physically created in Phase 394). Once `../vox-ops` exists on disk, `vox-ops` is indexed as a present repository (`4/4` present, `skipped_optional_repositories: []`), causing both tests to fail.
**Fix Applied:** Updated both tests to branch on `root.join("../vox-ops").exists()` so `cargo test -p vox-repository` passes deterministically both when `../vox-ops` is present (`present_repositories == 4`) and when `../vox-ops` is absent (`skipped_optional_repositories.contains("vox-ops")`).

## Warnings (Fixed)

### WR-01: `scan_vox_literals` corrupted multi-byte UTF-8 characters in `.vox` string literals

**File:** `crates/vox-drift-check/src/extractors/vox.rs:49-116`
**Issue:** When `VoxExtractor` was decoupled from `vox-compiler` to make `vox-drift-check` publishable on `crates.io`, `scan_vox_literals` iterated raw `bytes[i] as char` inside string literals, corrupting multi-byte UTF-8 sequences (such as `⌘` or `→`) into Latin-1 code points.
**Fix Applied:** Refactored `scan_vox_literals` to iterate via `src[i..].char_indices()` for both double- and single-quoted string literals and added a multi-byte UTF-8 unit test (`"hello ⌘K → world\n"`).

### WR-02: Inconsistent `repository_id` vs `declared_repository_id` in `refresh_federated_symbol_graph` edge builder

**File:** `crates/vox-repository/src/repo_catalog/federation.rs:532-540`
**Issue:** When constructing `shared_contract` edges in `refresh_federated_symbol_graph`, `r_a.repository_id` (which resolves to a 16-char hex digest of the git root) was preferred over `r_a.declared_repository_id` (`"vox"`, `"vox-ops"`, `"vox-axis"`, `"voxlang.org"`), whereas node indexing and `cargo_patch` edges preferred `declared_repository_id`.
**Fix Applied:** Standardized `id_a` and `id_b` to prefer `declared_repository_id` (when non-empty) before falling back to `repository_id` and `display_name`.

### WR-03: Unchecked addition in `TelemetryContractValidator::validate_event` clock-skew check

**File:** `/Users/brbrainerd/dev/vox-ops/crates/vox-ops-telemetry-ingest/src/validator.rs:100`
**Issue:** `current_epoch_ms + self.max_clock_skew_ms` used unchecked `+`, which could panic on overflow if a caller passed `u64::MAX` in a test or malformed clock reading.
**Fix Applied:** Changed to `current_epoch_ms.saturating_add(self.max_clock_skew_ms)`.

### WR-04: 9 compiler warnings across `vox-orchestrator-mcp` and `vox-cli` after Phase 388–389 splits

**Files:**
- `crates/vox-orchestrator-mcp/src/introspection_tools.rs:221` (unreachable `_` pattern on exhaustive `TaskPriority`)
- `crates/vox-orchestrator-mcp/src/attention_policy.rs:116` (unreachable `_` pattern on exhaustive `TaskPriority`)
- `crates/vox-cli/src/commands/diagnostics/doctor/common.rs:4` (unused `serde::Serialize` import)
- `crates/vox-cli/src/commands/{db_research/mod.rs,daemon_tool.rs,db_retention.rs,mod.rs,ci/run_body_helpers/mod.rs,ci/run_body_helpers/hash.rs}` (re-export and helper warnings after sub-crate extraction)
**Fix Applied:** Removed unreachable match arms and unused imports, and annotated intentional cross-crate facade re-exports.

## Info

### IN-01: `vox-code-audit` (`toestub`) OpenAI-compatible review client uses direct `reqwest` post-decoupling

**File:** `crates/vox-code-audit/src/review/client.rs:238-256`
**Note:** Documented with `// vox-arch-check: allow llm-egress` because `vox-code-audit` is a standalone Tier 3 `crates.io` package (`publish = true`) that cannot depend on unpublished workspace crates (`vox-actor-runtime` / `vox-config`).
