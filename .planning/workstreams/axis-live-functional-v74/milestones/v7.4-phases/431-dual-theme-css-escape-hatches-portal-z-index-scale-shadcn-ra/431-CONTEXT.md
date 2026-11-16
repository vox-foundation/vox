# Phase 431: Dual-Theme CSS Escape Hatches, Portal Z-Index Scale & shadcn/Radix Primitive Kit - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Upgrade `crates/vox-gui/ui` to unified `radix-ui@^1.8.1`, `class-variance-authority@^0.7.1`, `tw-animate-css@^1.4.0`, `cmdk@^1.1.1`, `react-resizable-panels@^4.22.1`, `@tanstack/react-table@^9.3.1`, `lucide-react@^1.72.0`, and `tailwindcss@^4.3.5`, scope legacy `index.css` `!important` selectors with `:not([data-slot])` and overlay scrim guards, establish the deterministic `@theme` `--z-*` portal stacking ladder and `body[data-scroll-locked]` overflow guard, and ship Vox-native `data-slot` primitives (`Button`, `Dialog`, `Popover`, `Select`, `DropdownMenu`, `Tabs`, `Switch`, `Slider`, `RomanRotaryKnob`, `Checkbox`, `RadioGroup`, `Sheet`, `Accordion`, `Collapsible`, `ScrollArea`, `AlertDialog`, `Command`, `Resizable`, `DataTable`, `Chart`) wired to active pilot consumers across all 4 Style Dictionary themes (`arcane`, `glacier`, `slate`, `high-contrast`).

</domain>

<decisions>
## Implementation Decisions

### Dual-Theme Token & CSS Specificity Contract
- **D-431-01 (`data-slot` Escape Hatch):** Tag every primitive in `crates/vox-gui/ui/src/components/ui/*` with `data-slot="<primitive>"` and append `:not([data-slot])` plus ARIA form-role exclusions (`switch`, `checkbox`, `radio`, `combobox`, `menuitem`, `option`) to legacy `!important` marble button rules in `crates/vox-gui/ui/src/index.css` (`:311`, `:327`, `:1076`).
- **D-431-02 (Translucent Modal Scrims):** Exclude `[data-slot$="-overlay"]` from `index.css:708-742` `[class*="bg-black"]` light-mode rewrites so `bg-black/55 backdrop-blur-[2px]` scrims stay translucent instead of turning into solid `#FAF5E8` Carrara marble walls.
- **D-431-03 (Style Dictionary `@theme inline` Aliasing):** Alias `--color-popover`, `--color-card`, `--color-primary`, `--color-muted`, `--color-border`, and `--color-ring` via `@theme inline` onto existing Style Dictionary CSS variables (`#C9A24A`/`#1C140C` `7.4:1` in `arcane`; `#38BDF8`/`#0C0E10` `9.0:1` in `slate`).
- **D-431-04 (Audit-Safe Animations):** Install `tw-animate-css@^1.4.0` for portal enter/exit keyframes and use explicit `transition-colors` / `transition-opacity` / `transition-transform` (never `transition-all`, preserving `vox/ui/layout-and-perf-hazards`).

### Tactile `RomanRotaryKnob` & Radix `Slider` Interaction
- **D-431-05 (Vertical Pointer Capture + Fine Detents):** `RomanRotaryKnob` uses vertical pointer-capture drag (`140px` full sweep, `Shift` `0.1x` fine mode), double-click reset to `defaultValue`, `ArrowUp/Down/Left/Right` + `Home/End` keyboard steps, and optional tick detents.
- **D-431-06 (Dual-Theme Chronometer Rendering):** Render a 270° SVG chronometer arc (`-135°..+135°`) with knurled brass bevel + sunlit gold progress arc in `arcane`/`glacier`, and anodized precision slate bezel + cyan arc in `slate`/`high-contrast`, paired with a live formatted value readout.
- **D-431-07 (`onValueCommit` IPC Debounce):** Update visual drag values at 60fps via `onValueChange` and fire Tauri/MCP persistence only on pointer release or settled keyboard step (`onValueCommit` + `150ms` trailing debounce).
- **D-431-08 (Mandatory ARIA Slider Naming):** Require `ariaLabel: string` in TypeScript props on both `RomanRotaryKnob` (`role="slider"`) and `<SliderPrimitive.Thumb>`, with finite clamped `aria-valuemin`/`aria-valuemax`/`aria-valuenow` and `aria-valuetext`.

