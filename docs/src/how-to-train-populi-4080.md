---
title: "How To: Train Populi on RTX 4080 Super"
category: how-to
constructs: [function, workflow]
last_updated: 2026-03-20
training_eligible: true
difficulty: intermediate
---

# How To: Train Populi on RTX 4080 Super

This runbook is the canonical first-time path to train Populi locally on Windows with PowerShell, using Vox native training.

## Recommended Path

- Default: `vox populi train --data-dir target/dogfood --output-dir populi/runs/v1`
- Input contract: `target/dogfood/train.jsonl`
- Backend: `wgpu` on Windows (Vulkan or DX12); no CUDA/Python required
- Fallback only: Python QLoRA for large-model workflows that need 4-bit quantization

## Prerequisites

1. Build Vox CLI (release binary):
   ```powershell
   & "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p vox-cli --release
   ```
2. Generate canonical corpus input:
   ```powershell
   New-Item -ItemType Directory -Force -Path populi/data,target/dogfood | Out-Null
   .\target\release\vox.exe populi corpus extract examples/ -o populi/data/validated.jsonl
   .\target\release\vox.exe populi corpus extract docs/ -o populi/data/validated.jsonl 2>$null
   .\target\release\vox.exe populi corpus validate populi/data/validated.jsonl --no-recheck -o populi/data/validated.jsonl
   .\target\release\vox.exe populi corpus pairs populi/data/validated.jsonl -o target/dogfood/train.jsonl --docs docs/src/ --docs docs/src/research/ --docs docs/src/adr/
   # Rustdoc merge skipped: response is Rust prose, not Vox code
   ```
3. Optional GPU backend selection:
   ```powershell
   $env:VOX_BACKEND = "vulkan"   # or "dx12" or "cpu"
   ```
4. Optional training profile (RTX 4080 Super 16GB VRAM):
   ```powershell
   $env:VOX_TRAIN_PROFILE = "safe"   # Conservative: batch 2, seq 256 (shared GPU, avoids OOM)
   # $env:VOX_TRAIN_PROFILE = "balanced"  # Default for 16GB: batch 4, seq 512, rank 16
   # $env:VOX_TRAIN_PROFILE = "throughput" # Aggressive: batch 6 (may OOM if OS uses GPU)
   ```
   Device probe auto-detects 16GB and recommends batch 4, seq 512, rank 16. Use `vox populi probe` to verify.

## Full mixed corpus → entire LoRA run (4080 preset)

Use this when you want **all sources** from `populi/config/mix.yaml` (not a tiny dogfood slice).

1. **Build** with GPU training enabled (native LoRA + eval-local inference):
   ```powershell
   & "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p vox-cli --release --features gpu
   ```
   If this fails, restore missing `vox-cli` modules (e.g. `commands/ai/checkpoint.rs`) and fix any `--features gpu` compile errors before training.

2. **Mix** into the default mix output path:
   ```powershell
   .\target\release\vox.exe corpus mix --config populi/config/mix.yaml
   ```
   Writes `target/dogfood/train_mixed.jsonl` per mix config.

3. **Point training** at that file as `train.jsonl` (preflight requires this exact name inside `--data-dir`):
   ```powershell
   New-Item -ItemType Directory -Force -Path target/dogfood | Out-Null
   Copy-Item -Force target/dogfood/train_mixed.jsonl target/dogfood/train.jsonl
   ```

4. **Train** with the SSOT 4080 profile (`populi/config/train-presets.yaml` preset `4080`):
   ```powershell
   .\target\release\vox.exe populi train `
     --preset 4080 `
     --model Qwen/Qwen2.5-Coder-3B-Instruct `
     --data-dir target/dogfood `
     --output-dir populi/runs/rtx4080_full `
     --log-dir populi/runs/logs
   ```
   Tail `populi/runs/logs/train_*.log` until epochs complete. On OOM, use `--preset safe` or reduce `--batch-size` / `--seq-len` per [runs/logs README](../../../populi/runs/logs/README.md).

## First Training Run (Native)

```powershell
.\target\release\vox.exe populi train --data-dir target/dogfood --output-dir populi/runs/v1
```

Or run the end-to-end automation script:

```powershell
.\scripts\run_populi_pipeline.ps1 -DataDir target/dogfood -OutputDir populi/runs/v1 -Backend vulkan
```

Expected outputs:

- `populi/runs/v1/model_final.bin`
- `populi/runs/v1/checkpoint_epoch_*.bin`
- `populi/runs/v1/eval_results.json`
- `populi/runs/v1/benchmark_results.json` (if benchmark gate enabled)

## Quality Gates

- Eval thresholds:
  - `VOX_EVAL_MIN_PARSE_RATE` (default `0.80`)
  - `VOX_EVAL_MIN_COVERAGE` (default `0.60`)
- Strict enforcement:
  - `VOX_EVAL_STRICT=1` to fail run on threshold miss

```powershell
.\target\release\vox.exe populi corpus eval target/dogfood/train.jsonl -o populi/runs/v1/eval_results.json
```

## Runtime Profiles

- Fast dogfood:
  - 1 epoch, smaller dataset while iterating on pipeline code/docs
- Full run:
  - Full corpus + rustdoc merge and benchmark gate enabled

## Model Card

After training, the model card is rendered from `populi/model_card/`:

```powershell
uv run --project scripts render-model-card --run-dir populi/runs/v1
```

## See Also

- [Native ML Training Pipeline](expl-ml-pipeline.md)
- [How To: Publish Populi to Hugging Face](how-to-publish-populi-hf.md)
- [scripts/README.md](../../scripts/README.md) - Python QLoRA fallback details
