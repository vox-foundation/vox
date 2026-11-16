# Phase 403: Public Rollout Path Portability & Hardcoded Abs-Dir Guard - Context

**Gathered:** 2026-11-12
**Status:** Complete / Ready for Archival Verification
**Mode:** Autonomous execution (`ABS-DIR-GUARD-01..04`)

<domain>
## Phase Boundary

Ensure the Vox public rollout contains no machine-local hardcoded absolute filesystem directories across scripts, GUI CV flywheel harnesses/reports (`vox` and `../vox-axis`), Rust tests, contracts, or active documentation, while enforcing zero-extra-pass regression protection via `vox-code-audit` (`toestub`), `vox-drift-check`, and `lefthook.yml` `pre-commit` hooks without regressing scan performance.

</domain>

<decisions>
## Implementation Decisions

### ABS-DIR-GUARD-01: Portable Relative & Env-Derived Path Rollout
- Scrub machine-local absolute paths (`/Users/<user>`, `/home/<user>`, `C:\Users\<user>`, `C:/Users/<user>`, `.gemini/antigravity/brain/`, `/private/tmp/`, `/private/var/folders/`) across workspace scripts, GUI CV flywheel harnesses & checked-in artifacts (`vox` and `../vox-axis`), Rust tests (`std::env::temp_dir()`), contracts, and 17 active `docs/src/` files.
- Preserve synthetic cross-OS test fixtures (`/home/alice`, `/Users/bob`, `C:\Users\bob`) inside unit test contexts so cross-platform path-handling coverage remains intact.

### ABS-DIR-GUARD-02: TOESTUB `vox/fs/hardcoded-abs-dir` + Hot-Path Performance
- Add `HardcodedAbsDirDetector` (`crates/vox-code-audit/src/detectors/hardcoded_abs_dir.rs`, rule ID `vox/fs/hardcoded-abs-dir` in `crates/vox-code-audit/src/diagnostics/catalog.rs`, registered in `crates/vox-code-audit/src/detectors/mod.rs`).
- Use O(1) file-level and line-level byte pre-filters (`/Users/`, `/home/`, `Users\`, `Users/`, `.gemini/antigravity/brain/`, `/private/tmp/`, `/private/var/folders/`, `:/`) so clean files skip allocation/regex work completely.
- Tier 1 (`Severity::Error`): machine-local home/scratch directories across Rust, TypeScript, Python, GDScript, and Vox (with synthetic test-user exemptions limited to test contexts).
- Tier 2 (`Severity::Warning`): unanchored OS root paths in non-test/non-const production code.
- Optimize `MagicValueDetector` (`crates/vox-code-audit/src/detectors/magic_value.rs`) by replacing 3 per-line regex checks (`comment_re`, `const_def_re`, `test_attr_re`) with `trim_start()` prefix checks and gating `port_rule`, `ip_rule`, `path_rule`, and `db_conn_rule` behind cheap byte checks (`:`, `.`, `/`, `\`).

### ABS-DIR-GUARD-03: Incremental AST/Token `vox-drift-check` `drift/hardcoded-abs-dir`
- Add `HardcodedAbsDirRule` (`drift/hardcoded-abs-dir`) in `crates/vox-drift-check/src/rules/hardcoded_abs_dir.rs` and register it in `crates/vox-drift-check/src/rules/mod.rs`.
- Update `RustExtractor` (`crates/vox-drift-check/src/extractors/rust.rs`) to skip `#[doc]` attributes in `visit_attribute` (preventing doc comments from being misclassified as `LiteralContext::Code` string literals) and track `in_test_depth` on `#[cfg(test)]` modules, `#[test]`/`#[*::test]` fns, and `minimal_repro` fns for `LiteralContext::Test`.
- Teach `parse_drift_allow_comments` in `crates/vox-drift-check/src/extractor.rs` to honor `// vox-arch-check: allow abs-path` for `hardcoded-abs-dir`.
- Annotate intentional Windows `APPDATA`-unset fallback `C:\Users\Default\...` literals in `crates/vox-telemetry/src/config.rs` with `drift-allow(hardcoded-abs-dir)`.
- Bump `CACHE_VERSION` from `2` to `3` in `crates/vox-drift-check/src/cache.rs` so cached literal contexts recompute with the updated `RustExtractor`.

### ABS-DIR-GUARD-04: Staged Pre-Commit Gate & Architecture SSOT
- Wire `vox/fs/hardcoded-abs-dir` into `lefthook.yml` `pre-commit` (`tdd-guard`: `toestub {staged_files} --rules skeleton,vox/fs/hardcoded-abs-dir` across `.rs,vox,ts,tsx,js,mjs,py`).
- Document the detector and its performance design in `docs/src/architecture/t3-vox-toestub-rules-and-enforcement-research-2026.md`.

</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-code-audit` (`toestub`) and `crates/vox-drift-check` already run as incremental static analysis and pre-commit gates, making zero-extra-pass integration inside their single-pass visitors faster and cleaner than standalone regex shell scripts.

</code_context>

<specifics>
## Specific Ideas

- Combine file-level byte scans (`contains_abs_dir_marker`) with AST/context-aware `LiteralContext` classification in `vox-drift-check` so CI stays fast (~4s workspace sweep) and false positives on doc comments and synthetic unit-test fixtures are eliminated.

</specifics>

<deferred>
## Deferred Ideas

None.

</deferred>
