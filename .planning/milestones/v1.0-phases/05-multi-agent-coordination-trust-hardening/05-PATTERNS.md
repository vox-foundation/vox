# Phase 5: Multi-Agent Coordination & Trust Hardening - Pattern Map

**Mapped:** 2026-09-28
**Files analyzed:** 16 (create/modify)
**Analogs found:** 16 / 16 — this is a wiring phase; every touched file has an in-tree sibling to copy from. All analog paths below are git-tracked (`git ls-files` verified for every path this document names).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/vox-orchestrator-mcp/src/dispatch.rs` (D-01/D-02, wrap `handle_tool_call_with_mode`) | controller (MCP dispatch) | request-response | same file's existing `te.run(...)` timeout-wrap boundary | exact (self-modification, wrap an existing call) |
| `crates/vox-orchestrator/src/tool_receipt.rs` (D-04, crypto routing fix) | utility (crypto) | transform | `crates/vox-crypto/src/facades.rs::keyed_hash` | exact (drop-in replacement, byte-identical construction) |
| New `crates/vox-orchestrator-mcp/src/receipt_tools.rs` (D-03, `vox_verify_task_claims`) | controller (MCP tool fn) | request-response | `crates/vox-orchestrator-mcp/src/task_tools/lifecycle.rs::fail_task` | role-match (typed Params + ToolResult, no DB) |
| `crates/vox-orchestrator-mcp/src/params.rs` (new `VerifyTaskClaimsParams`) | model (DTO) | transform | `FailTaskParams` (`params.rs:398-403`) | exact |
| `contracts/operations/catalog.v1.yaml` (new tool entry) | config (generated SSOT input) | batch | `id: a2a.ack` entry (`catalog.v1.yaml:265-284`) or `id: agy.doctor` (read-only shape, `:108-124`) | exact |
| `crates/vox-orchestrator-mcp/src/dispatch.rs` (new match arm) | controller (MCP dispatch) | request-response | `"vox_fail_task" => Ok(task_tools::fail_task(...).await)` (`dispatch.rs:737`) | exact |
| `crates/vox-orchestrator/src/orchestrator/safety.rs` (new `validate_agent_claims` wrapper, optional) | service (Orchestrator method) | request-response | `issue_tool_receipt`/`verify_tool_receipt` (`safety.rs:11-33`) | exact |
| `crates/vox-orchestrator-queue/src/locks/resource.rs` (D-06, sweep) | service (lock manager) | CRUD | `FileLockManager::force_release_stale` (`locks/refresh.rs:11`) | role-match (same "purge stale, return count" shape; different key field) |
| `crates/vox-orchestrator/src/orchestrator/core/mod.rs` (D-08, closure/spawn wiring) | provider (construction/DI site) | event-driven | `oplog` Arc-before-`Self`-literal pattern (`core/mod.rs:116,122-128,189`) | exact — literally the same problem (closure spawned before `Self` exists) already solved once for `oplog` |
| `crates/vox-orchestrator/src/orchestrator/dispatch.rs` (D-08, acquire call site) | service (dispatcher loop) | event-driven | `run_dispatcher_with_oplog`'s existing `oplog.as_ref()` optional-handle pattern (`dispatch.rs:80-88,104-116`) | exact |
| `crates/vox-orchestrator/src/hopper/types.rs` (`IntakeItem.resource_id`) | model (DTO field) | transform | `IntakeItem.session_id: Option<String>` (`hopper/types.rs:131`) | exact |
| `crates/vox-orchestrator/src/types/tasks.rs` (`AgentTask.resource_id`) | model (DTO field) | transform | `AgentTask.session_id`/`thread_id`/`plan_session_id` (`types/tasks.rs:632-634,641-643,650-652`) | exact |
| `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/mod.rs` (D-08 release) | service (task completion) | event-driven | same file's existing `complete_task_with_attestation` task-lookup-by-id shape (`success/mod.rs:101-109`) | exact (self-modification) |
| `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/fail.rs` (D-08 release) | service (task failure) | event-driven | same file's `fail_task_with_audit` task-lookup-by-id shape (`fail.rs:19-28`) | exact (self-modification) |
| `crates/vox-orchestrator/src/events.rs` (new `AgentEventKind` variant, D-10/D-12) | model (event enum) | event-driven | `LockAcquired`/`LockReleased` variants (`events.rs:243-252`) | exact — explicit anti-pattern warning: do NOT reuse `path: PathBuf`, give the new variant a proper `String` field |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (`turn_event_for_result`, D-10/D-12) | transform (event projection) | transform | same file's existing `"vox_spawn_agent" \| "vox_submit_task" => { kind: "delegation_spawned", ... }` arm (`agent_loop.rs:298-312`) | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx` (new `kind` branches) | component (React) | transform | same file's `if (event.kind === 'skill_activated')` branch (`ChatTurnEventRow.tsx:22-42`) | exact |
| `crates/vox-orchestrator/src/activity/project.rs` (D-13, thread real `session_id`) | transform (event projection) | transform | same file's `LockAcquired`/`LockReleased` arms (`project.rs:149-164`, currently hardcode `None` for session_id) | exact (self-modification) |
| `crates/vox-orchestrator/src/orchestrator/safety.rs` (D-13, `session_id` param on acquire/release) | service (Orchestrator method) | event-driven | same file's existing `acquire_resource_lock`/`release_resource_lock` (`safety.rs:71-115`) | exact (self-modification, add a param) |
| `crates/vox-gui/src/commands/activity.rs` (D-13, `ActivityFilter.session_id`) | controller (Tauri command) | request-response | `ActivityFilter.agent_id: Option<String>` (`activity.rs:12-17`) | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx` (D-13, lock chips) | component (React) | request-response (one-shot fetch, not poll) | same file's `Segment`/task-row rendering (`ChatExecutionRail.tsx:84-123,201-214`) plus its existing one-shot `getContextBudget(sessionId)` `useEffect` (`:168-181`) | role-match |
| New Playwright spec under `crates/vox-gui/ui/e2e/` (D-11) | test (e2e) | event-driven | `crates/vox-gui/ui/e2e/review/stepper.spec.ts` (interaction + screenshot-to-review-bundle) + `crates/vox-gui/ui/e2e/chat-interactions.spec.ts` (`__TAURI_EMIT__`/mock-chat pattern) | exact (compose both) |
| `docs/src/adr/025-multi-agent-lock-coherence.md`, `docs/src/adr/029-formal-intent.md` (D-05/D-09 ratification) | config (ADR doc) | transform | `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` (`:4,10` — `status: "current"` frontmatter + `**Status**: Accepted (date)` body line) | exact |

## Pattern Assignments

### `crates/vox-orchestrator-mcp/src/dispatch.rs` — D-01/D-02 receipt wrap

**Analog:** the function's own existing timeout-wrap boundary (self-modification, not a foreign file).

**Exact wrap boundary** `[dispatch.rs:353-384]`:
```rust
let call_timeout = crate::dispatch_timeout::timeout_for(name_canonical);
let result = te
    .run(|| {
        let args = args.clone();
        async move {
            match tokio::time::timeout(
                call_timeout,
                vox_telemetry::TRACE_CTX.scope(trace_ctx, async move {
                    handle_tool_call_inner(state, name_canonical, args).await
                }),
            )
            .await
            {
                Ok(inner) => inner,
                Err(_elapsed) => { /* timeout error path */ }
            }
        }
    })
    .await;
