---
phase: 101-urbs-civilization-visibility-functionality-live-taps
audited: 2026-10-19T18:38:00-07:00
baseline: 98-UI-SPEC.md, 99-UI-SPEC.md, 100-UI-SPEC.md, 101-UI-SPEC.md, SURFACE_REGISTRY (32 surfaces)
screenshots_captured: true
status: verified
---

# Phase 98–101 — 6-Pillar UI Review & Dynamic 32-Surface CV Flywheel Audit (`@ngi/platform-ssot`)

**Audited:** 2026-10-19
**Baseline:** Milestone `v1.8` (`Phases 98–101`) + Dynamic `SURFACE_REGISTRY` (`32` live surfaces)
**Screenshots:** Live Playwright captures (`1440×900`) in `crates/vox-gui/ui/artifacts/cv-flywheel/surfaces/*-after.png` + `crates/vox-gui/ui/artifacts/cv-flywheel/*-after.png`

---

## Pillar Scores (Before Remediation → After Remediation)

| Pillar | Before Score | After Score | Key Finding & Remediation Summary |
|--------|--------------|-------------|-----------------------------------|
| 1. Copywriting | 3/4 | **4/4** | Empty-state voids on `mercatus`, `sub-agents`, and `vox-search` lacked actionable copy or mock parity; `Loquela` `RiskPopover` clipped multi-line description inside `w-72`. Added structured empty states, `whitespace-normal` copy wrapping, and full Tauri mock parity (`mercatus_load_config`, `list_subagent_tree`, `vox_search_status`). |
| 2. Visuals | 2/4 | **4/4** | **BLOCKER fixed:** `Dashboard.tsx:398` positioned `Customize dashboard` as `absolute right-5 top-2 z-20`, directly occluding the primary `Open Chat` CTA (`data-testid="open-chat-cta"`). Also fixed wrong `<Icon.alert />` icon on `ActivitySurface.tsx:364` `Refresh` button (`-> <Icon.refresh />`), replaced all raw `🧍` citizen emoji tokens with deterministic Roman SVG sprites (`UrbsCitizenSprite.tsx`), and registered missing `terminal` + `urbs` dockview panels (`DockWorkspaceShell.tsx`). |
| 3. Color | 2/4 | **4/4** | **BLOCKER fixed:** `CodeRabbitView.tsx` leaked hardcoded light-theme pastel pills (`#EEEDFE`, `#E1F5EE`, `#FCEBEB`, `#f1efe8`) into the dark Obsidian theme. **Serious WCAG AA contrast defects fixed:** `Console/BlockStreamView.tsx:88,161` (`text-zinc-500` -> `text-zinc-400`), `SubAgents/SubAgentTree.tsx:41` (`opacity: 0.6` -> `text-text-secondary`), and `Harness/WaveformTierStrip.tsx:24` (`role="img"` for `aria-label`). |
| 4. Typography | 3/4 | **4/4** | Standardized `11px`/`12px` tabular monospace badges and `10px` uppercase Roman section headers (`VRBS ROMA`, `AETAS`, `SENATVS POPVLVSQVE`) across `Chat`, `Harness`, `Gamify`, `Runs`, and `SubAgents`. Fixed `RiskPopover` inherited `whitespace-nowrap` truncation. |
| 5. Spacing | 2/4 | **4/4** | **WARNING fixed:** `PublicationsView.tsx:66` used `flex gap-3 overflow-x-auto` with `w-44 shrink-0`, causing `140px` horizontal overflow and clipping the 7th `FAILED` stage column at `1440px` (`-> xl:grid-cols-7 min-w-0`). Fixed `RunsView.tsx:188` grid row stretching (`-> content-start`) and unified `28px` (`h-7`) toolbar control heights across `Loquela` composer pills (`0px` height spread). |
| 6. Experience Design | 3/4 | **4/4** | Unstyled native browser `<select>` dropdowns in `Chat`, `ModelsView.tsx`, and `SettingsView.tsx` broke VS Code dark chrome consistency. Added custom `Loquela` popover selectors, explicit dark select pills in `ModelsView.tsx`/`SettingsView.tsx`, and a global `@layer base` `select:not([data-unified-knob="true"])` rule in `src/index.css`. Unfroze embedded `LudusSandbox` landmark liveness (`AQVAE` & `COLOSSEVM` `6/6` lit) and unified `@ngi/platform-ssot` CV Flywheel + `auditDomQualityInPage` into CI/CD (`pnpm review:cv-flywheel`). |

**Overall Score:** `15/24` (Before) → **`24/24` (After Remediation)**

