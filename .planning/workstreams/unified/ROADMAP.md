# Roadmap: Vox Unified Workstream (`v3.9`)

## Overview

Milestone `v3.9` (**T3 Chat & T3 Code Ecosystem Deep-Clone, Full-Surface Architecture Graphing, Multi-Doc Vox KB Extraction & Best-of-All-Worlds Harness/Router Upgrade**, Phases 203–209) clones the five primary open-source repositories spanning the T3 Chat and T3 Code ecosystem (`pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, `helioschat/app`) into `~/dev`, graphs their complete surface area using a token-bounded `.vox` inventory tool, documents every subsystem across 5 modular architecture/research docs under `docs/src/architecture/`, and implements the highest-leverage router and harness upgrades in `vox-orchestrator`, `vox-llm-egress`, `vox-orchestrator-mcp`, and `vox-gui`.

---

## Phase Summary (`v3.9`)

| Phase | Title | Requirements | Status |
|---|---|---|---|
| **203** | Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph | `T3-CLONE-GRAPH-01..04` (4) | Complete (2026-10-25) |
| **204** | Deep-Dive & Document `pingdotgg/t3code` Agent Harness Control Surface | `T3-CODE-HARNESS-01..04` (4) | Complete (2026-10-25) |
| **205** | Deep-Dive & Document `t3.chat` Router, Model Discovery, Operational Status, Benchmark Feeds & Credit/Usage Accounting | `T3-ROUTER-SURFACE-01..04` (4) | Complete (2026-10-25) |
| **206** | Deep-Dive & Document `t3.chat` Chat UX, Resumable Server Streams, Prompt Enhancement, Branching & Attachment Pipelines | `T3-CHAT-UX-01..04` (4) | Complete (2026-10-25) |
| **207** | Synthesize the "Best-of-All-Worlds" Vox Gap Matrix & Architecture Blueprint | `T3-VOX-BLUEPRINT-01..04` (4) | Complete (2026-10-25) |
| **208** | Implement Extracted Router Upgrades in Vox (`ModelOperationalStatus`, `ReasoningEffort` Normalization & Credit/Cost Delta Receipts) | `VOX-ROUTER-T3-01..04` (4) | Complete (2026-10-25) |
| **209** | Implement Extracted Harness Upgrades in Vox (`ResumableTurnStreamBuffer`, `mcp__` Meta-Tool Discovery & Spoke-Aware Prompt Enhancer) | `VOX-HARNESS-T3-01..04` (4) | Complete (2026-10-25) |

---

## Phase Details

### Phase 203: Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph
**Goal:** Clone `pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, and `helioschat/app` into `~/dev`, build `scripts/t3-surface-inventory.vox` to deterministically index their file/module/route/symbol trees within strict token budgets, and publish the master surface map doc.
**Requirements:** `T3-CLONE-GRAPH-01`, `T3-CLONE-GRAPH-02`, `T3-CLONE-GRAPH-03`, `T3-CLONE-GRAPH-04`
**Success Criteria:**
1. All 5 repositories are cloned in `~/dev/{t3code,t3router,pi-t3chat,thom-chat,helioschat}` with pinned commit SHAs recorded in `contracts/reports/t3-ecosystem-surface-graph.v1.json`.
2. `vox check scripts/t3-surface-inventory.vox` and `vox run --interp scripts/t3-surface-inventory.vox` succeed and emit per-repo subsystem inventories.
3. `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md` passes `vox-doc-pipeline --lint-only` with 0 errors.

### Phase 204: Deep-Dive & Document `pingdotgg/t3code` Agent Harness Control Surface
**Goal:** Thoroughly graph and document `pingdotgg/t3code`'s monorepo architecture, Effect-TS / event-sourced server protocol, multi-provider CLI/SDK adapter layer (`Claude Code`, `Codex`, `Cursor`, `Grok Build`, `OpenCode`, `Antigravity`), permission modes, git worktree isolation, keybindings, and outside-agent MCP bridge across modular Vox KB docs.
**Requirements:** `T3-CODE-HARNESS-01`, `T3-CODE-HARNESS-02`, `T3-CODE-HARNESS-03`, `T3-CODE-HARNESS-04`
**Success Criteria:**
1. `docs/src/architecture/t3code-harness-architecture-research-2026.md` and `docs/src/architecture/t3code-worktree-permissions-mcp-research-2026.md` document every package, RPC contract, provider adapter, and permission mode in `~/dev/t3code`.
2. Includes a concrete symbol-level parity table comparing `t3code` against `vox-orchestrator`, `vox-gui`, and `vox-vcs`.
3. Both documents pass `vox-doc-pipeline --lint-only` with 0 warnings/errors.

### Phase 205: Deep-Dive & Document `t3.chat` Router, Model Discovery, Operational Status, Benchmark Feeds & Credit/Usage Accounting
**Goal:** Thoroughly graph and document `t3.chat`'s router and wire protocol via `~/dev/t3router` and `~/dev/pi-t3chat`, covering its 51+ model registry, dynamic JS bundle model discovery, real-time operational status endpoints, benchmark score feeds, reasoning-effort/search knobs, exact per-request credit tracking, and OpenAI/MCP coding-agent wrappers.
**Requirements:** `T3-ROUTER-SURFACE-01`, `T3-ROUTER-SURFACE-02`, `T3-ROUTER-SURFACE-03`, `T3-ROUTER-SURFACE-04`
**Success Criteria:**
1. `docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md` documents every endpoint, model metadata struct, credit-tracking flow, and tool/MCP bridge in `t3router` and `pi-t3chat`.
2. Maps `t3.chat`'s model status, benchmark, and credit mechanisms directly to `vox-orchestrator::models` and `vox-llm-egress`.
3. Passes `vox-doc-pipeline --lint-only` with 0 errors.

### Phase 206: Deep-Dive & Document `t3.chat` Chat UX, Resumable Server Streams, Prompt Enhancement, Branching & Attachment Pipelines
**Goal:** Thoroughly graph and document `t3.chat`'s product UX and backend mechanics via `~/dev/thom-chat` and `~/dev/helioschat`, covering server-resumable SSE streams across page reloads, client-side E2EE sync, conversation branching, prompt enhancement, Cursor-style rules, multimodal file uploads, and `:free` vs. BYOK OpenRouter model management.
**Requirements:** `T3-CHAT-UX-01`, `T3-CHAT-UX-02`, `T3-CHAT-UX-03`, `T3-CHAT-UX-04`
**Success Criteria:**
1. `docs/src/architecture/t3chat-ux-streaming-and-sync-research-2026.md` documents the data models, streaming resumption state machine, branching tree structure, and prompt enhancement pipeline.
2. Maps UX and RPC enhancements to `vox-gui` (`ChatSurface.tsx`, `Loquela.tsx`, `DriveConsole.tsx`).
3. Passes `vox-doc-pipeline --lint-only` with 0 errors.

### Phase 207: Synthesize the "Best-of-All-Worlds" Vox Gap Matrix & Architecture Blueprint
**Goal:** Consolidate all findings from Phases 203–206 into `docs/src/architecture/t3-vs-vox-harness-router-blueprint-2026.md` and `contracts/reports/t3-vox-feature-extraction-ledger.v1.json`, defining the exact Rust/TS interfaces for Phase 208 (Router upgrades) and Phase 209 (Harness upgrades).
**Requirements:** `T3-VOX-BLUEPRINT-01`, `T3-VOX-BLUEPRINT-02`, `T3-VOX-BLUEPRINT-03`, `T3-VOX-BLUEPRINT-04`
**Success Criteria:**
1. `docs/src/architecture/t3-vs-vox-harness-router-blueprint-2026.md` and `contracts/reports/t3-vox-feature-extraction-ledger.v1.json` are committed and verified.
2. All 5 new architecture/research docs pass `vox-doc-pipeline --lint-only` with 0 broken links or facet errors.

### Phase 208: Implement Extracted Router Upgrades in Vox (`ModelOperationalStatus`, `ReasoningEffort` Normalization & Credit/Cost Delta Receipts)
**Goal:** Upgrade `vox-orchestrator::models` and `vox-llm-egress` with live `ModelOperationalStatus` health gating (`Operational | Degraded | Outage`), unified cross-provider `ReasoningEffort` (`Low | Medium | High`) and search-grounding wire normalization, and exact per-turn credit/cost delta & `:free`/BYOK tier accounting.
**Requirements:** `VOX-ROUTER-T3-01`, `VOX-ROUTER-T3-02`, `VOX-ROUTER-T3-03`, `VOX-ROUTER-T3-04`
**Success Criteria:**
1. `ModelSelectionEngine` penalizes or bypasses `Degraded`/`Outage` models deterministically and routes `ReasoningEffort` cleanly across providers.
2. `AgentTurnResult` and `LlmSurfaceTelemetry` record exact turn cost/credit delta and `:free` vs. BYOK tier provenance.
3. All unit/integration tests in `vox-orchestrator` and `vox-llm-egress` pass with 0 regressions.

### Phase 209: Implement Extracted Harness Upgrades in Vox (`ResumableTurnStreamBuffer`, `mcp__` Meta-Tool Discovery & Spoke-Aware Prompt Enhancer)
**Goal:** Upgrade `vox-orchestrator`, `vox-orchestrator-mcp`, and `vox-gui` with server-buffered resumable turn streaming across reloads/reconnects (`ResumableTurnStreamBuffer`), lazy 3-tool external MCP discovery (`vox_list_mcp_servers`, `vox_list_mcp_tools`, `vox_call_mcp_tool`), spoke-aware one-click prompt enhancement (`enhance_user_prompt_for_spoke`), and temporary/privacy session posture.
**Requirements:** `VOX-HARNESS-T3-01`, `VOX-HARNESS-T3-02`, `VOX-HARNESS-T3-03`, `VOX-HARNESS-T3-04`
**Success Criteria:**
1. Reconnecting or reloading mid-turn replays buffered chunks from `ResumableTurnStreamBuffer` without dropping tokens or duplicating tool calls.
2. Agents can discover and call external MCP server tools lazily via the 3 meta-tools without inflating tier-1 prompt schemas.
3. Prompt enhancement and temporary/privacy session posture work end-to-end with `contracts/reports/t3-vox-best-of-all-worlds-scorecard.v1.json` verified.
