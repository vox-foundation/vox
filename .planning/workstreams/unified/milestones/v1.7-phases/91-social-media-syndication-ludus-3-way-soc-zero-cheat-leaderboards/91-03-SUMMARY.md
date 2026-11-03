---
phase: 91-social-media-syndication-multi-user-ludus-extraction-3-way-g
plan: 03
subsystem: gamify
tags: [ludus, vox-db, vox-gamify, vox-ops-ludus, separation-of-concerns]
requires:
  - phase: 91-social-media-syndication-multi-user-ludus-extraction-3-way-g
    provides: vox-ops-ludus PostgreSQL schema and multi-user modules
provides:
  - Tier 1 operational agent/cost telemetry in crates/vox-db/src/store/ops_agent_telemetry.rs
  - Deletion of unconsented shadow telemetry (hint_telemetry.rs, lsp_telemetry.rs)
  - Tier 2 multi-user Ludus (collegium, arena, disputes) extracted to vox-ops-ludus
  - Tier 3 solo local Ludus and FSRS discovery 100% functional offline
affects: [vox-db, vox-gamify, vox-orchestrator, vox-orchestrator-mcp, vox-lsp, vox-cli]
tech-stack:
  added: []
  patterns: [3-way Ludus separation of concerns]
key-files:
  created:
    - crates/vox-db/src/store/ops_agent_telemetry.rs
  modified:
    - crates/vox-db/src/store/mod.rs
    - crates/vox-db/src/lib.rs
    - crates/vox-gamify/src/lib.rs
    - crates/vox-gamify/src/db/mod.rs
    - crates/vox-gamify/src/db/profile.rs
    - crates/vox-gamify/src/db/process_rewards.rs
    - crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/task_submit.rs
    - crates/vox-orchestrator-mcp/src/a2a_tools.rs
    - crates/vox-orchestrator-mcp/src/dei_tools/control.rs
    - crates/vox-orchestrator-mcp/src/dei_tools/vcs_runtime.rs
    - crates/vox-orchestrator-mcp/src/dispatch.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
    - crates/vox-lsp/src/server.rs
    - crates/vox-cli/src/commands/check.rs
    - crates/vox-cli/src/commands/extras/ludus/mod.rs
    - crates/vox-cli/src/commands/extras/ludus_cli.rs
key-decisions:
  - "Moved AgentEventRecord, CostRecord, and AgentSessionRecord persistence to crates/vox-db/src/store/ops_agent_telemetry.rs so vox-orchestrator and vox-orchestrator-mcp do not depend on vox-gamify for operational token/cost budgets."
  - "Deleted unconsented shadow telemetry modules hint_telemetry.rs and lsp_telemetry.rs and removed their hooks from vox-lsp and vox-cli."
  - "Extracted collegium, arena, disputes, auth, and sync multi-user modules to vox-ops-ludus while keeping solo Ludus progression and FSRS discovery 100% offline."
requirements-completed: [LUDUS-SOC-01, LUDUS-SOC-02, LUDUS-SOC-03, LUDUS-SOC-04, LUDUS-SOC-05]
duration: 20min
completed: 2026-10-19
---

# Phase 91 Plan 03: 3-Way Ludus & Gamification Separation of Concerns Summary

**Extracted Tier 1 operational agent/cost telemetry into `vox_db::ops_agent_telemetry`, deleted shadow telemetry (`hint_telemetry.rs`, `lsp_telemetry.rs`), extracted Tier 2 multi-user Ludus (`collegium`, `arena`, `disputes`) to `vox-ops-ludus`, and verified Tier 3 solo offline progression and FSRS discovery.**

## Verification
- `cargo test -p vox-gamify --lib` — **229 passed, 0 failed**.
- `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml -p vox-ops-ludus` — **1 passed, 0 failed**.
