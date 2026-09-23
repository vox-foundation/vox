---
title: "VoxDB pooling & MVCC benchmark recommendation"
description: "Measured throughput, correctness, and safety comparison of VoxDb's shared-connection pattern vs VoxDbPool vs VoxDbPool+MVCC vs SQLite, with a default-on/default-off recommendation."
category: "Architecture SSOTs"
---

# VoxDB pooling & MVCC benchmark recommendation

> **Revised 2026-09-22** after a final whole-branch review found and fixed two
> measurement confounds in the concurrent-write harness: the `rusqlite`
> connections ran at SQLite's compiled default `synchronous=FULL` while every
> Turso connection got `synchronous=NORMAL`, and each mode's one-time schema
> setup (Turso's ~219-table baseline migration vs. SQLite's single
> `CREATE TABLE`) sat inside the timed window. Every concurrent-write number
> below is from a clean re-run with both fixed, so they differ substantially
> from an earlier version of this memo. The point-insert latency table is
> unchanged — it ran against `:memory:` connections and was never subject to
> either confound.

## Summary

At both tested concurrency levels (32 and 128 tasks, 20 writes/task, on-disk
file), the existing `shared`-connection pattern beat real per-task pooling on
throughput by a wide margin: median 33708 vs. 11447 writes/sec at 32 tasks,
and 38399 vs. 11042 writes/sec at 128 tasks — roughly a **66% deficit for
`pooled` at 32 tasks and ~71% at 128**. (Medians over four repetitions; this
harness is noisy enough that single-run percentages are false precision — see
"Run-to-run variance" below. The ordering, unlike the magnitudes, was stable
in every repetition.) Correcting the durability mismatch roughly tripled
`sqlite`'s measured throughput (median 4949 writes/sec at 32 tasks and 3774 at
128, up from 1686/1587 before the fix), but did **not** change its rank: plain
`rusqlite` with one connection per thread was still the slowest of the three
working modes at both levels, by 6.8–10.2× against `shared` and 2.3–2.9×
against `pooled`. `pooled-mvcc` failed to complete setup at both concurrency
levels — Turso's MVCC journal mode rejects the `AUTOINCREMENT` keyword that
VoxDB's production schema uses, so no throughput data exists for that mode. No
mode that completed (`shared`, `pooled`, `sqlite`) produced any
`misuse_errors`, `busy_errors`, or `rowid_mismatches`, and all three passed
`row_count_ok` and `PRAGMA integrity_check` at both concurrency levels — no
correctness or safety problem was observed in any working configuration, only
a throughput gap in `shared`'s favor. The documented `last_insert_rowid` race
in `GuardedConnection` did not reproduce in any run this session; a follow-up
mutation experiment established that this is because its window is orders of
magnitude too narrow to land naturally, not because the race is absent.

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

20 writes/task, on-disk file, `busy_timeout=5000` and `synchronous=NORMAL`
(Turso and SQLite both). Each mode's one-time schema/connection setup is
outside the timed window.

| mode | tasks | elapsed | throughput (writes/sec) | misuse_errors | busy_errors | rowid_mismatches | row_count_ok | integrity_check |
|---|---|---|---|---|---|---|---|---|
| shared | 32 | 15.83 ms | 40430 | 0 | 0 | 0 | true | ok |
| shared | 128 | 65.47 ms | 39104 | 0 | 0 | 0 | true | ok |
| pooled | 32 | 55.87 ms | 11456 | 0 | 0 | 0 | true | ok |
| pooled | 128 | 432.87 ms | 5914 | 0 | 0 | 0 | true | ok |
| pooled-mvcc | 32 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A | N/A |
| pooled-mvcc | 128 | N/A (setup incompatible) | N/A | N/A | N/A | N/A | N/A | N/A |
| sqlite | 32 | 130.30 ms | 4912 | 0 | 0 | 0 | true | ok |
| sqlite | 128 | 679.11 ms | 3770 | 0 | 0 | 0 | true | ok |

#### Run-to-run variance

The table above is one sample. Three further repetitions of the same six
commands (writes/sec):

