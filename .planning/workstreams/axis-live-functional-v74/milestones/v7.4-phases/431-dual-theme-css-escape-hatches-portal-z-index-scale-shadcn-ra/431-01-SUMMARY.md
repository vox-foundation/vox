---
phase: 431
plan: 01
subsystem: vox-gui/ui
tags:
  - shadcn
  - radix-ui
  - dual-theme
  - z-index
  - rotary-knob
  - accessibility
dependency_graph:
  requires: []
  provides:
    - unified radix-ui@1.8.1 + cva + cmdk + react-resizable-panels v4 + @tanstack/react-table v9 primitive foundation
    - dual-theme CSS escape hatches for data-slot primitives and translucent modal scrims
    - canonical 6-rung z-index ladder (40/60/70/80/90/100) and scroll-lock margin guard
    - 20 accessible data-slot UI primitives including vector RomanRotaryKnob
  affects:
    - crates/vox-gui/ui/package.json
    - crates/vox-gui/ui/knip.jsonc
    - crates/vox-gui/ui/src/index.css
    - crates/vox-gui/ui/src/components/layout/AppShell.tsx
    - crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx
    - crates/vox-gui/ui/src/components/layout/Omnibar.tsx
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx
    - crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Settings/WorkbenchPrimitivesPilotCard.tsx
    - crates/vox-gui/ui/src/components/ui/
tech_stack:
  added:
    - radix-ui@^1.8.1
    - class-variance-authority@^0.7.1
    - cmdk@^1.1.1
    - react-resizable-panels@^4.22.1
    - "@tanstack/react-table@^9.3.1"
    - lucide-react@^1.72.0
    - tw-animate-css@^1.4.0
  removed:
    - "@radix-ui/react-dialog"
    - "@radix-ui/react-slot"
    - "@radix-ui/react-tooltip"
key_files:
  created:
    - crates/vox-gui/ui/src/components/ui/Select.tsx
    - crates/vox-gui/ui/src/components/ui/DropdownMenu.tsx
    - crates/vox-gui/ui/src/components/ui/Tabs.tsx
    - crates/vox-gui/ui/src/components/ui/Switch.tsx
    - crates/vox-gui/ui/src/components/ui/Slider.tsx
    - crates/vox-gui/ui/src/components/ui/RomanRotaryKnob.tsx
    - crates/vox-gui/ui/src/components/ui/Checkbox.tsx
    - crates/vox-gui/ui/src/components/ui/RadioGroup.tsx
    - crates/vox-gui/ui/src/components/ui/Sheet.tsx
    - crates/vox-gui/ui/src/components/ui/Accordion.tsx
    - crates/vox-gui/ui/src/components/ui/Collapsible.tsx
    - crates/vox-gui/ui/src/components/ui/ScrollArea.tsx
    - crates/vox-gui/ui/src/components/ui/Command.tsx
    - crates/vox-gui/ui/src/components/ui/Resizable.tsx
    - crates/vox-gui/ui/src/components/ui/Chart.tsx
    - crates/vox-gui/ui/src/components/surfaces/Settings/WorkbenchPrimitivesPilotCard.tsx
    - crates/vox-gui/ui/src/components/ui/ShadcnPrimitives.test.tsx
  modified:
    - crates/vox-gui/ui/package.json
    - crates/vox-gui/ui/pnpm-lock.yaml
    - crates/vox-gui/ui/knip.jsonc
    - crates/vox-gui/ui/src/index.css
    - crates/vox-gui/ui/src/test-setup.ts
    - crates/vox-gui/ui/src/components/layout/AppShell.tsx
    - crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx
    - crates/vox-gui/ui/src/components/layout/Omnibar.tsx
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx
    - crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx
    - crates/vox-gui/ui/src/components/ui/Button.tsx
    - crates/vox-gui/ui/src/components/ui/Dialog.tsx
    - crates/vox-gui/ui/src/components/ui/Popover.tsx
    - crates/vox-gui/ui/src/components/ui/Tooltip.tsx
    - crates/vox-gui/ui/src/components/ui/RomanTooltip.tsx
    - crates/vox-gui/ui/src/components/ui/Toasts.tsx
    - crates/vox-gui/ui/src/components/ui/DataTable.tsx
decisions:
  - Normalized --z-shell-chrome (40) through --z-tooltip (100) on both :root and [data-theme] in index.css @layer base after Style Dictionary imports so legacy generated --z-toast: 1500 values in slate/high-contrast token outputs cannot invert the 6-rung elevation ladder.
  - Preserved backwards-compatible <Popover open={...}> sibling semantics for existing callers while exporting portaled Radix PopoverRoot/PopoverTrigger/PopoverContent/PopoverAnchor on --z-popover (80).
  - Integrated @tanstack/react-table v9 (useTable + rowSortingFeature + createSortedRowModel) inside DataTable.tsx while keeping the existing DataTableProps<T> contract intact for all downstream surfaces.
