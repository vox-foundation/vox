---
phase: 49-llm-first-structured-shell-nushell-vox-shell-skills-mcp-exec
plan: "04"
subsystem: vox-gui
tags: [console, nushell, structured-json, react, vitest]
provides:
  - Shell mode selector (default | nu | vox) in InputEditor.tsx and Console.tsx
  - Structured JSON badge (term-block-structured-badge) and formatted JSON display in BlockStreamView.tsx
key-files:
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
completed: 2026-10-15
---

# Phase 49 Plan 04: GUI Console Structured JSON Badge & Shell Mode Selector Summary

## Accomplishments
- Added `term-shell-mode-select` (`default | nu | vox`) to [`InputEditor.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/InputEditor.tsx) with automatic `!nu ` prefixing in `nu` mode.
- Added structured JSON detection and `term-block-structured-badge` (`structured json`) rendering in [`BlockStreamView.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx).

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/BlockStreamView.test.tsx src/components/surfaces/Console/InputEditor.test.tsx` -> `7 passed; 0 failed`.
