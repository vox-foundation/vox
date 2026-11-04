---
title: "SCIENTIA Automatic Publication & Deep Research: Claims vs. Reality Audit & Power Architecture (2026)"
description: "Codebase-grounded audit of the Deep Research (vox-research-shim, vox-search, Axis ResearchView) and SCIENTIA Automatic Publication (vox-scientia, vox-publisher, Axis Scientia/Publications) pipelines, separating shipped reality from stubbed illusions and specifying Phases 52–54."
category: "Architecture SSOTs"
status: "current"
topic: "scientia-research"
domain: "knowledge"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# SCIENTIA Automatic Publication & Deep Research: Claims vs. Reality Audit & Power Architecture (2026)

<!-- kb:summary -->
This document audits the Vox Deep Research (`vox-research-shim`, `vox-search`, `vox-cli-research`, Axis `ResearchView.tsx`) and SCIENTIA Automatic Publication (`vox-scientia-*`, `vox-publisher`, `scientia_phase_handlers.rs`, Axis `ScientiaSurface.tsx` / `PublicationsView.tsx`) pipelines against prior architectural claims. It documents which historical gaps have already been closed in tree, isolates the remaining ten concrete illusions across backend and GUI surfaces, and specifies the three-phase unified milestone (`Phases 52–54`: `RES-01..04`, `PUB-01..04`, `SCIGUI-01..04`) to deliver realistic end-to-end power, live stage and cost observability, and cohesive visual flair.
<!-- /kb:summary -->

## 1. Executive Summary & Correction of Stale 2026 Audits

Earlier audits ([`deep-research-implementation-divergence-audit-2026-08-01.md`](deep-research-implementation-divergence-audit-2026-08-01.md) and [`scientia-self-publication-gap-map-2026.md`](scientia-self-publication-gap-map-2026.md)) flagged several issues that have **since been fixed in the codebase**, while missing deeper structural illusions in the research-to-publication handoff, Architecture SSOT publisher, manuscript/LaTeX scaffolders, and Axis GUI telemetry.

### 1.1 What Actually Shipped Since the August 2026 Audits (Verified in Tree)

| Subsystem | Prior Audit Claim | Current Codebase Reality (2026-10-14) |
|---|---|---|
| **GUI Claim Verification** | "`ResearchView.tsx` omits `verifyClaims: true`, bypassing claim verification." | **Fixed** at `crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx:407`: `startResearchAsync({ query: activeQuery, verifyClaims: true, lane: activeLane })`. |
| **GUI Report Rendering** | "`ResearchView.tsx` renders Markdown inside a plain `<pre>` tag." | **Fixed** at `ResearchView.tsx:792` via `ResearchReportMarkdown.tsx` with clickable `[N]` citation pills and `claimVerdicts`. |
| **CLI Eval Harness** | "`vox research eval` calls `execute_search_plan` instead of the multi-hop pipeline." | **Fixed** at `crates/vox-cli-research/src/eval.rs:54-56`: calls `vox_research_shim::research::run_research_with_context`. |
| **Domain Research Engines (`Shopping` & `CodeGen`)** | "Planned in roadmap." | **Shipped in backend** at `crates/vox-research-shim/src/research/domain/{shopping,codegen}.rs` with Wave 2+ `rustc`/`cargo check` compilation self-correction in `pipeline.rs:677-763` — **but unreachable from the GUI** (`ResearchView.tsx` has no `domain_mode`, `scope`, `waves`, or `site_scope` controls). |
| **Discovery Scout & Archival Run** | "Missing `vox scientia scout` and `publication-archive-run`." | **Shipped** at `crates/vox-cli/src/commands/scout.rs` (`vox scientia scout`) and `crates/vox-cli/src/commands/db/publication/archive_run.rs` (`Zenodo` + `OpenReview` adapters). |

---

## 2. Claims vs. Reality Matrix: The 10 Concrete Gaps Holding Back Realistic Power

