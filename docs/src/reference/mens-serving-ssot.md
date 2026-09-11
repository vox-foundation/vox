---
title: "Mens local serving SSOT (in-process server + orchestrator)"
description: "Single operator story for serving Candle QLoRA training outputs: vox mens serve HTTP, POPULI_URL, orchestrator mesh config, and external handoff."
category: "Language Reference"
training_eligible: true

schema_type: "TechArticle"
---

# Mens local serving SSOT (in-process server + orchestrator)

## What this page is for

After **`vox mens train`** (Candle QLoRA, default), the **supported local inference server** is **`vox mens serve --model <run_dir>`**. This runs an in-process Axum server built into `vox-ml-cli`, gated behind the **`execution-api`** cargo feature — there is no standalone `vox-schola` binary in this workspace, so a build without `execution-api` cannot serve locally at all (rebuild with `cargo build -p vox-ml-cli --release --features gpu,execution-api,mens-candle-cuda`, swapping the backend plugin feature for the one matching your host). It loads the run directory (`candle_qlora_adapter.safetensors`, `tokenizer.json`, shards) and exposes:

- **`POST /v1/chat/completions`** — OpenAI Chat Completions
- **`POST /api/chat`** — Ollama-shaped chat (used by MCP `vox-mcp` when the provider is Ollama)
- **`POST /api/generate`** — Ollama-shaped generate (**required** for **`vox-gamify`** streaming and **`vox-actor-runtime` `PopuliClient::generate`**)
- **`GET /api/tags`** — model list for probes
- **`GET /api/version`** — JSON including a **`cuda`** hint when `--device` is CUDA (for capability probes)
- **`POST /api/embeddings`** — **501** (not implemented; use Ollama.app or another stack for embeddings)

This is **not** the same process as **Ollama.app** on `http://localhost:11434`, but it speaks a **compatible subset** of Ollama HTTP so you can point **`POPULI_URL`** (or **`OLLAMA_URL`**) at this server's listen address. Note that the CLI's own `--port` default is also `11434` (`DEFAULT_INFERENCE_PORT`), so pass `--port` explicitly (as in the example below) when Ollama.app might already be running on the same host.

## Quick start

1. Train (example): `vox mens train --device cuda --output-dir mens/runs/latest`
2. Serve: `vox mens serve --model mens/runs/latest --port 11435`  
   (requires a `vox-ml-cli` build with `--features execution-api`; see above)
3. Point clients at the server:
   - **`POPULI_URL=http://127.0.0.1:11435`** (precedence over **`OLLAMA_URL`**; see [`vox_config::inference::local_ollama_populi_base_url`](../../../crates/vox-config/src/inference.rs))
   - **`POPULI_MODEL=my-mens`** must match the name returned by **`GET /api/tags`** (the run directory's final path component)

## Orchestrator and agent-to-agent

The in-tree orchestrator’s **`AiTaskProcessor`** uses **`vox_gamify::FreeAiClient`**, which calls **`POST …/api/generate`** for the local Ollama lane. This server implements `/api/generate`, so orchestrator streaming works when **`POPULI_URL`** targets it.

**`Vox.toml` `[mesh]`** (or legacy **`[mens]`**) can record a stable inference base for operators and tooling:

```toml
[mesh]
control_url = "http://127.0.0.1:9847"   # Populi mesh control plane (optional)
inference_base_url = "http://127.0.0.1:11435"  # vox mens serve or Ollama-shaped server
```

This maps to **`OrchestratorConfig::populi_inference_base_url`**. **Processes still read `POPULI_URL` from the environment** today: when starting workers or daemons, set **`POPULI_URL`** to that value (or export **`VOX_ORCHESTRATOR_POPULI_INFERENCE_BASE_URL`** and copy into **`POPULI_URL`** in your launcher). The config field is the **SSOT for the intended URL** in workspace TOML.

The default model registry uses **`POPULI_MODEL`** for the local Ollama provider entry ([`ModelConfig::default`](../../../crates/vox-orchestrator/src/models/spec.rs)); keep it aligned with the served model id.

## MCP

MCP’s Ollama bridge uses **`POST /api/chat`**, which this server already supports. With **`OLLAMA_HOST`** or equivalent base URL pointing at it, MCP interoperates without code changes.

## Machine-readable handoff

Training completion writes **`external_serving_handoff_v1.json`** in the run directory (schema: [`contracts/eval/external-serving-handoff.schema.json`](../../../contracts/eval/external-serving-handoff.schema.json)). **`vox mens merge-qlora`** writes the same filename next to the merged shard’s parent directory for **external** (vLLM / HF / Ollama import) workflows.

## Burn `vox mens serve` (`execution-api`)

The same in-process **`execution-api`**-gated server also serves **Burn checkpoint** (`*.bin` / `merge-weights`) artifacts, not just Candle QLoRA run directories. See [Mens native training SSOT](mens-training.md) for the train → merge → serve matrix.

## Related

- [Mens native training SSOT (Candle QLoRA–first)](mens-training.md)
- [Model routing and provider cascade](../how-to/how-to-model-routing.md)

