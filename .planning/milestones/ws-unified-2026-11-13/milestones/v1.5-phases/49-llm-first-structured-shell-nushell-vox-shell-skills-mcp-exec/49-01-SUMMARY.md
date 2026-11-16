---
phase: 49-llm-first-structured-shell-nushell-vox-shell-skills-mcp-exec
plan: "01"
subsystem: vox-terminal-core
tags: [nushell, osc633, structured-shell, pty, vox-term]
provides:
  - NU_OSC633 full lifecycle markers (633;A, 633;B, 633;C, 633;D;<exit>, 633;E;<cmd>)
  - OpenSpec.shell custom shell selection
  - InputIntent::StructuredShell(!nu / /nu) and run_nu_structured_capture + diagnose_nu_posix_isms
key-files:
  modified:
    - crates/vox-terminal-core/src/pty.rs
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/input.rs
    - crates/vox-terminal-core/src/session.rs
completed: 2026-10-15
---

# Phase 49 Plan 01: Nushell Structured Shell Engine & OSC 633 Lifecycle Summary

## Accomplishments
- Upgraded `NU_OSC633` in [`crates/vox-terminal-core/src/pty.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/pty.rs) to emit full `633;A`, `633;B`, `633;C`, `633;D;<exit>`, and `633;E;<cmd>` markers.
- Added `diagnose_nu_posix_isms` and `run_nu_structured_capture` in `pty.rs` with graceful fallback when `nu` is absent on `PATH`.
- Added `shell: Option<String>` to `OpenSpec` in [`crates/vox-terminal-core/src/host.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/host.rs) and `InputIntent::StructuredShell` routing in [`crates/vox-terminal-core/src/input.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/input.rs) and [`crates/vox-terminal-core/src/session.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/session.rs).

## Verification
- `cargo --config 'build.rustc-wrapper=""' test -p vox-terminal-core` -> `43 passed; 0 failed` (unit) + `40 passed; 0 failed` (integration).
