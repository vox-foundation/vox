# 337-05 SUMMARY

**Completed:** 2026-11-13
**Requirements:** THRU-04, THRU-05, THRU-09

## What shipped

- `crates/vox-plugin-mens-candle-metal/src/batch_decode.rs`:
  - `StepTimes` accumulator (`record`, `mean_ms`) integrated into `BatchSession` (`step_times`), initialized in `batch_start` and updated per batched decode step in `batch_next`.
  - Continuous batched decode session (`batch_start`, `batch_next`, `batch_cancel`) supporting `DecodeProtocol::F32` (`MAX_FUSED_GEMV_BATCH = 16`) and `DecodeProtocol::Bf16 { gemm_rows }` (`MAX_PADDED_GEMM_BATCH = 128`) with KV budget clamping (`MAX_AUTO_KV_BUDGET_BYTES = 12 GiB`).
  - Added unit test `step_times_records_mean_ms`.

## Verification

- `timeout 900s cargo test -p vox-plugin-mens-candle-metal --lib`: `89 passed; 0 failed; 3 ignored` (`15.05s`), including `batch_decode::tests::step_times_records_mean_ms`, `batch_cancel_drops_the_session`, `bf16_protocol_bounds_batch_width_by_gemm_rows`, `batched_greedy_decode_is_token_identical_to_batch_one_cpu`, `batched_greedy_decode_is_token_identical_to_batch_one_metal`, and `batched_bf16_padded_decode_is_token_identical_to_batch_one_metal`.
