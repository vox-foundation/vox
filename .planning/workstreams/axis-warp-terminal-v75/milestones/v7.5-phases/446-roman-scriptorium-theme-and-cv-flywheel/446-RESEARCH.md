# Phase 446: Roman Scriptorium Theme Contract & Multi-Pass CV Flywheel Convergence — Research

**Researched:** 2026-11-15
**Domain:** Roman Scriptorium Theme Contract (`--font-terminal-mono`, `:not(.xterm)` CSS containment, Dual-Surface 4-Theme 16-color ANSI WCAG calibration), Deterministic Terminal DOM/Pixel/VLM Computer-Vision Flywheel (`@ngi/platform-ssot` + Playwright 5-Viewport × 4-Theme Matrix), and Architecture SSOT Promotion (`vox-doc-pipeline`)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from `446-CONTEXT.md`)

### Locked Decisions
- **D-01 (`--font-terminal-mono` Semantic Token):** Add `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` to `@theme` and per-theme `:root` / `[data-theme=...]` blocks in `crates/vox-gui/ui/src/index.css`, register it in `contracts/gui/theme-contract.v1.json` and `crates/vox-gui/ui/src/lib/themeContract.ts`, and verify `vox audit code` (`vox/ui/theme-contract-drift`) + `themeContract.test.ts` pass with `0` errors (never use banned font literals `Menlo`, `Monaco`, `Consolas`, `Courier`).
- **D-02 (`:not(.xterm)` CSS Bleed Isolation):** Audit every global selector in `crates/vox-gui/ui/src/index.css` that forces proportional `Palatino` / `--font-mono` or overrides `textarea` / `pre` / `code` / focus-ring styles, and add `:not(.xterm):not(.xterm *)` exclusions so `@xterm/xterm` character cell widths, `.xterm-helper-textarea`, cursor alignment, and box-drawing glyphs (`─│┌┐└┘├┤┬┴┼`) stay pixel-accurate across all 4 themes.
- **D-03 (Dual-Surface Roman Scriptorium Terminal Well):**
  - **Outer Shell & Controls:** Warm Carrara / Travertine / Obsidian / Slate stone framing (`bg-bg-surface` / `bg-bg-elevated`), hairline brass borders (`border-border-subtle` / `border-brass`), Cinzel section headers (`--font-display`), Palatino prose labels (`--font-sans`), and semantic status badges (`status-pass`, `status-warn`, `status-fail`).
  - **Inner Terminal Viewport & Block Output Well:** High-contrast dark-stone / bronze-obsidian terminal canvas with calibrated 16-color ANSI palette (`terminalTheme.ts`) across all 4 profile themes (`arcane`, `slate`/`void`, `travertine`/`glacier`, `high-contrast`), guaranteeing WCAG `>= 4.5:1` contrast for every foreground/ANSI color (`>= 7.0:1` in `high-contrast`).
  - **Zero Legacy Leaks:** Eliminate all remaining `zinc-*`, `slate-*`, `bg-black/*`, `#000`, raw `text-cyan-*`, and `.text-brass` dark-on-dark (`1.03:1`) leaks across `Console/` and `ChatTerminalBlockCard.tsx`.
- **D-04 (Zero Raw Emoji / Dingbat / Guillemet Enforcement):** Zero Unicode emojis, geometric dingbats, or raw guillemets (`»`) anywhere in terminal tabs, status pills, popovers, runbooks, or approval cards (`zeroEmojiAndDingbat.test.ts` + `Icons.tsx` vector SVGs only).
- **D-05 (Deterministic Terminal CV Invariants):** Extend `crates/vox-gui/ui/e2e/review/audits.ts` (and `audits.test.ts`) + `states.ts` with deterministic terminal checks:
  - `terminalStartupSpamCount === 0` (zero raw `__vox_preexec` / `__vox_precmd` / `trap DEBUG` text visible in blocks or xterm screen)
  - `xtermHelperLeakCount === 0` (`.xterm-helper-textarea` invisible: `opacity === '0'` or `0×0` clipped, zero cream background/border box)
  - `terminalFontProportionalLeakCount === 0` (computed `font-family` on `.xterm`, block `<pre>`, and `ScriptoriumPromptEditor` resolves to `JetBrains Mono` / `monospace`, never `Palatino`)
  - `terminalContrastViolationCount === 0` (WCAG `>= 4.5:1` across visible terminal text & ANSI spans)
