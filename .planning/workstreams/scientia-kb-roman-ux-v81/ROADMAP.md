# Roadmap: Milestone v8.1 (`scientia-kb-roman-ux-v81`) — Research & Knowledge Base Romanesque/Slate UX Overhaul, Progressive Disclosure, Click-Tax Reduction & Concept-Art CV Flywheel

**Workstream:** `.planning/workstreams/scientia-kb-roman-ux-v81/`
**Scope:** Systematically audit and overhaul all Vox Research (`research`, `scientia`, `publications`, `activity`/Discovery) and Knowledge Base (`memory`, `vox-search`/VoxGraph, `DocReader`) GUI surfaces across `Phases 477–482` (`24` requirements) to eliminate generic/Gemini-like purple-indigo-cyan-black-alpha palette leaks, unreadable micro-fonts, unframed button height spread, flat vertical card sprawl, and high click tax. Enforces the dual-mode Romanesque (`arcane`) & Slate (`void`) design contract, hierarchical progressive disclosure, zero raw emojis/dingbats, freshly generated Concept Art `v2` plates (`image-generator`), and Playwright `@ngi/platform-ssot` CV Flywheel + `/gsd-ui-review` Before/After convergence.

## Phase Checklist

- [x] **Phase 477: Critical Baseline Audit, Stricter DOM/CV Detectors & "Before" Screenshot Capture** (`KB-RES-AUDIT-01..04`)
- [x] **Phase 478: Zero-Emoji Concept Art Generation (`image-generator`) & Romanesque SVG Medallions** (`KB-RES-CONCEPT-01..04`)
- [x] **Phase 479: Overhaul Research Surfaces (`ResearchView`, `ResearchDagCanvas`, Tables, Inspector & Drawer)** (`RES-UX-OVERHAUL-01..04`)
- [x] **Phase 480: Overhaul Scientia, Publications & Discovery Surfaces** (`SCIENTIA-PUB-UX-01..04`)
- [x] **Phase 481: Overhaul Knowledge Base Surfaces (`MemoryView`, `VoxGraphStatusPanel`, `DocReader`)** (`KB-UX-OVERHAUL-01..04`)
- [x] **Phase 482: Verification, CV Flywheel Convergence (`CV_STAGE=after`) & `/gsd-ui-review` Before/After Audit** (`KB-RES-CV-REVIEW-01..04`)

---

### Phase 477: Critical Baseline Audit, Stricter DOM/CV Detectors & "Before" Screenshot Capture

**Directory**: `.planning/workstreams/scientia-kb-roman-ux-v81/phases/477-baseline-audit-and-detectors/`
**Goal**: Capture baseline Before screenshots (`CV_STAGE=before`) across Research and KB surfaces in `arcane` and `void` themes and upgrade `crates/vox-gui/ui/e2e/review/audits.ts` to strictly catch purple/indigo/cyan/teal/black-alpha wave leaks, light-cream-on-Carrara contrast washout, toolbar button height spread, and sub-floor typography.
**Depends on**: None
**Requirements**: KB-RES-AUDIT-01, KB-RES-AUDIT-02, KB-RES-AUDIT-03, KB-RES-AUDIT-04

### Phase 478: Zero-Emoji Concept Art Generation (`image-generator`) & Romanesque SVG Medallions

**Directory**: `.planning/workstreams/scientia-kb-roman-ux-v81/phases/478-zero-emoji-concept-art/`
**Goal**: Generate 4 zero-emoji Romanesque (`arcane`) and Slate (`void`) Concept Art reference plates (`v2` matching `v74` Vox Axis architectural grammar) for Scientia Research and Knowledge Base Codex via `image-generator` and wire them into `vlmVisionFlywheel.ts`.
**Depends on**: Phase 477
**Requirements**: KB-RES-CONCEPT-01, KB-RES-CONCEPT-02, KB-RES-CONCEPT-03, KB-RES-CONCEPT-04

