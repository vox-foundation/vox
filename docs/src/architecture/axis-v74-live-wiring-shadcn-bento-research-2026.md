---
title: "Vox Axis v7.4 Live Control Wiring, Dual-Theme shadcn/Radix Primitives & Concept-Art Bento Architecture (2026)"
description: "Source- and registry-verified architectural synthesis for Vox Axis Milestone v7.4 covering zero-paper-code Tauri/MCP wiring, dual-theme shadcn/Radix primitives, zero-emoji vector semiotics, @container Bento compaction, and multi-pass CV Flywheel convergence."
category: "Architecture SSOTs"
status: "current"
topic: "desktop-app"
domain: "gui"
authorship: "llm-generated"
authorship_model: "gemini-4-carbon"
---

# Vox Axis v7.4 Live Control Wiring, Dual-Theme shadcn/Radix Primitives & Concept-Art Bento Architecture (2026)

<!-- kb:summary -->
Synthesizes the Milestone v7.4 architectural audit and implementation plan (`Phases 431–436`) for Vox Axis (`crates/vox-gui`). Specifies the unified `radix-ui` and Tailwind v4 `@theme inline` dual-theme component kit across all four Style Dictionary profiles, a two-tier offline SVG icon system purging emojis and Unicode dingbats across fifty-plus files, single-funnel `voxTransport` IPC parity with zero catch-and-fake fallbacks, `@container` inline-size Bento compaction with resizable split workbenches, and full-document plus portal accessibility and computer-vision flywheel gates.
<!-- /kb:summary -->

## 1. Context & Baseline Defect Audit (`crates/vox-gui`)

Building on the multi-resolution framing and diegetic canvas foundations documented in [Vox Axis v7.2 Visual Audit, Photorealistic Diegetic FX & Multi-Resolution CV Flywheel](axis-v72-visual-audit-and-fx-ssot-2026.md), a full line-by-line source and runtime audit of `crates/vox-gui/src/` and `crates/vox-gui/ui/` identified five structural gaps separating v7.3's visual shell from a zero-deception desktop engineering workbench:

