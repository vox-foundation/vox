# 337-06 SUMMARY

**Completed:** 2026-11-13
**Requirements:** THRU-02, THRU-05

## What shipped

- `patches/qlora-rs-1.0.5/src/qlora.rs`:
  - Added `cached_weight(&self) -> Option<&Tensor>`, `bias(&self) -> Option<&Tensor>`, and `is_lora_zero(&self) -> bool` on `qlora_rs::qlora::QuantizedLinear`.
- `crates/vox-plugin-mens-candle-metal/src/model.rs`:
  - Added `QuantizedLinear::Dense { weight_t, bias }` supporting 2D and 3D matmul forward with automatic dtype alignment and optional bias broadcast-add.
  - Added `QuantizedLinear::into_inference(self) -> Result<Self>` verifying zero `LoRA B` weights before collapsing cached weights into pre-transposed contiguous `Dense` layers.
  - Added `into_inference(&mut self) -> Result<usize>` across `Qwen2Attention`, `Qwen2MLP`, `Qwen35LinearAttention`, `Qwen35Layer`, and `Qwen35Model`.
  - Added unit tests `into_inference_matches_qlora_padded_output_for_2d_and_3d_inputs` and `into_inference_rejects_nonzero_lora_b`.

## Verification

- `timeout 900s cargo test -p vox-plugin-mens-candle-metal --lib`: `89 passed; 0 failed; 3 ignored` (`15.05s`).
