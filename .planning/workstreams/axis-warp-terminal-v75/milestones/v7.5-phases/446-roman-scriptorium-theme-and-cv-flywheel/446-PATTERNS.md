# Phase 446: Roman Scriptorium Theme & CV Flywheel Convergence - Pattern Map

**Mapped:** 2026-11-15
**Files analyzed:** 34
**Analogs found:** 26 / 26 git-tracked analogs verified (`git ls-files --error-unmatch`); 2 working-tree Phase 443 files (`ChatTerminalBlockCard.tsx`, `ChatTerminalBlockCard.test.tsx`) flagged explicitly per `#3645`

---

## Anti-Hallucination & Discrepancy Ledger (Read First)

Before planning or editing any file in Phase 446, honor these 8 ground-truth discrepancies discovered between `446-RESEARCH.md` / `446-UI-SPEC.md` and actual workspace files:

1. **Cross-File Count Lock on `requiredSemanticCssVars` (`cv-flywheel.spec.ts:1250` & `SettingsView.tsx:1918`):**
   - `contracts/gui/theme-contract.v1.json:83-106` currently has **22** entries in `requiredSemanticCssVars`.
   - `SettingsView.tsx:1918` renders `{contractReport.resolvedSemanticVarCount} / {contractReport.requiredSemanticVarCount} Verified` (`themeContract.ts:94`).
   - `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts:1250` hard-asserts `await expect(ledgerCard).toContainText('22 / 22 Verified');`!
   - **Rule:** When adding `--font-terminal-mono`, `--terminal-well-bg`, and `--terminal-well-fg` to `requiredSemanticCssVars` (`22 -> 25`), you **MUST** include `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` in that plan's `files_modified` and update line `1250` to `'25 / 25 Verified'` (or `${THEME_CONTRACT_SPEC.requiredSemanticCssVars.length} / ${THEME_CONTRACT_SPEC.requiredSemanticCssVars.length} Verified`). Also verify `evaluateGuiThemeContract()` (`themeContract.ts:75-115`) checks `getComputedStyle(document.documentElement).getPropertyValue(v)` — so `--font-terminal-mono`, `--terminal-well-bg`, and `--terminal-well-fg` MUST all be defined on `:root` (`index.css:238` + `tokens.generated.css`) so `resolvedSemanticVarCount === 25` at runtime!
2. **`ChatTerminalBlockCard.tsx` `data-testid` Mismatch (`chat-term-block-card` vs `chat-terminal-block-card`):**
   - `446-RESEARCH.md:231` and `446-UI-SPEC.md:252,287` wrote `[data-testid="chat-terminal-block-card"]`.
   - Actual `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx:130,151` and `ChatTerminalBlockCard.test.tsx:124,132` use `data-testid="chat-term-block-card"` and `data-testid="chat-term-block-output"`! (Also note: `ChatTerminalBlockCard.{tsx,test.tsx}` are currently untracked `??` Phase 443 working-tree files — do NOT cite them as git-tracked analogs per `#3645`, and NEVER rename `chat-term-block-card` or `ChatTerminalBlockCard.test.tsx` will break.)
   - **Rule:** In `terminalAudits.ts` and `terminal-cv-flywheel.spec.ts`, query `[data-testid="chat-term-block-card"]` (optionally `[data-testid="chat-term-block-card"], [data-testid="chat-terminal-block-card"]` for forward safety).
3. **`playwright.config.ts` Port & Project Flag (`1420` + `--project=chromium`, NOT `4173` / `pnpm preview`):**
   - `446-RESEARCH.md:237` claimed `baseURL: 'http://127.0.0.1:4173'` and `webServer.command: 'pnpm preview'`.
   - Actual `crates/vox-gui/ui/playwright.config.ts:16,36-40` uses `baseURL: 'http://localhost:1420'` and `webServer.command: 'node ./node_modules/vite/bin/vite.js'`, with TWO browser projects (`chromium` at `L20-23` and `firefox-review` at `L27-33`).
   - **Rule:** Always pass `--project=chromium` on Playwright CLI invocations (matching `package.json:13` `"review:cv-flywheel"` and `cv-flywheel.spec.ts:10-12`), otherwise Playwright executes specs twice across Chromium + Firefox and hits timeout budgets.
4. **`index.css` Dark-Theme Text Clobbering (`L1105-1141`) & Exact `.font-mono` Selector (`L333-338`):**
   - `446-RESEARCH.md:135` sketched `main span.font-mono:not([data-code-literal="true"])`, whereas actual `index.css:333-338` is `main span.font-mono:not(pre *):not(code *), main div.font-mono:not(pre *):not(code *)`.
   - Furthermore, `446-RESEARCH.md` listed light-theme overrides (`L775-979`) and `L1041-1064`, `L1208-1220`, **but missed `L1105-1141`**: in dark themes (`:root`, `[data-theme='void']`, `[data-theme='slate']`, `[data-theme='high-contrast']`), `index.css:1105-1141` forces `[class*="text-[#7"]`, `[class*="text-[#8"]`, `[class*="text-[#9"]`, `.text-text-muted`, `.text-text-secondary` inside `main` to `color: var(--color-text-primary) !important;`! Both light (`L704-979`) AND dark (`L1041-1141`, `L1208-1220`) override blocks MUST exclude `:not(.xterm):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` so SGR spans and xterm rows preserve ANSI colors.
5. **Existing Unit Test Copy & Single-Textbox Contracts (`Console.test.tsx`, `SendToAgent.test.tsx`, `AgentTab.test.tsx`, `TerminalTab.test.tsx`, `sgrToSpans.test.tsx`):**
   - `Console.test.tsx:66,85` calls `screen.getByRole('textbox')` expecting **exact uniqueness (1 textbox)** on initial Console mount. `.xterm-helper-textarea` does not mount in `TerminalTab` mock mode unless `window.__VOX_E2E_REAL_XTERM__ === true`, and Phase 444/445 inputs (`HistorySearchPopover`, `term-ledger-filter`, `term-approval-edit-input`, `term-approval-steer-input`, `term-feedback-correct-input`) stay `null` until opened. Keep it that way!
   - `SendToAgent.test.tsx:20,32,45-46` asserts `screen.getByText('Send')` and `screen.getByText('Cancel')`. Either preserve button labels `'Send'` / `'Cancel'` when normalizing tokens OR update `SendToAgent.test.tsx` in the same task.
   - `AgentTab.test.tsx:34` asserts `screen.getByText('waiting for events…')` (`\u2026` horizontal ellipsis is NOT matched by `zeroEmojiAndDingbat.test.ts` `BANNED_GLYPH_RE`). Preserve `'waiting for events…'` unless updating `AgentTab.test.tsx`.
   - `TerminalTab.test.tsx:362,370` asserts `resolveTerminalPalette('arcane').cyan` (`#67e8f9` today -> `#5eead4` when updated to canonical UI-SPEC `arcane` cyan) and `resolveTerminalPalette('glacier').foreground === '#1c1813'`. Update `TerminalTab.test.tsx:362` if changing `ARCANE_ANSI_16.cyan` from `#67e8f9` to `#5eead4`, and keep hex strings **lowercase** (`#f87171` in `sgrToSpans.test.tsx:72`, `#1c1813` in `TerminalTab.test.tsx:370`).
