---
phase: 317-size-ladder-size-01
verified: 2026-10-29T04:02:57Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - .planning/phases/317-size-ladder-size-01/317-01-PLAN.md
  - .planning/phases/317-size-ladder-size-01/317-01-SUMMARY.md
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
  - crates/vox-populi/tests/training_presets_yaml_contract.rs
  - mens/config/gpu-specs.yaml
covered_digest: "v3:sha256:955560bec0a761d044ea16d7bef6259a06dfda965a8d64ec625dc6a50cb784dd"
behavior_unverified: 0
overrides_applied: 0
decision_coverage:
  honored: 0
  total: 0
  not_honored: []
---

# Phase 317: Size Ladder (`SIZE-01`) Verification Report

**Phase Goal:** Every Qwen3 dense size is a pinned, licensed rung that the trainer and the harness can name.
**Verified:** 2026-10-29T04:02:57Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | The ladder has Qwen3-0.6B, 1.7B, 4B, 8B, 14B, and 32B, each pinned to a real Hugging Face commit and carrying a license. | ✓ VERIFIED | `mens/config/gpu-specs.yaml` `train_bases.qwen3_code` rows 251–257. Each `hf_id` is `Qwen/Qwen3-<size>@` plus 40 lowercase hex, `license: apache-2.0`. `resolve_qwen3_size_covers_the_dense_ladder` passed and asserts those pins. Live Hugging Face was not queried; the 1.7B and 4B shas match the locked 2026-10-28 pins. |
| 2 | A rung whose memory floor has not been measured on hardware says so. | ✓ VERIFIED | 1.7B and 4B set `floor_measured: false` and omit `floor_mb`. `parse_overlay` rejects a measured row with no floor and an unmeasured row with a floor. `pick_base` keeps only `floor_measured && floor_mb` candidates. `Qwen3SizePin.floor_measured` copies the row flag. |
| 3 | A caller can name `0.6B`, `1.7B`, `4B`, `8B`, `14B`, or `32B`, or the `Qwen3-<size>` and `Qwen/Qwen3-<size>` forms, and get that size's `qwen3_code` QLoRA pin, revision, license, and `floor_measured` from the checked-in overlay. | ✓ VERIFIED | `resolve_qwen3_size` looks up only `qwen3_code` rows whose methods contain `qlora`. `resolve_qwen3_size_covers_the_dense_ladder` passed, including `4B` / `Qwen3-4B` / `Qwen/Qwen3-4B` and the same three spellings of 8B. |
| 4 | 1.7B and 4B are pinned to the 2026-10-28 commits, licensed apache-2.0 with `attribution_required: false`, carry no `floor_mb`, have `floor_measured: false`, and live only under `train_bases.qwen3_code`. | ✓ VERIFIED | Rows are `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` and `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c`. `unmeasured_pins_are_only_on_qwen3_code_and_measured_floors_stay` passed and asserts those pins appear only on `qwen3_code`. `small_code_default` and `strong_code_default` do not list them. |
| 5 | `pick_base` still returns the same measured `qwen3_code` rung: error below 2000 MB, Qwen3-8B at 16384 MB, 14B QLoRA at 24000 MB, 14B LoRA at 49152 MB. Unmeasured rows never win. | ✓ VERIFIED | Filter is `floor_measured && floor_mb <= vram_mb`, then largest floor. Passed: `qwen3_code_fail_closed_below_floor` (1000 MB), `resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged` (16384 MB stays on `b968826d…`), `qwen3_code_24g_returns_14b_qlora` (24000 MB), `qwen3_code_48g_prefers_unquantized_14b` (49152 MB, methods include `lora`). Measured floors stay 2000 / 12000 / 20000 / 44000 / 60000. |
| 6 | An unknown size spelling is an error. A capability tag such as `qwen3_16g` is not a size name. A full `hf_id` at a sha still passes through. `not-a-model` stays unverifiable. | ✓ VERIFIED | `qwen3_size_token` accepts only the six tokens and the two prefixes, no trim and no case fold. `resolve_base_pin` returns a revisioned input unchanged, calls `resolve_qwen3_size` only for size spellings, and otherwise returns the original string when the tag scan is not exactly one pin. `detect.rs` tests assert `2B` / `Qwen3-7B` / `Qwen/8B` error, `qwen3_16g` stays `qwen3_16g`, and `not-a-model` stays `not-a-model`. |
| 7 | Tests read `mens/config/gpu-specs.yaml`. They do not call Hugging Face, download weights, train, or touch the mesh. | ✓ VERIFIED | Ladder tests call `load_overlay(workspace_root())`, which reads `mens/config/gpu-specs.yaml`. `resolve_qwen3_size` has no network client. Harness absence tests use a temp hub and a scorer that panics. No Hugging Face call was made in this verification. |

