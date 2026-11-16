# Phase 86: Multi-Spoke Empirical Evaluation Suite, Longitudinal Scorecards & Promotion Gates — Context

**Gathered:** 2026-10-16
**Status:** Complete

<domain>
## Phase Boundary

Deliver the 5-spoke empirical evaluation suite (`EVAL-01`), paired candidate-vs-baseline multi-spoke comparisons (`EVAL-02`), longitudinal scorecard ledger (`EVAL-03`), automated adapter promotion/rollback gate (`EVAL-04`), and canonical Markdown + JSON reporting (`EVAL-05`) in `crates/vox-eval/`.

</domain>

<decisions>
## Implementation Decisions

### Unified 5-Spoke Scorecard (`EVAL-01`)
- `MensEvalSpoke` (`VoxLang`, `RustWorkspace`, `ShellCommand`, `McpToolInvocation`, `AgentHarnessPlan`), `SpokeAttemptOutcome`, `SpokeProblemInput`, `SpokeEvalMetrics`, `MensFiveSpokeScorecardV1`, and `evaluate_mens_five_spokes` live in [`crates/vox-eval/src/mens.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-eval/src/mens.rs).

### Paired Multi-Spoke Comparison & Promotion Gate (`EVAL-02`, `EVAL-04`)
- `SpokeDeltaSummary`, `PromotionGateVerdict` (`Promote`, `RejectRegression`, `HoldNoImprovement`), `MultiSpokeComparisonV1`, and `compare_mens_five_spokes` live in [`crates/vox-eval/src/compare.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-eval/src/compare.rs).

### Longitudinal Scorecard Ledger (`EVAL-03`)
- `MensLongitudinalScorecardV1` and `MensLongitudinalLedger` (with `append_run`, `verify_monotonic_progression`, `to_jsonl`, `from_jsonl`) live in [`crates/vox-eval/src/trial_flywheel.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-eval/src/trial_flywheel.rs).

### Multi-Spoke Markdown + JSON Reporting (`EVAL-05`)
- `render_multi_spoke_comparison_markdown` and `render_multi_spoke_comparison_json` live in [`crates/vox-eval/src/report.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-eval/src/report.rs).

</decisions>
