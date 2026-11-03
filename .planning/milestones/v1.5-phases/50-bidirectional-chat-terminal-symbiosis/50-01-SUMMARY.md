---
phase: 50-bidirectional-chat-terminal-symbiosis-security-levels-l1-l5
plan: "01"
subsystem: vox-terminal-core, vox-orchestrator-d, vox-gui
tags: [terminal-security, l1-l5, control-handoff, autopilot, session-host]
provides:
  - TermSecurityLevel (L1ReadOnly..L5AutopilotFloor) with allows_agent_command
  - TermControlMode (Manual, CoPilot, Autopilot) with automatic Autopilot -> Manual drop on user input/submit
  - term.security and term.control daemon RPC methods in vox-orchestrator-d
key-files:
  modified:
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/daemon_host.rs
    - crates/vox-terminal-core/src/lib.rs
    - crates/vox-orchestrator-d/src/term_dispatch.rs
    - crates/vox-gui/src/commands/pty.rs
completed: 2026-10-15
---

# Phase 50 Plan 01: Terminal Security Levels L1..L5 & Control Handoff Engine Summary

## Accomplishments
- Added `TermSecurityLevel` (`L1ReadOnly`, `L2SafeExec`, `L3WorkspaceWrite`, `L4SystemExec`, `L5AutopilotFloor`) and `TermControlMode` (`Manual`, `CoPilot`, `Autopilot`) in [`crates/vox-terminal-core/src/host.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/host.rs).
- Enforced `TermSecurityLevel::allows_agent_command` on `BlockOrigin::Agent` submissions and automatic `Autopilot -> Manual` drop whenever a user submits a command or types raw PTY bytes.
- Exposed `term.security` and `term.control` RPC handlers in [`crates/vox-orchestrator-d/src/term_dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-d/src/term_dispatch.rs).

## Verification
- `cargo --config 'build.rustc-wrapper=""' test -p vox-terminal-core` -> `43 passed; 0 failed`.
- `cargo --config 'build.rustc-wrapper=""' test -p vox-orchestrator-d --bin vox-orchestrator-d term_dispatch` -> `8 passed; 0 failed`.
