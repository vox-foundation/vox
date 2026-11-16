---
title: "Local, Populi Mesh & Cloud Lifecycle and Model Routing Research (2026)"
description: "Architectural audit and v7.8 blueprint for lease-managed local MENS/Ollama supervision, progressive ladder readiness, dynamic multi-factor model scoring, Populi QUIC mesh inference, cloud credit/fallback governance, and progressive-disclosure Chat provenance."
category: "Architecture SSOTs"
status: "research"
topic: "model-routing"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
training_eligible: false
---

# Local, Populi Mesh & Cloud Lifecycle and Model Routing Research (2026)

<!-- kb:summary -->
Audits Vox's local model supervision, multi-rung MENS checkpoint readiness, multi-factor auto model scoring, Populi QUIC mesh inference, cloud credit and fallback governance, and Axis GUI Chat provenance for Milestone v7.8. Documents thirty-seven verified codebase gaps across daemon lease counting, vox mens serve and Ollama VRAM lifecycle, progressive Qwen3 and Qwen3.5 and Qwen3.8 gate-based rung promotion, ModelSelectionEngine scoring bypasses, authenticated iroh bi-stream LLM dispatch, persisted turn-level routing explanations, and four-theme Concept-Art CV Flywheel verification.
<!-- /kb:summary -->

## 1. Motivation & Design Principles

Operators running Vox across Axis GUI, `vox` CLI, and external MCP clients need local models (`vox mens serve` and Ollama), federated Populi Mesh GPU peers (`iroh` QUIC), free cloud inference providers, and metered frontier cloud APIs to operate as one coherent, observable system rather than disconnected silos:

1. **Effortless Lease-Counted Local Lifecycle:** Starting any Vox harness client should automatically warm the best hardware-fitting, gate-verified local model on launch (unless disabled in user config), share a single supervised local serving instance across GUI, CLI, and MCP, and cleanly unload `vox mens serve` and Ollama VRAM when the last harness lease closes (unless `keep_alive` is configured).
2. **Progressive Multi-Family MENS Readiness (Prod Today + Zero-Edit Promotion):** Only a subset of Nov 2026 MENS ladder rungs are gate-complete at any moment (`Qwen3-0.6B/1.7B/4B/8B/14B/32B` are gate-passed in prod today; `Qwen3.5-2B/4B/9B` have trained weights awaiting gate receipts; `Qwen3.8-27B` is actively training). Discovery and scoring must serve verified prod rungs immediately while automatically promoting newly completed rungs the moment their gate and collateral receipts materialize on disk or Hugging Face Hub.
3. **Dynamic Multi-Factor Auto Model Scoring (Zero Rigid Hardcoding):** Rather than hard-pinning a blind `Local -> Mesh -> Cloud` waterfall regardless of task or operator intent, `ModelSelectionEngine` and `auto_score_parts` must genuinely score all eligible candidates across task spoke/complexity, operator `ClutchProfile` (`Free | Efficiency | Balanced | Genius`), `RiskPosture`, `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), inference VRAM fit, live provider credit/rate-limit health, cache-read discounts, external benchmark priors, and empirical `VoxDb` TTFT/TPOT/compile/quality feedback.
4. **Progressive-Disclosure Chat Provenance & 4-Theme Concept-Art Fidelity:** Chat bubbles must remain visually clean (tucking secondary Fork/Rewind/Compact actions behind hover/overflow progressive disclosure) while ensuring every assistant turn — live, fallback, background, or history-hydrated — renders a crisp inline provenance pill whose mouseover/focus popover reveals the exact endpoint, model, score breakdown, cascade hops, and orchestrator rationale across all four Style Dictionary themes.

```vox-diagram
graph {
  direction: down
  node client: "Axis GUI / vox CLI / MCP Harness Lease"
  node daemon: "vox-orchestrator-d LocalModelSupervisor + LeaseRegistry"
  node scorer: "Unified Auto Model Scorer (Clutch + Risk + Privacy + Credits + Spoke)"
  node local: "Local Tier (vox mens serve :11435 / Ollama :11434)"
  node mesh: "Trusted Populi Mesh Tier (iroh QUIC JobRequest::InferChat)"
  node free_cloud: "Free Cloud Tier (HF Router / OpenRouter :free)"
  node paid_cloud: "Metered Cloud Tier (OpenRouter / Direct Vendors)"
  node chat_ui: "Persisted RoutingProvenanceDto + Progressive Hover Card"

  client -> daemon: "acquire / heartbeat / release lease"
  daemon -> local: "auto-warm on 0->1, drain + VRAM unload on 1->0"
  client -> scorer: "turn intent + posture + context_fill_ratio"
  scorer -> local: "ranked candidate + health probe"
  scorer -> mesh: "failover or higher-rung peer fit"
  scorer -> free_cloud: "when privacy=any and free/credits favor"
  scorer -> paid_cloud: "when genius/complexity + cloud_fallback_policy allow"
  scorer -> chat_ui: "top-N ScoreParts + ExclusionGroups + fallback_hops"
}
```

## 2. Verified Codebase Audit & Gap Matrix

### 2.1 Dimension 1 — Local Model Lifecycle, Daemon Supervision & Progressive Ladder Readiness (`G-LIFE-01..10`)

Crate-layer constraints (`docs/src/architecture/layers.toml`) place `vox-orchestrator-types` and `vox-llm-config` at L0, `vox-foundation` at L1, `vox-config` and `vox-eval` at L2, `vox-orchestrator` / `vox-orchestrator-mcp` / `vox-populi` / `vox-cli-core` at L3 (`vox-ml-cli` is `kind = "binary"`), and `vox-gui` / `vox-cli` / `vox-orchestrator-d` at L5. Shared supervision therefore belongs at L3 (`vox-cli-core::daemon_ipc` + `vox-orchestrator-mcp`).

| ID | Subsystem & Citation | Verified Finding | v7.8 Remediation Target |
|---|---|---|---|
| `G-LIFE-01` | `vox-orchestrator-d` (`vox_orchestrator_d.rs:208-317`, `orch_daemon/mod.rs:1775-1922`) | Daemon has zero client lease tracking (`acquire`/`heartbeat`/`release`) and zero idle reaper. | Add `LeaseRegistry` (`orch.lease.*` + implicit TCP subscription leases) triggering `0 -> 1` auto-load and grace-window `1 -> 0` auto-unload. |
| `G-LIFE-02` | `vox-gui` (`commands/mens_serve.rs:53-165`, `main.rs:466-481`) | `MensServeState` lives solely in Tauri RAM, starts manually only, spawns un-staged binaries from `target/`, and never releases leases on adopted daemons. | Lift supervision into daemon-owned L3 `LocalModelSupervisor` staging `vox-ml-cli` into `~/.vox/bin`; forward GUI/CLI/MCP controls through daemon RPCs. |
| `G-LIFE-03` | `vox-ml-cli` (`ai/serve/mod.rs:171`) & `vox-orchestrator-d` (`L306`) | Both servers lack `ctrl_c`/`SIGTERM` graceful shutdown; `mens serve` dies via `SIGKILL` without dropping loaded GPU handles. | Wire `with_graceful_shutdown`, loopback `/v1/admin/shutdown`, and clean worker channel teardown + Ollama eviction on daemon exit. |
| `G-LIFE-04` | `vox-ml-cli` (`ai/serve/mod.rs:96-138`) | Passing a run directory (`mens/runs/qwen3-0.6b`) causes `config.model_path.parent()` to check `mens/runs/training_manifest.json` (`None`), skipping `serve_fit_note` and truncating model IDs (`"qwen3-0"`). | Branch `is_dir()` vs file path in `ai/serve/mod.rs` so manifest VRAM floor checks and full rung slugs resolve accurately. |
| `G-LIFE-05` | `vox-orchestrator-mcp` (`providers/probe.rs:176-205`, `types.rs:98-105`) | Zero calls to Ollama `GET /api/ps` (active VRAM residency) or `POST /api/generate` with `keep_alive: 0` for VRAM eviction. | Implement `probe_ollama_ps`, `unload_ollama_model` (`keep_alive: 0`), and configurable `keep_alive` on `OllamaChatRequest`. |
| `G-LIFE-06` | `vox-gui` (`mens_serve.rs:108-110`) | `start_with` rejects model switches while running; simultaneous Ollama + `vox mens serve` loads risk double-VRAM OOM. | Implement serialized `hot_swap`: drain in-flight turns, unload prior `mens serve` / Ollama VRAM, poll `read_free_memory()` rebound, then warm target. |
| `G-LIFE-07` | `vox-llm-config` (`keys.rs:54-144`) & `vox-config` (`inference.rs:343`, `orch_daemon/mod.rs:423`) | `VOX_LOCAL_ENDPOINT` bypasses `~/.vox/config.toml`; lifecycle keys (`VOX_LOCAL_AUTO_LOAD`, `VOX_LOCAL_KEEP_ALIVE`, `VOX_LOCAL_PREFERRED_BACKEND`) do not exist; `orch.reload_config` ignores user config. | Register strongly-typed config keys in `LLM_CONFIG_KEYS` + `OPERATOR_TUNING_ENVS` and reload `~/.vox/config.toml` across processes via `orch.reload_config`. |
| `G-LIFE-08` | `vox-orchestrator` (`catalog.rs:967-971, 1218-1295`) & `vox-populi` (`size_select.rs:192-199`) | `MensCatalog` advertises ungated `qwen3.5-{2b,4b,9b}` runs that `vox mens serve` refuses to boot, while `size_select` hardcodes `"qwen3_code"` and ignores `"qwen3_5_code"`. | Classify runs into `ServeablePassed`, `TrainedUngated`, `TrainingInProgress`, and `HubPullablePassed`; route only `ServeablePassed` today (`qwen3-0.6b..32b`) and watch `mens/runs/` to auto-promote `qwen3.5-*` / `qwen3.8-27b` upon gate pass. |
| `G-LIFE-09` | `mens_corpus.rs:324`, `mens_run_reports.rs:718`, `mens_spoke_routing.rs:5` | Stale 5-rung Qwen3 lists, wrong spoke gate paths, and synthetic `qwen3_27b_metal_check` entries diverge from `hub_repo` SSOT. | Consolidate on `vox_populi::mens::hub_repo` (`SIZES` + `LEGACY_SIZES`) and `gate_policy::spoke_gate_rows`. |
| `G-LIFE-10` | `docs/src/architecture/layers.toml` | `vox-orchestrator` (`max_dependents = 30`) and `vox-populi` are peer L3 crates. | Keep gate-state types in `vox-eval` (L2) and process/mesh wiring in `vox-orchestrator-mcp` / `vox-cli-core` (L3) with zero new L3 cycles. |

### 2.2 Dimension 2 — Auto Model Scoring, Populi Mesh Dispatch & Cloud Credit Governance (`G-ROUTE-01..15`)

Inspection of `vox-orchestrator/src/models/{select.rs,scoring.rs,ranking.rs,registry.rs}` and `vox-orchestrator-mcp/src/llm_bridge/` revealed six scoring bypasses, disconnected scoreboard/benchmark telemetry, and un-wired Populi QUIC inference:

| ID | Subsystem & Citation | Verified Finding | v7.8 Remediation Target |
|---|---|---|---|
| `G-ROUTE-01` | `resolve.rs:192, 551-568` & `select.rs:878, 923` | `VOX_LOCAL_PREFERRED_TASKS` and `select_local_first` bypass `decide()` and `auto_score_parts` completely on `CodeGen` turns, ignoring `Genius` clutch and capability requirements. | Remove pre-scorer hijacks; feed local locality/zero-cost advantages into `auto_score_parts` so `Clutch` and `Risk` govern selection cleanly. |
| `G-ROUTE-02` | `mens_spoke_routing.rs:5-60` & `discovery_pipeline.rs:92` | Six phantom `VoxLocal` specs are seeded unconditionally and granted `ModelConfidence::Confirmed` without `BenchEvidence` or live health. | Register only gate-verified/live local models and derive MENS quality from measured 5-spoke/8-spoke eval receipts (`QualitySource::Measured`). |
| `G-ROUTE-03` | `auto_select.rs:74, 184` & `vram.rs:72-92` | Host inference sizing checks QLoRA **training** floors (`122.8 GB` for 8B!) while `estimate_vram_fit` lacks `param_count_b`, assumes Q4 for BF16/Q8, and ignores Apple Silicon Metal. | Separate **inference** KV+weight memory floors (`serve_floor_mb`) from training floors and wire unified Metal/CUDA/NVML `VramProbe` + dtype bytes-per-param. |
| `G-ROUTE-04` | `mode.rs:197-203` & `select.rs:885, 952-992` | `RiskPosture::Low` overwrites `Free`/`Efficiency` axes to `(15, 15, 70)`, triggering static `premium_alias` hard-selection instead of scoring frontier models. | Blend risk modifiers multiplicatively while preserving `force_free_pool` / tier caps, and replace static `premium_alias` short-circuit with live frontier scoring. |
| `G-ROUTE-05` | `clutch.rs:106` vs `select.rs:511` & `ranking.rs:177` | `SelectionAxes::BALANCED.to_cost_preference()` returns `Performance` while `ClutchProfile::Balanced` resolves `Economy`, excluding all free models in `Balanced` chat turns. | Unify `CostPreference` derivation so `Balanced` evaluates free, local, mesh, and paid models on one consistent Pareto frontier. |
| `G-ROUTE-06` | `route_policy.rs:16-21, 81-92` | `is_local_http_provider` groups `PopuliMesh` with `VoxLocal`/`Ollama`, leaking `local_only` turns onto network peers and lacking a `mesh_only` privacy tier. | Split `InferencePrivacy` into `local_only` (loopback only), `mesh_only` (loopback + trusted `PopuliMesh`), and `any` (local + mesh + cloud). |
| `G-ROUTE-07` | `try_agent_turn.rs:167-189` & `infer.rs:124-148` | No `cloud_fallback_policy` (`auto | free_cloud_first | ask_before_cloud | never`) exists; fallback jumps straight to arbitrary/cheapest cloud models. | Add typed `CloudFallbackPolicy` in `vox-config` + `PersistedSessionChatPosture`, enforcing free-cloud preference or interactive `ask_before_cloud` consent. |
| `G-ROUTE-08` | `try_agent_turn.rs:148-205` & `salvage_and_stream.rs:17-22` | Agent loop does not cascade across ranked `[#1, #2, #3]` candidates on pre-first-token `429`, `5xx`, timeout, or OOM errors. | Add pre-first-token (`ttft_ms.is_none()`) bounded candidate cascade across healthy tiers through `vox_actor_runtime::llm`, recording every hop. |
| `G-ROUTE-09` | `usage.rs:318, 420` & `ranking.rs:235` | Provider credit balances (e.g. OpenRouter `/api/v1/auth/key`) are unpolled; `ranking.rs` passes `hints: None`; a single `429` locks out a model for 24h UTC. | Poll provider credit/key limits (`ProviderCreditSnapshot`), pass live `RemainingBudget` hints into `rank_pass`, and expire `429` cooldowns via `Retry-After` / exponential window. |
| `G-ROUTE-10` | `registry.rs:170-184, 456-482` & `catalog_seed.rs:284` | `inject_evidence_from_db` wipes `response_quality_ema` and `benchmark_prior`, flattens task-specific scoreboard rows by `model_id`, and `from_cache()` skips `CommunityIntelligenceBundleV1` and `AutonomicScoringOverlay`. | Merge DB `(model_id, task_category)` telemetry non-destructively and hydrate `CommunityIntelligenceBundleV1`, `ExternalBenchmarkPrior`, and autonomic weights on boot/refresh. |
| `G-ROUTE-11` | `catalog.rs:88-91, 302, 841` & `select.rs:346` | `OpenRouterCatalog` drops native `input_cache_read` pricing, `HuggingFaceCatalog` is a 3-model bootstrap stub, `ScoreBreakdown` uses uncached efficiency, and tool-refusal rates do not adjust scores. | Parse native OpenRouter cache pricing, refresh live HF Router models, align `ScoreBreakdown` with cached efficiency, and feed tool-call success rate into `scoreboard_feedback_boost`. |
| `G-ROUTE-12` | `routing/engine.rs:40` vs `select.rs:994` | Thompson bandit `pick_with_auto_score_thompson` is disconnected from `decide()`. | Integrate bounded empirical bandit tie-breaking (`arm_stats`) into `decide()` when top candidates fall within an epsilon score band. |
| `G-ROUTE-13` | `vox-mesh-transport/src/protocol.rs:111`, `interp_executor.rs:478`, `turn_events.rs:117` | `PopuliMesh` peers advertise `VoxScript` only without VRAM/model metadata, and all three LLM egress mappers fail or misroute `PopuliMesh` to `127.0.0.1` / OpenRouter. | Bump mesh protocol with `MeshInferenceInventory` + streaming `JobRequest::InferChat` over authenticated `iroh` QUIC bi-streams and wire `ProviderType::PopuliMesh` into `vox_actor_runtime::llm`. |
| `G-ROUTE-14` | `vox-gui/src/commands/mesh.rs:42` vs `vox-mesh-transport/src/trust.rs:56` | GUI trust controls mutate `~/.vox/trusted_nodes.json` while `iroh` QUIC checks `~/.vox/mesh_trust.json`. | Unify peer trust synchronization so trusting/revoking a node in Axis GUI or CLI governs `iroh` QUIC inference peers immediately. |
| `G-ROUTE-15` | `select.rs:294`, `resolve.rs:596`, `turn_events.rs:67` | Full `Ranking` (`ScoreParts` + `ExclusionGroupDto`) is computed then discarded before building `routing_decision`, while `/api/v2/models/routing-explain` runs a different code path than Chat. | Unify `/api/v2/models/routing-explain` and live Chat on `decide()`, attaching compact top-N `CandidateScoreDto` + `ExclusionGroupDto` to every `routing_decision` event. |

### 2.3 Dimension 3 & 4 — Chat Provenance, Progressive-Disclosure UI & 4-Theme CV Flywheel (`G-UX-01..12`)

| ID | Subsystem & Citation | Verified Finding | v7.8 Remediation Target |
|---|---|---|---|
| `G-UX-01` | `commands/chat.rs:319-434` & `gui_ops/dto/chat.rs:21-48, 130-150` | `persist_assistant_reply` stores only 4 fields and drops `events`, `tokens`, `ttft_ms`, `cost_usd`, `tier`, `provider`, `endpoint_label`, and `fallback_hops`. | Persist full `ChatTurnProvenanceDto` + `events` in `conversation_messages.payload_json` and hydrate via `chat_get_messages`. |
| `G-UX-02` | `App.tsx:1235`, `ChatTranscript.tsx:208`, `TurnTrace.tsx:24` | Hydrated `selectionReason` on `ChatMessage` is ignored by `MessageBubble` and `TurnTrace`. | Wire `provenance` + `selectionReason` fallback chain so reloaded and legacy turns always display endpoint and rationale on hover. |
| `G-UX-03` | `agent_loop/turn_events.rs:31-150` | `model_spec_to_llm_config` discards resolved `base_url` / proxy egress (including `Anthropic`/`GoogleDirect` routed over OpenRouter). | Return sanitized `EndpointProvenance` (`tier`, `provider`, `endpoint_label`, `via_gateway`) from egress resolution into `AgentTurnResult` and `routing_decision`. |
| `G-UX-04` | `provenance.rs:36` & `turnEvents.ts:22, 95` | `PopuliMesh` maps to `ResolvedFrom::Local` and `RESOLVED_FROM` rejects `'mesh'`. | Add `'mesh'` to `ResolvedFrom`, `contracts/gui/turn-event-kinds.v1.json`, and `turnEvents.ts`. |
| `G-UX-05` | `message.rs:275, 532`, `try_agent_turn.rs:423`, `agent_loop.rs:378`, `chat_turn.rs:358, 423` | Attachment, cognitive-profile, `/plan`, and background turns emit `events: vec![]`; mid-turn SOTA repair escalation leaves `routing_decision.resolved_id` stale. | Finalize `routing_decision` *after* turn execution (capturing repair escalations and cascade hops) across sync, background, plan, and fallback tracks. |
| `G-UX-06` | `ChatTranscript.tsx:118-130, 217-283, 479-491` | Header fabricates `'1.8s'` latency, filters nonexistent `'tool_start'`, renders 5 permanent action buttons per bubble, and `Regenerate` drops state updates. | Remove fake `'1.8s'` pill, fix `Regenerate` state/DB persistence, and tuck `Branch here` / `Rewind` / `Compact Here` into a hover/focus progressive-disclosure `DropdownMenu`. |
| `G-UX-07` | `TurnTrace.tsx:52-66` & `ChatTurnEventRow.tsx:124-139` | Healthy non-tool turns collapse `routing_decision` with zero hover affordance. | Render an always-present compact `<TurnProvenancePill>` on every assistant reply with a keyboard/pointer `<ProvenanceHoverPopover>` (`data-slot`, `--z-popover`) showing endpoint, model, score breakdown, cascade hops, and metrics. |
| `G-UX-08`..`12` | `ChatExecutionRail.tsx:867`, `Loquela.tsx:1652`, `BottomStatusBar.tsx:233`, `ModelsView.tsx:241`, `PopuliMeshBentoView.tsx:209` | Fake string-matched MENS spoke badge, mislabeled `$0.000 (Local)` on cloud free tier, non-interactive status-bar routing tile, split-brain `RoutingExplainer`, and unlinked Mesh peer cards. | Hydrate all five surfaces from unified `decide()` + `LocalModelSupervisor` + `MeshInferenceInventory` DTOs, backed by 4-theme Concept-Art plates and multi-pass CV Flywheel verification (`visualAfterScore >= 0.92`, `axeViolations === 0`, `rawEmojiCount === 0`). |

## 3. Recommended v7.8 Phase Architecture (`Phases 459–464`)

1. **Phase 459 — Lifecycle & Fallback Config Contract, Daemon Lease Supervision & Progressive MENS Readiness (`G-LIFE-01..10`):** Typed `vox-config` lifecycle/privacy/fallback keys, `vox-orchestrator-d` `LeaseRegistry` + `LocalModelSupervisor` (`vox mens serve` graceful shutdown, `ai/serve` path fix, Ollama `/api/ps` + `keep_alive: 0` VRAM eviction, zero-double-VRAM hot-swap), and gate-verified multi-family MENS discovery (`0.6B..32B` live in prod today + automatic `notify`/watcher promotion as `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` pass gates).
2. **Phase 460 — Unified Multi-Factor Auto Model Scorer, Live Cloud Catalog/Credit Signals & Score Breakdowns (`G-ROUTE-01..05, 09..12, 15`):** Eliminate all 6 scoring bypasses, separate inference VRAM floors from training floors, wire OpenRouter cache pricing + HF Router live catalog + provider credit/rate-limit cooldowns + non-destructive `(model_id, task_category)` `VoxDb` scoreboard + `CommunityIntelligenceBundleV1` + `ExternalBenchmarkPrior`, unify `/api/v2/models/routing-explain` with `decide()`, and attach top-N `ScoreParts` + `ExclusionGroupDto` to every turn.
3. **Phase 461 — Authenticated Populi Mesh LLM Dispatch (`iroh` QUIC) & 3-Tier Pre-First-Token Cascade (`G-ROUTE-06..08, 13..14`):** Unify GUI/CLI `MeshTrust`, extend `iroh` probe with `MeshInferenceInventory` and streaming `JobRequest::InferChat`, split `VOX_INFERENCE_PRIVACY` (`local_only | mesh_only | any`), and wire pre-first-token candidate failover + `CloudFallbackPolicy` (`auto | free_cloud_first | ask_before_cloud | never`) through `vox_actor_runtime::llm`.
4. **Phase 462 — End-to-End Chat Provenance Persistence, Progressive-Disclosure Bubble & Hover "Why Chosen" Explainer (`G-UX-01..07`):** Persist full `ChatTurnProvenanceDto` + `events` across sync/background/plan/regenerate/reload, capture sanitized endpoint/egress labels and mid-turn repair escalations, remove the fake `'1.8s'` pill and 5-button clutter via progressive disclosure, and ship `<TurnProvenancePill>` + `<ProvenanceHoverPopover>` displaying endpoint, model, score breakdown, cascade hops, and turn telemetry on mouseover/focus.
5. **Phase 463 — Cross-Surface Local/Mesh/Cloud Workbench Controls (`ModelsView`, `Loquela`, `BottomStatusBar`, `ChatExecutionRail`, `PopuliMeshBentoView`) & CLI/MCP Parity (`G-UX-08..12`):** Expose `vox model local {status, up, down, swap, pull}` + `vox_local_model_*` MCP tools, 1-click GUI warm/unload/swap & auto-load/keep-alive controls, live `Loquela` route preview + accurate free/mesh/local cost labels, interactive `BottomStatusBar` routing/lease popover, and real spoke/peer bindings.
6. **Phase 464 — Live Production Model Matrix Verification, Dual-Theme Concept-Art Plates & 4-Theme CV Flywheel Closure:** Test live end-to-end chat routing, auto-scoring, lease warm/unload, and failover against real currently-available prod local MENS/Ollama models and live cloud endpoints, verify simulated auto-promotion of newly gated `Qwen3.5`/`Qwen3.8-27B` checkpoints, generate `v78-concept-*` Carrara/Slate plates, and converge all modified surfaces and open Radix hover portals across `900x600`..`1920x1080` and all 4 themes (`axeViolations === 0`, `rawEmojiCount === 0`, `visualAfterScore >= 0.92`).
