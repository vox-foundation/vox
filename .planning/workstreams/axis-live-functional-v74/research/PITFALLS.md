# Pitfalls Research: Vox Axis Milestone v7.4

**Domain:** Desktop Orchestration GUI Upgrade — Live Tauri v2 / Daemon Control Wiring, Radix UI + shadcn/Tailwind v4 Portals under Romanesque CSS Overrides, Zero-Glyph SVG Migration, Dense Bento / Resizable Layouts & Multi-Pass Playwright CV + Axe Flywheel (`crates/vox-gui` & `crates/vox-gui/ui`)
**Researched:** 2026-11-14
**Confidence:** HIGH (grounded in live inspection of `crates/vox-gui/`, `crates/vox-code-audit/`, `contracts/gui/`, and verified npm package versions)

---

## Executive Summary & Proposed v7.4 Phase Anchors

Upgrading Vox Axis in v7.4 touches five tightly coupled enforcement planes simultaneously:
1. **Global CSS & Theme Contract (`index.css`, `themeContract.ts`, `vox/ui/theme-contract-drift`)** — Aggressive `!important` attribute-substring and element-tag selectors across 4 themes (`arcane`, `slate`/`void`, `glacier`/`travertine`, `high-contrast`) actively clobber unguarded Radix/shadcn DOM primitives and backdrops.
2. **Tauri IPC & Honesty Invariants (`t3Wave3Rpc.ts`, `sentryHonesty.ts`, `ipcBoundaries.test.ts`, `toastBodyGuard.test.ts`, `command-backends.v1.yaml`)** — Existing RPC wrappers conflate standalone browser-preview fallbacks with live Tauri/daemon errors, producing silent fake-optimistic state.
3. **Static Audit & Dead-Code Ratchets (`vox-code-audit` / `toestub`, `vox-drift-check`, `knip`)** — Upstream shadcn/ui copy-paste code immediately trips `vox/ui/layout-and-perf-hazards`, `vox/ui/theme-contract-drift`, `arch/god_object`, and `knip` unused-file/dependency gates.
4. **Accessibility Gates (`@axe-core/playwright@^4.13.0`)** — `cv-flywheel.spec.ts` currently scopes Axe to `.include('main')`, creating a blind spot for `document.body` portals that blows up in full-page `capture.spec.ts` runs, while dense Bento scroll panes and custom sliders/knobs trip `scrollable-region-focusable` and `aria-input-field-name`.
5. **Computer-Vision & VLM Rubric Thresholds (`cvFlywheel.ts`, `audits.ts`, `vlmVisionFlywheel.ts`)** — Compacting whitespace into Bento grids alters `viewportCoverageRatio`, `chartPanelFillRatio`, `controlHeightSpreadPx`, `flutedPillarCount`, and `romanFontCoverageRatio`, which hard-fail `cv-flywheel.spec.ts` if pushed outside calibrated bands.

Throughout this document, prevention ownership is mapped to the recommended 6-phase v7.4 structure:
- **Phase 1 (CSS & Portal Foundation):** Unified `radix-ui@1.8.1` + Tailwind v4 `@theme` z-index scale + `index.css` `data-slot` scoping across all 4 themes.
- **Phase 2 (Zero-Glyph & Icon Semiotics):** Full Unicode dingbat/emoji/geometric-glyph purge, engraved Roman/Lucide SVG replacement, `aria-label` + unit-test migration.
- **Phase 3 (Honest IPC & Live Control Wiring):** Eliminating fake-optimistic `catch` fallthroughs, wiring all dead/silent controls end-to-end via `voxTransport` + daemon RPCs + `tauriMock` parity.
- **Phase 4 (Dense Bento & Resizable Workbench Layouts):** `react-resizable-panels@4.22.1` + `dockview@6.6.1` definite-pixel sizing, `min-w-0 min-h-0` containment, `@container`-responsive KPIs/charts.
- **Phase 5 (Accessibility & Static Audit Hardening):** Full-body + portal Axe coverage, ARIA slider/spinbutton/knob compliance, `toestub` + `knip` + `themeContract` zero-finding sweep.
- **Phase 6 (Multi-Pass CV & VLM Flywheel Convergence):** `900×600`..`1920×1080` × 4-theme × 33-surface + 14-dock `BEFORE -> AFTER` Playwright CV Flywheel convergence.

---

## Critical Pitfalls

### Pitfall 1: Global `index.css` `!important` Selectors Clobber Radix/shadcn Backdrops, Switches, Checkboxes, Selects, and Unscoped Portals

**Severity:** CRITICAL

**What goes wrong:**
Introducing standard Radix UI / shadcn primitives (`DialogOverlay`, `SheetOverlay`, `Switch`, `Checkbox`, `RadioGroupItem`, `Select`, `DropdownMenu`, `HoverCard`, `Command`) causes catastrophic visual breakage in both `arcane`/`glacier` (White/Gold Carrara Marble) and `slate`/`void` (Minimalist Dark) themes:
1. **Opaque Carrara Marble Backdrop Wall:** `crates/vox-gui/ui/src/index.css:708-742` targets `:is(body, ...) [class*="bg-black"]` in light themes and forces `background-color: #FAF5E8 !important` + an opaque `0.96`-alpha marble `linear-gradient(...) !important`. Because `bg-black/60` and `bg-black/80` contain the substring `"bg-black"`, every modal overlay (`Dialog.tsx:40`, `Omnibar.tsx:618`, `ResearchEngineDrawer.tsx:239`, `SheetOverlay`, `AlertDialogOverlay`) renders as a solid cream wall that completely hides the underlying workspace instead of dimming it.
2. **Broken `Switch`, `Checkbox`, and `RadioGroupItem` States:** Radix renders switches (`<button role="switch">`), checkboxes (`<button role="checkbox">`), and radios (`<button role="radio">`) as `<button>` elements rather than `<input>` elements. Both `index.css:311-334` (light themes) and `index.css:1076-1082` (dark themes) apply `!important` backgrounds, borders, and two `1.2px` brass rivets (`radial-gradient(circle at 5px 50%...)`) to `:is(main, [role="dialog"]) button:not([role="tab"]):not([data-roman-gauge="true"]):not([disabled])`. Checked states (`data-[state=checked]:bg-primary`) are completely erased by `#FAF5E8 !important` / `var(--color-bg-elevated) !important`, `16×16px` checkboxes get brass rivets painted over their checkmarks, and hovering a switch shifts it upward (`transform: translateY(-1px)`).
3. **Unstyled / Theme-Leaking `Select`, `HoverCard`, and `ContextMenu` Portals:** `index.css:480` only styles native `<select>`, leaving Radix `<Select.Trigger>` (`button[role="combobox"]`) looking like a riveted action CTA rather than a field. Meanwhile, `Select.Content` (`role="listbox"`), `HoverCard.Content` (no `role`, portaled under `[data-radix-popper-content-wrapper]` on `document.body`), and `DropdownMenu.Content` (`role="menu"`) are omitted from `:is(main, [role="dialog"])` rules (`index.css:683-695, 1076, 1141`), causing dark/light token mismatches inside portaled menus and comboboxes.
4. **Forced `nowrap` Clipping on Bento Card Buttons:** `index.css:1180-1184` forces `white-space: nowrap` on `button.inline-flex:not(.whitespace-normal):not([data-multiline="true"])`. Multi-line shadcn command items, accordion headers, and rich Bento selector cards immediately overflow horizontally and trigger `clippedInteractiveCount > 0` in `audits.ts:119-134`.

**Why it happens:**
Milestones v6.4–v6.6 normalized legacy hardcoded Tailwind classes across 85+ files using broad structural/substring CSS rules in `index.css` before Radix button-based form controls (`role="switch"`, `role="checkbox"`, `role="combobox"`) and portaled `listbox`/`popper` wrappers existed in the app.

**How to avoid:**
- Use the unified `radix-ui@1.8.1` package (replacing fragmented `@radix-ui/react-*` packages in `package.json:26-28`) and tag every shadcn/Radix primitive wrapper with `data-slot="..."` attributes (`data-slot="dialog-overlay"`, `data-slot="switch"`, `data-slot="checkbox"`, `data-slot="select-trigger"`, `data-slot="select-content"`, `data-slot="popover-content"`).
- Update `index.css:708-737` to exclude `[data-slot$="-overlay"]`, `[data-radix-dialog-overlay]`, and `[class*="bg-black/"]` scrims (`:not([data-slot$="-overlay"]):not([class*="bg-black/"])`).
- Update `index.css:311` and `index.css:1076` button selectors to exclude ARIA form roles and primitive slots:
  `:not([role="tab"]):not([role="switch"]):not([role="checkbox"]):not([role="radio"]):not([role="combobox"]):not([role="menuitem"]):not([role="option"]):not([data-slot]):not([data-roman-gauge="true"])`.
- Extend portal container selectors in `index.css:708, 745, 974, 1038` from `:is(..., [role="dialog"], [role="menu"], [role="tooltip"])` to also include `[role="listbox"], [role="alertdialog"], [data-radix-popper-content-wrapper]`.
- Install `tw-animate-css@1.4.0` and `@import "tw-animate-css";` in `index.css` — currently `Dialog.tsx:41-42, 68-72` and `Tooltip.tsx:42` use `animate-in fade-in-0 zoom-in-95`, which are **silent no-ops** because neither `tw-animate-css` nor `tailwindcss-animate` is installed in `crates/vox-gui/ui/package.json`!

