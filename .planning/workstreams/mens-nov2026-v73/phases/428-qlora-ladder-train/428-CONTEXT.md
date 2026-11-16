# Phase 428 Context: Packed QLoRA Ladder Fine-Tuning & Adapter Recombine Verification

- **Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)
- **Goal:** Sequentially fine-tune packed QLoRA adapters for `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, and `Qwen3.8-27B` on Metal and verify `merge-qlora` recombine outputs.
- **Execution Mode:** Sequential Metal GPU per rung (`2B` -> `4B` -> `9B` -> `27B`), protecting tied `lm_head` on `2B`/`4B` via `--lora-exclude-lm-head`.
