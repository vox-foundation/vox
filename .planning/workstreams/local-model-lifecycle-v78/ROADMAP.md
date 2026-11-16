# Roadmap: Milestone v7.8 (`local-model-lifecycle-v78`) — Lease-Managed Local Model Supervision, Progressive MENS Readiness, Multi-Factor Auto Model Scoring, Populi QUIC Mesh Inference & Progressive-Disclosure Chat Provenance

**Workstream:** `.planning/workstreams/local-model-lifecycle-v78/`
**Research SSOT:** [`docs/src/architecture/local-mesh-cloud-lifecycle-and-routing-research-2026.md`](../../../docs/src/architecture/local-mesh-cloud-lifecycle-and-routing-research-2026.md)
**Scope:** Unify local model supervision (`vox mens serve` + Ollama VRAM eviction), progressive multi-family MENS ladder discovery (`Qwen3` prod rungs today + automatic `Qwen3.5`/`Qwen3.8-27B` gate-based promotion), bypass-free multi-factor auto model scoring (`Clutch`, `Risk`, inference VRAM fit, `VoxDb` per-task scoreboard, cloud cache/credit/rate-limit signals), authenticated `iroh` QUIC Populi Mesh LLM streaming (`JobRequest::InferChat`) + pre-first-token cascade (`VOX_INFERENCE_PRIVACY` & `VOX_CLOUD_FALLBACK_POLICY`), progressive-disclosure Chat hover provenance (`<TurnProvenancePill>` + `<ProvenanceHoverPopover>`), cross-surface CLI/MCP/GUI controls, live prod model E2E verification, and 4-theme Concept-Art CV Flywheel convergence across `Phases 459–464` (`24/24` requirements). Zero new third-party crates in `Cargo.lock`, zero new npm packages in `package.json`, zero `layers.toml` / `crate-edges` violations, zero `VoxDb` handles in `vox-gui`, `<400`-LoC child modules, and 100% model-agnostic LLM routing via `vox_actor_runtime::llm`.

## Phase Checklist

- [ ] **Phase 459: Lifecycle/Fallback Config Contract, Daemon Lease Supervisor & Progressive MENS Readiness** (`LIFE-01..04`, `LADDER-01..03`)
- [ ] **Phase 460: Unified Multi-Factor Auto Model Scorer, Live Cloud Catalog/Credit Signals & Candidate Score Breakdowns** (`SCORE-01..04`)
- [ ] **Phase 461: Authenticated Populi Mesh LLM Dispatch (`iroh` QUIC) & 3-Tier Pre-First-Token Failover Cascade** (`MESH-01..04`)
- [ ] **Phase 462: End-to-End Chat Provenance Persistence, Progressive-Disclosure Bubble & Hover "Why Chosen" Explainer** (`PROV-01..03`)
- [ ] **Phase 463: Cross-Surface Local/Mesh/Cloud Workbench Controls & CLI/MCP Parity** (`LADDER-04`, `PROV-04`)
- [ ] **Phase 464: Live Production Model Verification, Dual-Theme Concept-Art Plates & 4-Theme CV Flywheel Convergence** (`CV-01..04`)

---

### Phase 459: Lifecycle/Fallback Config Contract, Daemon Lease Supervisor & Progressive MENS Readiness