```vox-diagram
graph {
    direction: right
    container DR: "Deep Research Pipeline (vox-research-shim)" {
        node Q: "ResearchQuery (General | Shopping | CodeGen)"
        node W: "Multi-Wave Gather & Verify"
        node DB: "VoxDb: research_sessions + research_Session_events" { shape: cylinder }
        node CAND: "discovery_bridge: scientia_finding_candidates"
        Q -> W
        W -> DB
        W -> CAND
    }
    container ILLUSIONS: "Current Broken Handoffs (Gaps 1–5)" {
        node DOC: "docs/src/architecture/*.md"
        node INBOX: "scientia_discovery_inbox"
        node LATEX: "IMRaD / LaTeX / arXiv / FindingPage"
        node CLAIMS: "scientia_claims"
        node COST: "Dashboard & Cost ($0.00)"
        INBOX -> LATEX: "Gap 3: Manual --scaffold X.json required on disk"
        LATEX -> CLAIMS: "Gap 4: MiniCheckBackend::Mock word-overlap default"
    }
    DR.DB -> ILLUSIONS.DOC: "Gap 1: render_doc_draft ignores DB & hardcodes empty vec![]"
    DR.CAND -> ILLUSIONS.INBOX: "Gap 2: finding-* ID written as publication_id (no manifest row!)"
    DR.W -> ILLUSIONS.COST: "Gap 5: UsageSnapshot not wired to scientia_cost_telemetry"
}
```

### 2.1 Automatic Publication Pipeline (`vox-scientia-*` / `vox-publisher` / `vox-cli`)

1. **Gap 1 — "Publish Architecture SSOT" (`render_doc_draft`) is a Hardcoded Stub (`crates/vox-orchestrator-mcp/src/gui_ops/research.rs:199-239`)**:
   - **Claim**: Clicking `"Publish Architecture SSOT"` in `ResearchView.tsx` (`DocPublishModal.tsx`) synthesizes a repository-compliant `docs/src/architecture/<slug>-research-2026.md` SSOT from the completed research session's verified claims, codebase references, citations, and wave stability.
   - **Reality**: `render_doc_draft` never queries `VoxDb` for `session_id`! It constructs a dummy `DocSynthesizerInput` with `stability_score: 0.90`, `executive_summary: "Empirical investigation findings."`, and empty `vec![]` for `verified_claims`, `codebase_refs`, `competitive_matrix`, `sandbox_probes`, `gaps_and_recommendations`, and `roadmap_phases`. Furthermore, the template in `crates/vox-research-shim/src/research/doc_synthesizer.rs:41-52` omits required `topic`, `domain`, `authorship`, `authorship_model`, and `<!-- kb:summary -->` metadata, so any file written by `write_doc` fails `vox-doc-pipeline --lint-only`.
2. **Gap 2 — Research → Publication Handoff Drops Candidate Class & Breaks Inbox Review (`crates/vox-research-shim/src/research/discovery_bridge.rs:82-98, 226-231`)**:
   - **Claim**: High-signal Deep Research runs automatically populate the SCIENTIA Discovery Inbox and can be reviewed and archived from the GUI.
   - **Reality**:
     - `finding_candidate_from_research_result` hardcodes `candidate_class: EventCandidateClass::Other`, `title_hint: None`, and `novelty_evidence_bundle_id: None`.
     - `surface_research_discovery_inbox` passes `candidate.candidate_id` (`"finding-<uuid>"`) into the `publication_id` column of `scientia_discovery_inbox`, even though **no `publication_manifests` row exists** for that ID. Clicking `"Open review"` or `"Archive"` in `DiscoveryInbox.tsx` queries `publication_manifests` for `finding-<uuid>` and fails with `"not_found"`. There is no 1-click/1-command **"Promote Candidate / Research Session to Publication Manifest"** bridge.
3. **Gap 3 — Manuscript, LaTeX, arXiv Bundle, Replay & Finding Page Require Manual Offline JSON Files (`crates/vox-cli/src/commands/scientia_phase_handlers.rs:32-66, 727-890`)**:
   - **Claim**: SCIENTIA automatically drafts IMRaD manuscripts, renders venue-compliant LaTeX (`neurips_2025`, `icml2025`, `acmart`, `jmlr2e`), exports arXiv `.tar.gz` bundles, executes artifact replay, and renders public HTML finding pages.
   - **Reality**:
     - `publication-manuscript-draft --scaffold X.json`, `publication-render-latex --scaffold X.json`, and `publication-arxiv-bundle --scaffold X.json` require a hand-authored `ScaffoldInput` JSON file on disk. There is no `assemble_scaffold_from_db(db, publication_id)` thathydrates `ScaffoldInput` from `publication_manifests` + `scientia_claims` + `novelty_evidence_bundle` + `ro_crate` + `prereg` automatically.
     - `publication-replay-execute` (`scientia_phase_handlers.rs:727-796`) only appends an event `"artifact_replayability_measured"` and **never updates** `metadata_json.scientia_evidence` or `WorthinessInputs::artifact_replayability`.
     - `publication-finding-page-render --page X.json` requires a manual `FindingPageInput` JSON file instead of hydrating from `publication_id`.