| Audit Dimension | Measured v7.3 Baseline Defect | Root Cause in Codebase | Milestone v7.4 Architectural Target |
| --- | --- | --- | --- |
| **Paper-Code Controls & Fake RPC Fallbacks** | 6 broken `viewKey`/RPC casts (`HudPanels.tsx`, `HarnessRiverPanel.tsx`, `ImperiumInspector.tsx`), 11 local-only buttons (`Compact Context`, `Claim Yield`, `Sluice Gates`, `Pin Context`, `Approve`), ~65 passive `RomanBrassGauge` buttons, 7 hardcoded gauge ratios, and silent `_demo`/OID fabrication in `t3Wave3Rpc.ts`. | `if (backendAvailable()) { try ... catch {} }` blocks swallow real Tauri/MCP failures into preview stubs; 38 files call `invoke()` directly without `permissionMode`. | 100% live Tauri/MCP wiring via `voxTransport`; zero catch-and-fake fallbacks when `backendAvailable() === true`; empirical DTO ratios on all gauges. |
| **Fragmented Primitives & Dead CSS Transitions** | 28 raw native `<select>` menus, 5 raw `<input type="range">` sliders, non-portaled 19-line `Popover.tsx`, and dead `animate-in`/`fade-in-0` classes on `Dialog.tsx` and `Tooltip.tsx`. | Only 3 standalone `@radix-ui/react-*` packages installed; neither `tw-animate-css` nor a unified `radix-ui` primitive layer is present. | Unified `radix-ui@^1.8.1` + `tw-animate-css@^1.4.0` + `cva@^0.7.1` + `cmdk@^1.1.1` + tactile SVG `RomanRotaryKnob` chronometer dials. |
| **CSS `!important` Clobbering & Portal Z-Index Wars** | Radix `Switch`/`Checkbox`/`Select` states flattened to cream marble; `bg-black/60` modal scrims turned into opaque `#FAF5E8` walls; `Toasts.tsx` (`z-40`) hidden behind open `z-50` `Dialog`s; `body[data-scroll-locked]` tripping `outerOverflowPx > 0`. | Unscoped `:is(main, [role="dialog"]) button` (`index.css:311,1076`) and `[class*="bg-black"]` (`index.css:708`) overrides collide with ad-hoc `z-50` / `z-[99999]` layers. | Mandatory `data-slot` attribute scoping (`:not([data-slot])`), opaque `--color-popover` token binding, and a single `@theme` z-index scale (`40..100`). |
| **Residual Emojis & BMP Dingbat Leaks** | 50+ JSX files and 2 `<canvas>` renderers emit raw emojis (`🔒`, `⚡`, `⚔`, `🌿`, `⛵`, `⚱`, `⚠️`) and BMP dingbats (`✓`, `✕`, `✎`, `✂`, `☼`, `▸`, `▾`, `◆`, `⋮⋮`). | `e2e/review/audits.ts:269-281` exempted `[data-testid="urbs-imperium-inspector"]`, scanned only `button, h1..h3`, and omitted `\u{2190}..\u{27BF}`. | Two-tier offline SVG semiotics (`Icons.tsx` + `lucide-react@^1.72.0` + Canvas `Path2D`), zero exemptions in `audits.ts`, and static Vitest source guard. |
| **High-Whitespace & Viewport-Coupled Layouts** | `oratio` and `populi` render 2-card `<pre>` CLI stdout dumps (`CommandCardsView.tsx`); `catalog` leaves >50% empty right-hand gutter; `sm:`/`lg:` classes crush grids inside 320px Chat docks; `cv-flywheel.spec.ts` skips `document.body` portals in Axe scans. | Missing `container-type: inline-size` (`@container`) grid wrappers, missing `react-resizable-panels` master-detail splitters, and `.include('main')` Axe scoping. | `@container/surface` Bento grids, `react-resizable-panels@^4.22.1` splits, dedicated `OratioStudioView` & `PopuliMeshBentoView`, and full-body + portal Axe/CV gates. |

## 2. Verified Dependency & Design-Token Architecture

All frontend additions in `crates/vox-gui/ui/package.json` are pure ESM/TS packages verified against `react@19.2.8`, `tailwindcss@^4.3.5`, and Vite 8 with zero `install`/`postinstall` lifecycle scripts (preserving `crates/vox-gui/ui/pnpm-workspace.yaml` without `allowBuilds` expansion) and strict `knip` (`"files": "error"`, `"dependencies": "error"`) compliance:

- **Unified Accessible Primitives (`radix-ui@^1.8.1`):** Replaces fragmented `@radix-ui/react-dialog`, `@radix-ui/react-slot`, and `@radix-ui/react-tooltip` entries with a single tree-shakeable (`"sideEffects": false`) monorepo dependency eliminating `@radix-ui/react-compose-refs` version skew under React 19.
- **Tailwind v4 Animation & Variant Tooling (`tw-animate-css@^1.4.0`, `class-variance-authority@^0.7.1`):** Pure CSS entry/exit utilities respecting `@media (prefers-reduced-motion: reduce)` (`index.css:511`) paired with strongly-typed `cva()` variants using compositor-safe properties (`transition-[color,background-color,border-color,opacity,transform,box-shadow]`, never `transition-all`, which fails `vox/ui/layout-and-perf-hazards`).
- **Workbench Split, Command & Table Engines (`react-resizable-panels@^4.22.1`, `cmdk@^1.1.1`, `@tanstack/react-table@^9.3.1`):** Uses the v4 `Group` / `Panel` / `Separator` (`orientation`) API for 2- and 3-pane master-detail surfaces (`>960px` split, `<960px` `Sheet` slide-over) while retaining `dockview@6.6.1` exclusively for 14-panel multi-tab docking in `DockWorkspaceShell.tsx`; upgrades `Omnibar.tsx` onto `cmdk` while preserving `vox_search_structural` graph discovery; and backs `DataTable.tsx` with v9 `useTable` + `tableFeatures()` composed with `@tanstack/react-virtual@^3.14.10`.
- **Two-Tier Offline Vector Semiotics (`lucide-react@^1.72.0` + `crates/vox-gui/ui/src/components/ui/Icons.tsx`):** Standard UI chrome imports tree-shaken `lucide-react` icons normalized to `24x24` and `strokeWidth={1.5}` via `<VoxIcon>`, while Romanesque domain identity (`laurelWreath`, `gladiusShield`, `denariusCoin`, `brazierTorch`, `aqueductArch`, `waxSeal`, `amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`) renders from local SVGs in `Icons.tsx` — preserving 100% offline standalone sovereignty with zero runtime CDN/API icon fetches.