| mode | tasks | run 1 | run 2 | run 3 | run 4 | median |
|---|---|---|---|---|---|---|
| shared | 32 | 40430 | 36060 | 31356 | 26064 | 33708 |
| shared | 128 | 39104 | 37694 | 28401 | 43583 | 38399 |
| pooled | 32 | 11456 | 11558 | 11326 | 11437 | 11447 |
| pooled | 128 | 5914 | 10991 | 14054 | 11093 | 11042 |
| sqlite | 32 | 4912 | 4139 | 4986 | 5206 | 4949 |
| sqlite | 128 | 3770 | 2594 | 4449 | 3777 | 3774 |

`pooled`/128 swings 2.4× across repetitions, so **every percentage in this memo
is stated from the medians, as an approximation.** The mode ordering
(`shared` > `pooled` > `sqlite`) held in all four repetitions at both levels
with no overlap between the three modes' ranges; that ordering, not the
magnitudes, is what the decisions below rest on.

**Note on rowid_mismatches:** shared mode's `rowid_mismatches` was 0 at both 32 and 128 tasks. The documented `last_insert_rowid` race in `GuardedConnection` (see its doc comment in `crates/vox-db/src/lib.rs`) did not reproduce in this run. A mutation experiment on `VoxDbPool`'s regression test established why: forcing a genuinely shared connection *still* yields 0 mismatches, because the unguarded window between `execute()` and `last_insert_rowid()` is nanoseconds of synchronous code while a contended `tokio::Mutex` handoff costs microseconds. Inserting a `yield_now().await` into that window makes the shared-connection variant report 981/1000 mismatches while the independent-connection variant stays at 0. The race is real and structurally absent from `VoxDbPool`; it is simply not reachable by wall-clock luck at this scale.

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
  default-on for all users. **Not triggered** — `pooled` throughput was
  roughly 66–71% lower than `shared` at both tested concurrency levels
  (~66% at 32 tasks: median 11447 vs. 33708; ~71% at 128 tasks: median
  11042 vs. 38399), and `pooled` lost in every one of four repetitions at
  both levels.
- If `pooled` shows nonzero `busy_errors` or a failed `integrity_check` at
  128 tasks → **do not** default-switch; recommend the writer-actor pattern
  instead. **Also not triggered** — `pooled` had zero `busy_errors` and a
  passing `integrity_check` at both 32 and 128 tasks.
- **State the actual outcome reached:** following the rubric's evident
  intent — safety and speed both matter, and neither should regress — **do
  not switch `vox-gui`'s `GuiDbPool` from its shared-connection pattern to
  `VoxDbPool` by default.** `pooled` is measurably slower than `shared` at
  both tested concurrency levels — and the corrected measurement makes that
  gap *larger*, not smaller, than the earlier confounded numbers suggested
  (~66–71% rather than ~34%), because the setup cost that previously sat
  inside the timed window was masking it. There is no compensating safety
  win in this data: the specific bug real pooling would fix (the
  `last_insert_rowid` race) did not reproduce under either connection
  strategy. This does not mean the rowid race isn't real —
  `GuardedConnection`'s own doc comment documents it as a known gap, and the
  mutation experiment above reproduces it on demand once the window is
  widened — only that this benchmark gives no throughput-driven urgency to
  fix it via a full pooling-architecture change.
  A narrower fix (e.g. `INSERT ... RETURNING` at the identified call sites)
  would close the theoretical gap without the throughput cost this data shows
  pooling carries, and without the `GuiDbPool::handle()` becoming-`async`
  churn across its 6 current call sites (`chat.rs`, `chat_turn.rs`,
  `harness_eval.rs`, `plan_panel.rs`, `research.rs`, `scientia.rs`) that a
  full pooling switch would require.

### Post-decision update 2026-09-22: a possible data-corruption finding under `VoxDbPool`

