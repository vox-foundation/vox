# Phase 5: Multi-Agent Coordination & Trust Hardening - Context

**Gathered:** 2026-09-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Multiple concurrent agents can safely contend for shared resources, and every tool call they make
is independently auditable. Requirements: MESH-01, TRUST-01.

Scout findings (2026-09-28, verify before relying on them — both success criteria are far more
built than the ROADMAP text implies):
- **ADR-025** (`docs/src/adr/025-multi-agent-lock-coherence.md`, status `research`/Proposed) names
  `ResourceLockManager` with generic string resource IDs, mandatory-TTL leases, and bulletin
  synchronization. **All three exist**: `crates/vox-orchestrator-queue/src/locks/resource.rs`
  (`ResourceLockKind`, `ResourceLock`, `ResourceLockManager::{try_acquire,release,snapshot,is_locked}`),
  wired onto `Orchestrator` (`orchestrator.rs:130`, `orchestrator/core/mod.rs:201`), with
  `acquire_resource_lock`/`release_resource_lock` on `Orchestrator` (`orchestrator/safety.rs:71-115`)
  already publishing `AgentMessage::ResourceLockAcquired/Released` to the bulletin and
  `AgentEventKind::LockAcquired/Released` to the event bus. 9 passing tests
  (`crates/vox-orchestrator-queue/src/semcov_wave19_tests.rs:14-168`). **Gaps:** no eviction sweep
  (expired entries never removed from the map); `acquire_resource_lock`/`release_resource_lock`
  have zero callers repo-wide — only `.snapshot()` is read (a status endpoint), so the manager is
  built and tested but not load-bearing. It is purely in-process (`Arc<RwLock<HashMap>>`), unlike
  the sibling `FileLockManager`'s DB-backed distributed variant (`locks/lease.rs`).
- **ADR-029** (`docs/src/adr/029-formal-intent.md`, status `research`/Proposed, renumbered from
  ADR-024) names a two-tier system: agents claim a "receipt" per tool call; the orchestrator issues
  HMAC-signed receipts. **Both tiers exist**: `crates/vox-orchestrator/src/tool_receipt.rs` has
  `ToolReceipt`, `ToolReceiptLedger::{issue_intent,fulfill_intent,issue,verify,validate_agent_claims}`
  (the last already splits claims into valid/fabricated/unverified — the "two-tier" check itself).
  `issue_intent` fails closed on unknown tool names against `vox_mcp_registry::TOOL_REGISTRY`
  (Phase 2 D-02). `Orchestrator::issue_tool_receipt/fulfill_tool_receipt/verify_tool_receipt`
  (`orchestrator/safety.rs:11-33`) wrap it. **Gap:** `vox-orchestrator-mcp/src/dispatch.rs`'s live
  tool-call path (`handle_tool_call_with_mode` → `handle_tool_call_inner`, the ~2200-line
  `match name { "vox_..." => ... }` table) never calls any of these — zero matches repo-wide. This
  is the gap Phase 2's research (`02-RESEARCH.md:40,123,146,247,299,307`) explicitly deferred here.
  `ServerState` already carries `orchestrator: Arc<Orchestrator>`, so the hook needs no new plumbing.
- **Crypto-policy note:** `tool_receipt.rs` imports `blake3` directly (`blake3::Hasher::new_keyed`)
  instead of routing through `vox-crypto::facades::keyed_hash(&[u8;32], &[u8]) -> [u8;32]`, which
  already exists and does exactly this. Violates AGENTS.md §Cryptography Policy #1. The
  `crypto_ban.rs` audit detector only regexes named banned crates, not bare `blake3`/`sha3` outside
  `vox-crypto`, so this is currently silent debt.
- No standalone "two-tier formal-intent verification" design doc exists beyond ADR-029's three
  sentences; `validate_agent_claims`'s valid/fabricated/unverified split is itself the full spec.

</domain>

<decisions>
## Implementation Decisions

### TRUST-01: dispatch wiring
- **D-01:** Wrap `handle_tool_call_with_mode` uniformly for every `vox_*` tool: `issue_intent`
  before `handle_tool_call_inner` runs, `fulfill_intent` after it returns (success or error, with
  the result hash). No per-tool allowlist to maintain.
- **D-02:** Fail-open. An `issue_intent` error (e.g. unknown tool name) is logged at `warn` and the
  tool call still executes normally — the ledger is an audit/detection trail this phase, not an
  authorization gate. A `fabricated`/`unverified` verdict from `validate_agent_claims` is surfaced
  (D-03), not auto-punished. — **Reversibility:** reversible — flipping to fail-closed later is a
  local change to the wrapper, no contract to migrate.