6. **`console-workbench.spec.ts:69-72` Legacy `<select>` vs `<VoxSelect>`:**
   - Phase 443 converted `ConsoleSessionBar.tsx:143-151` `data-testid="term-security-level-select"` from raw `<select>` to `<VoxSelect>` (`<button role="combobox">`). Legacy `e2e/console-workbench.spec.ts:71` still calls `secSelect.selectOption('l4_system_exec')` (only valid on native `<select>`). Do NOT revert `<VoxSelect>` to `<select>` (`noRawNativeControlsGuard.test.ts` & `unstyledNativeSelectCount === 0` forbid native `<select>`); in `terminal-cv-flywheel.spec.ts` interact with `<VoxSelect>` via click -> option click.
7. **Workstream Sequencing (`Phases 444 & 445` Precede `Phase 446`):**
   - Phases `441`, `442`, `443`, `447` are executed on disk; Phases `444` (`ScriptoriumPromptEditor.tsx`, `HistorySearchPopover.tsx`, `JsonLedgerTable.tsx`, `RunbookPanel.tsx`) and `445` (`TerminalApprovalCard.tsx`, `AgentTurnContent.tsx`, `SlashMetaContent.tsx`, `useTerminalPendingApprovals.ts`) have approved `PLAN.md` files and execute before Phase 446. Pattern assignments below include exact `data-testid`s and Tauri mock hooks for Phase 444/445 components (`data-testid="term-approval-card"`, `vox_pending_approvals`, `term_runbooks_list`, `term_history_query`, `term_complete`).
8. **`vlmVisionFlywheel.ts` Dark-Void Assumption on Warm-Parchment Profiles (`L311-368`):**
   - `evaluateVlmSurfaceCritique` (`vlmVisionFlywheel.ts:361-367`) scores `aestheticAlignment` assuming `meanLuminance <= 65` (`darkVoidOk = pixels.meanLuminance >= 8 && pixels.meanLuminance <= 65`), penalizing light themes (`glacier` / `travertine`, where `meanLuminance` is ~`180–235`) by `-0.18` (`aestheticAlignment` drops to `0.82`). In `terminal-cv-flywheel.spec.ts`, pass theme-aware daylight expectations or assert `verdict.visualAfterScore >= 0.92` via `evaluateGuiSurfaceRubric` + `runUniversalCvFlywheelPass` (`cvFlywheel.ts:154-465`, which has zero dark-luminance bias!) while using `evaluateVlmSurfaceCritique` specifically on dark `arcane` / `void` / `high-contrast` profiles (or making `vlmVisionFlywheel.ts` inspect `document.documentElement.dataset.theme === 'travertine' || 'glacier'`).

---

## File Classification

| New / Modified File | Current LoC (Budget `< 450`) | Role | Data Flow | Closest Git-Tracked Analog (`git ls-files` verified) | Match Quality |
|---|---|---|---|---|---|
| `crates/vox-gui/ui/tokens/primitive.json` | 73 | Design-token source | Build-time JSON -> Style Dictionary | `crates/vox-gui/ui/tokens/primitive.json` | Exact |
| `crates/vox-gui/ui/src/styles/tokens*.generated.css` (4 files) | 41–56 each | Generated CSS vars | `pnpm --dir crates/vox-gui/ui run tokens:build` output | `crates/vox-gui/ui/style-dictionary.config.mjs` | Generated (never hand-edit) |
| `contracts/gui/theme-contract.v1.json` | 208 | Cross-layer theme SSOT | Read by TS Vitest/Settings + Rust `vox-code-audit` | `contracts/gui/theme-contract.v1.json` | Exact |
| `crates/vox-gui/ui/src/lib/themeContract.ts` | 571 (pre-existing >450; keep delta minimal) | Runtime & static theme contract | JSON contract + DOM/source regex checks | `crates/vox-gui/ui/src/lib/themeContract.ts` | Exact |
| `crates/vox-gui/ui/src/lib/themeContract.test.ts` | 196 | Vitest theme contract guard | Source scan across all enrolled surfaces/docks | `crates/vox-gui/ui/src/lib/themeContract.test.ts` | Exact |
| `crates/vox-code-audit/src/detectors/ui_theme_contract.rs` | 298 | Rust AST/regex audit detector | `theme-contract.v1.json` -> `vox/ui/theme-contract-drift` | `crates/vox-code-audit/src/detectors/ui_theme_contract.rs` | Exact |
| `crates/vox-gui/ui/src/index.css` | 1260 (CSS sheet) | Global Tailwind v4 `@theme` & theme overrides | `--font-terminal-mono`, `--terminal-well-*`, `:not(.xterm)` | `crates/vox-gui/ui/src/index.css` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` | 177 | 4-profile terminal & 16-ANSI resolver | CSS computed styles + fallback hex -> xterm `ITheme` & SGR | `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` | 391 (tight! keep `< 430`) | Live xterm.js + WebGL/DOM mount | PTY stream + `MutationObserver` theme sync + `__VOX_E2E_REAL_XTERM__` | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` | 380 | Pure SGR -> React span parser | ANSI sequence -> theme-aware `SgrSpan[]` | `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | 275 (+444/445 hooks; keep `< 430`) | Command / Agent / Ledger block card | `BlockDto` -> header + `sgrToSpans` well + action strip | `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` | 182 | Session tabs, split, control, security bar | Props -> semantic buttons + `<VoxSelect>` | `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` | 219 | Action Help + Runbooks right rail | Manifest / runbooks -> 4px-grid cards | `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` | Exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/AgentStrip.tsx` | 66 | Live agent badge strip | `AgentSnapshot[]` -> semantic pill strip | `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` | High |
| `crates/vox-gui/ui/src/components/surfaces/Console/AgentTab.tsx` | 53 | Streaming agent event log | `AgentEvent[]` -> semantic terminal-mono stream | `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` | High |
| `crates/vox-gui/ui/src/components/surfaces/Console/SendToAgent.tsx` | 65 | Inline block -> agent escalation form | Block text -> `<VoxSelect>` + semantic buttons | `crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx` | High |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` | 179 (untracked `??` in git) | Inline Chat `vox_terminal_*` tool card | `ToolCallEntry` -> terminal well + Open in Console | `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` (tracked analog) | High |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` | 380 | Chat workspace + `TerminalDockPanel` (`L275-315`) | `TerminalSessionPane compact` inside bottom/right dock | `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` | Exact |
| `crates/vox-gui/ui/e2e/review/terminalAudits.ts` (NEW) | 0 (target ~220 LoC) | Terminal-specific DOM/WCAG/canvas Playwright audits | `page.evaluate` -> `TerminalSpecificAuditMetrics` | `crates/vox-gui/ui/e2e/review/audits.ts` | Exact |
| `crates/vox-gui/ui/e2e/review/audits.test.ts` | 138 | Vitest unit suite for DOM/terminal audit helpers | Synthetic jsdom trees -> `auditTerminalInPage` assertions | `crates/vox-gui/ui/e2e/review/audits.test.ts` | Exact |
| `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` | 1564 | Master 33-surface CV Flywheel Playwright spec | Update `'25 / 25 Verified'` (`L1250`) | `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` | Exact |
| `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts` (NEW) | 0 (target ~360 LoC) | Dedicated 4-theme × 4-viewport terminal CV Flywheel | `CV_STAGE=before\|after` -> PNGs + JSON report + `>= 0.92` gate | `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` | Exact |
| `crates/vox-gui/ui/e2e/lib/tauriMock.ts` & `tauriMockRich.ts` | 2283 / 321 | Deterministic Tauri IPC mocks for Playwright | Rich 6-block ANSI/table/agent/failed/approval terminal fixture | `crates/vox-gui/ui/e2e/lib/tauriMock.ts` (`L1273-1470`) | Exact |
| `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` | 65 | Canonical v7.5 Architecture SSOT doc | Promote `status: "current"` + add §4 Dual-Surface & CV metrics | `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` | Exact |

