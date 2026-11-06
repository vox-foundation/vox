---
title: "Using the Vox Axis Desktop Workbench & Loquela Controls"
description: "Navigate the 32 Vox Axis GUI surfaces, configure real-time Loquela chat posture knobs (Alt+1..4), pin @kb context, and inspect turn telemetry receipts."
category: "How-To Guides"
status: "current"
sort_order: 12
topic: "desktop-app"
domain: "gui"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
code_anchors:
  - "crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx"
  - "crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx"
  - "crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx"
---

# Using the Vox Axis Desktop Workbench & Loquela Controls

<!-- kb:summary -->
The Vox Axis Desktop Workbench (`vox-gui`) is a Tauri 2 operator interface uniting multi-agent chat (`Loquela`), a VS Code-style multi-panel docking shell (`DockWorkspaceShell`), an interactive Documentation & Findings Reader (`DocReader`), and live telemetry across 32 verified surfaces. This guide covers launching the workbench, switching cognitive clutch and risk posture in real time (`Alt+1..4`), pinning `@kb` documentation context into agent turns, and inspecting BPE token compaction receipts.
<!-- /kb:summary -->

## Architecture & Workbench Layout

```vox-diagram
graph {
    direction: right
    node omnibar: "Omnibar (Cmd/Ctrl+K)\nFederated Search & Slash Router"
    node dock: "DockWorkspaceShell\nMulti-Panel Split Docking"
    node loquela: "Loquela Chat & Posture Bar\n(Alt+1..4 Clutch & Risk Knobs)"
    node reader: "DocReader & Scientia\n(Pinned Architecture + RRF Search)"
    node rail: "ChatExecutionRail\n(Turn Receipts & CAS Task Ledger)"
    omnibar -> dock
    dock -> loquela
    dock -> reader
    dock -> rail
}
```

## 1. Launching the Workbench

Run the desktop shell from the workspace root or open the web preview during development:

```bash
cargo run -p vox-gui
```

Use **`Cmd+K`** (macOS) or **`Ctrl+K`** (Windows/Linux) at any time to open the federated Omnibar and jump across all 32 registered surfaces (`Chat`, `Loquela`, `DocReader`, `Scientia`, `Mesh`, `Runs`, `Sub-Agents`, `Models`, `Policies`, `Approvals`).

## 2. Real-Time Loquela Posture Controls & Keybinds

The **Loquela** control bar (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`) lets you steer how the agent harness routes models, compacts context, and verifies code on every turn:

| Shortcut | Control | Effect on Agent Turn |
|---|---|---|
| `Alt+1` .. `Alt+4` | **Cognitive Clutch** | Switches between Fast Local (`MENS`), Balanced Hybrid, Deep Architecture, and Speculative Repair Cascade. |
| `Alt+R` | **Risk Posture** | Cycles `Conservative`, `Balanced`, and `Autonomous` tool approval boundaries. |
| `Alt+C` | **Compaction Strategy** | Switches `cl100k_base` BPE token budgeting (`2,400` / `4,000` / `8,000` tokens with 20% goal-anchor preservation). |
| `Alt+/` | **Keybind HUD Overlay** | Toggles the live shortcut reference overlay (`loquela-keybind-hud-panel`). |

## 3. Exposing Project Tools to the Workbench

Any `tool` or `@mcp.resource` declared in your `.vox` files is automatically discoverable in the workbench capability registry:

```vox
table DeploymentTarget {
    env_name: str
    healthy:  bool
}

query healthy_targets() to int {
    return len(db.DeploymentTarget.all())
}

tool "Check deployment target health from the Axis Workbench" check_target(env_name: str) to Result[str] {
    return Ok("Target verified: " + env_name + " (" + str(healthy_targets()) + " active)")
}
```

## 4. Using the Built-In Documentation & Findings Reader (`DocReader`)

Open the **Documentation Reader** (`DocReader.tsx`) from the sidebar or Omnibar to:
- Jump directly to **Pinned Core Architecture Guides** (`Where Things Live`, `5-Minute Quickstart`, `Subsystem Architecture Atlas L0–L5`, `AgentOS & Orchestrator SSOT`).
- Filter the knowledge base by **Topic** and **Domain** pill selectors using hybrid Tantivy BM25 + HNSW Reciprocal Rank Fusion (RRF).
- Review candidate research findings in the **Findings Inbox** and promote verified `[path:Lstart-Lend]` claims.
