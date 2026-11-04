---
phase: 316-before-after-comparison-bench-01
plan: 01
subsystem: mens-eval
tags: [mens, qwen3, reasoning-bench, eval, cli, hf-cache]

requires:
  - phase: 315
    provides: "32-item reasoning slices, score_loaded grader, BenchOutcome::to_json"
provides:
  - "vox mens eval-reasoning-compare (--base size|pin, --adapter, --hub-root, --bench-root, --out)"
  - "reasoning_before_after.v1 report: not_compared absence record (no score keys) and compared/incomplete report"
  - "Pinned-snapshot and adapter detection over the HF cache layout, model-free"
  - "Per-base default output mens/runs/reasoning_compare/<size>-<rev12>.json, outside any training dir"
affects: [317, 318, 319, 320]

actuals:
  tokens: 18661
  tasks: 3
  commits: 5

plan_head_before: 4bf115518a524636995d48d1693ec105b1f582d3
plan_head_after: 2b4721d295a7aa4458e3c9af874dfe8e09470a58

tech-stack:
  added: []
  patterns:
    - "Single tested entry (run_eval_reasoning_compare_with) with injected env, roots and side scorer; the CLI wrapper only supplies real values"
    - "Absence record carries no score keys, proven by a recursive key walk plus mutation"

key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
    - .planning/phases/316-before-after-comparison-bench-01/COVERAGE.md
  modified:
    - crates/vox-eval/src/reasoning_bench.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - docs/src/how-to/how-to-train-mens-macos-metal.md

key-decisions:
  - "--base size names resolve through gpu-specs train_bases to exactly one distinct pin; unknown or ambiguous input stays unpinned and is recorded as pin_unverifiable (no fallback revision, no download)"
  - "No command-registry projection added for eval-reasoning-compare: mens tail subcommands (e.g. eval-collateral-damage) are covered by the single mens registry entry; command-compliance could not run to confirm"

patterns-established:
  - "Mutation proofs at call sites: each guard deleted or bypassed at the entry, named test red, restored green, with grep -c presence counts"

requirements-completed: [BENCH-01]

coverage:
  - id: D1
    description: "Absent branch: real run on this checkout writes the not_compared record naming Qwen/Qwen3-8B at its pin with no score keys and exits 0"
    requirement: BENCH-01
    verification:
      - kind: integration
        ref: "cargo run -p vox-ml-cli --features gpu -- mens eval-reasoning-compare (NOT COMPARED: base_snapshot_absent; record has no accuracy/think_initiation)"
        status: pass
      - kind: unit
        ref: "crates/vox-eval/src/reasoning_bench.rs#absence_record_has_no_score_keys_anywhere"
        status: pass
    human_judgment: false
  - id: D2
    description: "Present branch scores the same checked-in items on both sides, matched by slice name; any not_scored entry gives incomplete and Err"
    requirement: BENCH-01
    verification:
      - kind: unit
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs#both_sides_get_identical_checked_in_items, present_branch_writes_all_six_entries, one_not_scored_side_returns_err_and_writes_incomplete, slice_load_failure_is_not_scored_on_both_sides"
        status: pass
    human_judgment: false
  - id: D3
    description: "Base chosen by size or pin through the real entry; detection at that exact revision; per-size output file"
    requirement: BENCH-01
    verification:
      - kind: unit
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs#cli_entry_records_the_resolved_pin; detect.rs#base_resolution_*"
        status: pass
    human_judgment: false
  - id: D4
    description: "A real compared run (8B snapshot plus trained adapter on disk) produces real scores"
    requirement: BENCH-01
    verification: []
    human_judgment: true
    rationale: "No Qwen3-8B snapshot or adapter on this host; the present branch is proven only with a fake scorer. A real compared run is manual per 316-VALIDATION.md and happens in phases 317-320."

duration: 52min
completed: 2026-10-26
status: complete
---

# Phase 316 Plan 01: Before/After Comparison Summary

**`vox mens eval-reasoning-compare` scores a pinned Qwen3 base (by size name or `hf_id@sha`, default the pinned 8B) and its adapter on the three 32-item reasoning slices into a per-base `reasoning_before_after.v1` report. When the exact pinned snapshot or a matching adapter is not on disk, it writes a `not_compared` record with no score keys. On this checkout it takes that branch.**

## Performance

- **Duration:** about 52 min wall clock, including the worktree being deleted and recreated mid-run
- **Started:** 2026-10-27T04:09:07Z (plan base 4bf115518)
- **Completed:** 2026-10-27T05:00:31Z
- **Tasks:** 3
- **Files modified:** 9 (plan commits only)

## Accomplishments

