---
title: "SCIENTIA SSOT handbook (glossary, vocabulary, checklists)"
description: "Single reference for SCIENTIA lifecycle terms, status vocabulary, SSOT routing, anti-drift checklists, operator flows, SLOs, and LLM task conventions."
category: "reference"
last_updated: 2026-03-26
training_eligible: true
---

# SCIENTIA SSOT handbook

Companion: [publication readiness audit](../architecture/scientia-publication-readiness-audit.md), [VoxGiantia publication map](../architecture/voxgiantia-publication-architecture.md), [how-to publication](../how-to/how-to-scientia-publication.md).

## 1. Glossary and canonical lifecycle (T001)

| Term | Meaning |
|------|---------|
| **Manifest** | Row in `publication_manifests`: canonical content + `content_sha3_256` digest. |
| **Digest** | `content_sha3_256`; binds approvals and external jobs to an immutable content fingerprint. |
| **Approval** | Row in `publication_approvers` / digest-bound approver set; **dual distinct approvers** required before live scholarly submit. |
| **Scholarly submission** | Row in `scholarly_submissions`: adapter + remote id + status for one publication digest. |
| **External job** | Row in `external_submission_jobs`: queued work keyed by `idempotency_key` (submit pipeline). |
| **Attempt** | Row in `external_submission_attempts`: one HTTP/adapter outcome with `error_class`, `retryable`. |
| **Status event** | Append-only row in `publication_status_events` (e.g. arXiv handoff stages); does **not** auto-update `publication_manifests.state`. |
| **Snapshot** | Row in `external_status_snapshots`: polled remote JSON at a point in time. |
| **Adapter** | Scholarly backend (`local_ledger`, `echo_ledger`, `zenodo`, `openreview`, …) resolved via `VOX_SCHOLARLY_ADAPTER` or CLI override. |

**Lifecycle (happy path):** `draft` manifest → `publication-prepare` / approvals → optional preflight + staging export → `publication-submit-local` or enqueued **`external_submission_jobs` tick** → `scholarly_submissions` + job terminal state → remote status sync.

## 2. Canonical status vocabulary (T002)

### `external_submission_jobs.status`

Operational queue states (string, lowercase). **Do not invent new values without migration + worker updates:**

| Value | Meaning |
|-------|---------|
| `queued` | Ready for worker; no active lease. |
| `running` | Leased (`lock_owner`, `lock_expires_at_ms`). |
| `retryable_failed` | Transient failure; `next_retry_at_ms` may gate re-entry. |
| `failed` | Permanent / operator dead-letter. |
| `succeeded` | Terminal success. |

**Future DB CHECK constraints:** see comments in `crates/vox-db/src/schema/domains/publish_cloud.rs`; until enforced in SQL, workers and upserts must stay within this set.

### `scholarly_submissions.status`

Venue-specific remote status strings stored as received (normalized to adapter semantics). Polling updates via `patch_scholarly_submission_status` without rewriting manifest state.

### `publication_status_events.status`

