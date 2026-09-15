---
title: "True workflow durability: corrected design"
description: "Audit-corrected design for making Vox workflow replay real — execute the workflow body in the interpreter, intercept activity and host-builtin calls, journal real results."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: true
training_rationale: "Normative design for the durable workflow engine; agents must follow the intercept surface, journal encoding, and lease rules rather than the superseded 2026-09-05 plan."
---

# True workflow durability: corrected design

**Date:** 2026-09-06
**Supersedes the framing of:** `docs/src/architecture/durability-runtime-audit-2026.md` (2026-05-01, stale)
**Amends this file's 2026-09-05 draft** with tree-verified findings G14–G27.
**Constrained by:** [ADR-019](../adr/019-durable-workflow-journal-contract-v1.md), [ADR-021](../adr/021-generated-workflow-durability-parity.md), [ADR-041](../adr/041-durable-functions-completion-2026.md) — Phase 0 amends those ADRs' supported-subset wording.
**Implementation plan:** [docs/superpowers/plans/2026-09-06-true-workflow-durability.md](../../superpowers/plans/2026-09-06-true-workflow-durability.md) (supersedes `2026-09-05-true-workflow-durability.md`)

## 1. What "true durability" means here

A workflow is durable when all six hold:

1. The **orchestrator is deterministic** — no clocks, RNG, network, or filesystem in the workflow body; those live in activities.
2. **History is the source of truth** — an activity's result is persisted before the next step starts.
3. **Resume is replay** — restart with the same `run_id`, completed activities return their journalled value without re-running.
4. **Timers and signals outlive the process** — a 24-hour wait is a row with a wake time, not `tokio::sleep` in a process that may die.
5. **Code can evolve** — inserting a line must not rename every later `activity_id`.
6. **Side effects are at-least-once**, with `activity_id` exposed as an idempotency key. Exactly-once for arbitrary I/O is not offered.

Explicit non-goals (unchanged from ADR-021): process/VM checkpointing, CRIU, "every async fn is durable", exactly-once for `process.run` or arbitrary HTTP, mesh/`PopuliActivity` execution (loud error or feature-gated deferral — do not silently drop).

## 2. Verified state of the code (2026-09-06)

Confirmed by reading the tree, not by reading the old audit. G1–G13 hold with the wording corrections below; G14–G27 are findings the 2026-09-05 draft missed.

### 2.1 G1–G13 (hold, some wording narrowed)

| # | Finding | Evidence |
|---|---|---|
| G1 | **Interpreter local activities never run user Vox code.** Non-mesh steps return canned `{"event":"LocalActivity","status":"executed"}`. Mesh runs HTTP; timers really sleep; generated activity Rust fns contain `func.body` but the workflow path never calls them. | `workflow/run.rs::execute_local_activity_step` |
| G2 | **The workflow body is discarded by codegen.** `emit_workflow_body` never emits `func.body`; it calls `interpret_workflow_durable`. Activity fns are unused on the *workflow* path, not absent. | `vox-codegen/src/codegen_rust/emit/durability_lower.rs` |
| G3 | **Three disagreeing identity schemes.** Interp: `blake3(workflow \0 name \0 position)`. Codegen `journal::execute`: `func.generated_hash`. Explicit `with { activity_id }` is a third key. | `run.rs::derive_activity_id`, `durability_lower.rs::emit_activity_body` |
| G4 | **Generated workflows use `DefaultTracker`** (RAM, always "not completed"). Crash ⇒ replay from zero. | `durability_lower.rs:57` |
| G5 | **`journal::execute` always runs the body**; only persist/replay is `cfg(test)`/`test-support`. Production is always-execute-never-replay, not a no-op. Delete it anyway — the id scheme disagrees with the runner. | `journal/execute.rs` |
| G6 | **Timers sleep in-process.** `workflow_wait` → `tokio::time::sleep`, persisted only *after* the sleep. | `run.rs::execute_step_once` |
| G7 | **Signals are tracker-split, not fail-then-satisfy on one path.** `VoxDbTracker` `bail!`s in `on_activity_started` before the stub runs. `DefaultTracker` / codegen emit `SignalWaitSatisfied` with no row. | `db_tracker.rs:173-184`, `run.rs:383-389` |
| G8 | **Control flow is a compile-time linearizer.** `if` requires `eval_const_bool`; `match`/`while`/`loop`/`break` hard-error; `for` only unrolls literal lists. Helper-fn activity calls are silently dropped. | `workflow/plan.rs` |
| G9 | **Planned arguments are always `vec![]`.** The 24h dedup cache hashes the empty array. The cache hooks are no-ops on **both** `VoxDbTracker` and `FileJournalTracker`. | `plan.rs`, `tracker.rs` defaults |
| G10 | **`WorkflowCompleted.return_value` is always `null`**, so `extract_terminal_return::<T>` fails for any non-nullable `T`. | `run.rs`, `return_extract.rs` |
| G11 | **`VoxDbTracker` inherits no-op patch methods**; no `workflow_patch_log` table. `FileJournalTracker` implements patches (not the cache) and is **tests-only** — no production mobile runner wires it. The only production path is `vox mens workflow run` → SQL, where patches do not survive resume. Do not claim "durable on mobile, not server." | `db_tracker.rs`, `file_journal.rs:323`, `schema/domains/execution.rs` |
| G12 | **`run.rs` emits `WorkflowPatch` and `ActivityCacheHit`**, which the v1 schema enum rejects. The contract test only validates hand-built retry events. | `contracts/workflow/workflow-journal.v1.schema.json` |
| G13 | **The interpreter is `!Send`.** `VoxValue` and `Scope` hold `Rc`. `assert_not_impl_any!(VoxValue: Send, Sync)` is a tripwire — do not Arc-ify values. Confine the interpreter to a dedicated OS thread (same pattern as interp `std.http`). | `eval/value.rs`, `eval/env.rs`, `eval_cow_semantics_test.rs` |

