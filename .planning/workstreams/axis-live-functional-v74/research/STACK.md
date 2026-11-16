# Stack Research

**Domain:** Tauri 2 + React 19.2 + Tailwind CSS v4.3 Desktop Workbench UI Upgrade (Vox Axis Milestone v7.4: Live Control Wiring, Zero-Emoji Icon/Graphic System, shadcn/Radix Ecosystem Upgrade & Concept-Art Bento CV Flywheel)
**Researched:** 2026-11-14
**Confidence:** HIGH (All versions live-verified via `pnpm view` against npm registry; grounded in direct codebase inspection of `crates/vox-gui/ui/`)

---

## Executive Baseline Audit (`crates/vox-gui/ui`)

Live inspection of `crates/vox-gui/ui/package.json`, `src/index.css`, `knip.jsonc`, `e2e/review/audits.ts`, and `src/components/ui/` surfaced six critical architectural facts that govern every stack decision in v7.4:

1. **Broken Radix Animations (`Dialog.tsx:41-42,68-72`, `Tooltip.tsx:42`):** Both components emit `animate-in`, `animate-out`, `fade-in-0`, `zoom-in-95`, and `slide-in-from-*` utility classes, **but neither `tw-animate-css` nor `tailwindcss-animate` is installed** in `package.json` or imported in `src/index.css:1-8`. Every Radix entry/exit transition is currently a dead CSS no-op.
2. **Stale Fragmented Radix Sub-Packages (`package.json:26-28`):** Only three standalone primitives are installed (`@radix-ui/react-dialog@^1.1.23`, `@radix-ui/react-slot@^1.3.3`, `@radix-ui/react-tooltip@^1.2.16`). Meanwhile, `Popover.tsx:10-18` is a 19-line hand-rolled `absolute bottom-9` `<div>` with **no Portal, no viewport collision flip, no Escape handling, and no outside-click dismissal**, and 28 files across `src/` fall back to native `<select>` elements (`CommandCatalogForm.tsx`, `Loquela.tsx`, `SettingsView.tsx`, `ModelsView.tsx`, `TasksView.tsx`, `ResearchView.tsx`, `MeshView.tsx`, etc.).
3. **Nuclear `button` `!important` Overrides in `src/index.css:311` & `1076`:** Global rules target `:is(main, [role="dialog"]) button:not([role="tab"]):not([data-roman-gauge="true"]):not([disabled])` with `background-color: #FAF5E8 !important` and riveted-brass `background-image: ... !important` in `arcane` (and `var(--color-bg-elevated) !important` in `slate`). Without adding `:not([data-slot])` to those two selectors, any Radix `<Switch>`, `<Checkbox>`, `<RadioGroup.Item>`, `<Select.Trigger>`, or `<Button variant="ghost" | "primary">` rendered inside `<main>` or a dialog will have its checked/variant state forcibly overwritten into a cream marble button.
4. **`.no-tauri` Headless Screenshot Escape Hatch (`src/index.css:1168-1174`):** Playwright CV Flywheel captures tag `<html>` with `.no-tauri`, forcing `backdrop-filter: none !important`. Translucent overlays like `Dialog.tsx:65` (`bg-bg-base/80 backdrop-blur-xl`) bleed background text during CV audits unless `--popover` / `bg-popover` resolves to an opaque surface token (`var(--color-overlay-solid)` / `var(--color-bg-elevated)`).
5. **35+ Residual Raw Emoji & Unicode Dingbat Leaks Across `src/`:** Live source scan (`rg "[\x{1F300}-\x{1FAFF}\x{2600}-\x{27BF}]"`) found raw emojis in `ImperiumInspector.tsx:199,275,309,337,346,378,603,823` (`🔒`, `⚡`, `⚙`, `⚔`, `🌿`, `⛵`, `⚱` — hidden from CI because `e2e/review/audits.ts:277` explicitly exempted `[data-testid="urbs-imperium-inspector"]`!), `MultiWaveProgressTimeline.tsx:136` (`⚠️`), and 30+ raw Unicode dingbats (`✓`, `✕`, `✗`, `✎`, `✂`, `⚒`, `☼`, `★`, `▶`, `▼`, `⋮⋮`, `☐`) across `DataTable.tsx:149`, `DashboardGrid.tsx:165`, `ChatTranscript.tsx:223,245`, `PlanPanel.tsx:39,137`, `StatusPill.tsx:14,24`, `LudusSandbox.tsx:2747,2764`, and modal close buttons.
6. **Strict `knip` & `pnpm` 11+ Build Script Guards (`knip.jsonc:12-14`, `pnpm-workspace.yaml:19-20`):** `knip` hard-fails CI on any unimported dependency (`"dependencies": "error"`) or unreferenced source file (`"files": "error"`), and `pnpm-workspace.yaml` blocks postinstall scripts unless allowlisted in `allowBuilds`. Every new package must be pure ESM/JS without postinstall binaries and wired into `src/components/ui/` + real surface consumers immediately.

---

## Recommended Stack

