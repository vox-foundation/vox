# Requirements: Vox Milestone v7.6 (`scientia-deep-research-v76`)

**Defined:** 2026-11-14
**Workstream:** `.planning/workstreams/scientia-deep-research-v76/`
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) keep building, staying verified, and passing CI from a single unified worktree or federated sibling repositories with zero LLM split-brain, while remaining 100% functional offline/standalone with zero dependency on `vox-ops` and 100% transparent, disableable, strongly-typed telemetry.

## v7.6 Requirements

Committed scope for Milestone v7.6 (`Phases 447–452`). Every requirement maps to exactly one roadmap phase, requires zero new third-party Cargo crates or npm packages, introduces zero `layers.toml` / `crate-edges` violations, and preserves zero `VoxDb` handles or child CLI-DB subprocesses in `vox-gui`.

### Continuum, Domains & KV-Cache-Safe Chat (`CONT`)

- [x] **CONT-01**: Operator can select orthogonal `research_mode` (`Off | Auto | Fast | Deep`) and `research_scope` (`Local | Web | Both`) independently across Chat (`Loquela`, `/research`), CLI (`vox research`), MCP, and `ResearchView`, eliminating the `App.tsx:1972` `research_scope` overload and `waves: 1` hardcode in `research_turn.rs`.
- [x] **CONT-02**: Operator can run both `<=1`-hop Light and `>1`-wave Deep Research across all four first-class domain modes (`General`, `CodeGen` with compiler-verified probes, `Shopping` with weighted decision matrices, and `Scholarly` with prior-art/DOI synthesis) plus `waves` (`1..=5`) and `site_scope` across CLI, MCP, Chat, and GUI.
- [x] **CONT-03**: In `Auto` mode, Chat executes `T1` Local Hybrid RAG + `SocratesGateOutcome` *before* `classify_research_intent_with_socrates` (`message.rs`), distinguishing retrievable epistemic uncertainty (silently escalating `T1 -> T2 -> T3` with warm-seeded hits and zero duplicate URL fetches) from specification ambiguity (asking at most one cost-gated clarifying question when `information_gain_bits >= 0.25` vs `CLARIFICATION_INTERRUPT_COST_MS`).
- [x] **CONT-04**: Switching `research_mode`, `domain_mode`, `$active_file`, `$operating_mode`, or dynamic `AttentionBudget` across Chat turns keeps `build_system_prompt_with_skill` and ordered MCP tool declarations 100% byte-identical within a calendar day, injecting all turn-volatile research context, wave summaries, and Socrates posture strictly via `turn_user_prompt`.

### Extreme-Data & Resumable Streaming (`STRM`)

- [x] **STRM-01**: Multi-wave research runs execute via durable async session handles decoupled from the `180s` synchronous Chat timeout wall and emit monotonic `SequencedResearchEvent` (`seq: u64`) deltas (`WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta`) over `GET /api/v2/research/sessions/{id}/stream?from_seq=` SSE and `/v1/ws` with explicit `resync_required` lag recovery.
- [x] **STRM-02**: Daemon batches research telemetry writes over a bounded `mpsc` channel reusing `ServerState.db` (eliminating `scientia_feed.rs` 5s `VoxDb::connect_default()` polling and per-event `tokio::spawn` lock storms), spills `ResearchHit.raw_content` to BLAKE3-keyed chunk storage (`<=2 KiB` snippets in RAM), exposes cursor-paginated `/claims` and `/sources` (`?summary_only=true`), and records real `UsageSnapshot` USD costs into `scientia_cost_telemetry`.
- [x] **STRM-03**: Operator and research agents can analyze 100K–1M+ row datasets via out-of-core `std.frame` (`Polars 2.1.0` Arrow `StreamReader` with pure-Rust `csv` standalone fallback), streaming bounded `frame_chunk` slices and statistical summaries over SSE without serializing raw full tables into LLM prompts.
- [x] **STRM-04**: Operator views live multi-wave progress in mounted `MultiWaveProgressTimeline.tsx`, explores a backend-hydrated hierarchical `ResearchDagCanvas.tsx` (`Query -> Wave Subquery -> Source -> Claim -> Verdict/Contradiction`) with automatic wave-cluster virtualization above `80` nodes, and scrolls `@tanstack/react-virtual` claim and dataframe tables in Chat and `ResearchView`.

