# Project Research Summary

**Project:** Vox (`v7.8` — `local-model-lifecycle-v78`)
**Domain:** Lease-Managed Local Model Supervision, Progressive MENS Readiness, Multi-Factor Auto Model Scoring, Populi QUIC Mesh Inference & Progressive-Disclosure Chat Provenance
**Researched:** 2026-11-14
**Confidence:** HIGH

## Executive Summary

Vox already ships a multi-provider model registry (`vox-orchestrator::models`), a Tauri 2 desktop workbench (`vox-gui`), a shared TCP orchestrator daemon (`vox-orchestrator-d`), an `iroh` QUIC peer transport (`vox-mesh-transport`), and native Candle/Metal/CUDA serving (`vox-ml-cli mens serve`). However, a deep four-dimension codebase audit uncovered 37 concrete architectural seams (`G-LIFE-01..10`, `G-ROUTE-01..15`, `G-UX-01..12`) that prevent local lifecycle management, auto model scoring, mesh inference, cloud fallback, and Chat observability from working as a unified production system. Today, `MensServeState` lives exclusively in Tauri GUI memory with manual startup and `SIGKILL` teardown, `vox-orchestrator-d` has zero client lease tracking or Ollama VRAM eviction (`keep_alive: 0`), `MensCatalog` registers ungated `Qwen3.5` runs that `vox mens serve` refuses to boot, six hardcoded pre-scorer shortcuts (`VOX_LOCAL_PREFERRED_TASKS`, `select_local_first`, `premium_alias`) bypass `auto_score_parts`, `PopuliMesh` only executes `VoxScript` jobs while misrouting LLM calls to `127.0.0.1` or OpenRouter, and `persist_assistant_reply` drops `events`, endpoint URLs, and candidate score breakdowns on reload while cluttering every Chat bubble with five permanent action buttons.

