---
phase: 383
slug: chat-surfaces-and-docks-overhaul
title: "Phase 383 UI Design Contract — Chat Surfaces, Multi-Panel Dock Tiling, Loquela Composer & Execution Rail Reorganization"
status: approved
created: 2026-11-08
---

# Phase 383 — UI Design Contract (`383-UI-SPEC.md`)

## 1. Executive Summary & Problem Statement

Computer Vision (`@ngi/platform-ssot`) and multi-resolution DOM inspection (`1280×800`, `1440×900`, `1920×1080`) of the Chat surface and its 14 Dockview panels revealed 6 structural layout and control defects:

1. **Loquela `Execution Posture & Routing` Popover Overflow & Occlusion (`Loquela.tsx` + `DriveConsole.tsx`):**
   - `loquela-posture-popover` was anchored to the inner posture button with `w-[min(42rem,94vw)]`, causing it to extend past the right edge of the Chat panel (`>120px` horizontal overflow over the dock divider) while floating directly on top of the composer `<textarea>` and `Run ↵` button.
   - Inside the popover, `DriveConsole` enforced `whitespace-nowrap` on a horizontal strip containing Mode radios + Spend meter + Risk trigger, forcing horizontal clipping in split-dock layouts.
2. **Loquela Bottom Toolbar Crowding (`Loquela.tsx`):**
   - `.loquela-bar` had `whitespace-nowrap` without `flex-wrap`, squeezing 7–8 controls (`+`, `📎`, `🎤`, `Efficient · moderate`, `Run on Auto`, `Quick chat`, `@kb:auto`, token/cost counter) into a single rigid row that overflowed at `1280×800` or when docked alongside `Execution` + `Urbs`.
3. **Multi-Panel Dock Tiling Squeeze (`ChatSurface.tsx` + `dockview-vox.css`):**
   - `applyWorkspacePreset` (`Split Code`: `['executionRail', 'terminal']`; `Multi-Agent`: `['harness', 'urbs']`) placed 3 panels in a single horizontal row (`direction: 'right'`) at `1280×800` (`1096px` main area width). With `TRANSCRIPT_MIN_WIDTH_PX = 460`, the remaining 2 panels were crushed to `<300px` each, causing horizontal overflow and cut-off controls.
   - Instead, multi-panel presets and opt-in docks must tile into a **balanced 2D Romanesque Workbench Grid**:
     - **Left Column (`transcript`, ~52–58% width):** Primary Chat Transcript & Loquela Composer (`min-width: 400px`).
     - **Right Column (`executionRail` + auxiliary panel stacked `below`, ~42–48% width):** Top-right primary inspector or tactical surface (`executionRail`, `urbs`, or `harness`) and bottom-right stacked companion panel (`terminal`, `todos`, `repository`, or `executionRail`), so every docked panel receives `>= 360px` of horizontal breathing room even at `1280×800`!
4. **Execution Rail Cut-Off Text (`ChatExecutionRail.tsx`):**
   - Task titles used single-line `truncate`, cutting off every task after ~25 characters (`Consolidate Loquela toolbar & ...`).
   - The `Routing` summary button (`Routes to anthropic/claude-sonnet (offline)...`) used single-line `truncate` instead of wrapping cleanly across 2 lines.
   - `Pinned Context` items (`@kb:where-things-live.md · RRF 0.98`) truncated aggressively next to edit/remove buttons.
   - `System Checks` footer row used `whitespace-nowrap` with 3 badges (`SYSTEM CHECKS`, `ROUTE: LOCAL`, `ALL CHECKS PASSING`), overflowing narrow rail widths.
5. **Chat Transcript Top Toolbar & Message Action Bar (`ChatTranscript.tsx`):**
   - Top bar (`Quiet / Normal / Verbose` + `Consensus (2-4) / Compare Branches / Arena Compare`) and per-message action bar (`✎ Edit`, `↻ Regenerate`, `✂ Compact Here`, `⑂ Branch here`, `↺ Rewind`) lacked compact flex-wrapping and responsive padding for split-dock widths.
6. **Left Sidebar Chat Session List (`SessionSidebarSection.tsx`):**
   - Session titles under `CHAT` in the left navigation rail were cramped and lacked clean active-session hierarchy and compact metadata badges.

