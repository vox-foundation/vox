# Phase 446: Dual-Surface Roman Scriptorium Theme Hardening & Multi-Pass 4-Theme CV Flywheel Convergence - Context

**Gathered:** 2026-11-15
**Status:** Ready for UI spec & planning
**Mode:** Autonomous Smart Discuss (`--ws axis-warp-terminal-v75`)

<domain>
## Phase Boundary

Deliver full Dual-Surface Roman Scriptorium typography/color containment and multi-pass 4-theme Computer-Vision (CV) Flywheel convergence across standalone `Console` and `Chat` + `TerminalDockPanel`:

1. **`--font-terminal-mono` Contract & `:not(.xterm)` CSS Containment (`TERM-ROMA-CV-01`)**:
   - Register `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` across `crates/vox-gui/ui/src/index.css`, `src/styles/tokens*.generated.css` (or theme token overrides), `contracts/gui/theme-contract.v1.json`, and `crates/vox-gui/ui/src/lib/themeContract.ts` (`REQUIRED_SEMANTIC_CSS_VARS`, zero banned `Menlo`/`Monaco`/`Consolas`/`Courier`/`Arial` literals), passing `vox audit code` (`vox/ui/theme-contract-drift`) and `themeContract.test.ts` with `0` errors.
   - Isolate `.xterm` and `.xterm *` (`:not(.xterm):not(.xterm *)`) from global proportional `Palatino` `!important` font overrides and global `main textarea` / focus-ring rules in `index.css` so xterm cell metrics are strictly fixed-width and `.xterm-helper-textarea` never renders a visible cream box or outline bleed.
2. **4-Theme Dual-Surface Roman Scriptorium Parity (`TERM-ROMA-CV-02`)**:
   - Harden `Console`, `BlockStreamView`, `BlockCard`, `ScriptoriumPromptEditor`, `TerminalApprovalCard`, `JsonLedgerTable`, `DiscoveryRail`, `RunbookPanel`, `ConsoleSessionBar`, `ChatTerminalBlockCard`, and `terminalTheme.ts` across all supported themes (`arcane`, `slate`/`void`, `travertine`/`glacier`, and `high-contrast`):
     - In `arcane` (light Carrara marble `#FAF6EE` / `#F5EFE2` + burnished gold `#7d5f20` / `rgb(var(--brass))` outer chrome), render a deliberate Dual-Surface warm dark-bronze/obsidian terminal viewport (`terminalTheme.ts` semantic CSS vars / allowlisted theme tokens) with 16-color ANSI contrast `>= 4.5:1` (`>= 7.0:1` in `high-contrast`).
     - Eliminate any remaining hardcoded `zinc-*`, `slate-*` islands, raw `#000`/`bg-black`, and dark-on-dark `.text-brass` (`1.03:1`) contrast defects across all 4 profiles.
