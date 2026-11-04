---
phase: 317-size-ladder-size-01
plan: 01
subsystem: mens training base ladder
status: complete
tags: [mens, qwen3, size-ladder, gpu-specs, floor-evidence, SIZE-01]
requires:
  - phase: 316-before-after-comparison-bench-01
    provides: resolve_base_pin, eval-reasoning-compare harness
provides:
  - qwen3_code ladder pinning all six Qwen3 dense sizes (0.6B, 1.7B, 4B, 8B, 14B, 32B)
  - floor_measured / measured_on / peak_mb on every train_bases rung, checked at parse time
  - TrainBase::floor_label, rungs_for_hf_id, floor_notes
  - base_rungs key in the compare record; floor note at train start
affects: [318-size-selection, 319-mesh-selection, 320-mac-training-measurement]
tech-stack:
  added: []
  patterns: ["serde-required key plus parse-time evidence check for honesty fields"]
key-files:
  created:
    - .planning/phases/317-size-ladder-size-01/deferred-items.md
  modified:
    - mens/config/gpu-specs.yaml
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-populi/src/mens/tensor/preset_schema.rs
    - crates/vox-populi/tests/training_presets_yaml_contract.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
    - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
    - docs/src/how-to/how-to-train-mens-macos-metal.md
key-decisions:
  - "floor_measured has no serde default, so a rung without it fails to load; parse_overlay rejects true without non-blank measured_on and peak_mb, and false with either"
  - "1.7B/4B floors 4200/7400 MB are estimates (NF4 non-embedding + bf16 tied embedding, x2.96, rounded up); every rung is floor_measured: false until Phase 320"
  - "detect_qwen_size_class reads only the repo id (part before the last @); 1.7B and 4B classify as Other until SIZE-02 presets"
requirements-completed: [SIZE-01]
metrics:
  duration: ~30 min
  completed: 2026-10-27
actuals:
  tokens: 10800
  tasks: 3
  commits: 4
plan_head_before: 2824de851d1e2d1c88ffcc25e267e60436c480de
plan_head_after: 443e46a466270e94eb02d52430b490efdb3d56f7
---

# Phase 317 Plan 01: Size Ladder (SIZE-01) Summary

All six Qwen3 dense sizes are now pinned Apache-2.0 rungs of `train_bases.qwen3_code`. Every rung has to say whether its memory floor was measured, and the loader enforces that. The compare harness and the train start print `floor unmeasured` for every rung today.

## What was built

- **`gpu-specs.yaml`:** added the `Qwen/Qwen3-1.7B@70d244cc…` (4200 MB) and `Qwen/Qwen3-4B@1cfa9a72…` (7400 MB) rungs to `qwen3_code`, with comments giving the ls-remote evidence and the floor derivations. Every one of the 23 rungs in all four tags now ends with `floor_measured: false`, and nothing else on those lines changed. The header comment now explains `floor_measured`, `measured_on` and `peak_mb`, and records the 2026-10-27 ls-remote re-confirmation. I re-ran `git ls-remote` for all six repos during execution and every `refs/heads/main` still equals its pin.
- **`spoke_base_resolver.rs`:**
  - `TrainBase` now derives `Serialize` and gains three fields: `floor_measured: bool` (no serde default), `measured_on: Option<String>` and `peak_mb: Option<u32>`.
  - New functions: `TrainBase::floor_label`, `rungs_for_hf_id` (exact match, sorted, dedupes only fully equal rungs) and `floor_notes` (reads the disk overlay or the embedded copy, returns an empty Vec if the overlay fails to load).
  - `parse_overlay` checks the floor evidence on every rung.
- **`reasoning_compare.rs`:** after the pin is resolved, stderr gets one `base <pin>: <floor_label>` line per rung. `run_before_after_with` takes `base_rungs` and writes it into both the not-compared record and the compared report.
- **`preset_schema.rs`:** `detect_qwen_size_class` now reads only the repo id. Before this, the hex of the 1.7B and 4B pins contains `8b`, so both were classed as `S8`.
- **`gpu.rs`:** right after `resolve_license_class`, each `floor_notes` entry is printed to stderr (6 inserted lines).
- **How-to:** one new paragraph listing the six `--base` names and what `floor_measured` means. Everything from `## Serve and pin from Drive` onward is byte-identical.
- **`deferred-items.md`:** records WR-03 and the Qwen3.8-27B rung.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | da1680f4e | feat(317-01): pin Qwen3-4B rung and report floors as unmeasured |
| 2 | 5de0c3250 | feat(317-01): pin all six Qwen3 dense sizes and require floor evidence |
| 3 (test fix, see Deviations) | 50527bec1 | test(317-01): pin expected ladder shas in the size-name resolution test |
| 3 | 443e46a46 | fix(317-01): class presets by repo id and print floor notes at train start |

Each task commit holds exactly the paths its plan task lists. Commit 50527bec1 holds only `detect.rs`. The pre-commit hooks (`ci-status`, `fmt-fix`, `tdd-guard`) all passed, and `--no-verify` was never used.

## Test counts

| Gate | Baseline | Gate floor | Final |
|------|----------|------------|-------|
| `vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'` | 55 | ≥63 | 63 passed |
| `vox-populi --features mens-train --test training_presets_yaml_contract` | 4 | ≥5 | 5 passed |
| `vox-ml-cli --features gpu --lib reasoning_compare` | 33 | ≥35 | 35 passed |

