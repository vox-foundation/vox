---
title: "Axis GUI v4.0 Progressive Disclosure, Surface Deduplication & Dynamic CV Scanner SSOT (2026)"
description: "Architectural blueprint and CV audit for Milestone v4.0 reorganizing the Axis desktop GUI through progressive disclosure, control deduplication, 5-domain navigation, and dynamic surface auto-discovery."
category: "Architecture SSOTs"
status: "current"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# Axis GUI v4.0 Progressive Disclosure, Surface Deduplication & Dynamic CV Scanner SSOT (2026)

<!-- kb:summary -->
This document records the comprehensive Computer Vision (CV) surface audit and Milestone `v4.0` reorganization blueprint for the Vox Axis desktop application (`crates/vox-gui/ui`). It catalogs all control duplications, multi-row toolbar wraps, and horizontal clipping defects discovered across the 26+ registered GUI surfaces and dock panels, specifies the dynamic runtime auto-discovery scanner in `e2e/review/cv-flywheel.spec.ts`, and defines the single-row progressive-disclosure `Loquela` composer, the 3-tab Session Inspector Rail (`Plan & Tasks`, `Context`, `Routing`), and the consolidated 5-domain workbench navigation.
<!-- /kb:summary -->

## 1. CV Audit Findings & Control Deduplication Matrix

Live Playwright CV captures (`crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts`) across `chat-default`, `loquela-knobs`, `chat-dock-multi`, and all leaf surfaces revealed three primary structural problems:

1. **4-Row Composer Toolbar Wrapping (`Loquela.tsx` + `DriveConsole.tsx` + `ChatSurface.tsx`)**: Over 20 simultaneous interactive pills (`+`, `link`, `mic`, `Free`, `Efficient`, `Balanced`, `Genius`, inline `Spend` bar, `Risk`, `Intent`, `Run on Auto`, `Skill`, `Quick chat`, `Oversight`, `@kb:auto`, `Live` summary badge, `Keys Alt+/`, `Research: Auto Socrates`, `Pinned Context`, `Notifications`) stacked across 4 horizontal rows.
2. **Simultaneous Multi-Surface Duplication**: Identical state and controls were duplicated up to 5 times within a single viewport.
3. **Horizontal Button Clipping & Below-the-Fold Overflow**: In `ContextWindowMeter.tsx` (at 340px right-rail width), `balanced ↻` plus `[Balanced | Compact | Keep more]` pushed `[Clear Tools]` into horizontal text clipping (`Clear T...`). On `Dashboard.tsx`, oversized KPI cards and the simulation mini-map pushed `Active Agents` and `Telemetry` below the `900px` fold.

| Capability / State | Legacy Duplicate Locations (`v3.9`) | `v4.0` Canonical Single Source of Truth |
|---|---|---|
| **Pinned `@kb` Context & Scope** | 4 locations: `Loquela` `@kb` select, `ChatSurface` bottom `Pinned Context` drawer, `ChatExecutionRail` `PINNED CONTEXT` card, `Loquela` top chip bar | **Inspector Rail `Context` Tab** + compact `[@kb: N pins ▾]` quick-trigger pill in composer |
| **Session & Engine Spend** | 4 locations: `DriveConsole` inline bar, `ChatExecutionRail` sparkline, `BottomStatusBar` `SPEND` popover, `Dashboard` 3 budget cards | **`BottomStatusBar` (`Session $X / $Y` + popover)** + composer per-turn `~$cost` estimate |
| **Posture & Model Routing** | 5 locations: `DriveConsole` + `Loquela` selects, `Live` badge row, `ChatExecutionRail` `ROUTING` + `Goal Anchor` badge, `BottomStatusBar` `ROUTING`, `Matrix` vs `ModelsView` vs `RunsView` | **Composer `[⚡ Posture & Model ▾]` Floating Popover** + Inspector Rail `Routing` Tab |
| **Tasks, Steps & Plans** | 3 locations: `ChatExecutionRail` `EXECUTION` list, `PlanPanel` (`To-dos` dock panel), `TasksView` | **Inspector Rail `Plan & Tasks` Tab** (unifying `Execution` and `PlanPanel` with CAS `Synced · v{rev}` and circular progress ring) |
| **Sub-Surface Navigation** | 2 simultaneous menus: expanded vertical child list in `Sidebar.tsx` + horizontal tab strip in `BreadcrumbBar.tsx` | **Left `Sidebar.tsx` = 5 top-level domains only**; child surfaces render exclusively in the **Top Workbench Header Tab Bar** |
| **Dock Layout & Panel Toggles** | 3 locations: `BottomStatusBar` 14-checkbox `Panels ▾`, `ChatExecutionRail` bottom `Terminal/Browser/Research` buttons, `Sidebar` | **Chat Top Header Layout Switcher**: `Layout: [Focus] [Split Code] [Multi-Agent] [+ Panel ▾]` |

---

## 2. Dynamic Surface & Sub-Panel Auto-Discovery (`Phase 210`)

To guarantee that every current and future GUI surface, dock panel, child tab, and popover is automatically verified without manual test array maintenance, `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` introspects the live runtime manifests:

