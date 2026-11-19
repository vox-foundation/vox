---
title: "Using the Vox Axis Desktop Workbench & Loquela Controls"
description: "Navigate the 33 Vox Axis GUI surfaces, explore the VRBS ROMA 3D/2.5D codebase simulator and AQVAEDUCTUS cascade, switch between the 4 WCAG theme profiles, and steer real-time Loquela chat posture knobs (Alt+1..4)."
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
The Vox Axis Desktop Workbench (`vox-gui`, published at `vox-foundation/vox-axis`) is a native Tauri 2 operator console and spatial codebase simulator uniting multi-agent chat (`Loquela`), the `VRBS ROMA` 3D/2.5D isometric workspace simulator, the `AQVAEDUCTUS` 3-tier hydraulic verification cascade, an interactive Documentation & Findings Reader (`DocReader`), and live telemetry across 33 catalog-generated surfaces under an enforced 4-profile WCAG theme contract.
<!-- /kb:summary -->

<div class="vox-media-frame">
  <img src="/assets/axis/axis_hero_tour.webp" alt="Animated WebP tour of Vox Axis: VRBS ROMA Codebase Simulator, Imperium Blast-Radius Inspector, Multi-Dock Chat, AQVAEDUCTUS 3-Tier Hydraulic Cascade, Deep Research & Epistemic DAG, Non-Modal Multi-Format Doc Studio, and Enforced Slate Dark Theme" width="960" height="600" loading="lazy" decoding="async" />
  <div class="vox-media-caption">
    <strong>Vox Axis Flagship Workbench Tour (<a href="https://github.com/vox-foundation/vox-axis"><code>vox-foundation/vox-axis</code></a>)</strong> — <em>Cycling through the <code>VRBS ROMA</code> Codebase Simulator, <code>Imperium Prefecture</code> Blast-Radius Inspector, Multi-Dock Chat, <code>AQVAEDUCTUS</code> 3-Tier Hydraulic Verification Cascade, Deep Research &amp; Epistemic DAG, Non-Modal Multi-Format Doc Studio, and the Enforced Theme Contract (<code>Imperial Carrara White &amp; Gold</code> vs. <code>Minimalist Grey-Blue Slate</code>).</em>
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
  <img src="/assets/axis/axis_dual_mode_architecture.webp" alt="Vox Axis Dual-Mode Transport Architecture: Local Tauri 2 IPC vs Remote TOFU SPKI-Pinned TLS Gateway" width="1376" height="768" loading="lazy" decoding="async" />
  <div class="vox-media-caption">
    <strong>Dual-Mode Transport (<code>transport.ts</code>)</strong> — <em>Runs 100% offline in Local Mode over in-process Tauri 2 IPC against local <code>VoxDb</code> (v99) and <code>Clavis</code>, or connects in Remote Mode over TOFU SPKI SHA-256 pinned TLS to <code>vox-orchestrator-d</code>.</em>
  </div>
</div>

## 1. Launching the Workbench

Run the desktop shell from the workspace root or launch via the `vox` CLI:

```bash
cargo run -p vox-cli --features gui -- gui
```

Use **`Cmd+K`** (macOS) or **`Ctrl+K`** (Windows/Linux) at any time to open the Universal Capability Palette and jump across all **33 registered surfaces** organized into 5 domains:
1. **Operate (12 views):** `dashboard`, `chat`, `tasks`, `sub-agents`, `flow`, `mesh`, `runs`, `needs-you`, `approvals`, `policies`, `mercatus`, `gsd`
2. **Develop (8 views):** `console`, `catalog`, `skills`, `harness` (`AQVAEDUCTUS`), `harness-health`, `repository`, `browser`, `coderabbit`
3. **Knowledge & Socrates Gate (6 views):** `vox-search`, `memory`, `activity`, `research`, `scientia`, `publications`
4. **Compute / MENS (4 views):** `mens`, `models`, `populi`, `oratio`
5. **System & `VRBS ROMA` (3 views):** `settings`, `explorer`, `gamify` (`VRBS ROMA` Simulator)

