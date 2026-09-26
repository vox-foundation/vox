---
title: "VoxDB pooling & MVCC benchmark recommendation"
description: "Measured throughput, correctness, and safety comparison of VoxDb's shared-connection pattern vs VoxDbPool vs VoxDbPool+MVCC vs SQLite, with a default-on/default-off recommendation."
category: "Architecture SSOTs"
---

# VoxDB pooling & MVCC benchmark recommendation

> **Revised 2026-09-22 (twice).** First revision: a final whole-branch review
> found and fixed two measurement confounds in the concurrent-write harness —
> `rusqlite` connections ran at SQLite's compiled default `synchronous=FULL`
> while every Turso connection got `synchronous=NORMAL`, and each mode's
> one-time schema setup (Turso's ~219-table baseline migration vs. SQLite's
> single `CREATE TABLE`) sat inside the timed window. Second revision: a
> subsequent code review found the original `sqlite` mode still wasn't a fair
> comparison against Turso's `shared` mode — `shared` serializes all tasks
> through **one** connection and never contends the OS file lock, while
> `sqlite` opens **independent, contending** connections. A new `sqlite-shared`
> mode (one `rusqlite` connection behind a mutex, matching `shared`'s
> architecture exactly) is the first genuinely apples-to-apples cross-engine
> comparison in this document, and it changes the cross-engine conclusion
> substantially — see "Relationship to the prior full-migration audit" below.
> The point-insert latency table is unaffected by either revision — it ran
> against `:memory:` connections and was never subject to any of these
> confounds.

## Summary

At both tested concurrency levels (32 and 128 tasks, 20 writes/task, on-disk
file), the existing `shared`-connection pattern beat real per-task pooling on
throughput by a wide margin: median 33708 vs. 11447 writes/sec at 32 tasks,
and 38399 vs. 11042 writes/sec at 128 tasks — roughly a **66% deficit for
`pooled` at 32 tasks and ~71% at 128**. (Medians over four repetitions; this
harness is noisy enough that single-run percentages are false precision — see
"Run-to-run variance" below. The ordering, unlike the magnitudes, was stable
in every repetition.) `pooled-mvcc` failed to complete setup at both
concurrency levels — Turso's MVCC journal mode rejects the `AUTOINCREMENT`
keyword that VoxDB's production schema uses, so no throughput data exists for
that mode, and root-cause work later confirmed this is a schema/engine
incompatibility, not an environment fluke. No mode that completed produced any
`misuse_errors`, `busy_errors`, or `rowid_mismatches`, and all passed
`row_count_ok` and `PRAGMA integrity_check` — no correctness or safety problem
was observed in any working Turso or SQLite configuration, only throughput
differences. The documented `last_insert_rowid` race in `GuardedConnection`
did not reproduce naturally in any run this session; a follow-up mutation
experiment confirmed the race is real and the pool's independent-connection
design structurally prevents it, but that its natural window is orders of
magnitude too narrow to land at this scale without deliberately widening it.

**A separate `sqlite` finding changed materially after this memo was first
drafted.** The original `sqlite` mode (independent, contending connections)
measured plain `rusqlite` as the slowest of all four modes — correcting a
durability-pragma mismatch roughly tripled its throughput (1686/1587 →
4949/3774 writes/sec) but didn't change that rank. A later code review found
the comparison itself was unfair: `shared` never contends the OS file lock
(one connection, serialized in-process), while `sqlite` opens up to 128
independent, genuinely-contending connections. A new `sqlite-shared` mode —
one `rusqlite` connection behind a mutex, architecturally identical to
`shared` — is the actual apples-to-apples comparison, and it **beats every
other mode measured, including Turso's `shared`, by roughly 2.1×** (median
71162 vs. 33708 at 32 tasks; 79165 vs. 38399 at 128 tasks). This does not
change the pooling recommendation below (that comparison is Turso-internal
and unaffected), but it substantially changes what this memo says about the
prior full-migration audit — see "Relationship to the prior full-migration
audit."

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
| sqlite-shared | 32 | 36.92 ms | 69361 | 0 | 0 | 0 | true | ok |
| sqlite-shared | 128 | 32.74 ms | 78196 | 0 | 0 | 0 | true | ok |

