# Architecture Research: Adaptive Model Scaling, PopuliMesh Dispatch & HF Hub Cards (v9.2)

**Domain:** Adaptive local/mesh model scaling, iroh `PopuliMesh` inference dispatch, Axis UI routing surfaces, and Hugging Face Hub card/asset generation in the Vox workspace.
**Researched:** 2026-11-19
**Confidence:** HIGH (verified directly against Rust/TypeScript source, `layers.toml`, Askama templates, and YAML specs)

---

## Standard Architecture

### Workspace Layer Constraints (`docs/src/architecture/layers.toml`)

All changes in Milestone v9.2 MUST respect the dependency direction `L0 -> L1 -> L2 -> L3 -> L4 (runtime-loaded plugins) -> L5 (surfaces)` enforced by `vox-arch-check`:

| Layer | Crates Relevant to v9.2 | Key Boundary Rules |
|---|---|---|
| **L0** (Foundation types) | `vox-orchestrator-types`, `vox-llm-config`, `vox-mesh-types` (+ `[planned]` `vox-mesh-models`) | Zero heavy runtime/IO deps; safe shared wire DTOs (`TaskKind`, `TaskSpec`). |
| **L1** (Protocol & policy primitives) | `vox-hf-layout`, `vox-secrets`, `vox-http-client`, `vox-openai`, `vox-scaling-policy`, `vox-telemetry`, `vox-mesh-transport` | `vox-mesh-transport` owns iroh QUIC `ALPN = b"vox/job/1"`, `JobExecutor`, `directory`, `MeshTrust`. |
| **L2** (Config & egress) | `vox-config`, `vox-llm-egress`, `vox-repository`, `vox-populi-types`, `vox-eval`, `vox-mesh-policy` | `vox-config::resolve_egress` resolves HTTP `EgressRequest` + keys; cannot depend on L3. |
| **L3** (Orchestration, runtime, MCP, Mens) | `vox-orchestrator`, `vox-orchestrator-models`, `vox-actor-runtime`, `vox-populi`, `vox-orchestrator-mcp`, `vox-gamify`, `vox-ml-cli` (`binary`) | **No L3-to-L4 compile deps.** `vox-actor-runtime` MUST NOT depend on `vox-orchestrator` (cycle). `vox-orchestrator` exceeds its `max_loc = 70_000` budget (~85.5K noted in `layers.toml`), so new transport/scaling modules should prefer L0/L1/`vox-populi`/`vox-orchestrator-models` over growing `vox-orchestrator`. |
| **L4** (Dynamic `cdylib`/`rlib` plugins) | `vox-plugin-nvml-probe`, `vox-plugin-populi-mesh`, `vox-plugin-mens-candle-{core,cuda,metal}` | Loaded dynamically at runtime by `vox-plugin-host`; NEVER compile-time deps of L0–L3. |
| **L5** (Operator surfaces) | `vox-cli`, `vox-gui`, `vox-orchestrator-d` | Hosts register L4 hooks (e.g., `register_nvml_vram_probe()`), expose Tauri/HTTP/CLI controls. |

### System Overview (Four Subsystems & Their Current Disconnects)

```
+-----------------------------------------------------------------------------------+
| L5 Surfaces: vox-gui (ModelsView, MensServePanel, ChatExecutionRail) / vox-cli   |
+-----------------------------------------+-----------------------------------------+
                                          | Tauri / HTTP /api/v2/models/*
+-----------------------------------------v-----------------------------------------+
| L3 MCP & Ops: vox-orchestrator-mcp (gui_ops, model_route_policy, llm_bridge)      |
|  - local_health & gui_ops treat PopuliMesh as :11434 HTTP Ollama                  |
|  - endpoint_for / bearer_for reject ProviderType::PopuliMesh                      |
+--------------------+---------------------------------------+----------------------+
                     |                                       |
+--------------------v--------------------+   +--------------v----------------------+
| L3 vox-orchestrator (Catalog & Scoring) |   | L3 vox-actor-runtime / vox-gamify   |
|  - PopuliMeshCatalog -> iroh Probe      |   |  - StreamRoute::Cascade drops peer  |
|  - MensCatalog / OllamaCatalog / HF     |   |  - llm_chat -> L2 resolve_egress    |
|  - vram.rs (NVML only; -0.05 penalty)   |   |    ("populi_local" -> :11434 HTTP)  |
|  - auto_select -> qwen3_code (stale)    |   +-------------------------------------+
|  - llm_config_for_spec -> OpenRouter!   |
+--------------------+--------------------+
                     | iroh Discovery vs Missing Dispatch
+--------------------v--------------------+   +-------------------------------------+
| L1 vox-mesh-transport (vox/job/1 QUIC)  |   | L3 vox-populi::mens (Hub & Scaling) |
|  - InterpExecutor only runs VoxScript   |   |  - gpu-specs.yaml (qwen3_5_code)    |
|  - Rejects TaskKind::TextInfer!         |   |  - hub_card (Askama + static JPEGs) |
|  - L4 vox-plugin-populi-mesh: HTTP 9847 |   |  - hub_sync / hub_cards_push        |
+-----------------------------------------+   +-------------------------------------+
```

### Component Responsibilities

