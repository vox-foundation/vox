---
title: "Research Trial Tier A Schema Contract (STORE-01)"
description: "Canonical campaign, run and sample schema for research trials and Phase 12 code benchmarks: DDL at baseline 97, vocabularies, invariants, run grain, and the model_benchmark_results boundary, awaiting human approval."
category: "Architecture SSOTs"
status: "roadmap"
training_eligible: false
---

# Research Trial Tier A Schema Contract (STORE-01)

This is the single reviewed source for the Tier A trial tables. Plan 07-06 copies the DDL
below verbatim into a vox-db domain fragment. **No migration code exists until the Decision
record at the bottom reads `Decision status: approved` (D-04).**

This is a one-way door. Once baseline 97 has created these tables on a shared database,
changing a column needs an explicit migration on every existing database.

## 1. Scope and home (D-01)

- The tables live in a new, active vox-db domain fragment named `trials`. They are not a
  scientia extension.
- The quarantined `scientia_prereg` domain is untouched.
- Research campaigns and Phase 12 code benchmarks share one identity and one table set.
- Tier A holds ids, hashes, versions, counters and bounded statuses only. Evidence storage
  (Tier C) is Phase 9 (STORE-03); until then campaigns are observational and never replay- or
  promotion-eligible.

## 2. Baseline (D-05)

- `main` was at baseline 94; baseline 95 belongs to `model_benchmark_results` (commit
  `139bdd20e`), and baseline 96 belongs to `kb` Phase 3 (`kb_findings` inbox and trust-tier
  filtering on `claude/vox-knowledge-base-research-2964d2`). The next free baseline in the
  unified stream is **97**.
- Re-check `origin/main` and open PRs immediately before the migration commit.
- **Loud guard.** When `schema_version` equals the baseline (97) but any of the four trial
  tables is missing, `migrate` fails with
  `StoreError::BaselineStampedWithoutTables { version, missing }` and remediation text. The
  failure mode it prevents: another branch or worktree binary stamps 97 on the shared store
  without creating these tables, and the first trial query then dies with "no such table".
- **Operational effect, stated plainly.** A database stamped 97 is refused by 94, 95, and 96
  binaries through the existing `LegacySchemaChain` guard, exactly as a database stamped 95 is
  refused by 94 binaries today. Databases already at 95 or 96 pick up the new tables on first open
  (`CREATE TABLE IF NOT EXISTS`); no ALTER is needed.

## 3. Run grain (gate question 1)

**Recommended: item-batch.** A run is one arm attempt over an item batch. `item_set_hash` and
`item_count` live on the run; `item_id` and `sample_index` live on samples.

- Research uses a batch of size 1 (one run per arm x item x replicate x attempt).
- Code benchmarks use one batch per invocation and arm, with `item_set_hash` equal to the
  problem-set hash and samples keyed by (fixture id, sample index).

| Grain | What a run is | Phase 8 reservations | Phase 12 (12-09) |
|---|---|---|---|
| **item-batch (recommended)** | one arm attempt over a batch | reserve per run; research gets per-item reservation by using batch size 1 | one run per invocation and arm; one provenance blob; samples per fixture |
| per-item | one arm x item x replicate x attempt | simplest research mapping; reserve per item | must persist one run per fixture-sample; `provenance_json` repeats on every row |
| per-invocation | one whole benchmark invocation across arms | cannot reserve per arm or per item without extra rows; reservation granularity too coarse for racing | simplest bench mapping; research cannot fit (arms and replicates collapse into one row) |

## 4. DDL

Applied by plan 07-06 verbatim. Every statement is idempotent. There are no check
constraints and no triggers (Turso rejects both), no content columns (G24), money is integer
micros, and campaigns and runs use text primary keys. The two self-references
(`supersedes_campaign_id`, `retry_of_run_id`) carry no foreign key, because row order inside
one table is not guaranteed on legacy import; both are validated in Rust inside the writing
transaction. The event table is named `trial_campaigns_log` so the alphabetical
`LEGACY_EXPORT_TABLES` order stays parent-before-child
(`trial_campaigns` < `trial_campaigns_log` < `trial_runs` < `trial_samples`).