**Warning signs:**
- Opening a Dialog in default `arcane` theme blanks the entire window in solid cream `#FAF5E8`.
- Toggling a `Switch` or `Checkbox` updates `aria-checked="true"` in DOM inspector, but its background color does not change in either `arcane` or `slate` mode.
- Opening a Radix `Select` in `slate` mode shows unstyled or Roman-serif options, or `Switch` grows brass rivets on its left/right edges.

**Phase to address:**
**Phase 1 (CSS & Portal Foundation)** — before migrating any surface controls to Radix/shadcn primitives.

---

### Pitfall 2: Silent Fake-Optimistic Success When Live Tauri IPC or Daemon Tools Fail (`t3Wave3Rpc.ts` & `.catch(() => {})` Swallows)

**Severity:** CRITICAL

**What goes wrong:**
Controls appear "wired" in demos and E2E mocks, mutating local React/in-memory state and displaying green confirmation pills or updated toggles even when the live Tauri IPC command or `vox-orchestrator-d` MCP tool rejects or returns `{ is_error: true }` / `{ success: false }`:
1. **Conflated Preview vs. Live-Error Catch Blocks in `t3Wave3Rpc.ts`:** Across `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts` (`chatConsensusRun` at `L155-212`, `vcsCheckpointCapture` at `L273-304`, `vcsCheckpointRestore` at `L306-338`, `stageWorkspaceFile` at `L490-505`, `revertWorkspaceFile` at `L507-522`, `setDistrictQuarantine` at `L524-557`, `worktreeSpawnVariant` at `L601-623`, `editChatMessageInPlace` at `L632-662`, `regenerateChatTurn` at `L664-693`, `compactChatUpTo` at `L695-730`, `cancelChatStream` at `L780-796`), `unwrapMcpData` (`L113-140`) rightly throws `new Error(...)` when `obj.is_error === true` or `!obj.success`, **but the surrounding `try { ... } catch { // Fall through }` block swallows that live error** and returns fabricated success objects (`{ file_path, staged: true }`, `{ reverted: true }`, fake `commit_oid`s)! Even worse, `setDistrictQuarantine` (`L529-532`) mutates `inMemoryQuarantinedPrefixes` *before* the RPC call and never rolls back on failure — showing a "Quarantined" portcullis in the UI while `agent_loop.rs` on the daemon remains unfenced.
2. **Fire-and-Forget `.catch(() => {})` Mutations Across Surfaces:** Over 60 call sites (`ChatExecutionRail.tsx:350-451`, `PlanPanel.tsx:221-331`, `ImperiumInspector.tsx:145-732`, `LudusSandbox.tsx:533, 2333-2743`, `MensTrainingView.tsx:928-2143`, `PoliciesView.tsx:318`) fire `void invoke(...).catch(() => {})` after optimistic state updates without rollback or user-visible error reporting.
3. **`sentryHonesty.ts` Invariant Violation:** `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.ts:12-54` (`checkHonestyInvariant`) flags `kind: 'fake_success'` (`severity: 'critical'`) whenever a stage reports `status === 'completed'` while all provider probes failed (`!p.ok || p.httpStatus >= 400`) or required payload fields are empty without rendering `[data-testid="empty-results-notice"]` or `emptyStateAcknowledged: true`.

**Why it happens:**
Developers wanted components to remain interactive in bare-browser Vite preview (`!backendAvailable()`, `.no-tauri`), so they wrapped `if (backendAvailable()) { try { await rpc() } catch {} }` and fell through to mock/in-memory state — accidentally masking real daemon/IPC failures when `backendAvailable() === true`.

**How to avoid:**
- **Strictly separate `!backendAvailable()` from live RPC errors:**
  ```ts
  if (!backendAvailable()) {
    return standalonePreviewFallback(input); // Honest preview-only branch
  }
  // Live Tauri / Remote Axis path: NEVER swallow errors into fake success!
  const raw = await voxTransport.invokeMcpTool('vox_stage_file', { file_path: filePath });
  const data = unwrapMcpData<{ file_path: string; staged: boolean }>(raw);
  if (!data?.file_path) throw new Error('vox_stage_file returned empty payload');
  return data;
  ```
- **Enforce Snapshot-Rollback + Sanitized Toast on Optimistic Controls:** Every mutating control (toggle, slider commit, task rename/reprioritize, quarantine switch, model select, config save) must capture `prevState`, disable or mark `aria-busy="true"` while in-flight (or revert `setState(prevState)` in `catch`), and emit `pushToast({ tone: 'warn', title: '...', body: sanitizeErrorForToast(err), cause: 'backend-error' })`.
- **Honor Structured Tauri v2 Rejections:** As documented in `App.tsx:223-262`, Rust commands returning `Result<T, MyTaggedEnum>` (`#[serde(tag = "kind")]`) reject `invoke()` with a deserialized object `{ kind, message }`, not an `Error` instance or string — never assume `err instanceof Error`, and never pass `String(err)` to state or toasts (blocked by `toastBodyGuard.test.ts:17`).

**Warning signs:**
- Clicking "Stage File", "Revert File", "Quarantine", or "Cancel Task" with `vox-orchestrator-d` stopped shows a green receipt instead of an error toast.
- `t3Wave3Rpc.ts` contains `catch { // Fall through }` inside an `if (backendAvailable())` block.
- `checkHonestyInvariant` returns `fake_success` during degraded/error mock sweeps (`installErrorStateMock` in `e2e/lib/tauriMockVariants.ts`).

**Phase to address:**
**Phase 3 (Honest IPC & Live Control Wiring)** — audited across every `invoke` / `voxTransport` mutation site and verified with `installErrorStateMock`.

---

### Pitfall 3: Z-Index Stacking Wars, Body Scroll-Lock `outerOverflowPx` Blowouts & `dockview` Pointer Lockups

**Severity:** CRITICAL

**What goes wrong:**
1. **Portal vs. Chrome Stacking Collisions:** Vox Axis currently mixes two incompatible z-index regimes:
   - `z-50`: `BottomStatusBar.tsx:290` (`relative z-50`), `Sidebar.tsx:325` (`sticky top-0 z-50`), `Dialog.tsx:40,63`, `Omnibar.tsx:618`, `AchievementsDrawer.tsx:32`, `DocViewerDrawer.tsx:74`, `ExplainDrawer.tsx:128`, `ResearchEngineDrawer.tsx:239`, `IntelliSensePopover.tsx:64`.
   - `z-[99999]`: `Tooltip.tsx:42`, `RomanTooltip.tsx:160`, `Popover.tsx:14`, `RiskPopover.tsx:44`, `DriveConsole.tsx:107`, `Sidebar.tsx:538`, `ChatSurface.tsx:1504`.
   - `z-40`: `Toasts.tsx:40` (renders *underneath* open `z-50` Dialogs/Drawers, hiding IPC error toasts triggered by buttons inside a Dialog!).
   Because `BottomStatusBar` (`z-50`) sits *after* `<main>` in `AppShell.tsx:192`, any non-portaled `z-50` dropdown inside `<main>` paints **behind** `BottomStatusBar`. Conversely, if a Radix `Select` or `Popover` uses default shadcn `z-50` while opened next to a `z-[99999]` `Sidebar` flyout or inside a `Dialog`, stacking breaks or tooltips get trapped behind/above the wrong layer.
2. **Radix `react-remove-scroll` vs. `AppShell` `w-screen` (`outerOverflowPx > 0`):** `AppShell.tsx:132` uses `w-screen overflow-hidden`, and `index.css:223-224` already locks `html, body, #root { height: 100%; margin: 0; overflow: hidden; }`. When Radix `Dialog`, `Select` (`modal` by default!), or `DropdownMenu` (`modal={true}` by default!) opens, `@radix-ui/react-remove-scroll` injects `data-scroll-locked` + `pointer-events: none` + `margin-right` compensation onto `<body>`. Combined with `w-screen` (`100vw`), any scrollbar compensation pushes `document.documentElement.scrollWidth > window.innerWidth`, tripping `outerOverflowPx > 0` in `audits.ts:89` and failing `cv-flywheel.spec.ts:459, 706`!
3. **Version-Skewed Radix `DismissableLayer` & `dockview` Pointer Capture:** `DockWorkspaceShell.tsx:159` forces `dndStrategy="pointer"` on `dockview@6.6.1`. If fragmented `@radix-ui/react-*` packages pull duplicate `@radix-ui/react-dismissable-layer` copies, or if a modal Radix `DropdownMenu`/`Select` sets `body { pointer-events: none }` while a user interacts with a `dockview` tab or nested `Dialog` control, clicks inside a nested `Select` close the parent `Dialog`, or `Toasts` (`z-40`, outside the active `DismissableLayer`) become unclickable while any modal is open.

**Why it happens:**
Default shadcn components assume a scrolling `<body>` web page with flat `z-50` portals, whereas Vox Axis is a fixed-viewport desktop shell (`overflow: hidden` root, `SurfaceScrollHost` inner scroller, `dockview` pointer DnD, sticky `z-50` chrome, and `z-[99999]` bespoke portals).

**How to avoid:**
- **Establish a Single Semantic `@theme` Z-Index Ladder in `index.css`:**
  - `--z-shell-chrome: 40` (`Sidebar`, `BottomStatusBar`)
  - `--z-drawer: 60` (`AchievementsDrawer`, `DocViewerDrawer`, `ExplainDrawer`, `ResearchEngineDrawer`)
  - `--z-dialog: 70` (`DialogOverlay`, `DialogContent`, `Omnibar`)
  - `--z-popover: 80` (`Popover`, `SelectContent`, `DropdownMenuContent`, `ContextMenuContent`, `HoverCardContent`, `ChatSurface` Panels menu) — guarantees nested `Select`/`Popover` inside `Dialog` (`70`) ALWAYS renders on top!
  - `--z-toast: 90` (`Toasts.tsx` — ensures error toasts triggered from inside a `Dialog` or `Drawer` remain visible above the modal!)
  - `--z-tooltip: 100` (`Tooltip`, `RomanTooltip`)
  Retire every ad-hoc `z-[99999]` class across the 8 files.
