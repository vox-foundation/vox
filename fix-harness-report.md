# concurrency_stress.rs — code review fixes + new mode

File: `crates/vox-db/examples/concurrency_stress.rs` (only file touched, as instructed).

## Fix 1 + Fix 5 (data loss / stale state) — done together

Root cause was shared: `run_sqlite` unconditionally `std::fs::remove_file(&path)`'d
whatever `--file` pointed at, while the Turso modes never reset the probe table at
all (`CREATE TABLE IF NOT EXISTS`), so a second run against the same file either
lost real data (sqlite) or accumulated stale rows (Turso).

Fix: removed the `remove_file` call entirely (a comment now documents that the
file is never deleted). Replaced the `CREATE_PROBE` const with:

```rust
const RESET_PROBE: &str = "DROP TABLE IF EXISTS stress_probe;
CREATE TABLE stress_probe (
    id INTEGER PRIMARY KEY,
    marker TEXT NOT NULL
)";
```

Used via `execute_batch` at all four call sites (`run_turso_shared`,
`run_turso_pooled`, `run_sqlite`, and the new `run_sqlite_shared`) — Turso's and
rusqlite's `Connection::execute_batch` both accept this multi-statement string
directly, so one const covers every engine.

## Fix 6 — read-back errors miscounted as rowid mismatches

`verify_and_count` (Turso) and the inline read-back in `run_sqlite` both had a
catch-all error arm that counted `rowid_mismatches`. Changed both so only "got a
row back, content is wrong" increments `rowid_mismatches`; a query failure or an
unexpectedly-missing row now goes through `classify_error` (so it surfaces as
`busy_errors`/`other_errors`), matching how insert errors are already classified.

## Fix 7 — throughput counted attempted writes, not successful ones

`main`'s throughput line now divides `stats.ok` (writes that actually succeeded)
by elapsed time instead of `expected_rows` (writes attempted). Also clamped the
denominator with `elapsed.as_secs_f64().max(f64::MIN_POSITIVE)` to avoid
`inf`/`NaN` on a near-zero elapsed time.

## Fix 8 — pooled-mvcc silently exited 0 on any failure

`run_turso_pooled`'s `Err(e) if mvcc` arm now only takes the graceful
exit(0)+explanation path when the error text actually contains `"AUTOINCREMENT"`
or `"MVCC"`. Any other error under `mvcc` falls through to the same
`panic!("pool init: {msg}")` the non-mvcc path uses.

**Real error text observed** (`--mode pooled-mvcc --tasks 16 --writes-per-task 5`):

```
setup_error       = Parse error: AUTOINCREMENT is not supported in MVCC mode (journal_mode=experimental_mvcc)
```

Contains `"AUTOINCREMENT"`, so the condition matches as designed.

## Fix 9 — integrity-check failure lost all other run stats

`check_integrity` no longer prints or asserts — it just returns the
`PRAGMA integrity_check` result string. Each `run_*` function's return tuple now
carries that string as a 5th element. `main` prints every stat (including
`integrity_check`) first, and only `assert_eq!`s on it afterward — so a
corruption finding is visible on stdout even if the process then panics.
`run_sqlite`'s and the new `run_sqlite_shared`'s inline integrity checks were
changed the same way.

## New feature: `sqlite-shared` mode

Added `run_sqlite_shared`: one `rusqlite::Connection` wrapped in
`Arc<Mutex<_>>`, shared across `std::thread::spawn` worker threads — the
like-for-like counterpart to Turso's `shared` mode (single connection,
serialized in-process, no OS file-lock contention on either side). Wired into
`main`'s mode dispatch (`"sqlite-shared" => run_sqlite_shared(&args)`) and into
the module doc comment and the `unknown --mode` panic text (now lists
`shared|pooled|pooled-mvcc|sqlite|sqlite-shared`). After joining all worker
threads, the Arc/Mutex is unwrapped (`Arc::try_unwrap` + `into_inner`) before
the final count/integrity queries so they run against a plain `Connection`.

## Verification

1. `cargo build -p vox-db --example concurrency_stress` — clean, exit 0.
2. `cargo clippy -p vox-db --all-targets -- -D warnings` — clean, exit 0, no warnings.
3. All five modes run once at small scale (`--tasks 16 --writes-per-task 5`):

   - `shared`: `row_count_ok = true`, `integrity_check = ok`, 0 errors, 0 mismatches.
   - `pooled`: `row_count_ok = true`, `integrity_check = ok`, 0 errors, 0 mismatches.
   - `pooled-mvcc`: graceful exit 0 with the real AUTOINCREMENT/MVCC error text
     (see Fix 8 above).
   - `sqlite`: `row_count_ok = true`, `integrity_check = ok`, 0 errors, 0 mismatches.
   - `sqlite-shared`: `row_count_ok = true`, `integrity_check = ok`, 0 errors, 0 mismatches.

4. **Two-runs-same-file proof (Fix 1+5).** For both `sqlite` and `sqlite-shared`,
   ran twice against the same scratch file without deleting it in between.
   Second run's `actual_rows` stayed at 80 (== `expected_rows`, not 160), and
   `row_count_ok = true` both times — proving the probe table is reset per run
   with no leftover rows and no file deletion required.

   `sqlite` run 1 → run 2 (same `/tmp/verify-fix-sqlite.db`):
   ```
   # run 1
   actual_rows       = 80
   row_count_ok      = true
   integrity_check   = ok
   # run 2 (same file, no delete)
   actual_rows       = 80
   row_count_ok      = true
   integrity_check   = ok
   ```

   `sqlite-shared` run 1 → run 2 (same `/tmp/verify-fix-sqlite-shared.db`):
   ```
   # run 1
   actual_rows       = 80
   row_count_ok      = true
   integrity_check   = ok
   # run 2 (same file, no delete)
   actual_rows       = 80
   row_count_ok      = true
   integrity_check   = ok
   ```

## Real-scale `sqlite-shared` run (data for the memo rewrite)

`cargo run --release -p vox-db --example concurrency_stress -- --mode sqlite-shared --tasks 128 --writes-per-task 20 --file /tmp/sqlite-shared-real-run.db`
(fresh empty file, release build):

```
mode              = sqlite-shared
tasks             = 128
writes_per_task   = 20
elapsed           = 36.726416ms
throughput        = 69705 writes/sec
expected_rows     = 2560
actual_rows       = 2560
row_count_ok      = true
ok_writes         = 2560
misuse_errors     = 0
busy_errors       = 0
other_errors      = 0
rowid_mismatches  = 0
integrity_check   = ok
```

This is the true like-for-like comparison point against Turso's `shared` mode
(single connection, serialized, no file-lock contention on either side) that the
memo rewrite needs.