### 2.1 Four-Theme `@theme inline` Token Binding & Z-Index Ladder

Never introduce standalone HSL channels (`--background: 0 0% 100%`) that bypass Style Dictionary (`style-dictionary.config.mjs`). Instead, `crates/vox-gui/ui/src/index.css` binds shadcn semantic aliases via Tailwind v4 `@theme inline` directly to Vox's 4 generated theme sheets (`arcane` Sunlit White/Gold Carrara Marble, `glacier`/`travertine` Warm Stone Light, `slate`/`void` Minimalist Grey-Blue Dark, and `high-contrast` WCAG AAA Dark):

```css
@theme inline {
  --color-background: var(--color-bg-base);
  --color-foreground: var(--color-text-primary);
  --color-card: var(--color-bg-surface);
  --color-card-foreground: var(--color-text-primary);
  /* Opaque surface token prevents .no-tauri headless CV captures from bleeding text */
  --color-popover: var(--color-overlay-solid);
  --color-popover-foreground: var(--color-text-primary);
  --color-primary: var(--vox-shadcn-primary);
  --color-primary-foreground: var(--vox-shadcn-primary-fg);
  --color-secondary: var(--color-bg-elevated);
  --color-secondary-foreground: var(--color-text-primary);
  --color-muted: var(--color-bg-elevated);
  --color-muted-foreground: var(--color-text-muted);
  --color-accent: var(--color-overlay-hover);
  --color-accent-foreground: var(--color-text-primary);
  --color-destructive: var(--color-status-fail);
  --color-destructive-foreground: var(--vox-shadcn-destructive-fg);
  --color-border: var(--color-border-subtle);
  --color-input: var(--color-border-strong);
  --color-ring: var(--color-accent-default);

  --z-shell-chrome: 40;
  --z-drawer: 60;
  --z-dialog: 70;
  --z-popover: 80;
  --z-toast: 90;
  --z-tooltip: 100;
}
```

Every `src/components/ui/*` root and sub-part carries `data-slot="<primitive>"` so legacy catch-all selectors in `index.css:311`, `index.css:708`, and `index.css:1076` opt out cleanly via `:not([data-slot])` and `:not([data-slot$="-overlay"]):not([class*="bg-black/"])`. Placing `--z-popover: 80` above `--z-dialog: 70` and `--z-toast: 90` above both ensures nested `Select`/`Popover` menus inside `Dialog`s and IPC failure toasts triggered from modals always paint on top.

## 3. End-to-End Tauri v2 & MCP Honesty Architecture

Per `docs/src/architecture/layers.toml`, `vox-gui` is an **L5 binary crate** (`crates/vox-gui/src/main.rs`) holding **zero persistent `VoxDb` connection pools** (`CORE-04`) so `vox-orchestrator-d` remains the sole Turso/libSQL writer, all LLM inference routes through `vox_actor_runtime::llm` (L3), and all secrets resolve exclusively via `vox_secrets` (Clavis, L1) returning redacted `SecretStatusDto` previews.

