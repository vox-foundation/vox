---
phase: 320-train-the-sizes-size-03
plan: 01
subsystem: mens-training
tags: [mens, qlora, plan-only, contamination, hf-cache, size-03]
status: complete
requires: []
provides:
  - "vox mens train --plan-only [--json] (schema vox_mens_train_plan_v1)"
  - "vox_corpus::training::preflight::check_heldout_contamination + HeldoutCheck"
  - "pin-aware vox_populi::mens::hub::is_model_cached via cache_has_revision"
affects: [320-04, 320-05, 320-06, 320-07]
tech-stack:
  added: []
  patterns: ["TrainRunOpts threaded as an argument (no env var)", "normalized whole-word containment over every string field of a row"]
key-files:
  created:
    - crates/vox-ml-cli/src/commands/schola/train/plan_only.rs
  modified:
    - crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
    - crates/vox-ml-cli/src/commands/mens/pipeline.rs
    - crates/vox-ml-cli/src/commands/schola/train/mod.rs
    - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
    - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
    - crates/vox-corpus/src/training/preflight.rs
    - crates/vox-populi/src/mens/hub.rs
decisions:
  - "The plugin heal now runs after check_corpus_threshold (and never under --plan-only), so a missing corpus is refused identically with or without --plan-only regardless of the installed plugin's state"
  - "TrainPlan is built by a plan_only.rs constructor (which also reads floor notes and the cache) to keep gpu.rs within its 25-line growth budget"
  - "Held-out matching pads normalized text with spaces, so a question matches only on whole-word boundaries ('2 2' does not match '2 22')"
metrics:
  duration: "~27 min"
  completed: 2026-10-27
actuals:
  tokens: 9800
  tasks: 3
  commits: 4
plan_head_before: f36074f89a1cd828af21cb1ed91b77c1a26f090a
plan_head_after: e3c12e4da9df0fb083c6b3d7dc8422da26b50431
---

# Phase 320 Plan 01: S0 tools (plan-only, contamination guard, pin-aware cache) Summary

`vox mens train --plan-only --json` now resolves a run up to the base-model download (size pick, free-memory reading, preset, seq_len, epochs, pinned-base cache state, corpus, license, floor notes, held-out contamination) and prints it as one JSON object, at zero GPU or network cost. Training, real or plan-only, refuses a corpus that contains a held-out `mens/data/*_think` question and names the offending lines.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | b26e60fce | feat(320-01): add --plan-only to vox mens train |
| 2 | 7ee6aacc8 | feat(320-01): refuse training rows that contain a held-out question |
| 3 | 6865e33f6 | feat(320-01): make the cached-base check pin-aware |
| 3 (lint) | e3c12e4da | fix(320-01): collapse the CUDA build check left by the heal move |

## Tests

| Set | Result |
|-----|--------|
| `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(plan_only) \| test(schola::train)'` | 27/27 pass (MIN 27). RED first: the 3 new `plan_only` tests failed against a stub `render` |
| `cargo nextest run -p vox-corpus --lib -E 'test(preflight)'` | 11/11 pass (MIN 10). RED first: 5 of 6 new tests failed against a stub (`no_workspace_is_not_checked` passes by design) |
| `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(hub::)'` | 19/19 pass (MIN 15). RED first: `pinned_id_needs_its_own_revision_in_cache` failed against a repo-name-only stub |

New tests: `json_plan_carries_every_d02_field`, `missing_reading_is_null_not_zero`, `text_plan_names_pick_memory_and_corpus`; `heldout_question_in_a_row_is_refused_with_its_line`, `question_inside_messages_is_found`, `clean_corpus_reports_slices_questions_and_rows`, `punctuation_only_question_matches_nothing`, `unreadable_slice_fails_closed`, `no_workspace_is_not_checked`; `pinned_id_needs_its_own_revision_in_cache`, `unpinned_id_matches_any_cached_revision`, `other_repo_never_matches`.

## End-to-end checks (debug binary, `VOX_MENS_NO_DOWNLOAD=1`, `--device cpu`)

- Qwen3-1.7B plan: exit 0; JSON has the schema, the `@70d244cc…` pin, `size 1.7B`, `explicit true`, `corpus_rows 100`, `seq_len 3072`, `epochs 1`, `free_mb null`, `license_class apache-2.0`; no `Downloading base model` on stderr; the output directory was never created.
- Missing corpus: `--plan-only` and the real run both exit 1 with the identical `Corpus file not found: <dir>/train.jsonl` (diffed).
- Contamination: clean corpus reports `{"status":"clean","slices":3,"heldout_questions":96,"rows_checked":100}`; appending one gsm8k question as row 101 is refused with `held-out contamination: 1 row(s) ... (lines 101; line 101 matches .../gsm8k_think/reasoning.jsonl line 1)`.
- `base_cached`: Qwen3-0.6B (pinned `c1899de…`, on disk) is `true`; Qwen3-1.7B is `false`; extra check: `Qwen/Qwen3-0.6B@000…0` (repo cached, revision not) is `false`.
- Scoped clippy (`-p vox-ml-cli --features cloud`, `-p vox-corpus`, `-p vox-populi --features mens-train`, `--lib --tests`) finished with no finding in a touched file; `rustfmt --check` clean on all 10 files.

