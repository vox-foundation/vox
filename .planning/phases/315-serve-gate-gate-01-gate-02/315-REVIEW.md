---
phase: 315-serve-gate-gate-01-gate-02
reviewed: 2026-10-26T18:30:00Z
depth: standard
files_reviewed: 6
files_reviewed_list:
  - crates/vox-eval/src/reasoning_slice.rs
  - crates/vox-eval/src/lib.rs
  - crates/vox-ml-cli/src/commands/mens/eval_collateral.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
  - docs/src/how-to/how-to-train-mens-macos-metal.md
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 315: Code Review Report

**Reviewed:** 2026-10-26T18:30:00Z
**Depth:** standard
**Files Reviewed:** 6
**Status:** issues_found

## Summary

Reviewed the six listed files as committed on `cursor/t3-deeper-extraction-v55-510f`. The working tree for these six files matches HEAD (`git diff HEAD` is empty for them). `lib.rs` only registers `pub mod reasoning_slice;`. The 26 `vox-eval` `reasoning_slice` tests pass.

No blockers. The phase goal holds on the traced paths:

- **Missing report:** serve refuses. The call site is `gpu`-gated like the function, so there is no cfg gap.
- **Old-format report:** serve refuses, because the `reasoning_slices` block is required.
- **Drop on either key:** serve recomputes from the stored numbers and refuses. The 5% boundary is exact for k/32 scores through the JSON round trip.
- **Zero, non-finite or out-of-range score, or `n` other than 32:** refused by `guard` and `eval_reasoning_slice`.
- **Failed run:** the old report is removed before anything can fail, and the report is written last, so no stale pass survives a failed run.
- **Slice label:** the report carries `scope_note`, `slice_size` and per-slice `n`.

The accepted locks were not treated as defects. These were not flagged: no model run, no numeric baseline, other adapter filenames, no numeric equality with the slice reports, generic zero-pre behavior, out-of-phase clippy failures, and the 3-test count.

The findings below are robustness and honesty gaps around the gate, not logic errors in it.

## Warnings

### WR-01: How-to claims a hand-edited `pass` is refused, but only inconsistent edits are

**File:** `docs/src/how-to/how-to-train-mens-macos-metal.md:80-85`
**Issue:** The text says the gate "does not trust status alone, so a hand-edited or old-format `pass` is refused." That is true for a flipped `status` over real numbers, and for an old-format report. It is not true for an edit that also rewrites `pre` and `post`. `verify_reasoning_slices` (`reasoning_slice.rs:196-271`) recomputes from whatever numbers are stored. Nothing ties them to the baseline file or to the per-slice reports, which is the accepted D-10 lock. The tests only edit `post` while leaving the stored verdict claiming "no drop". An operator reading the how-to would take this as tamper resistance, and it is a consistency check only.
**Fix:** Narrow the claim to what is enforced:
```md
It does not trust the stored `status` or verdict fields: the per-slice drops are
recomputed from the stored `pre`/`post` numbers, so a `pass` that contradicts
those numbers, or an old-format report, is refused. This is a consistency check,
not tamper protection: the numbers themselves are not re-measured at serve time.
```