**Score:** 7/7 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `mens/config/gpu-specs.yaml` | `qwen3_code` QLoRA rows for 1.7B and 4B with the locked pins, `floor_measured: false`, and no `floor_mb` | ✓ VERIFIED | Rows 252–253 match the locked pins. `verify.artifacts` passed. Data is the overlay `load_overlay` parses. |
| `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` | `TrainBase.floor_measured`, optional `floor_mb`, `pick_base` skip, `resolve_qwen3_size` | ✓ VERIFIED | `pub fn resolve_qwen3_size` at line 145. `floor_mb: Option<u32>` defaults to missing. `default_floor_measured` returns true. |
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` | Size-name branch of `resolve_base_pin` calls `resolve_qwen3_size` | ✓ VERIFIED | Lines 239–242 call `is_qwen3_size_spelling` then `resolve_qwen3_size`. |
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` | Compare entry records the 1.7B pin when the snapshot is absent | ✓ VERIFIED | `size_name_1_7b_records_the_pinned_unmeasured_base` and `size_name_4b_records_the_pinned_unmeasured_base` assert the locked revisions. Wired through `run_eval_reasoning_compare_with`. Not re-executed this pass; the resolver tests that supply the pin were. |

**Artifacts:** 4/4 verified

`verify.artifacts` on `317-01-PLAN.md` returned `all_passed: true` (4/4). The tool does not check substantive wiring; that was checked by reading the call sites above.

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `run_eval_reasoning_compare_with` | `detect::resolve_base_pin` | Result propagated with `?` before detection | ✓ WIRED | `reasoning_compare.rs:320` `let pin = detect::resolve_base_pin(args.base.as_deref(), &overlay)?;` after `load_overlay`. |
| `detect::resolve_base_pin` size-name branch | `spoke_base_resolver::resolve_qwen3_size` | `qwen3_code` QLoRA rows only | ✓ WIRED | `detect.rs:241`. Resolver filters `methods == "qlora"` and tag `qwen3_code`. |
| `pick_base` | `TrainBase.floor_measured` | Rows with `floor_measured: false` are not candidates | ✓ WIRED | `spoke_base_resolver.rs:85` `.filter(\|b\| b.floor_measured && b.floor_mb.is_some_and(\|floor\| floor <= vram_mb))`. |

**Wiring:** 3/3 connections verified

`verify.key-links` reported all three unverified because `from` is a function name rather than a file path (`Source file not found`). Manual read of the call sites shows each link is present. That tool result is not treated as NOT_WIRED.

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `gpu-specs.yaml` `qwen3_code` | `hf_id`, `license`, `floor_measured` | Checked-in overlay rows | Yes — `load_overlay` reads `mens/config/gpu-specs.yaml` | ✓ FLOWING |
| `resolve_qwen3_size` | `Qwen3SizePin` | Matching `qwen3_code` QLoRA row | Yes — pin, revision, license, and flag are copied from the row; a missing license or non-40-hex revision errors | ✓ FLOWING |
| `pick_base` | selected `TrainBase` | Measured rows with `floor_mb <= vram_mb` | Yes — unmeasured 1.7B and 4B are excluded | ✓ FLOWING |
| Compare absence record | `base_model`, `base_revision` | `resolve_base_pin` → `pin.hf_id` | Yes — the 1.7B test expects `70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` from the overlay, not a literal return inside the resolver | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Six sizes resolve to the locked pins; 4B is unmeasured | `cargo test -p vox-populi --features mens --lib -- resolve_qwen3_size_covers_the_dense_ladder qwen3_code_24g_returns_14b_qlora qwen3_code_48g_prefers_unquantized_14b qwen3_code_fail_closed_below_floor resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged unmeasured_pins_are_only_on_qwen3_code -- --test-threads=8` | 6 passed, 224 filtered, exit 0 (11s compile) | ✓ PASS |
| Harness writes the 1.7B and 4B absence records | `size_name_1_7b_records_the_pinned_unmeasured_base`, `size_name_4b_records_the_pinned_unmeasured_base` | Not re-run. Call site and assertions read. Resolver test covers the pin those assertions name. | ? SKIP |