---

## Pattern Assignments

### Scope 1: Token Pipeline, `theme-contract.v1.json`, `themeContract.ts`, & `vox-code-audit` (`vox/ui/theme-contract-drift`)

#### 1A. `crates/vox-gui/ui/tokens/primitive.json` (`L47-66`) & Style Dictionary Regeneration
**Analog:** `crates/vox-gui/ui/tokens/primitive.json` + `crates/vox-gui/ui/style-dictionary.config.mjs` (`L1-80`)

Current `tokens/primitive.json` (`L47-66`):
```json
  "font": {
    "display": { "value": "'Cinzel', 'Trajan Pro', 'Palatino Linotype', Georgia, serif" },
    "sans": { "value": "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" },
    "serif": { "value": "'Source Serif 4', 'Newsreader', Georgia, serif" },
    "mono": {
      "value": "'Palatino Linotype', 'Book Antiqua', 'Cinzel', Georgia, serif"
    },
    "family": {
      "display": { "value": "'Cinzel', 'Trajan Pro', 'Palatino Linotype', Georgia, serif" },
      "sans": { "value": "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif" },
      "serif": { "value": "'Source Serif 4', 'Newsreader', Georgia, serif" },
      "mono": {
        "value": "'Palatino Linotype', 'Book Antiqua', 'Cinzel', Georgia, serif"
      }
    },
```

**Exact addition pattern:**
- Add `"terminal-mono": { "value": "'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace" }` inside `"font"` (right after `"mono"` at `L53`) and `"terminal-mono"` inside `"font.family"` (right after `L64`).
- Note that `tokens/semantic.slate.json` (`L11-22`) overrides `font.{display,sans,serif,mono}` for `slate` (`data-theme="void"`), and `style-dictionary.config.mjs` (`L51-77`) merges `tokens/primitive.json` into all four generated outputs:
  - `src/styles/tokens.generated.css` (`:root, [data-theme="arcane"]`)
  - `src/styles/tokens.slate.generated.css` (`[data-theme="void"], [data-theme="slate"]`)
  - `src/styles/tokens.contrast.generated.css` (`[data-theme="high-contrast"]`)
  - `src/styles/tokens.travertine.generated.css` (`[data-theme="travertine"], [data-theme="glacier"], [data-theme="daylight"]`)
- **Exact regeneration command** (`crates/vox-gui/ui/package.json:10`):
  ```sh
  pnpm --dir crates/vox-gui/ui run tokens:build
  ```
  Produces `--font-terminal-mono: 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace;` across all 4 `tokens*.generated.css` files automatically.

#### 1B. `contracts/gui/theme-contract.v1.json` (`L83-135`, `L172-206`)
**Analog:** `contracts/gui/theme-contract.v1.json`

Current `requiredSemanticCssVars` (`L83-106`, 22 items) and `bannedTypographyPatterns` (`L128-135`) and `exclusions` (`L172-206`):
```json
  "requiredSemanticCssVars": [
    "--bg-primary",
    ...
    "--font-mono",
    "--font-terminal-mono",
    "--terminal-well-bg",
    "--terminal-well-fg"
  ],
```
```json
  "bannedTypographyPatterns": [
    "JetBrains Mono",
    "Fira Code",
    "Cascadia",
    "Consolas",
    "Courier New",
    "SFMono-Regular"
  ],
```

**Critical Guard Nuance across BOTH TS & Rust Detectors:**
1. **Rust `crates/vox-code-audit/src/detectors/ui_theme_contract.rs` (`L199-243`):**
   - Scans `.tsx`, `.ts`, `.css` under `crates/vox-gui/ui/src` (skipping `.test.`, `.spec.`, `/generated/`, `.generated.`, and `contract.exclusions.exactPaths`).
   - Checks `if line.contains(font)` for every entry in `bannedTypographyPatterns` (`"JetBrains Mono"`, `"SFMono-Regular"`).
   - **Notice:** `src/styles/tokens*.generated.css` is ALREADY skipped because of `.contains(".generated.")` (`ui_theme_contract.rs:212`)!
   - However, if `"JetBrains Mono"` or `"SFMono-Regular"` appears literally in `src/index.css` or `src/components/surfaces/Console/terminalTheme.ts`, `vox/ui/theme-contract-drift` **WILL FIRE** unless:
     - `index.css` references `--font-terminal-mono: var(--font-family-terminal-mono);` (or is scoped/excluded), AND `terminalTheme.ts` reads `getComputedStyle(root).getPropertyValue('--font-terminal-mono')` with fallback `'ui-monospace, monospace'` (which contains NEITHER `"JetBrains Mono"` NOR `"SFMono-Regular"`!), **OR** `crates/vox-gui/ui/src/index.css` / `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` is handled via an explicit `--font-terminal-mono` allowlist in `ui_theme_contract.rs` (`L230`: skip `banned_typography_patterns` check when `trimmed.starts_with("--font-terminal-mono")` or `line.contains("--font-terminal-mono")`) and `themeContract.ts` (`L517`).
   - **Best-of-both-worlds pattern (zero false positives, preserves full Console surface audit):**
     1. Put the literal `'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace` **only** in `tokens/primitive.json` (which generates `src/styles/tokens*.generated.css`, already skipped by `.generated.` in both `ui_theme_contract.rs:212` and outside `collectSurfaceSourceFiles()` in `themeContract.test.ts:19-46`).
     2. In `index.css` `@theme` (`L25`), bind `--font-terminal-mono: var(--font-family-terminal-mono);` — zero banned font literal in `index.css`!
     3. In `terminalTheme.ts`, resolve CSS `--font-terminal-mono` via `readCssVar('--font-terminal-mono') || 'ui-monospace, monospace'` — zero banned font literal in `terminalTheme.ts`!
     4. Additionally, in `ui_theme_contract.rs:230` and `themeContract.ts:517`, add a defensive guard skipping lines declaring `--font-terminal-mono` if `allowedTerminalMonoContexts` is added to `contracts/gui/theme-contract.v1.json`, while **NEVER** adding `Console/*` files to `exclusions.exactPaths` (so Console stays 100% protected against accidental raw hexes or banned fonts!).
