# Decisions

Synthesized from ADR classifications. One entry per ADR. LOCKED status means the decision cannot be auto-overridden by any non-locked source.

## ADR 001 — Burn Backend Selection for vox-tensor
- source: docs/src/adr/001-burn-backend-selection.md
- status: locked
- decision: Use Burn 0.19 with Wgpu and NdArray backends for native Rust ML training without Python dependency.
- scope: Burn, vox-tensor, ML training, Wgpu, NdArray, native Rust, Mens

## ADR 002 — Diátaxis Three-Tier Documentation Architecture
- source: docs/src/adr/002-diataxis-doc-architecture.md
- status: locked
- decision: Ground documentation system in audience and authority boundaries; use Diátaxis as reader-facing organizing principle.
- scope: documentation architecture, Diátaxis, mdBook, audience boundaries, frontmatter, governance

## ADR 003 — Native Rust Training Over Python
- source: docs/src/adr/003-native-training-over-python.md
- status: locked
- decision: Move Mens training from Python/Unsloth to native Rust using Candle and qlora-rs for HF-weight QLoRA.
- scope: Mens training, Rust, Candle, qlora-rs, QLoRA, Python migration, tokenizer

## ADR 004: Codex over Arca over Turso
- source: docs/src/adr/004-codex-arca-turso-ssot.md
- status: locked
- decision: Codex is the public database API, Arca is internal schema, Turso/libSQL is sole relational engine.
- scope: Codex, Arca, Turso, libSQL, database, vox-db, schema, secrets

## ADR 005: Socrates anti-hallucination SSOT
- source: docs/src/adr/005-socrates-anti-hallucination-ssot.md
- status: locked
- decision: Single vox-socrates-policy crate provides unified confidence thresholds and risk classification for LLM surfaces.
- scope: Socrates, anti-hallucination, confidence thresholds, LLM, orchestrator, reliability

## ADR 006: Mens full-graph Candle QLoRA with qlora-rs
- source: docs/src/adr/006-mens-full-graph-qlora-qlora-rs.md
- status: locked
- decision: Mens trains with Candle and qlora-rs supporting sequential multi-layer NF4 quantized modules and optional double quantization.
- scope: Mens, Candle, qlora-rs, QLoRA, NF4, double quantization, QuantizedLinear, training

## ADR 007: qlora-rs multi-layer training API (Phase 2c architecture gate)
- source: docs/src/adr/007-qlora-rs-multi-layer-training-api.md
- status: locked
- decision: Verify qlora-rs supports multi-layer training with shared optimizer without forking the library itself.
- scope: qlora-rs, multi-layer training, QLoRA, shared optimizer, VarMap, QuantizedLinear

## ADR 008: Mens transport
- source: docs/src/adr/008-populi-transport.md
- status: proposed
- decision: In-tree HTTP control plane via Axum with TLS at reverse proxy; evaluate QUIC/gRPC only as future replacement.
- scope: Mens, Populi, HTTP, Axum, TLS, mTLS, control plane, federation
- note: Superseded in practice by ADR-047 (iroh QUIC) for populi mesh transport; this ADR's HTTP-control-plane default no longer reflects the current transport layer.

## ADR 009: Hosted mens / BaaS (future scope)
- source: docs/src/adr/009-populi-hosted-baas.md
- status: proposed
- decision: Scoped future design for managed Mens service; default remains self-hosted; OAuth device flow for hosted auth.
- scope: Mens, BaaS, hosted, OAuth, control plane, multi-tenant, org-bound

## ADR 010 — TanStack as the Vox web spine
- source: docs/src/adr/010-tanstack-web-spine.md
- status: locked
- decision: Adopt TanStack React Router for routing codegen, plan TanStack Start for SSR with Axum reverse-proxy topology.
- scope: TanStack Router, React, Vite, routing, SSR, vox-codegen, Axum, code generation

## ADR 011: Scientia publication manifest SSOT
- source: docs/src/adr/011-scientia-publication-ssot.md
- status: locked
- decision: Unifies Scientia, news, and scholarly submission around one publication manifest and digest-bound approvals.
- scope: Scientia, publication manifests, vox-db, vox-publisher, scholarly submissions, digest-bound approvals

