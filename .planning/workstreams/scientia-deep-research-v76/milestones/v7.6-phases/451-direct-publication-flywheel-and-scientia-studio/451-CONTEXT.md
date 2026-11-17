---
phase: 451
title: "Direct Publication Flywheel and Scientia Studio"
slug: "direct-publication-flywheel-and-scientia-studio"
milestone: "scientia-deep-research-v76"
status: "ready"
---

# Phase 451: Direct Publication Flywheel and Scientia Studio — Context

**Gathered:** 2026-11-15
**Status:** Ready for planning
**Requirements:** PUBL-01, PUBL-02, PUBL-03, PUBL-04, STUD-01, STUD-02

<domain>
## Phase Boundary

Phase 451 ships the end-to-end DB-hydrated Direct Publication (`> Nanopublication`) flywheel and unifies Axis Studio over `/api/v2/scientia/*` with zero `VoxDb` handles or CLI-DB subprocesses in `vox-gui`:
- **Flywheel, SSOT Publishing & Hardened Artifacts (`PUBL-01..04`):** `crates/vox-research-shim/src/research/{discovery_bridge.rs, doc_synthesizer.rs}`, `crates/vox-scientia/src/review/{mod.rs, flywheel_gate.rs, flywheel_exec.rs}`, `crates/vox-scientia/src/review_flow.rs`, `crates/vox-scientia/src/manuscript/{latex/{escape.rs, render.rs, bundle.rs}, scaffold/architecture_ssot.rs}`, `crates/vox-orchestrator-mcp/src/gui_ops/{mod.rs, research.rs, scientia.rs, scientia_studio.rs}`, `crates/vox-cli-research/src/scientia_phase_handlers.rs`.
- **Zero-`VoxDb` Studio & Typed IPC (`STUD-01..02`):** `crates/vox-gui/src/{commands/scientia.rs, main.rs}`, `crates/vox-gui/ui/src/{transport.ts, api/scientia.ts}`, `crates/vox-gui/ui/src/components/surfaces/Publications/{PublicationsView.tsx, PublicationsView.test.tsx}`, `crates/vox-gui/ui/src/components/surfaces/Scientia/{ClaimsView.tsx, ClaimsView.test.tsx, ScientiaSurface.tsx, ScientiaSurface.test.tsx, DiscoveryInbox.tsx, DiscoveryInbox.test.tsx, DiscoveryReview.tsx, DiscoveryReview.test.tsx, WorthinessRadarPanel.tsx, DirectPublicationDrawer.tsx}`.

Deferred to Phase 452: Paired After-Audit Closure, inline Before/After diff inspector, and quarantined lesson extraction (`AUDT-02..04`).
</domain>

<decisions>
## Implementation Decisions