2. **TS `crates/vox-gui/ui/src/lib/themeContract.ts` (`L298-299`, `L507-547`):**
   ```ts
   const GENERAL_DARK_HEX_BG_REGEX =
     /(?:background|backgroundColor)\s*:\s*['"](#[0-2][0-9a-fA-F]{5})['"]/g;
   ```
   - `auditSourceForThemeDrift` (`L472-560`) runs on EVERY `.ts`/`.tsx` file inside `src/components/surfaces/**` (`themeContract.test.ts:19-46`), including `Console/terminalTheme.ts`!
   - In `terminalTheme.ts`, **NEVER** write `background: '#15110d'` or `background: '#0f172a'` or `background: '#070709'` as an object literal property — `GENERAL_DARK_HEX_BG_REGEX` matches `background: '#0...'` / `background: '#1...'` / `background: '#2...'`!
   - **Exact safe indirection pattern** (already proven at `terminalTheme.ts:103-155`): store fallback background hexes in a non-`background` property name (`wellBg: '#15110d'`, `wellFg: '#f4ece1'`) or `Record<CanonicalThemeId, string>`, then assign `background: readCssVar('--terminal-well-bg') || profile.wellBg`!
   - Also update `cv-flywheel.spec.ts:1250` from `'22 / 22 Verified'` to `'25 / 25 Verified'` when adding the 3 terminal CSS vars to `requiredSemanticCssVars`.

---

### Scope 2: `crates/vox-gui/ui/src/index.css` Containment & Dual-Surface Variables

**Analog:** `crates/vox-gui/ui/src/index.css` (`L1-1260`)

Map every exact line block in `src/index.css` where global Roman Scriptorium typography, focus rings, or theme-clobbering `!important` rules currently leak into `.xterm` or `[data-terminal-well="true"]`:

1. **`@theme` Block (`L10-65`, specifically `L21-26`):**
   ```css
   --font-display: 'Cinzel', 'Trajan Pro', 'Palatino Linotype', Georgia, serif;
   --font-sans: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
   --font-serif: 'Source Serif 4', 'Newsreader', Georgia, serif;
   --font-mono: 'Palatino Linotype', 'Book Antiqua', 'Cinzel', Georgia, serif;
   --font-terminal-mono: var(--font-family-terminal-mono);
   ```
   *(Tailwind v4 `@theme` automatically exposes `font-terminal-mono` utility class from `--font-terminal-mono`, while `var(--font-family-terminal-mono)` comes from `tokens.generated.css` without introducing banned literal strings in `index.css`!)*

2. **Theme Profile Variable Declarations (`L238-288`):**
   Add `--terminal-well-bg`, `--terminal-well-fg`, `--terminal-well-border`, `--terminal-well-selection`, and `--terminal-well-cursor` on all 4 profile selectors:
   - `:root, [data-theme="arcane"]` (`L238-262`):
     ```css
     --terminal-well-bg: #15110d;
     --terminal-well-fg: #f4ece1;
     --terminal-well-border: rgba(214, 166, 86, 0.24);
     --terminal-well-selection: rgba(214, 166, 86, 0.28);
     --terminal-well-cursor: #e6be7a;
     ```
   - `[data-theme="void"], [data-theme="slate"]` (`L264-272`):
     ```css
     --terminal-well-bg: #0f172a;
     --terminal-well-fg: #e2e8f0;
     --terminal-well-border: rgba(56, 189, 248, 0.24);
     --terminal-well-selection: rgba(56, 189, 248, 0.26);
     --terminal-well-cursor: #38bdf8;
     ```
   - `[data-theme="travertine"], [data-theme="glacier"], [data-theme="daylight"]` (`L274-280`):
     ```css
     --terminal-well-bg: #f3ead8;
     --terminal-well-fg: #1c1813;
     --terminal-well-border: rgba(122, 92, 34, 0.28);
     --terminal-well-selection: rgba(181, 132, 42, 0.22);
     --terminal-well-cursor: #7a5c22;
     ```
   - `[data-theme="high-contrast"]` (`L282-288`):
     ```css
     --terminal-well-bg: #070709;
     --terminal-well-fg: #ffffff;
     --terminal-well-border: rgba(255, 212, 0, 0.65);
     --terminal-well-selection: rgba(255, 212, 0, 0.35);
     --terminal-well-cursor: #ffd400;
     ```

3. **Proportional `.font-mono` Rules (`L321-338` and `L534-545`):**
   - `L333-338`:
     ```css
     main span.font-mono:not(pre *):not(code *):not(.xterm *):not([data-terminal-well="true"] *),
     main div.font-mono:not(pre *):not(code *):not(.xterm):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *) {
       font-family: var(--font-sans) !important;
       letter-spacing: 0.01em;
     }
     ```
   - `L534-545`: Keep `.font-mono` proportional for non-terminal Roman surfaces (`romanFontCoverageRatio >= 0.85` in `vlmVisionFlywheel.ts` expects non-terminal surfaces to stay in Roman serif/sans!), and add explicit `.xterm` + `[data-terminal-well="true"]` monospace isolation right below `L545`:
     ```css
     .xterm,
     .xterm .xterm-rows,
     .xterm .xterm-rows span,
     [data-terminal-well="true"],
     [data-terminal-well="true"] pre,
     [data-terminal-well="true"] code,
     [data-terminal-well="true"] textarea {
       font-family: var(--font-terminal-mono) !important;
       font-variant-ligatures: none;
       letter-spacing: 0 !important;
     }

     .xterm .xterm-helper-textarea {
       opacity: 0 !important;
       pointer-events: none !important;
       outline: none !important;
       box-shadow: none !important;
       border: 0 !important;
       background: transparent !important;
       color: transparent !important;
     }
     ```

4. **Global Focus Ring (`L571-575`) & Light-Theme `textarea` Rule (`L752-767`):**
   - `L571`: Change `*:focus-visible` to `*:not(.xterm-helper-textarea):focus-visible` so typing in live xterm never draws a 2px brass focus rectangle at `(0,0)` over the canvas.
   - `L756`: Change `[data-theme='travertine'] main textarea` (and glacier/daylight equivalents at `L757-758`) to `main textarea:not(.xterm-helper-textarea):not([data-terminal-well="true"] textarea)` so light-theme form overrides neither paint `.xterm-helper-textarea` solid `#FBF7EE` nor override `[data-terminal-well="true"]` prompt editor backgrounds.

5. **Theme `!important` Clobbering Exclusions (`L775-979`, `L1041-1141`, `L1208-1220`):**
   - Append `:not(.xterm):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` to:
     - Light-theme dark-surface & light-text override selectors (`L778-842`, `L847-979`) so `.xterm` and `[data-terminal-well="true"]` keep `--terminal-well-bg` (`#F3EAD8` on travertine) and exact 16-color SGR foreground/background spans.
     - Dark-theme dark-surface & muted-hex text clobbering selectors (`L1041-1064`, **`L1105-1141`**, `L1208-1220`) — especially `L1105-1141` which currently rewrites `[class*="text-[#7"]`, `[class*="text-[#8"]`, `[class*="text-[#9"]`, `.text-text-muted`, `.text-text-secondary` to `--color-text-primary !important`.

---

### Scope 3: `terminalTheme.ts` & Console / Chat Component Normalization

#### 3A. `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` (`L1-177`)
**Analog:** `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts`

