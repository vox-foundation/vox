---
phase: 97
phase_name: "live-hardware-qlora-fine-tune-safetensors-5-spoke-eval"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 3
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 97 Learnings: live-hardware-qlora-fine-tune-safetensors-5-spoke-eval

## Decisions

### MensCheckpointManifestV1::emit_verified_safetensors_checkpoint
`MensCheckpointManifestV1::emit_verified_safetensors_checkpoint`

**Rationale:** Capability delivered per summary metadata.
**Source:** 97-01-SUMMARY.md

---

### MensCheckpointManifestV1::verify_adapter_file_blake3
`MensCheckpointManifestV1::verify_adapter_file_blake3`

**Rationale:** Capability delivered per summary metadata.
**Source:** 97-01-SUMMARY.md

## Lessons

### CR-97-01 (Warning — Fixed)
**`CR-97-01` (Warning — Fixed): Consecutive `BLAKE3` Row Hash-Chain Link Verification in `VerifiedLeaderboardSnapshotV1`**

**Context:** Surfaced in structured phase code review.
**Source:** 97-REVIEW.md

---

### Location: crates/vox-gamify/src/verified_leaderboard.rs, contracts/ludus...
**Location:** `crates/vox-gamify/src/verified_leaderboard.rs`, `contracts/ludus/leaderboard-snapshot.v1.json`, `docs-astro/src/data/leaderboard-snapshot.v1.json`

**Context:** Surfaced in structured phase code review.
**Source:** 97-REVIEW.md

---

### Issue: verify_snapshot checked that row 0 anchored to the 64-zero genesi...
**Issue:** `verify_snapshot` checked that row 0 anchored to the 64-zero genesis hash and that the final row matched `chain_tip_blake3_hex`, but did not assert consecutive row linkage (`row[i].proof.prev_row_chain_hash_hex == row[i - 1].proof.row_chain_hash_hex`) or non-degeneracy (`row_chain_hash_hex != prev_row_chain_hash_hex`), and row 3 in `leaderboard-snapshot.v1.json` had duplicated row 2's chain hashes.

**Context:** Surfaced in structured phase code review.
**Source:** 97-REVIEW.md

## Patterns

## Surprises

