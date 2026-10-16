# Workstream `voxdiagram` — Milestone v1.7 VoxDiagram

> Workstream-scoped milestone context. The shared `.planning/PROJECT.md` is owned by the parallel `unified` v1.6 milestone and is intentionally not edited here; fold this section into PROJECT.md when the two land together.

**Goal:** Retire Mermaid as the primary diagram *authoring* format for agents and humans: a typed `vox-diagram` text DSL compiles to deterministic SVG and sanitized Mermaid, and the live docs corpus (41 fences / 26 files under `docs/src`, excluding `archive/`) migrates to it.

**Target features:** `vox-diagram` L2 crate (DSL parser, typed IR, span diagnostics), `dagre`-backed layout + deterministic SVG, sanitized Mermaid export, strict Mermaid importer, `vox_validate_diagram` MCP tool + `vox diagram` CLI, docs-build pre-render, shrink-only legacy-Mermaid ratchet, full live-corpus migration.

**Out of scope (v1.7):** `diagram` keyword in vox-compiler/LSP; native Axis GUI rendering; `docs/src/archive/`; ER/class/Gantt kinds.

**Phases:** 83+ (v1.6 `unified` owns 77–82).

**Sources:** `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md` (error-rate table unsourced — see `research/PITFALLS.md`), `research/SUMMARY.md`.