---

## All-Surface Audit Findings & Remediations (with `file:line` Citations)

### Priority 1 — Blockers & Serious Accessibility Defects (All Resolved)

1. **`dashboard` — Floating `Customize dashboard` Button Occluded `Open Chat` CTA**
   - **Location:** `crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx:398`
   - **Critique:** `div.absolute.right-5.top-2.z-20` placed the `Customize dashboard` button directly on top of the `Open Chat` primary action button (`data-testid="open-chat-cta"`) in the `WorkspaceOverviewCard` header (`overlappingControlPairs = 1`).
   - **Fix Applied:** Converted the toolbar row to normal document flow (`mb-3 flex items-center justify-end gap-2 px-5`) above the grid, eliminating CTA overlap (`overlappingControlPairs: 1 -> 0`, visual score `0.9012 -> 0.9181`).

2. **`coderabbit` — Hardcoded Light-Theme Pastel Hex Leaks & Unstyled Form Inputs**
   - **Location:** `crates/vox-gui/ui/src/components/surfaces/CodeRabbit/CodeRabbitView.tsx:16-39,135-182`
   - **Critique:** `TIER_COLORS` and status badges used light-mode pastel backgrounds (`#EEEDFE`, `#E1F5EE`, `#FCEBEB`, `#f1efe8`) and bare unstyled `<input>` / `<button>` elements (`Plan sweep`, `Run`) inside the dark Obsidian shell (`lightThemeContrastLeakCount = 5`).
   - **Fix Applied:** Replaced pastel hexes with dark-theme translucent surface/brass/emerald/rose tokens and styled all inputs/buttons with `border-border-subtle bg-bg-surface/80` while preserving `var(--color-text-secondary)` contract labels (`lightThemeContrastLeakCount: 5 -> 0`, visual score `0.8766 -> 0.9047`).

3. **`chat` / `loquela-knobs` / `chat-dock-multi` — Native `<select>` Chrome, Missing Dock Panels (`terminal`, `urbs`), & `RiskPopover` Clipping**
   - **Location:** `crates/vox-gui/ui/src/components/surfaces/Loquela/Composer.tsx:95-190`, `RiskPopover.tsx:58-72`, `crates/vox-gui/ui/src/components/workspace/DockWorkspaceShell.tsx:45-160`
   - **Critique:** Four native OS `<select>` elements in the Chat composer had `8px` height spread (`22px..30px`), `RiskPopover` inherited `whitespace-nowrap` from its trigger pill (`clippedInteractiveCount = 3`), and `DockWorkspaceShell` lacked `terminal` and `urbs` panels (`missingDockPanelIds = ['terminal', 'urbs']`).
   - **Fix Applied:** Unified all composer knobs into `28px` (`h-7`) custom popovers (`data-loquela-toolbar-item="true"`), added `whitespace-normal` to `RiskPopover` option descriptions, and registered `terminal` + `urbs` dock panels (`visualAfterScore: 0.7043 -> 0.9145`, `+0.2102` delta).

4. **`gamify` / `urbs-civilization` — Raw `🧍` Emoji Tokens, Frozen `AQVAE` Landmark, & Header Word-Spacing**
   - **Location:** `crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx:193-432`, `UrbsCitizenSprite.tsx:1-95`
   - **Critique:** Citizen walkers rendered raw `🧍` Unicode emojis, embedded `LudusSandbox` froze `metrics=null` leaving `AQVAE` permanently unlit (`unlitLandmarkCount = 2`), and `VRBSROMA` lacked word spacing at `tracking-[0.2em]`.
   - **Fix Applied:** Replaced all raw emojis with `UrbsCitizenSprite` SVG toga/tunic/galea vectors (`rawEmojiCount: 2 -> 0`), wired live `useLudusEvents` telemetry + active landmark baseline (`6/6` landmarks lit, `unlitLandmarkCount: 2 -> 0`), and added `mr-[0.45em]` word gap on `VRBS ROMA` (`visualAfterScore: 0.8228 -> 0.9180`).

5. **`console`, `harness`, `sub-agents` — Live WCAG 2.2 AA Contrast & WAI-ARIA Violations Caught by Dynamic 32-Surface Axe Sweep**
   - **Locations:**
     - `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx:88,161` (`text-zinc-500` on `#09090b` failed 4.5:1 AA contrast -> upgraded to `text-zinc-400`)
     - `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx:24` (`aria-label="Token throughput waveform"` on bare `<div>` violated `aria-prohibited-attr` -> added `role="img"`)
     - `crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentTree.tsx:41` (`style={{ opacity: 0.6 }}` on model ID failed AA contrast -> replaced with `className="font-mono text-xs text-text-secondary"`)