| Component | Responsibility | Current Implementation Seam |
|---|---|---|
| **VRAM & Hardware Probe** | Detect free/total device memory without linking L4 into L3 | `vox-orchestrator/src/models/vram.rs` (`VRAM_PROBE`, `refresh_free_vram_hint_from_nvml`) + `vox-orchestrator-mcp/src/server_state/vram_probe.rs` (`register_nvml_vram_probe`) + `vox-populi/src/mens/tensor/size_select.rs` (`read_free_memory` w/ Metal + `nvidia-smi`) |
| **Adaptive Size Selector** | Pick largest fitting ladder rung for host/peer free memory | `vox-orchestrator/src/models/auto_select.rs` (`AutoModelSelection`, `select_for_free_memory`, `select_optimal_local_model_for_host`) + `vox-populi/src/mens/tensor/{size_select.rs,spoke_base_resolver.rs}` + `mens/config/gpu-specs.yaml` |
| **Model Catalogs & Registry** | Discover/register local, HuggingFace, Ollama, Mens, and Mesh models | `vox-orchestrator/src/catalog.rs` (`OllamaCatalog`, `HuggingFaceCatalog`, `PopuliMeshCatalog`, `MensCatalog`) + `orchestrator/catalog_refresh.rs` + `models/registry/{catalog_seed,mens_spoke_routing,selection_queries}.rs` |
| **Scoring & Policy Resolution** | Score candidates across intelligence, cost, latency, VRAM, and locality | `vox-orchestrator/src/models/{select.rs,scoring.rs,ranking.rs,mode_select.rs,key_guard.rs}` + `vox-orchestrator-mcp/src/llm_bridge/model_route_policy/{resolve,policy,types}.rs` |
| **Mesh Discovery & Transport** | Discover trusted peers & execute remote work over iroh QUIC | `vox-orchestrator/src/models/mesh_directory.rs` + `vox-mesh-types/src/task.rs` (`TaskKind::TextInfer`) + `vox-mesh-transport/src/{endpoint,protocol,directory,interp_executor}.rs` + `vox-ml-cli/src/commands/mesh_cli.rs` |
| **Inference Egress & Streaming** | Execute selected route (`OpenAI`/`Ollama`/`PopuliMesh`/`VoxLocal`) | `vox-orchestrator/src/runtime.rs` (`stream_route_for_model_backend`) + `vox-gamify/src/ai/client/{mod,ctor}.rs` (`StreamRoute`) + `vox-actor-runtime/src/{model_resolution,llm/chat}.rs` + `vox-config/src/resolve_egress.rs` |
| **Axis UI & MCP GUI Ops** | Surface availability, routing reasons, active model, and server controls | `vox-orchestrator-mcp/src/gui_ops/{models.rs,dto/models.rs,dto/routing_explain.rs}` + `vox-gui/src/commands/models_host.rs` + `vox-gui/ui/src/components/surfaces/{Models/*,Chat/ChatExecutionRail.tsx}` |
| **HF Hub Cards & Push Pipeline** | Render validated model/index/Space cards + collateral and sync to HF | `vox-populi/src/mens/hub_card/{config,facts,card_facts,index_facts,render,svg,lints}.rs` + `mens/hub_repo.rs` + `mens/hub_sync.rs` + `mens/hub_cards_push.rs` + `templates/hub/**` + `mens/config/hub-cards.yaml` |

---

## Recommended Project Structure (Changes Targeted for v9.2)

```
crates/
├── vox-mesh-types/src/task.rs                 # Add TextInfer wire request/response schema DTOs (L0)
├── vox-mesh-transport/src/
│   ├── protocol.rs                            # Optional streaming token chunks or structured TextInfer Output (L1)
│   └── infer_executor.rs (new)                # Composite JobExecutor delegating VoxScript->InterpExecutor & TextInfer->local Candle/Ollama runtime (L1/L3 boundary)
├── vox-orchestrator/src/
│   ├── catalog.rs                             # Populate MensCatalog param_count_b; replace static HuggingFaceCatalog defaults or mark provenance
│   ├── runtime.rs                             # Preserve peer EndpointId/model on ModelRouteBackend::PopuliMesh instead of StreamRoute::Cascade
│   └── models/
│       ├── vram.rs                            # Unified host VRAM/unified-memory snapshot (NVML probe + Metal/sys fallback) & stronger Oversize/Tight penalties
│       ├── auto_select.rs                     # Default to qwen3_5_code ladder + expose host_free_memory / fitted rung consistently
│       ├── registry/mens_spoke_routing.rs     # Remove hardcoded 27.0 param_count_b & stale qwen3_27b_metal_check / vox-mens-8b-v0.6 IDs
│       └── registry/selection_queries.rs      # Stop mapping ProviderType::PopuliMesh -> LlmConfig::openrouter()
├── vox-orchestrator-mcp/src/
│   ├── llm_bridge/
│   │   ├── local_health.rs                    # Separate PopuliMesh (iroh Probed peers) from local :11434 Ollama health
│   │   ├── model_route_policy/{policy,resolve}.rs
│   │   ├── provider_adapter.rs / infer.rs     # Add MeshInferAdapter dialing mesh_directory / vox_mesh_transport instead of OpenAiCompatAdapter failure
│   │   └── ...
│   └── gui_ops/{models.rs,dto/models.rs,dto/routing_explain.rs}
│                                              # Separate PopuliMesh availability, expose VRAM headroom & AutoModelSelection rung
├── vox-populi/
│   ├── src/mens/hub_card/{svg,card_facts,index_facts,lints,config}.rs
│   │   # Dynamic data-driven SVG charts alongside static 16:9 JPEG banners; fix onboarding.md stale symbol
│   ├── src/mens/hub_cards_push.rs             # Allow .svg in is_card_collateral if data SVGs are reintroduced alongside JPEGs
│   ├── src/mens/hub_sync.rs                   # Stop deleting assets/{scorecard,overview}.svg if generated dynamically
│   └── templates/hub/partials/onboarding.md   # Fix deleted resolve_chat_provider_route reference
└── vox-gui/ui/src/components/surfaces/
    ├── Chat/ChatExecutionRail.tsx             # Replace hardcoded "· Qwen3-8B" on all 7 spoke branches with selected/active rung
    └── Models/{ModelsView,BackendAvailability,MensServePanel,RoutingExplainer}.tsx
                                               # Surface VRAM headroom, adaptive rung recommendation, PopuliMesh peers, HF card sync
```