- **Neutralize Radix Body Scroll-Lock Shifts on the Fixed Shell:** Because `html, body, #root` is already `overflow: hidden` (`SurfaceScrollHost` owns scrolling), neutralize `react-remove-scroll` margin/padding injection in `index.css`:
  ```css
  html body[data-scroll-locked] {
    margin-right: 0 !important;
    padding-right: 0 !important;
    overflow: hidden !important;
  }
  ```
  And replace `w-screen` in `AppShell.tsx:132` with `w-full` (`100%` of `#root` rather than `100vw`).
- **Pass `modal={false}` on Toolbar `DropdownMenu` / `Popover`:** For non-blocking workspace menus (status bar popovers, dock panel switchers, composer posture knobs), set `modal={false}` so `body` never receives `pointer-events: none` during rapid workbench multi-tasking.
- **Deduplicate Radix Primitives:** Migrate `package.json` to the unified `radix-ui@1.8.1` package (and verify with `pnpm dedupe --check`) so `DismissableLayer` and `FocusScope` share a single singleton context under `react@19.2.8`.

**Warning signs:**
- Clicking an item in a `Select` inside `DialogContent` closes the `Dialog` without selecting the option.
- An IPC failure inside `RunOutputDialog` or `ResearchEngineDrawer` fires `pushToast`, and the toast appears invisibly *behind* the dark backdrop (`z-40 < z-50`).
- `cv-flywheel.spec.ts` reports `outerOverflowPx: 15` only on captures where a modal or `Select` is open.

**Phase to address:**
**Phase 1 (CSS & Portal Foundation)** (z-index tokens, `radix-ui@1.8.1` dedupe, `body[data-scroll-locked]` lock, `AppShell` `w-full`).

---

### Pitfall 4: The `<main>` Axe Blind Spot + `scrollable-region-focusable` & ARIA Slider/Knob Violations

**Severity:** CRITICAL

**What goes wrong:**
1. **`cv-flywheel.spec.ts` Misses Every Portal Violation, Then `capture.spec.ts` Explodes:** Look at `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts:143`:
   `const axe = await new AxeBuilder({ page }).include('main').analyze();`
   Every Radix `<Portal>` (`Dialog`, `Select`, `Popover`, `DropdownMenu`, `Tooltip`, `Sheet`) mounts directly under `document.body` **outside `<main>`**! Similarly, `audits.ts:154` (`lightThemeContrastLeakCount`) and `audits.ts:488` (`digitalColorLeakCount`) query only `'main ...'`. A portaled Radix component with missing `DialogTitle`, unlabeled `Slider.Thumb`, or low-contrast slate text passes `cv-flywheel.spec.ts` with `axeViolations === 0` — and then fails `e2e/review/capture.spec.ts:82` (`new AxeBuilder({ page }).analyze()`, which audits the entire `document` including `moderate`, `serious`, and `critical` impacts)!
2. **`scrollable-region-focusable` in Dense Bento Cards & `ScrollArea`:** Compacting whitespace into fixed-height Bento cells introduces internal `overflow-y-auto` containers and Radix `<ScrollArea>` viewports (`[data-radix-scroll-area-viewport]`). When a scrollable pane contains read-only telemetry logs, diffs, or KPI lists with no focusable child (or in WebKit/Chromium scroll checks), `axe-core` fires `scrollable-region-focusable` (`impact: 'serious'`). (`SurfaceScrollHost.tsx:8-11` explicitly solved this at the top level with `role="region" aria-label="Surface content" tabIndex={0}`, as tested in `SurfaceScrollHost.test.tsx:6`, but inner Bento scroll boxes lack it!)
3. **ARIA `slider` / `spinbutton` / Rotary Knob Violations (`aria-input-field-name`, `aria-required-attr`):**
   - In Radix `<Slider>`, `role="slider"` lives on `<Slider.Thumb>` (`<span>`), NOT `<Slider.Root>`. Attaching `<label htmlFor={id}>` or `aria-label` to `<Slider.Root>` leaves `<Slider.Thumb>` unnamed, firing `aria-input-field-name` (`serious`). Multi-thumb range sliders require a distinct `aria-label` on *every* `<Slider.Thumb>`.
   - Custom Romanesque rotary knobs / interactive `<RomanBrassGauge>` dials or numeric steppers wired live in v7.4 fail `aria-required-attr` (`critical`) if `aria-valuenow`, `aria-valuemin`, or `aria-valuemax` transiently evaluates to `undefined` or `NaN` while daemon telemetry is still loading.
4. **Nested Interactive Controls in Bento Cards (`nested-interactive`):** Making a Bento card summary clickable via `<div role="button" tabIndex={0}>` while placing a live action `<button>`, `<Switch>`, or `<SelectTrigger>` inside that card header triggers `axe-core` `nested-interactive` (`serious`) and swallows keyboard `Space`/`Enter` events.

**Why it happens:**
Developers test surface layouts with `pnpm review:cv-flywheel` assuming its `axeViolations` metric covers the whole page, and wrap Bento scroll regions in raw `<div className="overflow-y-auto">` or attach `aria-label` to `<Slider.Root>` instead of `<Slider.Thumb>`.

**How to avoid:**
- **Expand `cv-flywheel.spec.ts:143` & `audits.ts` Beyond `<main>`:** Update `AxeBuilder` in `cv-flywheel.spec.ts:143` to include portaled overlays (`.include('main').include('[role="dialog"]').include('[role="listbox"]').include('[role="menu"]').include('[data-radix-popper-content-wrapper]')` or full-page `.analyze()`) and mirror those selectors in `audits.ts:154, 488`.
- **Standardize `<BentoScrollRegion label="...">`:** Never write raw `overflow-y-auto` on non-interactive Bento content panes; wrap scrollable Bento bodies (and Radix `<ScrollAreaPrimitive.Viewport>`) with `tabIndex={0}`, `role="region"`, and a non-empty `aria-label`.
- **Enforce Thumb-Level Naming & Finite Numeric Guards on Sliders/Knobs:**
  - Require `ariaLabel: string | [string, string]` as a **non-optional TypeScript prop** on Vox Axis's `<Slider>` wrapper and forward it directly to each `<SliderPrimitive.Thumb aria-label={...} />`.
  - On custom knobs / `role="spinbutton"` / `role="slider"`, clamp `aria-valuenow={Number.isFinite(val) ? val : min}` and keep `data-roman-gauge="true"` on custom gauge elements so `index.css:311` does not restyle them as rectangular marble buttons.
- **Use Sibling Overlay Link/Button Pattern Instead of Nested `role="button"`:** For clickable Bento cards containing inner controls, render the card as `<section>` / `<article>`, make only the heading `<button>` interactive (or use a sibling stretched trigger), and keep inner controls as flat siblings in the accessibility tree.

**Warning signs:**
- `cv-flywheel.spec.ts` passes (`axeViolations: 0`), while `VOX_REVIEW_CAPTURE=1 pnpm review:capture` logs `scrollable-region-focusable`, `aria-input-field-name`, `aria-dialog-name`, or `nested-interactive`.
- `<Slider.Root aria-label="...">` appears in code without `aria-label` on `<Slider.Thumb>`.
- Any `<DialogContent>` mounts without a `<DialogTitle>` (even `<DialogTitle className="sr-only">` is required by Radix and `aria-dialog-name`).

**Phase to address:**
**Phase 1 (CSS & Portal Foundation)** (primitive wrappers enforcing `DialogTitle` & `Slider.Thumb` `ariaLabel`) and **Phase 5 (Accessibility & Static Audit Hardening)** (full-body/portal Axe gate).

---

### Pitfall 5: `vox-code-audit` (`toestub`), `themeContract`, `knip`, and CI Manifest Gates Failing on Upstream shadcn & Live-Wiring Code

**Severity:** CRITICAL

**What goes wrong:**
Standard shadcn/ui CLI output, quick layout utilities, and new Tauri IPC wiring fail multiple hard CI gates in `vox ci pre-push` and Vitest:
1. **`vox/ui/layout-and-perf-hazards` (`crates/vox-code-audit/src/detectors/ui_perf_hazards.rs:78-96`):**
   - **Bans `transition-all` (`L78`):** Upstream shadcn `Accordion`, `Progress`, `Sheet`, `Tabs`, `Switch`, and `ResizableHandle` use `transition-all` extensively. Every single occurrence in `.tsx`, `.ts`, or `.css` triggers a `toestub` finding!
   - **Bans raw `new ResizeObserver(` without `requestAnimationFrame` (`L80-83`):** Any new Bento chart or responsive panel calling `new ResizeObserver(...)` without `requestAnimationFrame` within lines `[-1, +8]` triggers a hazard finding (see `DockWorkspaceShell.tsx:74-80` for the exact rAF pattern).
   - **Bans unscoped `:has(` in `.css` (`L84-85`):** Writing `:has(...)` or `.foo :has(...)` in `index.css` triggers `has_unscoped_css_has`.
   - **Bans `delete obj[key]` (`L79`)** and **`scrollbar-gutter: stable` without `both-edges` alongside `mx-auto` (`L86-89`)**.
