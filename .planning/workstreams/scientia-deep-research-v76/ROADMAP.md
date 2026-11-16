# Roadmap: Milestone v7.6 (`scientia-deep-research-v76`) — Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel

**Workstream:** `.planning/workstreams/scientia-deep-research-v76/`
**Scope:** Unify Chat RAG (`Loquela`), multi-wave Deep Research (`vox-research-shim`), out-of-core streaming (`vox-orchestrator-mcp`, `std.frame`), calibrated Socrates/NLI/novelty verification (`vox-scientia`, `vox-publisher`, `vox-search`, `vox-similarity`), continuous discovery, and human-gated Direct Publication (`> nanopublication`) + Axis Studio (`vox-gui`, `vox-cli-research`) across `Phases 447–452` (`26/26` requirements). Zero new third-party Rust crates in `Cargo.toml`/`Cargo.lock`, zero new npm packages in `package.json`, zero `layers.toml` / `crate-edges` violations, zero `VoxDb` handles or child CLI-DB subprocesses in `vox-gui`, `<400`-LoC child module/component decomposition, and 100% model-agnostic LLM routing via `vox_actor_runtime::llm`.

## Phase Checklist

- [x] **Phase 447: Deterministic Before-Audit Baseline, L0/L1 Continuum Contracts & L3 Shared Embedder** (`AUDT-01`, `DISC-01`) — completed 2026-11-15
- [x] **Phase 448: Unified Light ↔ Deep Continuum Router, Socrates Escalation & KV-Cache-Safe Chat Harness** (`CONT-01..04`) — completed 2026-11-15
- [x] **Phase 449: Extreme-Data Resumable Streaming, `std.frame` Chunk Cursors & Virtualized Epistemic Canvas** (`STRM-01..04`) — completed 2026-11-15
- [x] **Phase 450: Calibrated 3-Tier Verifier, Two-Stage Novelty × Utility Engine & Continuous Discovery** (`VERI-01..04`, `DISC-02..04`) — completed 2026-11-15
- [x] **Phase 451: Full DB-Hydrated Direct Publication (`> Nanopublication`) Flywheel & Zero-`VoxDb` Studio** (`PUBL-01..04`, `STUD-01..02`) — completed 2026-11-15
- [x] **Phase 452: Paired After-Audit Closure, Inline Before/After Diff Inspector & Quarantined Lesson Flywheel** (`AUDT-02..04`) — completed 2026-11-15
- [ ] **Phase 453: Deep Research Reasoning-Budget, Citation Audit & Daemon DB Proxy** (`LIVE-09..12`)
- [ ] **Phase 454: Causal Swarm Event Stamping & Persistent Sequence Board** (`CONC-01..04`)
- [ ] **Phase 455: Vox Concilium & Populi Unified Swarm Observatory GUI** (`CONC-05..08`)

---

