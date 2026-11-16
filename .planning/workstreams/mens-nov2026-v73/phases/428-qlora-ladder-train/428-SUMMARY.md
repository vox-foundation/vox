# Phase 428 Summary: Packed QLoRA Ladder Fine-Tuning & Adapter Recombine Verification

**Status:** Complete  
**Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)  
**Phase:** `428-qlora-ladder-train` (`428-01-PLAN.md`)

## Delivered Scope

Sequentially executed 1 full epoch (`1,411` micro-steps, `177` optimizer steps, `1,285,323` packed tokens, `capped: false`, `0` skipped batches) of packed rsLoRA QLoRA fine-tuning (`--backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --fast-corpus --epochs 1 --batch-size 1 --grad-accum 8 --warmup 15 --max-runtime-secs 86400 --sequence-packing --use-rslora --token-normalized-grad-accum --no-auto-heal --no-hf-sync --force-restart`) across all four Nov 2026 dense Qwen3.5/Qwen3.8 ladder rungs on Apple M5 Max (`128 GiB` Metal unified memory).

### Measured Training Metrics Across All 4 Rungs

| Rung | Run Dir | Pinned Base Model | Rank / Alpha (`rsLoRA`) | LR | `lm_head` Policy | Micro / Opt Steps | `final_loss_ema` | `avg_train_loss` | `avg_val_loss_last_epoch` | `wall_secs` | `peak_metal_allocated_bytes` (`peak_mb`) | Adapter Size (`candle_qlora_adapter.safetensors`) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `2B` | `mens/runs/qwen3.5-2b` | `Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` | `r=16, alpha=8` (`2.0`) | `1.0e-4` | `--lora-exclude-lm-head` (`150` targets, `24` layers: `18` linear + `6` full) | `1411 / 177` | `0.4712002098019612` | `0.7343629031606973` | `0.6800880977562219` | `6154.442516459` (`~1.71 h`) | `21,958,277,746` B (`20,942` MiB / `20.45` GiB) | `62,549,256` B (`59.65` MiB) |
| `4B` | `mens/runs/qwen3.5-4b` | `Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` | `r=16, alpha=8` (`2.0`) | `8.0e-5` | `--lora-exclude-lm-head` (`200` targets, `32` layers: `24` linear + `8` full) | `1411 / 177` | `0.4392423145554279` | `0.6652279480900194` | `0.6231685191790814` | `12310.021943375` (`~3.42 h`) | `42,713,716,494` B (`40,735` MiB / `39.78` GiB) | `121,934,432` B (`116.29` MiB) |
| `9B` | `mens/runs/qwen3.5-9b` | `Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a` | `r=16, alpha=8` (`2.0`) | `7.0e-5` | untied `lm_head` trained (`201` targets, `32` layers: `24` linear + `8` full + `lm_head`) | `1411 / 177` | `0.4438375644054037` | `0.6442429815465709` | `0.6141088862045639` | `14485.29433825` (`~4.02 h`) | `52,610,471,694` B (`50,174` MiB / `49.00` GiB) | `176,624,696` B (`168.44` MiB) |
| `27B` | `mens/runs/qwen3.8-27b` | `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` | `r=32, alpha=11` (`~1.94`) | `6.0e-5` | untied `lm_head` trained (`401` targets, `64` layers: `48` linear + `16` full + `lm_head`) | `1411 / 177` | `0.37329544086771926` | `0.538325773267379` | `0.5060296073621626` | `81556.132273292` (`~22.65 h`) | `48,723,963,834` B (`46,467` MiB / `45.38` GiB) | `902,835,176` B (`861.01` MiB) |

### Skip & Integrity Audit (`training_skip_stats.json`)
- Across all 4 rungs (`2B`, `4B`, `9B`, `27B`):
  - `optimizer_steps_executed = 177`, `micro_steps_executed = 1411`
  - `skip_no_supervised_positions = 0`, `skip_short_seq = 0`, `skip_curriculum = 0`, `skip_token_id_oob = 0`, `no_supervised_skip_rate = 0.0`
- Note on `27B` Metal Peak Memory: Activation checkpointing on full-attention layers (Phase 454) reduced the 27B full-epoch training peak from the uncheckpointed `73,503` MiB calibration probe floor to `46,467` MiB (`48,723,963,834` B), well under the `95 GiB` (`102,005,473,280` B) ceiling and below the `9B` uncheckpointed peak (`50,174` MiB).

### Recombine & Merge Status (`merged_model.safetensors` & `recombined_full/`)
- **`mens/runs/qwen3.5-2b`:** Pre-Phase-455 `merge-qlora` verified; `merged_model.safetensors` (`2,743,094,600` B) and `recombined_full/` (`model.safetensors` `7,527,341,880` B + `Modelfile` `202` B + `config.json` + `tokenizer.json` + `tokenizer_config.json`) present on disk.
- **`mens/runs/qwen3.5-4b`:** Pre-Phase-455 `merge-qlora` verified; `merged_model.safetensors` (`7,130,343,312` B) present on disk.
- **`mens/runs/qwen3.5-9b`:** Pre-Phase-455 `merge-qlora` verified; `merged_model.safetensors` (`15,858,690,360` B) present on disk.
- **`mens/runs/qwen3.8-27b` & Native BF16 Multi-Shard Re-Merge (`Phase 458 Wave 2`):** `27B` `merge-qlora` (`merged_model.safetensors` + `recombined_full/`) and native BF16 causal-LM `recombined_full/` + `peft/` re-merge for all rungs (`2B`, `4B`, `9B`, `27B` via Phase 455–457.1 `schola merge-qlora`) are scheduled in Phase 458 Wave 2 immediately after each rung completes paired Phase 429 `eval-gate`, ensuring `eval-local` evaluates `CURRENT_ADAPTER_SERVING` (`nf4_base_plus_lora_arm v2`) without merged checkpoint overlay interference.
