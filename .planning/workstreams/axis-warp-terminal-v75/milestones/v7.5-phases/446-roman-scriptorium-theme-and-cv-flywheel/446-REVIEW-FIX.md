---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
status: all_fixed
findings_total: 13
findings_fixed: 13
ui_findings_fixed: 9
vitest_parity_fixes: 2
fixed_at: 2026-11-16T03:58:00Z
---

# Phase 446 Code & UI Review Remediation Summary

All Critical (`CR-01..02`), Warning (`WR-01..07`), and Info (`IN-01..04`) findings from `446-REVIEW.md`, all layout/contrast/accessibility findings from `446-UI-REVIEW.md`, and 2 repo-wide Vitest parity regressions (`Loquela.test.tsx:394`, `ResearchModeDropdown.tsx:141`) are resolved and verified.

## 1. Critical Findings Remediated (`446-REVIEW.md`)

- **`CR-01` (`auto-tui-alt-screen` blank `.xterm` root cause):**
  - `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts`: Added and exported `unicode11CharProperties(codePoint)` returning `(unicode11Wcwidth(codePoint) & 3) << 1` so `@xterm/xterm@6.0.0` `UnicodeService.extractWidth` receives valid cell widths (`1` for ASCII, `2` for CJK, `0` for combining marks) instead of `0`.
  - `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts`: Added width-bit extraction assertions `(charProperties(cp, 0) >> 1) & 3` for ASCII, CJK, and combining glyphs.
  - `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx`: Preserved 1-argument `t.write(replaySnapshot.text)` followed by `requestAnimationFrame` `t.refresh(...)`.
  - `crates/vox-gui/ui/e2e/review/terminalAudits.ts`, `terminalAudits.test.ts`, and `states.ts`: Exempted xterm's own `aria-hidden="true"` on `.xterm-rows`/`.xterm-selection` inside visible `.xterm` hosts, added blank `.xterm-rows` detection, and waited for non-empty `.xterm-rows` text in `auto-tui-alt-screen`.
- **`CR-02` (`900x600` & `1100x720` split-pane / toolbar / popover overflow & clipping):**
  - `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`: Wrapped header (`min-h-8 flex-wrap`), bounded git branch chip (`max-w-36 truncate`), wrapped right toolbar, and added `data-terminal-mono="true"` on `dock-term-input`.
  - `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`: Compacted `<1100px` header padding, enforced single-row `AgentStrip` action strip, and moved `overflow-hidden` off the outer shell onto the inner split/stream flex body (`L392`) so `HistorySearchPopover` never clips at `900x600`.
  - `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx`: Relaxed split leaf minimum height to `min-h-16 min-[1100px]:min-h-[180px]`.
  - `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`: Raised tray cap at `900x600` (`max-h-44 min-[1100px]:max-h-none`) so command preview `<pre>` and all 4 CTAs stay unclipped.
  - `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx`: Replaced `shrink-0` with `min-w-0 max-w-full` on `renderProposalCard` CTA row (`L256`) to stop `Edit in Prompt` / `Copy` overflow in `1100x720` split panes.
  - `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts`: Added `max-w-full flex-wrap break-all text-left` on `LINK_BTN_CLS` to fix `1100x720` `term-output-file-link` overflow.
  - `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx`: Added `flex-wrap` on action buttons.

## 2. Warning & Audit Integrity Remediations (`WR-01..07` + Harness Hardening)

- **`WR-01` & `WR-05` (`e2e/review/vlmVisionFlywheel.ts`):** `AxeBuilder` errors now count fail-closed as `axeViolations = 1`; `decodePngRgbaInPage` throws explicitly on a null 2D canvas context; removed the `hasTerminalWell` header bypass.
- **Audit harness precision (`e2e/review/audits.ts`, `terminalAudits.ts`, `states.ts`, `capture.spec.ts`):** Fixed `style.opacity === '' ? 1` and `overflowX: hidden|clip` ancestor intersection in `audits.ts`, folded element `opacity` into WCAG contrast math in `terminalAudits.ts`, preserved `initFlags` across `SURFACE_STATES` and `capture.spec.ts`, removed `.catch(() => {})` on `term-approval-card`, and required `'visible'` on the first runbook card.
- **`WR-03` & `WR-04` (`RunbookPanel.tsx`, `JsonLedgerTable.tsx`, `HistorySearchPopover.tsx`, `TermSearchBar.tsx`, `TerminalSessionPane.tsx`):** Added `data-slot="inline-search-input"` (`WR-03`) and shared `TERMINAL_MONO_STYLE` + `data-terminal-mono="true"` (`WR-04`) across `<pre>` blocks, search inputs, and `dock-term-input`.
- **`WR-06` (`TerminalTab.tsx`, `sgrToSpans.ts`):** Parsed `mockReplay` via `sgrToSpans(mockReplay)` in `TerminalTab.tsx`; fixed `\x1b[2m` dim foreground contrast (`text-(--terminal-well-muted)` without `opacity-75`) and explicit `text-(--terminal-well-fg)` on search `<mark>` in `sgrToSpans.ts`.
- **`WR-07` (`warp-roman-terminal-v75-research-2026.md`, `446-UI-SPEC.md`):** Synchronized `446-UI-SPEC.md` lines `152-168` and `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` lines `27`, `62`, and `113-133` with calibrated `terminalTheme.ts` hexes and exact `ledger-after.json` (`task-2378`) slice statistics (`minVisualAfterScore: 0.9284`, `minVlmOverallScore: 0.9506`).

