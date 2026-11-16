---
phase: 443-unified-terminal-pane-webgl-and-split-panes
plan: 02
subsystem: ui
tags: [react, typescript, terminal, ansi-sgr, tanstack-react-virtual, vitest, xss-safety]

requires:
  - phase: 442-vt-actor-pty-event-bus-and-bridge
    provides: TermBlockDto, TermOutputChunk, and voxTransport.openLocator IPC contracts
provides:
  - Pure-TS ECMA-48 SGR 16/256/TrueColor + CR progress normalizer + URL/file:line:col React span parser (sgrToSpans.ts)
  - Extracted semantic-token Warp Command Block card with sticky header, collapse/expand, Pin/Unpin, and conditional AwaitingInput stdin + Ctrl+B controls (BlockCard.tsx)
  - "@tanstack/react-virtual virtualized command block stream (>24 blocks when clientHeight > 0) with deterministic jsdom direct-render fallback (BlockStreamView.tsx)"
affects:
  - 443-03-PLAN
  - 443-04-PLAN

actuals:
  tokens: 12250
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Zero-dangerouslySetInnerHTML ECMA-48 SGR state machine emitting typed React spans, <mark> search highlights, and scheme-allowlisted <button> locators"
    - "Custom observeElementRect reading clientHeight || offsetHeight || getBoundingClientRect().height so @tanstack/react-virtual activates when clientHeight > 0 and falls back deterministically in zero-layout jsdom"
    - "Strict conditional null unmount of AwaitingInput stdin bar so idle Console DOM exposes a single unique textbox landmark"

key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx

key-decisions:
  - "Preserved SGR brush sequences across bare carriage-return (\\r) line overwrites in normalizeTerminalText so colored cargo/pnpm spinners retain their foreground/background attributes on the latest frame."
  - "Configured useVirtualizer observeElementRect to inspect clientHeight || offsetHeight || getBoundingClientRect().height and gated measureElement on el.offsetHeight > 0 so real WebViews measure dynamic block heights while jsdom virtualization tests with mocked clientHeight=360 mount <35 cards cleanly."
  - "Kept Wave 1 imports 100% self-contained against existing src/lib/theme.ts, src/components/ui/*, and src/transport.ts with ExtendedTermBlockDto so Plan 443-02 compiles and passes independently alongside parallel Plan 443-01."

patterns-established:
  - "Header Pinned badge vs footer Pin/Unpin toggle button prevents duplicate 'Pinned' text collisions in Testing Library queries."
  - "All terminal output links strictly allowlist http:// and https:// (after localhost:<port> normalization) and reject javascript:, data:, vbscript:, and file:// prefixes."

requirements-completed:
  - TERM-PANE-03

coverage:
  - id: D1
    description: "Pure-TS sgrToSpans.ts parses 16-color, 256-color, and 24-bit TrueColor ECMA-48 SGR sequences, text attributes, CR progress overwrites, search highlights, and safe http(s)/localhost + file:line:col links with zero HTML injection"
    requirement: TERM-PANE-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx"
        status: pass
    human_judgment: false
  - id: D2
    description: "Extracted BlockCard.tsx renders sticky headers with cwd basename, git_branch, Pinned badge vs Pin/Unpin toggle, collapse/expand, and conditional AwaitingInput stdin (password-safe on credential_prompt, null when inactive) + Ctrl+B Promote to Background"
    requirement: TERM-PANE-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx"
        status: pass
    human_judgment: false
  - id: D3
    description: "BlockStreamView.tsx virtualizes >24 command blocks via @tanstack/react-virtual when clientHeight > 0 and falls back deterministically to direct rendering in zero-height jsdom"
    requirement: TERM-PANE-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx#falls back to direct rendering in zero-height jsdom and virtualizes >24 blocks when clientHeight > 0"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-11-15
status: complete
---

# Phase 443 Plan 02: Command Block SGR Renderer, BlockCard & Virtualized Stream Summary

**Pure-TS ECMA-48 SGR + clickable locator renderer (`sgrToSpans.ts`), semantic-token Warp `BlockCard.tsx` with conditional `AwaitingInput` stdin/background controls, and `@tanstack/react-virtual`-virtualized `BlockStreamView.tsx` (`TERM-PANE-03`).**

## Performance