### Phase 447: Deterministic Before-Audit Baseline, L0/L1 Continuum Contracts & L3 Shared Embedder

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/447-before-audit-baseline-and-continuum-contracts/`
**Goal**: Lock an objective, deterministic frozen-cassette Before-audit baseline across all 6 evaluation axes and live bug counters prior to behavioral runtime changes, while landing pure L0/L1 continuum/event contracts, consolidated `NoveltyEvidenceBundle`, L3 `vox-scientia::embedder::CachedLlmEmbedder`, and additive `VoxDb` cursor-pagination tables.
**Depends on**: None (continues sequentially after Milestone `v7.5` `Phase 446`)
**Requirements**: AUDT-01, DISC-01
**Success Criteria** (what must be TRUE):

  1. Running `cargo run -p vox-cli -- scientia v76-audit --stage before` executes deterministically offline against frozen HTTP/search/scholarly replay cassettes (zero live network calls, replacing `eval.rs:364` hardcoded `1.0` multi-hop stubs) and writes `contracts/reports/scientia-v76-before-baseline.v1.json` recording pre-v7.6 metrics across all 6 axes (**performance**, **quality**, **verification**, **integration**, **Socrates calibration**, and **novelty detection**) plus live bug reproduction counters (`research_scope` overwrite, `None => Novel` rate, `artifact_json` wipe, `Mock` verifier fallback rate, KV-cache prefix stability, IPC payload bytes, and `vox-gui` CLI-DB spawn count).
  2. Pure L0 continuum types (`ResearchContinuumMode::{Off, Auto, Fast, Deep}`, `ResearchContinuumTier`, `EscalationReason`, `CragEscalationPolicy` in `vox-orchestrator-types::socrates_policy` + `ResearchDomainMode::Scholarly`) and L1 wire envelopes (`SequencedResearchEvent` with monotonic `seq: u64` watermarks and `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta` in `vox-research-events`) compile and round-trip cleanly with zero async/IO dependencies in L0/L1.
  3. `NoveltyEvidenceBundle` is consolidated as one canonical L1 schema in `vox-research-events`, and `CachedLlmEmbedder` is lifted from L5 `vox-cli-research` to L3 `vox-scientia::embedder` (`vox_actor_runtime::llm::llm_embed` + deterministic pure-Rust local fallback + `VoxDb` `scientia_embedding_cache`, with thin re-export in `vox-cli-research`), introducing `0` new third-party crates in `Cargo.toml`/`Cargo.lock` and `0` violations in `cargo run -p vox-arch-check` or `contracts/ci/crate-edges.allow.v1.json`.
  4. Additive `VoxDb` schema migrations create indexed `research_session_sources` and add `continuum_tier` / `escalated_from` columns on `research_sessions`, with cursor-paginated `list_session_claims_page` and `list_session_sources_page` verified by round-trip unit tests in `<400`-LoC child modules.

**Plans**: 3/3 plans complete (`447-01`, `447-02`, `447-03`)

### Phase 448: Unified Light ↔ Deep Continuum Router, Socrates Escalation & KV-Cache-Safe Chat Harness

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/448-unified-continuum-socrates-and-kv-cache/`
**Goal**: Replace disconnected Chat/Research routing forks and prompt-cache-busting system mutations with one orthogonal 4-stage research continuum (`T0 Direct` → `T1 Local Hybrid RAG` → `T2 Quick 1-Wave Web` → `T3 Deep Multi-Wave Adversarial`) across all 4 domain modes while keeping day-stable Chat KV-cache prefixes byte-identical.
**Depends on**: Phase 447
**Requirements**: CONT-01, CONT-02, CONT-03, CONT-04
**Success Criteria** (what must be TRUE):

  1. Operators can independently select `research_mode` (`Off | Auto | Fast | Deep`) and `research_scope` (`Local | Web | Both`) across Chat (`Loquela`, `/research`), CLI (`vox research`), MCP (`vox_chat_message`), and `ResearchView`, eliminating the `App.tsx:1972` `research_scope` overload and `research_turn.rs:628` `waves: 1` hardcode (`Off` makes `0` external calls; `Fast` pins `<=1`-hop retrieval; `Deep` executes multi-wave synthesis).
  2. Both Light (`<=1`-hop) and Deep (`>1`-wave, `waves: 1..=5`, `site_scope`) runs execute across all 4 first-class domain modes — `General`, `CodeGen` (compiler/sandbox-verified probes), `Shopping` (weighted decision matrices), and `Scholarly` (prior-art/DOI synthesis) — across CLI, MCP, Chat, and GUI in `<400`-LoC child modules with `0` `vox-arch-check` violations.
  3. In `Auto` mode, `message.rs` runs `T1` Local Hybrid RAG + `SocratesGateOutcome` *before* `classify_research_intent_with_socrates`, silently escalating `T1 -> T2 -> T3` on retrievable epistemic uncertainty (`crag_quality < 0.68` or `snippet_conflict_score >= 0.25`) with warm-seeded hits (`0` duplicate URL fetches) while emitting at most `1` cost-gated clarifying question only on specification ambiguity (`information_gain_bits >= 0.25` vs `CLARIFICATION_INTERRUPT_COST_MS`).
  4. Switching `research_mode`, `domain_mode`, `$active_file`, `$operating_mode`, or dynamic `AttentionBudget` across Chat turns keeps `build_system_prompt_with_skill` and ordered MCP tool declarations `100%` byte-identical within a calendar day, injecting all turn-volatile research context, wave summaries, and Socrates posture strictly via `turn_user_prompt`.