```sql
CREATE TABLE IF NOT EXISTS trial_campaigns (
    campaign_id            TEXT    PRIMARY KEY,   -- Trusty URI of canonical prereg bytes (id blanked)
    schema_version         INTEGER NOT NULL,
    kind                   TEXT    NOT NULL,      -- observational|exploratory|holdout_confirmation (Rust-validated)
    state                  TEXT    NOT NULL,      -- signed|running|completed|failed|stopped (Rust-validated)
    stop_reason            TEXT,                  -- operator|superseded|validity (Rust-validated)
    prereg_json            TEXT    NOT NULL,      -- canonical bytes exactly as signed; never updated
    signer_pubkey_hex      TEXT    NOT NULL,
    signature_hex          TEXT    NOT NULL,
    signed_at_ms           INTEGER NOT NULL,
    supersedes_campaign_id TEXT,                  -- no FK (same-table, import-order safe); checked in Rust
    converted_from         TEXT,
    owner_instance_id      TEXT,
    last_heartbeat_ms      INTEGER,
    created_at_ms          INTEGER NOT NULL,
    updated_at_ms          INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS trial_campaigns_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,   -- append-only transition log; never read by rowid
    campaign_id TEXT    NOT NULL REFERENCES trial_campaigns(campaign_id),
    from_state  TEXT,
    to_state    TEXT    NOT NULL,
    reason_code TEXT,
    at_ms       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_trial_campaigns_log_campaign
    ON trial_campaigns_log(campaign_id);

CREATE TABLE IF NOT EXISTS trial_runs (
    run_id               TEXT    PRIMARY KEY,
    campaign_id          TEXT    NOT NULL REFERENCES trial_campaigns(campaign_id),
    arm_id               TEXT    NOT NULL,
    replicate_id         INTEGER NOT NULL,
    attempt_index        INTEGER NOT NULL,
    retry_of_run_id      TEXT,                    -- no FK (same-table, import-order safe); checked in Rust
    item_set_hash        TEXT    NOT NULL,
    item_count           INTEGER NOT NULL,
    state                TEXT    NOT NULL,        -- running|succeeded|failed|interrupted (Rust-validated)
    error_class          TEXT,
    research_session_id  INTEGER,                 -- scientia_research_sessions.id when research; join key is session_key trial:{run_id}
    provenance_json      TEXT    NOT NULL,        -- D-09: canonical run provenance (hashes and ids only)
    syntax_fingerprint   TEXT,                    -- D-09 indexed scalars
    model_id             TEXT,
    condition            TEXT,
    problem_set_hash     TEXT,
    run_complete         INTEGER NOT NULL,        -- 0/1
    tokens_in            INTEGER,
    tokens_out           INTEGER,
    cost_usd_micros      INTEGER,                 -- NULL = Missing: at least one call reported no cost
    llm_calls            INTEGER,
    tool_calls           INTEGER,
    duration_ms          INTEGER,
    ttft_kind            TEXT    NOT NULL,        -- not_applicable|measured (Rust-validated)
    ttft_ms              INTEGER,
    served_from_cache    INTEGER NOT NULL DEFAULT 0,
    owner_instance_id    TEXT,
    last_heartbeat_ms    INTEGER,
    started_at_ms        INTEGER NOT NULL,
    finished_at_ms       INTEGER
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_trial_runs_identity
    ON trial_runs(campaign_id, arm_id, item_set_hash, replicate_id, attempt_index);

CREATE INDEX IF NOT EXISTS idx_trial_runs_state
    ON trial_runs(campaign_id, state);

CREATE INDEX IF NOT EXISTS idx_trial_runs_model
    ON trial_runs(model_id, condition, run_complete);

CREATE TABLE IF NOT EXISTS trial_samples (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id         TEXT    NOT NULL REFERENCES trial_runs(run_id),
    item_id        TEXT    NOT NULL,              -- query hash or fixture id, never query text
    sample_index   INTEGER NOT NULL,
    outcome        TEXT    NOT NULL,              -- ok|failed|abstained|infra_error (Rust-validated)
    failure_class  TEXT,
    scores_json    TEXT    NOT NULL,              -- versioned typed metric blob; NotApplicable/Missing explicit
    evidence_ref   TEXT,                          -- reserved for Phase 9 Tier C; NULL now
    recorded_at_ms INTEGER NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_trial_samples_key
    ON trial_samples(run_id, item_id, sample_index);
```

Notes on columns:

- `cost_usd_micros` is nullable on purpose. NULL means Missing (at least one call reported no
  cost), never zero.
- `ttft_kind` is `not_applicable` for the whole research path today (non-streaming), with
  `ttft_ms` NULL. A measured value appears only when a streaming path records one.
- `served_from_cache = 1` rows are stratified in analysis; they are not comparable on tokens
  or latency.
- "Attempt" is not a table: it is `attempt_index` plus `retry_of_run_id` on `trial_runs`.

## 5. Vocabularies (validated in Rust, not in the database)