- The CLI variant and its gpu-gated dispatch arm, plus one tested entry. That entry runs the `--out` guard on every invocation before any write. The guard refuses gate file names, `collateral_eval` components, the hub root and the adapter dir.
- Detection is model-free. Only the exact `models--<owner>--<name>/snapshots/<pinned sha>/` directory counts, and only with config, tokenizer and every indexed shard present as files. The adapter manifest's `base_model` must canonicalize to that snapshot.
- On the present branch both sides get the same `load_slice` items through the unchanged `score_loaded`, and outcomes are matched by slice name. Each model is unloaded before the next load. Any `not_scored` entry writes `incomplete` and returns Err.
- `--base` accepts `Qwen3-0.6B`, `Qwen/Qwen3-8B`, or a full pin. Size names resolve through `train_bases`. Unknown or ambiguous input is recorded as `pin_unverifiable`.
- The real run here printed `NOT COMPARED: base_snapshot_absent` and wrote `mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json` (git-ignored). The record names `Qwen/Qwen3-8B` at `b968826d9c46dd6066d109eabc6255188de91218`, has `gate_input: false`, and has no `accuracy` or `think_initiation` key.

## Task Commits

1. **Task 1: Tracer, absence record end to end:** `88656e612` (feat)
2. **Task 2: Present branch scores identical items; detection rejects partial inputs:** `db5463ed9` (feat)
3. **Task 3: Base by size or pin, mutation proofs, how-to, COVERAGE.md:** `2b4721d29` (feat)

Measured range `4bf115518..HEAD` is 5 commits: the three task commits, `35dd5d679` (Rule-3 deviation, below), and `09a012bcc` (out-of-plan, below).

## Files Created/Modified

- `crates/vox-eval/src/reasoning_bench.rs`: `BEFORE_AFTER_SCHEMA`, `BaseLabel`, `AbsenceReason`, `AbsenceRecord::to_json`, `compared_report_json`, and their tests
- `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`: args, out guard, `run_before_after_with`, the tested entry, the CLI wrapper, the real side scorer with unload
- `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`: pin and base resolution, hub-root order, snapshot and adapter detection
- `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`: three `fn` to `pub(crate) fn` visibility changes only
- `crates/vox-ml-cli/src/commands/mens/mod.rs`, `populi/mens_tail_subcommands.rs`, `populi/dispatch.rs`: module registration, clap variant, dispatch arm
- `docs/src/how-to/how-to-train-mens-macos-metal.md`: one paragraph before `## Serve and pin from Drive`; text from that heading on is byte-identical
- `.planning/phases/316-before-after-comparison-bench-01/COVERAGE.md`: no-external-API declaration

## Verification (Task 3 gate and plan-level)

- `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare`: 28 passed (gate at least 28)
- `cargo test -p vox-eval --lib reasoning_bench`: 23 passed (gate at least 23)
- `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval`: 12 passed (gate at least 12)
- `cargo clippy --color never -p vox-eval --lib --tests` and `-p vox-ml-cli --features gpu --lib --tests` (no `-D warnings`): both exit 0 and end in `Finished`. No finding names a file this plan touches. The vox-eval log shows only the three pre-existing warnings.
- `rustfmt --edition 2024 --check` on the three Rust files: clean
- Size check (non-blank lines before `#[cfg(test)]`): `reasoning_compare.rs` 315, `detect.rs` 181 (limit under 450)
- Smoke run: exit 0, `NOT COMPARED: base_snapshot_absent`, `"gate_input": false`, no score key
- `git diff b86083a70 HEAD -- mens/config/gpu-specs.yaml`, `qwen_reasoning.rs`, `trial_flywheel.rs`, the research doc and the e2e script: all empty

## Mutation Checks (Task 3)

Each guard was broken, the named test ran red, the code was restored, and the test ran green. Counts are `grep -c` of the guarded line before the edit, during it, and after the restore.

