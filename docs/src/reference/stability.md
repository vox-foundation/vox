---
title: "Stability Matrix"
description: "Full per-surface stability grading for Vox — every feature area, its current tier, and the maturity context behind that grade."
category: "Language Reference"
status: "current"
training_eligible: true
schema_type: "TechArticle"
---

# Stability Matrix

See also: [v1.0 release criteria](../architecture/v1-release-criteria.md) for what counts as done — this page grades where each surface stands *today*; that page defines the machine-verifiable gates for calling the whole platform v1.0.

Vox is marching toward a production-hardened v1.0 release. Surfaces are graded by their architectural stability and proximity to the v1 criteria.

| Feature Area | Status | Context & Maturity |
|:---|:---|:---|
| **Core Intelligence** | | |
| Orchestrator Core | 🔵 Stable | Thread-safe dispatch, agent lifecycle, and [Superpowers](../architecture/superpowers-ssot.md) orchestration. |
| Agent Skills (MCP) | 🔵 Stable | Large first-party [MCP](https://modelcontextprotocol.io) registry (300+ tools in `contracts/mcp/tool-registry.canonical.yaml`). Protocol wiring is real; this is not a production SLA for every tool. |
| Socrates Research | 🟡 Preview | [Socrates protocol](./socrates-protocol.md) for automated fact-checking and retrieval. |
| **Language Platform** | | |
| Compiler Core | 🟣 Mature | Wave 2 complete: pure-HIR lowering and stable syntax grammar. |
| LSP & IDE Tools | 🟡 Preview | `vox-lsp` ships completions and hover ([capabilities SSOT](../architecture/vox-lsp-capabilities-ssot-2026.md)); go-to-definition and workspace symbols are partial or unadvertised. Not full cross-reference. |
| Durable Runtime | 🔵 Stable (interpreter) / 🟡 Preview (codegen) | Interpreter path: journal-backed replay for the supported subset ([ADR-019](../adr/019-durable-workflow-journal-contract-v1.md) / [ADR-021](../adr/021-generated-workflow-durability-parity.md)) ships; `@scheduled` runs on a persistent scheduler with crash-safe state. Codegen path: generated binaries link but `current_hir_module()` registration in emitted `main()` is a tracked Phase-5 follow-up — see [ADR-041](../adr/041-durable-functions-completion-2026.md). Unrestricted control-flow replay is explicit non-goal. |
| **Data & Foundation** | | |
| Database Engine | 🔵 Stable | [vox-db](https://github.com/vox-foundation/vox/tree/main/crates/vox-db/) with Turso integration and zero-downtime migrations. |
| Secrets & Safety | 🔵 Stable | [Clavis](https://github.com/vox-foundation/vox/tree/main/crates/vox-secrets/) hardened vault and [Rule Pack](https://github.com/vox-foundation/vox/tree/main/crates/vox-rule-pack/) CI guards. |
| Telemetry Facade | 🟣 Mature | Unified [vox-telemetry](https://github.com/vox-foundation/vox/tree/main/crates/vox-telemetry/) with trace propagation and cost rollups. |
| **AI/ML Engine** | | |
| Inference (Mens) | 🟡 Preview | Native CUDA/Metal/CPU inference with [Candle/Burn](https://github.com/vox-foundation/vox/blob/main/crates/vox-populi/src/inference/mod.rs). |
| Training (Populi) | 🟠 Emergent | QLoRA native pipeline; loss-parity verification in progress. |
| Visus (Vision) | 🟠 Emergent | [Voice of Vision](https://github.com/vox-foundation/vox/tree/main/crates/vox-cli/src/commands/visus/) for automated GUI bug detection. |
| **Platform & UI** | | |
| CLI & DX | 🟣 Mature | Rich diagnostic surface (`vox audit`, `vox ci`, `vox drift-check`). |
| Native GUI (Tauri) | 🟡 Preview | `vox-gui` is the **operator console** (dashboard, agent flow, Superpowers catalog) — not a compiler that turns your `.vox` app into a Tauri binary. |
| Distributed Mesh | 🟠 Emergent | Opt-in node discovery exists. Federation / off-process workload routing is experimental and default-off. |

## Stability Tiers

- 🟢 **Production Candidate**: Hardened for 1.0; feature-complete and regression-free.
- 🔵 **Stable**: API locked; high test coverage; used in core production internal loops.
- 🟣 **Mature**: Core logic stable; focus on ergonomics, documentation, and performance.
- 🟡 **Preview**: Feature-complete; API may shift based on early adopter feedback.
- 🟠 **Emergent**: Core logic functional; major feature parity or scaling remaining.
- 🚧 **Experimental**: Proof of concept; breaking changes are frequent and expected.
- 🔬 **Research**: Internal prototypes; not yet exposed in the standard CLI surface.

v1.0 criteria: [`docs/src/architecture/v1-release-criteria.md`](../architecture/v1-release-criteria.md). Roadmap: [GUI-native phases](../architecture/gui-native-roadmap-status-2026.md). History: [`CHANGELOG.md`](https://github.com/vox-foundation/vox/blob/main/CHANGELOG.md).
