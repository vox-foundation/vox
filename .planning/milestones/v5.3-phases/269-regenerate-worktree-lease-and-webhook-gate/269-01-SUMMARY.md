---
phase: 269-regenerate-worktree-lease-and-webhook-gate
plan: 01
subsystem: vox-orchestrator-mcp
tags: [t3, regenerate, edit-in-place, worktree-lease, webhook-dedupe, compact-status]
completed: 2026-10-25
---

# Phase 269 Plan 01 Summary: Message Regenerate/Edit-in-Place, Worktree Lease/Env Provisioning & Webhook Dedupe Gate

## Delivered Requirements (`T3-REGEN-WORKTREE-01..04`)

1. **`T3-REGEN-WORKTREE-01` — Assistant Message Edit-In-Place & Turn Regenerate (`GAP-09`):**
   - Implemented `chat_edit_message_in_place` (`vox_chat_edit_message_in_place`) and `chat_regenerate` (`vox_chat_regenerate`) in [`crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs) and registered both in [`dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch.rs) and [`input_schemas.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/input_schemas.rs).

2. **`T3-REGEN-WORKTREE-02` — `DelegationWorktree::create` Lease Serialization & `.env` Copy-on-Provision (`GAP-04`):**
   - Wired `WorkspaceLeaseMap::global().lease_for(repo_root)` and `provision_untracked_env_files` into `DelegationWorktree::create` in [`crates/vox-orchestrator-mcp/src/agy_worktree.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/agy_worktree.rs).

3. **`T3-REGEN-WORKTREE-03` — Live `WebhookDedupeAndRateGate` Enforcement in `submit_webhook_events` (`GAP-08`):**
   - Wired `WebhookDedupeAndRateGate::admit_delivery` into `submit_webhook_events` in [`crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs) so duplicate delivery IDs and bursts exceeding `60/min` are dropped before hopper submission.

4. **`T3-REGEN-WORKTREE-04` — Compact `vox_orchestrator_status` Dispatch & Real-Payload `transfer_budget.rs` Gate (`GAP-10`):**
   - Wired `args.get("compact")` in [`crates/vox-orchestrator-mcp/src/dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch.rs) to `dei_tools::orchestrator_status_compact` and added `live_orchestrator_status_compact_and_dom_shedding_respect_transfer_budgets` in [`crates/vox-orchestrator-mcp/tests/transfer_budget.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/tests/transfer_budget.rs).

## Verification Evidence
- `cargo test -p vox-orchestrator-mcp --lib -- agy_worktree webhook_intake history` → **PASS**
- `cargo test -p vox-orchestrator-mcp --test transfer_budget` → **PASS**
