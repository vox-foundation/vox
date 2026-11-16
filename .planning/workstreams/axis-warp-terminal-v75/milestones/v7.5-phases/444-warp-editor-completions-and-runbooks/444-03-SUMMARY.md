---
phase: 444-warp-editor-completions-and-runbooks
plan: 03
subsystem: ui
tags: [react, typescript, tanstack-table-v9, terminal, runbooks, omnibar, console, json-ledger]

requires:
  - phase: 444-01
    provides: "Typed termRunbooksList / TermRunbookDto IPC wrappers in transport.ts and requestOpenRunbook / requestTerminalCommand pub/sub in terminalBridge.ts"
provides:
  - "JsonLedgerTable.tsx built on @tanstack/react-table@9.3.1 v9 APIs (tableFeatures + rowSortingFeature + createSortedRowModel + sortFn_alphanumeric + useTable) with type=search filtering, 1 MiB / 500-row safety caps, nested cell expansion, and RFC-4180 CSV / JSON copy"
  - "BlockCard.tsx structured JSON / Table segmented view toggle (data-testid=term-ledger-view-toggle) defaulting to 'json' so existing BlockStreamView tests and textbox queries stay green"
  - "RunbookPanel.tsx with workspace .vox/Markdown runbook discovery, type=search filtering, VoxSelect / input parameter forms, and shell-quoted {{param}} command interpolation"
  - "DiscoveryRail.tsx tabbed Action Help | Runbooks rail defaulting to 'help', replacing guillemet glyphs with Icon.chevR / Icon.chevL SVG icons and normalizing 4px spacing & semantic tokens"
  - "OmnibarRunbookSheet.tsx + Omnibar.tsx (<18 added lines) enabling Cmd+P 0-param immediate runbook launch and parameterized inline omnibar-runbook-params sheet"
affects:
  - 444-04-PLAN
  - 445-terminal-agentic-mode-and-mens-flywheel
  - 446-roman-scriptorium-theme-and-cv-flywheel

actuals:
  tokens: 21457
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "TanStack Table v9 (@tanstack/react-table@9.3.1) composable feature plugin pattern (tableFeatures + rowSortingFeature + createSortedRowModel + sortFn_alphanumeric + useTable) matching DataTable.tsx"
    - "Shell-safe {{param}} interpolation (quoteShellParam wrapping non-bare tokens in single quotes with '\\'' internal quote escaping) across RunbookPanel and OmnibarRunbookSheet"
    - "Partial Vitest ESM mock guard (try/catch typeof fn === 'function' over transport.termRunbooksList and voxTransport.termRunbooksList) so partial vi.mock('transport') suites never throw"

key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.test.tsx
    - crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx
    - crates/vox-gui/ui/src/components/layout/Omnibar.tsx
    - crates/vox-gui/ui/src/components/layout/Omnibar.test.tsx

key-decisions:
  - "Defaulted BlockCard structured output view to 'json' with an explicit JSON / Table segmented switch (data-testid='term-ledger-view-toggle') and type='search' on term-ledger-filter so all pre-existing BlockStreamView assertions and Console getByRole('textbox') queries remain undisturbed."
  - "Defaulted DiscoveryRail active tab to 'help' (Action Help), moved useLabel('con-discovery') above the collapsed early return to guarantee stable React hook ordering across collapse/expand transitions, and replaced '»'/'«' guillemets with Icon.chevR / Icon.chevL."
  - "Wrapped transport.termRunbooksList / voxTransport.termRunbooksList resolution in a defensive try/catch helper in both RunbookPanel and OmnibarRunbookSheet so existing DiscoveryRail.test.tsx and Omnibar.test.tsx partial vi.mock('../../transport') stubs never throw missing-export errors."
  - "Extracted Cmd+P runbook state, filtering, Runbooks section, and inline parameter launch sheet into OmnibarRunbookSheet.tsx (192 non-blank LoC), keeping Omnibar.tsx delta to 16 added lines."

patterns-established:
  - "Structured block ledger inspection: JSON raw view by default with opt-in TanStack Table v9 Ledger Table (1 MiB byte ceiling + 500-row UI cap + RFC-4180 CSV export)."
  - "Shell-escaped runbook parameter substitution: bare [A-Za-z0-9_./:@=-]+ values pass cleanly while whitespace/metacharacters/quotes are single-quote escaped via quoteShellParam."

requirements-completed:
  - TERM-WARP-04

