---
title: "VoxDB pooling & MVCC benchmark recommendation"
description: "Measured throughput, correctness, and safety comparison of VoxDb's shared-connection pattern vs VoxDbPool vs VoxDbPool+MVCC vs SQLite, with a default-on/default-off recommendation."
category: "Architecture SSOTs"
---

# VoxDB pooling & MVCC benchmark recommendation

## Summary

At both tested concurrency levels (32 and 128 tasks, 20 writes/task, on-disk
file), the existing `shared`-connection pattern beat real per-task pooling on
throughput: 4095 vs. 2712 writes/sec at 32 tasks, and 12045 vs. 7965
writes/sec at 128 tasks — a ~34% throughput deficit for `pooled` at both
levels (33.8% at 32 tasks, 33.9% at 128 tasks). `sqlite` (plain `rusqlite`, one connection per task) was slower than
both Turso modes at every concurrency level tested (1686 writes/sec at 32
tasks, 1587 at 128), so SQLite's numbers did **not** beat Turso's in this
workload shape, contrary to what isolated point-latency numbers might
suggest. `pooled-mvcc` failed to complete setup at both concurrency levels —
Turso's MVCC journal mode rejects the `AUTOINCREMENT` keyword that VoxDB's
production schema uses, so no throughput data exists for that mode. No mode
that completed (`shared`, `pooled`, `sqlite`) produced any `misuse_errors`,
`busy_errors`, or `rowid_mismatches`, and all three passed `row_count_ok` and
`PRAGMA integrity_check` at both concurrency levels — no correctness or
safety problem was observed in any working configuration, only a throughput
gap in `shared`'s favor. The documented `last_insert_rowid` race in
`GuardedConnection` did not reproduce in any run this session.

## Results

### Point-insert latency (`cargo bench -p vox-db --bench concurrency_bench`)

| Benchmark | Mean time |
|---|---|
| `turso_shared_connection_insert` | 11.359 µs |
| `turso_pooled_connection_insert` | 30.821 µs |
| `rusqlite_insert` | 1.6965 µs |

*Table values are Criterion's reported mean across 100 samples.*

**Note on pooled vs. shared latency gap:** `turso_pooled_connection_insert` (30.821 µs) measures connection acquisition (`VoxDbPool::get()` invoking `connect()` + `apply_pragmas()`) plus the insert operation on every iteration, whereas `turso_shared_connection_insert` (11.359 µs) times only the insert operation on an already-open connection. The ~2.7× gap reflects connection-acquisition overhead, not a difference in insert speed. Task 5's concurrent-write throughput harness acquires a pooled connection once per task and reuses it across multiple writes, so its numbers do not pay this per-operation cost and are not directly comparable to this latency table.

HTML report: `target/criterion/report/index.html`

### Concurrent write stress (`cargo run -p vox-db --example concurrency_stress`)

20 writes/task, on-disk file, `busy_timeout=5000` (Turso and SQLite both).

| mode | tasks | throughput (writes/sec) | misuse_errors | busy_errors | rowid_mismatches | row_count_ok | integrity_check |
|---|---|---|---|---|---|---|---|
| shared | 32 | 4095 | 0 | 0 | 0 | true | ok |
| shared | 128 | 12045 | 0 | 0 | 0 | true | ok |
| pooled | 32 | 2712 | 0 | 0 | 0 | true | ok |
| pooled | 128 | 7965 | 0 | 0 | 0 | true | ok |
| pooled-mvcc | 32 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A |
| pooled-mvcc | 128 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A |
| sqlite | 32 | 1686 | 0 | 0 | 0 | true | ok |
| sqlite | 128 | 1587 | 0 | 0 | 0 | true | ok |

**Note on rowid_mismatches:** shared mode's `rowid_mismatches` was 0 at both 32 and 128 tasks. The documented `last_insert_rowid` race in `GuardedConnection` (see its doc comment in `crates/vox-db/src/lib.rs`) did not reproduce in this run. This is probabilistic/timing-dependent and does not disprove the race; it means this specific test dataset and environment did not independently demonstrate it.