### Structure Rationale

1. **Respect `vox-orchestrator` LoC ceiling:** `vox-orchestrator` is already over its `max_loc = 70_000` budget (~85.5K). Shared mesh text-inference wire DTOs belong in `vox-mesh-types` (L0) and iroh job/client execution helpers belong in `vox-mesh-transport` (L1) or `vox-actor-runtime` (L3), not inside `vox-orchestrator`.
2. **Respect L3 cycle & L4 isolation:** Neither `vox-actor-runtime` nor `vox-gamify` may import `vox-orchestrator`, and no L0–L3 crate may compile-time link L4 plugins (`vox-plugin-mens-candle-*`, `vox-plugin-nvml-probe`). Therefore, remote `PopuliMesh` execution in `vox-actor-runtime` / `vox-orchestrator-mcp` should dial `vox-mesh-transport` (L1) using the peer `EndpointId` (and trust/addresses from `MeshTrust`), while the peer-side daemon (`vox mesh join` / `vox-ml-cli`) supplies a `JobExecutor` backed by a local inference engine (local Ollama HTTP / Candle plugin / `vox mens serve`).

---

## Architectural Patterns

### Pattern 1: Exact `PopuliMesh` Execution Wire Gap (Before vs. After)

Today, `PopuliMesh` works **only for discovery and scoring**, and **fails or misroutes at every execution boundary**:

**Before (Current Broken Path — 4 Split-Brain Failure Modes):**

```
[1. Discovery & Scoring — WORKS]
  mesh_directory::trusted_peers() ──(iroh ALPN b"vox/job/1" JobRequest::Probe)──> Peer
         │
         ▼
  PopuliMeshCatalog::discover_populi_mesh_models()
    registers ModelSpec { id: "mesh/<endpoint_id>/<kind>", provider_type: PopuliMesh, is_free: true }
         │
         ▼
  key_guard::provider_secret_is_available(PopuliMesh) == true
  scoring.rs gives +0.8 (prefer_mesh) / +0.1 (free) / +0.15 (reciprocity)

[2. Execution Dispatch — BROKEN AT ALL 4 CALLSITES]
  ├── (A) vox-orchestrator/src/runtime.rs:46-48
  │     ModelRouteBackend::PopuliMesh ──> StreamRoute::Cascade (peer EndpointId DROPPED!)
  │                                       └─> vox-gamify::generate_stream_routed (HTTP Ollama/OpenRouter/Gemini)
  │
  ├── (B) vox-orchestrator/src/models/registry/selection_queries.rs:724
  │     llm_config_for_spec(PopuliMesh) ──> LlmConfig::openrouter("mesh/<peer>/<kind>") -> OpenRouter 404/error
  │
  ├── (C) vox-orchestrator-mcp/src/llm_bridge/{infer,provider_endpoints,provider_auth,turn_events}.rs
  │     ├── mcp_infer_tool_completion -> OpenAiCompatAdapter -> endpoint_for / bearer_for
  │     │     └─> Err(HttpInferError: "not applicable to provider PopuliMesh") -> records peer_reputation = "fail"
  │     └── try_agent_turn -> model_spec_to_llm_config -> provider = "populi_local", base_url = None
  │           └─> vox_config::resolve_egress -> VOX_LOCAL_ENDPOINT (http://127.0.0.1:11434/v1/chat/completions)
  │
  └── (D) vox-ml-cli/src/commands/mesh_cli.rs:234 + vox-mesh-transport/src/interp_executor.rs:478
        Peer's serve() only mounts InterpExecutor:
        ├── Probe responds: task_kinds = [TaskKind::VoxScript], engines = []
        └── JobRequest::Run { kind: TaskKind::TextInfer } ──> JobResponse::Failed("no engine installed for text_infer")
```

Additionally, `vox-orchestrator-mcp/src/llm_bridge/{local_health.rs, model_route_policy/policy.rs}` and `gui_ops/models.rs` check `PopuliMesh` availability by probing `:11434` (`/api/tags`), falsely equating a P2P iroh mesh peer with a local Ollama daemon, while `vox-plugin-populi-mesh` / `vox-populi/src/http_client.rs` run an older `/v1/populi/*` HTTP control plane on `127.0.0.1:9847` marked `vox-deprecated-since="0.6.0"` toward `vox_mesh_transport`.

