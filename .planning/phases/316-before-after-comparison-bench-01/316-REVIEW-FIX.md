---
phase: 316-before-after-comparison-bench-01
fixed_at: 2026-10-29T01:45:52Z
review_path: .planning/phases/316-before-after-comparison-bench-01/316-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 316: Code Review Fix Report

**Fixed at:** 2026-10-29T01:45:52Z
**Source review:** `.planning/phases/316-before-after-comparison-bench-01/316-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (WR-01 through WR-04; no critical findings)
- Fixed: 4
- Skipped: 0

Verification ran in the main checkout (`workflow.use_worktrees` is false). No isolated worktree was created. Targeted tests only; no model weights were downloaded and no scores were invented.

## Fixed Issues

### WR-01: `--out` containment is lexical, so `..` and a symlinked parent still write into the hub or adapter

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`
**Commit:** a39b1ed12
**Status:** fixed: requires human verification
**Applied fix:** `check_out_path` now collapses `.` and `..`, then canonicalizes the nearest existing ancestor of `--out`, the hub root, and the adapter directory before the prefix check. New tests refuse a `..` path and a symlinked parent that land inside either directory, and still allow a `..` path that leaves the hub. `cargo test -p vox-ml-cli --features gpu --lib commands::mens::reasoning_compare::tests::out_` passed (4 tests).

### WR-02: An incomplete snapshot is reported as not on disk

**Files modified:** `crates/vox-eval/src/reasoning_bench.rs`
**Commit:** 02c4fe2d2
**Status:** fixed: requires human verification
**Applied fix:** `snapshot_on_disk` is true for `BaseSnapshotIncomplete` as well as the adapter-stage reasons. `BaseSnapshotAbsent` and `PinUnverifiable` stay false. `cargo test -p vox-eval --lib reasoning_bench::tests::` passed (24 tests).

### WR-03: Detection can mark an adapter Ready when the loader cannot open it, and the base is scored first

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`, `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`, `crates/vox-eval/src/reasoning_bench.rs`
**Commit:** 28e988205
**Status:** fixed: requires human verification
**Applied fix:** A manifest that matches the pinned snapshot is `AdapterIncomplete` unless `tokenizer.json` and `config.json` are files in the adapter directory. Ready fixtures now include those files. A matching pair that lacks either file stays on the absence path and does not call the scorer. `cargo test -p vox-ml-cli --features gpu --lib commands::mens::reasoning_compare::` and `cargo test -p vox-eval --lib reasoning_bench::tests::` passed (55 tests across the two suites).

### WR-04: The dangling-shard test calls `std::os::unix` with no Unix cfg

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`
**Commit:** f7bac6a4b
**Status:** fixed
**Applied fix:** `snapshot_with_dangling_shard_link_is_incomplete` is gated with `#[cfg(unix)]`, so `std::os::unix::fs::symlink` is not compiled into the test module on Windows. The test still passed on this macOS host.

## Out of scope

Info findings were not in `fix_scope` (`critical_and_warning`) and were left unchanged:

- **IN-01** — how-to filename still says `<size>-<revision>.json` rather than the first 12 hex characters.
- **IN-02** — a failed comparison's process error still drops the `NotScored` reason text.
- **IN-03** — index shard checks still allow non-`Normal` components and skip non-string `weight_map` values.

---

_Fixed: 2026-10-29T01:45:52Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
