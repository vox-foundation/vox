# Requirements: Vox — Milestone v8.2 (`scientia-arxiv-publication-control-v82`)

**Defined:** 2026-11-18
**Completed:** 2026-11-18
**Workstream:** `.planning/workstreams/scientia-arxiv-publication-control-v82/`
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree or federated sibling repositories with zero LLM split-brain, while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## v8.2 Requirements

Requirements for Milestone `v8.2` (*Scientia Publication Classification, 7-Stage Lifecycle Control & `arXiv.org`-Quality Manuscript Synthesis*). Every requirement maps to one phase in `ROADMAP.md` (`Phases 483–488`, `24/24` verified).

### Multi-Dimensional Scientific Classification SSOT (`PUB-CLASS`) — Phase 483

- [x] **PUB-CLASS-01**: Update `FindingClass::from_str` in `crates/vox-scientia/src/class_routing/defaults.rs` to normalize case/whitespace and resolve content-type & domain aliases (`paper`, `scientia`, `preprint`, `arxiv`, `research`, `benchmark_report`, `benchmark`, `perf`, `reproducibility`, `governance`, `telemetry`, `negative_result`) instead of failing with `400 invalid_candidate_class`.
- [x] **PUB-CLASS-02**: Introduce `PublicationTier` (`nanopublication`, `technical_report`, `workshop_paper`, `conference_paper`, `arxiv_preprint`, `journal_article`) with IMRaD depth, minimum citation threshold, and class-based recommendations.
- [x] **PUB-CLASS-03**: Introduce the canonical `ArxivSubjectEntry` catalog (`21` subjects across `cs.*`, `stat.*`, `math.OC`, `q-bio.QM`, `physics.comp-ph`, `eess.SY`, `econ.EM`) with validation and automatic primary/cross-list recommendations from `FindingClass`.
- [x] **PUB-CLASS-04**: Re-export `PublicationTier` and `arXiv` catalog helpers cleanly through `crates/vox-scientia/src/class_routing/mod.rs` with full Rust unit test coverage.

### Full `arXiv.org`-Quality Scientific LaTeX, BibTeX & Bundle Synthesis (`PUB-LATEX-ARXIV`) — Phase 484

- [x] **PUB-LATEX-ARXIV-01**: Preserve 100% compatibility of existing `ScaffoldInput`, `render_latex`, and `render_arxiv_bundle` contracts while adding `crates/vox-scientia/src/manuscript/latex/scientific.rs` (`ScientificPublicationSpec`, `ScientificSections`, `ScientificAuthor`).
- [x] **PUB-LATEX-ARXIV-02**: Implement `ScientificPublicationSpec::hydrate_from_manifest` and deterministic empirical `autofill_missing_sections` synthesizing complete IMRaD sections from `ResultsRow` metrics, provenance digests, citations, and commands with zero `% TODO` stubs.
- [x] **PUB-LATEX-ARXIV-03**: Implement `render_references_bibtex` generating valid BibTeX entries (`references.bib`) for all `CitedFact` entries and artifact provenance records, linked via `\cite{...}` and `\bibliographystyle{plainnat}\bibliography{references}`.
- [x] **PUB-LATEX-ARXIV-04**: Implement `render_scientific_arxiv_bundle` packaging `main.tex`, `references.bib`, enriched `arxiv-metadata.json`, `UPLOAD-CHECKLIST.md`, and `figures/*` into a deterministic `.tar.gz` verified cleanly by `validate_arxiv_bundle_tar_gz` with zero errors/warnings.

### 7-Stage Lifecycle Controller & `arXiv.org` Readiness Gate Engine (`PUB-STAGE-GATE`) — Phase 485

- [x] **PUB-STAGE-GATE-01**: Define `PublicationLifecycleStage` (`draft`, `doi_reserved`, `orcid_attributed`, `approved`, `submitted`, `published`, `failed`), `ArxivReadinessCheckItem` (`12` checks), `StageGateSummary`, and `ArxivReadinessReport` (`0..=100` readiness score).
- [x] **PUB-STAGE-GATE-02**: Implement `evaluate_arxiv_readiness_from_db` and `classify_publication_in_db` in `crates/vox-scientia/src/review_flow.rs`, persisting classification/taxonomy/ORCID/DOI/arXiv ID/IMRaD sections in `metadata_json["scientific_publication"]` and `abstract_text`/`body_markdown`.
- [x] **PUB-STAGE-GATE-03**: Implement `advance_publication_stage_in_db` enforcing per-stage gates, syncing `publication_manifests.state`, `scientia_publication_queue.stage`, and `publication_status_events`.
- [x] **PUB-STAGE-GATE-04**: Update `flywheel_persist_evidence` and `flywheel_exec.rs` so completing flywheel stages advances `manifest.state` out of `draft` and studio LaTeX/bundle renders emit full `ScientificPublicationSpec` manuscripts + `references.bib` (passing static source-text guard tests).