**Directory**: `.planning/workstreams/local-model-lifecycle-v78/phases/459-daemon-lease-supervisor-and-mens-readiness/`
**Goal**: Establish strongly-typed lifecycle/privacy/fallback user configuration, centralize lease-counted `vox mens serve` + Ollama VRAM supervision inside `vox-orchestrator-d` (auto-warming on launch and cleanly unloading on harness exit unless overridden), fix `ai/serve` directory resolution, and wire gate-verified multi-family MENS discovery that serves prod `Qwen3-0.6B..32B` today while auto-promoting `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` the moment gate receipts land.
**Depends on**: None (continues sequentially after Milestone `v7.7` `Phase 458`)
**Requirements**: LIFE-01, LIFE-02, LIFE-03, LIFE-04, LADDER-01, LADDER-02, LADDER-03
**Success Criteria** (what must be TRUE):

  1. Operators can set `VOX_LOCAL_AUTO_LOAD` (`true` default), `VOX_LOCAL_KEEP_ALIVE` (`"0"` default, `"5m"`, `"forever"`), `VOX_LOCAL_PREFERRED_BACKEND` (`auto | mens_serve | ollama`), `VOX_LOCAL_ENDPOINT`, `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), and `VOX_CLOUD_FALLBACK_POLICY` (`auto | free_cloud_first | ask_before_cloud | never`) in `~/.vox/config.toml` or `Vox.toml`, with `orch.reload_config` propagating updates live into `vox-orchestrator-d` (`0` raw `std::env::var` bypasses).
  2. Launching Axis GUI, `vox` CLI chat/agent sessions, or MCP clients acquires an `orch.lease.*` / TCP-bound lease in `vox-orchestrator-d`: `0 -> 1` active leases automatically warms the best fitting gate-passed local model (when `VOX_LOCAL_AUTO_LOAD=true`), and `1 -> 0` leases (or `SIGINT`/`SIGTERM`) gracefully stops staged `~/.vox/bin/vox-ml-cli mens serve` (`/v1/admin/shutdown` + `with_graceful_shutdown`, zero `SIGKILL`) and evicts Vox-loaded Ollama VRAM models (`GET /api/ps` + `POST /api/generate` `keep_alive: 0`) unless `VOX_LOCAL_KEEP_ALIVE` retains residency — including when `vox-gui` exits an adopted daemon.
  3. Calling `LocalModelSupervisor::hot_swap` drains in-flight requests, unloads/evicts active `vox mens serve` and Ollama VRAM, verifies `read_free_memory()` rebound before loading the target model, and resolves `:11434`/`:11435` port conflicts with zero double-VRAM OOM spikes; `ai/serve/mod.rs` handles directory paths cleanly so `serve_fit_note` and full slugs (`qwen3-0.6b`, `qwen3.5-2b`, `qwen3.8-27b`) are preserved.
  4. `MensCatalog` and `size_select` (`qwen3_5_code` + `qwen3_code` fallback) classify checkpoints into `ServeablePassed`, `TrainedUngated`, `TrainingInProgress`, and `HubPullablePassed` — routing only `ServeablePassed` rungs today (`Qwen3-0.6B/1.7B/4B/8B/14B/32B`) while a live `mens/runs/` watcher promotes `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` automatically into `ModelRegistry` upon gate + collateral pass.

**Plans**: TBD

### Phase 460: Unified Multi-Factor Auto Model Scorer, Live Cloud Catalog/Credit Signals & Candidate Score Breakdowns

**Directory**: `.planning/workstreams/local-model-lifecycle-v78/phases/460-unified-auto-model-scorer-and-cloud-credits/`
**Goal**: Eliminate all six pre-scorer hijacks and telemetry blind spots so `decide()` + `auto_score_parts` dynamically score Local, Mesh, Free Cloud, and Metered Cloud candidates across `ClutchProfile`, `RiskPosture`, inference VRAM fit, live `VoxDb` per-task feedback, external benchmark priors, OpenRouter cache/credit state, and expiring rate-limit cooldowns, emitting per-turn candidate `ScoreParts` breakdowns.
**Depends on**: Phase 459
**Requirements**: SCORE-01, SCORE-02, SCORE-03, SCORE-04
**Success Criteria** (what must be TRUE):

  1. Unpinned chat and orchestrator turns route exclusively through `decide()` + `auto_score_parts` (`VOX_LOCAL_PREFERRED_TASKS`, `select_local_first`, and static `premium_alias` short-circuits removed; `RiskPosture::Low` multiplicative axis blending preserves `Free`/`Efficiency` caps; `Balanced` mode evaluates free, local, mesh, and paid candidates without `FreeInPerformanceMode` exclusion), using inference `serve_floor_mb` + dtype bytes-per-param + Metal/CUDA/NVML `VramProbe` instead of `122.8 GB` QLoRA training floors.
  2. `ModelRegistry` merges `(model_id, task_category)` `VoxDb` scoreboard rows without wiping `response_quality_ema` or `benchmark_prior`, feeds tool-call refusal/repair rates into `scoreboard_feedback_boost`, hydrates `CommunityIntelligenceBundleV1` + `ExternalBenchmarkPrior` + `AutonomicScoringOverlay` on boot/refresh, and breaks epsilon ties via Thompson `arm_stats`.
  3. Cloud candidate scoring incorporates native OpenRouter `input_cache_read` / `input_cache_write` pricing, live Hugging Face Router catalog entries, polled provider credit balances (`ProviderCreditSnapshot` via OpenRouter `/api/v1/auth/key`), `RemainingBudget` hints in `rank_pass`, and `Retry-After`-expiring `429` cooldowns (eliminating sticky 24h UTC lockouts).
  4. `ModelSelectionDecision` carries compact top-N `CandidateDto` (`ScoreParts`) and `ExclusionGroupDto` payloads, and `/api/v2/models/routing-explain` executes the exact same `decide()` pipeline as live Chat turns (`0` task-vs-chat explainer split-brain).

**Plans**: TBD

### Phase 461: Authenticated Populi Mesh LLM Dispatch (`iroh` QUIC) & 3-Tier Pre-First-Token Failover Cascade

**Directory**: `.planning/workstreams/local-model-lifecycle-v78/phases/461-populi-mesh-quic-dispatch-and-failover-cascade/`
**Goal**: Unify GUI/CLI and `iroh` QUIC peer trust, enable real streaming LLM + tool-call inference across trusted `PopuliMesh` GPU peers, enforce 3-level `VOX_INFERENCE_PRIVACY`, and execute deterministic pre-first-token failover across ranked candidates governed by `VOX_CLOUD_FALLBACK_POLICY`.
**Depends on**: Phases 459 and 460
**Requirements**: MESH-01, MESH-02, MESH-03, MESH-04
**Success Criteria** (what must be TRUE):

  1. Trusting or untrusting a node in Axis GUI (`commands/mesh.rs`) or CLI atomically syncs `TrustedNodeRegistry` (`~/.vox/trusted_nodes.json`) and `MeshTrust` (`~/.vox/mesh_trust.json`), immediately rejecting untrusted peers and closing active QUIC streams upon revocation.
  2. `iroh` `JobRequest::Probe` returns `MeshInferenceInventory` (peer models, `param_count_b`, quantization, free VRAM, warm residency, queue depth) registered as scored `ProviderType::PopuliMesh` candidates, and `JobRequest::InferChat` streams `ChatMessage` + tool schemas and `SseDeltaEvent` chunks + usage/TTFT/TPOT receipts end-to-end through `vox_actor_runtime::llm`.
  3. `VOX_INFERENCE_PRIVACY=local_only` restricts execution strictly to loopback `VoxLocal`/`Ollama` (`0` mesh or cloud packets), `mesh_only` permits loopback + trusted `PopuliMesh` peers (`0` cloud packets), and `any` permits all scored tiers.
  4. Pre-first-token (`ttft_ms.is_none()`) failures (`unreachable`, `timeout`, `429`, `5xx`, OOM/peer saturation) automatically fail over across the turn's ranked candidates (`Local <-> Trusted Mesh <-> Free Cloud <-> Paid Cloud`), enforcing `VOX_CLOUD_FALLBACK_POLICY` (`auto`, `free_cloud_first`, interactive `ask_before_cloud` consent prompt before paid/cloud egress, `never`) and recording structured `fallback_hops`.

**Plans**: TBD

### Phase 462: End-to-End Chat Provenance Persistence, Progressive-Disclosure Bubble & Hover "Why Chosen" Explainer

**Directory**: `.planning/workstreams/local-model-lifecycle-v78/phases/462-chat-provenance-persistence-and-hover-explainer/`
**Goal**: Guarantee 100% of live, fallback, escalated, background, `/plan`, regenerated, and reloaded Chat replies retain complete endpoint/model/score/hop provenance, declutter `MessageBubble` secondary actions via progressive disclosure, and render an always-available mouseover/focus "Why Chosen" provenance card.
**Depends on**: Phases 460 and 461
**Requirements**: PROV-01, PROV-02, PROV-03
**Success Criteria** (what must be TRUE):

  1. `ResolvedFrom` (`'catalog' | 'bootstrap' | 'local' | 'mesh'`), `contracts/gui/turn-event-kinds.v1.json`, and `turnEvents.ts` validate and carry XSS-safe `tier`, `provider`, sanitized `endpoint_label` (explicitly noting gateway egress such as OpenRouter), `selection_source`, top-N candidate `ScoreParts`, `fallback_hops`, and post-turn SOTA repair model escalations across sync, attachment, cognitive-profile, `/plan`, regenerate, and background turns.
  2. `persist_assistant_reply` and `chat_get_messages` persist and hydrate full `ChatTurnProvenanceDto` + `events` in `conversation_messages.payload_json` (falling back cleanly to `selectionReason` + `modelId` on pre-v7.8 rows) so reloading Axis GUI or switching sessions preserves 100% of hover provenance and `TurnTrace` steps.
  3. `MessageBubble` (`ChatTranscript.tsx`) eliminates the fabricated `'1.8s'` latency pill and dead `'tool_start'` counter, persists `Regenerate` updates end-to-end, tucks `Branch here`, `Rewind`, and `Compact Here` into a hover/focus `DropdownMenu` (`⋯`), and renders a compact inline `<TurnProvenancePill>` whose pointer-hover and keyboard-focus `<ProvenanceHoverPopover>` (`data-slot`, `--z-popover`) displays exact endpoint, model, orchestrator rationale, candidate score breakdown, cascade hops, and TTFT/TPOT/token/cost metrics.

**Plans**: TBD

### Phase 463: Cross-Surface Local/Mesh/Cloud Workbench Controls & CLI/MCP Parity

**Directory**: `.planning/workstreams/local-model-lifecycle-v78/phases/463-workbench-lifecycle-controls-and-cli-mcp-parity/`
**Goal**: Expose 1-click local model startup/unload/swap/pull and unified routing/credit/mesh observability across `vox` CLI, MCP tools, `Loquela`, `BottomStatusBar`, `ChatExecutionRail`, `ModelsView`, `Settings`, and `PopuliMeshBentoView`.
**Depends on**: Phases 459, 460, 461, and 462
**Requirements**: LADDER-04, PROV-04
**Success Criteria** (what must be TRUE):

  1. `vox model local {status, up, down, swap, pull}` and `vox_local_model_{status,up,down,swap,pull}` MCP tools (registered in `contracts/operations/catalog.v1.yaml` via `operations-sync`) drive `vox-orchestrator-d`'s `LocalModelSupervisor` and guided HF Hub checkpoint pulls cleanly across CLI, MCP, and GUI (`0` direct `invoke`/`listen` leaks outside `transport.ts`, `0` `VoxDb` handles in `vox-gui`).
  2. `Loquela` previews the live auto-selected target/tier before send and distinguishes `$0.000 (Local)` vs `$0.000 (Mesh)` vs `$0.000 (Free Cloud)` vs metered Cloud; `BottomStatusBar` opens a Radix `Popover` summarizing active local model/VRAM/leases, trusted mesh peers, cloud credits, and fallback policy.
  3. `ChatExecutionRail` renders empirical `postureAudit.classified_spoke` + last-turn vs next-turn route provenance (replacing hardcoded `Qwen3-8B` title heuristics), `ModelsView` pairs the unified `RoutingExplainer` with 1-click warm/unload/swap + `VOX_LOCAL_AUTO_LOAD` / `VOX_LOCAL_KEEP_ALIVE` toggles (showing `ServeablePassed` vs `TrainedUngated` / `TrainingInProgress` ladder badges), and `PopuliMeshBentoView` exposes live peer RTT/VRAM/model probe & pin actions.

**Plans**: TBD

### Phase 464: Live Production Model Verification, Dual-Theme Concept-Art Plates & 4-Theme CV Flywheel Convergence

**Directory**: `.planning/workstreams/local-model-lifecycle-v78/phases/464-live-prod-verification-and-cv-flywheel-closure/`
**Goal**: Verify live auto-scoring, lease warm/unload, failover, and progressive ladder promotion against real production local and cloud models, generate dual-theme `v78-concept-*` reference plates, and converge all modified surfaces and open hover portals across the 4-theme Computer-Vision Flywheel and workspace CI gates.
**Depends on**: Phases 459, 460, 461, 462, and 463
**Requirements**: CV-01, CV-02, CV-03, CV-04
**Success Criteria** (what must be TRUE):

  1. Live integration test suite executes real chat turns, auto-scoring comparisons, lease `0 -> 1 -> 0` warm/unload + Ollama `keep_alive: 0` VRAM eviction, and pre-first-token failover against real currently-available prod local MENS (`Qwen3` gated rungs) / Ollama models and live cloud endpoints, plus deterministic watcher verification proving automatic promotion when `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` gate receipts materialize, emitting `contracts/reports/v78-live-routing-and-lifecycle-proof.v1.json`.
  2. Dual-theme Concept-Art plates (`v78-concept-chat-provenance-{carrara,slate}.jpg` and `v78-concept-routing-lifecycle-{carrara,slate}.jpg`) are generated and wired into `vlmVisionFlywheel.ts`, passing `themeContract.test.ts`, `zeroEmojiAndDingbat.test.ts`, `noRawNativeControlsGuard.test.ts`, `noPaperFallbackGuard.test.ts`, and `ipcBoundaries.test.ts` with `0` violations and `<400`-LoC child modules.
  3. Multi-pass Playwright + `@ngi/platform-ssot` + VLM CV Flywheel across `900×600`, `1100×720`, `1280×800`, `1440×900`, and `1920×1080` and all 4 themes (`arcane`, `glacier`, `slate`, `high-contrast`) — including open `<ProvenanceHoverPopover>` and `BottomStatusBar` Radix portals — achieves `visualAfterScore >= 0.92`, `axeViolations === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `digitalColorLeakCount === 0`, and `rawEmojiCount === 0`.
  4. `cargo run -p vox-arch-check`, `vox audit code`, `vox ci operations-verify`, `cargo run -p vox-doc-pipeline -- --lint-only` (with `docs/src/architecture/local-mesh-cloud-lifecycle-and-routing-research-2026.md` promoted to `status: "current"`), and all affected Rust + Vitest suites pass with `0` errors.

