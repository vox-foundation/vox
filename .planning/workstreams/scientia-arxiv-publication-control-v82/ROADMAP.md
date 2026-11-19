# Roadmap: Milestone v8.2 (`scientia-arxiv-publication-control-v82`) — Scientia Publication Classification, 7-Stage Lifecycle Control & `arXiv.org`-Quality Manuscript Synthesis

**Workstream:** `.planning/workstreams/scientia-arxiv-publication-control-v82/`
**Scope:** Overhaul how Vox Scientia classifies, controls, gates, synthesizes, and stages scientific publications across all 7 lifecycle stages (`draft -> doi_reserved -> orcid_attributed -> approved -> submitted -> published` + `failed`), culminating in zero-stub `arXiv.org`-compliant scholarly bundles (`main.tex`, `references.bib`, `arxiv-metadata.json`, `UPLOAD-CHECKLIST.md`, validated `.tar.gz` archives) and an interactive Publication Control Studio in `PublicationsView.tsx` and `DirectPublicationDrawer.tsx` (`Phases 483–488`, `24` requirements).

## Phase Checklist

- [x] **Phase 483: Workstream Initialization & Multi-Dimensional Scientific Classification SSOT** (`PUB-CLASS-01..04`)
- [x] **Phase 484: Full `arXiv.org`-Quality Scientific LaTeX, BibTeX (`references.bib`) & Bundle Synthesis** (`PUB-LATEX-ARXIV-01..04`)
- [x] **Phase 485: 7-Stage Lifecycle Controller & `arXiv.org` Readiness Gate Engine (`vox-scientia`)** (`PUB-STAGE-GATE-01..04`)
- [x] **Phase 486: Daemon HTTP Routes, DTO Enrichment & Tauri/Transport Wiring** (`PUB-API-IPC-01..04`)
- [x] **Phase 487: Publications & Scientia GUI Stage Controller, Classification Matrix & `arXiv.org` Studio** (`PUB-STUDIO-UX-01..04`)
- [x] **Phase 488: End-to-End Verification, CV Flywheel Gate & Milestone Closeout** (`PUB-VERIFY-01..04`)

---

### Phase 483: Workstream Initialization & Multi-Dimensional Scientific Classification SSOT

**Directory**: `.planning/workstreams/scientia-arxiv-publication-control-v82/phases/483-scientific-classification-ssot/`
**Goal**: Initialize the `v8.2` workstream and define `FindingClass` alias normalization (`paper`, `scientia`, `benchmark_report`, etc.), `PublicationTier`, and the canonical `ArxivSubjectCategory` catalog + recommendation mapping in `crates/vox-scientia/src/class_routing/`.
**Depends on**: None
**Requirements**: PUB-CLASS-01, PUB-CLASS-02, PUB-CLASS-03, PUB-CLASS-04

### Phase 484: Full `arXiv.org`-Quality Scientific LaTeX, BibTeX (`references.bib`) & Bundle Synthesis

**Directory**: `.planning/workstreams/scientia-arxiv-publication-control-v82/phases/484-arxiv-latex-bibtex-synthesis/`
**Goal**: Implement `crates/vox-scientia/src/manuscript/latex/scientific.rs` with `ScientificPublicationSpec`, full IMRaD section hydration & deterministic empirical autofill (zero `% TODO(narrative)` stubs), `references.bib` synthesis, and `render_scientific_arxiv_bundle` passing `validate_arxiv_bundle_tar_gz` with zero errors/warnings while preserving legacy `render_latex` & `render_arxiv_bundle` tests.
**Depends on**: Phase 483
**Requirements**: PUB-LATEX-ARXIV-01, PUB-LATEX-ARXIV-02, PUB-LATEX-ARXIV-03, PUB-LATEX-ARXIV-04

### Phase 485: 7-Stage Lifecycle Controller & `arXiv.org` Readiness Gate Engine (`vox-scientia`)

**Directory**: `.planning/workstreams/scientia-arxiv-publication-control-v82/phases/485-stage-gate-lifecycle-engine/`
**Goal**: Implement `PublicationLifecycleStage`, 12-check `ArxivReadinessReport` evaluation, `classify_publication_in_db`, and `advance_publication_stage_in_db` across all 7 lifecycle stages (`draft -> doi_reserved -> orcid_attributed -> approved -> submitted -> published` + `failed`), syncing `publication_manifests.state`, `scientia_publication_queue.stage`, and `publication_status_events`, and updating `flywheel_persist_evidence` so advancing flywheel stages advances manifest state out of `draft`.
**Depends on**: Phase 484
**Requirements**: PUB-STAGE-GATE-01, PUB-STAGE-GATE-02, PUB-STAGE-GATE-03, PUB-STAGE-GATE-04

### Phase 486: Daemon HTTP Routes, DTO Enrichment & Tauri/Transport Wiring

**Directory**: `.planning/workstreams/scientia-arxiv-publication-control-v82/phases/486-daemon-dto-ipc-wiring/`
**Goal**: Enrich `PublicationManifestDto` and `get_manifests` to list all scholarly manifests with hydrated classification/stage/readiness metadata, expose `/api/v2/scientia/publications/{id}/{arxiv-readiness,classify,stage-advance}` and `/api/v2/scientia/arxiv-categories`, enrich `/arxiv-bundle` with `references_bib` and `tar_gz_base64`, and wire matching Tauri commands + `transport.ts` + `api/scientia.ts`.
**Depends on**: Phase 485
**Requirements**: PUB-API-IPC-01, PUB-API-IPC-02, PUB-API-IPC-03, PUB-API-IPC-04

### Phase 487: Publications & Scientia GUI Stage Controller, Classification Matrix & `arXiv.org` Studio

**Directory**: `.planning/workstreams/scientia-arxiv-publication-control-v82/phases/487-publications-studio-ux/`
**Goal**: Upgrade `PublicationsView.tsx` and `DirectPublicationDrawer.tsx` with tier/class/arXiv filtering, interactive Classification & Taxonomy controls (`VoxSelect`), 7-stage lifecycle progression actions, live 12-point `arXiv.org` readiness checklist, IMRaD autofill/editor, and multi-artifact preview (`main.tex`, `references.bib`, `arxiv-metadata.json`, `UPLOAD-CHECKLIST.md`) adhering strictly to the `arcane` / `void` theme contract.
**Depends on**: Phase 486
**Requirements**: PUB-STUDIO-UX-01, PUB-STUDIO-UX-02, PUB-STUDIO-UX-03, PUB-STUDIO-UX-04

### Phase 488: End-to-End Verification, CV Flywheel Gate & Milestone Closeout

**Directory**: `.planning/workstreams/scientia-arxiv-publication-control-v82/phases/488-e2e-verification-closeout/`
**Goal**: Format modified Rust crates, pass `cargo test -p vox-scientia --lib`, `cargo test -p vox-orchestrator-mcp --lib gui_ops::scientia`, `cargo test -p vox-gui commands::scientia`, `tsc --noEmit`, Vitest suites (`PublicationsView.test.tsx`, `DiscoveryReview.test.tsx`, `ScientiaSurface.test.tsx`, `ClaimsView.test.tsx`, `ArchivePanel.test.tsx`, `themeContract.test.ts`, `zeroEmojiAndDingbat.test.ts`), and Playwright `CV_STAGE=after` (`kb-research-cv-flywheel.spec.ts`), then close out Milestone `v8.2`.
**Depends on**: Phase 487
**Requirements**: PUB-VERIFY-01, PUB-VERIFY-02, PUB-VERIFY-03, PUB-VERIFY-04