2. **`vox/ui/theme-contract-drift` (`ui_theme_contract.rs:95-248`, `Severity::Error`) & `themeContract.test.ts:133-151`:**
   - Any dark Tailwind hex `bg-[#0...]` / `bg-[#1...]` / `bg-[#2...]` (except allowlisted emerald status hexes), dark inline `rgba(10..28, ...)`, or font literal (`Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`) anywhere in `crates/vox-gui/ui/src/` immediately fails **both** Vitest and `toestub` with `Severity::Error`!
3. **`vox/fs/hardcoded-abs-dir` (`hardcoded_abs_dir.rs:39-70, 130-199`) & `drift/hardcoded-abs-dir`:**
   - Any hardcoded workstation path (`/Users/<realuser>/...`, `/home/<realuser>/...`, `C:\Users\...`), `.gemini/.../brain/` artifact path, `/private/tmp/`, or non-test `/tmp/...`, `/var/...`, `/opt/...` string literal in `.rs`, `.ts`, `.tsx`, or `.vox` fails `toestub`, `vox-drift-check`, and `lefthook.yml`. Even in tests, only synthetic usernames in `SYNTHETIC_TEST_USERS` (`alice`, `bob`, `dev`, `vox`, `test`, etc. — `hardcoded_abs_dir.rs:13-33`) are allowed!
4. **Vitest Architectural Guards (`src/guards/*.test.ts`) & `knip` (`knip.jsonc:12-14`):**
   - **`ipcBoundaries.test.ts:43-94`:** Any *new* `.ts`/`.tsx` file importing `from '@tauri-apps/api/core'` fails Vitest! All new IPC must go through `voxTransport` (`src/transport.ts`, strictly inside `// __VOX_RAW_IPC_BEGIN__ ... // __VOX_RAW_IPC_END__` per `transportIpcGuard.test.ts:13-28`).
   - **`toastBodyGuard.test.ts:17-49`:** Any `body: String(err)`, `setFoo(String(err))`, or bare `String(e|err|error)` fails Vitest!
   - **`command-backends.v1.yaml` & `commandBackendsMigration.test.ts:95-100`:** Adding a new Rust command module in `crates/vox-gui/src/commands/*.rs` without classifying it as `daemon | host | shell` (NEVER `db`) in `contracts/gui/command-backends.v1.yaml` fails `vox ci gui-catalog-parity` and Vitest.
   - **`knip.jsonc:12-14`:** `files: "error"`, `dependencies: "error"`, `devDependencies: "error"`. Generating 15 shadcn `src/components/ui/*.tsx` files "for future use" when only 8 are imported — or leaving old `@radix-ui/react-dialog` in `package.json` after switching imports to `radix-ui` — hard-fails `pnpm --dir crates/vox-gui/ui knip`!
   - **`arch/god_object` (`god_object.rs:16-20, 102-124`, `Severity::Error` > 500 non-blank code lines):** Several GUI files (`App.tsx`, `LudusSandbox.tsx`, `MensTrainingView.tsx`, `Loquela.tsx`, `ChatSurface.tsx`, `SettingsView.tsx`, `BrowserView.tsx`, `t3Wave3Rpc.ts`) already exceed 500 lines; creating *new* monolithic Bento/RPC files > 500 non-blank lines (or > 12 exported functions/consts) triggers new `arch/god_object` errors/warnings, and unreasoned `// @ts-expect-error` or `// toestub-ignore(...)` without `-- <reason>` triggers `vox/lint/unreasoned-suppression` (`unreasoned_suppression.rs:79-107`)!

**Why it happens:**
Vox enforces industrial static hygiene across both Rust and TypeScript. Generic React/shadcn code snippets violate at least four Vox-specific static detectors out of the box.

**How to avoid:**
- **Never run raw `npx shadcn@latest add` unedited:** Author clean, semantic-token wrappers in `crates/vox-gui/ui/src/components/ui/` that use:
  - Compositor-only transitions (`transition-[color,background-color,border-color,opacity,transform,box-shadow]`, NEVER `transition-all`).
  - Semantic CSS variables (`bg-bg-surface`, `bg-bg-elevated`, `text-text-primary`, `border-border-strong`, `font-sans`, `font-display`, `font-mono`).
  - React 19 native `ref` props (`function SelectContent({ ref, ... })`) rather than legacy React 18 `forwardRef` mismatches.
- **Only install & create UI primitives in the exact plan that consumes them** (and remove superseded `@radix-ui/react-dialog`, `@radix-ui/react-slot`, `@radix-ui/react-tooltip` entries from `package.json` in the same commit) so `pnpm knip` stays 100% green.
- **Decompose new v7.4 hooks/panels into `< 250`-line focused modules** (`use<Surface>Actions.ts`, `<Surface>BentoGrid.tsx`) routing all IPC through `voxTransport`.
- **Avoid `page.waitForTimeout(...)` in all new E2E tests:** `vox/test/flaky-wait-and-unreasoned-skip` (`test_clock_and_skip.rs:95-103`) flags every new `waitForTimeout(` and `await new Promise(...requestAnimationFrame)` in `.spec.ts` / `.test.tsx` files! Use Playwright web-first assertions (`await expect(locator).toBeVisible()`, `toHaveAttribute(...)`) instead.

**Warning signs:**
- `rg "transition-all" crates/vox-gui/ui/src` returns any matches.
- `pnpm --dir crates/vox-gui/ui test` fails in `src/guards/` or `themeContract.test.ts`.
- `pnpm --dir crates/vox-gui/ui knip` reports unused files in `src/components/ui/` or unused dependencies in `package.json`.

**Phase to address:**
**Every Phase (Phases 1–6)**, with an automated pre-flight gate established in **Phase 1** and verified workspace-wide in **Phase 5**.

---

## Moderate Pitfalls

### Pitfall 6: Incomplete Unicode Glyph Purge, Zero-Size SVG Regressions & Broken Glyph-Asserting Unit Tests

**Severity:** MODERATE (becomes CRITICAL if `axe-core` `button-name` or Vitest breaks)

**What goes wrong:**
1. **`audits.ts` `rawEmojiCount` Only Catches ~15% of Remaining Glyphs:** `crates/vox-gui/ui/e2e/review/audits.ts:264-281` only scans `'main button, main h1, main h2, main h3, [data-testid="urbs-citizen-sprite"], [data-testid="loquela-composer"] button, header button, footer button'` and matches `/[\u{1F300}-\u{1F4FF}\u{1F500}-\u{1F511}\u{1F514}-\u{1FAFF}\u{2694}\u{26A1}\u{1F512}\u{1F513}]/gu` (even exempting `urbs-imperium-inspector` at `L277`!). Live `rg` across `crates/vox-gui/ui/src/components/` reveals **70+ remaining Unicode dingbats, geometric shapes, arrows, and symbols** sitting in `<span>`, `<td>`, `<summary>`, `StatusPill`, `PlanPanel`, `Kpi`, `DataTable`, and drawers:
   - Dingbats & symbols: `✓`, `✕`, `✎`, `⚠`, `⚠️`, `⚒`, `☼`, `★`, `⚙` (`StatusPill.tsx:14-28`, `PlanPanel.tsx:39,137`, `WidgetPickerDrawer.tsx:52`, `ResearchView.tsx:197,528,859`, `SpeculumLoupePopover.tsx:291-333`, `LudusHud.tsx:76`, `MultiWaveProgressTimeline.tsx:126,136`, `CodeGenVerificationCard.tsx:39`, `ShoppingDecisionCard.tsx:62,105`).
   - Geometric shapes & arrows: `◆`, `◇`, `●`, `○`, `◔`, `◑`, `▸`, `▾`, `▶`, `▼`, `▲`, `■`, `↻`, `↺`, `↗`, `↑↓` (`StatusPill.tsx:19-28`, `PlanPanel.tsx:35-38`, `Kpi.tsx:110`, `DataTable.tsx:149`, `BottomStatusBar.tsx:348`, `Mercatus.tsx:151-179`, `ContextWindowMeter.tsx:171`, `CapabilityBadge.tsx:19,34`, `ResearchTracePanel.tsx:117`, `GamifyView.tsx:277,618,624`, `Toasts.tsx:58`).
2. **Existing Vitest Suites Explicitly Assert Literal Glyphs:** Blindly replacing glyphs with `<svg>` icons breaks existing unit tests:
   - `Kpi.test.tsx:11`: `expect(screen.getByText('▲1')).toBeInTheDocument()`
   - `CapabilityBadge.test.tsx:8,11`: asserts `glyph: '✓'` and `glyph: '◇'`
   - `ExplorerView.test.tsx:103`: `expect(dedicatedChip.textContent).toContain('✓')`
   - `SecretaryToast.test.tsx:27`: queries dismiss button via `'✕'` text
   - `TurnTrace.test.tsx:37,119`: `expect(summary).toHaveTextContent('... · 1 receipt ✓ ...')`
3. **`button-name` Axe Critical Failures & `zero-size-svg` (`iconIssueCount`):**
   - Replacing text `'✕'` (`WidgetPickerDrawer.tsx:52`, `ResearchView.tsx:197`, `ResearchEngineDrawer.tsx:267`) or `'✎'` (`PlanPanel.tsx:137`) or `'↺'` (`ResearchView.tsx:810`) with `<Icon.close className="size-3.5" aria-hidden="true" />` leaves the `<button>` with **empty textContent**. Without `aria-label="Close"` / `"Edit"` / `"Retry"`, `axe-core` immediately fails `button-name` (`critical`).
   - `audits.ts:18-32` (`auditIconsInPage`) flags EVERY `<svg>` not inside `details:not([open]), .sr-only, [hidden]` that has `width === 0 || height === 0` (`zero-size-svg`) or lacks drawable primitives (`empty-svg`). An icon inside a CSS `max-h-0 overflow-hidden` collapsed Bento accordion (without `hidden`) or inside a tight `inline-flex` button without `shrink-0` collapses to `0px` width and fails `cv-flywheel.spec.ts:716` (`expect(dom.iconIssueCount).toBe(0)`)!