### Core Technologies (Retained + Upgraded)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `radix-ui` | `^1.8.1` *(NEW — replaces 3 standalone `@radix-ui/react-*` deps)* | Unified official Radix UI accessible primitives monorepo (`Dialog`, `Popover`, `Select`, `DropdownMenu`, `Tabs`, `Switch`, `Slider`, `Checkbox`, `RadioGroup`, `ToggleGroup`, `Tooltip`, `ScrollArea`, `Collapsible`, `Accordion`, `AlertDialog`, `HoverCard`, `ContextMenu`, `Progress`, `Separator`, `Label`, `Slot`) | Official shadcn/ui Tailwind v4 (`new-york-v4`) standard. Verified `peerDependencies: { react: "^19.0", "react-dom": "^19.0" }`, `"sideEffects": false` (100% Rollup/Rolldown tree-shakeable), zero postinstall scripts. Eliminates `@radix-ui/react-compose-refs` version-skew infinite render loops on React 19.2 and satisfies `knip` (`dependencies: "error"`) with a single `package.json` entry across all phases. |
| `tailwindcss` + `@tailwindcss/postcss` | `^4.3.5` *(upgrade from `^4.3.3`)* | CSS-first utility engine (`@theme`, `@theme inline`, `@theme reference`, `@utility`) | Already driving `src/index.css` via `postcss.config.js:3`. Keep `@tailwindcss/postcss` so `knip.jsonc:44` and `dashboardBundleBudgetInProcess.ts:48` remain zero-churn while picking up v4.3.5 `@container` and `@theme inline` fixes. |
| `tw-animate-css` | `^1.4.0` *(NEW devDep)* | Pure CSS Tailwind v4 animation utilities (`animate-in`, `animate-out`, `fade-in-*`, `zoom-in-*`, `slide-in-from-*`) | Official shadcn/ui Tailwind v4 replacement for deprecated Tailwind v3 JS plugin `tailwindcss-animate`. Zero JS runtime, zero peer dependencies; immediately activates existing broken animations in `Dialog.tsx` and `Tooltip.tsx` and powers all new `Popover`/`Select`/`DropdownMenu`/`Sheet` transitions. |
| `class-variance-authority` | `^0.7.1` *(NEW)* | Strongly-typed `cva()` + `VariantProps<T>` component variant matrices | Standard shadcn/ui variant engine (~1.2 KB gzip, zero peer deps). Replaces brittle ad-hoc lookup maps (`Button.tsx:6` `VARIANT_CLASS`, `Toasts.tsx:21` `TONE_ICON_CLASS`, `Segment.tsx:18`) with composable, type-checked variants (`variant`, `size`, `density`, `bentoSpan`). |
| `lucide-react` | `^1.72.0` *(upgrade from `^1.39.0`)* | Primary standard UI stroke icon library (1,600+ tree-shaken 24×24 SVG icons) | Already installed and imported in 5 files (`MensTrainingView.tsx:19`, `InspectorDrawer.tsx:12`, `SubAgentTree.tsx:2`, etc.); verified `peerDependencies: { react: "^19.0.0" }`. Shares exact `24×24` `stroke="currentColor"` geometry with Vox's bespoke Romanesque `Icons.tsx`, enabling a 100% offline Zero-Emoji icon system when normalized at `strokeWidth={1.5}`. |

### Supporting Libraries (Targeted Workbench Additions)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `cmdk` | `^1.1.1` *(NEW)* | Accessible WAI-ARIA command menu & searchable combobox primitive (`Command`, `CommandDialog`, `CommandInput`, `CommandList`, `CommandGroup`, `CommandItem`) | Verified `peerDependencies: { react: "^18 || ^19" }`. Use for: (1) refactoring `Omnibar.tsx` keyboard/group navigation onto WAI-ARIA `combobox`+`listbox` semantics while preserving Vox's federated/graph search hooks (`useFederatedSearchIndex`, `useCapabilityIndex`), and (2) searchable `<Combobox>` model/crate/branch/spoke selectors replacing long `<select>` lists (`ModelsView`, `Loquela`, `SettingsView`). |
| `react-resizable-panels` | `^4.22.1` *(NEW)* | WAI-ARIA keyboard-accessible split-pane layout (`Group`, `Panel`, `Separator`) with pixel/percentage constraints and `localStorage` persistence | Verified `peerDependencies: { react: "^18.0.0 || ^19.0.0" }`. Use for **2-pane and 3-pane Master-Detail inspector layouts** inside single surfaces (`RepositoryView`, `ModelsView`, `MensTrainingView`, `ExplorerView`, `PoliciesView`, `ApprovalsView`) where table/inspector collisions occurred in v7.2. **Do NOT replace `dockview@6.6.1`** in `DockWorkspaceShell.tsx` (keep `dockview` for multi-tab drag-and-drop workspace docking in Chat). |
| `@tanstack/react-table` | `^9.3.1` *(NEW)* | Headless, tree-shakeable (`tableFeatures()`) table state engine (`useTable`, sorting, column visibility, faceted filtering, multi-row selection) | Verified `peerDependencies: { react: ">=18" }`, React 19 Compiler-safe. Upgrades `src/components/ui/DataTable.tsx` (currently missing real column sorting despite `sortable?: boolean` at `DataTable.tsx:9`!) while preserving Vox's Roman Carrara Marble / Slate `<table>` styling (`src/index.css:637,1100`) and composing with already-installed `@tanstack/react-virtual@^3.14.10` for >200-row tables (`RunsView`, `ActivitySurface`, `ExplorerView`). |
| `recharts` *(EXISTING)* | `^3.8.1` *(already installed)* | Declarative SVG telemetry charts wrapped in new shadcn `ChartContainer` / `ChartTooltip` (`src/components/ui/Chart.tsx`) | Already installed (`package.json:41`) and budgeted in `dashboardBundleBudgetInProcess.ts:91`. Wrap with shadcn v3 `ChartContainer` (`height="100%"` instead of hardcoded `height={160}` at `LineChartWidget.tsx:29`) so Bento cards achieve `chartPanelFillRatio >= 0.84` across all row spans. |
| `@dnd-kit/core` + `@dnd-kit/sortable` + `@dnd-kit/utilities` *(EXISTING)* | `^6.3.1` / `^10.0.0` / `^3.2.2` *(already installed)* | Customizable Bento widget drag-and-drop reordering | Already powers `DashboardGrid.tsx:2-15`. Keep as-is; replace raw `⋮⋮` (`DashboardGrid.tsx:165`) and empty resize box (`170-177`) with `<GripVertical />` and `<MoveDiagonal2 />` icons from `lucide-react`. |
| `clsx` + `tailwind-merge` *(EXISTING)* | `^2.1.1` + `^3.6.0` *(already installed)* | Class merging via `cn()` (`src/lib/cn.ts:4`) | `tailwind-merge@^3.6.0` natively supports Tailwind CSS v4 utilities (`bg-linear-to-*`, `backdrop-blur-xs`, `rounded-xs`, CSS-var arbitrary syntax `bg-(--color-*)`). Keep `src/lib/cn.ts` unchanged. |

### Development & Verification Tools (Existing — Enforce in v7.4)