## ADR 012 — Internal Web IR strategy for Vox
- source: docs/src/adr/012-internal-web-ir-strategy.md
- status: proposed
- decision: Adopts WebIR as first-class compiler layer between HIR and frontend emitters with React/TanStack as primary target.
- scope: WebIR, frontend codegen, React, TanStack, islands, TypeScript emission, compiler layers
- note: Builds toward ADR-036 (WebIR/HIR unification, locked); proposal status, not itself contradicted.

## ADR 013 — OpenClaw WS-first native interop
- source: docs/src/adr/013-openclaw-ws-native-strategy.md
- status: locked
- decision: Adopts WS-first integration strategy with stable Rust adapter boundary for OpenClaw Gateway protocol.
- scope: OpenClaw, Gateway protocol, WebSocket, vox-skills, OpenClawRuntimeAdapter, TLS verification, secrets

## ADR 014: async-openai selective adoption (spike outcome)
- source: docs/src/adr/014-async-openai-selective-adoption-spike.md
- status: proposed
- decision: Spike outcome: no-go on async-openai as mandatory core dependency; use bespoke wiring for multi-provider matrix.
- scope: async-openai, OpenAI API, Clavis, vox-openai-wire, vox-openai-sse, vox-reqwest-defaults

## ADR 015: Vox Docker/OCI portability SSOT
- source: docs/src/adr/015-vox-docker-oci-portability-ssot.md
- status: locked
- decision: Docker/OCI-backed portability model as primary deployment boundary for deployed .vox applications.
- scope: Docker, OCI, portability, Vox.toml, vox.lock, vox-package, vox-container, deployment

## ADR 016: Oratio streaming Whisper and constrained decode
- source: docs/src/adr/016-oratio-streaming-whisper-and-constrained-decode.md
- status: locked
- decision: Ships wire-level streaming Whisper with logit-processor constrained decode in Candle backend.
- scope: Oratio, Whisper, Candle, streaming transcription, constrained decode, logit-processor, STT, speech-to-text

## ADR 017: Populi lease-based authoritative remote execution
- source: docs/src/adr/017-populi-lease-remote-execution.md
- status: locked
- decision: Single-owner lease lifecycle for remote execution with A2A transport and local fallback on expiry.
- scope: Populi, lease, remote execution, A2A, orchestrator, mesh, GPU, authoritativ ownership

## ADR 018: Populi GPU truth layering
- source: docs/src/adr/018-populi-gpu-truth-layering.md
- status: locked
- decision: Normative layering between probe-backed GPU facts, allocatable capacity, and operator policy labels.
- scope: Populi, GPU, hardware truth, capacity, scheduling, NVML, probe-backed, node records

## ADR 019: Durable workflow journal contract v1
- source: docs/src/adr/019-durable-workflow-journal-contract-v1.md
- status: locked
- decision: Freezes interpreted workflow durability boundary, replay source of truth, and v1 event contract.
- scope: workflow, durability, journal contract, replay, durable execution, interpreted workflows, event schema

## ADR 020: Populi mesh scaling — default transport posture
- source: docs/src/adr/020-populi-mesh-scaling-transport-default.md
- status: proposed
- decision: Keeps HTTP Populi as the default coordination SSOT, with optional additive layers like gossip and QUIC evaluated only after GPU truth and lease correctness are established.
- scope: Populi, mesh scaling, HTTP control plane, gossip membership, QUIC data planes, NAT traversal, transport posture
- note: SUPERSEDED by ADR-047 (locked): iroh QUIC replaces the bespoke HTTP/JWT populi mesh transport this ADR named as default. Source doc's own Status line now reads 'Superseded (2026-09-04) by ADR-047'. Retained here for historical record only — do not treat as current transport posture.

## ADR 021: Generated workflow durability parity
- source: docs/src/adr/021-generated-workflow-durability-parity.md
- status: locked
- decision: Defines compatibility contract for bringing generated Rust workflows to durable replay parity.
- scope: generated workflows, durability, replay, Rust codegen, durable contracts, activity execution