**Why it happens:**
Cinzel and Palatino (`index.css:18-21`) lack coverage for Unicode geometric/dingbat blocks, forcing the OS to synthesize fallback glyphs from `Apple Symbols` / `Segoe UI Symbol` with inconsistent baselines and widths — yet legacy tests and `audits.ts` treated ASCII/BMP dingbats differently fromastral-plane emojis.

**How to avoid:**
- Upgrade `audits.ts:261-281` (plus add a fast static Vitest source guard `glyphBan.guard.test.ts` excluding code blocks/comments) to cover BMP symbol/dingbat/geometric blocks (`\u{2190}-\u{21FF}`, `\u{25A0}-\u{25FF}`, `\u{2600}-\u{27BF}`, `\u{FE0F}`, `\u{1F300}-\u{1FAFF}`) across all rendered UI chrome (`span`, `button`, `th`, `td`, `summary`, `label`, `[role="dialog"]`, `[role="status"]`).
- Route all icons through `crates/vox-gui/ui/src/components/ui/Icons.tsx` (engraved Romanesque SVGs + curated `lucide-react@^1.39.0` 1.75px-stroke icons) with mandatory `shrink-0` and `aria-hidden="true"`.
- Whenever an icon replaces the sole glyph inside a `<button>`, require `aria-label="..."` on the `<button>` and update paired `.test.tsx` assertions in the same commit to assert semantic roles/labels (`getByRole('button', { name: /dismiss/i })`, `data-trend="up"`, `data-status="pass"`).
- Never hide SVG-bearing accordions/drawers purely with `max-h-0` or `opacity-0`; unmount closed content or apply the HTML `hidden` attribute (which `auditIconsInPage` at `audits.ts:20` explicitly respects).

**Warning signs:**
- `dom.iconIssueCount > 0` (`zero-size-svg`) in `cv-flywheel.spec.ts` at `900×600` compact resolution.
- Vitest failures in `Kpi.test.tsx`, `CapabilityBadge.test.tsx`, `ExplorerView.test.tsx`, `SecretaryToast.test.tsx`, or `TurnTrace.test.tsx`.

**Phase to address:**
**Phase 2 (Zero-Glyph & Icon Semiotics)**.

---

### Pitfall 7: Bento Grid & `react-resizable-panels` / `dockview@6.6.1` `0×0` Height Collapse, Intrinsic Min-Size Overflow & Stale Layout Storage Poisoning

**Severity:** MODERATE

**What goes wrong:**
1. **`0×0` Height Collapse Inside Bento Cells (`dockview` & `recharts` / Canvas):** As documented in `DockWorkspaceShell.tsx:48-66`, `dockview@6.6.1`'s `.dv-shell` uses `height: 100%`, which resolves to **`0×0` pixels** inside flex-stretched or `auto`-track CSS Grid parents unless broken by an explicit `px` wrapper measured via `ResizeObserver`! Similarly, `recharts@^3.8.1` `<ResponsiveContainer width="100%" height="100%">`, `@xyflow/react@^12.11.6`, `@xterm/addon-fit`, and `<DiegeticFxStage />` collapse to `0px` or throw `ResizeObserver loop completed with undelivered notifications` inside Bento grid tracks without definite heights.
2. **Intrinsic `min-width: auto` / `min-height: auto` Blowout (`scrollHostOverflowPx > 0`):** CSS Grid (`1fr`) and Flexbox items default to `min-width: auto` and `min-height: auto`. Placing a `<DataTable>`, `<pre>` diff, long model ID (`anthropic/claude-sonnet-4-5`), or `react-resizable-panels@4.22.1` `<Panel>` inside a Bento grid cell without `min-w-0 min-h-0` on *every* intermediate wrapper refuses to shrink below content width at `900×600` / `1100×720`, blowing out `scrollHostOverflowPx` (`cv-flywheel.spec.ts:707`) and `clippedInteractiveCount` (`cv-flywheel.spec.ts:345`).
3. **Persisted Layout Schema Poisoning (`localStorage`):** `DockWorkspaceShell.tsx:29` persists layouts under `${storageKeyPrefix}.dockview_layout.v3`, and `react-resizable-panels` persists split ratios by `autoSaveId`. Renaming panel IDs, changing default panel sets, or restructuring splits in v7.4 without bumping the storage key version (e.g., `.v4`) causes existing operator profiles to hydrate stale JSON layouts or resurrect closed panels (the `closedPanelIds` bug documented in `DockWorkspaceShell.tsx:16-27`).

**Why it happens:**
Bento layouts replace vertical document flow (`height: auto` stacks) with bounded 2D grid/split tracks where CSS percentage inheritance and intrinsic sizing rules interact nonlinearly across `900×600` to `1920×1080`.

**How to avoid:**
- Use `react-resizable-panels@4.22.1` for simple 2-way/3-way Bento splits within individual surfaces, and keep `DockWorkspaceShell` (`dockview@6.6.1` with its rAF `ResizeObserver` pixel-height bridge at `DockWorkspaceShell.tsx:70-89`) for multi-tabbed drag-and-drop docking in `ChatSurface`.
- Enforce `min-w-0 min-h-0 overflow-hidden` (or `BentoScrollRegion` `min-w-0 min-h-0 overflow-y-auto`) on every Bento cell and `<Panel>` child, plus `@container` (`@container/bento`) queries so inner toolbars wrap/compact based on *panel* width rather than `window.innerWidth` (`max-[1199px]:hidden` viewport media queries fail inside narrow `320px` Bento cells on a `1920×1080` monitor!).
- Bump any modified `dockview` or resizable layout persistence key (`v3 -> v4`) and wrap deserialization in schema/panel-ID validation with fallback to clean v7.4 presets.

**Warning signs:**
- Unit tests in `jsdom` pass (because `jsdom` mocks `0×0` layout boxes without real CSS Grid cascade), while Chromium renders blank chart/dock panels or `scrollHostOverflowPx > 0` at `900×600` and `1100×720`.
- Toolbar items with `xl:inline` overflow inside a `360px`-wide Bento column on a `1440×900` screen because viewport width is `>= 1280px`.

**Phase to address:**
**Phase 4 (Dense Bento & Resizable Workbench Layouts)**.

---

### Pitfall 8: Dense Bento Compaction Breaking Calibrated CV & VLM Flywheel Thresholds (`cvFlywheel.ts` & `vlmVisionFlywheel.ts`)

**Severity:** MODERATE

**What goes wrong:**
Compacting whitespace, shrinking headers, and rearranging cards into dense Bento grids unexpectedly trips hard numeric assertions in `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts:706-748`:
1. **`chartPanelFillRatio < 0.84` (`cv-flywheel.spec.ts:715`, `audits.ts:359-384`):** `audits.ts:365-371` measures every `.vox-marble-tablet` / `[data-cv-kpi-card="true"]` ≥ `120×36px` that contains `svg[data-cv-sparkline="true"]` or `svg[viewBox="0 0 60 18"]`, computing `svgRect.width / (cardRect.width - 16)`. If a Bento grid stretches a KPI card to `320px` wide while leaving a fixed `w-16` (`64px`) inline sparkline on the right, `chartPanelFillRatio` plunges to `0.21` and fails `< 0.84`!
2. **`vlmOverallScore < 0.88`, `flutedPillarCount < 2`, & `friezeHeaderCount > 1` (`cv-flywheel.spec.ts:728-739`, `vlmVisionFlywheel.ts:214-351`):**
   - Compacting a surface by removing `<RomanSurfaceHeader>` (`[data-roman-frieze="true"]`) flips `hasVisibleMarbleFrieze` to `false`, imposing a `-0.30` swing (`+0.16 -> -0.14`) on `q2Score` (`vlmVisionFlywheel.ts:331`) and failing `vlmOverallScore >= 0.88`.
   - Replacing `.vox-marble-tablet` classes with generic shadcn `<Card className="rounded-xl bg-card">` drops `flutedPillarCount` (`document.querySelectorAll('[data-roman-pillar="true"], .vox-marble-tablet, [data-roman-frieze="true"], [data-roman-medallion="true"]').length` at `vlmVisionFlywheel.ts:227-229`) below `2`, failing `cv-flywheel.spec.ts:735`!
   - Conversely, docking one surface inside another (or adding a sub-panel header that also uses `<RomanSurfaceHeader>`) sets `friezeHeaderCount > 1`, triggering a `-0.22` duplicate-frieze penalty (`vlmVisionFlywheel.ts:334`).
3. **`viewportCoverageRatio` Out-of-Band Penalty (`cvFlywheel.ts:359`):** `audits.ts:412-415` computes `mainRect.area / viewportArea`, and `cvFlywheel.ts:359` awards `+0.04` ONLY when `0.45 <= viewportCoverageRatio <= 0.92` (`-0.08` otherwise!). Over-compacting `<main>` so it stops height-stretching (`< 0.45` on `1920×1080`) or removing shell chrome so `<main>` exceeds `0.92` costs `0.12` on `compositionQuality`, dragging `visualAfterScore` below `0.90` (`cv-flywheel.spec.ts:747`).
4. **Control Height Spread & Composer Single-Row Invariant (`audits.ts:188-237`):** Mixing shadcn `h-9` (`36px`) selects with `h-7` (`28px`) buttons in `[data-loquela-toolbar-item="true"]` sets `controlHeightSpreadPx = 8` (penalizing `anatomicalAccuracy` above `4px` at `cvFlywheel.ts:345`), and wrapping even one button to a second line fails `composerToolbarRowCount === 1` and `composerToolbarHeightPx <= 52` (`cv-flywheel.spec.ts:741-745`).

