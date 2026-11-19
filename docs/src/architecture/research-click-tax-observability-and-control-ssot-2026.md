---
title: "Click Tax Reduction, Research Process Observability & Direct Storage / Search Control (SSOT)"
description: "Visual and interaction audit across Research, Memory, Search Index, Scientia, and Publications surfaces defining above-the-fold report compression, mid-flight wave steering, scoped storage destinations, inline chunk inspection, and zero-ID-typing navigation."
category: "Architecture SSOTs"
status: "current"
topic: "scientia-research"
domain: "knowledge"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Click Tax Reduction, Research Process Observability & Direct Storage / Search Control

<!-- kb:summary -->
Establishes the Milestone v8.5 click-tax, observability, and operator-control specification across Research, Memory, Search Index, Scientia, and Publications surfaces. Compresses redundant header strips so completed research syntheses lead above the fold, exposes observed subqueries, citation domains, and interactive +1 Wave Refinement steering, separates immediate scoped KB indexing (Repo-Local vs. Global Library) from Findings review quarantine, upgrades hit cards with inline expansion and 1-click Research handoff, and eliminates raw publication-ID typing across Scientia panels.
<!-- /kb:summary -->

## Visual Screenshot Audit & Bottlenecks Identified

Direct inspection of rendered dual-theme screenshots (`research-arcane-after.png`, `memory-arcane-after.png`, `scientia-arcane-after.png`, `publications-arcane-after.png`, `scientia-claims-arcane-after.png`, `vox-search-arcane-after.png`) identified five concrete friction points:

