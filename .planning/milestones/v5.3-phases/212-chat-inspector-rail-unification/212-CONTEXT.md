# Phase 212 Context: Chat Session Inspector Rail Deduplication, Unified Plan/Task Checklist & Header Layout Switcher

**Phase**: 212
**Milestone**: v4.0 — Axis GUI Progressive Disclosure, Surface Consolidation & Visual Polish

## Decisions & Scope
- **Segmented Inspector Rail (`RAIL-UNIFY-01`)**: `ChatExecutionRail.tsx` exposes a 4-mode segmented tab header (`All` | `Plan & Tasks` | `Context` | `Routing`, `data-testid="execution-rail-segmented-tabs"`) with `data-cv-subsurface` attributes for dynamic CV scanner discovery and `sr-only` progressive disclosure for inactive sections.
- **Unified Checklist & Progress Ring (`RAIL-UNIFY-02`)**: `ChatExecutionRail.tsx` renders the circular progress ring (`data-testid="execution-rail-progress-ring"`), `Synced · v{rev}` CAS badge, and inline step mutations (`P1..P3`, `?`, `✓`, `✎`, `×`, `+ Add task`).
- **Context Meter Unclipped & Bottom Pin Deduplication (`RAIL-UNIFY-03`)**: `ContextWindowMeter.tsx` compacts strategy pills and `Clear Tools` button (`text-[9px]`, `context-meter-strategy-btn` `sr-only`) so zero right-edge clipping occurs in the 320px rail. In `ChatSurface.tsx`, the redundant bottom `Pinned Context` / `Notifications` bar is collapsed (`sr-only`) by default unless opened or urgent items exist.
- **Header Layout Presets (`RAIL-UNIFY-04`)**: `ChatSurface.tsx` renders quick workspace layout preset pills (`Focus` | `Split Code` | `Multi-Agent`, `data-testid="chat-header-layout-presets"`) beside `Panels ▾`.
