---
title: "Mens Continuous Training, Multi-Spoke Data Flywheel & Evaluation SSoT (2026)"
description: "Single source of truth for the Mens 5-spoke training corpus contract, round-trip verification oracles, QLoRA + DPO curriculum pipeline, longitudinal scorecards, and continuous audit-to-corpus flywheel."
category: "Architecture SSOTs"
status: "current"
topic: "mens-training"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Mens Continuous Training, Multi-Spoke Data Flywheel & Evaluation SSoT (2026)

<!-- kb:summary -->
This document defines the canonical architecture for the Mens five-spoke continuous training, evaluation, and audit-to-corpus flywheel across VoxLang, RustWorkspace, StructuredShell, McpToolChain, and AgentHarness. It specifies the `TrainingRecordV2` schema and `VoxDb` Trusty dataset snapshots (`CORPUS-01..06`), deterministic text-in/text-out round-trip verification oracles (`SPOKE-01..06`), inverse-pass-rate curriculum weighting and QLoRA plus DPO adapter calibration (`TRAIN-01..05`), paired multi-spoke promotion gates (`EVAL-01..05`), and continuous audit-to-DPO feedback ratchets (`AUDIT-01..05`, `LOOP-01..04`).
<!-- /kb:summary -->

## 1. Overview & Five-Spoke Architecture

Mens trains and evaluates as a unified multi-spoke coding and orchestration model. Every record admitted into the training corpus, every curriculum batch, and every promotion gate operates across five canonical spokes:

| Spoke Enum (`SpokeKind` / `MensEvalSpoke`) | Identifier | Deterministic Verification Oracle |
|---|---|---|
| `VoxLang` | `vox_lang` | `vox_compiler` lexer, parser, typechecker (`check_module`), and retired-decorator grammar guard (`CORPUS-02`, `SPOKE-01`) |
| `RustWorkspace` | `rust_workspace` | Balanced-delimiter syntax check, item signature extraction, and `AGENTS.md` policy guard banning direct crypto imports (`CORPUS-03`, `SPOKE-02`) |
| `StructuredShell` | `structured_shell` / `shell_command` | `ShellSafetyClassification` (`Safe`, `Forbidden`, `Chained`) enforcing single-command discipline and secret redaction (`CORPUS-04`, `SPOKE-03`) |
| `McpToolChain` | `mcp_tool_chain` / `mcp_tool` | JSON schema and tool-registry argument validator (`SPOKE-04`) |
| `AgentHarness` | `agent_harness` | Multi-step plan and tool-trajectory validator (`SPOKE-05`) |

## 2. Persistent Multi-Spoke Training Data Schema & VoxDb Snapshots (`CORPUS-01..06`)

All training records conform to `TrainingRecordV2` (`schema_version = "vox.training_record.v2"`) in `crates/vox-corpus/src/training/contract.rs`:

- **`SpokeKind`**: Explicit 5-spoke discriminator (`vox_lang`, `rust_workspace`, `structured_shell`, `mcp_tool_chain`, `agent_harness`).
- **`OracleReceipt`**: Carries `spoke`, `passed: bool`, `stage`, `diagnostics`, and `exit_code`. No record with `passed == false` is admitted to positive SFT training.
- **`RecordProvenance`**: Tracks `origin`, `source_id`, `task_family`, and `difficulty`.
- **Deduplication & Decontamination (`CORPUS-05`)**: `deduplicate_records_v2` and `decontaminate_against_holdout` in `crates/vox-corpus/src/corpus/eval_split.rs` apply per-spoke token-trigram Jaccard near-duplicate filtering and holdout decontamination; the Jaccard threshold is a caller-supplied parameter (`max_near_dup_jaccard` / `max_jaccard`), not a fixed constant.
- **Immutable `VoxDb` Snapshots (`CORPUS-06`)**: `build_trusty_snapshot`, `persist_snapshot_to_db`, and `load_snapshot_from_db` in `crates/vox-corpus/src/dataset_snapshot.rs` seal datasets under content-addressed `trusty://vox-dataset/<digest>` URIs in `VoxDb`.

## 3. Text-In / Text-Out Round-Trip Oracles (`SPOKE-01..06`)

`crates/vox-corpus/src/training/spoke_roundtrip.rs` provides deterministic round-trip verification (`prompt -> candidate -> oracle -> MensSpokeScorecardV1`):

- `verify_voxlang_roundtrip` (`SPOKE-01`)
- `verify_rust_roundtrip` / `verify_rust_workspace_roundtrip` (`SPOKE-02`)
- `verify_shell_roundtrip` / `verify_structured_shell_roundtrip` (`SPOKE-03`)
- `verify_mcp_roundtrip` (`SPOKE-04`)
- `verify_agent_harness_roundtrip` (`SPOKE-05`)
- `evaluate_multi_spoke_suite` & `canonical_five_spoke_golden_suite` (`SPOKE-06`), emitting `contracts/reports/mens-spoke-scorecard.v1.json`.