**After (Unified iroh QUIC Dispatch):**

```
[1. Discovery]
  Peer runs Composite JobExecutor (VoxScript -> InterpExecutor; TextInfer -> local inference backend)
         │  responds to JobRequest::Probe with task_kinds: [VoxScript, TextInfer], engines: [model_id, ...]
         ▼
  PopuliMeshCatalog::discover_populi_mesh_models()
    registers ModelSpec { id: "mesh/<endpoint_id>/text_infer", canonical_model / engine metadata, param_count_b }

[2. Execution (Orchestrator / MCP / Actor-Runtime)]
  ModelRouteBackend::PopuliMesh { endpoint_id, model }
         │
         ├── Orchestrator AiTaskProcessor / vox-gamify StreamRoute (or Actor-Runtime)
         │     routes PopuliMesh via vox-mesh-transport (L1) instead of StreamRoute::Cascade / LlmConfig::openrouter
         │
         └── MCP llm_bridge & agent_loop
               ├── local_health reports iroh Probed peers (with TextInfer) separately from local Ollama :11434
               └── MeshInferAdapter / PopuliMesh egress dials EndpointId via iroh ALPN b"vox/job/1"
                     ──> JobRequest::Run { kind: TaskKind::TextInfer, payload_bytes: JSON TextInferRequest }
                     <── JobResponse::Output(JSON TextInferResponse) (or streamed chunks on vox/job/1 v3)
```

**Design note on `TextInfer` payload & streaming:**
- `vox-mesh-types/src/task.rs` already defines `TaskKind::TextInfer` (`"text_infer"`), and `vox-workflow-runtime/src/workflow/populi.rs` (`run_on_peer`) already implements the canonical iroh client framing (`Hello::current()`, `JobRequest::Run`, 16 MiB `read_frame`).
- Because `protocol::PROTO = 2` currently exchanges a single request payload and a single `JobResponse::Output(Vec<u8>)`, non-streaming `TextInfer` fits **zero-wire-breakage** on `PROTO = 2` using a shared serde DTO in `vox-mesh-types` (`L0`), while token-by-token streaming requires either a `JobResponse::Chunk` / `PROTO = 3` extension in `vox-mesh-transport` or buffered fallback (`Output` emitted as a single terminal stream delta in `StreamRoute`).
- `vox-actor-runtime/src/model_resolution.rs` (`ChatProviderRouteKind::PopuliMesh { base_url, model }`) currently represents HTTP `http(s)` base URLs (mapping to `"ollama"` `{base}/v1/chat/completions`); if iroh peer routes flow through `vox-actor-runtime`, `ChatProviderRouteKind` needs a distinct `IrohMesh { endpoint_id, model }` or `mesh://` handling separate from HTTP `ollama`.

---

### Pattern 2: Adaptive Model Scaling (Consolidating Two Disconnected Engines)

Currently, Vox has **two separate sizing systems** that disagree on model families, hardware sources, and parameter counts:

1. **QLoRA Rung Selector (`vox-orchestrator/src/models/auto_select.rs` + `vox-populi/src/mens/tensor/size_select.rs` + `mens/config/gpu-specs.yaml`):**
   - `size_select::read_free_memory()` probes `cuda_free_memory()` (`nvidia-smi` / CUDA) **plus** macOS Metal unified working-set memory and `/proc/meminfo` fallback.
   - However, `auto_select::select_for_free_memory` calls `size_select::select_size(&overlay, free, None, false)` — passing `None` defaults to `"qwen3_code"` (stale `0.6B/1.7B/8B/14B/32B` where `8B/14B/32B` in `gpu-specs.yaml` carry unmeasured placeholder floors `122876 / 195382 / 295822` MB!), ignoring the active `"qwen3_5_code"` ladder (`0.8B / 2B / 4B / 9B / 27B`, `floor_measured: true`).
   - Only surfaced via standalone Tauri command `get_auto_model_recommendation` (`vox-gui/src/commands/models_host.rs`); not integrated into `MensServePanel.tsx`, `ModelsView.tsx`, or MCP `/api/v2/models/*`.

2. **Registry / Scorer VRAM Fit (`vox-orchestrator/src/models/{vram,scoring,registry/mens_spoke_routing}.rs` + `catalog.rs`):**
   - `vram.rs` ONLY reads `VRAM_PROBE` (L4 `vox-plugin-nvml-probe` registered at host startup via `register_nvml_vram_probe()`). On Apple Silicon Metal, AMD, or hosts without `nvml-probe` installed, `FREE_VRAM_MB_HINT` stays `None` and `estimate_vram_fit()` returns `VramFit::Unknown`.
   - `estimate_vram_fit()` requires `spec.capabilities.param_count_b`. `OllamaCatalog` parses `/api/tags` parameter size (`parse_ollama_parameter_size`), **but `MensCatalog` (`catalog.rs`) never sets `param_count_b` from run manifests/labels**, `HuggingFaceCatalog` returns 3 static hardcoded strings without parameter metadata, and `mens_spoke_routing::register_calibrated_mens_adapter()` hardcodes `param_count_b: Some(27.0)` for every spoke regardless of whether the trained rung is `0.8B`, `2B`, `4B`, `9B`, or `27B`!
   - Even when `VramFit::Exceeds` fires, `scoring.rs` only subtracts `VRAM_EXCEEDS_PENALTY = -0.05` (negligible vs `+0.8` `prefer_mesh` or `+0.25` `mens` bonus), and `VramFit::Tight` applies `0.0`.
   - Hardcoded legacy defaults remain in `mens_spoke_routing.rs` (`qwen3_27b_metal_check/*`, `vox-mens-8b-v0.6`) and `ChatExecutionRail.tsx:873-880` (`· Qwen3-8B` on all 7 spoke branches).

