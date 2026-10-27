---
title: "How To: Train MENS on macOS Metal"
description: "Download, QLoRA-train, collateral-check, and serve a Mac Metal pack for Axis Drive."
category: "How-To Guides"
status: "current"
training_eligible: true
schema_type: "HowTo"
---

# Train MENS on macOS with Metal

This procedure creates a local QLoRA run that can be served by the native
VoxLocal path and pinned from Axis Drive. It requires Apple Silicon (`arm64`)
and a GPU-enabled Vox CLI build.

## Spike model

`SPIKE_MODEL_ID=Qwen/Qwen2.5-Coder-0.5B-Instruct`

The larger `Qwen/Qwen3.5-0.8B` checkpoint was rejected during config parsing
because it is a vision-language model. The text-only fallback above completed
the Metal micro-run and is the supported Mac-tier spike model.

## Install and pre-download

Build the GPU-enabled ML CLI and install the Metal runtime plugin:

```bash
cargo build -p vox-ml-cli --release --features gpu,execution-api
export PATH="$(pwd)/target/release:$PATH"   # or your CARGO_TARGET_DIR/release
vox plugin install mens-candle-metal --yes
vox plugin doctor
```

`execution-api` is required for in-process `vox mens serve` (there is no
`vox-schola` binary in this workspace). `vox mens train --model …` downloads
the hub checkpoint via hf-hub before training (there is no separate
`vox mens download` subcommand).

## Train the Metal pack

The checked-in corpus has 100 ChatML `prompt`/`response` pairs and is isolated
under `examples/mens/metal-e2e`. Do not rename it to `train.jsonl`: the
workspace contract and stale-corpus fallback can otherwise select a different
file. Pass `--fast-corpus` so a stale workspace fingerprint does not run the
full corpus pipeline over the e2e data-dir.

```bash
vox mens train --backend qlora --tokenizer hf --device metal \
  --model "$SPIKE_MODEL_ID" \
  --data-dir examples/mens/metal-e2e \
  --output-dir mens/runs/qwen35-08b-metal-e2e \
  --epochs 1 --max-runtime-secs 300 \
  --fast-corpus
```

The run directory must contain `tokenizer.json` and the trainer manifest,
adapter, and configuration artifacts. Before serving an adapter, it must also
contain `collateral_damage_report.json` with `"status": "pass"`. Generate it
after capturing a baseline:

```bash
vox mens eval-collateral-damage \
  --pre-score path/to/baseline.json \
  --post-adapter mens/runs/qwen35-08b-metal-e2e
```

The baseline file is a JSON object whose `benchmarks` object must hold all six
reasoning keys: `gsm8k_think`, `gsm8k_think_init`, `math500_think`,
`math500_think_init`, `vox_think`, and `vox_think_init`. Each slice has an
accuracy key and a think-initiation key (the `_init` partner). A baseline
missing any of the six is a hard error and no report is written. Accuracy and
think-initiation are compared separately, and a drop past 5% on either one
fails the check.

Copy `tokenizer.json` and `config.json` from the Hugging Face snapshot into the
run directory. Update `adapter_manifest.json` `base_model` to the **local
snapshot directory** (the Metal serve plugin does not download hub ids).

The serve command refuses a missing or non-passing collateral report. It also
re-checks the 32-item slice block in the report (`scope_note`, `slice_size`,
and each slice's `n`) and each `collateral_eval/<slice>/reasoning_report.json`
(status `scored`, `n` of 32), and it recomputes the per-slice drops from the
stored pre and post numbers. It does not trust status alone: a flipped
status, a missing slice block, an old-format report, or numbers that recompute
past 5% are refused. A consistent rewrite of pre and post is a new baseline and
is not detected as an edit. A quick first look at the stored status:

```bash
jq '.status' mens/runs/qwen35-08b-metal-e2e/collateral_damage_report.json
```

To compare a base with its adapter, `vox mens eval-reasoning-compare --adapter
<run dir>` scores the pinned base (default `Qwen/Qwen3-8B`, or `--base <size or
hf_id@sha>`) and the adapter on the three 32-item slices into
`mens/runs/reasoning_compare/<size>-<first 12 hex of revision>.json`, one file
per base. When the pinned snapshot or a matching adapter is not on disk, it
writes a `not_compared` record with no score keys and prints `NOT COMPARED`. An
unknown `--base` is an error. The command needs a `vox-ml-cli` built with
`--features gpu`; without it, it exits with an error. The file is not a
serve-gate input.

## Serve and pin from Drive

Ollama commonly occupies port 11434. Use a separate port for the native server:

```bash
vox mens serve --model mens/runs/qwen35-08b-metal-e2e \
  --host 127.0.0.1 --port 11435
curl -s http://127.0.0.1:11435/health
curl -s http://127.0.0.1:11435/v1/models
```

When **`VOX_LOCAL_ENDPOINT`** is unset, Axis and the GUI probe `:11434` then
`:11435` and pick the first server whose `/health` reports
`service == "vox-ml-cli"` (Ollama on `:11434` is ignored). Set
**`VOX_LOCAL_ENDPOINT=http://127.0.0.1:11435`** to pin a non-default port.

Pin the Drive stem `mens/qwen35-08b-metal-e2e`. After a picker open or catalog
refresh, QLoRA packs with `tokenizer.json` plus adapter artifacts
(`candle_qlora_adapter.safetensors`, `adapter_manifest.json`, or
`merged.safetensors`) are listed in `MensCatalog`. Legacy runs with a `final` or
`checkpoint-*` subdirectory are listed too. If a run dir is still empty of those
markers, the Drive path can still be green through the VoxLocal stem match; use
`pin_policy=warn` during catalog refresh when the local catalog is intentionally
incomplete.

## Run the automation

From the repository root:

```bash
vox run scripts/mens-macos-metal-e2e.vox
```

The script exits with status 2 on non-Darwin hosts. On Apple Silicon it runs
the download/train checks, validates the collateral-pass artifact, starts the
server on port 11435, and checks that `/v1/models` lists the
`qwen35-08b-metal-e2e` stem. If Task 6's host-aware training dispatch has not
landed yet, the train step reports that dependency rather than silently
falling back to CUDA.