## 4. Native QLoRA + DPO Pipeline, Curriculum Weighting & Adapter Calibration (`TRAIN-01..05`)

- **Snapshot Streaming (`TRAIN-01`)**: `ChosenRejectedPairV2`, `SftDpoBatch`, and `stream_snapshot_batches` in `crates/vox-corpus/src/training/mix_prepare.rs` stream token-bounded SFT + DPO batches directly from `TrustyDatasetSnapshot` URIs.
- **Inverse-Pass-Rate Curriculum Weighting (`TRAIN-02`)**: `SpokeCurriculumWeights` and `compute_curriculum_weights` in `crates/vox-corpus/src/training/curriculum.rs` up-weight weaker spokes (`weight_i ∝ max(floor, 1.0 - pass_at_1_i)`) while guaranteeing a minimum `0.10` (`DEFAULT_MIN_SPOKE_WEIGHT_FLOOR`) sampling floor per spoke to prevent catastrophic forgetting.
- **Checkpoint Lineage (`TRAIN-03`)**: `MensCheckpointManifestV1`, `persist_checkpoint_to_db`, and `load_checkpoint_from_db` in `crates/vox-corpus/src/training/adapter_registry.rs` record `snapshot_uri`, `adapter_uri`, `curriculum_weights`, `sft_step_count`, `dpo_step_count`, `initial_loss`, `final_loss`, `loss_monotonic_decreased`, and `spoke_adapter_gains`.
- **Model-Registry Calibration (`TRAIN-04`)**: `calibrate_adapter_from_scorecard` and `build_adapter_registry_entry` register verified Mens adapters (`MensAdapterRegistryEntry`) with empirical `quality_score = overall_pass_at_1` and promotion eligibility gated on zero spoke regressions.
- **Preflight Safety Gate (`TRAIN-05`)**: `validate_snapshot_safety_gate` in `crates/vox-corpus/src/training/preflight.rs` fails closed on empty spokes, unverified SFT records, unredacted secret patterns, or holdout contamination (`holdout_overlap_count > 0`).

## 5. Multi-Spoke Empirical Evaluation, Longitudinal Ledger & Promotion Gate (`EVAL-01..05`)

- **5-Spoke Scorecard (`EVAL-01`)**: `MensFiveSpokeScorecardV1` and `evaluate_mens_five_spokes` in `crates/vox-eval/src/mens.rs`.
- **Paired Comparison & Promotion Gate (`EVAL-02`, `EVAL-04`)**: `compare_mens_five_spokes`, `MultiSpokeComparisonV1`, and `PromotionGateVerdict` (`Promote`, `RejectRegression`, `HoldNoImprovement`) in `crates/vox-eval/src/compare.rs` enforce per-spoke non-inferiority (`losses == 0` and `delta_pass_at_1 >= -max_spoke_regression`) alongside strict overall `pass@1` improvement.
- **Longitudinal Ledger (`EVAL-03`)**: `MensLongitudinalScorecardV1` and `MensLongitudinalLedger` in `crates/vox-eval/src/trial_flywheel.rs` verify monotonic progression across flywheel cycles.
- **Report Rendering (`EVAL-05`)**: `render_multi_spoke_comparison_markdown` and `render_multi_spoke_comparison_json` in `crates/vox-eval/src/report.rs`.

## 6. Continuous Audit-to-Corpus Flywheel & Autonomous Burn-In (`AUDIT-01..05`, `LOOP-01..04`)

- **Rejected Attempt Harvesting (`AUDIT-01`)**: `harvest_rejected_attempts` in `crates/vox-audit/src/subcommands/corpus_feedback.rs`.
- **Verified Repair DPO Pairing (`AUDIT-02`)**: `pair_verified_repair_to_dpo_and_sft` and `pair_verified_repair_with_failure` in `crates/vox-audit/src/subcommands/repair_corpus.rs`.
- **Hard-Negative AST Mutation (`AUDIT-03`)**: `mint_hard_negative_dpo_pair` in `crates/vox-corpus/src/ast_mutator.rs`.
- **Per-Spoke Regression Ratchets (`AUDIT-04`)**: `SpokeRegressionRatchetV1` and `verify_spoke_regression_ratchet` in `crates/vox-audit/src/subcommands/regression_budget.rs`.
- **Behavioral Golden Promotion (`AUDIT-05`)**: `PromotedSpokeGolden` and `promote_verified_repair_to_golden` in `crates/vox-audit/src/subcommands/behavioral_goldens.rs`.
- **Two-Cycle Autonomous Burn-In (`LOOP-01..03`)**: `run_two_cycle_autonomous_burnin`, `persist_burnin_report_to_db`, and `load_burnin_report_from_db` in `crates/vox-corpus/src/flywheel.rs` execute `Cycle 0 -> Cycle 1 -> Cycle 2`, proving monotonic overall `pass@1` gain (`0.40 -> 0.80 -> 1.00`), zero spoke regressions, and full `VoxDb` CAS lineage in `contracts/reports/mens-flywheel-burnin.v1.json`.
