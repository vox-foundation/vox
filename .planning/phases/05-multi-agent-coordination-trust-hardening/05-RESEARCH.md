# Phase 5: Multi-Agent Coordination & Trust Hardening - Research

**Researched:** 2026-09-28
**Domain:** Rust orchestrator internals (lock coherence, HMAC audit receipts) + Tauri/React chat-surface event plumbing
**Confidence:** HIGH (all load-bearing claims below are read-verified against the live tree this session; the D-10/D-11 GUI-wiring design is a recommendation, not yet code, and is flagged accordingly)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**TRUST-01: dispatch wiring**
- **D-01:** Wrap `handle_tool_call_with_mode` uniformly for every `vox_*` tool: `issue_intent` before `handle_tool_call_inner` runs, `fulfill_intent` after it returns (success or error, with the result hash). No per-tool allowlist to maintain.
- **D-02:** Fail-open. An `issue_intent` error (e.g. unknown tool name) is logged at `warn` and the tool call still executes normally — the ledger is an audit/detection trail this phase, not an authorization gate. A `fabricated`/`unverified` verdict from `validate_agent_claims` is surfaced (D-03), not auto-punished. — Reversibility: reversible.
- **D-03:** Expose `validate_agent_claims`'s verdict via a new read-only MCP tool (working name `vox_verify_task_claims`: takes claimed receipt IDs, returns the valid/fabricated/unverified split). No automatic side effect on a fabricated claim this phase — detection and reporting only.
- **D-04 (housekeeping):** Fix `tool_receipt.rs`'s crypto-policy violation while the file is open for D-01: replace the direct `blake3::Hasher::new_keyed` calls with `vox_crypto::facades::keyed_hash`. Behavior must stay identical (same MAC construction, keyed BLAKE3) — this is a routing fix, not an algorithm change.
- **D-05 (housekeeping):** Ratify ADR-029 (flip `status: research` to Accepted with a dated Status line), matching the Phase 4 pattern used for ADR-045, once its shape is implemented and correct in this phase — not before.

**MESH-01: ResourceLockManager**
- **D-06:** Add a lazy sweep: on every `try_acquire` (and `is_locked`), purge *all* currently-expired entries from the map, not just the one resource_id being contended. No background timer/task.
- **D-07:** Single-orchestrator-process coordination is the full scope for this phase. Cross-node reach via populi-mesh is an explicit non-goal — deferred. — Reversibility: reversible.
- **D-08:** The real caller that makes the manager load-bearing: hopper task dispatch. Add an optional `resource_id: Option<String>` field to the task/intake spec. When present, dispatch acquires an exclusive `ResourceLock` on it before the agent starts the task and releases on completion or failure; concurrent tasks declaring the same `resource_id` now actually contend (block/error) instead of racing silently. Tasks with no `resource_id` skip locking entirely.
- **D-09 (housekeeping):** Ratify ADR-025 the same way as D-05, once the sweep (D-06) and the real caller (D-08) are implemented.

**Chat-GUI surfacing (added 2026-09-28, user directive mid-planning)**
- **D-10:** Nothing in this phase ships orchestrator-only. Tool receipts (D-01/D-03) and resource-lock state/contention (D-06/D-08) MUST surface in the `vox-gui` chat surface — the user must be able to see, from the chat, which tool calls in a turn carry a verified receipt (and any fabricated/unverified claim), and when a task is waiting on / holding a `resource_id` lock. Reuse the existing chat transcript/tool-call rendering and event plumbing (bulletin → event bus → GUI transport) rather than adding a new panel, unless research shows no existing surface fits.
- **D-11:** "Testable with our instruments" = every D-10 surface is covered by a deterministic Playwright spec under `crates/vox-gui/ui/e2e/` driven by the mock IPC harness (`installTauriMock`/`installTauriMockRich`), capturing screenshots into `crates/vox-gui/ui/review-bundle/latest/` per AGENTS.md §GUI Visual Verification Invariant, plus `pnpm --dir crates/vox-gui/ui typecheck` green. Where the real backend path is cheap to exercise (e.g. an MCP tool-call round trip producing a receipt), prefer an end-to-end check over mock-only. Phase verification must include a live look at the chat GUI, not just unit tests.

### Claude's Discretion
- Exact MCP tool name/schema for the D-03 verify surface (`vox_verify_task_claims` is a working name, not locked).
- Exact shape of the `resource_id` field addition to the task/intake spec (D-08) — follow the existing hopper/intake type conventions.
- Whether the D-06 sweep also runs on `release` (not just acquire/is_locked) if that's cleaner.

### Deferred Ideas (OUT OF SCOPE)
- Cross-node/mesh-backed resource locking (D-07) — a distinct, larger feature for a future phase.
- Automatic consequences for a fabricated tool-claim verdict (D-03) — detection/reporting only this phase.
- Background-timer-based lock sweeping (D-06 chose lazy-on-acquire instead) — revisit only if locks are observed sitting expired-and-uncontended for long periods in practice.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MESH-01 | The locks subsystem is extended with `ResourceLockManager` for multi-agent resource coordination, lease propagation, and contention handling per ADR-025. | §Architecture Patterns Pattern 2 (D-06 sweep), Pattern 3 (D-08 hopper wiring); §Code Examples; §Common Pitfalls 2/3/4 |
| TRUST-01 | Agent tool calls emit cryptographically verifiable HMAC receipts, checkable under the two-tier formal-intent verification system defined in ADR-029. | §Architecture Patterns Pattern 1 (D-01 wrap), Pattern 4 (D-03 tool + SSOT registration), Pattern 5 (D-04 crypto fix); §Common Pitfalls 1/5 |
</phase_requirements>

## Summary

Every piece of backend logic this phase needs already exists and passes tests — this is a wiring phase, not a build phase. `ToolReceiptLedger` (`crates/vox-orchestrator/src/tool_receipt.rs`) and `ResourceLockManager` (`crates/vox-orchestrator-queue/src/locks/resource.rs`), plus their `Orchestrator` wrapper methods (`crates/vox-orchestrator/src/orchestrator/safety.rs`), are fully implemented and unit-tested but have **zero production callers**. The entire TRUST-01/MESH-01 surface is: (1) call the existing wrapper methods from the two real dispatch sites, (2) add one new read-only MCP tool, (3) add a lazy-sweep loop to an existing method, (4) reroute three `blake3::Hasher::new_keyed` call sites through `vox_crypto::facades::keyed_hash`, and (5) ratify two ADRs. None of this requires a new external dependency.

D-10/D-11 (the GUI directive) is the one part of this phase with no existing implementation to wire up — it requires a small **design decision**, not just plumbing. This research found and verified the exact hook point (`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`'s `turn_event_for_result` + the `events: Vec<serde_json::Value>` field already threaded into `TurnEventDto` → `ChatTurnEventRow.tsx`), and confirms it is open-ended-by-design (unrecognized `kind` renders nothing), so this phase can add new `kind` values without touching existing behavior. The one real design problem: `handle_tool_call_with_mode` (where D-01's wrap lives, per the locked decision) doesn't currently return receipt metadata to its caller, so a new signal path is needed to get receipt/lock state from dispatch.rs back to the chat turn loop. See §Architecture Patterns Pattern 6 for the recommended approach (reuse the same bulletin+event_bus plumbing already used for `LockAcquired`/`LockReleased`, which is literally the "bulletin → event bus → GUI transport" the user's own D-10 text names).

