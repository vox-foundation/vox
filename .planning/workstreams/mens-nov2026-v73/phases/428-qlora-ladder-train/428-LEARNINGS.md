---
phase: 428
phase_name: "qlora-ladder-train"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 428 Learnings: qlora-ladder-train

## Decisions

### Sequentially executed 1 full epoch (1,411 micro-steps, 177 optimizer ste...
Sequentially executed 1 full epoch (`1,411` micro-steps, `177` optimizer steps, `1,285,323` packed tokens, `capped: false`, `0` skipped batches) of packed rsLoRA QLoRA fine-tuning (`--backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --fast-corpus --epochs 1 --batch-size 1 --grad-accum 8 --warmup 15 --max-runtime-secs 86400 --sequence-packing --use-rslora --token-normalized-grad-accum --no-auto-heal --no-hf-sync --force-restart`) across all four Nov 2026 dense Qwen3.5/Qwen3.8 ladder rungs on Apple M5 Max (`128 GiB` Metal unified memory).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 428-SUMMARY.md

## Lessons

### capped == false
**`capped == false`:** Verified `false` across `2B`, `4B`, `9B`, `27B`.

**Context:** Verified in phase acceptance checks.
**Source:** 428-VERIFICATION.md

---

### global_step > 50
**`global_step > 50`:** Verified `1411` micro-steps (`177` optimizer steps, `1,285,323` total tokens) across all 4 rungs.

**Context:** Verified in phase acceptance checks.
**Source:** 428-VERIFICATION.md

## Patterns

## Surprises