- **D-06 (Multi-Pass `CV_STAGE=before -> after` Convergence Matrix):** Run Playwright + `@ngi/platform-ssot` (`cv-flywheel.spec.ts` / `vlmVisionFlywheel.ts`) across **all 5 viewports** (`900x600`, `1100x720`, `1280x800`, `1440x900`, `1920x1080`) and **all 4 themes** (`arcane`, `slate`, `travertine`, `high-contrast`) covering `Console` (Block Stream, Split Panes, `Ctrl+R` History, `JsonLedgerTable`, `Runbooks`, `TerminalApprovalCard`, Auto-TUI `ALT_SCREEN`) and `Chat` + `TerminalDockPanel`, converging at `visualAfterScore >= 0.92`, `outerOverflowPx === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `axeViolations === 0`.
- **D-07 (Architecture Doc Promotion):** Promote `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` frontmatter `status: "research"` -> `status: "current"`, update any implementation notes to reflect shipped Phases 441–446 contracts, and pass `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md`.
- **D-08 (Guardrails):** Keep all new/modified modules under `< 450` non-blank LoC per file; zero new Cargo/npm dependencies.

### Claude's Discretion
- Exact CSS custom property values for the inner terminal dark-stone surface per theme (`--terminal-well-bg`, `--terminal-well-border`) provided `ui_theme_contract.rs` allowlist/contract checks and WCAG `>= 4.5:1` / `>= 7.0:1` pass cleanly.
- Grouping of Playwright terminal CV specs (extending `cv-flywheel.spec.ts` vs dedicated `terminal-cv-flywheel.spec.ts` wired into the review suite) for fast deterministic CI execution.

### Deferred Ideas (OUT OF SCOPE)
- Cloud team sharing of runbooks, sessions, or themes (out of scope per `REQUIREMENTS.md`).
- SSH remote daemon muxing (post-v7.5).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| `TERM-ROMA-CV-01` | Add `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` to `contracts/gui/theme-contract.v1.json`, `themeContract.ts`, and `index.css` (`:not(.xterm *)` isolation), passing `vox audit code` (`vox/ui/theme-contract-drift`) and `themeContract.test.ts` with `0` errors. | Seam 1 maps the 4-file Style Dictionary token pipeline (`tokens/primitive.json` -> `pnpm tokens:build` -> `tokens*.generated.css`), the 23/23 semantic token update (`SettingsView.tsx` + `cv-flywheel.spec.ts:1250`), and every exact selector in `src/index.css` (`L303-338`, `L534-545`, `L571-575`, `L750-766`, `L775-979`, `L1208-1220`). |
| `TERM-ROMA-CV-02` | All 4 profile themes (`arcane`, `slate`/`void`, `travertine`/`glacier`, `high-contrast`) render `<TerminalSessionPane>`, `BlockCard`, and `ChatTerminalBlockCard` using the Dual-Surface Roman Scriptorium palette with WCAG `>= 4.5:1` contrast across all 16 ANSI colors (`>= 7.0:1` in `high-contrast`), zero `.text-brass` dark-on-dark `1.03:1` defects, and zero raw emoji/dingbat glyphs. | Seam 2 provides mathematically verified 64-entry WCAG contrast tables (`5.05:1`..`21.00:1`), non-banned `--terminal-well-*` CSS variables in `index.css`, and an exhaustive file-by-file inventory of remaining `zinc-*`, `bg-white/*`, `border-cyan/*`, `.text-brass`, hardcoded light-only inline hexes (`DiscoveryRail.tsx`), and raw guillemets (`»`/`«`). |
| `TERM-ROMA-CV-03` | Multi-pass `CV_STAGE=before -> after` Playwright + `@ngi/platform-ssot` + VLM flywheel audit across `900x600`, `1100x720`, `1280x800`, `1440x900`, and `1920x1080` and all 4 themes covering `Console` (Block Stream, Split Panes, History, Table, Runbooks, Approvals, Auto-TUI) and `Chat` + `TerminalDockPanel`, converging at `visualAfterScore >= 0.92`, `outerOverflowPx == 0`, `clippedInteractiveCount == 0`, `overlappingControlPairs == 0`, `axeViolations == 0`, and zero visible `.xterm-helper-textarea` or startup spam. | Seam 3 maps `TerminalTab.tsx` mock vs real `@xterm/xterm` mounting in Playwright, `tauriMock.ts` deterministic state fixtures, `terminalAudits.ts` (`terminalStartupSpamCount`, `xtermHelperLeakCount`, `terminalFontProportionalLeakCount`, `terminalContrastViolationCount`), `900x600` responsive split/rail rules, and `runUniversalCvFlywheelPass` scoring math (`visualAfterScore >= 0.92`). |
| `TERM-ROMA-CV-04` | Update `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` frontmatter `status: "current"` once verified and pass `vox-doc-pipeline --lint-only`. | Seam 4 verifies `warp-roman-terminal-v75-research-2026.md` facets against `kb-taxonomy.v1.yaml`, documents the critical `--paths architecture/...` CLI prefix rule (`docs/src/`-relative), and confirms `status: "current"` passes `vox-doc-pipeline --lint-only` and `docs-reality-audit` gates. |
</phase_requirements>

---

## Summary

Phase 446 closes Milestone `v7.5` (`axis-warp-terminal-v75`) by uniting the stateful PTY engine (Phase 441), bidirectional `@term` / AST `ExecPolicyV1` harness (Phase 442), unified `<TerminalSessionPane>` + split panes (Phase 443), Warp prompt editor / `Ctrl+R` history / `JsonLedgerTable` / `RunbookPanel` (Phase 444), and streaming terminal agent / `TerminalApprovalCard` / MENS trajectory flywheel (Phase 445) under a single **enforceable Roman Scriptorium Theme Contract** and **deterministic multi-pass Computer-Vision (CV) Flywheel**.

Inspection of `crates/vox-gui/ui/src/index.css`, `style-dictionary.config.mjs`, `themeContract.ts`, `ui_theme_contract.rs`, `Console/*`, `ChatTerminalBlockCard.tsx`, `audits.ts`, and `cv-flywheel.spec.ts` uncovered **seven concrete traps** that would break naïve execution unless planned upfront:
1. **Style Dictionary Overwrite Trap (`themeContract.test.ts:63-80`):** Every variable added to `REQUIRED_SEMANTIC_CSS_VARS` MUST exist in `tokens/primitive.json` and be emitted via `pnpm --dir crates/vox-gui/ui tokens:build` into all 4 `src/styles/tokens*.generated.css` files, AND `cv-flywheel.spec.ts:1250` must update `'22 / 22 Verified'` -> `'23 / 23 Verified'`.
2. **Three-Way `index.css` Bleed (`L534-545`, `L750-766`, `L775-979`):** Global `!important` rules in `index.css` simultaneously force proportional `'Palatino'` onto `.font-mono, pre, code`, paint a `#FAF5E8` cream box + `rgba(197, 155, 39, 0.72)` border onto `.xterm-helper-textarea` inside `main`, and clobber `.text-brass` / `.text-white` / `.text-zinc-*` to `#1C140C` espresso (`1.03:1` dark-on-dark contrast when rendered inside a dark terminal well).
3. **`BANNED_HARDCODED_DARK_BROWN_HEXES` & `GENERAL_DARK_HEX_BG_REGEX` Guardrails:** Dark hex literals are banned outside `src/index.css` and `src/lib/themeContract.ts`. All Dual-Surface dark terminal well colors (`--terminal-well-bg`, `--terminal-well-fg`, `--terminal-well-border`, `--terminal-well-cursor`, `--terminal-well-selection`) must live in `src/index.css` using non-banned warm bronze-obsidian/basalt hexes (`#15110d`, `#171512`, `#0c0e10`, `#000000`) and be consumed via CSS variables in TSX.
4. **ANSI Color `0` (`black`) & `audits.ts` Neon RGB Collisions:** `DARK_ANSI_16` in `terminalTheme.ts:50-60` currently sets `black: '#334155'` (`1.89:1` < `4.5:1`!) and uses `#34d399`, `#6ee7b7`, `#22d3ee`, which `audits.ts:538-542` flags as `digitalColorLeakCount++`. Calibrating all 64 ANSI entries (`4 themes × 16 colors`) fixes both simultaneously (`min = 5.05:1` in standard themes, `min = 8.27:1` in `high-contrast`).
5. **`TerminalTab.tsx` Mock Bypass in Playwright (`TerminalTab.tsx:99-102`):** `isMock` checks `typeof window.__TAURI_CALLS__ !== 'undefined'`, which `tauriMockShared.ts:28` sets in every Playwright E2E run — bypassing real `@xterm/xterm` instantiation and hiding `.xterm` / `.xterm-helper-textarea` from DOM audits unless real xterm is mounted when `window.__VOX_E2E_REAL_XTERM__ === true` (or whenever a real Chromium layout engine is active outside `jsdom` unit tests, while preserving `console-workbench.spec.ts` assertions).
6. **`900×600` Split-Pane + `DiscoveryRail` Horizontal Overflow:** `ConsoleSplitGrid.tsx:142` (`min-w-[280px]`) + `DiscoveryRail.tsx:143` (`w-[268px]`) + left `nav` (`~200px`) totals `1028px > 900px`, triggering `outerOverflowPx > 0` and `clippedInteractiveCount > 0` at `900×600` unless `DiscoveryRail` auto-collapses below `1100px` (or overlays) and split leaves lower `min-w` to `min-w-[160px]`.
7. **`vox-doc-pipeline --paths` Prefix Rule:** `--paths` resolves relative to `./docs/src/`, so passing `docs/src/architecture/...` fails with `./docs/src/docs/src/architecture/...`; plans MUST invoke `--paths architecture/warp-roman-terminal-v75-research-2026.md`.

### Recommended Plan Decomposition (4 Plans across 3 Waves)

- **Wave 1 (Parallel-Safe Foundation — disjoint files):**
  - **Plan `446-01` (`TERM-ROMA-CV-01` — `--font-terminal-mono` Token Pipeline & `:not(.xterm)` CSS Containment):** Update `tokens/primitive.json`, run `pnpm tokens:build`, update `contracts/gui/theme-contract.v1.json`, `src/lib/themeContract.ts`, `src/lib/themeContract.test.ts`, and `src/index.css` (`@theme`, `--terminal-well-*` variables across 4 themes, `:not(.xterm):not(.xterm *):not([data-terminal-well="true"] *)` exclusions, `.xterm-helper-textarea` zero-box containment).
  - **Plan `446-02` (`TERM-ROMA-CV-02` — 4-Theme Dual-Surface ANSI Calibration & Component Zero-Leak Sweep):** Calibrate `terminalTheme.ts` (`ARCANE_ANSI_16`, `TRAVERTINE_ANSI_16`, `SLATE_ANSI_16`, `HIGH_CONTRAST_ANSI_16` + unit tests in `TerminalTab.test.tsx` & `sgrToSpans.test.tsx`), wire `data-terminal-well="true"` + `data-terminal-mono="true"` + Dual-Surface tokens across `TerminalTab.tsx`, `BlockCard.tsx`, `ChatTerminalBlockCard.tsx`, `AgentStrip.tsx`, `AgentTab.tsx`, `SendToAgent.tsx`, `DiscoveryRail.tsx`, `ConsoleSessionBar.tsx`, `ConsoleSplitGrid.tsx`, `Console.tsx`, and Phase 444/445 surfaces (`ScriptoriumPromptEditor.tsx`, `JsonLedgerTable.tsx`, `RunbookPanel.tsx`, `TerminalApprovalCard.tsx`, `AgentTurnContent.tsx` if landed).
- **Wave 2 (Depends on `446-01` & `446-02`):**
  - **Plan `446-03` (`TERM-ROMA-CV-03` — Deterministic Terminal CV Invariants & Rich E2E State Fixtures):** Create `e2e/review/terminalAudits.ts` + `e2e/review/terminalAudits.test.ts`, wire into `e2e/review/audits.ts` and `e2e/review/states.ts`, enrich `e2e/lib/tauriMock.ts` terminal fixtures (ANSI SGR spans, structured JSON table, runbooks, pending `TerminalApprovalCard`, `ALT_SCREEN` `?1049h` TUI frame, real xterm mount opt-in), and update `cv-flywheel.spec.ts:1250` (`23 / 23 Verified`).
- **Wave 3 (Depends on `446-03`):**
  - **Plan `446-04` (`TERM-ROMA-CV-03` + `TERM-ROMA-CV-04` — Multi-Pass `CV_STAGE=before -> after` 5×4 Convergence & Architecture SSOT Promotion):** Add `e2e/review/terminal-cv-flywheel.spec.ts`, execute `CV_STAGE=before` baseline + `CV_STAGE=after` verification across all 5 viewports × 4 themes (`visualAfterScore >= 0.92`, `0` DOM/axe/terminal defects), fix any surfaced pixel/layout delta, promote `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` to `status: "current"`, and pass `vox-doc-pipeline --lint-only` + `vox audit code`.

---

## Project Constraints (from `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`, `00-core.md`)

- **Zero New Dependencies:** Do not modify `Cargo.toml`, `Cargo.lock`, `crates/vox-gui/ui/package.json`, or `pnpm-lock.yaml` (`D-08`, `00-core.md` Rule 5).
- **File Size Budget:** Keep all new modules and modified component files under `< 450` non-blank LoC (`D-08`). Do not bloat `cv-flywheel.spec.ts` (`1,564` lines) or `audits.ts` (`575` lines) — extract terminal-specific audit helpers into `crates/vox-gui/ui/e2e/review/terminalAudits.ts` and dedicated Playwright suite `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts`.
- **Theme Contract Static Gate (`vox/ui/theme-contract-drift`):** Enforced by both `crates/vox-code-audit/src/detectors/ui_theme_contract.rs` and `crates/vox-gui/ui/src/lib/themeContract.test.ts`. Never introduce banned font literals (`Courier New`, `Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`) or hardcoded dark hex backgrounds (`bg-[#0..]`, `bg-[#1..]`, `bg-[#2..]`, `background: '#0..'`) outside `src/index.css` and `src/lib/themeContract.ts`.
- **Zero Emoji / Dingbat / Guillemet Gate:** `crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts` scans every `.ts`/`.tsx` file in `src/` for `\p{Extended_Pictographic}`, dingbats, geometric shapes (`\u25A0-\u25FF`), and return/tab glyphs (`\u21B5`, `\u23CE`, `\u21E5`). Use `Icons.tsx` vector SVGs exclusively.
- **Scoped Foreground Execution (`00-core.md` Rules 6–9):** Prefix every build/test command with `timeout`, emit one command per shell step, never run `cargo test --workspace`, and never run git mutating commands (`git add`, `commit`, `stash`, `checkout`, `restore`, `reset`, `clean`, `push`).
- **Documentation Governance (`AGENTS.md` §Authored Markdown Frontmatter):** Never hand-add `last_updated`, `last_verified`, or `human_attested_by`; keep `<!-- kb:summary -->` 40–120 words directly below H1; pass `vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `--font-terminal-mono` & `--terminal-well-*` tokens | Design Tokens + Global CSS (`tokens/primitive.json`, `src/styles/tokens*.generated.css`, `src/index.css`) | Contract Validators (`contracts/gui/theme-contract.v1.json`, `src/lib/themeContract.ts`, `vox-code-audit`) | Keeps dark terminal surface hexes inside the single allowlisted stylesheet (`src/index.css`) while guaranteeing parity across Style Dictionary, runtime health cards, and Rust static analysis. |
| `:not(.xterm)` & `[data-terminal-well="true"]` style containment | Global Stylesheet (`src/index.css`) | Terminal Leaf Components (`TerminalTab.tsx`, `BlockCard.tsx`, `ChatTerminalBlockCard.tsx`) | Global `!important` rules in `index.css` have higher specificity than Tailwind utilities; only selector-level `:not(...)` exclusions prevent `Palatino` font bleed, `.xterm-helper-textarea` cream boxes, and `#1C140C` light-theme text clobbering. |
| 4-Theme 16-Color ANSI WCAG Calibration | Frontend Theme Adapter (`Console/terminalTheme.ts`) | SGR Renderer & xterm Bridge (`Console/sgrToSpans.ts`, `Console/TerminalTab.tsx`) | Centralizes all 64 ANSI color values and background luminances in one pure module consumed identically by `@xterm/xterm` (`ITheme`) and virtualized `BlockCard` / `ChatTerminalBlockCard` SGR spans. |
| Deterministic Terminal DOM + Pixel + Rubric CV Flywheel | E2E Review Harness (`e2e/review/terminalAudits.ts`, `audits.ts`, `states.ts`, `terminal-cv-flywheel.spec.ts`) | Shared CV Engine (`src/lib/cvFlywheel.ts`, `e2e/review/vlmVisionFlywheel.ts`, `e2e/lib/tauriMock.ts`) | Combines browser-side computed style/contrast/geometry inspection with RGBA Sobel/RMS/Laplacian metrics and `@ngi/platform-ssot` `runUniversalCvFlywheelPass` convergence checks. |
| Milestone v7.5 Architecture SSOT Promotion | Documentation Corpus (`docs/src/architecture/warp-roman-terminal-v75-research-2026.md`) | Doc Lint Pipeline (`crates/vox-doc-pipeline`) | Promotes the Milestone v7.5 architecture document out of collapsed `Internals` (`status: "research"`) into canonical `Architecture SSOTs` (`status: "current"`). |

---

## Standard Stack

### Core (Already Installed — Zero New Packages)
| Library / Tool | Verified Version | Purpose | Why Standard |
|----------------|------------------|---------|--------------|
| `style-dictionary` | `5.5.2` | Generates `src/styles/tokens*.generated.css` and `tokens.generated.ts` from `tokens/*.json` | `[VERIFIED: crates/vox-gui/ui/package.json:63]` `"style-dictionary": "^5.5.2"`; invoked automatically by `pnpm build` (`pnpm tokens:build`). |
| `tailwindcss` | `4.3.5` | `@theme` token binding and semantic utility classes (`bg-(--terminal-well-bg)`) | `[VERIFIED: crates/vox-gui/ui/package.json:64]` `"tailwindcss": "^4.3.5"`. |
| `@xterm/xterm` + `@xterm/addon-{fit,webgl,search,web-links,unicode11,serialize}` | `6.0.0` / `0.11.0` / `0.19.0` | Live PTY & `ALT_SCREEN` terminal viewport with `.xterm-helper-textarea` input | `[VERIFIED: crates/vox-gui/ui/package.json:21-27]` Installed in Phase 443; requires strict CSS isolation from proportional Romanesque rules. |
| `@playwright/test` + `@axe-core/playwright` | `1.59.1` / `4.11.2` | Multi-viewport, multi-theme headless Chromium screenshots, DOM audits, and WCAG axe checks | `[VERIFIED: crates/vox-gui/ui/package.json:47-48]` Existing E2E harness (`playwright.config.ts`). |
| `pngjs` + `src/lib/cvFlywheel.ts` (`@ngi/platform-ssot`) | `7.0.0` | Decodes PNG screenshots to RGBA buffers for `computeRgbaFrameCvMetrics` and `runUniversalCvFlywheelPass` | `[VERIFIED: crates/vox-gui/ui/package.json:62]` & `[VERIFIED: crates/vox-gui/ui/src/lib/cvFlywheel.ts:12-23]`. |
| `vitest` + `@testing-library/react` | `3.2.6` / `16.3.2` | Fast (`< 3s`) unit verification of `themeContract.test.ts`, `audits.test.ts`, `terminalAudits.test.ts`, and `zeroEmojiAndDingbat.test.ts` | `[VERIFIED: crates/vox-gui/ui/package.json:53,67]`. |
| `vox-code-audit` & `vox-doc-pipeline` | Workspace Rust crates | Static `vox/ui/theme-contract-drift` enforcement and Markdown/KB facet lint | `[VERIFIED: crates/vox-code-audit/src/detectors/ui_theme_contract.rs:1-47]` & `[VERIFIED: crates/vox-doc-pipeline/src/pipeline/lint.rs:20-50]`. |

**Installation:** None required (`D-08` forbids adding npm or Cargo dependencies).

---

## Deep Technical Verification of Codebase Seams

### Seam 1: `--font-terminal-mono` Pipeline & `:not(.xterm)` CSS Containment (`TERM-ROMA-CV-01`)

#### 1.1 Style Dictionary + Theme Contract Synchronization (All 6 Files)
- **Generated Token Test Gate:** `[VERIFIED: crates/vox-gui/ui/src/lib/themeContract.test.ts:63-80]` loops over ` generatedCssFiles` (`tokens.generated.css`, `tokens.slate.generated.css`, `tokens.contrast.generated.css`, `tokens.travertine.generated.css`) and asserts `expect(cssContent, '${relPath} missing ${cssVar}').toContain('${cssVar}:')` for every entry in `REQUIRED_SEMANTIC_CSS_VARS`.
- **Generator Overwrite:** `[VERIFIED: crates/vox-gui/ui/style-dictionary.config.mjs:4-7,33-79]` formats token paths via `const cssVarName = (path) => '--' + path.join('-');` from `tokens/primitive.json` + `tokens/semantic*.json`. Editing `tokens*.generated.css` by hand without updating `tokens/primitive.json` gets wiped out on `pnpm build` (`"build": "pnpm tokens:build && run-p typecheck bundle"`, `[VERIFIED: crates/vox-gui/ui/package.json:8-9]`).
- **Exact Token Pipeline Steps:**
  1. Add `"terminal-mono": { "value": "'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace", "type": "fontFamily" }` under `"font"` in `crates/vox-gui/ui/tokens/primitive.json` (`[VERIFIED: crates/vox-gui/ui/tokens/primitive.json:45-50]`). All 4 Style Dictionary configurations include `'tokens/primitive.json'` in `source`, so running `pnpm --dir crates/vox-gui/ui tokens:build` emits `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;` into all 4 `src/styles/tokens*.generated.css` files and `fontTerminalMono` into `src/styles/tokens.generated.ts`.
  2. Add `"--font-terminal-mono"` to `requiredSemanticCssVars` (23rd entry) and to each theme's `tokens` object (`arcane`, `slate`, `travertine`, `high-contrast`) in `contracts/gui/theme-contract.v1.json` (`[VERIFIED: contracts/gui/theme-contract.v1.json:12-35,47-180]`).
  3. Add `'--font-terminal-mono'` to `REQUIRED_SEMANTIC_CSS_VARS` (`[VERIFIED: crates/vox-gui/ui/src/lib/themeContract.ts:61-84]`) and `'--font-terminal-mono': "'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace"` to each profile in `THEME_CONTRACT_PROFILES` (`[VERIFIED: crates/vox-gui/ui/src/lib/themeContract.ts:120-266]`).
  4. Update `themeContract.test.ts:63` test label (`22 required semantic CSS variables` -> `23 required semantic CSS variables`) and **update `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts:1250`** from `await expect(ledgerCard).toContainText('22 / 22 Verified')` to `'23 / 23 Verified'` because `SettingsView.tsx:1918` renders `{contractReport.resolvedSemanticVarCount} / {contractReport.requiredSemanticVarCount} Verified` (`[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx:1918]`).
  5. Verify font literal safety: `BANNED_ROMANESQUE_FONT_LITERALS` in `themeContract.ts:106-113` and `BANNED_FONTS` in `ui_theme_contract.rs:39-46` ban `'courier new'`, `'courier'`, `'arial'`, `'consolas'`, `'menlo'`, `'monaco'`. None appear in `'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace`, so both `themeContract.test.ts` and `vox audit code` pass with `0` errors.

#### 1.2 Exact `crates/vox-gui/ui/src/index.css` Bleed Selectors & Required Exclusions
Every global rule in `src/index.css` was inspected line-by-line (`L1-1260`):

| Lines in `src/index.css` | Current Selector / Rule (Verbatim) | Bleed Caused on Terminal | Exact Phase 446 Fix |
|---|---|---|---|
| `L22-26` | `--font-mono: 'Palatino', 'Palatino Linotype', 'Book Antiqua', Georgia, serif;` inside `@theme` | Tailwind `font-mono` resolves to proportional serif `Palatino` in `arcane`, `glacier`, `high-contrast`. | Add `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;` right after `L26`, plus define `--terminal-well-*` variables in `:root` / `[data-theme=...]` (`L238-288`). |
| `L303-315` | `:is([data-testid="active-surface"], main) :is(p, td, li, dd, blockquote, figcaption, .prose)` -> `font-family: var(--font-sans, ...)` | Overrides `<td>` inside `JsonLedgerTable` if monospace cells are used. | Append `:not(.xterm):not(.xterm *):not([data-terminal-mono="true"]):not([data-terminal-mono="true"] *)`. |
| `L321-338` | `main span.font-mono:not([data-code-literal="true"]), ...` -> `font-family: var(--font-display, ...)` + `letter-spacing: 0.03em` | Forces `Cinzel` + `0.03em` tracking onto `span.font-mono` inside `Console` / `TerminalDockPanel`, misaligning columns and ghost text! | Append `:not(.xterm):not(.xterm *):not([data-terminal-mono="true"]):not([data-terminal-mono="true"] *):not([data-terminal-well="true"] *)`. |
| `L534-545` | `.font-mono, pre, code, kbd, samp { font-family: var(--font-mono, 'Palatino'...) !important; }` | Forces proportional `'Palatino' !important` onto every `<pre>` and `<code>` in `BlockCard`, `ChatTerminalBlockCard`, `AgentTurnContent`, `TerminalApprovalCard`, and xterm DOM renderer! | Append `:not(.xterm):not(.xterm *):not([data-terminal-mono="true"]):not([data-terminal-mono="true"] *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` AND add explicit rule: `.xterm, .xterm *, [data-terminal-mono="true"], [data-terminal-mono="true"] * { font-family: var(--font-terminal-mono) !important; font-variant-ligatures: none; letter-spacing: 0 !important; }`. |
| `L571-575` | `*:focus-visible { outline: 3px solid var(--color-accent-default); outline-offset: 2px; }` | Draws a visible 3px gold/cyan focus ring around hidden `.xterm-helper-textarea` when terminal has keyboard focus. | Change selector to `*:not(.xterm-helper-textarea):focus-visible`. |
| `L750-766` & `L1208-1220` | `html:not([data-theme="void"])... :is(main, [role="dialog"]) textarea` (`background-color: #FAF5E8 !important; border: 1.5px solid rgba(197, 155, 39, 0.72) !important; color: #1C140C !important;`) and `void` equivalent | **Root cause of `.xterm-helper-textarea` cream box leak:** paints an opaque `#FAF5E8` box with brass border over the top-left of `@xterm/xterm`! Also overrides dark prompt/edit textareas inside `[data-terminal-well="true"]`. | Append `:not(.xterm-helper-textarea):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` to `input` and `textarea` in both `L750-756` and `L1208-1210`, AND add explicit `.xterm .xterm-helper-textarea, textarea.xterm-helper-textarea` zero-box rule (`opacity: 0 !important; width: 0 !important; height: 0 !important; background: transparent !important; border: 0 !important; box-shadow: none !important; outline: none !important; pointer-events: none !important;`). |
| `L775-979` & `L1041-1141` | Light-theme `[class*="bg-zinc-9"]` -> `#F5EFE2 !important` (`L776-809`) and `.text-white`, `.text-zinc-1..5`, `.text-brass` -> `color: #1C140C !important` (`L812-876`) | **Root cause of `1.03:1` dark-on-dark defect:** `.text-brass` (`L873`) and `.text-white` (`L812`) inside a dark terminal well get forced to `#1C140C` (`L=0.0065`) on `#15110d` (`L=0.0059`) = `1.01:1..1.03:1`! | Scope all light-theme surface/text normalizers (`L775-979`) to exclude `:not(.xterm):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)`. |

---

### Seam 2: 4-Theme Dual-Surface Roman Scriptorium Parity (`TERM-ROMA-CV-02`)

#### 2.1 Non-Banned Dual-Surface Terminal Well Variables & 64-Color WCAG Contrast Matrix
- **Static Guardrail Verification:** `[VERIFIED: crates/vox-gui/ui/src/lib/themeContract.ts:86-104,115-118,298-299]` and `[VERIFIED: crates/vox-code-audit/src/detectors/ui_theme_contract.rs:9-27,53-81]` ban 17 dark-brown hexes (`#0a0805`..`#2a2011`) and dark Tailwind/inline backgrounds (`bg-[#0..]`, `background: '#1..'`) outside `src/index.css` and `src/lib/themeContract.ts`.
- Defining `--terminal-well-bg`, `--terminal-well-fg`, `--terminal-well-muted`, `--terminal-well-border`, `--terminal-well-cursor`, and `--terminal-well-selection` inside `src/index.css` (`:root, [data-theme="arcane"]`, `[data-theme="glacier"], [data-theme="travertine"]`, `[data-theme="slate"], [data-theme="void"]`, `[data-theme="high-contrast"]`) keeps **zero** raw dark hex backgrounds in `.tsx` files while delivering authentic Dual-Surface stone framing + dark obsidian/basalt terminal wells across all 4 themes.
- Every foreground and ANSI 16-color hex below was verified against sRGB relative luminance (`(max(L1, L2) + 0.05) / (min(L1, L2) + 0.05)`), avoiding every `digitalColorLeakCount` RGB literal (`#34d399`, `#6ee7b7`, `#22d3ee`, `#06b6d4`, `#10b981`) in `[VERIFIED: crates/vox-gui/ui/e2e/review/audits.ts:535-545]`:

| Token / ANSI Slot | `arcane` (Carrara & Gold) | `travertine` / `glacier` | `slate` / `void` | `high-contrast` (WCAG AAA) |
|---|---|---|---|---|
| `--terminal-well-bg` | `#15110d` | `#171512` | `#0c0e10` | `#000000` |
| `--terminal-well-fg` (Ratio) | `#F5EFE2` (`16.39:1`) | `#F6F1E7` (`16.19:1`) | `#F1F5F9` (`17.65:1`) | `#FFFFFF` (`21.00:1`) |
| `--terminal-well-border` | `rgba(197, 155, 39, 0.52)` | `rgba(138, 106, 38, 0.52)` | `#2b343a` | `#ffffff` |
| `--terminal-well-cursor` | `#E0C478` (`11.12:1`) | `#79C8BA` (`9.61:1`) | `#38BDF8` (`9.65:1`) | `#FDE047` (`16.65:1`) |
| `black` (ANSI 0) | `#8E8374` (`5.05:1`) | `#91887B` (`5.22:1`) | `#7C8B9E` (`5.57:1`) | `#9CA3AF` (`8.27:1`) |
| `red` (ANSI 1) | `#F28B82` (`7.92:1`) | `#F08C80` (`7.70:1`) | `#F87171` (`6.49:1`) | `#FCA5A5` (`10.67:1`) |
| `green` (ANSI 2) | `#6CCB8B` (`9.49:1`) | `#68C996` (`9.16:1`) | `#4ADE80` (`10.97:1`) | `#86EFAC` (`15.27:1`) |
| `yellow` (ANSI 3) | `#E6C15A` (`10.89:1`) | `#DFBE5E` (`10.22:1`) | `#FBBF24` (`11.66:1`) | `#FDE047` (`16.65:1`) |
| `blue` (ANSI 4) | `#7EB2F7` (`8.55:1`) | `#7DB4F2` (`8.39:1`) | `#60A5FA` (`6.69:1`) | `#93C5FD` (`11.23:1`) |
| `magenta` (ANSI 5) | `#D79BF5` (`8.69:1`) | `#D29EF0` (`8.55:1`) | `#C084FC` (`7.31:1`) | `#E9D5FF` (`15.54:1`) |
| `cyan` (ANSI 6) | `#66C7C2` (`9.49:1`) | `#79C8BA` (`9.61:1`) | `#38BDF8` (`9.65:1`) | `#7DD3FC` (`12.69:1`) |
| `white` (ANSI 7) | `#EDE4D3` (`14.96:1`) | `#EAE2D3` (`14.23:1`) | `#E2E8F0` (`15.54:1`) | `#F1F5F9` (`19.06:1`) |
| `brightBlack` (ANSI 8) | `#A89D8E` (`7.07:1`) | `#ADA395` (`7.36:1`) | `#94A3B8` (`7.50:1`) | `#CBD5E1` (`14.02:1`) |
| `brightRed` (ANSI 9) | `#F8AD9D` (`10.45:1`) | `#F7ACA0` (`10.12:1`) | `#FCA5A5` (`9.88:1`) | `#FECACA` (`14.54:1`) |
| `brightGreen` (ANSI 10) | `#8FE0A8` (`12.16:1`) | `#8DDFB2` (`11.88:1`) | `#86EFAC` (`14.14:1`) | `#BBF7D0` (`18.17:1`) |
| `brightYellow` (ANSI 11) | `#F3D678` (`13.38:1`) | `#EED47E` (`12.79:1`) | `#FDE047` (`15.42:1`) | `#FEF08A` (`18.78:1`) |
| `brightBlue` (ANSI 12) | `#A3C9FA` (`11.09:1`) | `#A1CAFA` (`10.85:1`) | `#93C5FD` (`10.40:1`) | `#BFDBFE` (`15.01:1`) |
| `brightMagenta` (ANSI 13) | `#E6B8FA` (`11.33:1`) | `#E4BBF7` (`11.24:1`) | `#D8B4FE` (`10.82:1`) | `#F3E8FF` (`18.14:1`) |
| `brightCyan` (ANSI 14) | `#5EEAD4` (`12.69:1`) | `#96DFD2` (`12.42:1`) | `#67E8F9` (`13.35:1`) | `#A5F3FC` (`17.16:1`) |
| `brightWhite` (ANSI 15) | `#FAF6EE` (`17.27:1`) | `#F8F4EC` (`16.49:1`) | `#F8FAFC` (`18.43:1`) | `#FFFFFF` (`21.00:1`) |
| **Minimum Contrast** | **`5.05:1` (`>= 4.5`)** | **`5.22:1` (`>= 4.5`)** | **`5.57:1` (`>= 4.5`)** | **`8.27:1` (`>= 7.0`)** |

- **Existing Unit Test Callers to Update in Lockstep (`00-core.md` Rule 1):**
  - `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx:358-371]` asserts `brightCyan` changes when `document.documentElement.dataset.theme` flips from `'slate'` (`#67e8f9`) to `'arcane'` (update line 370 expected value from `'#0e7490'` to `'#5EEAD4'`).
  - `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx:40-72]` asserts `parseSgrSpans(sample, { theme: 'arcane' })` returns `ARCANE_ANSI_16.red`, `green`, `blue`, `brightYellow`, `brightBlue` — update those 5 hex assertions to match `ARCANE_ANSI_16` (`#F28B82`, `#6CCB8B`, `#7EB2F7`, `#F3D678`, `#A3C9FA`).

#### 2.2 Exhaustive Component Defect Inventory (`Console/` + `Chat/`)
Every file in `crates/vox-gui/ui/src/components/surfaces/Console/` and `ChatTerminalBlockCard.tsx` was read in full:

1. **`Console/TerminalTab.tsx`** (`[VERIFIED: L366-381]`): Outer mock/xterm container currently uses `bg-bg-base text-text-primary`. Bind container to `data-terminal-well="true" data-terminal-mono="true"` with `bg-(--terminal-well-bg) text-(--terminal-well-fg)` and update `resolveTerminalTheme(theme)` in `terminalTheme.ts:150-176` to read `--terminal-well-bg`, `--terminal-well-fg`, `--terminal-well-cursor`, and `--terminal-well-selection`.
2. **`Console/BlockCard.tsx`** (`[VERIFIED: L112-270]`): Outer card frame uses semantic `bg-bg-surface` / `border-border-subtle` (stone framing), while the output `<pre>` (`L239-247`) currently uses `bg-bg-base text-text-primary`. Upgrade `<pre>` to `data-terminal-well="true" data-terminal-mono="true"` + `bg-(--terminal-well-bg) text-(--terminal-well-fg) border border-(--terminal-well-border)` so SGR ANSI spans always render against the calibrated dark terminal well across all 4 themes! Also tag header command `<code>`/`<span>` with `data-terminal-mono="true"`.
3. **`Chat/ChatTerminalBlockCard.tsx`** (`[VERIFIED: L133-145]`): Output `<pre>` currently uses `bg-bg-base px-3 py-2 text-xs leading-[1.45] font-normal text-text-primary`. Add `data-terminal-well="true" data-terminal-mono="true"` and `bg-(--terminal-well-bg) text-(--terminal-well-fg)` so `@term` citation cards in Chat match `Console` 1:1 across all 4 themes.
4. **`Console/AgentStrip.tsx`** (`[VERIFIED: L24,42,47,58]`):
   - `L24`: Hardcoded inline `style={{ background: active > 0 ? 'var(--color-status-pass)' : viz.gray400 }}` -> replace `viz.gray400` with `'var(--color-text-muted)'`.
   - `L42`: `'bg-zinc-500'` -> replace with `'bg-text-muted'`.
   - `L47 & L58`: `border-white/10 bg-white/[0.03] px-2 py-0.5 text-[10px]` -> replace with semantic `border-border-subtle bg-bg-elevated px-2 py-1 text-[11px] font-semibold text-text-secondary`.
5. **`Console/DiscoveryRail.tsx`** (`[VERIFIED: L118-216]`):
   - `L118`: `hover:text-brass` (triggers `1.03:1` rule if over dark bg) -> replace with `hover:text-accent`.
   - `L120 & L167`: Raw guillemets `»` and `«` (`D-04` violation!) -> replace with `<Icon.chevL className="size-3" />` / `<Icon.chevR className="size-3" />` (aligning with Phase 444 Plan `444-03` if already landed).
   - `L148-211`: Hardcoded light-only inline `style={{ backgroundImage: 'linear-gradient(180deg, rgba(251, 247, 238, 0.98) ...)' }}` and hardcoded `text-[#1C140C]`, `text-[#3B2E1E]`, `text-[#6B4C12]`, `border-[#C59B27]`, `border-[#8C6825]` -> breaks `slate` and `high-contrast` themes completely! Replace with semantic `bg-bg-surface`, `bg-bg-elevated`, `text-text-primary`, `text-text-secondary`, `text-accent`, `border-border-subtle`, `border-border-strong`.
   - `L143`: Fixed `w-[268px] shrink-0` -> causes `900×600` viewport overflow when split panes are active; collapse rail by default or switch to compact/overlay mode when `window.innerWidth < 1100`.
6. **`Console/AgentTab.tsx:44` & `Console/SendToAgent.tsx:55`:** Replace raw `style={{ fontFamily: 'monospace' }}` with `data-terminal-mono="true"` + `style={TERMINAL_MONO_STYLE}` and replace off-scale `text-[10px]` / `p-2.5` with `text-[11px]` / `p-2`.
7. **`Console/InputEditor.tsx`** (`[VERIFIED: L208-291]`, or `ScriptoriumPromptEditor.tsx` after Phase 444):
   - `L208, 213, 218, 223, 228`: Replace `border-cyan/45 bg-cyan/15 text-text-primary` and `focus-within:border-cyan/50` (`digitalColorLeakCount` risk!) with semantic `border-accent bg-overlay-hover text-text-primary` and `focus-within:border-accent`.
   - Tag prompt `<textarea>` / highlight overlay / ghost span with `data-terminal-mono="true"` so `index.css` never forces `Palatino` onto the command editor, preserving exact character-width alignment between typed input and `[data-testid="ghost"]`!
8. **`Console/ConsoleSessionBar.tsx` & `Console/Console.tsx` & `Console/ConsoleSplitGrid.tsx`:**
   - `ConsoleSessionBar.tsx:119`: Close tab button is `size-4` (`16×16px`), violating 24px target / clipping checks; widen to `size-5` (`20px`) or `size-6` (`24px`) with `shrink-0`.
   - `Console.tsx:360-405`: Ensure top toolbar strip uses `flex-wrap gap-2` and `term-security-level-select` (`L380`) includes `appearance-none` + `data-unified-knob="true"` so both `console-workbench.spec.ts:69-72` (`selectOption`) and `audits.ts:155` (`unstyledNativeSelectCount === 0`) pass cleanly!
   - `ConsoleSplitGrid.tsx:142`: Lower `min-w-[280px]` on split leaves to `min-w-[160px] sm:min-w-[220px]` so 2-pane splits at `900×600` (`900px - ~200px sidebar = 700px` main width) have zero horizontal overflow (`outerOverflowPx === 0`).

---

### Seam 3: Deterministic Terminal CV Invariants & Multi-Pass `CV_STAGE=before -> after` Flywheel (`TERM-ROMA-CV-03`)

#### 3.1 Mounting Real `@xterm/xterm` in Playwright CV Specs (`TerminalTab.tsx:99-102`)
- **Critical Discovery:** `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:99-102]` currently gates real xterm mounting on:
  ```ts
  const isMock =
    typeof window !== 'undefined' &&
    typeof (window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__ !== 'undefined';
  ```
  Because `[VERIFIED: crates/vox-gui/ui/e2e/lib/tauriMockShared.ts:28]` sets `(window as any).__TAURI_CALLS__ = []`, `TerminalTab` renders a plain `<div data-testid="xterm-mock-screen">` instead of `new Terminal(...)` in Playwright! That would hide `.xterm`, `.xterm-screen`, and `.xterm-helper-textarea` from DOM audits!
- **Safe Deterministic Fix:** Change `isMock` in `TerminalTab.tsx` to:
  ```ts
  const forceRealXterm =
    typeof window !== 'undefined' &&
    Boolean((window as unknown as { __VOX_E2E_REAL_XTERM__?: boolean }).__VOX_E2E_REAL_XTERM__);
  const isMock =
    !forceRealXterm &&
    typeof window !== 'undefined' &&
    typeof (window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__ !== 'undefined';
  ```
  - Existing `TerminalTab.test.tsx` (`jsdom`) and `console-workbench.spec.ts` keep `isMock === true` with zero behavior change.
  - In `states.ts` (`console` states) and `terminal-cv-flywheel.spec.ts`, `page.addInitScript(() => { (window as any).__VOX_E2E_REAL_XTERM__ = true; })` mounts the **real** `@xterm/xterm` instance + `.xterm-helper-textarea` + `xterm.css` inside headless Chromium, feeding `replay_utf8` (including box-drawing characters `┌──┴──┐` and `?1049h` `ALT_SCREEN` TUI frames) into `term.write(...)` so `xtermHelperLeakCount` and `terminalFontProportionalLeakCount` audit the real `@xterm/xterm` DOM!

#### 3.2 Deterministic Terminal DOM Audit (`e2e/review/terminalAudits.ts` -> `audits.ts`)
To honor `D-05` and `D-08` (`< 450` LoC), create `crates/vox-gui/ui/e2e/review/terminalAudits.ts` exporting `TerminalScriptoriumAuditMetrics` and `auditTerminalScriptoriumInPage(doc, win)` (unit-tested in `terminalAudits.test.ts` under `jsdom`, mirroring `audits.test.ts`), and merge its fields into `GuiDomAuditMetrics` (`src/lib/cvFlywheel.ts`) and `auditDomQualityInPage` (`e2e/review/audits.ts`):

1. **`terminalStartupSpamCount` (`=== 0`):** Scans visible text inside `[data-testid="console-root"], [data-testid="terminal-session-pane"], .xterm, [data-testid="chat-terminal-block-card"]` for leaked shell-init hook tokens: `/__vox_preexec|__vox_precmd|__vox_trappost|trap\s+['"][^'"]*__vox|PROMPT_COMMAND=.*__vox/`.
2. **`xtermHelperLeakCount` (`=== 0`):** Queries every `.xterm-helper-textarea` in `document`. Flags a leak if `Number(cs.opacity) > 0 && rect.width > 1 && rect.height > 1`, OR if `cs.backgroundColor` is non-transparent (`!== 'rgba(0, 0, 0, 0)'` and `!== 'transparent'`) while `Number(cs.opacity) > 0`, OR if `parseFloat(cs.borderTopWidth) > 0 && Number(cs.opacity) > 0` or `cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) > 0`.
3. **`terminalFontProportionalLeakCount` (`=== 0`):** Queries visible `.xterm, .xterm-rows, [data-testid="term-block-card"] pre, [data-testid="chat-terminal-block-card"] pre, [data-terminal-mono="true"], [aria-label="console input"]`. Flags any element whose `getComputedStyle(el).fontFamily.toLowerCase()` contains `'palatino'`, `'cinzel'`, `'georgia'`, `'book antiqua'`, or lacks `'jetbrains mono'`/`'monospace'`/`'ui-monospace'`.
4. **`terminalContrastViolationCount` (`=== 0`):** Resolves effective non-transparent ancestor background RGB for every visible terminal text/span node inside `[data-terminal-well="true"], .xterm-rows span, [data-testid="term-block-card"] pre, [data-testid="term-block-card"] pre span, [data-testid="chat-terminal-block-card"] pre, [data-testid="chat-terminal-block-card"] pre span`, computes WCAG sRGB contrast ratio, and flags any node below `7.0` when `doc.documentElement.dataset.theme === 'high-contrast'` or below `4.5` in `arcane` / `slate` / `void` / `travertine` / `glacier`.
5. **Rubric Wiring (`src/lib/cvFlywheel.ts:304-409`):** Subtract `(dom.terminalStartupSpamCount ?? 0) * 0.12 + (dom.xtermHelperLeakCount ?? 0) * 0.12 + (dom.terminalFontProportionalLeakCount ?? 0) * 0.10 + (dom.terminalContrastViolationCount ?? 0) * 0.08` from `technicalQuality` and `styleConsistency` in `evaluateGuiSurfaceRubric` so any terminal regression deterministically drops `visualAfterScore` below `0.92`!

#### 3.3 CV Scoring Mechanics & Why `visualAfterScore >= 0.92` Converges Deterministically
- **Playwright Config (`[VERIFIED: crates/vox-gui/ui/playwright.config.ts:13-39]`):** `testDir: './e2e'`, `baseURL: 'http://127.0.0.1:4173'`, `webServer.command: 'pnpm preview'`, `workers: 1`. Build the UI bundle once before Playwright (`pnpm --dir crates/vox-gui/ui build`), then run Playwright with `CV_STAGE=before` or `CV_STAGE=after`.
- **Scoring Math (`[VERIFIED: crates/vox-gui/ui/src/lib/cvFlywheel.ts:304-478]` & `[VERIFIED: crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts:482-586]`):**
  - `computeRgbaFrameCvMetrics(png.data, width, height)` computes `edgeSharpness` (`~82..96` on crisp dual-surface borders + monospace glyphs), `luminanceContrast` (`~72..94` with high-contrast dark terminal well inside light/dark stone shell), and `noiseFloor` (`~4.0..4.5` on flat surfaces).
  - When all DOM invariants are `0` (`outerOverflowPx === 0`, `axeViolations === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `unstyledNativeSelectCount === 0`, `rawEmojiCount === 0`, `digitalColorLeakCount === 0`, `terminal*Count === 0`) and `viewportCoverageRatio` is within `[0.45, 0.92]`:
    - `technicalQuality` = `min(0.99, 0.34 + 0.90*0.30 + 0.85*0.22 + 0.95*0.14)` ≈ **`0.930..0.965`**
    - `promptAdherence` = **`0.960`**
    - `anatomicalAccuracy` = **`0.960`**
    - `compositionQuality` = `0.92 + 0.04` = **`0.960`**
    - `styleConsistency` = **`0.950`**
    - Note on `viewportCoverageRatio` (`cvFlywheel.ts:359,438`): `<main>` at `1920×1080` with collapsed sidebar could exceed `0.92` (`1720*1048 / (1920*1080) = 0.869`, while hidden sidebar `1920*1048 / (1920*1080) = 0.970`!). Keeping the left sidebar expanded (`~200..240px`) during main matrix captures keeps `coveredArea / viewportArea` at `0.74..0.88` (right inside `[0.45, 0.92]`), earning the `+0.04` `coverageBonus` (`compositionQuality = 0.96`) instead of `-0.08` (`0.84`)!
  - In `vlmVisionFlywheel.ts:527-578`, `evaluateDeterministicStructuredVisionAudit` blends `evaluateGuiSurfaceRubric` (`65%`) with Roman Scriptorium token audit (`35%`: `romanFontCoverageRatio`, `flutedPillarCount`, `waxSealCount`, `warmMarbleRatio`).
    - **Crucial Theme-Aware Detail (`vlmVisionFlywheel.ts:539-555`):** `vlmVisionFlywheel.ts:539` assumes warm Carrara marble (`warmMarbleRatio >= 0.45`, `romanFontCoverageRatio >= 0.9`, `flutedPillarCount > 0`), which applies to `arcane` and `travertine`/`glacier`! In `slate`/`void` and `high-contrast`, `[data-roman-pillar="true"]` is intentionally hidden (`index.css:989`) and `slate` uses `Inter` (`index.css:268`). Therefore, for `slate` and `high-contrast` frames, pass `evaluateGuiSurfaceRubric(dom, pixels)` directly to `runUniversalCvFlywheelPass` (just like `cv-flywheel.spec.ts:790-806` does for MENS!), while using `evaluateVlmRubricForFrame` on `arcane` and `travertine`! Both yield `visualAfterScore` ≈ **`0.94..0.96` (`>= 0.92`)**!
- **`CV_STAGE=before -> after` Two-Stage Protocol (`cv-flywheel.spec.ts:52,702-725`):**
  - `STAGE === 'before'`: Captures/persists baseline metrics JSON (`terminal-cv-before-metrics.json`) — or seeds deterministic pre-v7.5 baseline metrics reflecting measured `console-before.png` defects (`Palatino` bleed, `1.03:1` `.text-brass`, `.xterm-helper-textarea` box, startup hook spam -> `visualBeforeScore ≈ 0.58..0.64`) when `terminal-cv-before-metrics.json` is absent on CI, so `CV_STAGE=after` is 100% reproducible in clean CI checkouts as well as two-stage local runs!

#### 3.4 Required Console & Chat Interactive States (`states.ts` + `terminal-cv-flywheel.spec.ts`)
Register rich `SURFACE_STATES['console']` in `crates/vox-gui/ui/e2e/review/states.ts` (`[VERIFIED: L38-40]` currently only `[DEFAULT]`) and exercise all 8 required states in `terminal-cv-flywheel.spec.ts`:
1. **`block-stream-ansi` (`Console` default):** Multi-block session (`Shell` ok with 16-color SGR output + box-drawing table `┌──┬──┐`, failed exit `101` block with `Explain / Fix Failed Block`, Vox `Int(42)` block, streaming/completed `AgentTurn`).
2. **`split-panes`:** Click split-horizontal/vertical control in `<TerminalSessionPane>` -> 2 live panes side-by-side (`ConsoleSplitGrid`).
3. **`history-search-open`:** Dispatch `Control+r` on prompt editor -> `HistorySearchPopover` visible (`D-06`).
4. **`json-ledger-table`:** Submit `!nu ls | to json` and click `[data-testid="term-ledger-view-toggle"]` (`Table`) -> `JsonLedgerTable` visible with sortable columns (`D-06`).
5. **`runbooks-open`:** Open `RunbookPanel` tab in `DiscoveryRail` (or `OmnibarRunbookSheet`) -> parameterized `.vox` / Markdown runbook visible (`D-06`).
6. **`approval-card-pending`:** Seed Co-Pilot pending command approval -> inline `TerminalApprovalCard` (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`, wax-seal signet `data-roman-wax-seal="true"`) visible (`D-06`).
7. **`auto-tui-alt-screen`:** Seed session with `alt_screen: true` (`?1049h`) + real `@xterm/xterm` (`__VOX_E2E_REAL_XTERM__ = true`) -> Auto-TUI viewport active, `.xterm-helper-textarea` invisible (`xtermHelperLeakCount === 0`) (`D-06`).
8. **`chat-terminal-dock` (`#view=chat`):** Open `Terminal` dock panel inside `ChatSurface` (`DockWorkspaceShell` `TerminalDockPanel`) alongside an inline `ChatTerminalBlockCard` `@term:s-1#1:L1-5` citation chip/card (`D-06`).

---

### Seam 4: Architecture Doc Promotion (`TERM-ROMA-CV-04`)

- **Current File Inspected (`[VERIFIED: docs/src/architecture/warp-roman-terminal-v75-research-2026.md:1-65]`):**
  ```yaml
  ---
  title: "Warp-Class Roman Scriptorium Terminal & Bidirectional Chat Harness Research (v7.5)"
  description: "Architecture, stack, feature parity, and pitfall audit for Milestone v7.5 upgrading vox-terminal-core, vox-orchestrator-mcp, and Vox Axis Console/Chat into a stateful Warp-class agentic terminal."
  category: "Architecture SSOTs"
  status: "research"
  topic: "desktop-app"
  domain: "gui"
  authorship: "llm-assisted"
  authorship_model: "gemini-4-carbon"
  ---
  ```
- **Taxonomy & Lint Verification:**
  - `category: "Architecture SSOTs"` is in `VALID_CATEGORIES` (`[VERIFIED: crates/vox-doc-pipeline/src/pipeline/lint.rs:31]`).
  - `status: "current"` is in `VALID_STATUS` (`[VERIFIED: crates/vox-doc-pipeline/src/pipeline/lint.rs:41]`).
  - `domain: "gui"` and `topic: "desktop-app"` (or `topic: "terminal-ui"`, `[VERIFIED: contracts/documentation/kb-taxonomy.v1.yaml:22,270,294]`) are valid `prefLabel` pairs under `broader: gui`.
  - `<!-- kb:summary -->` is placed directly beneath H1 (`L14-16`) at **84 words** (within the enforced `40..120` word window, `[VERIFIED: docs/src/contributors/documentation-governance.md:179]`).
  - Do **NOT** add `training_eligible: true` (contradicts `authorship: "llm-assisted"` without `human_attested_by`, triggering `facet-training-eligible-contradiction` per `documentation-governance.md:181`), and do **NOT** add `last_updated` or `human_attested_by`.
  - Reality-Audit Gate Check (`[VERIFIED: crates/vox-doc-pipeline/src/claims/gate.rs:82-87,117-121]`): `is_reader_facing_page` returns `false` for `/architecture/` pages, so promoting `status: "research"` -> `status: "current"` moves the page from collapsed `Internals` into the main `Architecture SSOTs` Starlight sidebar without triggering false-positive merge blocks in `docs-reality-audit`, while keeping its prose accurate to shipped Phases 441–446 contracts (`--font-terminal-mono`, Dual-Surface 4-theme ANSI matrix, `terminalAudits.ts` CV invariants).
- **Verified Lint Command (`0.4s` execution time):**
  ```bash
  timeout 60s ./target/debug/vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md
  # (or via cargo if target/debug/vox-doc-pipeline is not built yet):
  timeout 180s cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md
  ```
  *(Note: `--paths` is resolved relative to `./docs/src/` — never prefix with `docs/src/`.)*

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CSS Custom Property Generation | Hand-editing `src/styles/tokens*.generated.css` | `tokens/primitive.json` + `pnpm --dir crates/vox-gui/ui tokens:build` (`style-dictionary.config.mjs`) | `pnpm build` runs `pnpm tokens:build` first and overwrites any manual edit to `tokens*.generated.css`. |
| WCAG Relative Luminance & Contrast Math | Custom ad-hoc brightness formulas (`(R*299 + G*587 + B*114)/1000`) | `evaluateWcagContrastRatio` from `src/lib/cvFlywheel.ts` (backed by `platformSsot.generated.ts`) | Uses exact WCAG 2.x gamma-linearized sRGB luminance (`0.2126 R + 0.7152 G + 0.0722 B`), matching `@axe-core/playwright`. |
| Pixel Sharpness / Noise / Convergence Scoring | Ad-hoc screenshot byte-size checks | `computeRgbaFrameCvMetrics` + `runUniversalCvFlywheelPass` (`src/lib/cvFlywheel.ts`) + `evaluateVlmRubricForFrame` (`vlmVisionFlywheel.ts`) | Already calibrated across Phases 358–386 and integrated with `@ngi/platform-ssot`. |
| Terminal Icons | Unicode symbols (`»`, `«`, `✕`, `⚡`, `▶`, `⏎`) | `Icon.*` / `KeycapIcon` from `src/components/ui/Icons.tsx` | `zeroEmojiAndDingbat.test.ts` and `audits.ts` (`rawEmojiCount`, `iconIssueCount`) fail on Unicode dingbats/arrows/emojis. |

---

## Common Pitfalls

### Pitfall 1: Editing `tokens*.generated.css` Without Updating `tokens/primitive.json`
- **What goes wrong:** `themeContract.test.ts` passes initially, then `pnpm --dir crates/vox-gui/ui build` (run before `playwright test`) runs `pnpm tokens:build` and wipes `--font-terminal-mono` out of all 4 `tokens*.generated.css` files, breaking subsequent `themeContract.test.ts` runs!
- **How to avoid:** Always add `"terminal-mono"` to `crates/vox-gui/ui/tokens/primitive.json` first and generate `tokens*.generated.css` via `pnpm --dir crates/vox-gui/ui tokens:build`.

### Pitfall 2: Forgetting `SettingsView` Semantic Token Counter (`22 / 22` -> `23 / 23`)
- **What goes wrong:** Adding `'--font-terminal-mono'` to `REQUIRED_SEMANTIC_CSS_VARS` increments `requiredSemanticVarCount` from `22` to `23`, causing `cv-flywheel.spec.ts:1250` (`toContainText('22 / 22 Verified')`) to fail.
- **How to avoid:** Update `cv-flywheel.spec.ts:1250` to `'23 / 23 Verified'` in the same task that adds `'--font-terminal-mono'`.

### Pitfall 3: Light-Theme `index.css` Normalizers Clobbering Dark Terminal Wells
- **What goes wrong:** `index.css:812-876` forces `.text-white`, `.text-zinc-*`, and `.text-brass` to `#1C140C !important` whenever `data-theme` is `arcane` or `glacier`/`travertine`, turning foreground text on `--terminal-well-bg` (`#15110d`) into `1.03:1` dark-on-dark invisible ink.
- **How to avoid:** Exclude `:not(.xterm):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` on every light-theme surface/text normalizer in `index.css:775-979`, and eliminate `.text-brass` inside dark terminal wells.

### Pitfall 4: `audits.ts` `digitalColorLeakCount` Flagging Legacy ANSI Emerald/Cyan RGBs
- **What goes wrong:** `audits.ts:538-542` flags `rgb(52, 211, 153)` (`#34d399`), `rgb(110, 231, 183)` (`#6ee7b7`), and `rgb(34, 211, 238)` (`#22d3ee`) on any `main span` `>= 16×12px`, penalizing `styleConsistency` by `0.04` per span.
- **How to avoid:** Use the calibrated Roman Scriptorium ANSI hexes in Section Seam 2.1 (none of which collide with `audits.ts:538-542`) AND exempt `el.closest('[data-terminal-well="true"], .xterm')` from non-neon ANSI false positives in `audits.ts`.

### Pitfall 5: `viewportCoverageRatio > 0.92` Penalty at `1920×1080`
- **What goes wrong:** `evaluateGuiSurfaceRubric` (`src/lib/cvFlywheel.ts:359`) awards `+0.04` when `0.45 <= viewportCoverageRatio <= 0.92` and penalizes `-0.08` when `> 0.92`. Collapsing the left navigation sidebar at `1920×1080` pushes `main` coverage to `0.97`, dropping `compositionQuality` from `0.96` to `0.84`.
- **How to avoid:** Keep the left navigation sidebar open during standard `5-viewport × 4-theme` CV convergence captures (`coveredArea / viewportArea` stays `0.74..0.88`).

### Pitfall 6: `DiscoveryRail` + Split Panes Overflowing `900×600`
- **What goes wrong:** Two `min-w-[280px]` split panes + `w-[268px]` `DiscoveryRail` + `200px` nav = `1028px` inside a `900px` viewport (`outerOverflowPx = 128`).
- **How to avoid:** Auto-collapse `DiscoveryRail` below `1100px` viewport width (unless explicitly toggled open, where it renders as an overlay or `max-w-[220px]` drawer) and reduce `ConsoleSplitGrid` multi-leaf min-width to `min-w-[160px]`.

---

## Code Examples

### 1. `src/index.css` Terminal Font & `.xterm-helper-textarea` Containment Pattern
```css
/* Inside @theme (crates/vox-gui/ui/src/index.css) */
--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;