**Primary recommendation:** Do the five pure-Rust wiring tasks (D-01 through D-09) first — they are self-contained, low-risk, and match locked decisions exactly. Do D-10/D-11 last, once receipt/lock events have a stable emission point to observe from the GUI side.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| HMAC tool-call receipts (issue/fulfill/verify) | API / Backend (`vox-orchestrator`) | — | `ToolReceiptLedger` is pure in-process state; no persistence or network hop needed this phase. |
| Receipt-claim verification (`vox_verify_task_claims`) | API / Backend (`vox-orchestrator-mcp` dispatch table) | — | Read-only query over the in-process ledger; same tier as every other MCP tool. |
| Resource lock acquire/release/sweep | API / Backend (`vox-orchestrator-queue`) | — | In-process `Arc<RwLock<HashMap>>`, single-orchestrator-process scope per D-07. |
| Hopper→agent dispatch lock wiring | API / Backend (`vox-orchestrator::orchestrator::dispatch`) | — | The dispatcher loop already owns agent assignment; lock acquire/release is a peer concern at the same call sites. |
| Receipt/lock status rendering in chat | Frontend Server / Browser (`vox-gui` Tauri + React) | API / Backend (event emission) | Rendering is client-tier; the *signal* (bulletin/event_bus emission) is backend-tier and must exist before the frontend has anything to render. |
| Playwright verification (D-11) | Browser (mock IPC harness) | — | Tests run against the Vite/React app under Playwright with `installTauriMock`, no live backend required for the mock path. |

## Standard Stack

No new external dependency is required by this phase.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox-crypto` (in-workspace) | workspace | `keyed_hash` facade for D-04's routing fix | Already a dependency of `vox-orchestrator` — `[VERIFIED: crates/vox-orchestrator/Cargo.toml:85]` `vox-crypto.workspace = true`. No crate-edge exception needed. |
| `blake3` | workspace (already a `vox-crypto` dependency) | Underlying MAC primitive, reached only through the facade after D-04 | AGENTS.md §Cryptography Policy #1 — `vox-crypto` is the only crate allowed to import `blake3` directly. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Lazy-sweep on `try_acquire`/`is_locked` (D-06, locked) | Background tokio task ticking every N seconds | Explicitly rejected by D-06 — stays in-process, self-driving, no new task lifecycle to manage/shut down. Revisit only if locks are observed sitting expired-and-uncontended (see Deferred Ideas). |
| In-process `ResourceLockManager` (D-07, locked) | DB-backed distributed lock mirroring `FileLockManager`'s `locks/lease.rs` | Explicitly deferred — cross-node reach is a distinct future phase. |

**Installation:** None — everything needed is already a workspace dependency.

## Package Legitimacy Audit

Not applicable — this phase adds no new external package to any ecosystem. All work reroutes or wires existing in-workspace crates (`vox-orchestrator`, `vox-orchestrator-queue`, `vox-orchestrator-mcp`, `vox-crypto`, `vox-mcp-registry`, `vox-gui`).

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                    ┌─────────────────────────────────────────────┐
                    │   vox-orchestrator-mcp/src/dispatch.rs       │
                    │   handle_tool_call_with_mode(name, args)     │
                    │                                               │
  MCP tool call ───▶│  [D-01] issue_tool_receipt(agent, name,args) │
  (chat, stdio,     │       ↓                                      │
   HTTP gateway)    │   handle_tool_call_inner(...)  ── dispatch   │
                    │       ↓                                      │
                    │  [D-01] fulfill_tool_receipt(id, result)     │──▶ ToolReceiptLedger
                    │  [new]  bulletin.publish / event_bus.emit    │    (in-process, session-
                    │         (tool_name, receipt_id, verified,    │     keyed HMAC)
                    │          agent_id, session_id)                │
                    └───────────────────┬───────────────────────────┘
                                         │
                    ┌────────────────────▼──────────────────────────┐
                    │  vox_verify_task_claims  [D-03, new MCP tool] │──▶ validate_agent_claims()
                    │  (agent-callable, read-only)                  │    valid/fabricated/unverified
                    └────────────────────────────────────────────────┘

                    ┌─────────────────────────────────────────────┐
                    │  vox-orchestrator::orchestrator::dispatch     │
                    │  run_dispatcher_with_oplog (hopper→agent loop)│
                    │                                               │
  HopperItemAdmitted│  intake_to_task(item) → AgentTask             │
  (event bus) ─────▶│  enqueue(task) → Some(agent_id)               │
                    │  [D-08] if resource_id: acquire_resource_lock │──▶ ResourceLockManager
                    │  hopper.assign(item_id, agent_id)             │    (lazy sweep on
                    └───────────────────┬───────────────────────────┘     try_acquire/is_locked,
                                         │                                  D-06)
                    ┌────────────────────▼──────────────────────────┐
                    │  task_dispatch/complete/{success,fail}         │
                    │  [D-08] if resource_id: release_resource_lock  │
                    └────────────────────────────────────────────────┘

                    ┌─────────────────────────────────────────────┐
                    │  bulletin (AgentMessage) + event_bus          │
                    │  (AgentEventKind) — ALREADY wired for         │
                    │  LockAcquired/LockReleased                    │
                    └───────────────────┬───────────────────────────┘
                                         │ (existing: activity/sink.rs
                                         │  drains event_bus → activity_log)
                    ┌────────────────────▼──────────────────────────┐
                    │  vox-gui Tauri command: activity_query         │──▶ ActivitySurface (existing
                    │  (crates/vox-gui/src/commands/activity.rs)     │     panel — NOT the chat
                    └────────────────────────────────────────────────┘     transcript)

                    ┌─────────────────────────────────────────────┐
                    │  vox-orchestrator-mcp/src/chat_tools/chat/    │
                    │  agent_loop.rs: turn_event_for_result(...)    │──▶ AgentTurnOutcome.events
                    │  [D-10, extend] add receipt/lock-status       │     (Vec<serde_json::Value>)
                    │  kinds alongside existing skill/delegation/    │
                    │  research kinds                                │
                    └───────────────────┬───────────────────────────┘
                                         │ (existing: TurnEventDto over Tauri IPC)
                    ┌────────────────────▼──────────────────────────┐
                    │  ChatTranscript.tsx → ChatTurnEventRow.tsx     │  <- new `kind` cases render
                    │  (existing, additive-by-design component)      │     receipt/lock chips here
                    └────────────────────────────────────────────────┘
```

### Component Responsibilities

| File | Responsibility | Status this phase |
|------|-----------------|--------------------|
| `crates/vox-orchestrator/src/tool_receipt.rs` | `ToolReceiptLedger`: issue/fulfill/verify/validate_agent_claims | Existing, fully implemented — only D-04 (crypto routing) touches it |
| `crates/vox-orchestrator/src/orchestrator/safety.rs` | `Orchestrator` wrapper methods (`issue_tool_receipt`, `fulfill_tool_receipt`, `verify_tool_receipt`, `acquire_resource_lock`, `release_resource_lock`) | Existing — D-01/D-08 call these, do not reimplement |
| `crates/vox-orchestrator-queue/src/locks/resource.rs` | `ResourceLockManager` | Existing — D-06 adds the sweep here |
| `crates/vox-orchestrator-mcp/src/dispatch.rs` | `handle_tool_call_with_mode` / `handle_tool_call_inner` | D-01's hook point |
| `crates/vox-orchestrator/src/orchestrator/dispatch.rs` | `run_dispatcher_with_oplog` (hopper→agent admit/enqueue/assign loop) | D-08's acquire hook point |
| `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/{success/mod.rs,fail.rs}` | Task completion / failure handlers | D-08's release hook point |
| `crates/vox-orchestrator/src/hopper/types.rs` | `IntakeItem` | D-08 adds `resource_id: Option<String>` here |
| `crates/vox-orchestrator/src/types/tasks.rs` | `AgentTask` | D-08 must propagate `resource_id` here too (see Pitfall 3) so completion/fail handlers can read it |
| `contracts/operations/catalog.v1.yaml` → `contracts/mcp/tool-registry.canonical.yaml` → `vox-mcp-registry::TOOL_REGISTRY` | Generated MCP tool SSOT chain | D-03's new tool MUST be added at the top of this chain, not just as a Rust match arm (see Pitfall 5) |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` | `turn_event_for_result` | D-10 extends this (or adds a sibling emission) with receipt/lock event kinds |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx` | Renders one turn event | D-10 adds new `if (event.kind === '...')` branches |
| `crates/vox-gui/ui/src/types/dashboard.ts` | `TurnEventDto` (open `kind: string` shape) | No schema change needed — already forward-compatible |

