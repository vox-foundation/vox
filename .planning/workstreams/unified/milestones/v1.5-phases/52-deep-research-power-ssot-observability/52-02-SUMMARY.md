# Plan 52-02 Summary: Full `ResearchQuery` Controls & 4-Layer Epistemic DAG Serializer (`RES-02`)

## Completed
- Added `site_scope: Option<String>` to [`commands/research.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/src/commands/research.rs) (`research_run_params` and `start_research_async`).
- Added `EpistemicDagDto`, `EpistemicDagNodeDto`, `EpistemicDagEdgeDto`, and wave/quality metadata fields to [`dto/research.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/dto/research.rs).
- Implemented `build_epistemic_dag` in [`gui_ops/research.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/research.rs) constructing the 4-layer hierarchy (`query -> subquery -> source_domain -> claim -> sandbox_probe`) and wired it into `get_research_session_detail`.
- Verified with `builds_four_layer_epistemic_dag_from_artifact_json` (`11 passed`).