### Probe Execution

No phase-declared probe and no `scripts/*/tests/probe-*.sh` for this phase. Step 7c skipped.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ----------- | ----------- | ------ | -------- |
| SIZE-01 | `317-01-PLAN.md` `requirements: [SIZE-01]` | The ladder pins all six Qwen3 dense sizes (0.6B, 1.7B, 4B, 8B, 14B, 32B), each with a real Hugging Face commit and a license. A rung whose memory floor has not been measured on hardware says so. | ✓ SATISFIED | Same evidence as truths 1 and 2. REQUIREMENTS.md maps `SIZE-01` to Phase 317 only. No other requirement ID is mapped to this phase, and the plan claims no ID besides SIZE-01. |

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `spoke_base_resolver.rs` (`resolve_qwen3_size_*`, `unmeasured_pins_*`, `qwen3_code_24g_*`, `qwen3_code_48g_*`, `qwen3_code_fail_closed_*`) | SIZE-01 | Yes | 0 | No — expected pins are the locked literals, compared to `load_overlay` | Value | PASS |
| `reasoning_compare/detect.rs` (`resolve_qwen3_size_rejects_unknown_spellings`, `resolve_base_pin_leaves_capability_tag_unpinned`) | SIZE-01 | Yes | 0 | No | Value | PASS (read, not re-run) |
| `reasoning_compare.rs` (`size_name_1_7b_*`, `size_name_4b_*`) | SIZE-01 | Yes | 0 | No | Value | PASS (read, not re-run) |
| `training_presets_yaml_contract.rs` | SIZE-01 | Yes — skips `!floor_measured` before `KNOWN_PARAMS_B` | 0 | No | Value on measured floors | PASS (read, not re-run) |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0
**Insufficient assertions:** 0 blocking. `qwen3_code_24g_returns_14b_qlora` asserts the repo name, not `methods == qlora`. At 24000 MB the LoRA row's `floor_mb` is 44000, so it cannot win. The 48 GB test asserts the LoRA methods.

### Decision Coverage

No trackable decisions in CONTEXT.md. `workflow.context_coverage_gate` is unset, so the gate is enabled. `check.decision-coverage-verify` returned `skipped: true`, `total: 0`, `honored: 0`. The markdown decisions in `317-CONTEXT.md` are not in the trackable entry shape the gate parses. Non-blocking.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `mens/config/gpu-specs.yaml` | 325 | `TODO: B6.0 vLLM compat spike` | ℹ️ Info | Pre-existing comment outside `qwen3_code`. Not `TBD` / `FIXME` / `XXX`. Not introduced by the 1.7B or 4B rows. |
| `spoke_base_resolver.rs` | 324–326, 546 | `PLACEHOLDER` | ℹ️ Info | Guard that rejects an unpinned revision. Not a debt marker. |

No `TBD`, `FIXME`, or `XXX` in the phase files. Commits `2d6aea459` and `3592ec1e6` both exist (`verify.commits` `all_valid: true`).

### Human Verification Required

N/A — infrastructure phase. The ladder is a checked-in overlay plus a resolver the trainer loads and the harness calls. Acceptance is the pins, the `floor_measured` flag, and `pick_base` selection. Those were checked in the file and by the six named tests above. No user-facing surface was added.

### Gaps Summary

None. The six dense sizes are licensed `qwen3_code` rungs. 1.7B and 4B use the locked commits, omit `floor_mb`, and cannot win `pick_base`. Naming goes through `resolve_qwen3_size`, which the compare entry calls.

---

_Verified: 2026-10-29T04:02:57Z_
_Verifier: gsd-verifier_