```
Issue immediately before this block; fulfill immediately after `result` is bound. `agent_id`/`session_id` extraction already exists at the top of the function (`dispatch.rs:62-63`):
```rust
let agent_id = args.get("agent_id").and_then(|v| v.as_str());
let session_id = args.get("session_id").and_then(|v| v.as_str());
```
No-agent-id default convention to reuse (`dispatch.rs:514`): `let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0u64);`.

**Orchestrator wrapper methods to call** (already implemented, do not reimplement) `[crates/vox-orchestrator/src/orchestrator/safety.rs:11-27]`:
```rust
pub fn issue_tool_receipt(&self, agent_id: AgentId, tool_name: &str, args_json: &str)
    -> Result<String, crate::tool_receipt::ToolReceiptError> { ... }
pub fn fulfill_tool_receipt(&self, receipt_id: &str, result_json: &str) -> bool { ... }
```
**Fail-open error handling (D-02):** on `Err` from `issue_tool_receipt`, `tracing::warn!` and proceed to `handle_tool_call_inner` unchanged — no `fulfill_tool_receipt` call in that branch (nothing to fulfill).
**Anti-pattern to avoid (Pitfall 1):** wrap only the block above, never the ~250 lines of approval/scope/skill/guardrail gates before it — a receipt must not be issued for a call rejected before it dispatched.

---

### `crates/vox-orchestrator/src/tool_receipt.rs` — D-04 crypto routing fix

**Analog:** `crates/vox-crypto/src/facades.rs::keyed_hash` (the replacement itself).

**Target facade** `[crates/vox-crypto/src/facades.rs:17-22]`:
```rust
pub fn keyed_hash(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(key);
    hasher.update(data);
    hasher.finalize().into()
}
```
Three call sites to fix, all in `tool_receipt.rs`: `issue_intent` (`:118-124`), `fulfill_intent` (`:154-161`), `verify` (`:185-193`) — each currently calls `.update()` multiple times in sequence (receipt_id, agent_id LE bytes, tool_name, args_hash, [result_hash], executed_at_ms LE bytes). `keyed_hash` takes one `&[u8]`, so build one `Vec<u8>` concatenating the exact same fields in the exact same order, then call `keyed_hash(&self.session_key, &buf)` once. Extract to a single private helper shared by all three call sites so the byte-ordering cannot drift between issue and verify.
**Regression check:** `tool_receipt.rs:225-263` (`issue_intent_accepts_registered_tool`, `issue_accepts_registered_tool_and_binds_result`) already assert the full issue→verify round trip and will fail on any ordering drift.

---

### New `crates/vox-orchestrator-mcp/src/receipt_tools.rs` — D-03 `vox_verify_task_claims`

**Analog:** `crates/vox-orchestrator-mcp/src/task_tools/lifecycle.rs::fail_task` (`lifecycle.rs:78-85` — small, typed-Params, `state.orchestrator.<method>()`, `ToolResult::…to_json()` return shape, no DB dependency):
```rust
pub async fn fail_task(state: &ServerState, params: FailTaskParams) -> String {
    let task_id = TaskId(params.task_id);
    let assigned = state.orchestrator.agent_assigned_to_task(task_id);
    let res = state.orchestrator.fail_task(task_id, params.reason).await.map_err(|e| e.to_string());
    match res {
        Ok(()) => { /* ... */ ToolResult::ok(...).to_json() }
        Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_TASK_ORCH_OP).to_json(),
    }
}
```
Params-struct precedent `[crates/vox-orchestrator-mcp/src/params.rs:398-403]`:
```rust
pub struct FailTaskParams {
    pub task_id: u64,
    pub reason: String,
}
```
**The logic to expose is already fully implemented — do not reimplement** `[crates/vox-orchestrator/src/tool_receipt.rs:204-214]`:
```rust
pub fn validate_agent_claims(&self, claimed_receipt_ids: &[String]) -> ReceiptValidationResult {
    let mut result = ReceiptValidationResult::default();
    for id in claimed_receipt_ids {
        match self.verify(id) {
            Ok(_) => result.valid.push(id.clone()),
            Err("Receipt not found in ledger") => result.fabricated.push(id.clone()),
            Err(_) => result.unverified.push(id.clone()),
        }
    }
    result
}
```
No `Orchestrator`-level wrapper exists for this yet — add one to `safety.rs` alongside `verify_tool_receipt` (same `&self` → `rw_read(&*self.tool_ledger)` shape, `safety.rs:30-33`).
**Where NOT to add it:** `crates/vox-orchestrator-mcp/src/trust_tools.rs` looks similarly named but wraps a *different*, DB-backed subsystem (`require_db()`-gated trust rollups, `trust_tools.rs:1,13-18`) — this tool needs no DB.

**Registration is a three-hop generated chain — do not add only a Rust match arm (Pitfall 5).**
1. Add an entry to `contracts/operations/catalog.v1.yaml`. Exact shape to copy `[catalog.v1.yaml:265-284]`:
```yaml
- id: a2a.ack
  title: A2a Ack
  description: Acknowledge a message in an agent's inbox.
  description_human: null
  product_lane: ai
  intent_tags: []
  side_effect_class: null
  scope_kind: null
  reversible: null
  requires_repo: null
  preferred_for_models: null
  human_takeover_friendly: null
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp:
    name: vox_a2a_ack
    http_read_role_eligible: false
    tier: core
  cli: null