| Tool | Version | Purpose | Notes |
|------|---------|---------|-------|
| `style-dictionary` | `^5.5.2` *(existing)* | Compiles `tokens/*.json` → `src/styles/tokens*.generated.css` + `tokens.generated.ts` | Always edit `tokens/*.json` and run `pnpm --dir crates/vox-gui/ui tokens:build`; never hand-edit `*.generated.css`. |
| `knip` | `5.50.0` *(existing)* | Unused file, export, and dependency gate (`knip.jsonc`) | `"files": "error"` and `"dependencies": "error"`. Any new `src/components/ui/*.tsx` primitive built before its first surface consumer lands in the same commit MUST either be consumed immediately or added temporarily to `knip.jsonc` `entry`. |
| `@playwright/test` + `@axe-core/playwright` | `^1.62.1` + `^4.13.0` *(existing)* | Multi-resolution (`900×600`..`1920×1080`) + dual-theme (`arcane` & `slate`) Concept-Art Bento CV Flywheel & WCAG 2.2 AA gate | Remove the `[data-testid="urbs-imperium-inspector"]` exemption in `e2e/review/audits.ts:277` and expand `emojiScanTargets` (`audits.ts:264-281`) to scan all visible text nodes + raw dingbats (`✓`, `✕`, `⚠`, `✎`, `✂`, `☼`) across all 33 surfaces and 14 docks. |

---

## Installation

Execute inside `/Users/brbrainerd/dev/vox` (using `pnpm --dir crates/vox-gui/ui` — never `npm` or `yarn`):

```bash
# 1. Add React 19 + Tailwind v4 verified runtime packages & upgrade lucide-react
pnpm --dir crates/vox-gui/ui add radix-ui@^1.8.1 class-variance-authority@^0.7.1 cmdk@^1.1.1 react-resizable-panels@^4.22.1 @tanstack/react-table@^9.3.1 lucide-react@^1.72.0

# 2. Add Tailwind v4 CSS animation devDependency & align Tailwind v4.3 patch
pnpm --dir crates/vox-gui/ui add -D tw-animate-css@^1.4.0 tailwindcss@^4.3.5 @tailwindcss/postcss@^4.3.5

# 3. Remove the 3 superseded standalone @radix-ui/react-* entries once Button.tsx, Dialog.tsx, and Tooltip.tsx import from "radix-ui"
pnpm --dir crates/vox-gui/ui remove @radix-ui/react-dialog @radix-ui/react-slot @radix-ui/react-tooltip
```

> **Note on `pnpm-workspace.yaml`:** Live registry inspection (`pnpm view <pkg> scripts`) confirmed that `radix-ui@1.8.1`, `class-variance-authority@0.7.1`, `cmdk@1.1.1`, `react-resizable-panels@4.22.1`, `@tanstack/react-table@9.3.1`, `lucide-react@1.72.0`, and `tw-animate-css@1.4.0` have **zero `install`/`postinstall` lifecycle scripts**, so `crates/vox-gui/ui/pnpm-workspace.yaml` (`allowBuilds: { esbuild: true }`) requires no modifications.

---

## Exact Component Inventory for `crates/vox-gui/ui/src/components/ui/`

Follow React 19 + shadcn Tailwind v4 (`new-york-v4`) conventions for every file in `src/components/ui/`:
- **React 19 native `ref` prop** (no `React.forwardRef` boilerplate needed on new components; keep existing `Button`/`Dialog` call signatures compatible).
- **Mandatory `data-slot="<name>"` attribute** on every primitive root and sub-part (`data-slot="button"`, `data-slot="select-trigger"`, `data-slot="switch"`, `data-slot="bento-card"`, etc.) — both for Tailwind v4 parent/slot styling and to opt out of `src/index.css:311,1076`'s legacy `button` `!important` fallback.
- **PascalCase filenames** matching existing `crates/vox-gui/ui/src/components/ui/` convention (`Button.tsx`, `Dialog.tsx`, `Popover.tsx`, `Select.tsx`, etc. — do NOT mix lowercase `button.tsx` alongside `Button.tsx` on macOS APFS case-insensitive filesystems, which breaks Linux CI!).

### A. Existing Primitives to Upgrade In-Place

