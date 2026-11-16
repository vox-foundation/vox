# Phase 444 — UI Review

**Audited:** 2026-11-15
**Baseline:** `444-UI-SPEC.md` (`TERM-WARP-01..04`) + Roman Scriptorium design system (`tokens.*.generated.css`, `src/index.css`)
**Screenshots:** not captured (code-level adversarial audit per orchestrator directive; no dev server started)

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 2/4 | `WARNING`: Generic single-word `Run` / `Clear` / `Insert` CTAs, non-7-bit-ASCII `\u2014` em-dashes in paste/voxify banners, and empty/error state copy drift from `444-UI-SPEC.md`. |
| 2. Visuals | 3/4 | `BLOCKER` (shared): `main textarea` `!important` opaque background in `src/index.css` hides `<pre>` syntax/ghost layer in live renders; `WARNING`: missing fuzzy-match & `[REDACTED]` highlights in `HistorySearchPopover`, badge taxonomy drift, and direct `lucide-react` import in `JsonLedgerTable`. |
| 3. Color | 2/4 | `BLOCKER`: Undefined CSS variables `--color-status-ok` and `--color-status-error` break string highlighting, destructive `Confirm Clear`, and `exit` status chips; `.text-brass` is clobbered to `#1C140C !important` in light themes (`src/index.css:873-875`). |
| 4. Typography | 3/4 | `BLOCKER` (shared): `pre { font-family: ... !important }` (`src/index.css:535`) forces `Palatino` serif onto `<pre>` while `<textarea>` uses `JetBrains Mono`, and `PROMPT_TOKEN_CLASSES` uses `font-semibold` (`600`) against `font-normal` (`400`) `<textarea>`; `WARNING`: legacy `9px`/`10px`/`13px` in `Omnibar.tsx`. |
| 5. Spacing | 3/4 | Zero `.5` fractional classes across all 16 newly built/normalized Console, `IntelliSensePopover`, and `OmnibarRunbookSheet` files; `WARNING`: `<pre>` `overflow-hidden` vs `<textarea>` `overflow-y-auto` (`8px` scrollbar wrap drift), `px-3 py-1` + `28px` floor vs `p-0` + `24px`, and legacy `.5` classes in `Omnibar.tsx`. |
| 6. Experience Design | 2/4 | `BLOCKER`: `HistorySearchPopover` `Enter` / primary row click passes `'accept'`, which `ScriptoriumPromptEditor.tsx:270` ignores—inserting instead of executing despite advertising `Enter Run`; `WARNING`: missing `initialQuery` seed on `Ctrl+R`, non-prefix `ArrowUp`, and missing required `{{param}}` validation. |

**Overall Score:** **15/24** (`3 BLOCKER`, `18 WARNING`)

---

## Top 3 Priority Fixes