- **Duration:** 18 min
- **Started:** 2026-11-15T11:07:35Z
- **Completed:** 2026-11-15T11:25:35Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Built `sgrToSpans.ts` (`235` non-blank LoC) supporting WCAG >= 4.5:1 light/dark 16-color palettes, xterm 256-color cube/grayscale (`38;5;n` & `38:5:n`), 24-bit TrueColor (`38;2;r;g;b` & colon subparameters), bold/dim/italic/underline/inverse/strikethrough + resets, OSC/CSI cleanup, `\r` spinner frame overwrite normalization, `<mark data-testid="term-search-highlight">` search highlighting, and scheme-allowlisted `http(s)` / `localhost:<port>` + `file:line(:col)?` buttons wired to `voxTransport.openLocator` with zero `dangerouslySetInnerHTML`.
- Extracted `BlockCard.tsx` (`231` non-blank LoC) replacing hardcoded dark `zinc-*` / `bg-black/*` classes with semantic tokens (`bg-bg-surface`, `bg-bg-base`, `bg-bg-elevated`, `border-border-subtle`, `border-brass`, `--color-status-*`), `4px` spacing rhythm, `11px`/`12px`/`14px` typography, sticky command headers (`cwd` basename, `git_branch`, `Pinned` badge, `structured json`, exit seal, `Interrupt Process`, duration), output collapse/expand (auto-expanding on `Failed`), `Pin`/`Unpin` footer toggle, ANSI-stripped `Focus in Chat` line-range selection, and conditional `AwaitingInput` stdin bar (`null` when inactive, `type="password"` + immediate state wipe on `credential_prompt`, `Ctrl+B` `Promote to Background`).
- Upgraded `BlockStreamView.tsx` (`173` non-blank LoC) to fill its parent pane (`flex min-h-0 flex-1`), render the `443-UI-SPEC.md` empty state (`Terminal session ready`), scroll `activeSearchBlockId`/`focusedBlockId` into view, and virtualize `>24` blocks via `@tanstack/react-virtual` when `clientHeight > 0` with deterministic direct-render fallback in zero-height jsdom.

## Task Commits

Wave 1 ran in shared-worktree parallel mode under non-git-mutation constraints; commits are staged and recorded by the phase orchestrator:

1. **Task 1: Build pure-TS sgrToSpans.ts ECMA-48 SGR + CR normalizer + clickable file:line:col & URL parser** - verified via TDD RED -> GREEN -> REFACTOR (`sgrToSpans.test.tsx`, `themeContract.test.ts`, `zeroEmojiAndDingbat.test.ts`)
2. **Task 2: Extract BlockCard.tsx and upgrade BlockStreamView.tsx with @tanstack/react-virtual and AwaitingInput controls** - verified via TDD RED -> GREEN -> REFACTOR (`BlockStreamView.test.tsx`, `src/guards`, `pnpm run typecheck`)

## Files Created/Modified

- `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` - Pure-TS ANSI SGR, `\r` progress overwrite normalizer, search highlighter, and safe `http(s)` / `file:line:col` React element parser (`235` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx` - Unit & RTL tests covering 16/256/TrueColor SGR, attributes/inverse/resets, CR/CSI normalization, locator clicks, search marks, and XSS/scheme rejection (`104` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` - Extracted Warp Command Block card component with semantic CSS variables, sticky header metadata, collapse/expand, `Pin`/`Unpin`, and `AwaitingInput` stdin + `Ctrl+B` bar (`231` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` - `@tanstack/react-virtual` virtualized command block stream with zero-height jsdom fallback and `ExtendedTermBlockDto` re-exports (`173` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx` - Expanded RTL suite (7 tests) verifying legacy landmarks, `cwd`/`git_branch`, `Pinned` vs `Unpin`, collapse/expand + `Failed` auto-expand, `AwaitingInput` `credential_prompt` + `Ctrl+B`, and 100-block virtualization (`308` non-blank LoC).

## Decisions Made

- Used `observeElementRect` fallback chain (`el.clientHeight || el.offsetHeight || rect.height`) inside `BlockStreamView.tsx` because `@tanstack/virtual-core`'s default observer reads `offsetHeight` (which stays `0` in jsdom when only `clientHeight` and `getBoundingClientRect` are mocked), ensuring both real browser `ResizeObserver` layouts and jsdom `clientHeight` mocks activate virtualization identically.
- Guarded `virtualizer.measureElement(el)` behind `el.offsetHeight > 0` so zero-layout jsdom rows retain `estimateSize: () => 136` instead of collapsing to `0px`.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `BlockStreamView`, `BlockCard`, `sgrToSpans`, and `stripAnsi` are ready for Wave 2 integration into `TerminalSessionPane.tsx` (`443-03`) and `ChatTerminalBlockCard.tsx` (`443-04`).
- All 12 Vitest suites (`sgrToSpans.test.tsx`, `BlockStreamView.test.tsx`, and all 10 `src/guards` suites = 50/50 tests), `themeContract.test.ts` (7/7), `Console.test.tsx` (7/7), and `pnpm --dir crates/vox-gui/ui run typecheck` (`tsc --noEmit`) are 100% green.

---
*Phase: 443-unified-terminal-pane-webgl-and-split-panes*
*Completed: 2026-11-15*
