# Project Research Summary

**Project:** Vox Milestone v7.6 — Unified Research Continuum, Extreme-Data Streaming, Epistemic Calibration & Direct Publication (`--ws scientia-deep-research-v76`, `Phases 447–452`)
**Domain:** Agentic Light ↔ Deep Research Continuum, Out-of-Core Streaming Telemetry, Calibrated Socrates/NLI/Novelty Verification, Human-Gated Direct Scientific/Architectural Publication & Deterministic Before/After Evaluation Flywheel
**Researched:** 2026-11-14
**Confidence:** HIGH

## Executive Summary

Milestone v7.6 unifies Vox's split Chat RAG, multi-wave Deep Research, epistemic verification, continuous discovery, and SCIENTIA scholarly publishing pipelines into one truthful, end-to-end architecture across CLI, Chat (`Loquela`), MCP, HTTP/SSE, and Axis GUI surfaces (`Phases 447–452`). Frontier 2025–2026 systems (Perplexity Effort Dials, FutureHouse *Crow–Falcon–Owl–Finch–Robin*, Google *AI Co-Scientist*, DeepResearch Bench) converge on adaptive depth routing, span-level NLI citation grounding, precedent-aware novelty filtering, and human-collaborative publication. Vox goes further by combining compiler- and sandbox-verified probes (`CodeGen`), out-of-core `std.frame` tabular streaming, calibrated Socrates risk gating, and multi-target **Direct Publication (`> nanopublication`)** from a single `VoxDb` source of truth—with **zero new third-party Rust crates in `Cargo.toml`**, **zero new npm packages in `package.json`**, **zero `layers.toml` / `crate-edges` violations**, and **zero `VoxDb` handles or child CLI-DB subprocesses in `vox-gui`**.

The recommended architecture replaces disconnected mode toggles and mock fallbacks with four tightly coupled spines inside existing L0–L5 crates: (1) a warm-seeded 4-stage research continuum (`T0 Direct` → `T1 Local Hybrid RAG` → `T2 Quick 1-Wave Web` → `T3 Deep Multi-Wave Adversarial`) across four domain modes (`General`, `CodeGen`, `Shopping`, `Scholarly`) that preserves day-stable Chat KV-cache prefixes; (2) watermarked `seq`-numbered SSE/WS delta streaming, `std.frame` Arrow chunk pagination, and virtualized DAG/claim views that eliminate the `180s` Chat timeout wall and multi-MB `artifact_json` IPC stalls; (3) a 3-tier claim/citation verification cascade + two-stage **Novelty × Utility** (`WorthinessSignalsV2`) filter backed by `vox-similarity`, `vox_actor_runtime::llm`, and rate-governed/offline-resilient scholarly adapters; and (4) an idempotent `flywheel_advance` Direct Publication pipeline producing `vox-doc-pipeline`-clean `docs/src/architecture/` SSOTs, hardened venue IMRaD/LaTeX, deterministic arXiv `.tar.gz` bundles, HTML Finding Pages, RO-Crate 1.2, Trusty-URI nanopubs, and Zenodo/OpenReview archives behind an unbreakable 4-layer human `ApprovalToken` gate.

Line-by-line codebase inspection uncovered seven live correctness hazards that v7.6 must remediate directly rather than paper over: `App.tsx:1972` overloading `research_scope` with depth modes; `message.rs:142` running intent classification *before* Socrates retrieval; `gui_ops/research.rs:715` (`write_research_doc`) wiping `artifact_json` to `"{}"`; `AtomicNoveltyScorer` (`novelty.rs:119`) certifying `max_semantic_score: None` as `Novel`; `MiniCheckVerifier::from_env()` silently falling back to `Mock` word-overlap and `std::env::var`; `pipeline.rs:1164` creating an **Epistemic Ouroboros** (`save_memory` reloaded at `trust_score: 1.0`); and `eval.rs:364` returning hardcoded `1.0` multi-hop scores. Capturing a deterministic frozen-cassette **Before-audit baseline in Phase 447** and closing with a paired **After-audit scorecard + inline diff inspector + quarantined `VoxDb` lesson loop in Phase 452** guarantees every v7.6 improvement is empirically proven.

---

## Key Findings

### Recommended Stack

See [STACK.md](./STACK.md) for full version matrices and external REST contracts. **Milestone v7.6 requires ZERO new Cargo crates and ZERO new npm dependencies.** Earlier August 2026 notes floated `fastembed`, `ort` (`onnxruntime-sys`), and C++ `hnswlib`; v7.6 explicitly **rejects** them because prebuilt C++ binaries, `cmake`, and `libclang` (`bindgen`) violate `AGENTS.md §Build-toolchain invariant`. All neural, lexical, statistical, packaging, and UI primitives are already pinned and building cleanly in the workspace.