## ADR 022 — Orchestrator bootstrap factory and daemon boundaries
- source: docs/src/adr/022-orchestrator-bootstrap-and-daemon-boundaries.md
- status: locked
- decision: Single factory for repo-scoped Orchestrator; relationship to MCP and daemon boundaries.
- scope: Orchestrator, bootstrap, daemon, MCP, vox-orchestrator-d, RPC, coordination

## ADR 023: Optional telemetry remote upload
- source: docs/src/adr/023-optional-telemetry-remote-upload.md
- status: locked
- decision: Opt-in local spool and explicit upload; no default transmission; Clavis-backed secrets.
- scope: telemetry, remote upload, secrets, vox telemetry, local spool, diagnostics

## ADR 024 — Dashboard as local Axum-served SPA
- source: docs/src/adr/024-dashboard-axum-spa.md
- status: proposed
- decision: Decision to build dashboard as Axum-served SPA is now fully superseded by Tauri-based GUI application (ADR-045).
- scope: dashboard, Axum, SPA, orchestration UI, GUI application
- note: SUPERSEDED by ADR-045: crates/vox-dashboard was decommissioned (verified absent from the tree); Tauri-based vox-gui is the canonical orchestration surface. Source doc's own Status line now reads 'Superseded (2026-05-11) by ADR-045'. Retained here for historical record only.

## ADR-025: Multi-Agent Lock Coherence and Lease Propagation
- source: docs/src/adr/025-multi-agent-lock-coherence.md
- status: proposed
- decision: Extends locks subsystem with ResourceLockManager for multi-agent resource coordination and contention.
- scope: multi-agent, lock coherence, resource management, bulletin board, lease expiration, agent coordination

## ADR-026: Third-Party Code Provenance Policy
- source: docs/src/adr/026-third-party-code-provenance.md
- status: locked
- decision: Governs ingestion of external open-source code with explicit AGPL guardrails and attribution.
- scope: licensing, AGPL, vendoring, code provenance, Apache-2.0, compliance, cargo-deny

## ADR-027: Dual-Track UI Surfaces (Vox-Native vs React/TanStack Interop)
- source: docs/src/adr/027-dual-track-ui-surfaces.md
- status: proposed
- decision: Splits UI primitives into Vox-native and React/TanStack interop tracks with explicit boundary.
- scope: UI surfaces, reactivity, Vox-native, React interop, training corpus, component model, state machine

## ADR-028: Remove Stub Durability/Scheduling Grammar from Public API
- source: docs/src/adr/028-deprecate-stub-durability-grammar.md
- status: proposed
- decision: Proposes removing @scheduled, @durable, workflow, activity from public grammar after audit found zero runtime implementation.
- scope: durability, scheduling grammar, public API, deprecation, stub detection, actor mailbox
- note: Superseded by ADR-041 (locked): the zero-implementation gap this ADR proposed closing by removal was instead closed by shipping a working durable-functions runtime. ADR-041 explicitly supersedes this ADR's reservation-gate approach per AGENTS.md Grammar Unification section.

## ADR-029: Formal Intent and Tool Receipt Auditing
- source: docs/src/adr/029-formal-intent.md
- status: proposed
- decision: Two-tier verification system for agent tool calls with cryptographic HMAC receipts for auditing.
- scope: formal intent, tool receipts, agent verification, auditing, hallucination defense, Socrates

## ADR 030 — state_machine as Single Source of Truth for reactive UI state
- source: docs/src/adr/030-state-machine-ssot.md
- status: locked
- decision: Formally adopts state_machine compiler primitive as SSoT for all reactive browser UI state.
- scope: state_machine, reactive UI, dashboard, SSoT, vox-dashboard, Vox-native reactivity
- note: NOTE: scope/body mentions 'vox-dashboard' as the reactive-UI host; that surface is now vox-gui (ADR-045). The state_machine-as-SSoT decision itself is unaffected and still Accepted/locked — only the hosting-surface terminology is stale. Not treated as a contradiction requiring correction (per operator instruction); flagged here so downstream synthesis does not assert vox-dashboard is current.

