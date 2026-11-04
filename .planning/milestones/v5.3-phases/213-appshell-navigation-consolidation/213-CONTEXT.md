# Phase 213 Context: AppShell 5-Domain Sidebar Consolidation, Top Workbench Tab Bar & Status Bar Streamlining

**Phase**: 213
**Milestone**: v4.0 — Axis GUI Progressive Disclosure, Surface Consolidation & Visual Polish

## Decisions & Scope
- **5-Domain Sidebar Grouping (`NAV-CONSOLIDATE-01`)**: `Sidebar.tsx` groups top-level navigation under the 5 v4.0 domain headers (`Operate · Chat`, `Agents & Runs`, `Workspace`, `Knowledge`, `Compute & System`, `data-testid="sidebar-domain-group-*"`).
- **Single Horizontal Child Sub-Tab Strip (`NAV-CONSOLIDATE-02`)**: `BreadcrumbBar.tsx` renders `data-testid="breadcrumb-child-tabs"` with `data-cv-subsurface` attributes as the single visible child tab strip; `Sidebar.tsx` collapses the default non-Chat vertical child list (`sr-only` unless explicitly peeked via chevron).
- **Streamlined Bottom Status Bar (`NAV-CONSOLIDATE-03`)**: `BottomStatusBar.tsx` preserves all HUD tiles (`Engine`, `Spend`, `Mesh`, `Routing`, `Needs You`), `Configure ▾`, and `#workbench-tabbar-trailing-slot` portal target.
- **Navigation SSOT Parity (`NAV-CONSOLIDATE-04`)**: All `TOP_LEVEL_VIEWS` and `CHILD_ORDER_BY_PARENT` routes remain 100% reachable via `Sidebar`, `BreadcrumbBar`, and `CommandPalette`.
