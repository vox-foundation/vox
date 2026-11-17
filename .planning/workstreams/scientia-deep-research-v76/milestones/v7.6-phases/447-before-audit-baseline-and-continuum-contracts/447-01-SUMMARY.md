---
phase: 447-before-audit-baseline-and-continuum-contracts
plan: 01
subsystem: research-contracts
tags:
  - scientia
  - continuum
  - socrates-policy
  - research-events
  - novelty-bundle
completed: "2026-11-14"
files_modified:
  - crates/vox-orchestrator-types/src/socrates_policy/continuum.rs
  - crates/vox-orchestrator-types/src/socrates_policy/mod.rs
  - crates/vox-orchestrator-types/src/lib.rs
  - crates/vox-research-shim/src/research/types.rs
  - crates/vox-research-shim/src/research/orchestrator/pipeline.rs
  - crates/vox-cli-research/src/lib.rs
  - crates/vox-research-events/src/events.rs
  - crates/vox-research-events/src/schema_types.rs
  - crates/vox-research-events/src/lib.rs
requirements_completed:
  - DISC-01
---

# Phase 447 Plan 01 Summary — L0/L1 Research Continuum Contracts, Scholarly Domain Mode & Canonical Novelty Bundle

## Delivered Artifacts

1. **Pure L0 Research Continuum Contracts (`crates/vox-orchestrator-types/src/socrates_policy/continuum.rs` — 374 LoC, `<400` budget):**
   - Added `ResearchContinuumMode` (`Off`, `#[default] Auto`, `Fast`, `Deep`) with `snake_case` serde, `as_str()`, `Display`, and case-insensitive `FromStr` (including `"quick"` -> `Fast` composer compatibility).
   - Added ordered `ResearchContinuumTier` (`Direct < LocalRag < QuickWeb < DeepMultiWave`) with `as_str()` and `next_tier()`.
   - Added `EscalationReason` (`LowLocalConfidence`, `HighContradiction`, `LowCragQuality`, `ExplicitDeepIntent`, `RecencySignal`, `CoverageParadox`).
   - Added `CragEscalationPolicy` (`Default`: `0.68`, `0.25`, `0.55`, `3`, `3`) with `initial_tier_for_mode`, `has_coverage_paradox`, and deterministic `evaluate_escalation` enforcing hard `Off`/`Fast` caps, progressive `Auto` escalation, and immediate `Deep` promotion.
   - Re-exported through `vox_orchestrator_types::socrates_policy` and `vox_orchestrator_types`.

2. **`ResearchDomainMode::Scholarly` (`vox-research-shim` & `vox-cli-research`):**
   - Added `Scholarly` to `ResearchDomainMode` in `crates/vox-research-shim/src/research/types.rs` with `as_str()`, `FromStr` (`"scholarly"`), and serde round-trip unit tests while keeping `General` as `#[default]`.
   - Updated all exhaustive `ResearchDomainMode` match arms across `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` (lines 196 and 925) and `crates/vox-cli-research/src/lib.rs` (line 696).

3. **L1 Watermarked Event Envelope & Canonical `NoveltyEvidenceBundle` (`vox-research-events`):**
   - Added `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, and `EpistemicDagDelta` to `ResearchEventKind` and `ResearchEvent` (`crates/vox-research-events/src/events.rs` — 392 LoC, `<400` budget).
   - Added `SequencedResearchEvent` (`seq: u64`, `session_id`, `recorded_at_ms`, `event`) and `AtomicU64`-backed `ResearchEventSequencer` (`new`, `with_start_seq`, `next_event`, `current_watermark`).
   - Hardened `NoveltyEvidenceBundle` and sub-types (`NoveltySource: Copy + Hash`, `RecencyBucket: Copy + Hash + Default`, `OverlapSummary: Default`, `NormalizedHit`, `QueryTrace`) in `crates/vox-research-events/src/schema_types.rs` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, `#[serde(deny_unknown_fields)]`, and `NoveltyEvidenceBundle::traces(&self) -> &[QueryTrace]`.
   - Re-exported `SequencedResearchEvent`, `ResearchEventSequencer`, `NormalizedHit`, `NoveltySource`, `OverlapSummary`, `QueryTrace`, and `RecencyBucket` from `crates/vox-research-events/src/lib.rs`.

## TDD RED -> GREEN Receipts

