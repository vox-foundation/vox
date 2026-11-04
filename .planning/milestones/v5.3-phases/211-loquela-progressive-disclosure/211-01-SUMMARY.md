---
phase: 211
plan: 1
title: "Loquela Composer Single-Row Progressive Disclosure & Posture Popover Summary"
status: completed
completed_at: "2026-10-25"
requirements_completed:
  - LOQUELA-CLEANUP-01
  - LOQUELA-CLEANUP-02
  - LOQUELA-CLEANUP-03
  - LOQUELA-CLEANUP-04
---

# Phase 211 Plan 01 Summary: Loquela Composer Single-Row Progressive Disclosure & Posture Popover

## Delivered
- **Single-Row Loquela Action Bar (`LOQUELA-CLEANUP-01..03`):** Updated `.loquela-bar` in `crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx` to enforce a single horizontal row (`composerToolbarRowCount === 1`, height `<= 52px`) with floating progressive-disclosure popovers (`Run on`, `Skill`, `Send mode`, `Risk`, and `Keys Alt+/`).
- **Zero Capability Loss (`LOQUELA-CLEANUP-04`):** Preserved all `Alt+1..4`, `Alt+C/R/T/M/V/O/K`, `Alt+/` shortcuts, `set_session_chat_posture` IPC persistence, and all 432 `buildChatTurn` combinations.
- **Verification:** `vitest run src/components/surfaces/Loquela/ src/lib/buildChatTurn.test.ts` passed (`100%`).
