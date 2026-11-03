---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 03
subsystem: database
tags: [turso, schema-baseline-96, kb_findings, trust-tier, fail-closed, migration]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "ContextTrustTier/FindingTier (03-01), SanitizedText/ContentHash/FindingOrigin/FindingStatus (03-02)"
provides:
  - "kb_findings inbox table + typed writer/readers (insert_pending_finding, kb_findings_list/counts/unqueued_reviewed/mark_mens_queued)"
  - "search_documents.trust_tier TEXT NOT NULL DEFAULT 'untrusted' with a one-time corpus-derived backfill"
  - "legacy VoxKB DDL as its own fragment, re-exported as vox_db::LEGACY_VOXKB_DDL; legacy_kb_counts / legacy_kb_entries_page"
  - "vox_db re-exports of ContextTrustTier, FindingTier, tier_for_curated, tier_for_knowledge_node_type"
affects: [03-04, 03-05, 03-08, 03-09, 03-12, 03-13, phase-09, phase-12]
tech-stack:
  added: []
  patterns: ["fail-closed column default as the DB-level guard (Turso has no CHECK/triggers)", "version-gated Rust backfill instead of a correlated UPDATE"]
key-files:
  created:
    - crates/vox-db/src/store/ops_memory/kb_findings.rs
    - crates/vox-db/src/store/ops_memory/kb_legacy.rs
    - crates/vox-db/tests/kb_findings_store.rs
    - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/deferred-items.md
  modified:
    - crates/vox-db/src/schema/domains/knowledge.rs
    - crates/vox-db/src/schema/manifest.rs
    - contracts/db/baseline-version-policy.yaml
    - crates/vox-db/src/store/open.rs
    - crates/vox-db/src/store/mod.rs
    - crates/vox-db/src/store/ops_memory/mod.rs
    - crates/vox-db/src/codex_legacy.rs
    - crates/vox-db/src/lib.rs
    - crates/vox-db/src/local_tests.rs
key-decisions:
  - "Baseline 96 taken (no competing branch had it); digest re-updated twice, once per task, integer stays 96"
  - "Backfill skips an empty-string corpus key in kb_index_state so web/legacy rows can never be promoted by a malformed state row"
  - "Legacy readers live in kb_legacy.rs (not kb_findings.rs) so the findings module carries no confidence column name"
requirements-completed: [TRUST-01, TRUST-04, RET-03]
status: complete
commits: 5
plan_head_before: 27a82d8a94581af82244f32c2b17cffda9840100
actuals:
  tokens: 13181
  tasks: 2
  commits: 5
duration: 45min
completed: 2026-10-06
coverage:
  - id: D1
    description: "kb_findings writer cannot express Trusted/SystemVerified and takes only SanitizedText/ContentHash (compile_fail doctest + passing twin)"
    requirement: TRUST-01
    verification:
      - {kind: unit, ref: "cargo test -p vox-db --doc NewKbFinding (compile fail + compile ok)", status: pass}
    human_judgment: false
  - id: D2
    description: "Each finding stores summary, quotes, source URL, fetch date, blake3 content hash and provenance, status pending"
    requirement: TRUST-04
    verification:
      - {kind: integration, ref: "crates/vox-db/tests/kb_findings_store.rs#kb_findings_round_trip_with_full_provenance", status: pass}
      - {kind: integration, ref: "crates/vox-db/tests/kb_findings_store.rs#kb_findings_list_orders_newest_first_then_id_ascending", status: pass}
      - {kind: integration, ref: "crates/vox-db/tests/kb_findings_store.rs#kb_findings_duplicate_dedupe_key_is_a_no_op", status: pass}
    human_judgment: false
  - id: D3
    description: "v95 database gains kb_findings on open; second open idempotent; schema_version 96"
    requirement: TRUST-04
    verification:
      - {kind: integration, ref: "crates/vox-db/src/local_tests.rs#kb_findings_migration_from_95", status: pass}
    human_judgment: false
  - id: D4
    description: "search_documents.trust_tier fail-closed default; backfill re-tiers (never purges) from kb_index_state, publication rows advisory"
    requirement: TRUST-01
    verification:
      - {kind: integration, ref: "crates/vox-db/src/local_tests.rs#kb_trust_tier_* (6 tests)", status: pass}
    human_judgment: false
  - id: D5
    description: "Legacy VoxKB counts readable, zeros when tables absent"
    requirement: RET-03
    verification:
      - {kind: integration, ref: "crates/vox-db/tests/kb_findings_store.rs#kb_findings_legacy_counts_and_paging", status: pass}
    human_judgment: false
---

# Phase 3 Plan 03: kb_findings inbox and stored trust tier (baseline 96) Summary

A `kb_findings` inbox with typed, sanitized, tier-limited writes and full provenance, plus a stored fail-closed `search_documents.trust_tier` backfilled from `kb_index_state`, shipped as one baseline bump 95 to 96 with legacy VoxKB readers that count rows before quarantine.