`sqlite-shared` (added after a code review flagged the fairness gap below) is
one `rusqlite::Connection` behind `Arc<Mutex<_>>`, shared across worker
threads exactly as `shared` shares one Turso connection — no OS file-lock
contention on either side, unlike `sqlite`'s independent, contending
connections.

#### Run-to-run variance

The table above is one sample. Three further repetitions of the same
commands (writes/sec):

| mode | tasks | run 1 | run 2 | run 3 | run 4 | median |
|---|---|---|---|---|---|---|
| shared | 32 | 40430 | 36060 | 31356 | 26064 | 33708 |
| shared | 128 | 39104 | 37694 | 28401 | 43583 | 38399 |
| pooled | 32 | 11456 | 11558 | 11326 | 11437 | 11447 |
| pooled | 128 | 5914 | 10991 | 14054 | 11093 | 11042 |
| sqlite | 32 | 4912 | 4139 | 4986 | 5206 | 4949 |
| sqlite | 128 | 3770 | 2594 | 4449 | 3777 | 3774 |
| sqlite-shared | 32 | 69361 | 71216 | 71108 | 74331 | 71162 |
| sqlite-shared | 128 | 78196 | 62247 | 80133 | 80191 | 79165 |

`pooled`/128 and `sqlite-shared`/128 both swing over 2× across repetitions, so
**every percentage in this memo is stated from the medians, as an
approximation.** The mode ordering (`sqlite-shared` > `shared` > `pooled` >
`sqlite`) held in every repetition at both levels with no overlap between any
two modes' ranges; that ordering, not the magnitudes, is what the decisions
below rest on.

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

**Root cause confirmed, 2026-09-22.** A follow-up experiment ruled out an
alternative hypothesis — that concurrent `apply_pragmas()` calls (every
`pool.get()` re-issues several pragmas) were the trigger rather than
concurrent row writes. Serializing all 100 connection acquisitions before any
concurrent write traffic began still corrupted the database in 11/20 runs
(55%), statistically indistinguishable from the 10/20 (50%) baseline with
pragmas and writes racing freely, and with the identical corruption signature.
This confirms genuine concurrent read/write traffic — not pragma
concurrency — is the cause. Default `cargo test -p vox-db --lib` coverage of
`VoxDbPool::get()`'s independence was restored via a second, non-`#[ignore]`d
test using the original (pre-yield) pattern, honestly documented as having
weaker detection power against this specific race.

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

> **Narrowed 2026-09-25 (next section).** The file-backed question is now
> answered: the failure did not reproduce on on-disk databases, and it
> reproduces on bare Turso `:memory:` connections with no vox-db code. The
> corruption argument above therefore does not apply to production
> (`DbConfig::Local`); the throughput argument stands on its own.

### Post-decision update 2026-09-25: file-backed databases do not reproduce; the bug is Turso `:memory:`

**Question.** Does the `Corrupt("Invalid page type: 0")` failure above also
happen on file-backed (`DbConfig::Local`) databases?

**Harness.** `crates/vox-db/tests/pool_corruption_probe.rs` (all tests
`#[ignore]`d). The workload is the same as the `pool.rs` reproducer: 100 tasks
on an 8-worker multi-threaded runtime, 10 writes per task, and for each write
`INSERT`, then `yield_now()`, then `last_insert_rowid()`, then `SELECT` the row
back. Each trial is a separate process. File variants use a fresh temp file
per trial (`-wal`/`-shm` removed afterwards). After the workload, each trial
runs `PRAGMA integrity_check`. A trial fails if any task panics or the check
is not `ok`.

| Variant | Connections | Backing | Failed trials | Failure signatures (first panic per failing trial) |
|---|---|---|---|---|
| `pooled_memory` (control) | `VoxDbPool`, one `get()` per task | `DbConfig::Memory` | **10 / 20** | 6× `Corrupt("Invalid page type: 0")` on `select back`, 4× `row present` (the `SELECT` returned no row for the id that was just inserted) |
| `pooled_file` | `VoxDbPool`, one `get()` per task | `DbConfig::Local`, fresh file | **0 / 100** | none |
| `shared_file` | one `VoxDb`, `Arc`-shared across tasks | `DbConfig::Local`, fresh file | **0 / 100** | none (976–990 of 1000 `rowid_mismatches` per trial, which is the known `last_insert_rowid` race on a shared connection, not corruption) |
| `raw_turso_memory` | plain `turso::Builder`, one `db.connect()` per task, no vox-db code | `:memory:` | **11 / 20** | 9× `Corrupt("Invalid page type: 0")` on `row query`, 2× `row present` |
| `raw_turso_file` | plain `turso::Builder`, one `db.connect()` per task, no vox-db code | fresh file | **0 / 100** | none |