```
   For a **read-only** tool, set `http_read_role_eligible: true` — precedent `[catalog.v1.yaml:120-124]` (`vox_agy_doctor`).
2. Run `vox ci operations-sync --target mcp --write` to regenerate `contracts/mcp/tool-registry.canonical.yaml` (itself generated — header says so, `tool-registry.canonical.yaml:1-2`) and, on next build, `vox_mcp_registry::TOOL_REGISTRY` (`crates/vox-mcp-registry/src/lib.rs:1-15`, `include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"))`).
3. Add the Rust fn (above) + dispatch.rs match arm, modeled on `[dispatch.rs:737]`:
```rust
"vox_fail_task" => Ok(task_tools::fail_task(state, serde_json::from_value(args)?).await),
```
   → `"vox_verify_task_claims" => Ok(receipt_tools::verify_task_claims(state, serde_json::from_value(args)?).await),`
**Skipping step 1/2 means `issue_intent`'s fail-closed `TOOL_REGISTRY` check (`tool_receipt.rs:108`) rejects receipts for the new tool's own calls** — the exact D-02 warn-log symptom, self-inflicted.

---

### `crates/vox-orchestrator-queue/src/locks/resource.rs` — D-06 lazy sweep

**Analog:** `FileLockManager::force_release_stale` (shape precedent only, not the DB dependency) `[crates/vox-orchestrator-queue/src/locks/refresh.rs:11]`: `pub fn force_release_stale(&self, timeout_ms: u128) -> usize`.
**Current state, no sweep exists** `[locks/resource.rs:38-108]` — `try_acquire`/`is_locked` only ever check the *one* contended `resource_id`; other expired entries are never purged.
**Concrete sweep** (ResourceLock already carries `expires_ms`, simpler than the elapsed-time-based `force_release_stale`):
```rust
// Source: pattern derived from FileLockManager::force_release_stale (locks/refresh.rs:11)
// applied to ResourceLockManager's expires_ms field (resource.rs:23).
fn sweep_expired(locks: &mut HashMap<String, ResourceLock>, now: u64) {
    locks.retain(|_, lock| lock.expires_ms > now);
}
```
Call at the top of `try_acquire` (`resource.rs:38-44`, already computes `now` locally) and `is_locked` (`resource.rs:97-102`, also computes `now` locally). D-06 leaves whether `release` (`:73-80`) also sweeps as discretion.
**Concurrency note (Pitfall 2):** `is_locked` currently only takes a read lock (`self.locks.read().unwrap()`); a sweep upgrades it to a write lock. `ResourceLockManager` uses `std::sync::RwLock` (not `parking_lot`) — verify no call site holds the write lock across an `.await` or re-enters `is_locked` while already holding the write lock.
**Test precedent to extend:** `crates/vox-orchestrator-queue/src/semcov_wave19_tests.rs:14-168` (`mod resource_lock`, existing `is_locked_returns_false_after_expiry`/`expired_lock_allows_new_holder`) — add a multi-resource sweep test (assert `len()`/`snapshot()` drop an *uncontended* expired resource, not just the one being acquired).

