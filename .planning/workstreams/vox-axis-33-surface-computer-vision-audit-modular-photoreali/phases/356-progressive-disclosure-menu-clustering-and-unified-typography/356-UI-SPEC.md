---
phase: 356
slug: progressive-disclosure-menu-clustering-and-unified-typography
status: approved
shadcn_initialized: true
preset: plate-2-romanesque-progressive-disclosure
created: 2026-11-08
---

# Phase 356–359 — UI Design Contract (`v6.0`: Progressive Disclosure, Menu Clustering, Zero Widget Duplication & Unified Readable Typography)

> Visual and interaction contract for de-cluttering all 32 Vox Axis surfaces, eliminating duplicate widgets/bars, replacing button walls with categorized dropdown menus, unifying readable Latin-inspired & coding typography (zero `V`-for-`U`, zero slanted `A`s, zero jagged IBM bitmap monospace), and executing a 2-round Before/After CV Flywheel.

---

## 1. Layout & Navigation Architecture (Concept Art Variations)

To solve the twin problems of **"too many buttons / too busy"** and **"top bar alone wastes vertical space"**, we evaluate three layout variations on our approved Plate 2 Carved Travertine & Burnished Brass theme:

| Layout Option | Primary Navigation | Surface Toolbar | Bottom Chrome | Canvas Area |
|---|---|---|---|---|
| **Option A: Slim Medallion Rail + Single Architrave** | Compact collapsible left rail (`56px` icon-medallion mode or `188px` categorized accordion mode) | Single `38px` top architrave with segmented mode tabs + `[View & Lens ▾]` dropdown menu | Single-row status dock (`32px`) | **~90%** |
| **Option B: Top-Left Megamenu Crest + Multi-Surface Suite** | `[ 🏛 Vox Axis ▾ ]` Megamenu inside the top architrave when sidebar is hidden | Categorized dropdown menus (`[Mode ▾]`, `[Actions ▾]`) replacing button strips | Unified status bar | **~93%** |
| **Option C: Categorized Accordion Rail + Unified Bottom Stone Dock** | Left rail grouped into 3 collapsible category accordions (`Operate`, `Workspace`, `System`) | Zero secondary button bars; controls tucked into `[Lens & Filters ▾]` and `[Drawers ▾]` menus | Merged status + telemetry dock | **~91%** |

**Implemented Hybrid Best-of-All-Worlds:**
- **Categorized Progressive-Disclosure Sidebar (`Sidebar.tsx`):** Supports 1-click collapse to hidden/slim mode AND groups secondary items into collapsible category sections so only essential top-level destinations are immediately visible.
- **Single-Row Surface Header (`BreadcrumbBar.tsx` + Surface Headers):** Eliminates stacked double/triple button rows. Secondary actions (`Lens`, `Repo Preset`, `Layout Focus`, `Verbosity`, `Multi-Agent Compare`, `Quick Actions`) are clustered into clean **Romanesque Dropdown Menus (`[ ... ▾ ]`)**.

---

## 2. Typography Contract (Zero Jagged IBM Monospace, Zero Slanted `A`s, Zero `V`-for-`U`)

| Role | Font Stack | Rules & Anti-Jagged Guarantees |
|---|---|---|
| **Display / Section Titles (`--font-display`)** | `'Optima', 'Palatino', 'Palatino Linotype', 'Book Antiqua', 'Inter', Georgia, serif` | Replaces `Cinzel` so capital `A` is 100% upright and symmetrical (no slanted calligraphic crossbar). `font-synthesis: none` prevents faux-bold jagged edges. |
| **UI Body, Buttons, Pills, Badges & Tables (`--font-sans` + `.font-mono` UI override)** | `'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif` with `font-variant-numeric: tabular-nums` | All UI buttons, badges, status bar tiles, and metric pills use smooth anti-aliased `Inter` with `tabular-nums` instead of jagged bitmap `ui-monospace` / `Courier` / `Consolas`. |
| **Code Blocks & Editor (`pre, code, .cm-editor`)** | `'JetBrains Mono', 'SF Mono', 'Menlo', ui-monospace, monospace` | Reserved strictly for source code, diffs, terminal output, and paths (`>= 12px`, `line-height: 1.55`, `-webkit-font-smoothing: antialiased`). |
| **Orthography Policy** | Plain English only | Zero `V`-for-`U` substitutions (`SIM PAUSED`, `FORUM`, `PORTUS`, `URBS`). |