**Plans**: 1/1 plan complete (`448-01`)

### Phase 449: Extreme-Data Resumable Streaming, `std.frame` Chunk Cursors & Virtualized Epistemic Canvas

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/449-extreme-data-streaming-and-epistemic-canvas/`
**Goal**: Decouple multi-wave Deep Research from the `180s` synchronous Chat timeout wall, bound daemon/DB/UI memory to `O(window_size)` via watermarked SSE/WS delta streams, externalized BLAKE3 chunks, and `std.frame` Arrow pagination, and mount live virtualized wave/DAG/tabular views in Axis GUI.
**Depends on**: Phase 448
**Requirements**: STRM-01, STRM-02, STRM-03, STRM-04
**Success Criteria** (what must be TRUE):

  1. Multi-wave research executes via durable async session handles decoupled from the `180s` Chat timeout wall and streams monotonic `SequencedResearchEvent` (`seq: u64`) deltas over `GET /api/v2/research/sessions/{id}/stream?from_seq=` SSE and `/v1/ws` (`"research.session.delta"`), recovering cleanly from subscriber lag via explicit `resync_required` + cursor reconciliation (`<64 KiB` peak wire frame).
  2. Daemon telemetry batches writes over a single bounded `mpsc(512)` worker sharing `ServerState.db` (eliminating `scientia_feed.rs` `5s` `VoxDb::connect_default()` polling and per-event `tokio::spawn` lock storms), spills `ResearchHit.raw_content` to BLAKE3-keyed chunk storage (`<=2 KiB` snippets in RAM), exposes `?summary_only=true` + cursor-paginated `/claims` and `/sources`, and persists real `UsageSnapshot` USD costs into `scientia_cost_telemetry`.
  3. Out-of-core `std.frame` (`Polars 2.1.0` Arrow `StreamReader` with pure-Rust `csv` standalone fallback) processes `100K–1M+` row datasets in `O(batch_size)` RAM and streams bounded `frame_chunk` slices plus statistical summaries over SSE without serializing raw full tables into LLM prompts.
  4. `MultiWaveProgressTimeline.tsx` renders live multi-wave progress (replacing hardcoded `currentWave={1}`), `ResearchDagCanvas.tsx` hydrates hierarchical `Query -> Wave Subquery -> Source -> Claim -> Verdict/Contradiction` graphs with automatic wave-cluster virtualization above `80` nodes, and `@tanstack/react-virtual` claim and dataframe tables scroll smoothly in Chat and `ResearchView` (`0` new npm packages, `<400`-LoC Radix `data-slot` components, `0` raw emojis/dingbats, `0` `VoxDb` handles in `vox-gui`).

**Plans**: 1/1 plan complete (`449-01`)

### Phase 450: Calibrated 3-Tier Verifier, Two-Stage Novelty × Utility Engine & Continuous Discovery

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/450-calibrated-verifier-novelty-and-discovery/`
**Goal**: Eliminate mock word-overlap verification, `None => Novel` false positives, Epistemic Ouroboros self-ingestion, and air-gapped scholarly deadlocks by shipping a calibrated 3-tier NLI claim/citation verifier, rate-governed/Clavis-backed hybrid novelty scoring, and two-stage Novelty × Utility continuous discovery.
**Depends on**: Phase 447 (and integrates with Phases 448–449 continuum & cost telemetry)
**Requirements**: VERI-01, VERI-02, VERI-03, VERI-04, DISC-02, DISC-03, DISC-04
**Success Criteria** (what must be TRUE):

  1. Claim verification executes the 3-tier calibrated cascade — Tier 0 deterministic `aho-corasick`/`strsim`/numeric/`statcheck` pre-filter (`0` LLM calls), Tier 1 SciFact entailment (`Support | Contradict | NotEnoughInfo | Abstain` enforcing `abstain_threshold`) via `vox_actor_runtime::llm::infer_with_retry`, and Tier 2 `3×` resampling restricted strictly to `[abstain, 0.80]` — while gating `MiniCheckBackend::Mock` strictly to `#[cfg(test)]`, migrating `VOX_MINICHECK_ENDPOINT` onto Clavis/`vox-config`, migrating `scientia_prior_art.rs` off `sha2` onto `vox-crypto`, and passing `vox audit code` (`llm_provider_call`, `vox/crypto/banned-crate-import`) + `vox ci secret-env-guard` with `0` errors.
  2. Post-synthesis citation auditing verifies quoted spans against retrieved source chunks, makes `ClaimVerdict::Contradicted` reachable so `contradiction_penalty` fires in `gate.rs`, weights `ResearchHit.trust_score` with Crossref Retraction Watch (`update-to` hard-zero) + OpenAlex venue/peer-review signals, and stamps autonomous syntheses/memory notes in `VoxDb` with `untrusted_synthesis` (`D-06`) so `ResearchScope::Both` (`web_gather.rs`) never re-ingests prior AI prose at `trust_score: 1.0`.
  3. `AtomicNoveltyScorer` fuses Stage-1 `vox-similarity` (`LshIndex`/MinHash/shingle Jaccard) and Stage-2 dense cosine similarity with `ChronoFilter` and `EvidenceConflict`, returning `NoveltyVerdict::InsufficientEvidence` (`0%` false-`Novel` rate) whenever prior-art `query_traces` fail HTTP health checks or embeddings are absent, backed by Clavis OpenAlex API-key auth (`Bearer`/`api_key=`), Crossref Polite `?mailto=`, `governor` GCRA rate limiters (`Semantic Scholar: 1 RPS`, `3.5s` timeouts), and an explicit human-signed `OfflineAttested` path for standalone/air-gapped promotion.
  4. Continuous discovery (`vox scientia scout`, `commit_watcher`, and `SocratesTelemetryProducer` high-entropy abstention/contradiction clustering) enforces the two-stage **Novelty × Utility** (`WorthinessSignalsV2`) filter, requiring both prior-art novelty and empirical utility proof (measured replay digest, benchmark delta, compiler-verified probe, or corroborated epistemic gap) before surfacing candidates to `scientia_discovery_inbox`.