The raw variants set only `journal_mode=WAL` and `busy_timeout=5000`. Without
`busy_timeout`, 99 of 100 tasks fail immediately with `Busy`. They run no
migrations and no other vox-db pragmas, and they do not use
`GuardedConnection`. The `pooled_memory` control uses this new harness rather
than reusing the earlier 11/20 figure, and at 10/20 it agrees with that figure.

Commands (Turso 0.6.1, macOS arm64, 2026-09-25). Each trial is one process:

```text
cargo test -p vox-db --test pool_corruption_probe --no-run
# repeated N times per variant, one process per trial:
target/debug/deps/pool_corruption_probe-<hash> --ignored --exact <variant> --nocapture
```

**Findings.**

1. **File-backed databases did not reproduce it.** Across 300 file-backed
   trials (pooled, shared, and raw) there were zero task panics and zero
   `integrity_check` failures. The same harness failed in about half of the
   `:memory:` trials. At zero failures in 100 trials, the 95% upper bound on
   the per-trial rate for each file variant is about 3%, against about 50%
   for memory.
2. **It is a Turso engine bug, not a vox-db bug.** It reproduces at the same
   rate with bare `turso::Builder::new_local(":memory:")` connections and no
   vox-db code involved: no migration, no `apply_pragmas`, and no
   `GuardedConnection`. `VoxDbPool` only exposes the bug, because it opens
   several connections on one in-memory `turso::Database`.
3. **The failure is transient on the read path. It was not detected as
   persistent corruption.** In all 40 memory trials, including the 21 that
   failed, the `PRAGMA integrity_check` that ran afterwards returned `ok`.
   The symptom is a reader that sees a zeroed page (`Invalid page type: 0`),
   or that does not see its own just-committed row (`row present`). Caveat:
   this conclusion depends on how complete Turso's `integrity_check` is,
   which this investigation did not audit.

**Effect on the recommendations.**

- *Do not switch `GuiDbPool` to `VoxDbPool`.* This is unchanged, but it now
  rests on throughput alone (a 66–71% deficit). The corruption finding is no
  longer an argument against file-backed pooling, which is how production
  would use it.
- *New constraint: do not use `VoxDbPool` with `DbConfig::Memory` for
  concurrent work.* Also do not use any other pattern that opens several
  connections on one in-memory `turso::Database`. It currently has no
  consumers outside `vox-db`. Tests that need concurrent pooled access should
  use a temp file.
- The rowid-race gate in `pool.rs` stays `#[ignore]`d on `:memory:`. Moving it
  to a temp file would probably make it a stable gate that could run by
  default. This session did not do that because it is a separate change.
- **Upstream:** `raw_turso_memory` is a self-contained reproducer with no vox
  code, suitable for a Turso issue. This investigation did not file one and
  did not look for a root cause inside Turso.

**What was not measured.** Only one workload shape was tested (100 tasks and
single-row inserts on one table). No larger rows, page splits under heavier
load, or checkpoint pressure were exercised, so the result is strong evidence
that file-backed databases behave differently, not proof that they are
immune. The `concurrency_stress` example was not used for this question
because it has no `yield_now()` between the insert and the read-back. That
yield is what widens the window enough for the failure to appear.

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

This memo went through two rounds of correction on this section — the first
fixed a durability-pragma mismatch, the second (below) fixed an unfair
architecture mismatch in the SQLite comparison itself. The current picture is
more decisive than either earlier version, and different in an important way:

- **Supports the prior audit, for isolated single-operation latency
  (unchanged).** The prior audit's driver-overhead argument for switching to
  SQLite rests partly on latency, and Task 4's point-insert numbers confirm
  it directly: `rusqlite_insert` (1.6965 µs) is roughly 6.7× faster than
  `turso_shared_connection_insert` (11.359 µs) for a single isolated insert.
