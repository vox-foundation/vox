# Mens Candle QLoRA (qlora-rs NF4, cpu)

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
Suffix CE: `--qlora-ce-last-k` = 8 (last K positions per row).
Middle stack active: 36 / 36 model `o_proj` slots; shard keys loaded: 36
LoRA adapter v2: mens/runs/v1\candle_qlora_adapter.safetensors
Sidecar v2: mens/runs/v1\candle_qlora_adapter_meta.json
Adapter manifest v3: mens/runs/v1\populi_adapter_manifest_v3.json
Training manifest: mens/runs/v1\training_manifest.json