## Mutation proofs

| ID | Guard | Count before | Mutated count | Named test under mutation | Restore (diff + shasum) / count | Re-run |
|----|-------|--------------|---------------|---------------------------|----------------------------------|--------|
| M1 | `if !hits.is_empty() {` -> `if false {` | 1 | 0 | `heldout_question_in_a_row_is_refused_with_its_line` FAIL (exit 100) | equal, sha a643a536…, count 1 | PASS |
| M2 | `questions.retain(\|(q, _)\| !q.trim().is_empty());` removed | 1 | 0 | `punctuation_only_question_matches_nothing` FAIL (the `???` question matched an all-empty row) | equal, sha a643a536…, count 1 | PASS |
| M3 | `if let Some(rev) = repo_revision(repo_id) {` -> `if let Some(rev) = None::<&str> {` | 1 | 0 | `pinned_id_needs_its_own_revision_in_cache` FAIL (exit 100) | equal, sha 4d1f1776…, count 1 | PASS |
| M4 | `if opts.plan_only {` (gpu.rs) -> `if false {`, rebuilt | 1 | 0 | plan-only 1.7B run exit 1 with `Downloading base model` (1 line) and `VOX_MENS_NO_DOWNLOAD is set` (1 line); no output dir | equal, sha 476f1ece…, count 1; rebuilt | exit 0, no download line; `git status` clean for gpu.rs |

## Acceptance

- `if opts.plan_only {` appears once in gpu.rs (line 228), before `download_model_blocking(` (252); `resolve_license_class(` (216) is also before it.
- `TrainRunOpts::default()`: 1 in pipeline.rs + 1 in dispatch.rs = 2.
- Non-blank non-test growth vs the anchor: gpu.rs 785 -> 810 (+25, limit 25); train_arm.rs 985 -> 995 (+10, limit 10).
- `check_heldout_contamination(` appears twice in gpu.rs; `pub fn check_heldout_contamination` once in preflight.rs.
- `git diff <anchor> -- '*.toml' contracts/` is empty: no crate edge, Cargo feature or env var added (P-320-01). Nothing under `/Users/brbrainerd/dev/vox` was edited (P-320-03).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plugin heal moved below the corpus check**
- **Found during:** Task 1, refusal-parity check.
- **Issue:** The real run hit the plugin heal (`ensure_plugin`) before `check_corpus_threshold`. With this Mac's installed `mens-candle-metal` plugin stale (abi_stable layout mismatch, the thing Plan 04's reinstall fixes), the real run refused with a plugin error while `--plan-only` (which skips the heal) refused with the corpus error, so D-02 parity failed for an environmental reason.
- **Fix:** The heal now runs right after `check_corpus_threshold` and is skipped under `--plan-only`. Cheap checks go first; a real run still refuses on a stale plugin, just after the corpus is checked.
- **Files modified:** run_train.rs. **Commit:** b26e60fce (follow-up lint fix e3c12e4da).

**2. [Rule 1 - Lint] collapsible_if from the move**
- The QLoRA branch was left holding only the CUDA-build check; collapsed into one `if`. **Commit:** e3c12e4da.

**3. M3 mutation text adjusted**
- The plan's mutation `if let Some(_rev) = None::<&str> {` does not compile, because the body uses `rev`, so the "failure" was a compile error and not a test failure. I used `if let Some(rev) = None::<&str> {` instead. It is the same semantic mutation (the revision branch is never taken), and the named test FAILs under it.

**4. Verification harness substitutions (environment, not code)**
- The local shell allowlist blocks running `target/debug/vox-ml-cli` directly, `cmp`, `mktemp`, and `bash -c`. So every end-to-end run used `cargo run -q -p vox-ml-cli --features cloud --bin vox-ml-cli -- ...` (the same debug binary). Equality checks used `diff -q` plus `shasum`, and scratch files lived under `$TMPDIR/gsd-320-01-exec/` instead of `/tmp/320-01-*`.

**5. Small design choices inside the plan's latitude**
- `TrainPlan::new` reads the floor notes and the cache, so gpu.rs stays within its budget. The text rendering has a `held-out check:` line. `HeldoutCheck` implements `Display` for the one-line stderr report.
- When there is no size pick, the free-memory reading is taken as `reading.or_else(live_free_memory)` inside the plan-only block. This replaces a separate `else` branch.

## Known Stubs

None.

## Threat Flags

None. The plan reads only local files and the local HF cache index. The new refusal is fail-closed.

## Self-Check: PASSED

- FOUND: crates/vox-ml-cli/src/commands/schola/train/plan_only.rs
- FOUND commits: b26e60fce, 7ee6aacc8, 6865e33f6, e3c12e4da (all ancestors of HEAD)
