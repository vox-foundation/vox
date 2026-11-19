---
phase: 446
phase_name: "roman-scriptorium-theme-and-cv-flywheel"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 28
  lessons: 9
  patterns: 13
  surprises: 1
missing_artifacts:
  - "UAT.md"
---

# Phase 446 Learnings: roman-scriptorium-theme-and-cv-flywheel

## Decisions

### Used exact allowlisted monospace stack 'JetBrains Mono', ui-monospace, '...
Used exact allowlisted monospace stack 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace with zero banned Romanesque font literals

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-01-SUMMARY.md

---

### Excluded :not(.xterm), :not([data-terminal-mono='true']), and :not([data...
Excluded :not(.xterm), :not([data-terminal-mono='true']), and :not([data-terminal-well='true']) across @layer base display/mono/button rules AND both light-theme and dark-theme normalizers in src/index.css so terminal spans and inline path/URL buttons remain fixed-width and unboxed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-01-SUMMARY.md

---

### Added font.terminal-mono and font.family.terminal-mono ('JetBrains Mono'...
Added `font.terminal-mono` and `font.family.terminal-mono` (`'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace`) to `crates/vox-gui/ui/tokens/primitive.json` and regenerated `tokens.generated.css`, `tokens.slate.generated.css`, `tokens.contrast.generated.css`, `tokens.travertine.generated.css`, and `tokens.generated.ts` via `pnpm tokens:build` with zero banned font literals.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-01-SUMMARY.md

---

