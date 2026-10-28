# Phase 320 — Stage S4: full training runs (D-07)

Recorded 2026-10-28 (America/Phoenix). One GPU job at a time. Binaries `vox` / `vox-ml-cli` / Metal plugin at `c56e13a33` (Metal private-pool LoRA-grad fix).

## Commands

```text
# 0.6B (Task 1)
target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu \
  --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal \
  --model Qwen3-0.6B --output-dir mens/runs/qwen3-0.6b

# 1.7B (Task 2)
target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu \
  --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal \
  --model Qwen3-1.7B --output-dir mens/runs/qwen3-1.7b
```

No `--max-runtime-secs` (uncapped full pass). No `--force-restart`.

## Outcomes

| size | status | preset | opt / micro | final loss EMA | peak MB | wall h | trunc / prompt-trim | adapter | manifest pin check |
|------|--------|--------|-------------|----------------|---------|--------|---------------------|---------|--------------------|
| 0.6B | trained | qwen3_dev_cpu | 1108 / 1108 | 0.690 | 15,319 | 2.82 | 34 / 34 | `mens/runs/qwen3-0.6b/candle_qlora_adapter.safetensors` | snapshot `c1899de289…` |
| 1.7B | trained | qwen3_6g | 138 / 1108 | 0.617 | 27,007 | 4.17 | 34 / 34 | `mens/runs/qwen3-1.7b/candle_qlora_adapter.safetensors` | snapshot `70d244cc86…` |
| 4B | not_trained | qwen3_12g | — | — | 84,541 (S3 probe) | — | — | none | — |
| 8B | not_trained | qwen3_16g | — | — | — | — | — | none | above 4B (D-06) |
| 14B | not_trained | qwen3_24g | — | — | — | — | — | none | above 4B (D-06) |
| 32B | not_trained | qwen3_96g | — | — | — | — | — | none | above 4B (D-06) |

Loss quarters (`evidence/s4-loss-*.json`): 0.6B 1.069 → 0.737; 1.7B 1.193 → 0.681.

## Findings

- **Pool fix paid off on 1.7B.** S3 probe at accum 8 peaked **54,583 MB**; the full S4 epoch peaked **27,007 MB** on the same preset after `c56e13a33` (LoRA grads kept off the Metal private pool).
- **4B remains not trained** at preset accum 8 (S3 watchdog). No `--grad-accum 1` override was used (D-06 / S3 rule).
- Sparse agent checkins (~20–30 min) monitored both runs; no null-loss divergence.

Evidence: `evidence/s4-record-*.json`, `evidence/s4-loss-{0.6B,1.7B}.json`, `evidence/s4-{0.6B,1.7B}-{exit,log.txt}`, monitor logs under `evidence/s4-*-monitor.log`.

Stage S4: PASS