The recommended `v7.8` architecture resolves these root causes cleanly within existing `layers.toml` boundaries without new third-party Rust or npm dependencies. First, centralize lease-counted `LocalModelSupervisor` in L3 (`vox-cli-core::daemon_ipc` + `vox-orchestrator-mcp`) hosted by `vox-orchestrator-d`, auto-warming the highest gate-verified fitting rung (`Qwen3-0.6B..32B` in prod today, auto-promoting `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` via watcher as gate receipts land) on lease `0 -> 1` and draining/evicting `vox mens serve` + Ollama VRAM on lease `1 -> 0` unless overridden in `vox-config`. Second, remove all six pre-scorer bypasses so `decide()` + `auto_score_parts` genuinely score Local, Trusted Populi Mesh (`JobRequest::InferChat` over `iroh` QUIC), Free Cloud, and Metered Cloud across `ClutchProfile`, `RiskPosture`, `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), inference VRAM floors, provider credit/rate-limit health, cache-read discounts, and empirical `VoxDb` TTFT/TPOT/quality feedback, with pre-first-token failover governed by `CloudFallbackPolicy`. Third, persist full `ChatTurnProvenanceDto` + top-N `ScoreParts` onto every turn and replace `MessageBubble`'s 5-button clutter and fake `'1.8s'` pill with progressive-disclosure hover actions plus an always-available `<TurnProvenancePill>` + `<ProvenanceHoverPopover>`, converged against dual-theme Concept-Art plates in the 4-theme CV Flywheel.

The primary technical risks are (1) double-VRAM spikes when hot-swapping between Ollama and `vox mens serve` on unified-memory Apple Silicon hosts, mitigated by serialized drain-evict-recheck-load sequencing, (2) split-brain mesh trust between `~/.vox/trusted_nodes.json` and `~/.vox/mesh_trust.json`, mitigated by unifying trust synchronization before enabling `JobRequest::InferChat`, and (3) CSS/portal regressions in light/dark themes, mitigated by open-portal Axe + `@ngi/platform-ssot` CV Flywheel enforcement across all four themes and five resolutions.

## Key Findings

### Recommended Stack

Stay 100% within the pinned Vox Rust + TypeScript workspace stack (`0` new Cargo.lock crates, `0` new npm packages, `0` `layers.toml` inversions).

**Core technologies:**
- `vox-orchestrator-d` + `vox-cli-core::daemon_ipc`: Shared lease-counted `LocalModelSupervisor`, staged `~/.vox/bin/vox-ml-cli` execution, graceful shutdown, and Ollama `/api/ps` + `keep_alive: 0` VRAM lifecycle.
- `vox-orchestrator::models` + `vox-eval`: Gate-verified multi-family MENS discovery (`GateState::Passed`), unified `decide()` + `auto_score_parts` multi-factor scorer, `CommunityIntelligenceBundleV1` + `ExternalBenchmarkPrior` + non-destructive `(model_id, task_category)` `VoxDb` scoreboard feedback, and `ProviderCreditSnapshot` / `Retry-After` budget hints.
- `vox-mesh-transport` (`iroh` QUIC) + `vox-actor-runtime::llm`: Authenticated bi-stream `JobRequest::InferChat` + `MeshInferenceInventory` peer probing and pre-first-token 3-tier failover (`Local -> Trusted Mesh -> Free/Paid Cloud`).
- `vox-gui` (`radix-ui@^1.8.1` `data-slot` primitives + Style Dictionary 4-theme contract + `@ngi/platform-ssot` CV Flywheel): Persisted `ChatTurnProvenanceDto`, `<TurnProvenancePill>`, `<ProvenanceHoverPopover>`, progressive-disclosure turn overflow menu, and dual-theme Concept-Art plates.

### Expected Features

**Must have (table stakes):**
- Auto-load best fitting local model on harness launch (`0 -> 1` lease) and auto-unload `vox mens serve` + Ollama VRAM when harness unloads (`1 -> 0` lease), each configurable in `vox-config` (`VOX_LOCAL_AUTO_LOAD`, `VOX_LOCAL_KEEP_ALIVE`).
- Gate-verified MENS readiness: serve prod-verified rungs (`0.6B..32B`) today without advertising unbootable ungated runs, and automatically promote `Qwen3.5-2B/4B/9B` & `Qwen3.8-27B` when gate receipts appear.
- Genuine multi-factor auto model scoring across Local, Mesh, Free Cloud, and Paid Cloud honoring `Clutch` (`Free | Efficient | Balanced | Genius`), `Risk`, `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), and `CloudFallbackPolicy` (`auto | free_cloud_first | ask_before_cloud | never`).
- Clean progressive-disclosure Chat bubbles where hovering/focusing any response's provenance pill always shows exact endpoint/provider, model ID, score breakdown, cascade hops, and orchestrator rationale (including after session reload).

**Should have (competitive):**
- Authenticated `iroh` QUIC `PopuliMesh` LLM streaming dispatch (`JobRequest::InferChat`) with live peer VRAM/model/queue scoring when local hardware is unavailable or outclassed.
- Live provider credit/rate-limit feedback (`OpenRouter` key remaining balance, native prompt-cache read pricing, expiring `429` cooldowns) dynamically shifting scoring toward Free/Local/Mesh before credits run dry.
- Dual-theme Concept-Art reference plates (`v78-concept-*`) and 4-theme Playwright + VLM CV Flywheel verification with open Radix hover-card Axe/contrast scanning.

**Defer (v2+):**
- Cross-WAN untrusted public marketplace inference settlement (keep `v7.8` strictly scoped to Ed25519/`MeshTrust`-authenticated Populi peers).
- Mid-generation token splicing across heterogeneous model families after tokens have already streamed to the user (keep automatic failover strictly pre-first-token, while keeping post-verification `[HARNESS REPAIR INTERVENTION]` escalation explicit).

### Architecture Approach