- **D-03:** Expose `validate_agent_claims`'s verdict via a new read-only MCP tool (working name
  `vox_verify_task_claims`: takes claimed receipt IDs, returns the valid/fabricated/unverified
  split). No automatic side effect on a fabricated claim this phase — detection and reporting only.
- **D-04 (housekeeping):** Fix `tool_receipt.rs`'s crypto-policy violation while the file is open
  for D-01: replace the direct `blake3::Hasher::new_keyed` calls with
  `vox_crypto::facades::keyed_hash`. Behavior must stay identical (same MAC construction, keyed
  BLAKE3) — this is a routing fix, not an algorithm change.
- **D-05 (housekeeping):** Ratify ADR-029 (flip `status: research` to Accepted with a dated Status
  line), matching the Phase 4 pattern used for ADR-045, once its shape (receipt format, the new
  verify tool) is implemented and correct in this phase — not before, so the ADR describes what was
  actually built.

### MESH-01: ResourceLockManager
- **D-06:** Add a lazy sweep: on every `try_acquire` (and `is_locked`), purge *all* currently-expired
  entries from the map, not just the one resource_id being contended. No background timer/task —
  stays in-process, self-driving, consistent with the manager's existing synchronous design.
- **D-07:** Single-orchestrator-process coordination is the full scope for this phase. Cross-node
  reach via populi-mesh is an explicit non-goal — deferred, not designed against here. — **Reversibility:**
  reversible — the in-process manager can gain a DB/mesh-backed variant later (mirroring
  `FileLockManager`'s distributed sibling) without changing this phase's API surface.
- **D-08:** The real caller that makes the manager load-bearing: hopper task dispatch. Add an
  optional `resource_id: Option<String>` field to the task/intake spec. When present, dispatch
  acquires an exclusive `ResourceLock` on it before the agent starts the task and releases on
  completion or failure; concurrent tasks declaring the same `resource_id` now actually contend
  (block/error) instead of racing silently. Tasks with no `resource_id` skip locking entirely —
  opt-in, no behavior change for existing tasks/callers.
- **D-09 (housekeeping):** Ratify ADR-025 the same way as D-05, once the sweep (D-06) and the real
  caller (D-08) are implemented.

### Chat-GUI surfacing (added 2026-09-28, user directive mid-planning)
- **D-10:** Nothing in this phase ships orchestrator-only. Tool receipts (D-01/D-03) and resource-lock
  state/contention (D-06/D-08) MUST surface in the `vox-gui` chat surface — the user must be able to
  see, from the chat, which tool calls in a turn carry a verified receipt (and any fabricated/unverified
  claim), and when a task is waiting on / holding a `resource_id` lock. Reuse the existing chat
  transcript/tool-call rendering and event plumbing (bulletin → event bus → GUI transport) rather than
  adding a new panel, unless research shows no existing surface fits.
- **D-11:** "Testable with our instruments" = every D-10 surface is covered by a deterministic
  Playwright spec under `crates/vox-gui/ui/e2e/` driven by the mock IPC harness
  (`installTauriMock`/`installTauriMockRich`), capturing screenshots into
  `crates/vox-gui/ui/review-bundle/latest/` per AGENTS.md §GUI Visual Verification Invariant, plus
  `pnpm --dir crates/vox-gui/ui typecheck` green. Where the real backend path is cheap to exercise
  (e.g. an MCP tool-call round trip producing a receipt), prefer an end-to-end check over mock-only.
  Phase verification must include a live look at the chat GUI, not just unit tests.

### Claude's Discretion
- Exact MCP tool name/schema for the D-03 verify surface (`vox_verify_task_claims` is a working
  name, not locked).
- Exact shape of the `resource_id` field addition to the task/intake spec (D-08) — follow the
  existing hopper/intake type conventions.
- Whether the D-06 sweep also runs on `release` (not just acquire/is_locked) if that's cleaner.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### ADRs (this phase's spec)
- `docs/src/adr/025-multi-agent-lock-coherence.md` — ResourceLockManager decision (D-06..D-09).
- `docs/src/adr/029-formal-intent.md` — two-tier formal-intent / HMAC receipts decision (D-01..D-05).