coverage:
  - id: D1
    description: "JsonLedgerTable (@tanstack/react-table@9.3.1 v9 sorting, type=search filtering, 1 MiB / 500-row caps, nested JSON expansion, RFC-4180 CSV & JSON copy) and BlockCard JSON / Table view switch defaulting to 'json'"
    requirement: TERM-WARP-04
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx"
        status: pass
    human_judgment: false
  - id: D2
    description: "RunbookPanel (workspace .vox/Markdown runbooks, type=search filter, VoxSelect + text params, shell-quoted {{param}} substitution) and DiscoveryRail Action Help | Runbooks tabs with Icon.chevR / Icon.chevL"
    requirement: TERM-WARP-04
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/guards/noRawNativeControlsGuard.test.ts"
        status: pass
    human_judgment: false
  - id: D3
    description: "OmnibarRunbookSheet + Omnibar Cmd+P Runbooks section with immediate 0-param dispatch and inline omnibar-runbook-params sheet for parameterized runbooks"
    requirement: TERM-WARP-04
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/layout/Omnibar.test.tsx"
        status: pass
    human_judgment: false

duration: 28min
completed: 2026-11-15
status: complete
---

# Phase 444 Plan 03: Structured JSON Ledger Table, Runbook Panel & Omnibar Runbook Launch Summary

**TanStack Table v9 (`@tanstack/react-table@9.3.1`) `JsonLedgerTable` + `BlockCard` `JSON`/`Table` switch, shell-quoted parameterized `RunbookPanel` + `DiscoveryRail` `Action Help | Runbooks` tabs, and `Cmd+P` `OmnibarRunbookSheet` launch flow (`TERM-WARP-04`).**

## Performance

- **Duration:** 28 min
- **Started:** 2026-11-15T15:00:05Z
- **Completed:** 2026-11-15T15:28:30Z
- **Tasks:** 3
- **Files modified:** 11 (5 created, 6 modified)

## Accomplishments

- **Task 1 (`JsonLedgerTable` + `BlockCard`):**
  - Built `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx` (`269` non-blank LoC) using `@tanstack/react-table@9.3.1` v9 APIs (`tableFeatures({ rowSortingFeature, sortedRowModel: createSortedRowModel(), sortFns: { alphanumeric: sortFn_alphanumeric } })` + `useTable`), handling arrays of objects, single plain objects (`field` / `value` 2-column ledger), and scalar arrays (`index` / `value` ledger), enforcing a `1 MiB` payload guard and `500`-row display cap, `type="search"` filtering (`data-testid="term-ledger-filter"`), inline nested JSON cell expansion, and `Copy JSON` + RFC-4180 `Copy CSV` actions.
  - Updated `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` (`297` non-blank LoC) with a `JSON` / `Table` segmented switch (`data-testid="term-ledger-view-toggle"`) defaulting to `'json'` so pre-existing `BlockStreamView` JSON `<pre>` tests and `getByRole('textbox')` queries remain unaffected until switched to `Table`, plus normalized `size-3.5` to `size-4`.
- **Task 2 (`RunbookPanel` + `DiscoveryRail`):**
  - Built `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx` (`189` non-blank LoC) loading workspace `.vox` and Markdown runbooks via guarded `termRunbooksList`, rendering `type="search"` filtering (`data-testid="term-runbook-filter"`), `No workspace runbooks found` empty state, `<VoxSelect>` choice / `<input>` text `{{param}}` editors, and shell-quoted `quoteShellParam` + `interpolateRunbookCommand` preview with `Insert` and `Run` actions.
  - Updated `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` (`171` non-blank LoC) with `Action Help` (default `'help'`) and `Runbooks` (`'runbooks'`) tabs, replaced `»`/`«` guillemet characters with `<Icon.chevR>` / `<Icon.chevL>`, normalized fractional spacing to the 4px grid and semantic `vox-*` tokens, and hoisted `useLabel('con-discovery')` above the collapsed early return.
- **Task 3 (`OmnibarRunbookSheet` + `Omnibar`):**
  - Built `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx` (`192` non-blank LoC) providing `useOmnibarRunbooks`, `OmnibarRunbooksSection` (`data-testid="omnibar-runbook-row-{id}"`), and `OmnibarRunbookParamsSheet` (`data-testid="omnibar-runbook-params"`) with `<VoxSelect>` / `<input>` controls, shell-quoted command preview, `Insert in Console` (`mode: 'insert'`), and `Run in Console` (`mode: 'run'`).
  - Wired `crates/vox-gui/ui/src/components/layout/Omnibar.tsx` (+16 lines) to render `Runbooks` in `Cmd+P` search results, immediately dispatch 0-parameter runbooks (`navigateTo('console')` + `requestTerminalCommand(rb.command, true)` + `requestOpenRunbook`), open the inline parameter sheet for parameterized runbooks, and unwind `pendingRunbook` cleanly on `Escape`.

## Task Commits

Changes left uncommitted in working tree per orchestrator git-safety instructions (`commits: 0`):

