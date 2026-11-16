# Requirements: Vox Unified Workstream (`v3.9`)

**Milestone:** `v3.9` — T3 Chat & T3 Code Ecosystem Deep-Clone, Full-Surface Architecture Graphing, Multi-Doc Vox KB Extraction & Best-of-All-Worlds Harness/Router Upgrade
**Phases:** `203–209` (`7` phases, `28` requirements)
**Status:** Complete (`28/28` verified)

---

## Phase 203: Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph (`T3-CLONE-GRAPH`)

- [x] **T3-CLONE-GRAPH-01**: **Clone All 5 Open-Source T3 Ecosystem Repositories into `~/dev` with Pinned Provenance** — Clone `pingdotgg/t3code` (`~/dev/t3code`), `vibheksoni/t3router` (`~/dev/t3router`), `vibheksoni/pi-t3chat` (`~/dev/pi-t3chat`), `TGlide/thom-chat` (`~/dev/thom-chat`), and `helioschat/app` (`~/dev/helioschat`), recording exact commit SHAs, licenses, and package/crate manifests.
- [x] **T3-CLONE-GRAPH-02**: **Build Token-Bounded `.vox` Surface & Symbol Graph Extractor (`scripts/t3-surface-inventory.vox`)** — Author and type-check `scripts/t3-surface-inventory.vox` (`vox check` + `vox run --interp`) to deterministically scan the cloned `~/dev` repositories by subsystem, emitting a structured JSON/Markdown surface graph (`contracts/reports/t3-ecosystem-surface-graph.v1.json`) without blowing context windows.
- [x] **T3-CLONE-GRAPH-03**: **Author Master T3 Ecosystem Surface Map & Session-Bounded Ingestion Index (`docs/src/architecture/t3-ecosystem-surface-map-research-2026.md`)** — Write the master architecture/research index in `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md` with compliant YAML frontmatter, `<!-- kb:summary -->`, and validated Mermaid topology diagrams covering `t3code`, `t3router`, `pi-t3chat`, `thom-chat`, and `helioschat`.
- [x] **T3-CLONE-GRAPH-04**: **Verify `vox-doc-pipeline --lint-only` & Deterministic Inventory Reproducibility** — Verify that `scripts/t3-surface-inventory.vox` runs cleanly and `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/t3-ecosystem-surface-map-research-2026.md` passes with 0 errors.

---

## Phase 204: Deep-Dive & Document `pingdotgg/t3code` Agent Harness Control Surface (`T3-CODE-HARNESS`)

- [x] **T3-CODE-HARNESS-01**: **Graph & Document `t3code` Monorepo Architecture, Server Event Log & Resumable WebSocket/RPC Protocol** — Analyze `~/dev/t3code` (`apps/server`, `apps/web`, `apps/desktop`, `packages/contracts`, `packages/shared`) and document its Effect-TS / event-sourced orchestration model, state projection, and remote/mobile sync architecture in `docs/src/architecture/t3code-harness-architecture-research-2026.md`.
- [x] **T3-CODE-HARNESS-02**: **Graph & Document Multi-Provider CLI/SDK Adapter Layer (Claude Code, Codex, Cursor, Grok Build, OpenCode, Antigravity)** — Extract every provider adapter contract, auth discovery flow, multi-account profile switcher, and streaming event normalization in `~/dev/t3code`, documenting how external agent harnesses are supervised.
- [x] **T3-CODE-HARNESS-03**: **Graph & Document Permission Modes, Git Worktree Sandboxing & Outside-Agent MCP Bridge** — Document `t3code`'s permission-mode state machine, automated git worktree creation/isolation/PR lifecycle, keybinding system, and MCP server bridge (`docs/user/outside-agents.md` + implementation) in `docs/src/architecture/t3code-worktree-permissions-mcp-research-2026.md`.
- [x] **T3-CODE-HARNESS-04**: **Cross-Reference Every `t3code` Feature Against `vox-orchestrator`, `vox-gui`, and `vox-vcs`** — Produce a file-and-symbol-anchored feature parity ledger mapping `t3code` capabilities to `vox-orchestrator`, `vox-gui`, and `vox-vcs` (`jj-lib`), validated by `vox-doc-pipeline --lint-only`.

---

## Phase 205: Deep-Dive & Document `t3.chat` Router, Model Discovery, Operational Status, Benchmark Feeds & Credit/Usage Accounting (`T3-ROUTER-SURFACE`)

