---
title: "Using the Vox Axis Desktop Workbench & Loquela Controls"
description: "Navigate the 32 Vox Axis GUI surfaces, explore the VRBS ROMA 3D/2.5D codebase simulator and AQVAEDUCTUS cascade, switch between the 4 WCAG theme profiles, and steer real-time Loquela chat posture knobs (Alt+1..4)."
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
The Vox Axis Desktop Workbench (`vox-gui`, published at `vox-foundation/vox-axis`) is a native Tauri 2 operator console and spatial codebase simulator uniting multi-agent chat (`Loquela`), the `VRBS ROMA` 3D/2.5D isometric workspace simulator, the `AQVAEDUCTUS` 3-tier hydraulic verification cascade, an interactive Documentation & Findings Reader (`DocReader`), and live telemetry across 32 catalog-generated surfaces under an enforced 4-profile WCAG theme contract.
<!-- /kb:summary -->

<div class="vox-media-frame">
  <img src="/assets/axis/axis_hero_tour.webp" alt="Animated WebP tour of Vox Axis: VRBS ROMA Codebase Simulator, Imperium Blast-Radius Inspector, Mission Control, Multi-Dock Chat, AQVAEDUCTUS Cascade, MENS Studio, and Dual Theme Contract" width="1100" height="688" />
  <div class="vox-media-caption">
    <strong>Vox Axis Flagship Workbench Tour (<a href="https://github.com/vox-foundation/vox-axis"><code>vox-foundation/vox-axis</code></a>)</strong> — <em>Cycling through the <code>VRBS ROMA</code> Codebase Simulator, <code>Imperium Prefecture</code> Blast-Radius Inspector, Mission Control, Multi-Dock Chat, <code>AQVAEDUCTUS</code> 3-Tier Hydraulic Verification Cascade, MENS Corpus Studio, and the Enforced Theme Contract (<code>Imperial Carrara White &amp; Gold</code> vs. <code>Minimalist Grey-Blue Slate</code>).</em>
  </div>
</div>

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

<div class="vox-media-frame">
  <img src="/assets/axis/axis_dual_mode_architecture.webp" alt="Vox Axis Dual-Mode Transport Architecture: Local Tauri 2 IPC vs Remote TOFU SPKI-Pinned TLS Gateway" width="1100" height="580" loading="lazy" />
  <div class="vox-media-caption">
    <strong>Dual-Mode Transport (<code>transport.ts</code>)</strong> — <em>Runs 100% offline in Local Mode over in-process Tauri 2 IPC against local <code>VoxDb</code> (v97) and <code>Clavis</code>, or connects in Remote Mode over TOFU SPKI SHA-256 pinned TLS to <code>vox-orchestrator-d</code>.</em>
  </div>
</div>

## 1. Launching the Workbench

Run the desktop shell from the workspace root or launch via the `vox` CLI:

```bash
cargo run -p vox-cli --features gui -- gui
```

Use **`Cmd+K`** (macOS) or **`Ctrl+K`** (Windows/Linux) at any time to open the Universal Capability Palette and jump across all **32 registered surfaces** organized into 5 domains:
1. **Operate (11 views):** `dashboard`, `chat`, `tasks`, `sub-agents`, `flow`, `mesh`, `runs`, `needs-you`, `approvals`, `policies`, `mercatus`
2. **Develop (8 views):** `console`, `catalog`, `skills`, `harness` (`AQVAEDUCTUS`), `harness-health`, `repository`, `browser`, `coderabbit`
3. **Knowledge & Socrates Gate (6 views):** `vox-search`, `memory`, `activity`, `research`, `scientia`, `publications`
4. **Compute / MENS (4 views):** `mens`, `models`, `populi`, `oratio`
5. **System & `VRBS ROMA` (3 views):** `settings`, `explorer`, `gamify` (`VRBS ROMA` Simulator)