```text
[React 19 Surface / BentoCard / RomanRotaryKnob / Select (data-slot)]
    │  (optimistic pending + snapshot for rollback; onValueCommit debounce on dials)
    ▼
[voxTransport (crates/vox-gui/ui/src/transport.ts — SOLE @tauri-apps/api importer)]
    ├── !backendAvailable() ──► Reject mutating RPC with typed BackendUnavailableError
    │                           (ZERO fake _demo tokens, fake OIDs, or 500 BU fallbacks)
    ▼
[axis_invoke_handler! -> remote::gate::gate_invoke (contracts/gui/command-backends.v1.yaml)]
    ├── class: host   ──► commands::{secrets,llm_settings,mercatus} -> app.emit("vox://*-changed")
    └── class: daemon ──► BackendCalls (PersistentDaemon local TCP | GatewayClient /v1/ws)
                            └─► vox-orchestrator-d / vox-orchestrator-mcp (VoxDb + LLM L3)
```

Whenever a `#[tauri::command]` is added or updated, all four registration points must stay in lockstep (`vox ci gui-catalog-parity`):
1. Command handler in `crates/vox-gui/src/commands/<mod>.rs` (returning structured `AxisError`, never holding `State<'_, VoxDb>`).
2. `axis_invoke_handler![...]` entry in `crates/vox-gui/src/main.rs`.
3. Module classification (`daemon`, `host`, or `shell` — never `db`) in `contracts/gui/command-backends.v1.yaml`.
4. `REMOTE_ROUTED_DAEMON_MODULES` entry in `crates/vox-gui/src/remote/manifest.rs` when `class: daemon`.

## 4. `@container` Bento Compaction, Tactile Controls & Multi-Pass CV Flywheel

- **Container-Query Geometry Over Viewport Breakpoints:** `SurfaceScrollHost.tsx` establishes `@container/surface` (`container-type: inline-size`) and `<BentoCard>` establishes `@container/tile`. Grids collapse deterministically from 12 columns (`>= 768px`) to 2 columns (`>= 512px`) to 1 column (`< 512px`) based on actual allocated panel width, preventing column crush inside 320px `DockWorkspaceShell` companion panes on `1920x1080` monitors. Every grid/flex cell enforces `min-w-0 min-h-0`, and scrollable sub-regions use `<BentoScrollRegion>` (`tabIndex={0} role="region" aria-label="..."`) to satisfy `axe-core` `scrollable-region-focusable`.
- **Tactile `RomanRotaryKnob` & Clickable `RomanBrassGauge` Instruments:** Bounded ratios, temperatures, routing exploration weights, and `Oratio` VAD thresholds use `RomanRotaryKnob` (layered 2D SVG + Pointer Capture + wheel fine-step + double-click reset + finite `role="slider"` ARIA attributes committing via `onValueCommit`). Semicircular `RomanBrassGauge` header chronometers hydrate strictly from live snapshot DTOs and expose `active` filter/drawer `onClick` actions.
- **Dedicated Live Consoles for `Oratio` & `Populi`:** Retires the generic 2-card `<pre>` stdout dump (`CommandCardsView.tsx`) on `oratio` and `populi` in favor of `OratioStudioView` (live mic waveform, push-to-talk `oratio_transcribe` pad, VAD `RomanRotaryKnob`, `get_stt_config`/`set_stt_config`) and `PopuliMeshBentoView` (QUIC peer topology, Ed25519 attestation status, structured port matrix, live gossip probes).
- **CV & Accessibility Convergence Invariants:** `cv-flywheel.spec.ts` and `audits.ts` expand beyond `.include('main')` to audit `document.body` Radix portals (`[role="dialog"]`, `[role="listbox"]`, `[role="menu"]`, `[data-radix-popper-content-wrapper]`). Every surface across `900x600`..`1920x1080` and all 4 theme profiles must preserve single `<RomanSurfaceHeader>` hierarchy (`friezeHeaderCount === 1`, `flutedPillarCount >= 2` in Roman light themes), full-bleed telemetry charts (`chartPanelFillRatio >= 0.84`), uniform toolbar rhythm (`composerToolbarRowCount === 1`, `controlHeightSpreadPx <= 2`), `axeViolations === 0`, `rawEmojiCount === 0`, `iconIssueCount === 0`, `vlmOverallScore >= 0.88`, and `visualAfterScore >= 0.90`.

## 5. Dependency-Ordered Execution Sequence (`Phases 431–436`)

