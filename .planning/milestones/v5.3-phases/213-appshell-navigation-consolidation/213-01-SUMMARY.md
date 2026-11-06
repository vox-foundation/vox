# Summary 213-01: AppShell 5-Domain Sidebar Consolidation & Child Sub-Tab Deduplication

## Completed Changes
- **`crates/vox-gui/ui/src/components/layout/Sidebar.tsx`**: Added 5-domain consolidation headers (`sidebar-domain-group-operate-chat`, `sidebar-domain-group-agents-runs`, `sidebar-domain-group-workspace`, `sidebar-domain-group-knowledge`, `sidebar-domain-group-compute-system`) and collapsed default non-Chat vertical child list (`sr-only` unless explicitly peeked via chevron).
- **`crates/vox-gui/ui/src/components/layout/BreadcrumbBar.tsx`**: Added `data-testid="breadcrumb-child-tabs"` and `data-cv-subsurface="breadcrumb-tab-${subKey}"` for dynamic CV sub-tab discovery.
- **`crates/vox-gui/ui/src/components/layout/Sidebar.test.tsx`**: Added regression test verifying 5-domain headers and `sr-only` child list deduplication.

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/layout/Sidebar.test.tsx src/components/layout/Sidebar.landmarks.test.tsx src/components/layout/BreadcrumbBar.test.tsx src/components/layout/BottomStatusBar.test.tsx src/lib/navigation.test.ts` → `5 passed (5)` (`96/96` tests passing).
