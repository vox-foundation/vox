# Phase 87: Continuous Audit-to-Corpus Flywheel & Per-Spoke Regression Ratchets — Context

**Gathered:** 2026-10-16
**Status:** Complete

<domain>
## Phase Boundary

Deliver the continuous audit-to-corpus feedback loop (`AUDIT-01..05`):
- `AUDIT-01`: Harvest structured `RejectedAttempt` records across all 5 spokes in `crates/vox-audit/src/subcommands/corpus_feedback.rs`.
- `AUDIT-02`: Pair verified post-repair candidates (`chosen`) with pre-repair failing code (`rejected`) into `VerifiedRepairDpoRecord` in `crates/vox-audit/src/subcommands/repair_corpus.rs`.
- `AUDIT-03`: Deterministically mutate golden `.vox`, Rust, shell, and MCP tool-call records into hard-negative DPO pairs (`mint_hard_negative_dpo_pair`) in `crates/vox-corpus/src/ast_mutator.rs`.
- `AUDIT-04`: Enforce per-spoke non-decreasing `pass@1` regression ratchets (`SpokeRegressionRatchetV1`, `verify_spoke_regression_ratchet`) in `crates/vox-audit/src/subcommands/regression_budget.rs`.
- `AUDIT-05`: Promote verified multi-spoke repairs into permanent behavioral regression goldens (`PromotedSpokeGolden`, `promote_verified_repair_to_golden`) in `crates/vox-audit/src/subcommands/behavioral_goldens.rs`.

</domain>