4. **Gap 4 — Claim Extraction Defaults to Trivial Word-Overlap Mock (`crates/vox-cli/src/commands/scientia_phase_handlers.rs:304, 332-336`)**:
   - **Claim**: `publication-extract-claims` performs NLI / MiniCheck grounding verification over extracted claims.
   - **Reality**: When `VOX_MINICHECK_ENDPOINT` is unset (the default local setup), `ExtractionPipeline` falls back to `MiniCheckBackend::Mock` (`crates/vox-scientia-claims/src/minicheck.rs`), which computes naive token overlap instead of routing through the workspace's model-agnostic LLM boundary (`vox_actor_runtime::llm::infer_with_retry`).
5. **Gap 5 — Dashboard Confidence & Cost Rollup Blind Spots (`scientia_phase_handlers.rs:230, 391, 577-605`)**:
   - **Claim**: `vox scientia dashboard` and `vox scientia cost` provide real-time candidate confidence, reply-window tracking, and per-stage cost attribution (`extraction`, `novelty`, `critic`, `scholarly`).
   - **Reality**:
     - `scientia_dashboard()` only queries `publication_manifests` with hardcoded `confidence: 0.0`, ignoring `scientia_finding_candidates` rows and hardcoding `manifests_in_reply_window: Vec::new()`.
     - `scientia_cost()` records `0.0_f64` for `extraction` and `critic` and `0.0` for `novelty` and `scholarly`, never reading `vox_research_shim::research::metering::snapshot()` or LLM token telemetry.

### 2.2 Deep Research Pipeline & Axis GUI Observability / Visual Flair (`vox-research-shim` / `vox-gui`)

6. **Gap 6 — `ResearchView.tsx` Hides `Shopping`, `CodeGen`, `Scope`, `Waves`, and `Site` Power Controls**:
   - `ResearchQuery` supports `domain_mode: ResearchDomainMode::{General, Shopping, CodeGen}`, `scope: ResearchScope::{Web, Local, Both}`, `waves: u8` (`1..=5`), and `site_scope: Option<String>`, plus `verify_Compilation` in Wave 2+ for `CodeGen` and weighted decision matrices for `Shopping`. None of these controls exist in `ResearchView.tsx`.
7. **Gap 7 — Static Pipeline Timeline & Orphaned `MultiWaveProgressTimeline.tsx`**:
   - During an active run, `ResearchView.tsx:596` renders `<PipelineTimeline stages={RESEARCH_STAGES} statuses={deriveStages('active')} />`, which freezes stage 0 (`Query Decomposition`) as `'active'` for the entire run instead of reflecting live `session.status` (`planning` → `retrieving` → `verifying_claims` → `auditing_citations` → `synthesizing` → `persisting_artifact` → `completed`) and `research_session_events`.
   - `crates/vox-gui/ui/src/components/surfaces/Research/MultiWaveProgressTimeline.tsx` (with wave stability bars, contradiction counts, and sandbox check badges) is implemented and unit-tested in tree, **but is never imported or mounted anywhere in `ResearchView.tsx`**.
8. **Gap 8 — Synthetic Linear Epistemic DAG (`ResearchView.tsx:762-778`)**:
   - `dagNodes` and `dagEdges` in `ResearchView.tsx` are fabricated client-side by assigning `wave: Math.floor(i / 2)` and connecting `dagNodes[i] -> dagNodes[i + 1]` in a single chain (`chain-${i}`), rather than rendering the true hierarchical epistemic graph (`Query -> Subquery (Wave N) -> Source Domain -> Extracted Claim -> Verification Verdict / Contradiction Edge`).
