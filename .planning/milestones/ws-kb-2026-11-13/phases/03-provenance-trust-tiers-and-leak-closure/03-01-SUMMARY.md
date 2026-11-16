---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 01
subsystem: trust
tags: [trust-tier, provenance, promotion, fail-closed, crossref]
requires: []
provides:
  - "vox_orchestrator_types::trust::{ContextTrustTier, FindingTier, tier_for_curated, tier_for_knowledge_node_type}"
  - "vox_search::trust::{TrustLookup, PromotionEligibility, promotion_eligibility, TrustScorer::lookup_for_promotion}"
affects: [03-03, 03-04, 03-05, 03-09, phase-09]
tech-stack:
  added: []
  patterns: ["no-Default enum at the type boundary (compile_fail doctest)", "exhaustive match without wildcard for security verdicts"]
key-files:
  created:
    - crates/vox-orchestrator-types/src/trust.rs
  modified:
    - crates/vox-orchestrator-types/src/lib.rs
    - crates/vox-orchestrator/src/context_envelope.rs
    - crates/vox-search/src/trust.rs
key-decisions:
  - "D-06 taint rule encoded once: only node types fact and concept map to Advisory; everything else, including None, is Untrusted"
  - "NotApplicable (no DOI) stays distinct from Unavailable (lookup failed) so non-academic sources remain promotable"
requirements-completed: [TRUST-01, TRUST-06]
status: complete
commits: 4
plan_head_before: b0ec783d25c8d79ec7a1e9d9cbae73b7097cafd2
actuals:
  tokens: 4500
  tasks: 2
  commits: 4
duration: 16 min
completed: 2026-10-06
coverage:
  - deliverable: "Single ContextTrustTier in the L0 crate, no Default, fail-closed parse, wire strings unchanged"
    human_judgment: false
    verification:
      - {kind: test, ref: "crates/vox-orchestrator-types/src/trust.rs#tests (10 tests) + 2 doctests", status: pass}
  - deliverable: "FindingTier (Untrusted|Advisory) and D-06 node-type mapping"
    human_judgment: false
    verification:
      - {kind: test, ref: "crates/vox-orchestrator-types/src/trust.rs#node_type_allowlist_is_fact_and_concept_only", status: pass}
  - deliverable: "Fail-closed promotion eligibility verdict and lookup_for_promotion"
    human_judgment: false
    verification:
      - {kind: test, ref: "crates/vox-search/src/trust.rs#promotion_eligibility_fails_closed", status: pass}
      - {kind: test, ref: "crates/vox-search/src/trust.rs#promotion_lookup_* (5 wiremock tests)", status: pass}
---

# Phase 3 Plan 01: Trust-tier vocabulary and fail-closed promotion verdict Summary

One L0 `ContextTrustTier` (no `Default`, fail-closed parsing, D-06 taint mapping, `FindingTier`) used unchanged on the wire by the orchestrator envelope, plus a `promotion_eligibility` verdict in vox-search that treats a failed Crossref lookup as ineligible.

## Tasks

| Task | Commits | Result |
|------|---------|--------|
| 1 (tracer) tier vocabulary | RED `7bdc94946`, GREEN `3f653555e` | 10 unit tests + 2 doctests; consumers compile |
| 2 promotion verdict | RED `08f2acbe8`, GREEN `302c44359` | 18/18 `vox-search trust::` tests green |

Tracer feedback gate: auto/`end-of-phase` mode with automated-only verify; the `<verify>` commands were re-run end to end (nextest, doctests, `cargo check -p vox-orchestrator -p vox-orchestrator-mcp -p vox-db`) and passed, so expansion to Task 2 proceeded.

## TDD

- RED 1: tests passed `cargo nextest run -p vox-orchestrator-types trust` as 5 passed / 5 failed, each failing on a behavior assertion (`as_db_str`, `from_db_str` round-trip, `tier_for_curated`, node-type allowlist, FindingTier encoding) against stubbed bodies.
- RED 2: `cargo nextest run -p vox-search trust::` gave 13 passed / 5 failed (`promotion_eligibility_fails_closed` and four `promotion_lookup_*` tests) against a stub that always returned `Eligible` / `NotApplicable`.
- GREEN: both suites fully pass. No REFACTOR commit needed.

