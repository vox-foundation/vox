---
phase: 316-before-after-comparison-bench-01
verified: 2026-10-27T06:30:00Z
status: human_needed
score: 8/9 must-haves verified
covered_files:
  - .planning/phases/316-before-after-comparison-bench-01/316-01-PLAN.md
  - .planning/phases/316-before-after-comparison-bench-01/316-01-SUMMARY.md
  - crates/vox-eval/src/reasoning_bench.rs
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
  - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
  - docs/src/how-to/how-to-train-mens-macos-metal.md
covered_digest: "v3:sha256:f3148ffbd0e916fdba11ec61d424fdea19c85210a5e6e071f8ad31c80015b45e"
behavior_unverified: 1
overrides_applied: 0
behavior_unverified_items:
  - truth: "SC-1: with the pinned Qwen3 snapshot and a trained adapter on disk, one report holds accuracy and think-initiation for gsm8k/math500/vox on both sides (real model)"
    test: "On a host with the pinned Qwen3 snapshot in the HF cache and an adapter trained on it, run `vox mens eval-reasoning-compare --base <size> --adapter <run dir>` (gpu build)"
    expected: "status `compared`, six scored entries (3 slices x base/adapter) each with separate `accuracy` and `think_initiation_rate`, n=32, adapter.dir named, base unloaded before adapter loads, exit 0"
    why_human: "Present branch is proven only with an injected fake scorer; `score_side_with_backend` (real plugin load, inference, unload) has never run because no 8B snapshot or adapter exists on this host. Presence and wiring cannot show the load/unload ordering or a real score."
human_verification:
  - test: "Real compared run (see behavior_unverified_items)"
    expected: "status `compared` with six scored entries"
    why_human: "Needs weights not on this host. Roadmap-wise this lands in phases 320 (train sizes) and 321 (SC-3: at least two sizes with real trained scores); not a phase 316 gap."
---

# Phase 316: Before/After Comparison (BENCH-01) Verification Report

**Phase Goal:** A pinned Qwen3 base of any ladder size and its adapter are scored on the same slices when the weights exist, and no score is invented when they do not.
**Verified:** 2026-10-27
**Status:** human_needed
**Re-verification:** No, initial verification (post-review-fix tree, HEAD `6044b8a56`)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `eval-reasoning-compare` exists; `--base` = pin or Qwen3 size via `train_bases`; default `DEFAULT_MODEL_ID`; resolved pin is the one in the record | VERIFIED | clap variant `mens_tail_subcommands.rs`, gpu-gated arm + non-gpu bail in `dispatch.rs`; `resolve_base_pin` in `detect.rs:89`; smoke record names `Qwen/Qwen3-8B@b968826d...`; test `cli_entry_records_the_resolved_pin` passes |
| 2 | Only the exact `models--<o>--<n>/snapshots/<sha>/` dir counts, with config, tokenizer and all shards as files; no weight open/model load | VERIFIED | `snapshot_dir`, `snapshot_complete` use `is_dir`/`is_file`/index JSON only; tests `hub_with_only_qwen3_0_6b_is_absent_for_the_8b_pin`, `other_revision_is_not_the_pinned_snapshot` pass |
| 3 | Adapter present only with both files and manifest `base_model` canonicalizing to the pinned snapshot | VERIFIED | `detect_inputs` L167-184 (`same_dir`); trainer `finalize.rs:65-78` writes the snapshot dir as `base_model`, so real adapters satisfy it; mismatch tests pass |
| 4 | Missing/incomplete/mismatched gives absence record (`not_compared`, `gate_input` false), stderr `NOT COMPARED: <reason>`, exit 0, no `accuracy`/`think_initiation_rate`/`n`/`scored` anywhere | VERIFIED | `AbsenceRecord::to_json`; test `absence_record_has_no_score_keys_anywhere` (recursive walk, all 6 reasons) passes; real smoke run below |
| 5 | Present branch loads the three slices once and scores identical items on both sides via unchanged `score_loaded`; per-entry `BenchOutcome::to_json`, separate accuracy/think keys | VERIFIED (fake scorer) | `run_before_after_with` loads once, passes same `loaded` to both `score_side` calls; `reasoning_eval.rs` diff vs `b86083a70` is three `pub(crate)` visibility changes only; tests `both_sides_get_identical_checked_in_items`, `present_branch_writes_all_six_entries` pass. Real model path: see truth 9 |
| 6 | Any `not_scored` (including slice load failure) gives `incomplete` and Err; `compared` only if all six scored | VERIFIED | `compared_report_json` + bail in `run_before_after_with`; tests `one_not_scored_side_returns_err_and_writes_incomplete`, `slice_load_failure_is_not_scored_on_both_sides` pass |
| 7 | Default `--out` is `mens/runs/reasoning_compare/<size>-<rev12>.json`; guard on every entry path refuses gate names, `collateral_eval`, hub root, adapter dir (and, post-review, bench root and symlinks); stale record replaced | VERIFIED | `check_out_path` called at entry before any write (L342); `resolved()` canonicalizes (CR-01 fix); `remove_stale_record` only deletes a prior v1 record (WR-05); `mens/runs/` is git-ignored; guard tests incl. symlink/case tests pass |
| 8 | Real run on this checkout takes the absent branch and writes `Qwen3-8B-b968826d9c46.json` with no score keys | VERIFIED | Re-run by verifier (see Behavioral Spot-Checks) |
| 9 | SC-1 end to end: real pinned snapshot + trained adapter produce a `compared` report | PRESENT_BEHAVIOR_UNVERIFIED | No weights on host; `score_side_with_backend` real load/infer/unload unexercised |