After the analysis above was written and committed, running the full `vox-db`
test suite surfaced that `VoxDbPool::new(DbConfig::Memory)`-backed independent
connections, under enough genuine concurrent read/write pressure (the same
100-task, 8-worker-thread configuration as the regression test, widened by a
`yield_now()` added to make that test a reliable gate — see
`crates/vox-db/src/pool.rs`'s test doc comment), panic with
`Corrupt("Invalid page type: 0")` from inside `turso`'s own code in ~55% of
runs (11 of 20 reproduced on this machine, 2026-09-22). This is **not** the
rowid race the test was written to catch — it is the underlying in-memory
B-tree corrupting under concurrent access from multiple "independent"
connections that, for `:memory:` databases, all share one
`Arc<turso::Database>`.

**This strengthens, not weakens, the recommendation above.** It is a second,
independent, and more serious reason not to adopt `VoxDbPool` as `GuiDbPool`'s
default connection strategy: beyond the measured ~66–71% throughput cost, real
per-task pooling now has a reproduced — if not yet fully characterized —
data-corruption failure mode under concurrent load. Whether this also affects
`DbConfig::Local` (file-backed, what production and every other benchmark in
this memo actually measures) is **untested and unknown**; this plan makes no
production code changes, so characterizing, reproducing against a real file,
and either fixing upstream in `turso` or documenting it as a hard constraint
on `VoxDbPool`'s `:memory:` mode is out of scope here and should be tracked as
a separate, higher-priority follow-up — this finding has direct bearing on
"does it break our DB," independent of any throughput question. The
regression test that surfaced this is `#[ignore]`d rather than deleted or
weakened, specifically so this finding is not lost; see its doc comment for
the exact reproduction procedure.

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
- **Contradicts the prior audit's implicit concurrency framing — and this
  survives the durability correction.** The prior audit's scorecard and §3.1
  throughput table treat moving off Turso as a concurrency improvement,
  predicting a "Max Concurrent Write TPS" of ~3,500–6,000 for standard SQLite
  against ~2,000–3,500 for Turso local. The corrected measurement splits that
  prediction in half. Its **SQLite** figure holds up well: measured `sqlite`
  medians are 4949 writes/sec at 32 tasks and 3774 at 128, squarely inside the
  predicted band. Its **Turso** figure does not: measured `shared` medians are
  33708 / 38399 and `pooled` 11447 / 11042 — 3× to 11× above the predicted
  ceiling. So the audit's *relative* claim is inverted by this workload: plain
  `rusqlite` (one OS thread + one connection per task — the idiomatic SQLite
  pooling pattern, and structurally the closest analog to `VoxDbPool`'s
  per-task connections) was the **slowest of all three working modes at both
  concurrency levels in all four repetitions**, slower than both `shared` and
  `pooled` Turso. The original version of this memo reached the same ordering
  from a confounded comparison, so it is worth stating explicitly that the
  correction *did* matter and *did not* change the answer: matching
  `synchronous=NORMAL` roughly tripled SQLite's measured throughput
  (1686 → 4949 at 32 tasks, 1587 → 3774 at 128) without closing the gap. A
  plausible shared cause for both `pooled` and `sqlite` trailing `shared`:
  SQLite/Turso's storage engine only lets one connection hold the write lock
  on a local file at a time regardless of connection count, so both pay real
  cross-connection lock-acquisition/contention overhead on top of that same
  single-writer constraint without gaining actual write parallelism from it —
  `shared` mode never contends this at all, since only one connection ever
  touches the file, serialized cheaply through an in-process `tokio::Mutex`.
  SQLite likely also carries added OS-thread overhead relative to Tokio's
  cheaper task model in this harness.

**Net assessment:** this memo's data does not on its own justify the prior
audit's full-migration recommendation, and does not contradict it either —
the two evaluate different things. The isolated-latency case for SQLite
remains strong; the concurrent-write-throughput case for switching engines
does not hold up under this benchmark, since the plain-SQLite mode most
comparable to a post-migration architecture underperformed even Turso's
suboptimal `shared` pattern here.

**Recommendation to the prior audit's authors (restated on the corrected
evidence).** The earlier version of this recommendation was reached from a
comparison with a durability mismatch in it, so it is re-derived here rather
than carried over. It still holds, and now with a sharper target: §3.1's
"Max Concurrent Write TPS" row should be re-scoped, because its SQLite
estimate is confirmed by measurement while its Turso estimate is low by
roughly an order of magnitude, and the ordering the row implies is the
reverse of what this workload produces. Until that row is re-measured,
"switch to SQLite" should not be presented as a general concurrency or
write-throughput fix. The audit's other pillars are untouched by this data
and remain live on their own merits: the `last_insert_rowid` race (§2.2),
which this branch independently reproduced on demand under a widened window;
the `GuardedConnection` global-mutex serialization of reads; the `libclang`
build-invariant and binary-size arguments; and the point-latency case in
§3.1, which Task 4's Criterion numbers confirm. Nothing here argues against
migrating for those reasons — only against citing concurrent-write
throughput as one of them.

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
