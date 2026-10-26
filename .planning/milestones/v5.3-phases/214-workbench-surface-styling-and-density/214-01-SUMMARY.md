# Summary 214-01: Workbench Surface Above-the-Fold Layout & Density Standardization

## Completed Changes
- **`crates/vox-gui/ui/src/components/surfaces/Dashboard/Dashboard.tsx`**: Inlined `Customize dashboard` into the `Submit tasks in Chat` banner row, added `data-testid="dashboard-kpi-strip"`, and compacted vertical spacing and `Workspace Simulation Mini-Map` height (`h-[116px]`).
- **`crates/vox-gui/ui/src/components/surfaces/Dashboard/ResourcesWidget.tsx`**: Compacted `Resources` card padding (`p-3.5`, `gap-2.5`) so `Active Agents` and `System · Telemetry & Alerts` appear above the fold at `1440×900`.

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Dashboard/Dashboard.test.tsx` → `1 passed (1)` (`16/16` tests passing).