TDD order was followed for every task:
- **Task 1:** red was a compile failure, because the new fields, `floor_label` and `rungs_for_hf_id` did not exist yet (16 errors in vox-populi, 3 in vox-ml-cli).
- **Task 2:** red was 4 test failures: `measured_floor_needs_evidence`, `every_dense_ladder_pin_resolves_its_license`, the six-size contract test and the detect test.
- **Task 3:** red was first a compile failure (`floor_notes` missing), then `size_class_ignores_the_pinned_revision_text` returning `Some(S8)` for the 1.7B pin.

## Mutation checks (red, then green after restore)

Before each mutation and after each restore, `grep -cF` on the guarded line gave 1; while mutated it gave 0. Each restored file was `cmp`-identical to its backup. "Red" means nextest exited 100 (a test failure, not a compile error).

| ID | Mutation | Guard count before/during/after | Failing test (red) | Green after restore |
|----|----------|------|------|------|
| M1 | `parse_overlay` evidence check disabled (`if false && !ok`) | 1/0/1 | `measured_floor_needs_evidence` | pass |
| M2 | `#[serde(default)]` added to `floor_measured` | 1/0/1 | `overlay_rung_without_floor_measured_is_rejected` | pass |
| M3 | `floor_label` always uses the measured wording (`if false`) | 1/0/1 | `floor_label_says_floor_unmeasured_for_an_estimate` | pass |
| M4 | `rungs_for_hf_id` compares only the part before `@` | 1/0/1 | `rungs_for_hf_id_dedupes_repeats_across_tags` | pass |
| M5 | Entry passes `&serde_json::json!([])` instead of `&base_rungs` to `run_before_after_with` (bypassed at the call site) | 1/0/1 | `record_reports_the_base_rung_floor_status` | pass |
| M6 | `detect_qwen_size_class` matches the whole hint again (`map_or(hint, \|_\| hint)`) | 1/0/1 | `size_class_ignores_the_pinned_revision_text` | pass |
| M7 | 4B pin `1cfa…` changed to `0cfa…` in `gpu-specs.yaml` | 1/0/1 (each run) | `gpu_specs_qwen3_code_ladder_pins_all_six_dense_sizes` (M7a) and `base_resolution_resolves_every_ladder_size` (M7b) | pass |

## Other verification

- **Scoped clippy** on `vox-populi --features mens-train --lib --tests` and `vox-ml-cli --features gpu --lib --tests`, without `-D warnings`: both exit 0, both logs end in `Finished`, and no finding names a file this plan touched.
- **`rustfmt --edition 2024 --check`** on the six touched Rust files exits 0.
- **Size limits:**
  - Non-test non-blank lines are 359 in `spoke_base_resolver.rs` and 361 in `reasoning_compare.rs`, both under 450.
  - Over `adbac9d8e`, `preset_schema.rs` grew from 623 to 629 and `gpu.rs` from 761 to 767; the cap is +8.
- **Six-size smoke:** each `--base Qwen3-<size>` with an empty `--hub-root` printed `NOT COMPARED: base_snapshot_absent` and `floor unmeasured`, and wrote a record with `"base_model": "Qwen/<size>"` and `"floor_measured": false`. That makes 6 records, all git-ignored. The 14B and 32B runs each print two rungs (QLoRA and LoRA).
- **YAML counts on non-comment `train_bases` lines:** 23 `hf_id:`, 23 `floor_measured: false`, 0 `floor_measured: true`. `qwen3_code` has 7 rungs.
- **Scope:** `git diff adbac9d8e -- crates/vox-eval crates/vox-plugin-mens-candle-core contracts/ci` is empty, so there is no vox-eval change, no model-card change, and no crate-edge change.
- **Doc lint:** `vox-doc-pipeline --lint-only --paths how-to/how-to-train-mens-macos-metal.md` finished with no hard errors. The plan left this to hosted CI; I ran it anyway because it compiles without `vox-cli`.

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 1 - Test gap] `base_resolution_resolves_every_ladder_size` could not catch M7**
- **Found during:** Task 3 mutation planning.
- **Issue:** As specified, the test only compared each resolved pin with the overlay it was read from. A changed hex in the YAML changes both sides, so M7 would pass it.
- **Fix:** The test now carries the six ls-remote SHAs and asserts that each size resolves to `Qwen/<name>@<sha>`. The other assertions are unchanged.
- **Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`
- **Commit:** 50527bec1, a separate commit so that the Task 3 commit still holds exactly its five paths. M7b then went red as planned.

**2. [Style] `gpu.rs` import placement**
- The plan asked for at most six lines inserted after `resolve_license_class` and nothing else changed. Written with the full path, rustfmt expanded the call to 8 lines.
- I added `use vox_populi::mens::tensor::spoke_base_resolver::floor_notes;` next to the existing `use owo_colors::OwoColorize;` inside `run_gpu_training`, so the call fits in 5 lines. The total is 6 inserted lines, and `rg -c 'floor_notes\('` is still 1.

## Known Stubs

None.

## Threat Flags

None. No network endpoint, auth path or file-access pattern was added. `base_rungs` is additive output in a file the harness already writes.

## Self-Check: PASSED

- Files exist: `deferred-items.md`, plus all modified files.
- Commits are ancestors of HEAD: da1680f4e, 5de0c3250, 50527bec1, 443e46a46.
