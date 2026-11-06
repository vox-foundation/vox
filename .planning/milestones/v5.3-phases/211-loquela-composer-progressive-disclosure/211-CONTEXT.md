# Phase 211: Loquela Composer Progressive Disclosure, Single-Row Toolbar & Floating Posture Popover - Context

**Gathered:** 2026-10-25
**Status:** Ready for execution

<domain>
## Phase Boundary

Collapse the 4-row stacked composer toolbar in `Loquela.tsx` into a single horizontal action row (`<= 52px` tall) with a floating glassmorphic **Execution Posture & Routing** popover (`[⚡ Balanced · moderate ▾]`), while preserving 100% of keyboard shortcuts (`Alt+1..4`, `Alt+C/R/T/M/V/O/K`, `Alt+/`), `buildChatTurn` RPC parameters, and existing unit tests.
</domain>

<decisions>
## Implementation Decisions

- **Single-Row Composer Bar**: Primary toolbar items (`+`, `link`, `mic`, `[⚡ Posture & Routing ▾]`, `[Run on <Tier> ▾]`, `[<Track> ▾]`, `[@kb:<scope>]`, and `~$cost`) sit on a single non-wrapping row (`composerToolbarRowCount === 1`).
- **Floating Glassmorphic "Execution Posture & Routing" Popover (`loquela-posture-popover`)**: Clicking `loquela-posture-trigger` (`[⚡ <Clutch> · <Risk> ▾]`) opens the floating popover anchored above the composer containing `DriveConsole` (Clutch detents + Risk posture + Spend gauge), `Oversight` (`1..5`), `@kb` Scope (`auto | docs | code | memory | verified | off`), `Skill` picker, `Intent` toggle, `trailingSlot` (`Research: Auto Socrates`), `loquela-live-posture-badge`, and `Keys Alt+/` (`KeybindHud`).
- **Zero Capability or Test Loss**: When closed, `loquela-posture-popover` uses `sr-only` so it occupies 0px in the visual toolbar while remaining 100% accessible to `Alt+*` keyboard shortcuts and existing JSDOM unit tests.
</decisions>