**Target Adaptive Scaling Architecture:**
- **Single Memory-Probe Contract:** Feed `vram::refresh_free_vram_hint()` from `VRAM_PROBE` (NVML) when present, with fallback to host unified/Metal/system memory telemetry (`vox-populi` / `sysinfo` / Metal probe) labeled with provenance (`Nvml | Metal | SystemRam`).
- **Accurate Parameter Metadata Everywhere:** Derive `param_count_b` dynamically in `MensCatalog` (from `config.json` / `adapter_manifest.json` / rung size `0.8B..27B`), in `register_calibrated_mens_adapter`, and in `PopuliMeshCatalog` (advertised via `Probe` `engines` metadata).
- **Scale-Aware Scoring & Routing:** Align `auto_select` default overlay tag to `"qwen3_5_code"` (`0.8B / 2B / 4B / 9B / 27B`), promote `VramFit` in `ranking.rs`/`scoring.rs` (hard filter or strong penalty on `Exceeds` when an adapter/rung that fits `Comfortable`/`Tight` exists, plus `Tight` tie-breaker), and propagate selected model label + fit state to `/api/v2/models/*` and `ChatExecutionRail.tsx`.

---

### Pattern 3: Hugging Face Hub Cards & Visual Assets (`vox-populi/src/mens/hub_card/`)

Currently, the HF Hub Card pipeline is **purely deterministic Rust + Askama** driven by strict release-gate evidence:

1. **Ladder SSOT (`hub_repo.rs` + `mens/config/hub-cards.yaml` + `hub_card/config.rs`):**
   - Active ladder (`SIZES`): `["0.8B", "2B", "4B", "9B", "27B"]` (`qwen3.5` for `0.8B..9B`, `qwen3.8` for `27B`; `LEGACY_SIZES = ["0.6B", "1.7B", "8B", "14B", "32B"]`).
   - `hub-cards.yaml` (`schema: vox_mens_hub_cards_v1`) stores editorial `why` blurbs (validated as 2–3 sentences, zero stray digits) and spoke descriptions (`vox-lang`).
2. **Evidence & Fact Extraction (`hub_card/{facts,card_facts,index_facts}.rs`):**
   - `RunRecords::load` reads run receipts (`gate_receipt.json`, `collateral_damage_report.json`, held-out reports, `telemetry_loss` / `telemetry_raw`, `measured_peak_mb`, `adapter_manifest.json`).
   - Strict truthfulness gate: `GateState::Passed` and `AdapterEvidence::Current` (`CURRENT_ADAPTER_SERVING = "nf4_base_plus_lora_arm v2"`, `PROMPT_PROTOCOL = "qwen3_non_thinking_codegen_v1"`). Never prints fabricated metrics when runs are ungated or stale.
3. **Templates & Assets (`templates/hub/**` + `hub_card/{render,svg}.rs` + `hub_sync.rs` + `hub_cards_push.rs`):**
   - **Notable anomaly in `hub_card/svg.rs`:** Despite its filename, `svg.rs` **generates zero SVGs** — it uses `include_bytes!` to return 7 pre-baked 16:9 JPEG banners (`overview.jpg`, `banner-{0.8b,2b,4b,9b,27b,generic}.jpg`) from `crates/vox-populi/assets/hub/`.
   - Meanwhile, `hub_sync.rs:1877,1906` explicitly **deletes** `assets/scorecard.svg` and `assets/overview.svg`, and `hub_cards_push.rs`'s `is_card_collateral` allowlist only admits `README.md`, `MODEL_CARD.md`, `assets/*.{jpg,jpeg,png,webp}`, `.eval_results/*.yaml`, and `external_serving_handoff_v1.json` (explicitly excluding `.svg`).
   - `templates/hub/partials/onboarding.md:49` still references the deleted symbol `resolve_chat_provider_route`.
   - `hub_sync.rs` also maintains `<namespace>/vox-mens-index` (`README.md`, `ladder_index.json`) and the static WebGPU Playground Space `<namespace>/vox-mens-playground` (`templates/hub/space/{README.md.tmpl, index.html.tmpl}`) which fetches `ladder_index.json`, `gate_receipt.json`, and `collateral_damage_report.json` client-side.

If v9.2 reintroduces live data-driven charts (e.g. training loss curves from `telemetry_loss`, pass-rate ladder bars, or VRAM scaling charts) on HF cards, **three coordinated seams must move together**: (a) render sanitized SVG (or PNG/rasterized chart) from `RunRecords`/`CardFacts` in `hub_card/svg.rs` (keeping or supplementing the 16:9 JPEG hero banners), (b) stop deleting those SVG paths in `hub_sync.rs`, and (c) include `assets/*.svg` in `hub_cards_push.rs::is_card_collateral` (while remembering HF Markdown strips `<script>` and external SVG script tags, so charts must be pure static `<svg>` primitives with `viewBox` and inline `<style>`/attributes).

