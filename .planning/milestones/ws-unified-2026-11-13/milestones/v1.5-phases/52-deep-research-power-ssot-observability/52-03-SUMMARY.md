# Plan 52-03 Summary: Wave 2+ Claim Re-Verification, Canonical `[N]` Citations, CoVE Repair & Cost/Memory Flywheel (`RES-03`)

## Completed
- Updated [`stages.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/orchestrator/stages.rs) (`format_packed_evidence_block`) to emit canonical `[u.unit_id + 1]` 1-based citation numbers, widened `judge_quality` and `run_self_verification` to `.take(15)`, and added `repair_inconsistent_synthesis`.
- Updated [`pipeline.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/orchestrator/pipeline.rs) to re-verify `{Contradicted, Contested, Unverified}` claims in Wave `2..=query.waves` after `all_hits.extend(wave_hits)`, invoke `repair_inconsistent_synthesis` when `self_verification.critical_inconsistency == true`, and persist `VoxDb::save_memory` (`memory_type = "research_result"`) + `VoxDb::insert_scientia_cost_telemetry` (`pipeline_phase = "scholarly"`).
- Verified with `cargo test -p vox-research-shim --lib` (`153 passed`).