### Portal Z-Index Ladder, `cmdk` Omnibar & Resizable Splitters
- **D-431-09 (`@theme` Z-Index Ladder):** Enforce `--z-shell-chrome: 40` (`BottomStatusBar`) < `--z-drawer: 60` (`Sheet`) < `--z-dialog: 70` (`Dialog`/`CommandDialog`) < `--z-popover: 80` (`Select`/`DropdownMenu`/`Popover`) < `--z-toast: 90` (`Toasts`) < `--z-tooltip: 100` (`Tooltip`/`RomanTooltip`).
- **D-431-10 (Scroll-Lock Overflow Guard):** Switch `AppShell.tsx` root from `h-screen w-screen` to `h-dvh w-full max-w-full overflow-hidden` and neutralize `body[data-scroll-locked]` horizontal margin compensation (`outerOverflowPx === 0`).
- **D-431-11 (`react-resizable-panels` v4 vs `dockview`):** Use `react-resizable-panels@^4.22.1` (`Group`/`Panel`/`Separator` with visible grip handle + keyboard resize) for 2/3-pane master-detail surfaces, retaining `dockview@6.6.1` strictly for `DockWorkspaceShell.tsx`.
- **D-431-12 (Zero-Unused-File `knip` Compliance):** Wire every new Phase 431 `src/components/ui/*` primitive into Vitest contract suites and active pilot consumers (`AppShell`, `Omnibar`, `Loquela`, `SettingsView`, `Toasts`) in the same phase so `pnpm --dir crates/vox-gui/ui knip` exits `0`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-gui/ui/src/components/ui/{Button,Dialog,Popover,Tooltip,DataTable,Toasts}.tsx` — existing primitives to upgrade in-place onto unified `radix-ui@^1.8.1` + `data-slot` without breaking existing imports.
- `crates/vox-gui/ui/style-dictionary.config.mjs` & generated CSS token sheets (`tokens.{arcane,glacier,void,high-contrast}.css`) — existing 4-theme token SSOT.

### Established Patterns
- `clsx` + `tailwind-merge` utility helper (`cn`) for deterministic Tailwind v4 class merging.
- `themeContract.test.ts` and `knip.jsonc` (`files: "error"`, `dependencies: "error"`) run in CI and must stay 100% green.

### Integration Points
- `crates/vox-gui/ui/package.json` — upgrade `@radix-ui/react-{dialog,slot,tooltip}` -> unified `radix-ui@^1.8.1` + `cva`, `tw-animate-css`, `cmdk`, `react-resizable-panels`, `@tanstack/react-table`, `lucide-react@^1.72.0`, `tailwindcss@^4.3.5`.
- `crates/vox-gui/ui/src/index.css` — `@import "tw-animate-css"`, `@theme inline` token/z-index bindings, `:not([data-slot])` specificity guards, portal theme selectors (`[role="listbox"]`, `[role="menu"]`, `[data-radix-popper-content-wrapper]`).
- `crates/vox-gui/ui/src/components/shell/AppShell.tsx`, `Omnibar.tsx`, `Toasts.tsx` — root `TooltipProvider`, `h-dvh w-full` scroll-lock guard, `--z-*` ladder, and `cmdk` pilot integration.

</code_context>

<specifics>
## Specific Ideas

- Keep `RomanRotaryKnob` purely vector SVG + Pointer Capture (`< 4 KB`, zero WebGL canvas overhead) with crisp tick marks and detents so operators immediately feel real hardware precision when adjusting model temperature, VAD sensitivity, or routing weights.

</specifics>

<deferred>
## Deferred Ideas

- Full surface-wide replacement of all 28 native `<select>`s and 5 `<input type="range">`s across all 33 surfaces happens in **Phase 434** once Phase 431 lands the primitive kit and Phase 433 closes IPC/DTO bypasses.

</deferred>
