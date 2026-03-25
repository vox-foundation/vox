---
title: "Mobile and edge AI — SSOT"
description: "Official documentation for Mobile and edge AI — SSOT for the Vox language. Detailed technical reference, architecture guides, and impleme"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Mobile and edge AI — SSOT

This page is the **single place** for how Vox treats **Android / iOS / browser** relative to desktop Populi training, **Ollama**, **mesh** coordination, and **GPU** advertisement. It complements [Populi training SSOT](populi-training.md), [mesh SSOT](mesh.md), and [unified orchestration](orchestration-unified.md).

## Non-goals (near term)

- Running **Ollama** or a full **Ollama-compatible** daemon on stock consumer phones.
- Running **`vox populi train`** with **Candle QLoRA** or **Burn LoRA** *on* the phone (Rust + wgpu/Candle stacks are workstation targets).
- Promising **end-to-end LLM LoRA fine-tuning** on-device with the same maturity as workstation `vox populi train` (industry runtimes still steer operators toward **train off-device, infer on-device** for LLMs).

## Industry context (2025–2026)

- **On-device LLM inference:** Google **LiteRT-LM** is the cross-platform direction for Android, iOS, web, and desktop with hardware acceleration; see [LiteRT-LM](https://ai.google.dev/edge/litert-lm) and [LLM inference (AI Edge)](https://ai.google.dev/edge/mediapipe/solutions/genai/llm_inference). Older MediaPipe-only flows are being superseded; plan migrations against current AI Edge docs.
- **LoRA / adapters:** Practical path is **fine-tune on a workstation or cloud**, then **ship base + adapter** (or converted bundle) to the device. LiteRT LLM LoRA on-device is still **integration-heavy** (see discussion in [LiteRT issue #1420](https://github.com/google-ai-edge/LiteRT/issues/1420)).
- **Web tier:** **WebGPU** helps browser-side compute but is **not universal** (OS version, browser policy, and security modes can disable it). Treat PWA / WebGPU as an **optional** tier, not the only mobile story.

## Vox tiers

| Tier | Train | Infer | Mesh node | Notes |
|------|--------|--------|-----------|--------|
| **Workstation** | `vox populi train` (Burn / Candle) | `vox populi serve`, Ollama, cloud OpenAI-compatible | Yes (`vox-mcp`, `vox run`, `vox mesh`) | Default SSOT paths. |
| **Mobile native** | **Off-device** (`mobile_edge` contract / preset) | LiteRT-LM, Core ML, vendor SDKs | Yes — HTTP control plane + [`NodeRecord`](../../../crates/vox-mesh/src/lib.rs) | Register capabilities from the app; see mesh env vars below. |
| **Browser** | Off-device | WebGPU + WASM (when available) | Optional (HTTP client to mesh) | Not WASI `vox run --isolation wasm` (that is desktop Wasmtime). |

## Training pathway for mobile (`mobile_edge`)

1. On a **GPU or CPU workstation**, run:

   `vox populi train … --deployment-target mobile_edge`

   or `--preset mobile_edge` (implies the same deployment target).

2. The **execution planner** applies **gates**: bounded `seq_len` / `rank` / `batch_size`, no `--qlora-require-full-proxy-stack`, and **`--device cpu`** is **required** so adapters are trained without binding to a desktop-only GPU stack (see planner errors for the exact message).

3. **Artifacts** (`adapter_schema_v3`, `training_manifest.json`) record `training_deployment_target` and an operator **note** pointing here and to [HF finetune capability matrix](../architecture/hf-finetune-capability-matrix.md). **Conversion** to LiteRT / Core ML / TFLite is **out of tree** until a supported exporter exists.

Canonical trainer documentation remains [populi-training.md](populi-training.md).

## Export contract (out of tree)

Training emits artifacts that are consumed by an **exporter outside this repository** until a first supported exporter lands in-tree.

### Inputs (already produced by the Populi pipeline)

- `adapter_schema_v3`
- `training_manifest.json`
- `training_deployment_target` (for example `mobile_edge`)

### Outputs

*TBD by the chosen on-device runtime* (for example LiteRT bundle layout, Core ML, or vendor-specific packages).

### Definition of done (first supported exporter)

- [ ] Documented output format(s) and a version pin for the target runtime.
- [ ] Reproducible build: same inputs and toolchain version produce artifacts described by a checksum or manifest.
- [ ] `training_manifest.json` (or its successor) records exporter version and output checksums (or equivalent integrity fields).
- [ ] Documented validation step (for example a dry-run load in the target runtime, or a future `vox populi` verify subcommand when one exists).

Further context: [HF finetune capability matrix](../architecture/hf-finetune-capability-matrix.md), [Populi training SSOT](populi-training.md).

## Inference profiles (no Ollama on loopback for mobile)

Desktop MCP and CLI default to a **local Ollama** URL for **workstation** use only. Mobile apps should set an explicit profile (environment) so routing does not assume `localhost:11434`.

**`vox-mcp` HTTP inference:** local Ollama calls and cloud→Ollama fallback are enabled only when the profile is **`desktop_ollama`** or **`lan_gateway`**. Other profiles skip Ollama probes and reject `ProviderType::Ollama` with a clear error unless you switch profile or model.

| Profile | Meaning |
|---------|---------|
| `desktop_ollama` | Default when unset: `OLLAMA_HOST` / `POPULI_URL` / `http://localhost:11434` (see [`vox_config::inference`](../../../crates/vox-config/src/inference.rs)). |
| `cloud_openai_compatible` | Use `OPENROUTER_*`, `HF_*`, or dedicated OpenAI-compatible URLs from config. |
| `mobile_litert` | On-device LiteRT-LM (app-owned); Vox tooling does not spawn the runtime. |
| `mobile_coreml` | Apple Core ML (app-owned). |
| `lan_gateway` | Ollama or Populi HTTP on **LAN** (explicit base URL). |

Registry: [Environment variables (SSOT)](env-vars.md) (`VOX_INFERENCE_PROFILE`).

## Mesh and GPU / NPU advertisement

Mesh nodes embed [`TaskCapabilityHints`](../../../crates/vox-orchestrator/src/contract.rs). **CUDA** and **Metal** are not sufficient for Android **Vulkan** phones or **NPU** classes.

- **Legacy:** `VOX_MESH_ADVERTISE_GPU=1` still sets **`gpu_cuda`** (workstation-oriented; unchanged for backward compatibility).
- **Additive:** `VOX_MESH_ADVERTISE_VULKAN`, `VOX_MESH_ADVERTISE_WEBGPU`, `VOX_MESH_ADVERTISE_NPU` (each `1` / `true`) set the matching capability flags.
- **Class label:** `VOX_MESH_DEVICE_CLASS` — optional free-form hint (`server`, `desktop`, `mobile`, `browser`, …) stored in `TaskCapabilityHints.device_class`.

See [mesh SSOT](mesh.md) for the full `VOX_MESH_*` table.

## GPU probing (Populi vs mesh)

- **Populi training** uses [`probe_gpu`](../../../crates/vox-populi/src/tensor/device.rs) for VRAM heuristics. Overrides: **`VOX_GPU_MODEL`**, **`VOX_GPU_VRAM_MB`**. **Windows:** `wmic`; **Linux:** best-effort `nvidia-smi` / `lspci`. **Android / iOS:** no in-crate probe — the **host app** should set env overrides or pass capabilities into mesh JSON.
- **Mesh** does not require Populi; capability flags come from **env + host** as above.

## Related

- [Cross-platform Vox — lanes & Docker matrix (SSOT)](../architecture/vox-cross-platform-runbook.md) — script worker vs app vs mobile; Docker feature matrix.
- [Deployment compose SSOT](deployment-compose.md) — server/container Compose vs mobile (inference profiles, no phone OCI).
- [Orchestration unified SSOT](orchestration-unified.md) — capability merge rules.
- [Environment variables (SSOT)](env-vars.md).
- [vox-mcp API](../api/vox-mcp.md) — Ollama fallback is **desktop-oriented**.
