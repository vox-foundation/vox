# Architecture Research

**Domain:** Unified Light ↔ Deep Research Continuum, Extreme-Data Epistemic Streaming, Calibrated Socrates/Novelty Verification & Continuous Direct Scientific Publication Flywheel (Milestone v7.6)
**Researched:** 2026-11-14
**Confidence:** HIGH (grounded in live Vox L0–L5 source inspection across `vox-research-shim`, `vox-search`, `vox-scientia`, `vox-publisher`, `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-cli-research`, `vox-gui`, and `docs/src/architecture/layers.toml`, paired with 2025–2026 agentic RAG, NLI verification, and machine-actionable scholarly publishing prior art)

## Standard Architecture

### System Overview

Milestone v7.6 integrates all four pillars—(1) the **Unified Light ↔ Deep Research Continuum**, (2) **Extreme-Data Resumable Streaming & Cursor Pagination**, (3) **Calibrated Socrates + Hybrid Semantic/Lexical Novelty Verification**, and (4) the **Continuous Direct Publication Flywheel**—inside Vox's existing L0–L5 crate DAG with **zero new workspace crate-edge inversions**, **zero `[[forbidden_deps]]` violations**, and **zero `VoxDb` handles opened in `vox-gui`**.

```
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                         L5 SURFACES  (No reverse dependents)                             │
│                                                                                          │
│  ┌──────────────────────────────┐  ┌─────────────────────────┐  ┌─────────────────────┐  │
│  │ vox-gui (Tauri + React UI)   │  │ vox-orchestrator-d      │  │ vox-cli /           │  │
│  │ - Loquela + ResearchDropdown │  │ (Single local VoxDb     │  │ vox-cli-research    │  │
│  │ - ChatSurface + TracePanel   │  │  owner, IPC + WS/SSE    │  │ (Headless CLI +     │  │
│  │ - ResearchView + DagCanvas   │  │  gateway host)          │  │  audit runners)     │  │
│  │ - ScientiaSurface + Studio   │  └────────────┬────────────┘  └──────────┬──────────┘  │
│  │ - PublicationsView           │               │                          │             │
│  │ ZERO VoxDb / ZERO CLI-DB!    │               │                          │             │
│  └──────────────┬───────────────┘               │                          │             │
│                 │ BackendState (IPC / /api/v2 + vox://agent-events + /v1/ws)             │
└─────────────────┼───────────────────────────────┼──────────────────────────┼─────────────┘
                  ▼                               ▼                          ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│                         L3 HEAVY RUNTIMES  (Acyclic within-L3 DAG)                       │
│                                                                                          │
│  ┌────────────────────────────────────────────────────────────────────────────────────┐  │
│  │ vox-orchestrator-mcp (MCP + /api/v2/* HTTP/SSE/WS Gateway + Chat Continuum Spine)  │  │
│  │ - chat_tools/chat/{message.rs, research_intent.rs, research_turn.rs, agent_loop}   │  │
│  │ - gui_ops/{research.rs, scientia.rs}   - http_gateway/scientia_feed.rs             │  │
│  └──────┬─────────────────────┬──────────────────────┬────────────────────┬───────────┘  │
│         │                     │                      │                    │              │
│         ▼                     ▼                      ▼                    ▼              │
│  ┌──────────────────┐  ┌────────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │ vox-research-shim│  │ vox-orchestrator   │  │ vox-publisher   │  │ vox-scientia    │  │
│  │ (Multi-wave      │  │ - socrates.rs      │  │ - scientia_     │  │ - claim_        │  │
│  │  pipeline, gate, │  │ - research_gate.rs │  │   prior_art.rs  │─▶│   extractor/    │  │
│  │  verifier,       │  │ - event_bus        │  │ - scientia_     │  │ - inspect_      │  │
│  │  discovery_bridge│  └────────────────────┘  │   discovery.rs  │  │   bridge/novelty│  │
│  └────────┬─────────┘                          │ - archive_run   │  │ - review_flow   │  │
│           │                                    └─────────────────┘  │ - nanopub/latex │  │
│           ▼                                                         └────────┬────────┘  │
│  ┌───────────────────────────────────────────────────────────────────────────┴────────┐  │
│  │ vox-search (Hybrid Tantivy + Dense Embeddings + CRAG Quality + WebSearchDispatcher)│  │
│  └────────────────────────────┬──────────────────────────────────────┬────────────────┘  │
│                               ▼                                      ▼                   │
│  ┌───────────────────────────────────────┐    ┌───────────────────────────────────────┐  │
│  │ vox-db (Codex/Turso Single-Writer DB) │    │ vox-actor-runtime::llm (LLM Facade)   │  │
│  └───────────────────────────────────────┘    └───────────────────────────────────────┘  │
└───────────────────────────────────────┬──────────────────────────────────────────────────┘
                                        ▼
┌──────────────────────────────────────────────────────────────────────────────────────────┐
│            L2 DOMAIN LIBS  │  vox-similarity, vox-config, vox-eval, vox-mcp-registry     │
├────────────────────────────┼─────────────────────────────────────────────────────────────┤
│            L1 PRIMITIVES   │  vox-research-events, vox-secrets (Clavis), vox-crypto      │
├────────────────────────────┼─────────────────────────────────────────────────────────────┤
│            L0 PURE TYPES   │  vox-orchestrator-types (socrates_policy), vox-db-types     │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Layer | Crate / Module | Responsibility in Milestone v7.6 | Existing Seams Inspected |
|-------|----------------|----------------------------------|--------------------------|
| **L0** | `vox-orchestrator-types` | Pure `SocratesPolicy`, `ConfidenceDecision` (`Answer` / `Ask` / `Abstain`), and new `ResearchContinuumTier` (`Direct`, `LocalRag`, `QuickWeb`, `DeepMultiWave`) escalation thresholds (zero async/DB). | `crates/vox-orchestrator/src/socrates.rs:14-18` (re-exporting `vox_orchestrator_types::socrates_policy`) |
| **L0** | `vox-db-types` | Row/DTO types for cursor-paginated claims, sources, wave snapshots, and novelty overlap summaries. | `crates/vox-scientia/Cargo.toml:26` |
| **L1** | `vox-research-events` | Pure typed `ResearchEvent` signal ladder (`T0..T4` + v7.6 `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta`), `NoveltyEvidenceBundle`, `FindingCandidateV1`, `PreregistrationV1`. | `crates/vox-research-events/src/events.rs:18-160` |
| **L2** | `vox-similarity` | Pure deterministic lexical shingle/Jaccard/SimHash/MinHash similarity used for offline novelty fallback and near-duplicate wave pruning. | `crates/vox-orchestrator-mcp/Cargo.toml:83` |
| **L3** | `vox-actor-runtime::llm` | Mandatory model-agnostic facade (`infer_with_retry`, `llm_chat`, `llm_stream`, `llm_embed`) for all synthesis, judge, MiniCheck LLM fallback, and prior-art embeddings. | `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:27` |
| **L3** | `vox-search` | Hybrid lexical+dense retrieval (`SearchExecution`), CRAG evidence grading (`web_research_crag_quality`), `WebSearchDispatcher`, and `embedding_env::embedding_config_from_env`. | `crates/vox-search/src/research.rs:17-155`, `crates/vox-search/src/novelty.rs:34-90` |
| **L3** | `vox-scientia` | Atomic claim decomposition + calibrated MiniCheck (`claim_extractor/`), hybrid semantic+lexical `AtomicNoveltyScorer` (`inspect_bridge/novelty.rs`), shared `CachedLlmEmbedder`, `review_flow` SSOT, IMRaD/LaTeX/arXiv/FindingPage/Nanopub builders, and Socrates telemetry aggregation. | `crates/vox-scientia/src/review_flow.rs:1-395`, `crates/vox-scientia/src/inspect_bridge/novelty.rs:75-120` |
| **L3** | `vox-publisher` | Federated prior-art retrieval (`OpenAlex`, `Crossref`, `Semantic Scholar` in `scientia_prior_art.rs`), discovery ranking (`scientia_discovery.rs`), worthiness rubrics (`scientia_evidence/`), and archival/scholarly submission (`archive_run` for Zenodo, Software Heritage, OpenReview). | `crates/vox-publisher/src/scientia_prior_art.rs:1-300`, `crates/vox-publisher/src/scientia_discovery.rs:1-300` |
| **L3** | `vox-orchestrator` | `socrates.rs` risk-budgeted gate, `research_gate.rs` pre-registration enforcement, and durable `event_bus` (`AgentEventKind` with replay offsets). | `crates/vox-orchestrator/src/socrates.rs:1-350` |
| **L3** | `vox-research-shim` | Multi-wave research pipeline (`orchestrator/pipeline.rs`, `wave.rs`), `ResearchGate` (`gate.rs`), calibrated `ClaimVerifier` (`verifier.rs`), domain sandboxes (`domain/{codegen,shopping,polyglot_sandbox,git_probe,micro_benchmark}`), and `discovery_bridge.rs`. | `crates/vox-research-shim/src/research/orchestrator/pipeline.rs:1-1250`, `verifier.rs:1-650` |
| **L3** | `vox-orchestrator-mcp` | Unified composition hub: chat continuum (`message.rs`, `research_intent.rs`, `research_turn.rs`), `/api/v2/research/*` and `/api/v2/scientia/*` REST + SSE/WS delta streams (`gui_ops/research.rs`, `gui_ops/scientia.rs`, `http_gateway/scientia_feed.rs`), and `BroadcastEmitter -> event_bus` bridge. | `crates/vox-orchestrator-mcp/src/gui_ops/research.rs:1-1291`, `scientia.rs:1-1045` |
| **L5** | `vox-cli-research` / `vox-cli` | Headless CLI wrappers (`vox scientia *`, `vox db publication-*`) plus Phase 447/452 deterministic Before/After audit runner. | `crates/vox-cli-research/src/scientia_phase_handlers.rs:1-905` |
| **L5** | `vox-gui` | Zero-`VoxDb` Tauri shell (`BackendState` local daemon IPC or remote `GatewayClient`) + React Axis surfaces (`Loquela`, `ChatSurface`, `ResearchView`, `MultiWaveProgressTimeline`, `ResearchDagCanvas`, `ScientiaSurface`, `DiscoveryReview`, `PublicationsView`). | `crates/vox-gui/src/commands/{research.rs,scientia.rs,orchestrator.rs}`, `crates/vox-gui/ui/src/components/surfaces/*` |

---

## Recommended Project Structure

All v7.6 functionality lands inside existing workspace crates—**no new Cargo crates and zero new inter-crate `Cargo.toml` edges are required**:

```
crates/
├── vox-orchestrator-types/src/            # [L0] Pure policy & continuum enums
│   └── socrates_policy.rs                 # MOD: add ResearchContinuumTier + EscalationReason + CRAG thresholds
├── vox-research-events/src/               # [L1] Pure cross-crate wire events
│   ├── events.rs                          # MOD: add seq_no + WaveStarted/WaveCompleted/ContinuumEscalated/EpistemicDagDelta
│   └── schema_types.rs                    # EXISTING: NoveltyEvidenceBundle, FindingCandidateV1
├── vox-search/src/                        # [L3] Retrieval & CRAG grading
│   ├── research.rs                        # MOD: expose CragEscalationSignal (coverage, contradiction_density, quality)
│   └── novelty.rs                         # EXISTING: CorpusNoveltyIndex (in-session MMR/cosine novelty)
├── vox-scientia/src/                      # [L3] Claim verification, novelty, manuscript, nanopub, review SSOT
│   ├── embedder.rs                        # NEW (lifted from vox-cli-research): CachedLlmEmbedder over VoxDb + llm_embed
│   ├── inspect_bridge/novelty.rs          # MOD: hybrid semantic + calibrated lexical fallback (fix None => Novel bug!)
│   ├── producers/socrates_telemetry.rs    # MOD: join research_metrics + scientia_claims + novelty verdicts into snapshot
│   └── review_flow.rs                     # MOD: hydrate novelty verdict + prior_art_uris in nanopub_build; add flywheel_advance
├── vox-publisher/src/                     # [L3] Federated prior art & external scholarly adapters
│   ├── scientia_semantic.rs               # MOD: re-export Embedder trait compatibly with vox-scientia::embedder
│   ├── scientia_prior_art.rs              # MOD: populate calibrated lexical_score + optional dense semantic_score per hit
│   └── archive_run.rs                     # MOD: structured FlywheelStageReport for GUI/daemon consumption
├── vox-research-shim/src/research/        # [L3] Multi-wave Deep Research engine
│   ├── orchestrator/pipeline.rs           # MOD: emit wave/subquery/DAG delta events; accept warm Quick-Research seed
│   ├── orchestrator/wave.rs               # MOD: bounded per-wave source/claim caps + streaming checkpoint persistence
│   ├── verifier.rs                        # MOD: unify ClaimVerdict confidence calibration with Socrates thresholds
│   └── gate.rs                            # MOD: expose continuum escalation predicate (Local -> Quick -> Deep)
├── vox-orchestrator-mcp/src/              # [L3] Composition spine, Chat router & /api/v2 HTTP/SSE/WS gateway
│   ├── chat_tools/params.rs               # MOD: add explicit research_mode ("auto"|"fast"|"deep"|"off") distinct from research_scope
│   ├── chat_tools/chat/message.rs         # MOD: run Socrates retrieval BEFORE continuum routing; wire escalation spine
│   ├── chat_tools/chat/research_intent.rs # MOD: honor explicit research_mode + SocratesGateOutcome + CRAG escalation
│   ├── chat_tools/chat/research_turn.rs   # MOD: forward scope/domain/site/waves into run_deep; stream live TurnEvents; Quick->Deep handoff
│   ├── gui_ops/research.rs                # MOD: cursor-paginated /sessions/{id}/claims & /sources, SSE /sessions/{id}/stream
│   ├── gui_ops/scientia.rs                # MOD: add /publications/{id}/claims, /venue-recommend, /novelty-fetch, /flywheel-advance
│   └── http_gateway/scientia_feed.rs      # MOD: replace VoxDb::connect_default() with ServerState.db + subscribe to state.research_events
├── vox-cli-research/src/                  # [L5] Headless CLI & Before/After Audit Harness
│   ├── db/publication/embedder.rs         # MOD: thin re-export of vox_scientia::embedder::CachedLlmEmbedder
│   └── scientia_v76_audit.rs              # NEW: Phase 447 Before-audit & Phase 452 After-audit deterministic harness
└── vox-gui/                               # [L5] Zero-VoxDb Desktop Shell & Axis Surfaces
    ├── src/commands/{research.rs,scientia.rs} # MOD: BackendState wrappers for paginated claims, novelty-fetch, flywheel, claims/venue
    └── ui/src/
        ├── App.tsx & lib/buildChatTurn.ts # MOD: send research_mode cleanly instead of overloading research_scope
        └── components/surfaces/
            ├── Chat/{ResearchTracePanel.tsx, ChatExecutionRail.tsx} # MOD: live streaming wave/escalation progress
            ├── Research/{ResearchView.tsx, MultiWaveProgressTimeline.tsx, ResearchDagCanvas.tsx} # MOD: live wave telemetry + virtualized DAG
            ├── Scientia/{DiscoveryReview.tsx, NoveltyEvidencePanel.tsx} # MOD: one-click prior-art novelty fetch + flywheelstepper
            └── Publications/PublicationsView.tsx # MOD: replace execute_command CLI-DB spawns with BackendState API calls
```

### Structure Rationale

1. **Strict `layers.toml` & `crate-edges` Compliance (Zero Inversions):**
   - `vox-orchestrator` (`max_dependents = 30`, `layers.toml:300`) **never** depends on `vox-research-shim`, `vox-scientia`, or `vox-publisher`. Pure continuum enums live in `vox-orchestrator-types` (L0) and typed pipeline events live in `vox-research-events` (L1).
   - `vox-publisher` (L3) **already** depends on `vox-scientia` (`crates/vox-publisher/Cargo.toml:55`), and `vox-scientia` (L3) **already** depends on `vox-db`, `vox-actor-runtime`, and `vox-search` (`crates/vox-scientia/Cargo.toml:25-33`). Lifting `CachedLlmEmbedder` from L5 `vox-cli-research/src/db/publication/embedder.rs` down into L3 `vox-scientia::embedder` lets both `vox-orchestrator-mcp` (daemon/GUI) and `vox-cli-research` (CLI) run `VoxDb`-cached dense prior-art embeddings via `vox_actor_runtime::llm::llm_embed` with **zero new `Cargo.toml` dependencies**.
   - None of the `[[forbidden_deps]]` pairs in `docs/src/architecture/layers.toml:500-535` (`vox-db -> vox-repository`, `vox-orchestrator-driver -> vox-repository`, etc.) are touched.
2. **Strict Zero-`VoxDb`-in-`vox-gui` Enforcement:**
   - Per `crates/vox-gui/src/commands/gui_db_pool.rs:1-28`, opening `VoxDb` inside `vox-gui` alongside `vox-orchestrator-d` triggers Turso/SQLite file-lock contention (`SQLITE_BUSY` / Windows `os error 33` / `"concurrent use forbidden"`).
   - Inspection of `PublicationsView.tsx:237-254` uncovered a remaining hidden violation: `PublicationDetail` invokes `execute_command` (`vox scientia claims` and `vox scientia publication-venue-recommend`), spawning child CLI processes that call `VoxDb::connect_default()` (`scientia_phase_handlers.rs:505`) concurrently with the daemon—and failing outright when `vox-gui` is connected to a remote `vox serve` gateway. Exposing `/api/v2/scientia/publications/{id}/claims` and `/api/v2/scientia/venue-recommend` on `vox-orchestrator-mcp` and routing `vox-gui` through `BackendState` eliminates this lock hazard completely.

---

## Architectural Patterns

### Pattern 1: Four-Stage Adaptive Research Continuum (Warm-Seeded Escalation)

**What:** Replace the disconnected upfront binary fork (`ResearchMode::None | Quick | Deep` in `message.rs:142`) with a monotonic, evidence-gated 4-stage continuum (`T0 Direct` → `T1 Local Hybrid RAG` → `T2 Quick 1-Wave Web` → `T3 Deep Multi-Wave Adversarial`) where each stage reuses retrieved context from earlier stages rather than restarting cold.

**Why (Live Code Grounding + 2026 Prior Art):**
- Today in `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:142`, `classify_research_intent(&params.prompt, params.force_research)` runs **before** `run_retrieval_bundle` (`message.rs:204`), so `SocratesGateOutcome` (`preamble_and_retrieval.rs:297`) is never passed to `classify_research_intent_with_socrates` (`research_intent.rs:191`, currently dead on the live turn path).
- Worse, `App.tsx:1972-1973` writes `'deep' | 'quick' | 'auto'` into `research_scope` (which `params.rs:139` defines as `"local" | "web" | "both"`), while `research_turn.rs:620-630` hardcodes `scope: ResearchScope::Web, waves: 1, domain_mode: DomainMode::General`.
- Modern 2025–2026 Adaptive RAG + Corrective RAG (CRAG) architectures evaluate retrieval sufficiency at each tier and escalate only when uncertainty or contradiction exceeds calibrated thresholds, carrying forward verified hits as warm seeds so latency and token spend stay minimal on easy turns.

**Trade-offs:** Adds ~5–15 ms of deterministic CRAG/Socrates scoring between tiers on `auto` turns, while saving 5×–15× token spend vs unconditional Deep Research and eliminating ungrounded hallucinations on deceptive "simple-looking" queries.

```rust
// In vox-orchestrator-types::socrates_policy (L0 pure)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchContinuumMode {
    Off,   // Strictly parametric / user-supplied files; zero external web calls
    Auto,  // Socrates + CRAG adaptive escalation (T0 -> T1 -> T2 -> T3)
    Fast,  // Pin to T2 Quick 1-wave retrieval (no T3 multi-wave expansion unless user clicks escalate)
    Deep,  // Pin directly to T3 Multi-Wave Deep Research with warm T1 local RAG seed
}
```

### Pattern 2: Watermarked Event-Driven Delta Streaming & Cursor Pagination

**What:** Bridge `ServerState::research_events` (`tokio::sync::broadcast::Sender<ResearchEvent>`) directly into both (a) `state.orchestrator.event_bus()` (consumed by `vox-gui`'s existing `spawn_agent_event_stream` → `"vox://agent-events"` with `op_id` reconnect replay) and (b) `http_gateway::scientia_feed` (`TopicMessage` topic `"research.session.delta"` over `/v1/ws` + `GET /api/v2/research/sessions/{id}/stream` SSE), paired with cursor-paginated detail endpoints (`?cursor=&limit=100&include_raw_artifact=false`).

**Why:**
- Today `vox-gui/src/commands/scientia.rs:178-228` polls `/scientia/manifests?limit=500` and `/research/sessions?limit=200` on a timer, `get_research_session_detail` (`gui_ops/research.rs:1167-1186`) serializes the entire `artifact_json` + full `EpistemicDagDto` in one response, `research_turn.rs:605` buffers deep-research progress in a `Mutex<Vec<_>>` until completion, and `MultiWaveProgressTimeline.tsx` receives a hardcoded `currentWave={1}` (`ResearchView.tsx:786`).
- Under extreme-data research runs (3–5 waves, 200+ sources, 500+ extracted atomic claims, polyglot sandbox stdout), monolithic JSON payloads cause multi-megabyte IPC spikes and SVG layout jank. Sequence-numbered delta frames + bounded cursor windows keep wire frames `< 32 KB` and UI state `O(window_size)`.

### Pattern 3: Calibrated Hybrid Semantic + Lexical Novelty & Socrates Convergence

**What:** Combine dense citation/abstract embeddings (`CachedLlmEmbedder` in `vox-scientia::embedder` via `vox_actor_runtime::llm::llm_embed`) with calibrated lexical shingle/BM25 overlap in `AtomicNoveltyScorer::score` (`vox-scientia/src/inspect_bridge/novelty.rs`) so prior-art hits from OpenAlex, Crossref, and Semantic Scholar **never** default to `NoveltyVerdict::Novel` when embeddings are offline or unconfigured.

**Why:**
- Live inspection of `crates/vox-scientia/src/inspect_bridge/novelty.rs:94-120` revealed that `AtomicNoveltyScorer::score` inspects **only** `bundle.overlap_summary.max_semantic_score` and maps `None => NoveltyVerdict::Novel` at line 119—even if `max_lexical_score` is `0.98` (an exact title/abstract match on OpenAlex or Crossref when `semantic_score` is `None` at `scientia_prior_art.rs:202,249,299`)!
- By scoring `effective_overlap = match (max_semantic, max_lexical) { (Some(s), Some(l)) => 0.70 * s + 0.30 * l, (Some(s), None) => s, (None, Some(l)) => calibrated_lexical_projection(l), (None, None) => return InsufficientEvidence }`, Vox achieves 100% standalone/offline honesty (never fabricating `Novel` on missing evidence, and catching lexical prior-art collisions offline).

### Pattern 4: Digest-Bound Continuous Direct Publication Flywheel

**What:** Connect Deep Research completion → `FindingCandidateV1` emission → Manifest promotion → Atomic claim extraction (`MiniCheck`) → Federated prior-art novelty bundle attachment → Replay measurement → IMRaD / LaTeX / arXiv `.tar.gz` / FindingPage HTML / RO-Crate assembly → Human `ApprovalToken` + `CriticGate` → Signed offline-validated Nanopub + Zenodo/SWHID/OpenReview archival into a state-machine-driven `flywheel_advance` pipeline in `vox-scientia` + `vox-orchestrator-mcp`.

**Why:** Every individual stage already exists across `vox-scientia` and `vox-publisher`, hardened by SHA3-256 `bound_digest` checks (`review_flow.rs:204-211`) and dual-approver `CriticGate` (`scientia_phase_handlers.rs:193-205`), **except** two missing links:
1. `nanopub_build` (`review_flow.rs:262-263`) still passes `"insufficient_evidence"` and `&[]` instead of hydrating the manifest's `scientia_novelty_bundle` verdict and `prior_art_uris`.
2. Operators currently have to click or run 6 separate manual commands (`promote` → `extract-claims` → `novelty-fetch` → `replay` → `manuscript/latex/arxiv` → `claim-review` → `nanopub/archive`) instead of an automated pre-review preparation flywheel that stops cleanly at the mandatory human `ApprovalToken` gate.

---

## Data Flow

### Request Flow (Unified Light ↔ Deep Research Continuum)

```
User / Agent submits turn (Loquela / MCP vox_chat_message / ResearchView)
    │  params: { prompt, research_mode: auto|fast|deep|off, research_scope: local|web|both,
    │            domain_mode: general|shopping|codegen, site_scope, waves }
    ▼
[Stage 0: Explicit Mode & Policy Gate] (vox-orchestrator-mcp::research_intent)
    ├─ research_mode == Off  ──▶ T0 Direct Parametric / Active-File Turn (0 web calls)
    ├─ research_mode == Deep ──▶ Seed T1 Local RAG (if scope != web) ──▶ Jump to Stage 3 (Deep)
    ├─ research_mode == Fast ──▶ Seed T1 Local RAG (if scope != web) ──▶ Jump to Stage 2 (Quick)
    └─ research_mode == Auto ──▶ Proceed to Stage 1
                                     │
                                     ▼
[Stage 1: T1 Local Hybrid RAG + Socrates Gate] (preamble_and_retrieval + vox-orchestrator::socrates)
    │  Runs Tantivy BM25 + Dense Vector + RRF over local workspace/KB (`vox-search`)
    │  Computes SocratesGateOutcome { decision: Answer|Ask|Abstain, confidence, contradiction_ratio }
    ├─ Decision == Answer AND !has_external_freshness_trigger(prompt)
    │      ──▶ Inject local RAG preamble ──▶ Agent Loop / Direct Synthesis ( Fast ~250ms )
    └─ Decision == Ask/Abstain OR contradiction_ratio >= 0.30 OR external/recency intent
           ──▶ Emit TurnEvent::ContinuumEscalated { from: "local_rag", to: "quick_web", reason }
           ──▶ Proceed to Stage 2 (carrying local RAG chunks as warm seed)
                   │
                   ▼
[Stage 2: T2 Quick 1-Wave Web Retrieval + CRAG Grader] (research_turn::run_quick + vox-search::research)
    │  Dispatches 1-hop federated search (Tavily / SearXNG / OpenAlex / arXiv / Wikipedia / DDG)
    │  Computes CragEscalationSignal { crag_quality, distinct_domains, snippet_conflict_score }
    ├─ research_mode == Fast OR (crag_quality >= 0.68 AND snippet_conflict_score < 0.25 AND !has_deep_signal)
    │      ──▶ Grounded Single-Pass Synthesis + Citation Audit ──▶ Return Reply + ResearchTrace
    └─ research_mode == Auto AND (crag_quality < 0.68 OR snippet_conflict_score >= 0.25 OR has_deep_signal)
           ──▶ Emit TurnEvent::ContinuumEscalated { from: "quick_web", to: "deep_multi_wave", reason }
           ──▶ Proceed to Stage 3 (passing Stage 1+2 hits as Wave-1 warm seed — zero duplicate fetch!)
                   │
                   ▼
[Stage 3: T3 Multi-Wave Deep Research Engine] (vox-research-shim::orchestrator::pipeline)
    │  Wave 1 (Reconnaissance, warm-seeded) ──▶ Wave 2 (Contradiction Isolation) ──▶ Wave 3 (Adversarial/Sandbox)
    │  Streams ResearchEvent deltas -> state.research_events -> event_bus & SSE/WS -> GUI Timeline & Rail
    │  Runs Calibrated ClaimVerifier + Domain Sandboxes + Socrates Confidence Convergence
    ▼
Persists Session + Paginated Claims/Sources in VoxDb ──▶ Emits FindingCandidateProposed (if worthiness >= gate)
```

### State Management

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ vox-orchestrator-d (Single Process Owner of Local VoxDb)                    │
│                                                                             │
│  VoxDb (Turso/SQLite)                                                       │
│  ├── research_sessions (id, status, continuum_tier, wave_count, stability)  │
│  ├── research_artifacts (summary metadata + report_markdown; lazy raw JSON) │
│  ├── scientia_claims + claim_verdicts (indexed by session_id, claim_id)     │
│  ├── research_session_sources (NEW/indexed per-session source rows)         │
│  ├── scientia_finding_candidates -> publication_manifests (SHA3-256 digest) │
│  ├── scientia_review_decisions (content-bound human ApprovalToken source)   │
│  └── scientia_nanopubs + scientia_embedding_cache                           │
│                                                                             │
│  In-Memory Broadcast Buses (Bounded Ring Buffers, Backpressure-Safe)        │
│  ├── state.research_events: broadcast::Sender<SequencedResearchEvent> (1024)│
│  ├── state.orchestrator.event_bus(): AgentEvent (Tier-A durable op_id log)  │
│  └── http_gateway::TopicMessage: WS `/v1/ws` + SSE `/sessions/{id}/stream`  │
└───────────────────────────────┬─────────────────────────────────────────────┘
                                │ Zero VoxDb across boundary!
                                ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ vox-gui (React State + Bounded Client Stores)                               │
│  ├── ChatTranscript / ExecutionRail: consumes `vox://agent-events` deltas   │
│  ├── ResearchView: subscribes to session delta stream; fetches claims &     │
│  │   sources via cursor pages (`limit=100`); virtualizes DAG > 80 nodes     │
│  └── ScientiaSurface / PublicationsView: consumes `scientia.queue.changed`  │
│      push notifications + typed `/api/v2/scientia/*` endpoints              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Data Flows

1. **Extreme-Data Resumable Streaming Flow:**
   - `vox-research-shim::orchestrator::pipeline` assigns a monotonic per-session `seq: u64` to every emitted `ResearchEvent` (`WaveStarted`, `SubqueryDispatched`, `ClaimExtracted`, `ClaimVerified`, `EpistemicDagDelta`, `WaveCompleted`).
   - A lightweight bridge task in `vox-orchestrator-mcp::ServerState` forwards `state.research_events` onto both `state.orchestrator.event_bus()` (as `AgentEventKind::ResearchProgress`) and `scientia_feed::TopicMessage` (`topic: "research.session.delta"`).
   - If a UI subscriber lags (`RecvError::Lagged(n)`), the stream emits `{ "kind": "resync_required", "from_seq": last_seq }`, and the client reconciles via `GET /api/v2/research/sessions/{id}?summary_only=true` + cursor page `GET /api/v2/research/sessions/{id}/claims?after_claim_id=...&limit=100` without ever pulling a 10 MB `artifact_json` blob over IPC.
2. **Socrates + Hybrid Prior-Art Novelty Verification Flow:**
   - When a finding candidate is promoted or `POST /api/v2/scientia/publications/{id}/novelty-fetch` runs inside `vox-orchestrator-mcp`, it invokes `vox_publisher::scientia_prior_art::fetch_prior_art_federated` with `vox_scientia::embedder::CachedLlmEmbedder::from_env(&db)`.
   - Online mode queries OpenAlex, Crossref, and Semantic Scholar concurrently via `vox-http-client` (using Clavis `SecretId::VoxOpenAlexEmail` and `SecretId::VoxSemanticScholarApiKey` when present, keyless polite-pool fallback otherwise), computes both shingle/token lexical overlap and cached `llm_embed` cosine similarity, and persists `scientia_novelty_bundle` in `metadata_json` (recomputing `content_sha3_256`).
   - Offline mode (or when no embedding model is configured) skips network/embedder calls cleanly and runs `AtomicNoveltyScorer` over local `VoxDb` + lexical overlap, returning `Novel` / `PossiblyNovel` / `NotNovel` when prior-art hits exist or `InsufficientEvidence` when zero corpus hits were checked.
3. **Continuous Direct Publication Flywheel Flow:**
   - Calling `POST /api/v2/scientia/publications/{id}/flywheel-advance` (or `vox scientia publication-flywheel-advance`) executes an idempotent, non-destructive preparation state machine in `vox_scientia::review_flow::advance_publication_flywheel`:
     1. Ensure manifest exists (auto-promoting `FindingCandidateV1` if given a `candidate_id` or `session_id`).
     2. Run calibrated `ExtractionPipeline` (`MiniCheck`) if `extracted_claims` is absent/stale for current body digest.
     3. Run federated + local `novelty_fetch` if `scientia_novelty_bundle` is absent/stale.
     4. Run deterministic replay check (`record_replay_for_publication`) when a `MainEntity` replay spec is attached.
     5. Validate IMRaD + LaTeX + arXiv `.tar.gz` + FindingPage HTML + RO-Crate completeness (`ManifestCompletionReport`).
     6. **Hard Security Stop:** Transition to `awaiting_review` and surface in Axis `DiscoveryReview` / `NeedsYou`. Once human `record_claim_review(..., "approved")` (+ `CriticGate` where required by venue policy) is recorded on the current `content_sha3_256`, unlock one-click signed `nanopub_build` (now hydrated with real `novelty_verdict` and `prior_art_uris`!) and `publication_archive_run` (Zenodo / Software Heritage / OpenReview / Nanopub test server).

### Exact New vs. Modified Contracts (For Phase Planners)

| Kind | Identifier | Location | Change Type & Contract Summary |
|------|------------|----------|--------------------------------|
| **Struct / Enum** | `ResearchContinuumMode` & `EscalationReason` | `crates/vox-orchestrator-types/src/socrates_policy.rs` | **NEW (L0):** `Off \| Auto \| Fast \| Deep`; reasons: `LowLocalConfidence`, `HighContradiction`, `LowCragQuality`, `ExplicitDeepIntent`, `RecencySignal`. |
| **Field** | `ChatMessageParams::research_mode` | `crates/vox-orchestrator-mcp/src/chat_tools/params.rs:137` | **NEW:** `Option<String>` (`"auto"\|"fast"\|"deep"\|"off"`), decoupling depth routing from `research_scope` (`"local"\|"web"\|"both"`). |
| **Enum Variants** | `ResearchEvent::{WaveStarted, WaveCompleted, SubqueryDispatched, ContinuumEscalated, EpistemicDagDelta}` | `crates/vox-research-events/src/events.rs` | **MOD (L1):** Add monotonic `seq: u64` envelope (`SequencedResearchEvent`) + fine-grained wave/DAG delta variants. |
| **Struct** | `CachedLlmEmbedder<'a>` | `crates/vox-scientia/src/embedder.rs` | **MOVED L5→L3:** Lifted from `vox-cli-research/src/db/publication/embedder.rs` so daemon + CLI share `scientia_embedding_cache` + `llm_embed`. |
| **Method** | `AtomicNoveltyScorer::score` | `crates/vox-scientia/src/inspect_bridge/novelty.rs:94` | **MOD:** Fuse `max_semantic_score` + `max_lexical_score`; return `InsufficientEvidence` (never `Novel`!) when both are `None` or hit count is `0`. |
| **HTTP / SSE** | `GET /api/v2/research/sessions/{id}/stream` | `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` | **NEW:** `axum::response::sse::Sse` stream of `SequencedResearchEvent` frames with `?from_seq=` cursor resume. |
| **HTTP REST** | `GET /api/v2/research/sessions/{id}/claims` & `/sources` | `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` | **NEW:** Cursor-paginated (`?after_id=&limit=100`) claims and sources; `GET /sessions/{id}?summary_only=true` omits multi-MB `artifact_json`. |
| **HTTP REST** | `GET /api/v2/scientia/publications/{id}/claims` & `GET /api/v2/scientia/venue-recommend` | `crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs` | **NEW:** Replaces `PublicationsView.tsx` child-CLI `execute_command` calls (`VoxDb` lock hazard). |
| **HTTP REST** | `POST /api/v2/scientia/publications/{id}/novelty-fetch` & `/flywheel-advance` | `crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs` | **NEW:** Daemon-native prior-art bundle fetch (`?offline=false`) and pre-review flywheel state machine. |
| **DB Schema** | `research_session_sources` + `research_sessions` columns (`continuum_tier`, `escalated_from`, `wave_count`, `wave_stability`) | `crates/vox-db/src/schema/` | **MOD (Additive Migration):** Indexed `(session_id, source_id)` table so 500-source sessions paginate in `O(limit)` SQL without parsing `artifact_json`. |

---

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| **Normal Run (1 wave, ≤ 20 sources, ≤ 30 claims)** | `T1 Local RAG` (~150 ms) or `T2 Quick Web` (~1.8 s) answers inline; `ResearchDetailDto` summary fits in `< 16 KB`; SVG `ResearchDagCanvas` renders full 4-layer graph directly. |
| **Deep Adversarial Run (3–5 waves, 150–500 sources, 300–1,000 claims)** | `GET /research/sessions/{id}?summary_only=true` omits raw `artifact_json`; claims & sources load in `limit=100` cursor pages from `scientia_claims` / `research_session_sources`; `ResearchDagCanvas.tsx` collapses/clusters `source_domain` and `claim` nodes per wave when `nodes.length > 80` (preventing 1,000+ SVG `<path>` Bezier DOM stalls). |
| **Extreme Archive Scale (10,000+ sessions, 250,000+ claims, concurrent CLI + GUI + MCP)** | Single-writer `VoxDb` inside `vox-orchestrator-d` eliminates `SQLITE_BUSY`; `scientia_feed.rs` event-driven watermarks replace `limit=500` table scans; `scientia_embedding_cache` deduplicates `llm_embed` calls by `(blake3(text), model)`. |

### Scaling Priorities

1. **First bottleneck — Monolithic `artifact_json` & Polling IPC Transfer:** Today `get_research_session_detail` clones the entire `artifact_json` string on every view open (`gui_ops/research.rs:1176`), and `spawn_scientia_queue_stream` polls 500 manifests + 200 sessions (`vox-gui/src/commands/scientia.rs:178-228`). **Fix:** Add `?summary_only=true` (default for initial GUI render), cursor-paginated `/claims` and `/sources` tables, and event-driven push on `state.research_events`.
2. **Second bottleneck — Unbounded `ResearchDagCanvas` SVG Node Count:** `ResearchDagCanvas.tsx:189-250` renders every node and cubic Bezier edge in a single expanded SVG viewport. At 400 nodes × 800 edges, layout width/height explodes (`NODE_GAP_Y = 85` × 400 = `34,000px`). **Fix:** Group claims/sources into expandable wave cluster summaries above 80 nodes with viewport windowing and filter pills (`All | Contradicted | Corroborated | Sandbox`).

---

## Anti-Patterns

### Anti-Pattern 1: Opening `VoxDb` or Spawning DB-Opening CLI Subprocesses from `vox-gui`

- **What people do:** Add `VoxDb::connect_default()` in a Tauri `#[tauri::command]` or call `invokeCmd('execute_command', { path: ['scientia', ...] })` from React (as `PublicationsView.tsx:237-254` currently does).
- **Why it's wrong:** Turso/SQLite single-file locking rejects concurrent connections across `vox-gui` (or its spawned `vox` child process) and `vox-orchestrator-d` (`SQLITE_BUSY` / `os error 33`), and `execute_command` breaks completely when `vox-gui` is attached to a remote `vox serve` backend (`Backend::Remote(GatewayClient)`).
- **Do this instead:** Every GUI read, write, and stream MUST route through `State<'_, Arc<BackendState>>` (`api_v2_get`, `api_v2_post`, `call_tool`, or `/v1/ws` / `vox://agent-events`) into `vox-orchestrator-mcp::gui_ops`. Also update `http_gateway/scientia_feed.rs:59,122` to use the daemon's shared `Arc<VoxDb>` from `ServerState` instead of calling `VoxDb::connect_default()` every 5 seconds.

### Anti-Pattern 2: Scoring Missing Dense Embeddings (`None`) as `NoveltyVerdict::Novel`

- **What people do:** Treat `max_semantic_score: None` as zero prior-art overlap (`crates/vox-scientia/src/inspect_bridge/novelty.rs:119`).
- **Why it's wrong:** Whenever `Embedder` is `None` (offline mode or unconfigured embedding key) or a provider returns hits before semantic enrichment, `scientia_prior_art.rs:202,249,299` leaves `semantic_score: None` while computing a valid `lexical_score`. Mapping `None => Novel` falsely certifies verbatim published papers as novel discoveries!
- **Do this instead:** Fuse semantic and calibrated lexical scores in `AtomicNoveltyScorer::score`, and return `NoveltyVerdict::InsufficientEvidence` whenever zero prior-art sources were successfully evaluated.

### Anti-Pattern 3: Overloading `research_scope` with Continuum Depth Tiers

- **What people do:** Pass `'deep' | 'quick' | 'auto'` inside `research_scope` (`crates/vox-gui/ui/src/App.tsx:1973`) while backend `ChatMessageParams::research_scope` (`params.rs:139`) means retrieval corpus (`'local' | 'web' | 'both'`).
- **Why it's wrong:** Conflates *where* to search (local repo/KB vs web vs both) with *how deep* to reason (0-hop, 1-wave, multi-wave), causing explicit UI dropdown selections (`Fast Web` vs `Deep Research`) to be silently ignored by `classify_research_intent` and `run_deep`.
- **Do this instead:** Carry `research_mode: Option<String>` (`"auto" | "fast" | "deep" | "off"`) orthogonal to `research_scope: Option<String>` (`"local" | "web" | "both"`), and wire both all the way from `Loquela` / `App.tsx` → `buildChatTurn.ts` → `ChatMessageParams` → `message.rs` → `research_turn.rs` → `ResearchSessionConfig`.

### Anti-Pattern 4: Adding Reverse L3 Dependencies (`vox-orchestrator -> vox-scientia` or `vox-publisher -> vox-research-shim`)

- **What people do:** Import `vox-research-shim` or `vox-scientia` directly inside `vox-orchestrator` (`socrates.rs`), or import `vox-orchestrator-mcp` inside `vox-scientia`.
- **Why it's wrong:** Creates an L3 dependency cycle and blows `vox-orchestrator`'s `max_dependents = 30` fan-in budget in `docs/src/architecture/layers.toml:300`, failing `cargo run -p vox-arch-check`.
- **Do this instead:** Keep pure policy/threshold structs in `vox-orchestrator-types` (L0), shared event/bundle payloads in `vox-research-events` (L1), reusable domain logic in `vox-scientia` / `vox-search` / `vox-publisher` (lower-fan-out L3), and cross-subsystem orchestration in `vox-research-shim` and `vox-orchestrator-mcp` (top of L3).

### Anti-Pattern 5: Bypassing `vox_actor_runtime::llm`, Clavis, or `ApprovalToken`

- **What people do:** Call external embedding/verifier/LLM URLs directly with `reqwest` + `std::env::var`, or allow `flywheel_advance` to push to external publication networks unattended.
- **Why it's wrong:** Violates `AGENTS.md` normative policies (`vox-code-audit` `llm_provider_call` and `secret-env-guard`) and breaks the non-negotiable SCIENTIA digest-bound human review invariant (`no_network_publish_symbol_in_review_flow` in `review_flow.rs:1385`).
- **Do this instead:** Route every LLM/embedding call through `vox_actor_runtime::llm`, resolve every credential (`TavilyApiKey`, `VoxOpenAlexEmail`, `VoxSemanticScholarApiKey`, `VoxArxivAccessToken`, `VoxZenodoAccessToken`, `VoxOpenReviewAccessToken`, `VoxUserRsaNanopubPrivateKeyB64`) via `vox_secrets::resolve_secret`, hash digests via `vox-crypto` (`content_sha3_256`), and halt automated flywheel progression at `awaiting_review` until an unforgeable `ApprovalToken` is minted on the exact manifest digest.

---

## Integration Points

### External Services

| Service | Integration Crate & File | Clavis `SecretId` | Standalone / Offline Fallback |
|---------|--------------------------|-------------------|-------------------------------|
| **LLM Chat / Judge / Embeddings** | `vox-actor-runtime::llm` (`llm_chat`, `llm_stream`, `llm_embed`) | Provider keys resolved inside `vox-actor-runtime` / `vox-secrets` | Deterministic heuristic planner, lexical MiniCheck calibration, and lexical shingle novelty fallback when offline/local-only. |
| **Tavily / SearXNG / DDG / Wikipedia** | `vox-search` (`web_dispatcher.rs`, `tavily_budget.rs`) | `SecretId::TavilyApiKey` (+ SearXNG URL in `SearchPolicy`) | Keyless providers (`Wikipedia`, `arXiv`, `OpenAlex`, `DDG`) when Tavily unconfigured; `Local` Workspace/KB Tantivy search when 100% offline. |
| **OpenAlex / Crossref / Semantic Scholar** | `vox-publisher::scientia_prior_art` (`scientia_prior_art.rs`) | `SecretId::VoxOpenAlexEmail`, `SecretId::VoxSemanticScholarApiKey` | `offline = true` (or network timeout) skips HTTP cleanly, marks `sources_checked`, and scores against local `VoxDb` prior-art + lexical overlap (`InsufficientEvidence` if empty). |
| **arXiv Staging & Discovery** | `vox-search` + `vox-scientia::manuscript::latex` | `SecretId::VoxArxivAccessToken` | Generates complete offline `.tar.gz` bundle (`main.tex` + `arxiv_handoff.json` + figures) without network access. |
| **Nanopub Signing & Test Server** | `vox-scientia::nanopub` & `review_flow.rs` | `SecretId::VoxUserRsaNanopubPrivateKeyB64` | 100% offline RSA signing + Trusty URI generation + TriG validation (`published_state = "local"`); test-server push requires `ApprovalToken` + explicit opt-in. |
| **Zenodo / Software Heritage / OpenReview** | `vox-publisher::archive_run` & `scholarly` | Zenodo / SWH / OpenReview secrets via Clavis | `plan_archive_run(..., dry_run: true)` produces full deterministic step plan + RO-Crate JSON-LD locally with zero network I/O. |

### Internal Boundaries

| Boundary | Communication Mechanism | Direction & Invariant |
|----------|-------------------------|-----------------------|
| `vox-gui` ↔ `vox-orchestrator-d` | `BackendState` (`OrchDaemonClient` JSON-RPC + `/api/v2/*` REST/SSE + `vox://agent-events` / `/v1/ws`) | `L5 -> L3` protocol types (`vox-foundation::dei_method`); **zero `VoxDb` in `vox-gui`**. |
| `vox-orchestrator-mcp` ↔ `vox-research-shim` | Direct Rust async calls (`ResearchOrchestrator::run`, `BroadcastEmitter`) + `vox-research-events` | `L3 -> L3` (already in `Cargo.toml`); `BroadcastEmitter` feeds `state.research_events`. |
| `vox-publisher` ↔ `vox-scientia` | Direct Rust trait/struct calls (`Embedder`, `NoveltyConfig`, `AtomicNoveltyScorer`, `manuscript`, `ro_crate`) | `vox-publisher -> vox-scientia` (`L3 -> L3`, already in `Cargo.toml`); **never** `vox-scientia -> vox-publisher`. |
| `vox-orchestrator` ↔ `vox-research-shim` | `vox-research-shim -> vox-orchestrator` (`socrates.rs`, `research_gate.rs`) + `vox-orchestrator-types` (L0) | One-way `vox-research-shim -> vox-orchestrator`; **never** reverse. |

---

## Build Order & Dependency-Ordered Phase Breakdown (`Phases 447–452`)

Milestone v7.6 executes across **6 strictly ordered phases (`Phases 447–452`)**. Following Vox's empirical verification doctrine, **Phase 447 captures a deterministic Before-audit baseline first** before any behavioral changes land, and **Phase 452 executes the After-audit + inline Before/After diff closure** at the end.

```
Phase 447 (Before-Audit Baseline + L0/L1 Contracts + Additive DB Schema)
    │
    ├──▶ Phase 448 (Unified Light ↔ Deep Continuum & Socrates/CRAG Escalation Spine)
    │        │
    │        ▼
    ├──▶ Phase 449 (Extreme-Data Resumable Streaming, Cursor Pagination & Virtualized DAG)
    │
    └──▶ Phase 450 (Calibrated Hybrid Semantic+Lexical Novelty & Unified Socrates Telemetry)
             │
             ▼  (merges 448 + 449 + 450)
         Phase 451 (Continuous Direct Publication Flywheel & Zero-VoxDb Axis Studio Unification)
             │
             ▼
         Phase 452 (After-Audit Execution, E2E Benchmarks & Inline Before/After Diff Closure)
```

### Phase 447: Before-Audit Baseline Capture, L0/L1 Contracts & Additive Session-Source Schema
- **Why first:** Locks an objective, reproducible `"before"` snapshot of every v7.6 metric (continuum routing accuracy, `research_scope` bug reproduction, IPC payload byte size at 200+ claims/sources, `None => Novel` false-positive rate, `PublicationsView` CLI-DB spawn count, and manual steps per publication) **and** lands the foundational L0/L1 types + additive `VoxDb` migration that Phases 448–450 depend on.
- **Exact Touchpoints:**
  - `crates/vox-cli-research/src/scientia_v76_audit.rs` (NEW) + CLI subcommand `vox scientia v76-audit --stage before` writing `contracts/reports/scientia-v76-before-baseline.v1.json`.
  - `crates/vox-orchestrator-types/src/socrates_policy.rs` (`ResearchContinuumMode`, `ResearchContinuumTier`, `EscalationReason`, `CragEscalationPolicy`).
  - `crates/vox-research-events/src/events.rs` (`SequencedResearchEvent`, `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta`).
  - `crates/vox-db/src/schema/` + store methods (`research_session_sources` table, cursor-paginated `list_session_claims_page` & `list_session_sources_page`, `continuum_tier` / `escalated_from` columns on `research_sessions`).
- **Verification Gate:** `cargo run -p vox-arch-check` (0 violations), `vox-db` migration round-trip tests, and committed `scientia-v76-before-baseline.v1.json`.

### Phase 448: Unified Light ↔ Deep Research Continuum & Socrates/CRAG Escalation Spine
- **Depends on:** Phase 447 (`ResearchContinuumMode`, `EscalationReason`, `ContinuumEscalated`).
- **Why second:** Fixes the core routing discontinuities so both Chat (`Loquela`) and `ResearchView` share one truthful 4-stage escalation engine before streaming UI layers attach to it.
- **Exact Touchpoints:**
  - `crates/vox-search/src/research.rs` (`CragEscalationSignal`: coverage, distinct-domain corroboration, snippet conflict score).
  - `crates/vox-research-shim/src/research/gate.rs` & `orchestrator/pipeline.rs` (accept `warm_seed_hits` from T1 Local RAG / T2 Quick Web so Wave 1 never re-fetches already-retrieved URLs; honor `scope`, `domain_mode`, `site_scope`, `waves`).
  - `crates/vox-orchestrator-mcp/src/chat_tools/params.rs`, `message.rs`, `research_intent.rs`, `research_turn.rs` (reorder `message.rs` so `SocratesGateOutcome` feeds `classify_research_intent_with_socrates`; implement `T1 -> T2 -> T3` auto-escalation; pass `research_mode` cleanly).
  - `crates/vox-gui/ui/src/App.tsx` & `crates/vox-gui/ui/src/lib/buildChatTurn.ts` (stop writing `'deep'|'quick'|'auto'` into `research_scope`; pass dedicated `research_mode` while keeping `research_scope` for `'local'|'web'|'both'`).
- **Verification Gate:** Unit + integration tests proving: (a) `Off` makes 0 external calls, (b) `Fast` runs 1-wave Quick, (c) `Deep` runs multi-wave with user `scope`/`domain_mode`/`site_scope`, and (d) `Auto` answers directly on high-confidence local RAG, escalates to Quick on local abstention, and escalates Quick → Deep with warm seeds when CRAG quality `< 0.68` or contradiction `>= 0.25`.

### Phase 449: Extreme-Data Resumable Streaming, Cursor Pagination & Virtualized Epistemic Canvas
- **Depends on:** Phase 447 (`SequencedResearchEvent`, paginated DB queries) and Phase 448 (continuum events).
- **Why third:** Eliminates memory/IPC buffering bottlenecks across both the backend gateway and the React DOM so multi-minute, 500-source Deep Research runs stream smoothly in Chat and Research surfaces.
- **Exact Touchpoints:**
  - `crates/vox-orchestrator-mcp/src/server_state.rs` & `http_gateway/scientia_feed.rs` (replace `VoxDb::connect_default()` with `ServerState.db`; bridge `state.research_events` onto `state.orchestrator.event_bus()` and `TopicMessage` `"research.session.delta"`).
  - `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` (replace post-hoc `Mutex<Vec<_>>` buffering with live `AgentEventKind` emissions per wave/subquery/escalation).
  - `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` & `crates/vox-gui/src/commands/research.rs` (`GET /api/v2/research/sessions/{id}?summary_only=true`, `GET /sessions/{id}/claims?after_id=&limit=100`, `GET /sessions/{id}/sources?after_id=&limit=100`, `GET /sessions/{id}/stream?from_seq=`).
  - `crates/vox-gui/ui/src/components/surfaces/Research/{ResearchView.tsx, MultiWaveProgressTimeline.tsx, ResearchDagCanvas.tsx}` & `Chat/{ResearchTracePanel.tsx, ChatExecutionRail.tsx}` (drive `MultiWaveProgressTimeline` from live wave events instead of hardcoded `currentWave={1}`; add wave-cluster virtualization in `ResearchDagCanvas` when `nodes.length > 80`; paginate claim accordion).
- **Verification Gate:** Stress benchmark test streaming 5 waves / 300 sources / 600 claims with `< 64 KB` peak delta frame size, resumable `?from_seq=` recovery on simulated disconnect, and `< 16ms` React render budget on clustered DAG.

### Phase 450: Calibrated Hybrid Semantic+Lexical Prior-Art Novelty & Unified Socrates Telemetry
- **Depends on:** Phase 447 (can run in parallel with Phase 449 if desired, before converging in Phase 451).
- **Why fourth:** Closes the false-novelty defect (`None => Novel`) and lifts `VoxDb`-cached dense embeddings into L3 before the automated publication flywheel consumes novelty verdicts in Phase 451.
- **Exact Touchpoints:**
  - `crates/vox-scientia/src/embedder.rs` (lifted `CachedLlmEmbedder` using `VoxDb` `scientia_embedding_cache` + `vox_actor_runtime::llm::llm_embed`) + thin re-export in `crates/vox-cli-research/src/db/publication/embedder.rs`.
  - `crates/vox-scientia/src/inspect_bridge/novelty.rs` (hybrid semantic + calibrated lexical overlap scoring; `None` + 0 hits => `InsufficientEvidence`, never `Novel`; recency + contradiction weighting).
  - `crates/vox-publisher/src/scientia_prior_art.rs` (populate calibrated token/shingle `lexical_score` + dense `semantic_score` across OpenAlex, Crossref, and Semantic Scholar; emit phase-tagged `"novelty"` cost row to `scientia_cost_telemetry`).
  - `crates/vox-scientia/src/producers/socrates_telemetry.rs` & `crates/vox-research-shim/src/research/verifier.rs` (unify `ClaimVerifier` resample stability + Socrates abstention/contradiction telemetry across `research_metrics` and `agent_telemetry_flat`).
- **Verification Gate:** Hermetic property + unit tests proving: (a) verbatim prior-art abstract with `semantic_score: None` (offline mode) scores `NotNovel` via calibrated lexical fallback (0% false-`Novel`), (b) empty offline bundle scores `InsufficientEvidence`, and (c) cached `llm_embed` hits `scientia_embedding_cache` with 0 duplicate facade calls.

### Phase 451: Continuous Direct Publication Flywheel & Zero-`VoxDb` Axis Studio Unification
- **Depends on:** Phases 448, 449, and 450.
- **Why fifth:** Wires verified Deep Research sessions, calibrated novelty bundles, digest-bound claim review, nanopub signing, and archival packaging into a seamless end-to-end flywheel across daemon, CLI, and GUI—while eliminating the last `execute_command` CLI-DB calls in `vox-gui`.
- **Exact Touchpoints:**
  - `crates/vox-scientia/src/review_flow.rs` (hydrate `nanopub_build` lines 257–264 with persisted `scientia_novelty_bundle` verdict + `prior_art_uris` + atomic claim tuple when present; add `advance_publication_flywheel` orchestrating promote → extract-claims → novelty-fetch → replay → IMRaD/LaTeX/arXiv/FindingPage/RO-Crate readiness up to the mandatory `ApprovalToken` / `CriticGate` stop).
  - `crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs` & `crates/vox-gui/src/commands/scientia.rs` (add `GET /scientia/publications/{id}/claims`, `GET /scientia/venue-recommend`, `POST /scientia/publications/{id}/novelty-fetch`, `POST /scientia/publications/{id}/flywheel-advance`).
  - `crates/vox-cli-research/src/scientia_phase_handlers.rs` (`vox scientia publication-flywheel-advance`).
  - `crates/vox-gui/ui/src/components/surfaces/Publications/PublicationsView.tsx`, `Scientia/{DiscoveryReview.tsx, NoveltyEvidencePanel.tsx}`, & `Research/ResearchView.tsx` (remove `execute_command` spawns in `PublicationsView.tsx:237-254`; add one-click `"Run Prior-Art Novelty Check"` and `"Advance Flywheel"` stepper in `DiscoveryReview` and `PublicationsView`; keep `no_network_publish_symbol_in_review_ops` guard green).
- **Verification Gate:** End-to-end integration test from `ResearchSession` → `FindingCandidateV1` → `flywheel-advance` → human `record_claim_review("approved")` → `nanopub_build` (asserting real Trusty URI + hydrated novelty + prior-art URIs in signed TriG) + `archive_run --dry-run`, plus AST/source audit proving **zero** `VoxDb` or `execute_command` DB calls in `vox-gui`.

### Phase 452: After-Audit Execution, End-to-End Flywheel Benchmarks & Inline Before/After Diff Closure
- **Depends on:** Phases 447–451.
- **Why last:** Re-runs the exact Phase 447 audit harness (`vox scientia v76-audit --stage after --compare contracts/reports/scientia-v76-before-baseline.v1.json`), verifies every claim against live execution evidence, updates the architecture SSOT with inline Before → After metric deltas, and runs workspace architectural/audit gates.
- **Exact Touchpoints:**
  - `crates/vox-cli-research/src/scientia_v76_audit.rs` (After-audit capture + deterministic diff verifier).
  - `contracts/reports/scientia-v76-after-closure.v1.json` (measured Before vs After metrics across continuum accuracy, streaming frame ceiling, false-novelty rate = 0%, flywheel automation coverage, and `vox-gui` direct-DB count = 0).
  - `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` (inline v7.6 Before/After diff closure table + frontmatter lint check via `vox-doc-pipeline --lint-only`).
- **Verification Gate:** `cargo run -p vox-arch-check`, `vox audit code`, Vitest/Playwright Research+Scientia+Publications+Chat suites, and `vox scientia v76-audit --verify-closure` exiting `0`.

---

## Sources

### Vox Codebase Sources (Inspected Directly)
- `docs/src/architecture/layers.toml` (lines 1–628) — L0–L5 crate layer assignments, `max_dependents` budgets, and `[[forbidden_deps]]`.
- `docs/src/architecture/where-things-live.md` (lines 1–247) & `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` (lines 1–171).
- `crates/vox-research-shim/src/research/` — `orchestrator/pipeline.rs` (lines 1–1250), `orchestrator/wave.rs` (lines 1–400), `verifier.rs` (lines 1–650), `gate.rs` (lines 1–363), `discovery_bridge.rs` (lines 1–350).
- `crates/vox-search/src/` — `research.rs` (lines 1–158), `novelty.rs` (lines 1–145).
- `crates/vox-scientia/src/` — `review_flow.rs` (lines 1–395, 1150–2049), `inspect_bridge/novelty.rs` (lines 1–284), `claim_extractor/minicheck.rs` (lines 1–300), `producers/socrates_telemetry.rs` (lines 1–101), `manuscript/scaffold/architecture_ssot.rs` (lines 1–300).
- `crates/vox-publisher/src/` — `scientia_prior_art.rs` (lines 1–300), `scientia_discovery.rs` (lines 1–300), `scientia_evidence/mod.rs` (lines 1–300).
- `crates/vox-orchestrator/src/socrates.rs` (lines 1–350) & `crates/vox-research-events/src/events.rs` (lines 1–160).
- `crates/vox-orchestrator-mcp/src/` — `Cargo.toml` (lines 1–120), `gui_ops/research.rs` (lines 1–1690), `gui_ops/scientia.rs` (lines 1–1200), `http_gateway/scientia_feed.rs` (lines 1–250), `daemon_extra.rs` (lines 50–160), `chat_tools/params.rs` (lines 35–156), `chat_tools/chat/{message.rs, research_intent.rs, research_turn.rs, agent_loop.rs}`.
- `crates/vox-cli-research/src/` — `scientia_phase_handlers.rs` (lines 1–1300), `db/publication/{mod.rs, discovery.rs, embedder.rs}`.
- `crates/vox-gui/` — `src/remote/backend.rs` (lines 1–211), `src/commands/{gui_db_pool.rs, orchestrator.rs, research.rs, scientia.rs, scientia_review.rs, chat_turn.rs}`, `ui/src/App.tsx` (lines 497–505, 1967–2025), `ui/src/lib/buildChatTurn.ts` (lines 1–119), `ui/src/api/scientia.ts` (lines 1–243), and `ui/src/components/surfaces/{Research,Scientia,Publications,Chat,Loquela}/*`.

### 2025–2026 External Prior Art
- **Adaptive & Corrective Multi-Hop RAG Routing:** Yan et al., *Corrective Retrieval Augmented Generation (CRAG)*; Jeong et al., *Adaptive-RAG: Learning to Adapt Retrieval-Augmented Large Language Models through Question Complexity*; 2025–2026 modular agentic RAG escalation patterns (tiered parametric → hybrid local RAG → single-step CRAG web → iterative multi-wave deep research with warm-context carry-over).
- **Atomic Claim Verification & Scientific Novelty Calibration:** Tang et al., *MiniCheck: Efficient Fact-Checking of LLMs on Grounding Documents* (ACL/EMNLP, Bespoke-MiniCheck-7B calibration); Song et al., *VeriScore: Evaluating the Factuality of Verifiable Claims in Long-Form Text Generation*; Singh et al. (AI2), *SPECTER2: Adapting Scientific Document Representations with Task-Specific Adapters* for citation/abstract novelty proximity.
- **Streaming Epistemic Graphs & Machine-Actionable Scholarly Publishing:** Soiland-Reyes et al., *Packaging research artefacts with RO-Crate* (`researchobject.org` JSON-LD FAIR bundles); Kuhn et al. / Knowledge Pixels (2025–2026), *Nanopublications & Trusty URIs for AI-Agent-Generated Atomic Scientific Assertions* (`nanopub.net`); SSE/WebSocket resumable cursor-watermarked event streaming for long-horizon deep research agents.

---
*Architecture research for: Vox Milestone v7.6 — Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel*
*Researched: 2026-11-14*