| Field | Values |
|---|---|
| campaign kind | `observational`, `exploratory`, `holdout_confirmation` |
| campaign state | `signed`, `running`, `completed`, `failed`, `stopped` |
| stop_reason | `operator`, `superseded`, `validity` (Phase 8 may add `spend_cap`, `safety`, `wall_time` with no migration) |
| run state | `running`, `succeeded`, `failed`, `interrupted` (Phase 8 adds `reserved`) |
| sample outcome | `ok`, `failed`, `abstained`, `infra_error` |
| ttft_kind | `not_applicable`, `measured` |

## 6. Invariants

- Lifecycle transitions are enforced by a conditional UPDATE plus `changed rows == 1`:
  `signed -> running`, `running -> terminal`, `signed -> stopped`.
- Terminal rows are never updated.
- `trial_campaigns_log` is append-only: no UPDATE or DELETE code path exists.
- Superseding a non-terminal campaign moves it to `stopped` with reason `superseded` in the
  same transaction as the new campaign row.
- No rowid reads (`last_insert_rowid()` is racy on the shared connection); text primary keys
  or `RETURNING` only.
- Self-references (`supersedes_campaign_id`, `retry_of_run_id`) are checked in Rust inside
  the writing transaction.

## 7. Atomicity (STORE-02)

- One exclusive transaction per campaign creation and per run batch (terminal run update plus
  its samples), held across the whole closure on the shared connection.
- A public `PersistFailpoint` parameter exists for tests in any crate (not `cfg(test)`),
  shaped `PersistFailpoint::{None, AfterStatements(u32)}`.
- A forced mid-write failure leaves no partial run and no partial samples.

## 8. Identity mapping (TRIAL-01)

- Session key: `trial:{run_id}`. Joins `scientia_research_sessions` to a run with no ALTER.
- The LLM telemetry label is the `run_id`.
- `research_trial.*` metric rows carry `campaign_id`, `run_id`, `arm_id`, `replicate_id` and
  `attempt_index`.
- `campaign_id` is the Trusty URI of the canonical preregistration bytes, recomputed and
  compared during verification (a tampered id must fail).

## 9. Phase 12 ask (D-09)

Phase 12 needs a run-level `provenance_json TEXT NOT NULL` plus indexed scalars
`syntax_fingerprint`, `model_id`, `condition`, `problem_set_hash` and `run_complete`. All six
are in the `trial_runs` DDL above.

How the already-shipped `model_benchmark_results` (baseline 95, written by `vox models bench`)
relates to the trial tables:

| Option | Effect | Assessment |
|---|---|---|
| **A. Separate writer, documented boundary (recommended)** | `vox models bench` keeps writing `model_benchmark_results`; trial tables are written by trial/benchmark persist; nullable link columns can be added later | no migration risk, no routing effect |
| B. Nullable `trial_run_id` / `campaign_id` link columns on `model_benchmark_results` | needs an explicit ALTER in `migrate` (precedent: `hopper_inbox.resource_id` in `open.rs`) | only worth it if bench runs become campaigns |
| C. Projection or view over `trial_samples` | table already exists with data at 95 (DROP and recreate); its columns (`suite`, `task_category`, `tokens_per_sec`) are not generic sample fields | **rejected**: its only reader, `model_benchmark_summary`, aggregates all rows per model into live routing evidence, so projecting campaign samples would change routing. That is a policy change, which Phase 7 forbids |

Under option A, 12-09 never writes `model_benchmark_results`, and its projection must-have is
replaced by a test that `list_trial_samples(run_id).len()` equals the report's sample count.

## 10. Legacy boundary

- `research_eval_runs` and `research_eval_samples` keep being written by `vox research eval`
  when no campaign is given.
- `harness_eval_*` is unrelated and untouched.

## 11. Canonical bytes caveat

Canonical bytes are serde_json compact output of a byte-ordered key sort applied at every
depth, with non-finite numbers rejected through a serialize/deserialize/equality round trip.
They are **not** RFC 8785 (JCS): key order is byte order rather than UTF-16 order, and number
formatting is serde_json's rather than ECMAScript's. Signed bytes stay stable only while
serde_json's float formatting does, so they are pinned by golden-bytes tests. Third-party
offline verifiers must reproduce this exact form; moving to JCS later would be a new signed
`schema_version`.

## 12. Decision record

Filled in by plan 07-04 Task 3 after the human gate. Plans 07-06, 07-07 and 07-09, and the
12-09 amendment, check the lines below.

- Decision status: approved
- Run grain: item-batch
- model_benchmark_results: option-a
- Approved by: user, 2026-10-12 (chat answer "approve-recommended" to the 07-04-T2 STORE-01 question; recorded by the orchestrator session)