For a complete screen-by-screen visual reference of all 33 views, see the [**`vox-foundation/vox-axis` README**](https://github.com/vox-foundation/vox-axis).

## 2. Exploring `VRBS ROMA` & the `AQVAEDUCTUS` Verification Cascade

<div class="vox-showcase-grid">
  <div class="vox-showcase-card">
    <img src="/assets/axis/urbs-civilization.webp" alt="VRBS ROMA 3D/2.5D Codebase Simulator" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">gamify · VRBS ROMA</span>
      <h3 class="vox-showcase-title">Isometric Codebase Metropolis &amp; Forma Urbis Radar</h3>
      <p class="vox-showcase-desc">
        Computes Kahn topological layers (<code>L0</code>–<code>L5</code>) and Tarjan SCCs across Cargo, TypeScript, Python, and Go workspaces, inflating high-churn districts while <code>LEGIO</code> magistrate agents patrol active crate roads.
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/urbs-imperium-inspector.webp" alt="Imperium Prefecture Blast-Radius DAG Inspector" width="1280" height="800" loading="lazy" decoding="async" />
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
    <img src="/assets/axis/theme-arcane.webp" alt="Imperial Carrara White and Gold Default Theme" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">Light · arcane &amp; glacier</span>
      <h3 class="vox-showcase-title">Carrara Light &amp; Glacier Profiles</h3>
      <p class="vox-showcase-desc">
        Light palette (<code>#FAF6EE</code> canvas, <code>#1C140C</code> ink, <code>#C59B27</code> gold accents, <code>#1B6B3A</code> status tokens) with WCAG AA contrast.
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/theme-slate.webp" alt="Minimalist Grey-Blue Slate Dark Theme" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge">Dark · slate &amp; high-contrast</span>
      <h3 class="vox-showcase-title">Slate Dark &amp; WCAG AAA Profiles</h3>
      <p class="vox-showcase-desc">
        Slate dark palette (<code>#0c0e10</code> / <code>#15191c</code> / <code>#1f262b</code>) paired with <code>Inter</code> and tabular monospace fonts.
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

## 6. Federated Knowledge Base, Non-Modal Multi-Format Doc Studio & Deep Research Flywheel

Vox Axis unites live research, federated corpus search, passive discovery auto-sync, and multi-format documentation editing without trapping you inside modal overlays:

- **Non-Modal Multi-Format Doc Dock (`DocViewerDrawer` + `DocReader`):** Opens on the right with `aria-modal="false"` so the underlying workspace stays 100% interactive. Toggle **`Read | Split | Edit`** modes, adjust **`Width: Split | Wide | Full`** or **`Minimize`**, author templated files via **`+ New Doc`**, and render GFM tables, checklists, GitHub alerts, YAML frontmatter facet pills, `kb:summary` banners, **AsciiDoc**, **JSON/JSONL**, **YAML/TOML**, **CSV/TSV**, and line-numbered source code (`.vox`, `.rs`, `.ts`, `.tsx`, `.sql`).
- **Federated Memory Search (`MemoryView`):** Toggle **`Repo-Local`**, **`Global Library` (`~/.vox/library/`)**, and **`Vox Baseline`** shards simultaneously, tune retrieval depth with the **`Top-K Depth`** rotary dial, and trigger single-row **`Expand`**, **`Research`**, **`Open`** (straight into the non-modal Doc Dock), or **`Pin`** actions per citation card.
- **Above-the-Fold Deep Research & Steer (`ResearchView`):** Completed syntheses lead with the `HeadlineVerdictBanner` and rendered report at the top, expose subquery/domain observability and **`+1 Wave Refinement`** steering, offer 1-click **`Index Now`** vs. **`Save to KB`** quarantine routing, and include **`Edit in Doc Dock`** for instant Split-mode publication drafting.
- **Passive Live Discovery (`Activity` / `Discovery`) & `VoxGraph` Index (`Search Index`):** Passive event + `visibilitychange` `Live auto-sync` eliminates manual refresh clicks while surfacing 1-click jumps across **Memory**, **Research**, **Publications**, and **Search Index** RRF/node diagnostics.

<div class="vox-showcase-grid">
  <div class="vox-showcase-card">
    <img src="/assets/axis/doc-reader-arcane.webp" alt="Non-modal DocViewerDrawer docked on the right beside MemoryView with Read Split Edit modes and GFM/frontmatter rendering" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">Doc Dock · Split / Wide / Full</span>
      <h3 class="vox-showcase-title">Non-Modal Multi-Format Doc Studio</h3>
      <p class="vox-showcase-desc">
        Edit Markdown, MDX, AsciiDoc, JSON, YAML, TOML, CSV, and code in <code>Split | Wide | Full</code> widths while <code>Memory</code> or <code>Research</code> stays live.
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/memory-arcane.webp" alt="MemoryView showing 3-layer Federated KB controls, Top-K Depth rotary dial, provenance tags, and unwrapped citation action rows" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">memory · Federated KB</span>
      <h3 class="vox-showcase-title">3-Layer Retrieval &amp; In-App <code>Open</code></h3>
      <p class="vox-showcase-desc">
        Filter Repo-Local, Global Library, and Baseline canon with provenance tags and 1-click <code>Open</code> handoff into the right Doc Dock.
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/research-arcane.webp" alt="ResearchView with above-the-fold Synthesis Report, Domain Mode selector, Archetype presets, and Edit in Doc Dock button" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge vox-showcase-badge--gold">research · Multi-Wave Steer</span>
      <h3 class="vox-showcase-title">Deep Research &amp; Scoped Storage</h3>
      <p class="vox-showcase-desc">
        Run Statistical Archetypes + KB Preflight, switch <code>General | CodeGen | Scholarly | Shopping</code> modes, and steer <code>+1 Wave</code> refinement.
      </p>
    </div>
  </div>

  <div class="vox-showcase-card">
    <img src="/assets/axis/discovery-arcane.webp" alt="Discovery chronicle view with passive Live auto-sync and direct jumps to Research, Memory, and Publications" width="1280" height="800" loading="lazy" decoding="async" />
    <div class="vox-showcase-body">
      <span class="vox-showcase-badge">activity · Discovery Flywheel</span>
      <h3 class="vox-showcase-title">Passive Chronicle &amp; Flywheel Jumps</h3>
      <p class="vox-showcase-desc">
        Tracks candidate syntheses, claims, and publication events via passive <code>Live auto-sync</code> with 1-click cross-surface jumps.
      </p>
    </div>
  </div>
</div>

Step-by-step tutorials:
- **[Deep Research, Knowledge Preflight & Scientia Publication](../tutorials/tut-deep-research-and-scientia.mdx)**
- **[Managing Knowledgebases, Self-Healing OpenWiki & the Doc Studio](../tutorials/tut-knowledgebase-and-openwiki.mdx)**
