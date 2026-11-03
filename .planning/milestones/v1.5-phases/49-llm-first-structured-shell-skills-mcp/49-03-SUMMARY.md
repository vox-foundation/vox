---
phase: 49-llm-first-structured-shell-nushell-vox-shell-skills-mcp-exec
plan: "03"
subsystem: vox-skills, vox-terminal-core
tags: [agentskills, nushell, slash-commands]
provides:
  - nushell-structured-shell SKILL.md in crates/vox-skills/skills/superpowers/
  - /nu slash command registration in vox-terminal-core::commands::CommandRegistry
key-files:
  created:
    - crates/vox-skills/skills/superpowers/nushell-structured-shell/SKILL.md
  modified:
    - crates/vox-terminal-core/src/commands.rs
    - crates/vox-terminal-core/src/input.rs
completed: 2026-10-15
---

# Phase 49 Plan 03: Nushell Structured Shell Skill & `/nu` Command Summary

## Accomplishments
- Authored spec-compliant [`crates/vox-skills/skills/superpowers/nushell-structured-shell/SKILL.md`](file:///Users/brbrainerd/dev/vox/crates/vox-skills/skills/superpowers/nushell-structured-shell/SKILL.md) with `vox-shell: nushell` metadata and structured JSON pipeline patterns.
- Registered `/nu` in [`crates/vox-terminal-core/src/commands.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-terminal-core/src/commands.rs) (`CommandRegistry::default()`) and routed `/nu --help` / bare `/nu` to help output while `/nu <pipeline>` executes structured capture.

## Verification
- `cargo --config 'build.rustc-wrapper=""' test -p vox-terminal-core` -> `43 passed; 0 failed`.