## Behavior to test map

- Ordering and `min`: `tier_order_is_ascending_trust`, `min_of_equal_tiers_is_that_tier`, `min_trusted_untrusted_is_untrusted`
- Wire strings and fail-closed parse: `as_db_str_matches_phase2_wire_strings`, `from_db_str_round_trips_all_four`, `from_db_str_fails_closed`, `serde_wire_strings_unchanged`
- FindingTier: `finding_tier_converts_and_encodes`; curated: `tier_for_curated_splits_on_evidence`; D-06: `node_type_allowlist_is_fact_and_concept_only`
- No `Default`: `compile_fail` doctest (trust.rs line 16) plus passing `from_db_str("trusted")` doctest (line 22)
- Verdicts: `promotion_eligibility_fails_closed`, `promotion_eligibility_open_for_clean_and_not_applicable`
- Lookup: `promotion_lookup_without_doi_is_not_applicable_and_makes_no_call` (zero requests asserted), `promotion_lookup_http_error_is_unavailable_naming_doi`, `promotion_lookup_retraction_is_retracted`, `promotion_lookup_no_updates_is_clean`, `promotion_lookup_explicit_doi_wins_over_non_doi_url`

## Mutation proof (T-04, TRUST-06)

Presence counts via `grep -c` guarded each edit (the file was restored from a scratchpad copy after each run).

1. Unavailable arm to `Eligible`: marker `trust lookup unavailable` 1 -> 0, mutated arm 0 -> 1. `cargo nextest run -p vox-search promotion_eligibility`: 1 passed, 1 failed (`promotion_eligibility_fails_closed` red). Restored: marker 1; full suite 18/18 green.
2. NotApplicable arm to `Ineligible`: `Clean | NotApplicable => Eligible` 1 -> 0, mutated 0 -> 1. `cargo nextest run -p vox-search promotion_`: 6 passed, 1 failed (`promotion_eligibility_open_for_clean_and_not_applicable` red). Restored: 1, mutated 0; full suite 18/18 green.

## Acceptance criteria

- Single `enum ContextTrustTier` at `crates/vox-orchestrator-types/src/trust.rs:33`: PASS
- No `impl Default for (ContextTrustTier|FindingTier)`: PASS (empty)
- `pub use vox_orchestrator_types::trust::ContextTrustTier` in `context_envelope.rs:88`: PASS
- `cargo clippy -p vox-orchestrator-types -- -D warnings`: PASS; `cargo clippy -p vox-search --lib --tests -- -D warnings`: PASS
- Four public items in `vox-search/src/trust.rs`: PASS (lines 87, 260, 269, 276); wildcard count in `promotion_eligibility`: 0
- `vox ci crate-edges`: NOT RUN. The installed `vox` binary refused as stale (built at commit 6382, tree at 6618). No `Cargo.toml` was touched in this plan, so no crate edge can have changed; re-run after `cargo install --locked --path crates/vox-cli --force` if desired.

## Deviations from Plan

None - plan executed exactly as written. Notes:

- The first RED attempt for Task 1 used an empty-string stub body, which the `tdd-guard` toestub hook flagged as a skeleton warning; the stub was changed to `"unset"` and the hook passed (no `--no-verify`).
- No `.clone()` on a tier value existed in the repo (`rg trust_tier\.clone` only hits unrelated populi string fields), so no clippy `clone_on_copy` fixups were needed.
- `ContextTrust` (envelope struct) did not need changes; `Copy` is additive.

## Known Stubs

None.

## Threat Flags

None. T-03-04, T-03-05 and T-03-39 are mitigated as planned (exhaustive match plus mutation proof; no `Default` plus `compile_fail` doctest and fail-closed parsing; two-type allowlist).

## Self-Check: PASSED

- `crates/vox-orchestrator-types/src/trust.rs`, `crates/vox-search/src/trust.rs` exist; commits `7bdc94946`, `3f653555e`, `08f2acbe8`, `302c44359` exist.
