---
phase: 431
phase_name: "dual-theme-css-escape-hatches-portal-z-index-scale-shadcn-ra"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 0
  lessons: 5
  patterns: 0
  surprises: 3
missing_artifacts:
  - "UAT.md"
---

# Phase 431 Learnings: dual-theme-css-escape-hatches-portal-z-index-scale-shadcn-ra

## Decisions

## Lessons

### [Rule 1 - Bug] Style Dictionary --z-toast
**[Rule 1 - Bug] Style Dictionary `--z-toast: 1500` token override:** `tokens.slate.generated.css` and `tokens.contrast.generated.css` emit `--z-toast: 1500` on `[data-theme="slate"]` and `[data-theme="high-contrast"]`, which would place toasts above `--z-tooltip: 100`. Resolved by pinning the 6-rung z-index ladder (`40..100`) on `:root, [data-theme]` in `index.css` `@layer base` after the generated token imports.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 431-01-SUMMARY.md

---

### [Rule 1 - Bug] Chat type-scale floor in Loquela.tsx
**[Rule 1 - Bug] Chat type-scale floor in `Loquela.tsx`:** `chatTypeScale.test.ts` enforces an `11px` floor across chat/composer surfaces; adjusted two `text-[10px]` labels in the new `Loquela.tsx` sampling popover trigger/header to `text-[11px]`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 431-01-SUMMARY.md

---

### [Rule 3 - Blocking] knip CSS-only devDependency recognition
**[Rule 3 - Blocking] `knip` CSS-only devDependency recognition:** Added `"tw-animate-css"` alongside `"postcss"` in `crates/vox-gui/ui/knip.jsonc` `ignoreDependencies` because `tw-animate-css` is consumed via CSS `@import` in `src/index.css`, and pruned 3 unreferenced internal re-exports (`DialogPortal`, `DialogClose`, `DialogOverlay`) from `Dialog.tsx` so zero Phase 431 symbols appear in `knip` unused-export reports.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 431-01-SUMMARY.md

---

### timeout 300s pnpm --dir crates/vox-gui/ui build
timeout 300s pnpm --dir crates/vox-gui/ui build

**Context:** Verified in phase acceptance checks.
**Source:** 431-VERIFICATION.md

---

### $ pnpm tokens:build && run-p typecheck bundle $ node style-dictionary.co...
$ pnpm tokens:build && run-p typecheck bundle $ node style-dictionary.config.mjs tokens built: tokens.generated.css, tokens.slate.generated.css, tokens.contrast.generated.css, tokens.travertine.generated.css, tokens.generated.ts $ tsc --noEmit $ vite build vite v8.2.2 building client environment for production... ✓ 3311 modules transformed. dist/index.html                                        0.48 kB │ gzip:   0.30 kB dist/assets/index-DYjdOvXP.css                       410.43 kB │ gzip:  48.59 kB dist/assets/capabilityIndex.generated-C1w0fBhI.js    915.21 kB │ gzip:  76.08 kB dist/assets/index-Deo-SQc4.js                      3,336.58 kB │ gzip: 888.74 kB ✓ built in 828ms

**Context:** Verified in phase acceptance checks.
**Source:** 431-VERIFICATION.md

## Patterns

## Surprises

### [Rule 1 - Bug] Style Dictionary --z-toast
**[Rule 1 - Bug] Style Dictionary `--z-toast: 1500` token override:** `tokens.slate.generated.css` and `tokens.contrast.generated.css` emit `--z-toast: 1500` on `[data-theme="slate"]` and `[data-theme="high-contrast"]`, which would place toasts above `--z-tooltip: 100`. Resolved by pinning the 6-rung z-index ladder (`40..100`) on `:root, [data-theme]` in `index.css` `@layer base` after the generated token imports.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 431-01-SUMMARY.md

---

### [Rule 1 - Bug] Chat type-scale floor in Loquela.tsx
**[Rule 1 - Bug] Chat type-scale floor in `Loquela.tsx`:** `chatTypeScale.test.ts` enforces an `11px` floor across chat/composer surfaces; adjusted two `text-[10px]` labels in the new `Loquela.tsx` sampling popover trigger/header to `text-[11px]`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 431-01-SUMMARY.md

---

### [Rule 3 - Blocking] knip CSS-only devDependency recognition
**[Rule 3 - Blocking] `knip` CSS-only devDependency recognition:** Added `"tw-animate-css"` alongside `"postcss"` in `crates/vox-gui/ui/knip.jsonc` `ignoreDependencies` because `tw-animate-css` is consumed via CSS `@import` in `src/index.css`, and pruned 3 unreferenced internal re-exports (`DialogPortal`, `DialogClose`, `DialogOverlay`) from `Dialog.tsx` so zero Phase 431 symbols appear in `knip` unused-export reports.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 431-01-SUMMARY.md