| # | Mutation | Guarded line count (before / mutated / restored) | Red | Green |
|---|----------|---------------------------------------------------|-----|-------|
| 1 | Delete the `check_out_path(&out, &hub_root, ..)?` call at the entry | 1 / 0 / 1 | `out_named_like_a_gate_file_is_refused` and `out_inside_hub_root_or_adapter_is_refused` FAILED | both ok |
| 2 | Replace `detect::resolve_base_pin(..)` at the entry with `DEFAULT_MODEL_ID.to_string()` | 1 / 0 / 1 | `cli_entry_records_the_resolved_pin` FAILED | ok |
| 3 | `snapshot_dir` joins the first entry under `snapshots/` instead of the revision | `.join(revision),` 1 / 0 / 1 | `other_revision_is_not_the_pinned_snapshot` FAILED | ok |
| 4 | Adapter check: `b.is_dir() && (true \|\| same_dir(..))` | 1 / 0 / 1 | `adapter_manifest_base_must_be_the_pinned_snapshot` FAILED | ok |
| 5 | `NotCompared` arm calls `score_side(Side::Base, ..)` before writing | mutation marker 0 / 1 / 0; write line 1 / 1 / 1 | `absent_branch_never_calls_the_scorer` FAILED (panicking scorer fired) | ok |
| 6 | Insert `"accuracy": 0.0` into `AbsenceRecord::to_json` | marker 0 / 1 / 0; `"accuracy"` total 7 / 8 / 7; vox-eval file diff-clean after | `absence_record_has_no_score_keys_anywhere` FAILED | ok |
| 7 | File-name refusal inside `check_out_path` becomes `if false && ..` | 1 / 0 / 1 | `out_named_like_a_gate_file_is_refused` FAILED | ok |
| 8 | Skip the pre-write `remove_file` and the absent-branch `write_report` | remove line 1 / 0 / 1; write line 1 / 0 / 1 | `stale_compared_report_is_replaced` FAILED | ok |

Mutations 1 and 2 delete or bypass a call at the entry, not just a validator body. After all eight, `grep -c MUTATION` is 0 in both files. The full 28-test filter passed again on the restored tree before the commit.

## Decisions Made

- `resolve_base_pin` matches a `train_bases` hf_id when its repo id equals the input or ends with `/<input>`, and keeps only 40-hex pins. Exactly one distinct pin is returned; anything else returns the input unchanged. No `train_bases` rung was added, and no VRAM or memory is read.
- command-compliance: no projection added. See Issues Encountered.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] vox-code-audit detector modules missing from the branch**
- **Found during:** Task 1 (first build of the fresh worktree)
- **Issue:** `detectors/mod.rs` declares four modules whose files existed only untracked in `~/dev/vox`. Every clean checkout failed to compile `vox-code-audit` and everything that depends on it.
- **Fix:** Copied the four files verbatim from that checkout
- **Committed in:** `4bf115518` (just before the plan-head ledger base)

**2. [Rule 3 - Blocking] vox-orchestrator modules missing from the branch**
- **Found during:** Task 1
- **Issue:** `lib.rs` declares nine modules that weren't committed, so `vox-orchestrator`, `vox-cli-core` and `vox-ml-cli` failed to compile
- **Fix:** Copied the nine files verbatim
- **Committed in:** `35dd5d679`

**3. [Process] Task 2 implementation preceded its five detect tests**
- In Task 2 the detect implementation was written before its five tests, so there was no separate RED for those tests. Task 3's mutation checks (3 and 4 in particular) show those guards are load-bearing.

### Out-of-plan commit on the branch

- `09a012bcc` fix(vox-orchestrator): no fixed guest-share key, constant-time MAC compare. This is an orchestrator security fix to the copied `guest_share.rs`. It is not plan work.

**Total deviations:** 2 auto-fixed (Rule 3), 1 process note
**Impact on plan:** The Rule-3 copies only made the crate graph compile. They are not part of BENCH-01.

## Issues Encountered

- `cargo run -p vox-cli -- ci command-compliance` exited 101 because `vox-cli` does not compile on this branch. `vox-orchestrator-mcp` has 19 errors, for example E0609 for the missing `ServerState` fields `id_allocator`, `thread_run_queue` and `steered_prompts` in `chat_tools/chat/history.rs`. The installed `vox` is stale (252cfe7af) and refuses to run guards. This is pre-existing and unrelated to this plan. It is logged in `deferred-items.md` and in `.planning/WINDOWS.md` as `unrun-verify`. The sibling `eval-collateral-damage` has no individual registry entry either, so no catalog projection was added.
- The previous executor's worktree was deleted by an outside process mid-Task 3. The orchestrator recreated it and re-applied the uncommitted patch. This executor reviewed the patch against the plan, formatted it, and ran it through every gate.

## User Setup Required

None. No external service configuration is required.

## Next Phase Readiness

- Phases 317-320 can call `vox mens eval-reasoning-compare --base <size> --adapter <run dir>` and get one file per base under `mens/runs/reasoning_compare/`.
- A real compared run needs the pinned snapshot and a trained adapter on disk. No score exists yet. A green test is not a score.
- `command-compliance` should be re-run once `vox-cli` builds on this branch.

---
*Phase: 316-before-after-comparison-bench-01*
*Completed: 2026-10-26*

## Self-Check: PASSED

All created files exist; commits 4bf115518, 35dd5d679, 09a012bcc, 88656e612, db5463ed9, 2b4721d29 are ancestors of HEAD.
