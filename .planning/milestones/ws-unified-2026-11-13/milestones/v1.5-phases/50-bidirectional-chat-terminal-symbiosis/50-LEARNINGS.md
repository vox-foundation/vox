---
phase: 50
phase_name: "bidirectional-chat-terminal-symbiosis"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 9
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 50 Learnings: bidirectional-chat-terminal-symbiosis

## Decisions

### Added TermSecurityLevel (L1ReadOnly, L2SafeExec, L3WorkspaceWrite, L4Sys...
Added `TermSecurityLevel` (`L1ReadOnly`, `L2SafeExec`, `L3WorkspaceWrite`, `L4SystemExec`, `L5AutopilotFloor`) and `TermControlMode` (`Manual`, `CoPilot`, `Autopilot`) in [`crates/vox-terminal-core/src/host.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/host.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-01-SUMMARY.md

---

### Enforced TermSecurityLevel::allows_agent_command on BlockOrigin::Agent s...
Enforced `TermSecurityLevel::allows_agent_command` on `BlockOrigin::Agent` submissions and automatic `Autopilot -> Manual` drop whenever a user submits a command or types raw PTY bytes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-01-SUMMARY.md

---

### Exposed term.security and term.control RPC handlers in [crates/vox-orche...
Exposed `term.security` and `term.control` RPC handlers in [`crates/vox-orchestrator-d/src/term_dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-d/src/term_dispatch.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-01-SUMMARY.md

---

### Created [crates/vox-gui/ui/src/lib/terminalBridge.ts](file:///Users/brbr...
Created [`crates/vox-gui/ui/src/lib/terminalBridge.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/lib/terminalBridge.ts) to synchronize active terminal session, blocks, pinned block IDs, `L1..L5` security level, and `Manual | Co-Pilot | Autopilot` control mode with `Loquela`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-02-SUMMARY.md

---

### Added term-security-level-select and term-control-mode-pill to [Console....
Added `term-security-level-select` and `term-control-mode-pill` to [`Console.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx) with automatic `Autopilot -> Manual` drop on user submission.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-02-SUMMARY.md

---

### Extended Loquela @ autocomplete popover in [Loquela.tsx](file:///Users/b...
Extended `Loquela` `@` autocomplete popover in [`Loquela.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx) with `@term`, `@term:last`, `@term:failed`, and `@term:pinned` options (`data-testid="loquela-at-term-option"`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-03-SUMMARY.md

---

### Rendered data-testid="loquela-ambient-term-pill" in Loquela when failed ...
Rendered `data-testid="loquela-ambient-term-pill"` in `Loquela` when failed or pinned terminal blocks exist in the active Console session.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-03-SUMMARY.md

---

### Wired Cmd/Ctrl+Shift+L (open Ask Agent composer) and Cmd/Ctrl+Shift+K (p...
Wired `Cmd/Ctrl+Shift+L` (open Ask Agent composer) and `Cmd/Ctrl+Shift+K` (pin/unpin latest block) keyboard shortcuts in [`Console.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-04-SUMMARY.md

---

### Verified end-to-end bidirectional symbiosis across Console.test.tsx, Blo...
Verified end-to-end bidirectional symbiosis across `Console.test.tsx`, `BlockStreamView.test.tsx`, `InputEditor.test.tsx`, and `Loquela.test.tsx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 50-04-SUMMARY.md

## Lessons

### CTERM-01: 5-tier TermSecurityLevel (L1..L5), TermControlMode (Manual | C...
**`CTERM-01`**: 5-tier `TermSecurityLevel` (L1..L5), `TermControlMode` (Manual | CoPilot | Autopilot), and instant human takeover

**Context:** Verified in phase acceptance checks.
**Source:** 50-VERIFICATION.md

---

### Command: cargo test -p vox-terminal-core --test host
Command: `cargo test -p vox-terminal-core --test host`

**Context:** Verified in phase acceptance checks.
**Source:** 50-VERIFICATION.md

## Patterns

## Surprises

