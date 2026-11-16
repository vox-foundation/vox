---
phase: 338-spoke-aware-gating-and-hub-repos
plan: "05"
subsystem: mens-eval
tags: [agent-episodes, salvage, suite-report, mcp]
requires:
  - phase: 338-01..04
    provides: "Core gate state, receipt v2, and spoke gate tables"
provides:
  - "Tool catalog export mens/data/agent_episodes/tool-catalog.v1.json (vox_tool_catalog_v1, 11 tools)"
  - "72 held-out multi-turn episodes in mens/data/agent_episodes/episodes.v1.jsonl (12 per Behavior)"
  - "12 shared salvage parity test cases in mens/data/agent_episodes/salvage-cases.v1.jsonl"
  - "Episode runner, control matrix, SuiteReport verifier, and salvage parity in vox-eval and vox-orchestrator-mcp"
affects: [vox-eval, vox-orchestrator-mcp, mens/data/agent_episodes]
tech-stack:
  added: []
  patterns: [schema-validated episode runner, shared salvage test fixtures]
key-files:
  created:
    - mens/data/agent_episodes/tool-catalog.v1.json
    - mens/data/agent_episodes/episodes.v1.jsonl
    - mens/data/agent_episodes/salvage-cases.v1.jsonl
    - mens/data/agent_episodes/README.md
    - crates/vox-orchestrator-mcp/src/tool_catalog.rs
    - crates/vox-eval/src/episode.rs
    - crates/vox-eval/src/episode_controls.rs
    - crates/vox-eval/src/suite_report.rs
  modified:
    - crates/vox-orchestrator-mcp/src/lib.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs
    - crates/vox-eval/src/lib.rs
key-decisions:
  - "Require >=2 offered tools per episode and expect.tool != offered_tools[0] on right_tool cases so trivial controls fail cleanly"
  - "Verify nonce echo either as an exact argument value or embedded substring (e.g. src/${nonce}.vox)"
  - "Pin 12 shared salvage cases between vox-orchestrator-mcp and vox-eval via salvage_cases_agree"
requirements-completed: [SPOKE-05, SPOKE-07]
completed: 2026-11-13
---

# Phase 338 Plan 05: Agent Episodes, Salvage Parity & Suite Report Summary

**Delivered the 72-episode MCP behavioral suite, tool-catalog snapshot, shared salvage parity fixture, and paired `SuiteReport` scoring engine.**

## Key Changes
- Exported `vox_tool_catalog_v1` (11 tools) from `crates/vox-orchestrator-mcp/src/tool_catalog.rs` to `mens/data/agent_episodes/tool-catalog.v1.json`.
- Authored 72 held-out episodes across all 6 `Behavior` classes (12 each) in `mens/data/agent_episodes/episodes.v1.jsonl`, plus 12 shared salvage test vectors in `mens/data/agent_episodes/salvage-cases.v1.jsonl` and `mens/data/agent_episodes/README.md`.
- Implemented `episode`, `episode_controls`, and `suite_report` in `crates/vox-eval`, and verified parity with `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs` (`salvage_cases_agree`).

## Verification
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib -- tool_catalog salvage`: `6 passed; 0 failed; 1006 filtered out`.
- `timeout 900s cargo test -p vox-eval --lib`: `238 passed; 0 failed; 4 ignored` (including `episode_controls::tests::control_matrix` and `episode::tests::salvage_cases_agree`).