## Governance sign-off (D-01)

Frozen-core vox-db schema change **approved before the first DDL edit**: decision D-01 in `03-CONTEXT.md` (recorded 2026-10-04). Signer: **brbrainerd (delegated to agent, 2026-10-04)**. Scope signed: baseline **95 to 96**, `kb_findings` table, `search_documents.trust_tier`, and re-tiering of existing rows (D-03: re-tier Untrusted by default, never purge). No competing branch had taken integer 96. Include this section verbatim in the PR body.

## Performance

- **Duration:** about 45 min
- **Completed:** 2026-10-06T11:17:33Z
- **Tasks:** 2
- **Files modified:** 12 (4 created)

## Accomplishments

- `kb_findings` table (no CHECK, no trigger, no confidence column) with `NewKbFinding` (no `Default`; `FindingTier`, `SanitizedText`, `ContentHash`, `FindingOrigin` typed) and `insert_pending_finding` that always writes `pending` and dedupes on `dedupe_key`. A `compile_fail` doctest proves `FindingTier::Trusted` does not compile and a `no_run` twin proves the same value with `Untrusted` does.
- Readers: `kb_findings_list` (`created_at_ms DESC, id ASC`), `kb_findings_counts`, `kb_findings_unqueued_reviewed` (reviewed_by set, approved/rejected, not queued), `kb_findings_mark_mens_queued`. Unknown status/origin errors on read; unknown tier reads Untrusted.
- `search_documents.trust_tier TEXT NOT NULL DEFAULT 'untrusted'`; version-gated Rust backfill (per-corpus UPDATE from `kb_index_state.trust_tier` canonicalized through `ContextTrustTier::from_db_str`, then `vox://publication/%` to advisory). No index on the column (a v95 DB lacks it when the baseline batch runs).
- VoxKB DDL split into `SCHEMA_LEGACY_VOXKB` (fragment `legacy_voxkb`), re-exported as `vox_db::LEGACY_VOXKB_DDL`; `legacy_kb_counts` and `legacy_kb_entries_page` return zeros/empty when the tables are gone.
- `kb_findings` added to `LEGACY_EXPORT_TABLES` in sorted position.

## Task Commits

1. **Task 1 (tracer): kb_findings write/read on fresh and upgraded DBs**
   - RED `80cf75eda` (test): schema bump + stubs + 10 failing tests
   - GREEN `4f7054a2b` (feat): writer, readers, legacy counters
   - REFACTOR `8e6b45cca` (refactor): legacy readers moved to `kb_legacy.rs`
2. **Task 2: search_documents.trust_tier**
   - RED `f74f9e2b0` (test): 6 failing `kb_trust_tier_*` tests
   - GREEN `1f0390a27` (feat): column, ALTER row, backfill, digest

Tracer feedback gate (auto/end-of-phase, automated-only verify): the task `<verify>` commands were re-run end to end before Task 2 and passed, so expansion proceeded.

## TDD Gate Compliance

RED then GREEN present for both tasks. RED evidence: Task 1 RED run was 10 tests, 10 failed (writer/reader stubs return `Err("not implemented")` or empty, tests fail on the first unwrap/assertion). Task 2 RED run: 8 matched, 6 new failed with `no such column: trust_tier`, 2 pre-existing Phase 2 tests passed. The repo's pre-commit toestub (`enforce-strict`) rejects hollow `Ok(Vec::new())` bodies and files with pub fns but no in-file `#[test]`, so the RED stubs return a not-implemented error and `kb_findings.rs`, `kb_legacy.rs` and `codex_legacy.rs` each carry one small in-file test.

## Verification

- `cargo nextest run -p vox-db -E 'test(/baseline_policy_matches_compiled_schema|kb_library_migration_from_94|kb_findings_|trust_tier/)'`: 21/21 pass (also `cargo nextest run -p vox-db kb_findings`: 11/11).
- `cargo test -p vox-db --doc NewKbFinding`: 1 compile-fail ok, 1 compile ok.
- `cargo run -p vox-cli -- ci check-codex-ssot` (installed `vox` is stale; `VOX_SKIP_FRESHNESS_CHECK=1`): exit 0. It prints a digest-mismatch warning that is the by-design `quarantine`-feature digest (`print_baseline_digest --features quarantine` gives the same `0x1e7c7e...`), not drift.
- `cargo run -p vox-cli -- ci sql-surface-guard --all`: OK.
- Acceptance greps: single `CREATE TABLE IF NOT EXISTS kb_findings`; `BASELINE_VERSION: i64 = 96` and `repository_baseline_integer: 96`; no `confidence` in `kb_findings.rs`; `"kb_findings"` in `codex_legacy.rs`; `LEGACY_VOXKB_DDL` in `lib.rs`; no `CREATE INDEX.*trust_tier`, no `library:%`, no `delete from search_documents` in `open.rs`; `kb_index_state` appears in `open.rs`.