---

### Priority 2 — Layout, Overflow, & Chrome Warnings Across Remaining Surfaces (All Resolved)

6. **`publications` — 7-Stage Pipeline Horizontal Overflow (`140px`)**
   - **Location:** `crates/vox-gui/ui/src/components/surfaces/Publications/PublicationsView.tsx:66`
   - **Critique:** `flex gap-3 overflow-x-auto pb-3` with `w-44 shrink-0` overflowed horizontally by `140px` at `1440×900`, clipping the 7th `FAILED` column.
   - **Fix Applied:** Converted to responsive 7-column grid (`grid grid-cols-2 gap-2.5 pb-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-7` with `min-w-0`), bringing `scrollHostOverflowPx: 140 -> 0` and `visualAfterScore: 0.7644 -> 0.9047` (`+0.1403`).

7. **`models`, `settings`, & Global `<select>` Controls — Unstyled Native OS Dropdowns**
   - **Locations:** `crates/vox-gui/ui/src/index.css:218-235`, `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx:96-119`, `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx:167-177`
   - **Critique:** `Routing mode` and `Task` selects in `ModelsView.tsx` had zero CSS classes, and `SettingsView.tsx` lacked custom select appearance (`unstyledNativeSelectCount = 2` on `models`, `1` on `settings`).
   - **Fix Applied:** Added global `@layer base` `select:not([data-unified-knob="true"])` styling (`appearance: none`, SVG chevron, dark surface border) in `src/index.css` plus explicit dark pill styling in `ModelsView.tsx` and `SettingsView.tsx` (`unstyledNativeSelectCount -> 0` across all 32 surfaces).

8. **`runs` — Stretched Grid Row Height on `Latest Route Decision` Card**
   - **Location:** `crates/vox-gui/ui/src/components/surfaces/Runs/RunsView.tsx:188`
   - **Critique:** `grid grid-cols-12 gap-5 p-4 h-full` lacked `content-start`, stretching the single row to `250px` of empty vertical space.
   - **Fix Applied:** Added `content-start` so cards size naturally to their content (`visualAfterScore: 0.8972 -> 0.9152`).

9. **`activity` — Warning Triangle Icon (`Icon.alert`) Used on `Refresh` Button**
   - **Location:** `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx:364`
   - **Critique:** The `Refresh` toolbar button rendered `<Icon.alert />` (hazard triangle) instead of `<Icon.refresh />`.
   - **Fix Applied:** Replaced with `<Icon.refresh className="w-3.5 h-3.5 mr-1.5" />`.

10. **`mercatus`, `sub-agents`, `vox-search` — Blank Voids & Missing Tauri Mock Commands**
    - **Locations:** `crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx:48-56`, `crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx:52-73`, `crates/vox-gui/ui/e2e/lib/tauriMock.ts:95-155`
    - **Critique:** Missing mock handlers (`mercatus_load_config`, `list_subagent_tree`, `vox_search_status`) left `mercatus`, `sub-agents`, and `vox-search` rendering near-empty voids or error banners during automated review captures.
    - **Fix Applied:** Added structured empty states, bordered card chrome in `SubAgentsView.tsx`, and realistic mock payloads in `tauriMock.ts`.

---

## Dynamic 32-Surface `@ngi/platform-ssot` CV Flywheel Scorecard (`1440×900` Chromium)

