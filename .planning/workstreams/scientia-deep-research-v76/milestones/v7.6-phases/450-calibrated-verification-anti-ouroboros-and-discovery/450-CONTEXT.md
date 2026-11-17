---
phase: 450
title: "Calibrated Verification, Anti-Ouroboros, and Discovery"
slug: "calibrated-verification-anti-ouroboros-and-discovery"
milestone: "scientia-deep-research-v76"
status: "ready"
---

# Phase 450: Calibrated Verification, Anti-Ouroboros, and Discovery — Context

**Gathered:** 2026-11-15
**Status:** Ready for planning
**Requirements:** VERI-01, VERI-02, VERI-03, VERI-04, DISC-02, DISC-03, DISC-04

<domain>
## Phase Boundary

Phase 450 replaces mock/word-overlap claim checking, Epistemic Ouroboros self-ingestion, `None => Novel` false positives, and air-gapped scholarly deadlocks across three file-disjoint tracks:
- **Calibrated 3-Tier Verifier & Clavis Secrets (`VERI-01`, `VERI-02`):** `crates/vox-secrets/src/spec/{ids.rs, registry/scholarly.rs}`, `docs/src/reference/secrets-ssot.md`, `crates/vox-scientia/src/claim_extractor/{mod.rs, minicheck.rs, tier0_lexical.rs, pipeline.rs}`, `crates/vox-research-shim/src/research/{verifier.rs, gate.rs}`.
- **Scholarly Trust, GCRA Rate Governance & Anti-Ouroboros Taint (`VERI-03`, `VERI-04`, `DISC-03`):** `crates/vox-search/src/{trust.rs, openalex.rs}`, `crates/vox-research-shim/src/research/orchestrator/{pipeline.rs, web_gather.rs}`.
- **Fused Lexical+Semantic Novelty, `vox-crypto` Migration & Novelty × Utility Discovery (`VERI-02`, `DISC-02`, `DISC-04`):** `crates/vox-scientia/src/{lib.rs, inspect_bridge/{mod.rs, novelty.rs, lexical_shingles.rs}, producers/{mod.rs, worthiness.rs, worthiness_v2.rs, socrates_telemetry.rs, doc_corpus.rs, api_surface.rs, adr_emergence.rs, dep_adoption.rs, bench_history.rs, test_corpus.rs, mens_training_run.rs}}`, `crates/vox-publisher/src/scientia_prior_art.rs`, `crates/vox-cli-research/src/scout.rs`.

Deferred to Phase 451+: Direct Publication Flywheel & Studio UI (`PUBL-01..04`, `STUD-01..02`) and paired After-Audit Closure (`AUDT-02..04`).
</domain>

<decisions>
## Implementation Decisions

### 3-Tier Calibrated Verifier & Clavis Migration (`VERI-01`, `VERI-02`)
- **Zero production `Mock` fallback:** Gate `MiniCheckBackend::Mock` and `MiniCheckVerifier::mock()` strictly under `#[cfg(test)]`. Expose `pub const fn is_mock_backend(&self) -> bool` for external audit probes. Default `MiniCheckVerifier::from_env()` and `from_env_with_lookup` to `Self::calibrated_hybrid(...)` using Clavis (`SecretId::VoxMinicheckEndpoint`, `SecretId::VoxMinicheckModel`, `SecretId::VoxOpenAlexApiKey` as `SecretPolicy::optional_skip()`).
- **3-tier cascade (`tier0_lexical.rs` < 400 LoC, `minicheck.rs`, `verifier.rs`):**
  - *Tier 0 (`0` LLM calls):* deterministic token/quote containment, negation asymmetry (`not`/`never`/`without`/`does not`/`cannot`), directional antonym pairs (`reduce`/`faster`/`improve` vs `increase`/`slower`/`degrade`), and anchored numeric/percentage/`statcheck` mismatch scoring.
  - *Tier 1:* structured SciFact entailment (`Support | Contradict | NotEnoughInfo | Abstain`) routed exclusively through `vox_actor_runtime::llm::infer_with_retry` (removing `LlmConfig::openrouter`).
  - *Tier 2:* `3x` self-consistency resampling restricted strictly to uncertain scores in `[abstain_threshold, 0.80]`; scores below `abstain_threshold` classify as `Abstain` rather than coercing to `Contradicted`.
- **Span & contradiction gating (`gate.rs`):** Make `ClaimVerdict::Contradicted` reachable so `contradiction_penalty` fires, and audit `[n]` inline citations against extracted span text.