G13 is the constraint that shapes the host bridge.

### 2.2 G14–G27 (missed by the 2026-09-05 draft)

| # | Finding | Evidence |
|---|---|---|
| G14 | **Durability primitives are not activities.** `HirExpr::WorkflowVersion` and `HirExpr::With` error via the parity matrix (`PARITY_UNIMPLEMENTED`). `workflow_wait` / `workflow_wait_signal` are special-cased only in `plan.rs`. An activity-only hook cannot see them. | `eval/expr.rs:831-848` |
| G15 | **`Result::Err` is not a crash.** A successful activity may return `Error(...)`. That value must flow to `?`. Retry only on interpreter panic / `_Panic` / I/O failure. | Task 1.4 of the 2026-09-05 plan treated `failed: true` as retryable |
| G16 | **Two encodings, one extractor.** Journal replay needs a `__vox` envelope. `extract_terminal_return::<T>` `serde_json::from_value`s `return_value`. `{"__vox":"Ok","value":"tx"}` will not deserialize to `Result<String,_>`. Fix extract in Phase 1. | `return_extract.rs` |
| G17 | **Lease makes naive crash tests fail for the wrong reason.** A new `VoxDbTracker` mints a new `owner_id`. `next_activity_attempt_start` bails if another owner holds a live 30s lease. Crash-resume tests must expire the lease. | `db_tracker.rs:332-343` |
| G18 | **A body-first rewrite that hardcodes `execution_boundary: "local"` silently drops mesh.** Defer mesh with a loud error; do not delete the `mens` path unnoticed. | `run.rs::execute_step_once` |
| G19 | **A host without `Drop` deadlocks.** `blocking_recv` on the interpreter thread hangs forever if the runner forgets `respond` or drops the host without `finish()`. | planned `WorkflowHost` |
| G20 | **Unknown journal envelopes must fail the run**, not decode to `Null`. Silent `Null` is replay corruption. | planned `from_journal_json` |
| G21 | **`HirExpr::Call` is a 4-tuple.** Adding `call_ordinal` is workspace-wide HIR churn and breaks embedded HIR JSON. Stamp ordinals on a `HirFn` side table or a `HashMap<Span, u32>` at hook-install time. | `hir/nodes/stmt_expr.rs` |
| G22 | **New tables need the data-storage pipeline** (`contracts/db/` fragment, version bump, `vox schema generate`, `vox ci data-storage-guard`). New `VOX_*` vars need `contracts/config/env-vars.v1.yaml`. `workflow_signal_log` already exists. | data-storage SSOT |
| G23 | **Intercept after callee eval, not only `Call(Ident)`.** `let f = charge; f(x)` bypasses a syntactic Ident hook. | `eval/expr.rs:399-416` |
| G24 | **Nested workflow / helper-fn activities are invisible** to the planner (`plan.rs` skips non-activity callees). Ban nested `workflow` calls as a compile error until designed. | `plan.rs:333-335` |
| G25 | **`__vox_run_workflow` is called from `emit_main` and never defined.** `VOX_RUN_WORKFLOW` binaries fail to link. | `codegen_rust/emit/http.rs` ~600 |
| G26 | **Generated `Cargo.toml` uses `vox-workflow-runtime` with `default-features = false`.** `VoxDbTracker` is `#[cfg(feature = "sql")]`. Phase 6 must emit `features = ["sql"]` and `vox-db`. | `codegen_rust/emit/mod.rs` |
| G27 | **Persist workflow args on start.** A waker that resumes with `vec![]` loses parameters. Store `args_json` on `workflow_run_log` in Phase 1. | planned Task 3.2 vs 3.1 DDL |