Current structure (`L31-177`):
- Already exports `TerminalPalette` (`ITheme & Record<Ansi16Key, string>`), `ANSI_16_KEYS`, `ARCANE_ANSI_16`, `SLATE_ANSI_16`, `TRAVERTINE_ANSI_16`, `CONTRAST_ANSI_16`, `resolveTerminalThemeId`, `resolveTerminalPalette`, and `resolveAnsi16Hex`.
- **Exact updates needed for 100% `446-UI-SPEC.md` compliance + Vitest parity:**
  1. Keep `wellBg` / `wellFg` indirection (lines `103-155`) instead of literal `background: '#15110d'` so `GENERAL_DARK_HEX_BG_REGEX` (`themeContract.ts:299`) stays at **0 violations**:
     ```ts
     interface TerminalWellSpec {
       wellBg: string;
       wellFg: string;
       cursor: string;
       cursorAccent: string;
       selectionBackground: string;
       ansi: Record<Ansi16Key, string>;
     }
     ```
  2. Read `--terminal-well-bg`, `--terminal-well-fg`, `--terminal-well-cursor`, and `--terminal-well-selection` via `readCssVar(...)` first, falling back to `spec.wellBg`, `spec.wellFg`, `spec.cursor`, `spec.selectionBackground`.
  3. Reconcile exact 64 ANSI hexes with `446-UI-SPEC.md` §Color (**all lowercase** to match `sgrToSpans.test.tsx:72` `#f87171` and `TerminalTab.test.tsx:370` `#1c1813`):
     - `ARCANE_ANSI_16`: `black: '#261e17'`, `red: '#e06c60'`, `green: '#84b87c'`, `yellow: '#e0b054'`, `blue: '#7ca6d8'`, `magenta: '#c88db8'`, `cyan: '#5eead4'` *(update `TerminalTab.test.tsx:362` from `'#67e8f9'` to `'#5eead4'`!)*, `white: '#e8dccb'`, `brightBlack: '#9c8b78'`, `brightRed: '#f28b80'`, `brightGreen: '#a3d49b'`, `brightYellow: '#f2ca78'`, `brightBlue: '#9ec1ec'`, `brightMagenta: '#dfb0d1'`, `brightCyan: '#88f3e2'`, `brightWhite: '#fbf5ec'`.
     - `SLATE_ANSI_16`: `black: '#1e293b'`, `red: '#f87171'`, `green: '#4ade80'`, `yellow: '#facc15'`, `blue: '#60a5fa'`, `magenta: '#c084fc'`, `cyan: '#22d3ee'`, `white: '#e2e8f0'`, `brightBlack: '#94a3b8'`, `brightRed: '#fca5a5'`, `brightGreen: '#86efac'`, `brightYellow: '#fde047'`, `brightBlue: '#93c5fd'`, `brightMagenta: '#d8b4fe'`, `brightCyan: '#67e8f9'`, `brightWhite: '#f8fafc'`.
     - `TRAVERTINE_ANSI_16`: `black: '#1c1813'`, `red: '#9e2a22'`, `green: '#1f6340'`, `yellow: '#7a520c'`, `blue: '#1e4e79'`, `magenta: '#6f2c68'`, `cyan: '#135e63'`, `white: '#574a3b'`, `brightBlack: '#665746'`, `brightRed: '#b5281e'`, `brightGreen: '#195436'`, `brightYellow: '#694406'`, `brightBlue: '#163f63'`, `brightMagenta: '#5c2156'`, `brightCyan: '#0e4c50'`, `brightWhite: '#120f0b'`.
     - `CONTRAST_ANSI_16`: `black: '#1c1c24'`, `red: '#ff6b6b'`, `green: '#51cf66'`, `yellow: '#ffd43b'`, `blue: '#74c0fc'`, `magenta: '#e599f7'`, `cyan: '#66d9e8'`, `white: '#f8f9fa'`, `brightBlack: '#adb5bd'`, `brightRed: '#ff8787'`, `brightGreen: '#69db7c'`, `brightYellow: '#ffe066'`, `brightBlue: '#91a7ff'`, `brightMagenta: '#eebefa'`, `brightCyan: '#99e9f2'`, `brightWhite: '#ffffff'`.
  4. Add a helper `resolveTerminalFontFamily(): string` returning `readCssVar('--font-terminal-mono') || 'ui-monospace, monospace'` (zero banned literal font strings in TS source!).

#### 3B. `TerminalTab.tsx` (`L128-390`) — Real xterm Playwright Hook, Cursor Determinism & Font Sync
**Analog:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx`

- **Line `128-136` (`isHeadlessMockEnv`):**
  ```ts
  function isHeadlessMockEnv(): boolean {
    if (typeof window === 'undefined') return true;
    if ((window as unknown as { __VOX_E2E_REAL_XTERM__?: boolean }).__VOX_E2E_REAL_XTERM__ === true) {
      return false;
    }
    if ((window as unknown as { __TAURI_CALLS__?: unknown }).__TAURI_CALLS__ !== undefined) {
      return true;
    }
    return typeof navigator !== 'undefined' && /jsdom/i.test(navigator.userAgent);
  }
  ```
  Setting `window.__VOX_E2E_REAL_XTERM__ = true` in `terminal-cv-flywheel.spec.ts` `addInitScript` mounts the **real** `@xterm/xterm` + WebGL/DOM renderer in Playwright while keeping existing `Console.test.tsx` / `cv-flywheel.spec.ts` in lightweight mock mode!
- **Line `211-225` (`new Terminal({...})`):**
  - Replace hardcoded `fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace'` (`L213` — which triggers `ui_theme_contract.rs` if not careful!) with `fontFamily: resolveTerminalFontFamily()`.
  - Set `cursorBlink: (window as unknown as { __VOX_E2E_REAL_XTERM__?: boolean }).__VOX_E2E_REAL_XTERM__ ? false : true` so two-frame Playwright pixel snapshots have deterministic cursor luminance (`0` blink jitter).
- **Line `372-389` (Outer container):**
  - Add `data-terminal-well="true"` and replace `style={{ backgroundColor: palette.background, color: palette.foreground }}` with CSS custom property consumption (`bg-[var(--terminal-well-bg)] text-[var(--terminal-well-fg)] border-[var(--terminal-well-border)]`).

#### 3C. `sgrToSpans.ts` (`L162-177`) & `BlockCard.tsx` (`L124-272`) & `ChatTerminalBlockCard.tsx` (`L127-177`)
**Analogs:** `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts`, `BlockCard.tsx`

1. **`sgrToSpans.ts` (`L165-168`):**
   ```ts
   const fg = state.inverse
     ? (state.bg ?? 'var(--terminal-well-bg, var(--color-bg-base))')
     : state.fg;
   const bg = state.inverse
     ? (state.fg ?? 'var(--terminal-well-fg, var(--color-text-primary))')
     : state.bg;
   ```
   Prevents `1.03:1` inverse-video contrast failure when a block well uses `--terminal-well-bg` inside a light-theme or warm-mahogany card.
2. **`BlockCard.tsx` (`L124-272`):**
   - Outer `<article>` (`L124-131`): keep Scriptorium Card Chrome (`bg-bg-surface border-border-subtle`); normalize `.5` spacing (`gap-1.5` -> `gap-2`, `px-1.5 py-0.5` -> `px-2 py-1`, `py-1.5` -> `py-2`, `px-2.5` -> `px-2`).
   - Command header `<code>` (`L154-156`) and output `<div data-testid={\`term-block-output-${block.id}\`}>` (`L217-249`): add `data-terminal-well="true"` on the output well container with `bg-[var(--terminal-well-bg)] text-[var(--terminal-well-fg)] border-t border-[var(--terminal-well-border)] font-terminal-mono text-xs leading-[1.45]`.
