# Phase 426 Verification: Parallel HF Text-Tower Prefetch & Multi-Spoke Corpus Regeneration

**Result:** PASS  
**Verified By:** Coordinator (Milestone v7.3 `mens-nov2026-v73`)

## Requirement Coverage

| Requirement | Status | Evidence |
|---|---|---|
| `MENS-NOV26-PREP-01` | Verified | All 5 pinned checkpoints (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) cached under `~/.cache/huggingface/hub/` with `tokenizer.json`, `config.json`, and 0 vision-only shards; upstream `generation_config.json` absence on `Qwen3.5-{2B,4B,9B}` handled cleanly via `<|im_end|>` EOS fallback + `tokenizer_config.json` / `generation_config.json` sidecar copies verified by `qwen35_without_generation_config_includes_im_end_eos` |
| `MENS-NOV26-PREP-02` | Verified | Freshly regenerated `mens/data/synthetic.jsonl` (`6622` lines), `mens/data/validated.clean.jsonl` (`229` clean records; `mens/data/validate_report.json`: `232` input, `1` compiler reject, `0` JSON/reward rejects), `docs.jsonl` (`6969`), `gsd_workflows.jsonl` (`38`), `rust_source.jsonl` (`25440`), `target/dogfood/rust_authoring.validated.jsonl` (`25440`), and `target/dogfood/train.jsonl` (`1485` pairs; `59.1%` parse rate, `34.1%` coverage) |
| `MENS-NOV26-PREP-03` | Verified | `mens corpus mix` succeeded for all 7 configs (`train_mixed.jsonl`: `4125`, `train_mixed_vox_lang.jsonl`: `7089`, `train_mixed_rust.jsonl`: `25440`, `train_mixed_tool_selection.jsonl`: `252`, `train_mixed_argument_generation.jsonl`: `260`, `train_mixed_agents.jsonl`: `164`, `train_mixed_harness.jsonl`: `550`); `wc -l` confirmed `39365` total lines across all 8 JSONL outputs |

## Command & Test Evidence
- `rustfmt --edition 2024` on all 8 Wave 2A `.rs` files: `exit 0`
- `timeout 600s cargo test -p vox-corpus --lib`: `ok. 312 passed; 0 failed; 2 ignored` (`4.79s`)
- `timeout 600s cargo test -p vox-plugin-mens-candle-metal --test qwen38_vlm_e2e_smoke`: `ok. 2 passed; 0 failed; 0 ignored` (`7.79s`)
- `timeout 120s ./target/release/vox run scripts/mens-corpus/build_rust_authoring_corpus.vox`: `exit 0`, `input_lines=25440`, `relabeled_lines=25440`, `output_lines=25440`
- `timeout 900s cargo build --release -p vox-ml-cli --features mens-candle-metal`: `exit 0` (`2m 51s`)
- `timeout 900s cargo build --release -p vox-plugin-mens-candle-metal`: `exit 0` (`40.92s`), installed as `mens-candle-metal` v0.6.0
- `timeout 900s ./target/release/vox-ml-cli mens corpus stats -i target/dogfood/train_mixed.jsonl`: `Total records: 4125`
- `timeout 60s wc -l target/dogfood/train.jsonl target/dogfood/train_mixed.jsonl mens/data/train_mixed_vox_lang.jsonl mens/data/train_mixed_rust.jsonl mens/data/train_mixed_tool_selection.jsonl mens/data/train_mixed_argument_generation.jsonl mens/data/train_mixed_agents.jsonl mens/data/train_mixed_harness.jsonl`: `39365` total, every file non-empty
