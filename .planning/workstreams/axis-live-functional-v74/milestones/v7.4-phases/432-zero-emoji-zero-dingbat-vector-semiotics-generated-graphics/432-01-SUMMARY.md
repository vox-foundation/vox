---
phase: 432-zero-emoji-zero-dingbat-vector-semiotics-generated-graphics
plan: 01
subsystem: gui
tags:
  - lucide-react
  - vector-semiotics
  - path2d
  - generated-graphics
  - theme-contract
dependency_graph:
  requires:
    - 431-01
  provides:
    - VoxIcon
    - RomanIcons
    - IconGlyphs
    - RomanCrestPlate
    - canvasVectorBadges
    - zeroEmojiAndDingbatGuard
  affects:
    - crates/vox-gui/ui/src
    - crates/vox-gui/ui/e2e/review/audits.ts
completed_date: "2026-11-14"
---

# Phase 432 Plan 01 Summary: Zero-Emoji & Zero-Dingbat Vector Semiotics & Generated Graphics Sweep

## Accomplishments

1. **Two-Tier Vector Icon Architecture (`AXIS-ICONS-01`):**
   - Shipped `VoxIcon.tsx` (`lucide-react@^1.72.0` `strokeWidth={1.5}`, `shrink-0`), `RomanIcons.tsx` (`amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`, `homeVilla`, `penQuill`, etc.), and `IconGlyphs.tsx` (`PlanStepGlyph`, `RoleRankEmblem`, `KeycapIcon`), keeping `Icons.tsx` below the 500-non-blank-line `toestub` God Object budget.
   - Replaced 100% of raw pictographic emojis and BMP Unicode dingbats/shapes/arrows across 60+ JSX/TSX components and string formatters.

2. **Canvas 2D `Path2D` Badges & Sovereign Dual-Theme Crests (`AXIS-ICONS-02`):**
   - Created `canvasVectorBadges.ts` (`drawDiamondBadge`, `drawTriangleUpBadge`, `drawArrowUpBadge`, `drawArrowRightBadge`, `drawChevronRightBadge`, `drawCheckBadge`, `drawCrossBadge`, `drawTargetRingBadge`) and eliminated all glyph `ctx.fillText` hacks in `worldRenderer.ts` and `hydraulicNetwork.ts`.
   - Generated and compressed 4 bespoke dual-theme (`arcane`/`glacier` Carrara + `slate`/`high-contrast` Basalt) relief plates (`v74-crest-empty-{carrara,slate}.jpg`, `v74-medallion-imperium-{carrara,slate}.jpg`, `61–139 KB` each) and wired `<RomanCrestPlate />` into `EmptyState.tsx`, `ImperiumInspector.tsx`, and `LudusSandbox.tsx`.
   - Resolved all 7 `themeContract.test.ts` findings in `LudusSandbox.tsx` and `GamifyView.tsx` (`themeContract.test.ts` `7/7` passing).

3. **Accessible Icon Naming, Vitest Migrations & Un-Exempted Glyph Gates (`AXIS-ICONS-03`, `AXIS-ICONS-04`):**
   - Migrated `Kpi.test.tsx`, `CapabilityBadge.test.tsx`, `ExplorerView.test.tsx`, `SecretaryToast.test.tsx`, `TurnTrace.test.tsx`, `turnTrace.test.ts`, `PlanPanel.test.tsx`, `NoveltyEvidencePanel.test.tsx`, `Omnibar.test.tsx`, `ImperiumInspector.test.tsx`, and `viaductInspector.test.tsx` to semantic `data-icon` / `aria-label` / role assertions.
   - Added `crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts` and removed the `urbs-imperium-inspector` exemption in `crates/vox-gui/ui/e2e/review/audits.ts` while expanding visible DOM scanning across open Radix portals.