## Mutation proof (T-03-20)

Each edit was guarded by `grep -c` presence counts and the file restored from a scratchpad copy after the run.

- **Column default.** In `knowledge.rs`, count of `trust_tier TEXT NOT NULL DEFAULT 'untrusted'` was 2 (kb_findings, search_documents) and `'trusted'` 0. After changing the `search_documents` default to `'trusted'`: untrusted 1, trusted 1. `nextest trust_tier`: 1 failed (`kb_trust_tier_raw_insert_without_tier_reads_back_untrusted`), 7 passed. Restored: untrusted 2.
- **Per-corpus derivation.** In `open.rs`, `ContextTrustTier::from_db_str(&tier)` count 1, literal `let tier = "trusted";` 0. After replacing the derivation with the literal: derivation 0, literal 1. `nextest trust_tier`: 2 failed (advisory-corpus and invalid-string tests), 6 passed. Restored: derivation 1, literal 0.

## Decisions Made

- Skip an empty corpus key in the backfill loop so a malformed `kb_index_state` row cannot retag web/legacy rows.
- `kb_findings_mark_mens_queued` returns `Result<(), StoreError>`, mirroring the legacy `kb_mark_mens_queued`.
- `LegacyKbEntry.accepted` and `.mens_queued` are `bool`, `routing_confidence` is `f64` (plan left the types open).

## Deviations from Plan

**1. [Rule 3 - Blocking] `NewKbFinding` has no lifetime parameter.** The plan wrote `NewKbFinding<'a>` but every field is owned (`String`, `Vec<SanitizedText>`); an unused lifetime does not compile. Struct is `NewKbFinding`; callers write the same literal. Commit `80cf75eda`.

**2. [Rule 3 - Blocking] Legacy readers placed in `kb_legacy.rs`, not `kb_findings.rs`.** Plan step (4) said "in the same file" but the same plan's acceptance grep requires no `confidence` in `kb_findings.rs`, and `LegacyKbEntry.routing_confidence` is spec'd. A sibling module satisfies both; public names and `vox_db::` re-exports are unchanged. Commit `8e6b45cca`.

**3. [Rule 3 - Blocking] In-file tests added to `kb_findings.rs`, `kb_legacy.rs`, `codex_legacy.rs`.** The pre-commit `tdd-guard` (toestub `enforce-strict`) blocks commits that touch a file with pub fns and no `#[test]`. Small real tests (insert/dedupe, fresh-DB legacy readers, `LEGACY_EXPORT_TABLES` sorted and contains `kb_findings`). Commits `80cf75eda`, `8e6b45cca`.

**4. [Scope boundary] Pre-existing failure logged, not fixed.** `legacy_export_covers_all_baseline_tables` (feature `legacy-import`) fails because `LEGACY_EXPORT_TABLES` lacks `kb_index_state`, `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events`, `web_cache`; all predate this plan. `kb_findings` is correctly on both sides. Logged in `deferred-items.md`. The plan's third verify line for this test therefore cannot go green from this plan.

**Total deviations:** 3 auto-fixed (Rule 3), 1 deferred. **Impact:** none on the contract surface other plans consume (names, signatures, re-exports) except the dropped lifetime.

## Issues Encountered

- `cargo fmt -p vox-db` after a Python edit collapsed an import line and broke a follow-up string replace; fixed by hand, no code impact.
- Installed `vox` refuses `vox ci` guards (stale); used `cargo run -p vox-cli -- ci ...` with `VOX_SKIP_FRESHNESS_CHECK=1`.

## Known Stubs

None. (RED-phase stubs were replaced in the GREEN commit.)

## Threat Flags

None. New surface is the planned table and column only; no endpoints or file access.

## User Setup Required

None.

## Next Phase Readiness

- 03-04 can call `insert_pending_finding(&NewKbFinding{..})`; 03-05 can make every `search_documents` writer set the tier explicitly (library `apply_library_document` and `upsert_search_document` still rely on the column default today, which is the fail-closed value); 03-08 can use `legacy_kb_counts`, `legacy_kb_entries_page`, `kb_findings_unqueued_reviewed`, `kb_findings_mark_mens_queued`, `LEGACY_VOXKB_DDL`.
- Note for 03-05: Library rows written after this plan default to `untrusted` until its writer sets the tier; the `kb_index_state` tier is only applied by the one-time upgrade backfill.

## Self-Check: PASSED

Verified present: `kb_findings.rs`, `kb_legacy.rs`, `tests/kb_findings_store.rs`, `deferred-items.md`; commits `80cf75eda 4f7054a2b 8e6b45cca f74f9e2b0 1f0390a27` exist.

---
*Phase: 03-provenance-trust-tiers-and-leak-closure*
*Completed: 2026-10-06*