- [x] **T3-ROUTER-SURFACE-01**: **Graph & Document `t3.chat` tRPC Wire Protocol, Request Envelopes & SSE Stream Events (`~/dev/t3router`)** — Inspect every Rust module in `~/dev/t3router` to document `t3.chat`'s production wire format, conversation state representation, reasoning-effort knobs (`low | medium | high`), search toggle options, and image generation endpoints in `docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md`.
- [x] **T3-ROUTER-SURFACE-02**: **Extract & Document `t3.chat`'s 51+ Model Catalog, Dynamic JS Bundle Discovery, Live Operational Statuses & Benchmark Score Feeds** — Document how `t3router` discovers models from `t3.chat`, tracks real-time per-model operational status (`operational | degraded | outage`), and ingests live model benchmark scores and pricing tiers.
- [x] **T3-ROUTER-SURFACE-03**: **Graph & Document Exact Per-Request Credit Deduction Tracking & Coding-Agent Tool/MCP Bridge (`~/dev/pi-t3chat`)** — Analyze `send_with_credits()` / `send_with_credits_stream()` in `~/dev/t3router` and the OpenAI-compatible tool-calling + `mcp__` discovery wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`) + text-protocol fallback in `~/dev/pi-t3chat`.
- [x] **T3-ROUTER-SURFACE-04**: **Compare `t3.chat` Routing & Credit Telemetry Against `vox-orchestrator::models` & `vox-llm-egress`** — Produce a structured router comparison matrix in `docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md` and pass `vox-doc-pipeline --lint-only`.

---

## Phase 206: Deep-Dive & Document `t3.chat` Chat UX, Resumable Server Streams, Prompt Enhancement, Branching & Attachment Pipelines (`T3-CHAT-UX`)

- [x] **T3-CHAT-UX-01**: **Graph & Document Server-Resumable Streaming & Convex/Local-First Synchronization (`~/dev/thom-chat` & `~/dev/helioschat`)** — Analyze how `thom-chat` maintains interruption-free server-side SSE generation across page reloads and how `helioschat` implements 100% client-side storage with passphrase-based E2EE cross-device sync in `docs/src/architecture/t3chat-ux-streaming-and-sync-research-2026.md`.
- [x] **T3-CHAT-UX-02**: **Graph & Document Conversation Branching, Message Editing, Thread Pinning & Temporary/Privacy Modes** — Extract the exact data models and UI interactions for conversation tree branching, message regeneration across alternative models, thread pinning, screen-share privacy blur mode, and disposable temporary chats.
- [x] **T3-CHAT-UX-03**: **Graph & Document Prompt Enhancement, Cursor-Style Rules, Attachment Parsing & OpenRouter 400+ Model / `:free` Tier UX** — Document the one-click "Enhance Prompt" pipeline, user-configurable system rules, multimodal file/PDF/image attachment preprocessing, and BYOK + `:free` model filtering.
- [x] **T3-CHAT-UX-04**: **Map `t3.chat` UX Mechanics to `vox-gui` (`ChatSurface.tsx`, `Loquela.tsx`, `DriveConsole.tsx`)** — Document concrete UI/UX and backend RPC deltas for `vox-gui` and pass `vox-doc-pipeline --lint-only`.

---

## Phase 207: Synthesize the "Best-of-All-Worlds" Vox Gap Matrix & Architecture Blueprint (`T3-VOX-BLUEPRINT`)

- [x] **T3-VOX-BLUEPRINT-01**: **Author Unified T3 Ecosystem vs. Vox Feature Taxonomy & Gap Matrix (`docs/src/architecture/t3-vs-vox-harness-router-blueprint-2026.md`)** — Synthesize all findings from Phases 203–206 into a comprehensive, evidence-anchored architecture SSOT ranking every T3 Chat and T3 Code capability by `Already Superior in Vox | Complementary Upgrade for Vox | Out of Scope`.
- [x] **T3-VOX-BLUEPRINT-02**: **Specify Concrete Rust/TypeScript Type Contracts for Router & Harness Enhancements** — Define exact struct/enum signatures for `ModelOperationalStatus`, external benchmark-prior blending, exact pre/post turn credit/cost delta receipts, resumable turn stream buffers, prompt-enhancement pre-pass, and lazy MCP tool discovery wrappers.
- [x] **T3-VOX-BLUEPRINT-03**: **Emit Machine-Readable Feature Extraction Ledger (`contracts/reports/t3-vox-feature-extraction-ledger.v1.json`)** — Generate a schema-validated JSON ledger mapping every extracted T3 feature to its source repo/file and target Vox crate/module (`vox-orchestrator`, `vox-llm-egress`, `vox-orchestrator-mcp`, `vox-gui`).
- [x] **T3-VOX-BLUEPRINT-04**: **Pass `vox-doc-pipeline` Scoped Lint & Link Verification Across All 5 New T3 Architecture Docs** — Verify zero broken links, valid SKOS taxonomy facets, and valid Mermaid diagrams across all T3 research/blueprint docs.

---

## Phase 208: Implement Extracted Router Upgrades in Vox (`VOX-ROUTER-T3`)

- [x] **VOX-ROUTER-T3-01**: **Live Model Operational Health Status (`Operational | Degraded | Outage`) & Automatic Degraded-Provider Penalization in `ModelSelectionEngine`** — Inspired by `t3router`'s real-time model operational status tracking, add strongly-typed `ModelOperationalStatus` to `ModelRegistryEntry` and `select::decide` so degraded or rate-limited models are automatically down-weighted or bypassed before turn failure.
- [x] **VOX-ROUTER-T3-02**: **Unified Reasoning-Effort (`low | medium | high`) & Web-Search Grounding Knob Normalization Across Providers** — Inspired by `t3.chat` / `t3router`'s unified reasoning-effort and search toggles, normalize `ReasoningEffort` across OpenAI/OpenRouter/Anthropic/DeepSeek/Gemini wire payloads in `vox-llm-egress` and `SelectionIntent`.
- [x] **VOX-ROUTER-T3-03**: **Exact Per-Turn Balance/Credit Delta Accounting & `:free` vs. BYOK Tier Transparency** — Inspired by `t3router::send_with_credits()`, enrich `AgentTurnResult` and `LlmSurfaceTelemetry` with exact pre/post credit/cost delta tracking and zero-cost `:free` vs. BYOK provenance.
- [x] **VOX-ROUTER-T3-04**: **Comprehensive Unit & Integration Test Suite for T3-Enhanced Model Router (`cargo test -p vox-orchestrator -p vox-llm-egress`)** — Verify 100% deterministic routing across operational status transitions, reasoning-effort tiers, and credit-delta accounting with zero regressions on the 64-case 8-spoke benchmark.

---

## Phase 209: Implement Extracted Harness Upgrades in Vox (`VOX-HARNESS-T3`)

- [x] **VOX-HARNESS-T3-01**: **Resumable Server-Buffered Turn Streaming (`ResumableTurnStreamBuffer`) Across Client Reloads/Reconnects** — Inspired by `t3code`'s event-sourced turn log and `thom-chat`'s server-resumable SSE streams, add a ring-buffered active-turn event log in `vox-orchestrator` / `vox-gui` so reloading or reconnecting mid-turn replays buffered chunks and resumes live streaming without losing the turn.
- [x] **VOX-HARNESS-T3-02**: **Lazy `mcp__` Meta-Tool Discovery & Dispatch Bridge (`vox_list_mcp_servers`, `vox_list_mcp_tools`, `vox_call_mcp_tool`)** — Inspired by `pi-t3chat`'s 3-tool MCP discovery wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`), add on-demand MCP server/tool discovery in `vox-orchestrator-mcp` so agents can invoke external MCP ecosystems without bloating the tier-1 prompt schema.
- [x] **VOX-HARNESS-T3-03**: **One-Click Prompt Enhancement Pre-Pass (`enhance_user_prompt_for_spoke`) & Privacy/Temporary Chat Posture** — Inspired by `t3.chat` / `thom-chat`'s prompt enhancer and `helioschat`'s temporary/privacy mode, add spoke-aware prompt enhancement and ephemeral/redacted session posture support across `vox-orchestrator-mcp` and `vox-gui`.
- [x] **VOX-HARNESS-T3-04**: **End-to-End Harness Verification, Scorecard Update & Workspace Hygiene Gate** — Verify all new T3-extracted harness and router capabilities via `cargo test`, Vitest, and `vox-doc-pipeline`, and publish the updated `contracts/reports/t3-vox-best-of-all-worlds-scorecard.v1.json`.