**Plans**: 1/1 plan complete (`450-01`)

### Phase 451: Full DB-Hydrated Direct Publication (`> Nanopublication`) Flywheel & Zero-`VoxDb` Studio

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/451-direct-publication-flywheel-and-scientia-studio/`
**Goal**: Deliver an idempotent, zero-manual-JSON `flywheel_advance` pipeline from research/discovery candidates to `vox-doc-pipeline`-clean Architecture SSOTs, hardened venue IMRaD/LaTeX, arXiv `.tar.gz`, RO-Crate 1.2, HTML Finding Pages, signed Trusty-URI nanopubs, and archival receipts behind a 4-layer human `ApprovalToken` gate, unified in a zero-`VoxDb` Axis Scientia/Publications Studio.
**Depends on**: Phases 448, 449, and 450
**Requirements**: PUBL-01, PUBL-02, PUBL-03, PUBL-04, STUD-01, STUD-02
**Success Criteria** (what must be TRUE):

  1. One-click `promote_candidate_to_publication` creates canonical `publication_manifests` rows preserving `EventCandidateClass` + `novelty_evidence_bundle_id` (fixing orphan `finding-<uuid>` IDs), and external emission remains gated behind a 4-layer content-digest-bound human `ApprovalToken` + `CriticGate` (`no_network_publish_symbol_in_review_flow` invariant passing).
  2. Publishing an Architecture SSOT (`render_doc_draft` / `write_research_doc`) hydrates real verified claims, citations, and probes from `VoxDb` without wiping `artifact_json` (`gui_ops/research.rs:715` regression test passing) and validates in-memory against `vox-doc-pipeline` (`topic`, `domain`, `authorship`, `authorship_model`, `status: "research"`, 40–120 word `<!-- kb:summary -->`, `vox-diagram` fences, `0` `vox:skip`, `0` ````mermaid`) before writing to `docs/src/architecture/`.
  3. Idempotent `flywheel_advance` + `publication-replay-execute` run directly from `publication_id` with **zero manual JSON scaffold files** (`assemble_scaffold_from_db` + `assemble_finding_page_from_db`), writing replay reproducibility digests into `publication_manifests.metadata_json.scientia_evidence` / `WorthinessInputs`, reporting real confidences, reply windows, and per-stage USD costs in `vox scientia dashboard` / `vox scientia cost`, and producing Unicode/comment/URL-hardened IMRaD LaTeX (`-no-shell-escape`), deterministic pure-Rust `flate2` arXiv `.tar.gz` bundles, RO-Crate 1.2 packages, interactive HTML Finding Pages, Clavis-per-user RSA `RA` Trusty-URI nanopublications (`Nanopub::check()` hydrated with real `novelty_verdict` + `prior_art_uris`), and Zenodo/OpenReview archival receipts.
  4. `PublicationsView.tsx`, `ScientiaSurface.tsx`, and `DiscoverySurface.tsx` route `100%` of queries and mutations through `crates/vox-gui/ui/src/transport.ts` (`/api/v2/scientia/*` over `BackendState`) with `0` persistent or transient `VoxDb` handles and `0` `execute_command` CLI-DB spawns in `vox-gui`, allowing operators to triage `DiscoveryInbox`, inspect 6-axis `WorthinessSignalsV2` & `NoveltyEvidencePanel` breakdowns, preview/advance drafts, mint `ApprovalToken` sign-offs, and trigger archival via `<400`-LoC Radix `data-slot` components with `0` raw emojis/dingbats.

