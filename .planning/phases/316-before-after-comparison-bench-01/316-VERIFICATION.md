---
phase: 316-before-after-comparison-bench-01
verified: 2026-10-29T01:57:00Z
status: passed
score: 8/8 must-haves verified
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - ".planning/phases/316-before-after-comparison-bench-01/316-01-PLAN.md"
  - ".planning/phases/316-before-after-comparison-bench-01/316-01-SUMMARY.md"
  - "crates/vox-eval/src/reasoning_bench.rs"
  - "crates/vox-ml-cli/src/commands/mens/mod.rs"
  - "crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs"
  - "crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs"
  - "crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs"
  - "crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs"
  - "crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs"
  - "docs/src/how-to/how-to-train-mens-macos-metal.md"
  - "mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json"
covered_digest: "v3:sha256:18cc6934c3aa23c1e0428a6e0e07301d048a7560eb4b873632ccabaf5af1ec75"
decision_coverage:
  honored: 0
  total: 0
  not_honored: []
---

# Phase 316: Before/After Comparison Verification Report

**Phase Goal:** A pinned Qwen3 base of any ladder size and its adapter are scored on the same slices when the weights exist, and no score is invented when they do not.
**Verified:** 2026-10-29T01:57:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

Roadmap success criteria are covered by truths 4, 5, and 8 (absent record with no scores; both sides scored on the three checked-in slices when the pin and adapter are present). This checkout has no Qwen3-8B snapshot. The present-branch truths are proven by tests that build a fake snapshot and inject `BenchOutcome` values. No model score was produced or invented.

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `vox mens eval-reasoning-compare` takes `--base` (pinned `hf_id@sha` or a Qwen3 size via `train_bases`), defaults to `DEFAULT_MODEL_ID`, and the resolved pin is the one named in the record | ✓ VERIFIED | Clap variant `eval-reasoning-compare` in `mens_tail_subcommands.rs`. `vox mens` forwards trailing args to `vox-ml-cli`. `resolve_base_pin(None)` returns `DEFAULT_MODEL_ID` (`Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218`). `cli_entry_records_the_resolved_pin` passed: `--base Qwen3-0.6B` writes `Qwen3-0.6B-c1899de289a0.json` with that revision. The real file names the default pin. |
| 2 | Only the exact pinned snapshot directory counts, and only when `config.json`, `tokenizer.json`, and every weight shard are files; detection never opens a weight file | ✓ VERIFIED | `snapshot_dir` joins `models--<owner>--<name>/snapshots/<sha>`. `snapshot_complete` uses `is_file` plus a capped read of `model.safetensors.index.json` only. `shard_is_file` rejects absolute paths and `..`. `reasoning_compare::detect::` tests passed (13). |
| 3 | An adapter counts as present only when `candle_qlora_adapter.safetensors` and `adapter_manifest.json` are files and `base_model` canonicalizes to that snapshot | ✓ VERIFIED | `detect_inputs` requires both files, then `manifest_matches` canonicalizes. A matching pair still stays absent unless `tokenizer.json` and `config.json` are files (stricter than the must-have, same absence outcome). |
| 4 | Missing, incomplete, or mismatched inputs write schema `reasoning_before_after.v1`, status `not_compared`, `gate_input` false, name the requested repo and revision, print `NOT COMPARED`, exit 0, and contain no `accuracy`, `think_initiation_rate`, `n`, or status `scored` | ✓ VERIFIED | `AbsenceRecord::to_json` and `eprintln!("NOT COMPARED: ...")` then `Ok`. `absence_record_has_no_score_keys_anywhere` passed. `absent_branch_never_calls_the_scorer` passed (scorer panics if called; result is `Ok`). |
| 5 | When both are present, `gsm8k_think`, `math500_think`, and `vox_think` are loaded once with `load_slice(.., SLICE_SIZE)` and the same items are scored on both sides; each side is `BenchOutcome::to_json`, so accuracy and think-initiation are separate keys | ✓ VERIFIED | `run_before_after_with` fills one `loaded` vec and passes it to both `score_side` calls. `both_sides_get_identical_checked_in_items` passed (`assert_eq` of the two item vecs against `load_slice` of `mens/data`). `present_branch_writes_all_six_entries` and `compared_report_has_separate_keys_per_slice_and_side` passed. The CLI closure calls `score_slices_loaded` → unchanged `score_loaded` (visibility-only diff). |
| 6 | Any `not_scored` entry, including a slice that fails to load, sets status `incomplete` and returns `Err`; status `compared` only when all six entries are scored | ✓ VERIFIED | `compared_report_json` sets `compared` only when each slice name appears once and both sides are `Scored`. `one_not_scored_side_returns_err_and_writes_incomplete`, `slice_load_failure_is_not_scored_on_both_sides`, and `one_not_scored_entry_makes_the_report_incomplete` passed. |
| 7 | Default `--out` is `mens/runs/reasoning_compare/<size>-<first 12 hex>.json`, outside a training dir; the out guard refuses serve-gate names, `collateral_eval`, and paths under the hub root or adapter dir; a stale file at `--out` is replaced | ✓ VERIFIED | `run_eval_reasoning_compare` uses `mens/runs` + `report_file_name()`. Guard tests in `reasoning_compare::tests::` passed (`out_named_like_a_gate_file_is_refused`, `out_inside_hub_root_or_adapter_is_refused`, `stale_compared_report_is_replaced`, `default_out_is_outside_a_runs_v1_adapter`). `check_out_path` runs before `detect_inputs`. |
| 8 | This checkout's real run takes the absent branch and writes `mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json` with no accuracy or think-initiation key | ✓ VERIFIED | File on disk: `status` `not_compared`, `reason` `base_snapshot_absent`, `gate_input` false, `snapshot_on_disk` false, slices the three names, `base_revision` `b968826d9c46dd6066d109eabc6255188de91218`. `jq` walk found no accuracy, think-initiation, `n`, or status `scored`. Shape matches `AbsenceRecord::to_json`. No weights downloaded. |

