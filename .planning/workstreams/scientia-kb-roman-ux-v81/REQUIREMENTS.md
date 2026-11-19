# Requirements: Vox — Milestone v8.1 (`scientia-kb-roman-ux-v81`)

**Defined:** 2026-11-17
**Completed:** 2026-11-18
**Workstream:** `.planning/workstreams/scientia-kb-roman-ux-v81/`
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree or federated sibling repositories with zero LLM split-brain, while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## v8.1 Requirements

Requirements for Milestone `v8.1` (*Research & Knowledge Base Romanesque/Slate UX Overhaul, Progressive Disclosure, Click-Tax Reduction & Concept-Art CV Flywheel*). Every requirement maps to one phase in `ROADMAP.md` (`Phases 477–482`, `24/24` mapped and verified).

### Baseline Audit & Critical CV/DOM Detectors (`KB-RES-AUDIT`) — Phase 477

- [x] **KB-RES-AUDIT-01**: Capture baseline "Before" Playwright screenshots (`CV_STAGE=before`) across `research`, `scientia`, `scientia-claims`, `publications`, `activity` (Discovery), `memory`, `vox-search` (VoxGraph / Search Index), and `doc-reader` in both `arcane` (Imperial Carrara White & Gold) and `void` (Minimalist Basalt Slate) themes, copying them to the session artifact directory.
- [x] **KB-RES-AUDIT-02**: Expand `auditDomQualityInPage` in `crates/vox-gui/ui/e2e/review/audits.ts` so `digitalColorLeakCount` catches hardcoded purple/indigo/cyan/teal/black-alpha wave palette leaks (`rgb(46, 16, 101)`, `rgb(30, 27, 75)`, `rgb(8, 51, 68)`, `rgb(4, 47, 46)`, `rgb(6, 78, 59)`, dark indigo/cyan hexes).
- [x] **KB-RES-AUDIT-03**: Expand `lightThemeContrastLeakCount` in `crates/vox-gui/ui/e2e/review/audits.ts` to catch hardcoded light-cream text hexes (`#F3E5AB`, `#D6CEBF`, `#C8B99E`) rendered on light Carrara/Glacier backgrounds outside dark frieze headers.
- [x] **KB-RES-AUDIT-04**: Add toolbar/tablist height-spread (`surfaceToolbarHeightSpreadPx`) and unreadable button/prose font checks (`< 10.5px` floor) across `main [role="tablist"], main [role="group"], main [data-testid="kb-library-tabs"]` in `audits.ts` so button framing variance cannot silently pass.

### Zero-Emoji Concept Art & Visual Reference Plates (`KB-RES-CONCEPT`) — Phase 478

- [x] **KB-RES-CONCEPT-01**: Generate `v81-concept-scientia-research-carrara.jpg` (`v2`) via `image-generator` (zero emojis, zero dingbats) matching Vox Romanesque frieze/pillar/brass-gauge grammar for the Imperial Carrara White & Gold (`arcane`) Deep Research & Epistemic DAG Workbench.
- [x] **KB-RES-CONCEPT-02**: Generate `v81-concept-scientia-research-slate.jpg` (`v2`) via `image-generator` illustrating the Minimalist Basalt Slate (`void`) Deep Research & Epistemic DAG Workbench.
- [x] **KB-RES-CONCEPT-03**: Generate `v81-concept-kb-codex-carrara.jpg` and `v81-concept-kb-codex-slate.jpg` (`v2`) via `image-generator` (zero emojis) illustrating the Knowledge Base, Memory Shards & Taxonomic Codex Reader in `arcane` and `void`.
- [x] **KB-RES-CONCEPT-04**: Wire `conceptArtPlateForSurface` in `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts` to resolve `research`, `scientia`, `scientia-claims`, `publications`, and `activity` to `v81-concept-scientia-research-${variant}.jpg`, and `memory`, `doc-reader`, `vox-search`, and `vox-graph` to `v81-concept-kb-codex-${variant}.jpg`, and stage plates in the session artifact directory.

### Research Surfaces Overhaul (`RES-UX-OVERHAUL`) — Phase 479

- [x] **RES-UX-OVERHAUL-01**: Replace all hardcoded digital wave hexes (`#2e1065`, `#1e1b4b`, `#083344`, `#042f2e`, `#064e3b`) and `bg-black/*` / `text-emerald-400` / `text-red-400` in `ResearchDagCanvas.tsx` with theme-contract CSS tokens and Cinzel/serif/sans typography.
- [x] **RES-UX-OVERHAUL-02**: Refactor `ResearchView.tsx` and child panels (`VirtualizedClaimsTable.tsx`, `VirtualizedFrameChunkTable.tsx`, `MultiWaveProgressTimeline.tsx`, `HeadlineVerdictBanner.tsx`, `JudgeInspector.tsx`, `LiveSourceProber.tsx`, `ResearchReportMarkdown.tsx`) to eliminate all `bg-black/*`, `text-cyan`, `bg-cyan/*`, and `text-amber-300/90` leaks.
- [x] **RES-UX-OVERHAUL-03**: Re-architect `ResearchView.tsx` around a 1-click workspace switcher (`Workbench Overview`, `Synthesis & Citations`, `DAG & Claims`, `Diagnostics & Waves`) with categorized evidence clusters and `<details>` progressive disclosure so users avoid scrolling through unclustered monolithic cards.
- [x] **RES-UX-OVERHAUL-04**: Standardize all `Research` buttons, filter pills, and inputs to uniform `h-8` / `h-9` heights with `data-slot="button"`, readable prose in `--font-serif`/`--font-sans`, and preserve 100% compatibility with `ResearchView.test.tsx`.