**Plans**: 1/1 plan complete (`451-01`)

### Phase 452: Paired After-Audit Closure, Inline Before/After Diff Inspector & Quarantined Lesson Flywheel

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/452-after-audit-closure-diff-inspector-and-lessons/`
**Goal**: Prove empirical improvement across all 6 evaluation axes against Phase 447's frozen baseline via `vox_eval::paired_ni`, surface inline Before/After run and report diffs in Axis GUI, and extract `SanitizedText`-scrubbed, quarantined "next lessons" into `VoxDb`, `vox-effort-route`, and MENS Auto-DPO without unreviewed SSOT mutation.
**Depends on**: Phases 447, 448, 449, 450, and 451
**Requirements**: AUDT-02, AUDT-03, AUDT-04
**Success Criteria** (what must be TRUE):

  1. Running `cargo run -p vox-cli -- scientia v76-audit --stage after --compare contracts/reports/scientia-v76-before-baseline.v1.json` executes the identical frozen-cassette corpus plus stratified live trials with cross-family blinded evaluation, emits `contracts/reports/scientia-v76-after-closure.v1.json` and `vox audit`-compatible JSONL (`schema_version` + per-finding discriminator), and proves `vox_eval::paired_ni` superiority / zero regression across all 6 axes (`0%` false-`Novel` rate, `100%` calendar-day KV-cache prefix invariance, `0` `vox-gui` DB handles).
  2. Operators can inspect paired Before vs. After run, claim, report, and 6-axis metric differences inline across `ResearchView`, `ScientiaSurface`, and Chat turn receipts via `<BeforeAfterAuditInspector />` built with `similar` inline diff hunks, `@tanstack/react-virtual`, and `<400`-LoC Radix `data-slot` components (`0` new Cargo/npm deps, `0` raw emojis/dingbats).
  3. Post-audit and post-research synthesis automatically extracts structured, `SanitizedText`-scrubbed **"next lessons"** via `vox_actor_runtime::llm` into `VoxDb` (`kb_findings` quarantined as `Untrusted` until `>=2`-run/domain corroboration in `kb_inbox`, `.proposed` enforcement rules via `vox-effort-route`, and verified repair/grounding pairs for MENS Auto-DPO) with `0` unreviewed mutations to `AGENTS.md` or canonical `docs/src/architecture/` SSOTs.
  4. `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` is updated with the measured inline Before → After closure table and passes `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md`, while `cargo run -p vox-arch-check` and `vox audit code` pass with `0` errors.

**Plans**: 1/1 plan complete (`452-01`)

### Phase 453: Deep Research Reasoning-Budget, Citation Audit & Daemon DB Proxy

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/453-deep-research-reasoning-budget-separation-citation-scoped-au/`
**Goal**: Close the remaining live deep-research and Scientia accuracy/concurrency bottlenecks uncovered during the November 2026 parallel CLI audit — separating reasoning vs. visible synthesis budgets, scoping citation precision to cited `[N]` spans, stripping Markdown/frontmatter in `AtomicDecomposer`, auto-projecting novelty overlap into discovery scans, and auto-proxying locked `VoxDb` CLI calls over daemon IPC.
**Depends on**: Phase 452
**Requirements**: LIVE-09, LIVE-10, LIVE-11, LIVE-12
**Success Criteria** (what must be TRUE):

  1. Frontier reasoning models (`gemini-4-carbon`, DeepSeek Pro/R1) synthesize full multi-section reports without mid-sentence truncation or `0`-char reasoning exhaustion (`LIVE-09`).
  2. `audit_citations` measures precision strictly over inline `[N]` citations referenced in the report, `try_fast_lexical_verify` does not self-contradict on contrastive `"not only"` phrasing, and `research_cache` keys include `domain_mode` while skipping `[INSUFFICIENT_EVIDENCE]` abstentions (`LIVE-10`).
  3. `AtomicDecomposer` strips YAML frontmatter, `<!-- kb:summary -->`, and Markdown table chrome before claim splitting; `publication-novelty-fetch --persist-metadata` projects `max_lexical_score` directly into `scientia_evidence`; `EventCandidateClass` classifies empirical domain science accurately; and `vox scientia cost` totals include `research`/`codegen`/`scholarly` runs (`LIVE-11`).
  4. Running `vox research` or `vox scientia` from the CLI while `vox-gui` or `vox-orchestrator-d` holds the Turso `flock` on canonical `vox.db` transparently proxies reads/writes over daemon IPC instead of failing or dropping persistence (`LIVE-12`).