### Pattern 1: D-01 — wrap `handle_tool_call_with_mode` with issue/fulfill

**What:** Issue a receipt intent before dispatching, fulfill it with the result hash after.
**When to use:** Every `vox_*` tool call routed through `handle_tool_call_with_mode`.
**Verified call boundary** `[VERIFIED: crates/vox-orchestrator-mcp/src/dispatch.rs:353-384]`:
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
`agent_id`/`session_id` are already extracted at the top of the function `[VERIFIED: crates/vox-orchestrator-mcp/src/dispatch.rs:62-63]`:
```rust
let agent_id = args.get("agent_id").and_then(|v| v.as_str());
let session_id = args.get("session_id").and_then(|v| v.as_str());
```
Existing convention for the "no agent_id present" case (interactive/CLI calls) `[VERIFIED: crates/vox-orchestrator-mcp/src/dispatch.rs:514]`: `let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0u64);` — D-01 should follow this same default-to-`AgentId(0)` convention rather than skipping receipt issuance when `agent_id` is absent, since D-02 already establishes fail-open semantics and the ledger is a detection trail, not a gate.

The `Orchestrator` wrapper methods to call are already implemented `[VERIFIED: crates/vox-orchestrator/src/orchestrator/safety.rs:11-27]`:
```rust
pub fn issue_tool_receipt(&self, agent_id: AgentId, tool_name: &str, args_json: &str)
    -> Result<String, crate::tool_receipt::ToolReceiptError> { ... }
pub fn fulfill_tool_receipt(&self, receipt_id: &str, result_json: &str) -> bool { ... }
pub fn verify_tool_receipt(&self, receipt_id: &str) -> bool { ... }
```
`args_json`/`result_json` for these calls: `args.to_string()` (already computed once at line 122 for the TOESTUB check — reuse rather than re-stringify) and the dispatch `result` (`Ok(payload)`/`Err(e).to_string()`) respectively.

**D-02 fail-open implementation shape:** `issue_tool_receipt` returns `Result<String, ToolReceiptError>`. On `Err`, `tracing::warn!` and proceed to call `handle_tool_call_inner` exactly as today (no `receipt_id` to fulfill afterward — skip the `fulfill_tool_receipt` call in that branch, it's a no-op on a nonexistent id anyway per `fulfill_intent`'s `ok_or("Receipt not found")` path `[VERIFIED: crates/vox-orchestrator/src/tool_receipt.rs:146-148]`).

### Pattern 2: D-06 — lazy sweep on `try_acquire`/`is_locked`

**What:** Purge all expired entries from the lock map as a side effect of every `try_acquire`/`is_locked` call, not just the contended resource_id.
**Current state** `[VERIFIED: crates/vox-orchestrator-queue/src/locks/resource.rs:38-108]` — `try_acquire` only checks the *one* `resource_id` key against `now`; expired entries for *other* resource_ids are never removed from the `HashMap`, so `len()`/`snapshot()` over-report until something else happens to touch that specific key.
**Precedent to model the sweep signature on** (not to replicate the DB dependency) `[VERIFIED: crates/vox-orchestrator-queue/src/locks/refresh.rs:11]`: `pub fn force_release_stale(&self, timeout_ms: u128) -> usize` — same "purge, return count" shape, but keyed on elapsed time since acquisition rather than an explicit `expires_ms` field (ResourceLock already carries `expires_ms`, so the sweep is simpler: `locks.retain(|_, l| l.expires_ms > now)`).
**Example:**
```rust
// Source: pattern derived from FileLockManager::force_release_stale (locks/refresh.rs:11)
// applied to ResourceLockManager's expires_ms field (resource.rs:23).
fn sweep_expired(locks: &mut HashMap<String, ResourceLock>, now: u64) {
    locks.retain(|_, lock| lock.expires_ms > now);
}
```
Call `sweep_expired` at the top of `try_acquire` and `is_locked` (both already compute `now` locally — reuse it). D-06 marks whether `release` also sweeps as Claude's Discretion; given `release` already does a targeted single-key removal, adding a full sweep there is low-cost and keeps `len()` accurate sooner, but is not required by the locked decision.
**Existing test suite to extend, not duplicate:** `crates/vox-orchestrator-queue/src/semcov_wave19_tests.rs:14-168` (`mod resource_lock`) already has `is_locked_returns_false_after_expiry` and `expired_lock_allows_new_holder` — add a sweep-specific test asserting `len()`/`snapshot()` drop the expired *other* resource, not just the contended one.

### Pattern 3: D-08 — hopper task dispatch as the real lock caller