9. **Gap 9 — Verifier Resampling Ignores Fast Lexical/NLI Pre-Filter & `abstain_threshold` (`crates/vox-research-shim/src/research/verifier.rs:15-26`)**:
   - `VerifierConfig.abstain_threshold` is marked `Phase 0a — fields are placeholders` and ignored; `verify_claims_With_resampling` runs 3 LLM calls on every claim without a fast lexical/BM25/token-containment pre-filter or calibrated abstention band.
10. **Gap 10 — Fragmented Scientia / Discovery / Publications GUI Surfaces**:
    - `ScientiaSurface.tsx` (`scientia` view) only shows `Dashboard | Claims | Harness Issues`.
    - `DiscoverySurface.tsx` (`activity` view) buries `Inbox | Review | Archive` inside the generic Activity timeline.
    - `PublicationsView.tsx` (`publications` view) is a read-only Kanban board that invokes slow CLI subprocesses (`execute_command` running `vox scientia ...`) instead of typed `/api/v2/scientia/*` daemon routes, with zero buttons to promote candidates, extract claims, run novelty/critic gates, preview IMRaD/LaTeX manuscripts, or trigger Zenodo/OpenReview archival.

---

## 3. Target Architecture: Realistic Power, End-to-End Observability & Visual Flair

```vox-diagram
graph {
    container P52: "Phase 52: Deep Research Realistic Power & Live Observability (RES-01..04)" {
        node R1: "RES-01: DB-Grounded DocSynthesizer (kb:summary + taxonomy frontmatter + real claims/citations)"
        node R2: "RES-02: Full ResearchQuery API (domain_mode, scope, waves, site_scope) + Epistemic Graph Builder"
        node R3: "RES-03: Live Stage/Wave Telemetry Stream + UsageSnapshot -> scientia_cost_telemetry"
        node R4: "RES-04: Calibrated Verifier (Lexical/Containment Pre-Filter + abstain_threshold Enforcement)"
    }
    container P53: "Phase 53: SCIENTIA Automatic Publication Closure (PUB-01..04)" {
        node P1: "PUB-01: Candidate Classifier + 1-Click Candidate/Session -> publication_manifests Promotion"
        node P2: "PUB-02: DB-Backed ScaffoldInput & FindingPage Assembler (Zero Manual JSON Files)"
        node P3: "PUB-03: LLM-Backed Claim Grounding Verifier (vox_actor_runtime::llm) + Measured Replay Writeback"
        node P4: "PUB-04: Unified Dashboard (Candidates + Manifests + Reply Window) & Real Cost Rollup"
    }
    container P54: "Phase 54: Unified Research & Scientia Studio GUI (SCIGUI-01..04)" {
        node G1: "SCIGUI-01: Research Power Bar (General|Shopping|CodeGen, Scope, Waves, Site) + Mode Cards"
        node G2: "SCIGUI-02: Live Stage Telemetry Ribbon + Mounted MultiWaveProgressTimeline + Cost Meter"
        node G3: "SCIGUI-03: True Multi-Layer Epistemic DAG Canvas (Query->Subquery->Source->Claim->Verdict)"
        node G4: "SCIGUI-04: Unified Scientia & Publications Workbench (1-Click Promote, Worthiness Radar, Manuscript/LaTeX Preview)"
    }
    P52.R4 -> P53.P1
    P53.P4 -> P54.G1
}
```

### 3.1 Phase 52 (`RES-01..04` — Wave 2 Backend): Deep Research Realistic Power, DB-Grounded Architecture SSOT Synthesis & Live Stage Observability
- **`RES-01` (DB-Grounded Architecture SSOT Synthesizer)**:
  - Upgrade `crates/vox-research-shim/src/research/doc_synthesizer.rs` to emit repository-compliant YAML frontmatter (`title`, `description`, `category: "Architecture SSOTs"`, `status: "current"`, `topic: "scientia-research"`, `domain: "knowledge"`, `authorship: "llm-assisted"`, `authorship_model`) and a 40–120 word `<!-- kb:summary -->` block directly under `# <Title>`.
  - Upgrade `render_doc_draft` in `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` to load the real `research_sessions` row (`query_text`, `report_markdown`, `artifact_json`, `config_json`) and `research_session_events` from `VoxDb`, populating `verified_claims`, `codebase_refs`, `competitive_matrix` (when `Shopping` mode or comparison tables exist), `sandbox_probes` (when `CodeGen` compilation probes exist), `gaps_and_recommendations`, and `wave_stability`.
