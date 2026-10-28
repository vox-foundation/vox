# Phase 320 — Stage S4: full training runs (D-07)

**Host:** Apple Silicon GPU (Metal). **Binaries:** `vox` / `vox-ml-cli` `c56e13a33` (Metal buffer-pool fix). **Corpus:** `target/dogfood/train_mixed.jsonl` (1166 rows), `--seq-len 3072`, `--epochs 1`.

## Commands

```text
./target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu \
  --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal \
  --model Qwen3-<S> --output-dir mens/runs/qwen3-<s>
```

One size at a time. No `--max-runtime-secs` for 0.6B / 1.7B. Non-fitting sizes did not run.

## Outcomes

| size | status | preset | opt / micro | final_loss_ema | peak MB | wall h | trunc / trim | adapter | pin check |
|---|---|---|---|---|---|---|---|---|---|
| 0.6B | trained | qwen3_dev_cpu | 1108 / 1108 | 0.690 | 15,319 | 2.82 | 34 / 34 | `mens/runs/qwen3-0.6b/candle_qlora_adapter.safetensors` | pin `c1899de289…` in summary + manifest path |
| 1.7B | trained | qwen3_6g | 138 / 1108 | 0.617 | 27,007 | 4.17 | 34 / 34 | `mens/runs/qwen3-1.7b/candle_qlora_adapter.safetensors` | pin `70d244cc86…` in summary + manifest path |
| 4B | not_trained | qwen3_12g | — | — | 84,541 (S3 probe) | — | — | none | probe OOM / watchdog |
| 8B | not_trained | qwen3_16g | — | — | — | — | — | none | above non-fitting 4B (D-06) |
| 14B | not_trained | qwen3_24g | — | — | — | — | — | none | above non-fitting 4B (D-06) |
| 32B | not_trained | qwen3_96g | — | — | — | — | — | none | above non-fitting 4B (D-06) |

Evidence: `evidence/s4-record-<S>.json`, `evidence/s4-loss-{0.6B,1.7B}.json`, `evidence/s4-{0.6B,1.7B}-log.txt`, exit files `0`.

## Findings

- **Metal pool fix paid off on 1.7B.** S3 probe peak was 54,583 MiB; S4 full-epoch peak was **27,007 MiB** on the same preset after `c56e13a33`. Floor updates (Plan 08) should use `max(probe, S4)` per D-08, so 54,583 remains the conservative measured floor until a re-probe on the fixed binary.
- Loss fell on both trained sizes (`s4-loss-*.json` first-quarter mean > last-quarter mean).
- No divergence, no mid-run OOM on fitting sizes.

Stage S4: PASS
