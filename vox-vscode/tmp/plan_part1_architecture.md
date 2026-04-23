# Webview Architecture Review & Consolidation Strategy

## Current State Analysis
The VS Code Webview currently has 12 side-nav tabs:
1. `Dashboard` (LayoutDashboard): Shows operations, active agents, pipeline health.
2. `Flow` (Network): Agent task visualization.
3. `Chat` (MessageSquare): Chat interface.
4. `Composer` (Sparkles): Code composer.
5. `Context` (ScanSearch): Workspace context.
6. `Scrubber` (RotateCcw): Workflow snapshots.
7. `Intentions` (BrainCircuit): Intention matrix.
8. `Mesh` (Globe2): Mesh topology.
9. `Pipeline` (Blocks): Pipeline/AST status.
10. `Ast` (Code2): AST Inspector.
11. `Telemetry` (ActivityIcon): Financial metrics / model usage.
12. `Ludus` (Trophy): Gamification.

## Consolidation Strategy (Fewer Buttons, clustered info)
The user explicitly wants "fewer buttons, but more interesting information" and to "cluster useful information into an overlay or the dashboard".

**Proposed Consolidated Layout:**
1. **Chat & Composer**: The main conversational and coding interface (combine Chat/Composer/Context into one primary view with sub-panels/overlays).
2. **Command Center (Unified Dashboard)**: Consume Dashboard, Telemetry (Financials), Ludus (Gamification), Mesh Topology, and Agent Flow into a single high-information-density central dashboard. 
3. **Engineering / Trace**: Combine Pipeline, AST, Workflow Scrubber, and Intentions into an "Engineering / System Trace" view.

This reduces 12 tabs to 3 highly dense, perfectly wired tabs.

## "Fake Data" & Wiring Analysis
The user noted things look "fake" or "all zero".
- `Dashboard.tsx`: Hardcodes "10ms" fallback or string manipulations for opRow. If `pipeline` is null, emits fake "No vox_pipeline_status yet". Falls back `activeAgents` to count of working agents if orchestrator status lacks `agent_count`.
- `FinancialDashboard.tsx`: Displays `--` or zero if budgetHistory is empty.
- `LudusPanel.tsx`: Hardcoded default zeroish text `—` for KPI fields.
- `IntentionMatrix.tsx`, `MeshTopology.tsx`: Unclear if their data structure rigidly binds to real MCP events.

Everything must be rewritten to strictly process nullable boundaries and say "0" ONLY if explicitly 0.
