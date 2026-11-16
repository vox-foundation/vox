# Project Research Summary

**Project:** Vox Axis Desktop IDE & Agent Control Plane (`crates/vox-gui` — Milestone v7.4 `axis-live-functional-v74`)
**Domain:** Native Multi-Agent Desktop Workbench (Tauri v2 + React 19.2 + Tailwind CSS v4.3 + Dual-Theme Romanesque / Slate Design System)
**Researched:** 2026-11-14
**Confidence:** HIGH

## Executive Summary

Vox Axis v7.3 established cohesive architectural semiotic framing (`RomanSurfaceHeader`, `PhotorealisticCanvasFx`, and multi-resolution CV Flywheel scoring across 33 `SURFACE_REGISTRY` views and 14 `DockWorkspaceShell` companion panels), yet deep codebase inspection exposes four systemic gaps between visual framing and a trustworthy, operator-grade desktop workbench: (1) **paper-code and broken controls** (~65 passive `<RomanBrassGauge>` buttons, 7 hardcoded gauge ratios, 11 client-only/fake-receipt buttons across `HarnessRiverPanel`, `ImperiumInspector`, `LudusSandbox`, and `SessionInteropModal`, 6 broken `viewKey`/RPC argument casts, 38 files bypassing `voxTransport` without `permissionMode`, and silent `catch` blocks in `t3Wave3Rpc.ts` fabricating `_demo` tokens and Git OIDs when the live daemon errors); (2) **fragmented UI primitives** (only 3 standalone `@radix-ui/react-*` packages, missing `tw-animate-css` causing dead Radix transitions, a 19-line non-portaled `Popover.tsx`, 28 raw native `<select>` menus, and 5 raw `<input type="range">` sliders); (3) **50+ files still rendering raw emojis and BMP Unicode dingbats** (`🔒`, `⚡`, `⚔`, `🌿`, `⛵`, `⚱`, `⚠️`, `✓`, `✕`, `✎`, `✂`, `☼`, `▸`, `▾`, `◆`, `⋮⋮`), masked by an explicit `[data-testid="urbs-imperium-inspector"]` exemption in `e2e/review/audits.ts`; and (4) **high-whitespace, viewport-coupled CRUD layouts** (`oratio` and `populi` rendering 2-card `<pre>` CLI dumps via `CommandCardsView`, `catalog` leaving >50% empty right-hand space, and `sm:`/`lg:` viewport breakpoints crushing grids inside narrow 320px docks).

The recommended v7.4 architecture resolves all four gaps without inventing new Rust daemon protocols (`crates/vox-gui/src/main.rs` already exposes 175+ `#[tauri::command]` endpoints). On the frontend, unify accessible primitives on `radix-ui@^1.8.1`, `tw-animate-css@^1.4.0`, `class-variance-authority@^0.7.1`, `cmdk@^1.1.1`, `react-resizable-panels@^4.22.1`, `@tanstack/react-table@^9.3.1`, and `lucide-react@^1.72.0` (`strokeWidth={1.5}` paired with bespoke Romanesque SVGs in `Icons.tsx` and tactile SVG `RomanRotaryKnob` chronometer dials), bound via Tailwind v4 `@theme inline` directly to Vox's 4-sheet Style Dictionary token pipeline (`arcane`, `glacier`/`travertine`, `slate`/`void`, `high-contrast`). Across the IPC boundary, funnel 100% of Tauri calls and listeners through `voxTransport`, close the remaining DTO gaps in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`, and strictly separate `!backendAvailable()` browser-preview states from live Tauri/daemon errors. On layout, wrap surfaces in `@container/surface` (`container-type: inline-size`) Bento grids and resizable master-detail splitters benchmarked against generated dual-theme concept-art plates.

The primary execution risks are **CSS specificity collisions**, **portal accessibility blind spots**, and **static audit ratchets**: global `!important` selectors in `crates/vox-gui/ui/src/index.css` (`:311`, `:708`, `:1076`) clobber Radix button-based `Switch`/`Checkbox`/`Select` states and turn `bg-black/60` modal scrims into solid cream `#FAF5E8` walls unless scoped with `:not([data-slot])`; `cv-flywheel.spec.ts` currently scopes Axe to `.include('main')`, missing `document.body` Radix portals that later fail full-page `capture.spec.ts`; and unedited `shadcn add` snippets immediately fail `vox-code-audit` (`transition-all`, raw `ResizeObserver`), `themeContract.test.ts`, and `knip` (`files: "error"`, `dependencies: "error"`). Executing the strict 6-phase sequence (`Phases 431–436`) neutralizes CSS, z-index, glyph, and IPC hazards before surface-wide Bento compaction and CV Flywheel sign-off.

## Key Findings

### Recommended Stack