### DB-Hydrated Architecture SSOT & `artifact_json` Fix (`PUBL-02`)
- **Fix `write_research_doc` (`gui_ops/research.rs:715`):** Fetch `db.get_research_artifact(session_id).await` *before* `db.store_research_artifact(...)` and preserve existing `artifact_json` (defaulting to `"{}"` only when absent or whitespace-empty) so `probe_doc_publish_wipes_artifact` passes.
- **In-memory `vox-doc-pipeline` clean SSOT:** `render_doc_draft`, `doc_synthesizer.rs`, and `architecture_ssot.rs` hydrate verified claims, citations, code refs, comparison matrices, and probes from `VoxDb`, emit required frontmatter (`title`, `description`, `category: "Architecture SSOTs"`, `status: "research"`, `topic: "scientia-research"`, `domain: "knowledge"`, `authorship: "llm-assisted"`, `authorship_model`), 40–120 word `<!-- kb:summary -->`, and `vox-diagram` blocks (zero ````mermaid`, zero `// vox:skip`), and validate in-memory before writing `docs/src/architecture/`.

### 1-Click Promotion, 4-Layer `ApprovalToken` Gate & Flywheel (`PUBL-01`, `PUBL-03`, `PUBL-04`)
- **Candidate promotion (`discovery_bridge.rs`):** Populate `EventCandidateClass`, `title_hint`, `novelty_evidence_bundle_id`, and structured `confidence_json` (`signal_strength`, `reproducibility_support`, `contradiction_risk`), eliminating orphan `finding-<uuid>` IDs.
- **4-layer `ExternalEmissionGate` (`review/flywheel_gate.rs` < 400 LoC):** Enforce (1) SHA3-256 content-digest-bound human `ApprovalToken` (via `vox-crypto` facade), (2) `CriticGate` + negative-result quota, (3) `NoveltyVerdict` (`Novel | PossiblyNovel`), and (4) scholarly trust / `OfflineAttested` clearance. Preserve `no_network_publish_symbol_in_review_flow` / `_ops` source-text invariants.
- **Zero-manual-JSON `flywheel_advance_from_db` (`review/flywheel_exec.rs` < 400 LoC):** Idempotently advance `extract_claims -> novelty_check -> scaffold_imrad -> render_latex -> arxiv_bundle -> ro_crate -> finding_page -> replay_verify` from `publication_id` alone (`VoxDb`-backed), persisting `artifact_replayability` and RO-Crate digests into `publication_manifests.metadata_json.scientia_evidence`.
- **Hardened artifacts & cost telemetry:** Strip/escape `\end{verbatim}` and normalize Unicode punctuation/math for `-no-shell-escape` TeX (`escape.rs`, `render.rs`), enforce deterministic `mtime = 0` tar/gzip headers + path-traversal checks (`bundle.rs`), hydrate `nanopub_build` with `novelty_verdict` + `prior_art_uris` (Clavis per-user RSA `RA` Trusty-URI), and surface real confidences, reply windows, and per-stage USD costs in `scientia_phase_handlers.rs` + `gui_ops/scientia_studio.rs`.

### Zero-`VoxDb` Studio & Unified Controls (`STUD-01`, `STUD-02`)
- **Eliminate all CLI-DB spawns:** Remove every `execute_command` call in `PublicationsView.tsx` and `ClaimsView.tsx`; implement and register all 9 typed `BackendCalls` Tauri commands in `commands/scientia.rs` and `main.rs`, wired through `transport.ts` (`safeInvoke`) and `api/scientia.ts`.
- **Radix Studio Panels (`< 400` LoC each):** Add `<WorthinessRadarPanel />` (6-axis SVG radar + Novelty × Utility pill) and `<DirectPublicationDrawer />` (interactive flywheel stepper) mounted in `ScientiaSurface.tsx`, `DiscoveryReview.tsx`, `PublicationsView.tsx`, and `DiscoveryInbox.tsx`, adhering strictly to Radix `data-slot` primitives (`0` raw `<select>` / `<input type="range">`, `0` emojis/dingbats).

### Architectural Invariants
- Zero edits to `Cargo.toml`, `Cargo.lock`, `package.json`, `layers.toml`, or `crate-edges.allow.v1.json`; zero `VoxDb` handles in `vox-gui`; `< 400` LoC per new file; batched Wave F verification.
</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-orchestrator-mcp/src/gui_ops/research.rs:715`: `write_research_doc` overwrites `artifact_json` with `"{}"` when saving a published doc marker.
- `crates/vox-gui/ui/src/components/surfaces/Publications/PublicationsView.tsx:257,271` and `Scientia/ClaimsView.tsx:65-107`: invoke `execute_command` CLI subprocesses that spawn `VoxDb::connect_default()`.
- `crates/vox-gui/ui/src/transport.ts:1889`: already exposes 6 of the 9 `/api/v2/scientia/*` wrappers; only `scientiaPublicationClaims`, `scientiaPublicationVenueRecommend`, and `scientiaPublicationFlywheelAdvance` remain to be added.
</code_context>

<specifics>
## Parallel Wave Boundaries & Batched Verification

- **Wave E parallel file-disjoint workers:**
  1. `p451-flywheel`: `vox-research-shim` (`discovery_bridge.rs`, `doc_synthesizer.rs`) + `vox-scientia` (`review/*`, `review_flow.rs`, `manuscript/*`) + `vox-orchestrator-mcp/src/gui_ops/{mod.rs, research.rs, scientia.rs, scientia_studio.rs}` + `vox-cli-research/src/scientia_phase_handlers.rs`.
  2. `p451-studio-ui`: `vox-gui/src/{commands/scientia.rs, main.rs}` + `vox-gui/ui/src/{transport.ts, api/scientia.ts}` + `Publications/*` + `Scientia/*`.
- **Wave F (batched verification):** Run serialized `cargo test` + `tsc --noEmit` + Vitest (`PublicationsView`, `ClaimsView`, `ScientiaSurface`, `DiscoveryInbox`, `DiscoveryReview`, UI guard suites) + `vox-arch-check` + `vox audit code` after Phase 452 lands.
</specifics>

<deferred>
## Deferred Ideas

- Advisory VLM figure/layout critic (`visus_review`) and live Knowledge Pixels production broadcast (`PUBL-F01`, `PUBL-F02` in `v7.6.x`/`v8.0+`).
</deferred>

---
*Phase: 451-direct-publication-flywheel-and-scientia-studio*
*Context gathered: 2026-11-15*