requirements_completed:
  - AXIS-SHADCN-01
  - AXIS-SHADCN-02
  - AXIS-SHADCN-03
  - AXIS-SHADCN-04
metrics:
  completed_date: "2026-11-14"
  tasks_completed: 3
  files_created: 17
  files_modified: 17
---

# Phase 431 Plan 01: Dual-Theme CSS Escape Hatches, Portal Z-Index Scale & shadcn/Radix Primitive Kit Summary

Established the unified `radix-ui@1.8.1` + Tailwind v4 `@theme inline` primitive architecture across `crates/vox-gui/ui`, neutralizing legacy `:is(main, [role="dialog"]) button` `!important` overrides via `[data-slot]` escape hatches, enforcing a monotonic 6-rung z-index ladder (`40..100`), shipping 20 accessible dual-theme `data-slot` primitives including the vector `RomanRotaryKnob`, and wiring active pilot consumers with zero dead files or unused exports.

## Task Execution Breakdown

### Task 1: Dependencies, CSS Escape Hatches, Z-Index Ladder & Scroll-Lock Guard (`AXIS-SHADCN-01`, `AXIS-SHADCN-04`)
- Upgraded `crates/vox-gui/ui/package.json` with runtime dependencies `radix-ui@^1.8.1`, `class-variance-authority@^0.7.1`, `cmdk@^1.1.1`, `react-resizable-panels@^4.22.1`, `@tanstack/react-table@^9.3.1`, and `lucide-react@^1.72.0`, plus devDependencies `tw-animate-css@^1.4.0`, `tailwindcss@^4.3.5`, and `@tailwindcss/postcss@^4.3.5`.
- Migrated all `@radix-ui/react-*` imports across `Dialog.tsx`, `Button.tsx`, `Tooltip.tsx`, and `RomanTooltip.tsx` to unified `from 'radix-ui'`, then removed `@radix-ui/react-dialog`, `@radix-ui/react-slot`, and `@radix-ui/react-tooltip`.
- Updated `crates/vox-gui/ui/src/index.css`:
  - Added `@import 'tw-animate-css';`.
  - Declared `@theme inline` shadcn semantic aliases (`--color-background`, `--color-foreground`, `--color-card`, `--color-popover`, `--color-primary`, `--color-secondary`, `--color-muted`, `--color-accent`, `--color-destructive`, `--color-border`, `--color-input`, `--color-ring`, `--color-chart-1..5`, `--radius-sm/md/lg/xl`) mapped directly onto Style Dictionary CSS variables, plus the 6-rung z-index ladder (`--z-shell-chrome: 40`, `--z-drawer: 60`, `--z-dialog: 70`, `--z-popover: 80`, `--z-toast: 90`, `--z-tooltip: 100`).
  - Appended `:not([data-slot]):not([role="switch"]):not([role="checkbox"]):not([role="radio"]):not([role="combobox"])` exclusions to legacy `!important` button overrides in `index.css`.
  - Excluded `:not([data-slot$="-overlay"])` from `[class*="bg-black"]` light-mode rewrites so modal/sheet/alert-dialog scrims remain translucent dark (`bg-black/55`).
  - Extended portal theme inheritance rules to `[data-radix-popper-content-wrapper]`, `[role="listbox"]`, `[role="menu"]`, and `[role="alertdialog"]`.
  - Added `html body[data-scroll-locked] { margin-right: 0 !important; padding-right: 0 !important; }` to prevent horizontal layout jumps when Radix modal overlays lock document scroll.
- Updated `AppShell.tsx` root container from `h-screen w-screen` to `h-dvh w-full max-w-full overflow-hidden` wrapped in a root `<TooltipProvider delayDuration={280}>`, and aligned `BottomStatusBar.tsx` (`z-(--z-shell-chrome)`), `Toasts.tsx` (`z-(--z-toast)`), and `RomanTooltip.tsx` (`z-(--z-tooltip)`).

### Task 2: 20-Component `data-slot` Primitive Kit & `RomanRotaryKnob` (`AXIS-SHADCN-01`, `AXIS-SHADCN-02`, `AXIS-SHADCN-03`)
- Upgraded in-place with backwards compatibility and zero `transition-all`:
  - `Button.tsx` (`cva` variants `primary`/`secondary`/`ghost`/`destructive`/`brass`/`outline`, sizes `xs`/`sm`/`md`/`lg`/`icon`, `asChild` via `SlotPrimitive.Root`, `data-slot="button"`).
  - `Dialog.tsx` (`data-slot="dialog-overlay"`, `z-(--z-dialog)`, plus `AlertDialog` family with `data-slot="alert-dialog-overlay"`).
  - `Popover.tsx` (preserves legacy `<Popover open>` sibling API while adding portaled `PopoverRoot`, `PopoverTrigger`, `PopoverContent`, `PopoverAnchor` at `z-(--z-popover)`).
  - `Tooltip.tsx` (unified `radix-ui` `TooltipPrimitive` at `z-(--z-tooltip)` with optional standalone provider fallback).
  - `DataTable.tsx` (`@tanstack/react-table` v9 `useTable` + `rowSortingFeature` + `createSortedRowModel` integration preserving `DataTableProps<T>`).
