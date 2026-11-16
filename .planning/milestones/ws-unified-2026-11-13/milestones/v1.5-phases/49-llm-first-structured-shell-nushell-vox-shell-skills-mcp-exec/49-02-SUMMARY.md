---
phase: 49-llm-first-structured-shell-nushell-vox-shell-skills-mcp-exec
plan: "02"
subsystem: vox-cli, vox-orchestrator-mcp
tags: [nushell, aci, mcp, exec-policy]
provides:
  - run_nu_structured and diagnose_nu_posix_isms in vox-cli shell backend
  - structured_data and syntax_hints in ShellExecutionProbe and ACI execution_probe envelope
  - run_check_with_backend("nu" | "grammar") in check_terminal.rs
key-files:
  modified:
    - crates/vox-cli/src/commands/runtime/shell/backends/nushell.rs
    - crates/vox-cli/src/commands/runtime/shell/aci_adapter.rs
    - crates/vox-cli/src/commands/runtime/shell/check_terminal.rs
    - crates/vox-orchestrator-mcp/src/aci/envelope.rs
completed: 2026-10-15
---

# Phase 49 Plan 02: ACI Structured Probe & Exec-Policy Nushell Bridge Summary

## Accomplishments
- Added `run_nu_structured` and `diagnose_nu_posix_isms` in [`crates/vox-cli/src/commands/runtime/shell/backends/nushell.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/runtime/shell/backends/nushell.rs) and wired `structured_data` + `syntax_hints` into [`crates/vox-cli/src/commands/runtime/shell/aci_adapter.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/runtime/shell/aci_adapter.rs).
- Extended [`crates/vox-orchestrator-mcp/src/aci/envelope.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/aci/envelope.rs) to validate and preserve `structured_data` and `syntax_hints` inside `aci.execution_probe`.
- Added `run_check_with_backend` in [`crates/vox-cli/src/commands/runtime/shell/check_terminal.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/runtime/shell/check_terminal.rs) to route `nu`/`grammar` checks through `vox_container::exec_grammar`.

## Verification
- `cargo --config 'build.rustc-wrapper=""' test -p vox-orchestrator-mcp --lib aci_` -> `6 passed; 0 failed`.