## ADR 031 — Deprecate `vox-vscode`; `vox-dashboard` is the primary user surface
- source: docs/src/adr/031-deprecate-vox-vscode.md
- status: locked
- decision: Deprecates vox-vscode; vox-dashboard is primary surface. VS Code extension retained for LSP only.
- scope: vox-vscode, vox-dashboard, VS Code extension, LSP, orchestration
- note: NOTE: this ADR's own title/decision names 'vox-dashboard' as the primary user surface. vox-dashboard was decommissioned and does not exist in the tree (per ADR-024/045 resolution) — the current primary surface is vox-gui. The vox-vscode deprecation itself is unaffected and still Accepted/locked; only the named successor surface is stale. Not treated as a contradiction requiring correction (per operator instruction); flagged here so downstream synthesis does not assert vox-dashboard is current.

## ADR 032 — `.vox.ui` reactive modules
- source: docs/src/adr/032-vox-ui-reactive-modules.md
- status: locked
- decision: Introduces `.vox.ui` file-suffix for module-scope reactive members lowering to React context and hooks.
- scope: reactive modules, .vox.ui files, React context, state management, file-suffix convention

## ADR 033 — Typed parametric fragment primitive
- source: docs/src/adr/033-typed-fragment-primitive.md
- status: locked
- decision: Introduces `fragment` bare-keyword for typed parametric markup blocks passable as component props.
- scope: fragments, parametric markup, typed props, React components, markup rendering

## ADR 034 — Candle / QLoRA stack upgrades (deferred batch)
- source: docs/src/adr/034-candle-qlora-stack-upgrades.md
- status: proposed
- decision: Defer Candle/peft/qlora version unification to dedicated upgrade train with GPU CI.
- scope: Candle, QLoRA, peft, MENS training, GPU CI, dependency management

## ADR 035 — SWC parser vs alternatives (evaluation only)
- source: docs/src/adr/035-swc-parser-alternatives-eval.md
- status: proposed
- decision: No migration without explicit ADR sign-off; evaluates swc versus oxc, biome, tsgo.
- scope: SWC parser, JS/TS validation, parser evaluation, vox-cli, alternatives

## ADR 036 — WebIR vs HIR unification (compare-both)
- source: docs/src/adr/036-webir-hir-unification-compare-both.md
- status: locked
- decision: Adopt HIR core + WebIR projections; collapse duplicate emit paths for GUI-any-platform goals.
- scope: WebIR, HIR, compiler lowering, IR unification, projections

## ADR 037 — Tauri Convergence
- source: docs/src/adr/037-tauri-convergence.md
- status: proposed
- decision: Desktop-mobile application convergence on Tauri 2; mobile-target clause partially superseded by adr-NNN, desktop convergence remains Accepted.
- scope: Tauri 2, desktop application shell, mobile application packaging, application compilation, GUI code generation
- note: PARTIALLY SUPERSEDED (2026-06-12) by adr-NNN (locked), mobile-target clause only: Tauri no longer covers `--target mobile-android`/`--target mobile-ios`; React Native + Expo + uniffi does (see adr-NNN). The desktop-convergence clause of this ADR is unaffected and remains Accepted/active. Source doc's own Status line now reads 'Partially superseded (2026-06-12) by adr-NNN'.

## ADR 038 — AI fixture `@prompt` decorator
- source: docs/src/adr/038-ai-fixture-prompt-decorator.md
- status: locked
- decision: Introduces `@prompt` decorator for stage-aware prompt fixture lowering onto runtime cascade.
- scope: AI fixtures, @prompt decorator, research stages, runtime cascade, LLM

## ADR 039 — AI fixture `@hole` decorator
- source: docs/src/adr/039-ai-fixture-hole-decorator.md
- status: locked
- decision: Introduces `@hole` decorator for deferred-fill fixtures with compile-time enforcement.
- scope: AI fixtures, @hole decorator, deferred implementation, compile safety, reviewer accountability