---

## 3. De-Cluttering & Anti-Duplication Contract Per Surface

1. **Gamify / Workspace Map (`GamifyView.tsx` + `LudusSandbox.tsx`):**
   - **Removed Duplicate Sidebar Buttons:** Deleted the 5 redundant `DASHBOARD / CHAT / MENS / TASKS / REPOSITORY` buttons from `GamifyView.tsx` header.
   - **Consolidated Mode Tabs:** Merged `Workspace Map | Arena & Quests | Engineering Leaderboard` and `Urbs Map | Live River` into a single 4-tab segmented control: `Workspace Map`, `Pipeline Cascade`, `Arena & Quests`, `Leaderboard`.
   - **Eliminated Duplicate HUD Card:** `<LudusHud>` (`ASCENDANT CENTURIO`) is hidden when viewing `Workspace Map` or `Pipeline Cascade` (it only renders on `Arena & Quests` and `Leaderboard`).
   - **Consolidated 13 Map Controls into 2 Progressive-Disclosure Menus:**
     - Replaced the 6 wide top landmark pills + 2 `<select>` bars + 5 lens buttons (`13` controls across 2 rows) with **1 compact top architrave bar** containing:
       - 4 District Medallion Quick-Pills (`Compiler`, `Orchestrator`, `MENS Models`, `Mesh & DB`)
       - **`[ Lens: <Active> ▾ ]` Menu** (houses the 5 lenses + Repo & Layout selectors inside a clean popover menu)
       - **`[ Drawers & History ▾ ]` Menu** (toggles `Priority Alerts` and `Git Timeline`, which default to **closed** so the city map is unobstructed!).
   - **Simplified Bottom Stone Telemetry Console (`HudPanels.tsx`):**
     - Single compact row with the 3 `<RomanBrassGauge>` dials (`Budget`, `Energy`, `Speed` — clicking the `Speed` dial cycles `1x → 3x → 0x` without 3 extra clock buttons) + `[ ⌂ Fit ]`.

2. **Dashboard (`Dashboard.tsx`):**
   - **Eliminated Duplicate `RESOURCES` Row:** Removed the redundant `COMPUTE MESH / VECTOR STORE / TOKEN BUDGET` row that duplicated the top 4 KPI cards (`Active Agents`, `Queue Depth`, `Budget Spent`, `Mesh Peers`).
   - **Clustered Bottom Panels into Tabbed Progressive Disclosure:** Replaced stacked `Attention Budget` + `Resources` + `Active Agents` + `System Alerts` cards with a single **Tabbed Bottom Inspector (`[ Active Agents | System Alerts | Attention Budget ]`)** so only one clean panel is shown below the map.
   - **Clean Embedded Mini-Map:** Hides internal floating monument text plaques in embedded mode so the mini-map is a crisp, unobstructed architectural diorama with 1-click `[ Full-Screen Map ↗ ]`.

3. **Chat & Orchestrator (`ChatSurface.tsx` + `ChatExecutionRail.tsx`):**
   - **Eliminated Duplicate Routing Section:** Merged `ORCHESTRATOR HUB & SPOKES` and the duplicate `ROUTING / SPOKE` block in `ChatExecutionRail.tsx` into a single unified card, and defaulted the inspector tab to `Tasks` instead of `All` so the right rail shows 1 focused view at a time.