```vox-diagram
graph {
    direction: right
    node reg: "SURFACE_REGISTRY\n+ surfaceComponents\n+ CHAT_DOCK_COMPONENTS"
    node crawler: "Playwright Dynamic\nSurface Crawler"
    node subtabs: "Auto-Discover Child\n[role='tab'] & Popovers"
    node detectors: "DOM Clipping, Row-Wrap\n& Duplicate Detectors"
    node report: "cv-flywheel-all-surfaces.json\n+ Concept Art v4.0 Gate"
    reg -> crawler
    crawler -> subtabs
    subtabs -> detectors
    detectors -> report
}
```

- **Zero Hardcoded View Lists**: Union of `SURFACE_REGISTRY`, `surfaceComponents` route keys, and `CHAT_DOCK_COMPONENTS` dock panel keys.
- **Sub-Tab & Popover Crawling**: Automatically queries and activates `[role="tab"]`, `[data-cv-subsurface]`, and `[data-cv-popover-trigger]` on every visited view.
- **Automated Layout Invariants**:
  - Horizontal text clipping guard: `scrollWidth <= clientWidth + 2` on all buttons and pills.
  - Composer single-row budget: `offsetHeight <= 52px` at `>= 900px` pane width.
  - Above-the-fold visibility on primary workbench cards at `1440×900`.

---

## 3. Progressive Disclosure & 5-Domain Architecture (`Phases 211–215`)

1. **Single-Row `Loquela` v4.0 Composer (`Phase 211`)**:
   - Bottom action bar: `[+ Attach ▾]` (`File`, `URL`, `Voice`), `[⚡ Balanced · <Model> ▾]` (floating glassmorphic **Execution Posture & Routing** popover housing Clutch, Risk, Oversight `1..5`, Model/Tier override, Research mode, and Grounding check), `[🛠 Track & Skill ▾]` (`Chat | Task | Plan`, `Skill`, `Intent`), `[@kb: N pins ▾]`, `~$cost`, and `[▶ Send ↵]`.
   - 100% preservation of `Alt+1..4`, `Alt+C/R/T/M/V/O/K`, `Alt+/` (`KeybindHud`), and all `432` `buildChatTurn` RPC matrix combinations.
2. **Unified 3-Tab Session Inspector Rail (`Phase 212`)**:
   - Segmented header: `[Plan & Tasks]` | `[Context]` | `[Routing]` with a compact collapsible turn-receipt footer.
3. **5-Domain Workbench Navigation (`Phase 213`)**:
   - `Chat` (with inline session list)
   - `Agents & Runs` (`Overview & Health`, `Needs You`, `Tasks & Flow`, `Sub-Agents`, `Run Ledger`, `Policies & Approvals`)
   - `Workspace` (`Console`, `Repository`, `Browser`, `Skills & Catalog`)
   - `Knowledge` (`Memory`, `Search Index`, `Scientia Research`)
   - `Compute & System` (`Models & Routing Matrix`, `MENS Training`, `Mesh`, `Market`, `Settings`)
4. **Concept Art Reference Assets**:
   - Stored in `crates/vox-gui/ui/artifacts/cv-flywheel/concept-art-v4/axis-v4-chat-composer-concept.jpg` and `axis-v4-workbench-concept.jpg`.

---

## 4. Measured Milestone `v4.0` CV Flywheel Convergence (`Phase 215`)

All 32 dynamically discovered surfaces in `SURFACE_REGISTRY` and the 4 core workbench captures converged through `@ngi/platform-ssot` (`crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts`):

| Surface / Capture | `visualBeforeScore` | `visualAfterScore` | Delta | `composerToolbarRowCount` | `composerToolbarHeightPx` | `clippedInteractiveCount` | `duplicateControlGroupCount` |
|---|---:|---:|---:|---:|---:|---:|---:|
| `chat-default` | `0.8502` | **`0.9164`** | `+0.0662` | `1` | `28px` | `0` | `0` |
| `loquela-knobs` (Posture Popover Open) | `0.8280` | **`0.9178`** | `+0.0898` | `1` | `28px` | `0` | `0` |
| `chat-dock-multi` | `0.7985` | **`0.9161`** | `+0.1176` | `1` | `28px` | `0` | `0` |
| `urbs-civilization` | `0.8228` | **`0.9180`** | `+0.0952` | `0` | `0px` | `0` | `0` |
| `chat` (32-Surface Dynamic Audit) | `0.6878` | **`0.9164`** | `+0.2286` | `1` | `28px` | `0` | `0` |
| `dashboard` (32-Surface Dynamic Audit) | `0.9012` | **`0.9185`** | `+0.0173` | `0` | `0px` | `0` | `0` |
| `models` (32-Surface Dynamic Audit) | `0.8712` | **`0.9174`** | `+0.0462` | `0` | `0px` | `0` | `0` |
| `tasks` (32-Surface Dynamic Audit) | `0.8961` | **`0.9198`** | `+0.0237` | `0` | `0px` | `0` | `0` |
| **All 32 `SURFACE_REGISTRY` Views** | `0.6878–0.9254` | **`0.9142–0.9254`** | **100% Pass** | `1` (on Chat) | `28px` (`<= 52px`) | **`0` across all 32** | **`0` across all 32** |
