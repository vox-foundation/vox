---
phase: 50-bidirectional-chat-terminal-symbiosis-security-levels-l1-l5
plan: "03"
subsystem: vox-gui
tags: [loquela, terminal-bridge, attention-chips, chat-symbiosis]
provides:
  - @term, @term:last, @term:failed, @term:pinned mention autocomplete in Loquela.tsx
  - Ambient terminal context pill (loquela-ambient-term-pill) in Loquela.tsx
key-files:
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.test.tsx
completed: 2026-10-15
---

# Phase 50 Plan 03: `@term` Attention Chips & Ambient Terminal Context Pill Summary

## Accomplishments
- Extended `Loquela` `@` autocomplete popover in [`Loquela.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx) with `@term`, `@term:last`, `@term:failed`, and `@term:pinned` options (`data-testid="loquela-at-term-option"`).
- Rendered `data-testid="loquela-ambient-term-pill"` in `Loquela` when failed or pinned terminal blocks exist in the active Console session.

## Verification
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela/Loquela.test.tsx` -> `42 passed; 0 failed`.
