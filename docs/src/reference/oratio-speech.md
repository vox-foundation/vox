---
title: "Oratio & speech SSOT (Candle Whisper, no whisper.cpp)"
description: "Official documentation for Oratio & speech SSOT (Candle Whisper, no whisper.cpp) for the Vox language."
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Oratio & speech SSOT (Candle Whisper, no whisper.cpp)

## Why

- **STT without clang/native C++ toolchains**: inference is **Hugging Face Candle** (Rust), not whisper.cpp bindings.
- **One refined transcript path**: consumers use **display/refined** text where Oratio applies `light_trim` after decode.

## What (artifacts)

| Piece | Role |
|--------|------|
| **`vox-oratio`** | Candle Whisper, symphonia decode, `transcribe_path`, `eval` (WER/CER), env `VOX_ORATIO_*`. |
| **`vox-cli`** `vox populi oratio` | CLI transcription + status. |
| **`vox-mcp`** | `vox_oratio_transcribe`, `vox_oratio_status` (+ JSON schemas in tool registry). |
| **`vox-codex-api`** | `GET /api/audio/status`, `POST /api/audio/transcribe`; binary **`vox-codex-dashboard`**. |
| **Typeck / codegen** | Builtin **`Speech`**, **`Speech.transcribe(path) → Result[str]`** → `vox_oratio::transcribe_path` + refined text. |
| **Corpus mix** | `record_format: asr_refine` + schema **`populi/schemas/asr_refine_pairs.schema.json`**. |
| **LSP** | Hover for **`Speech`**; **`transcribe`** only when the line looks like **`Speech.transcribe`** (`builtin_hover_markdown_in_line`). |
| **TS codegen** | **`Speech.transcribe`** → **throw** (points at `examples/oratio/codexAudioTranscribe.ts` + `@server` / HTTP). |
| **TS example** | **`examples/oratio/codexAudioTranscribe.ts`** — `fetch` for `/api/audio/status` and `/api/audio/transcribe`. |

## Who / when

- **Implementers**: compiler (`vox-typeck`, `vox-codegen-rust`, `vox-codegen-ts`, `vox-lsp`), product surfaces (`vox-cli`, `vox-mcp`, `vox-codex-api`), data (`vox-corpus` mix).
- **When to touch**: any change to Oratio env vars, transcript shape, HTTP contract, or builtin `Speech` API.

## Where (files)

- `crates/vox-oratio/` — STT + `eval`, `traits`, `refine`, `backends/*`
- `crates/vox-cli/src/commands/populi/oratio_cmd.rs`
- `crates/vox-mcp/src/tools/oratio_tools.rs`, `mod.rs` (registry + schemas)
- `crates/vox-capability-registry/`, `crates/vox-tools/` (`populi_chat` + `DirectToolExecutor`; Populi chat ∩ executor)
- `crates/vox-codex-api/src/lib.rs`, `src/bin/codex_dashboard.rs`, `Cargo.toml` (`[[bin]]`)
- `crates/vox-typeck/src/builtins.rs` — `Speech` / `SpeechModule` / `transcribe`
- `crates/vox-codegen-rust/src/emit.rs` — `Cargo.toml` template + `MethodCall` for `Speech`
- `crates/vox-codegen-ts/src/jsx.rs`, `component.rs` — `Speech.transcribe` stub
- `crates/vox-lsp/src/lib.rs` — `word_at_position`, `line_has_speech_transcribe`, `builtin_hover_markdown_in_line`; `main.rs` — hover
- `examples/oratio/codexAudioTranscribe.ts`, `examples/oratio/README.md`
- `crates/vox-corpus/src/corpus/mix.rs` — `record_format`, `normalize_training_jsonl_line`
- `populi/schemas/asr_refine_pairs.schema.json`, `populi/config/mix.example.yaml`
- `AGENTS.md`, `docs/src/ref-cli.md`, [`populi-training.md`](populi-training.md), **this file**

## How (contracts)

- **Build check:** `cargo check -p vox-oratio --features stt-candle`; for the **`vox`** CLI Oratio commands, `cargo check -p vox-cli --features populi-oratio` (Oratio is **not** in default **`populi-base`**).
- **Env**: `VOX_ORATIO_MODEL`, `VOX_ORATIO_REVISION`, `VOX_ORATIO_LANGUAGE`, `VOX_ORATIO_CUDA` (feature-gated), `VOX_ORATIO_WORKSPACE` (HTTP path resolution), `VOX_DASH_HOST` / `VOX_DASH_PORT` (dashboard bind). With the **`cuda`** feature, default inference is **CPU** until **`VOX_ORATIO_CUDA=1`**; status JSON includes **`cuda_feature_enabled`**, **`cuda_requested_via_env`**, **`inference_note`**. **`RUST_LOG=vox_oratio_gpu=info`** emits **`oratio_inference_cpu_default`** vs **`oratio_inference_gpu`** on first session load.
- **HTTP transcribe body**: `{"path":"relative-or-absolute"}`.
- **Mix YAML**: optional per-source `record_format: asr_refine`.

## Related

- **Native fine-tuning** (Burn LoRA / `vox populi train`): [`populi-training.md`](populi-training.md).
- **Populi chat tool allowlist**: `vox-tools` module **`populi_chat`** (`chat_tool_definitions` / `execute_tool_calls`), intersecting `vox-capability-registry` with `DirectToolExecutor` — **same MCP names** as `vox-mcp`. Callers (CLI, daemons, tests) import `vox_tools::populi_chat` when they need OpenAI-style tool JSON or in-process execution.

## Out of scope / deprecated

- **whisper.cpp / ggml / clang STT**: not supported in-tree; old plans under `.cursor/plans/` that cite `whispercpp.rs` are **historical** — canonical STT is **Candle** in `vox-oratio`.