1. **Task 1: `JsonLedgerTable` (`@tanstack/react-table` v9) and `BlockCard` `Table` / `JSON` toggle** — uncommitted (`JsonLedgerTable.tsx`, `JsonLedgerTable.test.tsx`, `BlockCard.tsx`, `BlockStreamView.test.tsx`)
2. **Task 2: `RunbookPanel` and `DiscoveryRail` (`Action Help | Runbooks` tabs + `Icon.chevR`/`Icon.chevL`)** — uncommitted (`RunbookPanel.tsx`, `RunbookPanel.test.tsx`, `DiscoveryRail.tsx`, `DiscoveryRail.test.tsx`)
3. **Task 3: `OmnibarRunbookSheet` and `Omnibar` `Cmd+P` Runbook launch integration** — uncommitted (`OmnibarRunbookSheet.tsx`, `Omnibar.tsx`, `Omnibar.test.tsx`)

## Files Created/Modified

- `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx` — TanStack Table v9 sortable/filterable structured JSON ledger grid with 1 MiB / 500-row caps, nested JSON expansion, and CSV/JSON copy (`269` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.test.tsx` — Unit tests covering v9 sorting, `searchbox` filtering, nested expansion, CSV/JSON copy, and 1 MiB / 500-row guards (`81` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` — Structured JSON `JSON` / `Table` segmented view switch defaulting to `'json'` (`297` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx` — Extended regression tests verifying default `'json'` view, unmounted `term-ledger-filter`, and switching to `Table` (`396` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx` — Parameterized `.vox`/Markdown runbook browser, `<VoxSelect>`/`<input>` form, and shell-quoted `Insert`/`Run` actions (`189` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.test.tsx` — Unit tests for runbook filtering, `<VoxSelect>`, shell-escaped `{{param}}` interpolation (`'a b; rm -rf /'`), and empty/partial-mock states (`100` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` — Tabbed `Action Help` | `Runbooks` rail with `<Icon.chevR>`/`<Icon.chevL>` and normalized 4px tokens (`171` non-blank LoC)
- `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx` — Regression + tab switch tests verifying default `Action Help`, guillemet removal, and `Runbooks` switching (`108` non-blank LoC)
- `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx` — `Cmd+P` runbook discovery hook, `Runbooks` result list, and inline `omnibar-runbook-params` sheet (`192` non-blank LoC)
- `crates/vox-gui/ui/src/components/layout/Omnibar.tsx` — Minimal (+16 lines) `Cmd+P` runbook section and inline parameter sheet integration
- `crates/vox-gui/ui/src/components/layout/Omnibar.test.tsx` — `Cmd+P Runbooks` suite testing partial-mock resilience, 0-param immediate dispatch, and parameterized `omnibar-runbook-params` launch

## Decisions Made

- Kept `JsonLedgerTable` strictly on `@tanstack/react-table@9.3.1` v9 APIs (`tableFeatures` + `rowSortingFeature` + `createSortedRowModel` + `sortFn_alphanumeric` + `useTable`) matching `src/components/ui/DataTable.tsx`.
- Used `<input type="search">` (`role="searchbox"`) on both `term-ledger-filter` and `term-runbook-filter` and defaulted `BlockCard` to `'json'` and `DiscoveryRail` to `'help'` so existing `getByRole('textbox')` assertions in Console tests stay unambiguous.
- Quoted every interpolated `{{param}}` value via `quoteShellParam` across both `RunbookPanel` and `OmnibarRunbookSheet` to neutralize shell metacharacters, whitespace, and embedded single quotes (`'\''`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed React Rules-of-Hooks call order in `DiscoveryRail.tsx`**
- **Found during:** Task 2 (`DiscoveryRail.tsx` update)
- **Issue:** `DiscoveryRail.tsx` previously called `useLabel('con-discovery')` after `if (collapsed) { return ... }`, which violates React hook ordering whenever the rail toggles between collapsed and expanded states.
- **Fix:** Hoisted `const railLabel = useLabel('con-discovery')` above `if (collapsed)` so hook count is invariant across renders.
- **Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx`
- **Verification:** `DiscoveryRail.test.tsx` collapse/expand and tab-switching tests pass (`7/7`).

---

**Total deviations:** 1 auto-fixed (Rule 1 hook-order bug fix in touched component)
**Impact on plan:** Eliminated latent hook-order violation on rail collapse/expand; zero scope creep.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All `444-03` structured output ledger, Runbook panel, DiscoveryRail tabs, and `Cmd+P` Omnibar runbook launch deliverables are green (`58/58` component tests, `7/7` static UI guards, `tsc --noEmit` exit `0`) and ready for Wave 3 Console integration in `444-04-PLAN.md`.

---
*Phase: 444-warp-editor-completions-and-runbooks*
*Completed: 2026-11-15*
