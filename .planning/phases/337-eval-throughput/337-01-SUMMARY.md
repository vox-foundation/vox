# 337-01 SUMMARY

**Completed:** 2026-11-07
**Requirements:** THRU-07, THRU-10 (foundation)

## What shipped

- `contracts/config/env-vars.v1.yaml`: Added entries for `VOX_MENS_DECODE_PROTOCOL` and `VOX_MENS_DECODE_BF16_GEMM_ROWS` (target edit: exactly +12 lines, 0 deleted).
- `crates/vox-eval/src/decode_protocol.rs`:
  - `DecodeProtocol`: `F32` (default) and `Bf16 { gemm_rows: usize }`.
  - `DecodeSettings`: parses environment (`VOX_MENS_DECODE_PROTOCOL`, `VOX_MENS_DECODE_BF16_GEMM_ROWS`) and records checkpoint and report fields.
  - Serialization helpers: `checkpoint_protocol_fields` (`decode_protocol: "bf16"`, `bf16_gemm_rows: 32` when bf16; omitted when f32) and `report_object` (`decode_protocol: "f32"` or `{"bf16": {"gemm_rows": 32}}`).
  - Strict parsing: unknown protocol tags error naming `f32` and `bf16`; non-positive `gemm_rows` errors; `f32` with `gemm_rows` errors.
- `crates/vox-eval/src/sequential_mcnemar.rs` & `sequential_mcnemar_tests.rs`:
  - `SequentialRule`: construct via `new` or `from_fractions` (with slice size `n`), enforces strict monotonicity and validity intervals.
  - `worst_case_size`: exact depth-first search over discordant-pair increment paths with closed-form binomial evaluations at the final look, bounding overall false-rejection rate under $H_0$.
  - `check_size`: asserts worst-case size $\le \alpha + 10^{-9}$.
  - `judge`: incremental evaluation over pairs of responses yielding `Loss`, `Gain`, or `Tie`. Stops at the earliest look where $p < \alpha_k$, returning `SequentialVerdict`.
  - Pre-registered schedule constants `SEQUENTIAL_RULE` and `SEQUENTIAL_RULE_ID` (`sequential_mcnemar_v1`).

## Verification

- `cargo test -p vox-eval --lib -- decode_protocol sequential_mcnemar`: 20 passed, 0 failed in 2.79s.
- `cargo test -p vox-eval`: 212 passed, 0 failed, 4 ignored in 3.15s.
- Exhaustive brute-force cross-check (`brute_force_cross_check_equals_worst_case_size`) proves exact equality ($< 10^{-12}$) between the DP algorithm and exhaustive enumeration of all $2^{12} = 4096$ paths.
- Pre-registered schedule test (`preregistered_schedule_worst_case_size`) confirms exact worst-case false-rejection size of $0.04884 \le 0.05$ (within $10^{-5}$) with execution runtime of ~0.65s in debug mode (well under the 60s threshold).

## Mutation Proofs

All five required mutation proofs were executed against `crates/vox-eval/src/sequential_mcnemar.rs`, observed failing on the specified tests, restored, and verified with identical before/after presence counts:

1. **Interim look alpha loosened to `self.alpha`** (`look_alpha[k]` -> `self.alpha`):
   - Before presence count (`grep -c 'let a = self.look_alpha\[k\];'`): 1
   - Observed test failure: `preregistered_schedule_worst_case_size` failed (computed size 0.12278 > 0.04884), `two_look_and_pocock_worst_case_size` failed, `brute_force_cross_check_equals_worst_case_size` failed.
   - After presence count: 1
2. **Rejection comparison loosened to `<` -> `<=`** (`p < self.look_alpha[look_idx]` -> `p <= ...`):
   - Before presence count (`grep -c 'p < self.look_alpha\[look_idx\]'`): 1
   - Observed test failure: `rejection_comparison_is_strictly_less` failed (`assert!(!rule.rejects_at(0, 1, 0))` panicked because $p=0.5 \le 0.5$).
   - After presence count: 1
3. **Final look skipped in `worst_case_size`** (skipping step when `k == K - 1`):
   - Before presence count (`grep -c 'if k == self.looks.len() - 1'`): 1
   - Observed test failure: `preregistered_schedule_worst_case_size` failed (computed size 0.00650 vs expected 0.04884).
   - After presence count: 1
4. **Previous look's total discordant count passed to `critical_losses`** (`d` instead of `d_k`):
   - Before presence count (`grep -c 'Self::critical_losses(d_k, self.look_alpha\[k\])'`): 1
   - Observed test failure: `brute_force_cross_check_equals_worst_case_size` failed (DP size 0.9375 != 0.03906).
   - After presence count: 1
5. **`stopped_at_look` 0-based instead of 1-based** (`look_idx + 1` -> `look_idx`):
   - Before presence count (`grep -c 'stopped_at_look: Some(look_idx + 1)'`): 1
   - Observed test failure: `judge_stops_at_look_1` failed (`Some(0)` vs `Some(1)`) and `judge_stops_at_look_4` failed (`Some(3)` vs `Some(4)`).
   - After presence count: 1