3. **`ChatTerminalBlockCard.tsx` (`L127-177`, untracked `??` Phase 443 working-tree file — analog `BlockCard.tsx`):**
   - Preserve `data-testid="chat-term-block-card"` (`L130`) and `data-testid="chat-term-block-output"` (`L151`) so `ChatTerminalBlockCard.test.tsx` stays 100% green.
   - Add `data-terminal-well="true"` on `data-testid="chat-term-block-output"` (`L149-165`), switch `font-mono` -> `font-terminal-mono`, `bg-bg-base/70` -> `bg-[var(--terminal-well-bg)] text-[var(--terminal-well-fg)]`, and normalize `.5` spacing (`px-2.5` -> `px-3`, `py-1.5` -> `py-2`, `gap-1.5` -> `gap-2`, `px-1.5 py-0.5` -> `px-2 py-1`) and `text-[10px]` -> `text-[11px]`.

#### 3D. Remaining Console Chrome Components (Scriptorium Chrome + 4px Grid + 4-Size Typography)
- **`ConsoleSessionBar.tsx` (`L63-180`):** Replace `py-1.5`, `gap-1.5`, `px-2.5`, `text-[10px]`, `font-bold`, and `font-mono` on chrome labels with `py-2`, `gap-2`, `px-2`, `text-[11px]`, `font-semibold`, `font-sans`. Keep `data-testid="term-security-level-select"` on `<VoxSelect>` (`L143-151`).
- **`DiscoveryRail.tsx` (`L59-217`):** Replace `text-[10px]`, `text-[9px]`, `py-1.5`, `px-2.5`, `mt-1.5`, `mb-1.5` with 4px spacing and `text-[11px]` / `text-xs`. Note Phase 444 (`444-03-PLAN.md`) also replaces `»` / `«` (`L68,108`) with `<Icon.chevR>` / `<Icon.chevL>`; verify zero guillemets or raw dingbats remain.
- **`AgentStrip.tsx` (`L14-64`), `AgentTab.tsx` (`L12-51`), `SendToAgent.tsx` (`L18-63`), `TermSearchBar.tsx` (`L28-76`), `ConsoleSplitGrid.tsx` (`L121-190`):**
  - Replace hardcoded Tailwind palette classes (`bg-zinc-*`, `border-zinc-*`, `text-zinc-*`, `bg-emerald-*`, `bg-sky-*`, `bg-amber-*`, `bg-rose-*`, `bg-indigo-*`) with semantic Roman tokens (`bg-bg-surface`, `bg-bg-elevated`, `border-border-subtle`, `text-text-primary`, `text-text-secondary`, `text-text-muted`, `text-status-pass`, `text-status-warn`, `text-status-fail`, `bg-accent/15 text-accent border-accent/40`).
  - In `SendToAgent.tsx`: replace raw `<select>` (`L31-42`) with `<VoxSelect>` (matching `noRawNativeControlsGuard.test.ts` and `unstyledNativeSelectCount === 0`) while keeping button text `'Send'` and `'Cancel'` so `SendToAgent.test.tsx` passes unchanged.
  - In `AgentTab.tsx`: use `data-terminal-well="true"` + `font-terminal-mono` for event stream lines while keeping empty-state text `'waiting for events…'` (`AgentTab.test.tsx:34`).
- **Phase 444 & 445 Components to Verify/Polish in Phase 446:**
  - Phase 444: `ScriptoriumPromptEditor.tsx` (`[aria-label="console input"]` inside `data-terminal-well="true"`), `HistorySearchPopover.tsx` (`data-testid="term-history-search"`), `JsonLedgerTable.tsx` (`data-testid="term-ledger-table"`), `RunbookPanel.tsx`.
  - Phase 445: `TerminalApprovalCard.tsx` (`data-testid="term-approval-card"`, `<span data-roman-wax-seal="true">`), `AgentTurnContent.tsx` (`data-testid="term-agent-turn-content"`, `data-testid="term-agent-proposed-cmd"`), `SlashMetaContent.tsx` (`data-testid="term-slash-meta-content"`).

---

### Scope 4: Playwright CV Flywheel (`terminalAudits.ts`, `cvFlywheel.ts`, `vlmVisionFlywheel.ts`, Mocks, & `CV_STAGE` CLI)

#### 4A. Exact `visualAfterScore` Formula & `CV_STAGE` Mechanics
**Analogs:** `crates/vox-gui/ui/src/lib/cvFlywheel.ts` (`L154-465`) + `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`L56-107`, `L582-755`)

How `visualAfterScore` is computed end-to-end:
1. **Pixel Metrics (`decodePngRgbaInPage` -> `computeRgbaFrameCvMetrics`, `cvFlywheel.ts:70-152`):**
   - Screenshot PNG is downsampled `0.25x` (`360x225` at `1440x900`) on an in-page `<canvas>` (`cv-flywheel.spec.ts:89-106`) and passed to `computeRgbaFrameCvMetrics(rgba, w, h)` returning `{ edgeSharpness, luminanceContrast, noiseFloor, meanLuminance }`.
2. **Rubric Scores (`evaluateGuiSurfaceRubric(dom, pixels)`, `cvFlywheel.ts:154-275`):**
   - Computes 5 rubric axes (`0..1`): `promptAdherence`, `structuralIntegrity`, `materialFidelity`, `visualClarity`, `aestheticCohesion` from `GuiDomAuditMetrics` (`outerOverflowPx`, `scrollHostOverflowPx`, `clippedInteractiveCount`, `overlappingControlPairs`, `lightThemeContrastLeakCount`, `unstyledNativeSelectCount`, `rawEmojiCount`, `axeViolations`, `iconIssueCount`, `digitalColorLeakCount`, etc.) + `pixels`.
   - Note `cvFlywheel.ts:251`: `digitalColorLeakCount` deducts `0.06` per leak from `aestheticCohesion`. Ensure `audits.ts` (`auditDomQualityInPage`, `L388-560`) excludes `[data-terminal-well="true"]` and `.xterm` from any non-terminal digital-color or monospace penalty!
3. **Weighted Score & `runUniversalCvFlywheelPass` (`cvFlywheel.ts:392-465`):**
   - `weightedRubricScore = promptAdherence * 0.25 + structuralIntegrity * 0.25 + materialFidelity * 0.20 + visualClarity * 0.15 + aestheticCohesion * 0.15`.
   - Inside `runUniversalCvFlywheelPass` (`L398-454`):
     ```ts
     const afterPixelQuality =
       clamp01(input.afterMetrics.edgeSharpness / 28) * 0.4 +
       clamp01(input.afterMetrics.luminanceContrast / 0.22) * 0.4 +
       clamp01(1 - input.afterMetrics.noiseFloor / 18) * 0.2;
     const visualAfterScore = round4(
       input.afterMetrics.rubricScore * 0.78 + afterPixelQuality * 0.22,
     );
     ```
   - `flywheelConverged` (`L447-452`) requires: `visualAfterScore >= targetScore` (default `0.85`; Phase 446 asserts `>= 0.92` on desktop `1440x900` / `1920x1080` and `>= 0.90` on `1280x800` / `900x600`), `afterGrounded === true`, and `promptAdherence >= 0.80 && structuralIntegrity >= 0.80`.

#### 4B. Exact CLI Commands for `CV_STAGE=before` and `CV_STAGE=after`
**Analog:** `crates/vox-gui/ui/playwright.config.ts` (`L16,20-39`) + `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`L9-13,56-59`)