Anchor full-stack truth in `docs/src/architecture/local-mesh-cloud-lifecycle-and-routing-research-2026.md`: L0/L2 (`vox-llm-config`, `vox-config`, `vox-eval`) hold typed lifecycle/privacy/fallback policies and gate-state contracts; L3 (`vox-cli-core`, `vox-orchestrator`, `vox-mesh-transport`, `vox-orchestrator-mcp`, `vox-actor-runtime`) owns `LeaseRegistry`, `LocalModelSupervisor`, unified `decide()` auto-scoring, `iroh` `InferChat`, and persisted `ChatTurnProvenanceDto`; and L5 (`vox-orchestrator-d`, `vox-cli`, `vox-gui`) exposes thin CLI, MCP, and Radix `data-slot` UI surfaces.

**Major components:**
1. `LeaseRegistry` & `LocalModelSupervisor` (`vox-orchestrator` / `vox-orchestrator-mcp` / `vox-orchestrator-d`) — client lease refcounting, staged `vox mens serve` lifecycle, Ollama `/api/ps` + `keep_alive: 0` eviction, serialized hot-swap, and `mens/runs/` gate-receipt watcher.
2. Unified Auto Model Scorer & Cascade Engine (`vox-orchestrator::models` + `vox-actor-runtime::llm`) — bypass-free `decide()` + `auto_score_parts`, inference VRAM fit, live catalog/cache/credit/scoreboard signals, and pre-first-token failover across Local, Mesh, Free Cloud, and Paid Cloud.
3. `PopuliMesh` Inference Bridge (`vox-mesh-transport` + `vox-populi`) — unified `MeshTrust`, `MeshInferenceInventory` peer probes, and `JobRequest::InferChat` QUIC streaming.
4. Progressive Chat Provenance & 4-Theme Workbench (`vox-gui`) — persisted `ChatTurnProvenanceDto`, `<TurnProvenancePill>` + `<ProvenanceHoverPopover>`, de-cluttered `MessageBubble` overflow actions, live `ModelsView`/`Loquela`/`BottomStatusBar`/`PopuliMeshBentoView` controls, and CV Flywheel verification.

### Critical Pitfalls

1. **Advertising ungated MENS checkpoints (`MensCatalog` split-brain)** — Gate live `ModelRegistry` insertion on `GateState::Passed` + `collateral_damage == pass` while surfacing `TrainedUngated` / `TrainingInProgress` strictly as status telemetry until receipts land.
2. **Checking QLoRA training floors (`122.8 GB` for 8B) during inference selection** — Introduce dedicated inference `serve_floor_mb` + dtype bytes-per-param (`BF16`/`Q8`/`Q4`) + Metal/CUDA unified free-memory probing.
3. **Double-VRAM OOM during model/backend swap** — Drain in-flight requests, terminate/evict old `vox mens serve` and Ollama VRAM (`keep_alive: 0`), and verify `read_free_memory()` rebound before loading the next model.
4. **Provenance loss on session reload, fallback, or repair escalation** — Finalize `routing_decision` *after* `run_agent_turn` (recording cascade/repair hops and sanitized `endpoint_label`) and persist full `events` + `ChatTurnProvenanceDto` inside `conversation_messages.payload_json`.
5. **Theme/guard violations in new popovers & controls** — Use semantic tokens (`--color-popover`, `--z-popover`), `<VoxSelect>` / `<Switch>` `data-slot` primitives, `lucide-react` / `Icon.*` vectors (`rawEmojiCount === 0`), and `<400`-LoC module decomposition.

## Implications for Roadmap

Based on research, suggested 6-phase structure (`Phases 459–464`, continuing from `v7.7` `Phase 458`):

### Phase 459: Lifecycle/Fallback Config Contract, Daemon Lease Supervisor & Progressive MENS Readiness
**Rationale:** Establish config SSOT, daemon lease refcounting, clean `vox mens serve`/Ollama VRAM supervision, and truthful gate-verified MENS discovery before changing routing or UI.
**Delivers:** `VOX_LOCAL_AUTO_LOAD` / `VOX_LOCAL_KEEP_ALIVE` / `VOX_INFERENCE_PRIVACY` / `VOX_CLOUD_FALLBACK_POLICY`, `LeaseRegistry`, `LocalModelSupervisor` (graceful shutdown, `ai/serve` dir fix, Ollama `/api/ps` + `keep_alive: 0`, zero-OOM hot-swap), and `ServeablePassed` vs `TrainedUngated`/`TrainingInProgress` discovery + live watcher.
**Addresses:** `G-LIFE-01..10`
**Avoids:** Orphaned processes, Windows `target/` binary locks, double-VRAM OOM, and unbootable ungated MENS routing.