1. **`[BLOCKER]` Isolate `ScriptoriumPromptEditor` mirror & `<textarea>` from `main` `@layer base !important` overrides and synchronize wrap/weight metrics** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:344-382`, `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts:18-29`) — Users in live `<main>` renders cannot see syntax highlighting or ghost completions at all because `src/index.css:756-762,1209-1215` forces an opaque `background-color !important`, opaque `color !important`, and `1.5px solid ... !important` border onto `main textarea` (`z-10`), while `src/index.css:535,541` forces `Palatino` serif onto `<pre>` (`font-family: ... !important`). Without touching `src/index.css` (frozen for Phase 446 per `444-UI-SPEC.md:86`), replace `<pre>` at `ScriptoriumPromptEditor.tsx:344` with `<div aria-hidden="true" data-testid="scriptorium-highlight-layer">`, apply `style.setProperty('background-color', 'transparent', 'important')`, `style.setProperty('color', value.length > 0 ? 'transparent' : 'var(--color-text-primary)', 'important')`, and `style.setProperty('border', 'none', 'important')` on `textareaRef.current` in a `useLayoutEffect`, align both layers to `p-0` + `overflow-y-auto` (`scrollbar-gutter: stable`) + `24px` (`h-6`) floor, and drop `font-semibold` from `PROMPT_TOKEN_CLASSES` (`promptHighlighter.ts:19,24-26`) so glyph widths match 1:1.
2. **`[BLOCKER]` Replace nonexistent `--color-status-ok` and `--color-status-error` variables and avoid `.text-brass` light-theme clobbering** (`crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts:19,22,25-26`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:214,315`) — Neither `--color-status-ok` nor `--color-status-error` exists in `tokens.*.generated.css` or `themeContract.ts` (the canonical tokens are `--color-status-pass` and `--color-status-fail`), causing string literals, destructive `Confirm Clear`, and `exit 0` vs `exit 1` chips to render with no semantic color. Replace `--color-status-ok` with `--color-status-pass`, `--color-status-error` with `--color-status-fail`, and replace `.text-brass` (overridden to `#1C140C !important` in light themes by `src/index.css:873-875`) with `text-accent` (`var(--color-accent-default)`) or theme-aware `resolveAnsi16Palette(theme)` slots (`444-UI-SPEC.md:109,114-121`).
3. **`[BLOCKER]` Honor `HistorySearchPopover`'s `action === 'accept'` parameter on `Enter` and primary row click** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:270-273`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx:150-156`) — `HistorySearchPopover` advertises `Enter Run - Tab Insert` (`HistorySearchPopover.tsx:186-191,301,325,337`), passing `'accept'` on `Enter`/row-click and `'insert'` on `Tab`/`Insert`-button, yet `ScriptoriumPromptEditor.tsx:270` drops the second callback argument (`onSelect={(cmd) => ...}`) and only populates the buffer. Handle `(cmd, action)`: when `action === 'accept'`, record history, clear the buffer, and dispatch `onSubmit(classifyPromptIntent(cmd, effectiveMode).toSubmitPayload(cmd))`; when `action === 'insert'`, populate the editor and focus; also pass `initialQuery={value.includes('\n') ? '' : value}` at `L268`.

---

## Static Guards & Structural Contract Verification

| Check | Status | Evidence |
|-------|--------|----------|
| `zeroEmojiAndDingbat` (`src/guards/zeroEmojiAndDingbat.test.ts`) | **PASS** | Zero banned emojis, dingbats, geometric shapes, guillemets (`«`/`»` replaced with `Icon.chevronLeft`/`Icon.chevronRight` at `DiscoveryRail.tsx:93,125`), or return glyphs (`\u21b5` removed at `IntelliSensePopover.tsx:150-156`) across all 17 audited files. |
| `noRawNativeControlsGuard` (`src/guards/noRawNativeControlsGuard.test.ts`) | **PASS** | Zero raw `<select>` or `<input type="range">` controls; `VoxSelect` used at `ScriptoriumPromptEditor.tsx:303`, `RunbookPanel.tsx:168`, `OmnibarRunbookSheet.tsx:179`, and `ConsoleSessionBar.tsx:159`. |
| `toastBodyGuard` (`src/guards/toastBodyGuard.test.ts`) | **PASS** | Every `toast.error` catch site routes through `sanitizeErrorForToast(...)` (`useConsoleWarpActions.ts:93`, `TerminalSessionPane.tsx:201,250,270,281`, `Console.tsx:251,269,306,330,372`). |
| Unique `getByRole('textbox')` & `queryByRole('dialog') === null` | **PASS** | `HistorySearchPopover.tsx:162,197,256` returns `null` when closed and uses `role="region"` + `<input type="search">` (`role="searchbox"`); `BlockCard.tsx:125` defaults `ledgerView` to `'json'`; `DiscoveryRail.tsx:50` defaults `internalTab` to `'help'`. |
| Zero nested `<button>` controls | **PASS** | Sibling buttons inside `<div role="option">` (`HistorySearchPopover.tsx:287-330`), `<div className="rounded border...">` (`RunbookPanel.tsx:144-199`), and session tab container (`ConsoleSessionBar.tsx:89-128`). |
| Zero `dangerouslySetInnerHTML` | **PASS** | Prompt highlighter renders pure React `<span>` array (`ScriptoriumPromptEditor.tsx:351`). |
| TanStack Table v9 (`@tanstack/react-table` `9.0.0-alpha.11`) | **PASS** | `JsonLedgerTable.tsx:2-10,28-32,139-146` uses `tableFeatures({ rowSortingFeature })`, `createSortedRowModel(sortFn_alphanumeric)`, and `useTable` (zero v8 imports). |
| Non-blank LoC budget (`<= 450` lines on new/rewritten files) | **PASS** | Largest Console/Runbook files are `TerminalSessionPane.tsx` (`456` total / `~408` non-blank), `Console.tsx` (`434` total / `~382` non-blank), and `ScriptoriumPromptEditor.tsx` (`409` total / `~376` non-blank). `Omnibar.tsx` (`817` total) received only a minimal +16-line delegation seam to `OmnibarRunbookSheet.tsx` (`212` lines). |