### Provenance Hygiene, Retraction & Rate Governance (`VERI-03`, `VERI-04`, `DISC-03`)
- **Anti-Ouroboros taint:** Stamp `memory_type: "research_result"` entries in `pipeline.rs` (`save_memory`) with `untrusted_synthesis` provenance (`[origin:untrusted_synthesis]`). In `web_gather.rs` (`append_local_lines`), reject or cap `untrusted_synthesis` hits (`trust_score <= 0.20`) and exclude them from `corroboration_counts`.
- **Scholarly trust & GCRA without Cargo edits:** `governor` is not in `vox-search/Cargo.toml`; implement `GcraLimiter` directly in `trust.rs` with `tokio::sync::Mutex` + `tokio::time::{Instant, Duration}` (Semantic Scholar `1 RPS` / `10 RPS` keyed, Crossref `5 RPS`, OpenAlex `10 RPS`, `3.5s` HTTP timeouts). Resolve `VoxOpenAlexEmail`, `VoxOpenAlexApiKey`, `VoxScientiaCrossrefMailto`, `VoxCrossrefPlusApiKey`, and `VoxSemanticScholarApiKey` via `vox_secrets::resolve_secret(...)`. Crossref `message.update-to` retraction/withdrawal forces `trust_score = 0.0`.
- **Standalone air-gapped path:** Provide explicit human-signed `OfflineAttested` outcome in `trust.rs` so offline promotion never deadlocks on `TrustLookup::Unavailable`.

### `vox-crypto`, Fused Novelty & Two-Stage Discovery (`VERI-02`, `DISC-02`, `DISC-04`)
- **No Cargo/crate-edge edits:** `vox-publisher` depends on `vox-scientia` (not `vox-crypto`), and `vox-scientia` does not depend on `vox-similarity`. Expose `compliance_hash_hex` and `interop_sha256_hex` on `vox-scientia/src/lib.rs`, replace all `sha3` imports in `producers/*.rs` and `sha2` in `scientia_prior_art.rs`, and implement 3-gram shingle Jaccard + 64-perm seeded MinHash using `vox_crypto::fast_hash` in `lexical_shingles.rs` (< 400 LoC).
- **Zero `None => Novel` false positives (`novelty.rs`):** Fuse calibrated Stage-1 lexical overlap with Stage-2 semantic cosine (`ChronoFilter` + `EvidenceConflict`). Return `NoveltyVerdict::InsufficientEvidence` whenever traces failed or signals are absent; return `NotNovel` on high lexical overlap even when `max_semantic_score` is `None`.
- **Novelty × Utility (`worthiness_v2.rs` < 400 LoC, `socrates_telemetry.rs`, `scout.rs`):** Require both novelty clearance (`Novel | PossiblyNovel`) and empirical utility proof (`>= 0.70`) before surfacing candidates to `scientia_discovery_inbox`.

### Architectural Invariants
- Zero edits to `Cargo.toml`, `Cargo.lock`, `package.json`, `layers.toml`, or `crate-edges.allow.v1.json`; `< 400` LoC per new file; zero literal `"/tmp"`, `"/var"`, `"/home"`; batched Wave F verification.
</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-cli-research/tests/fixtures/scientia_v76_cassettes.json` provides frozen `claim_pairs` and `novelty_bundles` (including `bundle-lexical-prior-art-1`) for deterministic offline verification.
- `crates/vox-secrets/src/spec/registry/scholarly.rs` already defines `VoxOpenAlexEmail`, `VoxScientiaCrossrefMailto`, `VoxCrossrefPlusApiKey`, and `VoxSemanticScholarApiKey`.
- `crates/vox-research-shim/src/research/orchestrator/pipeline.rs` (`save_memory`) and `web_gather.rs` (`append_local_lines`) currently allow prior model syntheses into local memory retrieval without taint separation.
</code_context>

<specifics>
## Parallel Wave Boundaries & Batched Verification

- **Wave D parallel file-disjoint workers:**
  1. `p450-verifier`: `vox-secrets` Clavis specs + `secrets-ssot.md` + `claim_extractor/*` + `research/{verifier,gate}.rs`.
  2. `p450-trust-ouroboros`: `vox-search/src/{trust,openalex}.rs` + `orchestrator/{pipeline,web_gather}.rs`.
  3. `p450-novelty-disc`: `vox-scientia/src/{lib,inspect_bridge/*,producers/*}.rs` + `scientia_prior_art.rs` + `scout.rs`.
- **Wave F (batched verification):** Run `cargo test` per affected crate sequentially alongside `vox audit code`, `vox ci secret-env-guard`, `vox ci secrets-parity`, and `vox-arch-check` after Phases 450–452 land so compilation/verification cost is paid once cleanly.
</specifics>

<deferred>
## Deferred Ideas

- Pure-Rust local `candle` cross-encoder reranker and `SPECTER2` citation-graph adapter (`RETR-F01`, `RETR-F02` in `v7.6.x`).
</deferred>

---
*Phase: 450-calibrated-verification-anti-ouroboros-and-discovery*
*Context gathered: 2026-11-15*
