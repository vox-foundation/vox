---
phase: 316-before-after-comparison-bench-01
reviewed: 2026-10-29T01:48:39Z
depth: deep
files_reviewed: 8
files_reviewed_list:
  - crates/vox-eval/src/reasoning_bench.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
  - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
  - docs/src/how-to/how-to-train-mens-macos-metal.md
findings:
  critical: 0
  warning: 0
  info: 3
  total: 3
status: clean
---

# Phase 316: Code Review Report

**Reviewed:** 2026-10-29T01:48:39Z
**Depth:** deep
**Files Reviewed:** 8
**Status:** clean

## Summary

Re-reviewed the four warnings from the 2026-10-29T01:32:32Z review against the current source. Each is fixed. Info notes stay as non-blocking observations.

- **WR-01** — `containment_path` collapses `.` and `..`, then canonicalizes the nearest existing ancestor of `--out`, the hub root, and the adapter dir before `starts_with` (`reasoning_compare.rs:52-129`). `..` into the hub or adapter is refused, a `..` that leaves the hub is allowed, and a symlinked parent is refused. The symlink test is `#[cfg(unix)]`.
- **WR-02** — `snapshot_on_disk` is true for `BaseSnapshotIncomplete` and the adapter-stage reasons, and false for `BaseSnapshotAbsent` and `PinUnverifiable` (`reasoning_bench.rs:278-284`).
- **WR-03** — A manifest that matches the pin is `AdapterIncomplete` unless `tokenizer.json` and `config.json` are files in the adapter directory (`detect.rs:111-125`). `NotCompared` returns before either side is scored (`reasoning_compare.rs:227-231`). The loader still reads both files (`inference.rs:178`, `inference.rs:361`).
- **WR-04** — `snapshot_with_dangling_shard_link_is_incomplete` is gated with `#[cfg(unix)]` (`detect.rs:415-417`).

## Info

Non-blocking. Left unchanged by the fix pass.

### IN-01: The how-to names a longer revision than the file that is written

**File:** `docs/src/how-to/how-to-train-mens-macos-metal.md:93`
**Issue:** The paragraph says the report is `mens/runs/reasoning_compare/<size>-<revision>.json`. `BaseLabel::report_file_name` uses the first 12 hex characters (`Qwen3-8B-b968826d9c46.json`, `reasoning_bench.rs:214-218`). An operator looking for the full 40-character revision in the filename will not find the file.
**Fix:** Say `<size>-<first 12 hex of revision>.json`, and mention the `-unpinned` suffix for a pin with no 40-hex revision.

### IN-02: A failed comparison's process error drops the NotScored reason

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:287-294`
**Issue:** After the incomplete report is written, `first_unscored` returns `"{slice} base not_scored"` or `"{slice} adapter not_scored"`. The load or inference text is only inside that slice entry. A base `load_model` failure therefore surfaces as `gsm8k_think base not_scored`.
**Fix:** Include `reason` from the `NotScored` variant in the `anyhow` error. Keep the slice name so the existing assertions still match.

### IN-03: Index shard checks are narrower than the comment claims

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs:177-182`
**Issue:** `shard_is_file` rejects absolute paths and `..`. It does not reject `Component::RootDir` or `Component::Prefix`. On Windows, a root-relative shard name such as `\outside\model.safetensors` is not `is_absolute` and joins out of the snapshot, so an index with no in-snapshot shard can still look complete. Non-string `weight_map` values are also dropped (`filter_map(as_str)` at line 167); a map whose only real file entry exists is complete even when other entries are not strings. Neither case marks the report `compared` by itself: the loader globs `*model*.safetensors` inside the snapshot and a bad load stays `NotScored`. On macOS, `/etc/passwd` is absolute and is already rejected.
**Fix:** Accept a shard only when every component is `Normal` (and optionally `CurDir`). Treat a non-string `weight_map` value as incomplete instead of skipping it.

---

_Reviewed: 2026-10-29T01:48:39Z_
_Reviewer: gsd-code-reviewer_
_Depth: deep_