### WR-02: The token cap that shapes the score is not recorded, so the comparison's validity can't be checked

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs:115-124, 238-259`
**Issue:** `VOX_REASONING_EVAL_MAX_TOKENS` changes what is measured, in both directions. `n.max(1)` accepts `-5` or `0` as a 1-token cap. A larger-than-baseline cap lifts accuracy and think-initiation by letting truncated completions finish. `write_outcome` records `temperature` and `prompt` but not `max_tokens`. The collateral report does not record it either. A post-run under a different cap than the baseline therefore produces a valid-looking, `n`=32 `pass` that serve accepts. A lower cap fails safe, but a higher one can mask a real regression. The gate exists to detect regressions, so a silent change of measurement is in scope.
**Fix:** Add `"max_tokens": max_tokens` to the scored body in `write_outcome` (thread it through `ReasoningReport` or the outcome). Surface it in the collateral `reasoning_slices` block. Consider rejecting a value below 1 instead of clamping it.

### WR-03: Serve compares `scope_note` by exact text, so any rewording refuses every existing adapter with a misleading error

**File:** `crates/vox-eval/src/reasoning_slice.rs:17, 221-224`
**Issue:** `verify_reasoning_slices` requires the stored `scope_note` to equal the current `REASONING_SLICE_SCOPE_NOTE` byte for byte. Editing the constant, even to fix a typo, makes every previously valid report fail with "scope_note is missing or not the expected label". That reads as tampering or an old format. The gate error tells the operator to re-run an expensive eval. Wording is a presentation concern and has become a compatibility key.
**Fix:** Gate on the structured fields already stored (`slice_size`, per-slice `n`, `benches`) and require only that `scope_note` is a non-empty string. Or add a `label_version` integer and compare that, leaving the prose free to change.

### WR-04: `an_out_of_range_stored_number_is_refused` cannot detect removal of the range guard

**File:** `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:1161-1178`
**Issue:** The test swaps `0.123456789` for `1e999`. serde_json rejects that literal at parse time, so the refusal comes from "report is not valid JSON". It does not come from `guard`'s `[0, 1]` check. The only assertion is `e.contains("eval_collateral_damage")`, which every refusal in `assert_collateral_gate` contains. Deleting the range guard, or the whole `verify_reasoning_slices` call, would leave this test green. Range handling is covered one layer down by `an_accuracy_above_one_is_refused`, but this test's name claims the serve boundary is covered.
**Fix:** Use a finite out-of-range value and assert the message from `guard`:
```rust
edit_report(
    dir.path(),
    "/reasoning_slices/benches/gsm8k_think/accuracy/post",
    serde_json::json!(1.5),
);
let e = gate_err(dir.path());
assert!(e.contains("score outside [0, 1]"), "{e}");
```
Keep the `1e999` case as a separate test named for what it checks (unparseable report).

## Info

### IN-01: A zero or invalid reasoning baseline is rejected only after every bench has been scored

**File:** `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs:89-126`
**Issue:** `require_reasoning_keys` checks key presence before scoring. The `pre` value checks (zero, above 1) live in `eval_reasoning_slice`, which runs after the `score_bench` loop at lines 91-95. A baseline with `"math500_think": 0.0` still pays for the full 3x32 thinking-mode generations before refusing. `a_zero_reasoning_baseline_refuses_and_writes_no_report` (line 548) does not assert the scorer call count, so the ordering is untested.
**Fix:** After `require_reasoning_keys`, check each reasoning `pre` value (finite, in `(0, 1]`) before the scoring loop. Assert `calls == 0` in that test.

### IN-02: Serve's refusal text tells the operator to re-run the eval even when the drop is real

**File:** `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:832-835`
**Issue:** Every `verify_reasoning_slices` error is wrapped with "Re-run vox mens eval-collateral-damage for this adapter." That is right for a stale or old-format report. It is wrong for a recomputed degradation, such as "gsm8k_think dropped 0.1000 ... past the 0.05 bar". There a re-run reproduces the failure, and the adapter needs retraining or a re-baseline. The existing `FAILED.` branch correctly says the adapter cannot be served.
**Fix:** Append the re-run advice only for structural errors, or word it as "Re-run ... if this report predates the 32-item slice block; otherwise the adapter degraded and cannot be served."

### IN-03: `score_bench` applies the `_init` reasoning lookup to any bench name

**File:** `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs:221-243`
**Issue:** Any baseline key ending in `_init` is mapped to `mens/data/<base>/reasoning.jsonl`, not just the three names in `SLICE_NAMES`. A non-slice key whose base directory holds a `reasoning.jsonl` would be scored as a think-initiation rate but compared through the generic path, where the zero-pre pass still applies. This is latent because no such bench exists today. It also runs against the CONTEXT decision that reasoning keys get strict handling and generic keys keep the old path. `reasoning_slice_of`, which classifies by name only, already exists.
**Fix:** Use `reasoning_slice_of(bench)` to choose the reasoning branch and fall through to the generic eval-local path for everything else.

---

_Reviewed: 2026-10-26T18:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