3. **Deterministic Terminal CV Invariants (`TERM-ROMA-CV-03`)**:
   - Extend `crates/vox-gui/ui/e2e/review/audits.ts`, `crates/vox-gui/ui/src/lib/cvFlywheel.ts` (or `vlmVisionFlywheel.ts`), and `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (+ `audits.test.ts`) to assert deterministic terminal CV invariants:
     - `cursorBlink: false` during deterministic capture
     - All 16 ANSI palette colors against terminal viewport background `>= 4.5:1` WCAG contrast (`>= 7.0:1` in `high-contrast`)
     - Fixed-width monospace cell alignment (`'M'` width === `'i'` width within `<= 0.05px` inside `.xterm` and `[data-terminal-mono]`)
     - Zero visible `.xterm-helper-textarea` (`opacity: 0` or `width <= 1px`, zero visible background/border)
     - Zero raw `<select>` / `<input type="range">` / emoji / dingbat / bracket template leaks / horizontal overflow / clipped interactive controls (`>= 24×24px` hit targets where applicable, zero clipping in split panes or `TerminalDockPanel`).
4. **Multi-Pass `CV_STAGE=before -> after` Convergence & Architecture Doc Promotion (`TERM-ROMA-CV-04`)**:
   - Execute multi-pass Playwright + `@ngi/platform-ssot` + VLM (`vox_actor_runtime::llm` / CV runner) audits across standalone `Console` (including Block Stream, Split Panes, `Ctrl+R` History, `JsonLedgerTable`, `Runbooks`, `TerminalApprovalCard`, and Auto-TUI `ALT_SCREEN` state) and `Chat` + `TerminalDockPanel` at `900×600`, `1100×720`, `1280×800`, `1440×900`, and `1920×1080` across `arcane`, `slate`, `travertine`, and `high-contrast`, iterating fixes until `visualAfterScore >= 0.92` and `0` axe/overflow/clipping/contrast defects.
   - Promote `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` frontmatter to `status: "current"`, updating final architecture notes and verifying with `./target/debug/vox-doc-pipeline --lint-only --paths docs/src/architecture/warp-roman-terminal-v75-research-2026.md`.

Covers requirements: `TERM-ROMA-CV-01`, `TERM-ROMA-CV-02`, `TERM-ROMA-CV-03`, `TERM-ROMA-CV-04`.

</domain>

<decisions>
## Implementation Decisions

### 1. `--font-terminal-mono` & `:not(.xterm)` CSS Containment (`TERM-ROMA-CV-01`)
- **D-01 (Allowlisted Monospace Stack):** Define `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` in `index.css` (`@theme` + per-theme `:root` / `[data-theme=...]` blocks), `contracts/gui/theme-contract.v1.json`, and `src/lib/themeContract.ts` (`REQUIRED_SEMANTIC_CSS_VARS`). Never include `Menlo`, `Monaco`, `Consolas`, `Courier`, `Courier New`, or `Arial` (`BANNED_ROMANESQUE_FONT_LITERALS`).
- **D-02 (Strict `.xterm` Isolation in `index.css`):** Update global `Palatino` / serif font-family selectors and `main textarea` / `textarea` background/border/padding selectors in `src/index.css` with `:not(.xterm):not(.xterm *):not(.xterm-helper-textarea)` and explicit `.xterm .xterm-helper-textarea { opacity: 0 !important; pointer-events: none; background: transparent !important; border: 0 !important; box-shadow: none !important; }` so xterm canvas/DOM cells and helper textarea have zero style bleed across all 4 themes.

### 2. Dual-Surface Roman Scriptorium 4-Theme Parity (`TERM-ROMA-CV-02`)
- **D-03 (Dual-Surface Token Architecture):** Outer chrome (`ConsoleSessionBar`, `DiscoveryRail`, `RunbookPanel`, `ScriptoriumPromptEditor` frame, `TerminalApprovalCard` parchment/marble card) uses `--color-bg-base` / `--color-bg-surface` / `--color-bg-elevated` + `--color-accent-default` / `rgb(var(--brass))`, while the terminal output well (`TerminalTab` VT canvas + dark obsidian/slate block output wells where configured via `--terminal-well-bg` / `--terminal-well-fg` semantic vars in `index.css` & `terminalTheme.ts`) guarantees `>= 4.5:1` WCAG AA contrast for all 16 ANSI colors and foreground text in `arcane`, `slate`, `travertine`/`glacier`, and `>= 7.0:1` in `high-contrast` (`BANNED_HARDCODED_DARK_BROWN_HEXES` respected everywhere outside `DARK_BROWN_HEX_ALLOWLIST_PATHS`).
- **D-04 (Zero Legacy Dark Islands or `.text-brass` Contrast Failures):** Audit and eliminate every hardcoded `bg-zinc-*`, `text-zinc-*`, `border-zinc-*`, `bg-black`, and low-contrast `.text-brass` usage in `Console/*` and `ChatTerminalBlockCard.tsx`.

### 3. Deterministic Terminal CV Invariants (`TERM-ROMA-CV-03`)
- **D-05 (Playwright + Unit CV Audit Gates):** Add terminal-specific deterministic checks to `e2e/review/audits.ts` (and `e2e/review/states.ts` rich mock fixtures for `Console` + `Chat` `TerminalDockPanel`):
  1. `terminalFixedCellAlignment`: verifies `measureText('MMMMMMMMMM')` vs `measureText('iiiiiiiiii')` inside `.xterm` / `[data-terminal-mono]` matches within `<= 0.1px`.
  2. `terminalHelperTextareaHidden`: verifies `.xterm-helper-textarea` has `opacity === '0'` or zero visible paint box.
  3. `terminalAnsiContrast`: verifies all 16 `resolveAnsi16Palette(theme)` colors against `terminalWellBg` satisfy `>= 4.5:1` (`>= 7.0:1` in `high-contrast`).
  4. `terminalZeroClippedControls`: verifies zero horizontal scroll overflow and zero clipped buttons/badges at `900×600`, `1100×720`, `1280×800`, `1440×900`, and `1920×1080`.

### 4. Multi-Pass CV Flywheel Execution & Documentation Promotion (`TERM-ROMA-CV-04`)
- **D-06 (5-Viewport × 4-Theme Multi-Pass Flywheel):** Run Playwright `e2e/review/cv-flywheel.spec.ts` (and Console/Chat e2e specs) across `[900×600, 1100×720, 1280×800, 1440×900, 1920×1080]` × `[arcane, slate, travertine/glacier, high-contrast]` in `CV_STAGE=before`, inspect captured screenshots + structured defect reports, apply CSS/layout remediations, and re-run `CV_STAGE=after` until `visualAfterScore >= 0.92` and `0` axe/overflow/clipping/contrast defects.
- **D-07 (Architecture Doc Promotion):** Update `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` YAML frontmatter `status: "current"` with shipped module references and validate via `vox-doc-pipeline --lint-only`.

### 5. Non-Negotiable Repo Guardrails
- **D-08:** Zero new Cargo or npm packages; zero git mutating commands; `< 450` non-blank LoC per touched source file; `./target/debug/vox audit code`, `./target/debug/vox-arch-check`, `themeContract.test.ts`, and all static UI guards exit `0`.

</decisions>

<canonical_refs>
## Canonical References

- `.planning/workstreams/axis-warp-terminal-v75/ROADMAP.md` — Phase 446 goal & 4 success criteria
- `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` — `TERM-ROMA-CV-01..04`
- `crates/vox-gui/ui/src/index.css` — Theme CSS variables, `@theme`, `:not(.xterm)` containment rules
- `contracts/gui/theme-contract.v1.json` & `crates/vox-gui/ui/src/lib/themeContract.ts` — Theme contract SSOT (`vox/ui/theme-contract-drift`)
- `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` — 4-profile terminal ANSI 16-color palette & xterm theme builder
- `crates/vox-gui/ui/e2e/review/{audits.ts,audits.test.ts,states.ts,cv-flywheel.spec.ts,vlmVisionFlywheel.ts}` — CV flywheel harness
- `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` — v7.5 architecture research doc to promote to `status: "current"`

</canonical_refs>

<deferred>
## Deferred Ideas

- None — Phase 446 is the final convergence phase of Milestone `v7.5`.

</deferred>
