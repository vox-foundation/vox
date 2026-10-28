# Milestone `v1.7` (`Phases 89–97`) Code Review & Fix Report (`97-REVIEW.md`)

**Date:** 2026-10-19
**Scope:** All source files created or modified across Milestone `v1.7` (`Phases 89–97`: `vox-telemetry`, `vox-telemetry-otlp`, `vox-db`, `vox-ops/*`, `vox-publisher`, `vox-gamify`, `vox-orchestrator`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-corpus`, `vox-cli`, `vox-ml-cli`, `vox-gui`, `docs-astro`, `contracts/`, `scripts/`).
**Mode:** `/gsd-code-review --fix`

## Findings Identified & Fixed

1. **`CR-97-01` (Warning — Fixed): Consecutive `BLAKE3` Row Hash-Chain Link Verification in `VerifiedLeaderboardSnapshotV1`**
   - **Location:** `crates/vox-gamify/src/verified_leaderboard.rs`, `contracts/ludus/leaderboard-snapshot.v1.json`, `docs-astro/src/data/leaderboard-snapshot.v1.json`
   - **Issue:** `verify_snapshot` checked that row 0 anchored to the 64-zero genesis hash and that the final row matched `chain_tip_blake3_hex`, but did not assert consecutive row linkage (`row[i].proof.prev_row_chain_hash_hex == row[i - 1].proof.row_chain_hash_hex`) or non-degeneracy (`row_chain_hash_hex != prev_row_chain_hash_hex`), and row 3 in `leaderboard-snapshot.v1.json` had duplicated row 2's chain hashes.
   - **Fix:** Updated row 3 in `contracts/ludus/leaderboard-snapshot.v1.json` and `docs-astro/src/data/leaderboard-snapshot.v1.json` to chain from row 2 (`a11f3d23... -> c94e8b7a...`) and enforced consecutive `prev_row_chain_hash_hex` linkage and non-degeneracy in `verify_snapshot`.

2. **`CR-97-02` (Warning — Fixed): Exhaustive $k \ge 20$ / $n \ge 50$ Cohort & Expiry Enforcement Across All 8 Prior Vectors in `CommunityIntelligenceBundleV1`**
   - **Location:** `crates/vox-orchestrator/src/models/intelligence_bundle.rs`
   - **Issue:** `CommunityIntelligenceBundleV1::verify_bundle_signature` checked `distinct_clients >= 20 && total_events >= 50` on `model_routing_priors`, but did not enforce the cohort floor on the other 7 prior vectors (`model_bench_spoke_priors`, `error_surface_priors`, `default_decision_priors`, `prompt_profile_priors`, `skill_reliability_priors`, `training_throughput_priors`, `domain_misguidance_hotspots`) or `expires_at_ms > generated_at_ms`.
   - **Fix:** Added `expires_at_ms > generated_at_ms` validation and enforced `distinct_clients >= k_anonymity_floor && total_events >= min_events_floor` across all 8 prior vectors.

3. **`CR-97-03` (Warning — Fixed): Checked Arithmetic & Tensor Descriptor Validation in `verify_adapter_file_blake3`**
   - **Location:** `crates/vox-corpus/src/training/adapter_registry.rs`
   - **Issue:** `verify_adapter_file_blake3` cast `u64` `header_len` to `usize` and computed `8 + header_len` without `checked_add`, and allowed a `.safetensors` file with an empty tensor body (`bytes.len() == 8 + header_len`) or missing `lora_a`/`lora_b` descriptors.
   - **Fix:** Used `usize::try_from` + `8usize.checked_add(header_len)`, enforced `bytes.len() > header_end`, and verified that the parsed JSON header contains both `lora_a` and `lora_b` descriptors.

## Verdict

- **Critical:** 0
- **Warning:** 0 remaining (3 fixed and verified)
- **Status:** **PASSED (`READY TO COMPLETE MILESTONE v1.7`)**
