# Phase 344: MENS Debt Sweep, Zero Paper Code, and Retired Spoke Cleanup - Context

**Gathered:** 2026-11-07
**Status:** Ready for planning
**Milestone:** v0.6 MENS Multi-Spoke & Full Ladder Gate Closure
**Source:** User-confirmed milestone decisions and repository health governance.

<domain>
## Phase Boundary

Phase 344 conducts a comprehensive hygiene sweep across the entire MENS training, evaluation, and serving subsystem, eliminating paper code, obsolete stubs, unreferenced fixtures, and fencing retired/future knowledge lanes.

Key deliverables:
1. **Paper Code & Dead Stub Elimination**:
   - Delete dead `bfcl` (Berkeley Function Calling Leaderboard) parsing/eval stubs that are unreferenced.
   - Delete orphaned baseline JSON files and legacy 32-item benchmark remnants.
   - Eliminate unused CLI flags, config options, and dead gate struct fields identified during Phase 338.
2. **Domain Profile Integrity & Knowledge Lane Fencing**:
   - Audit `mens/config/domain-profiles.yaml` across all 9 profiles:
     - **Active QLoRA Spokes (5)**: `vox-lang`, `rust`, `harness`, `tool-selection`, `argument-generation`.
     - **Retrieval / Grounding Lanes (3)**: `rocks`, `research`, `populi-meta` — cleanly documented without `base:` fields, tagged for retrieval via B3 embedder `Qwen/Qwen3-0.6B`.
     - **RLVR / Expert Lane (1)**: `research-expert` (Lane G) — cleanly preserved with `reward_hook: research_synthesis_eval` and trajectory weighting for future GRPO+RLVR cycles.
3. **Receipt and Manifest Coherence**:
   - Ensure all published run receipts, training manifests, and model cards across all sizes match live code reality.
   - Zero `TODO`, `FIXME`, or `unimplemented!` markers in core dispatch and evaluation modules.
4. **Final Workspace Audit**:
   - Run `vox audit code` and structural policy checks across `vox-populi`, `vox-eval`, and `vox-orchestrator-mcp`.
</domain>

<decisions>
## Implementation Decisions

### D-01: Zero Tolerance for Paper Code
- Any gate key, CLI argument, or profile option that is declared but not read by runtime logic must either be wired to real behavior or permanently deleted.

### D-02: Strict Knowledge Lane Demarcation
- Retrieval-only profiles (`rocks`, `research`, `populi-meta`) must fail fast if passed to `vox mens train --domain` with a clear explanation that they are retrieval corpora for the B3 embedder rather than fine-tuning targets.
</decisions>

<verification>
## Verification Requirements
- `cargo test -p vox-populi -p vox-eval` passes with 0 failures and 0 warnings.
- `vox audit code` passes with 0 policy violations in MENS crates.
- `domain-profiles.yaml` parses cleanly with all 9 profiles categorized.
</verification>