---

### D-08 — hopper task dispatch as the real lock caller

**Critical architecture fact (resolves RESEARCH Assumption A1):** `enqueue_closure` is built in `Orchestrator::new` **before** `Self` exists (`crates/vox-orchestrator/src/orchestrator/core/mod.rs:29-75`), capturing only `Arc<RwLock<HashMap<AgentId, ...>>>` clones (`enqueue_agents`, `enqueue_assignments`, lines 53-54) — it has **no** `Orchestrator`/`resource_locks` handle. `run_dispatcher_with_oplog` is spawned at line 123, also before `Self { .. }` is constructed at line 140. `resource_locks: crate::locks::ResourceLockManager::new()` is only constructed inline in the `Self { .. }` struct literal (`core/mod.rs:201`) — one field with **no `Arc<...>` wrapper**, because `ResourceLockManager` (`crates/vox-orchestrator-queue/src/locks/resource.rs:26-30`) is itself `#[derive(Clone, Default)]` wrapping an internal `Arc<RwLock<HashMap<...>>>` — i.e. it's already cheaply cloneable, the same trick used for `BulletinBoard` (`crates/vox-orchestrator/src/bulletin.rs:18-20`, `#[derive(Clone)]` wrapping `broadcast::Sender`).

**Exact precedent to copy — the `oplog` Arc-before-closure pattern**, already solving this identical problem for the durable op-log `[core/mod.rs:113-129,189]`:
```rust
// Constructed here (rather than only in the `Self { .. }` literal below) so the
// hopper dispatcher can share the same durable op-log the rest of the orchestrator uses.
let oplog = Arc::new(RwLock::new(crate::oplog::OpLog::default()));
...
let dispatcher_oplog = Arc::clone(&oplog);
tokio::spawn(crate::orchestrator::dispatch::run_dispatcher_with_oplog(
    dispatcher_rx, dispatcher_hopper, enqueue_closure, None, Some(dispatcher_oplog),
));
...
Self { ..., oplog, ... }   // same Arc instance shared with the spawned loop
```
**Apply the identical shape to `resource_locks` (and `bulletin`/`event_bus`, which the acquire/release wrapper methods need for the D-10 bulletin+event_bus publish, per `safety.rs:70-115`):** construct `resource_locks: ResourceLockManager::new()` (plus reuse the already-constructed `bulletin`/`event_bus`, both `Clone`) **before** the `enqueue_closure`/dispatcher spawn, `.clone()` them into a new parameter on `run_dispatcher_with_oplog` (mirroring `oplog: Option<Arc<RwLock<OpLog>>>`), and pass the same instances into the `Self { resource_locks, ... }` literal instead of a fresh `::new()`. Do the acquire call inside `run_dispatcher_with_oplog`'s loop body, right after `enqueue(task)` returns `Some(agent_id)` `[crates/vox-orchestrator/src/orchestrator/dispatch.rs:90-124]`:
```rust
let task = intake_to_task(&item);
if let Some(agent_id) = enqueue(task) {
    if hopper.assign(&item.item_id, agent_id.to_string()).await.is_ok() { ... }
}
```
**Extract the acquire+publish logic to a small free fn** shared by `Orchestrator::acquire_resource_lock` (`safety.rs:71-99`) and this loop, to avoid duplicating the bulletin/event_bus publish — the loop only has raw `ResourceLockManager`/`BulletinBoard`/`EventBus` handles, not a full `&Orchestrator`.

