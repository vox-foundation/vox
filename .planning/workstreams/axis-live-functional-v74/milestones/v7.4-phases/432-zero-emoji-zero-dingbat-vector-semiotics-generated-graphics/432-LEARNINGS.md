---
phase: 432
phase_name: "zero-emoji-zero-dingbat-vector-semiotics-generated-graphics"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 432 Learnings: zero-emoji-zero-dingbat-vector-semiotics-generated-graphics

## Decisions

### Shipped VoxIcon.tsx (lucide-react@^1.72.0 strokeWidth={1.5}, shrink-0), ...
Shipped `VoxIcon.tsx` (`lucide-react@^1.72.0` `strokeWidth={1.5}`, `shrink-0`), `RomanIcons.tsx` (`amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`, `homeVilla`, `penQuill`, etc.), and `IconGlyphs.tsx` (`PlanStepGlyph`, `RoleRankEmblem`, `KeycapIcon`), keeping `Icons.tsx` below the 500-non-blank-line `toestub` God Object budget.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 432-01-SUMMARY.md

---

### Replaced 100% of raw pictographic emojis and BMP Unicode dingbats/shapes...
Replaced 100% of raw pictographic emojis and BMP Unicode dingbats/shapes/arrows across 60+ JSX/TSX components and string formatters.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 432-01-SUMMARY.md

---

### Created canvasVectorBadges.ts (drawDiamondBadge, drawTriangleUpBadge, dr...
Created `canvasVectorBadges.ts` (`drawDiamondBadge`, `drawTriangleUpBadge`, `drawArrowUpBadge`, `drawArrowRightBadge`, `drawChevronRightBadge`, `drawCheckBadge`, `drawCrossBadge`, `drawTargetRingBadge`) and eliminated all glyph `ctx.fillText` hacks in `worldRenderer.ts` and `hydraulicNetwork.ts`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 432-01-SUMMARY.md

---

### Generated and compressed 4 bespoke dual-theme (arcane/glacier Carrara + ...
Generated and compressed 4 bespoke dual-theme (`arcane`/`glacier` Carrara + `slate`/`high-contrast` Basalt) relief plates (`v74-crest-empty-{carrara,slate}.jpg`, `v74-medallion-imperium-{carrara,slate}.jpg`, `61–139 KB` each) and wired `<RomanCrestPlate />` into `EmptyState.tsx`, `ImperiumInspector.tsx`, and `LudusSandbox.tsx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 432-01-SUMMARY.md

---

### Resolved all 7 themeContract.test.ts findings in LudusSandbox.tsx and Ga...
Resolved all 7 `themeContract.test.ts` findings in `LudusSandbox.tsx` and `GamifyView.tsx` (`themeContract.test.ts` `7/7` passing).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 432-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