---

## Detailed Findings

### Pillar 1: Copywriting (2/4)

- **`[WARNING]` Generic single-word primary submit CTA lacks Verb+Noun intent awareness and `aria-label`** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:397-405`):
  - Renders `<span>Run</span>` unconditionally without `aria-label`. `444-UI-SPEC.md:133` requires `Run Command` (`aria-label="Run command"`) for shell/command intents and `Ask Agent` (`aria-label="Ask agent"`) when `intent.kind === 'agent'`.
- **`[WARNING]` Non-7-bit-ASCII em-dashes (`\u2014`) and generic `Clear` label in multi-line paste banner and session chrome** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:292,298`, `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx:150`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:373,411`, `crates/vox-gui/ui/src/components/layout/Omnibar.tsx:802`):
  - `444-UI-SPEC.md:127` mandates pure 7-bit ASCII punctuation (`-`, `/`, `|`) in new Phase 444 UI chrome. `ScriptoriumPromptEditor.tsx:292` renders ``Pasted ${pastedLineCount} lines — press Enter to run or Esc to clear`` with button `Clear` (`L298`) instead of `444-UI-SPEC.md:139` `Pasted {N} lines into editor - review script and press Enter to execute.` and `Clear Prompt` (`aria-label="Clear prompt"`). `ConsoleSessionBar.tsx:150` and `TerminalSessionPane.tsx:373,411` also embed `\u2014` in `aria-label` / status copy, and `ConsoleSessionBar.tsx:149-151` disables re-injection (`disabled={isVoxified}`) instead of `444-UI-SPEC.md:135` `aria-label="Subshell voxified - click to re-inject hooks"`.
- **`[WARNING]` `HistorySearchPopover` action labels, confirmation prompt, empty states, and shortcut legend diverge from copywriting contract** (`crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:216,225,259,273-278,328,337`):
  - Row secondary button (`L328`) uses single-word `Insert` instead of `Insert Command` (`444-UI-SPEC.md:136`).
  - Inline destructive confirmation (`L207-227`) omits the confirmation prompt `Remove all scrubbed workspace command history entries?` (`444-UI-SPEC.md:150`; note `Confirm Clear` / `Keep` labels match `444-02-PLAN.md:197` while `444-UI-SPEC.md:150` wrote `Confirm Clear History` / `Keep History`).
  - Empty states (`L273-278`) use `'No recorded commands yet'` / `'No matching commands'` instead of `'No command history recorded yet'` / `'0 matching commands for "{query}"'` (`444-UI-SPEC.md:142-143`), search placeholder (`L259`) uses `'Search command history...'` instead of `'Filter command history (workspace boosted)...'` (`444-UI-SPEC.md:208`), and footer (`L337`) renders plain text `'Up/Down Navigate - Enter Run - Tab Insert - Esc Cancel'` without `<KeycapIcon>` (`444-UI-SPEC.md:44,213`).
- **`[WARNING]` `JsonLedgerTable` and `RunbookPanel` empty/count copy divergences & missing `[]` collection state** (`crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:55,199,237-238`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:129,136`):
  - `tryParseStructuredJson` (`JsonLedgerTable.tsx:55`) returns `null` when `parsed.length === 0`, hiding the `Table` tab for `[]` outputs instead of rendering `444-UI-SPEC.md:144` `'Empty JSON collection'` / `'Command returned 0 records ([]).'`.
  - `JsonLedgerTable.tsx:199` shows `${filteredRows.length} rows` instead of `{filteredRows.length} of {totalRows} rows` (`444-UI-SPEC.md:237`), and `L237` shows `'No matching rows'` instead of `'0 rows match "{filter}"'` (`444-UI-SPEC.md:144`).
  - `RunbookPanel.tsx:129,136` uses `'No workspace runbooks found'` (vs `'No workspace runbooks discovered'`) and omits `'Clear Filter or press Esc to show all workspace runbooks.'` (`444-UI-SPEC.md:144-145`).

