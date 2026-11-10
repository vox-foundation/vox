<!--
  README-SYNC-MAP (Maintainer Contract):
  1. Canonical scope: This file owns the 32-surface GUI catalog, VRBS ROMA simulator, 4-profile theme contract,
     and Local/Remote transport docs. Language grammar and compiler status live in vox-foundation/vox/README.md.
  2. Mirror parity: Keep /Users/brbrainerd/dev/vox-axis/README.md and /Users/brbrainerd/dev/vox/crates/vox-gui/README.md
     byte-identical (`cp /Users/brbrainerd/dev/vox-axis/README.md /Users/brbrainerd/dev/vox/crates/vox-gui/README.md`).
  3. Updating screenshots & tours:
     - Surface registry SSOT: `contracts/operations/catalog.v1.yaml` -> `vox ci gui-surface-registry --write`
     - Capture v6.6 Roman Carrara White & Gold + Slate views: `pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts`
     - Copy `artifacts/cv-flywheel/surfaces/*-after.png` -> `assets/screens/<view>.png` and rebuild `assets/tours/*.webp` via `img2webp`.
  4. Graphics rule: Use real generated raster graphics (`.webp` / `.png`) and Playwright captures only — never hand-drawn `.svg` diagrams.
-->
<div align="center">
  <img src="assets/diagrams/axis_hero_banner.webp" alt="Vox Axis — Imperium Multi-Agent Console, VRBS ROMA Codebase Simulator, and MENS ML Studio" width="100%" />

  <p>
    <strong>The desktop operator console and codebase simulator for the Vox language, multi-agent harness, and MENS ML studio</strong><br />
    <em>32 catalog-generated surfaces · <code>VRBS ROMA</code> isometric codebase simulator · Local Tauri 2 IPC &amp; Remote TOFU TLS · 4 enforced theme profiles · Universal <code>⌘K</code> palette</em>
  </p>

  <p>
    <sub>Led by <strong>Bertrand Reyna-Brainerd</strong> · Stewarded by the <strong>Vox Foundation</strong> (in formation) · Apache-2.0 · <strong>Pre-1.0 Research Prototype</strong></sub>
  </p>

  <p>
    <a href="https://voxlang.org"><strong>voxlang.org</strong></a> &nbsp;·&nbsp;
    <a href="https://github.com/vox-foundation/vox"><strong>vox-foundation/vox</strong></a> &nbsp;·&nbsp;
    <a href="https://github.com/vox-foundation/vox-axis"><strong>vox-foundation/vox-axis</strong></a> &nbsp;·&nbsp;
    <a href="https://github.com/vox-foundation/voxlang.org"><strong>vox-foundation/voxlang.org</strong></a>
  </p>

  <p>
    <img src="https://img.shields.io/badge/status-research%20prototype-red?style=flat-square" alt="Status: research prototype" />&nbsp;
    <a href="https://github.com/vox-foundation/vox-axis/commits/main"><img src="https://img.shields.io/github/last-commit/vox-foundation/vox-axis?style=flat-square&label=updated" alt="Last Updated" /></a>&nbsp;
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green?style=flat-square" alt="License" /></a>
  </p>
</div>

