---
phase: 403-public-rollout-path-portability-hardcoded-abs-dir-guard
plan: 01
subsystem: code-audit-and-drift-check
tags: [toestub, vox-code-audit, vox-drift-check, portability, lefthook]
completed: 2026-11-12
---

# Phase 403 Plan 01 Summary: Public Rollout Path Portability & Hardcoded Abs-Dir Guard

## Accomplishments

1. **Portable Relative & Env-Derived Path Rollout (`ABS-DIR-GUARD-01`):**
   - Scrubbed machine-local hardcoded absolute paths (`/Users/<user>`, `/home/<user>`, `C:\Users\<user>`, `.gemini/antigravity/brain/`, `/private/tmp/`, `/private/var/folders/`) across workspace scripts, GUI CV flywheel specs & checked-in reports (`vox` and `../vox-axis`), Rust tests (`std::env::temp_dir()`), contracts, and 17 active `docs/src/` files.
   - Preserved synthetic cross-OS test fixtures (`/home/alice`, `/Users/bob`, `C:\Users\bob`) inside unit-test contexts.
2. **TOESTUB `vox/fs/hardcoded-abs-dir` Detector & Hot-Path Optimization (`ABS-DIR-GUARD-02`):**
   - Added `FS_HARDCODED_ABS_DIR = "vox/fs/hardcoded-abs-dir"` in `crates/vox-code-audit/src/diagnostics/catalog.rs` and `HardcodedAbsDirDetector` in `crates/vox-code-audit/src/detectors/hardcoded_abs_dir.rs` (registered in `crates/vox-code-audit/src/detectors/mod.rs`, `rule_count() == 67`).
   - Added O(1) file-level and line-level byte pre-filters so clean files skip allocation/pattern scanning; classified machine-local home/scratch paths as `Severity::Error` (Tier 1) and unanchored OS root paths in non-test/non-const production code as `Severity::Warning` (Tier 2).
   - Optimized `MagicValueDetector` (`crates/vox-code-audit/src/detectors/magic_value.rs`) by replacing 3 per-line regexes (`comment_re`, `const_def_re`, `test_attr_re`) with `trim_start()` prefix checks and gating `port_rule`/`ip_rule`/`path_rule`/`db_conn_rule` behind byte checks.
3. **Incremental AST/Token `vox-drift-check` `drift/hardcoded-abs-dir` (`ABS-DIR-GUARD-03`):**
   - Added `HardcodedAbsDirRule` (`drift/hardcoded-abs-dir`) in `crates/vox-drift-check/src/rules/hardcoded_abs_dir.rs` and registered it in `crates/vox-drift-check/src/rules/mod.rs`.
   - Updated `RustExtractor` (`crates/vox-drift-check/src/extractors/rust.rs`) to skip `#[doc]` attributes in `visit_attribute` and track `in_test_depth` for `#[cfg(test)]`, `#[test]`, and `minimal_repro` scopes (`LiteralContext::Test`).
   - Updated `crates/vox-drift-check/src/extractor.rs` to honor `// vox-arch-check: allow abs-path` comments, bumped `CACHE_VERSION` from `2` to `3` in `crates/vox-drift-check/src/cache.rs`, and annotated Windows `APPDATA`-unset fallback `C:\Users\Default\...` literals in `crates/vox-telemetry/src/config.rs` with `drift-allow(hardcoded-abs-dir)`.
4. **Staged Pre-Commit Gating & Architecture SSOT (`ABS-DIR-GUARD-04`):**
   - Wired `vox/fs/hardcoded-abs-dir` into `lefthook.yml` `pre-commit` (`tdd-guard`: `toestub {staged_files} --rules skeleton,vox/fs/hardcoded-abs-dir` across `.rs,vox,ts,tsx,js,mjs,py`).
   - Documented `HardcodedAbsDirDetector` and `drift/hardcoded-abs-dir` in `docs/src/architecture/t3-vox-toestub-rules-and-enforcement-research-2026.md` (§3.5).

## Verification Results

- `timeout 900s cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3-vox-toestub-rules-and-enforcement-research-2026.md`: `vox-doc-pipeline lint complete — no hard errors.`
- `timeout 900s cargo test -p vox-code-audit --lib detectors::hardcoded_abs_dir`: `7 passed; 0 failed` (`0.02s`).
- `timeout 900s cargo test -p vox-drift-check --lib rules::hardcoded_abs_dir`: `3 passed; 0 failed` (`0.01s`).
- `timeout 900s ./target/debug/vox-drift-check --severity error --fail-on error`: exit `0` (`✓ No drift findings at Error level or above.`).
