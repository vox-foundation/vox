---
phase: 49-llm-first-structured-shell-skills-mcp
verified: 2026-10-15T00:08:00Z
status: passed
score: 4/4 must-haves verified
---
# Phase 49 Verification: LLM-First Structured Shell, Skills & MCP

## Executive Summary
All must-haves for Phase 49 (`SHELL-01..04`) have been delivered and verified:
1. `NU_OSC633` emits all 5 OSC-633 markers (`A/B/C/D/E`), `diagnose_nu_posix_isms` catches POSIX idioms, `OpenSpec.shell` configures custom shells, and `!nu` / `/nu` dispatches structured captures via `run_nu_structured_capture`.
2. `run_nu_structured` executes Nushell pipelines with `| to json --raw`, `aci.execution_probe` preserves `structured_data` and `syntax_hints`, and `vox shell check` supports `--backend auto|pwsh|nu|grammar` with pure-Rust `exec_grammar` validation.
3. Spec-compliant `nushell-structured-shell` skill installed with 3-Layer Execution Pyramid and POSIX translation table, and `/nu` slash command registered in `CommandRegistry`.
4. Console UI supports shell mode toggle (`default` | `nu` | `vox`) and renders `structured json` badges and formatted previews for structured blocks.

## Automated Test Results
- `vox-terminal-core`: 42 lib tests + 35 integration tests passed (including `commands`, `pty`, `session`, `input`).
- `vox-orchestrator-mcp`: 6/6 ACI envelope tests passed.
- `vox-cli`: 20/20 check_terminal policy tests passed.
- `vox-gui`: 37/37 Console component tests passed in Vitest.