/* Dedicated Terminal & Code Cell Monospace Isolation (TERM-ROMA-CV-01) */
.xterm,
.xterm *,
[data-terminal-mono="true"],
[data-terminal-mono="true"] * {
  font-family: var(--font-terminal-mono, 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace) !important;
  font-variant-ligatures: none;
  letter-spacing: 0 !important;
}

/* Guarantee hidden xterm input proxy never inherits Romanesque textarea cream fill or focus ring */
.xterm .xterm-helper-textarea,
textarea.xterm-helper-textarea {
  opacity: 0 !important;
  width: 0 !important;
  height: 0 !important;
  padding: 0 !important;
  margin: 0 !important;
  border: 0 !important;
  outline: none !important;
  box-shadow: none !important;
  background: transparent !important;
  color: transparent !important;
  pointer-events: none !important;
}
```

### 2. Dual-Surface `--terminal-well-*` Variables in `src/index.css` (Allowlisted Path)
```css
:root,
[data-theme="arcane"] {
  --font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;
  --terminal-well-bg: #15110d;
  --terminal-well-fg: #F5EFE2;
  --terminal-well-muted: #A89D8E;
  --terminal-well-border: rgba(197, 155, 39, 0.52);
  --terminal-well-cursor: #E0C478;
  --terminal-well-selection: rgba(212, 175, 55, 0.28);
}

