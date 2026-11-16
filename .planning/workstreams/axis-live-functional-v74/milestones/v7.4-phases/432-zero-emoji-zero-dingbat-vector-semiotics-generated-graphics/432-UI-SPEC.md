# Phase 432: UI Design Contract (`432-UI-SPEC.md`)

**Phase:** 432 — Zero-Emoji & Zero-Dingbat Vector Semiotics & Generated Graphics Sweep
**Requirements:** `AXIS-ICONS-01`, `AXIS-ICONS-02`, `AXIS-ICONS-03`, `AXIS-ICONS-04`

## 1. Two-Tier Vector Semiotics Contract

| Tier | Source | Stroke / ViewBox | Sizing Scale | Accessibility Rule |
|------|--------|------------------|--------------|--------------------|
| **Tier 1: Technical IDE / Control Icons** | `VoxIcon.tsx` (`lucide-react@^1.72.0`) | `viewBox="0 0 24 24"`, `strokeWidth={1.5}` | `size-3` (`12px` inline pill/chevron), `size-3.5` (`14px` dense button), `size-4` (`16px` toolbar/nav) | `aria-hidden="true"` when text label exists; `aria-label` on parent button when icon-only |
| **Tier 2: Romanesque Domain Semiotics** | `Icons.tsx` (`amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`, `homeVilla`, `penQuill`, `PlanStepGlyph`, `RoleRankEmblem`, `KeycapIcon`) | `viewBox="0 0 24 24"`, `strokeWidth={1.5}` | `size-3.5` (`14px`) to `size-5` (`20px`) | `data-icon="<name>"`, `shrink-0`, `aria-hidden="true"` unless standalone `role="img"` with `aria-label` |

## 2. Canvas 2D `Path2D` Badge Specification (`canvasVectorBadges.ts`)

- Zero `ctx.fillText` calls containing pictographic emojis or BMP geometric/arrow/dingbat characters (`◆`, `▲`, `⬆`, `✓`, `✕`, `❯`, `◎`, `◉`, `→`).
- Deterministic `Path2D` helpers centered on `(cx, cy)` with radius `r`:
  - `drawDiamondBadge(ctx, cx, cy, r, style)`
  - `drawTriangleUpBadge(ctx, cx, cy, r, style)`
  - `drawArrowUpBadge(ctx, cx, cy, r, style)`
  - `drawArrowRightBadge(ctx, cx, cy, r, style)`
  - `drawChevronRightBadge(ctx, cx, cy, r, style)`
  - `drawCheckBadge(ctx, cx, cy, r, style)`
  - `drawCrossBadge(ctx, cx, cy, r, style)`
  - `drawTargetRingBadge(ctx, cx, cy, r, style)`

## 3. Dual-Theme Generated Crests & Empty-State Plates (`RomanCrestPlate.tsx`)

| Asset Path (`crates/vox-gui/ui/public/assets/romanesque/`) | Theme Binding | Max File Size | Consumers |
|------------------------------------------------------------|---------------|---------------|-----------|
| `v74-crest-empty-carrara.jpg` | `arcane`, `glacier` | `<= 160 KB` | `EmptyState.tsx`, `DiscoverySurface.tsx` |
| `v74-crest-empty-slate.jpg` | `slate`, `high-contrast` | `<= 160 KB` | `EmptyState.tsx`, `DiscoverySurface.tsx` |
| `v74-medallion-imperium-carrara.jpg` | `arcane`, `glacier` | `<= 160 KB` | `ImperiumInspector.tsx`, `LudusSandbox.tsx` |
| `v74-medallion-imperium-slate.jpg` | `slate`, `high-contrast` | `<= 160 KB` | `ImperiumInspector.tsx`, `LudusSandbox.tsx` |

## 4. Verification Invariants

1. `zeroEmojiAndDingbat.test.ts` passes with `0` violations across all non-test, non-generated `.ts`/`.tsx` files in `crates/vox-gui/ui/src/`.
2. `themeContract.test.ts` passes `7/7` (`0` hardcoded dark-brown/banned hexes in `LudusSandbox.tsx` and `GamifyView.tsx`).
3. `Kpi.test.tsx`, `CapabilityBadge.test.tsx`, `ExplorerView.test.tsx`, `SecretaryToast.test.tsx`, `TurnTrace.test.tsx`, and `turnTrace.test.ts` pass `100%`.
4. `audits.ts` deletes `el.closest('[data-testid="urbs-imperium-inspector"]')` exemption and scans full `document.body` + open Radix portals (`rawEmojiCount === 0`, `iconIssueCount === 0`).