| # | Surface (`viewKey`) | Before Score | After Score | Delta | Edge Sharpness | Contrast | Noise Floor | Overflow (Outer/Host) | Unstyled `<select>` | Contrast Leaks | CTA Overlap | Broken Icons | Axe Serious/Crit |
|---|---------------------|--------------|-------------|-------|----------------|----------|-------------|-----------------------|---------------------|----------------|-------------|--------------|------------------|
| 1 | `activity` | `0.9052` | **`0.9048`** | `-0.0004` | `88.2` | `13.8` | `4.8` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 2 | `approvals` | `0.8960` | **`0.8967`** | `+0.0007` | `80.1` | `12.2` | `4.8` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 3 | `browser` | `0.9240` | **`0.9665`** | **`+0.0425`** | `99.0` | `100.0` | `4.7` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 4 | `catalog` | `0.9033` | **`0.9038`** | `+0.0005` | `87.8` | `12.8` | `5.0` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 5 | `chat` | `0.7043` | **`0.9145`** | **`+0.2102`** | `99.0` | `14.3` | `6.0` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 6 | `coderabbit` | `0.8766` | **`0.9047`** | **`+0.0281`** | `87.9` | `14.0` | `4.8` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 7 | `console` | `0.9144` | **`0.9168`** | `+0.0024` | `99.0` | `17.8` | `7.4` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 8 | `dashboard` | `0.9012` | **`0.9181`** | **`+0.0169`** | `99.0` | `20.0` | `7.8` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 9 | `explorer` | `0.9254` | **`0.9254`** | `+0.0000` | `99.0` | `31.6` | `7.4` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 10 | `flow` | `0.9015` | **`0.9159`** | **`+0.0144`** | `99.0` | `16.4` | `5.5` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 11 | `gamify` | `0.8794` | **`0.9180`** | **`+0.0386`** | `99.0` | `19.9` | `6.2` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 12 | `harness` | `0.9239` | **`0.9230`** | `-0.0009` | `99.0` | `34.0` | `18.4` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 13 | `harness-health` | `0.8863` | **`0.8868`** | `+0.0005` | `68.9` | `12.0` | `4.7` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 14 | `memory` | `0.9160` | **`0.9161`** | `+0.0001` | `99.0` | `16.8` | `5.4` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 15 | `mens` | `0.9145` | **`0.9145`** | `+0.0000` | `99.0` | `14.2` | `5.9` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 16 | `mercatus` | `0.8642` | **`0.8972`** | **`+0.0330`** | `80.8` | `12.1` | `4.8` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 17 | `mesh` | `0.9141` | **`0.9141`** | `+0.0000` | `99.0` | `13.6` | `5.4` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 18 | `models` | `0.8712` | **`0.9174`** | **`+0.0462`** | `99.0` | `18.9` | `5.7` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 19 | `needs-you` | `0.8995` | **`0.9000`** | `+0.0005` | `84.0` | `12.1` | `5.1` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 20 | `oratio` | `0.9028` | **`0.9035`** | `+0.0007` | `87.9` | `12.1` | `4.9` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 21 | `policies` | `0.9069` | **`0.9074`** | `+0.0005` | `92.3` | `12.2` | `5.1` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 22 | `populi` | `0.9023` | **`0.9029`** | `+0.0006` | `87.3` | `12.0` | `4.9` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 23 | `publications` | `0.7644` | **`0.9047`** | **`+0.1403`** | `88.9` | `12.7` | `4.9` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 24 | `repository` | `0.9149` | **`0.9149`** | `+0.0000` | `99.0` | `14.8` | `5.0` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 25 | `research` | `0.9140` | **`0.9146`** | `+0.0006` | `99.0` | `14.4` | `5.7` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 26 | `runs` | `0.8972` | **`0.9152`** | **`+0.0180`** | `99.0` | `15.3` | `6.3` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 27 | `scientia` | `0.9084` | **`0.9101`** | `+0.0017` | `93.8` | `14.4` | `4.9` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 28 | `settings` | `0.8800` | **`0.9175`** | **`+0.0375`** | `99.0` | `19.1` | `6.3` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 29 | `skills` | `0.9010` | **`0.9015`** | `+0.0005` | `85.4` | `12.5` | `4.9` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 30 | `sub-agents` | `0.8675` | **`0.8935`** | **`+0.0260`** | `76.6` | `12.0` | `4.7` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 31 | `tasks` | `0.8961` | **`0.9207`** | **`+0.0246`** | `99.0` | `24.1` | `7.2` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |
| 32 | `vox-search` | `0.8702` | **`0.9189`** | **`+0.0487`** | `99.0` | `21.2` | `5.7` | `0 / 0` | `0` | `0` | `0` | `0` | `0` |

---

## Files Audited & Modified

- `crates/vox-gui/ui/e2e/review/audits.ts`
- `crates/vox-gui/ui/e2e/review/audits.test.ts`
- `crates/vox-gui/ui/e2e/review/capture.spec.ts`
- `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts`
- `crates/vox-gui/ui/src/lib/cvFlywheel.ts`
- `crates/vox-gui/ui/src/index.css`
- `crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx`
- `crates/vox-gui/ui/src/components/surfaces/CodeRabbit/CodeRabbitView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Publications/PublicationsView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Runs/RunsView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Mercatus/Mercatus.tsx`
- `crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentsView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/SubAgents/SubAgentTree.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Harness/WaveformTierStrip.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Loquela/RiskPopover.tsx`
- `crates/vox-gui/ui/src/components/gamify/LudusSandbox.tsx`
- `crates/vox-gui/ui/e2e/lib/tauriMock.ts`
- `crates/vox-gui/ui/package.json`