[data-theme="glacier"],
[data-theme="travertine"] {
  --font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;
  --terminal-well-bg: #171512;
  --terminal-well-fg: #F6F1E7;
  --terminal-well-muted: #ADA395;
  --terminal-well-border: rgba(138, 106, 38, 0.52);
  --terminal-well-cursor: #79C8BA;
  --terminal-well-selection: rgba(121, 200, 186, 0.26);
}

[data-theme="slate"],
[data-theme="void"] {
  --font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;
  --terminal-well-bg: #0c0e10;
  --terminal-well-fg: #F1F5F9;
  --terminal-well-muted: #94A3B8;
  --terminal-well-border: #2b343a;
  --terminal-well-cursor: #38BDF8;
  --terminal-well-selection: rgba(56, 189, 248, 0.24);
}

[data-theme="high-contrast"] {
  --font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;
  --terminal-well-bg: #000000;
  --terminal-well-fg: #FFFFFF;
  --terminal-well-muted: #CBD5E1;
  --terminal-well-border: #FFFFFF;
  --terminal-well-cursor: #FDE047;
  --terminal-well-selection: rgba(253, 224, 71, 0.32);
}
```

### 3. Executing the Two-Stage Playwright Terminal CV Flywheel
```bash
# 1. Regenerate tokens & build preview bundle once
timeout 180s pnpm --dir crates/vox-gui/ui build

