---
phase: 431
plan: 01
status: clean
reviewed_at: "2026-11-14T06:13:00Z"
reviewer: self-review-gate
scope:
  - crates/vox-gui/ui/package.json
  - crates/vox-gui/ui/knip.jsonc
  - crates/vox-gui/ui/src/index.css
  - crates/vox-gui/ui/src/lib/cn.ts
  - crates/vox-gui/ui/src/components/layout/AppShell.tsx
  - crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx
  - crates/vox-gui/ui/src/components/layout/Omnibar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx
  - crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Settings/WorkbenchPrimitivesPilotCard.tsx
  - crates/vox-gui/ui/src/components/ui/
findings_open: 0
findings_resolved: 3
---

# Phase 431 Code Review (`status: clean`)

## Review Axes Audited

1. **CSS Specificity & Dual-Theme Escape Hatches (`AXIS-SHADCN-01`, `AXIS-SHADCN-04`):**
   - Verified `:not([data-slot])` in Tailwind v4 has zero-specificity `:not(...)` overhead? No: in CSS Selectors Level 4, `:not([data-slot]):not([role="switch"])...` carries specificity `(0, 5, 0)` on top of the legacy selector, cleanly excluding every `[data-slot]` element and ARIA form role (`switch`, `checkbox`, `radio`, `combobox`) from `!important` background/color/padding overrides without mutating legacy non-slotted buttons.
   - Verified `[class*="bg-black"]:not([data-slot$="-overlay"])` preserves translucent `bg-black/55` scrims on `dialog-overlay`, `alert-dialog-overlay`, and `sheet-overlay` across Travertine and Arcane Parchment light themes.
   - Verified `html body[data-scroll-locked]` neutralizes `margin-right` and `padding-right` shifts alongside `AppShell.tsx` `h-dvh w-full max-w-full overflow-hidden`.
   - Verified zero hardcoded dark-brown hexes, inline dark RGBAs, or banned font literals exist in any Phase 431 file (`themeContract.test.ts` reports zero violations on Phase 431 paths).

2. **Performance, Static Hygiene & Line Budgets (`toestub` + `vox/ui/layout-and-perf-hazards`):**
   - Ran `./target/debug/toestub crates/vox-gui/ui/src/components/ui --min-severity error`: `0` issues (`No issues found — code is clean!`).
   - Ran `./target/debug/toestub crates/vox-gui/ui/src/components/surfaces/Settings/WorkbenchPrimitivesPilotCard.tsx --min-severity info`: `0` errors, `0` warnings.
   - Verified zero `transition-all` classes and zero raw un-rAF'd `new ResizeObserver(` instantiations across all 20 primitives and pilot files.
   - Verified every new file stays well below the 450-line ceiling (`RomanRotaryKnob.tsx`: 280 lines; `DataTable.tsx`: 236 lines; `Dialog.tsx`: 209 lines; `Chart.tsx`: 221 lines; `WorkbenchPrimitivesPilotCard.tsx`: 383 lines).
   - Verified zero emoji or Unicode dingbat characters in new files; all indicators use `lucide-react` stroke icons (`strokeWidth={1.75}`).

3. **Accessibility & Interaction Contracts (`AXIS-SHADCN-02`, `AXIS-SHADCN-03`):**
   - `Slider.tsx`: TypeScript-mandatory `ariaLabel: string` forwarded onto every `SliderPrimitive.Thumb`; supports `onValueCommit` for debounced backend commits.
   - `RomanRotaryKnob.tsx`: `role="slider"`, `tabIndex={0}`, `aria-label`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuetext`, vertical pointer-capture drag (`140px` sweep), `Shift` `0.1x` fine adjustment, double-click reset to `defaultValue`, `ArrowUp/Down/Left/Right` + `Home/End` keyboard stepping, optional `detents`, and `onValueCommit` on pointer release / keyboard step / reset.
   - `ScrollArea.tsx`: `Viewport` sets `tabIndex={0}`, `role="region"`, and `aria-label={ariaLabel}` for keyboard scroll accessibility.

4. **Dead-Code & Unused-Export Hygiene (`knip`):**
   - Resolved 3 internal unused re-exports (`DialogPortal`, `DialogClose`, `DialogOverlay`) in `Dialog.tsx` and registered CSS-imported `"tw-animate-css"` in `knip.jsonc`.
   - Both `pnpm --dir crates/vox-gui/ui knip` and `pnpm --dir crates/vox-gui/ui run knip:files` exit `0` with zero Phase 431 unused files, dependencies, or exports.

## Resolved Findings During Self-Review

| ID | File | Severity | Issue | Resolution |
|---|---|---|---|---|
| R-01 | `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` | Medium | Sampling popover trigger/header used `text-[10px]`, violating `chatTypeScale.test.ts` `11px` minimum in chat/composer scope | Upgraded both classes to `text-[11px]`; `chatTypeScale.test.ts` passes (7/7) |
| R-02 | `crates/vox-gui/ui/src/index.css` | Medium | Generated `tokens.slate.generated.css` / `tokens.contrast.generated.css` emitted `--z-toast: 1500` on `[data-theme]`, exceeding `--z-tooltip: 100` | Pinned the 6-rung z-index ladder (`40..100`) on `:root, [data-theme]` in `index.css` `@layer base` after token imports |
| R-03 | `crates/vox-gui/ui/src/components/ui/Dialog.tsx` | Low | `DialogPortal`, `DialogClose`, and `DialogOverlay` were re-exported without external callers | Kept internal to `Dialog.tsx` so `knip` reports zero unused exports across `src/components/ui/` |

## Verdict

`status: clean` — ready for verification sign-off.