## 3. The correction: one engine, not two

Executing planned activities first, then later executing the workflow body, is the wrong order. Body execution **subsumes** planned-step execution:

- real arguments appear for free (G9 dissolves),
- real results appear for free (G1 dissolves),
- `if result.ok`, `match`, `while`, `?`, and non-literal `for` are ordinary interpreter control flow (G8 dissolves — `plan.rs`'s linearizer is deleted, not extended),
- the return value is the workflow function's return value (G10 dissolves).

Do **not** delete `plan.rs` until the replacement runner is green. `plan_workflow_replay_ir` **is** today's engine. `plan_workflow_activities` is a public export used by ignored integration tests — keep a thin walker or update those tests in the same PR.

### 3.1 Intercept surface (three families, not one hook)

Parking does **not** fall out of an activity-only hook (G14). The interpreter must hand these to the async runner:

| Family | When | `HostRequest` |
|---|---|---|
| Activity | After evaluating the callee, if it resolves to a `DurabilityKind::Activity` name (G23) | `Begin` / `End` |
| Patch | `HirExpr::WorkflowVersion` (must stop being `PARITY_UNIMPLEMENTED` in Phase 1) | `Patch { change_id, min, max }` |
| Host builtin | Bare calls named `workflow_wait` / `workflow_wait_signal` | `Wait { deadline_ms }` / `WaitSignal { key }` |

`HirExpr::With` evaluates the options object, stashes it on the interpreter for the inner evaluation, then the activity `Begin` carries those options. This lands in Phase 1 — before the linearizer is deleted — because every `with { activity_id, retries, timeout }` workflow otherwise hard-errors.

Nested `workflow` calls are a **compile error** until a follow-up designs child-run identity (G24).

The hook returns a **decision**, not a value (it is invoked from `&mut Interpreter` and must not re-enter):

```rust
enum HostDecision {
    Replay(serde_json::Value), // journal-encoded VoxValue
    Execute,
    Park(String),
    Ack,
}
```

`failed` on `End` means the body panicked or returned `_Panic`. `VoxValue::Result(Err(_))` is `failed: false` and flows to `?` (G15). There is no `Retry` decision — the runner re-sends `Execute` after recording an attempt failure.

### 3.2 The `!Send` bridge

`Interpreter` is `!Send`; journal I/O is `async`. Confine, do not Arc-ify `VoxValue` (`eval_cow_semantics_test.rs` tripwire):

```
async runner (tokio task)              workflow thread (std::thread, owns Interpreter)
─────────────────────────              ──────────────────────────────────────────────
spawn thread with HirModule            interp.run_module(&hir)
  (Clone; already in a OnceLock)       interp.call(workflow_name, args)
loop {                                   └─ hook serializes to HostRequest JSON
  req = req_rx.recv().await                   req_tx.send(...)
  journal I/O via WorkflowTracker             ← blocking_recv on resp_rx
  resp_tx.send(decision)
}
finish() via spawn_blocking + join → HostOutcome
```

Values cross as `serde_json::Value` via `VoxValue::to_journal_json`. The hook type does **not** need `+ Send`.

`WorkflowHost` must implement `Drop` that aborts / unblocks the thread (G19). `blocking_recv` has no timeout; the contract is exactly one outstanding request and a mandatory `respond` before the next `next()`.

Add `assert_not_impl_any!(Interpreter: Send, Sync)` next to the existing `VoxValue` guard.

### 3.3 Journal value contract

`json.encode` (`vox_to_json`) stays lossy. The journal uses a separate tagged envelope:

- Scalars and lists encode as plain JSON.
- `Result` / `Option` / `Tagged` / `Tuple` / `Decimal` wrap as `{"__vox": "Ok"|"Err"|"Some"|"None"|"Tagged"|"Tuple"|"Decimal", ...}`.
- User objects that contain a `"__vox"` key are escaped as `{"__vox": "Object", "fields": ...}`.
- Unjournalable kinds (`Fn`, `Regex`, sentinels) encode as `{"__vox": "Unjournalable", "kind": ...}`.
- **`from_journal_json` returns `Result<VoxValue, JournalDecodeError>`.** Unknown or `Unjournalable` envelopes fail the run (G20). Do not decode to `Null`.

`extract_terminal_return::<T>` decodes the envelope into serde-facing JSON (e.g. `{"__vox":"Ok","value":"tx"}` → a value `T` can deserialize), then `from_value`. Land this in Phase 1 (G16).

Pre-Phase-1 journal rows are canned `LocalActivity` objects. Resume of those runs **refuses** with a clear error. Do not `Replay` a canned object as a `VoxValue`.

### 3.4 Parking, leases, clocks

A parked run **abandons its thread**. Resume replays from the top. Timers and signals are `HostRequest::Wait` / `WaitSignal`, not fake activities named `__durable_timer_wait`.

- First `workflow_wait(d)`: persist `wake_at_ms = host_now_ms() + d` on `workflow_run_log`, set status `parked`, **clear `lease_owner`**, park.
- First `workflow_wait_signal(k)`: atomic consume-or-park in one DB transaction (TOCTOU). Existing table: `workflow_signal_log`. `record_workflow_signal` has no production caller today; add `vox workflow signal`.
- Waker: small loop using the `@scheduled` **wall-clock remaining-time clamp** (`scheduled/runner.rs`), not a copy of that runner. `SystemTime` does not follow `tokio::time::advance` — tests backdate `wake_at_ms`.
- Crash-resume steals the run if `lease_until_ms < now`. Tests expire the lease explicitly (G17).
- Workflow args are stored as `args_json` on `workflow_run_log` at start (G27). The waker never resumes with `vec![]`.

Timer tests use backdate + waker, not a real 5-second sleep. Design gate 4 (wall-clock ≈ 5 s after a mid-wait crash) is **not** a required CI gate unless a fake host clock is injected.

### 3.5 Identity

Phase 1 may key implicit ids as `blake3(workflow \0 name \0 per-name-counter)` so the body-first runner can ship. Phase 5 replaces that with:

`(enclosing_fn, call_ordinal_from_HirFn_side_table, iteration_index)`

Do not widen the `HirExpr::Call` 4-tuple (G21). Explicit `with { activity_id }` always wins; a duplicate explicit id in one run is an error (today loop iterations silently alias).

Completed-run resume policy: **refuse** (the run already has `WorkflowCompleted`). Code-upgrade tests that append a trailing activity must use a run that stopped *before* completion (park or crash), not `ids_for(V1)` which completes.

### 3.6 Generated path

Generated workflows already call `interpret_workflow_durable` (ADR-021 option 1). Phase 6 does **not** invent a second engine. It must:

- emit `VoxDbTracker` and register `VOX_WORKFLOW_RUN_ID` in `contracts/config/env-vars.v1.yaml`,
- emit `features = ["sql"]` + `vox-db` (G26),
- **define `__vox_run_workflow`** or stop calling it (G25),
- stop wrapping activities in `journal::execute` (update `durability_compiles.rs` in the same task),
- prove interp ≡ generated by **emitting and compiling** the generated workflow fn against the same tracker/`run_id` — not by calling `interpret_workflow_durable` twice,
- stop emitting `compile_error!` for `WorkflowVersion`.

ADR-041's "Stable codegen / crash-replay proves integration" overclaims. Until Phase 6 lands, codegen durability is Preview ([stability.md](stability.md)).

## 4. Ordering

| Phase | Delivers | Notes |
|---|---|---|
| 0 | Docs + ADRs stop over-claiming; schema admits **only** `WorkflowPatch` and `ActivityCacheHit`; real-journal conformance on today's 3-arg runner | Do not pre-admit Phase 3–4 event names |
| 1 | Body executes; intercept surface (activity + With + Version + wait names); host bridge; args persisted; extract understands `__vox`; linearizer deleted **after** the new runner is green | ~600 LoC net |
| 2 | Crash windows with a `CountingTracker` and expired leases; side-effect file counts (`@uses(fs)` / `std.fs.write`) | Mutation-verified |
| 3 | `HostRequest::Wait`, `wake_at_ms` + `parked_reason` on `workflow_run_log`, waker, backdated tests | Shared park machinery with Phase 4 |
| 4 | `HostRequest::WaitSignal`, consume-or-park, `vox workflow signal` | Uses existing `workflow_signal_log` |
| 5 | Call-site side-table ids; `workflow_patch_log` via data-storage pipeline; duplicate explicit ids error | |
| 6 | Generated tracker + sql features + dispatcher + real ADR-021 compile gate; delete `journal::execute` | |
| 7 *(deferred)* | Transactional outbox | Not in this plan |

Phases 3–6 are meaningless before Phase 1: they would park and replay canned strings.

## 5. What is deleted, not extended

- `workflow/plan.rs` linearizer (609 LoC), **after** Phase 1.4 is green. Keep duration/mesh parse helpers. Replace or thin-wrap `plan_workflow_activities`.
- `durable_promise.rs` runtime struct (separate PR): keep the language type `DurablePromise[T]`.
- `journal/execute.rs` + `journal/test_support` in Phase 6, together with the codegen tests that require the symbol.

`vox-compiler` is already over its 45_000 LoC budget (~52k). Stay in-crate; do not add a crate edge. `vox-workflow-runtime` is L4 in `crate-layers.v1.json`.

## 6. Test gates

A phase is not done until its gate passes, and each guard is verified **by mutation**.

1. **Crash after complete (mid-workflow)** — `CountingTracker` faults after first `on_activity_completed`; resume; activity 1's side-effect file count stays 1; activity 2 runs.
2. **Crash during activity** — seed `started` without `completed`, **expire the lease**; resume retries; `resume_attempt == 2`; body runs (file count).
3. **Runtime branch** — `if charge.ok { … } else { … }`; both branches reachable; replay takes the same branch.
4. **Timer** — persist `wake_at_ms`, backdate, waker resumes; no real multi-second sleep. Optional fake-clock mid-wait crash is not required for Phase 3 close.
5. **Signal** — empty log ⇒ status `parked` (not `failed`); insert signal; resume completes. Late signal between check and park still completes.
6. **Interp ≡ generated** — emit generated Rust, compile/link, run the generated workflow fn with the same `run_id` and tracker; same `activity_id` set and decoded return (ADR-021).
7. **Code upgrade** — append a trailing activity on an **in-flight** (not completed) `run_id`; prior ids replay.
8. **Schema** — every event the runner actually emits validates against `workflow-journal.v1.schema.json` on a real journal. New event *names* land in the same PR as the first emitter.

## 7. ADR amendment (Phase 0)

ADR-019 / ADR-021 / ADR-041 list unrestricted `match` / unbounded loops as non-goals of the **linearizer-era** subset. This design moves runtime interp control flow in-scope under the existing determinism lint (no `time.now` / `random` / `uuid` in workflow bodies). Generated-Rust *subset* may stay linear until Phase 6's compile gate says otherwise. Phase 0 updates those three ADRs so they do not contradict this spec.

## 8. Related

- [ADR-019: Durable workflow journal contract v1](../adr/019-durable-workflow-journal-contract-v1.md)
- [ADR-021: Generated workflow durability parity](../adr/021-generated-workflow-durability-parity.md)
- [ADR-041: Durable functions completion](../adr/041-durable-functions-completion-2026.md)
- [Explanation: Durable Execution](../explanation/expl-durable-execution.md)
- [Stability tiers](stability.md)
- [Data storage SSOT](data-storage-ssot-2026.md)
- [Implementation plan (2026-09-06)](../../superpowers/plans/2026-09-06-true-workflow-durability.md)