**Why it happens:**
The CV/VLM Flywheel encodes Vox Axis's exact visual identity (Romanesque materiality, full-bleed telemetry charts, single-frieze hierarchy, uniform toolbar height rhythm) as mathematical invariants.

**How to avoid:**
- Keep **exactly one** compact `<RomanSurfaceHeader>` per top-level surface (auto-suppressed via `EmbeddedSurfaceProvider` when embedded in a dock or Bento sub-pane!).
- Ensure Vox Axis's Bento `<Card>` primitive applies `.vox-marble-tablet` (or semantic surface tokens mapped in `index.css:940-958` so `slate` strips marble cleanly while `arcane` keeps Carrara tablet relief and `flutedPillarCount >= 2` stays satisfied).
- Always render sparklines inside Bento KPI cards as full-bleed `w-full` bottom area+line SVGs (`preserveAspectRatio="none"`), never fixed-width inline badges, keeping `chartPanelFillRatio >= 0.90`.
- Standardize compact toolbar controls to a uniform `h-7` (`28px`) height across `[data-loquela-toolbar-item="true"]` and Bento card headers (`controlHeightSpreadPx <= 2`).

**Warning signs:**
- `visualAfterScore` drops from `0.92` to `0.86` after a Bento refactor even with zero overflow.
- `vlmCritique.actionableDirectives` logs missing `<RomanSurfaceHeader>` or `chartPanelFillRatio < 0.84`.

**Phase to address:**
**Phase 4 (Dense Bento & Resizable Workbench Layouts)** and **Phase 6 (Multi-Pass CV & VLM Flywheel Convergence)**.

---

## Minor Pitfalls

### Pitfall 9: Tauri Mock Desynchronization (`tauriMock.ts` vs `tauriMockRich.ts` vs `tauriMockVariants.ts`) Causing Skeleton & Console-Error Regressions

**Severity:** MINOR (fast to fix once spotted, high friction if forgotten)

**What goes wrong:**
Wiring previously static or local-only controls to new or newly invoked Tauri commands (`invoke('...')` / `voxTransport.*`) without adding deterministic handlers to **all three** E2E mock layers (`e2e/lib/tauriMock.ts`, `e2e/lib/tauriMockRich.ts`, and `e2e/lib/tauriMockVariants.ts`) causes queries to stay `pending` or reject during Playwright sweeps. Unhydrated queries leave `.animate-pulse` / `[aria-busy="true"]` placeholders mounted during screenshot capture, tripping `expect(dom.skeletonCount ?? 0).toBe(0)` (`cv-flywheel.spec.ts:719`, `audits.ts:463-472`) and failing `e2e/lib/tauriMockVariants.test.ts`!

**How to avoid:**
Whenever Phase 3 wires a control to a Tauri command or MCP tool (`invoke_mcp_tool`), update `tauriMock.ts`, `tauriMockRich.ts`, and `tauriMockVariants.ts` in the same PR slice and run `pnpm --dir crates/vox-gui/ui test e2e/lib/` before running Playwright.

**Phase to address:**
**Phase 3 (Honest IPC & Live Control Wiring)**.

---

### Pitfall 10: Exit-Animation Ghost Portals Colliding with Fast Playwright State Transitions & `.no-tauri` `backdrop-filter` Hangs

**Severity:** MINOR

**What goes wrong:**
1. Once `tw-animate-css@1.4.0` is enabled, Radix portals with `data-[state=closed]:animate-out fade-out-0 duration-150` stay mounted in `document.body` for `150ms` after pressing `Escape`. In `cv-flywheel.spec.ts` (e.g., `L312` `await page.keyboard.press('Escape')`, `L1388` cycling all 14 dock panels in a tight loop), a closing portal still in the DOM can get caught in the next screenshot or `overlappingControlPairs` / `AxeBuilder` scan!
2. `index.css:1168-1174` disables `backdrop-filter` under `.no-tauri` because headless software compositing hangs on `backdrop-filter: blur(...)`. However, `installTauriMock` injects `window.__TAURI_INTERNALS__`, so `main.tsx` does **not** add `.no-tauri` during mocked E2E runs! Adding heavy `backdrop-blur-2xl` across 12 Bento cards on a single surface slows headless Chromium screenshot compositing by 5–10× and risks hitting `cv-flywheel.spec.ts` timeouts.

**How to avoid:**
- Keep exit animations crisp (`duration-100`) and ensure `prefers-reduced-motion: reduce` (`index.css:511-520`, emulated in `capture.spec.ts:57`) collapses Radix exit animations to `0.01ms`. In E2E tests, assert `await expect(popover).toBeHidden()` after `Escape` rather than fixed sleeps.
- Avoid stacking `backdrop-blur-*` on dense Bento cards (opaque/gradient surface tokens `bg-bg-surface` / `.vox-marble-tablet` are faster, sharper on Sobel `edgeSharpness`, and theme-contract safe).

**Phase to address:**
**Phase 1 (CSS & Portal Foundation)** and **Phase 6 (Multi-Pass CV & VLM Flywheel Convergence)**.

---

## Technical Debt Patterns

Shortcuts that seem reasonable during v7.4 execution but create immediate or long-term failures.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Running `shadcn add` and keeping default Tailwind classes (`bg-popover`, `transition-all`, `z-50`) | Saves 15 minutes per primitive | Trips `vox/ui/layout-and-perf-hazards` (`transition-all`), breaks `arcane`/`slate` overrides in `index.css`, and causes z-index collisions with `BottomStatusBar` | **Never** — author Vox-native semantic wrappers once in Phase 1 |
| Catching live RPC errors and falling back to in-memory state (`catch { return mockOk }`) | Makes broken/missing daemon tools look smooth locally | Violates zero-mock honesty (`sentryHonesty.ts`), desyncs UI from `vox-orchestrator-d` (e.g., fake quarantine), masks regressions | **Never** when `backendAvailable() === true`; fallback is strictly for `!backendAvailable()` browser preview |
| Importing `invoke` from `@tauri-apps/api/core` directly inside new Bento components | Skips adding typed methods to `voxTransport` | Hard-fails `src/guards/ipcBoundaries.test.ts:43-94` and bypasses Remote Axis / gateway routing | **Never** — all new IPC goes through `src/transport.ts` |
| Suppressing lint/audit hits with bare `// @ts-expect-error` or `// toestub-ignore(...)` | Silences compiler/audit warning quickly | Hard-fails `vox/lint/unreasoned-suppression` (`unreasoned_suppression.rs:79-107`) in `vox ci pre-push` | **Never** bare; reasoned `-- <specific invariant>` only for genuine hardware/ResizeObserver boundaries |
| Scaffolding unused shadcn primitives into `src/components/ui/` ahead of consumers | Batch-creates UI kit files in Phase 1 | Hard-fails `pnpm knip` (`knip.jsonc:12` `files: "error"`, `dependencies: "error"`) | **Never** unless exported & consumed in the same slice (or explicitly entry-listed during Phase 1 -> Phase 3 bridge) |
| Using viewport media queries (`lg:grid-cols-3`, `max-[1199px]:hidden`) inside Bento panels | Familiar Tailwind responsive syntax | Breaks whenever a panel is docked at `340px` width on a `1920×1080` monitor (`ChatSurface` 14-dock layout) | **Only** on outer `AppShell` / `BottomStatusBar`; inside surfaces/docks always use `@container` queries |

---

## Integration Gotchas

Common mistakes when connecting Vox Axis UI controls to Tauri v2, `vox-orchestrator-d`, and workspace contracts.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| **`invoke_mcp_tool` (`vox-orchestrator-mcp`)** | Reading `res.data` directly or assuming `invoke()` rejects on tool error (`invoke_mcp_tool` resolves `{ tool, is_error: true, result }` with HTTP 200 IPC!) | Always unwrap via `unwrapMcpData<T>(raw)` (`t3Wave3Rpc.ts:97-145`) **without** a swallowing `catch`, so `is_error: true` and `result.success === false` surface as real errors |
| **Tagged Rust `CommandError` (`App.tsx:223-262`)** | Writing `err instanceof Error ? err.message : String(err)` when Tauri v2 rejects with a deserialized `{ kind, message }` struct | Route every error through `sanitizeErrorForToast(err)` (`src/lib/backendGuard.ts`) and match structured `kind` fields first (`toastBodyGuard.test.ts` bans `String(err)`) |
| **New Rust Command Modules (`crates/vox-gui/src/commands/*.rs`)** | Adding `pub mod foo;` + `tauri::generate_handler![foo_cmd]` without updating `contracts/gui/command-backends.v1.yaml` or opening `VoxDb` directly | Register every module in `command-backends.v1.yaml` as `daemon`, `host`, or `shell` (`class: db` is banned by `CORE-04` / `commandBackendsMigration.test.ts:98`) and verify `vox ci gui-catalog-parity` |
| **Remote Axis vs. Host Commands (`command-backends.v1.yaml`)** | Calling a `class: host` command (local PTY, local git/locator) without checking capability availability when connected to a remote Axis gateway | Gate `host`-only controls via `backendGuard` / `SurfaceUnavailable` or disable with an honest tooltip explaining host-local requirement |
| **Style Dictionary Theme Tokens (`pnpm tokens:build`)** | Editing `src/styles/tokens.*.generated.css` by hand or updating `themeContract.ts` without syncing `contracts/gui/theme-contract.v1.json` | Update token sources + `style-dictionary.config.mjs`, run `pnpm tokens:build`, and keep `contracts/gui/theme-contract.v1.json` in 1:1 parity (`themeContract.test.ts:42-61`) |

