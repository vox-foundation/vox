# Requirements: Vox — Milestone v7.8 (`local-model-lifecycle-v78`)

**Defined:** 2026-11-14
**Workstream:** `.planning/workstreams/local-model-lifecycle-v78/`
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`) or federated sibling repositories with zero LLM split-brain, while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## v7.8 Requirements

Requirements for Milestone `v7.8` (*Lease-Managed Local Model Supervision, Progressive MENS Readiness, Multi-Factor Auto Model Scoring, Populi QUIC Mesh Inference & Progressive-Disclosure Chat Provenance*). Every requirement maps to exactly one phase in `ROADMAP.md` (`Phases 459–464`).

### Local Model Lifecycle & Daemon Supervision (`LIFE`)

- [ ] **LIFE-01**: Operator can configure local model auto-load on harness launch (`VOX_LOCAL_AUTO_LOAD`, default `true`), unload/keep-alive behavior on harness exit (`VOX_LOCAL_KEEP_ALIVE`, default `"0"` = unload on last lease close; supports duration `"5m"`/`"30m"` or `"forever"`), preferred local runtime (`VOX_LOCAL_PREFERRED_BACKEND = auto | mens_serve | ollama`), `VOX_LOCAL_ENDPOINT`, `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), and `VOX_CLOUD_FALLBACK_POLICY` (`auto | free_cloud_first | ask_before_cloud | never`) in `~/.vox/config.toml` and `Vox.toml` with live cross-process reload via `orch.reload_config` (`G-LIFE-07`).
- [ ] **LIFE-02**: Starting Axis GUI, `vox` CLI chat/agent sessions, or MCP clients acquires a tracked harness lease (`orch.lease.acquire` / `heartbeat` / `release` + TCP subscription lifetime) on `vox-orchestrator-d` (`LeaseRegistry`), automatically warming the configured or best-fitting local model on `0 -> 1` active leases (unless `VOX_LOCAL_AUTO_LOAD=false`) and releasing the lease cleanly on GUI/CLI exit including for adopted daemons (`G-LIFE-01`, `G-LIFE-02`).
- [ ] **LIFE-03**: When active harness leases drop `1 -> 0` (after a short grace window) or `vox-orchestrator-d` receives `SIGINT`/`SIGTERM`, `LocalModelSupervisor` gracefully shuts down staged `~/.vox/bin/vox-ml-cli mens serve` (`/v1/admin/shutdown` + `with_graceful_shutdown` dropping GPU handles without `SIGKILL`) and evicts Vox-loaded Ollama VRAM models via `GET /api/ps` + `POST /api/generate` (`keep_alive: 0`) unless `VOX_LOCAL_KEEP_ALIVE` retains residency (`G-LIFE-02`, `G-LIFE-03`, `G-LIFE-05`).
- [ ] **LIFE-04**: Operator or orchestrator can hot-swap active local models or switch between `vox mens serve` and Ollama via serialized drain -> unload/evict -> `read_free_memory()` rebound verification -> load (`LocalModelSupervisor::hot_swap`), resolving `:11434`/`:11435` port collisions automatically without double-VRAM OOM spikes (`G-LIFE-06`).

### Progressive MENS Ladder Readiness & Serve Hygiene (`LADDER`)