**What:** Thread `resource_id: Option<String>` from intake through to the two acquire/release call sites.
**Current shape** `[VERIFIED: crates/vox-orchestrator/src/hopper/types.rs:120-152]` — `IntakeItem` has no `resource_id` field; add one following the existing `Option<String>` pattern already used for `session_id` (line 131: `pub session_id: Option<String>,`).
**Real production dispatch site** (not `hopper.assign()` alone — the whole admit→enqueue→assign loop) `[VERIFIED: crates/vox-orchestrator/src/orchestrator/dispatch.rs:39-133]`:
```rust
let task = intake_to_task(&item);
if let Some(agent_id) = enqueue(task) {
    if hopper.assign(&item.item_id, agent_id.to_string()).await.is_ok() {
        /* record HopperAssign oplog entry */
    }
}
```
`intake_to_task` `[VERIFIED: crates/vox-orchestrator/src/orchestrator/dispatch.rs:18-26]` builds an `AgentTask` from the `IntakeItem` — this is the natural place to copy `item.resource_id.clone()` onto the task, since `AgentTask` currently has no such field `[VERIFIED: crates/vox-orchestrator/src/types/tasks.rs:559-589 — fields enumerated: id, description, priority, status, file_manifest, victory_condition, depends_on, estimated_complexity, model_preference, model_override, mode, task_category, test_decision; no resource_id]`. Propagating it onto `AgentTask` (rather than keeping a side lookaside map keyed by `TaskId`) means the completion/failure handlers — which operate on `AgentTask`/`TaskId`, not `IntakeItem` — can read `resource_id` directly without a second data structure.
**Acquire call site:** inside the closure/site that constructs `enqueue` (has `Arc<Orchestrator>` in scope; `run_dispatcher_with_oplog` itself only receives a generic `enqueue: impl Fn(AgentTask) -> Option<AgentId>` closure, so the lock acquire belongs either inside that closure at its construction site, or immediately after `enqueue(task)` returns `Some(agent_id)` if `run_dispatcher_with_oplog` is given direct orchestrator access for this purpose — confirm the exact closure construction site during planning, it is not shown in the excerpt above).
**Release call sites** (both required per D-08's "completion or failure"): `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/mod.rs` (`complete_task`/`complete_task_with_audit`/`complete_task_with_attestation` — `[VERIFIED: crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/mod.rs:69,73,101 — fn signatures]`) and `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/fail.rs` (file exists, confirmed present in the `complete/` directory listing; exact fn name to be confirmed during planning).
**Example:**
```rust
// Source: pattern derived from orchestrator/safety.rs's existing acquire_resource_lock
// wrapper (crates/vox-orchestrator/src/orchestrator/safety.rs:71-99), called from the
// hopper dispatch loop instead of a new call site inside ResourceLockManager directly.
if let Some(resource_id) = &item.resource_id {
    let acquired = orchestrator.acquire_resource_lock(
        agent_id,
        resource_id,
        vox_orchestrator::locks::ResourceLockKind::Exclusive,
        /* ttl_ms */ 300_000, // discretion: pick a TTL appropriate to task duration
    );
    if !acquired {
        // contended: block/error per D-08 — exact error-vs-retry policy is
        // Claude's Discretion; ResourceLockManager::try_acquire already returns
        // an Err(String) naming the holder (resource.rs:55-58) — surface that.
    }
}
```

### Pattern 4: D-03 — new `vox_verify_task_claims` MCP tool

**What:** A new read-only MCP tool exposing `ToolReceiptLedger::validate_agent_claims`.
**Existing logic to expose** (already fully implemented, no new logic needed) `[VERIFIED: crates/vox-orchestrator/src/tool_receipt.rs:204-214]`:
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
No `Orchestrator`-level wrapper exists yet for `validate_agent_claims` (only `issue_tool_receipt`/`fulfill_tool_receipt`/`verify_tool_receipt` are wrapped in `safety.rs`) — add one following the same pattern.
**Where NOT to add the tool function:** `crates/vox-orchestrator-mcp/src/trust_tools.rs` looks like a natural home by name but is unrelated — it wraps a different subsystem, DB-backed "trust rollup" observations `[VERIFIED: crates/vox-orchestrator-mcp/src/trust_tools.rs:1,13-18 — "Trust rollup inspection tools over connected VoxDb"; require_db() gates on state.db]`. `vox_verify_task_claims` needs no DB — it's a pure in-process ledger read. A new small file (e.g. `receipt_tools.rs`) or an addition to `agent_tools.rs` is more appropriate; exact placement is Claude's Discretion per CONTEXT.md.
**Registration is a THREE-HOP generated chain — do not add only a Rust match arm.** `TOOL_REGISTRY` is build-script-generated `[VERIFIED: crates/vox-mcp-registry/src/lib.rs:1-15]`:
```rust
//! MCP tool registry built from `contracts/mcp/tool-registry.canonical.yaml`.
...
include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));
```
And that YAML file is itself generated `[VERIFIED: contracts/mcp/tool-registry.canonical.yaml:1-2]`:
```
# Canonical MCP tool names + descriptions (generated).
# GENERATED FROM contracts/operations/catalog.v1.yaml via `vox ci operations-sync --target mcp --write`.
```
So the true SSOT is `contracts/operations/catalog.v1.yaml` (entry shape verified, e.g. the `a2a.ack` entry `[VERIFIED: contracts/operations/catalog.v1.yaml:281-284]`: `name: vox_a2a_ack`, `http_read_role_eligible: false`, `tier: core`, `cli: null`, plus a sibling entry with `id`, `title`, `description`, `product_lane`, etc.). **Skipping this chain means `issue_intent`'s fail-closed `TOOL_REGISTRY` check (`tool_receipt.rs:108`) will reject the new tool's own receipts**, and dispatch.rs's match table needs a `"vox_verify_task_claims" => ...` arm added alongside the existing ones (pattern at `[VERIFIED: crates/vox-orchestrator-mcp/src/dispatch.rs:990-993]`, e.g. `"vox_db_trust_rollups" => Ok(trust_tools::trust_rollups_list(state, args).await),`).
**Sequence:** (1) add entry to `contracts/operations/catalog.v1.yaml`, (2) run `vox ci operations-sync --target mcp --write` to regenerate `tool-registry.canonical.yaml` (regenerates `TOOL_REGISTRY` via the build script on next build), (3) add the Rust function + dispatch.rs match arm, (4) run `vox ci command-sync --write` if a CLI-visible surface is also wanted (optional — this tool is agent-facing only, `cli: null` is fine per the a2a.ack precedent).

### Pattern 5: D-04 — crypto-policy fix in `tool_receipt.rs`

**What:** Replace three inline `blake3::Hasher::new_keyed(&self.session_key)` sequences with `vox_crypto::facades::keyed_hash`.
**Confirmed equivalence** `[VERIFIED: crates/vox-crypto/src/facades.rs:17-22]`:
```rust
pub fn keyed_hash(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(key);
    hasher.update(data);
    hasher.finalize().into()
}
```
This is the exact same construction `tool_receipt.rs` uses today (`Hasher::new_keyed` + `update` + `finalize`) — `[VERIFIED: crates/vox-orchestrator/src/tool_receipt.rs:118-124,154-161,185-193]`. **Critical detail for byte-identical output:** `tool_receipt.rs` calls `.update()` multiple times in sequence (receipt_id, agent_id LE bytes, tool_name, args_hash, [result_hash], executed_at_ms LE bytes) whereas `keyed_hash` takes one `&[u8]` slice. BLAKE3's streaming API is order/framing-equivalent to hashing the concatenation of all `update()` calls (no length-prefixing between calls in the underlying construction), so D-04's fix must build ONE `Vec<u8>` by concatenating the exact same fields in the exact same order/byte-encoding, then call `keyed_hash(&self.session_key, &buf)` once — a fresh field ordering or an added separator byte would silently change every receipt's HMAC tag and break `verify()` against receipts issued before the change (harmless in this phase since the ledger is in-memory and session-scoped, but must still match within a single running session's issue→verify pair). Three call sites to fix: `issue_intent` (`tool_receipt.rs:118-124`), `fulfill_intent` (`:154-161`), `verify` (`:185-193`) — all three must build the identical concatenation logic (extract to one private helper to guarantee this, rather than duplicating the byte-ordering three times).
**Existing tests that will catch a mismatch:** `crates/vox-orchestrator/src/tool_receipt.rs:225-263` (`issue_intent_accepts_registered_tool`, `issue_accepts_registered_tool_and_binds_result` — both assert `l.verify(&r.receipt_id).is_ok()`) already exercise the full issue→verify round trip and will fail if concatenation order drifts between the issue and verify call sites.

### Pattern 6: D-10 — chat-surface event plumbing for receipts and locks

**What:** Surface tool-receipt status and resource-lock wait/hold state inline in the chat transcript.
**Two existing, independent plumbing paths were found — do not conflate them:**

1. **Synchronous, chat-turn-scoped** (`turn_event_for_result`) — fires once per dispatched tool call, inside the same request that produced the chat reply. `[VERIFIED: crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:276-327]` is a `match tool_name { "vox_skill_use" => ..., "vox_spawn_agent" | "vox_submit_task" => ..., "vox_deep_research" | ... => ..., _ => None }` that returns an open-shaped `serde_json::Value` with a `"kind"` field. Call site `[VERIFIED: crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:887-910]`:
   ```rust
   let result = crate::dispatch::handle_tool_call_with_mode(state, &call.name, dispatch_args, permission_mode).await;
   let dispatch_ok = result.is_ok();
   let content = match result { Ok(s) => s, Err(e) => format!("Error: {e}") };
   let call_succeeded = dispatch_ok && !content.starts_with("Error:") && !crate::server_state::tool_json_envelope_is_error(&content);
   if let Some(ev) = turn_event_for_result(&call.name, &call.arguments, &content, call_succeeded) {
       events.push(ev);
   }
   ```
   `events: Vec<serde_json::Value>` is a field on `AgentTurnOutcome` (`[VERIFIED: crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:199-203]`) that already flows to the frontend as `TurnEventDto[]` — the TS type is deliberately open (`[VERIFIED: crates/vox-gui/ui/src/types/dashboard.ts:58-64]`: `kind: string; [key: string]: unknown;` with a comment: "unrecognized future `kind` renders as a no-op chip instead of a crash"). The render component already exists and already handles this exact contract, defaulting unknown kinds to `null` (no render) `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx:22-44]`. Wired into the transcript at `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx:8,89]`: `import { ChatTurnEventRow } from './ChatTurnEventRow';` / `<ChatTurnEventRow key={i} event={ev} onExcludeSkill={onExcludeSkill} />`.

   **The problem:** `turn_event_for_result` only has `call.name`/`call.arguments`/`content`/`call_succeeded` in scope — it has no receipt_id, because D-01's issue/fulfill wrap lives inside `handle_tool_call_with_mode` (dispatch.rs), one call frame away, and `handle_tool_call_with_mode`'s return type (`Result<String, anyhow::Error>`) carries no receipt metadata back to this call site. Changing that return type would ripple into every caller of `handle_tool_call_with_mode`/`handle_tool_call` (stdio MCP server, HTTP gateway, tests) — larger than this phase's scope.