**Score:** 8/8 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | ----------- | ------ | ------- |
| `crates/vox-eval/src/reasoning_bench.rs` | Absence and compared report shapes | ✓ VERIFIED | `verify.artifacts` passed. Contains `reasoning_before_after.v1`, `AbsenceRecord::to_json`, `compared_report_json`. |
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` | Entry, out guard, both branches | ✓ VERIFIED | `run_eval_reasoning_compare_with` writes only after detection. Absent branch does not call the scorer. |
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` | Pin, hub root, snapshot, adapter | ✓ VERIFIED | `detect_inputs` is file-stat detection. Module tests passed. |
| `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs` | Clap variant | ✓ VERIFIED | `#[command(name = "eval-reasoning-compare")]`. |
| `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` | Gpu-gated arm | ✓ VERIFIED | `#[cfg(feature = "gpu")]` calls `run_eval_reasoning_compare`. |

### Key Link Verification

`verify.key-links` returned unverified because each `from` is a symbol description, not a relative path. Wiring was checked in source.

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `PopuliMensTail::EvalReasoningCompare` in `dispatch.rs` | `run_eval_reasoning_compare` | `cfg(feature = "gpu")` | WIRED | `dispatch.rs` lines 733–752 |
| `run_eval_reasoning_compare` | `run_eval_reasoning_compare_with` | env hub root and real backend scorer | WIRED | `reasoning_compare.rs` lines 345–359 |
| `run_eval_reasoning_compare_with` | `resolve_base_pin` and `check_out_path` | both before any write | WIRED | lines 320–331, then `detect_inputs` |
| `run_before_after_with` | `AbsenceRecord` / `compared_report_json` | only JSON builders | WIRED | `record.to_json()` or `compared_report_json` |
| real side scorer | `reasoning_eval::score_loaded` | `pub(crate)` visibility; grader unchanged | WIRED | `score_side_with_backend` → `score_slices_loaded` → `score_loaded`. Diff vs `0629773a7^` is `pub(crate)` on `parse_max_tokens`, `max_tokens_env`, and `score_loaded` only. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json` | absence record | `detect_inputs` on `~/.cache/huggingface/hub/.../b968826d9c46...` | Yes — `searched` is that snapshot path; no score fields | ✓ FLOWING |
| compared report (present branch) | per-slice `base` / `adapter` | `load_slice` of `mens/data/<slice>/reasoning.jsonl`, then `BenchOutcome::to_json` | Yes in tests. Not executed on this checkout because the 8B snapshot is absent | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Absence file has no score keys | `jq` walk of `mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json` | `true` / `ABSENCE_JSON_OK` | ✓ PASS |
| Compare orchestration | `cargo test -p vox-ml-cli --features gpu --lib reasoning_compare::tests::` | 15 passed, exit 0 | ✓ PASS |
| Snapshot and adapter detection | `cargo test -p vox-ml-cli --features gpu --lib reasoning_compare::detect::` | passed (13 of the following combined 16), exit 0 | ✓ PASS |
| Absence key walk and compared/incomplete shape | `cargo test -p vox-eval --lib` on `absence_record_has_no_score_keys_anywhere`, `compared_report_has_separate_keys_per_slice_and_side`, `one_not_scored_entry_makes_the_report_incomplete` | those three plus the detect run: 16 passed across 4 suites, exit 0 | ✓ PASS |

### Probe Execution

No phase plan or success criterion declares a `probe-*.sh`. None run.

| Probe | Command | Result | Status |
| ----- | ------- | ------ | ------ |
| — | — | no probe declared | SKIPPED |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| BENCH-01 | 316-01-PLAN.md | The same checked-in GSM8K slice and MATH-500 slice are scored for the base model and for an adapter. Accuracy and think-initiation are stored as separate keys. | ✓ SATISFIED | Present branch loads `gsm8k_think` and `math500_think` (and `vox_think`) once and stores `accuracy` and `think_initiation_rate` as separate keys. Absent branch writes no score keys. REQUIREMENTS.md maps BENCH-01 only to Phase 316. No other phase-316 IDs in the plan or in REQUIREMENTS.md. |

### Prohibitions

| ID | Statement | Status | Evidence |
| --- | --- | --- | --- |
| P-316-01 | No numeric score when the pin or adapter is absent; no stand-in revision | Holds | Absence `to_json` has no score keys. `absence_record_has_no_score_keys_anywhere` passed. Real file has none. Detection looks only at the requested sha. |
| P-316-02 | Never writes a file `vox mens serve` reads; not a serve-gate input | Holds | Both report builders set `gate_input: false`. `check_out_path` refuses `collateral_damage_report.json`, `reasoning_report.json`, and `collateral_eval`. Refusal tests passed. |
| P-316-03 | No grader, slice gold, or token-cap field change; no new rung; no training; no download | Holds | `reasoning_eval.rs` diff is visibility only. `qwen_reasoning.rs`, `reasoning_slice.rs`, and `mens/config/gpu-specs.yaml` are absent from the phase commit range. Detection does not download. |

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|-----------|-----------|--------|---------|----------|-----------------|---------|
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` | BENCH-01 | 15 (module run) | 0 | No — scores in the present-branch tests are injected `BenchOutcome`s; item identity is checked against `load_slice` of checked-in jsonl | Behavioral | PASS |
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` | BENCH-01 | 13 | 0 (`dangling-shard` test is `#[cfg(unix)]`, and this host is Unix) | No | Value | PASS |
| `crates/vox-eval/src/reasoning_bench.rs` | BENCH-01 | absence key walk, separate keys, incomplete status (named tests run) | 0 | No — the key walk forbids score keys rather than snapshotting the serializer's own output as an oracle file | Value | PASS |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0
**Insufficient assertions:** 0

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| — | — | No `TBD` / `FIXME` / `XXX` in the phase files | — | — |

Info: a non-gpu build of `vox-ml-cli` parses `eval-reasoning-compare` and returns `Ok(())` without writing a record, because `reasoning_compare` is `#[cfg(feature = "gpu")]`. That matches the plan's gpu-gated arm (same shape as collateral eval). The required run and the tests above use `--features gpu`. It does not write a score.

### Decision Coverage

No trackable decisions in CONTEXT.md. The gate skipped (`total` 0, `honored` 0).

### Human Verification Required

N/A — infrastructure/tooling phase. Acceptance is the absence file and the unit tests above. No user-facing UI.

### Gaps Summary

None. The goal holds: the gpu command scores both sides on the checked-in slices when the exact pin and a matching adapter are files, and the real run on this checkout wrote a `not_compared` record with no score keys.

---

_Verified: 2026-10-29T01:57:00Z_
_Verifier: Claude (gsd-verifier)_
