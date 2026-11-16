---
title: "Release Notes"
description: "Release notes for the Vox language, compiler, runtime, and companion tooling."
category: "ci"
status: "current"
training_eligible: true
training_rationale: "Documents verified release capabilities, architecture milestones, and maturity tiers."
---
# Vox `0.6.0` Release Notes (`v7.1` Public Readiness)

Vox `0.6.0` unifies full-stack `.vox` compilation, capability-sandboxed scripting, local-first agent orchestration, hybrid knowledge retrieval, and native QLoRA fine-tuning under a zero-red verification spine.

## Highlights

### Unified Compiler & Full-Stack Code Generation (`vox-compiler`, `vox-codegen`)
- **Single-file full-stack pipeline:** Compiles `.vox` schemas (`table`), typed server endpoints (`@server`, `@query`, `@mutation`), actors, workflows, and UI islands into Rust (`Axum` + `Turso`/`libSQL`) backends and React 19 / TanStack Start TypeScript frontends.
- **Structured LLM outputs:** `@ai(structured_output = T, max_iterations = N)` derives strict JSON-schema response contracts from Vox typedefs with bounded validation retries.
- **Uniform machine-readable build lane:** `vox --json check`, `vox --json build`, `vox --json test`, and `vox --json run` emit single-line `BuildLaneEnvelope` JSON payloads on stdout while routing progress notes to stderr.

### Capability-Sandboxed Scripting & Automation (`vox-eval`)
- **Fast interpreter tier:** `vox run --interp <script.vox>` executes `.vox` automation directly without invoking `cargo`, enforcing explicit capability grants (`--caps fs.read`, `fs.write`, `process`, `net`, `env`).
- **VoxLang-first repository automation:** Project hooks, formatters (`scripts/fmt.vox`), and workflow scripts run cross-platform on Windows, macOS, and Linux via `vox run`.

### Local-First Persistence & Curated Knowledge Base (`vox-db`, `vox-search`, `vox-kb-facets`)
- **Embedded + remote Turso store:** `vox-db` provides unified local/replica SQLite persistence (schema baseline v97+) for sessions, telemetry rollups, skill manifests, and RLHF interaction feedback (`vox-actor-runtime::feedback`).
- **Hybrid Library search (`vox kb`):** Combines Tantivy 0.22 lexical search with semantic vector indexing and Reciprocal Rank Fusion (`vox_kb_search`, `vox_kb_get`, `vox_kb_cite`, `vox_kb_status`, `vox_kb_reindex`).
- **Taxonomy & trust boundaries:** Enforces SKOS-lite documentation facets (`contracts/documentation/kb-taxonomy.v1.yaml`) and strict trust tiers (`SystemVerified`, `Trusted`, `Advisory`, `Untrusted`) before injecting retrieved context into agent prompts.

### Multi-Agent Orchestration & Vox Axis Workbench (`vox-orchestrator`, `vox-orchestrator-mcp`, `vox-gui`)
- **Model-agnostic agent runtime:** All LLM chat, streaming, and embedding traffic routes through `vox-actor-runtime::llm` with Clavis secret resolution (`vox-secrets`) and per-task cost/latency telemetry (`vox-telemetry`).
- **Vox Axis desktop workbench:** Tauri 2 + React 19 operator console spanning `32` registered surfaces across dual themes (`Imperial Carrara White & Gold` and `Minimalist Grey-Blue Slate`), including Multi-Dock Chat, the `VRBS ROMA` multi-agent simulator, and the `AQVAEDUCTUS` cascade inspector.

### MENS Local Fine-Tuning, Serving & Evaluation (`vox-populi`, `vox-ml-cli`)
- **Cross-platform Candle backends:** Native QLoRA fine-tuning (`vox mens train`), adapter merging, local HTTP serving (`vox mens serve`), and benchmark evaluation across `mens-candle-metal` (macOS), `mens-candle-cuda` (NVIDIA), and `mens-candle-cpu` (Linux/Windows CPU).
- **Calibrated VRAM & model-free preflight:** Enforces measured memory-model budgets (`contracts/mens/memory-model.v1.yaml`) and zero-weight-load snapshot/adapter verification (`vox mens eval-reasoning-compare`).

### Standalone `crates.io` Ecosystem & Zero-Red Governance
- **33 standalone Rust crates:** `crates/_public.toml` publishes 33 self-contained packages in 5 topological tiers—including `vox-crypto`, `vox-secrets`, `vox-bounded-fs`, `vox-arch-check`, `vox-plugin-sdk`, `vox-code-audit` (`toestub`), `vox-drift-check`, `vox-ast`, `vox-openai`, and `vox-llm-egress`.
- **Enforced architectural invariants:** `vox ci pre-push`, `vox-arch-check`, `vox-doc-pipeline`, `toestub`, and `vox-drift-check` verify layer boundaries, fan-in budgets, crate-edge allowlists, portable paths (`hardcoded-abs-dir`), and `#[ignore]` test governance with zero warnings or errors.

## Maturity Tiers

| Surface | Status in `0.6.0` |
|---|---|
| `vox-compiler`, `vox-codegen`, `vox-eval` (`vox check/build/test/run`) | **Verified & Shipped** |
| `vox-db`, `vox-search` (`vox kb`), `vox-secrets` (Clavis), `vox-crypto` | **Verified & Shipped** |
| `vox-orchestrator`, `vox-orchestrator-mcp`, **Vox Axis** (`32` GUI surfaces) | **Verified & Shipped** |
| `vox mens` local QLoRA training, serving, and heldout evaluation | **Verified & Shipped** |
| `33` standalone `crates.io` packages (`crates/_public.toml`) | **Verified & Shipped** |
| Multi-node WAN **Populi** mesh scheduling & browser WebContainer sandbox | **Experimental / Active Development** |