**Core technologies (workspace-native):**
- **`vox_actor_runtime::llm` (`0.6.0`, L3):** Mandatory model-agnostic LLM facade (`infer_with_retry`, `llm_chat`, `llm_stream_events_activity`, `llm_embed`, offline `local_embed_text`) enforced by `vox-code-audit` (`llm_provider_call`); replaces hardcoded `LlmConfig::openrouter` and `VOX_MINICHECK_ENDPOINT` env reads in `minicheck.rs`.
- **`vox-similarity` (`0.6.0`, L2) + `VoxDb` `scientia_embedding_cache` + `rayon` (`1.10`):** Pure-Rust k-shingling, SimHash, `blake3`-seeded MinHash, and banded `LshIndex` in Layer 2 for Stage-1 near-duplicate/novelty filtering, paired with `VoxDb`-cached dense embeddings (`<2 ms` parallel cosine scan under 50k vectors, zero C++ ANN FFI).
- **`candle-core` / `candle-nn` / `candle-transformers` (`0.10.2` patched) + `safetensors` (`0.7`) + `tokenizers` (`0.21`, `fancy-regex`):** Optional 100% pure-Rust local NLI cross-encoder and scientific dense embeddings (`SPECTER2`/`BGE`) without `ort` or `onig_sys`.
- **`vox-search` (`tantivy 0.22` + `turso 0.6` FTS5 + RRF) + `governor` (`0.10`) + `vox-http-client` (`reqwest 0.12`):** Hybrid retrieval, CRAG quality grading, Clavis-authenticated scholarly REST adapters, and strict per-provider GCRA rate limiting (`Semantic Scholar: 1 RPS`, `Crossref Polite (?mailto=): 5–10 RPS / concurrency 3`, `OpenAlex Bearer/api_key: 20 RPS`—resolving OpenAlex's 2025–2026 retirement of the `mailto=` polite pool).
- **`std.frame` (`vox-python-bridge` Polars `2.1.0` / Arrow IPC) + pure-Rust streaming (`quick-xml`, `csv`, `text-splitter`, `tiktoken-rs`, `flate2` `rust_backend`):** Out-of-core tabular and extreme-corpus streaming (`O(batch_size)` RAM) with pure-Rust standalone fallback and byte-reproducible `.tar.gz` arXiv bundles.
- **`vox-crypto` + `nanopub` (`0.2`, strictly isolated in `vox-scientia::nanopub`) + `similar` (`2`, `inline`):** `BLAKE3` / `SHA3-256` / `ed25519` provenance digests via `vox-crypto` (remediating banned `sha2` in `scientia_prior_art.rs:11`), Clavis-custodied per-user RSA Trusty-URI (`RA...`) nanopub signing, and Myers/Patience inline diff hunks rendered via React 19 + `@tanstack/react-virtual`, `@xyflow/react`, and `recharts`.

### Expected Features

See [FEATURES.md](./FEATURES.md) for full dependency graphs and prior-art comparisons.

**Must have — Table Stakes (`P1` launch spine, zero mocks):**
- **TS-01 & TS-02: Unified Light (`<=1`-Hop) ↔ Deep (`>1`-Wave) Continuum Across 4 Domain Modes** — Single parameterized pipeline (`Off | Auto | Fast | Deep` orthogonal to `Local | Web | Both`) supporting `General`, `CodeGen`, `Shopping`, and newly added `Scholarly` across CLI, MCP, Chat, and GUI.
- **TS-03 & TS-04: Live Watermarked SSE Streaming, Real USD Cost Telemetry & `Loquela` Chat Ergonomics** — Resumable `/api/v2/research/sessions/{id}/stream` SSE + Chat turn events driving `MultiWaveProgressTimeline.tsx` and real `UsageSnapshot` → `scientia_cost_telemetry` (eliminating frozen Stage 0 timelines and fake `$0.00` costs).
- **TS-05 & TS-06: Calibrated 3-Tier Claim/Citation Verifier & Honest Prior-Art `InsufficientEvidence` Guard** — Delete `MiniCheckBackend::Mock` production fallback; enforce deterministic quote/numeric pre-filter + facade-routed SciFact NLI (`Support | Contradict | NotEnoughInfo | Abstain`) + HTTP-health-aware `NoveltyEvidenceBundle` where missing embeddings or failed lookups **never** default to `Novel`.
- **TS-07 & TS-08: 4-Layer Human-Gated Promotion & Full DB-Hydrated Direct Publication (`> Nanopublication`)** — Atomic 1-click candidate → `publication_manifests` promotion (`ApprovalToken` digest-bound) hydrating `VoxDb` directly into `vox-doc-pipeline`-clean `docs/src/architecture/` SSOTs, venue IMRaD/LaTeX, arXiv `.tar.gz`, RO-Crate 1.2, HTML Finding Pages, signed `RA` nanopubs, and Zenodo/OpenReview bundles.
- **TS-09: Reproducible Paired Before/After 6-Axis Evaluation Scorecard** — Deterministic cassette + stratified live harness (`vox scientia v76-audit` / `vox research eval --compare` via `vox_eval::paired_ni`) replacing hardcoded `eval.rs` stubs.

**Should have — Competitive Differentiators (`P1/P2` within v7.6):**
- **DF-01: Socrates Epistemic Auto-Escalation & Cost-Bounded Questioning** — Distinguishes retrievable *epistemic* uncertainty (runs `<2s` Light probe first, then auto-escalates `T1 -> T2 -> T3` with warm seeds) from *specification* ambiguity (asks at most 1 question when `information_gain_bits >= 0.25` vs `CLARIFICATION_INTERRUPT_COST_MS = 23_250`).
- **DF-02: Extreme-Data Chunked Streaming (`std.frame` Arrow Batches + Virtualized UI)** — Streams paginated `RecordBatch` slices and statistical summaries over SSE to `@tanstack/react-virtual` tables without dumping million-row CSVs into LLM context windows.
- **DF-03 & DF-04: Continuous Multi-Signal Discovery with Two-Stage Novelty × Utility (`WorthinessSignalsV2`)** — Mines `SocratesTelemetryProducer` high-entropy abstention clusters, commits, and `vox scientia scout` locally/continuously, requiring **both** Stage-1 prior-art novelty and Stage-2 practical utility (measured replay digest, benchmark delta, or compiler-verified probe) before surfacing to `DiscoveryInbox`.
- **DF-05, DF-06 & DF-07: Compiler-Verified Replay Projection, True Hierarchical Epistemic DAG Studio & Guarded "Next Lesson" Flywheel** — Backend-built hierarchical DAG (clustered `>80` nodes), unified zero-`VoxDb` `ScientiaSurface`, inline `BeforeAfterAuditInspector`, and quarantined `Untrusted` → `>=2`-corroborated lesson extraction into `kb_findings` / `vox-effort-route` / MENS Auto-DPO.

**Defer (`v7.6.x` / `v8.0+`) & Explicit Anti-Features:**
- **Defer to `v7.6.x`:** Optional local `candle` cross-encoder RRF reranker, `SPECTER2` adapter weights (if `Scholarly` offline recall needs uplift), and advisory VLM PDF layout review (`visus_review`).
- **Defer to `v8.0+`:** Live public Knowledge Pixels Nanopub production registry broadcast (keep offline `Nanopub::check()` + opt-in test-server in v7.6) and `Populi` cross-peer federated discovery gossip.
- **Hard Anti-Features (Never Build):** Unreviewed auto-publishing to public venues (`AF-01`), unbounded BFTS cost loops (`AF-02`), dumping raw million-row tables into prompts (`AF-03`), single-scalar novelty without utility gating (`AF-04`), and self-mutating `AGENTS.md`/canonical SSOTs from uncorroborated LLM lessons (`AF-07`).

### Architecture Approach

See [ARCHITECTURE.md](./ARCHITECTURE.md) for full layer diagrams and endpoint contracts. All v7.6 changes fit cleanly inside Vox's existing L0–L5 crate DAG with **zero new inter-crate `Cargo.toml` dependencies** and **zero `[[forbidden_deps]]` breaches**. Crucially, `vox-publisher` (L3) already depends on `vox-scientia` (L3), and `vox-scientia` already depends on `vox-db`, `vox-actor-runtime`, and `vox-search`; lifting `CachedLlmEmbedder` down from `L5` `vox-cli-research` into `L3` `vox-scientia::embedder` gives both `vox-orchestrator-mcp` (daemon/GUI) and `vox-cli-research` (CLI) shared `VoxDb`-cached embeddings without adding a single crate edge or touching `vox-orchestrator`'s `max_dependents = 30` budget.

**Major components & exact layer boundaries:**
1. **`L0` `vox-orchestrator-types::socrates_policy` & `vox-db-types`:** Pure `ResearchContinuumMode` (`Off | Auto | Fast | Deep`), `ResearchContinuumTier`, `EscalationReason`, `CragEscalationPolicy`, and cursor-paginated claim/source DTOs (zero async/IO).
2. **`L1` `vox-research-events`:** Monotonic `SequencedResearchEvent` (`seq: u64` watermark) adding `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, and `EpistemicDagDelta`, plus canonical `NoveltyEvidenceBundle` (consolidating duplicate `NoveltyEvidenceBundleV1`).
3. **`L2` `vox-similarity` & `vox-eval`:** Pure-Rust `LshIndex` / MinHash / shingle Jaccard for Stage-1 novelty + `paired_ni` statistical non-inferiority/superiority intervals for Before/After scorecards.
4. **`L3` `vox-scientia`, `vox-publisher`, `vox-search` & `vox-research-shim`:** Shared `vox-scientia::embedder::CachedLlmEmbedder`, fused semantic+lexical `AtomicNoveltyScorer`, 3-tier calibrated `MiniCheck`/`ClaimVerifier`, rate-governed scholarly prior-art (`OpenAlex`/`Crossref`/`S2`) with `OfflineAttested` fallback, warm-seeded multi-wave pipeline (`DomainMode::{General, CodeGen, Shopping, Scholarly}`), and `review_flow::advance_publication_flywheel`.
5. **`L3` `vox-orchestrator-mcp` (Composition & Streaming Hub):** Reorders `message.rs` so Socrates retrieval gates continuum routing; decouples `>1`-wave runs from the `180s` synchronous chat wall via durable session handles; bridges `state.research_events` via bounded MPSC batching (`ServerState.db`) to `event_bus` and SSE/WS; and exposes cursor-paginated `/api/v2/research/sessions/{id}/{stream,claims,sources}` and `/api/v2/scientia/publications/{id}/{claims,venue-recommend,novelty-fetch,flywheel-advance}`.
6. **`L5` `vox-gui` & `vox-cli-research`:** Zero-`VoxDb` desktop UI routing 100% through `crates/vox-gui/ui/src/transport.ts` (`BackendState` IPC/HTTP), eliminating `PublicationsView.tsx:237-254` `execute_command` CLI-DB lock hazards, plus headless `scientia_v76_audit.rs` Before/After harness.

### Critical Pitfalls

See [PITFALLS.md](./PITFALLS.md) for all seven pitfalls, recovery playbooks, and verification checklists.

1. **Breaking the Day-Stable Chat KV-Cache Prefix (`8–12×` Input Token Cost Spike):** Mutating `build_system_prompt_with_skill` (`chat_tools/mod.rs:341-375` `AttentionBudget` / `$operating_mode` / `$active_file`) or dynamically filtering MCP tool schemas per `ResearchMode` invalidates Anthropic/OpenAI/DeepSeek prompt caches on every turn. **Mitigation:** Keep `system_prompt` and ordered MCP tool schemas byte-identical across `Off | Auto | Fast | Deep`; inject all turn-volatile retrieval, wave state, and Socrates posture strictly into `turn_user_prompt` (`preamble_and_retrieval.rs`).
2. **Uncovered Live Data-Loss, Routing & Epistemic Bugs:**
   - `gui_ops/research.rs:715` (`write_research_doc`) passes `"{}"` to `store_research_artifact`, wiping persisted `artifact_json` (`claims`, `citations`, `DAG`) via `ON CONFLICT DO UPDATE`.
   - `App.tsx:1972` stuffs `'deep'|'quick'|'auto'` into `research_scope` (`"local"|"web"|"both"`), `message.rs:142` runs before Socrates retrieval (`message.rs:204`), and `research_turn.rs:628` hardcodes `waves: 1, scope: Web, domain_mode: General`.
   - `AtomicNoveltyScorer` (`novelty.rs:119`) maps `max_semantic_score: None => Novel` even when `max_lexical_score == 0.98`, and `MiniCheckVerifier::from_env()` (`pipeline.rs:96`) silently hits `Mock` word-overlap.
   - `pipeline.rs:1164` + `web_gather.rs:48-85` (**Epistemic Ouroboros**) saves unattested `report_markdown` to `VoxDb::save_memory` and re-ingests it in `ResearchScope::Both` at `trust_score: 1.0`.
   **Mitigation:** Fix each exact seam in dedicated red-green regression tests and tag autonomous syntheses as `untrusted_synthesis` (`D-06` taint quarantine).
3. **Extreme-Data OOM, `BroadcastStream` `Lagged` Drops & `180s` Chat Timeout Wall:** Synchronous `run_deep`, `256`/`64`-slot broadcast rings dropping stage transitions, per-event `tokio::spawn` SQLite writes, `5s` `VoxDb::connect_default()` polling in `scientia_feed.rs`, and full HTML `raw_content` clones bloat RSS and freeze React. **Mitigation:** Spill `raw_content` to BLAKE3-keyed chunk storage (`<=2 KiB` snippets in RAM), batch DB telemetry on one `mpsc(512)` writer over `ServerState.db`, emit explicit `resync_required` (`after_seq`) frames on lag, and paginate/cluster UI state (`>80` DAG nodes).
4. **Upstream Scholarly Rate-Limit Cliffs & Offline Promotion Deadlocks:** OpenAlex retired `&mailto=` in favor of API keys (`Bearer` / `api_key=`), `trust.rs:67` hits Crossref without `?mailto=` (concurrency=1 Public Pool with `30s` timeout stalls), Semantic Scholar enforces strict `1 RPS`, and `TrustScorer::lookup_for_promotion` (`trust.rs:97`) blocks air-gapped promotion via `TrustLookup::Unavailable`. **Mitigation:** Wire Clavis OpenAlex API key + Crossref `?mailto=`, enforce `governor` rate limiters + `3.5s` timeouts + `VoxDb` caching, and introduce `OfflineAttested` human sign-off for 100% standalone operation.
5. **Direct Publication `vox-doc-pipeline` Blocks, LaTeX Breakouts & Parallel `v7.4`/`v7.5` Collisions:** `vox-doc-pipeline` hard-bans `vox:skip` (`doctest.rs:77`) and ````mermaid` (`mermaid_gate.rs:30`) and requires `authorship_model` + `status: "research"`; `latex/escape.rs` crashes `pdflatex` on Unicode math/Greek (`≤`, `Δ`) and permits `%`-newline / `\end{verbatim}` breakouts; meanwhile `pipeline.rs`, `research_turn.rs`, `gui_ops/research.rs`, `review_flow.rs`, and `ResearchView.tsx` already exceed the `500`-LoC `arch/god_object` cap while `v7.4` (`Phases 431–436`) and `v7.5` (`Phases 441–446`) enforce Radix `data-slot` primitives, zero emojis/dingbats, `transport.ts` funneling, and `catalog.v1.yaml` discipline. **Mitigation:** Run `vox-doc-pipeline` in-memory pre-write, harden LaTeX escaping + `-no-shell-escape`, decompose new code into `<400`-LoC child modules/components, and honor all `v7.4`/`v7.5` UI and contract guards.

---

## Implications for Roadmap

Harmonizing the architectural dependency DAG with pitfall prevention yields **6 strictly sequenced phases (`Phases 447–452`)** under `--ws scientia-deep-research-v76`:

### Phase 447: Deterministic Before-Audit Baseline, L0/L1 Continuum Contracts & Additive Session Schema
**Rationale:** Empirical verification requires capturing an objective, frozen-cassette `"before"` baseline *before* altering runtime behavior—replacing `eval.rs`'s Goodhartable hardcoded `1.0` heuristics first—while landing pure L0/L1 types and additive `VoxDb` pagination tables needed by all downstream phases.
**Delivers:**
- `vox scientia v76-audit --stage before` (`crates/vox-cli-research/src/scientia_v76_audit.rs`) + frozen HTTP/search/scholarly replay cassettes recording baseline metrics (`contracts/reports/scientia-v76-before-baseline.v1.json`) across all live bugs (`research_scope` overwrite, `None => Novel` rate, `artifact_json` wipe, `Mock` verifier rate, KV-cache stability, IPC payload size, and `vox-gui` CLI-DB spawn count).
- L0 `ResearchContinuumMode`, `ResearchContinuumTier`, `EscalationReason`, `CragEscalationPolicy` (`vox-orchestrator-types::socrates_policy`) + `ResearchDomainMode::Scholarly`.
- L1 `SequencedResearchEvent` (`seq: u64` watermark + `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta`) and consolidated `NoveltyEvidenceBundle` (`vox-research-events`).
- Additive `VoxDb` schema migration (`research_session_sources`, cursor-paginated `list_session_claims_page` / `list_session_sources_page`, `continuum_tier` / `escalated_from` columns).
**Addresses:** `TS-09` (baseline half), foundational contracts for `TS-01..06`.
**Avoids:** Pitfall 7 (Goodharted `eval.rs:364` `1.0` multi-hop & index-only citation metrics, live-web drift) and L3 dependency cycles.

### Phase 448: Unified Light ↔ Deep Continuum Router, Socrates Escalation & KV-Cache-Safe Chat Harness
**Rationale:** Fixes the core routing and prompt-caching seams so Chat (`Loquela`), CLI, MCP, and `ResearchView` share one truthful 4-stage continuum before streaming UI layers attach.
**Delivers:**
- Orthogonal `research_mode` (`"off"|"auto"|"fast"|"deep"`) vs `research_scope` (`"local"|"web"|"both"`) + 4 domain modes (`General | CodeGen | Shopping | Scholarly`) wired end-to-end from `Loquela.tsx` / `App.tsx:1972` / `buildChatTurn.ts` → `ChatMessageParams` → `research_turn.rs` → `ResearchOrchestrator`.
- Reordered `message.rs` executing T1 Local Hybrid RAG + `SocratesGateOutcome` *before* `classify_research_intent_with_socrates`, distinguishing retrievable *epistemic* uncertainty (silent Light probe → warm-seeded `T1 -> T2 -> T3` CRAG escalation with zero duplicate URL fetches) from *specification* ambiguity (`QuestioningPolicy` cost-gated clarification).
- Strict day-stable KV-cache prefix enforcement: relocate volatile `AttentionBudget`, `$operating_mode`, `$active_file`, and `[WEB RESEARCH]` blocks exclusively into `turn_user_prompt` (`preamble_and_retrieval.rs`), keeping `build_system_prompt_with_skill` and MCP tool declarations byte-identical across turns.
**Uses:** `vox-orchestrator::socrates`, `vox-search::research` (`CragEscalationSignal`), `vox-research-shim`, `vox-orchestrator-mcp`.
**Addresses:** `TS-01`, `TS-02`, `TS-04`, `DF-01`.
**Avoids:** Pitfall 1 (KV-cache prefix invalidation), Pitfall 3a (pre-search Socratic interrogation loops), and Anti-Pattern 3 (`research_scope` overload).

### Phase 449: Extreme-Data Resumable Streaming, `std.frame` Chunk Cursors & Virtualized Epistemic Canvas
**Rationale:** Once continuum routing emits `SequencedResearchEvent` deltas, decouple multi-wave execution from the `180s` synchronous Chat wall, bound backend/frontend memory to `O(window_size)`, and wire real wave/cost/tabular streaming.
**Delivers:**
- Durable async session handles + `GET /api/v2/research/sessions/{id}/stream?from_seq=` SSE & `/v1/ws` `"research.session.delta"` topic with explicit `resync_required` lag recovery, plus cursor-paginated `/claims` and `/sources` (`?summary_only=true` omitting multi-MB `artifact_json`).
- Bounded `mpsc::channel(512)` batched DB event persistence reusing `ServerState.db` (eliminating `scientia_feed.rs:59` 5s `VoxDb::connect_default()` polling and per-event `tokio::spawn` lock storms), BLAKE3-chunked externalization of `ResearchHit.raw_content`, and real `UsageSnapshot` → `scientia_cost_telemetry` recording.
- Out-of-core `std.frame` (`Polars 2.1.0` Arrow `StreamReader` + pure-Rust `csv` fallback) bounded `frame_chunk` SSE streaming + statistical summaries (`DF-02`).
- Live `MultiWaveProgressTimeline.tsx` (replacing hardcoded `currentWave={1}`), backend-hydrated hierarchical `ResearchDagCanvas.tsx` with wave-cluster virtualization (`nodes.length > 80`), and `@tanstack/react-virtual` claim/dataframe tables built in `<400`-LoC child components using `v7.4` Radix `data-slot` + `<VoxIcon>` primitives.
**Addresses:** `TS-03`, `DF-02`, `DF-06` (research half).
**Avoids:** Pitfall 2 (`180s` timeout, `BroadcastStream` `Lagged` drops, SQLite `BUSY`, V8 heap OOM) and Pitfall 6 (`v7.4`/`v7.5` UI & god-object collisions).

### Phase 450: Calibrated 3-Tier Verifier, Two-Stage Novelty × Utility Engine & Continuous Discovery
**Rationale:** Before automating publication preparation in Phase 451, claim verification, prior-art novelty, and worthiness scoring must be epistemically honest, rate-governed, Clavis-compliant, and 100% functional offline.
**Delivers:**
- **3-Tier Claim & Citation Verifier (`TS-05`):** Tier 0 `aho-corasick`/`strsim`/numeric pre-filter (`0` LLM calls) → Tier 1 SciFact entailment (`Support | Contradict | NotEnoughInfo | Abstain` enforcing `abstain_threshold` via `vox_actor_runtime::llm::infer_with_retry`) → Tier 2 `3×` resampling *only* in uncertain margin `[abstain, 0.80]`; delete `MiniCheckVerifier::from_env()` (`Mock` gated to `#[cfg(test)]`) and migrate `VOX_MINICHECK_ENDPOINT` to Clavis/`vox-config`.
- **L3 `vox-scientia::embedder::CachedLlmEmbedder` & Fused `AtomicNoveltyScorer` (`TS-06`):** Lift `CachedLlmEmbedder` from `vox-cli-research` to `vox-scientia`; fuse `max_semantic_score` + calibrated `max_lexical_score` + `vox-similarity::LshIndex` (fixing `None => Novel` to return calibrated lexical verdict or `InsufficientEvidence`); remediate banned `sha2` in `scientia_prior_art.rs:11` via `vox-crypto`.
- **Rate-Governed & Offline-Resilient Scholarly Adapters:** Clavis OpenAlex API key (`Bearer`/`api_key=`), Crossref Polite Pool `?mailto=` (`3.5s` timeout, concurrency `3`), Semantic Scholar `1 RPS` `governor`, `VoxDb` 14-day response cache, and `OfflineAttested` promotion mode (eliminating `trust.rs:97` air-gapped deadlocks).
- **Two-Stage Novelty × Utility (`WorthinessSignalsV2`) & Continuous Discovery (`DF-03`, `DF-04`):** Require both prior-art novelty and replayable/empirical utility before inbox surfacing; upgrade `SocratesTelemetryProducer` (clustering high-entropy abstentions/contradictions) and `vox scientia scout`.
**Addresses:** `TS-05`, `TS-06`, `DF-03`, `DF-04`.
**Avoids:** Pitfall 3b/c (`3×` verifier cost blowup & `Mock` call-site divergence), Pitfall 4 (OpenAlex/Crossref/S2 `429` cliffs & offline promotion deadlock), and Anti-Pattern 2 (`None => Novel`).

### Phase 451: Full DB-Hydrated Direct Publication (`> Nanopublication`) Flywheel, Provenance Quarantine & Zero-`VoxDb` Studio
**Rationale:** Converges verified claims, calibrated novelty bundles, replay proofs, and multi-target scholarly rendering into a seamless pre-review state machine halted strictly at human `ApprovalToken` sign-off, while eliminating the last `VoxDb` subprocess leaks in `vox-gui`.
**Delivers:**
- **Bug Fixes & Provenance Quarantine:** Fix `write_research_doc` (`gui_ops/research.rs:715`) so publishing an SSOT never overwrites `artifact_json` with `"{}"`; quarantine unattested `save_memory` syntheses (`origin = "untrusted_synthesis"`) so `web_gather.rs` never reloads prior AI prose at `trust_score: 1.0` (blocking **Epistemic Ouroboros**); fix `discovery_bridge.rs` Gap 2 so 1-click promotion creates real `publication_manifests` rows.
- **DB-Hydrated Direct Publication (`> Nanopublication`) & `flywheel_advance`:** Idempotent `advance_publication_flywheel` orchestrating promote → `MiniCheck` claims → `novelty_fetch` → measured `publication-replay-execute` writeback → DB-hydrated `docs/src/architecture/*-research-2026.md` SSOTs (validated *in-memory* via `vox-doc-pipeline`: `status: "research"`, `training_eligible: false`, non-empty `authorship_model`, 40–120 word `kb:summary`, `vox-diagram` fences, compiler-checked ````vox` blocks with `0` `vox:skip`) → DB-hydrated IMRaD + hardened venue LaTeX (Unicode math/Greek transliteration, `%`-newline/URL/`\end{verbatim}` sanitization, `-no-shell-escape` confinement) + pure-Rust `flate2` arXiv `.tar.gz` handoff + HTML Finding Pages + RO-Crate 1.2 → mandatory human `ApprovalToken` + `CriticGate` stop → hydrated `nanopub_build` (`RA` Trusty URI + real `novelty_verdict` + `prior_art_uris` via per-user Clavis RSA key) + Zenodo / Software Heritage / Crossref Deposit / OpenReview staging & submission.
- **Zero-`VoxDb` Unified Scientia Studio:** Add `/api/v2/scientia/publications/{id}/{claims,venue-recommend,novelty-fetch,flywheel-advance}`; remove `PublicationsView.tsx:237-254` `execute_command` CLI-DB spawns; mount `WorthinessRadarPanel`, `NoveltyEvidencePanel`, and `DirectPublicationDrawer` exclusively over `voxTransport`.
**Addresses:** `TS-07`, `TS-08`, `DF-05`, `DF-06` (studio half).
**Avoids:** Pitfall 5 (`artifact_json` wipeout, `vox-doc-pipeline` rejections, LaTeX breakouts, Ouroboros self-citation) and Anti-Patterns 1 & 5 (`vox-gui` DB lock contention & `ApprovalToken` bypass).

### Phase 452: Paired After-Audit Closure, Inline Before/After Diff Inspector & Quarantined Lesson Flywheel
**Rationale:** Closes the empirical verification loop by re-running Phase 447's frozen cassettes alongside stratified live canaries, surfacing inline diffs in Axis GUI, and distilling verified improvements/failure modes into workspace memory without rubric overfitting.
**Delivers:**
- `vox scientia v76-audit --stage after --compare contracts/reports/scientia-v76-before-baseline.v1.json` producing `contracts/reports/scientia-v76-after-closure.v1.json` with `vox_eval::paired_ni` confidence intervals, cross-family blinded pairwise judge scoring (distinct model family from `synthesis_model`, randomized A/B order), bridge-entity multi-hop recall, claim-entailment F1, Socrates ECE, `0%` false-`Novel` rate, KV-cache hit verification, and `0` `vox-gui` direct/CLI DB handles.
- Reusable `BeforeAfterAuditInspector` React component (`similar` inline word/line hunks + `@tanstack/react-virtual` + 6-axis delta radar) across `ResearchView`, `ScientiaSurface`, and `TaskDiffDialog`.
- Guarded "Next Lesson" extraction via `vox_actor_runtime::llm` → `SanitizedText` → `kb_findings` (`Untrusted` quarantine requiring `>=2`-corroboration in `kb_inbox.rs` or human attestation) → `.proposed` `vox-effort-route` detectors and MENS Auto-DPO preference pairs, plus inline Before/After closure table in `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` passing `vox-doc-pipeline --lint-only`.
**Addresses:** `TS-09` (closure half), `DF-07`.
**Avoids:** Pitfall 6 (shared `catalog.v1.yaml` / `audits.ts` regressions) and Pitfall 7 (same-family judge self-preference, position bias, and unquarantined prompt-lesson drift).

### Phase Ordering Rationale

- **Baseline First, Closure Last (`447` → `452`):** Locking frozen replay cassettes and L0/L1 types in Phase 447 prevents moving-goalpost metrics and unblocks parallel-ready L3 work without touching `layers.toml` edges.
- **Routing Before Streaming, Verification Before Publication (`448` → `449` → `450` → `451`):** Streaming delta frames (`449`) depend on continuum event emission (`448`); automated Direct Publication (`451`) is unsafe until calibrated claim grounding, rate-governed/offline-honest novelty, and `WorthinessSignalsV2` utility filters (`450`) eliminate mock word-overlap and `None => Novel` false positives.
- **Parallel Workstream Hygiene Across Every Phase:** Keeping new files `<400` LoC, isolating `catalog.v1.yaml` + `operations-sync --write` to single-row `[SEQUENTIAL]` steps, and enforcing `v7.4` UI gates (`voxTransport`, Radix `data-slot`, `<VoxIcon>`, `<BentoScrollRegion>`) avoids merge conflicts with active `v7.4` and `v7.5` workstreams.

### Research Flags

Phases needing targeted contract inspection during `/gsd-plan-phase`:
- **Phase 450 (OpenAlex Auth & Calibration Thresholds):** Verify exact Clavis `SecretId` naming/alias migration (`VoxOpenAlexEmail` vs new `VoxOpenAlexApiKey` in `crates/vox-secrets/src/spec/registry/scholarly.rs` + `vox ci secrets-parity`) and calibrate the lexical-to-semantic projection curve against `wiremock` SciFact/OpenAlex fixtures.
- **Phase 451 (Venue `.sty` Packaging & Rubric Narrative Contract):** Confirm license-clean bundling vs TeX Live system lookup for `neurips_2025`/`icml2025`/`acmart`/`jmlr2e` class headers in `crates/vox-scientia/src/manuscript/latex/render.rs`, while preserving the existing `forbidden_sections_appear_as_section_with_todo_comments_only` (`render.rs:374`) human-narrative boundary (machine-verified empirical tables/methods populated automatically; author narrative slots explicitly demarcated for human attestation).

Phases using established in-repo patterns (skip extra external research):
- **Phase 447 & Phase 448:** Pure L0/L1 Rust types, additive Turso migrations, and `turn_user_prompt` KV-cache isolation follow well-tested `vox-db` and `vox-orchestrator-mcp` patterns.
- **Phase 449 & Phase 452:** `seq`/`after_seq` SSE cursor recovery mirrors `v7.5` `TERM-PTY-04`, `std.frame` Arrow `StreamReader` already exists in `vox-python-bridge/src/frame_columns.rs`, and `vox_eval::paired_ni` + `similar` inline diffing are already in tree.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| **Stack** | **HIGH** | 100% of crates and npm packages verified in `Cargo.toml`, `layers.toml`, and `crates/vox-gui/ui/package.json`; C++ `ort`/`fastembed`/`hnswlib` hazard verified against `AGENTS.md §Build-toolchain invariant`. |
| **Features** | **HIGH** | Grounded in the 10-gap repo audit (`scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`) and benchmarked against 2025–2026 frontier systems (FutureHouse, AI Co-Scientist, DeepResearch Bench, DRACO). |
| **Architecture** | **HIGH** | Every crate boundary (`L0` `vox-orchestrator-types`, `L1` `vox-research-events`, `L2` `vox-similarity`, `L3` `vox-scientia::embedder`), `VoxDb` lock rule, and live bug line number verified directly in source. |
| **Pitfalls** | **HIGH** | Seven concrete failure modes verified against live Rust/TS call sites (`App.tsx:1972`, `message.rs:142`, `gui_ops/research.rs:715`, `novelty.rs:119`, `minicheck.rs:76`, `pipeline.rs:1164`, `eval.rs:364`) and 2025–2026 upstream API shifts. |

**Overall confidence:** **HIGH**

### Gaps to Address During Phase Planning

- **OpenAlex Auth Transition (`STACK.md` vs `PITFALLS.md` reconciliation):** Keep `VoxOpenAlexEmail` as a backward-compatible fallback parameter, but add Clavis `SecretId` support for OpenAlex API keys (`Authorization: Bearer` / `api_key=`) and enforce `governor` `3 RPS` anonymous / `20 RPS` keyed rate limits in Phase 450 so live novelty sweeps never hit OpenAlex's post-`mailto` `$0.10/day` anonymous cliff.
- **Windows vs Unix `pdflatex` Availability in CI:** Never require a local TeX Live installation to build or pass unit tests; verify LaTeX escaping/AST structural invariants deterministically in pure Rust (`insta` + adversarial Unicode/URL property tests) and invoke `pdflatex -no-shell-escape` only as an optional environment-detected smoke check.
- **Active `v7.4` / `v7.5` File Contention:** Before executing UI tasks in Phases 448, 449, 451, and 452, run `git log -n 5` (orchestrator check) / read current HEAD of `ResearchView.tsx`, `ScientiaSurface.tsx`, `PublicationsView.tsx`, `ChatTranscript.tsx`, and `contracts/operations/catalog.v1.yaml` so thin 5–15 line mounts align cleanly with landed `v7.4`/`v7.5` commits.

---

## Sources

### Primary (HIGH confidence — Verified Vox Codebase & Governance SSOTs)
- [STACK.md](./STACK.md), [FEATURES.md](./FEATURES.md), [ARCHITECTURE.md](./ARCHITECTURE.md), [PITFALLS.md](./PITFALLS.md) — Milestone v7.6 workstream research reports (`2026-11-14`).
- `Cargo.toml`, `docs/src/architecture/layers.toml`, `docs/src/architecture/where-things-live.md`, `docs/src/architecture/cryptography-ssot-2026.md`, and `docs/src/contributors/documentation-governance.md` — Normative workspace dependency pins, L0–L5 DAG budgets, crypto/LLM boundaries, and `vox-doc-pipeline` KB facet/doctest/`vox-diagram` gates.
- `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`, `docs/src/architecture/deep-research-implementation-divergence-audit-2026-08-01.md`, and `docs/src/architecture/scientia-micropublication-ssot-and-surfacing-design-2026.md` — Grounded 10-gap matrix, call-site divergence root causes, and 4-layer `ApprovalToken` / per-user Clavis RSA nanopub design.
- Live L0–L5 source seams inspected: `crates/vox-orchestrator-types/src/socrates_policy/`, `crates/vox-research-events/src/events.rs`, `crates/vox-similarity/src/{signature,index}.rs`, `crates/vox-actor-runtime/src/llm/`, `crates/vox-search/src/{research,trust,openalex,kb_inbox}.rs`, `crates/vox-scientia/src/{review_flow.rs,claim_extractor/minicheck.rs,inspect_bridge/novelty.rs,manuscript/,nanopub/}`, `crates/vox-publisher/src/{scientia_prior_art,archive_run,scholarly}.rs`, `crates/vox-research-shim/src/research/{orchestrator/pipeline.rs,verifier.rs,discovery_bridge.rs}`, `crates/vox-orchestrator-mcp/src/{chat_tools/,gui_ops/,http_gateway/scientia_feed.rs}`, `crates/vox-cli-research/src/{eval.rs,scientia_phase_handlers.rs}`, and `crates/vox-gui/{src/commands/,ui/src/}`.

### Secondary (HIGH/MEDIUM confidence — 2025–2026 External Specifications & Prior Art)
- **Scholarly & Archival API Contracts (2025–2026):** OpenAlex API key authentication & `is_retracted` Work schema (`docs.openalex.org`); Crossref REST v1 Retraction Watch integration & Polite Pool `?mailto=` rules (`api.crossref.org`); Semantic Scholar Academic Graph v1 `SPECTER2` batch endpoint & `1 RPS` quota (`api.semanticscholar.org`); Zenodo `/api/deposit/depositions` `prereserve_doi` (`developers.zenodo.org`); OpenReview API v2 (`api2.openreview.net`); Nanopublication Trusty-URI `RA` spec & `nanopub 0.2` testsuite (`nanopub.net`, `docs.rs/nanopub`); RO-Crate 1.2 (`researchobject.org`).
- **Agentic Deep Research, Verification & Evaluation Literature (2024–2026):** Yan et al. (*CRAG*), Jeong et al. (*Adaptive-RAG*), Tang et al. (*MiniCheck*, EMNLP), Song et al. (*VeriScore*), FutureHouse (*Crow, Falcon, Owl, Finch, Robin*), Google DeepMind (*Towards an AI Co-Scientist*), Sakana AI (*The AI Scientist-v2* superficial-novelty & sandboxing lessons), *DeepResearch Bench* (**RACE** & **FACT**), Perplexity *DRACO*, and Anthropic/OpenAI/DeepSeek byte-exact prompt KV-caching specifications.

---
*Research completed: 2026-11-14*
*Ready for roadmap: yes (`Phases 447–452`, `--ws scientia-deep-research-v76`)*
