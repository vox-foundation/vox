---
phase: 450-calibrated-verification-anti-ouroboros-and-discovery
verified: 2026-11-15
verifier: rust-verifier
status: passed
requirements: [VERI-01, VERI-02, VERI-03, VERI-04, DISC-02, DISC-03, DISC-04]
---

# Phase 450 Verification Matrix

## Requirement Coverage
| Requirement | Plan | Verification Evidence | Status |
|---|---|---|---|
| **VERI-01** | `450-01` | 3-tier `tier0_lexical.rs` + `vox_actor_runtime::llm` Tier 1 + `[abstain, 0.80]` Tier 2 cascade verified in `vox-scientia --lib` (`376 passed`) and `vox-research-shim --lib` (`171 passed`); `claim_precision` `0.5 -> 1.0`, `contradiction_detection_recall` `0.333333 -> 1.0` in `scientia-v76-after-closure.v1.json` | Passed |
| **VERI-02** | `450-01` | `MiniCheckBackend::Mock` gated to `#[cfg(test)]` (`mock_verifier_fallback_rate: 0.0`); Clavis `vox ci secrets-parity` OK (`508 secrets`, `vox-secrets --lib` `129 passed`); `vox-crypto` migration verified in `vox-scientia --lib` (`376 passed`) & `vox-publisher --lib` (`197 passed`) | Passed |
| **VERI-03** | `450-01` | `InlineCitationSpanAudit`, reachable `Contradicted` quality penalty (`contradiction_false_support_rate: 0.666667 -> 0.0`), and Crossref `update-to` zeroing verified in `vox-research-shim --lib` (`171 passed`) and `vox-search --lib` (`279 passed`) | Passed |
| **VERI-04** | `450-01` | `untrusted_synthesis` provenance tagging (`pipeline.rs`) & trust cap + corroboration exclusion (`web_gather.rs`) verified in `vox-research-shim --lib` (`171 passed`) | Passed |
| **DISC-02** | `450-01` | `lexical_shingles.rs` + fused `AtomicNoveltyScorer` verified in `vox-scientia --lib`; `prior_art_recall_at_5` `0.5 -> 1.0`, `false_novelty_rate` `1.0 -> 0.0`, `none_to_novel_false_positive_rate: 0.0` | Passed |
| **DISC-03** | `450-01` | `GcraLimiter`, Clavis `VoxOpenAlexApiKey`, `3.5s` timeouts, and `OfflineAttested` verified in `vox-search --lib` (`offline_attested_resolves_when_scholarly_api_unreachable`, `test_search_injects_polite_mailto_and_bearer_auth`, `279 passed`) and `vox-secrets --lib` (`129 passed`) | Passed |
| **DISC-04** | `450-01` | `WorthinessSignalsV2` two-stage Novelty x Utility filter and `socrates_telemetry.rs` cluster mining verified in `vox-scientia --lib` and `vox-cli-research --lib` (`665 passed`, `scout.rs`) | Passed |

## Key Wave F Command Receipts
- `cargo test -p vox-secrets --lib`: `129 passed; 0 failed` (`0.78s`)
- `cargo test -p vox-search --lib`: `279 passed; 0 failed; 1 ignored` (`12.13s`)
- `cargo test -p vox-scientia --lib`: `376 passed; 0 failed; 1 ignored` (`15.48s`)
- `cargo test -p vox-publisher --lib`: `197 passed; 0 failed` (`32.05s`)
- `cargo test -p vox-research-shim --lib`: `171 passed; 0 failed` (`47.69s`)
- `./target/debug/vox ci secrets-contracts` (`508 secrets`) + `./target/debug/vox ci secrets-parity`: `secrets-parity OK`