- `playwright.config.ts` auto-starts Vite on `http://localhost:1420` (`reuseExistingServer: !process.env.CI`).
- Note line `59` of `cv-flywheel.spec.ts`: `const RUN_CV_FLYWHEEL = !process.env.CI || process.env.VOX_CV_FLYWHEEL === '1';` — always pass `VOX_CV_FLYWHEEL=1` when `CI` might be set in shell environments, and always pass `--project=chromium`:
  ```sh
  # Pass 0 — Capture BEFORE baselines (writes artifacts/cv-flywheel/terminal/before-metrics.json + before PNGs)
  timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=before pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium

  # Pass 1..N — Run AFTER verification & convergence assertions (writes artifacts/cv-flywheel/terminal/terminal-cv-report.json + after PNGs)
  timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=after CV_PASS=1 pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium

  # Regression check on master 33-surface CV flywheel (verifies '25 / 25 Verified' on Settings + Console/Chat surfaces)
  timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts --project=chromium
  ```

#### 4C. `crates/vox-gui/ui/e2e/review/terminalAudits.ts` (NEW) & `audits.ts` Exclusion Hook
**Analog:** `crates/vox-gui/ui/e2e/review/audits.ts` (`L1-575`, self-contained `page.evaluate` serialization pattern: **zero outer-scope helper references inside `page.evaluate`!**)

`auditTerminalInPage(page: Page)` in `terminalAudits.ts` MUST compute and return `TerminalSpecificAuditMetrics`:
```ts
export interface TerminalSpecificAuditMetrics {
  terminalMonoComputedOk: boolean;       // computed font-family on .xterm-rows & [data-terminal-well="true"] matches /JetBrains Mono|ui-monospace|monospace/i and NOT /Palatino|Cinzel|Source Serif/i
  sgrSpanMinContrastRatio: number;       // min WCAG 2.1 contrast across all visible [data-terminal-well="true"] spans vs effective ancestor background (>= 4.5)
  ansi16MinContrastRatio: number;        // algebraic WCAG 2.1 min contrast across all 16 resolveTerminalPalette(themeId) colors vs palette.background (>= 4.5)
  xtermCanvasNonBlank: boolean;          // if real xterm mounted: canvas width > 0 && height > 0 (or non-empty .xterm-rows textContent when WebGL falls back to DOM in headless Chromium)
  xtermHelperTextareaLeakCount: number;  // 0 (.xterm-helper-textarea has opacity === '0', outlineStyle === 'none' or outlineWidth === '0px', pointerEvents === 'none')
  promptEditorClipped: boolean;          // false ([aria-label="console input"] bounding rect bottom <= window.innerHeight && right <= window.innerWidth)
  stackedRedundantWellCount: number;     // 0 (at most 1 visible primary session stream well per TerminalSessionPane — never BlockStreamView + TerminalTab stacked simultaneously)
  rawOscLeakCount: number;               // 0 (no visible text matching /\x1b\]633;|\]633;[A-P]|__vox_precmd|__vox_trappost/)
}
```
> **Headless Chromium WebGL Note (`xtermCanvasNonBlank`):** In headless Chromium without GPU hardware, `@xterm/addon-webgl` either renders into a WebGL `<canvas>` (`preserveDrawingBuffer` is false by default in WebGL, so `canvas.getContext('2d')` returns `null`!) OR `attachTerminalAddons` cleanly falls back to xterm's DOM renderer (`.xterm-rows`). Therefore, verify `xtermCanvasNonBlank` by checking `(webglCanvasRect.width > 0 && webglCanvasRect.height > 0 && xtermRowsText.trim().length > 0)` plus `decodePngRgbaInPage(page, wellScreenshotBuf)` (`luminanceContrast > 0.03 && edgeSharpness > 1.0`) — NEVER call `canvas.getContext('2d')` on an active WebGL canvas!

Also check `crates/vox-gui/ui/e2e/review/audits.ts` (`L290-335` light-theme contrast leak scanner & `L475-525` digital color leak scanner):
- Ensure `audits.ts` skips `.xterm-helper-textarea` (invisible `0x0`/`1x1` xterm input helper) and resolves effective background upward to `[data-terminal-well="true"]` (so dark `#15110d` terminal wells inside `arcane`/`void`/`high-contrast` or `#f3ead8` wells inside `travertine` compute contrast against the well's actual `backgroundColor`, never assuming `main`'s background!).

#### 4D. Deterministic Rich Terminal Fixtures in `tauriMock.ts` & `tauriMockRich.ts`
**Analogs:** `crates/vox-gui/ui/e2e/lib/tauriMock.ts` (`L1273-1470`), `crates/vox-gui/ui/e2e/lib/tauriMockRich.ts` (`L281-304`)

Remember the **cardinal serialization rule** of `tauriMockShared.ts` (`L5-11`): `installTauriMock` and `installTauriMockRich` are serialized via `.toString()` into `page.addInitScript` — **NEVER reference imported functions/constants inside `installTauriMock` or `installTauriMockRich`!**