---

## Data Flow

### Flow A: Model Discovery -> Selection -> Dispatch (After Target Alignment)

```
[Startup / Refresh]
  vox-gui / vox-orchestrator-mcp / vox-cli
    ├── calls register_nvml_vram_probe() [L5/L3 -> L4 nvml-probe]
    └── spawns run_catalog_refresh_loop()
          ├── refresh_free_vram_hint() [NVML + Metal/sys fallback]
          ├── OllamaCatalog::sync (/api/tags -> param_count_b)
          ├── MensCatalog::scan (mens/runs/* -> rung param_count_b + scorecard)
          ├── PopuliMeshCatalog::discover (iroh Probe -> task_kinds + engines)
          └── AutoModelSelection::select_for_free_memory (qwen3_5_code ladder)

[Request Routing]
  Select request (Chat / MCP / Orchestrator / Axis UI)
    └── vox-orchestrator::models::{select, scoring, vram, key_guard}
          ├── filters Provider/Scope/Secret & scores intelligence/cost/latency/VRAM_fit
          └── emits ModelSelectionDecision { spec, score_breakdown, vram_fit }
                │
                ├── Cloud / Ollama / VoxLocal HTTP ──> vox_config::resolve_egress -> vox_llm_egress
                └── PopuliMesh (mesh/<peer>/text_infer)
                      └── vox-mesh-transport (L1) iroh ALPN b"vox/job/1"
                            ──> Peer Composite JobExecutor (TaskKind::TextInfer)
                            ──> Local Candle plugin / mens serve / Ollama -> JobResponse::Output
```

### Flow B: Hub Card Rendering & Card-Only Push

```
mens/runs/<family>-<size>/ (gate_receipt, collateral_damage, heldout, telemetry_loss)
  + mens/config/hub-cards.yaml (editorial why + spokes)
  + crates/vox-populi/assets/hub/*.jpg (hero banners)
         │
         ▼
  hub_card::{facts::RunRecords::load, card_facts, index_facts, render, svg, lints}
         │
         ▼
  hub_sync::write_local_cards() ──> mens/runs/hub_cards/{vox-mens-qwen*, vox-mens-index, vox-mens-playground}
         │
         ├── Full release push (hub_sync) ──> uploads weights + cards + index + Space + tags
         └── Card-only sync (hub_cards_push::sync_cards_only) ──> updates cards/assets/manifest WITHOUT weight bump
```

---

## Scaling Considerations

| Scaling Dimension | Constraint / Bottleneck | Architectural Mitigation |
|---|---|---|
| **VRAM / Unified Memory Rungs** | `0.8B -> 2B -> 4B -> 9B -> 27B` QLoRA & serving jump steeply; wrong rung causes OOM or 10x CPU offload | Enforce `VramFit` margin (`COMFORTABLE_HEADROOM = 1.25`) in `scoring.rs`/`ranking.rs`, default `auto_select` to `qwen3_5_code`, populate `param_count_b` on all local/mens/mesh specs |
| **Apple Silicon & Non-CUDA Hosts** | `vram.rs` only queries `VRAM_PROBE` (`nvml-probe`), leaving Metal/AMD hosts blind (`VramFit::Unknown`) during routing | Share Metal/system memory fallback (already in `size_select::read_free_memory()`) into `vram::FREE_VRAM_MB_HINT` with explicit memory provenance |
| **Mesh Inference Latency & Payload Bounds** | iroh `run_on_peer` currently uses single-shot `16 MiB` `JobResponse::Output` without token streaming; `InterpExecutor` spawns a fresh OS process per job | Dedicated `TextInfer` executor must keep model/HTTP connection pooled in-process (never per-request subprocess); add streaming framing (`PROTO = 3`) if interactive chat token streaming over mesh is required |
| **Crate LoC & Dependency Fan-in** | `vox-orchestrator` is ~85.5K LoC (budget `70_000`) and `vox-secrets` has `max_dependents = 35` | Put shared mesh inference wire types in `vox-mesh-types` (L0) and iroh dispatch helpers in `vox-mesh-transport` (L1); keep UI/DTO work in `vox-orchestrator-mcp` & `vox-gui` |
| **HF Card Collateral Churn** | `sync_cards_only` avoids re-uploading multi-GB weights; `hub-cards.yaml` + `lints.rs` block stale numbers/unmeasured claims | Keep all dynamic SVG/chart generation derived strictly from `RunRecords` gated by `GateState::Passed` and `AdapterEvidence::Current` |

---

## Anti-Patterns to Avoid

