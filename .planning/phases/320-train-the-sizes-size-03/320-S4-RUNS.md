# Phase 320 — Stage S4: full training runs (D-07)

Recorded 2026-10-28 (America/Phoenix); 4B added 2026-10-30. One GPU job at a time. Binaries `vox` / `vox-ml-cli` / Metal plugin at `c56e13a33` (Metal private-pool LoRA-grad fix).

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
| 4B | trained (2026-10-30, forced estimate, one resume) | qwen3_12g | 138 / 1108 | 0.543 (EMA of the 28 resumed rows) | 34,460 | 10.96 train / 11.31 elapsed | 0 / 0 (resumed rows only) | `mens/runs/qwen3-4b/candle_qlora_adapter.safetensors` | pin `1cfa9a72…` |
| 8B | not_trained | qwen3_16g | — | — | — | — | — | none | above 4B (D-06) |
| 14B | not_trained | qwen3_24g | — | — | — | — | — | none | above 4B (D-06) |
| 32B | not_trained | qwen3_96g | — | — | — | — | — | none | above 4B (D-06) |

Loss quarters (`evidence/s4-loss-*.json`): 0.6B 1.069 → 0.737; 1.7B 1.193 → 0.681; 4B 1.091 → 0.603.

## Findings

- **Pool fix paid off on 1.7B.** S3 probe at accum 8 peaked **54,583 MB**; the full S4 epoch peaked **27,007 MB** on the same preset after `c56e13a33` (LoRA grads kept off the Metal private pool).
- **4B was trained on 2026-10-30** after the post-fix re-probe showed it fits (user decision: "Yes, train 4B"). See the 4B section below. The pre-fix S3 watchdog stop no longer applies. No `--grad-accum` override was used.
- Sparse agent checkins (~20–30 min) monitored both runs; no null-loss divergence.

Evidence: `evidence/s4-record-*.json`, `evidence/s4-loss-{0.6B,1.7B}.json`, `evidence/s4-{0.6B,1.7B}-{exit,log.txt}`, monitor logs under `evidence/s4-*-monitor.log`.

## 4B full run (2026-10-30)

Binaries: `target/release/vox` `c56e13a33`; the installed Metal plugin and `target/release/libvox_plugin_mens_candle_metal.dylib` are identical (`diff -q`), sha256 `cdaf90fa…bb17` for both, which is the c56e13a33 plugin. `vox-ml-cli` was `3ec31a00f` during training. Nothing else was training or evaluating at launch.

```text
VOX_MENS_FORCE_TRAIN=1 target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu \
  --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal \
  --model Qwen3-4B --output-dir mens/runs/qwen3-4b --max-runtime-secs 86400
# resume (launch c): the same plus --resume mens/runs/qwen3-4b, no --force-restart
```

Preset `qwen3_12g` (batch 1, accum 8). Checkpoints every 20 min (`CHECKPOINT_WALL_INTERVAL`), resume state kept. Runner and watchdog: `evidence/s4-4B-run.sh`. The watchdog sent SIGINT straight to the `vox-ml-cli` PID (then SIGKILL 120 s later) on a peak above free-at-launch minus 4,096 MiB, a row over 5× the previous one (and over 60 s), a stall over 5× the last row interval (and over 60 s), or a 90,000 s deadline.

| launch | free at launch (Metal working set) | watchdog limit | outcome |
|---|---|---|---|
| a, 03:06Z | 54,812 MiB (`evidence/s4-plan-4B.json`); the trainer read 57,695 | — | **Refused** before training: `estimated peak 61637 MB … exceeds 57695 MB free … set VOX_MENS_FORCE_TRAIN=1 to try anyway`. Exit 1 (`evidence/s4-4B-a-*`). |
| b, 03:06:48Z | 57,500 MiB (`evidence/s4-plan-4B-b.json`) | 53,404 MiB | Rows 1-1108 trained, then a **false watchdog stop** during validation (below). Exit 137 (`evidence/s4-4B-b-*`). |
| c, 14:01:22Z | 60,139 MiB (`evidence/s4-plan-4B-c.json`) | 56,043 MiB | Resumed from `checkpoint_step_1080` (`global_step 1080`, `pair_offset 1080`), trained rows 1081-1108, validated, wrote the adapter. Exit 0, watchdog never fired (`evidence/s4-4B-c-*`). |

- **Estimate override.** `VOX_MENS_FORCE_TRAIN=1` was set for launches b and c, so this run is `forced: true`. The trainer's Metal estimate (61,637 MB) was above live free memory, but that estimate over-predicts the measured 4B peak by about 1.9× (re-probe: 33,037 measured, ratio 0.54). The watchdog's measured-peak limit was the guard. The measured peak never came within 18 GB of the limit.
- **Why a resume was needed.** After row 1108 the trainer runs validation on 58 pairs, which emits no step rows. The watchdog's stall rule only excluded `run cap reached` and `Training complete`, so it read validation as a stall: `SIGINT … stall 165993ms since last row (> 5x 32731ms)` at 13:58:50Z, then SIGKILL at 14:00:50Z. The launcher printed `killed by signal 9 (SIGKILL: usually the OS memory killer …)`, but the signal came from the watchdog, not the OS. Memory was not involved. The rows after the last checkpoint (1081-1108) were lost and redone by launch c. The stall rule now also skips `Running validation`.
- **Wall time.** Rows 1-1108 in launch b took 38,900 s, **35.11 s/row** (re-probe projected 38.29). The adapter's lineage is launch b rows 1-1080 (38,060 s) plus launch c (`train_wall_secs` 1,378 s), so **10.96 h** of training. From launch b start to launch c end was **11.31 h**, under the 24 h cap.
- **Peak.** **34,460 MiB** (`run_summary` of launch c, `peak_metal_allocated_bytes` 36,133,076,992). Launch b peaked at 33,071 MiB (telemetry max 34,677,030,912 B) over all 1108 rows. The resumed segment peaks about 1.4 GB higher; the trainer's resume path is the likely cause, but that is not measured here. The re-probe had 33,037.
- **Loss.** `final_loss_ema` **0.543**, but the EMA restarted at the resume and covers only rows 1081-1108. Launch b's EMA at micro-step 1107 was 0.600. `avg_val_loss` 0.697. Loss quarters over the adapter's lineage (`evidence/s4-loss-4B.json`): 1.091 → 0.603.
- **Truncation.** The record's 0 truncated / 0 prompt-trimmed covers only the 28 resumed rows. Launch b was killed before it wrote its counts. 0.6B and 1.7B had 34 / 34 on the same corpus.
- **Floor.** `floor_mb = peak_mb` = 34,460 in `mens/config/gpu-specs.yaml` (was 33,037 from the re-probe), commit `04df7d397`.

Evidence: `evidence/s4-record-4B.json`, `evidence/s4-run-summary-4B.json`, `evidence/s4-loss-4B.json`, `evidence/s4-4B-{a,b,c}-{log.txt,watchdog.txt}`, `.exit`, `.start`, `.end`, `evidence/s4-4B-a-refusal-record.json`.

Stage S4: PASS