**Plans**: TBD

---

## Progress

| Phase | Requirements | Plans Complete | Status | Completed |
|-------|--------------|----------------|--------|-----------|
| 459. Lifecycle/Fallback Config Contract, Daemon Lease Supervisor & Progressive MENS Readiness | `LIFE-01..04`, `LADDER-01..03` (7) | 0/TBD | Not started | - |
| 460. Unified Multi-Factor Auto Model Scorer, Live Cloud Catalog/Credit Signals & Candidate Score Breakdowns | `SCORE-01..04` (4) | 0/TBD | Not started | - |
| 461. Authenticated Populi Mesh LLM Dispatch (`iroh` QUIC) & 3-Tier Pre-First-Token Failover Cascade | `MESH-01..04` (4) | 0/TBD | Not started | - |
| 462. End-to-End Chat Provenance Persistence, Progressive-Disclosure Bubble & Hover "Why Chosen" Explainer | `PROV-01..03` (3) | 0/TBD | Not started | - |
| 463. Cross-Surface Local/Mesh/Cloud Workbench Controls & CLI/MCP Parity | `LADDER-04`, `PROV-04` (2) | 0/TBD | Not started | - |
| 464. Live Production Model Verification, Dual-Theme Concept-Art Plates & 4-Theme CV Flywheel Convergence | `CV-01..04` (4) | 0/TBD | Not started | - |