---

## Performance Traps

Patterns that look smooth on an isolated dev view but degrade severely across 33 surfaces, 14 docks, and 60fps canvas FX.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| **`transition-all` on Bento cards & resizable panes** | Janky 15fps splitter dragging; `vox/ui/layout-and-perf-hazards` static failure | Use explicit compositor properties (`transition-[opacity,transform]` or `transition-colors`); disable width/height CSS transitions on resizable panels during active pointer drag | Immediately on `vox ci pre-push` (`ui_perf_hazards.rs:78`) and when dragging `dockview` / `ResizableHandle` |
| **Unthrottled `ResizeObserver` state updates in Bento charts** | `ResizeObserver loop completed with undelivered notifications` console error (captured as failure evidence in `capture.spec.ts:48-54`) | Coalesce all `ResizeObserver` dimension state updates inside `requestAnimationFrame` with `cancelAnimationFrame` cleanup (exact pattern in `DockWorkspaceShell.tsx:74-88`) | >= 4 simultaneous responsive charts/braids resizing inside Multi-Dock Chat |
| **Per-cell `<RadixTooltip.Provider>` instantiation** | Noticeable mount lag (30–80ms) when rendering dense Bento tables/matrices with 100+ icon buttons | Mount **one** `<RadixTooltip.Provider delayDuration={150}>` at `AppShell` root instead of creating a new `Provider` inside every `<Tooltip>` (`Tooltip.tsx:27` currently wraps each instance!) | > 50 tooltipped controls on `ExplorerView`, `ModelsView`, or `MensTrainingView` |
| **High-frequency slider/knob IPC spam (`onValueChange` -> `invoke`)** | Flooding Tauri IPC & daemon SQLite/config writes 60 times/sec during thumb drag; out-of-order responses | Update local visual value on `onValueChange`, and fire `voxTransport` RPC only on `onValueCommit` (or trailing 180ms debounce with latest-request cancellation) | Dragging any live temperature, budget, or routing-weight slider (`ImperiumInspector`, `SettingsView`, `ContextWindowMeter`) |
| **Stacked `backdrop-blur-*` across Bento grid cards** | Headless Chromium screenshot capture slows 5–10× under `installTauriMock` (where `.no-tauri` is absent) | Reserve `backdrop-blur` strictly for floating portaled popovers/dialogs; use opaque/gradient `bg-bg-surface` / `.vox-marble-tablet` on stationary Bento cards | > 8 blurred cards per surface during 33-surface × 5-resolution `cv-flywheel.spec.ts` runs |

---

## Security & Sovereignty Mistakes

Domain-specific safety, privacy, and architecture invariants enforced in Vox Axis.

| Mistake | Risk | Prevention |
|---------|------|------------|
| **Leaking raw Tauri/daemon error strings into UI state or toasts** | Exposes `__TAURI_INTERNALS__`, internal filesystem paths, or raw SQL/HTTP stack traces to operator screenshots/logs; fails `toastBodyGuard.test.ts` | Pass **every** caught error through `sanitizeErrorForToast(err)` (`src/lib/backendGuard.ts`); never use `String(err)` in UI state |
| **Bypassing Action-Only Safety Floor (`D21`) on newly wired destructive buttons** | One-click execution of destructive operations (`Revert File`, `Delete Knowledge Node`, `Reset Config`, `Cancel Run`, `Purge Cache`) without confirmation | Route destructive mutations through `<ConfirmRunDialog>` (Radix `AlertDialog` pattern) and preserve server-side `permission-modes.v2.yaml` / approval gates |
| **Hardcoding developer home/temp paths in live-control defaults or fixtures** | Leaks workstation usernames (`/Users/...`) into public `vox` / `vox-axis` repos; fails `vox/fs/hardcoded-abs-dir` & `lefthook.yml` | Resolve workspace/config paths strictly on the Rust `host`/`daemon` side; use repo-relative paths (`crates/...`, `.planning/...`) in UI state and `SYNTHETIC_TEST_USERS` in tests |
| **Adding network-fetched icon fonts, CDNs, or telemetry calls** | Violates **Offline/Standalone Sovereignty** (`PROJECT.md:36, 449`) and **Telemetry Consent (`ConsentState::Unset` default OFF)** | Bundle all SVGs locally (`lucide-react`, `Icons.tsx`) and local fonts (`styles/fonts.css`); zero external runtime HTTP fetches from UI components |

---

## UX Pitfalls

Common user-experience mistakes when compacting 33 surfaces into dense Bento workspaces and wiring every control live.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| **Removing visible labels during Bento compaction without tooltips + `aria-label`** | Mystery-meat toolbar of 12 engraved Roman/Lucide icons that operators cannot decode | Pair compact icon-only controls with single-Provider `<Tooltip>` progressive disclosure (`Alt+/` shortcut hint where applicable) AND explicit `aria-label` |
| **Zero in-flight / saved feedback on instant-apply toggles & sliders** | User clicks a live policy/posture control and cannot tell if it persisted to `vox-orchestrator-d` or silently dropped | Show crisp subtle pending state (`disabled` or brass pulse < 150ms) + deterministic status pill update (or compact error toast on failure) |
| **Over-packing Bento grids at `900×600` (`compact`) and `1100×720` (`laptop`)** | Multi-column Bento cards crush tables into 1-word-per-line columns or clip action buttons | Collapse 3–4 column Bento grids to 2 columns below `@container (width < 820px)` and 1 column below `@container (width < 520px)` |
| **Breaking Romanesque vs. Minimalist Slate visual parity** | Surfaces look rich in `arcane` White/Gold marble, but portals/badges show hardcoded `#FAF6EE` cream backgrounds or `#C59B27` gold borders in `slate` dark mode | Test every new primitive and Bento card across all 4 contract profiles (`arcane`, `slate`/`void`, `glacier`, `high-contrast`) before closing its phase |

---

## "Looks Done But Isn't" Checklist

Verification checklist to catch "demo-wired" illusions during v7.4 execution:

- [ ] **Live Control Persistence Across Reload:** Toggling/editing the control updates the backend (`vox-orchestrator-d` / host config), **and reloading the surface (`/#view=...`) re-hydrates the exact saved value** rather than resetting to `useState` initial defaults.
- [ ] **Live Control Failure Honesty (`installErrorStateMock`):** When the backing Tauri command / MCP tool rejects, the control **reverts its optimistic UI state** and displays a sanitized warning toast — zero silent `.catch(() => {})` fake successes.
- [ ] **Portal Theme Parity (4 Themes):** Every Radix `Dialog`, `Select`, `Popover`, `DropdownMenu`, `Sheet`, and `Tooltip` renders with translucent dark scrim (never opaque `#FAF5E8` marble!), crisp readable text, and visible checked states in **`arcane`**, **`slate`/`void`**, **`glacier`**, and **`high-contrast`**.
- [ ] **Portal Z-Index Hierarchy:** Opening a `Select` or `Popover` *inside* a `Dialog` renders above the `Dialog`, and triggering a `Toast` while a `Dialog` or `Drawer` is open renders the `Toast` *above* the modal backdrop.
- [ ] **Full-Document Axe Zero (`capture.spec.ts` Parity, Not Just `<main>`):** Running `AxeBuilder({ page }).analyze()` with portals open AND closed yields `0` `moderate`/`serious`/`critical` violations (specifically checking `scrollable-region-focusable`, `aria-input-field-name`, `aria-dialog-name`, `button-name`, and `nested-interactive`).
- [ ] **Zero Unicode Dingbats/Glyphs + Updated Unit Tests:** `rg` confirms zero `✓ ✕ ✎ ⚠ ◆ ◇ ● ○ ▸ ▾ ▶ ▼ ▲ ■ ↻ ↺ ↗` glyphs in UI chrome, every icon-only button has `aria-label` + `shrink-0`, and all Vitest tests (`Kpi.test.tsx`, `CapabilityBadge.test.tsx`, `ExplorerView.test.tsx`, `SecretaryToast.test.tsx`, `TurnTrace.test.tsx`) pass.
- [ ] **Small-Resolution & 14-Dock Bento Integrity:** At `900×600` and `1100×720` (and with all 14 Chat docks cycled), `outerOverflowPx === 0`, `scrollHostOverflowPx === 0`, `clippedInteractiveCount === 0`, `iconIssueCount === 0`, `composerToolbarRowCount === 1`, and `chartPanelFillRatio >= 0.84`.
- [ ] **Static Gates 100% Green:** `pnpm --dir crates/vox-gui/ui build`, `pnpm --dir crates/vox-gui/ui test`, `pnpm --dir crates/vox-gui/ui knip`, `vox ci gui-catalog-parity`, and `vox ci pre-push` pass with `0` warnings/errors.

---

## Recovery Strategies