**Plans**: 0 plans (`TBD — run /gsd-plan-phase 453`)

### Phase 454: Causal Swarm Event Stamping & Persistent Sequence Board

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/454-causal-swarm-event-stamping-persistent-sequence-board/`
**Goal**: Unify `vox-orchestrator` (`BulletinBoard`, `a2a`, `FileAffinityMap`, `AttentionBudget`, `circuit_breaker`) and `vox-populi` (`RemoteTaskEnvelope`) behind a monotonic `#seq` Shared Sequence Board, W3C `traceparent` causal event spans, `0 tok/s` parked wakeups, and pilot steering RPCs across Cloud, Local MENS GPU, and QUIC mesh peers.
**Depends on**: Phase 453
**Requirements**: CONC-01, CONC-02, CONC-03, CONC-04
**Success Criteria** (what must be TRUE):

  1. Every `AgentEvent`, `BulletinBoard` broadcast, `DbA2AMessage`, and `RemoteTaskEnvelope` carries `(swarm_seq, channel, traceparent, span_id, compute_tier, file_lock_prefix, workspace_mode, parked_since_seq)` (`CONC-01`).
  2. Append-only `#seq` board persists to Codex `a2a_messages` with `read_board(since_seq, seqs, channel)` cursoring, `@agent` / `@#channel` doorbell wakeups, and `0 tok/s` `waiting` actor hibernation (`CONC-02`).
  3. Operator pilot RPCs (`/api/v2/concilium/*` + MCP + CLI) execute `park_agent`, `steer_agent`, `fork_agent` (`jj`/git `share` + warm local MENS KV prefix), and `swap_agent_compute` across Cloud Bandit, Local MENS GPU, and Populi QUIC Peers (`CONC-03`).
  4. Zero-token Rust watchdog (`circuit_breaker` + lease TTL monitor) posts deterministic lock-contention, quota-failover, and stall alerts onto `#seq`, waking an LLM `@monitor` only on unresolved stalls (`CONC-04`).

**Plans**: 0 plans (`TBD — run /gsd-plan-phase 454`)

### Phase 455: Vox Concilium & Populi Unified Swarm Observatory GUI