Full details in [STACK.md](STACK.md). All versions were live-verified against the npm registry for React `19.2.8`, Tailwind CSS `v4.3`, Vite 8, and zero postinstall lifecycle scripts (preserving `crates/vox-gui/ui/pnpm-workspace.yaml` without `allowBuilds` churn).

**Core technologies:**
- **`radix-ui@^1.8.1` (replaces 3 standalone `@radix-ui/react-*` deps):** Official unified shadcn Tailwind v4 (`new-york-v4`) monorepo (`Dialog`, `Popover`, `Select`, `DropdownMenu`, `Tabs`, `Switch`, `Slider`, `Checkbox`, `RadioGroup`, `Tooltip`, `ScrollArea`, `Collapsible`, `Accordion`, `AlertDialog`, `Slot`) — 100% tree-shakeable (`"sideEffects": false`), native React 19 `ref`-as-prop support, zero `@radix-ui/react-compose-refs` version skew, and single-entry `knip` compliance.
- **`tailwindcss@^4.3.5` + `@tailwindcss/postcss@^4.3.5` + `tw-animate-css@^1.4.0`:** Pure CSS animation utilities activating currently dead `animate-in`/`fade-in-0`/`zoom-in-95` classes in `Dialog.tsx` and `Tooltip.tsx`, plus `@theme inline` binding Style Dictionary tokens across `document.body` portals and nested `[data-theme]` previews.
- **`class-variance-authority@^0.7.1` (`cva`):** Strongly-typed `variant` / `size` / `bentoSpan` matrices with mandatory `data-slot="<primitive>"` hooks opting out of legacy `index.css` `!important` overrides.
- **`lucide-react@^1.72.0` + bespoke `src/components/ui/Icons.tsx`:** 100% offline two-tier vector icon system (standard UI chrome normalized to `24×24`, `strokeWidth={1.5}` via `<VoxIcon>`, plus custom Romanesque domain icons including `amphoraUrn`, `solInvictus`, `triremeShip`, and `crystalGem`).
- **`cmdk@^1.1.1`, `react-resizable-panels@^4.22.1`, & `@tanstack/react-table@^9.3.1`:** WAI-ARIA command palette/combobox backing `Omnibar.tsx` and searchable model/crate pickers; v4 `<Group>`/`<Panel>`/`<Separator>` split panes for 2/3-pane master-detail surfaces (retaining `dockview@6.6.1` exclusively for multi-tab docking in `DockWorkspaceShell.tsx`); and v9 `useTable`/`tableFeatures()` headless sorting/filtering composed with `@tanstack/react-virtual@^3.14.10`.
- **Retained investments:** `recharts@^3.8.1` wrapped in shadcn v3 `ChartContainer` (`height="100%"` for `chartPanelFillRatio >= 0.84`), `@dnd-kit/*` (`DashboardGrid.tsx`), `Toasts.tsx` (preserving Vox's `ToastCause` & `groupKey` coalescing contract rather than replacing with `sonner`), and `style-dictionary@^5.5.2`.

### Expected Features

Full codebase audit and inventory in [FEATURES.md](FEATURES.md).

**Must have (table stakes):**
- **100% Live Control-to-Backend Wiring (Zero Paper Code):** Fix all 6 broken `viewKey` routes and `vcsCheckpointCapture`/`Restore` type-cast bugs (`HudPanels.tsx`, `HarnessRiverPanel.tsx`, `ImperiumInspector.tsx`), and wire all 11 local-only controls (`Compact Context`, `Claim Yield`, `Sluice Gates I–IV`, `Verification Gate`, `timeWindow`, `PRAETORIVM Approve`, `FORVM modelSlider`, `Ludus Pin Context` -> `set_session_context_pins`, `Ludus Yield`, `SessionInteropModal` live guest moderation, and `mercatus_save_config`).
- **Honest Backend Error Propagation (`t3Wave3Rpc.ts` & `voxTransport`):** Eliminate silent `catch` blocks that return fabricated `_demo` tokens, fake Git OIDs, or `500 BU` balances when `backendAvailable() === true`; route all 38 direct-`invoke` and 8 direct-`listen` bypass files through `voxTransport` with `permissionMode` intact.
- **Zero Raw Emojis & Zero Unicode Dingbats:** Purge all pictographic emojis and BMP dingbats/arrows/shapes across 50+ JSX files and 2 `<canvas>` renderers (`worldRenderer.ts`, `hydraulicNetwork.ts`), removing the `urbs-imperium-inspector` audit exemption.
- **Accessible Styled Primitives & Empirical Telemetry:** Replace all 28 native `<select>` elements and 5 `<input type="range">` sliders; hydrate all ~65 `RomanBrassGauge` dials and `ContextWindowMeter` from real snapshot DTOs instead of hardcoded `0.78` / `0.85` / `92%` constants.
- **Daylight-Legible Dense Bento Layouts (WCAG AA >= 4.5:1):** Eliminate pale-gold-on-parchment leaks (`#F3E5AB`), dark-zinc islands (`bg-zinc-900`/`bg-zinc-950`) in light mode, and >50% empty right-hand whitespace gutters.

**Should have (competitive differentiators):**
- **Tactile SVG `RomanRotaryKnob` Chronometer Dials:** Custom 2D SVG + Pointer Capture + `role="slider"` ARIA detent knobs for bounded parameters (LLM temperature, routing exploration weight, `ImperiumInspector` `modelSlider`, `Oratio` VAD sensitivity, `Loquela` cadence, `VoxGraph` TTL detents).
- **Interactive `RomanBrassGauge` Filters & Recharts `<Brush>` Drill-Down:** One-click header gauge filtering/drawer toggles across 25+ surfaces plus formatted Recharts axes, `<ReferenceLine>` budget caps, `<Brush>` time scrubbing, and bar/point click drill-down.
- **Dedicated `OratioStudioView` & `PopuliMeshBentoView` Consoles:** Retire generic 2-card `<pre>` CLI dumps (`CommandCardsView`) on `oratio` and `populi` in favor of purpose-built live Bento consoles (`oratio_transcribe` push-to-talk + mic meter; QUIC peer topology + gossip probes).
- **Concept-Art + Dual-Theme CV Flywheel Gates:** Benchmark refactored Bento surfaces against generated Romanesque/Slate concept-art plates with automated Playwright geometry, contrast, and VLM rubric assertions.

**Defer (post-v7.4):**
- **New Rust Backend Subsystems or Daemon Protocols:** Existing `vox-gui` + `vox-orchestrator-mcp` surfaces already cover v7.4's control needs (save for targeted DTO alignment and `vox_guest_share_list` in `t3_wave3_tools.rs`).
- **3D WebGL Skeuomorphic Shaders or Nested `dockview` Tiling Outside Chat:** Avoid >600 KB bundle bloat, GPU contention with `DiegeticFxStage`, and tab-bar inception on 2-pane CRUD views.

### Architecture Approach

Full architectural blueprint in [ARCHITECTURE.md](ARCHITECTURE.md). `vox-gui` remains a strict **Layer L5 binary crate** (`crates/vox-gui/src/main.rs`, zero `lib.rs`, and **zero persistent `VoxDb` handles** per `CORE-04` so `vox-orchestrator-d` remains the sole Turso/libSQL writer). Stateful additions execute in `crates/vox-orchestrator-mcp` (`t3_wave3_tools.rs`) and traverse the 4-point command registration contract (`commands/<mod>.rs` -> `axis_invoke_handler!` -> `contracts/gui/command-backends.v1.yaml` -> `remote/manifest.rs`), while all LLM inference remains behind `vox_actor_runtime::llm` (L3) and all credentials remain redacted behind `vox_secrets` (Clavis, L1).

**Major components:**
1. **`@container` Surface, Bento & Split Layout Layer (`SurfaceScrollHost.tsx`, `BentoGrid.tsx`, `SplitWorkbench.tsx` / `Resizable.tsx`):** Establishes `container-type: inline-size` (`@container/surface`, `@container/tile`) so 33 surfaces, 14 Chat docks, and `DashboardGrid` tiles adapt to panel width rather than `window.innerWidth`, backed by two-tier layout persistence (`localStorage` L1 + `getGuiPreference`/`setGuiPreference` L2).
2. **Dual-Theme `data-slot` shadcn/Radix Primitive Layer (`crates/vox-gui/ui/src/components/ui/*` + `Icons.tsx`):** Accessible React 19 primitives bound via `@theme inline` to Style Dictionary's 4 generated CSS token sheets (`--color-popover: var(--color-overlay-solid)` for `.no-tauri` opaque CV captures), preserving `<RomanSurfaceHeader>` Cinzel friezes in `arcane`/`travertine` while rendering clean `Inter` basalt panels in `slate`/`high-contrast`.
3. **Single Frontend IPC Funnel (`crates/vox-gui/ui/src/transport.ts` & `transport/t3Wave3Rpc.ts`):** Sole legal importer of `@tauri-apps/api/{core,event}`, automatically injecting `permissionMode` on `invokeMcpTool`, rejecting offline mutations with typed `BackendUnavailableError`, and surfacing live daemon errors via `sanitizeErrorForToast`.
4. **Tauri v2 Gate & Rust MCP Spine (`crates/vox-gui/src/remote/{gate,calls,manifest}.rs` + `crates/vox-orchestrator-mcp`):** Enforces fail-closed remote capability routing (`daemon | host | shell`; `0` `db`), extends `deadline_for_tool` for `vox_consensus_run`, and aligns Wave 3 guest-share/session DTOs.
5. **Concept-Art Dual-Theme CV Flywheel (`e2e/review/vlmVisionFlywheel.ts` & `cv-flywheel.spec.ts`):** Scores `CV_STAGE=before` vs `CV_STAGE=after` captures across `900×600`–`1920×1080` and all 4 theme profiles against reference concept-art plates, full-document + portal `axe-core`, and DOM geometry invariants.

### Critical Pitfalls

Full 10-pitfall catalog and recovery playbooks in [PITFALLS.md](PITFALLS.md).

1. **Global `index.css` `!important` Selectors Clobbering Radix Backdrops, Switches, Checkboxes & Portals** — Scope `index.css:311` and `:1076` button rules with `:not([data-slot])` and ARIA form roles (`switch`, `checkbox`, `radio`, `combobox`, `menuitem`, `option`); exclude `[data-slot$="-overlay"]` and `[class*="bg-black/"]` from `index.css:708-742` so modal backdrops do not turn into solid `#FAF5E8` cream walls; extend portal theme rules to `[role="listbox"]`, `[role="alertdialog"]`, and `[data-radix-popper-content-wrapper]`.
2. **Silent Fake-Optimistic Success on Live IPC/Daemon Errors (`t3Wave3Rpc.ts` & `.catch(() => {})`)** — Never wrap `if (backendAvailable())` RPC calls in `catch { /* fall through to mock */ }`; gate preview stubs strictly behind `if (!backendAvailable())`, snapshot-rollback optimistic UI state on rejection, and route structured Tauri v2 `{ kind, message }` errors through `sanitizeErrorForToast(err)` (satisfying `sentryHonesty.ts` and `toastBodyGuard.test.ts`).
3. **Z-Index Stacking Wars, `body[data-scroll-locked]` `outerOverflowPx` Blowouts & `dockview` Pointer Lockups** — Replace colliding `z-50` / `z-[99999]` / `z-40` classes with a single `@theme` z-index scale (`--z-shell-chrome: 40`, `--z-drawer: 60`, `--z-dialog: 70`, `--z-popover: 80`, `--z-toast: 90`, `--z-tooltip: 100`), neutralize `html body[data-scroll-locked]` margin-right shifts, switch `AppShell.tsx` `w-screen` -> `w-full`, pass `modal={false}` on non-blocking toolbar menus, and dedupe on unified `radix-ui@^1.8.1`.
4. **The `<main>` Axe Blind Spot + `scrollable-region-focusable` & ARIA Slider/Knob Violations** — Expand `cv-flywheel.spec.ts:143` and `audits.ts` beyond `.include('main')` to audit `document.body` Radix portals (matching `capture.spec.ts`), wrap scrollable Bento panes in `<BentoScrollRegion>` (`tabIndex={0} role="region" aria-label="..."`), forward required `ariaLabel` props directly onto `<SliderPrimitive.Thumb>`, and clamp finite `aria-valuenow` values on `RomanRotaryKnob`.
5. **`vox-code-audit` (`toestub`), `themeContract`, `knip` & Bento CV Threshold Regressions** — Never run raw `npx shadcn@latest init/add` unedited: ban `transition-all` (`vox/ui/layout-and-perf-hazards`), coalesce `ResizeObserver` callbacks inside `requestAnimationFrame`, consume every new `src/components/ui/*.tsx` file in the same slice (`knip` `files: "error"`), keep `< 500` LoC focused modules (`arch/god_object`), enforce `min-w-0 min-h-0` across Bento/split cells, preserve `.vox-marble-tablet` + single `<RomanSurfaceHeader>` (`flutedPillarCount >= 2`, `friezeHeaderCount === 1`), and render full-bleed KPI sparklines (`chartPanelFillRatio >= 0.84`).

## Implications for Roadmap

Based on empirical dependency constraints across CSS specificity, SVG semiotics, IPC honesty, and CV scoring, Milestone v7.4 should execute in **6 sequential phases (`Phases 431–436`)**:

### Phase 431: Dual-Theme CSS Escape Hatches, Portal Z-Index Scale & shadcn/Radix Primitive Kit
**Rationale:** Every surface migration breaks visually or structurally until `index.css` `!important` selectors stop clobbering Radix button-role controls/scrims, `tw-animate-css` activates animations, and a unified z-index/portal contract is in place.
**Delivers:** Installed `radix-ui@^1.8.1`, `cva@^0.7.1`, `cmdk@^1.1.1`, `react-resizable-panels@^4.22.1`, `@tanstack/react-table@^9.3.1`, `lucide-react@^1.72.0`, `tw-animate-css@^1.4.0`, and `tailwindcss@^4.3.5` (removing legacy `@radix-ui/react-{dialog,slot,tooltip}`); `@theme inline` 4-theme Style Dictionary bindings (`arcane`, `travertine`, `slate`, `high-contrast`); `--z-*` ladder (`Toasts` at `90` above `Dialog` `70` and `Popover` `80`); `:not([data-slot])` and overlay scrim guards in `index.css`; `body[data-scroll-locked]` overflow lock; single `AppShell` `<TooltipProvider>`; and Vox-native `data-slot` primitives (`Button`, `Dialog`, `Popover`, `Select`, `DropdownMenu`, `Tabs`, `Switch`, `Slider`, `RomanRotaryKnob`, `Checkbox`, `RadioGroup`, `Sheet`, `Command`, `Resizable`, `DataTable`, `Chart`, `Badge`, `Card`) wired to initial pilot call sites so `knip` stays green.
**Addresses:** Accessible styled primitives, `RomanRotaryKnob`, 4-theme WCAG AA contrast pairs (`7.4:1` `arcane`, `9.0:1` `slate`).
**Avoids:** Pitfall 1 (`index.css` clobbering), Pitfall 3 (z-index & scroll-lock `outerOverflowPx`), Pitfall 4 (primitive-level `DialogTitle` & `Slider.Thumb` `ariaLabel`), Pitfall 5 (`transition-all` & `knip` unused-file errors).

### Phase 432: Zero-Emoji & Zero-Dingbat Vector Semiotics Sweep
**Rationale:** Purging emojis and BMP dingbats across shared UI primitives (`DataTable`, `Kpi`, `StatusPill`, `CapabilityBadge`, `DashboardGrid`, `Toasts`) and 50+ surface/canvas files before layout compaction prevents glyph baseline jitter, `button-name` Axe failures, and `zero-size-svg` (`iconIssueCount`) surprises.
**Delivers:** Expanded `Icons.tsx` (`amphoraUrn`, `solInvictus`, `triremeShip`, `crystalGem`, `homeVilla`, `<PlanStepGlyph />`) + `<VoxIcon strokeWidth={1.5} />` (`lucide-react`); 100% vector replacement across all 50+ JSX files and Canvas `Path2D` helpers (`worldRenderer.ts`, `hydraulicNetwork.ts`); `aria-label` + `shrink-0` on all icon-only triggers; updated unit tests (`Kpi.test.tsx`, `CapabilityBadge.test.tsx`, `ExplorerView.test.tsx`, `SecretaryToast.test.tsx`, `TurnTrace.test.tsx`); removal of the `urbs-imperium-inspector` exemption in `e2e/review/audits.ts`; and fast Vitest `zeroEmojiAndDingbat.test.ts` source guard.
**Uses:** `lucide-react@^1.72.0`, `crates/vox-gui/ui/src/components/ui/Icons.tsx`, Canvas 2D `Path2D`.
**Implements:** Two-Tier Offline Vector Semiotics & CI Glyph Gate.
**Avoids:** Pitfall 6 (incomplete BMP dingbat purge, `button-name` critical Axe errors, `zero-size-svg`, broken glyph unit tests).

### Phase 433: Rust/MCP Seam Closure, Single `voxTransport` Funnel & Honest IPC Contract
**Rationale:** Live UI controls cannot be trusted until backend DTO mismatches are fixed, all 38 direct-`invoke` / 8 direct-`listen` bypasses route through `voxTransport` (propagating `permissionMode`), and `t3Wave3Rpc.ts` stops masking real daemon errors with `_demo` payloads.
**Delivers:** `vox_guest_share_list` + aligned `guest_share_mint`, `session_list_external`, and `muse_pool_adopt` DTOs in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` (removing synthetic moderation fabrication); `fs::create_dir_all` + `vox://mercatus-config-changed` in `crates/vox-gui/src/commands/mercatus.rs`; `vox_consensus_run` extended deadline in `crates/vox-gui/src/remote/calls.rs`; migration of all 38 direct-`invoke` and 8 direct-`listen` files into `voxTransport` (`ipcBoundaries.test.ts` allowlists locked to `new Set()`); elimination of all catch-and-fake fallbacks when `backendAvailable() === true` in `t3Wave3Rpc.ts`; `noPaperFallbackGuard.test.ts` + expanded `honestyScan.ts`; and synchronized `tauriMock.ts` / `tauriMockRich.ts` / `tauriMockVariants.ts`.
**Uses:** `voxTransport`, `BackendCalls`, `vox-orchestrator-mcp`, `contracts/gui/command-backends.v1.yaml`.
**Implements:** Layer C (`voxTransport` Single IPC Boundary) & Layer D/E DTO Parity.
**Avoids:** Pitfall 2 (silent fake-optimistic IPC swallows & `sentryHonesty.ts` failures), Pitfall 9 (`tauriMock*` desync `skeletonCount > 0`).

### Phase 434: End-to-End Live Control, Gauge & Interactive Telemetry Wiring
**Rationale:** With clean primitives (Phase 431) and honest IPC plumbing (Phase 433), every dead button, broken navigation target, passive gauge, native `<select>`, and native `<input type="range">` can be wired to real Rust state with snapshot-rollback and Reload persistence.
**Delivers:** Fixes for all 6 broken `viewKey` routes & `vcsCheckpointCapture`/`Restore` casts (`HudPanels`, `HarnessRiverPanel`, `ImperiumInspector`); live backend wiring for `HarnessRiverPanel` (time-window cutoff, `Verification Gate`, `Sluice Gates I–IV`, `Compact Context`, `Claim Yield`, live `Ludus` energy), `ImperiumInspector` (`PRAETORIVM Approve`, `FORVM` `RomanRotaryKnob` routing slider), `LudusSandbox` (`Pin Context` -> `set_session_context_pins`, `Yield`), `SessionInteropModal` (live guest list & file picker), `Mercatus` (`mercatusSaveConfig` switches/thresholds/dialog), and `ChatExecutionRail` (`axisBackendStatus`); replacement of all 28 native `<select>`s (`Select` / `Command` combobox) and 5 `<input type="range">`s (`RomanRotaryKnob` / `Slider` with `onValueCommit` debounce); empirical DTO hydration + `onClick` filters across all ~65 `RomanBrassGauge` dials and `ContextWindowMeter`; interactive Recharts frames (`<Brush>`, formatted axes, `<ReferenceLine>`, click drill-down); and two-tier layout persistence (`DashboardGrid` & `DockWorkspaceShell` synced via `getGuiPreference`/`setGuiPreference`).
**Addresses:** 100% live controls, empirical header gauges/context meters, interactive Recharts telemetry, tactile rotary detents.
**Avoids:** Pitfall 2 (fire-and-forget `.catch(() => {})`), Performance Trap 4 (60Hz slider IPC spam via `onValueCommit`).

### Phase 435: `@container` Concept-Art Bento Compaction & Resizable Master-Detail Rollout
**Rationale:** Compacting layouts after all live controls and vector icons are in place guarantees accurate intrinsic sizing across full-screen (`1920×1080`), compact (`900×600`), and narrow 320px docked viewports.
**Delivers:** `@container/surface` (`SurfaceScrollHost.tsx`), `<BentoGrid>` / `<BentoCard>` (`.vox-marble-tablet` in Roman light themes, basalt card in `slate`, full-bleed `w-full` KPI sparklines), `<BentoScrollRegion>` (`tabIndex={0}` + `role="region"`), and `ResizablePanelGroup` (`>960px` horizontal split, `<960px` `Sheet` drawer); dedicated **`OratioStudioView`** and **`PopuliMeshBentoView`** retiring generic `CommandCardsView` stdout dumps; structural Bento & resizable upgrades across Tier A (`catalog`, `harness-health`, `approvals`, `gsd`, `mercatus`, `vox-graph`), Tier B (`policies`, `explorer`, `memory`, `repository`, `publications`, `scientia`, `models`, `coderabbit`, `runs`, `tasks`, `sub-agents`, `skills`, `activity`, `needs-you`, `settings`, `mesh`, `mens-runs`, `research`), and `cmdk`-backed `Omnibar.tsx`; and `.v4` bumped layout storage keys with schema validation.
**Uses:** `react-resizable-panels@^4.22.1`, `@tanstack/react-table@^9.3.1`, `cmdk@^1.1.1`, Tailwind v4 `@container`.
**Implements:** Layer A (`@container` Bento & Split Workbenches).
**Avoids:** Pitfall 7 (`0×0` height collapse, `min-w-0` overflow, stale `localStorage` layout poisoning), Pitfall 8 (`chartPanelFillRatio < 0.84`, missing/duplicate `<RomanSurfaceHeader>`).

### Phase 436: Concept-Art Plates, Full-Document Accessibility, Static Audit & Multi-Pass CV Flywheel Convergence
**Rationale:** Final sign-off must close the `<main>`-only Axe/audit blind spot on `document.body` portals and prove mathematical convergence across all 33 surfaces, 14 docks, 5 viewport classes, and 4 theme profiles alongside `vox ci pre-push`.
**Delivers:** Dual-theme (`travertine`/`arcane` + `slate`) Bento concept-art reference plates (`crates/vox-gui/ui/public/assets/romanesque/v74-concept-*.jpg`) registered in `vlmVisionFlywheel.ts`; upgraded full-document + portal `AxeBuilder` and contrast/glyph selectors in `cv-flywheel.spec.ts` and `audits.ts`; multi-pass `CV_STAGE=before` -> `CV_STAGE=after` refinement across `900×600`..`1920×1080` achieving `axeViolations === 0`, `rawEmojiCount === 0`, `iconIssueCount === 0`, `outerOverflowPx === 0`, `scrollHostOverflowPx === 0`, `clippedInteractiveCount === 0`, `composerToolbarRowCount === 1`, `chartPanelFillRatio >= 0.84`, `flutedPillarCount >= 2` (Roman themes), `friezeHeaderCount === 1`, `vlmOverallScore >= 0.88`, and `visualAfterScore >= 0.90`; and 100% green `pnpm build`, `pnpm test`, `pnpm knip`, `vox ci gui-catalog-parity`, and `vox ci pre-push` (plus synced screenshots/tours per multi-repo doc discipline).
**Addresses:** Concept-Art CV Flywheel gate, WCAG 2.2 AA portal parity, zero static/dead-code findings.
**Avoids:** Pitfall 4 (`<main>` Axe portal blind spot), Pitfall 5 (`toestub`/`knip`/`hardcoded-abs-dir`), Pitfall 8 & Pitfall 10 (CV rubric drift & exit-animation ghost portals).

### Phase Ordering Rationale

- **CSS & Portals (431) before Glyphs (432) and Controls (434):** Mounting Radix `Select`, `Switch`, or `Dialog` before scoping `index.css:311,708,1076` produces broken cream backdrops and invisible checked states, wasting debugging cycles on false component bugs.
- **Glyph Semiotics (432) before Bento Compaction (435):** Replacing variable-width system emojis/dingbats with deterministic `24×24` / `16×16` `shrink-0` SVGs stabilizes button intrinsics and unit tests before tightening Bento toolbar heights (`controlHeightSpreadPx <= 2`).
- **Rust DTOs & Honest `voxTransport` (433) before Surface Control Wiring (434):** Stripping `_demo` catch blocks first exposes any wire-schema mismatch immediately during Phase 434 wiring instead of hiding behind fake optimistic receipts.
- **Live Controls (434) before Bento Layout (435) before CV Flywheel Sign-Off (436):** Bento grid cell spans (`OratioStudioView`, `PopuliMeshBentoView`, `Catalog`, `Mercatus`) must wrap the *actual* interactive controls (`RomanRotaryKnob`, Recharts `<Brush>`, `DataTable`), and VLM/CV histogram scoring must run once final geometry and theme tokens settle.

### Research Flags

Phases needing targeted design/contract verification during phase planning:
- **Phase 433 (Rust/MCP Seam Closure):** Inspect exact `guest_share_service` state locking in `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` when adding `vox_guest_share_list` so local `PersistentDaemon` and remote `GatewayClient` serialize identical `GuestSubmissionDto` arrays.
- **Phase 435 (`OratioStudioView` & `PopuliMeshBentoView`):** Verify browser/headless fallback states for `start_mic_capture` (`crates/vox-gui/src/commands/mic.rs`) and `execute_command(['populi', 'status'])` in `tauriMockRich.ts` so `OratioStudioView` and `PopuliMeshBentoView` render rich deterministic telemetry without `skeletonCount > 0` in Playwright CI.

Phases with fully established codebase patterns (skip extra research-phase):
- **Phase 431 (CSS & shadcn/Radix Primitives):** Exact `@theme inline` token map, `:not([data-slot])` selectors, z-index ladder, and package versions (`radix-ui@^1.8.1`, `tw-animate-css@^1.4.0`) are completely specified in `STACK.md` and `PITFALLS.md`.
- **Phase 432 (Zero-Emoji & Vector Semiotics):** Complete file-and-line ledger of all 50+ JSX files, 2 Canvas renderers, and 5 glyph-asserting Vitest files is already cataloged in `FEATURES.md` and `PITFALLS.md`.
- **Phase 434 (Live Control & Gauge Wiring):** Every unwired button, broken `viewKey`, magic ratio, native `<select>`, and `<input type="range">` has its exact file, line number, and target Tauri/MCP command mapped in `FEATURES.md`.
- **Phase 436 (CV Flywheel Convergence):** Existing `cv-flywheel.spec.ts`, `audits.ts`, and `vlmVisionFlywheel.ts` harnesses and numeric thresholds are live-audited and ready for extension.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | **HIGH** | Every package version (`radix-ui@1.8.1`, `tw-animate-css@1.4.0`, `cmdk@1.1.1`, `react-resizable-panels@4.22.1`, `@tanstack/react-table@9.3.1`, `lucide-react@1.72.0`, `cva@0.7.1`, `tailwindcss@4.3.5`) verified live via `pnpm view` for React 19.2 peer compatibility and zero postinstall scripts. |
| Features | **HIGH** | Grounded in exhaustive line-by-line source audit of all 33 `SURFACE_REGISTRY` views, 14 docks, `main.rs` (175+ Tauri commands), `t3Wave3Rpc.ts`, and `e2e/review/audits.ts`. |
| Architecture | **HIGH** | Directly verified against `docs/src/architecture/layers.toml` (`vox-gui` L5 binary, `0` `VoxDb` handles), `contracts/gui/command-backends.v1.yaml`, `remote/gate.rs`, `ipcBoundaries.test.ts`, and `AGENTS.md` Clavis/LLM boundaries. |
| Pitfalls | **HIGH** | Every CSS specificity clash (`index.css:311,708,1076`), Axe `.include('main')` blind spot (`cv-flywheel.spec.ts:143`), `toestub` rule (`ui_perf_hazards.rs:78`), and CV threshold (`cvFlywheel.ts`) verified directly against live enforcement code. |

**Overall confidence:** **HIGH**

### Gaps to Address

- **`ContextBudgetDto` Per-Slice Token Telemetry Availability:** `ContextWindowMeter.tsx:62-66` currently fabricates `0.18 / 0.32 / 0.28` slice multipliers and `92% / 74% / 86%` KV cache hits. During Phase 434 planning, check whether `crates/vox-gui/src/commands/context_budget.rs` (`get_context_budget`) already serializes per-category token counts; if optional/absent on certain providers, render honest aggregate occupancy (`used / max`) and omit the synthetic sub-slice bar rather than guessing ratios.
- **Keyboard Legend `<kbd>` Symbols vs. UI Chrome Dingbats:** While purging arrows/symbols in Phase 432, keep semantic `<kbd>` keycap legends (`⌘`, `⇧`, `↵` inside `<kbd>` elements or `<KeycapIcon />` SVGs) explicitly scoped in `zeroEmojiAndDingbat.test.ts` and `audits.ts` so legitimate shortcut hints are distinguished from decorative button/table dingbats.
- **Incremental `knip` Compliance Across Phases 431–435:** Because `crates/vox-gui/ui/knip.jsonc` enforces `"files": "error"` and `"dependencies": "error"`, Phase 431 plans must wire at least one real consumer per new primitive (or export through an active barrel consumed by `AppShell`/pilot views) in the same commit that adds the file and npm package.

## Sources

### Primary (HIGH confidence — Live Codebase & Registry Inspection)
- `crates/vox-gui/src/main.rs`, `crates/vox-gui/src/remote/{gate,manifest,calls}.rs`, `crates/vox-gui/src/commands/*.rs`, & `contracts/gui/command-backends.v1.yaml` — 175+ Tauri commands, 4-point registration contract, and fail-closed `0`-`db` remote routing.
- `crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs` & `crates/vox-gui/ui/src/transport/{t3Wave3Rpc.ts}` — Wave 3 guest-share/checkpoint/consensus RPC wire schemas and catch-and-fake fallback audit.
- `crates/vox-gui/ui/src/index.css`, `crates/vox-gui/ui/style-dictionary.config.mjs`, `crates/vox-gui/ui/tokens/*.json`, & `contracts/gui/theme-contract.v1.json` — 4-profile token pipeline and legacy `!important` override selectors.
- `crates/vox-gui/ui/e2e/review/{audits.ts,cv-flywheel.spec.ts,vlmVisionFlywheel.ts,capture.spec.ts}` & `crates/vox-gui/ui/src/lib/cvFlywheel.ts` — CV/VLM scoring equations, `.include('main')` Axe scope, and `urbs-imperium-inspector` emoji exemption.
- `crates/vox-code-audit/src/detectors/{ui_perf_hazards.rs,ui_theme_contract.rs,hardcoded_abs_dir.rs,god_object.rs,test_clock_and_skip.rs,unreasoned_suppression.rs}` & `crates/vox-gui/ui/knip.jsonc` — Static CI gates enforced by `vox ci pre-push`.
- Live npm registry metadata (`pnpm view`, verified 2026-11-14): `radix-ui@1.8.1`, `tw-animate-css@1.4.0`, `class-variance-authority@0.7.1`, `cmdk@1.1.1`, `react-resizable-panels@4.22.1`, `@tanstack/react-table@9.3.1`, `lucide-react@1.72.0`, `tailwindcss@4.3.5`.

### Secondary (HIGH confidence — Ecosystem Specifications)
- Official shadcn/ui Tailwind v4 (`@theme inline`) & React 19 (`data-slot`, `ref`-as-prop, unified `radix-ui`, Recharts v3 `ChartContainer`) architecture guides.
- `react-resizable-panels` v4 (`Group` / `Panel` / `Separator`, `orientation`) and `@tanstack/react-table` v9 (`useTable` / `tableFeatures`) migration specifications.
- W3C WAI-ARIA 1.2 & Deque `axe-core` 4.x specifications (`scrollable-region-focusable`, `aria-input-field-name`, `aria-dialog-name`, `button-name`, `nested-interactive`).

---
*Research completed: 2026-11-14*
*Ready for roadmap: yes*
