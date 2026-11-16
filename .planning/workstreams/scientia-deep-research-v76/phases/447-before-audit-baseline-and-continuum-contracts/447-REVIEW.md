---
phase: 447-before-audit-baseline-and-continuum-contracts
review_scope: plans 447-01, 447-02, 447-03
reviewed_at: 2026-11-15
reviewer: rust-verifier
status: approved_with_notes
---

# Phase 447 Code & Architecture Review

## Scope Reviewed

- **Plan 447-01 (`L0`/`L1` Continuum Contracts & `vox-scientia` Bridge):** `crates/vox-orchestrator-types/src/continuum.rs`, `crates/vox-research-events/src/events.rs`, `crates/vox-scientia/src/inspect_bridge.rs`, and contract schema/bindings.
- **Plan 447-02 (`L2`/`L3` Schema & Shared Embedder):** `crates/vox-db-types`, `crates/vox-db` (`research_session_pages.rs`, baseline schema v99, legacy export coverage), `crates/vox-scientia/src/embedder.rs`, `crates/vox-research-shim`.
- **Plan 447-03 (`L5` Audit Harness & `L1`/`L3` Publisher Unification):** `crates/vox-publisher/tests/novelty_bundle_contract_parity.rs`, `crates/vox-cli-research/src/db/publication/embedder.rs`, `crates/vox-cli-research/src/eval.rs`, `crates/vox-cli-research/src/scientia_v76_audit/{mod.rs,scoring.rs,cassettes.rs,bug_probes.rs}`, `crates/vox-cli-research/tests/fixtures/scientia_v76_cassettes.json`, `contracts/reports/scientia-v76-audit.v1.schema.json`, `contracts/reports/scientia-v76-before-baseline.v1.json`, and `contracts/index.yaml`.
- **Step A0 Coordinator-Authorized Build Unblock:** `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` and `crates/vox-orchestrator-mcp/src/chat_tools/chat/term_mentions.rs`.

## Findings Summary

- **Blocking Issues:** `0`
- **Architectural Violations (`vox-arch-check`):** `0` errors (12 pre-existing release-delta LoC warnings across workspace crates; no new violations introduced)
- **Non-Blocking Findings:** `3` documented scope/hygiene notes below (no phase blockers)

## 1. Crate Layering & Dependency Discipline

- `vox-arch-check` (`cargo run -p vox-arch-check`) exited `0` with zero layer-inversion errors.
- `vox-publisher` and `vox-cli-research` share the canonical `L1` `vox_research_events::schema_types::NoveltyEvidenceBundle` type and `L3` `vox_scientia::embedder::Embedder` trait, verified at compile/test time via `std::any::TypeId::of` assertions in `crates/vox-publisher/tests/novelty_bundle_contract_parity.rs`.
- `crates/vox-cli-research/src/db/publication/embedder.rs` is reduced to a 3-line pure re-export facade over `vox_scientia::embedder::{CachedLlmEmbedder, Embedder, EmbedderBackend, LocalHashEmbedder}` with zero duplicated embedding logic.

## 2. File & Function Budget Compliance (LoC & Scope)

| File | Observed LoC | Target Budget | Status |
|---|---|---|---|
| `crates/vox-orchestrator-types/src/continuum.rs` | `374` | `<400` | Pass (from `447-01-SUMMARY.md`) |
| `crates/vox-research-events/src/events.rs` | `392` | `<400` | Pass (from `447-01-SUMMARY.md`) |
| `crates/vox-db` (`research_session_pages.rs`) | `391` | `<400` | Pass (from `447-02-SUMMARY.md`) |
| `crates/vox-scientia/src/embedder.rs` | `324` | `<400` | Pass (from `447-02-SUMMARY.md`) |
| `crates/vox-cli-research/src/scientia_v76_audit/scoring.rs` | `111` | `<150` | Pass |
| `crates/vox-cli-research/src/scientia_v76_audit/mod.rs` | `205` | `<400` | Pass |
| `crates/vox-cli-research/src/scientia_v76_audit/cassettes.rs` | `388` | `<400` | Pass |
| `crates/vox-cli-research/src/scientia_v76_audit/bug_probes.rs` | `391` | `<400` | Pass |
| `crates/vox-cli-research/tests/fixtures/scientia_v76_cassettes.json` | `258` | `<400` | Pass |
| `crates/vox-cli-research/src/eval.rs` | `729` (`538` pre-test) | — | Non-test code ends cleanly at line `538` before `#[cfg(test)]` |

- Banned hardcoded OS temp/home path scan (`"/tmp|"/var|"/home`) across modified crates: `0` matches.

## 3. Correctness, Determinism & STRIDE Mitigations (`T-447-07..09`)

- **Deterministic Reproducibility:** `vox scientia v76-audit --stage before` uses frozen cassettes (`deterministic_seed: 20260447`) and passes `audit_report_is_byte_for_byte_deterministic_and_schema_valid`, proving repeated runs emit identical JSON bytes.
- **Schema Strictness (`T-447-07`):** `contracts/reports/scientia-v76-audit.v1.schema.json` sets `"x-vox-version": 1` and `"additionalProperties": false` on report objects, preventing unmodeled fields or schema drift. `contracts/index.yaml` passes `vox ci contracts-index` (`211` entries).
- **Honest Scoring (`AUDT-01`):** `scoring.rs` returns `0.0` when `sources_count == 0` or `model_answer` is blank, and computes bounded `[0.0, 1.0]` scores across subquery progression, source span, and optional bridge-entity grounding, replacing the pre-v7.6 unconditional `1.0` stub.
- **Authenticity of Pre-Remediation Probes:** `before_stage_captures_authentic_pre_v76_live_bug_reproductions` confirms all 9 pre-v7.6 defects remain authentically observable prior to Wave B–F fixes (including `composer_research_scope_overwritten: true`, `research_turn_waves_hardcoded_to_one: true`, `none_to_novel_false_positive_rate: 1.0`, `artifact_json_wiped_on_doc_publish: true`, `mock_verifier_fallback_rate: 1.0`).

## 4. Non-Blocking Observations & Pre-Existing Repository State

1. **Unused-imports warning (`embedder.rs:3`):** Compiling `vox-cli-research` emits 1 non-blocking unused-imports warning at `crates/vox-cli-research/src/db/publication/embedder.rs:3`. Safe to retain or clean up in Wave B; all re-exported symbols resolve cleanly.
2. **Pre-existing `vox-db` `--features legacy-import` gaps:** Under `cargo test -p vox-db legacy_export_covers_all_baseline_tables`, `"research_session_sources"` (added by `447-02`) is covered, while 4 pre-existing `gsd_*` tables (`gsd_documents`, `gsd_milestones`, `gsd_phases`, `gsd_requirements`) remain omitted on `HEAD` outside Phase 447 scope.
3. **Pre-existing `vox ci data-storage-guard` baseline failures:** `./target/debug/vox ci data-storage-guard` exited `1` with `531` pre-existing repo-wide violations (`ignore-tracked-parity`, pre-existing `version-header-parity`, pre-existing `env-parity`). Filtering full tool output confirms `0` violations attributable to Phase 447 files or guards, and `cargo run -p vox-jsonschema-util --example emit_agent_harness -- --verify` passed independently (`exit 0`).

## Verdict

**APPROVED** — Phase 447 satisfies `AUDT-01` and `DISC-01` requirements, passes all scoped unit/integration/contract/architecture checks, and provides an immutable, schema-validated `before` baseline for Milestone v7.6.
