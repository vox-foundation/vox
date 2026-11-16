---
phase: 403-public-rollout-path-portability-hardcoded-abs-dir-guard
reviewed_at: 2026-11-12T20:24:00-07:00
status: clean
reviewers:
  - vox-code-audit
  - vox-drift-check
  - vox-doc-pipeline
---

# Phase 403 Code Review Ledger (`403-REVIEW.md`)

| Finding ID | Target | Issue Discovered | Resolution & Verification |
|---|---|---|---|
| `REV-403-01` | `crates/vox-drift-check/src/extractors/rust.rs` & `cache.rs` | `RustExtractor` previously extracted `#[doc = "..."]` attributes as `LiteralContext::Code` string literals and did not track `#[cfg(test)]`/`#[test]` scopes for `LiteralContext::Test`, causing false-positive findings on doc examples and unit-test synthetic paths (`/home/alice`, `/Users/bob`). | Added `visit_attribute` `doc` skip and `in_test_depth` tracking across `#[cfg(test)]` modules, `#[test]`/`#[*::test]` fns, and `minimal_repro` fns; bumped `CACHE_VERSION` from `2` to `3` in `cache.rs`. Verified `3/3` `rules::hardcoded_abs_dir` tests pass and `./target/debug/vox-drift-check --severity error --fail-on error` exits `0`. |
| `REV-403-02` | `crates/vox-drift-check/src/extractor.rs` & `crates/vox-telemetry/src/config.rs` | `crates/vox-telemetry/src/config.rs` Windows `APPDATA`-unset fallback `r"C:\Users\Default\AppData\Roaming\vox..."` already carried `// vox-arch-check: allow abs-path`, which `parse_drift_allow_comments` did not recognize. | Extended `parse_drift_allow_comments` in `extractor.rs` to treat `vox-arch-check: allow abs-path` as allowing `hardcoded-abs-dir`, and added explicit `drift-allow(hardcoded-abs-dir)` annotations in `crates/vox-telemetry/src/config.rs`. |
| `REV-403-03` | `crates/vox-code-audit/src/detectors/magic_value.rs` & `hardcoded_abs_dir.rs` | Adding a full-workspace regex detector per line would add hot-path latency to `toestub` and `lefthook.yml` `pre-commit`. | Gated `HardcodedAbsDirDetector` behind O(1) file/line byte pre-filters (`contains_abs_dir_marker`) and eliminated 3 per-line regexes (`comment_re`, `const_def_re`, `test_attr_re`) + added byte pre-gates in `MagicValueDetector`. Verified `7/7` `detectors::hardcoded_abs_dir` unit tests pass (`0.02s`). |
