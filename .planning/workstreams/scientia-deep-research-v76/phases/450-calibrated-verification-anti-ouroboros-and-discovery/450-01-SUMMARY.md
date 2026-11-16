---
phase: 450-calibrated-verification-anti-ouroboros-and-discovery
plan: 01
subsystem: verification
tags: [rust, clavis, minicheck, nli, novelty, vox-crypto, anti-ouroboros, vox-scientia, vox-search, vox-research-shim]

requires:
  - phase: 447-before-audit-baseline-and-continuum-contracts
    provides: Frozen baseline Baseline Cassettes (`scientia_v76_cassettes.json`), `NoveltyEvidenceBundle` L1 schema, and L3 `CachedLlmEmbedder`
provides:
  - Clavis `SecretId::{VoxOpenAlexApiKey, VoxMinicheckEndpoint, VoxMinicheckModel}` (`SecretPolicy::optional_skip()`) synced into `contracts/secrets/`
  - 3-tier calibrated claim verifier (`tier0_lexical.rs` 379 LoC + `vox_actor_runtime::llm::infer_with_retry` SciFact Tier 1 + `[abstain, 0.80]` 3x Tier 2 resampling) with `MiniCheckBackend::Mock` gated to `#[cfg(test)]`
  - Post-synthesis citation span audit (`InlineCitationSpanAudit`), reachable `Contradicted` quality penalty (`gate.rs`), Crossref `update-to` hard-zero retraction, `GcraLimiter` (`1/10 RPS` S2, `5 RPS` Crossref, `10 RPS` OpenAlex), and `OfflineAttested` fallback (`trust.rs`, `openalex.rs`)
  - Anti-Ouroboros `untrusted_synthesis` taint tagging (`pipeline.rs`) and trust capping + corroboration exclusion (`web_gather.rs`)
  - `vox-crypto` hash facade migration (`compliance_hash_hex`, `interop_sha256_hex`), `lexical_shingles.rs` (180 LoC), fused `AtomicNoveltyScorer` (`0%` `None => Novel` false positives), and `WorthinessSignalsV2` Novelty x Utility continuous discovery (`worthiness_v2.rs` 360 LoC, `socrates_telemetry.rs`, `scout.rs`)
affects:
  - Phase 451 (`FlywheelGate`, `WorthinessRadarPanel`, `nanopub_build` novelty hydration)
  - Phase 452 (`scientia v76-audit --stage after` closure)

tech-stack:
  added: []
  patterns:
    - "Tiered verification cascade: deterministic Tier 0 (`0` LLM calls) -> model-agnostic `vox_actor_runtime::llm::infer_with_retry` Tier 1 -> bounded Tier 2 resampling in `[abstain_threshold, 0.80]`"
    - "Anti-Ouroboros provenance quarantine: synthetic `research_result` memory notes tagged `[origin:untrusted_synthesis]` capped and excluded from corroboration"
    - "Fused lexical+semantic novelty: 3-gram shingle Jaccard + 64-perm seeded MinHash (`vox_crypto::fast_hash`) paired with dense cosine similarity (`InsufficientEvidence` on failed/empty traces)"

key-files:
  created:
    - crates/vox-scientia/src/claim_extractor/tier0_lexical.rs
    - crates/vox-scientia/src/inspect_bridge/lexical_shingles.rs
    - crates/vox-scientia/src/producers/worthiness_v2.rs
  modified:
    - crates/vox-secrets/src/spec/ids.rs
    - crates/vox-secrets/src/spec/registry/scholarly.rs
    - docs/src/reference/secrets-ssot.md
    - crates/vox-scientia/src/claim_extractor/mod.rs
    - crates/vox-scientia/src/claim_extractor/minicheck.rs
    - crates/vox-scientia/src/claim_extractor/pipeline.rs
    - crates/vox-research-shim/src/research/verifier.rs
    - crates/vox-research-shim/src/research/gate.rs
    - crates/vox-search/src/trust.rs
    - crates/vox-search/src/openalex.rs
    - crates/vox-research-shim/src/research/orchestrator/pipeline.rs
    - crates/vox-research-shim/src/research/orchestrator/web_gather.rs
    - crates/vox-scientia/src/lib.rs
    - crates/vox-scientia/src/inspect_bridge/mod.rs
    - crates/vox-scientia/src/inspect_bridge/novelty.rs
    - crates/vox-scientia/src/producers/mod.rs
    - crates/vox-scientia/src/producers/socrates_telemetry.rs
    - crates/vox-scientia/src/producers/doc_corpus.rs
    - crates/vox-scientia/src/producers/api_surface.rs
    - crates/vox-scientia/src/producers/adr_emergence.rs
    - crates/vox-scientia/src/producers/dep_adoption.rs
    - crates/vox-scientia/src/producers/bench_history.rs
    - crates/vox-scientia/src/producers/test_corpus.rs
    - crates/vox-scientia/src/producers/mens_training_run.rs
    - crates/vox-publisher/src/scientia_prior_art.rs
    - crates/vox-cli-research/src/scout.rs

key-decisions:
  - "Retired non-test `MiniCheckBackend::Mock` default in `minicheck.rs` via `#[cfg(test)]` gating and Clavis-resolved `CalibratedHybrid`."
  - "Implemented `GcraLimiter` natively over `tokio::sync::Mutex` and `tokio::time::{Instant, Duration}` in `trust.rs` with zero `Cargo.toml` edits."
  - "Exposed `compliance_hash_hex` and `interop_sha256_hex` from `vox-scientia` (delegating to `vox-crypto`) to keep `crate-edges.allow.v1.json` and `layers.toml` untouched."

