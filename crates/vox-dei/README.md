# vox-dei (workspace-excluded)

Fragment sources under `src/` are **not** wired into a full library graph yet. A minimal **`Cargo.toml`** + **`src/lib.rs`** exists so:

- Socrates-aligned research floors stay type-checked:  
  `cargo check --manifest-path crates/vox-dei/Cargo.toml`
- `research/orchestrator.rs` can reference [`vox_socrates_policy::ConfidencePolicy`] when that tree is reattached to the crate root.

Do not add `vox-dei` as a dependency of workspace members (`vox-cli`, etc.); see **AGENTS.md**.

Current runtime authority for retrieval triggers and Socrates surface telemetry is
`vox-mcp` + `vox-orchestrator`. Treat this crate as staged/deferred until the
historical research module graph is reattached to the workspace build.
