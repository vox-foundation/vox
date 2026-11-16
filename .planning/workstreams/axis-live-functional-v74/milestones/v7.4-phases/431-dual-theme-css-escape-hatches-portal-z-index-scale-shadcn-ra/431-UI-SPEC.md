---
phase: 431
slug: dual-theme-css-escape-hatches-portal-z-index-scale-shadcn-radix-primitive-kit
title: "Phase 431 UI Design Contract — Dual-Theme CSS Escape Hatches, Portal Z-Index Ladder & shadcn/Radix Primitive Kit"
status: approved
created: 2026-11-14
---

# Phase 431 — UI Design Contract (`431-UI-SPEC.md`)

## 1. Executive Summary & Problem Statement

Auditing `crates/vox-gui/ui` across all 4 Style Dictionary themes (`arcane`, `glacier`, `slate`, `high-contrast`) exposed four foundational UI primitive blockers:
1. **CSS Specificity Collisions (`crates/vox-gui/ui/src/index.css`):** Legacy `:is(main, [role="dialog"]) button:not([role="tab"])` rules (`:311`, `:327`, `:1076`) clobber Radix `<button role="switch"|"checkbox"|"radio"|"combobox">` controls, and `:is(body, ...) [class*="bg-black"]` (`:708-742`) turns `bg-black/60` modal scrims into opaque `#FAF5E8` Carrara cream walls.
2. **Missing Keyframe Animations & Fragmented Radix Packages:** `Dialog.tsx` and `Tooltip.tsx` emit dead `animate-in fade-in-0 zoom-in-95` classes because `tw-animate-css` is absent, while 3 standalone `@radix-ui/react-*` packages lack `Select`, `Switch`, `Slider`, `Tabs`, `DropdownMenu`, `Sheet`, `Accordion`, `Collapsible`, `ScrollArea`, and `AlertDialog`.
3. **Portal Z-Index Inversion & Scroll-Lock Overflow:** `Toasts.tsx` (`z-40`) hides behind `Dialog.tsx` (`z-50`), while `AppShell.tsx` (`w-screen`) overflows horizontally when Radix `body[data-scroll-locked]` applies scrollbar compensation (`outerOverflowPx > 0`).
4. **Absence of Tactile Continuous Controls, Command Comboboxes & Split Panes:** Operators lack a hardware-grade `RomanRotaryKnob`, accessible Radix `Slider` (`onValueCommit` debounce), `cmdk` command palette, `@tanstack/react-table` v9 sorting/filtering, and `react-resizable-panels` v4 master-detail splitters.

---

## 2. Visual & Architectural Contract

### 2.1 Token & CSS Specificity Layer (`crates/vox-gui/ui/src/index.css`)
- Import `tw-animate-css` and define `@theme inline` aliases mapping shadcn semantic slots (`--color-popover`, `--color-card`, `--color-primary`, `--color-muted`, `--color-border`, `--color-ring`) onto Style Dictionary CSS variables (`>= 7.4:1` contrast in `arcane`, `>= 9.0:1` in `slate`).
- Define deterministic z-index tokens: `--z-shell-chrome: 40`, `--z-drawer: 60`, `--z-dialog: 70`, `--z-popover: 80`, `--z-toast: 90`, `--z-tooltip: 100`.
- Append `:not([data-slot]):not([role="switch"]):not([role="checkbox"]):not([role="radio"]):not([role="combobox"])` to legacy button catch-alls (`:311`, `:327`, `:1076`) and `:not([data-slot$="-overlay"])` to `[class*="bg-black"]` rewrites (`:708-742`).
- Extend portal container selectors to `[data-radix-popper-content-wrapper]`, `[role="listbox"]`, `[role="menu"]`, and `[role="alertdialog"]`.
- Switch `AppShell.tsx` root container to `h-dvh w-full max-w-full overflow-hidden` and lock `html body[data-scroll-locked] { margin-right: 0 !important; }`.

### 2.2 `data-slot` Primitive Suite (`crates/vox-gui/ui/src/components/ui/*`)
- Every primitive emits `data-slot="<primitive>"`, uses explicit `transition-colors` / `transition-opacity` / `transition-transform` (never `transition-all`), and enforces WCAG AA contrast across all 4 themes:
  - Upgraded in-place: `Button.tsx` (`cva`), `Dialog.tsx` (`AlertDialog` + scrim `data-slot="dialog-overlay"`), `Popover.tsx` (`Radix Popover.Portal`), `Tooltip.tsx`, `DataTable.tsx` (`@tanstack/react-table`), `Toasts.tsx` (`z-(--z-toast)`).
  - Added and wired to active pilot consumers (`knip` `0` unused files): `Select.tsx`, `DropdownMenu.tsx`, `Tabs.tsx`, `Switch.tsx`, `Slider.tsx` (mandatory `ariaLabel` on `Thumb` + `onValueCommit`), `RomanRotaryKnob.tsx` (270° SVG chronometer, vertical pointer-capture drag, `Shift` `0.1x` fine mode, double-click reset, keyboard steps, detents), `Checkbox.tsx`, `RadioGroup.tsx`, `Sheet.tsx`, `Accordion.tsx`, `Collapsible.tsx`, `ScrollArea.tsx` (`tabIndex={0}` focusable viewport), `Command.tsx` (`cmdk`), `Resizable.tsx` (`react-resizable-panels` v4 `Group`/`Panel`/`Separator`), `Chart.tsx` (`Recharts` container).

### 2.3 Verification Invariants
- `pnpm --dir crates/vox-gui/ui build`, `pnpm --dir crates/vox-gui/ui test`, `pnpm --dir crates/vox-gui/ui knip`, `themeContract.test.ts`, and `toestub` (`vox/ui/layout-and-perf-hazards`) all exit `0`.