requirements-completed: [VERI-01, VERI-02, VERI-03, VERI-04, DISC-02, DISC-03, DISC-04]

duration: 45min
completed: 2026-11-15
---

# Phase 450 Plan 01: Calibrated 3-Tier Verifier, Two-Stage Novelty x Utility & Continuous Discovery Summary

**Phase 450 Plan 01 retired production mock claim checking, `None => Novel` false positives, Epistemic Ouroboros self-ingestion, direct `sha2`/`sha3` imports in SCIENTIA producers/prior-art, and air-gapped scholarly deadlocks across Wave D (`p450-verifier`, `p450-trust-ouroboros`, `p450-novelty-disc`).**

## Accomplishments

### Task 1 (`p450-verifier`): Clavis Secrets, 3-Tier Calibrated Verifier & Citation Gate (`VERI-01`, `VERI-02`)
- Registered `SecretId::VoxOpenAlexApiKey`, `SecretId::VoxMinicheckEndpoint`, and `SecretId::VoxMinicheckModel` (`SecretPolicy::optional_skip()`) in `crates/vox-secrets/src/spec/{ids.rs, registry/scholarly.rs}` and documented them in `docs/src/reference/secrets-ssot.md`.
- Created `crates/vox-scientia/src/claim_extractor/tier0_lexical.rs` (`379` LoC, `< 400`) implementing deterministic containment, negation asymmetry, directional antonym flips, and numeric/`statcheck` mismatch scoring, exported via `claim_extractor/mod.rs`.
- Gated `MiniCheckBackend::Mock` and `MiniCheckVerifier::mock()` behind `#[cfg(test)]` in `minicheck.rs`, exposed `is_mock_backend() -> bool`, wired Clavis resolution + `CalibratedHybrid` default, and routed Tier 1 entailment through `vox_actor_runtime::llm::infer_with_retry`.
- Wired Tier 0/1/2 resampling (`[abstain_threshold, 0.80]`), `Verdict::Abstain`, `InlineCitationSpanAudit`, and reachable `Contradicted` quality penalty in `crates/vox-research-shim/src/research/{verifier.rs, gate.rs}`.

### Task 2 (`p450-trust-ouroboros`): GCRA Rate Limiters, Retraction Watch, `OfflineAttested` & Anti-Ouroboros (`VERI-03`, `VERI-04`, `DISC-03`)
- Added `GcraLimiter` (`1/10 RPS` Semantic Scholar, `5 RPS` Crossref, `10 RPS` OpenAlex), `SCHOLARLY_HTTP_TIMEOUT` (`3.5s`), Clavis secret resolution, Crossref `update-to` hard-zero retraction (`parse_crossref_retraction_json`), and `OfflineAttested` air-gapped fallback in `crates/vox-search/src/trust.rs` and `openalex.rs`.
- Tagged persisted `research_result` syntheses with `untrusted_synthesis` provenance (`pipeline.rs`) and enforced trust capping (`0.15`) + corroboration exclusion in `web_gather.rs` (`append_local_lines`).

### Task 3 (`p450-novelty-disc`): `vox-crypto` Migration, Fused Novelty & Novelty x Utility (`VERI-02`, `DISC-02`, `DISC-04`)
- Exposed `compliance_hash_hex` and `interop_sha256_hex` in `crates/vox-scientia/src/lib.rs` (backed by `vox_crypto`), migrating all 8 `producers/*.rs` files and `crates/vox-publisher/src/scientia_prior_art.rs` off direct `sha2`/`sha3` imports alongside Clavis + `3.5s` HTTP timeouts.
- Created `crates/vox-scientia/src/inspect_bridge/lexical_shingles.rs` (`180` LoC, `< 400`) and upgraded `AtomicNoveltyScorer` (`novelty.rs`) to fuse 3-gram shingle Jaccard & 64-perm seeded MinHash with dense cosine similarity (`ChronoFilter`, `InsufficientEvidence` when traces fail or embeddings are absent).
- Created `crates/vox-scientia/src/producers/worthiness_v2.rs` (`360` LoC, `< 400`), added high-entropy abstention/contradiction cluster mining in `socrates_telemetry.rs`, and gated `crates/vox-cli-research/src/scout.rs` candidate promotion through two-stage Novelty x Utility (`WorthinessSignalsV2`).

## Verification Receipts (Observed in Wave F)
- `cargo test -p vox-secrets --lib`: `129 passed; 0 failed` (`0.78s`).
- `cargo test -p vox-search --lib`: `279 passed; 0 failed; 1 ignored` (`12.13s`; includes `trust::tests::*`, `openalex::tests::*`).
- `cargo test -p vox-scientia --lib`: `376 passed; 0 failed; 1 ignored` (`15.48s`; includes `claim_extractor`, `producers`, `inspect_bridge`).
- `cargo test -p vox-publisher --lib`: `197 passed; 0 failed` (`32.05s`).
- `cargo test -p vox-research-shim --lib`: `171 passed; 0 failed` (`47.69s`).
- `cargo test -p vox-cli-research --lib` (`665 passed; 0 failed; 2 ignored`) + re-run `cargo test -p vox-cli-research --lib scientia` (`30 passed; 0 failed`).
- `./target/debug/vox ci secrets-contracts` (`508 secrets`) + `./target/debug/vox ci secrets-parity`: `secrets-parity OK`.

---
*Phase: 450-calibrated-verification-anti-ouroboros-and-discovery*
*Completed: 2026-11-15*
