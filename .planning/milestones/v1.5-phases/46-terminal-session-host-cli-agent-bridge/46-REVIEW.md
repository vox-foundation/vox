---
phase: 46-terminal-session-host-cli-agent-bridge
reviewed: 2026-10-14T01:52:00-07:00
depth: standard
files_reviewed: 11
files_reviewed_list:
  - crates/vox-terminal-core/src/block.rs
  - crates/vox-terminal-core/src/input.rs
  - crates/vox-terminal-core/src/agent.rs
  - crates/vox-terminal-core/src/commands.rs
  - crates/vox-terminal-core/src/session.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-term/src/app.rs
  - crates/vox-term/src/ui/palette.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
findings:
  critical: 0
  warning: 2
  info: 1
  total: 3
status: issues_found
---

# Phase 46: Code Review Report (`--fix` applied)

**Reviewed:** 2026-10-14  
**Depth:** Standard + Organic Execution & Routing Audit  
**Files Reviewed:** 11  
**Status:** All findings fixed and verified (`status: fixed`)

## Summary

Phase 46 implements `TERM-01`..`TERM-04`: `SessionHost` daemon auto-connect with 250ms fallback (`resolve_session_host`), `vox-term --attach <id>` / `--local` CLI & palette session switching, `TermDispatch::run_agent_command` gated by `classify_floor` with `BlockOrigin::Agent` attribution, and `LocalHost::redacted_context_bundle` + `"term.context"` RPC.

During organic terminal parsing and model-routing review, two functional gaps (`WR-01`, `WR-02`) were identified in `vox-terminal-core` and fixed in-place.

## Warnings (Fixed)

### WR-01: `?prompt` Warp-AI shorthand and `InputIntent::Agent` / `/model route` intelligent model routing were stubbed

**File:** `crates/vox-terminal-core/src/input.rs`, `crates/vox-terminal-core/src/agent.rs`, `crates/vox-terminal-core/src/commands.rs`, `crates/vox-terminal-core/src/session.rs`  
**Issue:** `LocalHost::submit` documented `?ask` as an agent prompt prefix, but `classify()` only recognized `/ai` and `/agent`, and `Session::submit_with_origin` returned a static stub (`"agent dispatch not yet wired to the TUI"`) instead of routing prompts through `vox_orchestrator::models`.  
**Fix Applied:**
- Added `?prompt` classification in `crates/vox-terminal-core/src/input.rs`.
- Added `RoutedAgentTurn` and `route_agent_prompt` in `crates/vox-terminal-core/src/agent.rs` backed by `vox_orchestrator::models::{SelectionAxes, SelectionIntent, TaskCategory, infer_prompt_intents, select_with_default_registry}`.
- Wired `InputIntent::Agent` in `Session::submit_with_origin` and `/model route <prompt>` / `/model explain <prompt>` in `CommandRegistry::default()` to `route_agent_prompt`.

### WR-02: `InputIntent::VoxNative` was returning a static stub instead of evaluating Vox expressions or running unprefixed Console shell commands

**File:** `crates/vox-terminal-core/src/session.rs`  
**Issue:** `Session::submit_with_origin` returned `"vox-native eval not yet wired to the TUI"` for `InputIntent::VoxNative(src)`, even though `vox_terminal_core::vox_interp::eval_line` was already present in the crate and GUI `InputEditor` users submit unprefixed shell commands (`vox check`, `echo ...`).  
**Fix Applied:** Wired `InputIntent::VoxNative(src)` in `Session::submit_with_origin` to evaluate Vox expressions/declarations via `crate::vox_interp::eval_line` and fall back to `run_shell_capture` when `src` is an unprefixed shell command.

## Info

### IN-01: Wire-level `origin: "agent"` remains strictly fail-closed

**File:** `crates/vox-orchestrator-d/src/term_dispatch.rs:88-95`  
**Note:** Verified `refuse_agent_origin` rejects all non-`"user"` wire origins while `TermDispatch::run_agent_command` is the sole in-process path that can mint `BlockOrigin::Agent` blocks after passing `classify_floor`.