| File | Current State | v7.4 Upgrade Specification |
|------|---------------|----------------------------|
| `Button.tsx` | Uses `@radix-ui/react-slot` + plain `VARIANT_CLASS` object; `primary` uses `bg-brass text-bg-base` (2.9:1 contrast without CSS `!important` rescue). | Migrate to `Slot.Root` from `radix-ui` + `cva()` (`buttonVariants`); add `data-slot="button"`; fix `primary` to high-contrast theme tokens (`bg-primary text-primary-foreground`), add `roman` / `subtle` / `outline` / `ghost` / `danger` / `link` variants, replace inline spinner SVG with `<Loader2 className="size-3.5 animate-spin" />` (`lucide-react`). |
| `Dialog.tsx` | Uses `@radix-ui/react-dialog@^1.1.23`; `bg-bg-base/80` bleeds under `.no-tauri`; animations broken without `tw-animate-css`. | Import `Dialog as DialogPrimitive` from `radix-ui`; add `DialogHeader`, `DialogFooter`; switch `DialogContent` surface to opaque `bg-popover text-popover-foreground border-border-strong` (`vox-marble-tablet` in `arcane`); replace all raw `✕` close buttons across modals (`SandboxReplModal`, `DocPublishModal`, `MensTrainingView`) with built-in `<X className="size-4" />`. |
| `Popover.tsx` | Hand-rolled 19-line `div` (`absolute bottom-9 z-[99999]`), no Portal, clips inside `overflow-hidden` cards/headers, no Escape/outside-click close. | Replace internals with `Popover as PopoverPrimitive` from `radix-ui` (`Popover`, `PopoverTrigger`, `PopoverContent`, `PopoverAnchor`) rendered in a `<PopoverPrimitive.Portal>` with `collisionPadding={8}`, while keeping a backwards-compatible wrapper or migrating the few call sites (`BottomStatusBar`, `Loquela`, `IntelliSensePopover`). |
| `Tooltip.tsx` | Wraps every single `<Tooltip>` instance in its own `<RadixTooltip.Provider>` (`Tooltip.tsx:27`), defeating shared skip-delay behavior across toolbars. | Hoist a single `<TooltipProvider delayDuration={150}>` into `AppShell.tsx`, import `Tooltip as TooltipPrimitive` from `radix-ui`, and keep `<RomanTooltip>` (`RomanTooltip.tsx`) composing over the shared provider. |
| `DataTable.tsx` | Manual `<table>`, `sortable?: boolean` prop (`line 9`) is unimplemented dead config, group toggle uses raw `'▶'`/`'▼'` (`line 149`), native `<input type="checkbox">` (`line 115`). | Back with `@tanstack/react-table@^9.3.1` (`tableFeatures` + `useTable` for real column sorting, filtering, column visibility) + `<Checkbox>` primitive + `<ChevronRight>` / `<ChevronDown>` from `lucide-react` (zero Unicode triangles). Support optional `virtualized` mode via `@tanstack/react-virtual`. |
| `Segment.tsx` | Hardcoded dark `border-white/10 bg-black/30 text-zinc-50` (`line 20,30`) that fights `arcane` light marble theme. | Rebuild over `ToggleGroup` / `Tabs` from `radix-ui` using semantic `bg-muted text-muted-foreground data-[state=on]:bg-primary data-[state=on]:text-primary-foreground` + `data-slot="segment"`. |
| `Icons.tsx` + `VoxIcon.tsx` *(NEW helper)* | 36 bespoke stroke-`1.5` SVGs (`24` standard + `12` Romanesque domain icons). | Keep all 12 bespoke Romanesque semiotics (`romanColumn`, `laurelWreath`, `gladiusShield`, `denariusCoin`, `hourglassVia`, `brazierTorch`, `astrolabeRadar`, `aqueductArch`, `waxSeal`, `scrollTabula`, `romanGalley`, `anvilForge`) + add missing Roman/status icons, and expose `<VoxIcon>` normalizing `lucide-react` icons to `strokeWidth={1.5}` + `aria-hidden="true"`. |
| `StatusPill.tsx` & `Toasts.tsx` | `StatusPill.tsx:14,24` uses `'✓'`; `Toasts.tsx:53,58` uses `'×'` and `'▸'`. | Replace raw Unicode glyphs with crisp vector SVG icons (`Check`, `X`, `ChevronRight`, `AlertTriangle`, `Info`). Keep `Toasts.tsx` (do NOT replace with `sonner` — `Toasts.tsx` implements Vox's `groupKey`/`count` coalescing and `ToastCause` contract in `types/tauri.ts` tested in `Toasts.test.tsx`). |

### B. New shadcn / Radix Primitives to Create in `crates/vox-gui/ui/src/components/ui/`

| File | Backing Package | Target Surfaces & Live Controls Wired |
|------|-----------------|--------------------------------------|
| `Select.tsx` | `radix-ui` (`Select`) | **Replaces all 28 native `<select>` occurrences** across `CommandCatalogForm.tsx`, `Loquela.tsx`, `IntentPanel.tsx`, `SettingsView.tsx`, `PriorityChainEditor.tsx`, `TaskPolicySection.tsx`, `ModelsView.tsx`, `TasksView.tsx`, `IsolationPanel.tsx`, `ResearchView.tsx`, `MeshView.tsx`, `ExplorerRail.tsx`, `BrowserView.tsx`, `MemoryView.tsx`. Eliminates OS-native popup styling bugs (`audits.ts:138` `unstyledNativeSelectCount`). |
| `DropdownMenu.tsx` | `radix-ui` (`DropdownMenu`) | Dense row action menus (`RunsView`, `TasksView`, `SubAgentsView`, `ApprovalsView`, `PublicationsView`), `DockWorkspaceShell` layout preset menu, and header overflow actions (`[More ▾]`) for `900×600` progressive disclosure. |
| `Command.tsx` | `cmdk@^1.1.1` + `Dialog.tsx` | Powers `Omnibar.tsx` (`⌘K` palette) keyboard navigation/grouping and reusable `<Combobox>` (Popover + Command) for searchable model/crate/branch/skill pickers. |
| `Tabs.tsx` | `radix-ui` (`Tabs`) | Standardizes `SubTabs.tsx`, `SettingsView`, `MensTrainingView`, `ScientiaDashboard`, `ChatExecutionRail`, and `ImperiumInspector` with full WAI-ARIA `tablist`/`tab`/`tabpanel` arrow-key roving focus. |
| `Sheet.tsx` | `radix-ui` (`Dialog`) | Side-anchored slide-over drawers (`side="right" | "left" | "bottom"`). Replaces manual focus-trap `useEffect` plumbing in `DocViewerDrawer.tsx:31-69`, `ExplainDrawer.tsx`, `ResearchEngineDrawer.tsx`, `WidgetPickerDrawer.tsx`, and `InspectorDrawer.tsx`. |
| `Switch.tsx` | `radix-ui` (`Switch`) | Live boolean toggles in `SettingsView.tsx` (telemetry consent, offline mode, auto-heal), `PoliciesView.tsx`, `Loquela.tsx` governance knobs, and `LudusSandbox` layer toggles. |
| `Slider.tsx` | `radix-ui` (`Slider`) | Replaces `.vox-range` native `<input type="range">` (`index.css:114`) for temperature, token budget (`ContextWindowMeter.tsx`), oversight level (`1..5`), and `ViaTemporisScrubber.tsx`. |
| `Checkbox.tsx` & `RadioGroup.tsx` | `radix-ui` (`Checkbox`, `RadioGroup`) | Replaces native checkboxes/radios (`DataTable.tsx`, `ArchivePanel.tsx:188` `'☐'`, `CommandCatalogForm.tsx`, `SettingsView.tsx` theme/posture selectors). |
| `Resizable.tsx` | `react-resizable-panels@^4.22.1` | Wraps v4 `<Group>`, `<Panel>`, `<Separator>` (`ResizablePanelGroup`, `ResizablePanel`, `ResizableHandle` with brass/slate drag grip) for collision-free Master-Detail splits on `RepositoryView`, `ModelsView`, `MensTrainingView`, `ExplorerView`, and `CodeRabbitView`. |
| `ScrollArea.tsx` | `radix-ui` (`ScrollArea`) | Consistent cross-OS thin brass/slate scrollbars inside bounded Bento cards, `Omnibar` results, `ChatExecutionRail`, and inspector drawers. |
| `Collapsible.tsx` & `Accordion.tsx` | `radix-ui` (`Collapsible`, `Accordion`) | Progressive disclosure sections replacing native `<details>/<summary>` across `ExplorerView`, `PoliciesView`, `HarnessIssuesPanel`, and `AttentionBudgetMeter`. |
| `Badge.tsx` | `class-variance-authority` + `Slot` | Unified CVA status/tier/spoke chip (`pass`, `fail`, `warn`, `info`, `brass`, `outline`) bound to `--color-status-*` tokens (`src/styles/tokens.ts`). |
| `AlertDialog.tsx` | `radix-ui` (`AlertDialog`) | Destructive/safety-floor confirmations (`ConfirmRunDialog.tsx`, quarantine toggles, branch rewind, task cancel, secret clear). |
| `BentoGrid.tsx` & `Card.tsx` | Pure React 19 + `cva()` + Tailwind v4 `@container` | **Concept-Art Bento Layout System:** `<BentoGrid>` (responsive 12-col / auto-fit CSS Grid with `@container` queries) + `<BentoCard span="sm|md|lg|wide|tall|hero">` composing Carrara marble relief / `slate` basalt surfaces, optional `<RomanFlutedPillar>` / brass corner ticks (`vox-tick`), header actions slot, and `min-h-0 flex-1` body so embedded `<ChartContainer>` and `<DiegeticFxStage>` fill `>= 84%` of panel area (`chartPanelFillRatio >= 0.84`). |
| `Chart.tsx` | `recharts@^3.8.1` | shadcn Recharts v3 `ChartContainer`, `ChartTooltip`, `ChartTooltipContent`, `ChartLegend` — injects `--color-<key>` CSS vars, sets `ResponsiveContainer width="100%" height="100%"`, and targets Recharts v3 `.recharts-cartesian-axis-tick-label_text`. |

### C. Existing Domain Primitives to Retain As-Is (`src/components/ui/`)

Retain `RomanSurfaceHeader.tsx`, `RomanTooltip.tsx`, `Glass.tsx`, `Backdrop.tsx`, `AxisMark.tsx`, `Kpi.tsx`, `Sparkline.tsx`, `Async.tsx`, `EmptyState.tsx`, `ErrorBoundary.tsx`, `BackendBanner.tsx`, `Pill.tsx`, `Panel.tsx`, and `StateChip.tsx` (composing them inside `<BentoCard>` and upgrading any internal Unicode glyphs to `<VoxIcon>`).

---

## Tailwind v4 + Style Dictionary + shadcn Token Binding Conventions

### 1. Single Source of Truth Architecture

Vox Axis enforces a 4-profile theme contract (`arcane` default Sunlit White/Gold Carrara Marble, `travertine`/`glacier` Warm Stone Light, `slate`/`void` Minimalist Grey-Blue Dark, and `high-contrast` WCAG AAA Dark) compiled by `style-dictionary.config.mjs` into `src/styles/tokens*.generated.css` and tested by `src/index.css.test.ts`, `src/__tests__/statusColorTokens.test.ts`, and `src/styles/__tests__/contrast.test.ts`.

**Rule:** Never introduce parallel raw HSL channels (`--background: 0 0% 100%`) that bypass Style Dictionary. Instead, bind shadcn's semantic variables directly to Vox's Style Dictionary CSS custom properties using Tailwind v4's `@theme inline` directive in `src/index.css` so every shadcn component (`bg-popover`, `text-muted-foreground`, `border-input`, `bg-primary`, `text-primary-foreground`, `ring-ring`) automatically switches across **all 4 themes**, including inside `document.body` Radix Portals and nested `[data-theme-preview]` containers!

### 2. Exact `src/index.css` Token Wiring Blueprint

```css
@import './styles/fonts.css' layer(base);
@import './styles/tokens.generated.css' layer(base);
@import './styles/tokens.slate.generated.css' layer(base);
@import './styles/tokens.contrast.generated.css' layer(base);
@import './styles/tokens.travertine.generated.css' layer(base);
@import './styles/dockview-vox.css' layer(base);
@import 'tailwindcss';
@import 'tw-animate-css';

/* Bind shadcn/ui Tailwind v4 tokens directly to Vox Style Dictionary tokens.
   `@theme inline` inlines var(...) at utility call sites so nested [data-theme]
   previews and document.body Radix portals always resolve the active theme. */
@theme inline {
  --color-background: var(--color-bg-base);
  --color-foreground: var(--color-text-primary);

  --color-card: var(--color-bg-surface);
  --color-card-foreground: var(--color-text-primary);

  /* MUST be opaque (--color-overlay-solid) so .no-tauri headless CV captures
     (which disable backdrop-filter) never bleed background text through popovers! */
  --color-popover: var(--color-overlay-solid);
  --color-popover-foreground: var(--color-text-primary);

  --color-primary: var(--vox-shadcn-primary);
  --color-primary-foreground: var(--vox-shadcn-primary-fg);

  --color-secondary: var(--color-bg-elevated);
  --color-secondary-foreground: var(--color-text-primary);

  --color-muted: var(--color-bg-elevated);
  --color-muted-foreground: var(--color-text-muted);

  --color-accent: var(--color-overlay-hover);
  --color-accent-foreground: var(--color-text-primary);

  --color-destructive: var(--color-status-fail);
  --color-destructive-foreground: var(--vox-shadcn-destructive-fg);

  --color-border: var(--color-border-subtle);
  --color-input: var(--color-border-strong);
  --color-ring: var(--color-accent-default);

  --color-chart-1: var(--color-accent-default);
  --color-chart-2: var(--color-accent-secondary);
  --color-chart-3: var(--color-status-info);
  --color-chart-4: var(--color-status-warn);
  --color-chart-5: var(--color-status-fail);

  --radius-sm: var(--radius-sm);
  --radius-md: var(--radius-md);
  --radius-lg: var(--radius-lg);
  --radius-xl: var(--radius-xl);
}
```

And in `@layer base` (`src/index.css:203-221`), define the WCAG AA-verified foreground contrast pairs per theme (keeping existing `color-scheme` declarations untouched so `src/index.css.test.ts:24-27` stays green):

| Theme Selector | `--vox-shadcn-primary` | `--vox-shadcn-primary-fg` | Contrast Ratio | `--vox-shadcn-destructive-fg` |
|----------------|------------------------|---------------------------|----------------|-------------------------------|
| `:root, [data-theme="arcane"]` | `#C9A24A` (`--color-gold-500`) | `#1C140C` (`--color-carrara-ink`) | **7.4 : 1** (AAA) | `#FAF6EE` |
| `[data-theme="glacier"], [data-theme="travertine"]` | `#C9A24A` (`--color-gold-500`) | `#2A2620` (`--color-travertine-ink`) | **6.6 : 1** (AA+) | `#F6F1E7` |
| `[data-theme="void"], [data-theme="slate"]` | `#38BDF8` (`--color-accent-default`) | `#0C0E10` (`--color-basalt-900`) | **9.0 : 1** (AAA) | `#0C0E10` (on `#f87171` = **7.6 : 1**) |
| `[data-theme="high-contrast"]` | `var(--color-accent-default)` | `#000000` | **> 11.0 : 1** (AAA) | `#000000` |

### 3. Mandatory `src/index.css` Escape Hatch for `[data-slot]` Primitives

Update the two legacy global `<button>` normalization selectors in `src/index.css` so they do not clobber CVA/Radix state styles (`Switch`, `Checkbox`, `RadioGroup`, `SelectTrigger`, `Button` variants):

- **`src/index.css:311` (and `:hover` at `327`):** Append `:not([data-slot])` → `button:not([role="tab"]):not([data-roman-gauge="true"]):not([data-slot]):not([disabled])`
- **`src/index.css:1076`:** Append `:not([data-slot])` → `button:not([role="tab"]):not([data-roman-gauge="true"]):not([data-slot]):not([disabled])`

This single surgical guard preserves 100% of v6.5/v7.2 Carrara marble styling on un-migrated buttons while liberating every `src/components/ui/*` `[data-slot]` component to render clean CVA variants in both `arcane` and `slate`.

---

## Zero-Emoji Icon & Graphic System Architecture

1. **Two-Tier Offline Vector Semiotics (`lucide-react@^1.72.0` + `src/components/ui/Icons.tsx`):**
   - **Tier 1 — Standard Workbench Chrome (`lucide-react@^1.72.0`):** Use tree-shaken named imports (`Check`, `X`, `AlertTriangle`, `ChevronRight`, `ChevronDown`, `Pencil`, `Scissors`, `Lock`, `Zap`, `Settings2`, `GitBranch`, `Hammer`, `GripVertical`, `MoveDiagonal2`, `Search`, `SlidersHorizontal`, `ExternalLink`, `Loader2`) rendered with `strokeWidth={1.5}` and `aria-hidden="true"`.
   - **Tier 2 — Bespoke Romanesque Diegetic Semiotics (`src/components/ui/Icons.tsx`):** Retain and extend Vox's custom 24×24 stroke-`1.5` engraved icons (`laurelWreath`, `gladiusShield`, `denariusCoin`, `hourglassVia`, `brazierTorch`, `astrolabeRadar`, `aqueductArch`, `waxSeal`, `scrollTabula`, `romanGalley`, `anvilForge`, `romanColumn`, plus new `amphoraUrn` for `⚱` at `ImperiumInspector.tsx:823` and `solInvictus` sun-ray for `☼` at `LudusHud.tsx:76`).
2. **Close the CI/E2E Emoji & Dingbat Audit Blindspot:**
   - Remove `if (!el.closest('[data-testid="urbs-imperium-inspector"]'))` from `crates/vox-gui/ui/e2e/review/audits.ts:277`.
   - Add a fast static Vitest source guard (`src/__tests__/zeroEmojiAndDingbat.test.ts` using `sourceScan.ts`) scanning **all** `src/**/*.tsx` and `src/**/*.ts` (non-test) files for `[\u{1F300}-\u{1FAFF}\u{2600}-\u{27BF}]` and UI chrome dingbats (`✕`, `✓`, `✗`, `✎`, `✂`, `☼`, `★`, `☐`, `⋮⋮`, `▶`, `▼`), guaranteeing regressions fail in `<200ms` during `vitest run` before Playwright even launches.
3. **Concept-Art Graphic & Telemetry Richness (No New Heavy Runtime Needed):**
   - Compose existing `<DiegeticFxStage />` (`src/components/fx/DiegeticFxStage.tsx`), `<RomanBrassGauge />` (`RomanTooltip.tsx`), `<Sparkline />` (`Sparkline.tsx`), `<AgentTrajectoryBraid />`, and the upgraded shadcn `<ChartContainer />` (`recharts@3.8.1`) inside `<BentoGrid>` / `<BentoCard>` cells.

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Unified `radix-ui@^1.8.1` | 18 separate `@radix-ui/react-*` packages (`@radix-ui/react-popover@^1.3.1`, `@radix-ui/react-select@^2.4.1`, etc.) | Only if publishing individual `src/components/ui/*.tsx` files as standalone external npm micro-packages. Inside `vox-gui-ui` (a single Vite app bundle with `"sideEffects": false` tree-shaking and strict `knip` dependency checks), unified `radix-ui@^1.8.1` produces identical bundle output with zero `@radix-ui/react-compose-refs` version drift. |
| `@Base-UI/react` (`@base-ui-components/react`) | `radix-ui@^1.8.1` | Base UI 1.x is strong for greenfield design systems, but `vox-gui` already uses Radix (`Dialog`, `Slot`, `Tooltip`) and the user explicitly authorized official Radix / shadcn peer packages. Mixing Base UI and Radix focus-scopes/portals in the same Tauri webview causes competing `aria-hidden` / Escape-stack bugs. |
| `Sheet.tsx` built on `radix-ui` `Dialog` | `vaul@^1.1.2` | `vaul` (`1.1.2`) adds mobile touch-physics drag-to-dismiss gestures designed for iOS Safari bottom sheets. Vox Axis is a desktop Tauri workbench (`macOS`/`Windows`/`Linux`) where right/left inspector sheets (`DocViewerDrawer`, `ExplainDrawer`, `InspectorDrawer`) need deterministic keyboard focus trapping without accidental text-selection drag dismissal. |
| Existing `Toasts.tsx` (upgraded with CVA + SVG icons) | `sonner@^2.0.8` | Use `sonner` in apps without an existing toast state machine. Vox Axis already has a bespoke `groupKey`/`count` coalescing + `ToastCause` contract wired to Tauri IPC (`types/tauri.ts`, `Toasts.tsx:12-17`, `Toasts.test.tsx`); replacing it with `sonner` would break existing unit/E2E contracts for zero functional gain. |
| `react-resizable-panels@^4.22.1` + `dockview@6.6.1` (hybrid) | Using `dockview` everywhere OR `react-resizable-panels` everywhere | Use `dockview@6.6.1` when users tear off, tabify, and rearrange 14 dynamic panels (`DockWorkspaceShell.tsx`). Use `react-resizable-panels@^4.22.1` for deterministic 2-pane table + inspector splits (`RepositoryView`, `ModelsView`, `ExplorerView`) where tab chrome wastes 28px of vertical space. |
| `@tailwindcss/postcss@^4.3.5` | `@tailwindcss/vite@^4.3.5` | `@tailwindcss/vite@^4.3.5` supports Vite 8 (`peerDependencies: vite ^5.2..^8`) and is ~15% faster on cold CSS builds, but switching requires updating `vite.config.ts`, `dashboardBundleBudgetInProcess.ts`, and `knip.jsonc:44` (`postcss`). Optional clean-up, not required for v7.4. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `tailwindcss-animate` | Legacy Tailwind v3 JS plugin (`require("tailwindcss-animate")`). Incompatible with CSS-first Tailwind v4 `@import 'tailwindcss'` without `@plugin` compat shims, and officially deprecated by shadcn/ui. | `tw-animate-css@^1.4.0` via `@import 'tw-animate-css';` in `src/index.css`. |
| `@iconify/react` (runtime API loader) | Fetches icon SVGs dynamically over HTTPS from `api.iconify.design` at runtime — **violates Vox's core 100% Offline/Standalone Sovereignty constraint** (`PROJECT.md:449`) and causes blank icons in air-gapped Tauri builds and hermetic Playwright CI. | `lucide-react@^1.72.0` (bundled ESM SVGs) + `src/components/ui/Icons.tsx` bespoke Romanesque SVGs. |
| `react-icons` | Massive multi-pack barrel files mix incompatible icon grids (`16×16`, `24×24`, `512×512`), solid fills vs strokes, and hurt Vite dev-server cold-start / Rolldown prebundling. | `lucide-react@^1.72.0` (`24×24`, `strokeWidth={1.5}`) + `Icons.tsx`. |
| `framer-motion` / `motion` | Adds ~34 KB gzipped runtime for simple popover/tab/bento transitions, conflicts with `index.css:511` `@media (prefers-reduced-motion: reduce)` CSS rules unless manually plumbed on every `<motion.div>`, and is unnecessary alongside `<DiegeticFxStage>` canvas FX. | `tw-animate-css@^1.4.0` + Tailwind v4 CSS `@keyframes` (`src/index.css:28-60,411-463`). |
| `@mui/material`, `antd`, `@chakra-ui/react`, `@mantine/core` | Inject runtime CSS-in-JS (`@emotion/react`) or competing global CSS resets that destroy Vox's Style Dictionary 4-theme contract (`arcane` Carrara marble + `slate` basalt) and bloat Tauri bundle size by >150 KB gzipped. | Headless `radix-ui@^1.8.1` + `cva()` + Tailwind v4 `@theme inline`. |
| `react-grid-layout` | Legacy CommonJS/React-16/18 class lifecycle baggage and peer-dep friction on React 19; duplicates existing widget drag/resize grid already working and tested in `DashboardGrid.tsx` (`@dnd-kit/*`). | Existing `DashboardGrid.tsx` (`@dnd-kit/core@^6.3.1` + `@dnd-kit/sortable@^10.0.0`) for customizable dashboards, and pure CSS Grid `<BentoGrid>` (`@container`) for structured Concept-Art Bento surfaces. |
| Running `npx shadcn@latest init` blindly | `shadcn init` overwrites `src/index.css` with generic `oklch(...)` `:root`/`.dark` boilerplate, creates `components.json` assuming `@/*` path aliases (`tsconfig.json` currently has NO `paths` alias!), and wipes Vox's Roman marble/slate CSS rules and `index.css.test.ts` invariants. | Install peer packages explicitly via `pnpm` and author `src/components/ui/*.tsx` directly against `../../lib/cn` (or add `"@/*": ["./src/*"]` to `tsconfig.json` + `vite.config.ts` intentionally without running `shadcn init`). |
| Copying v3 `react-resizable-panels` or v8 `@tanstack/react-table` snippets | **Critical version trap:** `react-resizable-panels@^4.22.1` renamed `PanelGroup` → `Group`, `PanelResizeHandle` → `Separator`, `direction` → `orientation`, and numeric `defaultSize={50}` → `defaultSize="50%"`. `@tanstack/react-table@^9.3.1` renamed `useReactTable` → `useTable` with explicit `tableFeatures()`. | Follow the v4 `react-resizable-panels` and v9 `@tanstack/react-table` APIs documented in the Version Compatibility table below. |

---

## Stack Patterns by Variant

**If building a Concept-Art Bento Dashboard / Overview / Telemetry Surface (`Dashboard`, `NeedsYou`, `MensTraining`, `Scientia`, `Gamify`, `GsdStudio`):**
- Use `<BentoGrid>` (`@container` responsive 12-column CSS Grid) + `<BentoCard>` (`vox-marble-tablet` in `arcane`, `bg-card border-border` in `slate`, `min-h-0 flex flex-col`).
- Inside telemetry Bento cells, mount `<ChartContainer>` (`recharts@^3.8.1` with `ResponsiveContainer width="100%" height="100%"`) or `<DiegeticFxStage />` (`PhotorealisticCanvasFx.ts`) so `chartPanelFillRatio >= 0.84` at every resolution from `900×600` to `1920×1080`.
- Only use `@dnd-kit` (`DashboardGrid.tsx`) when user-customizable widget drag-reorder is an explicit feature of that surface.

**If building a Master-Detail Workbench Surface (`RepositoryView`, `ModelsView`, `ExplorerView`, `PoliciesView`, `ApprovalsView`, `CodeRabbitView`):**
- Use `<ResizablePanelGroup orientation="horizontal">` (`react-resizable-panels@^4.22.1`) above `960px` with `defaultSize="62%"` / `defaultSize="38%"` and `minSize="280px"`.
- Below `960px` (`compact` `900×600` CV Flywheel viewport), collapse the detail inspector into `<Sheet side="right">` (`radix-ui` `Dialog`) so table columns and inspector panes never collide (`table/inspector collisions == 0`).

**If building Multi-Panel Workspace Docking (`ChatSurface` + 14 companion panels):**
- Keep `DockWorkspaceShell.tsx` (`dockview@6.6.1` + `src/styles/dockview-vox.css`) as the sole multi-tab docking engine, and use Radix `<DropdownMenu>` / `<Popover>` / `<Select>` for its header layout switcher and `Loquela.tsx` single-row toolbar controls.

**If rendering any Icon, Status Indicator, or Control Affordance:**
- If it is general UI chrome (close, check, chevron, edit, filter, search, terminal, branch, lock, settings, grip, spinner), import from `lucide-react@^1.72.0` via `<VoxIcon>` (`strokeWidth={1.5}`).
- If it carries Romanesque / diegetic domain identity (medallion, laurel, gladius, denarius, brazier, astrolabe, aqueduct, wax seal, tabula, galley, forge, amphora), import from `src/components/ui/Icons.tsx`.
- **Never** render Unicode emojis or ASCII/dingbat text glyphs (`✓`, `✕`, `⚠`, `✎`, `✂`, `[Action]`).

---

## Version Compatibility

| Package | Verified Version | Compatible With | Critical Integration & API Notes |
|---------|------------------|-----------------|----------------------------------|
| `radix-ui` | `1.8.1` (`^1.8.1`) | `react@^19.2.8`, `react-dom@^19.2.8` | Official peer range includes `^19.0`. Import as `import { Dialog, Popover, Select, Slot, Tooltip } from 'radix-ui'` (each namespace exposes `.Root`, `.Trigger`, `.Portal`, `.Content`, etc.). Remove legacy `@radix-ui/react-{dialog,slot,tooltip}` to prevent duplicate `@radix-ui/react-compose-refs` trees. |
| *(Reference: standalone `@radix-ui/react-*`)* | `dialog@1.3.1`, `popover@1.3.1`, `select@2.4.1`, `dropdown-menu@2.2.1`, `tabs@1.2.1`, `switch@1.4.1`, `slider@1.6.1`, `checkbox@1.4.1`, `radio-group@1.5.1`, `tooltip@1.4.1`, `slot@1.5.0`, `scroll-area@1.4.0` | `react@^19.2.8` | Exact internal versions bundled inside `radix-ui@1.8.1`. Prefer unified `radix-ui@^1.8.1` so `knip.jsonc` (`dependencies: "error"`) stays green across incremental phase rollouts. |
| `react-resizable-panels` | `4.22.1` (`^4.22.1`) | `react@^19.2.8`, `@types/react@^19.2.18` | **v4 Breaking API:** Exports `Group`, `Panel`, `Separator`, `type PanelImperativeHandle` (NOT v3 `PanelGroup` / `PanelResizeHandle`). Use `orientation="horizontal" \| "vertical"` (NOT `direction`), CSS unit strings `defaultSize="60%"` / `minSize="260px"`, and `panelRef` (NOT `ref`) for imperative handles. |
| `@tanstack/react-table` | `9.3.1` (`^9.3.1`) | `react@^19.2.8`, `@tanstack/react-virtual@^3.14.10` | **v9 Breaking API:** Exports `useTable` + `tableFeatures(...)` + `<table.FlexRender />` (NOT v8 `useReactTable`). Tree-shakes unused features cleanly in Vite 8 / Rolldown. |
| `cmdk` | `1.1.1` (`^1.1.1`) | `react@^19.2.8`, `radix-ui@^1.8.1` | Official peer range `^18 \|\| ^19`. Style via `[cmdk-root]`, `[cmdk-input]`, `[cmdk-item][data-selected="true"]` bound to `bg-accent text-accent-foreground`. |
| `class-variance-authority` | `0.7.1` (`^0.7.1`) | `typescript@^5.0.2`, `tailwind-merge@^3.6.0` | Framework-agnostic pure TS utility; compose output through `cn(buttonVariants({ variant, size, className }))`. |
| `tw-animate-css` | `1.4.0` (`^1.4.0`) | `tailwindcss@^4.3.5` | CSS-only import (`@import 'tw-animate-css';` right after `@import 'tailwindcss';` in `src/index.css`). Respects `src/index.css:511` `@media (prefers-reduced-motion: reduce)` automatically. |
| `lucide-react` | `1.72.0` (`^1.72.0`) | `react@^19.2.8` | Official peer range includes `^19.0.0`. Tree-shakes cleanly under Vite 8 (`target: 'es2022'`). |
| `recharts` | `3.8.1` *(installed)* | `react@^19.2.8`, shadcn `Chart.tsx` | **Recharts v3 Selector Note:** Cartesian axis tick labels use `.recharts-cartesian-axis-tick-label_text` (not v2 `.recharts-cartesian-axis-tick_text`). Reference CSS variables directly as `var(--color-chart-1)` without `hsl(...)` wrappers. |
| `dockview` | `6.6.1` *(installed)* | `react@^19.2.8`, Tailwind v4 | Keep definite-pixel wrapper in `DockWorkspaceShell.tsx:68-89` and `.dockview-theme-vox .dv-shell` specificity override in `src/styles/dockview-vox.css:23-24`. |

---

## Sources

- Live npm registry inspection (`pnpm view <pkg> version peerDependencies dependencies exports sideEffects scripts dist-tags`, verified 2026-11-14): `radix-ui@1.8.1`, `cmdk@1.1.1`, `react-resizable-panels@4.22.1`, `class-variance-authority@0.7.1`, `tw-animate-css@1.4.0`, `@tanstack/react-table@9.3.1`, `lucide-react@1.72.0`, `tailwindcss@4.3.5`, `@tailwindcss/vite@4.3.5`, `sonner@2.0.8`, `vaul@1.1.2` — **HIGH confidence**
- Live Vox Axis codebase audit (`crates/vox-gui/ui/package.json`, `pnpm-workspace.yaml`, `knip.jsonc`, `vite.config.ts`, `tsconfig.json`, `style-dictionary.config.mjs`, `src/index.css`, `src/index.css.test.ts`, `src/__tests__/statusColorTokens.test.ts`, `src/styles/__tests__/contrast.test.ts`, `src/components/ui/*.tsx`, `src/components/dashboard/DashboardGrid.tsx`, `src/components/dock/DockWorkspaceShell.tsx`, `e2e/review/audits.ts`, `e2e/review/vlmVisionFlywheel.ts`) — **HIGH confidence**
- Official shadcn/ui Tailwind v4 + React 19 migration guides (`ui.shadcn.com/docs/tailwind-v4`, `ui.shadcn.com/docs/react-19`, unified `radix-ui` & `tw-animate-css` architecture, Recharts v3 `chart.tsx` updates) — **HIGH confidence**
- `react-resizable-panels` v4 (`Group`/`Panel`/`Separator`) & `@tanstack/react-table` v9 (`useTable`/`tableFeatures`) official release migration documentation — **HIGH confidence**

---
*Stack research for: Vox Axis Tauri 2 + React 19.2 + Tailwind CSS v4.3 Workbench (Milestone v7.4)*
*Researched: 2026-11-14*