### Pillar 2: Visuals (3/4)

- **`[BLOCKER]` (Cross-pillar with Color & Typography) `main textarea` `!important` opaque fill occludes `<pre>` syntax & ghost mirror in live renders** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:344-382` vs `crates/vox-gui/ui/src/index.css:756-762,1209-1215`):
  - Because `<Console>` renders inside `<main>`, `src/index.css:756-762` (`html[data-theme="arcane"] main textarea`, `html[data-theme="travertine"] main textarea`) and `src/index.css:1209-1215` (`html.dark main textarea`, `html[data-theme="slate"] main textarea`, `html[data-theme="high-contrast"] main textarea`) apply `@layer base` `!important` opaque backgrounds (`#FAF5E8 !important` / `var(--color-bg-base) !important`), opaque text color, and `1.5px solid ... !important` borders. In CSS Cascade Level 5, `@layer base` `!important` beats unlayered normal utilities (`bg-transparent`, `text-transparent`, `border-none`) and normal inline `style` props, completely hiding `<pre data-testid="scriptorium-highlight-layer">` (`L344`) and `<span data-testid="ghost">` (`L352`) in browser renders (missed by unit tests because Vitest/jsdom does not load `index.css` cascade).
- **`[WARNING]` `HistorySearchPopover` omits matched-character accent spans, `[REDACTED]` warning badges, and relative timestamps** (`crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:304-319`):
  - `{entry.command}` (`L305`) renders as a flat string instead of wrapping fuzzy-matched query indices in `text-accent font-semibold` spans and rendering `[REDACTED]` substrings as `border-(--color-status-warn)` inline chips (`444-UI-SPEC.md:105,113,211`). Also omits relative timestamp chip (`2m ago`) alongside `cwd` and `exit` chips (`444-UI-SPEC.md:211`).
- **`[WARNING]` Completion source badge taxonomy drift and missing `runbook` completions in `IntelliSensePopover`** (`crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts:13,37-48,64-75`):
  - Emits uppercase `BIN | DIR | FILE | GIT | FLAG | SUB | MACRO | SLASH | HIST` instead of `444-UI-SPEC.md:39,221-222` 9 lowercase badges (`bin | path | git | flag | vox | nu | slash | term | runbook`) and does not include workspace runbooks in completion candidates (note: `444-02-PLAN.md:146` specified the `BIN..HIST` set).
- **`[WARNING]` `DiscoveryRail` icon-only collapse/expand controls omit `<Tooltip>` wrapper and `size-4` icon sizing** (`crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx:86-95,119-126`):
  - Relies on native `title` attribute and `size-3` (`12px`) icons instead of wrapping icon-only rail toggles in `<Tooltip>` with `size-4` (`16px`) icons (`444-UI-SPEC.md:45,242`).
- **`[WARNING]` `JsonLedgerTable` imports sort icons directly from `lucide-react` and omits 12-column cap** (`crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:11,19,104-107,224-226`):
  - Imports `ArrowUp`, `ArrowDown`, `ArrowUpDown` from `'lucide-react'` instead of `Icon.arrowUp` / `Icon.arrowDown` from `../../ui/Icons` (`444-UI-SPEC.md:24,43,238`), caps rendered rows at `500` (`L19`) instead of `200` (`444-UI-SPEC.md:238`), and does not cap object-array union columns at `12` (`444-UI-SPEC.md:228,309`).
- **`[WARNING]` `OmnibarRunbookSheet` row icon uses `Icon.terminal` instead of `Icon.book`** (`crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:108`):
  - `444-UI-SPEC.md:43,248` specifies `Icon.book` (`size-4 text-accent`) for runbook list entries.

### Pillar 3: Color (2/4)