1. **Above-the-Fold Pushdown (`ResearchView` & `PublicationsView`):** Stacked `ARCHETYPES`, full-width idle `KB Preflight`, `FLYWHEEL` (duplicating the shell header's `MEMORY | FINDINGS | RESEARCH | DISCOVERY | PUBLICATIONS | SEARCH INDEX` strip), and collapsed `RETRIEVAL PARAMETERS` consumed ~310px, pushing the actual **Research Synthesis Report** below the viewport fold even when a session was active.
2. **Duplicate Search Bar Controls (`MemoryView`):** The top search bar repeated `• MEMORY | • KNOWLEDGE | • CHUNK` chips and `TOP-K 8` immediately above the left tablet's `CORPORA 3/3 ACTIVE` selector tree and `TOP-K DEPTH` rotary dial, crowding the search box and causing dual controls for one state.
3. **Opaque Completed Pipeline & Missing Mid-Flight Steering (`ResearchView`):** Completed sessions showed static green dots (`queued -> completed`) without exposing *which subqueries ran*, *which domains were cited*, or allowing an operator to **steer a `+1 Wave Refinement`** constraint cleanly without starting over from scratch.
4. **Forced Quarantine vs. Immediate Storage Choice (`ResearchView`):** `Save to KB` always sent citations only into quarantine (`not yet searchable`), forcing operators to jump to another panel before recalled notes entered RAG/search; operators need explicit **Scoped Storage Control** (`Repo-Local KB` vs `Global Library` + **`Index Now`** alongside **`Save to KB`** quarantine).
5. **High Click Tax Across Search & Scientia (`MemoryView`, `ClaimsView`, `DiscoveryReview`, `ScientiaDashboard`, `VoxSearchView`):** Recall `HitCard` results truncated to 2 lines with no inline expand and no **`Research Topic`** button; `ClaimsView` and `DiscoveryReview` required manually typing raw `publication id` strings despite DB manifests existing; `ScientiaDashboard` candidates (`cand-100`, `pub-7`, `cand-77`) were static non-clickable text; and `VoxSearchView` showed `"Health: Fresh"` beside `"Graph File: Missing"` with no explanation or quick jump to `MemoryView`.

```vox-diagram
graph {
  direction: right
  node R: "ResearchView: Subquery Telemetry + +1 Wave Steer"
  node S: "Storage Switch: Repo-Local | Global Library -> Index Now or Quarantine"
  node M: "MemoryView HitCard: Inline Expand + Research Topic"
  node C: "Scientia Panels: Clickable Rows + Quick Manuscript Pills"

  R -> S: "completed report or claim"
  S -> M: "instant recall under chosen layer"
  M -> R: "vox_research_seed_query"
  R -> C: "Promote -> 1-click review & Claims"
}
```

## Surface Controls & Reduced Click Tax Architecture

| Surface | Before (Click Tax / Friction) | After (`v8.5` Control Upgrade) |
|---|---|---|
| `ResearchView` | `FLYWHEEL` nav bar + full idle preflight strip + 8-dot timeline push report below fold; no wave steering or direct scoped KB index button | `FLYWHEEL` removed; idle preflight folds cleanly into compact status pill; `research-process-control-strip` displays subqueries, domain breakdown & **`+1 Wave Refinement`** steering; `research-storage-control-bar` offers **`Repo-Local` / `Global Library`** + **`Index Now`** alongside **`Save to KB`** |
| `MemoryView` | Duplicate top `CorpusChip` + `TOP-K` number input; 2-line truncated `HitCard` without expand or research trigger | Single SSOT corpora tree + rotary knob; `HitCard` gains **`Expand` / `Collapse`** inline reader and **`Research Topic`** 1-click launcher |
| `VoxSearchView` | Contradictory `"Health: Fresh"` + `"Graph File: Missing"` empty state; cryptic `TTL`/`LOD` headers | Live Mounted Shard & Corpus Summary + hover tooltips on `TTL (DAYS)`, `LOD`, `SYNC GRAPH` + **`Search Shard in Memory`** button |
| `ClaimsView` & `DiscoveryReview` | Empty text input forcing manual `publication id` lookup and click | 1-click **Recent Manuscripts Quick-Picker** pills (`claims-manifest-quick-picker`, `review-manifest-quick-picker`) populate & load immediately |
| `ScientiaDashboard` | Static text rows for `cand-*`, reply-window `pub-*`, and stalls | Clickable candidate/manifest/stall buttons formatted cleanly and deep-linking straight to `Studio & Review` via `vox://scientia-open-studio` |

## 7. Milestone v8.6: Above-the-Fold Synthesis First, Discovery Dedup & Non-Modal Multi-Format Doc Editor/Parser

Post-v8.5 screenshot inspection across all 8 KB & Research views (`activity-arcane-after.png`, `doc-reader-arcane-after.png`, `research-arcane-after.png`, `memory-arcane-after.png`) identified four remaining structural UX regressions:

1. **Discovery Double Marble Header & Manual Polling Chrome (`DiscoverySurface` / `ActivitySurface`):** Embedding `ActivitySurface` inside `DiscoverySurface` without `EmbeddedSurfaceProvider` rendered two stacked Romanesque marble frieze headers and retained a manual `Refresh` button beside a chronicle header already marked `CHRONICLE LIVE`, alongside raw hex color literals in `DiscoverySurface`. Wrapping presets in `EmbeddedSurfaceProvider` collapses the inner header to a 1-line summary bar, replaces `Refresh` with passive event + `visibilitychange` `Live auto-sync`, and adds 1-click `Open Research` / `Open Memory` / `Open Publications` flywheel jumps.
2. **Research Synthesis Buried Below Process/Storage Strips (`ResearchView`):** Even with completed timelines collapsed, `ResearchView` still stacked the Process/Steer/Storage card above the synthesized report, lacked an inline Domain Mode switcher (`general | codegen | scholarly | shopping`), and lacked instant keyword filtering over past sessions. Promoting `HeadlineVerdictBanner` and `ResearchReportMarkdown` immediately below the active session header puts empirical conclusions above the fold, while `research-history-filter-input` and `research-domain-mode-pill` remove hidden drawer discovery steps.
3. **Memory HitCard Action Wrapping & Fixed Invariant Height (`MemoryView`):** Long button labels (`Research Topic`) caused action rows to wrap onto a second line on two-column grids, hit provenance wasn't surfaced on the card header, and the static Codex `INVARIANT` block consumed ~95px below hits. Labeling provenance (`Memory Shard` / `Code Chunk` / `Codex Canon`), tightening the button label to `Research` (while preserving `research-hit-{i}`), and folding `INVARIANT` into a 1-line `<details>` keeps citation rows compact.
4. **Blocking Modal Dimmer & Read-Only Markdown Drawer (`DocViewerDrawer` & `DocReader`):** Opening any doc trapped keyboard focus inside a `bg-black/50` backdrop (`aria-modal="true"`), blocking the entire Axis workspace behind it, while `ResearchReportMarkdown` omitted GFM tables, frontmatter cards, callout alerts, checklists, AsciiDoc, JSON/JSONL, YAML/TOML, CSV/TSV, and code views, and `DocReader` lacked inline editing + save/revert persistence (`write_doc_markdown`). Converting `DocViewerDrawer` into a non-modal docked right pane (`aria-modal="false"`, Split/Wide/Full widths, Minimize/Expand) alongside `Read | Split | Edit` modes keeps the entire engine interactive during document work.

## 8. Milestone v8.7: Workspace-Wide Non-Modal Doc Dock Routing, Multi-Root Path Resolution & Research-to-Editor Seeding

Auditing cross-surface integration after `v8.6` surfaced three remaining wiring seams between Search, Research, and the non-modal Doc Dock:

1. **Multi-Root Path Resolution (`docs_index.rs`):** `resolve_doc_path` previously forced all relative paths under `docs/src`, causing workspace-relative paths such as `docs/src/reference/cli.md` to double-prefix into `docs/src/docs/src/...` and blocking valid workspace files under `docs/wiki/`, `contracts/`, `crates/`, repo root, and `~/.vox/library/`. `resolve_workspace_doc_path` cleanly resolves repo-relative prefixes against `repo_root`, expands `~/` against `~/.vox/library/`, rejects `.env*` secrets (except `.env.example`), blocks parent-directory traversal (`..`), and supports source extensions (`rs`, `ts`, `tsx`, `sql`) alongside documentation and data formats.
2. **Universal `vox://open-doc` Bus & Unsaved Draft Seeding (`useDocViewer` / `App.tsx` / `DocReader`):** Surfaces outside the Omnibar and Explain Drawer (`MemoryView` hits, `ResearchView` syntheses & KB preflight hits) had no channel to open or seed the non-modal dock in-app. Subscribing `App.tsx` to `vox://open-doc` (`{ path, title, initialContent, initialMode }`), forwarding drafts into `DocReader`, and adding `+ New Doc` (`doc-new-file-btn`) enables 1-click authoring directly beside live engine telemetry.
3. **In-App Memory & Research Loops:** `MemoryView` `HitCard` **Open** routes supported workspace doc/config/code locators directly into the non-modal dock rather than leaving the app, while `ResearchView` adds **`Edit in Doc Dock`** (`open-synthesis-in-doc-dock-btn`) to materialize synthesized Markdown reports with valid frontmatter and `<!-- kb:summary -->` directly in Split mode.
