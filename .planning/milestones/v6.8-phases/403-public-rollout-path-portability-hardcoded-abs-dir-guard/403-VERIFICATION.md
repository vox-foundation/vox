---
phase: 403-public-rollout-path-portability-hardcoded-abs-dir-guard
verified_at: 2026-11-12T20:30:10-07:00
status: passed
score: 4/4
---

# Phase 403 Verification Report (`403-VERIFICATION.md`)

**Goal:** Scrub machine-local hardcoded absolute directories across workspace scripts, GUI CV flywheel specs/reports (`vox` and `../vox-axis`), Rust tests, contracts, and active `docs/src/` files in favor of repo-relative or env-derived paths; enforce `vox/fs/hardcoded-abs-dir` (`toestub` / `vox-code-audit`) + `MagicValueDetector` pre-filter optimization; enforce `drift/hardcoded-abs-dir` (`vox-drift-check`) with `RustExtractor` doc/test context (`CACHE_VERSION = 3`); and gate staged commits in `lefthook.yml`.

## Verification Matrix

| Requirement | Description | Status | Evidence |
|---|---|---|---|
| `ABS-DIR-GUARD-01` | Scrub machine-local paths (`/Users/<user>`, `/home/<user>`, `C:\Users\<user>`, `.gemini/antigravity/brain/`, `/private/tmp/`) across scripts, GUI CV flywheel specs/reports (`vox` & `../vox-axis`), Rust tests, contracts, and 17 active `docs/src/` files while keeping synthetic unit-test fixtures intact. | PASSED | Full workspace `./target/debug/vox-drift-check --severity error --fail-on error` exited `0` (`✓ No drift findings at Error level or above.`); prior verification suites (`agent-guard.mjs`, `pnpm --dir crates/vox-gui/ui typecheck`, `vox-repository`, `vox-populi`, `vox-doc-pipeline`) clean. |
| `ABS-DIR-GUARD-02` | Add `HardcodedAbsDirDetector` (`vox/fs/hardcoded-abs-dir`) with O(1) file/line byte pre-filters and optimize `MagicValueDetector` by replacing 3 per-line regexes with `trim_start()` prefix checks and byte pre-gates. | PASSED | `timeout 900s cargo test -p vox-code-audit --lib detectors::hardcoded_abs_dir` → `7 passed; 0 failed` (`0.02s`). |
| `ABS-DIR-GUARD-03` | Add `HardcodedAbsDirRule` (`drift/hardcoded-abs-dir`), update `RustExtractor` (`visit_attribute` doc skip + `in_test_depth` tracking), honor `vox-arch-check: allow abs-path` / `drift-allow(hardcoded-abs-dir)`, and bump `CACHE_VERSION` to `3`. | PASSED | `timeout 900s cargo test -p vox-drift-check --lib rules::hardcoded_abs_dir` → `3 passed; 0 failed` (`0.01s`); `./target/debug/vox-drift-check --severity error --fail-on error` → exit `0`. |
| `ABS-DIR-GUARD-04` | Wire `vox/fs/hardcoded-abs-dir` into `lefthook.yml` `pre-commit` (`tdd-guard`) and document §3.5 in `docs/src/architecture/t3-vox-toestub-rules-and-enforcement-research-2026.md`. | PASSED | `timeout 900s cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3-vox-toestub-rules-and-enforcement-research-2026.md` → `vox-doc-pipeline lint complete — no hard errors.` |

## Commands Verified in Phase Closeout

1. `timeout 900s cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3-vox-toestub-rules-and-enforcement-research-2026.md` → exit `0` (`no hard errors`).
2. `timeout 900s cargo test -p vox-code-audit --lib detectors::hardcoded_abs_dir` → exit `0` (`7 passed; 0 failed`).
3. `timeout 900s cargo test -p vox-drift-check --lib rules::hardcoded_abs_dir` → exit `0` (`3 passed; 0 failed`).
4. `timeout 900s ./target/debug/vox-drift-check --severity error --fail-on error` → exit `0` (`✓ No drift findings at Error level or above.`).