- Authored 15 new dual-theme primitives under `crates/vox-gui/ui/src/components/ui/`:
  - `Select.tsx`, `DropdownMenu.tsx`, `Tabs.tsx`, `Switch.tsx`, `Slider.tsx` (mandatory `ariaLabel: string` forwarded onto `SliderPrimitive.Thumb`, `onValueCommit`), `Checkbox.tsx`, `RadioGroup.tsx`, `Sheet.tsx` (`z-(--z-drawer)` + `data-slot="sheet-overlay"`), `Accordion.tsx`, `Collapsible.tsx`, `ScrollArea.tsx` (`tabIndex={0}`, `role="region"`, `ariaLabel`), `Command.tsx` (`cmdk` + `CommandDialog`), `Resizable.tsx` (`react-resizable-panels` v4 `Group`/`Panel`/`Separator` with grip handle), and `Chart.tsx` (`Recharts` responsive container + CSS variable theme injector + tooltip/legend primitives).
  - `RomanRotaryKnob.tsx`: pure vector SVG 270-degree rotary dial (`-135deg..+135deg`), vertical pointer-capture drag (`140px` full sweep), `Shift` `0.1x` fine-adjustment mode, double-click reset to `defaultValue`, `ArrowUp/Down/Left/Right` + `Home/End` keyboard stepping, optional snap `detents`, full `role="slider"` ARIA contract (`aria-label`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuetext`), `onChange` vs `onValueCommit` separation, and dual-theme brass/gold vs slate/cyan token styling (`sm: 36px`, `md: 48px`, `lg: 64px`).

### Task 3: Pilot Wiring, Dead-Code Hygiene & Verification (`AXIS-SHADCN-01..04`)
- Wired active pilot call sites across:
  - `Omnibar.tsx` (`CommandShortcut` keychord hints + accessible `CommandDialog` launcher binding).
  - `Loquela.tsx` (`PopoverRoot`/`PopoverTrigger`/`PopoverContent` Precision & Sampling popover hosting `RomanRotaryKnob` for Temperature and `Slider` with `onValueCommit` for Top-P, with outside-click portal guard).
  - `SettingsView.tsx` + `WorkbenchPrimitivesPilotCard.tsx` (exercising `Select`, `Switch`, `Checkbox`, `RadioGroup`, `Tabs`, `DropdownMenu`, `Accordion`, `Collapsible`, `ScrollArea`, `Resizable`, `Sheet`, `AlertDialog`, `Command`, `Chart`, `RomanRotaryKnob`, `Slider`, and `Button` variants).
- Authored `crates/vox-gui/ui/src/components/ui/ShadcnPrimitives.test.tsx` (11 comprehensive tests covering CSS escape hatches, z-index ladder ordering, `RomanRotaryKnob` pointer drag / `onValueCommit` / `Shift` `0.1x` fine mode / double-click reset / keyboard steps / detents / ARIA, `Slider` thumb `aria-label` + `onValueCommit`, `Select`, `Switch`, `Checkbox`, `RadioGroup`, `Dialog`/`AlertDialog`/`Sheet` `-overlay` slots, `Popover`, `DropdownMenu`, `Tooltip`, `Tabs`, `Accordion`, `Collapsible`, `ScrollArea`, `Command` filtering, `Resizable` grip, `DataTable` TanStack sorting, and `Chart` CSS variable injection).

## Deviations from Plan

- **[Rule 1 - Bug] Style Dictionary `--z-toast: 1500` token override:** `tokens.slate.generated.css` and `tokens.contrast.generated.css` emit `--z-toast: 1500` on `[data-theme="slate"]` and `[data-theme="high-contrast"]`, which would place toasts above `--z-tooltip: 100`. Resolved by pinning the 6-rung z-index ladder (`40..100`) on `:root, [data-theme]` in `index.css` `@layer base` after the generated token imports.
- **[Rule 1 - Bug] Chat type-scale floor in `Loquela.tsx`:** `chatTypeScale.test.ts` enforces an `11px` floor across chat/composer surfaces; adjusted two `text-[10px]` labels in the new `Loquela.tsx` sampling popover trigger/header to `text-[11px]`.
- **[Rule 3 - Blocking] `knip` CSS-only devDependency recognition:** Added `"tw-animate-css"` alongside `"postcss"` in `crates/vox-gui/ui/knip.jsonc` `ignoreDependencies` because `tw-animate-css` is consumed via CSS `@import` in `src/index.css`, and pruned 3 unreferenced internal re-exports (`DialogPortal`, `DialogClose`, `DialogOverlay`) from `Dialog.tsx` so zero Phase 431 symbols appear in `knip` unused-export reports.