2. **Asynchronous, event-bus-scoped** (bulletin + `event_bus` → `activity_log` → `activity_query`) — this is the literal mechanism named in D-10's own text ("bulletin → event bus → GUI transport") and is **already fully wired for resource locks**: `acquire_resource_lock`/`release_resource_lock` publish both a bulletin `AgentMessage::ResourceLockAcquired/Released` and an `AgentEventKind::LockAcquired/Released` `[VERIFIED: crates/vox-orchestrator/src/orchestrator/safety.rs:70-115]`. The event bus is drained by a sink that persists "loggable" kinds to the `activity_log` table `[VERIFIED: crates/vox-orchestrator/src/activity/sink.rs:8-26, crates/vox-orchestrator/src/activity/mod.rs:12-38]` — `LockAcquired`/`LockReleased` are already in the loggable allowlist (`activity/mod.rs:26-27`). The GUI reads this table via the `activity_query` Tauri command `[VERIFIED: crates/vox-gui/src/commands/activity.rs:66-110]`, filterable by `agent_id`/`kind`/`before_id` — **but not currently by `session_id`**, even though the DB row has one, because the `ActivityRow` projection for `LockAcquired`/`LockReleased` always sets `session_id: None` `[VERIFIED: crates/vox-orchestrator/src/activity/project.rs:149-164]`. This is the gap to close for D-10: threading a real `session_id` through `acquire_resource_lock`/`release_resource_lock` (today's signature takes `agent_id`/`resource_id`/`kind`/`ttl_ms` only — no `session_id` param `[VERIFIED: crates/vox-orchestrator/src/orchestrator/safety.rs:71-77]`) so lock events can be correlated to the chat session that triggered them, and adding `ActivityFilter.session_id` to `activity.rs`'s filter DTO for the GUI to query "activity for this chat session."
   **Important:** this is the `ActivitySurface` panel's data source (`crates/vox-gui/ui/src/components/surfaces/Activity/ActivitySurface.tsx`), a *separate* surface from the chat transcript — confirming this path alone does not satisfy "visible in the chat surface" without either (a) the Chat surface polling `activity_query` scoped to its own session_id and rendering inline, or (b) reusing path 1 above.

**Recommendation for planning (not yet implemented — flag as a design decision, not a locked fact):** Use path 1 (`turn_event_for_result`-adjacent) for the chat-transcript receipt/lock chips, since it is exactly the mechanism the component contract already supports and requires no new polling loop in the frontend. To close the "receipt_id doesn't reach agent_loop.rs" gap without changing `handle_tool_call_with_mode`'s signature: have D-01's wrap **also** call `state.orchestrator.event_bus().emit(...)` (a new small `AgentEventKind` variant, e.g. `ToolReceiptRecorded { agent_id, session_id, tool_name, receipt_id, verified }`) at the point inside dispatch.rs where `agent_id`/`session_id`/`name_canonical` are already in scope, mirroring the exact pattern `acquire_resource_lock` already uses (bulletin+event_bus, `safety.rs:70-115`) — then have `agent_loop.rs`, immediately after each `handle_tool_call_with_mode` call, **subscribe or query the ledger by (agent_id, tool_name, most-recent)** rather than relying on the return value. A simpler alternative worth considering during planning: since `agent_loop.rs` already has `state` (== `ServerState`, which carries `orchestrator: Arc<Orchestrator>`), it could call `state.orchestrator.issue_tool_receipt`/`fulfill_tool_receipt` itself, redundantly with D-01's dispatch.rs-level wrap, purely to obtain the `receipt_id` for the turn event — but this double-issues receipts for chat-originated calls (one from D-01's wrap, one from this local call) unless the ledger's `issue_intent`/`fulfill_intent` are made idempotent per (agent_id, tool_name, args_hash, timestamp), which they are not today. **This double-issue risk is exactly why the event-bus-emission approach (previous paragraph) is the safer default** — it reads the *one* receipt D-01 already created rather than creating a second one. This tradeoff should be confirmed with the user or resolved by the planner as a concrete task, not deferred to execution time.