## 3. Info & UI Review Remediations (`IN-01..04`, `446-UI-REVIEW.md`, Repo Vitest Parity)

- **`IN-01` & 4px grid:** Updated `HistorySearchPopover.tsx` (`mx-0.5` -> `mx-1`, bounded `max-h-[min(340px,calc(100vh-140px))]`, added `tabIndex={0}` on `[role="listbox"]` resolving Axe `scrollable-region-focusable`) and `TermSearchBar.tsx` (`size-3.5` -> `size-3`).
- **`IN-02` & `IN-03`:** Imported shared `TERMINAL_MONO_STYLE` in `AgentTurnContent.tsx`, `JsonLedgerTable.tsx`, and `RunbookPanel.tsx`; excluded `.xterm` and `[data-terminal-well="true"]` from `[role="tab"]`, `thead`, `section > header`, and `[role="tablist"]` in `crates/vox-gui/ui/src/index.css`.
- **`446-UI-REVIEW.md` polish:** Fixed `DiscoveryRail.tsx` collapsed `<Glass padding="p-0" ... py-1>`, chevrons (`<Icon.chevL>` collapsed / `<Icon.chevR>` expanded), and expanded width (`w-60 xl:w-64`); hid duplicate in-tab branch pill `<1100px` and added `title={`Close session ${s.id}`}` in `ConsoleSessionBar.tsx`; moved `TERMINAL_MONO_STYLE` off outer `<section>` in `BlockStreamView.tsx` so `<EmptyState>` keeps Roman typography; replaced Unicode dashes/ellipses/middle dots with pure ASCII `-` / `...` in `TermSearchBar.tsx`, `AgentStrip.tsx`, `AgentTab.tsx` (+ test), and `SendToAgent.tsx`; set WCAG 2.5.3 `aria-label` prefix in `AgentStrip.tsx` and capped `visibleAgents` to `agents.slice(0, 2)`; standardized `TerminalSessionPane.tsx` disconnect copy to `'Terminal session bridge unavailable: ...'`; replaced versioned model literals in `AgentTurnContent.test.tsx` & `SlashMetaContent.test.tsx` with `openrouter/claude-sonnet` and `ollama/qwen3-coder-14b`.
- **Repo-wide Vitest parity fixes (Step 3B.1):** Updated `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.test.tsx` line `394` from `role="button"` to `role="option"` (matching `IntelliSensePopover.tsx`) and `crates/vox-gui/ui/src/components/surfaces/Chat/ResearchModeDropdown.tsx` line `141` from `text-[10px]` to `text-[11px]`.

## 4. Verification Results

- `pnpm --dir crates/vox-gui/ui exec tsc --noEmit` -> **exit `0`** (`0` errors)
- `pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=4 e2e/review/audits.test.ts e2e/review/terminalAudits.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/lib/themeContract.test.ts src/lib/cvFlywheel.test.ts src/guards/ src/__tests__/noVersionedModelIds.test.ts src/__tests__/chatTypeScale.test.ts src/components/surfaces/Loquela/Loquela.test.tsx` -> **exit `0`** (`40` files passed, `276` tests passed)
- `VOX_CV_FLYWHEEL=1 CV_STAGE=before pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium` -> **exit `0`** (`8` passed)
- `VOX_CV_FLYWHEEL=1 CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium` -> **exit `0`** (`8` passed; `ledger-after.json`: `112/112` passed, `0` failed, `minVisualAfterScore: 0.9284`, `minVlmOverallScore: 0.9506`)
- `e2e/console-workbench.spec.ts --project=chromium` (`2/2` passed), `e2e/review/cv-flywheel.spec.ts -g "Phase 386" --project=chromium` (`1/1` passed)
- `toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src` -> **exit `0`** (`No issues found`)
- `vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md` -> **exit `0`** (`no hard errors`)
- `vox-arch-check` -> **exit `0`**