### Daemon HTTP Routes, DTO Enrichment & Tauri/Transport Wiring (`PUB-API-IPC`) — Phase 486

- [x] **PUB-API-IPC-01**: Enrich `PublicationManifestDto` (`research.rs`) and `get_manifests` (`scientia.rs`) to return all scholarly manifests hydrated with title, author, classification, tier, `arXiv` categories, SPDX license, ORCID, DOI, `arXiv` ID, and readiness score.
- [x] **PUB-API-IPC-02**: Expose `GET /api/v2/scientia/publications/{id}/arxiv-readiness`, `POST /api/v2/scientia/publications/{id}/classify`, `POST /api/v2/scientia/publications/{id}/stage-advance`, and `GET /api/v2/scientia/arxiv-categories` in `scientia_studio.rs`, plus `references_bib` and `tar_gz_base64` on `/arxiv-bundle`.
- [x] **PUB-API-IPC-03**: Add Tauri commands `scientia_publication_arxiv_bundle`, `scientia_publication_arxiv_readiness`, `scientia_publication_classify`, and `scientia_publication_stage_advance` in `crates/vox-gui/src/commands/scientia.rs` and register them in `crates/vox-gui/src/main.rs`.
- [x] **PUB-API-IPC-04**: Wire transport route mappings (`crates/vox-gui/ui/src/transport.ts`, `lib/pipeline.ts`) and typed TypeScript DTOs/client helpers (`crates/vox-gui/ui/src/api/scientia.ts`).

### Publications & Scientia GUI Stage Controller, Classification Matrix & `arXiv.org` Studio (`PUB-STUDIO-UX`) — Phase 487

- [x] **PUB-STUDIO-UX-01**: Add board filter controls (Publication Tier, Finding Class, `arXiv` Subject Category), `ARXIV READY` header gauge, and multi-dimensional classification/readiness badges on Kanban cards in `PublicationsView.tsx`.
- [x] **PUB-STUDIO-UX-02**: Add interactive Classification & `arXiv.org` Taxonomy Control Tablet, 7-Stage Lifecycle Gate & Progression Controller (`Reserve DOI`, `Bind ORCID`, `Approve for arXiv`, `Submit Validated Bundle`, `Record arXiv Publication`), and 12-Point `arXiv.org` Readiness Checklist in `PublicationsView.tsx`.
- [x] **PUB-STUDIO-UX-03**: Upgrade the preview pane in `PublicationsView.tsx` to inspect `IMRaD Markdown`, `main.tex`, `references.bib`, `arxiv-metadata.json`, and `UPLOAD-CHECKLIST.md`.
- [x] **PUB-STUDIO-UX-04**: Upgrade `DirectPublicationDrawer.tsx` with `PublicationTier`, `arXiv` primary category, `Autofill Scientific IMRaD`, `BibTeX & Bundle`, `Approve for arXiv`, and theme-contract compliant controls (`arcane` / `void`, `h-8`, `>= 11px` font sizes, zero emojis).

### End-to-End Verification, CV Flywheel Gate & Milestone Closeout (`PUB-VERIFY`) — Phase 488

- [x] **PUB-VERIFY-01**: Format changed Rust files with `rustfmt --edition 2024` and pass `cargo test -p vox-scientia --lib`, `cargo test -p vox-orchestrator-mcp --lib gui_ops::scientia`, and `cargo test -p vox-gui commands::scientia`.
- [x] **PUB-VERIFY-02**: Expand `PublicationsView.test.tsx` and `DiscoveryReview.test.tsx` and pass `tsc --noEmit` + targeted Vitest suites (`7/7` files, `28/28` tests).
- [x] **PUB-VERIFY-03**: Pass `VOX_CV_FLYWHEEL=1 CV_STAGE=after` Playwright `kb-research-cv-flywheel.spec.ts` (`16/16` captures, `0` DOM/Axe/contrast/font/overflow defects across `arcane` and `void`).
- [x] **PUB-VERIFY-04**: Update `.planning/workstreams/scientia-arxiv-publication-control-v82/{STATE.md,state.json,REQUIREMENTS.md,ROADMAP.md}` and `.planning/PROJECT.md` marking Milestone `v8.2` shipped.