For **resource-lock wait/hold state** in chat (the other half of D-10), the same event-bus path applies once `session_id` is threaded through `acquire_resource_lock`/`release_resource_lock` (see above) — `turn_event_for_result` has no natural call site for lock state (locks aren't tied to a single tool call's result, they're tied to task-level dispatch, see Pattern 3), so this one is a stronger fit for path 2: the Chat surface (or a lightweight rail component within it, e.g. `ChatExecutionRail.tsx`, which already exists as a chat-adjacent status area — `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx exists per directory listing]`, not yet read for its exact contract) polling `activity_query` scoped to the active `session_id` and filtering `kind: LockAcquired|LockReleased`.

### Recommended Project Structure

No new top-level module/crate is warranted — every change is a small addition to an existing file, except:
- One new small Rust file for the D-03 tool function (or an addition to `agent_tools.rs` — Claude's Discretion).
- One new `AgentEventKind` variant (in `crates/vox-orchestrator/src/events.rs`, alongside `LockAcquired`/`LockReleased`) if the event-bus-emission approach in Pattern 6 is taken.
- New Playwright spec(s) under `crates/vox-gui/ui/e2e/` for D-11 (see §Validation Architecture).

### Anti-Patterns to Avoid
- **Adding a per-tool allowlist for receipt issuance** — D-01 explicitly rejects this; wrap uniformly.
- **Making `issue_intent` failure block tool execution** — D-02 is fail-open; this is an audit trail, not a gate, this phase.
- **Reusing `AgentEventKind::LockAcquired`/`LockReleased`'s `path: PathBuf` field for the generic `resource_id` string** — the *existing* code already does this (`orchestrator/safety.rs:92`: `path: std::path::PathBuf::from(resource_id)`), which is a slightly awkward reuse of a file-lock-shaped event for a generic string id; do not extend this same reuse to the new tool-receipt event — give it its own properly-typed variant with a `String` field, not a `PathBuf`.
- **Confusing the two existing "tool call dispatched" surfaces.** `AgentEventKind::ToolCallDispatched { task_id, agent_id, tool_name, ok }` already exists `[VERIFIED: crates/vox-orchestrator/src/events.rs:231-240]` and is already emitted `[VERIFIED: crates/vox-orchestrator/src/runtime.rs:1038-1043]` — but from a *different*, autonomous "bounded executor" `@tool <name>` intent-line-parsing path in `runtime.rs`, not from the live `vox-orchestrator-mcp/src/dispatch.rs::handle_tool_call_with_mode` path this phase's D-01 targets. It is also Tier B (never durably journaled, `[VERIFIED: crates/vox-orchestrator/src/events.rs:901,943]`) and not in `activity::is_loggable`'s allowlist, so it doesn't reach `activity_log`/the GUI today either. Do not assume wiring this existing event kind satisfies D-01/D-10 — it covers a different dispatch mechanism.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HMAC tool-call receipts | A new receipt/audit struct | `crate::tool_receipt::ToolReceiptLedger` (already exists, tested) | Duplicating this is the exact anti-pattern Phase 2's research already flagged as deferred work for this phase. |
| Resource contention locking | A new lock map | `crate::locks::ResourceLockManager` (already exists, tested, 9 passing tests) | Same reasoning — ADR-025 already specifies this exact shape. |
| Keyed hashing / HMAC | Direct `blake3`/`sha3` import outside `vox-crypto` | `vox_crypto::facades::keyed_hash` | AGENTS.md §Cryptography Policy #1 (hard rule); `vox-code-audit` detector `vox/crypto/banned-crate-import` exists for this class but doesn't currently catch bare `blake3` outside `vox-crypto` per CONTEXT.md's scout note — don't rely on the detector to catch a new violation, fix D-04's existing one and don't introduce more. |
| Stale-lock cleanup | A new timer/scheduler abstraction | The lazy-sweep pattern already used by this exact subsystem's sibling (`FileLockManager::force_release_stale`) | D-06 explicitly chose this shape; a background task is more moving parts for no locked benefit this phase. |

**Key insight:** This phase's backend work is almost entirely "delete zero, add wiring" — every hand-rolled temptation here already has a tested sibling implementation in the same crate family.

## Common Pitfalls

### Pitfall 1: Wrapping the wrong call boundary in dispatch.rs
**What goes wrong:** Wrapping `handle_tool_call_with_mode`'s *entire* body (including the ~250 lines of approval/scope/skill/guardrail gates before the actual dispatch) instead of just the `handle_tool_call_inner` call, so a receipt gets issued for calls that were rejected before ever executing (e.g. an unapproved dangerous-tool call that timed out waiting for HITL approval, line 250-297).
**Why it happens:** The function is long (~2200 lines including the match table) and easy to wrap at the outer boundary by reflex.
**How to avoid:** Issue/fulfill exactly around the `te.run(...)` block that calls `handle_tool_call_inner` (dispatch.rs:353-384), per D-01's own wording ("before `handle_tool_call_inner` runs... after it returns").
**Warning signs:** Receipts issued for tool calls that never actually executed (e.g. rejected by scope_guard, lock_guard, skill_permissions, or the HITL approval timeout) — these should never reach the ledger.

### Pitfall 2: D-06's sweep changing lock semantics under concurrent access
**What goes wrong:** `locks.retain(...)` requires a write lock; `is_locked` currently only takes a read lock `[VERIFIED: crates/vox-orchestrator-queue/src/locks/resource.rs:96-108]`. Adding a sweep to `is_locked` upgrades it to a write-lock operation, which changes its concurrency profile (readers now briefly block each other) and could reintroduce a lock-ordering hazard if `is_locked` is ever called while another write lock on the same `RwLock` is held elsewhere in the same call stack.
**Why it happens:** The lazy-sweep decision (D-06) is correct in principle but changes a previously read-only method into a write method.
**How to avoid:** Use `std::sync::RwLock` (already the type in use, not `parking_lot`) — confirm no call site holds the write lock across an `.await` or re-enters `is_locked` while already holding the manager's own lock. The existing tests already exercise `is_locked` immediately after `try_acquire` in the same thread (no async boundary), so this is a low but non-zero risk to verify with a targeted test during planning.
**Warning signs:** A deadlock or panic-on-poison surfacing only under concurrent multi-agent load, not in the existing single-threaded unit tests.

### Pitfall 3: D-08 — `resource_id` added to `IntakeItem` but not propagated to `AgentTask`
**What goes wrong:** The lock is acquired at admit/enqueue time (where `IntakeItem` is in scope) but never released, because the completion/failure handlers operate on `AgentTask`/`TaskId`, which doesn't carry `resource_id` unless it's explicitly copied over in `intake_to_task` (dispatch.rs:18-26).
**Why it happens:** `IntakeItem` and `AgentTask` are two different structs bridged by one pure function; it's easy to add a field to one and forget the other.
**How to avoid:** Add `resource_id` to *both* `IntakeItem` (hopper/types.rs) and `AgentTask` (types/tasks.rs), and copy it explicitly in `intake_to_task`. Write a test asserting the round-trip (`intake_to_task(&item_with_resource_id).resource_id == item_with_resource_id.resource_id`) before wiring acquire/release.
**Warning signs:** Locks that are acquired but never appear released in `ResourceLockManager::snapshot()` after a task completes — a slow leak of "stuck" locks until their TTL expires.

### Pitfall 4: Forgetting the failure path for lock release
**What goes wrong:** Only wiring release into the success handler (`complete/success/mod.rs`), leaving a lock held for its full TTL whenever a task fails.
**Why it happens:** The success path is more discoverable (it's the "happy path" most people wire first); `complete/fail.rs` is a separate file.
**How to avoid:** D-08's own text says "releases on completion or failure" — treat this as two required call sites, not one. `complete/fail.rs` exists as a confirmed separate file in the same directory tree `[VERIFIED: crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/ directory listing includes fail.rs]`.
**Warning signs:** A test that only exercises the success path will not catch this — write a dedicated failure-path release test.

### Pitfall 5: Registering the D-03 tool as only a Rust match arm
**What goes wrong:** Adding `"vox_verify_task_claims" => ...` to dispatch.rs's match table without adding the corresponding entry to `contracts/operations/catalog.v1.yaml`. The tool dispatches fine when called directly, but (a) `issue_intent`'s fail-closed `TOOL_REGISTRY` check rejects receipts for the tool's *own* calls (a `vox_verify_task_claims` call would itself get a `warn`-logged unknown-tool receipt failure per D-02), and (b) any SSOT-drift CI gate (`vox ci ssot-drift`) that cross-checks the dispatch table against the generated registry will flag it.
**Why it happens:** The generated chain (`catalog.v1.yaml` → `tool-registry.canonical.yaml` → `TOOL_REGISTRY` via build script) is three hops from the Rust code that actually needs the entry, and none of those hops are visible from `dispatch.rs` itself.
**How to avoid:** See Pattern 4's registration sequence. Run `vox ci operations-sync --target mcp --write` and rebuild before considering the tool "done."
**Warning signs:** `vox_verify_task_claims` calls fail closed with `warn`-level "Unknown tool" log lines despite the match arm existing and working.

## Code Examples

See Patterns 1-6 above for verified, cited code excerpts at each of the five wiring points (D-01, D-03, D-04, D-06, D-08) plus the D-10 design recommendation.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `vox-mcp-meta`'s hardcoded `A2A_MESSAGE_TYPES` constants | `vox-mcp-registry::TOOL_REGISTRY` generated from `contracts/operations/catalog.v1.yaml` | Phase 2 (2026-09-25), `[VERIFIED: .planning/ROADMAP.md:56,61]` | The tool-name SSOT is now a generated YAML chain, not a hand-maintained Rust list — D-03 must follow this chain (Pitfall 5), not the pre-Phase-2 pattern. |

**Deprecated/outdated:** None specific to this phase's domain beyond the above.

## Runtime State Inventory

Not applicable — this is not a rename/refactor/migration phase. No strings are being renamed; no stored data, live service config, OS-registered state, secrets, or build artifacts carry a name that this phase changes.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The exact closure-construction site for `run_dispatcher_with_oplog`'s `enqueue` parameter (where `Arc<Orchestrator>` is available for the D-08 acquire call) was not located this session — only the loop body that calls it (`orchestrator/dispatch.rs:39-133`) was read. | Pattern 3 | Planner must locate the actual `Orchestrator::new`/`core/mod.rs` wiring site that constructs this closure before writing the D-08 acquire-call task; if the closure has no orchestrator handle, the acquire call may need to move to a different point in the loop (e.g., directly in `run_dispatcher_with_oplog` if it's given orchestrator access, or the closure needs to capture it). |
| A2 | `complete/fail.rs`'s exact function name/signature for the task-failure handler was not read this session — only its existence in the directory listing was confirmed. | Pattern 3, Pitfall 4 | Planner must open this file to confirm the correct hook point for D-08's failure-path release before writing that task. |
| A3 | `ChatExecutionRail.tsx`'s exact contract (whether it already polls anything, and whether it's the right home for lock-wait-state UI) was not read this session — only its existence and chat-adjacency were confirmed via directory listing. | Pattern 6 | Planner should read this file before deciding whether to extend it vs. add a new small component for lock wait/hold chips. |
| A4 | The double-issue-receipt risk described in Pattern 6 (if agent_loop.rs independently calls issue/fulfill) is reasoned from `issue_intent`'s non-idempotent signature (`tool_receipt.rs:102-139`, always mints a fresh UUIDv7) rather than from an explicit locked decision against it — this is architectural reasoning, not a verified prohibition. | Pattern 6 | If the planner instead prefers the double-issue approach for simplicity, it should be an explicit, acknowledged tradeoff (extra ledger entries per chat-dispatched call), not an accidental side effect. |
| A5 | Whether any CI gate (`vox ci ssot-drift` or similar) actually cross-checks dispatch.rs's match table against `TOOL_REGISTRY` (asserted in Pitfall 5) was not directly verified — reasoned from the existence of `vox ci ssot-drift` as a general SSOT-drift gate named in AGENTS.md, not from reading its specific rule set. | Pitfall 5 | Low risk either way — the registration sequence in Pattern 4 is correct regardless of whether a CI gate catches a skipped step; worst case is the gate doesn't exist and the only symptom is the fail-closed `TOOL_REGISTRY` rejection already described. |

## Open Questions

1. **D-08 lock TTL value**
   - What we know: `ResourceLockManager::try_acquire` requires an explicit `ttl_ms` (mandatory per ADR-025's "lease-based expiration" decision); no default exists in the codebase to inherit.
   - What's unclear: What TTL is appropriate for a task-scoped lock — too short risks the lock expiring mid-task (letting a second agent race in); too long risks a crashed agent holding a lock long after its task died.
   - Recommendation: Either derive it from an existing task-timeout config value (search for one during planning) or pick a conservative default (e.g. 10-30 minutes) with a comment flagging it as a tunable, not a considered value — this is genuinely a product decision, not something research can resolve from code alone.

2. **D-10's receipt-to-chat-event bridging mechanism**
   - What we know: Two independent, already-existing plumbing paths (Pattern 6); the return-value approach is out of scope (too invasive) and the double-issue approach has a real (if minor) correctness cost.
   - What's unclear: Whether the user has a preference between "new AgentEventKind + query in agent_loop.rs after dispatch" vs. some other shape not yet considered.
   - Recommendation: Confirm the event-bus-emission approach (Pattern 6, recommended) with the user/planner before committing tasks to it, since D-10 itself only sketches the destination (chat surface, bulletin→event bus→GUI transport) and not this specific bridging detail.

## Environment Availability

Not applicable — no new external tool, service, runtime, or CLI dependency is introduced by this phase. All work is within the existing Rust workspace and the existing `pnpm`/Vite/Playwright toolchain already used by `vox-gui`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Rust framework | `cargo nextest` (workspace profile `ci`) — `[VERIFIED: docs/src/contributors/local-ci-pre-push.md:24]` "cargo nextest run --workspace --profile ci --no-fail-fast" |
| GUI unit/component framework | Vitest (existing `*.test.tsx` files throughout `crates/vox-gui/ui/src`) |
| GUI e2e framework | Playwright, existing specs under `crates/vox-gui/ui/e2e/`, mock IPC via `installTauriMock`/`installTauriMockRich` (`crates/vox-gui/ui/e2e/lib/tauriMock.ts`, `tauriMockRich.ts`) |
| Config file | Workspace root `Cargo.toml` (nextest profile config); `crates/vox-gui/ui/playwright.config.ts` |
| Quick run command | `cargo test -p vox-orchestrator tool_receipt::`, `cargo test -p vox-orchestrator-queue resource_lock`, `pnpm --dir crates/vox-gui/ui typecheck` |
| Full suite command | `vox ci pre-push --full`; `pnpm --dir crates/vox-gui/ui test:e2e` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|--------------------|-------------|
| TRUST-01 (D-01) | Receipt issued before dispatch, fulfilled after, for every `vox_*` tool call | unit/integration | `cargo test -p vox-orchestrator-mcp dispatch` | ❌ Wave 0 — no existing test exercises the receipt wrap in dispatch.rs; existing `safety.rs` tests only cover the wrapper methods directly, not the dispatch.rs call site |
| TRUST-01 (D-02) | Unknown-tool `issue_intent` failure logs warn, tool still executes | unit | new test in dispatch.rs or an integration test | ❌ Wave 0 |
| TRUST-01 (D-03) | `vox_verify_task_claims` returns correct valid/fabricated/unverified split | unit | `cargo test -p vox-orchestrator tool_receipt::` (existing `validate_agent_claims` logic already covered indirectly; add a dispatch-level test for the new tool) | ⚠️ Partial — `validate_agent_claims` itself has no dedicated unit test today (only `verify`/`issue`/`issue_intent` are tested at `tool_receipt.rs:217-264`) — add one, plus a dispatch-level test for the new MCP tool |
| TRUST-01 (D-04) | HMAC output identical before/after crypto routing fix | unit | `cargo test -p vox-orchestrator tool_receipt::` | ✅ Existing — `issue_intent_accepts_registered_tool`, `issue_accepts_registered_tool_and_binds_result` (tool_receipt.rs:240-263) already assert the full issue→verify round trip; re-running them green after D-04 is the regression check |
| MESH-01 (D-06) | Sweep purges all expired entries, not just the contended one | unit | `cargo test -p vox-orchestrator-queue resource_lock` | ⚠️ Partial — existing tests cover single-resource expiry (`semcov_wave19_tests.rs:113-125,154-167`); add a multi-resource sweep test |
| MESH-01 (D-08) | Concurrent tasks declaring the same `resource_id` contend (block/error) | integration | `cargo test -p vox-orchestrator hopper::` or `dispatch::` | ❌ Wave 0 — no existing test exercises hopper→lock integration |
| MESH-01 (D-08) | Lock released on both success and failure | integration | `cargo test -p vox-orchestrator task_dispatch::` | ❌ Wave 0 |
| D-10 (chat receipt status) | Receipt status renders as a chat-turn event chip | e2e | `pnpm --dir crates/vox-gui/ui test:e2e -- <new-spec>.spec.ts` | ❌ Wave 0 — no existing spec covers turn events beyond skill/delegation kinds |
| D-10 (chat lock state) | Lock wait/hold state renders in chat surface | e2e | `pnpm --dir crates/vox-gui/ui test:e2e -- <new-spec>.spec.ts` | ❌ Wave 0 |
| D-11 | Screenshots captured to review-bundle | e2e | `pnpm --dir crates/vox-gui/ui test:e2e -- --grep @review-capture` (pattern per `e2e/review/capture.spec.ts:114`, `test.describe('review-bundle capture @review-capture', ...)`) | ✅ Existing capture harness (`e2e/review/capture.spec.ts`) — new specs should follow its `OUT` path convention (`crates/vox-gui/ui/review-bundle/latest`, `[VERIFIED: crates/vox-gui/ui/e2e/review/capture.spec.ts:23]`) rather than inventing a new screenshot location |

### Sampling Rate
- **Per task commit:** targeted `cargo test -p <touched-crate> <module>::` and, for GUI changes, `pnpm --dir crates/vox-gui/ui typecheck`
- **Per wave merge:** `cargo nextest run --workspace --profile ci --no-fail-fast` (excludes slow `#[ignore]` tests) plus `pnpm --dir crates/vox-gui/ui test:e2e` for any wave touching D-10/D-11
- **Phase gate:** `vox ci pre-push --full` green, plus a live look at the chat GUI per D-11's explicit requirement ("Phase verification must include a live look at the chat GUI, not just unit tests")

### Wave 0 Gaps
- [ ] Dispatch-level integration test proving `issue_intent`/`fulfill_intent` actually fire from `handle_tool_call_with_mode` (not just the wrapper methods in isolation)
- [ ] `validate_agent_claims` dedicated unit test (valid/fabricated/unverified mix in one call)
- [ ] Multi-resource sweep test for `ResourceLockManager` (D-06)
- [ ] Hopper→lock integration test (D-08 acquire) and completion/failure release tests
- [ ] New Playwright spec(s) for chat-surface receipt/lock rendering (D-10/D-11), extending or sitting alongside `chat-interactions.spec.ts`/`tasks-interactions.spec.ts`

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | No | This phase adds no new authn surface. |
| V3 Session Management | Marginal | `session_id`/`agent_id` are already caller-supplied `args` fields on every MCP tool call (dispatch.rs:62-63) — D-01/D-10 read but do not introduce new trust in these values; no change to how they're validated. |
| V4 Access Control | No | D-02 is explicitly fail-open/audit-only this phase — no new access-control decision is gated on receipts or locks yet. |
| V5 Input Validation | Yes | `resource_id` (D-08) is a caller/agent-influenced string that becomes a `HashMap` key and (via the existing `LockAcquired`/`LockReleased` event) a `PathBuf` — no length/content validation currently exists on `ResourceLockManager::try_acquire`'s `resource_id: &str` parameter `[VERIFIED: crates/vox-orchestrator-queue/src/locks/resource.rs:38-44]`. Unbounded string keys from agent-composed task descriptions are a minor DoS/memory-growth surface if not capped. |
| V6 Cryptography | Yes | D-04's entire purpose is a cryptography-policy compliance fix — must not weaken the MAC construction (see Pattern 5's byte-identical-concatenation requirement). |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|-----------------------|
| Agent claims a fabricated tool receipt to make an unexecuted action look verified | Repudiation / Spoofing | This is precisely what TRUST-01/ADR-029 exists to catch — `validate_agent_claims`'s `fabricated` bucket (D-03). Already implemented; this phase only needs to expose it. |
| Unbounded `resource_id` strings causing memory growth in `ResourceLockManager`'s map | Denial of Service | Not currently mitigated — consider a length cap on `resource_id` at the D-08 intake boundary (where it enters `IntakeItem`) rather than deep inside `ResourceLockManager`, consistent with the existing pattern of validating model/agent-composed strings at the boundary (see `is_plausible_skill_id`'s cap pattern, `agent_loop.rs:234-254`, as prior art for this exact class of guard). |
| Chat-surface receipt/lock chips rendering agent-composed content as trusted-looking system UI | Spoofing (UI) | `turn_event_for_result`'s existing doc comment already states this exact rationale for gating strictly on `success`/dispatch-actual-outcome, never on `args` alone (`agent_loop.rs:256-267`) — any new D-10 event kind must follow the same rule: derive `verified`/lock-state fields from the ledger/lock-manager's actual state, never echo agent-supplied strings (e.g. `resource_id`) into the UI without the same bounding `is_plausible_skill_id`-style treatment applies to. |

