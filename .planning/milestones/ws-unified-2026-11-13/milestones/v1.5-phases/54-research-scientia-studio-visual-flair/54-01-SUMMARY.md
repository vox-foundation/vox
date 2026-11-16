# Phase 54 Plan 01 Summary: Research Power Controls & Domain Cards

**Status:** Complete  
**Date:** 2026-10-14  
**Requirements:** `SCIGUI-01`

## Delivered
1. **Typed Research API Surface ([`research.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/api/research.ts) & [`researchActions.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts))**:
   - Extended `StartResearchArgs` and `startResearchAsync` with `scope`, `waves`, `domainMode`, `siteScope`, and `verifyCompilation`.
   - Added `EpistemicDagNodeDto`, `EpistemicDagEdgeDto`, and `EpistemicDagDto` types.
2. **Domain Mode & Retrieval Power Control Bar ([`ResearchView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx))**:
   - Added `data-testid="research-power-control-bar"` with `General | Shopping | CodeGen` mode buttons (`data-testid="research-domain-mode-group"`), `Web | Local | Both` scope buttons (`data-testid="research-scope-group"`), `1W | 2W | 3W` wave selectors (`data-testid="research-waves-group"`), and `site:` domain scope input (`data-testid="research-site-scope-input"`).
3. **Domain Decision & Compiler Verification Cards**:
   - Created [`ShoppingDecisionCard.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Research/ShoppingDecisionCard.tsx) (`data-testid="shopping-decision-card"`) for weighted multi-criteria option comparison.
   - Created [`CodeGenVerificationCard.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Research/CodeGenVerificationCard.tsx) (`data-testid="codegen-verification-card"`) for `.vox` / Rust compiler verification diagnostics.

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Research/ResearchView.test.tsx` (`28 passed`).