**Directory**: `.planning/workstreams/scientia-deep-research-v76/phases/455-vox-concilium-populi-unified-swarm-observatory-gui/`
**Goal**: Execute the operator-approved `5 -> 2` Axis GUI rewrite consolidating `SubAgentsView`, `AgentFlow`, `AgentTrajectoryBraid`, `MeshView`, and `PopuliMeshBentoView` into **Mode 1: Vox Concilium** (Causal Swimlanes + `#seq` Board + Agent Lens) and **Mode 2: Vox Populi** (Hybrid Compute & `FileAffinityMap` Placement Map).
**Depends on**: Phase 454
**Requirements**: CONC-05, CONC-06, CONC-07, CONC-08
**Success Criteria** (what must be TRUE):

  1. `SubAgentsView`, `AgentFlow`, and `AgentTrajectoryBraid` consolidate into `<400`-LoC Radix `data-slot` **Vox Concilium** components (`ConciliumView.tsx`) matching the approved obsidian + gold concept art while preserving `SURFACE_REGISTRY` route compatibility (`CONC-05`).
  2. **Vox Concilium** renders the global **"Now Playing" strip**, Perfetto-style **W3C `traceparent` Causal Swimlanes** (`Thinking`, live tool arguments, `[FileLock]` shields, `Parked · 0 tok/s` sleep bars, golden `@mention` wakeup arcs), `#channel`-filtered **Shared `#seq` Board**, and **Live Agent Lens** (`Park`, `Steer`, `Fork`, `Swap Compute`) (`CONC-06`).
  3. `MeshView` and `PopuliMeshBentoView` consolidate into **Vox Populi Placement Map** (`PopuliTopologyView.tsx`) docking live `@agent` pills onto Local Host MENS GPU, Populi QUIC Peer (`iroh`), and Cloud Bandit Router cards around the **`FileAffinityMap` Lock Matrix** over event-driven WS/SSE (`0` `5000ms` polling loops) (`CONC-07`).
  4. Vitest and Playwright end-to-end specs verify live causal swimlane updates, `#seq` doorbell wakeups, `0 tok/s` parking, lock shields, and hybrid node hot-swap with `0` `VoxDb` handles in `vox-gui` and `0` `vox-arch-check` / `toestub` errors (`CONC-08`).

**Plans**: 0 plans (`TBD — run /gsd-plan-phase 455`)

---

## Progress

| Phase | Requirements | Plans Complete | Status | Completed |
|-------|--------------|----------------|--------|-----------|
| 447. Deterministic Before-Audit Baseline, L0/L1 Continuum Contracts & L3 Shared Embedder | `AUDT-01`, `DISC-01` (2) | 3/3 | Complete | 2026-11-15 |
| 448. Unified Light ↔ Deep Continuum Router, Socrates Escalation & KV-Cache-Safe Chat Harness | `CONT-01..04` (4) | 1/1 | Complete | 2026-11-15 |
| 449. Extreme-Data Resumable Streaming, `std.frame` Chunk Cursors & Virtualized Epistemic Canvas | `STRM-01..04` (4) | 1/1 | Complete | 2026-11-15 |
| 450. Calibrated 3-Tier Verifier, Two-Stage Novelty × Utility Engine & Continuous Discovery | `VERI-01..04`, `DISC-02..04` (7) | 1/1 | Complete | 2026-11-15 |
| 451. Full DB-Hydrated Direct Publication (`> Nanopublication`) Flywheel & Zero-`VoxDb` Studio | `PUBL-01..04`, `STUD-01..02` (6) | 1/1 | Complete | 2026-11-15 |
| 452. Paired After-Audit Closure, Inline Before/After Diff Inspector & Quarantined Lesson Flywheel | `AUDT-02..04` (3) | 1/1 | Complete | 2026-11-15 |
| 453. Deep Research Reasoning-Budget, Citation Audit & Daemon DB Proxy | `LIVE-09..12` (4) | 0/0 | Not Planned | — |
| 454. Causal Swarm Event Stamping & Persistent Sequence Board | `CONC-01..04` (4) | 0/0 | Not Planned | — |
| 455. Vox Concilium & Populi Unified Swarm Observatory GUI | `CONC-05..08` (4) | 0/0 | Not Planned | — |