### Synchronized contracts/gui/theme-contract.v1.json (surgical +10/-5 diff)...
Synchronized `contracts/gui/theme-contract.v1.json` (surgical `+10/-5` diff), `crates/vox-gui/ui/src/lib/themeContract.ts`, `crates/vox-gui/ui/src/lib/themeContract.test.ts`, and `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`'23 / 23 Verified'`) around the expanded 23-variable `REQUIRED_SEMANTIC_CSS_VARS` contract.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-01-SUMMARY.md

---

### Declared calibrated Dual-Surface --terminal-well-{bg,fg,muted,border,cur...
Declared calibrated Dual-Surface `--terminal-well-{bg,fg,muted,border,cursor,selection}` custom properties across `arcane`, `glacier`/`travertine`, `void`/`slate`, and `high-contrast` in `crates/vox-gui/ui/src/index.css`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-01-SUMMARY.md

---

### Guarded @layer base proportional Cinzel/Palatino typography, 3D marble b...
Guarded `@layer base` proportional `Cinzel`/`Palatino` typography, 3D marble button chrome, `*:focus-visible`, and both light-theme (`arcane`/`travertine`) and dark-theme (`slate`/`void`/`high-contrast`) normalizer rules with `:not(.xterm):not(.xterm *):not([data-terminal-mono="true"]):not([data-terminal-mono="true"] *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` and `:not(.xterm-helper-textarea)`, plus an explicit zero-box `.xterm-helper-textarea` containment lock.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-01-SUMMARY.md

---

### Included [data-terminal-well="true"], [data-terminal-well="true"] * alon...
Included `[data-terminal-well="true"], [data-terminal-well="true"] *` alongside `.xterm, .xterm *, [data-terminal-mono="true"], [data-terminal-mono="true"] *` in the `@layer base` terminal monospace rule and guarded `@layer base` light-theme 3D button rules (`L378-401`) so any `<pre>` or inline clickable path/URL `<button>` inside a terminal well stays fixed-width and unboxed across all 4 themes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-01-SUMMARY.md

---

### Authorized scope addition for xtermAddons.test.ts (L68-86) to align its ...
Authorized scope addition for xtermAddons.test.ts (L68-86) to align its Dual-Surface expectedBg matrix (#15110d arcane, #171512 travertine/glacier, #0c0e10 slate/void, #000000 high-contrast >= 7.0:1) with terminalTheme.ts

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-02-SUMMARY.md

---

### Styled TerminalTab mock viewport internals with --terminal-well-* tokens...
Styled TerminalTab mock viewport internals with --terminal-well-* tokens alongside data-terminal-well='true' so dark recessed mock wells avoid dark-on-dark contrast leaks on light themes

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-02-SUMMARY.md

---

### Replaced duplicate local TERMINAL_MONO_STYLE/MIRROR_FONT_STYLE/RAIL_MONO...
Replaced duplicate local TERMINAL_MONO_STYLE/MIRROR_FONT_STYLE/RAIL_MONO_STYLE definitions with shared import/re-export from terminalTheme.ts, keeping every file under 416 non-blank LoC

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-02-SUMMARY.md

---

### Updated terminalTheme.ts (228 LoC) with TERMINAL_MONO_STYLE, ARCANE_ANSI...
Updated `terminalTheme.ts` (`228` LoC) with `TERMINAL_MONO_STYLE`, `ARCANE_ANSI_16` (`>= 4.74:1` on `#15110d`), `TRAVERTINE_ANSI_16` (`>= 4.96:1` on `#171512`), `SLATE_ANSI_16` (`>= 5.57:1` on `#0c0e10`, `cyan: '#38bdf8'`), `HIGH_CONTRAST_ANSI_16` (`>= 9.66:1` AAA on `#000000`), and `resolveTerminalWellSpec` (`wellBg`/`wellFg` indirection keeping `GENERAL_DARK_HEX_BG_REGEX` at `0` violations).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-02-SUMMARY.md

---

### Updated sgrToSpans.ts (354 LoC) to resolve inverse SGR (\x1b[7m) default...
Updated `sgrToSpans.ts` (`354` LoC) to resolve inverse SGR (`\x1b[7m`) defaults against `resolveTerminalWellSpec(opts?.theme)` (`wellFg`/`wellBg`), keep `<mark data-testid="term-search-highlight">` readable (`text-inherit`), and style `term-output-file-link` / `term-output-url-link` with `text-(--terminal-well-cursor) underline decoration-dotted hover:opacity-90`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-02-SUMMARY.md

---

### Updated TerminalTab.tsx (355 LoC) to respect window.__VOX_E2E_REAL_XTERM...
Updated `TerminalTab.tsx` (`355` LoC) to respect `window.__VOX_E2E_REAL_XTERM__ === true` with `cursorBlink: false` and attach `data-terminal-well="true"` + `data-terminal-mono="true"` plus `--terminal-well-*` tokens on both mock and live viewports.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-02-SUMMARY.md

---

### Updated TerminalTab.test.tsx, sgrToSpans.test.tsx, and caller-authorized...
Updated `TerminalTab.test.tsx`, `sgrToSpans.test.tsx`, and caller-authorized `xtermAddons.test.ts` with programmatic WCAG 2.1 contrast assertions across all 64 palette entries and `__VOX_E2E_REAL_XTERM__` coverage.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-02-SUMMARY.md

---

### Updated BlockCard.tsx (359 LoC) and ChatTerminalBlockCard.tsx (209 LoC) ...
Updated `BlockCard.tsx` (`359` LoC) and `ChatTerminalBlockCard.tsx` (`209` LoC) to mark command header `<code>` with `data-terminal-mono="true"` and wrap output `<pre>` wells (`data-testid="term-block-output"` / `data-testid="chat-term-block-output"`) in Dual-Surface `--terminal-well-*` styling while preserving `JsonLedgerTable`, `AgentTurnContent`, `SlashMetaContent`, `TermBlockFeedbackStrip`, `term-explain-fix-block`, and compact `<TerminalApprovalCard />`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-02-SUMMARY.md

---

### Kept collectTerminalAuditMetrics, auditDomQualityInPage, buildTerminalRi...
Kept collectTerminalAuditMetrics, auditDomQualityInPage, buildTerminalRichFixtures, and installTerminalRichMock 100% self-contained with zero outer-scope references, adding explicit new Function('return (' + fn.toString() + ')')() tests in Vitest to prevent Chromium ReferenceError regressions

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-03-SUMMARY.md

---

### Paired ANSI color 0 black foreground with light cell background (\x1b[30...
Paired ANSI color 0 black foreground with light cell background (\x1b[30;47m) in Block 1 SGR output so all 16 SGR slots exercise real contrast without manufacturing a synthetic black-on-charcoal WCAG failure

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-03-SUMMARY.md

---

### Calibrated VLM rubric in vlmVisionFlywheel.ts so recessed dark terminal ...
Calibrated VLM rubric in vlmVisionFlywheel.ts so recessed dark terminal wells inside Carrara/Travertine stone chrome and Inter/flat-chrome accessibility overrides in Slate/High-Contrast are scored accurately without false daylight/serif penalties

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 446-03-SUMMARY.md

---

### terminalStartupSpamCount
`terminalStartupSpamCount`: raw OSC 133 (`\x1b]133;`, `]133;[ABCD]`) and shell bootstrap leakage (`__vox_(ps|preexec|precmd|trappost)`, `PROMPT_COMMAND=`, `precmd_functions`, `chpwd_functions`, `export PS1=`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-03-SUMMARY.md

---

### xtermHelperLeakCount
`xtermHelperLeakCount`: `.xterm-helper-textarea` bounding box `> 1px`, `opacity > 0.01`, non-transparent `backgroundColor`, outline, or `boxShadow`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-03-SUMMARY.md

---

### terminalFontProportionalLeakCount
`terminalFontProportionalLeakCount`: proportional serif font stacks (`Cinzel`, `Palatino`, `Book Antiqua`, `Georgia`, `Times New Roman`) OR 2D canvas `'MMMMMMMMMM'` vs `'iiiiiiiiii'` glyph width delta `> 0.1px` inside terminal output wells.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-03-SUMMARY.md

---

### terminalContrastViolationCount
`terminalContrastViolationCount`: WCAG 2.1 relative luminance with ancestor background alpha compositing + 1x1 canvas CSS Color 4 (`oklch`/`color-mix`) fallback, flagging `< 4.5:1` on `arcane`/`travertine`/`glacier`/`slate`/`void` and `< 7.0:1` AAA on `high-contrast`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-03-SUMMARY.md

---

### Extended crates/vox-gui/ui/src/lib/cvFlywheel.ts (428 non-blank LoC) wit...
Extended `crates/vox-gui/ui/src/lib/cvFlywheel.ts` (`428` non-blank LoC) with optional terminal audit fields on `GuiDomAuditMetrics` and `afterDom` on `UniversalCvFlywheelInput`, deducting `Math.min(0.6, terminalDefectCount * 0.3)` across `technicalQuality`, `promptAdherence`, and `styleConsistency` (forcing `visualAfterScore < 0.92` on any single defect) and emitting targeted remediation bootstrap directives.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-03-SUMMARY.md

---

### Runs all 8 TERMINAL_CV_STATES across 5 viewports (900x600, 1100x720, 128...
Runs all 8 `TERMINAL_CV_STATES` across 5 viewports (`900x600`, `1100x720`, `1280x800`, `1440x900`, `1920x1080`) × 4 themes (`arcane`, `slate`, `travertine`, `high-contrast`) = `112` cells when `VOX_CV_FLYWHEEL=1` is set on Chromium (`test-results/cv-flywheel-terminal/ledger-after.json` + screenshots).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-04-SUMMARY.md

---

### Enforces hard zero tolerances (outerOverflowPx === 0, clippedInteractive...
Enforces hard zero tolerances (`outerOverflowPx === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `unstyledNativeSelectCount === 0`, `axeViolations === 0`, `terminalStartupSpamCount === 0`, `xtermHelperLeakCount === 0`, `terminalFontProportionalLeakCount === 0`, `terminalContrastViolationCount === 0`) alongside `verdict.visualAfterScore >= 0.92` (`@ngi/platform-ssot` `2026.10.19-v3.5`) and `vlmCritique.vlmOverallScore >= 0.90`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-04-SUMMARY.md

---

### crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts & crates/vox-gui/ui/e2...
`crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts` & `crates/vox-gui/ui/e2e/review/terminalAudits.test.ts`: switched `ANSI0-BLK` fixture from `\x1b[30;47m` to `\x1b[30m` so ANSI 0 renders against dark `--terminal-well-bg` and updated the unit test expectation in lockstep.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-04-SUMMARY.md

---

### crates/vox-gui/ui/src/components/surfaces/Console/{BlockStreamView,Block...
`crates/vox-gui/ui/src/components/surfaces/Console/{BlockStreamView,BlockCard,TerminalTab,TerminalSessionPane,AgentTurnContent,HistorySearchPopover,TerminalApprovalCard,ConsoleSessionBar}.tsx`: fixed Axe `scrollable-region-focusable`, hidden inactive VT/Block layers (`hidden` / `aria-hidden`), fixed `travertine` `term-agent-model-pill` contrast (`bg-bg-elevated`), removed nested `<button>` inside `role="option"` in `HistorySearchPopover`, bounded `TerminalPaneApprovalBanner` at `< 1100px`, and prevented session tab label truncation in `ConsoleSessionBar`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-04-SUMMARY.md

---

### crates/vox-gui/ui/src/lib/cvFlywheel.ts
`crates/vox-gui/ui/src/lib/cvFlywheel.ts`: tightened flat-region Laplacian grain mask (`mag < 0.008` and 3×3 neighborhood luminance `span < 0.04`) so crisp 1px `#ffffff`-on-`#000000` hairlines in `high-contrast` do not inflate `noiseFloor`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 446-04-SUMMARY.md

## Lessons

### Unscoped ./target/debug/vox audit code timed out at 300s (exit 124) scan...
Unscoped `./target/debug/vox audit code` timed out at 300s (`exit 124`) scanning the entire workspace Rust AST; verified `vox/ui/theme-contract-drift` directly via `./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src` (`exit 0`, `✅ No issues found — code is clean!`) plus Vitest's `auditSourceTextAgainstThemeContract` full-`src/` sweep (`8/8` tests passing).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 446-01-SUMMARY.md

---

### Caller-authorized scope addition (xtermAddons.test.ts)
**Caller-authorized scope addition (`xtermAddons.test.ts`):** `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts` (`L68-86`) still asserted pre-Dual-Surface `#FAF6EE` (`arcane`) and `#ece5d6` (`travertine`/`glacier`) backgrounds for `resolveTerminalTheme(pref)`; paused for caller authorization and updated `expectedBg` to `#15110d` / `#171512` (with `>= 7.0` contrast threshold for `high-contrast`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 446-02-SUMMARY.md

---

### None after caller authorization of xtermAddons.test.ts.
None after caller authorization of `xtermAddons.test.ts`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 446-02-SUMMARY.md

---

### Initial edits to cvFlywheel.ts (458 non-blank LoC) and vlmVisionFlywheel...
Initial edits to `cvFlywheel.ts` (`458` non-blank LoC) and `vlmVisionFlywheel.ts` (`461` non-blank LoC) exceeded the `< 440` non-blank LoC ceiling; compacted multi-line JSDoc headers and object literals down to `428` and `405` non-blank LoC respectively with zero behavioral or type-signature changes.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 446-03-SUMMARY.md

---

### crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts
`crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts`: Added and exported `unicode11CharProperties(codePoint)` returning `(unicode11Wcwidth(codePoint) & 3) << 1` so `@xterm/xterm@6.0.0` `UnicodeService.extractWidth` receives valid cell widths (`1` for ASCII, `2` for CJK, `0` for combining marks) instead of `0`.

**Context:** Surfaced in structured phase code review.
**Source:** 446-REVIEW-FIX.md

---

### crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts
`crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts`: Added width-bit extraction assertions `(charProperties(cp, 0) >> 1) & 3` for ASCII, CJK, and combining glyphs.

**Context:** Surfaced in structured phase code review.
**Source:** 446-REVIEW-FIX.md

---

### crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
`crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx`: Preserved 1-argument `t.write(replaySnapshot.text)` followed by `requestAnimationFrame` `t.refresh(...)`.

**Context:** Surfaced in structured phase code review.
**Source:** 446-REVIEW-FIX.md

---

### Ledger: crates/vox-gui/ui/test-results/cv-flywheel-terminal/ledger-after...
**Ledger:** `crates/vox-gui/ui/test-results/cv-flywheel-terminal/ledger-after.json` (`ssotVersion: "2026.10.19-v3.5"`)

**Context:** Verified in phase acceptance checks.
**Source:** 446-VERIFICATION.md

---

### Total / Passed / Failed
**Total / Passed / Failed:** `112` / `112` / `0`

**Context:** Verified in phase acceptance checks.
**Source:** 446-VERIFICATION.md

## Patterns

### Terminal output wells and monospace blocks consume var(--font-terminal-m...
Terminal output wells and monospace blocks consume var(--font-terminal-mono) and var(--terminal-well-*) while outer workbench chrome retains Roman Scriptorium Cinzel/Palatino stone styling

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-01-SUMMARY.md

---

### Dual-Surface terminal CSS containment via --font-terminal-mono, --termin...
Dual-Surface terminal CSS containment via --font-terminal-mono, --terminal-well-*, and :not(.xterm):not(.xterm *):not([data-terminal-mono='true']):not([data-terminal-mono='true'] *):not([data-terminal-well='true']):not([data-terminal-well='true'] *) exclusions

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-01-SUMMARY.md

---

### Zero-box .xterm-helper-textarea isolation preventing focus-ring and ligh...
Zero-box .xterm-helper-textarea isolation preventing focus-ring and light-theme form background bleed

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-01-SUMMARY.md

---

### All terminal command previews, SGR <pre> wells, and prompt mirrors carry...
All terminal command previews, SGR <pre> wells, and prompt mirrors carry data-terminal-mono='true' + TERMINAL_MONO_STYLE, while output wells bind data-terminal-well='true' and bg-(--terminal-well-bg) text-(--terminal-well-fg) border-(--terminal-well-border)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-02-SUMMARY.md

---

### Interactive Console icon controls (session close, DiscoveryRail expand/c...
Interactive Console icon controls (session close, DiscoveryRail expand/collapse) enforce a minimum 24x24px (size-6) hit target with 4px-grid spacing

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-02-SUMMARY.md

---

### Dual-Surface wellBg/wellFg property indirection in TERMINAL_WELL_SPECS s...
Dual-Surface wellBg/wellFg property indirection in TERMINAL_WELL_SPECS so themeContract.test.ts GENERAL_DARK_HEX_BG_REGEX reports 0 violations

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-02-SUMMARY.md

---

### Shared TERMINAL_MONO_STYLE exported from terminalTheme.ts across BlockCa...
Shared TERMINAL_MONO_STYLE exported from terminalTheme.ts across BlockCard, ChatTerminalBlockCard, AgentTab, SendToAgent, ScriptoriumPromptEditor, TerminalApprovalCard, and DiscoveryRail

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-02-SUMMARY.md

---

### Responsive DiscoveryRail auto-collapse (<980px, preserving 1024px jsdom ...
Responsive DiscoveryRail auto-collapse (<980px, preserving 1024px jsdom default) and <1100px z-20 absolute overlay inside Console's relative flex body

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-02-SUMMARY.md

---

### Browser-injected audit and mock functions must pass standalone new Funct...
Browser-injected audit and mock functions must pass standalone new Function serialization tests with zero module-scope closures

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-03-SUMMARY.md

---

### Terminal CV states toggle deterministic window.__MOCK_TERM_* flags insid...
Terminal CV states toggle deterministic window.__MOCK_TERM_* flags inside Playwright state setup hooks while preserving VIEWPORTS (['wide', 'laptop', 'compact']) untouched

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-03-SUMMARY.md

---

### 100% self-contained .toString() browser serialization for page.evaluate ...
100% self-contained .toString() browser serialization for page.evaluate / addInitScript collectors and rich IPC mocks (verified via new Function standalone evaluation in Vitest)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-03-SUMMARY.md

---

### Dual-Surface WCAG 2.1 contrast auditing (4.5:1 standard, 7.0:1 AAA on hi...
Dual-Surface WCAG 2.1 contrast auditing (4.5:1 standard, 7.0:1 AAA on high-contrast) + canvas 'MMMMMMMMMM' vs 'iiiiiiiiii' fixed-width glyph verification

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-03-SUMMARY.md

---

### Smart 16-color SGR fixture pairing ANSI black (\x1b[30;47m) with a light...
Smart 16-color SGR fixture pairing ANSI black (\x1b[30;47m) with a light cell background on dark terminal wells while rendering \x1b[31..37m and \x1b[90..97m directly against --terminal-well-bg

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 446-03-SUMMARY.md

## Surprises

### Estimate variance on 446-01
Estimate variance on 446-01: planned 42000 tokens vs 14800 actual (0.35x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 446-01-PLAN.md, 446-01-SUMMARY.md