> [!WARNING]
> **Last reviewed: 2026-11-10 (`v6.8` / `0.6.0` workspace).** **Vox Axis** (`vox-gui`, invoked via `vox gui` or `vox axis`) is an active pre-1.0 research prototype, not production software. All 32 views are generated from [`contracts/operations/catalog.v1.yaml`](https://github.com/vox-foundation/vox/blob/main/contracts/operations/catalog.v1.yaml) (`SURFACE_REGISTRY`) and verified at `1280×800`, `1440×900`, and `1920×1080` (`axeViolations: 0`) under the **4-Profile Theme Contract** (`arcane` Sunlit Imperial Carrara White & Gold default, `slate` Minimalist Grey-Blue Dark, `glacier` Travertine Daylight, and `high-contrast` WCAG AAA Obsidian & Gold). See [Status](#status) for per-lane maturity.

---

## Animated Flagship Tour (`axis_hero_tour.webp`)

<div align="center">
  <img src="assets/tours/axis_hero_tour.webp" alt="Animated WebP tour of Vox Axis v6.6: VRBS ROMA Codebase Simulator, Imperium Blast-Radius Inspector, Mission Control Dashboard, Multi-Dock Chat + Aqueduct Cascade, MENS Corpus Studio, Needs You DAG, and Dual Theme Contract" width="96%" />
  <p>
    <strong>Vox Axis <code>v6.6–v6.8</code> Flagship Tour (Animated WebP · Real Playwright Captures)</strong><br />
    <em>Cycling through the <code>VRBS ROMA</code> Codebase Civilization Simulator, <code>Imperium Prefecture</code> Blast-Radius DAG Inspector, Mission Control Dashboard, Multi-Dock Chat Workbench, <code>AQVAEDUCTUS</code> 3-Tier Hydraulic Verification Cascade, MENS Corpus &amp; Pipeline Studio, <code>Needs You</code> Live Execution DAG, and the Enforced Theme Contract (<code>Imperial Carrara White &amp; Gold</code> vs. <code>Minimalist Grey-Blue Slate</code>).</em>
  </p>
</div>

---

## Contents

1. [Why Axis Exists (Engineering Problem & Solution)](#why-axis-exists-engineering-problem--solution)
2. [Architecture & Generated Visual Schematics](#architecture--generated-visual-schematics)
3. [Highlight Surfaces: `VRBS ROMA` Simulator, `AQVAEDUCTUS` Cascade & 4-Profile Theme Contract](#highlight-surfaces-vrbs-roma-simulator-aqvaeductus-cascade--4-profile-theme-contract)
4. [Complete Visual Reference: All 32 `SURFACE_REGISTRY` Screens Explained](#complete-visual-reference-all-32-surface_registry-screens-explained)
   - [Shell Chrome & Navigation (`⌘K` Palette, Wide Sidebar & Compact Rail)](#shell-chrome--navigation-k-palette-wide-sidebar--compact-rail)
   - [Domain I: Operate (11 Views — Orchestration, Multi-Dock Chat & Governance)](#domain-i-operate-11-views--orchestration-multi-dock-chat--governance)
   - [Domain II: Develop (8 Views — `AQVAEDUCTUS` Harness, PTY Console & Code)](#domain-ii-develop-8-views--aqvaeductus-harness-pty-console--code)
   - [Domain III: Knowledge & Socrates Gate (6 Views — Hybrid RAG & SCIENTIA)](#domain-iii-knowledge--socrates-gate-6-views--hybrid-rag--scientia)
   - [Domain IV: Compute / MENS (4 Views — Rust QLoRA Studio & Model Routing)](#domain-iv-compute--mens-4-views--rust-qlora-studio--model-routing)
   - [Domain V: System & `VRBS ROMA` (3 Views — Clavis, Doctor & Simulator)](#domain-v-system--vrbs-roma-3-views--clavis-doctor--simulator)
5. [Tradeoffs](#tradeoffs)
6. [Status](#status)
7. [Trying It & Maintaining This README](#trying-it--maintaining-this-readme)
8. [Intellectual Lineage, Prior Art & References](#intellectual-lineage-prior-art--references)

---

## Why Axis Exists (Engineering Problem & Solution)

Coordinating multiple AI coding agents, a local RAG knowledge base, and a QLoRA fine-tuning pipeline inside a scrolling terminal stream hides four failure modes:

1. **Silent prompt poisoning.** Terminal logs do not distinguish compiler-verified documentation (`SystemVerified`) from unverified web scrapes (`Untrusted`)[^1].
2. **File-lock collisions and unverified tool claims.** Concurrent subagents overwrite working-tree files unless file-affinity locks (`hopper_inbox.resource_id`) and cryptographic tool-dispatch receipts (`HMAC-SHA256` / `BLAKE3`) are visible in real time[^2][^3].
3. **Undiscoverable CLI/MCP surface area.** Vox exposes **1,056 CLI operations** and **376 MCP tools** (`contracts/operations/catalog.v1.yaml`). Without a schema-generated UI map, capabilities stay hidden behind subcommand flags.
4. **Spatial blindness in large workspaces.** A 130+ crate workspace has topological layers (`L0`–`L5`), fan-in keystones, and active git churn that a flat file tree cannot convey[^4].

**How Axis solves them:**

- **Catalog-driven UI generation (`SURFACE_REGISTRY`).** `vox ci gui-surface-registry --write` compiles `contracts/operations/catalog.v1.yaml` into [`surfaceRegistry.generated.ts`](https://github.com/vox-foundation/vox/blob/main/crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts). Every CLI and MCP operation is bound to one of **32 views** or the **`⌘K` Universal Capability Palette** (enforced by `vox ci gui-surface-coverage`).
- **Diegetic spatial telemetry (`VRBS ROMA` & `AQVAEDUCTUS`).** Inspired by Wettel & Lanza's *CodeCity*[^4] and classical Vitruvian cartography[^5], Axis maps crate layers (`L0`–`L5`), live git diffs (`+LoC / -LoC`), and active agent patrols onto an interactive isometric metropolis (`LudusSandbox.tsx`) and a 3-tier hydraulic verification cascade (`HarnessRiverPanel.tsx`).
- **Dual-Mode Transport (`transport.ts`).** The same React 19 bundle runs **100% offline in Local Mode** over in-process Tauri 2 IPC (`window.__TAURI_INTERNALS__.invoke` → local `VoxDb` v97 & `Clavis` keyring) or connects in **Remote Mode** over **TOFU SPKI SHA-256 TLS pinning**[^6] to a headless `vox-orchestrator-d` and GPU mesh node.

---

## Architecture & Generated Visual Schematics

### 1. Dual-Mode Transport: Local Tauri 2 IPC vs. Remote TOFU TLS Daemon

<div align="center">
  <img src="assets/diagrams/axis_dual_mode_architecture.webp" alt="Dual-Mode Transport Architecture: Local Tauri 2 IPC vs Remote TOFU SPKI-Pinned TLS Gateway" width="92%" />
  <p>
    <strong>Single Frontend Bundle, Two Transport Paths (<code>transport.ts</code>)</strong><br />
    <em>Local Mode calls Rust commands in-process via Tauri 2 IPC against local <code>VoxDb</code> (schema v97) and <code>Clavis</code> with zero network dependency. Remote Mode connects over a Trust-On-First-Use (TOFU) SPKI SHA-256 pinned TLS session to <code>vox-orchestrator-d</code> (`vox serve --tls-cert/--tls-key`).</em>
  </p>
</div>

### 2. The 5 Workbench Domains & `⌘K` Universal Capability Palette

<div align="center">
  <img src="assets/diagrams/axis_five_lanes_diagram.webp" alt="Vox Axis 32 Surfaces Organized Across 5 Workbench Domains Around the Command+K Palette" width="92%" />
  <p>
    <strong>5 Workbench Domains · 32 Registered Surfaces · 1,056 Cataloged Operations</strong><br />
    <em>Defined in <a href="https://github.com/vox-foundation/vox/blob/main/crates/vox-gui/ui/src/lib/navigation.ts"><code>navigation.ts</code></a> and <a href="https://github.com/vox-foundation/vox/blob/main/crates/vox-gui/ui/src/lib/surfaceRegistry.ts"><code>surfaceRegistry.ts</code></a>: <strong>Operate</strong> (11), <strong>Develop</strong> (8), <strong>Knowledge</strong> (6), <strong>Compute / MENS</strong> (4), and <strong>System &amp; VRBS ROMA</strong> (3).</em>
  </p>
</div>

### 3. End-to-End Trust Pipeline: Retrieval Tiers, Socrates Gate & `AQVAEDUCTUS` Sluices

<div align="center">
  <img src="assets/diagrams/axis_trust_tiers_diagram.webp" alt="Retrieval Trust Tiers, 5-Stage Socrates HITL Gate, and Cryptographic Execution Receipts" width="92%" />
  <p>
    <strong>Four Retrieval Trust Tiers &amp; Five Verification Gates</strong><br />
    <em>External scrapes land in <code>kb_findings</code> as <code>Untrusted</code> and must pass the 5-stage Socrates HITL gate before promotion. Tool dispatches emit verifiable <code>BLAKE3</code> / <code>HMAC-SHA256</code> receipts (<a href="https://voxlang.org/adr/029-formal-intent/">ADR-029</a>).</em>
  </p>
</div>

---

## Highlight Surfaces: `VRBS ROMA` Simulator, `AQVAEDUCTUS` Cascade & 4-Profile Theme Contract

### 1. `VRBS ROMA` — Polyglot Codebase Civilization & Blast-Radius Simulator (`gamify`)

| Simulator State | Real Playwright Capture (`1440×900`) | Engineering Mechanism & Controls |
|:---|:---|:---|
| **Isometric Metropolis & `Forma Urbis` 7-Tier Astrolabe Radar**<br/>(`urbs-civilization.png`) | <img src="assets/screens/urbs-civilization.png" alt="VRBS ROMA Workspace Civilization and Engineering Simulator" width="440" /> | `workspace_town.rs` scans any repository (Cargo `crates/*`, TS `packages/*`, Python, Go) and computes Kahn topological layers (`L0`–`L5`) + Tarjan SCCs (`topology.ts`)[^4]. Attention-weighted layout (`layout.ts`) inflates dirty/failing districts (`1.0×–3.2×`). Autonomous A* magistrate agents (`LEGIO-01..03`) patrol active crate roads while the bottom-right **`FORMA VRBIS` 7-Tier Astrolabe Radar** provides click-to-pan frustum navigation. |
| **`Imperium Prefecture` Dock & Blast-Radius DAG Inspector**<br/>(`urbs-imperium-inspector.png`) | <img src="assets/screens/urbs-imperium-inspector.png" alt="Imperium Prefecture Dock and Dependency Blast-Radius DAG Inspector" width="440" /> | Clicking any monument (`COMPILER & TYPECHECK`, `AGENT ORCHESTRATOR`, `GUI & SURFACES`, `SECURITY & SECRETS`) opens `ImperiumInspector.tsx` (`bottom-[82px]` bounded so it never occludes toolbars), rendering upstream providers, downstream blast radius, per-file `+LoC / -LoC` diff slabs, and 1-click `[+ SPAWN]`, `[CHECK]`, `[✓ TEST]`, `[TEST IMPACTED]`, and `[FIX ERROR]` dispatch buttons. |

### 2. Multi-Dock Responsive Chat Workbench & `AQVAEDUCTUS` 3-Tier Hydraulic Cascade

| Workbench Surface | Real Playwright Capture (`1440×900`) | Engineering Mechanism & Controls |
|:---|:---|:---|
| **Multi-Dock Chat + Simulator + Cascade**<br/>(`chat-multidock.png`) | <img src="assets/screens/chat-multidock.png" alt="Multi-Dock Chat Workbench with URBS Civilization and Execution Pipeline Cascade" width="440" /> | `ChatSurface.tsx` supports `FOCUS`, `SPLIT CODE`, `MULTI-AGENT`, and `TACTICAL MAP ⌘M` layouts (auto-consolidating to 2 columns below `1560px`), pairing multi-model consensus (`CONSENSUS 2–4`), git LCA branch diffing (`COMPARE BRANCHES`), and the single-row `Loquela` posture bar (`Alt+1..4`)[^7] with live companion docks. |
| **`AQVAEDUCTUS · CASTELLVM DIVISORIVM`**<br/>(`harness.png`) | <img src="assets/screens/harness.png" alt="Aquaeductus 3-Tier Hydraulic Verification Cascade and Parallel Agent Execution Braid" width="440" /> | `HarnessRiverPanel.tsx` visualizes live model routing and verification as a 3-tier Roman hydraulic sluice cascade (`CURSUS I · Fast Local MENS`, `CURSUS II · Balanced Hybrid`, `CURSUS III · Deep Frontier`) flowing through 4 verification gates (`AST PARSE` → `TYPECHECK` → `TOESTUB` → `TEST SUITE`) above the `Parallel Agent Execution Braid`. |

### 3. Enforceable 4-Profile Theme Contract (`contracts/gui/theme-contract.v1.json`)

Every `.tsx`, `.ts`, and `.css` file under `crates/vox-gui/ui/src/` is statically gated by `vox-code-audit` detector `vox/ui/theme-contract-drift` (`Severity::Error`) and Style-Dictionary token generation (`22/22` semantic CSS variables, WCAG 2.1 contrast $\ge 4.5:1$):

| Default: `Imperial Carrara White & Gold` (`arcane`) | Dark Mode: `Minimalist Grey-Blue Slate` (`slate`) |
|:---:|:---:|
| <img src="assets/screens/theme-arcane.png" alt="Imperial Carrara White and Gold Default Theme" width="430" /> | <img src="assets/screens/theme-slate.png" alt="Minimalist Grey-Blue Slate Dark Theme" width="430" /> |
| *Sunlit honed Carrara marble (`#FAF6EE`), burnished gold mouldings (`#C59B27`), deep espresso ink (`#1C140C`), Romanesque serif (`Cinzel` / `Palatino`), and emerald laurel working signets (`#1B6B3A`).* | *Distraction-free basalt/slate dark palette (`#0c0e10` / `#15191c` / `#1f262b`), steel-blue & emerald active indicators, crisp geometry, and zero marble textures.* |

*(Also ships with **`Travertine Daylight` (`glacier`)** [`assets/screens/theme-glacier.png`](assets/screens/theme-glacier.png) and **`High Contrast Obsidian & Gold` (`high-contrast`, WCAG AAA)** [`assets/screens/theme-high-contrast.png`](assets/screens/theme-high-contrast.png).)*

---

## Complete Visual Reference: All 32 `SURFACE_REGISTRY` Screens Explained

### Shell Chrome & Navigation (`⌘K` Palette, Wide Sidebar & Compact Rail)

| Shell Mode | Real Playwright Capture | Purpose & Key Shortcuts |
|:---|:---|:---|
| **Universal `⌘K` Capability Palette**<br/>(`_command-palette.png`) | <img src="assets/screens/_command-palette.png" alt="Command+K Universal Capability Palette" width="360" /> | Hybrid Tantivy BM25 + RRF semantic search[^8] over all **1,056 CLI operations**, **376 MCP tools**, and **32 GUI views** (`contracts/capabilities/capability-index.v1.json`). Includes side-effect confirmation gates before running mutating commands. |
| **Wide Grouped Sidebar & Hidden/Rail Mode**<br/>(`_sidebar-wide.png` / `_sidebar-rail.png`) | <img src="assets/screens/_sidebar-wide.png" alt="Wide Sidebar Mode" width="360" /><br/><img src="assets/screens/_sidebar-rail.png" alt="Sidebar Hidden Restore Pill Mode" width="360" /> | Switches between the 5-domain grouped sidebar (`OPERATE · CHAT`, `AGENTS & RUNS`, `KNOWLEDGE`, `WORKSPACE`, `COMPUTE & SYSTEM`) and collapsed full-canvas mode while pinning the interactive bottom status bar (`ENGINE`, `SPEND`, `MESH`, `ROUTING`, `NEEDS YOU`, `RESEARCH`, `CONFIGURE ▾`). |

---

### Domain I: Operate (11 Views — Orchestration, Multi-Dock Chat & Governance)

<div align="center">
  <img src="assets/tours/axis_operate_lane_tour.webp" alt="Animated WebP tour of all 11 Operate Domain screens in Vox Axis" width="94%" />
</div>

| # | `viewKey` & Title | Real Screenshot (`v6.6`) | Why It Exists & What It Does | Backing CLI / Crate |
|:---|:---|:---|:---|:---|
| **1** | `dashboard`<br/>**Mission Control** | <img src="assets/screens/dashboard.png" alt="Dashboard Screen" width="340" /> | At-a-glance health cockpit: 4 sparkline KPIs (`Active Agents`, `Queue Depth`, `Budget Spent`, `Mesh Peers`), embedded interactive `VRBS ROMA` architecture map (`260px`), active agent cards, and telemetry alerts. | `vox status`<br/>`vox-orchestrator-d` |
| **2** | `chat`<br/>**Chat Workbench (`Loquela`)** | <img src="assets/screens/chat.png" alt="Chat Workbench Screen" width="340" /> | Multi-turn engineering chat with single-row progressive-disclosure posture controls (`Loquela.tsx`), BPE compaction receipts (`Goal Anchor: Kept`), retrieval trust chips, HMAC tool receipts, and multi-dock tiling. | `vox chat`<br/>`vox_actor_runtime::llm` |
| **3** | `tasks`<br/>**Tasks (Hopper Queue)** | <img src="assets/screens/tasks.png" alt="Tasks Hopper Queue Screen" width="340" /> | Prevents concurrent file-write races by surfacing `hopper_inbox` task ordering, priority (`Urgent`/`Normal`/`Background`), and per-file `resource_id` lock ownership ([ADR-025](https://voxlang.org/adr/025-multi-agent-lock-coherence/)). | `vox plan`<br/>`vox_orchestrator::hopper` |
| **4** | `sub-agents`<br/>**Sub-Agents Tree** | <img src="assets/screens/sub-agents.png" alt="Sub-Agents Screen" width="340" /> | Live parent→child subagent supervision tree showing workspace isolation (`inherit`, `branch`, `share`), role assignments, turn progress, and kill-switch controls. | `vox-orchestrator`<br/>`mcp_agent_*` |
| **5** | `flow`<br/>**Durable Workflows** | <img src="assets/screens/flow.png" alt="Durable Workflow Flow Screen" width="340" /> | Visualizes event-sourced `.vox` `workflow` and `activity` replay journals (`workflow_patch_log`, activity ordinals, signal/timer parking per [ADR-019](https://voxlang.org/adr/019-durable-workflow-journal-contract-v1/))[^9]. | `vox workflow`<br/>`vox-workflow-runtime` |
| **6** | `mesh`<br/>**Populi Mesh Topology** | <img src="assets/screens/mesh.png" alt="Mesh Topology Screen" width="340" /> | Displays Iroh QUIC peer-to-peer nodes ([ADR-047](https://voxlang.org/adr/047-iroh-transport/)), ALPN negotiation, direct vs. relay paths, and `donations.vox` worker quotas. | `vox populi`<br/>`vox-plugin-populi-mesh` |
| **7** | `runs`<br/>**Execution Runs Ledger** | <img src="assets/screens/runs.png" alt="Runs History Screen" width="340" /> | Auditable ledger of past agent episodes, script runs, and workflow executions with duration, token cost, exit code, and commit SHA links. | `vox-db` execution store |
| **8** | `needs-you`<br/>**Needs You (Attention Gate)** | <img src="assets/screens/needs-you.png" alt="Needs You Attention Gate Screen" width="340" /> | Filters out low-value agent interruptions using expected-information-gain scoring (`attention_policy.rs`), pairing pending approvals/doubts with the **Live Agent Topology & Execution DAG**. | `vox attention`<br/>`vox-orchestrator-mcp` |
| **9** | `approvals`<br/>**Approvals & Diff Gate** | <img src="assets/screens/approvals.png" alt="Approvals Screen" width="340" /> | Human-in-the-loop sign-off queue for privileged shell commands, secret resolutions, and schema migrations with inline `Inspect Diff` previews. | `vox policy`<br/>`vox-orchestrator-mcp` |
| **10** | `policies`<br/>**Execution & Safety Policies** | <img src="assets/screens/policies.png" alt="Policies Screen" width="340" /> | Inspects terminal allowlists (`contracts/terminal/exec-policy.v1.yaml`), scaling budgets (`contracts/scaling/policy.yaml`), and Socrates confidence thresholds. | `vox shell check`<br/>`vox-scaling-policy` |
| **11** | `mercatus`<br/>**Mercatus (Token & Cost Market)** | <img src="assets/screens/mercatus.png" alt="Mercatus Cost Screen" width="340" /> | Tracks real-time token spend and battery units (`1 BU = $0.001`) across OpenRouter providers vs. local MENS QLoRA inference savings. | `vox model`<br/>`vox-orchestrator-models` |

---

### Domain II: Develop (8 Views — `AQVAEDUCTUS` Harness, PTY Console & Code)

<div align="center">
  <img src="assets/tours/axis_develop_lane_tour.webp" alt="Animated WebP tour of all 8 Develop Domain screens in Vox Axis" width="94%" />
</div>

| # | `viewKey` & Title | Real Screenshot (`v6.6`) | Why It Exists & What It Does | Backing CLI / Crate |
|:---|:---|:---|:---|:---|
| **12** | `console`<br/>**Daemon PTY Console** | <img src="assets/screens/console.png" alt="Console PTY Screen" width="340" /> | Persistent daemon-hosted terminal sessions (`SessionHost`, `term.*` RPCs with ring-buffer replay, `#641`) that survive GUI restarts and work over Remote Mode. | `vox term`<br/>`vox-terminal-core` |
| **13** | `catalog`<br/>**Capability Catalog** | <img src="assets/screens/catalog.png" alt="Capability Catalog Screen" width="340" /> | Interactive browser and parameter form runner for all **1,056 cataloged operations** with single-line `[OPEN FORM]`, `[EXPLAIN CAPABILITY]`, and `[OPEN DOCS]` actions. | `vox commands`<br/>`vox explain` |
| **14** | `skills`<br/>**Agent Skills Registry** | <img src="assets/screens/skills.png" alt="Skills Registry Screen" width="340" /> | Discovers and searches [agentskills.io](https://agentskills.io/specification) `SKILL.md` directories (`.vox/skills/`, `.agents/skills/`, Superpowers[^10]) via `vox-search`. | `vox skill list`<br/>`vox-skills` |
| **15** | `harness`<br/>**`AQVAEDUCTUS` Pipeline Cascade** | <img src="assets/screens/harness.png" alt="Harness Aqueduct Cascade Screen" width="340" /> | 3-tier hydraulic sluice cascade (`Fast Local MENS`, `Balanced Hybrid`, `Deep Architecture`) + 4-stage verification gates (`AST Parse`, `Typecheck`, `TOESTUB`, `Test Suite`) + `Parallel Agent Execution Braid`. | `vox harness eval`<br/>`vox-orchestrator` |
| **16** | `harness-health`<br/>**Harness Health (`vox audit`)** | <img src="assets/screens/harness-health.png" alt="Harness Health Audit Screen" width="340" /> | Aggregates static audit findings from `vox audit code` (`toestub`), `vox audit arch` (`layers.toml`), `vox audit retirement`, and `vox audit effort`. | `vox audit`<br/>`vox-code-audit` |
| **17** | `repository`<br/>**4-Repo Federation** | <img src="assets/screens/repository.png" alt="Repository Federation Screen" width="340" /> | Displays `.vox/repositories.yaml` status across `vox`, `vox-axis`, `voxlang.org`, and `vox-ops`, verifying `55/55` shared contract digests and `404` federated symbol-graph nodes. | `vox repo`<br/>`vox-repository` |
| **18** | `browser`<br/>**App Preview & DOM Inspector** | <img src="assets/screens/browser.png" alt="Browser Preview Screen" width="340" /> | Live preview frame for full-stack `.vox` apps (`vox dev` / `vox build`) with responsive viewport presets and console/DOM diagnostic capture. | `vox dev`<br/>`vox-codegen` |
| **19** | `coderabbit`<br/>**AI Diff Review** | <img src="assets/screens/coderabbit.png" alt="CodeRabbit Review Screen" width="340" /> | Runs structured pre-merge diff reviews grouped by semantic domain (`contracts/review/coderabbit-semantic-groups.v1.yaml`) and severity. | `vox review`<br/>`vox-code-audit` |

---

### Domain III: Knowledge & Socrates Gate (6 Views — Hybrid RAG & SCIENTIA)

<div align="center">
  <img src="assets/tours/axis_knowledge_hitl_tour.webp" alt="Animated WebP tour of all 6 Knowledge Domain screens in Vox Axis" width="94%" />
</div>

| # | `viewKey` & Title | Real Screenshot (`v6.6`) | Why It Exists & What It Does | Backing CLI / Crate |
|:---|:---|:---|:---|:---|
| **20** | `vox-search`<br/>**Hybrid Search (`vox kb`)** | <img src="assets/screens/vox-search.png" alt="Hybrid Search Screen" width="340" /> | Queries the Tantivy 0.22 lexical + semantic RRF index[^8] (`search_documents`), filtering by trust tier (`SystemVerified` → `Untrusted`) and W3C SKOS facets (`kb-taxonomy.v1.yaml`)[^11]. | `vox kb search`<br/>`vox-search` |
| **21** | `memory`<br/>**Episodic & Fact Memory** | <img src="assets/screens/memory.png" alt="Episodic Memory Screen" width="340" /> | Inspects and prunes persistent agent facts (`MemoryManager::persist_fact` / `lookup_fact_by_key`) and session handoff bundles stored in `VoxDb`. | `vox memory`<br/>`vox-orchestrator` |
| **22** | `activity`<br/>**Discovery Scout Feed** | <img src="assets/screens/activity.png" alt="Discovery Scout Screen" width="340" /> | Triages incoming research signals from Phase A/F signal producers (`vox-scientia-producers`) before promoting them to full investigations. | `vox scientia scout`<br/>`vox-scientia` |
| **23** | `research`<br/>**Deep Research & 5-Stage Socrates Gate** | <img src="assets/screens/research.png" alt="Deep Research and Socrates Gate Screen" width="340" /> | Steerable multi-source research (`OpenAlex`, `arXiv`, `Wikipedia`, `SearXNG`, `Tavily`) with per-provider RRF diversity caps and the 5-stage Socrates HITL stepper (`Planning` → `Retrieving` → `Verifying Claims` → `Judge Inspector` → `Sentry Audit`). | `vox research`<br/>`vox-scientia` |
| **24** | `scientia`<br/>**SCIENTIA Findings Ledger** | <img src="assets/screens/scientia.png" alt="SCIENTIA Findings Screen" width="340" /> | Structured ledger of verified empirical findings (`kb_findings`), replay receipts, McNemar benchmark proofs, and nanopublication attestations[^11]. | `vox scientia`<br/>`vox-scientia-core` |
| **25** | `publications`<br/>**Publications Studio** | <img src="assets/screens/publications.png" alt="Publications Studio Screen" width="340" /> | Scaffolds citation-locked technical reports and manuscript sections from verified SCIENTIA findings (`vox-manuscript-scaffold`). | `vox scientia`<br/>`vox-findings-site` |

---

### Domain IV: Compute / MENS (4 Views — Rust QLoRA Studio & Model Routing)

<div align="center">
  <img src="assets/tours/axis_compute_system_tour.webp" alt="Animated WebP tour of Compute (MENS), System, VRBS ROMA Simulator, and 4 Theme Profiles" width="94%" />
</div>

| # | `viewKey` & Title | Real Screenshot (`v6.6`) | Why It Exists & What It Does | Backing CLI / Crate |
|:---|:---|:---|:---|:---|
| **26** | `mens`<br/>**MENS Corpus & Pipeline Studio** | <img src="assets/screens/mens.png" alt="MENS Corpus and Pipeline Studio Screen" width="340" /> | Pure-Rust (Candle) QLoRA[^12] studio across Qwen3 (`0.6B`–`32B`) and 8 domain spokes: `Mix Sources & Auto-DAG`, `Live JSONL Record Inspector & AST Editor` (inline `vox_compiler::parser` verification), and `Ladder Checkpoints & McNemar α=0.05 Gate`[^13]. | `vox mens train`<br/>`vox-populi` |
| **27** | `models`<br/>**Models Router & Leaderboard** | <img src="assets/screens/models.png" alt="Models Router Screen" width="340" /> | Inspects `vox-orchestrator-models` registry scores, OpenRouter `~family-latest` aliases, multi-benchmark priors (`SweBench`, `LiveBench`), and McNemar `eval-compare` leaderboards. | `vox model`<br/>`vox-orchestrator-models` |
| **28** | `populi`<br/>**Populi Compute Nodes** | <img src="assets/screens/populi.png" alt="Populi Compute Nodes Screen" width="340" /> | Hardware-aware model selector (`select_mens_backend`) over Apple Silicon Metal (`monitor_metal`) and CUDA (`nvidia-smi` `accelerator_free_mb`) with VRAM floor guardrails. | `vox populi`<br/>`vox-populi` |
| **29** | `oratio`<br/>**Oratio (Speech-to-Code)** | <img src="assets/screens/oratio.png" alt="Oratio Voice Screen" width="340" /> | Grammar-aware speech-to-code capture (`vox-tauri-stt` / `vox-speech`), ASR filler normalization, and spoken-operator lowering. | `vox speech`<br/>`vox-speech` |

---

### Domain V: System & `VRBS ROMA` (3 Views — Clavis, Doctor & Simulator)

| # | `viewKey` & Title | Real Screenshot (`v6.6`) | Why It Exists & What It Does | Backing CLI / Crate |
|:---|:---|:---|:---|:---|
| **30** | `settings`<br/>**Settings, Clavis & Theme Contract** | <img src="assets/screens/settings.png" alt="Settings and Theme Contract Screen" width="340" /> | Manages **Clavis** secrets (`vox-secrets`), Local IPC vs. Remote TOFU TLS connection pins, orchestrator concurrency/budget caps, and the **4-Profile Theme Contract** with live DOM WCAG observability (`22/22 Verified`). | `vox secrets`<br/>`vox config` |
| **31** | `explorer`<br/>**System Explorer (`vox doctor`)** | <img src="assets/screens/explorer.png" alt="Explorer Doctor Screen" width="340" /> | Runs diagnostic probes across toolchain, build-broker shim (`vox-cargo-shim`), `VoxDb` schema baseline (`v97`), and Clavis secret parity. | `vox doctor`<br/>`vox-cli-diagnostics` |
| **32** | `gamify`<br/>**`VRBS ROMA` Simulator & Ludus Arena** | <img src="assets/screens/gamify.png" alt="VRBS ROMA Gamify Simulator Screen" width="340" /> | Combines the **`VRBS ROMA` Workspace Civilization & Engineering Simulator** (`WORKSPACE MAP`), `LIVE RIVER · PIPELINE`, `ARENA & QUESTS`, and `BLAKE3 + Ed25519` signed `LEADERBOARD` snapshots. | `vox ludus`<br/>`vox-gamify` |

---

## Tradeoffs

| Design Choice | What You Gain | What It Costs |
|:---|:---|:---|
| **Catalog-generated 32-surface registry (`SURFACE_REGISTRY`)** | Zero drift between CLI, MCP, and GUI (`vox ci gui-surface-coverage`). | Wide surface area; operators rely on `⌘K` and progressive-disclosure tabs rather than memorizing 32 routes. |
| **Tauri 2 native desktop shell + React 19** | Small memory footprint, in-process Rust IPC in Local Mode, and zero required local web server. | Building `vox-gui` on Linux requires `libwebkit2gtk-4.1-dev`, so `vox-gui` is excluded from default Cargo builds (`default-members = ["crates/vox-cli"]`). |
| **Enforced 4-profile CSS token contract (`vox/ui/theme-contract-drift`)** | Every view passes WCAG contrast (`>= 4.5:1`) and `axeViolations: 0` across Carrara marble light and Slate dark themes. | Hardcoded dark hex colors (`bg-[#120E09]`) or banned monospace fallbacks (`Courier`, `Monaco`) fail `vox audit code` with `Severity::Error`. |
| **TOFU SPKI SHA-256 TLS pinning (`Remote Mode`)** | Connects securely to headless GPU/daemon boxes without public CA setup. | Rotating a remote daemon's TLS key requires re-approving the pinned SPKI fingerprint in `Settings → Server`. |

---

## Status

Graded into four honest states (`Working`, `Prototype`, `Research`, `Not started`):

| Subsystem | State | Checked | Notes |
|:---|:---|:---|:---|
| **App Shell, 5-Domain Sidebar, Status Bar & `⌘K` Palette** | Working | 2026-11-10 | Verified across `1280×800`, `1440×900`, and `1920×1080` in `cv-flywheel.spec.ts` (`0` occlusions, `0` clipped controls). |
| **4-Profile Theme Contract (`arcane`, `slate`, `glacier`, `high-contrast`)** | Working | 2026-11-10 | Enforced by Style-Dictionary, `themeContract.test.ts` (`37` views + `14` docks), and `vox/ui/theme-contract-drift` (`Severity::Error`). |
| **`VRBS ROMA` Simulator (`LudusSandbox`) & `AQVAEDUCTUS` Cascade (`HarnessRiverPanel`)** | Working | 2026-11-10 | Synchronized panorama camera, `ImperiumInspector` blast-radius DAG, 7-tier `Forma Urbis` radar, and 3-tier hydraulic cascade verified in Phase 385–386. |
| **Multi-Dock Chat (`ChatSurface`, `Loquela`, `ChatExecutionRail`)** | Prototype | 2026-11-10 | Responsive 2-column auto-consolidation (`< 1560px`), `432` posture combinations, BPE compaction receipts, and LCA branch comparison work on primary paths. |
| **Dual Transport (`Local Tauri 2 IPC` & `Remote TOFU TLS`)** | Prototype | 2026-11-10 | `VoxDb` file-lock contention between `ModelRegistry` and `step_db` resolved (`Phase 385`). Multi-day remote packet-loss recovery is still being hardened. |
| **MENS Corpus & Crucible Studio (`mens`, `models`, `populi`)** | Prototype | 2026-11-10 | Inline `vox_compiler::parser` AST verification of `<think>` JSONL rows and McNemar gate scorecards work; `oratio` voice dictation remains `Research`. |

---

## Trying It & Maintaining This README

```bash
# 1. Build UI tokens & verify TypeScript + Vitest theme/surface contracts:
./scripts/build.sh
./scripts/test.sh

# 2. Run the full multi-resolution Playwright CV Flywheel (1280x800, 1440x900, 1920x1080):
pnpm --dir /Users/brbrainerd/dev/vox/crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts --project=chromium

# 3. Launch the native Tauri 2 desktop app from the vox workspace:
cargo run -p vox-cli --manifest-path /Users/brbrainerd/dev/vox/Cargo.toml --features gui -- gui
```

**Cross-Repo Sync Rule:** `vox-foundation/vox-axis` shares `contracts/gui/` and `contracts/telemetry/` with [`vox-foundation/vox`](https://github.com/vox-foundation/vox), [`vox-foundation/voxlang.org`](https://github.com/vox-foundation/voxlang.org), and private `vox-ops`. Always run `vox ci cross-repo-parity` and `vox ci federated-symbol-graph` after modifying shared contracts or GUI surfaces (see [`AGENTS.md`](AGENTS.md)).

---

## Intellectual Lineage, Prior Art & References

[^1]: **Retrieval Trust Tiers & Prompt-Injection Quarantine:** Indirect prompt injection defense follows Greshake et al., *"Not What You've Signed Up For: Compromising Real-World LLM-Integrated Applications with Indirect Prompt Injection"* ([arXiv:2302.12173](https://arxiv.org/abs/2302.12173), 2023), enforced in Vox via 4 database-level `search_documents.trust_tier` levels (`SystemVerified` → `Untrusted`) and the `kb_findings` quarantine inbox.
[^2]: **Actor Supervision & File-Affinity Locks:** Multi-agent task routing and per-resource locking build on Carl Hewitt, Peter Bishop, and Richard Steiger, *"A Universal Modular ACTOR Formalism for Artificial Intelligence"* (IJCAI 1973), Erlang/OTP supervision trees (Joe Armstrong, 2003), and Vox [ADR-025: Multi-Agent Lock Coherence](https://voxlang.org/adr/025-multi-agent-lock-coherence/).
[^3]: **Model Context Protocol (MCP) & Formal Intent Receipts:** Tool dispatch uses Anthropic's [Model Context Protocol](https://modelcontextprotocol.io/) augmented with HMAC-SHA256 / BLAKE3 execution receipts ([ADR-029: Formal Intent](https://voxlang.org/adr/029-formal-intent/)).
[^4]: **Codebase Visualization (`VRBS ROMA`):** The mapping of software packages, LoC, and churn into an interactive 3D/isometric urban metaphor credits Richard Wettel and Michele Lanza, *"Visualizing Software Systems as Cities"* (*CodeCity*, VISSOFT 2007), combined with Kahn's topological layering (1962), Tarjan's strongly connected components (1972), and *Caesar III* (Impressions Games, 1998) civic telemetry walkers.
[^5]: **Roman Cartography (`Forma Urbis Romae` & Vitruvian Proportions):** The 7-Regio astrolabe radar and architectural hierarchy draw from the Severan Marble Plan (*Forma Urbis Romae*, c. 203–211 CE) and Vitruvius, *De Architectura* (Book III symmetry and module proportions).
[^6]: **Trust-On-First-Use (TOFU) & SPKI Pinning:** Remote daemon authentication adapts Dan Wendlandt, David G. Andersen, and Adrian Perrig, *"Perspectives: Improving SSH-style Host Authentication with Multi-Path Probing"* (USENIX ATC 2008) and RFC 7469 Subject Public Key Info (SPKI) SHA-256 fingerprinting.
[^7]: **T3 Ecosystem Harness & Router Patterns:** Stream resumption (`ResumableTurnStreamBuffer`), multi-benchmark routing priors, and thin-transport static checks credit [`pingdotgg/t3code`](https://github.com/pingdotgg/t3code) and [`vibheksoni/t3router`](https://github.com/vibheksoni/t3router), documented in [`t3-vs-vox-harness-router-blueprint-2026.md`](https://github.com/vox-foundation/vox/blob/main/docs/src/architecture/t3-vs-vox-harness-router-blueprint-2026.md).
[^8]: **Hybrid Search & Reciprocal Rank Fusion (RRF):** Lexical + semantic ranking combines [Tantivy](https://github.com/quickwit-oss/tantivy) BM25 with Gordon V. Cormack, Charles L. A. Clarke, and Stefan Büttcher, *"Reciprocal Rank Fusion Outperforms Condorcet and Individual Rank Learning Methods"* (SIGIR 2009).
[^9]: **Durable Execution Journals:** Deterministic workflow replay over non-deterministic activity logs follows [Temporal](https://temporal.io/) / Uber Cadence and Microsoft Azure Durable Functions ([ADR-019](https://voxlang.org/adr/019-durable-workflow-journal-contract-v1/), [ADR-041](https://voxlang.org/adr/041-durable-functions-completion-2026/)).
[^10]: **Portable Agent Skills (`SKILL.md`):** Skill discovery implements the open [agentskills.io specification](https://agentskills.io/specification) and integrates Jesse Vincent's [*Superpowers*](https://github.com/obra/superpowers) workflow skill library.
[^11]: **SKOS Taxonomies & Nanopublications:** Knowledge-base facets follow [W3C SKOS (Simple Knowledge Organization System)](https://www.w3.org/2004/02/skos/), and SCIENTIA finding attestations adapt Paul Groth, Andrew Gibson, and Jan Velterop, *"The Anatomy of a Nanopublication"* (*Information Services & Use*, 2010).
[^12]: **QLoRA & Pure-Rust Tensor Compute:** Native ML fine-tuning builds on Tim Dettmers et al., *"QLoRA: Efficient Finetuning of Quantized LLMs"* ([arXiv:2305.14314](https://arxiv.org/abs/2305.14314), NeurIPS 2023), Hugging Face [`candle`](https://github.com/huggingface/candle), and Alibaba Cloud's [Qwen3](https://huggingface.co/Qwen) base models.
[^13]: **Statistical Model Evaluation:** Paired code-generation benchmarks use Mark Chen et al., *"Evaluating Large Language Models Trained on Code"* (`pass@k`, [arXiv:2107.03374](https://arxiv.org/abs/2107.03374), 2021), Quinn McNemar's paired nominal test (*Psychometrika*, 1947), and Sture Holm's step-down multiple-testing correction (*Scandinavian Journal of Statistics*, 1979).