When a pitfall slips past initial implementation, use these targeted recovery playbooks rather than ad-hoc CSS/state band-aids.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| **Radix primitive clobbered by `index.css` `!important` rule** | LOW | Do **not** add inline `style={{ ... }}` hacks per call site. Add `data-slot="<primitive>"` on the shared UI wrapper in `src/components/ui/` and add `:not([data-slot="<primitive>"])` to `index.css:311, 708, 1076`. |
| **`cv-flywheel.spec.ts` `vlmOverallScore` or `visualAfterScore` drops < threshold after Bento compaction** | MEDIUM | Inspect `artifacts/cv-flywheel/cv-flywheel-all-surfaces.json` for the exact failing sub-metric (`chartPanelFillRatio`, `flutedPillarCount`, `hasVisibleMarbleFrieze`, `viewportCoverageRatio`, `controlHeightSpreadPx`) rather than guessing layout tweaks. |
| **`ResizeObserver` loop or `0×0` chart/dock panel inside Bento grid** | LOW | Ensure every ancestor flex/grid node has `min-w-0 min-h-0`, wrap the `ResizeObserver` setter in `requestAnimationFrame` (`DockWorkspaceShell.tsx:75-80`), and bind explicit container height (`h-full` chain or `min-h-[220px]`). |
| **`knip` or `arch/god_object` blocks pre-push mid-milestone** | LOW | Delete unconsumed UI wrapper files until their consuming phase lands, remove superseded `@radix-ui/react-*` entries from `package.json`, and extract >400-line surface action logic into `src/components/surfaces/<Name>/use<Name>Actions.ts`. |
| **Persisted `localStorage` layout crashes after Bento/dock refactor** | LOW | Increment the storage key version suffix (`.v3` -> `.v4` in `DockWorkspaceShell.tsx:29` / `autoSaveId`) so existing Tauri WebView profiles cleanly reset to the new preset. |

---

## Pitfall-to-Phase Mapping

How the v7.4 roadmap phases must sequence and verify prevention so regressions cannot compound.

| Pitfall | Prevention Phase | Verification Gate |
|---------|------------------|-------------------|
| **Pitfall 1:** `index.css` `!important` clobbering Radix backdrops, switches, checkboxes & portals | **Phase 1: CSS & Portal Foundation** | Vitest + Playwright visual matrix opening `Dialog`, `Switch`, `Checkbox`, `Select`, `Popover`, `DropdownMenu` across all 4 themes (`arcane`, `slate`, `glacier`, `high-contrast`) |
| **Pitfall 3:** Z-index wars (`z-50` vs `z-[99999]` vs `Toasts` `z-40`), `body[data-scroll-locked]` `outerOverflowPx`, & Radix package dedupe | **Phase 1: CSS & Portal Foundation** | `pnpm dedupe --check`; Playwright test opening nested `Select` inside `Dialog` + firing `Toast` above `Dialog` with `outerOverflowPx === 0` |
| **Pitfall 6:** Incomplete glyph purge, `zero-size-svg`, `button-name` Axe errors & glyph unit-test breakage | **Phase 2: Zero-Glyph & Icon Semiotics** | New `glyphBan.guard.test.ts` + upgraded `audits.ts` `rawEmojiCount === 0` + `iconIssueCount === 0` + `100%` green Vitest |
| **Pitfall 2:** Fake-optimistic `t3Wave3Rpc.ts` fallthroughs & `.catch(() => {})` silent IPC swallows | **Phase 3: Honest IPC & Live Control Wiring** | Unit tests with rejected `invoke` / `{ is_error: true }` asserting state rollback + `sanitizeErrorForToast` toast; `installErrorStateMock` + `checkHonestyInvariant` (`0` `fake_success`) |
| **Pitfall 9:** `tauriMock.ts` / `tauriMockRich.ts` / `tauriMockVariants.ts` desync causing `skeletonCount > 0` | **Phase 3: Honest IPC & Live Control Wiring** | `tauriMockVariants.test.ts` green + `skeletonCount === 0` across all 33 surfaces |
| **Pitfall 7:** Bento & `react-resizable-panels` / `dockview` `0×0` collapse, `min-w-0` overflow & stale `localStorage` | **Phase 4: Dense Bento & Resizable Layouts** | Playwright multi-res sweep (`900×600`..`1920×1080`) + 14-dock cycle (`scrollHostOverflowPx === 0`, `clippedInteractiveCount === 0`) |
| **Pitfall 8:** Bento compaction breaking `chartPanelFillRatio`, `flutedPillarCount`, `friezeHeaderCount`, `viewportCoverageRatio` | **Phase 4 & Phase 6: Bento Layouts & CV Flywheel** | `chartPanelFillRatio >= 0.84`, `flutedPillarCount >= 2`, `friezeHeaderCount === 1`, `vlmOverallScore >= 0.88`, `visualAfterScore >= 0.90` |
| **Pitfall 4:** `<main>` Axe blind spot on `body` portals, `scrollable-region-focusable`, & ARIA slider/knob naming | **Phase 1 (wrappers) & Phase 5 (A11y/Audit Gate)** | Full-page `AxeBuilder({ page }).analyze()` (`0` serious/critical violations with portals open) in both `cv-flywheel.spec.ts` and `capture.spec.ts` |
| **Pitfall 5:** `toestub` (`transition-all`, raw `ResizeObserver`, `hardcoded-abs-dir`, `god_object`), `themeContract`, `ipcBoundaries`, `knip` | **Phases 1–5 (Continuous) & Phase 5 Gate** | `pnpm build && pnpm test && pnpm knip && vox ci gui-catalog-parity && vox ci pre-push` (`0` errors, `0` warnings) |
| **Pitfall 10:** Exit-animation ghost portals & stacked `backdrop-blur` screenshot slowdowns | **Phase 1 & Phase 6: CV Flywheel Convergence** | Full 33-surface CV Flywheel completes well within suite timeout with `overlappingControlPairs === 0` |

---

## Sources

### Codebase Ground Truth (Inspected Live)
- `crates/vox-gui/ui/src/index.css:236-408, 626-1184` — Global Romanesque Carrara Marble (`arcane`/`glacier`) and Minimalist Dark (`slate`/`void`/`high-contrast`) `!important` overrides, `[class*="bg-black"]` scrim trap (`L713`), button role overrides (`L311`, `L1076`), and `white-space: nowrap` invariant (`L1180-1184`).
- `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts:97-796` — `unwrapMcpData` error throwing swallowed by `if (backendAvailable()) { try ... catch { // Fall through } }` fake-optimistic fallbacks.
- `crates/vox-gui/ui/src/debugger/sentries/sentryHonesty.ts:12-54` — `checkHonestyInvariant` `fake_success` critical invariant.
- `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts:141-200, 456-481, 702-750` — `.include('main')` Axe blind spot (`L143`) and hard CV/VLM convergence assertions.
- `crates/vox-gui/ui/e2e/review/capture.spec.ts:80-90` — Full-page `AxeBuilder({ page }).analyze()` check (`moderate`/`serious`/`critical`).
- `crates/vox-gui/ui/e2e/review/audits.ts:13-39, 87-533` — `auditIconsInPage` (`zero-size-svg`), `clippedInteractiveCount`, `rawEmojiCount` selector/regex gap (`L264-281`), `chartPanelFillRatio` (`L340-384`), `skeletonCount` (`L463-472`).
- `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts:214-445` — `hasVisibleMarbleFrieze`, `friezeHeaderCount`, `flutedPillarCount`, `romanFontCoverageRatio`, and 5-query VLM rubric scoring.
- `crates/vox-gui/ui/src/lib/cvFlywheel.ts:304-409` — `viewportCoverageRatio` (`0.45..0.92`), `controlHeightSpreadPx`, and `VisionRubricScores` math.
- `crates/vox-gui/ui/src/components/dock/DockWorkspaceShell.tsx:16-89, 134-162` & `src/styles/dockview-vox.css:1-80` — `dockview@6.6.1` `.dv-shell` `0×0` percentage-height trap, rAF `ResizeObserver` bridge, `closedPanelIds` hazard, and `dndStrategy="pointer"`.
- `crates/vox-gui/ui/src/components/layout/{AppShell.tsx,BottomStatusBar.tsx,SurfaceScrollHost.tsx}` & `src/components/ui/{Dialog.tsx,Tooltip.tsx}` — `z-50` vs `z-[99999]` vs `Toasts.tsx` `z-40` stacking conflicts and `scrollable-region-focusable` pattern.
- `crates/vox-code-audit/src/detectors/{ui_perf_hazards.rs,ui_theme_contract.rs,hardcoded_abs_dir.rs,god_object.rs,test_clock_and_skip.rs,unreasoned_suppression.rs}` — Exact static audit rules enforced by `toestub` / `vox ci pre-push`.
- `crates/vox-gui/ui/src/guards/{ipcBoundaries.test.ts,transportIpcGuard.test.ts,toastBodyGuard.test.ts,commandBackendsMigration.test.ts}`, `src/components/surfaces/__guards__/{honestyScan.ts,surfaceHonesty.guard.test.ts}`, `contracts/gui/command-backends.v1.yaml`, and `crates/vox-gui/ui/knip.jsonc`.

### Verified Ecosystem References (2025/2026)
- npm registry (verified via `pnpm view`): `radix-ui@1.8.1` (unified primitive package superseding fragmented `@radix-ui/react-*`), `react-resizable-panels@4.22.1`, `tw-animate-css@1.4.0`, `cmdk@1.1.1`, paired with workspace `react@19.2.8`, `tailwindcss@^4.3.3`, `dockview@6.6.1`, `lucide-react@^1.39.0`, and `@axe-core/playwright@^4.13.0`.
- shadcn/ui Tailwind v4 & React 19 architecture docs — `data-slot` attribute conventions, native `ref` props replacing `React.forwardRef`, and unified `radix-ui` imports.
- Deque `axe-core` rule specifications — `scrollable-region-focusable`, `aria-input-field-name` (Radix `<Slider.Thumb>` accessible naming), `aria-dialog-name`, `button-name`, and `nested-interactive`.
- Radix UI `DismissableLayer` / `react-remove-scroll` overlay behavior — `data-scroll-locked` body margin injection and nested portal `z-index` / `pointer-events` interactions.

---
*Pitfalls research for: Vox Axis Milestone v7.4 (`axis-live-functional-v74`)*
*Researched: 2026-11-14*
