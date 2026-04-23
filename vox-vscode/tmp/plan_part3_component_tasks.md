# Detailed Component Tasks

To create a flawless, responsive layout without fake data, we must rip out the hardcoded 12-tab architecture and meticulously reconstruct the UI components. 

## 1. App.vox / index.tsx Overhaul (Tasks 1-45)
1. Delete unused tab icons from lucide-react imports.
2. Defensively wrap host `message` listeners in try/catch to prevent partial UI death.
3. Remove hardcoded TabId enum of 12 elements.
4. Replace TabId with 'UnifiedDashboard' | 'ChatMode' | 'Diagnostics'.
5. Convert `activeTab` state logic to handle only 3 modes.
6. Remove the hardcoded `taskFallback` array generator.
7. Stop injecting `--` if `agent_id` is missing; conditionally render the span entirely.
8. Validate `Array.isArray(oplog)` before slicing it.
9. Implement a top-level `ErrorBoundary` for the single unified dashboard.
10. Remove `vox-exec-hint` status string builder and migrate it to a structured Header component.
11. Migrate `agentCount` logic out of App.tsx into the UnifiedDashboard state.
12. Create a standardized `vscode.postMessage` hook.
... (Tasks 13-45 will involve strict `null` checks on every single `useState` initializer and `useEffect` dependency array involving the host messages).

## 2. The Unified Dashboard (Overlay Design) (Tasks 46-120)
46. Create `UnifiedDashboard.tsx`.
47. Implement a CSS Grid layout that utilizes `1fr` dynamic columns based on VS Code window width.
48. Build a collapsible `LudusKpiWidget` that hides itself if `total_xp == null`.
49. Delete `LudusPanel.tsx`.
50. Build a `FinancialWidget` anchored to the top right of the dashboard.
51. Delete `FinancialDashboard.tsx`.
52. Replace the massive 48px icons with standard VS Code 16px codicons.
53. Remove hardcoded CSS color `rgba(59,130,246,0.1)`.
54. Replace with semantic `var(--vscode-charts-blue)`.
55. Remove `gap-8` hardcodes, replace with scalable `rem` spacing aligned to VS Code standard layout gaps (e.g., 8px, 16px).
... (Tasks 56-120 cover migrating `AgentFlow`, `MeshTopology` and removing 'fake' placeholder rendering when telemetry lists are empty).