### Calibrated Verification & Provenance Hygiene (`VERI`)

- [x] **VERI-01**: Claim verification executes a 3-tier calibrated cascade — Tier 0 deterministic `aho-corasick`/`strsim`/numeric quote & `statcheck` pre-filter (`0` LLM calls), Tier 1 SciFact entailment (`Support | Contradict | NotEnoughInfo | Abstain` enforcing `abstain_threshold`) via `vox_actor_runtime::llm::infer_with_retry`, and Tier 2 `3×` resampling restricted to the uncertain margin `[abstain, 0.80]`.
- [x] **VERI-02**: Production claim extraction and verification permanently retire `MiniCheckVerifier::from_env()`'s `MiniCheckBackend::Mock` default (`Mock` gated strictly to `#[cfg(test)]`), migrate `VOX_MINICHECK_ENDPOINT` off raw `std::env::var` onto Clavis/`vox-config`, and migrate `scientia_prior_art.rs` off direct `sha2` onto `vox-crypto`.
- [x] **VERI-03**: Post-synthesis citation auditing verifies quoted spans against retrieved source chunks, makes `ClaimVerdict::Contradicted` reachable so `contradiction_penalty` fires in `gate.rs`, and weights `ResearchHit.trust_score` using Crossref Retraction Watch (`update-to` hard-zero) and OpenAlex venue/peer-review signals.
- [x] **VERI-04**: Autonomous research syntheses and memory notes persisted to `VoxDb` carry explicit `untrusted_synthesis` provenance taint (`D-06`), preventing `ResearchScope::Both` (`web_gather.rs`) from re-ingesting prior model-generated reports at `trust_score: 1.0` (eliminating the Epistemic Ouroboros loop).

### Two-Stage Novelty × Utility & Continuous Discovery (`DISC`)

- [x] **DISC-01**: Workspace consolidates `NoveltyEvidenceBundle` (`L1` `vox-research-events`) and lifts `CachedLlmEmbedder` to `L3` `vox-scientia::embedder` (`vox_actor_runtime::llm::llm_embed` + pure-Rust local fallback + `scientia_embedding_cache`) with zero new inter-crate Cargo dependencies.
- [x] **DISC-02**: `AtomicNoveltyScorer` fuses Stage-1 `vox-similarity` (`LshIndex`/MinHash/shingle Jaccard) and Stage-2 dense cosine similarity with `ChronoFilter` and `EvidenceConflict`, returning `NoveltyVerdict::InsufficientEvidence` (never `Novel`) whenever prior-art `query_traces` fail HTTP health checks or embeddings are absent.
- [x] **DISC-03**: Prior-art and trust adapters enforce Clavis-backed OpenAlex API-key authentication (`Bearer` / `api_key=`, replacing the retired `&mailto=` pool), Crossref Polite `?mailto=`, and `governor` GCRA rate limiters (`Semantic Scholar: 1 RPS`, `3.5s` timeouts), while providing an explicit human-signed `OfflineAttested` path so standalone/air-gapped promotion never deadlocks on `TrustLookup::Unavailable`.
- [x] **DISC-04**: Continuous discovery (`vox scientia scout`, `commit_watcher`, and `SocratesTelemetryProducer` high-entropy abstention/gap clustering) enforces a two-stage **Novelty × Utility** (`WorthinessSignalsV2`) filter — requiring both prior-art novelty and practical utility proof (measured replay digest, benchmark delta, compiler-verified probe, or corroborated epistemic gap) before surfacing candidates to `scientia_discovery_inbox`.

