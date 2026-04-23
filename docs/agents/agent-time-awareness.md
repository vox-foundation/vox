---
title: "Agent Time Awareness"
description: "Agent support documentation for agent time awareness"
category: "contributor"
status: "current"
training_eligible: true
---
# Agent Time Awareness: Architecture & Implementation

> [!IMPORTANT]
> This document is the SSOT for time-awareness in the Vox orchestrator and agent mens.
> All orchestrator, A2A, and MCP tool changes relating to temporal reasoning must stay
> aligned with the patterns recorded here.

---

## 1. The Problem of Temporal Blindness

Conventional LLMs, and by extension the agents that use them, operate in a "timeless void."
They do not possess an internal clock or a subjective sense of elapsed time.

- **Amnesia between turns**: An agent restarted 5 seconds after the last run, or 5 months
  later, has no intrinsic way to differentiate those scenarios unless explicitly told.
- **Redundant work**: Agents trigger expensive operations (full `cargo check`, workspace
  re-index, file tree walks) immediately after a restart, even when nothing changed.
- **Misestimation**: LLMs cannot reliably judge how long tasks should take, leading to
  stalled loops or premature timeouts.

## 2. Creative Applications

Making agents time-aware unlocks advanced behaviors beyond timestamp injection:

- **"Boredom" workflows** — If significant time has passed without interaction, agents can
  autonomously execute low-priority background tasks (linting, corpus generation, etc.).
- **Freshness decay** — Knowledge can self-assess its relevance. A test failure from
  2 minutes ago is immediately actionable; one from 3 weeks ago needs a clean re-run.
- **Impatient escalation** — Tracking wall-clock vs. expected duration lets agents
  interrupt hung terminal commands rather than waiting indefinitely.
- **Session heartbeats** — Agents can cite their absence and use that absence to scope
  what needs to be verified, e.g. "3 days since last session — re-checking build status."

## 3. Scope

### In-scope (implemented)
1. **Task-level timestamps** — `AgentTask.started_at_ms`, `last_expensive_op_ms` with
   `start()` / `record_expensive_op()` / `elapsed_since_last_expensive_op_ms()`.
2. **A2A message freshness** — `A2AMessage.elapsed_ms()` + passive inbox TTL filtering
   (messages >5 min are silently dropped at `MessageBus::inbox()` read time).
3. **Context entry timestamps** — `ContextEntry.set_at`, `ContextStore::age_secs()`,
   `ContextStore::is_fresh()` for key/max-age checks.
4. **Session temporal summary** — `Session.last_expensive_op_at`,
   `Session::temporal_summary()`, persisted via `SessionEvent::ExpensiveOpRecorded`.
5. **Heartbeat staleness gate** — `HeartbeatMonitor::seconds_since_last_seen()` and
   `should_recheck_workspace()` for gating re-index decisions on agent liveness.
6. **Orchestrator temporal context** — `Orchestrator::build_temporal_context()` combines
   session summary + task age into a single string for system prompt injection.
7. **Bootstrap context preamble** — `MemoryManager::bootstrap_context()` prepends
   current date and Unix epoch to every agent context window.
8. **MCP tool freshness gate** — `vox_repo_index_status` and `vox_repo_index_refresh`
   check `ContextStore::is_fresh("workspace_index_*", 30)` before walking the file tree.
9. **Queue start() wiring** — `AgentQueue::dequeue()` calls `task.start()` at the
   moment the task transitions to `InProgress`.

### Out-of-scope
- Native temporal-head training (ML research, long-term).
- Real-time polling loops (time-awareness is event-driven, not continuous).
- New Arca/DB schema migrations (all state lives in existing in-process fields).
- New crates (all changes are within existing crates).

## 4. Affected Files

| File | Change |
|------|--------|
| `crates/vox-orchestrator/src/types.rs` | `AgentTask`: `started_at_ms`, `last_expensive_op_ms`, `start()`, `record_expensive_op()`, `elapsed_since_last_expensive_op_ms()`. `A2AMessage`: `elapsed_ms()`. `now_unix_ms()` helper. Unit tests for all methods. |
| `crates/vox-orchestrator/src/context.rs` | `ContextEntry.set_at`, `ContextStore::age_secs()`, `ContextStore::is_fresh()`. |
| `crates/vox-orchestrator/src/session.rs` | `Session.last_expensive_op_at`, `record_expensive_op()`, `expensive_op_age_secs()`, `temporal_summary()`. `SessionEvent::ExpensiveOpRecorded` persisted in JSONL. |
| `crates/vox-orchestrator/src/heartbeat.rs` | `HeartbeatMonitor::seconds_since_last_seen()`, `should_recheck_workspace()`. |
| `crates/vox-orchestrator/src/orchestrator/core.rs` | `Orchestrator::context_store()` getter, `Orchestrator::build_temporal_context()`. |
| `crates/vox-orchestrator/src/queue.rs` | `AgentQueue::dequeue()` calls `task.start()` at transition to `InProgress`. |
| `crates/vox-orchestrator/src/a2a.rs` | `MessageBus::inbox()` filters messages where `elapsed_ms() > 300_000` (5 min). |
| `crates/vox-orchestrator/src/memory.rs` | `MemoryManager::bootstrap_context()` prepends `"Current date: YYYY-MM-DD.\nCurrent timestamp: Ns.\n\n"`. |
| `crates/vox-orchestrator/src/mcp_tools/tools/repo_index.rs` | `repo_index_status` / `repo_index_refresh` now `async`; check `ContextStore::is_fresh("workspace_index_*", 30)` before walking. |

## 5. Prompt Contract

Every agent system prompt assembled through `bootstrap_context()` now opens with:

```
Current date: 2026-03-22.
Current timestamp: 1742682869s.
```

Agents receiving a task via `build_temporal_context()` additionally see:

```
Session last expensive op: Ns ago. Task created: Ns ago.
```

**Enforcement rule (from `AGENTS.md` §1 Tenet 7):** Agents and workflows MUST read
these lines and use them to gate expensive operations. A re-index or full compilation
must not be triggered if the context window shows elapsed time < the relevant TTL.

## 6. TTL Reference

| Operation | Cache TTL | Mechanism |
|-----------|-----------|-----------|
| Workspace file-tree index | 30 s | `ContextStore::is_fresh("workspace_index_status", 30)` |
| A2A inbox message | 300 s (5 min) | `A2AMessage::elapsed_ms() > 300_000` dropped at read |
| Context store entry (default) | Set per caller | `ContextStore::set(..., ttl_seconds)` |

## 7. Test Coverage

All core temporal methods have unit tests in their respective files:

- `types.rs` — `task_start_sets_started_at_ms`, `expensive_op_elapsed_ms_is_monotone`,
  `a2a_message_elapsed_ms_grows_over_time`, `task_start_idempotent_timestamp_stable`.
- `context.rs` — `ttl_expiration`, `vcs_context_retrieval` (pre-existing, covered `set_at`).
- `session.rs` — integrate via `cargo test -p vox-orchestrator session`.
- `queue.rs` — `dequeue()` wiring verified via existing `mark_complete_and_unblock` test.

Run:
```bash
cargo test -p vox-orchestrator
```

---
*last_updated: 2026-03-22*