- **Supports the prior audit's concurrency framing too, once compared
  fairly — this is the finding that changed.** A code review of this branch
  caught that the original `sqlite` mode wasn't a fair counterpart to
  Turso's `shared` mode: `shared` serializes every task through **one**
  connection and never contends the OS file lock, while `sqlite` opens
  **independent, contending** connections — comparing "no contention" against
  "real contention," not comparing engines. The new `sqlite-shared` mode
  (one `rusqlite` connection behind a mutex, architecturally identical to
  `shared`) is the actual apples-to-apples comparison, and on it, plain
  SQLite **beats Turso's best-measured mode by ~2.1×** at both concurrency
  levels (median 71162 vs. 33708 at 32 tasks; 79165 vs. 38399 at 128 tasks).
  The prior audit's §3.1 predicted a "Max Concurrent Write TPS" of
  ~3,500–6,000 for SQLite against ~2,000–3,500 for Turso local — this
  measurement lands well above both bands for both engines (this harness's
  minimal single-table workload is not the audit's, so absolute magnitudes
  aren't meant to match), but the *relative* claim — SQLite ahead of Turso
  on concurrent writes — now holds, where an earlier, unfairly-compared
  version of this memo said the opposite.
- **The same data also shows *why* the earlier comparison got this
  backwards, and it's the more important lesson.** Connection architecture,
  not engine choice, is what actually dominates this workload. Within each
  engine, a single serialized connection beats independent contending
  connections by a wide margin: Turso's `shared`/`pooled` differ by ~3.5×
  (38399 vs. 11042 at 128 tasks); SQLite's `sqlite-shared`/`sqlite` differ by
  **~21×** (79165 vs. 3774). SQLite/Turso's storage engine only lets one
  connection hold the write lock on a local file at a time regardless of
  connection count, so independent connections on either engine pay real
  cross-connection lock-acquisition/contention overhead on top of that same
  single-writer constraint, without gaining actual write parallelism from it.
  `shared` and `sqlite-shared` never pay this at all — one connection each,
  serialized cheaply in-process. The original `sqlite` mode measured the
  *worst* architecture (independent, contending) against Turso's *best*
  (serialized); that mismatch, not anything about SQLite itself, is what
  made the earlier version of this memo conclude SQLite was slowest.

**Net assessment — sharper than either earlier version.** The isolated-latency
case for SQLite remains strong and is now joined by a real, fairly-measured
concurrent-write case: for the one architecture pattern this workload
rewards (single serialized connection), plain SQLite is genuinely faster than
Turso. But this is *not* a case for "switch to `sqlx::SqlitePool` with a
normal multi-connection pool" — that would land near the `sqlite` (independent
connections) number, not `sqlite-shared`, and `sqlite` was the single slowest
mode measured in this entire investigation. Any future migration argument
built on this data has to specifically preserve the serialized-single-
connection pattern (or design an equivalent, such as a writer-actor), not
assume a conventional connection pool inherits SQLite's raw-engine speed
advantage — the data here shows the opposite: independent connections erase
SQLite's ~2.1× engine advantage and replace it with roughly a 10× *deficit*
against Turso's `shared` mode.

**Recommendation to the prior audit's authors (restated a second time, on the
now-fair comparison).** §3.1's "Max Concurrent Write TPS" row can stand as
directionally correct — SQLite ahead of Turso on concurrent writes is what a
fair, architecture-matched measurement shows here — but the row's framing as
a property of the *engines* rather than of *connection architecture* is
incomplete enough to be actively misleading: read on its own, it invites
exactly the naive-migration reading the paragraph above warns against. A
revision should state the finding as "SQLite outperforms Turso by ~2× under
matched single-connection-serialized access; both engines lose an order of
magnitude or more of throughput under independent-connection contention," not
as an unqualified engine ranking. The audit's other pillars are untouched by
this data and remain live on their own merits: the `last_insert_rowid` race
(§2.2), which this branch independently reproduced on demand under a widened
window (and found a related, more severe `:memory:` corruption bug alongside
it — see the pooling decision's post-decision update above); the
`GuardedConnection` global-mutex serialization of reads; the `libclang`
build-invariant and binary-size arguments; and the point-latency case in
§3.1, which Task 4's Criterion numbers confirm.

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
