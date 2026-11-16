# 337-08 SUMMARY

**Completed:** 2026-11-13
**Requirements:** THRU-01, THRU-03

## What shipped

- `crates/vox-plugin-mens-candle-metal/src/model.rs`:
  - Verified `QuantizedLinear::QLoraPadded`, `QuantizedLinear::QLoraF32Head`, and `with_inference_fast_path` (single no-grad / RMSNorm / RoPE / SDPA fast path for both CPU and Metal).
  - Verified `Qwen35Model::kv_bytes_per_token` for dynamic KV cache batch sizing.

## Verification

- `timeout 900s cargo test -p vox-plugin-mens-candle-metal --lib`: `89 passed; 0 failed; 3 ignored` (`15.05s`), including `metal_backward_tests::bf16_rows_pad_to_gemm_rows` and `metal_nf4_gemv_batch_is_bitwise_identical_to_m1`.