## Sources

### Primary (HIGH confidence — read this session)
- `crates/vox-orchestrator/src/tool_receipt.rs` — full file read
- `crates/vox-orchestrator-queue/src/locks/resource.rs` — full file read
- `crates/vox-orchestrator-queue/src/locks/lease.rs` — full file read
- `crates/vox-orchestrator-queue/src/semcov_wave19_tests.rs` — full file read
- `crates/vox-orchestrator/src/orchestrator/safety.rs` — full file read
- `crates/vox-crypto/src/facades.rs` — full file read
- `crates/vox-orchestrator/src/types/messages.rs` (partial, `AgentMessage` enum region)
- `crates/vox-orchestrator/src/orchestrator.rs` (partial, field declarations)
- `crates/vox-orchestrator/src/orchestrator/core/mod.rs` (partial, construction site)
- `crates/vox-orchestrator-mcp/src/dispatch.rs` — read in full (top-to-bottom in sections)
- `crates/vox-orchestrator-mcp/src/chat_hop.rs` — full file read
- `crates/vox-orchestrator/src/activity/{mod.rs,project.rs,sink.rs}` — full files read
- `crates/vox-gui/src/commands/activity.rs` — full file read
- `crates/vox-gui/src/commands/chat.rs` (partial)
- `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs` (large sections read: struct/field docs, `turn_event_for_result` + tests, dispatch call site)
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatAgentEventRow.tsx`, `ChatTurnEventRow.tsx` — full files read
- `crates/vox-gui/ui/src/types/dashboard.ts` (first 108 lines)
- `crates/vox-mcp-registry/src/lib.rs` (partial)
- `contracts/mcp/tool-registry.canonical.yaml` (header + first entry)
- `contracts/operations/catalog.v1.yaml` (one entry region)
- `crates/vox-orchestrator-mcp/src/trust_tools.rs` (partial)
- `crates/vox-orchestrator/src/events.rs` (multiple regions: `LockAcquired`/`LockReleased`, `ToolCallDispatched`, Tier B lists)
- `crates/vox-orchestrator/src/runtime.rs` (region around `ToolCallDispatched` emission)
- `crates/vox-orchestrator/src/hopper/{types.rs,store.rs}` — key sections read
- `crates/vox-orchestrator/src/orchestrator/dispatch.rs` — full file read (first 160 lines, which is the whole dispatcher-loop logic)
- `crates/vox-orchestrator/src/types/tasks.rs` (struct field list, `AgentTask`)
- `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/mod.rs`, `fail.rs` (existence + fn-name grep only, not full read — see Assumption A2)
- `.planning/phases/05-multi-agent-coordination-trust-hardening/05-CONTEXT.md`
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/ROADMAP.md`
- `docs/src/adr/025-multi-agent-lock-coherence.md`, `docs/src/adr/029-formal-intent.md`
- `AGENTS.md` (§Cryptography Policy, §Dependency Discipline, §Test-First Policy, §GUI Visual Verification Invariant, §Local CI Gate Tiers)
- `docs/src/contributors/local-ci-pre-push.md` (nextest command line)
- `crates/vox-gui/ui/e2e/review/capture.spec.ts` (OUT path constant)
- `crates/vox-gui/ui/e2e/chat-interactions.spec.ts` (import line confirming `installTauriMock` usage)

### Secondary (MEDIUM confidence)
- Directory listings only (existence confirmed, contents not read): `crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx`, `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/fail.rs`, `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/{gates,healing,persistence,socrates}.rs`

### Tertiary (LOW confidence)
- None — no WebSearch-only claims were needed for this phase; everything is in-repo.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependency, every claim traced to a `Cargo.toml` line or an existing facade function.
- Architecture (D-01 through D-09): HIGH — every wiring point read and quoted from the live tree this session.
- Architecture (D-10/D-11): MEDIUM — the *existing* plumbing (both paths) is HIGH confidence (read and quoted), but the *recommended new* bridging mechanism (event-bus emission for chat-turn receipt display) is a design proposal, not yet code, flagged explicitly as such and left as an Open Question for planner/user confirmation.
- Pitfalls: HIGH — all five are derived directly from reading the actual call sites and existing test coverage, not from general domain knowledge.

**Research date:** 2026-09-28
**Valid until:** Stable — this is a closure-pass phase over a codebase under active but not fast-moving development; re-verify file:line citations if more than ~2 weeks and several merged PRs pass before planning executes, since `dispatch.rs` and `agent_loop.rs` are both large, frequently-touched files per the commit history in this milestone.