For a complete screen-by-screen visual reference of all 32 views, see the [**`vox-foundation/vox-axis` README**](https://github.com/vox-foundation/vox-axis).

## 2. Exploring `VRBS ROMA` & the `AQVAEDUCTUS` Verification Cascade

<div class="vox-showcase-grid">
  <div class="vox-showcase-card">
    <img src="/assets/axis/urbs-civilization.webp" alt="VRBS ROMA 3D/2.5D Codebase Simulator" width="720" height="450" loading="lazy" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">gamify · VRBS ROMA</span>
      <h3 class="vox-showcase-title">Isometric Codebase Metropolis &amp; Forma Urbis Radar</h3>
      <p class="vox-showcase-desc">
        Computes Kahn topological layers (<code>L0</code>–<code>L5</code>) and Tarjan SCCs across Cargo, TypeScript, Python, and Go workspaces, inflating high-churn districts while <code>LEGIO</code> magistrate agents patrol active crate roads.
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/urbs-imperium-inspector.webp" alt="Imperium Prefecture Blast-Radius DAG Inspector" width="720" height="450" loading="lazy" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">ImperiumInspector.tsx</span>
      <h3 class="vox-showcase-title">Imperium Blast-Radius DAG Inspector</h3>
      <p class="vox-showcase-desc">
        Click any monument to inspect upstream providers, downstream blast radius, per-file <code>+LoC / -LoC</code> diff slabs, and 1-click <code>[CHECK]</code>, <code>[✓ TEST]</code>, and <code>[TEST IMPACTED]</code> actions.
      </p>
    </div>
  </div>
</div>

## 3. Real-Time Loquela Posture Controls & Keybinds

The **Loquela** control bar (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`) lets you steer how the agent harness routes models, compacts context, and verifies code on every turn:

| Shortcut | Control | Effect on Agent Turn |
|---|---|---|
| `Alt+1` .. `Alt+4` | **Cognitive Clutch** | Switches between Fast Local (`MENS`), Balanced Hybrid, Deep Architecture, and Speculative Repair Cascade. |
| `Alt+R` | **Risk Posture** | Cycles `Conservative`, `Balanced`, and `Autonomous` tool approval boundaries. |
| `Alt+C` | **Compaction Strategy** | Switches `cl100k_base` BPE token budgeting (`2,400` / `4,000` / `8,000` tokens with 20% goal-anchor preservation). |
| `Alt+/` | **Keybind HUD Overlay** | Toggles the live shortcut reference overlay (`loquela-keybind-hud-panel`). |

## 4. Switching Between the 4 Enforced WCAG Theme Profiles

Open **`Settings → Theme Contract`** (`settings`) to switch live between the 4 Style-Dictionary-generated theme profiles (enforced by `vox-code-audit` detector `vox/ui/theme-contract-drift` and the unified 4-token font contract `--font-display`, `--font-serif`, `--font-sans`, `--font-mono`):

<div class="vox-showcase-grid">
  <div class="vox-showcase-card">
    <img src="/assets/axis/theme-arcane.webp" alt="Imperial Carrara White and Gold Default Theme" width="720" height="450" loading="lazy" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">Light · arcane &amp; glacier</span>
      <h3 class="vox-showcase-title">Imperial Carrara White &amp; Gold (Default)</h3>
      <p class="vox-showcase-desc">
        Sunlit honed Carrara marble (<code>#FAF6EE</code>), burnished gold mouldings (<code>#C59B27</code>), deep espresso ink (<code>#1C140C</code>), and emerald laurel signets (<code>#1B6B3A</code>).
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/theme-slate.webp" alt="Minimalist Grey-Blue Slate Dark Theme" width="720" height="450" loading="lazy" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge">Dark · slate &amp; high-contrast</span>
      <h3 class="vox-showcase-title">Minimalist Grey-Blue Slate &amp; WCAG AAA</h3>
      <p class="vox-showcase-desc">
        Basalt/slate dark palette (<code>#0c0e10</code> / <code>#15191c</code> / <code>#1f262b</code>), crisp <code>Inter</code> + tabular monospace geometry, and zero marble textures.
      </p>
    </div>
  </div>
</div>

## 5. Exposing Project Tools to the Workbench

Any `tool` or `resource` declared in your `.vox` files is automatically discoverable in the workbench capability registry:

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

## 6. Using the Built-In Documentation & Findings Reader (`DocReader`)

Open the **Documentation Reader** (`DocReader.tsx`) from the sidebar or Omnibar to:
- Jump directly to **Pinned Core Architecture Guides** (`Where Things Live`, `5-Minute Quickstart`, `Subsystem Architecture Atlas L0–L5`, `AgentOS & Orchestrator SSOT`).
- Filter the knowledge base by **Topic** and **Domain** pill selectors using hybrid Tantivy BM25 + HNSW Reciprocal Rank Fusion (RRF).
- Review candidate research findings in the **Findings Inbox** and promote verified `[path:Lstart-Lend]` claims through the 5-stage Socrates gate.