- **Task 1 RED (`/tmp/vox-447-01-t1-red.log`):**
  - Command: `timeout 900s cargo test -p vox-orchestrator-types continuum`
  - Result: Exit `101` (`test result: FAILED. 1 passed; 4 failed; 0 ignored; 0 measured; 46 filtered out`)
- **Task 2 RED (`/tmp/vox-447-01-t2-red.log`):**
  - Command: `timeout 900s cargo test -p vox-research-events`
  - Result: Exit `101` (`error: could not compile vox-research-events (lib test) due to 21 previous errors`)

## Verification Receipts

### 1. `timeout 900s cargo test -p vox-orchestrator-types` — PASS (exit 0)

```text
running 51 tests
...
test socrates_policy::continuum::tests::auto_progressive_escalation_and_deep_promotion ... ok
test socrates_policy::continuum::tests::continuum_tier_ordering_and_serde ... ok
test socrates_policy::continuum::tests::continuum_mode_serde_and_from_str_round_trip ... ok
test socrates_policy::continuum::tests::escalation_reason_serde_snake_case ... ok
test socrates_policy::continuum::tests::policy_defaults_and_off_fast_caps ... ok
...
test result: ok. 51 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
Running tests/providers_and_routes.rs: 3 passed; 0 failed
Running tests/routing_and_ids_smoke.rs: 5 passed; 0 failed
Doc-tests vox_orchestrator_types: 2 passed; 0 failed
```

### 2. `timeout 900s cargo test -p vox-research-events` — PASS (exit 0)

```text
running 37 tests
...
test events::tests::continuum_wave_dag_variants_and_sequencer_round_trip ... ok
test events::tests::event_kind_matches_variant ... ok
test events::tests::preregistration_submitted_event_serializes ... ok
test events::tests::research_event_kind_round_trips ... ok
test schema_types::tests::novelty_evidence_bundle_ergonomics_and_strict_schema ... ok
test schema_types::tests::novelty_evidence_bundle_round_trips ... ok
...
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
```

### 3. `timeout 900s cargo test -p vox-scientia --lib inspect_bridge` — PASS (exit 0)

```text
running 18 tests
test inspect_bridge::chronofact::tests::filter_keeps_past_hits ... ok
test inspect_bridge::conflict::tests::conflict_detected_when_opposing_polarity ... ok
test inspect_bridge::novelty::tests::all_sources_failed_is_insufficient_evidence ... ok
test inspect_bridge::novelty::tests::cached_trace_counts_as_success ... ok
test inspect_bridge::novelty::tests::empty_bundle_is_insufficient_evidence ... ok
test inspect_bridge::novelty::tests::high_score_is_not_novel ... ok
test inspect_bridge::novelty::tests::low_score_is_novel ... ok
test inspect_bridge::novelty::tests::mid_score_is_possibly_novel ... ok
...
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 329 filtered out; finished in 0.00s
```

### 4. `timeout 900s cargo run -p vox-arch-check` — PASS (exit 0)

```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6m 34s
Running `target/debug/vox-arch-check`
(0 errors; zero new Cargo dependencies or crate-edge inversions)
```

### 5. `timeout 900s cargo test -p vox-research-shim --lib` & `timeout 900s cargo check -p vox-cli-research` — Upstream Shared-Worktree Blocker Note

- During incremental check (`1571/1579` passed, including `vox-scientia`, `vox-populi`, and `vox-publisher`), `vox-orchestrator` (`979/981` / `1572/1579`) failed to compile due to an unrelated untracked file left in the shared working tree by another concurrent workstream (`?? crates/vox-orchestrator/src/oversight/exec_policy.rs`, lines 448 & 465: `error[E0277]: can't compare str with &str` in `in_w!(a, ...)`).
- Per strict scope isolation (`00-core.md` Rule 1), `crates/vox-orchestrator/src/oversight/exec_policy.rs` was not modified. All `ResearchDomainMode` match sites across the workspace (`rg -n "ResearchDomainMode" crates/`) were verified updated (`types.rs`, `pipeline.rs:196,925`, `vox-cli-research/src/lib.rs:696`).

### 6. Structural & Formatting Receipts

- `wc -l crates/vox-orchestrator-types/src/socrates_policy/continuum.rs` -> `374` (`< 400` LoC)
- `wc -l crates/vox-research-events/src/events.rs` -> `392` (`< 400` LoC)
- All 9 touched `.rs` files formatted individually via `rustfmt --edition 2024 <file.rs>` (exit 0).
