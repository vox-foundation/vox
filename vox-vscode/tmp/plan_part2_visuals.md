# Visual & Layout Engineering

## Current Flaws
1. **Misalignments**: Icons vs text rendering, inconsistent padding/margins inside standard VsCode containers.
2. **Fake CSS Elements**: Unconnected borders, non-standard CSS coloring like `rgba(59,130,246,0.1)` instead of standard `var(--vscode-...)` semantic colors everywhere.
3. **Information Density**: Empty panes taking up 80% of space, small text lost in massive bubbles. 

## Responsive Central Overlay Strategy
To provide "all information in one place" without cluster, we build a layout based on:
1. **HUD Overlay Header**: At the absolute top, a tightly integrated "Status Bar" containing Agent count, Telemetry Budget, and Mesh Status. No more dedicated giant charts unless explicitly requested.
2. **Dynamic Operations Grid**: An accordion or clustered tile system that replaces `Dashboard.tsx`. It intelligently hides empty info (like Ludus when there's no KPI diff) and expands active info.
3. **Action Trays**: No more arbitrary "Fmt Build" / "Rebalance" buttons floating inside dashboard real estate. Move all executable operations (Clear Cache, Restarts, Agent Toggles) into an Action Tray context menu.

## Strict Wiring Checks
- `Dashboard`: Map all `stats` directly to the `VoxMcpClient.ts` payload `voxStatus`.
- `Financial`: Remove default placeholder lines. Render raw values directly, or suppress the widget.
- `CompanionHUD`: Stop floating at the bottom overlapping scrollbars. Anchor it properly.