### Phase 479: Overhaul Research Surfaces (`ResearchView`, `ResearchDagCanvas`, Tables, Inspector & Drawer)

**Directory**: `.planning/workstreams/scientia-kb-roman-ux-v81/phases/479-research-surface-overhaul/`
**Goal**: Eliminate purple/indigo/cyan/black-alpha wave leaks in `ResearchDagCanvas.tsx` and `ResearchView.tsx`, introduce 1-click workspace switching (`Workbench Overview`, `Synthesis & Citations`, `DAG & Claims`, `Diagnostics & Waves`) with progressive disclosure and categorized citations, unify `h-8`/`h-9` control framing, and enforce readable `--font-serif`/`--font-sans`/`--font-display` typography.
**Depends on**: Phase 478
**Requirements**: RES-UX-OVERHAUL-01, RES-UX-OVERHAUL-02, RES-UX-OVERHAUL-03, RES-UX-OVERHAUL-04

### Phase 480: Overhaul Scientia, Publications & Discovery Surfaces

**Directory**: `.planning/workstreams/scientia-kb-roman-ux-v81/phases/480-scientia-publications-discovery-overhaul/`
**Goal**: Remediate light-on-Carrara cream hex leaks (`#F3E5AB`, `#D6CEBF`, `#C8B99E`) and dark-only/cyan/emerald/rose leaks across `ScientiaSurface`, `ScientiaDashboard`, `ClaimsView`, `DiscoveryInbox`, `DiscoveryReview`, `ArchivePanel`, `BeforeAfterAuditInspector`, `PublicationsView`, `DiscoverySurface`, and `ActivitySurface`; unify button heights, elevate prose readability, and fold raw metadata/hunks behind progressive disclosure.
**Depends on**: Phase 478
**Requirements**: SCIENTIA-PUB-UX-01, SCIENTIA-PUB-UX-02, SCIENTIA-PUB-UX-03, SCIENTIA-PUB-UX-04

### Phase 481: Overhaul Knowledge Base Surfaces (`MemoryView`, `VoxGraphStatusPanel`, `DocReader`)

**Directory**: `.planning/workstreams/scientia-kb-roman-ux-v81/phases/481-knowledgebase-surfaces-overhaul/`
**Goal**: Unify search/filter bar geometry (`h-8`/`h-9`), add clear categorized corpus/source badges and non-hidden 1-click Pin/Open actions in `MemoryView.tsx`, add collapsible progressive disclosure for dense HNSW shard telemetry, modernize `VoxGraphStatusPanel.tsx` and `CodeAtlasCanvas.tsx` shard/TTL framing, and frame `DocReader.tsx` and `DocReaderPanels.tsx` with a readable `max-w-[74ch]` scholarly measure, unclipped single-row tabs, and 1-click facet filters.
**Depends on**: Phase 478
**Requirements**: KB-UX-OVERHAUL-01, KB-UX-OVERHAUL-02, KB-UX-OVERHAUL-03, KB-UX-OVERHAUL-04

### Phase 482: Verification, CV Flywheel Convergence (`CV_STAGE=after`) & `/gsd-ui-review` Before/After Audit

**Directory**: `.planning/workstreams/scientia-kb-roman-ux-v81/phases/482-cv-flywheel-and-ui-review/`
**Goal**: Run all GUI unit/contract tests (`themeContract`, `zeroEmojiAndDingbat`, `audits`, `v79CvFlywheelConvergence`, component suites) and `CV_STAGE=after` Playwright CV Flywheel across `arcane` and `void` themes, produce `482-UI-REVIEW.md` 6-pillar audit, stage Before/After images in the session artifact directory, and present inline Before/After visual comparisons.
**Depends on**: Phase 479, Phase 480, Phase 481
**Requirements**: KB-RES-CV-REVIEW-01, KB-RES-CV-REVIEW-02, KB-RES-CV-REVIEW-03, KB-RES-CV-REVIEW-04