| Phase | Title | Core Deliverables & Verification Gate |
| --- | --- | --- |
| **Phase 431** | **Dual-Theme CSS Escape Hatches, Portal Z-Index Scale & shadcn/Radix Primitive Kit** | Install `radix-ui@^1.8.1`, `tw-animate-css@^1.4.0`, `cva@^0.7.1`, `cmdk@^1.1.1`, `react-resizable-panels@^4.22.1`, `@tanstack/react-table@^9.3.1`, `lucide-react@^1.72.0`; wire `@theme inline` & `--z-*` scale; scope `index.css:311,708,1076` with `:not([data-slot])`; neutralize `body[data-scroll-locked]`; ship `data-slot` primitives (`Select`, `Switch`, `Slider`, `RomanRotaryKnob`, `Popover`, `Dialog`, `DropdownMenu`, `Tabs`, `Sheet`, `Command`, `Resizable`, `DataTable`, `Chart`, `BentoGrid`) with green `knip` & 4-theme Playwright portal checks. |
| **Phase 432** | **Zero-Emoji & Zero-Dingbat Vector Semiotics Sweep** | Extend `Icons.tsx` (`amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`, `homeVilla`, `<PlanStepGlyph />`) + `<VoxIcon>`; purge emojis/dingbats across 50+ JSX files and Canvas `Path2D` renderers (`worldRenderer.ts`, `hydraulicNetwork.ts`); add `aria-label` + `shrink-0`; update glyph unit tests; remove `urbs-imperium-inspector` exemption in `audits.ts` + add `zeroEmojiAndDingbat.test.ts`. |
| **Phase 433** | **Rust/MCP Seam Closure, Single `voxTransport` Funnel & Honest IPC Contract** | Add `vox_guest_share_list` & align Wave 3 DTOs in `t3_wave3_tools.rs`; fix `mercatus.rs` `create_dir_all` + change event & `calls.rs` `vox_consensus_run` deadline; migrate all 38 direct-`invoke` & 8 direct-`listen` files into `voxTransport` (`ipcBoundaries.test.ts` allowlists = `0`); remove live catch-and-fake fallbacks in `t3Wave3Rpc.ts`; sync `tauriMock*` variants. |
| **Phase 434** | **End-to-End Live Control, Gauge & Interactive Telemetry Wiring** | Fix all 6 broken `viewKey`/RPC casts; wire live handlers for `HarnessRiverPanel`, `ImperiumInspector`, `LudusSandbox`, `SessionInteropModal`, `Mercatus`, and two-tier layout persistence (`getGuiPreference`/`setGuiPreference`); replace all 28 `<select>`s & 5 `<input type="range">`s; hydrate ~65 `RomanBrassGauge` dials & `ContextWindowMeter` from live DTOs; add Recharts `<Brush>` + drill-down. |
| **Phase 435** | **`@container` Concept-Art Bento Compaction & Resizable Master-Detail Rollout** | Roll out `@container/surface` `<BentoGrid>`, `<BentoScrollRegion>`, and `ResizablePanelGroup` (`>960px` split / `<960px` `Sheet`); ship `OratioStudioView` & `PopuliMeshBentoView`; compact Tier A/B surfaces (`catalog`, `harness-health`, `approvals`, `gsd`, `mercatus`, `vox-graph`, `policies`, `explorer`, `memory`, `repository`, `models`, `Omnibar`); bump `.v4` layout storage keys. |
| **Phase 436** | **Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel** | Generate dual-theme (`travertine` + `slate`) Bento concept-art plates in `public/assets/romanesque/v74-concept-*.jpg`; expand `cv-flywheel.spec.ts` & `audits.ts` to full-body + Radix portals; converge `CV_STAGE=before -> after` across 33 surfaces, 14 docks, 5 resolutions, and 4 themes (`visualAfterScore >= 0.90`, `axeViolations === 0`); pass `pnpm build`, `pnpm test`, `pnpm knip`, `vox ci gui-catalog-parity`, and `vox ci pre-push`. |