Operator and automation labels (e.g. `arxiv_handoff:staging_exported`). Free-form but **document new slugs** in [operator flow §6](#6-one-page-arxiv-operator-assist-t016).

### Preflight / errors

Job-layer preflight uses `last_error_class = "preflight"`. Adapter errors use `ScholarlyError` classes: `disabled`, `config`, `auth`, `rate_limit`, `transient`, `fatal` (see schema comment on `external_submission_attempts`).

## 3. Source-of-truth map: DB → publisher → CLI → MCP → docs (T003)

| Layer | SSOT location |
|-------|----------------|
| Schema | `crates/vox-db/src/schema/domains/publish_cloud.rs` |
| Store ops | `crates/vox-db/src/store/ops_publication.rs` |
| Worker / adapters | `crates/vox-publisher/src/scholarly_external_jobs.rs`, `crates/vox-publisher/src/scholarly/` |
| CLI implementation | `crates/vox-cli/src/commands/db.rs` (handlers), `db_cli/subcommands.rs` (Clap), `scientia.rs` (facade) |
| MCP | `crates/vox-mcp/src/tools/scientia_tools.rs`, `dispatch.rs`, `input_schemas.rs` |
| CLI contract | `contracts/cli/command-registry.yaml` |
| MCP contract | `contracts/mcp/tool-registry.canonical.yaml` |
| Human reference | `docs/src/reference/cli.md`, this handbook |

**Rule:** Add behavior in **store + publisher** first; then CLI; then MCP + contracts; then docs. Never document a command that is not in `command-registry.yaml` when `ref_cli_required` applies.

## 4. Command registry vs command catalog (T004)

- **Registry** (`contracts/cli/command-registry.yaml`): semantic metadata, compliance (`ref_cli_required`, ownership). **SSOT for “what exists and what docs must mention”.**
- **Catalog paths baseline** (`crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt`): structural snapshot of the Clap tree. **Update via** `UPDATE_CLI_CATALOG_BASELINE=1` when adding/removing commands.

## 5. MCP registry vs dispatch / schemas (T005)

- **Registry** (`contracts/mcp/tool-registry.canonical.yaml`): tool names and descriptions for parity checks.
- **Dispatch** (`vox-mcp/src/tools/dispatch.rs`): routes tool name → async handler.
- **Input schemas** (`input_schemas.rs`): JSON Schema for each tool; must cover every canonical tool (*tests enforce coverage*).

After registry changes: `pnpm run generate:mcp-registry` (VS Code) and `pnpm run check:mcp-parity`.

**Zenodo metadata MCP:** there is intentionally no separate MCP tool for `publication-zenodo-metadata` (stdout-only JSON helper); agents should call `vox_scientia_publication_preflight` / staging export or run the CLI directly when they need deposition JSON.

## 6. Anti-drift checklists

### New CLI command (T006)

1. Handler in `db.rs` (or appropriate module).
2. Variant in `db_cli/subcommands.rs`; mirror in `scientia.rs` if user-facing.
3. `command-registry.yaml` entry if part of scientia surface.
4. `cargo run -p vox-cli -- ci command-sync --write` if generated surfaces change.
5. Mention in `docs/src/reference/cli.md` when `ref_cli_required: true`.
6. Refresh `command_catalog_paths_baseline` if paths change.

### New MCP tool (T007)

1. Handler in `scientia_tools.rs` (or module).
2. Arm in `dispatch.rs`.
3. Schema in `input_schemas.rs` + registry coverage test.
4. `tool-registry.canonical.yaml`.
5. `pnpm run generate:mcp-registry` + `pnpm run check:mcp-parity`.

### `publish_cloud` schema change (T008)

1. Edit `publish_cloud.rs` DDL; verify greenfield + migration notes.
2. Update `ops_publication.rs` and row types.
3. Extend `publication_flow_tests.rs` (or crate tests).
4. Document status vocabulary / migration in this handbook if user-visible.

### Adapter API change (T009)

1. Update adapter module + `ScholarlyError` mapping.
2. Remote status mapping (`scholarly_remote_status` module) if polling semantics shift.
3. MCP/CLI outputs that embed raw JSON: bump documented schema if needed.

### Worker loop behavior change (T010)

1. Clamp `iterations` / `interval_secs` / new `max_runtime_secs` consistently in CLI + MCP + publisher.
2. Add unit test for loop metadata and clamps.
3. Note operator impact in rollout section of readiness audit.

### Metrics payload change (T011)

1. Bump `metrics_schema_version` in `summarize_scholarly_external_pipeline_metrics` JSON.
2. Update golden / structure tests in `publication_flow_tests.rs`.
3. Document keys in [metrics §](#12-metrics-schema-version-t050t051).

### Docs-only semantic change (T012)

1. If behavior is described, grep code to confirm (`rg` command name / table name).
2. Run `vox ci command-compliance` if CLI strings change.

## 7. One-page operator flows

### Happy path publication (T013)

1. `vox scientia publication-prepare --publication-id <id> …` (+ optional `--preflight`).
2. Two approvers: `vox scientia publication-approve …`.
3. Optional: `publication-scholarly-staging-export`, Zenodo metadata check.
4. Submit: `publication-submit-local` **or** enqueue + `publication-external-jobs-tick`.
5. Track: `publication-status`, `publication-scholarly-remote-status-sync-batch` (or loop).

### Dead-letter incident (T014)

1. `publication-external-jobs-failed-list` → inspect `last_error_class` / attempts.
2. Fix root cause (credentials, policy, manifest digest).
3. If transient resolved: replay job to `queued` when supported **or** operator-corrected re-enqueue.
4. Record narrative in status events if policy requires audit trail.

### Status-sync recovery (T015)

1. Run `publication-scholarly-remote-status-sync-batch` for one publication or batch.
2. Confirm `external_status_snapshots` and `scholarly_submissions` updated.
3. Verify `external_submission_jobs` sync via mapped terminal status.

### arXiv operator assist (T016)

1. Staging export → custody → validate bundle → manual arXiv UI submit.
2. After each milestone: `vox scientia publication-arxiv-handoff-record --stage …` (append-only events).
3. When live: `--stage published --arxiv-id <id>`.

## 8. Non-goals (explicit) (T017)

- **Not** a replacement for venue submission UX (TMLR ScholarOne, internal portals).
- **Not** guaranteed real-time remote state; polling + adapter limits apply.
- **Not** legal/compliance advice; adapters enforce platform ToS.
- **Not** silent cross-publication ID reuse: upserts must reject identity mismatch (see store).

## 9. Adapter support matrix (limits) (T018)

| Adapter | Automation level | Notes |
|---------|------------------|-------|
| `local_ledger` | Full (dev) | No network; deterministic. |
| `echo_ledger` | Full (dry) | No network; echoes payloads. |
| `zenodo` | API submit + poll | Tokens via Clavis / env; rate limits. |
| `openreview` | API notes/venues | Invitation + permission bound. |
| arXiv | **Assist** | Export + handoff events; human submit. |

## 10. SLOs and KPIs (T019)

**SLO (targets for ops, not enforced in code):**

- **P95** manifest-ready → first successful external job `succeeded` under profile-specific minutes (staging vs prod).
- **Error budget**: retryable ratio < threshold per adapter/week.

**KPI JSON:** `vox scientia publication-external-pipeline-metrics` — job counts, attempts, error_class histogram, latency averages; extend with percentile fields as schema version bumps.

## 11. LLM execution style guide (T020)

When implementing SCIENTIA tasks agents should:

1. State **objective** in one sentence.
2. List **absolute file paths** to touch.
3. Prefer **extending** existing modules over new crates.
4. Add **one** focused test or `cargo check -p …` acceptance per change batch.
5. Avoid breaking **digest / approval** invariants;never skip dual-approval in production paths.
6. After CLI/MCP edits run **command-sync** and **command-compliance** as required by CI.

## 12. Metrics schema version (T050–T051)

The rollup includes `"metrics_schema_version": <integer>` at the top level. Increment when adding/removing keys or changing types of required fields.
