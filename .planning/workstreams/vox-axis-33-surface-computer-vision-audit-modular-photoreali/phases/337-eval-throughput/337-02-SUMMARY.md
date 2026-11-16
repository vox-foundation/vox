# Phase 337 Plan 02 Summary: Protocol identity from env to checkpoint key, plugin load, decode object in reports, baseline protocol refusal

## Overview
Implemented protocol identity end-to-end:
- Environment variable `VOX_MENS_DECODE_PROTOCOL` (`f32` default, `bf16`) parsed into `DecodeProtocol` and `DecodeSettings`.
- `reasoning_inputs` in `item_checkpoint.rs` appends `checkpoint_protocol_fields()` (`[]` for f32 to preserve golden hashes byte-for-byte; `[("decode_protocol", "bf16"), ("bf16_gemm_rows", N)]` for bf16).
- Metal inference plugin loads protocol from env via `supported_protocol` (accepts `f32`, refuses `bf16` with explicit pointer to 337-11), printing `protocol <tag>` in load line.
- Top-level `decode` object added to `eval-reasoning-compare` and `eval-collateral-damage` reports via `DecodeSettings::report_object()`.
- `baseline_from_compare` propagates `decode` into pre-score JSON when present.
- `eval_collateral.rs` enforces `require_baseline_protocol`, refusing mismatched baseline protocols before any model load or scoring.

## Real Checkpoint Cross-Check
Verified against `mens/runs/reasoning_compare/checkpoints/1ab761b1df380809/gsm8k_think.jsonl`:
- Header line `key` is `82ec042040fa9d1996ac6b4b8b6f30bd1d2a247961374d130762c576a5d26064`.
- Recomputing `hash_json` of stored inputs confirms bitwise match.

## Mutation Proofs
1. **Plugin load refusal (`supported_protocol`)**:
   - `grep -c 'fn supported_protocol' crates/vox-plugin-mens-candle-metal/src/inference.rs`: 2 before, 2 after.
   - Mutated `supported_protocol` to accept `Bf16` without error -> `supported_protocol_accepts_f32_and_refuses_bf16` FAILED (`unwrap_err` on `Ok`).
   - Restored and verified test passes.
2. **Golden-key guard (`the_f32_checkpoint_key_is_unchanged`)**:
   - Mutated `checkpoint_protocol_fields()` to append `[("decode_protocol", "f32")]` for f32 -> `the_f32_checkpoint_key_is_unchanged` FAILED (hash mismatch vs golden `febfd9768573f761620373d72b53778b31a3ecd8fe8bb1944c7b4fd2a277e675`).
   - Restored and verified test passes.
3. **Baseline-protocol refusal (`require_baseline_protocol`)**:
   - `grep -c 'fn require_baseline_protocol' crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`: 1 before, 1 after.
   - Mutated `require_baseline_protocol` to return `Ok(())` unconditionally -> `a_baseline_from_another_protocol_is_refused` FAILED (`unwrap_err` on `Ok(0)`).
   - Restored and verified test passes.

## Notice for Orchestrator (Phase 338-02 Amendment)
Per D-24, batch width is deliberately omitted from `DecodeSettings::report_object()`, as batch width is throughput-only and does not alter greedy deterministic decode output. 338-02's `decode_blake3` must hash the top-level `decode` object whose keys are `protocol`, (optional `bf16_gemm_rows`), `max_tokens`, `temperature`, `seed`, `thinking`. 338-02's requirement referencing "batch size" in the decode hash must be amended accordingly.