### Existing implementation (already built, being wired/extended, not rebuilt)
- `crates/vox-orchestrator-queue/src/locks/resource.rs` — `ResourceLockManager` and friends.
- `crates/vox-orchestrator-queue/src/locks/lease.rs` — `FileLockManager`'s distributed variant,
  the pattern to *not* replicate this phase (D-07), and `force_release_stale` as a sweep precedent.
- `crates/vox-orchestrator-queue/src/semcov_wave19_tests.rs:14-168` — existing lock manager tests.
- `crates/vox-orchestrator/src/orchestrator.rs:128,130` — `tool_ledger`/`resource_locks` fields.
- `crates/vox-orchestrator/src/orchestrator/core/mod.rs:199,201` — construction sites.
- `crates/vox-orchestrator/src/orchestrator/safety.rs:11-33,71-115` — the wrapper methods to call
  from dispatch.rs (D-01) and from hopper task dispatch (D-08).
- `crates/vox-orchestrator/src/tool_receipt.rs` — `ToolReceiptLedger` (D-04's target file).
- `crates/vox-orchestrator/src/types/messages.rs:118-127` — `AgentMessage::ResourceLock{Acquired,Released}`.
- `crates/vox-orchestrator-mcp/src/dispatch.rs` — `handle_tool_call_with_mode`/`handle_tool_call_inner`,
  the D-01 hook point; `ServerState.orchestrator: Arc<Orchestrator>` is already in scope there.
- `crates/vox-crypto/src/facades.rs:17-22` — `keyed_hash`, D-04's replacement call.
- `crates/vox-mcp-registry` — `TOOL_REGISTRY`, already used by `issue_intent`'s fail-closed check.

### Prior-phase precedent
- `.planning/phases/02-wire-up-reclassify-dormant-crates/02-RESEARCH.md:40,123,146,247,299,307` —
  the explicit deferral of this exact wiring to Phase 5.
- `.planning/phases/04-gui-dashboard-architecture-consolidation/` — the ADR-ratification pattern
  (Status line + intel sync) to follow for D-05/D-09.
- AGENTS.md §Cryptography Policy #1 (application crypto must go through `vox-crypto`) — D-04.
- AGENTS.md §Dependency Discipline — crate-edge exceptions are user-authorized only; check whether
  any new crate needs to depend on `vox-crypto`/`vox-mcp-registry`/`vox-orchestrator*` it doesn't
  already, during planning/research, not assumed here.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Orchestrator::acquire_resource_lock`/`release_resource_lock` and
  `issue_tool_receipt`/`fulfill_tool_receipt`/`verify_tool_receipt` — fully implemented, just need
  real callers (D-01, D-08).
- `vox_crypto::facades::keyed_hash` — the correct primitive for D-04, already exported.
- `FileLockManager::force_release_stale` — a sweep-method precedent to look at for D-06's shape,
  even though D-06 chose lazy-on-acquire over a background task.

### Established Patterns
- Bulletin + event-bus broadcast on lock acquire/release already established
  (`orchestrator/safety.rs:71-115`) — D-08's new caller gets this for free by calling the existing
  wrapper methods, not `ResourceLockManager` directly.
- `handle_tool_call_with_mode` already extracts `agent_id`/`session_id`/`trace_id` near its top for
  telemetry — D-01's wrapper can reuse that extraction rather than re-deriving it.

### Integration Points
- vox-orchestrator-mcp dispatch.rs (D-01, D-03).
- Hopper/task-intake spec (D-08) — wherever the task struct that `HopperIntake` submits/dispatches
  is defined; research should confirm the exact type and file (Phase 3's D-13 webhook work touched
  hopper intake shapes recently and is a nearby precedent to check for conventions).

</code_context>

<specifics>
## Specific Ideas

The user-facing surface is the `vox-gui` chat (D-10/D-11): receipt status on tool-call entries in
the transcript, and lock wait/hold state on tasks, both visually verified via Playwright.

</specifics>

<deferred>
## Deferred Ideas

- Cross-node/mesh-backed resource locking (D-07) — a distinct, larger feature for a future phase if
  MESH-01's scope ever needs to expand past single-orchestrator-process coordination.
- Automatic consequences for a fabricated tool-claim verdict (D-03) — detection/reporting only this
  phase; deciding what to *do* about a fabricated claim (reject the task? flag the agent?) is future
  work once the detection surface has been used for a while.
- Background-timer-based lock sweeping (D-06 chose lazy-on-acquire instead) — revisit if locks are
  observed sitting expired-and-uncontended for long periods in practice.

</deferred>

---

*Phase: 05-multi-agent-coordination-trust-hardening*
*Context gathered: 2026-09-28*