- [ ] **LADDER-01**: `vox-ml-cli mens serve` resolves run-directory vs file paths accurately in `ai/serve/mod.rs` (`is_dir()` branch), enforcing `training_manifest.json` / `serve_fit_note` VRAM checks and preserving full multi-family rung slugs (`qwen3-0.6b`, `qwen3.5-2b`, `qwen3.8-27b`) instead of truncating directory stems (`G-LIFE-04`).
- [ ] **LADDER-02**: Multi-family MENS discovery classifies `mens/runs/*` and `ladder_index.json` checkpoints into explicit readiness states (`ServeablePassed`, `TrainedUngated`, `TrainingInProgress`, `HubPullablePassed`), registering only `ServeablePassed` checkpoints (`GateState::Passed` + `collateral_damage == pass` — e.g. prod `Qwen3-0.6B/1.7B/4B/8B/14B/32B` today) in live `ModelRegistry` routing while surfacing ungated/training rungs truthfully in status DTOs (`G-LIFE-08`, `G-ROUTE-02`).
- [ ] **LADDER-03**: A live `mens/runs/` readiness watcher (`notify` / periodic daemon tick) and multi-family `size_select` (`qwen3_5_code` `0.6B/2B/4B/9B/27B` with `qwen3_code` fallback) automatically promote newly completed rungs (`Qwen3.5-2B/4B/9B`, `Qwen3.8-27B`, and spoke adapters) into `ModelRegistry` and auto-warm eligibility the moment passing gate + collateral receipts materialize, with zero code edits (`G-LIFE-08`, `G-LIFE-09`).
- [ ] **LADDER-04**: Operator can manage local models from CLI (`vox model local {status, up, down, swap, pull}`), MCP (`vox_local_model_*` synced in `contracts/operations/catalog.v1.yaml`), and Axis GUI with guided HF Hub checkpoint pull when no local weights exist yet (`G-LIFE-02`, `G-LIFE-10`).

### Dynamic Multi-Factor Auto Model Scoring & Cloud Credits (`SCORE`)

- [ ] **SCORE-01**: Every unpinned chat and orchestrator turn routes through unified `decide()` + `auto_score_parts` without pre-scorer hijacks (`VOX_LOCAL_PREFERRED_TASKS`, `select_local_first`, and static `premium_alias` short-circuits removed; `RiskPosture::Low` axis clobber and `Balanced` `CostPreference` free-model exclusion fixed), using dedicated inference `serve_floor_mb` + dtype bytes-per-param + unified Metal/CUDA/NVML `VramProbe` instead of QLoRA training VRAM floors (`G-ROUTE-01`, `G-ROUTE-03`, `G-ROUTE-04`, `G-ROUTE-05`).
- [ ] **SCORE-02**: `ModelRegistry` non-destructively merges per-`(model_id, task_category)` `VoxDb` scoreboard telemetry (`compile_rate`, `satisfaction`, `ttft_ms`, `tpot_ms`, `response_quality_ema`, tool-call refusal/repair rates), hydrates `CommunityIntelligenceBundleV1`, `ExternalBenchmarkPrior`, and `AutonomicScoringOverlay` on boot/refresh, and applies bounded Thompson bandit tie-breaking (`arm_stats`) across near-tied candidates (`G-ROUTE-10`, `G-ROUTE-11`, `G-ROUTE-12`).
- [ ] **SCORE-03**: Cloud model scoring integrates live OpenRouter native prompt-cache pricing (`input_cache_read` / `input_cache_write`), live Hugging Face Router catalog entries, polled provider credit balances (`OpenRouter` `/api/v1/auth/key` remaining credits / `ProviderCreditSnapshot`), `RemainingBudget` quota-scarcity hints in `rank_pass`, and expiring `Retry-After` `429` cooldowns (`G-ROUTE-09`, `G-ROUTE-11`).
- [ ] **SCORE-04**: `ModelSelectionDecision` retains compact top-N `CandidateDto` (`ScoreParts`: quality, efficiency, latency, availability, bonuses) and `ExclusionGroupDto` breakdowns on every turn, unifying `/api/v2/models/routing-explain` and live Chat routing on the exact same `decide()` pipeline (`G-ROUTE-15`, `G-UX-11`).

### Populi Mesh Inference Dispatch & 3-Tier Failover Cascade (`MESH`)

