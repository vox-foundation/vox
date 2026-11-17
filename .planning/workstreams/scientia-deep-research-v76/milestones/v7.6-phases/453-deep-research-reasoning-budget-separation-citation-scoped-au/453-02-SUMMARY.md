---
phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au
plan: 02
subsystem: cli-daemon-proxy
tags: [vox-cli-research, daemon-ipc, flock-fallback, vox-db]
requires:
  - phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au
    provides: Canonical research/scientia daemon API v2 endpoints and DeI methods
provides:
  - Transparent daemon IPC fallback (`daemon_db_proxy.rs`, `397` LoC) when canonical `vox.db` holds a Turso/POSIX `flock`
  - Auto-proxying for `vox research {run,history,show,result,watch}` and `vox scientia {dashboard,cost,claims,retrieval-status}`
affects: [vox-cli-research]
tech-stack:
  added: []
  patterns:
    - "`is_canonical_db_lock_error` gates IPC fallback on unset/empty `VOX_DB_PATH` + Turso/POSIX lock substrings (`failed locking file`, `databasebusy`, `database is locked`, `wouldblock`)"
    - "180s polling loop over `orch.api_v2_get` (`/research/sessions/{id}`) after submitting `dei_method::RESEARCH_RUN`"
key-files:
  created:
    - crates/vox-cli-research/src/daemon_db_proxy.rs
  modified:
    - crates/vox-scientia/src/claim_extractor/atomic.rs
    - crates/vox-scientia/src/claim_extractor/pipeline.rs
    - crates/vox-research-events/src/schema_types.rs
    - crates/vox-scientia/src/class_routing/defaults.rs
    - crates/vox-research-shim/src/research/discovery_bridge.rs
    - crates/vox-scientia/src/dashboard/cost.rs
    - crates/vox-scientia/src/review_flow.rs
    - crates/vox-cli-research/src/lib.rs
    - crates/vox-cli-research/src/scientia.rs
    - crates/vox-cli-research/src/scientia_phase_handlers.rs
    - crates/vox-cli-research/src/db/publication/discovery.rs
key-decisions:
  - "Bypass canonical daemon proxying whenever an explicit non-empty `VOX_DB_PATH` override is set so isolated/test DBs fail loudly rather than hitting the daemon."
  - "Keep `daemon_db_proxy.rs` at `397` LoC (< `400` LoC module budget) with deterministic unit tests covering lock error matching, envelope unwrapping, and payload rendering."
patterns-established:
  - "CLI research & scientia commands degrade cleanly from local `VoxDb::connect` to `daemon_tool::daemon_client()` when `vox-orchestrator-d` holds `vox.db`."
requirements-completed: [LIVE-12]
duration: 30min
completed: 2026-11-16
---

# Phase 453 Plan 02: Transparent Daemon IPC Fallback for Locked Canonical DB Summary

**Phase 453 Plan 02 implemented [`daemon_db_proxy.rs`](crates/vox-cli-research/src/daemon_db_proxy.rs) (`397` LoC), allowing `vox research` and `vox scientia` commands to transparently proxy through `vox-orchestrator-d` whenever canonical `vox.db` is `flock`-locked.**

## Accomplishments

### `LIVE-11`: Markdown-Aware Decomposer, Novelty Projection, Empirical Class & Cost Rollup
- Added byte-preserving `mask_non_prose_regions` (`vox-scientia/src/claim_extractor/pipeline.rs`) and `trim_markdown_segment` (`atomic.rs`) so YAML frontmatter, `<!-- kb:summary -->`, code fences, and Markdown tables never pollute extracted claims while keeping `SpanChecker` offsets exact.
- Updated `merge_novelty_bundle_into_metadata_json_str` and `publication_discovery_scan` (`discovery.rs`) to project `prior_art_max_lexical_overlap` / `max_lexical_score` into `scientia_evidence`, dropping `novelty_overlap_assumed_default`.
- Added `EmpiricalScience` to `FindingCandidateClass` / `FindingClass` + `infer_candidate_class` (`discovery_bridge.rs`), and added `research_usd` to `QuarterlyCostSummary` / `CostInputs` (`cost.rs`, `review_flow.rs`).

### `LIVE-12`: Canonical `vox.db` Lock Fallback
- Implemented [`crates/vox-cli-research/src/daemon_db_proxy.rs`](crates/vox-cli-research/src/daemon_db_proxy.rs):
  - `is_canonical_db_lock_error` / `is_canonical_db_lock_error_with_override` match Turso/POSIX lock failures only when `VOX_DB_PATH` is unset or whitespace-only.
  - `research_history_via_daemon`, `research_show_via_daemon`, `research_result_via_daemon`, `research_watch_via_daemon`, and `try_run_research_via_daemon` call `orch_daemon_method::API_V2_GET` and `dei_method::RESEARCH_RUN` with a `180s` completion wait.
  - `scientia_dashboard_via_daemon`, `scientia_cost_via_daemon`, `scientia_claims_via_daemon`, and `retrieval_status_via_daemon` proxy `/scientia/queue`, `/scientia/cost`, `/scientia/publications/{id}/claims`, `/research/engine-status`, and `vox_repo_index_status`.
- Wired call sites in [`crates/vox-cli-research/src/lib.rs`](crates/vox-cli-research/src/lib.rs), [`crates/vox-cli-research/src/scientia.rs`](crates/vox-cli-research/src/scientia.rs), and [`crates/vox-cli-research/src/scientia_phase_handlers.rs`](crates/vox-cli-research/src/scientia_phase_handlers.rs).

## Verification
- `cargo test -p vox-cli-research --lib`: `671 passed; 0 failed; 2 ignored` (including `is_canonical_db_lock_error_matches_flock_and_turso_errors_only_without_override`, `unwrap_api_v2_envelope_handles_data_and_error`, `format_research_history_rows_matches_tab_delimited_shape`, and `format_research_show_detail_and_result_payload_round_trip`).

---
*Phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au*
*Completed: 2026-11-16*