Seed `s-1` in `tauriMockRich.ts` (or via a `window.__VOX_TERMINAL_RICH_FIXTURE__` flag read inside `term_session_attach` / `term_session_list` / `invoke_mcp_tool`) with all 6 visual block archetypes + Phase 444/445 IPC handlers:
1. **Block 1 (`Shell`, `status: 'Ok'`, `exit_code: 0`):** Full 16-color SGR ANSI showcase (`\x1b[30m..\x1b[37m`, `\x1b[90m..\x1b[97m`, bold `\x1b[1;32m`, inverse `\x1b[7m PASS \x1b[0m`) so `sgrSpanMinContrastRatio` measures all 16 ANSI colors in live DOM!
2. **Block 2 (`Shell`, `!nu` structured JSON, `status: 'Ok'`, `exit_code: 0`):** JSON array output (`[{"crate":"vox-terminal-core","tests":42,"status":"pass"},{"crate":"vox-gui","tests":128,"status":"pass"}]`) exercising `BlockCard` JSON / `JsonLedgerTable` (`data-testid="term-ledger-tab-table"`).
3. **Block 3 (`Shell`, `status: 'Failed'`, `exit_code: 101`):** Rust compiler diagnostic with SGR red/yellow/cyan spans (`error[E0308]: mismatched types`), surfacing Phase 445 `Explain Failure` (`data-testid="term-block-explain-failure"`) and `Fix Command` (`data-testid="term-block-fix-command"`).
4. **Block 4 (`AgentTurn`, `status: 'Ok'`):** Streaming agent response containing a proposed command block (`data-testid="term-agent-turn-content"`, `data-testid="term-agent-proposed-cmd"`).
5. **Block 5 (`SlashCommand` / `Meta`, `status: 'Ok'`):** `/budget` or `/model` digest (`data-testid="term-slash-meta-content"`).
6. **Inline Co-Pilot Pending Approval (`vox_pending_approvals` in `tauriMockRich.ts:282-299`):** Include a session-scoped terminal approval item (`session_id: 's-1'`, `approval_id: 'apr-term-1'`, `tool: 'vox_terminal_exec'`, `command: 'cargo test -p vox-terminal-core'`, `summary: 'cargo test -p vox-terminal-core'`, `risk_tier: 'Ask'`, `floor_category: null`, `credential_prompt: false`, `requested_at_ms: Date.now() - 5000`) when `window.__VOX_TERMINAL_RICH_FIXTURE__ === true` so `<TerminalApprovalCard data-testid="term-approval-card">` renders in Console! (Gate behind `window.__VOX_TERMINAL_RICH_FIXTURE__` if needed so existing `cv-flywheel.spec.ts` expectations on `console` aren't disrupted.)
7. **Chat `ChatTerminalBlockCard` fixture:** Seed a rich Chat session turn containing a completed `vox_terminal_exec` tool call so navigating to `/#view=chat` renders `[data-testid="chat-term-block-card"]` and opening `TerminalDockPanel` (`data-testid="chat-dock-tab-terminal"` or workbench dock toggle) renders `<TerminalSessionPane compact>`.
8. **Phase 444/445 Tauri IPC mock stubs in `tauriMock.ts` (`L1275`):** Return safe deterministic payloads for `term_complete`, `term_history_query`, `term_runbooks_list`, `term_session_voxify`, `term_session_set_control`, `term_session_set_security`, `term_agent_prompt`, `term_agent_explain_fix`, `term_agent_cancel`, `term_slash_exec`, `term_block_feedback`, and `resolve_terminal_approval` so zero unhandled IPC warnings occur during E2E runs.

---

### Scope 5: Architecture SSOT Promotion (`docs/src/architecture/warp-roman-terminal-v75-research-2026.md`)

**Analog:** `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` (`L1-65`)

Current YAML frontmatter (`L1-16`):
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

**Exact update rules (`AGENTS.md §Authored Markdown Frontmatter`):**
1. Change `status: "research"` -> `status: "current"` (`L5`).
2. Keep `category: "Architecture SSOTs"`, `topic: "desktop-app"`, `domain: "gui"`, `authorship: "llm-assisted"`, `authorship_model: "gemini-4-carbon"`. Do **NOT** add `last_updated` or `human_attested_by`.
3. Keep the `<!-- kb:summary --> ... <!-- /kb:summary -->` block (`L14-16`) directly below H1 and strictly within **40–120 words** (currently ~92 words — passes `vox-doc-pipeline` cleanly).
4. Append `## 4. Dual-Surface Roman Scriptorium Contract & CV Flywheel Convergence (Phase 446)` documenting:
   - The `--font-terminal-mono` / `--terminal-well-*` token contract (`contracts/gui/theme-contract.v1.json` + `vox/ui/theme-contract-drift` enforcement).
   - The 4-profile (`arcane`, `slate`, `travertine`, `high-contrast`) 64-entry WCAG AA (`>= 4.5:1`) ANSI palette table.
   - Measured before/after CV Flywheel convergence table (`4` profiles × `4` viewports `1440x900`, `1920x1080`, `1280x800`, `900x600`, `visualAfterScore >= 0.92` desktop / `>= 0.90` compact, `0` axe/overflow/clipping/contrast leaks).
5. **Doctest Warning:** Do NOT add raw ` ```vox ` code blocks unless they compile standalone or begin with `// vox:skip <reason>`. Use ` ```json `, ` ```css `, ` ```ts `, or markdown tables.
6. **Verified Lint Command (`0` exit code confirmed in workspace):**
   ```sh
   timeout 30s ./target/debug/vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md
   ```
   *(Note: `--paths` takes path relative to `docs/src/`, i.e. `architecture/warp-roman-terminal-v75-research-2026.md`, resolving to `./docs/src/architecture/warp-roman-terminal-v75-research-2026.md`.)*

---

## Shared Patterns & Guardrails

1. **4px Spacing & 4-Size Typography Guard:**
   - Allowed sizes in touched Console/Chat components: `text-[11px]`, `text-xs` (`12px`), `text-sm` (`14px`), `text-lg` (`18px`). Zero `text-[9px]`, `text-[10px]`, `text-[13px]`.
   - Allowed weights: `font-normal` (`400`) and `font-semibold` (`600`). Zero `font-bold` (`700`) or `font-medium` (`500`) in newly written/normalized terminal chrome.
   - Zero `.5` fractional Tailwind spacing (`p-0.5`, `px-1.5`, `px-2.5`, `py-0.5`, `py-1.5`, `gap-1.5`, `mt-1.5`, `mb-1.5`, `size-3.5`) in touched files.
2. **UI Static Guards (`crates/vox-gui/ui/src/guards/`):**
   - `zeroEmojiAndDingbat.test.ts`: Zero unicode dingbats/symbols (`✕`, `✓`, `⚠`, `●`, `▸`, `▾`, `»`, `«`, emojis) — always use `<Icon.* />` from `@/components/icons`.
   - `noRawNativeControlsGuard.test.ts`: Zero raw `<select>` elements — always use `<VoxSelect>`.
   - Toast errors: Always sanitize via `sanitizeErrorForToast(err)` with `cause: 'backend-error'`.
3. **File Size Budget (`layers.toml` / `D-10`):**
   - Keep every new or modified `.ts`/`.tsx`/`.rs` file `< 450` non-blank LoC (except pre-existing large monoliths `themeContract.ts` and `cv-flywheel.spec.ts` where edits must stay minimal `< 25` lines; put all new terminal audit & flywheel logic in dedicated `e2e/review/terminalAudits.ts` and `e2e/review/terminal-cv-flywheel.spec.ts`).
4. **Formatting & Git Discipline (`00-core.md`):**
   - Rust edits (`ui_theme_contract.rs`): run `rustfmt --edition 2024 crates/vox-code-audit/src/detectors/ui_theme_contract.rs` (never `cargo fmt`).
   - Never run `git add`, `commit`, `stash`, `checkout`, `restore`, `reset`, `clean`, or `push`.

---

## Verification Command Reference

Emit one command per terminal step (no `&&` or `;` chaining):

```sh
# 1. Regenerate Style Dictionary CSS tokens across all 4 theme profiles
pnpm --dir crates/vox-gui/ui run tokens:build

# 2. Run Vitest theme contract, SGR, terminal theme, Console, Chat, and UI static guards
timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/themeContract.test.ts src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/sgrToSpans.test.tsx src/components/surfaces/Console/Console.test.tsx src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts e2e/review/audits.test.ts

# 3. TypeScript compile check
timeout 120s pnpm --dir crates/vox-gui/ui run typecheck

# 4. Rust vox-code-audit detector unit tests & workspace vox/ui/theme-contract-drift check
timeout 300s cargo test -p vox-code-audit ui_theme_contract

# 5. Architecture layer & LoC budget check
timeout 180s cargo run -p vox-arch-check

# 6. Terminal CV Flywheel Pass 0 (before baseline) & Pass 1..N (after convergence)
timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=before pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium
timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=after CV_PASS=1 pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium

# 7. Master 33-surface CV Flywheel regression check (Settings '25 / 25 Verified' + Console + Chat)
timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts --project=chromium

# 8. Architecture SSOT frontmatter & doctest lint
timeout 30s ./target/debug/vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md
```