---

## Out of Scope

| Feature | Reason |
|---------|--------|
| Cookie-scraping or TLS-impersonating `t3.chat`'s proprietary web backend in Vox runtime | Vox uses official provider APIs, OpenRouter, Clavis (`vox-secrets`), and local MENS/Ollama — `t3router` is studied strictly as an architectural/protocol reference |
| Replacing Tauri 2 (`vox-gui`) with Electron (`t3code`) or SvelteKit (`thom-chat`) | ADR-045 locks Tauri 2 + React as Vox's desktop surface; we extract architectural patterns, state machines, and UX mechanics into Vox's native stack |
| Replacing Turso/VoxDb (`crates/vox-db`) with Convex | ADR-004 locks Codex/Arca/Turso as Vox's database stack |

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| `T3-CLONE-GRAPH-01..04` | Phase 203 | Complete |
| `T3-CODE-HARNESS-01..04` | Phase 204 | Complete |
| `T3-ROUTER-SURFACE-01..04` | Phase 205 | Complete |
| `T3-CHAT-UX-01..04` | Phase 206 | Complete |
| `T3-VOX-BLUEPRINT-01..04` | Phase 207 | Complete |
| `VOX-ROUTER-T3-01..04` | Phase 208 | Complete |
| `VOX-HARNESS-T3-01..04` | Phase 209 | Complete |

**Coverage:**
- v3.9 requirements: 28 total
- Mapped to phases: 28
- Unmapped: 0 ✓