### Scientia, Publications & Discovery Surfaces Overhaul (`SCIENTIA-PUB-UX`) — Phase 480

- [x] **SCIENTIA-PUB-UX-01**: Eliminate light-on-light hardcoded cream hexes (`#F3E5AB`, `#D6CEBF`, `#C8B99E`, `bg-[#78350F]/20`) across `ScientiaSurface.tsx`, `ScientiaDashboard.tsx`, `PublicationsView.tsx`, and `DiscoverySurface.tsx` outside `<RomanSurfaceHeader>`, ensuring crisp contrast in both `arcane` and `void` themes.
- [x] **SCIENTIA-PUB-UX-02**: Eliminate dark-only/cyan/emerald/rose hardcoded classes (`bg-black/30`, `bg-cyan/*`, `text-cyan`, `text-violet-300/90`, `text-emerald-300`, `text-rose-200/90`) across `ClaimsView.tsx`, `DiscoveryInbox.tsx`, `DiscoveryReview.tsx`, `ArchivePanel.tsx`, `BeforeAfterAuditInspector.tsx`, and `ActivitySurface.tsx`, replacing them with `--color-status-*`, `--color-bg-elevated`, and `brass`/gold semantic tokens.
- [x] **SCIENTIA-PUB-UX-03**: Upgrade prose/control typography (`--font-display` headers, `--font-serif`/`--font-sans` readable claims and descriptions, `--font-terminal-mono` reserved for IDs, DOIs, SWHIDs, and numeric metrics) and unify button/tab heights (`h-8`) plus `BreadcrumbBar.tsx` `>= 11px` font floor.
- [x] **SCIENTIA-PUB-UX-04**: Apply progressive disclosure (`<details>` collapsible secondary metadata, field provenance, and raw diff hunks) and categorized epistemic clusters across Scientia, Publications (`stageFilter` & lifecycle stage cards), Worthiness radar SVG, and Discovery.

### Knowledge Base Surfaces Overhaul (`KB-UX-OVERHAUL`) — Phase 481

- [x] **KB-UX-OVERHAUL-01**: Unify `MemoryView.tsx` tab strip, search bar, corpus toggle chips (`h-8`/`h-9`), and health indicator tokens (`--color-status-pass`/`error`), and surface clear `Open` + 1-click `Pin` actions on `HitCard` without hiding primary actions behind hover-only opacity.
- [x] **KB-UX-OVERHAUL-02**: Improve `MemoryView.tsx` categorization & progressive disclosure: group hits/corpora clearly, elevate hit snippets to readable `--font-serif` `13px`, and wrap dense HNSW Vector Shards sparklines in a collapsible `<details>` disclosure region when browsing hits.
- [x] **KB-UX-OVERHAUL-03**: Overhaul `VoxGraphStatusPanel.tsx` and `CodeAtlasCanvas.tsx` with uniform `h-8` TTL/filter controls, crisp semantic status framing, categorized health badges, and progressive disclosure of stale-reason CLI rebuild commands inside shard cards.
- [x] **KB-UX-OVERHAUL-04**: Overhaul `DocReader.tsx`, `DocReaderPanels.tsx`, `OpenWikiHealStudio.tsx`, `OpenWikiDiffPane.tsx`, and `EmpiricalVerificationBadge.tsx` with a readable `max-w-[74ch]` scholarly measure, unclipped single-row tablist, uniform taxonomy facet filters, Romanesque framed callouts, and 1-click finding promotion/rejection controls.

### Verification, CV Flywheel Convergence & `/gsd-ui-review` (`KB-RES-CV-REVIEW`) — Phase 482

- [x] **KB-RES-CV-REVIEW-01**: Pass Vitest contract and component test suites (`themeContract.test.ts`, `zeroEmojiAndDingbat.test.ts`, `audits.test.ts`, `v79CvFlywheelConvergence.test.ts`, `BreadcrumbBar.test.tsx`, `ResearchView.test.tsx`, `ResearchDagCanvas.test.tsx`, `MemoryView.test.tsx`, `ScientiaSurface.test.tsx`, `ClaimsView.test.tsx`, `ArchivePanel.test.tsx`, `BeforeAfterAuditInspector.test.tsx`, `DocReader.test.tsx`, `EmpiricalVerificationBadge.test.tsx`, `VoxGraphStatusPanel.test.tsx`, `ActivitySurface.test.tsx`, plus TypeScript `tsc --noEmit`) with 0 failures (`16/16` files, `125/125` tests).
- [x] **KB-RES-CV-REVIEW-02**: Execute post-overhaul Playwright + `@ngi/platform-ssot` CV Flywheel verification (`CV_STAGE=after`) across all Research & Knowledge Base surfaces in both `arcane` and `void` themes, achieving `0` `digitalColorLeakCount`, `0` `lightThemeContrastLeakCount`, `0` `rawEmojiCount`, `0` `unreadableSmallFontCount`, `0` `surfaceToolbarHeightSpreadPx`, `0` `axeViolations`, `16/16` `flywheelConverged: true`, and `visualAfterScore >= 0.9237`.
- [x] **KB-RES-CV-REVIEW-03**: Produce a 6-pillar `/gsd-ui-review` audit report (`482-UI-REVIEW.md`) scoring Copywriting, Visuals, Color, Typography, Spacing, and Experience Design across the Research & Knowledge Base suite.
- [x] **KB-RES-CV-REVIEW-04**: Stage Before & After PNG comparisons and Concept Art `v2` plates in the session artifact directory, build a Before/After carousel artifact (`v81_research_kb_before_after_review.md`), and present inline Before/After visual comparisons and verified metric tables to the operator.