**Score:** 8/9 truths verified (1 present, behavior-unverified). Roadmap SC-2 (snapshot absent, record and no numeric scores) and SC-3 (checked-in items on both sides) are covered by truths 4, 5, 8; SC-1 by truths 5 and 9.

### Prohibitions

| ID | Statement | Status | Evidence |
|----|-----------|--------|----------|
| P-316-01 | No numeric score when snapshot/adapter absent; no stand-in model | VERIFIED (test-tier wired) | `absent_branch_never_calls_the_scorer` (panicking scorer), key-walk test, 0.6B-hub-vs-8B-pin test; smoke record grep: 0 score keys |
| P-316-02 | Never writes a file `vox mens serve` reads; never a serve-gate input | VERIFIED (test-tier wired) | gate-name / `collateral_eval` / hub / adapter / bench-root refusals tested; `gate_input:false` in both report shapes; no consumer added to serve |
| P-316-03 | No grader/gold/token-cap change; no new rung, training, download | VERIFIED | `git diff b86083a70 HEAD` empty for `mens/config/gpu-specs.yaml`, `qwen_reasoning.rs`, `trial_flywheel.rs`, `reasoning_slice.rs`, `mens/data`; `reasoning_eval.rs` visibility-only; grep finds no network/download call in the new files |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-eval/src/reasoning_bench.rs` | VERIFIED | `BEFORE_AFTER_SCHEMA`, `BaseLabel::report_file_name`, `AbsenceReason`, `AbsenceRecord::to_json`, `compared_report_json` (adapter dir recorded) |
| `reasoning_compare.rs` | VERIFIED | 1001 lines total, ~370 before tests; entry, guard, scorer with `unload_model` |
| `reasoning_compare/detect.rs` | VERIFIED | 601 lines total, ~236 before tests |
| `mens_tail_subcommands.rs` / `dispatch.rs` / `mod.rs` | VERIFIED | variant, arm (gpu and non-gpu), module registered |
| `docs/.../how-to-train-mens-macos-metal.md` | VERIFIED | paragraph present |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| dispatch arm | `run_eval_reasoning_compare` | WIRED |
| `run_eval_reasoning_compare` | `run_eval_reasoning_compare_with` | WIRED |
| entry | `resolve_base_pin` and `check_out_path` before write | WIRED |
| `run_before_after_with` | `AbsenceRecord::to_json` / `compared_report_json` (sole builders) | WIRED |
| real scorer | `reasoning_eval::score_loaded` via `score_slices_loaded` | WIRED |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|----------|------|--------|-----------|--------|
| compared report | per-slice outcomes | `score_loaded` over `load_slice(mens/data/*/reasoning.jsonl)` | Yes for items; scores need a real backend | FLOWING (items) / UNVERIFIED (model scores) |
| absence record | pin, searched path | `resolve_base_pin`, `snapshot_dir` | Yes | FLOWING |

### Behavioral Spot-Checks (run by verifier, scoped)

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| compare + reasoning tests | `timeout 2400s cargo nextest run -p vox-ml-cli --features gpu --lib reasoning` | 50 passed, 0 failed (18 `reasoning_compare::tests`, plus detect and 12 `reasoning_eval`) | PASS |
| collateral regression (phase 314) | `cargo nextest run -p vox-ml-cli --features gpu --lib eval_collateral` | 17 passed | PASS |
| vox-eval regression (314/315) | `cargo nextest run -p vox-eval` | 177 passed, 4 skipped | PASS |
| real absent-branch smoke | removed old git-ignored record, then `cargo run -q -p vox-ml-cli --features gpu -- mens eval-reasoning-compare` | exit 0; stderr `NOT COMPARED: base_snapshot_absent (mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json)`; file written, `status: not_compared`, `gate_input: false`, grep for `accuracy`/`think_initiation_rate`/`"n"` = 0 | PASS |

Host has `models--Qwen--Qwen3-0.6B` but no `Qwen3-8B`, confirming the absent branch is the genuine outcome.

### Probe Execution

None declared. SKIPPED.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BENCH-01 | 316-01 | Same checked-in GSM8K and MATH-500 slice scored for base and adapter; accuracy and think-initiation as separate keys | SATISFIED in code; real-model scoring awaits human/later phase | Truths 1-8; truth 9 deferred to real weights |

No orphaned requirements: BENCH-01 is the only ID mapped to phase 316 and is claimed by the plan.

### Anti-Patterns Found

None in the new files (no TBD/FIXME/XXX/TODO; no stubs; no hardcoded empty returns on a render path).

### Warnings (non-blocking)

1. **Bookkeeping:** `.planning/REQUIREMENTS.md` has BENCH-01 checked `[x]` (line 16) but the traceability row (line 57) still says `Pending`. Update at phase close.
2. **WR-03 deferred (review):** `snapshot_complete` requires every `weight_map` shard, while `vox mens download` skips vision/MTP-only shards. Affects non-dense rungs only; the six dense Qwen3 sizes are unaffected. Phase 317 should own it (share `required_shard_filenames` or `vox_hf_layout::is_vision_or_mtp_key`).
3. **WR-04 partial:** compared report records `adapter.dir` but not a manifest digest; a second adapter on the same base replaces the first record at the default path (the surviving record names its adapter).
4. **IN-02..IN-06 deferred** (schema/name changes, Windows `repo_id` validation, base-load-failure control flow, atomic write). No goal impact.
5. **Not a phase 316 gap:** `vox-cli` does not compile on this branch (`vox-orchestrator-mcp` errors, uncommitted work elsewhere; recorded in `deferred-items.md`). Consequently `vox ci command-compliance` could not run; no command-registry projection was added, consistent with the sibling `eval-collateral-damage`. Re-run once `vox-cli` builds. Verification here used `vox-ml-cli` directly, which builds and passes.
6. The review-fix commits `b15479690..6044b8a56` were checked in the code: `resolved()` canonicalization (CR-01), non-gpu bail (WR-01), hf-hub 1.0 root order (WR-02), strict `resolve_base_pin` (WR-06), `remove_stale_record` (WR-05), adapter dir in report (WR-04), no `_plugin` rebinding (IN-01). All present and their tests pass.

### Human Verification Required

#### 1. Real compared run

**Test:** With the pinned Qwen3 snapshot in the HF cache and an adapter trained on it, run `vox mens eval-reasoning-compare --base <size> --adapter <run dir>` from a `--features gpu` build.
**Expected:** `status: compared`, six scored entries with separate `accuracy` and `think_initiation_rate` (n=32), `adapter.dir` set, exit 0; base unloaded before the adapter loads.
**Why human:** No 8B (or any pinned-size other than the 0.6B tree) snapshot or adapter on this host; `score_side_with_backend` has only been exercised through a fake scorer. This is classified as present, behavior-unverified, not as a failure: phase 316 SC-1 is conditional on the weights existing, and ROADMAP places training the sizes in phase 320 and real before/after scores for at least two sizes in phase 321 (SC-3 there). It should be carried to those phases rather than block 316.

### Gaps Summary

No gaps. The command, detection, absence record, present-branch scoring logic, output guard and post-review fixes are all in the tree and tested; the real absent-branch run reproduces the claimed output. The only open item is a real-weights scored run, which cannot be produced on this host and is scheduled for phases 320/321.

---

_Verified: 2026-10-27_
_Verifier: Claude (gsd-verifier)_
