# Plan 51-03 Summary: Terminal InputEditor/DiscoveryRail & Chat Loquela Unified Ghost Text + Keyboard-Navigable Popover Wiring

## Execution Summary
- **Phase**: 51-unified-intellisense-dynamic-completions
- **Plan**: 03
- **Status**: Complete & Verified

## Accomplishments
1. **Terminal Surface (`crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx` & `DiscoveryRail.tsx`)**:
   - Integrated `computeIntelliSenseItems` + `discoverySuggest` across all input shapes (`vox ...`, `/slash`, `!nu`, `?prompt`, `@term`, `@file`, and pipeline stages `| ...`).
   - Connected `<IntelliSensePopover />` and inline ghost text (`data-testid="ghost"`).
   - Enabled full keyboard navigation:
     - `ArrowDown` / `ArrowUp` to cycle active suggestion and notify `DiscoveryRail` (`onActiveSuggestion`),
     - `Tab` / `ArrowRight` to accept ghost or selected popover suggestion,
     - `Ctrl+Space` to trigger dynamic suggestions even on empty input,
     - `Escape` to close the popover without losing the current input buffer.
   - Tested in `InputEditor.test.tsx` (all 5 tests passed; 41/41 Console suite tests passed).

2. **Chat Surface (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`)**:
   - Replaced fragmented popup menus with the unified `IntelliSensePopover` and inline ghost text hint (`data-testid="loquela-ghost"`).
   - Unified `/` slash commands, installed skills, and all 6 `@` categories:
     - `@term` live terminal session blocks from `terminalBridge` (with exit status badge and error recovery boosts),
     - `@file` repository files via `list_repo_files`,
     - `@skill` installed capabilities,
     - `@mcp` canonical tools from `CAPABILITY_INDEX`,
     - `@symbol` Vox decorators and language keywords,
     - `@agent` orchestrator agent codenames.
   - Verified keyboard cycling (`ArrowDown`, `ArrowUp`), `Tab` acceptance, and chip badges.
   - Tested in `Loquela.test.tsx` (all 44 tests passed; 72/72 Loquela suite tests passed).

## Verification Evidence
```bash
timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/
# Test Files  9 passed (9)
# Tests       41 passed (41)

timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela/
# Test Files  5 passed (5)
# Tests       72 passed (72)

timeout 300s pnpm --dir crates/vox-gui/ui exec tsc --noEmit
# Exit code 0, no errors
```