- **`RES-02` (Full `ResearchQuery` Control Surface & True Epistemic Graph Serializer)**:
  - Extend `StartResearchBody` in `crates/vox-orchestrator-mcp/src/gui_ops/research.rs` and `crates/vox-gui/src/commands/scientia.rs` to accept `domain_mode` (`"general" | "shopping" | "codegen"`), `scope` (`"web" | "local" | "both"`), `waves` (`1..=5`), `site_scope`, and `verify_compilation`.
  - Add a deterministic `build_epistemic_dag(session, events)` helper in `gui_ops/research.rs` that emits typed nodes (`query`, `subquery`, `source_domain`, `claim`, `sandbox_probe`) and typed edges (`decomposes_to`, `retrieves_from`, `grounds_claim`, `contradicts`, `verifies_compile`) on `get_session`.
- **`RES-03` (Live Stage/Wave Event Telemetry & Cost Metering Persistence)**:
  - Emit granular `research_session_events` (`wave_started`, `subquery_executed`, `claim_verified`, `contradiction_detected`, `sandbox_check_completed`, `wave_completed` with `stability_score`, `entropy_delta`, and `UsageSnapshot`) and wire `vox_research_shim::research::metering::snapshot()` into `VoxDb::record_scientia_cost`.
- **`RES-04` (Calibrated Claim Verifier with Fast Containment Pre-Filter & `abstain_threshold`)**:
  - Remove the `PHASE_0a_STUB` placeholders in `crates/vox-research-shim/src/research/verifier.rs`: add a fast lexical/n-gram containment pre-filter that immediately confirms verbatim/high-overlap citations without 3x LLM resampling, routes ambiguous claims through `vox_actor_runtime::llm`, and enforces `VerifierConfig.abstain_threshold` to classify low-margin results as `ClaimLabel::Abstain` rather than silently coercing to `Contradicted`.

### 3.2 Phase 53 (`PUB-01..04` — Wave 2 Backend): SCIENTIA Automatic Publication Pipeline Closure
- **`PUB-01` (Classified Candidate Handoff & 1-Click Candidate/Session → Publication Manifest Promotion)**:
  - Upgrade `finding_candidate_from_research_result` in `crates/vox-research-shim/src/research/discovery_bridge.rs` to infer `EventCandidateClass` (`BenchmarkDelta`, `ArchitectureDecision`, `ReproducibilityAudit`, `CorpusFinding`, etc.), `title_hint`, and `novelty_evidence_bundle_id` from the research artifact.
  - Add `promote_candidate_to_publication(db, candidate_id_or_session_id, venue)` in `crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs` (and CLI `vox scientia candidate-promote --id <id>`) that creates a canonical `publication_manifests` row, links `scientia_discovery_inbox.publication_id` to the real manifest, and seeds initial `scientia_claims` from the research session's verified claims.
- **`PUB-02` (DB-Backed `ScaffoldInput` & `FindingPageInput` Assembler — Zero Manual JSON Files)**:
  - Add `assemble_scaffold_from_db(db, publication_id, venue)` and `assemble_finding_page_from_db(db, publication_id)` so `vox scientia publication-manuscript-draft`, `publication-render-latex`, `publication-arxiv-bundle`, and `publication-finding-page-render` work directly from `--publication-id <id>` (with `--scaffold` kept only as an optional override) and are exposed over `/api/v2/scientia/publications/:id/{manuscript,latex,arxiv-bundle,finding-page}`.
- **`PUB-03` (LLM-Backed Claim Grounding Verifier & Measured Replay Writeback)**:
  - Upgrade `crates/vox-scientia-claims/src/minicheck.rs` & `scientia_phase_handlers.rs` so when `VOX_MINICHECK_ENDPOINT` is unset, claim verification routes through `vox_actor_runtime::llm` (structured entailment prompt) with `MiniCheckBackend::Mock` restricted to deterministic unit tests.
  - Update `publication_replay_execute` (`scientia_phase_handlers.rs:727-796`) to write the measured replayability score (`1.0` on exit 0 + digest match, `0.5` on exit 0 without digest, `0.0` on failure) back into `publication_manifests.metadata_json.scientia_evidence` and recompute `WorthinessInputs::artifact_replayability`.
