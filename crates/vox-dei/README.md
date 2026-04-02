# vox-dei (staging crate)

Minimal **workspace member**: `src/lib.rs` exposes Socrates-aligned floors; other directories under `src/` are **not** in the compiled module tree yet.

- Type-check: `cargo check -p vox-dei`
- When reattaching `research/` or `selection/`, wire them from `lib.rs` deliberately.

Do not add `vox-dei` as a dependency of `vox-cli` (CI: `vox ci no-vox-dei-import`); see **AGENTS.md**.

Runtime authority for retrieval triggers and Socrates telemetry is **`vox-mcp`** + **`vox-orchestrator`**.
