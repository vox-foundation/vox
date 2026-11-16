# Phase 447: Deterministic Before-Audit Baseline, L0/L1 Continuum Contracts & L3 Shared Embedder - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning
**Mode:** Auto-generated (pure infrastructure, contracts, schema migration & deterministic CLI baseline phase)

<domain>
## Phase Boundary

Lock an objective, deterministic frozen-cassette Before-audit baseline (`contracts/reports/scientia-v76-before-baseline.v1.json`) across all 6 evaluation axes (**performance**, **quality**, **verification**, **integration**, **Socrates calibration**, and **novelty detection**) and live bug reproduction counters prior to behavioral runtime changes (`AUDT-01`), while landing pure L0 continuum policy contracts (`vox-orchestrator-types::socrates_policy` + `ResearchDomainMode::Scholarly`), L1 `SequencedResearchEvent` wire watermarks + consolidated canonical `NoveltyEvidenceBundle` (`vox-research-events`), lifted L3 `vox-scientia::embedder::CachedLlmEmbedder` (`DISC-01`), and additive `VoxDb` `research_session_sources` + cursor-pagination queries.

</domain>

<decisions>
## Implementation Decisions

### the agent's Discretion
All implementation choices are at the agent's discretion within the locked Milestone v7.6 architectural invariants (`0` new Cargo/npm packages, `0` `layers.toml` / `crate-edges` violations, `<400`-LoC child modules, `100%` `vox_actor_runtime::llm`, offline deterministic frozen cassettes for `--stage before`, and thin backward-compatible re-exports where existing callers import `NoveltyEvidenceBundleV1` or `CachedLlmEmbedder`).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets & Integration Seams
- **Evaluation & CLI Research (`crates/vox-cli-research/` & `crates/vox-eval/`):** `crates/vox-cli-research/src/eval.rs` (currently hardcoding `multi_hop_pipeline_score = 1.0` at line 364), `crates/vox-cli-research/src/db/publication/embedder.rs` (`CachedLlmEmbedder` to lift to `crates/vox-scientia/src/embedder.rs`), and `crates/vox-eval/src/paired_ni.rs` (`paired_ni` intervals).
- **L0 & L1 Pure Contract Crates:** `crates/vox-orchestrator-types/src/socrates_policy/` (pure L0 policy types), `crates/vox-research-shim/src/research/types.rs` (`ResearchDomainMode`), and `crates/vox-research-events/src/` (L1 research event envelopes and `NoveltyEvidenceBundle`, consolidating duplicate `NoveltyEvidenceBundleV1` in `crates/vox-publisher/src/scientia_finding_ledger.rs`).
- **L3 Scientia & Database (`crates/vox-scientia/` & `crates/vox-db/`):** `vox-scientia` already depends on `vox-db`, `vox-actor-runtime`, and `vox-search`, enabling `vox-scientia::embedder` with zero new `Cargo.toml` edges; `crates/vox-db/src/store/` hosts `research_sessions` / `research_session_events` / `scientia_claims` queries where additive `research_session_sources` (`continuum_tier`, `escalated_from`, cursor-paginated `list_session_claims_page` / `list_session_sources_page`) belong.

</code_context>

<specifics>
## Specific Ideas

- Ensure `cargo run -p vox-cli -- scientia v76-audit --stage before` runs 100% deterministically offline against checked-in frozen replay cassettes (zero live network calls) and records exact pre-remediation bug counters (`research_scope` overwrite, `None => Novel` false-novelty rate, `artifact_json` wipe, `MiniCheckBackend::Mock` fallback rate, KV-cache prefix stability, IPC payload bytes, and `vox-gui` CLI-DB spawn count) into `contracts/reports/scientia-v76-before-baseline.v1.json`.

</specifics>

<deferred>
## Deferred Ideas

None — pure infrastructure, contract, migration, and baseline capture phase; behavioral fixes land in Phases 448–452 so the Before-baseline remains an honest pre-change measurement.

</deferred>