**Fields to add** (both `IntakeItem` and `AgentTask` — Pitfall 3, forgetting either breaks release at completion time):
- `IntakeItem.resource_id: Option<String>` — model on `session_id` `[crates/vox-orchestrator/src/hopper/types.rs:130-131]`: `/// Optional session context...` / `pub session_id: Option<String>,`
- `AgentTask.resource_id: Option<String>` — model on `session_id`/`thread_id`/`plan_session_id` `[crates/vox-orchestrator/src/types/tasks.rs:632-634,641-643,650-652]`, e.g.:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub session_id: Option<String>,
```
- Copy explicitly in `intake_to_task` `[dispatch.rs:18-26]` (currently builds `AgentTask::new(task_id, item.intent.clone(), item.classified_priority, vec![])` — add `.resource_id = item.resource_id.clone()` or thread it through `AgentTask::new`/a builder, matching the existing `.with_session(...)` builder at `types/tasks.rs:857-858`).

**Release call sites (both required, D-08 says "completion or failure" — Pitfall 4):**
- Success: `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/mod.rs` — `complete_task_with_attestation` looks up the task by id via `task_assignments`/`agents` (`success/mod.rs:101-109`); read `task.resource_id` from the same lookup and call `release_resource_lock` before returning `Ok(())`.
- Failure: `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/fail.rs` — resolved (RESEARCH Assumption A2): the handler is `fail_task_with_audit(&self, task_id: TaskId, reason: String, audit_report: Option<String>) -> Result<(), OrchestratorError>` (`fail.rs:19-24`), which already resolves `agent_id` via `task_assignments` (`fail.rs:25-28`) and locks the agent's queue to look up the task (`fail.rs:41-53`, `queue.find_task_mut(task_id)` / `queue.current_task_mut()`) — read `task.resource_id` from that same `find_task_mut`/`current_task_mut` result and release there. `fail_task` (`fail.rs:10-16`) is a thin wrapper calling `fail_task_with_audit`, so wiring the latter covers both.

**TTL value (D-13, resolves the RESEARCH open question):** reuse the existing `OrchestratorConfig.task_timeout_ms` field — **already exists**, default `1_800_000` ms / 30 min (`crates/vox-orchestrator/src/config/orchestrator_fields.rs:153-155`, doc comment: "Default task execution timeout in milliseconds (default: 1800000 / 30min)"). This is a *different* field from `lock_timeout_ms` (`orchestrator_fields.rs:70-76`, default 30_000 ms / 30 sec) — `lock_timeout_ms` already drives `FileLockManager::force_release_stale`'s sweep interval in `orchestrator/scaling.rs:183,208` and is the wrong value (file-lock subsystem, 30-second scale, not task-scoped). `task_timeout_ms` is the correct semantic match: D-13 wants the lock TTL to track how long the task is expected to run, and the research's own fallback suggestion ("30-minute constant") is exactly `task_timeout_ms`'s existing default — no new constant needed, just read the live config value (`Orchestrator` already has a `config: Arc<RwLock<OrchestratorConfig>>` field, `core/mod.rs:141`).

---

### D-10/D-12 — receipt → chat turn event

**Analog:** `turn_event_for_result`'s existing `delegation_spawned` arm `[crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:298-312]`:
```rust
"vox_spawn_agent" | "vox_submit_task" => {
    let envelope: serde_json::Value = serde_json::from_str(result_content).ok()?;
    let data = envelope.get("data")?;
    let agent_id = data.get("agent_id").and_then(serde_json::Value::as_u64)?;
    Some(serde_json::json!({
        "kind": "delegation_spawned",
        "tool": tool_name,
        "agent_id": agent_id,
        "task_id": data.get("task_id").and_then(serde_json::Value::as_u64),
    }))
}
```
Per D-12's locked design: add a sibling fn `handle_tool_call_with_receipt` in `dispatch.rs` that does D-01's issue/fulfill wrap and returns `(Result<String>, Option<receipt_id>)`; `handle_tool_call_with_mode` becomes a thin wrapper discarding the receipt. Only `agent_loop.rs`'s call site (`agent_loop.rs:887-910` in RESEARCH's citation) calls the new fn and pushes a `kind: "tool_receipt"` event using the receipt_id/verified state it now has directly in scope — no event-bus round trip needed for this path (D-12 explicitly rejects that, "extra moving parts for data already in hand one frame up").

**Frontend render branch — analog is the file's own existing branch** `[crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx:22-42]`:
```tsx
export function ChatTurnEventRow({ event, onExcludeSkill }: ChatTurnEventRowProps) {
  if (event.kind === 'skill_activated') {
    const skillId = typeof event.skill_id === 'string' ? event.skill_id : 'unknown';
    return (
      <div data-testid="chat-turn-event-row" className="...">
        <span>skill activated · {skillId}</span>
        {onExcludeSkill && skillId !== 'unknown' && (
          <button ... onClick={() => onExcludeSkill(skillId)}>not this one</button>
        )}
      </div>
    );
  }
  return null;
}
```
Add `if (event.kind === 'tool_receipt') { ... }` following this exact shape (own `data-testid`, e.g. `chat-turn-receipt-row`). No `TurnEventDto`/`dashboard.ts` schema change needed — the type is deliberately open (`kind: string; [key: string]: unknown;`, `dashboard.ts:58-64`). Already wired into the transcript: `ChatTranscript.tsx:8,89` (`import { ChatTurnEventRow } from './ChatTurnEventRow';` / `<ChatTurnEventRow key={i} event={ev} onExcludeSkill={onExcludeSkill} />`, driven by `message.events`, `ChatTranscript.tsx:86-92`).

**New event kind, not a reused one (Anti-pattern to avoid):** do NOT reuse `AgentEventKind::LockAcquired`/`LockReleased`'s `path: PathBuf` field for anything receipt-shaped — the existing code already stretches `path: PathBuf::from(resource_id)` for the generic lock case (`safety.rs:92`); don't extend that same reuse further. If an `AgentEventKind` variant is needed at all for receipts (only if the double-issue-avoiding in-scope approach above isn't used), give it its own `String`-typed field, modeled on the shape of `[crates/vox-orchestrator/src/events.rs:243-252]` (`LockAcquired`/`LockReleased`), not on `path`.

---

### D-13 — `session_id` threading for lock events + chat lock chips

**Orchestrator method signature change** — analog is the method's own current shape `[crates/vox-orchestrator/src/orchestrator/safety.rs:71-115]`:
```rust
pub fn acquire_resource_lock(&self, agent_id: AgentId, resource_id: &str, kind: crate::locks::ResourceLockKind, ttl_ms: u64) -> bool { ... }
pub fn release_resource_lock(&self, agent_id: AgentId, resource_id: &str) { ... }
```
Add `session_id: Option<&str>` (or `Option<String>`) as a new parameter to both; thread it into the `AgentMessage::ResourceLockAcquired/Released` bulletin publish and the new event kind (once threaded, `project.rs` below has a real value to project instead of `None`).

**Activity projection gap to close — analog is the same file's own current (wrong) arms** `[crates/vox-orchestrator/src/activity/project.rs:149-164]`:
```rust
LockAcquired { agent_id, path, exclusive } => (
    Some(agent_id.to_string()),
    None,                              // <- session_id, always None today
    "LockAcquired",
    format!("Lock acquired on {path:?} (exclusive: {exclusive})"),
),
LockReleased { agent_id, path } => (
    Some(agent_id.to_string()),
    None,                              // <- session_id, always None today
    "LockReleased",
    format!("Lock released on {path:?}"),
),
```
Once `AgentEventKind::LockAcquired`/`LockReleased` carry a `session_id` field (added alongside the safety.rs signature change), replace `None` with `session_id.clone().map(...)`/equivalent here.

**GUI filter DTO — analog is the sibling `agent_id` filter field** `[crates/vox-gui/src/commands/activity.rs:12-17]`:
```rust
pub struct ActivityFilter {
    pub agent_id: Option<String>,
    pub kind: Option<String>,
    pub limit: u32,
    pub before_id: Option<i64>,
}
```
Add `pub session_id: Option<String>,` following the same `Option<String>` shape as `agent_id`.

**Chat surface host — resolved (RESEARCH Assumption A3):** `ChatExecutionRail.tsx` does **not** poll. It receives `tasks: ChatExecutionTask[]` (`{ id, title, status? }`) as a prop and renders each as a row (`ChatExecutionRail.tsx:12-16,197-215`); it does have one existing one-shot fetch-on-`sessionId`-change `useEffect` (`getContextBudget(sessionId)`, lines 168-181) as a precedent for "fetch once per session, not poll." For lock chips: either (a) extend `ChatExecutionTask` with an optional `lockState?: { resourceId: string; state: 'holding' | 'waiting' }` and render a small `Pill`/badge next to the existing `task.status` line (`ChatExecutionRail.tsx:206-211`), fed by whatever parent already assembles the `tasks` prop (find that call site during planning — not yet located this session), or (b) add the same one-shot-fetch pattern as `SessionSpendTrack`/the `getContextBudget` effect, calling `activity_query` scoped to `session_id` + `kind: LockAcquired|LockReleased`. Per D-13's own text, if the intake item carries no session, the chip still renders under the task correlated by task id — so (a) (prop-driven, keyed by task) is the closer fit to that requirement than (b) (session-scoped query).

---

### D-11 — Playwright spec for receipt/lock chat chips

**Analog 1 (interaction + screenshot-to-review-bundle shape):** `crates/vox-gui/ui/e2e/review/stepper.spec.ts` (65 lines, full file is the template):
```ts
const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'review-bundle', 'latest');
test.describe('Research visual debugger stepper', () => {
  test('walks through diagnostic prober and inspector drawer', async ({ page }) => {
    await addMockInitScript(page, installTauriMock, 'research');
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto('/');
    await page.waitForSelector('nav', { timeout: 20_000 });
    // ...drive interaction via getByTestId(...)...
    await page.waitForFunction(() => (window as any).__VOX_IPC_ACTIVE_COUNT__ === 0);
    mkdirSync(OUT_DIR, { recursive: true });
    await page.screenshot({ path: join(OUT_DIR, '<name>.png') });
  });
});
```
**Analog 2 (injecting `TurnEventDto`/chat events via the mock):** `crates/vox-gui/ui/e2e/chat-interactions.spec.ts` (89 lines, full file) — `installTauriMock` + `addMockInitScript(page, installTauriMock, 'chat')`, then drives `vox://agent-events` via `window.__TAURI_EMIT__` (`chat-interactions.spec.ts:52-70`) and/or asserts on `window.__TAURI_CALLS__` for `chat_append_message` payload shape (`:39-49,82-88`) — the precedent for constructing a message whose `events: [{ kind: 'tool_receipt', ... }]` array the mock should return from `chat_history`/inject via emit, then asserting the `chat-turn-event-row`/new testid renders.
**No existing spec covers `TurnEventDto`/turn-event chips at all** (confirmed: `grep -rl "skill_activated\|TurnEventDto\|events:"` over `e2e/` matches no chat-turn-event spec) — this is a net-new spec file, composing Analog 1's screenshot-capture skeleton with Analog 2's mock-injection technique. Mock-only is acceptable per D-11 ("prefer end-to-end where cheap" — an MCP round trip producing a real receipt is the stronger option if time allows, but not required).

