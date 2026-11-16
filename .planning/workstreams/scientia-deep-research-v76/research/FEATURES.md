# Feature Research

**Domain:** Agentic Research Continuum (`<=1`-Hop Light ↔ `>1`-Wave Deep), Extreme-Data Streaming, Calibrated Epistemic/Novelty Verification, Human-Gated Direct Scientific/Architectural Publication & Before/After Evaluation Flywheel (Milestone v7.6)
**Researched:** 2026-11-14
**Confidence:** HIGH (grounded in live Vox tree audits + 2025–2026 frontier prior art: OpenAI/Perplexity/Google/Anthropic Deep Research, FutureHouse Crow–Falcon–Owl–Finch–Robin, Sakana AI Scientist-v2, Google AI Co-Scientist, DeepResearch Bench, and DRACO)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features operators and researchers assume work end-to-end in a 2026 agentic research and scientific workbench. Missing or stubbed versions of these break trust immediately ("potemkin UI" / silent $0.00 telemetry / broken handoffs).

| Feature | Why Expected | Complexity | Notes & Vox Subsystem Seams |
|---------|--------------|------------|-----------------------------|
| **TS-01: Unified Light (`<=1`-Hop) ↔ Deep (`>1`-Wave) Routing Continuum** | Every 2026 frontier research assistant (Perplexity Effort Dials, Gemini Deep Research, ChatGPT `/research`) treats quick cited lookup and multi-wave investigation as one parameterized pipeline rather than disconnected loops. | MEDIUM | Unify `ResearchMode::{None, Quick, Deep}` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_intent.rs:8-20` with `vox-research-shim`'s `ResearchQuery` (`crates/vox-research-shim/src/research/types.rs:53-69`) and the autonomous orchestrator loop (`crates/vox-orchestrator/src/orchestrator/task_dispatch/research_dispatch.rs`) so `waves == 1` (`Light`) and `waves >= 2` (`Deep`) share one trace, sanitizer, and citation contract. |
| **TS-02: Four First-Class Domain Modes (`General`, `CodeGen`, `Shopping`, `Scholarly`) Across CLI, MCP, Chat & GUI** | Users expect domain-appropriate decomposition: literature/DOI synthesis for `Scholarly`, compiler-verified probes for `CodeGen`, weighted decision matrices for `Shopping`, and open-web synthesis for `General`. | MEDIUM | Add `Scholarly` to `ResearchDomainMode` (`crates/vox-research-shim/src/research/types.rs:24-45`, currently only `General \| Shopping \| CodeGen`); wire `domain_mode`, `scope` (`Web \| Local \| Both`), `waves` (`1..=5`), and `site_scope` through `StartResearchBody` (`crates/vox-orchestrator-mcp/src/gui_ops/research.rs`), `Loquela.tsx` (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx:44-94`), `ResearchModeDropdown.tsx`, and `ResearchView.tsx`. |
| **TS-03: Live SSE Stage, Wave, Subquery, Claim & Real-Cost Streaming** | Multi-minute research runs that freeze at "Stage 0: Query Decomposition" or show `$0.00` cost feel hung or broken; 2026 users expect real-time streaming progress across HTTP SSE, WebSocket, MCP notifications, and GUI/Chat rails. | MEDIUM | Bridge `BroadcastEmitter` (`crates/vox-research-shim/src/research/emitter.rs:5-25`) and `research_session_events` to an HTTP SSE endpoint (`GET /api/v2/research/sessions/{id}/stream` in `gui_ops/research.rs:1278-1288`) and Chat turn-event stream (`turn_events.rs`). Mount the already-built `MultiWaveProgressTimeline.tsx` (`crates/vox-gui/ui/src/components/surfaces/Research/MultiWaveProgressTimeline.tsx`) in `ResearchView.tsx` and wire `metering::snapshot()` into `VoxDb::record_scientia_cost` (never fabricate `$0.00`). |
| **TS-04: First-Class Chat Research Ergonomics (`Loquela` Knobs + `/research` Flags)** | Chat is the primary daily-driver surface; operators expect `/research [light\|deep] [--domain scholarly\|codegen\|shopping\|general]` and `Loquela` rotary/capsule knobs to control depth, scope, and domain without leaving Chat. | LOW | Fix `message.rs:142` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:142-146`) to call `classify_research_intent_with_socrates` instead of passing `None`, extend `slashCommands.ts:30` and `ResearchModeDropdown.tsx:16-42`, and persist research posture alongside `get_session_chat_posture` / `set_session_chat_posture`. |
| **TS-05: Calibrated Claim Grounding & Post-Hoc Citation Verification (No Mock Fallback)** | Unverified `[N]` badges erode trust faster than having no citations at all (DeepResearch Bench FACT / Perplexity DRACO standard). Every cited claim must be checked against retrieved spans with honest abstention. | MEDIUM | Replace the default `MiniCheckBackend::Mock` token-overlap fallback (`crates/vox-scientia/src/claim_extractor/minicheck.rs`, `scientia_phase_handlers.rs:304-336`) and `PHASE_0a_STUB` `abstain_threshold` (`crates/vox-research-shim/src/research/verifier.rs:15-26`) with a fast lexical/n-gram containment pre-filter + structured NLI entailment/contradiction/abstain pass routed strictly through `vox_actor_runtime::llm::infer_with_retry`. Make `ClaimVerdict::Contradicted` reachable so `refuted > 0` fires `contradiction_penalty`. |
| **TS-06: Honest Prior-Art Retrieval & `InsufficientEvidence` Guard** | In continuous discovery (FutureHouse *Owl* / precedent search), offline/rate-limited HTTP failures must never report `max_semantic = 0.0` → `"Novel!"`. | LOW | Consolidate `NoveltyEvidenceBundle` (`vox-research-events`) and `NoveltyEvidenceBundleV1` (`vox-publisher/src/scientia_finding_ledger.rs`), drive `NoveltyVerdict::InsufficientEvidence` from `query_traces` HTTP health (`scientia_prior_art.rs:149`), and wire `vox_actor_runtime::llm::llm_embed` with local Candle/fastembed offline fallback (`scientia_semantic.rs`). |
| **TS-07: Four-Layer Human-Gated Discovery Review & 1-Click Manifest Promotion** | Operators require an unbreakable guarantee that autonomous discovery never auto-publishes externally, paired with a working 1-click path from inbox candidate to `publication_manifests`. | MEDIUM | Fix Gap 2 in `discovery_bridge.rs:82-98, 226-231` (`EventCandidateClass` classification + `promote_candidate_to_publication` creating a real `publication_manifests` row instead of writing orphan `finding-<uuid>` into `scientia_discovery_inbox.publication_id`). Enforce digest-bound `ApprovalToken` (`review_flow.rs`) before any emission. |
| **TS-08: DB-Hydrated Direct Publication Artifacts (`docs/src/architecture/` SSOT + IMRaD + LaTeX + arXiv + Finding Page)** | Requiring manual `--scaffold X.json` files on disk or emitting frontmatter-invalid `.md` stubs defeats "Direct Publication". Every output format must hydrate directly from `VoxDb` (`session_id` / `publication_id`). | MEDIUM | Fix `render_doc_draft` (`gui_ops/research.rs:199-239`) and `doc_synthesizer.rs:41-52` to hydrate real `VoxDb` claims/citations/probes and emit `vox-doc-pipeline`-compliant YAML frontmatter (`topic`, `domain`, `authorship`, `authorship_model`) + `<!-- kb:summary -->` + `vox-diagram` fences. Add `assemble_scaffold_from_db` and `assemble_finding_page_from_db` in `scientia_phase_handlers.rs:32-66, 727-890`. |
| **TS-09: Reproducible Paired Before/After Evaluation Scorecard** | Any claim that v7.6 improved research accuracy, latency, verification, or novelty requires a deterministic paired benchmark rather than anecdotal spot-checks. | MEDIUM | Upgrade `crates/vox-cli-research/src/eval.rs:22-130` (`vox research eval`) with paired before/after run comparison (`vox_eval::paired_ni` in `crates/vox-eval/src/paired_ni.rs`) emitting `vox audit`-compatible JSONL + summary JSON covering RACE quality, FACT citation precision, groundedness, latency/TTFT, cost, Socrates ECE, and novelty precision/recall. |

---

### Differentiators (Competitive Advantage)

Capabilities where Vox's unified compiler + `std.frame` runtime + Socrates policy engine + `VoxDb` + SCIENTIA stack outperforms both commercial Deep Research products (which stop at prose summaries) and autonomous paper generators like Sakana AI Scientist-v2 (which lack formal human-gated provenance and architectural SSOT integration).

| Feature | Value Proposition | Complexity | Notes & Vox Subsystem Seams |
|---------|-------------------|------------|-----------------------------|
| **DF-01: Socrates Epistemic Auto-Escalation (`Light -> Deep`) & Cost-Bounded Questioning** | Instead of making users guess upfront whether a question needs 3 seconds or 3 waves, Socrates starts in `<=1`-hop Light mode (or evaluates local KB confidence), detects high contradiction / low coverage / high entropy, and either asks **one** information-gain-optimal clarifying question (`min_information_gain_bits >= 0.08`, weighted against `CLARIFICATION_INTERRUPT_COST_MS = 23_250`) or auto-escalates mid-turn into `>1`-wave Deep Research. | HIGH | Compose `QuestioningPolicy` (`crates/vox-orchestrator-types/src/socrates_policy/policy_types.rs:78-138`), `SessionRetrievalEnvelope` (`crates/vox-orchestrator/src/socrates.rs:18-140`), and `research_turn.rs` so a `Quick` pass with `citation_diversity_below_threshold` or `contradiction_count > 0` seamlessly promotes to Wave 2+ CRAG retrieval while streaming an explicit `socrates_escalated` turn event. |
| **DF-02: Extreme-Data Chunked Streaming (`std.frame` Arrow Batches + Virtualized Tables)** | Commercial deep research truncates tabular evidence to ~50 Markdown rows to fit LLM context windows. Vox can execute `std.frame` (Polars/Arrow IPC) plans over 100K–1M+ row datasets/telemetry/benchmarks, stream bounded Arrow/JSON record chunks over SSE/IPC, virtualize them in GUI/Chat tables, and feed statistical summaries + schema invariants to the synthesizer without blowing token budgets. | HIGH | Extend `crates/vox-python-bridge/src/frame_columns.rs:53-88` (`StreamReader` Arrow `RecordBatch` iterator) and `crates/vox-compiler/src/eval/frame_plan.rs` with bounded chunk/window cursors (`scan_csv` / `scan_parquet` / `VoxDb` query frames -> paginated batch slices + summary stats), exposed via streaming SSE `frame_chunk` events to `ResearchView.tsx` and `ChatTranscript.tsx` virtualized table cards (FutureHouse *Finch* capability natively inside Vox). |
| **DF-03: Two-Stage Novelty × Utility (`WorthinessSignalsV2`) Discovery Filter** | Solves the #1 failure mode of 2025–2026 automated science (Sakana AI Scientist-v2 "trivial/useless novelty"): Stage 1 proves **epistemic novelty** (MinHash/LSH + semantic cosine + ChronoFilter + prior-art conflict across OpenAlex/Crossref/S2 + local repo); Stage 2 proves **practical utility & worthiness** (retraction hard-gate, venue soft-gate, measured sandbox `artifact_replayability`, benchmark/latency delta, or unresolved Socrates epistemic gap). Only *novel AND useful* candidates enter the human inbox. | HIGH | Wire `crates/vox-similarity/` (Stage-1 LSH), `crates/vox-publisher/src/scientia_novelty_assess.rs` (`assess_novelty`), `crates/vox-scientia/src/inspect_bridge/{chronofact,conflict}.rs`, and `crates/vox-scientia/src/producers/worthiness.rs:87-102` (`WorthinessSignalsV2`) + measured replay writeback (`scientia_phase_handlers.rs:727-796`) into a unified two-stage gate before surfacing to `scientia_discovery_inbox`. |
| **DF-04: Continuous Multi-Signal Autonomous Discovery Engine (Scout + Socrates Gaps + Telemetry + Commits)** | Adapts Google *AI Co-Scientist* & FutureHouse *Robin* to software/ML research engineering: continuously mines workspace commit graphs, benchmark deltas, MENS training runs, and **recurring Socrates abstentions/low-confidence episodes** to propose concrete hypotheses and candidate findings while idle or on schedule, operating 100% locally/offline when needed. | MEDIUM | Upgrade `ProducerRegistry` (`crates/vox-scientia/src/producers/registry.rs:56-78`), `socrates_telemetry.rs:25-80` (expanding beyond `trust_obs` mean deltas to cluster high-entropy `RiskDecision::Abstain` / `InsufficientEvidence` epistemic gaps), `commit_watcher.rs`, and `vox scientia scout` (`crates/vox-cli-research/src/scout.rs:15-70`, migrating `VOX_SCOUT_CODE_UNIQUENESS` to Clavis/config). |
| **DF-05: Compiler- & Sandbox-Verified `CodeGen` + Replayable Direct Publication (`> Nanopublication`)** | Unlike prose-only research reports, Vox `CodeGen` waves compile-check Rust/`.vox` probes (`cargo check` / `vox check`) and SCIENTIA `publication-replay-execute` verifies artifact digest reproducibility, projecting a single verified `AtomicClaim` (with spec-compliant `RA` Trusty URI) simultaneously into: (1) lint-verified `docs/src/architecture/*.md` SSOTs, (2) venue-ready IMRaD LaTeX/arXiv `.tar.gz` bundles + RO-Crate 1.2, (3) interactive HTML Finding Pages, and (4) Zenodo/Software Heritage/OpenReview archival receipts. | HIGH | Unify projection over canonical `AtomicClaim` + `nanopub/spec.rs` (`RA` Trusty URI + per-user Clavis RSA custody + ORCID PKCE `orcid_oauth.rs`), `manuscript/latex/bundle.rs`, `ro_crate/`, `findings_site/`, and `archive_run.rs` (`crates/vox-publisher/src/archive_run.rs`), with CI gate asserting byte-identical Trusty URIs across all projections. |
| **DF-06: True Hierarchical Epistemic DAG Canvas & Unified Scientia Studio Workbench** | Replaces opaque spinner logs and synthetic linear node chains with an inspectable causal graph (`Query -> Wave Subquery -> Source Domain -> Extracted Claim -> NLI Verdict / Contradiction Edge / CodeGen Sandbox Probe`) and a single studio (`Discovery Inbox` → `6-Axis Worthiness Radar` → `IMRaD/LaTeX Preview` → `Archive`) backed by typed `/api/v2/scientia/*` RPCs. | MEDIUM | Replace the client-side `chain-${i}` mock in `ResearchView.tsx:762-778` with backend `build_epistemic_dag(session, events)` (`gui_ops/research.rs`), upgrade `ResearchDagCanvas.tsx`, and unify `ScientiaSurface.tsx`, `DiscoverySurface.tsx`, and `PublicationsView.tsx` off slow CLI subprocess shelling (`execute_command`) onto typed HTTP/Tauri endpoints. |
| **DF-07: Inline Before/After Diff Inspector + Automated "Next Lesson" Flywheel into `VoxDb` / KB / MENS** | Closes the self-improvement loop: every research/harness/publication run can be compared inline against its baseline across 6 axes (performance, quality, verification, integration, Socrates calibration, novelty), and verified failure/improvement patterns are automatically distilled into quarantined `kb_findings` lessons, enforcement proposals (`vox-effort-route`), and MENS Auto-DPO pairs. | HIGH | Extend `TaskDiffDialog.tsx` / `JudgeInspector.tsx` into a reusable `BeforeAfterAuditInspector` component consuming paired `ResearchEvalSampleRecord` + `SotaHarnessScorecard` diffs; wire post-audit lesson extraction through `vox_actor_runtime::llm` -> `capture_finding` (`crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:1-40`) + `kb_inbox.rs` (`GROW-01..05` `>=2`-corroboration gate) + `vox-effort-route` (`crates/vox-effort-route/src/pipeline.rs:55-90`). |

---

### Anti-Features (Commonly Requested, Often Problematic)

Features frequently attempted in "AI Scientist" / "Deep Research" projects that create severe reputational, security, architectural, or token-waste hazards. Explicitly out of scope for v7.6.

| Feature | Why Requested | Why Problematic | Vox v7.6 Alternative |
|---------|---------------|-----------------|----------------------|
| **AF-01: Unreviewed Auto-Submission / Auto-Spamming Public Venues (arXiv, OpenReview, Zenodo Prod, Nanopub Prod, X/Bluesky)** | "Full autonomy" demos promise zero-click paper/post publishing while the user sleeps. | Violates arXiv/OpenReview/COPE authorship and anti-spam policies, risks publishing hallucinated or retracted claims under a user's real ORCID, and caused major community backlash against uncurated AI Scientist outputs in 2025–2026. | **Four-Layer Human-Gated Direct Publication:** Autonomous discovery, drafting, NLI verification, and bundle assembly run unattended, but external emission requires a human-minted, content-digest-bound `ApprovalToken` (`review_flow.rs`), defaults to Zenodo Sandbox / local bundle / offline nanopub conformance validator, and keeps arXiv handoff-only. |
| **AF-02: Unbounded Recursive Tree-Search Loops Without Cost/Wave/Convergence Ceilings** | Sakana BFTS / open-ended agent loops appear more thorough by spawning dozens of unbounded branches. | Causes runaway API spend, rate-limit starvation across shared free tiers, context poisoning from low-trust SEO farms, and hung tokio tasks on daemon restart. | **Bounded Wave & Entropy Stopping (`1..=5` Waves + `WebFallbackSpendLedger`):** Hard-cap waves, enforce daily spend caps (`kb_inbox.rs:23-55`), apply Bayesian/entropy-delta early stopping when marginal claim gain `< threshold`, and persist wave checkpoints in `VoxDb`. |
| **AF-03: Dumping Raw Million-Row Dataframes into LLM Prompts ("Long-Context Brute Force")** | Frontier 1M–2M token windows tempt developers to serialize entire CSV/SQL dumps into chat prompts. | Destroys KV-cache reuse, spikes TTFT/cost 50x, degrades needle-in-a-haystack arithmetic accuracy, and crashes WebView DOM rendering. | **Out-of-Core `std.frame` Execution + Chunked Streaming:** Compute aggregations, filters, and statistical invariants deterministically in `std.frame` (Polars/Arrow IPC), pass compact typed schema + summary tables to the LLM, and stream paginated Arrow/JSON chunks directly to virtualized UI tables. |
| **AF-04: Single-Scalar Novelty Without Utility/Worthiness Gating** | Embedding distance > threshold is easy to compute and produces lots of "novel" hits. | Flags random noise, typos, broken benchmarks, and failed HTTP retrievals (`max_semantic = 0.0`) as "breakthrough discoveries", flooding the operator's review inbox with slop. | **Two-Stage Novelty × Utility (`WorthinessSignalsV2`):** Require HTTP-healthy `query_traces` (else `InsufficientEvidence`) AND Stage-2 utility proof (replayable benchmark delta, compiler-verified fix, or corroborated Socrates gap) before surfacing. |
| **AF-05: Direct Vendor SDKs / Hardcoded Provider Endpoints in Research or Verifier Crates** | Tempting to wire `api.openai.com`, `api.anthropic.com`, or standalone MiniCheck HTTP containers directly inside `vox-research-shim` or `vox-scientia`. | Violates `AGENTS.md §Model-Agnostic LLM Boundary` (`vox-code-audit` `llm_provider_call` Error), breaks offline/standalone MENS operation, and fragments token/cost accounting. | **100% Facade & Local-First Routing:** Route all planner, NLI verifier, critic-gate, synthesizer, and lesson-extraction calls through `vox_actor_runtime::llm` (`infer_with_retry`, `llm_stream`, `llm_embed`) with `ModelRegistry` per-stage scoring and local Candle/MENS fallback. |
| **AF-06: Shared Global Bot Signing Key for Nanopublications & Scholarly Provenance** | Simplifies key management by shipping or generating one workspace-wide keypair. | Collapses scientific attribution across operators, violates ORCID non-repudiation, and creates catastrophic blast radius on key leak. | **Per-User Clavis-Custodied RSA/ORCID Identity:** Store per-user nanopub RSA keys encrypted in Clavis (`vox_vault.rs`) bound to `user_identities` + user ORCID PKCE (`orcid_oauth.rs`), keeping `vox-crypto` strictly Ed25519/ChaCha20/BLAKE3 for app data. |
| **AF-07: Auto-Mutating Canonical `docs/src/architecture/` or `AGENTS.md` from Unreviewed "Next Lessons"** | Self-modifying prompts/docs sound like fast continual learning. | Corrupts repository SSOTs, trips `vox-doc-pipeline` / doctest gates, and creates feedback-loop drift from uncorroborated web text (prompt injection vector). | **Quarantined Lesson Pipeline (`Untrusted` -> Corroborated Promotion):** Sanitize via `SanitizedText` (`kb_text.rs`), stage lessons as `Untrusted` in `kb_findings` / `.proposed` enforcement artifacts (`vox-effort-route`), and require `>=2`-domain corroboration (`kb_inbox.rs`) or explicit human approval before SSOT promotion. |

---

## Feature Dependencies

```
[TS-01: Unified Light ↔ Deep Continuum]
    ├──requires──> [TS-02: 4 Domain Modes (General/CodeGen/Shopping/Scholarly)]
    ├──requires──> [TS-03: Live SSE Stage/Wave/Claim/Cost Streaming]
    └──enhances──> [TS-04: Chat Loquela Knobs & /research Ergonomics]
                       └──enables──> [DF-01: Socrates Auto-Escalation & Calibrated Questioning]

[DF-02: Extreme-Data Streaming (std.frame Chunked Arrow + SSE)]
    └──requires──> [TS-03: Live SSE Stage/Wave/Claim/Cost Streaming]

[TS-05: Calibrated NLI Claim & Citation Verifier]
    ├──enables──> [TS-06: Honest Prior-Art & InsufficientEvidence Guard]
    │                 └──enables──> [DF-03: Two-Stage Novelty × Utility Gate]
    │                                   ├──enables──> [DF-04: Continuous Discovery (Scout/Socrates/Telemetry)]
    │                                   └──requires─> [TS-07: 4-Layer Human Review & 1-Click Promotion]
    │                                                     └──enables──> [TS-08: DB-Hydrated Direct Publication]
    │                                                                       └──enhances──> [DF-05: Replayable Multi-Target Projection]
    └──enables──> [DF-06: Hierarchical Epistemic DAG & Unified Scientia Studio]

[TS-09: Paired Before/After Audit Harness]
    ├──requires──> [TS-01, TS-05, DF-03]
    └──enables──> [DF-07: Inline Diff Inspector + Automated "Next Lesson" Extraction]

[AF-01: Auto-Publish Public Venues] ──conflicts──> [TS-07: 4-Layer Human-Gated Review]
[AF-03: Raw Dataframe Prompt Dumping] ──conflicts──> [DF-02: Extreme-Data Chunked Streaming]
```

### Dependency Notes

- **`DF-01` (Socrates Auto-Escalation) requires `TS-01` + `TS-03` + `TS-04`:** Mid-turn promotion from `<=1`-hop Light to `>1`-wave Deep only works cleanly when Light and Deep share one `ResearchQuery`/trace contract (`TS-01`), stream stage transitions live to the Chat transcript (`TS-03`), and honor session `Loquela` posture overrides (`TS-04`).
- **`DF-03` (Two-Stage Novelty × Utility) requires `TS-05` + `TS-06`:** Stage-2 worthiness scoring (`WorthinessSignalsV2`) is garbage-in/garbage-out until claim grounding (`Contradicted`/`Abstain` reachability in `TS-05`) and HTTP-health-aware `InsufficientEvidence` + real semantic embeddings (`TS-06`) replace mock word-overlap and `0.0` defaults.
- **`TS-08` & `DF-05` (DB-Hydrated Direct Publication) require `TS-07` (1-Click Manifest Promotion & `ApprovalToken`):** `assemble_scaffold_from_db(db, publication_id)` and `render_doc_draft(session_id)` need canonical `publication_manifests` and verified `scientia_claims` rows populated by promotion (`TS-07`), and external archival (`Zenodo`/`OpenReview`) must remain structurally blocked without `ApprovalToken`.
- **`DF-07` (Inline Diff Inspector & "Next Lesson" Flywheel) requires `TS-09` (Before/After Audit Harness):** Inline diffing and automated lesson extraction consume structured paired run deltas (`ResearchEvalSampleRecord` + Socrates ECE + novelty confusion matrix) produced by `TS-09`.

---

## MVP Definition

### Launch With (v7.6 Core — Realistic End-to-End Power, Zero Illusions)

Minimum complete milestone scope required so every v7.6 pillar works truthfully end-to-end with zero hardcoded mocks:

- [ ] **`TS-01` + `TS-02` + `TS-04` — Unified Continuum & 4-Domain Control Surface:** Unify `<=1`-hop Light and `>1`-wave Deep execution across `General`, `CodeGen`, `Shopping`, and `Scholarly`; wire `Loquela` knobs, `/research`, and `ResearchView.tsx` power controls.
- [ ] **`TS-03` + `DF-02` — Live SSE Telemetry & Chunked `std.frame` Streaming:** Ship `/api/v2/research/sessions/{id}/stream` SSE + Chat turn streaming for live stages, waves, claims, real `UsageSnapshot` USD costs, and paginated Arrow/`std.frame` table chunks.
- [ ] **`DF-01` + `TS-05` — Calibrated Socrates Escalation & Facade-Backed Claim/Citation Verification:** Wire `classify_research_intent_with_socrates`, mid-turn `Light -> Deep` auto-escalation, cost-aware clarification (`QuestioningPolicy`), lexical containment pre-filter + `vox_actor_runtime::llm` NLI verification, and post-hoc citation span check.
- [ ] **`TS-06` + `DF-03` + `DF-04` — Two-Stage Novelty × Utility & Continuous Discovery:** Unify `NoveltyEvidenceBundle`, enforce `InsufficientEvidence` on failed prior-art HTTP traces, wire real embeddings (`llm_embed` + local fallback), enforce Stage-2 `WorthinessSignalsV2` utility gates, and upgrade `vox scientia scout` + `SocratesTelemetryProducer` epistemic-gap mining.
- [ ] **`TS-07` + `TS-08` + `DF-06` — Human-Gated Direct Publication (`> Nanopublication`) & Unified Studio:** Fix `discovery_bridge` → `publication_manifests` 1-click promotion, enforce 4-layer `ApprovalToken`, ship DB-hydrated `docs/src/architecture/` SSOT synthesizer (passing `vox-doc-pipeline --lint-only`) + DB-hydrated IMRaD/LaTeX/arXiv/FindingPage assemblers + measured replay writeback, and mount the true hierarchical Epistemic DAG + unified `ScientiaSurface` workbench.
- [ ] **`TS-09` + `DF-07` — Paired Before/After Audit Harness, Inline Diff Inspector & Quarantined Lesson Extraction:** Ship paired `vox research eval --compare` / `vox audit` before/after scorecards across all 6 axes, inline GUI diff inspector, and automated "next lesson" capture into `VoxDb` (`kb_findings` quarantined `Untrusted` → corroborated promotion).

### Add After Validation (v7.6.x Polish & Scale)

Features to layer once the core v7.6 continuum and publication flywheel are verified on real runs:

- [ ] **Cross-Encoder Reranking Pass (`P2` Retrieval Uplift):** Add optional local ONNX/Candle cross-encoder reranker over RRF-fused hits once baseline FACT citation precision is benchmarked in `TS-09` (triggered if multi-hop recall@10 plateaus below target).
- [ ] **SPECTER2 Proximity Adapter for `Scholarly` Domain:** Upgrade local general sentence embeddings (`MiniLM`/`BGE-small`) with optional scientific adapter weights when offline `Scholarly` novelty false-negatives appear in the evaluation harness.
- [ ] **VLM Layout & Figure Critic on Rendered LaTeX PDFs:** Adapt Sakana AI Scientist-v2's visual layout check as an *advisory* `visus_review` pass over rendered Finding Pages / LaTeX figures before human sign-off.

### Future Consideration (v8.0+)

Explicitly deferred beyond v7.6 until governance and multi-node ecosystems warrant them:

- [ ] **Live Production Nanopublication Network Broadcast:** Keep nanopubs offline-validated (`nanopub-testsuite`) + optional default-off test-server only in v7.6; defer live Knowledge Pixels production registry publishing until multi-user `user_identities` + ORCID revocation UX has soaked.
- [ ] **Cross-Peer Federated Discovery Mesh (`Populi` Scientia Gossip):** Sharing anonymized `NoveltyEvidenceBundle` / `FindingCandidate` digests across trusted iroh QUIC mesh peers.

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| **TS-01:** Unified `<=1`-Hop Light ↔ `>1`-Wave Deep Continuum | HIGH | MEDIUM | **P1** |
| **TS-02:** 4 Domain Modes (`General`, `CodeGen`, `Shopping`, `Scholarly`) | HIGH | MEDIUM | **P1** |
| **TS-03:** Live SSE Stage/Wave/Claim/Cost Streaming | HIGH | MEDIUM | **P1** |
| **TS-04:** First-Class Chat Ergonomics (`Loquela` Knobs + `/research`) | HIGH | LOW | **P1** |
| **DF-01:** Socrates Auto-Escalation (`Light -> Deep`) & Calibrated Questioning | HIGH | MEDIUM | **P1** |
| **TS-05:** Facade-Backed NLI Claim Verifier & Post-Hoc Citation Audit | HIGH | MEDIUM | **P1** |
| **TS-06:** Unified `NoveltyEvidenceBundle` + `InsufficientEvidence` + Embeddings | HIGH | MEDIUM | **P1** |
| **DF-03:** Two-Stage Novelty × Utility (`WorthinessSignalsV2`) Filter | HIGH | MEDIUM | **P1** |
| **TS-07:** 4-Layer Human-Gated Review & 1-Click Manifest Promotion | HIGH | LOW | **P1** |
| **TS-08:** DB-Hydrated Direct Publication (Arch SSOT, IMRaD, LaTeX, arXiv, Page) | HIGH | MEDIUM | **P1** |
| **DF-02:** Extreme-Data Chunked Streaming (`std.frame` Arrow + Virtualized UI) | HIGH | HIGH | **P1** |
| **DF-04:** Continuous Autonomous Discovery (Scout + Socrates Gaps + Telemetry) | HIGH | MEDIUM | **P2** |
| **DF-05:** Replayable Multi-Target `RA` Trusty-URI Projection & Archive Run | MEDIUM | MEDIUM | **P2** |
| **DF-06:** True Hierarchical Epistemic DAG & Unified Scientia Studio GUI | HIGH | MEDIUM | **P2** |
| **TS-09:** Paired Before/After 6-Axis Audit Harness (`vox research eval`) | HIGH | MEDIUM | **P1** |
| **DF-07:** Inline Before/After Diff Inspector + Automated `VoxDb`/KB Lessons | HIGH | MEDIUM | **P2** |

**Priority key:**
- **P1:** Must land in early/core v7.6 waves — closes existing broken handoffs/stubs and establishes the verifiable continuum + evaluation spine.
- **P2:** High-leverage differentiator waves immediately atop the P1 spine within v7.6 — completes continuous discovery, studio UX, and the closed-loop lesson flywheel.
- **P3:** Deferred post-v7.6 (`v7.6.x` / `v8.0+`).

---

## Competitor & Prior-Art Feature Analysis

| Capability Dimension | Commercial Deep Research (OpenAI / Perplexity / Gemini / Anthropic) | Sakana *The AI Scientist-v2* (2025–2026) | FutureHouse (*Crow / Falcon / Owl / Finch / Robin*) & Google *AI Co-Scientist* | Vox Milestone v7.6 Approach |
|----------------------|---------------------------------------------------------------------|------------------------------------------|--------------------------------------------------------------------------------|-----------------------------|
| **Light ↔ Deep Continuum & Chat Integration** | Separate "Search" vs slow 5–30m "Deep Research" toggle (OpenAI/Gemini) or Perplexity Effort Dials; no mid-turn epistemic auto-escalation. | Batch offline job only; no interactive chat or `<=1`-hop fast path. | Separate named agents (*Crow* = fast QA, *Falcon* = deep review); user or *Robin* script picks agent. | **Unified Continuum (`TS-01`, `DF-01`):** Single `ResearchQuery` spine from `<2s` 1-hop Light to multi-wave Deep across `General\|CodeGen\|Shopping\|Scholarly`, with **Socrates mid-turn auto-escalation** and `Loquela` knob parity. |
| **Extreme-Data / Tabular Handling** | Truncates CSV/tables into prompt text or opaque server Python sandbox; cannot stream large Arrow frames to local UI. | Runs ad-hoc generated Python scripts on local ML logs; fragile error handling. | *Finch* automates biological data analysis in cloud notebooks. | **Native `std.frame` + SSE Chunk Streaming (`DF-02`):** Lazy Polars/Arrow IPC execution via sandboxed `vox-python-bridge`, streaming paginated batches + invariants to virtualized Chat/Research tables with zero prompt bloat. |
| **Epistemic Calibration & Citation Grounding** | Inline `[N]` links; frequent post-hoc citation drift / unsupported synthesis claims; rarely abstains honestly. | VLM checks figure aesthetics; LLM reviewer checks prose, but susceptible to hallucinated citations and weak baselines. | *Crow/Falcon* use PaperQA chunk-ranking; *AI Co-Scientist* uses test-time debate + literature cross-exam. | **Calibrated Socrates + NLI Cascade (`TS-05`, `DF-01`):** Information-gain questioning (`CLARIFICATION_INTERRUPT_COST_MS`), fast lexical containment pre-filter + `vox_actor_runtime::llm` NLI entailment/contradiction/abstain, Crossref retraction hard-zero, and post-hoc quote verification. |
| **Novelty vs. Utility Detection** | None — summarizes existing web consensus without scoring prior-art novelty. | Optimizes for novelty via BFTS code mutations; criticized for generating "novel but trivial/useless" ML tweaks. | *Owl* ("HasAnyone") checks precedent; *AI Co-Scientist* debates testability + impact with human scientists. | **Two-Stage Novelty × Utility (`TS-06`, `DF-03`):** Stage 1 federated OpenAlex/Crossref/S2 + semantic/MinHash + `InsufficientEvidence` guard; Stage 2 `WorthinessSignalsV2` utility gate requiring replayable benchmark/compile/epistemic value. |
| **Publication Output & Governance** | Stops at chat Markdown / PDF export. | Fully autonomous LaTeX generation; high reputational risk if pointed at live venues without human gating. | Human-collaborative hypothesis reports & preprints (*Robin* dAMD study). | **4-Layer Human-Gated Direct Publication (`TS-07`, `TS-08`, `DF-05`):** Digest-bound `ApprovalToken`, hydrating `VoxDb` directly into `vox-doc-pipeline`-passing Architecture SSOTs, venue LaTeX/arXiv `.tar.gz`, RO-Crate 1.2, HTML Finding Pages, spec-valid `RA` nanopubs, and Zenodo/OpenReview archival. |
| **Before/After Evaluation & Continual Learning** | Static external leaderboards (DeepResearch Bench RACE/FACT, DRACO, BrowseComp, HLE). | Per-paper automated peer-review score; no workspace-level memory ratchet. | Tournament Elo ranking over hypotheses (*AI Co-Scientist*). | **6-Axis Paired Audit + Inline Diff + Lesson Flywheel (`TS-09`, `DF-07`):** Paired statistical scorecards (`paired_ni`), inline GUI diff inspector, and quarantined `kb_findings` / `vox-effort-route` / MENS Auto-DPO extraction. |

---

## Sources

### Internal Vox Codebase & Architecture SSOTs (Verified 2026-11-14)
- `docs/src/architecture/scientia-publication-and-deep-research-claims-vs-reality-audit-2026.md` — 10 concrete backend/GUI gaps (`Gaps 1–10`) and `RES-01..04`, `PUB-01..04`, `SCIGUI-01..04` seams.
- `docs/src/architecture/deep-research-synthesis-and-priorities-2026-08-01.md` — P0–P4 trust/novelty/multi-provider synthesis.
- `docs/src/architecture/scientia-micropublication-ssot-and-surfacing-design-2026.md` — 4-layer no-auto-publish `ApprovalToken` guarantee, `RA` Trusty-URI per-user Clavis RSA custody, and unified `NoveltyEvidenceBundle`.
- `crates/vox-research-shim/src/research/{types.rs,verifier.rs,emitter.rs,discovery_bridge.rs,doc_synthesizer.rs}` — `ResearchDomainMode`, `ResearchQuery`, `BroadcastEmitter`, and candidate-handoff seams.
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/{message.rs,research_intent.rs,research_turn.rs}` & `gui_ops/research.rs` — `/research` intent routing, Socrates hook gap at `message.rs:142`, and `render_doc_draft` stub at `gui_ops/research.rs:199-239`.
- `crates/vox-orchestrator-types/src/socrates_policy/policy_types.rs` & `crates/vox-orchestrator/src/socrates.rs` — `QuestioningPolicy` (`CLARIFICATION_INTERRUPT_COST_MS`), `RiskBand`, and `SessionRetrievalEnvelope`.
- `crates/vox-scientia/src/{producers/socrates_telemetry.rs,producers/worthiness.rs,claim_extractor/minicheck.rs,manuscript/,findings_site/,nanopub/spec.rs}` & `crates/vox-cli/src/commands/scientia_phase_handlers.rs` — discovery producers, `WorthinessSignalsV2`, and manuscript/LaTeX/replay handlers.
- `crates/vox-python-bridge/src/frame_columns.rs` & `crates/vox-compiler/src/eval/frame_plan.rs` — `std.frame` Arrow IPC `StreamReader` batch decoding.
- `crates/vox-cli-research/src/{eval.rs,scout.rs}`, `crates/vox-search/src/kb_inbox.rs`, & `crates/vox-effort-route/src/pipeline.rs` — research evaluation harness, Scout CLI, quarantined KB promotion, and verified lesson routing.
- `crates/vox-gui/ui/src/components/surfaces/{Loquela/Loquela.tsx,Chat/ResearchModeDropdown.tsx,Chat/TaskDiffDialog.tsx,Research/ResearchView.tsx,Scientia/ScientiaSurface.tsx}` — Axis GUI control, DAG, timeline, diff, and studio seams.

### External 2025–2026 Prior Art & Standards
- **FutureHouse (2025–2026):** *Crow, Falcon, Owl, Finch, and Robin* — role-specialized scientific agents (concise QA, deep literature synthesis, precedent/prior-art verification, dataframe analysis, and closed-loop multi-agent hypothesis/discovery workflows).
- **Sakana AI (2025–2026):** *The AI Scientist-v2: Workshop-Level Automated Scientific Discovery via Agentic Tree Search* — template-free BFTS experimentation, VLM manuscript review, and empirical lessons on superficial-novelty filtering and sandboxing.
- **Google Research / DeepMind (2025–2026):** *Towards an AI Co-Scientist* — multi-agent scientific debate, test-time verification scaling, tournament hypothesis ranking, and human-in-the-loop collaborator framing.
- **DeepResearch Bench (2025–2026) & Perplexity DRACO (2025–2026):** Dual-axis evaluation standards separating report comprehensiveness/insight (**RACE**) from citation accuracy and span-level factual grounding (**FACT**), plus real-time effort-dial streaming UX.
- **Nanopublication & Trusty-URI Standards:** Kuhn & Dumontier (*Trusty URIs*, `RA` RDF graph normalization), `Nanopublication/nanopub-testsuite` conformance vectors, and RO-Crate 1.2 specification.

---
*Feature research for: Vox Milestone v7.6 — Unified Light ↔ Deep Research Continuum, Extreme-Data Streaming, Socrates/Novelty Verification & Continuous Direct Publication Flywheel*
*Researched: 2026-11-14*
