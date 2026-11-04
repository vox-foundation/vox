---
phase: 316-before-after-comparison-bench-01
fixed_at: 2026-10-27T00:00:00Z
review_path: .planning/phases/316-before-after-comparison-bench-01/316-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 6
skipped: 1
status: partial
---

# Phase 316: Code Review Fix Report

**Fixed at:** 2026-10-27
**Source review:** .planning/phases/316-before-after-comparison-bench-01/316-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope (critical + warning): 7
- Fixed: 6
- Skipped (deferred): 1 (WR-03)
- Info items fixed as one-liners: IN-01, IN-07. Other info items deferred.

**Where verification ran:** in the `gsd/mens-compare-316` checkout at `/Users/brbrainerd/dev/vox-mens-compare`. `workflow.use_worktrees` is `false`, so no nested review-fix worktree was made. Final state:
- `cargo nextest run -p vox-ml-cli --features gpu --lib reasoning_compare`: 33/33 passed (28 before the fixes).
- `cargo nextest run -p vox-eval reasoning_bench`: 23/23 passed.
- `cargo clippy -p vox-ml-cli --features gpu --all-targets`: no findings in the touched files.
- `cargo check -p vox-ml-cli` with default features (no `gpu`) builds clean, which covers the WR-01 arm.
- Touched files were formatted with `rustfmt --edition 2024`.

## Fixed Issues

### CR-01: `--out` guard is lexical

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`
**Commit:** b15479690
**Applied fix:** `normalized` (lexical) is replaced by `resolved`. It walks the absolute path one component at a time and canonicalizes every prefix that exists. That resolves symlinks, `..` and the on-disk letter case. A `..` after a missing directory is popped the same way `create_dir_all` would handle it. A path entry that exists but will not canonicalize (a dangling symlink) is refused. The hub root and the adapter dir go through the same resolution. An `--out` that is itself a symlink is refused. The gate-name and `collateral_eval` checks now run on both the given path and the resolved one.

New tests:
- `out_through_a_symlink_into_the_snapshot_is_refused` (unix). It covers a directory-symlink parent, `link/../<sha>/…`, `link/new/../…`, and an `--out` that is a symlink to a snapshot file.
- `out_with_a_case_variant_of_the_hub_root_is_refused` (macOS). It skips itself on a case-sensitive volume.

Both tests check that the snapshot files stay byte-identical. Both failed before the fix.

**Mutation check:** I made `resolved` lexical again (`Ok(real) => cur = real` became `Ok(_real) => {}`; `grep -c` showed 1 before and 1 after). Both new tests failed. I restored the line (`grep -c` back to 1).

I did not add the reviewer's `(dev, ino)` walk. The case-variant test passes on APFS through canonicalization alone.

### WR-01: Non-`gpu` build exits 0

**Files modified:** `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`
**Commit:** 0166ad161
**Applied fix:** The match arm is split by cfg. Without `gpu` it bails with the rebuild command; with `gpu` it returns the command's own `Result`. Checked by compiling both feature sets. There is no runtime test, because the lib tests always build with `gpu`.

### WR-02: Hub-root order disagrees with hf-hub 1.0

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`, `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`, `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs`
**Commit:** 4aa14d576
**Applied fix:** No vox helper exists for this. `vox-ml-cli` does not depend on hf-hub directly, and `hf_hub::resolve_cache_dir` reads the process env, which would make the tests env-dependent. Instead, a `HubEnv` struct captures `HF_HUB_CACHE`, `HUGGINGFACE_HUB_CACHE`, `HF_HOME`, `XDG_CACHE_HOME` and home once. `resolve_hub_root` applies hf-hub 1.0's order: `HF_HUB_CACHE`, `HUGGINGFACE_HUB_CACHE`, `<hf_home>/hub`, where hf_home is `HF_HOME`, then `XDG_CACHE_HOME/huggingface`, then `~/.cache/huggingface`. The `hub_root_order` test now covers every step, and the clap help text was updated. Empty values are still skipped. hf-hub does not skip them, but an empty path is never a usable cache.

### WR-04: Compared report does not identify the adapter

**Files modified:** `crates/vox-eval/src/reasoning_bench.rs`, `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`
**Commit:** 5cfcc2e59
**Applied fix:** `compared_report_json` takes `adapter_dir: &Path` and emits `"adapter": {"dir": …}`. The CLI passes the canonicalized adapter dir. Tests cover this in both crates. Not done: the manifest BLAKE3 digest (it would need a `vox-crypto` edge) and keying the default file name on the adapter. A second adapter on the same base still replaces the first record at the default path, but the surviving record now names its adapter.

### WR-05: `--out` may delete any unguarded file

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`
**Commit:** a47ca1453
**Applied fix:** `check_out_path` also refuses an `--out` that resolves under the bench root. The new `remove_stale_record` only removes an existing `--out` that is a regular file and parses as JSON with `"schema": "reasoning_before_after.v1"`. Anything else is refused and left in place. New tests: `existing_file_that_is_not_a_prior_record_is_refused_and_kept` and `out_under_the_bench_root_is_refused`. Both failed before the fix. The seed in `stale_compared_report_is_replaced` now carries the schema.

### WR-06: An unresolvable `--base` exits 0

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`, `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`
**Commit:** 3d515f62c
**Applied fix:** `resolve_base_pin` now returns `Result`. Unknown or ambiguous input, a non-hex `@rev` and an uppercase sha all become an error that names the known sizes. The error is raised before anything is written. A well-formed base whose snapshot is missing still records `base_snapshot_absent`, as covered by `cli_entry_records_the_resolved_pin`. New and updated tests: `base_resolution_unknown_or_ambiguous_is_an_error` and `unresolvable_base_is_an_error_and_writes_nothing`. `detect_inputs` keeps its `PinUnverifiable` path for direct callers.

### IN-01: `let _plugin = loaded;` contradicts its comment

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`
**Commit:** c7787a94c
**Applied fix:** Deleted the rebinding and its comment. `loaded` is declared before `handle`, so it now drops after it.

### IN-07: How-to text imprecise

**Files modified:** `docs/src/how-to/how-to-train-mens-macos-metal.md`
**Commit:** b3aab6ecd
**Applied fix:** The text now gives the name as `<size>-<first 12 hex of revision>.json` and states that the command needs a `--features gpu` build and that an unknown `--base` is an error. It is prose only, and `vox-doc-pipeline --lint-only` was not run on it.

## Skipped Issues

### WR-03: Shard completeness rule disagrees with the downloader's shard filter

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs:164-192`
**Reason:** Deferred, as the orchestrator directed. The mismatch only affects rungs that are not dense. The six dense Qwen3 sizes are unaffected. The phase 317 size ladder should own it: share `required_shard_filenames` from `vox-populi`, or use `vox_hf_layout::is_vision_or_mtp_key`.
**Original issue:** `snapshot_complete` requires every `weight_map` shard. `vox mens download` skips shards that hold only vision or MTP tensors, so a snapshot the downloader treats as complete is reported as `base_snapshot_incomplete`.

### IN-02, IN-03, IN-04, IN-05, IN-06 (info, deferred)

**Reason:** None of these is a safe one-liner. IN-02 changes the absence record's schema. IN-03 renames the default report files. IN-04 adds `repo_id` validation, which only matters on Windows. IN-05 changes control flow on base-side load failure. IN-06 changes the report write to a temp file plus rename. These are left for a follow-up.

---

_Fixed: 2026-10-27_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