- **`PUB-04` (Unified Scientia Dashboard & Real Cost Attribution)**:
  - Upgrade `scientia_dashboard` and `/api/v2/scientia/dashboard` to merge `scientia_finding_candidates` (with real `confidence_json` and `candidate_class`), `publication_manifests`, and active reply-window items (`openreview` / `arxiv` manifests with pending comment windows).
  - Wire real `UsageSnapshot` / token-cost telemetry into `scientia_cost_telemetry` across `extraction`, `novelty`, `critic`, and `scholarly` stages.

### 3.3 Phase 54 (`SCIGUI-01..04` — Wave 3 GUI Slot 17): Unified Research & Scientia Publication Studio GUI
- **`SCIGUI-01` (Deep Research Power Controls & Domain Mode Cards in `ResearchView.tsx`)**:
  - Add tasteful, compact power controls to the `ResearchView.tsx` query bar: Domain Mode selector (`General` | `Shopping Matrix` | `CodeGen Verified`), Scope selector (`Web` | `Local Repo` | `Hybrid Both`), Wave budget stepper (`1–3 Waves`), and optional `site:` filter pill.
  - Render dedicated rich cards when `domain_mode === 'shopping'` (weighted criteria & scored options table from `ShoppingDecisionMatrix`) or `domain_mode === 'codegen'` (sandbox `cargo check`/`rustc` compilation badge + repair iteration log).
- **`SCIGUI-02` (Live Stage Telemetry Ribbon, Mounted `MultiWaveProgressTimeline.tsx` & Cost/Latency Pill)**:
  - Poll/stream `getSession(sessionId)` during active research runs so `PipelineTimeline` highlights the true live backend stage (`planning` → `retrieving` → `verifying_claims` → `auditing_citations` → `synthesizing` → `persisting_artifact` → `completed`).
  - Mount `MultiWaveProgressTimeline.tsx` inside the active/completed `ResearchView.tsx` workspace and display a live telemetry header ribbon (`Wave N/M · Stability Δ · Verified Claims X/Y · Cost $Z`).
- **`SCIGUI-03` (Interactive Multi-Layer Epistemic DAG Canvas in `ResearchDagCanvas.tsx`)**:
  - Upgrade `ResearchDagCanvas.tsx` to consume the backend's true epistemic graph (`Query -> Subquery -> Source Domain -> Claim -> Verdict / Contradiction Edge`), with distinct visual encodings for `Supported` (emerald pulse), `Contradicted` (amber/crimson dashed conflict edge), `Abstain` (muted slate), and `CodeGen Probe` (cyan badge), plus click-to-inspect node detail drawer and `"Promote to Scientia Publication"` CTA along the header.
- **`SCIGUI-04` (Unified Scientia & Publications Studio Workbench)**:
  - Unify `ScientiaSurface.tsx`, `DiscoverySurface.tsx`, and `PublicationsView.tsx` into a cohesive studio backed by typed `/api/v2/scientia/*` client methods (`crates/vox-gui/ui/src/api/scientia.ts`):
    - Add `Discovery Inbox`, `Worthiness & Review`, and `Manuscript & Archive Studio` tabs directly inside `ScientiaSurface.tsx`.
    - Add a 1-click **"Promote to Publication Manifest"** button on every `DiscoveryInbox.tsx` candidate and completed `ResearchView.tsx` session.
    - Add interactive stage progression controls on `PublicationsView.tsx` and `DiscoveryReview.tsx` (`Extract & Verify Claims`, `Compute Novelty Bundle`, `Run Critic Gate`, `Preview IMRaD / LaTeX`, `Run Artifact Replay`, and `Archive to Zenodo / OpenReview`) with a visual 6-axis **Worthiness Radar / Meter** (`Epistemic`, `Novelty`, `Replayability`, `Signal-to-Noise`, `Venue Fit`, `Claim Grounding`).
