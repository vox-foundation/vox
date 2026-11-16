---
phase: 55-autonomous-mermaid-validation-repair
plan: 01
type: execute
completed_at: 2026-10-14T11:55:00Z
files_modified:
  - crates/vox-doc-pipeline/src/pipeline/mermaid.rs
  - crates/vox-doc-pipeline/src/pipeline/types.rs
  - crates/vox-doc-pipeline/src/pipeline/mod.rs
  - crates/vox-doc-pipeline/src/pipeline/lint.rs
  - crates/vox-orchestrator/src/models/select.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mod.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/post_verification.rs
  - docs-astro/src/components/PageSidebar.astro
  - docs-astro/src/styles/reader-layout.css
  - docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md
  - docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md
  - docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md
requirements-completed:
  - MERMAID-01
  - MERMAID-02
  - MERMAID-03
  - MERMAID-04
---

# Phase 55 Plan 01–04 Summary: Autonomous Mermaid Observation, Validation, LLM Self-Repair & Astro Rendering

## Overview

Delivered end-to-end deterministic Mermaid diagram validation, automatic LLM turn observation and budgeted tool-call self-repair via `SelectionIntent::mermaid_repair()`, full `docs/src/` Mermaid audit and repair, and client-side Mermaid SVG rendering with error boundaries in `docs-astro` (`voxlang.org`).

## Key Changes

1. **Deterministic Mermaid Structural Validator & Auto-Fixer (`MERMAID-01`)**:
   - Created [`crates/vox-doc-pipeline/src/pipeline/mermaid.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/mermaid.rs) with `extract_mermaid_blocks`, `validate_and_repair_mermaid`, and `validate_and_repair_markdown_mermaid`.
   - Validates supported diagram headers (`flowchart`, `graph`, `sequenceDiagram`, `stateDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta`), balanced `subgraph`/`end` blocks, unquoted node labels containing `()`/`{}`/HTML tags, and unquoted `-- ... -->` edge labels containing special characters.
   - Wired `LintKind::InvalidMermaid` into [`crates/vox-doc-pipeline/src/pipeline/lint.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/lint.rs) and `--fix` auto-repair into [`crates/vox-doc-pipeline/src/pipeline/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/pipeline/mod.rs).

2. **Automatic LLM Turn Observation, Tool-Call Self-Repair & Model Selection (`MERMAID-02`)**:
   - Added `SelectionIntent::mermaid_repair()` (`TaskCategory::CodeGen`, `SelectionAxes::COST_FIRST`, `complexity: 3`, `max_cost_usd_per_call: Some(0.01)`) in [`crates/vox-orchestrator/src/models/select.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/models/select.rs).
   - Registered the `vox_validate_mermaid` MCP tool in [`crates/vox-orchestrator-mcp/src/dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch.rs) and implemented [`crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/mermaid_repair.rs).
   - Wired `observe_and_repair_turn_mermaid` into [`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs) and Markdown write verification into [`crates/vox-orchestrator-mcp/src/post_verification.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/post_verification.rs).

3. **Repository Mermaid Audit & `docs-astro` SVG Renderer (`MERMAID-03`)**:
   - Audited all 1,118 Markdown/MDX files in `docs/src/` and repaired broken Mermaid diagrams in `deep-research-waves-and-competitive-analysis-research-2026.md`, `as-any-casts-reduction-strategy-research-2026.md`, and `deep-research-full-surface-audit-and-roadmap-2026.md`.
   - Added client-side Mermaid SVG rendering (`mermaid@11` ESM) with strict error-boundary fallback in [`docs-astro/src/components/PageSidebar.astro`](file:///Users/brbrainerd/dev/vox/docs-astro/src/components/PageSidebar.astro) and [`docs-astro/src/styles/reader-layout.css`](file:///Users/brbrainerd/dev/vox/docs-astro/src/styles/reader-layout.css).

## Verification

- `cargo test -p vox-doc-pipeline`: 74 passed, 0 failed (`mermaid::tests::*` all green).
- `./target/debug/vox-doc-pipeline --lint-only`: 0 hard errors across `docs/src/`.
- `pnpm --dir docs-astro build`: 845 pages built cleanly.
