# Phase 341: Harness & Agent Spokes: Corpus Extraction, 0.6B Mono vs Split Training, and 72-Episode Gate - Context

**Gathered:** 2026-11-07
**Status:** Ready for planning
**Milestone:** v0.6 MENS Multi-Spoke & Full Ladder Gate Closure
**Source:** User-confirmed milestone decisions and `mens/config/domain-profiles.yaml` v1 sequencing.

<domain>
## Phase Boundary

Phase 341 executes the complete training and verification pipeline for the Harness and Agent spokes on `Qwen3-0.6B`, resolving the head-to-head evaluation between a unified mono adapter and decomposed micro-adapters.

Key deliverables:
1. **Deterministic Agent Corpus Extraction**:
   - Extract training datasets for:
     - `mix-harness.yaml` (union corpus of tool selection + argument generation)
     - `mix-tool-selection.yaml` (routing decisions, tool picking from catalog)
     - `mix-argument-generation.yaml` (schema-conforming argument synthesis)
   - Source: Verified MCP tool traces + live `tool-catalog.v1.json` from `vox-orchestrator-mcp`.
   - Ensure strict held-out separation against the 72-episode test suite.
2. **Profile Configuration Updates in `mens/config/domain-profiles.yaml`**:
   - Update `harness` profile with `base: { model: agentic_default, method: qlora, preset: qwen3_16g }` and `eval_gate: mens/config/eval-gates-agents.yaml`.
3. **Training & Gate Sequencing (v1 Sequencing Protocol)**:
   - Step 1: Train `Qwen3-0.6B` mono `harness` adapter (`grad_accum=8`, `rank=8`, `alpha=16`).
   - Step 2: Gate mono adapter against 72-episode 6-behavior `suite:agent_episodes` using live `tool-catalog.v1.json` and `eval-gates-ladder.yaml` collateral check.
   - Step 3: Train decomposed `tool-selection` and `argument-generation` `0.6B` adapters.
   - Step 4: Run head-to-head comparison on `suite:agent_episodes`.
   - Step 5: Adopt the winner (or route per-lane if decomposed exceeds mono on distinct sub-tasks).
4. **HuggingFace Multi-Spoke Synchronization**:
   - Publish passing adapter(s) to `brbrainerd/vox-mens-qwen3-0.6b` under `spokes/harness/`, `spokes/tool-selection/`, and/or `spokes/argument-generation/`.
   - Update `spoke_gates.json` and re-render 2-panel SVG strips (`overview.svg` and `scorecard.svg`).
</domain>

<decisions>
## Implementation Decisions

### D-01: v1 Sequencing Order
- Monolithic union adapter `harness` is trained first as the end-to-end smoke.
- Decomposed adapters are trained next with identical hyperparameters (`lr=2e-4`, `rank=8`, `alpha=16`, `grad_accum=8`) on their respective slices.
- The 72-episode suite serves as the objective judge.

### D-02: 72-Episode 6-Behavior Suite Execution
- 6 critical behaviors evaluated:
  1. Pick the correct tool for the intent
  2. Emit schema-valid JSON arguments matching parameter types
  3. Consume and reason over returned tool observations
  4. Recover from simulated tool failure / schema errors
  5. Conclude and stop when task is accomplished
  6. Explicitly refuse out-of-scope or unauthorized tool invocations
- Scored via `MockWorld` execution and strict JSONSchema validator, never text regex.

### D-03: Collateral Safety
- In addition to the agent suite, all candidate adapters must pass `eval-gates-ladder.yaml` reasoning preservation checks (GSM8K and MATH-500 slices).
</decisions>

<verification>
## Verification Requirements
- Extracted corpus JSONL passes schema validation and contains 0% leakage into test episodes.
- Training loss curves decrease monotonically without catastrophic divergence.
- `suite:agent_episodes` produces detailed JSON outcome report with per-behavior breakdown.
- `spoke_gates.json` reflects verified pass/fail for the agent lanes.
- SVG scorecard updated with agent benchmark deltas.
</verification>