---

## 2. Visual & Architectural Contract

### 2.1 Loquela Composer & Docked Posture Drawer (`Loquela.tsx` + `DriveConsole.tsx`)
- **Inline Above-Composer Posture Drawer:**
  - Anchor `loquela-posture-popover` to the full width of the `Loquela` `<Glass>` container (`left-0 right-0 bottom-full mb-2 w-full max-w-full`) rather than the small trigger button, so it **never** extends past the right border of the Chat panel and **never** covers the `<textarea>` or `Run ↵` button!
  - Make `DriveConsole` responsive (`flex-wrap h-auto min-h-7 py-0.5 gap-y-1`) so Mode detents (`Free`, `Efficient`, `Balanced`, `Genius`), Spend gate, and `Risk` pill wrap gracefully inside narrow Chat docks (`400px–520px`).
- **Responsive Composer Control Bar (`.loquela-bar`):**
  - Replace rigid `whitespace-nowrap` overflow on `.loquela-bar` with `flex-wrap items-center justify-between gap-x-1.5 gap-y-1.5` so controls stay on 1 line at wide viewports and wrap cleanly into at most 2 compact rows in narrow multi-dock mode without horizontal clipping.

### 2.2 Balanced 2D Multi-Panel Dock Tiling (`ChatSurface.tsx` + `dockview-vox.css`)
- **Smart 2D Grid Tiling (`applyWorkspacePreset` & `positionForActivation`):**
  - Lower `TRANSCRIPT_MIN_WIDTH_PX` from `460` to `400` so 2-column split layouts at `1280×800` give the right dock column `480px+` of width.
  - When a second right-hand panel is opened (e.g., `Split Code` with `executionRail` + `terminal`, `Multi-Agent` with `executionRail` + `urbs` + `harness`, or opening any opt-in panel when `executionRail` is already open), stack the secondary right-hand panel **`below` `executionRail`** (or `below` the last right-hand panel) once 1 right-hand column exists (`OPT_IN_ROW_STACK_THRESHOLD = 1`), creating a **2-column × 2-row right split** instead of 3–4 skinny vertical columns!
  - Add a 4th layout preset button in the header bar: **`Overview`** (`executionRail` + `urbs` + `todos`) alongside `Focus`, `Split Code`, `Multi-Agent`, and `🏛 Tactical Map ⌘M`.
  - Make `chat-top-header-bar` and `layoutPresetsBar` `flex-wrap` with compact padding so nothing clips at `1280×800`.

### 2.3 Execution Rail Typography & Non-Cutoff Cards (`ChatExecutionRail.tsx`)
- **Task Cards (`Plan & Execution Steps`):**
  - Replace single-line `truncate` on task titles with `line-clamp-2 wrap-break-word` so full task descriptions up to 2 lines are readable without hover.
  - Keep task action buttons (`P1..P3`, `?`, `✓`, `✎`, `×`) compact and aligned to the top-right without colliding with the title.
- **Routing & Spoke Cards:**
  - Allow `execution-rail-routing` to wrap up to 2 lines (`line-clamp-2 wrap-break-word`) instead of cutting off mid-model name.
  - Allow `execution-rail-mens-spoke` to wrap cleanly with `flex-wrap`.
- **Pinned Context & System Checks:**
  - Allow `Pinned Context` labels to wrap or display with clean `break-all`/`line-clamp-1` and compact action buttons.
  - Make the `System Checks` header row `flex-wrap gap-1` so `SYSTEM CHECKS`, `ROUTE: LOCAL`, and `ALL CHECKS PASSING` never overflow the rail width.

### 2.4 Multi-Resolution & Dual-Theme Verification Matrix (`cv-flywheel.spec.ts`)
- Verify across **1280×800**, **1440×900**, and **1920×1080** in both:
  - **Imperial Carrara White & Gold (`arcane`)**
  - **Minimalist Grey-Blue Slate (`void`)**
- Enforce `outerOverflowPx === 0`, `scrollHostOverflowPx === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, and `loquela-posture-popover` bounding-box containment within `[data-testid="loquela-composer"]`.