### Phase 460: Unified Multi-Factor Auto Model Scorer, Cloud Catalog/Credit Signals & Candidate Score Breakdowns
**Rationale:** Fix all six scoring bypasses and telemetry drops so Local, Mesh, Free Cloud, and Paid Cloud compete fairly under `Clutch`, `Risk`, inference VRAM fit, and live empirical/credit signals.
**Delivers:** Removal of `VOX_LOCAL_PREFERRED_TASKS` / `select_local_first` / static `premium_alias` hijacks, fixed `Balanced`/`Risk::Low` axes, inference `serve_floor_mb` sizing, non-destructive `(model_id, task_category)` scoreboard + `CommunityIntelligenceBundleV1` + `ExternalBenchmarkPrior` hydration, OpenRouter cache pricing + HF Router live sync + `ProviderCreditSnapshot` + expiring `429` cooldowns, and top-N `ScoreParts` + `ExclusionGroupDto` attached to `ModelSelectionDecision`.
**Uses:** `vox-orchestrator::models`, `vox-orchestrator-models`, `vox-eval`
**Implements:** Unified Auto Model Scorer (`G-ROUTE-01..05, 09..12, 15`)

### Phase 461: Authenticated Populi Mesh LLM Dispatch (`iroh` QUIC) & 3-Tier Pre-First-Token Failover Cascade
**Rationale:** With scoring and local supervision solid, wire real remote peer inference over `iroh` QUIC and bounded pre-first-token cascade + `CloudFallbackPolicy` enforcement.
**Delivers:** Unified `MeshTrust` (`trusted_nodes.json` ↔ `mesh_trust.json`), `MeshInferenceInventory` peer probes, streaming `JobRequest::InferChat` wired into `vox_actor_runtime::llm`, 3-level `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), pre-first-token failover across ranked candidates, and `CloudFallbackPolicy` (`auto | free_cloud_first | ask_before_cloud | never`).
**Addresses:** `G-ROUTE-06..08, 13..14`

### Phase 462: End-to-End Chat Provenance Persistence, Progressive-Disclosure Bubble & Hover "Why Chosen" Explainer
**Rationale:** Surface the rich routing/endpoint/score/cascade truth on 100% of Chat turns while de-cluttering `MessageBubble`.
**Delivers:** `ResolvedFrom::Mesh` + extended `routing_decision` contract (`turn-event-kinds.v1.json`), sanitized `endpoint_label` + gateway egress disclosure, post-turn escalation/fallback hop capture across sync/background/plan/regenerate, full `ChatTurnProvenanceDto` + `events` DB persistence/hydration, progressive-disclosure `MessageBubble` action menu (removing fake `'1.8s'` pill & 5-button row), and `<TurnProvenancePill>` + `<ProvenanceHoverPopover>`.
**Addresses:** `G-UX-01..07`

### Phase 463: Cross-Surface Local/Mesh/Cloud Workbench Controls & CLI/MCP Parity
**Rationale:** Connect daemon supervision, live scoring, mesh peer status, and fallback policies across `vox` CLI, MCP tools, and companion Axis GUI surfaces.
**Delivers:** `vox model local {status, up, down, swap, pull}` + `vox_local_model_*` MCP tools (`catalog.v1.yaml` synced), 1-click warm/unload/swap + auto-load/keep-alive controls in `ModelsView` & `Settings`, unified `RoutingExplainer`, live `Loquela` route preview + accurate free/mesh/local cost labels, interactive `BottomStatusBar` routing/lease/credit popover, real spoke telemetry in `ChatExecutionRail`, and live peer test/pin actions in `PopuliMeshBentoView`.
**Addresses:** `G-UX-08..12`

### Phase 464: Live Prod Model Verification, Dual-Theme Concept-Art Plates & 4-Theme CV Flywheel Convergence
**Rationale:** Prove the entire stack against real prod local + cloud models and lock visual/accessibility quality across all themes and viewports.
**Delivers:** End-to-end live integration verification on real prod `Qwen3` MENS checkpoints, Ollama, and cloud endpoints (plus watcher auto-promotion harness for `Qwen3.5`/`Qwen3.8-27B`), `v78-concept-*` Carrara & Slate plates, open-popover Playwright + `@ngi/platform-ssot` + VLM CV Flywheel convergence across `900x600`..`1920x1080` and all 4 themes (`visualAfterScore >= 0.92`, `axeViolations === 0`, `rawEmojiCount === 0`), and research doc promotion to `status: "current"`.

### Phase Ordering Rationale

- Config, daemon leases, and gate-verified local discovery (`Phase 459`) are prerequisites for accurate candidate availability in the scorer (`Phase 460`).
- Unified multi-factor scoring and score-breakdown DTOs (`Phase 460`) must exist before cascading across `PopuliMesh`/Cloud (`Phase 461`) and rendering candidate breakdowns in Chat (`Phase 462`).
- Chat provenance & progressive disclosure (`Phase 462`) and cross-surface controls (`Phase 463`) culminate in live prod E2E + Concept-Art CV Flywheel verification (`Phase 464`).

### Research Flags

Phases likely needing focused protocol verification during `/gsd-plan-phase`:
- **Phase 461:** `iroh` QUIC bi-stream framing backpressure and cancellation propagation when a user aborts a streaming Chat turn mid-flight.

Phases with established repo patterns (skip extra research):
- **Phases 459, 460, 462, 463, 464:** Direct extensions of audited L2/L3/L5 modules with exact file/line targets documented in `docs/src/architecture/local-mesh-cloud-lifecycle-and-routing-research-2026.md`.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | 100% in-repo crates (`layers.toml` verified); zero external deps needed |
| Features | HIGH | Directly traced to user requirements and 37 line-cited codebase gaps |
| Architecture | HIGH | Every RPC, DB column, scorer function, and UI component inspected live |
| Pitfalls | HIGH | Caught live `ai/serve` path bug, `MensCatalog` ungated bug, and training-vs-inference VRAM bug |

**Overall confidence:** HIGH

### Gaps to Address

- **In-flight `Qwen3.8-27B` Metal training (`v7.3` `Phase 428`):** Live prod verification in `Phase 464` tests against the 6 currently gate-passed `Qwen3` rungs + Ollama + live Cloud APIs today, while deterministic fixture/watcher tests verify automatic `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` promotion so they activate seamlessly once `v7.3` Phase 429/430 receipts land.

## Sources

### Primary (HIGH confidence)
- `docs/src/architecture/local-mesh-cloud-lifecycle-and-routing-research-2026.md` — Full 4-dimension codebase audit (`G-LIFE-01..10`, `G-ROUTE-01..15`, `G-UX-01..12`)
- `crates/vox-orchestrator/src/models/{select.rs,scoring.rs,ranking.rs,registry.rs}` — Selector & auto-scorer implementation
- `crates/vox-orchestrator-d/`, `crates/vox-gui/src/commands/{daemon.rs,mens_serve.rs,chat.rs}` — Daemon & GUI lifecycle/persistence
- `crates/vox-mesh-transport/src/{protocol.rs,trust.rs,interp_executor.rs}` — `iroh` QUIC peer protocol & trust store
- `crates/vox-gui/ui/e2e/review/{cv-flywheel.spec.ts,vlmVisionFlywheel.ts,audits.ts}` — 4-theme CV Flywheel & Axe contracts

---
*Research completed: 2026-11-14*
*Ready for roadmap: yes*
