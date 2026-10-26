# Phase 211 Plan 01 Summary: Loquela Composer Progressive Disclosure, Single-Row Toolbar & Floating Posture Popover

**Status:** Complete
**Requirements Verified:** `LOQUELA-CLEANUP-01`, `LOQUELA-CLEANUP-02`, `LOQUELA-CLEANUP-03`, `LOQUELA-CLEANUP-04`

## Key Changes

1. **Single-Row Composer Action Bar (`crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx`)**:
   - Collapsed the 4-row stacked toolbar into a single horizontal row (`+`, `link`, `mic`, `[⚡ <Clutch> · <Risk> ▾]`, `[Run on <Tier> ▾]`, `[<Track> ▾]`, `[@kb:<scope>]`, and `~$cost`).
2. **Floating Glassmorphic "Execution Posture & Routing" Popover (`loquela-posture-popover`)**:
   - Consolidated `DriveConsole` (Clutch detents + Risk posture + Spend gauge), `Oversight` (`1..5`), `@kb` Scope (`auto | docs | code | memory | verified | off`), `Skill` picker, `Intent` toggle, `trailingSlot` (`Research: Auto Socrates`), `loquela-live-posture-badge`, and `Keys Alt+/` (`KeybindHud`) inside the floating popover anchored to `loquela-posture-trigger`.
3. **Verification**:
   - `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela/Loquela.test.tsx src/components/surfaces/Loquela/DriveConsole.test.tsx src/components/surfaces/Loquela/RiskPopover.test.tsx src/components/surfaces/Loquela/IntentPanel.test.tsx src/lib/buildChatTurn.test.ts` — `5/5` suites (`86/86` tests) passed.