- [ ] **MESH-01**: Trusting or revoking a peer in Axis GUI (`commands/mesh.rs`) or CLI synchronizes `vox_identity::TrustedNodeRegistry` and `vox_mesh_transport::MeshTrust` (`~/.vox/mesh_trust.json`) atomically, immediately gating `iroh` QUIC peer connections (`G-ROUTE-14`).
- [ ] **MESH-02**: Trusted `PopuliMesh` peers advertise live `MeshInferenceInventory` (`ModelAdvertisement`s, `param_count_b`, quantization, `accelerator_free_mb`, warm `loaded_llm_models`, active queue depth) during `iroh` `JobRequest::Probe` and execute streaming chat + tool-call completions over authenticated `iroh` QUIC bi-streams (`JobRequest::InferChat`) through `vox_actor_runtime::llm` (`G-ROUTE-13`).
- [ ] **MESH-03**: `VOX_INFERENCE_PRIVACY` enforces three distinct boundaries across all routing and fallback paths: `local_only` (loopback `VoxLocal`/`Ollama` only, zero network egress), `mesh_only` (loopback + authenticated `PopuliMesh` peers, zero cloud egress), and `any` (Local, Trusted Mesh, and Cloud candidates) (`G-ROUTE-06`).
- [ ] **MESH-04**: Pre-first-token (`ttft_ms.is_none()`) failures (`unreachable`, `timeout`, `429`, `5xx`, OOM/saturation) cascade deterministically across ranked candidates (`Local <-> Trusted Mesh <-> Free Cloud <-> Paid Cloud`) via `vox_actor_runtime::llm` while honoring `VOX_CLOUD_FALLBACK_POLICY` (`auto`, `free_cloud_first`, `ask_before_cloud` interactive HITL confirmation, `never`) and recording every hop (`G-ROUTE-07`, `G-ROUTE-08`).

### Progressive-Disclosure Chat Provenance & Workbench Sync (`PROV`)

- [ ] **PROV-01**: `ResolvedFrom` (`'catalog' | 'bootstrap' | 'local' | 'mesh'`) and `contracts/gui/turn-event-kinds.v1.json` (`routing_decision`) carry XSS-safe `tier`, `provider`, sanitized `endpoint_label` (disclosing gateway egress such as OpenRouter), `selection_source`, top-N candidate `ScoreParts`, `fallback_hops`, and post-turn SOTA repair escalations across 100% of sync, attachment, cognitive-profile, `/plan`, regenerate, and background turns (`G-UX-03`, `G-UX-04`, `G-UX-05`).
- [ ] **PROV-02**: `persist_assistant_reply` and `chat_get_messages` persist and hydrate full `ChatTurnProvenanceDto` + `events` in `conversation_messages.payload_json` (with graceful `selectionReason`/`modelId` fallback for historical rows) so session reloads preserve full turn traces, endpoints, and score explanations (`G-UX-01`, `G-UX-02`).
- [ ] **PROV-03**: `MessageBubble` (`ChatTranscript.tsx`) removes the fabricated `'1.8s'` pill and dead `'tool_start'` counter, fixes `Regenerate` state/DB updates, tucks secondary actions (`Branch here`, `Rewind`, `Compact Here`) into a hover/focus progressive-disclosure `DropdownMenu`, and renders a clean inline `<TurnProvenancePill>` whose mouseover/keyboard-focus `<ProvenanceHoverPopover>` (`data-slot`, `--z-popover`) always displays exact endpoint, model, orchestrator "why chosen" rationale, top candidate score breakdown, cascade hops, and turn metrics (`G-UX-06`, `G-UX-07`).
- [ ] **PROV-04**: `Loquela` previews the live auto-resolved route and displays accurate tier cost labels (`Local` vs `Mesh` vs `Free Cloud` vs metered Cloud), `BottomStatusBar` opens an interactive routing/lease/credit breakdown popover, `ChatExecutionRail` binds real `postureAudit.classified_spoke` telemetry, `ModelsView` renders unified `RoutingExplainer` + 1-click local warm/unload/swap controls, and `PopuliMeshBentoView` exposes live peer RTT/model pin & probe actions (`G-UX-08`..`G-UX-12`).

### Live Production Verification, Concept-Art & 4-Theme CV Flywheel (`CV`)