**Note on pooled-mvcc:** Turso's MVCC journal mode rejects the AUTOINCREMENT keyword. VoxDB's baseline production schema (automatically applied by `VoxDbPool::new()`'s migration step) uses AUTOINCREMENT in multiple tables. A clean re-run with wildcard cleanup (`rm -f /tmp/vox-stress-test.db*`, needed because MVCC mode creates a `.db-log` sidecar the narrower cleanup command missed) confirmed that both 32-task and 128-task pooled-mvcc runs fail consistently with `Parse error: AUTOINCREMENT is not supported in MVCC mode`. This is a real and consistent compatibility finding, not an environment fluke: the production schema cannot currently run against MVCC-enabled Turso databases. The rows above are marked N/A because the harness never completed its initialization phase — there is no throughput number to report, zero or otherwise.

## Decision: real per-task pooling (`VoxDbPool` vs `GuiDbPool`'s shared connection)

The rubric below was written anticipating two outcomes: pooling clearly wins
(switch), or pooling clearly fails with errors (don't switch, use a
writer-actor instead). Neither branch's condition is literally true here —
`pooled` did not beat `shared` on throughput, but `pooled` also produced zero
`busy_errors` and a passing `integrity_check` at every concurrency level, so
it is not a failure mode either. The rubric's premise — that pooling would
either clearly win or clearly fail with errors — didn't anticipate a clean,
error-free loss, so neither branch applies as written:

- If `pooled` throughput ≥ `shared` throughput **and** `pooled` has zero
  `rowid_mismatches` at every concurrency level **and** `pooled`'s
  `integrity_check` is `ok` at every concurrency level →
  **recommend**: file a follow-up plan to switch `vox-gui`'s `GuiDbPool` from
  one shared `Arc<VoxDb>` to `VoxDbPool`-vended per-command connections,
  default-on for all users. **Not triggered** — `pooled` throughput was ~34%
  lower than `shared` at both tested concurrency levels (33.8% at 32 tasks:
  2712 vs. 4095; 33.9% at 128 tasks: 7965 vs. 12045).
- If `pooled` shows nonzero `busy_errors` or a failed `integrity_check` at
  128 tasks → **do not** default-switch; recommend the writer-actor pattern
  instead. **Also not triggered** — `pooled` had zero `busy_errors` and a
  passing `integrity_check` at both 32 and 128 tasks.
- **State the actual outcome reached:** following the rubric's evident
  intent — safety and speed both matter, and neither should regress — **do
  not switch `vox-gui`'s `GuiDbPool` from its shared-connection pattern to
  `VoxDbPool` by default.** `pooled` is measurably slower than `shared` at
  both tested concurrency levels, with no compensating safety win: the
  specific bug real pooling would fix (the `last_insert_rowid` race) did not
  reproduce under either connection strategy in this testing. This does not
  mean the rowid race isn't real — `GuardedConnection`'s own doc comment
  documents it as a known gap — only that this benchmark gives no
  throughput-driven urgency to fix it via a full pooling-architecture change.
  A narrower fix (e.g. `INSERT ... RETURNING` at the identified call sites)
  would close the theoretical gap without the throughput cost this data shows
  pooling carries, and without the `GuiDbPool::handle()` becoming-`async`
  churn across its 6 current call sites (`chat.rs`, `chat_turn.rs`,
  `harness_eval.rs`, `plan_panel.rs`, `research.rs`, `scientia.rs`) that a
  full pooling switch would require.

## Decision: `VOX_DB_MVCC=1` default

- If `pooled-mvcc` throughput is not meaningfully higher than `pooled` at
  128 tasks in this workload → **do not** flip the default; Turso's own
  docs describe MVCC concurrent writes as beta/early-access with known gaps
  (no `CREATE INDEX` support under MVCC, memory-inefficient row versioning),
  and this measurement gives no throughput reason to accept that risk yet.
  Keep `VOX_DB_MVCC` opt-in.
- If `pooled-mvcc` throughput is meaningfully higher **and** shows zero
  `rowid_mismatches`/`busy_errors` and a passing `integrity_check` →
  recommend a follow-up spike specifically scoped to MVCC's documented
  gaps before proposing default-on.
- **State the actual outcome reached:** this finding is stronger than either
  rubric branch anticipated. The question was framed as throughput-conditioned
  ("is `pooled-mvcc` meaningfully faster?"), but `pooled-mvcc` could not be
  measured at all — it failed setup consistently at both 32 and 128 tasks
  with `Parse error: AUTOINCREMENT is not supported in MVCC mode`, confirmed
  via a clean re-run rather than an environment fluke. **Keep `VOX_DB_MVCC`
  opt-in — not because throughput doesn't clear a bar, but because the
  production schema cannot run under MVCC mode today at all.** This is an
  unconditional "no" independent of any throughput question: MVCC cannot be
  turned on by default while VoxDB's schema uses `AUTOINCREMENT`, full stop.
  Enabling MVCC as a default would require either removing `AUTOINCREMENT`
  from the production schema or Turso adding MVCC support for it — both out
  of scope here.

## Relationship to the prior full-migration audit

`docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md`
already documents the same `GuardedConnection`/`last_insert_rowid` defect at
the same lines and recommends a full engine migration to
`sqlx::sqlite::SqlitePool` (Turso demoted behind `feature = "cloud-sync"`).
This memo evaluates a narrower, cheaper alternative that prior audit does not
mention — real per-task pooling via the already-existing `VoxDbPool`, staying
on Turso.

This memo's measured data is genuinely two-sided on the prior audit's
claims, and both halves should be stated honestly rather than picked between:

- **Supports the prior audit, for isolated single-operation latency.** The
  prior audit's driver-overhead argument for switching to SQLite rests
  partly on latency, and Task 4's point-insert numbers confirm it directly:
  `rusqlite_insert` (1.6965 µs) is roughly 6.7× faster than
  `turso_shared_connection_insert` (11.359 µs) for a single isolated insert.
  For any call path dominated by one-off point operations, the prior audit's
  latency case for SQLite holds up under this measurement.
- **Contradicts the prior audit's implicit concurrency framing.** The prior
  audit's scorecard and recommendation treat moving off Turso as a
  concurrency improvement, but Task 5's throughput data shows the opposite
  under this benchmark's concurrent-write workload: plain `rusqlite` (one OS
  thread + one connection per task — the idiomatic SQLite pooling pattern,
  and structurally the closest analog to `VoxDbPool`'s per-task connections)
  was the **slowest of all three working modes at both concurrency levels**
  — slower than both `shared` and `pooled` Turso configurations, including
  `pooled`, the Turso mode most structurally similar to how SQLite is being
  used here. The likely shared cause: SQLite/Turso's storage engine only
  lets one connection hold the write lock on a local file at a time
  regardless of connection count, so both `pooled` Turso and `sqlite` pay real
  cross-connection lock-acquisition/contention overhead on top of that same
  single-writer constraint without gaining actual write parallelism from it
  — `shared` mode never contends this at all, since only one connection ever
  touches the file, serialized cheaply through an in-process
  `tokio::Mutex`. SQLite likely also carries added OS-thread overhead
  relative to Tokio's cheaper task model in this harness.

**Net assessment:** this memo's data does not on its own justify the prior
audit's full-migration recommendation, and does not contradict it either —
the two evaluate different things. The isolated-latency case for SQLite
remains strong; the concurrent-write-throughput case for switching engines
does not hold up under this benchmark, since the plain-SQLite mode most
comparable to a post-migration architecture underperformed even Turso's
suboptimal `shared` pattern here. Recommend that the prior audit's authors
(or a follow-up) re-scope its throughput/concurrency claims specifically
before treating "switch to SQLite" as a general concurrency fix, rather than
treating this memo's narrower pooling result as either confirming or
superseding that audit's broader migration recommendation.

## Out of scope / follow-ups

- Wiring either winning approach into `vox-gui`'s production `GuiDbPool` —
  separate plan, gated on the decisions above. Per the decisions reached,
  no switch is currently recommended, so this follow-up is not actionable
  today; it would become relevant if a narrower `INSERT ... RETURNING` fix
  or a different pooling strategy changes the throughput picture.
- A full Turso→SQLite engine migration was **not** evaluated here beyond
  this workload-shaped latency/throughput comparison; it remains a larger,
  separately-scoped decision (loses `DbConfig::EmbeddedReplica`/`Remote`,
  used today by `vox-orchestrator`'s mesh sync) — see the reconciliation
  section above for how this memo's data bears on that prior recommendation.
