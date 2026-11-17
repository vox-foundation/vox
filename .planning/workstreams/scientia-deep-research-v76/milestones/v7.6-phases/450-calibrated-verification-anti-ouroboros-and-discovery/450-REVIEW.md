---
phase: 450-calibrated-verification-anti-ouroboros-and-discovery
review_scope: plan 450-01 (Tasks 1, 2, 3 & Wave F verification)
reviewed_at: 2026-11-15
reviewer: rust-verifier
status: approved
---

# Phase 450 Code & Architecture Review

## Scope Reviewed
- `crates/vox-secrets/src/spec/{ids.rs,registry/scholarly.rs}`, `docs/src/reference/secrets-ssot.md`
- `crates/vox-scientia/src/{lib.rs,claim_extractor/*,inspect_bridge/*,producers/*}`
- `crates/vox-research-shim/src/research/{verifier.rs,gate.rs,orchestrator/{pipeline.rs,web_gather.rs}}`
- `crates/vox-search/src/{trust.rs,openalex.rs}`, `crates/vox-publisher/src/scientia_prior_art.rs`, `crates/vox-cli-research/src/scout.rs`

## Findings Summary
- **Blocking Issues:** `0`
- **Phase 450 Architectural / Policy Violations:** `0`

## 1. Security, Provenance & Calibration Integrity
- **Non-mock production verification (`VERI-01`, `VERI-02`):** `MiniCheckBackend::Mock` and `MiniCheckVerifier::mock()` are gated under `#[cfg(test)]`; `is_mock_backend()` returns `false` outside test builds (`mock_verifier_fallback_rate: 0.0`), and Tier 1 entailment calls `vox_actor_runtime::llm::infer_with_retry`.
- **Clavis & Cryptography (`VERI-02`, `DISC-03`):** All 3 new `SecretId` variants resolve via `vox_secrets::resolve_secret` and pass `vox ci secrets-parity` after contract sync (`508 secrets`). Producers and `scientia_prior_art.rs` route SHA hashing through `vox-scientia`'s `vox-crypto` facades (`0` direct `sha2`/`sha3` imports remaining in touched files).
- **Anti-Ouroboros & Novelty Safeguards (`VERI-03`, `VERI-04`, `DISC-02`):** `web_gather.rs` caps `untrusted_synthesis` entries and excludes them from corroboration counts; Crossref `update-to` forces `0.0`; `AtomicNoveltyScorer` returns `InsufficientEvidence` on failed/empty traces (`none_to_novel_false_positive_rate: 0.0`).

## 2. File Size & Dependency Discipline
| New Module | Observed LoC | Budget | Status |
|---|---|---|---|
| `crates/vox-scientia/src/claim_extractor/tier0_lexical.rs` | `379` | `<400` | Pass |
| `crates/vox-scientia/src/inspect_bridge/lexical_shingles.rs` | `180` | `<400` | Pass |
| `crates/vox-scientia/src/producers/worthiness_v2.rs` | `360` | `<400` | Pass |

- Zero edits to `Cargo.toml`, `Cargo.lock`, `layers.toml`, or `crate-edges.allow.v1.json`; `cargo run -p vox-arch-check` exited `0`; scoped `toestub --mode legacy` reported `0` critical and `0` error findings across all new Wave D modules.

## Verdict
**APPROVED** — All Phase 450 verification, trust, provenance, novelty, and continuous discovery invariants are verified green in Wave F.