## ADR 040 — AI fixture `@search` decorator
- source: docs/src/adr/040-ai-fixture-search-decorator.md
- status: locked
- decision: Introduces `@search` decorator for retrieval fixture composition across multiple corpora.
- scope: AI fixtures, @search decorator, retrieval, docs search, memory search, web search

## ADR 041: Durable functions completion (workflow, activity, actor, @scheduled)
- source: docs/src/adr/041-durable-functions-completion-2026.md
- status: locked
- decision: Records closure of parse-only stub gap; features now backed by working runtime, codegen, journal-backed replay, and scheduler.
- scope: durable functions, workflow, activity, actor, @scheduled, HIR interpreter, codegen

## ADR 044: AI fixture @subagent decorator
- source: docs/src/adr/044-ai-fixture-subagent-decorator.md
- status: locked
- decision: Introduces @subagent decorator for subagent dispatch policy without introducing new bare keyword.
- scope: subagent routing, @subagent decorator, DispatchRouter, orchestrator policy, ai-fixtures

## ADR 045 — Tauri GUI Replaces Axum Dashboard
- source: docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md
- status: locked
- decision: Decommission legacy Axum dashboard and unify into Tauri 2 GUI with CLI as SSOT.
- scope: Tauri 2, vox-gui, vox-cli, CommandCatalog, dashboard, UI
- note: Ratified 2026-09-22 — the ADR body now carries a `**Status**: Accepted (2026-09-22)` line directly under the H1, mirroring ADR-037's convention. Frontmatter `status: "current"` is intentionally unchanged: vox-doc-pipeline's VALID_STATUS enum has no `accepted` value, so the body line is this repo's mechanism for ratification, not the frontmatter key.

## ADR 046 — Pareto-Frontier Reporting for Model Surfaces
- source: docs/src/adr/046-pareto-frontier-reporting.md
- status: proposed
- decision: Present model scoreboards as Pareto frontier over reliability, cost, and latency; reporting-only, not routing change.
- scope: model scoring, Pareto frontier, model selection, reliability, cost, latency

## ADR 047: iroh QUIC replaces the bespoke populi mesh transport
- source: docs/src/adr/047-iroh-transport.md
- status: locked
- decision: Adopts iroh 1.1 for mesh transport, identity, and NAT traversal, retiring hand-rolled HTTP/JWT control plane.
- scope: iroh QUIC, mesh transport, populi, identity, NAT traversal, ed25519

## ADR 048: Interpreter is the execution and sandbox tier
- source: docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md
- status: locked
- decision: Makes HIR interpreter the default isolation tier for VoxScripts locally and over mesh, retiring wasm/container/microvm lanes.
- scope: HIR interpreter, VoxScript execution, isolation, sandbox, capabilities, mesh

## ADR-042: Extract NodeRecord into vox-populi-types (L2)
- source: docs/src/architecture/adr-042-vox-populi-types.md
- status: locked
- decision: NodeRecord moved to new L2 crate vox-populi-types to resolve layering violation (was L0, depends on L2 TaskCapabilityHints).
- scope: NodeRecord, vox-populi-types, vox-mesh-types, vox-repository, layer 2, Rust coherence

## ADR-043: Quantized SafeTensors On-Disk Format
- source: docs/src/architecture/adr-043-quantized-safetensors-ondisk-format.md
- status: locked
- decision: Quantized models stored as u8-block SafeTensors with quant-metadata.json sidecar mapping each tensor to GGML dtype/shape/quantization info.
- scope: SafeTensors, quantization, GGML, model weights, on-disk format, quant-metadata.json

## ADR-NNN: Scope Tauri to desktop only; pick React Native + Expo + uniffi for mobile
- source: docs/src/architecture/adr-NNN-scope-tauri-desktop-only.md
- status: locked
- decision: Tauri scoped to desktop (unchanged); mobile GUI targets React Native + Expo managed workflow via uniffi-bindgen-react-native bridging Vox Rust runtime.
- scope: Tauri, desktop GUI, React Native, Expo, uniffi, mobile target, architecture decision