1. **Treating `PopuliMesh` as `:11434` Ollama HTTP:** Do NOT extend `VOX_LOCAL_ENDPOINT` / `http://127.0.0.1:11434` probing (`local_health.rs`, `try_agent_turn.rs`, `gui_ops/models.rs`, `resolve_egress.rs`) as a fix for `PopuliMesh` — P2P mesh identity is an `iroh::EndpointId` (`mesh/<peer>/<kind>`), not localhost HTTP.
2. **Routing `PopuliMesh` through `StreamRoute::Cascade` or `LlmConfig::openrouter`:** `runtime.rs:46-48` and `selection_queries.rs:724` silently drop mesh peer targeting; fix those branches cleanly rather than layering another fallback wrapper on top.
3. **Hardcoding model sizes/names in routing or UI:** Never duplicate `"Qwen3-8B"` (`ChatExecutionRail.tsx:873-880`), `param_count_b: Some(27.0)` (`mens_spoke_routing.rs:145`), `qwen3_27b_metal_check/*`, or static 3-item `HuggingFaceCatalog` lists — bind UI labels and capabilities dynamically to `ModelSpec` / `SizeSyncRow` / `gpu-specs.yaml`.
4. **Adding L3->L4 compile dependencies or `vox-actor-runtime -> vox-orchestrator` edges:** Violates `layers.toml` and fails `vox-arch-check`. Use L0/L1 abstractions or host-registered callbacks/plugins via `vox-plugin-host`.
5. **Reintroducing legacy `http_client.rs` `/v1/populi/*` HTTP dispatch as primary mesh path:** `crates/vox-populi/src/transport/handlers/dispatch.rs` and `vox-plugin-populi-mesh` mark HTTP job dispatch deprecated in favor of `vox_mesh_transport`; new `TextInfer` dispatch belongs on `vox/job/1` over `iroh`.
6. **Publishing un-gated numbers or raw SVGs outside `is_card_collateral`:** Any card chart or prose metric must pass `hub_card/lints.rs` + `GateState::Passed` and be explicitly whitelisted in `hub_cards_push.rs::is_card_collateral`.

---

## Integration Points

| Seam | Upstream Producer | Downstream Consumer | Required Alignment in v9.2 |
|---|---|---|---|
| **Host VRAM -> Scorer** | `vram_probe.rs` (NVML) + `size_select::read_free_memory` (Metal/CUDA/RAM) | `vram.rs` -> `scoring.rs` / `ranking.rs` / `auto_select.rs` | Unify Metal+NVML free-memory feeding into `vram.rs`; switch `auto_select` to `qwen3_5_code`; increase `VramFit` influence |
| **Mesh `Probe` -> `ModelRegistry`** | Peer `JobExecutor` (`vox/job/1` `Probe`) -> `mesh_directory::trusted_peers` | `PopuliMeshCatalog` -> `ModelRegistry` | Advertise `TaskKind::TextInfer` + hosted model IDs/`param_count_b` in `JobResponse::Probed.engines` |
| **Registry/Policy -> Mesh Wire** | `selection_queries.rs`, `runtime.rs`, `llm_bridge/infer.rs`, `turn_events.rs` | `vox-mesh-transport` (`JobRequest::Run` `TaskKind::TextInfer`) | Replace `Cascade`/`OpenRouter`/`HttpInferError` stubs with real iroh `EndpointId` `TextInfer` dispatch + reputation feedback |
| **MCP GUI Ops -> Axis UI** | `gui_ops/{models.rs, dto/models.rs, dto/routing_explain.rs}` + `models_host.rs` | `ModelsView.tsx`, `MensServePanel.tsx`, `BackendAvailability.tsx`, `ChatExecutionRail.tsx` | Expose VRAM headroom, recommended ladder rung, distinct `PopuliMesh` peer count, and dynamic active model label |
| **Run Evidence -> HF Hub** | `mens/runs/*`, `mens/config/{gpu-specs,hub-cards}.yaml` | `hub_card/*` -> `hub_sync.rs` / `hub_cards_push.rs` -> HF Hub & Space | Fix `onboarding.md` stale symbol; coordinate `svg.rs` / `hub_sync.rs` / `hub_cards_push.rs` if data-driven SVG charts are added |

---

## Sources

- Layer & crate boundary SSOT: [`docs/src/architecture/layers.toml`](docs/src/architecture/layers.toml)
- Model selection, VRAM, scoring, auto-scaling, key guard, mesh directory, catalogs, routing:
  - [`crates/vox-orchestrator/src/models/vram.rs`](crates/vox-orchestrator/src/models/vram.rs)
  - [`crates/vox-orchestrator/src/models/scoring.rs`](crates/vox-orchestrator/src/models/scoring.rs)
  - [`crates/vox-orchestrator/src/models/select.rs`](crates/vox-orchestrator/src/models/select.rs)
  - [`crates/vox-orchestrator/src/models/auto_select.rs`](crates/vox-orchestrator/src/models/auto_select.rs)
  - [`crates/vox-orchestrator/src/models/key_guard.rs`](crates/vox-orchestrator/src/models/key_guard.rs)
  - [`crates/vox-orchestrator/src/models/mesh_directory.rs`](crates/vox-orchestrator/src/models/mesh_directory.rs)
  - [`crates/vox-orchestrator/src/models/spec.rs`](crates/vox-orchestrator/src/models/spec.rs)
  - [`crates/vox-orchestrator/src/models/ranking.rs`](crates/vox-orchestrator/src/models/ranking.rs)
  - [`crates/vox-orchestrator/src/models/registry/catalog_seed.rs`](crates/vox-orchestrator/src/models/registry/catalog_seed.rs)
  - [`crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs`](crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs)
  - [`crates/vox-orchestrator/src/models/registry/selection_queries.rs`](crates/vox-orchestrator/src/models/registry/selection_queries.rs)
  - [`crates/vox-orchestrator/src/catalog.rs`](crates/vox-orchestrator/src/catalog.rs)
  - [`crates/vox-orchestrator/src/orchestrator/catalog_refresh.rs`](crates/vox-orchestrator/src/orchestrator/catalog_refresh.rs)
  - [`crates/vox-orchestrator/src/runtime.rs`](crates/vox-orchestrator/src/runtime.rs)