- **`[BLOCKER]` Nonexistent `--color-status-ok` and `--color-status-error` CSS custom properties** (`crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts:22`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:214,315`):
  - Style Dictionary (`src/styles/tokens.generated.css:48-51`, `tokens.slate.generated.css:48-51`, `tokens.contrast.generated.css:48-51`, `tokens.travertine.generated.css:48-51`) defines `--color-status-pass`, `--color-status-warn`, `--color-status-fail`, and `--color-status-info`. `--color-status-ok` and `--color-status-error` do not exist anywhere in the workspace:
    - `promptHighlighter.ts:22`: `string: 'text-(--color-status-ok)'` resolves to `inherit`, leaving single/double-quoted strings unhighlighted (`444-UI-SPEC.md:117` requires green / `--color-status-pass`).
    - `HistorySearchPopover.tsx:214`: `Confirm Clear` uses `text-(--color-status-error)` instead of `text-(--color-status-fail)` (`444-UI-SPEC.md:101,150`), losing its destructive oxblood/crimson treatment.
    - `HistorySearchPopover.tsx:315`: `exit {entry.exit_code}` chip uses `'text-(--color-status-ok)' : 'text-(--color-status-error)'`, rendering `exit 0` and `exit 1` identically.
- **`[WARNING]` `.text-brass` clobbered to `#1C140C !important` in light themes and `resolveAnsi16Palette(theme)` unused in `promptHighlighter.ts`** (`crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts:18-29`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:40,42,337,402`, `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx:82,124,147`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:203`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:154`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:112`):
  - `src/index.css:873-875` declares `html[data-theme="arcane"] .text-brass, html[data-theme="travertine"] .text-brass { color: #1C140C !important; }`. Using `.text-brass` instead of `text-accent` (`var(--color-accent-default)`) strips accent contrast from command/macro/slash tokens, `.vox` badges, active IntelliSense filter pills, and the `Run` CTA in both light themes. In addition, `promptHighlighter.ts` never reads `resolveAnsi16Palette(theme)` (`444-UI-SPEC.md:109,114-121`), collapsing distinct ANSI cyan/green/magenta/yellow token slots.
- **`[WARNING]` `shell` mode badge and `Voxify Subshell` CTA omit semantic status/accent tones** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:44`, `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx:153`):
  - `MODE_BADGE_TONES.shell` (`ScriptoriumPromptEditor.tsx:44`) uses neutral `'border-border-subtle bg-bg-elevated text-text-secondary'` instead of `var(--color-status-pass)` (`444-UI-SPEC.md:112`), and `ConsoleSessionBar.tsx:153` styles `Voxify Subshell` in muted neutral instead of `border-brass bg-overlay-subtle text-accent` (`444-UI-SPEC.md:108,253`).
- **`[WARNING]` Residual `bg-black/55` backdrop literal in touched `Omnibar.tsx`** (`crates/vox-gui/ui/src/components/layout/Omnibar.tsx:627`):
  - Pre-existing backdrop retains `bg-black/55` (`444-UI-SPEC.md:94` bans `bg-black/*` in touched files, though Plan `444-03` only added a 16-line delegation seam to `Omnibar.tsx`).

### Pillar 4: Typography (3/4)

- **`[BLOCKER]` (Cross-pillar) `<pre>` mirror vs `<textarea>` font-family `!important` override and `font-semibold` vs `font-normal` character-advance mismatch** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:344-350,378-381`, `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts:19,24-26`):
  - `src/index.css:535,541` forces `pre { font-family: var(--font-mono, 'Palatino', ...) !important; }` (proportional `'Palatino'` serif in `arcane`/`travertine`/`high-contrast`), overriding `<pre style={{ fontFamily: TERMINAL_MONO_FONT_STACK }}>` (`L348`) while `<textarea>` (`L378`) renders in `'JetBrains Mono'`.
  - Additionally, `PROMPT_TOKEN_CLASSES` (`promptHighlighter.ts:19,24-26`) applies `font-semibold` (`600`) to `command`, `operator`, `macro`, and `slash` spans inside `<pre>`, while `<textarea>` (`L379`) uses `font-normal` (`400`), violating `444-UI-SPEC.md:85` (`fontWeight: 400` on both layers so caret position never drifts on variable/fallback fonts).
- **`[WARNING]` Tailwind `font-mono` class used without `TERMINAL_MONO_FONT_STACK` inline override on badges and `IntelliSensePopover` items** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:320,324`, `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx:80,122,130,147,150`):
  - In `src/index.css:25,321-338,541`, `.font-mono` resolves to `'Palatino'` / `'Cinzel'` serif in `arcane`, `travertine`, and `high-contrast` rather than `TERMINAL_MONO_FONT_STACK` (`'JetBrains Mono'`, `444-UI-SPEC.md:25,82`).
- **`[WARNING]` Legacy off-scale font sizes (`text-[9px]`, `text-[10px]`, `text-[13px]`) remain in `Omnibar.tsx`** (`crates/vox-gui/ui/src/components/layout/Omnibar.tsx:163,674,713,717,723,728,738`):
  - All 16 newly created/rewritten Console, `IntelliSensePopover`, and `OmnibarRunbookSheet` files strictly follow the `(11px, 12px, 14px, 18px)` / `(400, 600)` contract; only pre-existing markup in `Omnibar.tsx` retains `9px`/`10px`/`13px`.

### Pillar 5: Spacing (3/4)

- **`[WARNING]` `<pre>` (`overflow-hidden`) vs `<textarea>` (`overflow-y-auto`) scrollbar width mismatch and `px-3 py-1` / `28px` height floor drift** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:68,104,349,379`):
  - When multi-line input exceeds `MAX_HEIGHT_PX = 160` (`L69`), `<textarea>` (`L379`, `overflow-y-auto`) reserves an `8px` scrollbar (`src/index.css:564`), narrowing its line-wrap box by `8px` relative to `<pre>` (`L349`, `overflow-hidden`) and misaligning wrapped lines. Both layers also use `px-3 py-1` and `MIN_HEIGHT_PX = 28` instead of `p-0` (`padding: '0px'`) and `24px` (`h-6`) floor (`444-UI-SPEC.md:59,85,186-187`).
- **`[WARNING]` Minor `12px` (`--space-3`) usage outside the two allowed exception sites and residual `.5` classes in `Omnibar.tsx`** (`crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:164`, `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx:84,182`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:403,410,420`, `crates/vox-gui/ui/src/components/layout/Omnibar.tsx:163,655,709-712,717,723,728,740-741,782`):
  - Zero `.5` fractional classes exist anywhere in `Console/*`, `IntelliSensePopover.tsx`, or `OmnibarRunbookSheet.tsx`. Minor `12px` (`gap-3 py-3` at `OmnibarRunbookSheet.tsx:164`, `px-3` at `ConsoleSessionBar.tsx:84,182` & `TerminalSessionPane.tsx:403,410,420`) appears outside the two `444-UI-SPEC.md:65` `--space-3` exceptions, and legacy `Omnibar.tsx` lines retain `px-1.5`, `py-0.5`, `gap-2.5`, `size-3.5`, `gap-1.5`, `gap-0.5`, `size-2.5`, `py-px`.

### Pillar 6: Experience Design (2/4)

- **`[BLOCKER]` `HistorySearchPopover` `Enter` and primary row click fail to execute the selected command** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:270-273` vs `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:186-191,301,325,337`):
  - `HistorySearchPopover` differentiates `onSelect(cmd, 'accept')` (`Enter` at `L187`, row button click at `L301`) from `onSelect(cmd, 'insert')` (`Tab` at `L190`, `Insert` button at `L325`), and its footer (`L337`) instructs the user `Enter Run - Tab Insert` (`444-UI-SPEC.md:200,211,213`). `ScriptoriumPromptEditor.tsx:270` ignores the second argument (`onSelect={(cmd) => { setHistoryPopoverOpen(false); setValue(cmd); ... }}`), making `Enter` identical to `Tab` (insert-only) instead of executing the command (`ScriptoriumPromptEditor.test.tsx:153-155` codified this wrong assertion).
- **`[WARNING]` `Ctrl+R` does not seed `HistorySearchPopover` with the current single-line prompt buffer** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:267-270,391`):
  - `HistorySearchPopover` implements `initialQuery?: string` (`HistorySearchPopover.tsx:124,130,141`), and `444-UI-SPEC.md:205` requires `Ctrl+R` to seed the filter input with the current single-line draft, yet `ScriptoriumPromptEditor.tsx:267-270` omits `initialQuery`.
- **`[WARNING]` Inline `ArrowUp` history recall ignores typed prefix filtering** (`crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:218-226`):
  - `444-UI-SPEC.md:215` specifies `ArrowUp recalls previous prefix-matched redacted history entry`, whereas `ScriptoriumPromptEditor.tsx:219-224` steps unconditionally through `getRecallHistoryList()`, overwriting whatever prefix the operator typed on line 1.
- **`[WARNING]` Missing IPC error states, offline daemon notices, and required `{{param}}` validation** (`crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:60-62,162-194`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx:38-39,173-209`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:155`, `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx:64-164`):
  - `RunbookPanel.tsx:60-62` catches `termRunbooksList` rejections and sets `runbooks = []`, displaying `'No workspace runbooks found'` instead of `444-UI-SPEC.md:148` `'Runbook scan failed: {sanitizedError}. Click Reload Runbooks to retry.'`.
  - Neither `RunbookPanel.tsx` nor `OmnibarRunbookSheet.tsx` blocks `Run Runbook` when a required `{{param}}` (with no `defaultValue`) is empty (`444-UI-SPEC.md:101,148,308`: `--color-status-fail` ring + `'Fill required parameter {{param}} before running.'`), allowing malformed shell commands with blank arguments to execute.
  - `HistorySearchPopover.tsx:155` and `IntelliSensePopover.tsx` omit the inline offline daemon fallback status notices specified in `444-UI-SPEC.md:146-147`.
- **`[WARNING]` Unconditional `Voxify Subshell` button in `ConsoleSessionBar`, unused `sessionId` prop on `ScriptoriumPromptEditor`, and `data-testid` drift** (`crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx:146-158`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx:28,58-61,347,362`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:292`):
  - `ConsoleSessionBar.tsx:146` renders `Voxify Subshell` on every session tab bar even when `subshellDetected` is false (`444-UI-SPEC.md:108,252` gates it on `subshellDetected === true`, which `TerminalSessionPane.tsx:347` properly does for `term-subshell-voxify-banner`).
  - `ScriptoriumPromptEditorProps` declares `sessionId?: string | null` (`L28`) and `TerminalSessionPane.tsx:444` passes `sessionId={sessionId}`, but `ScriptoriumPromptEditor` (`L58-61`) never destructures or forwards `sessionId` to `resolveTerminalCompletions` (`444-UI-SPEC.md:219`).
  - Minor `data-testid` divergence from `444-UI-SPEC.md:179,186,210`: `term-prompt-textarea` vs `scriptorium-prompt-textarea` (`ScriptoriumPromptEditor.tsx:362`), `scriptorium-highlight-layer` vs `prompt-syntax-mirror` (`L347`), and `term-history-item-${idx}` vs `term-history-option` (`HistorySearchPopover.tsx:292`).

---

## Files Audited

1. `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts`
2. `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts`
3. `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts`
4. `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx`
5. `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`
6. `crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx`
7. `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx`
8. `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`
9. `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx`
10. `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx`
11. `crates/vox-gui/ui/src/components/surfaces/Console/useConsoleWarpActions.ts`
12. `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`
13. `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`
14. `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`
15. `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx`
16. `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`
17. `crates/vox-gui/ui/src/components/layout/Omnibar.tsx`

*(Supporting contracts cross-checked read-only: `crates/vox-gui/ui/src/index.css`, `crates/vox-gui/ui/src/styles/tokens*.generated.css`, `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts`, `crates/vox-gui/ui/src/lib/themeContract.ts`, `crates/vox-gui/ui/src/guards/{zeroEmojiAndDingbat,noRawNativeControlsGuard,toastBodyGuard}.test.ts`.)*

---

## Registry Audit

- **Not applicable:** `444-UI-SPEC.md` specifies `shadcn_initialized: false` with zero third-party registries (`Registry Safety` section, lines 316-323). All components use first-party Roman Scriptorium primitives and `@tanstack/react-table` `9.0.0-alpha.11`.
