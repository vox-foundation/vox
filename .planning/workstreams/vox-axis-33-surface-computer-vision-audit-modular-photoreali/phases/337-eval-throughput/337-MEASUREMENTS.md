# Phase 337 Measurements & Parity Record

**Measured:** 2026-11-13
**Command:** `timeout 900s cargo test -p vox-plugin-mens-candle-metal --lib`

## Unit & Metal Parity Verification

- Total unit suite (`vox-plugin-mens-candle-metal --lib`): **89 passed; 0 failed; 3 ignored** (`15.05s` wall-clock after build).
- Ignored tests are `inference::live_metal_tests::*` (`base_only_load_runs_real_inference_with_no_adapter_present`, `generation_sustains_a_usable_token_rate`, `the_demo_config_eos_is_read_from_disk_not_hardcoded`), which require a full local Hugging Face base-model directory on disk.
- Verified parity tests:
  - `batch_decode::tests::batched_greedy_decode_is_token_identical_to_batch_one_cpu` — **ok**
  - `batch_decode::tests::batched_greedy_decode_is_token_identical_to_batch_one_metal` — **ok**
  - `batch_decode::tests::batched_bf16_padded_decode_is_token_identical_to_batch_one_metal` — **ok**
  - `batch_decode::tests::bf16_protocol_bounds_batch_width_by_gemm_rows` — **ok**
  - `batch_decode::tests::step_times_records_mean_ms` — **ok**
  - `model::metal_backward_tests::into_inference_matches_qlora_padded_output_for_2d_and_3d_inputs` — **ok**
  - `model::metal_backward_tests::into_inference_rejects_nonzero_lora_b` — **ok**
  - `model::metal_backward_tests::metal_nf4_gemv_batch_is_bitwise_identical_to_m1` — **ok**
  - `model::metal_backward_tests::metal_bf16_batched_equals_single` — **ok**
  - `model::metal_backward_tests::metal_no_grad_cached_decode_matches_eager_reference` — **ok**