- Mesh transport, actor runtime, egress, and MCP LLM bridge:
  - [`crates/vox-mesh-types/src/task.rs`](crates/vox-mesh-types/src/task.rs)
  - [`crates/vox-mesh-transport/src/endpoint.rs`](crates/vox-mesh-transport/src/endpoint.rs)
  - [`crates/vox-mesh-transport/src/protocol.rs`](crates/vox-mesh-transport/src/protocol.rs)
  - [`crates/vox-mesh-transport/src/interp_executor.rs`](crates/vox-mesh-transport/src/interp_executor.rs)
  - [`crates/vox-ml-cli/src/commands/mesh_cli.rs`](crates/vox-ml-cli/src/commands/mesh_cli.rs)
  - [`crates/vox-workflow-runtime/src/workflow/populi.rs`](crates/vox-workflow-runtime/src/workflow/populi.rs)
  - [`crates/vox-actor-runtime/src/model_resolution.rs`](crates/vox-actor-runtime/src/model_resolution.rs)
  - [`crates/vox-config/src/resolve_egress.rs`](crates/vox-config/src/resolve_egress.rs)
  - [`crates/vox-orchestrator-mcp/src/server_state/vram_probe.rs`](crates/vox-orchestrator-mcp/src/server_state/vram_probe.rs)
  - [`crates/vox-orchestrator-mcp/src/llm_bridge/infer.rs`](crates/vox-orchestrator-mcp/src/llm_bridge/infer.rs)
  - [`crates/vox-orchestrator-mcp/src/llm_bridge/provider_endpoints.rs`](crates/vox-orchestrator-mcp/src/llm_bridge/provider_endpoints.rs)
  - [`crates/vox-orchestrator-mcp/src/llm_bridge/provider_auth.rs`](crates/vox-orchestrator-mcp/src/llm_bridge/provider_auth.rs)
  - [`crates/vox-orchestrator-mcp/src/llm_bridge/local_health.rs`](crates/vox-orchestrator-mcp/src/llm_bridge/local_health.rs)
  - [`crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs`](crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs)
  - [`crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/policy.rs`](crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/policy.rs)
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/turn_events.rs`](crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/turn_events.rs)
- GUI Ops & Axis UI:
  - [`crates/vox-orchestrator-mcp/src/gui_ops/models.rs`](crates/vox-orchestrator-mcp/src/gui_ops/models.rs)
  - [`crates/vox-orchestrator-mcp/src/gui_ops/dto/models.rs`](crates/vox-orchestrator-mcp/src/gui_ops/dto/models.rs)
  - [`crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs`](crates/vox-orchestrator-mcp/src/gui_ops/dto/routing_explain.rs)
  - [`crates/vox-gui/src/commands/models_host.rs`](crates/vox-gui/src/commands/models_host.rs)
  - [`crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx`](crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx)
  - [`crates/vox-gui/ui/src/components/surfaces/Models/MensServePanel.tsx`](crates/vox-gui/ui/src/components/surfaces/Models/MensServePanel.tsx)
  - [`crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx`](crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx)
- HF Hub Cards, Templates, Sync & Config:
  - [`crates/vox-populi/src/mens/hub_card/svg.rs`](crates/vox-populi/src/mens/hub_card/svg.rs)
  - [`crates/vox-populi/src/mens/hub_card/card_facts.rs`](crates/vox-populi/src/mens/hub_card/card_facts.rs)
  - [`crates/vox-populi/src/mens/hub_card/index_facts.rs`](crates/vox-populi/src/mens/hub_card/index_facts.rs)
  - [`crates/vox-populi/src/mens/hub_card/render.rs`](crates/vox-populi/src/mens/hub_card/render.rs)
  - [`crates/vox-populi/src/mens/hub_card/config.rs`](crates/vox-populi/src/mens/hub_card/config.rs)
  - [`crates/vox-populi/src/mens/hub_card/lints.rs`](crates/vox-populi/src/mens/hub_card/lints.rs)
  - [`crates/vox-populi/src/mens/hub_repo.rs`](crates/vox-populi/src/mens/hub_repo.rs)
  - [`crates/vox-populi/src/mens/hub_sync.rs`](crates/vox-populi/src/mens/hub_sync.rs)
  - [`crates/vox-populi/src/mens/hub_cards_push.rs`](crates/vox-populi/src/mens/hub_cards_push.rs)
  - [`crates/vox-populi/src/mens/tensor/size_select.rs`](crates/vox-populi/src/mens/tensor/size_select.rs)
  - [`mens/config/gpu-specs.yaml`](mens/config/gpu-specs.yaml)
  - [`mens/config/hub-cards.yaml`](mens/config/hub-cards.yaml)