---

### D-05/D-09 — ADR ratification

**Analog:** `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md`:
- Frontmatter (`:4`): `status: "current"`
- Body (`:10`): `**Status**: Accepted (2026-09-22)`

Apply the same two edits to `docs/src/adr/025-multi-agent-lock-coherence.md` and `docs/src/adr/029-formal-intent.md`, both currently `status: "research"` (`025:5`, `029:5`) — flip the frontmatter `status` and add/update the `**Status**` line with the actual date these are ratified, only after D-06/D-08 (for 025) and D-01..D-04 (for 029) are implemented and passing.

## Shared Patterns

### Orchestrator wrapper-method pattern (thin `&self` methods delegating to a subsystem field)
**Source:** `crates/vox-orchestrator/src/orchestrator/safety.rs` (whole file)
**Apply to:** every new `Orchestrator` method this phase needs (`validate_agent_claims` wrapper for D-03; the `session_id`-extended `acquire_resource_lock`/`release_resource_lock` for D-13).
```rust
pub fn verify_tool_receipt(&self, receipt_id: &str) -> bool {
    let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
    ledger.verify(receipt_id).is_ok()
}
```

### Bulletin + event_bus dual-publish on state change
**Source:** `crates/vox-orchestrator/src/orchestrator/safety.rs:70-115` (`acquire_resource_lock`/`release_resource_lock`)
**Apply to:** any new "this state changed, tell the chat surface" signal (D-10's receipt event, if the event-bus-emission path is chosen over the in-scope D-12 approach).
```rust
self.bulletin.publish(crate::types::AgentMessage::ResourceLockAcquired { agent_id, resource_id: resource_id.to_string() });
self.event_bus.emit(crate::events::AgentEventKind::LockAcquired { agent_id, path: std::path::PathBuf::from(resource_id), exclusive: ... });
```

### `Arc`-before-`Self`-literal construction (sharing state with a pre-`Self` spawned task)
**Source:** `crates/vox-orchestrator/src/orchestrator/core/mod.rs:113-129,189` (the `oplog` field)
**Apply to:** D-08's `resource_locks`/`bulletin`/`event_bus` handles reaching the dispatcher loop (see D-08 section above — this is the one genuinely non-obvious pattern in the whole phase).

### `Option<String>` field addition to a DTO/task struct
**Source:** `crates/vox-orchestrator/src/types/tasks.rs:632-634` (`session_id`) and `hopper/types.rs:130-131` (`session_id`)
**Apply to:** `IntakeItem.resource_id`, `AgentTask.resource_id` (D-08).
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub session_id: Option<String>,
```

### MCP tool registration (three-hop generated SSOT chain)
**Source:** `contracts/operations/catalog.v1.yaml` → `vox ci operations-sync --target mcp --write` → `contracts/mcp/tool-registry.canonical.yaml` → `vox_mcp_registry::TOOL_REGISTRY` (build-script `include!`)
**Apply to:** D-03's `vox_verify_task_claims`. Never add only a Rust match arm (Pitfall 5).

### Unrecognized-`kind`-renders-nothing forward compatibility
**Source:** `crates/vox-gui/ui/src/types/dashboard.ts:58-64` (`TurnEventDto`, comment: "unrecognized future `kind` renders as a no-op chip instead of a crash") + `ChatTurnEventRow.tsx:43` (`return null;` fallthrough)
**Apply to:** every new chat-surface event kind this phase adds — no schema migration needed, just a new `if` branch.

## No Analog Found

None. Every file this phase touches has a same-crate, same-shape sibling already read and cited above — this is consistent with RESEARCH's framing ("wiring phase, not a build phase").

## Metadata

**Analog search scope:** `crates/vox-orchestrator/src/{orchestrator,types,hopper,activity}`, `crates/vox-orchestrator-mcp/src/{dispatch.rs,task_tools,chat_tools,params.rs}`, `crates/vox-orchestrator-queue/src/locks`, `crates/vox-crypto/src`, `crates/vox-gui/{src/commands,ui/src/components/surfaces/Chat,ui/e2e}`, `contracts/operations`, `contracts/mcp`, `docs/src/adr`.
**Files scanned:** ~30 (full or targeted reads); all analog paths confirmed git-tracked via `git ls-files`.
**Pattern extraction date:** 2026-09-28
**Open items intentionally left to the planner (not resolvable from static analysis):**
- Exact call site inside `run_dispatcher_with_oplog`'s loop for the D-08 acquire vs. a small shared free-fn extraction (design choice, not a missing analog — both options are documented above).
- The parent component that assembles `ChatExecutionRail`'s `tasks` prop (needed to wire lock-chip data in) — not located this session; a `Grep` for `<ChatExecutionRail` usage is the next step.
- Whether D-10's receipt event uses the in-scope D-12 approach (no new `AgentEventKind`) or a new event-bus variant — D-12's locked decision already answers this (in-scope, no new variant) but the events.rs analog is documented above in case planning revisits it.