### Full Direct Publication (`> Nanopublication`) & Axis Studio (`PUBL` / `STUD`)

- [x] **PUBL-01**: Operator can promote a discovery candidate or completed research session to `publication_manifests` in 1 click (`promote_candidate_to_publication`, fixing orphan `finding-<uuid>` IDs and preserving `EventCandidateClass` + `novelty_evidence_bundle_id`), gated before any external emission by a 4-layer content-digest-bound human `ApprovalToken`.
- [x] **PUBL-02**: Publishing an Architecture SSOT (`render_doc_draft` / `write_research_doc`) hydrates real verified claims, citations, and probes from `VoxDb` without wiping `artifact_json` (`gui_ops/research.rs:715` fix) and validates in-memory against `vox-doc-pipeline` (`topic`, `domain`, `authorship`, `authorship_model`, `status: "research"`, `<!-- kb:summary -->`, `vox-diagram` fences, zero `vox:skip`, zero ````mermaid`) before writing to `docs/src/architecture/`.
- [x] **PUBL-03**: Operator can execute idempotent `flywheel_advance` from `publication_id` with **zero manual JSON scaffold files** (`assemble_scaffold_from_db` + `assemble_finding_page_from_db`), producing Unicode/comment/URL-hardened venue IMRaD LaTeX (`-no-shell-escape`), deterministic pure-Rust `flate2` arXiv `.tar.gz` bundles, RO-Crate 1.2 packages, interactive HTML Finding Pages, spec-compliant Clavis-per-user RSA `RA` Trusty-URI nanopublications, and Zenodo/OpenReview archival receipts.
- [x] **PUBL-04**: Executing `publication-replay-execute` measures deterministic artifact reproducibility and writes `artifact_replayability` and digest receipts back into `publication_manifests.metadata_json.scientia_evidence` and `WorthinessInputs`, while `vox scientia dashboard` and `vox scientia cost` report real candidate confidences, reply windows, and per-stage USD costs.
- [x] **STUD-01**: `PublicationsView.tsx`, `ScientiaSurface.tsx`, and `DiscoverySurface.tsx` route 100% of queries and mutations through `crates/vox-gui/ui/src/transport.ts` (`/api/v2/scientia/*` over `BackendState`), eliminating all `execute_command` child CLI `VoxDb::connect_default()` spawns (`0` persistent or transient `VoxDb` handles in `vox-gui`).
- [x] **STUD-02**: Operator can triage `DiscoveryInbox` candidates, inspect the 6-axis `WorthinessSignalsV2` & `NoveltyEvidencePanel` breakdown, advance/preview IMRaD/LaTeX/SSOT drafts, mint human `ApprovalToken` sign-offs, and trigger sandbox/offline archival from a unified Axis Scientia/Publications studio built with `<400`-LoC child components, Radix `data-slot` primitives, and zero raw emojis/dingbats.

### Paired Before/After Audit, Inline Diff & Next-Lesson Flywheel (`AUDT`)

- [x] **AUDT-01**: Headless `vox scientia v76-audit --stage before` (`Phase 447`) captures a deterministic frozen-cassette baseline scorecard (`contracts/reports/scientia-v76-before-baseline.v1.json`) replacing `eval.rs`'s hardcoded `1.0` stubs and measuring pre-v7.6 metrics across all 6 axes (**performance**, **quality**, **verification**, **integration**, **Socrates calibration**, and **novelty detection**) plus live bug reproduction counters.
- [x] **AUDT-02**: Headless `vox scientia v76-audit --stage after --compare` (`Phase 452`) executes the identical frozen-cassette corpus plus stratified live trials, computes `vox_eval::paired_ni` confidence intervals and `vox audit`-compatible JSONL findings (`contracts/reports/scientia-v76-after-closure.v1.json`), and proves strict improvement / zero regression across all 6 axes.
- [x] **AUDT-03**: Operator can inspect paired Before vs. After run and claim/report/metric differences **inline** in Axis GUI (`ResearchView`, `ScientiaSurface`, and Chat turn receipts) via `<BeforeAfterAuditInspector />` powered by `similar` inline diff hunks and 6-axis delta badges.
- [x] **AUDT-04**: Post-audit and post-research synthesis automatically extracts structured, `SanitizedText`-scrubbed **"next lessons"** into `VoxDb` (`kb_findings` quarantined as `Untrusted` until `>=2`-run/domain corroboration in `kb_inbox`, `.proposed` enforcement rules via `vox-effort-route`, and verified repair/grounding pairs for MENS Auto-DPO) without ever self-mutating `AGENTS.md` or canonical SSOT docs unreviewed.

### Live Research Hardening & Daemon DB Proxy (`LIVE`)

- [ ] **LIVE-09**: Unary and streaming research synthesis (`vox-llm-egress` + `vox-research-shim::stages`) decouple reasoning-token budgets from visible output budgets (`synthesis_max_tokens >= 12,000` or explicit reasoning cap) so frontier thinking models (`gemini-4-carbon`, DeepSeek R1/Pro) never truncate visible Markdown mid-sentence or emit `0`-char answers after reasoning exhaustion.
- [ ] **LIVE-10**: `audit_citations` evaluates precision over citations actually referenced `[N]` in the synthesized report (extracting claims across cited hits rather than only `all_hits[..8]`), `try_fast_lexical_verify` ignores contrastive `"not only"` constructions when computing negation conflict, and `research_cache` keys include `domain_mode` while refusing to cache `[INSUFFICIENT_EVIDENCE]` abstentions.
- [ ] **LIVE-11**: `vox_scientia::claim_extractor::AtomicDecomposer` strips YAML frontmatter (`---`), HTML comments (`<!-- kb:summary -->`), and Markdown table delimiters prior to sentence splitting; `publication-novelty-fetch --persist-metadata` auto-projects `max_lexical_score` into `scientia_evidence` so `publication-discovery-scan` drops `novelty_overlap_assumed_default`; `EventCandidateClass` adds empirical/domain science classification; and `vox scientia cost` includes `research` / `codegen` / `scholarly` runs in `this_quarter.total_usd`.
- [ ] **LIVE-12**: When canonical `vox.db` holds an active Turso `flock` from `vox-gui` or `vox-orchestrator-d` / `vox-mcp`, CLI `vox research` and `vox scientia` commands automatically proxy DB reads/writes through the running daemon IPC socket (falling back cleanly to explicit `VOX_DB_PATH` when standalone).

### Vox Concilium & Populi Causal Swarm Observatory (`CONC`)

- [ ] **CONC-01**: Backend `AgentEvent`, `BulletinBoard`, `DbA2AMessage`, and `vox-populi` `RemoteTaskEnvelope` stamp canonical causal swarm metadata: `(swarm_seq: u64, channel: String, traceparent: String, span_id: String, compute_tier: ComputeTier, file_lock_prefix: Option<String>, workspace_mode: WorkspaceMode, parked_since_seq: Option<u64>)`.
- [ ] **CONC-02**: Append-only `#seq` Shared Sequence Board persists across `BulletinBoard` and Codex `a2a_messages` with `read_board(since_seq, seqs, channel)` cursoring, `@agent` / `@#channel` doorbell wakeup routing, and `0 tok/s` parked actor state (`waiting` until mentioned).
- [ ] **CONC-03**: Pilot steering RPCs (`/api/v2/concilium/*` + MCP + CLI) expose live operator interventions: `park_agent` (`0 tok/s`), `steer_agent` (append `#seq` directive + wake), `fork_agent` (branch warm conversation + `jj`/git `share` workspace preserving local MENS KV-cache prefix), and `swap_agent_compute` (hot-migrate waiting/throttled workers across Cloud Bandit, Local MENS GPU, and Populi QUIC Peers).
- [ ] **CONC-04**: Zero-token Rust watchdog (`circuit_breaker` + lease TTL monitor) feeds live stall, quota-failover, `AttentionBudget`, and `FileAffinityMap` lock-contention events directly onto the `#seq` board, escalating to an LLM `@monitor` worker only when deterministic recovery rules trip.
- [ ] **CONC-05**: Operator-approved `5 -> 2` Axis GUI consolidation merges `SubAgentsView`, `AgentFlow`, and `AgentTrajectoryBraid` into **Mode 1: Vox Concilium** (`ConciliumView.tsx`, `<400`-LoC Radix `data-slot` child components, obsidian + gold telemetry palette, `0` raw emojis/dingbats, `SURFACE_REGISTRY` route compatibility preserved).
- [ ] **CONC-06**: **Vox Concilium** renders a global **"Now Playing" strip**, Perfetto-style **W3C `traceparent` Causal Swimlanes** (showing `Thinking`, streaming tool calls, `[FileLock]` shields, `Parked · 0 tok/s` sleep bars, and golden `@mention` wakeup arcs), `#channel`-filtered **Shared `#seq` Sequence Board** with diff/test receipts, and **Live Agent Lens** pilot controls.
- [ ] **CONC-07**: Operator-approved consolidation merges `MeshView` and `PopuliMeshBentoView` into **Mode 2: Vox Populi Placement Map** (`PopuliTopologyView.tsx`), docking live `@agent` pills directly onto **Local Host MENS GPU**, **Populi QUIC Peer (`iroh`)**, and **Cloud Frontier Router** cards around the Rust-enforced **`FileAffinityMap` Lock Matrix** (`0` `5000ms` blind polling loops; event-driven WS/SSE updates).
- [ ] **CONC-08**: Playwright / Vitest visual and behavioral verification proves live causal swimlane rendering, `#seq` doorbell wakeups, `0 tok/s` parking, `FileAffinityMap` shield updates, and hybrid compute hot-swap with `0` `VoxDb` handles in `vox-gui` and `0` `vox-arch-check` / `toestub` regressions.

## Future Requirements (Deferred to `v7.6.x` / `v8.0+`)

Tracked for subsequent iterations; explicitly excluded from v7.6 phases to keep scope tight and build-toolchain clean.

### Retrieval & Layout Uplift (`v7.6.x`)

- **RETR-F01**: Optional pure-Rust `candle` local cross-encoder reranker over RRF-fused hits if `Scholarly` multi-hop recall@10 requires post-v7.6 uplift.
- **RETR-F02**: Optional local `SPECTER2` adapter weights in `vox-scientia::embedder` for air-gapped scholarly citation-graph proximity.
- **PUBL-F01**: Advisory VLM figure/layout critic (`visus_review`) over rendered LaTeX PDFs and Finding Pages prior to human sign-off.

### Federated Network Expansion (`v8.0+`)

- **PUBL-F02**: Live production Knowledge Pixels Nanopublication registry broadcast (v7.6 proves full offline `Nanopub::check()` conformance + opt-in test server only).
- **DISC-F01**: Cross-peer `Populi` iroh QUIC mesh gossip for anonymized `NoveltyEvidenceBundle` and `FindingCandidate` digests.

## Out of Scope

Explicit exclusions and anti-features documented to prevent scope creep and policy regressions.

| Feature | Reason |
|---------|--------|
| Unreviewed auto-publishing to public venues (arXiv, Zenodo Prod, OpenReview, Nanopub Prod, social) | Violates venue authorship/anti-spam rules and human-attribution ethics; 4-layer human `ApprovalToken` gate is non-negotiable (`AF-01`). |
| C++ `ort` / `fastembed` / `onnxruntime-sys` / C++ `hnswlib` dependencies | Violates `AGENTS.md §Build-toolchain invariant` (`no cmake, nasm, Go, perl, libclang`); workspace-native `vox-similarity` + `vox_actor_runtime::llm` + pure-Rust `candle` cover all needs. |
| Unbounded recursive tree-search (BFTS) without wave/spend ceilings | Causes runaway token spend and rate-limit starvation; v7.6 enforces `waves: 1..=5`, entropy stopping, and `WebFallbackSpendLedger` (`AF-02`). |
| Dumping raw 100K+ row CSV/SQL tables into LLM prompts | Destroys KV-cache reuse and accuracy; `std.frame` computes deterministic aggregations and streams paginated Arrow chunks (`AF-03`). |
| Single-scalar novelty scoring without utility/worthiness gates | Surfaces trivial noise and failed HTTP fetches as "discoveries"; two-stage Novelty × Utility (`WorthinessSignalsV2`) is required (`AF-04`). |
| Direct vendor LLM SDKs or hardcoded provider URLs in research/verifier crates | Violates `AGENTS.md §Model-Agnostic LLM Boundary` (`llm_provider_call` Error); 100% of calls route through `vox_actor_runtime::llm` (`AF-05`). |
| Shared workspace/bot signing key for nanopublications | Violates per-researcher non-repudiation; RSA keys are per-user in Clavis bound to ORCID (`AF-06`). |
| Auto-mutating `AGENTS.md` or canonical `docs/src/architecture/` from unreviewed lessons | Corrupts SSOTs and risks prompt-injection persistence; lessons enter `kb_findings` quarantined as `Untrusted` (`AF-07`). |

## Traceability

Populated and verified during roadmap creation (`Phases 447–452`).

| Requirement | Phase | Status |
|-------------|-------|--------|
| AUDT-01 | Phase 447 | Complete |
| CONT-01 | Phase 448 | Complete |
| CONT-02 | Phase 448 | Complete |
| CONT-03 | Phase 448 | Complete |
| CONT-04 | Phase 448 | Complete |
| STRM-01 | Phase 449 | Complete |
| STRM-02 | Phase 449 | Complete |
| STRM-03 | Phase 449 | Complete |
| STRM-04 | Phase 449 | Complete |
| VERI-01 | Phase 450 | Complete |
| VERI-02 | Phase 450 | Complete |
| VERI-03 | Phase 450 | Complete |
| VERI-04 | Phase 450 | Complete |
| DISC-01 | Phase 447 | Complete |
| DISC-02 | Phase 450 | Complete |
| DISC-03 | Phase 450 | Complete |
| DISC-04 | Phase 450 | Complete |
| PUBL-01 | Phase 451 | Complete |
| PUBL-02 | Phase 451 | Complete |
| PUBL-03 | Phase 451 | Complete |
| PUBL-04 | Phase 451 | Complete |
| STUD-01 | Phase 451 | Complete |
| STUD-02 | Phase 451 | Complete |
| AUDT-02 | Phase 452 | Complete |
| AUDT-03 | Phase 452 | Complete |
| AUDT-04 | Phase 452 | Complete |
| LIVE-09 | Phase 453 | Pending |
| LIVE-10 | Phase 453 | Pending |
| LIVE-11 | Phase 453 | Pending |
| LIVE-12 | Phase 453 | Pending |
| CONC-01 | Phase 454 | Pending |
| CONC-02 | Phase 454 | Pending |
| CONC-03 | Phase 454 | Pending |
| CONC-04 | Phase 454 | Pending |
| CONC-05 | Phase 455 | Pending |
| CONC-06 | Phase 455 | Pending |
| CONC-07 | Phase 455 | Pending |
| CONC-08 | Phase 455 | Pending |

**Coverage:**
- v7.6 requirements: 38 total (`26` completed in Phases 447–452; `12` added across Phases 453–455)
- Mapped to phases: 38
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-14*
*Last updated: 2026-11-15 after adding Phases 453–455 (`LIVE-09..12`, `CONC-01..08`)*