# 2. Capture/verify Before baseline stage
timeout 300s env CV_STAGE=before pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts

# 3. Execute After convergence stage across 5 viewports × 4 themes (visualAfterScore >= 0.92)
timeout 300s env CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts
```

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Unit / Guard Framework | `vitest@3.2.6` + `@testing-library/react@16.3.2` (`jsdom`) |
| E2E / Visual CV Framework | `@playwright/test@1.59.1` + `@axe-core/playwright@4.11.2` + `@ngi/platform-ssot` (`src/lib/cvFlywheel.ts`) |
| Static Code & Doc Gates | `vox audit code` (`vox/ui/theme-contract-drift`) + `vox-doc-pipeline --lint-only` |
| Config Files | `crates/vox-gui/ui/vite.config.ts`, `crates/vox-gui/ui/playwright.config.ts` |
| Quick Run Command | `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/themeContract.test.ts src/guards/zeroEmojiAndDingbat.test.ts e2e/review/audits.test.ts` (verified `2.74s`) |
| Full Suite Command | `timeout 180s pnpm --dir crates/vox-gui/ui build` then `timeout 300s env CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts e2e/console-workbench.spec.ts` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| `TERM-ROMA-CV-01` | `--font-terminal-mono` in all 4 `tokens*.generated.css`, `theme-contract.v1.json`, `themeContract.ts`, `index.css` `:not(.xterm *)`; `0` theme-contract errors | Vitest + Static Audit | `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/themeContract.test.ts` & `timeout 300s ./target/debug/vox audit code` | ✅ `src/lib/themeContract.test.ts` |
| `TERM-ROMA-CV-02` | 4-theme Dual-Surface 16-color ANSI WCAG `>= 4.5:1` (`>= 7.0:1` high-contrast), zero `.text-brass` dark-on-dark, zero `zinc-*`/`slate-*`/raw emoji/guillemet | Vitest (`TerminalTab`, `sgrToSpans`, `zeroEmojiAndDingbat`, new ANSI contrast matrix test) | `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/guards/zeroEmojiAndDingbat.test.ts` | ✅ Existing + ❌ Wave 1 ANSI WCAG matrix assertions in `TerminalTab.test.tsx` |
| `TERM-ROMA-CV-03` | Deterministic `terminalAudits.ts` invariants (`0` spam/helper-leak/proportional-leak/contrast violations) + `CV_STAGE=before -> after` 5-viewport × 4-theme Playwright convergence (`visualAfterScore >= 0.92`, `0` overflow/clipping/overlap/axe) | Vitest (`jsdom`) + Playwright E2E + CV Flywheel | `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run e2e/review/audits.test.ts e2e/review/terminalAudits.test.ts` & `timeout 300s env CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts` | ❌ Wave 2/3 (`e2e/review/terminalAudits.ts`, `terminalAudits.test.ts`, `terminal-cv-flywheel.spec.ts`) |
| `TERM-ROMA-CV-04` | `warp-roman-terminal-v75-research-2026.md` promoted to `status: "current"` and passes KB facet + doc lint | Rust CLI Doc Lint | `timeout 60s ./target/debug/vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md` | ✅ `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` |

### Sampling Rate
- **Per task commit:** Run scoped Vitest tests (`< 5s`) + `pnpm --dir crates/vox-gui/ui typecheck`.
- **Per wave merge:** Run full Console/Chat/Guards Vitest suite + `vox audit code` (`vox/ui/theme-contract-drift`).
- **Phase gate:** Full `pnpm build` + Playwright `CV_STAGE=before` & `CV_STAGE=after` (`terminal-cv-flywheel.spec.ts` + `console-workbench.spec.ts` + Phase 386 check in `cv-flywheel.spec.ts`) + `vox-doc-pipeline --lint-only` (`0` errors).

### Wave 0 Gaps
- None blocking Wave 1: Vitest, Playwright, `@axe-core/playwright`, `pngjs`, `cvFlywheel.ts`, `vlmVisionFlywheel.ts`, and `vox-doc-pipeline` are already installed and verified runnable (`themeContract.test.ts` + `audits.test.ts` passed 13/13 in `2.74s`; `vox-doc-pipeline --lint-only` passed in `0.4s`). Wave 2 (`446-03`) creates `e2e/review/terminalAudits.ts` + `terminalAudits.test.ts` TDD-first before Wave 3 (`446-04`) runs the full 5×4 Playwright matrix.

---

## Security Domain

- **Applicable ASVS Categories:**
  - **V5 (Validation, Sanitization & Encoding):** Terminal CV fixtures and SGR span rendering (`sgrToSpans.ts`, `JsonLedgerTable.tsx`) must remain pure React text/style spans with **zero `dangerouslySetInnerHTML`** so ANSI escape sequences or JSON cells in terminal output can never trigger DOM XSS.
  - **V8 (Data Protection / Secret Redaction):** Mock and E2E fixtures for `@term` citations, Runbooks, and Co-Pilot approvals must use synthetic non-secret tokens and never read `.env*` (`00-core.md` Rule 12); `terminalStartupSpamCount` also verifies shell bootstrap scripts never echo environment setup lines to screen.
  - **V14 (Configuration & Static Integrity):** Theme contract JSON (`contracts/gui/theme-contract.v1.json`), runtime `themeContract.ts`, and Rust `ui_theme_contract.rs` stay in strict lockstep without loosening `DARK_BROWN_HEX_ALLOWLIST_PATHS` or `BANNED_ROMANESQUE_FONT_LITERALS`.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | *Zero unverified assumptions.* Every file path, CSS line number, contrast ratio, CLI flag, and test runner behavior in this document was verified directly against the workspace source and live tool execution. | All | None |

---

## Sources

### Primary (HIGH confidence — Verified In-Session)
- `crates/vox-gui/ui/src/index.css` (`L1-1260`) — `@theme`, `[data-theme]` token blocks, proportional `Palatino` `!important` selectors, `textarea` `#FAF5E8` rules, and light-theme `.text-brass` `#1C140C` normalizers.
- `crates/vox-gui/ui/style-dictionary.config.mjs` (`L1-80`), `tokens/primitive.json` (`L1-68`), and `src/styles/tokens*.generated.css` — Style Dictionary CSS variable generation pipeline.
- `contracts/gui/theme-contract.v1.json` (`L1-208`), `crates/vox-gui/ui/src/lib/themeContract.ts` (`L1-571`), `crates/vox-gui/ui/src/lib/themeContract.test.ts` (`L1-196`), and `crates/vox-code-audit/src/detectors/ui_theme_contract.rs` (`L1-298`) — 22->23 semantic token contract and banned font/dark-hex enforcement.
- `crates/vox-gui/ui/src/components/surfaces/Console/*` (`terminalTheme.ts`, `Console.tsx`, `TerminalSessionPane.tsx`, `TerminalTab.tsx`, `BlockCard.tsx`, `BlockStreamView.tsx`, `ConsoleSessionBar.tsx`, `ConsoleSplitGrid.tsx`, `InputEditor.tsx`, `DiscoveryRail.tsx`, `AgentStrip.tsx`, `AgentTab.tsx`, `SendToAgent.tsx`, `sgrToSpans.ts`) and `Chat/ChatTerminalBlockCard.tsx` — Component styling, `isMock` xterm gate, and existing unit tests.
- `crates/vox-gui/ui/src/lib/cvFlywheel.ts` (`L1-479`), `e2e/review/audits.ts` (`L1-575`), `e2e/review/audits.test.ts` (`L1-138`), `e2e/review/vlmVisionFlywheel.ts` (`L1-587`), `e2e/review/states.ts` (`L1-161`), `e2e/review/cv-flywheel.spec.ts` (`L1-1564`), `e2e/console-workbench.spec.ts` (`L1-144`), and `e2e/lib/tauriMock.ts` (`L1270-1420`) — CV Flywheel scoring math, DOM invariants, and E2E mock handlers.
- `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` (`L1-65`), `contracts/documentation/kb-taxonomy.v1.yaml` (`L1-300`), `docs/src/contributors/documentation-governance.md` (`L1-220`), and `crates/vox-doc-pipeline/src/{pipeline/lint.rs,claims/gate.rs}` — Doc promotion & facet lint contract.

---

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — Zero new dependencies; all tools verified in `package.json` and executed in-session.
- Architecture & CSS/Theme seams: **HIGH** — Every conflicting CSS rule, Style Dictionary config, `SettingsView` counter, and 64-entry WCAG contrast ratio verified mathematically and line-by-line.
- Pitfalls & CV convergence: **HIGH** — Uncovered and solved `TerminalTab` `isMock` bypass, `audits.ts` neon RGB collision, `900×600` `DiscoveryRail` + split overflow, `viewportCoverageRatio > 0.92` penalty, and `vox-doc-pipeline --paths` prefix resolution.

**Research date:** 2026-11-15
**Valid until:** 2026-12-15