- [ ] **CV-01**: Live production integration harness verifies real end-to-end auto-scoring, lease auto-warm/unload, Ollama `keep_alive: 0` eviction, and failover across currently gate-passed prod local MENS/Ollama models and live cloud endpoints, plus deterministic watcher auto-promotion when `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` gate receipts land.
- [ ] **CV-02**: Dual-theme Concept-Art reference plates (`v78-concept-chat-provenance-{carrara,slate}.jpg` and `v78-concept-routing-lifecycle-{carrara,slate}.jpg`) are generated via `image-generator` and wired into `vlmVisionFlywheel.ts` with 100% 4-theme semantic token parity (`arcane`, `glacier`/`travertine`, `slate`/`void`, `high-contrast`), `<400`-LoC modules, `0` raw `<select>`/`range` tags, and `rawEmojiCount === 0`.
- [ ] **CV-03**: Multi-pass Playwright + `@ngi/platform-ssot` + VLM CV Flywheel (`900x600`, `1100x720`, `1280x800`, `1440x900`, `1920x1080`) verifies open `<ProvenanceHoverPopover>` and `BottomStatusBar` Radix portals in-document with `axeViolations === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `digitalColorLeakCount === 0`, and `visualAfterScore >= 0.92`.
- [ ] **CV-04**: `vox-arch-check`, `vox audit code` (`llm_provider_call`, `vox/ui/theme-contract-drift`, `arch/god_object`), `vox ci operations-verify`, `vox-doc-pipeline --lint-only` (promoting `local-mesh-cloud-lifecycle-and-routing-research-2026.md` to `status: "current"`), and all Vitest/Rust regression suites pass with `0` errors.

## Future Requirements (`v7.9+`)

- **MESH-WAN-01**: Cryptographic credit settlement and untrusted WAN peer sandboxing for open public Populi inference marketplaces beyond `MeshTrust`-paired operator nodes.
- **STREAM-SPLICE-01**: Mid-sentence cross-family KV/token splicing after partial tokens have already rendered to the transcript (pre-first-token failover + post-turn repair escalation cover `v7.8`).

## Out of Scope

| Feature | Reason |
|---------|--------|
| Direct vendor HTTP SDK calls bypassing `vox_actor_runtime::llm` | Violates `AGENTS.md §Model-Agnostic LLM Boundary` (`llm_provider_call` hard error) |
| Direct `VoxDb` handles or child CLI-DB spawns inside `vox-gui` | Violates Axis IPC boundary; all GUI mutations/queries traverse `voxTransport` -> `vox-orchestrator-d` |
| Unauthenticated LAN HTTP inference exposure for `PopuliMesh` | Security hazard; peer inference runs strictly over `MeshTrust`-authenticated `iroh` QUIC streams |
| Hardcoded per-task model pins bypassing `decide()` | Replaced by continuous multi-factor scoring (`Clutch`, `Risk`, spoke evidence, VRAM fit, credits) |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| LIFE-01 | Phase 459 | Pending |
| LIFE-02 | Phase 459 | Pending |
| LIFE-03 | Phase 459 | Pending |
| LIFE-04 | Phase 459 | Pending |
| LADDER-01 | Phase 459 | Pending |
| LADDER-02 | Phase 459 | Pending |
| LADDER-03 | Phase 459 | Pending |
| LADDER-04 | Phase 463 | Pending |
| SCORE-01 | Phase 460 | Pending |
| SCORE-02 | Phase 460 | Pending |
| SCORE-03 | Phase 460 | Pending |
| SCORE-04 | Phase 460 | Pending |
| MESH-01 | Phase 461 | Pending |
| MESH-02 | Phase 461 | Pending |
| MESH-03 | Phase 461 | Pending |
| MESH-04 | Phase 461 | Pending |
| PROV-01 | Phase 462 | Pending |
| PROV-02 | Phase 462 | Pending |
| PROV-03 | Phase 462 | Pending |
| PROV-04 | Phase 463 | Pending |
| CV-01 | Phase 464 | Pending |
| CV-02 | Phase 464 | Pending |
| CV-03 | Phase 464 | Pending |
| CV-04 | Phase 464 | Pending |

**Coverage:**
- v7.8 requirements: 24 total
- Mapped to phases: 24 (`Phases 459–464`)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-14*
*Last updated: 2026-11-14 after initial v7.8 definition*
