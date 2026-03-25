# Mens Candle QLoRA (qlora-rs NF4, cuda:0)

## Base model
Qwen/Qwen2.5-Coder-3B-Instruct

## Data
- train file: `\\?\C:\Users\Owner\vox\target\dogfood\train_mixed.jsonl`

## Architecture
- vocab: 151936
- d_model: 2048
- layers: 36
- heads: 16

## Notes
Frozen embed key `model.embed_tokens.weight` (f32 mmap for context); stacked NF4 projections + LM head via qlora-rs (ADR 006; bounded proxy v1).
Suffix CE: `--qlora-ce-last-k` = 1 (last K positions per row).
Middle stack active: 36 / 36 model `o_proj` slots; shard keys loaded: 36
LoRA adapter v2: C:\Users\Owner\vox\mens\runs\qwen25_qlora\candle_qlora_adapter.safetensors
Sidecar v2: C:\Users\Owner\vox\mens\runs\qwen25_qlora\candle_qlora_adapter_meta.json
Adapter manifest v3: C:\Users\Owner\vox\mens\runs\qwen25_qlora\populi_adapter_manifest_v3.json
Training manifest: C:\Users\Owner\vox\mens\runs\qwen25_qlora\training_manifest.json
