# Phase 91: Measurement Harness & Advisory Closure - Context

**Gathered:** 2026-10-27
**Status:** Ready for research (DEEP flag)

<domain>
## Phase Boundary

Measure, honestly, whether agents author VoxDiagram at least as reliably as Mermaid (EVAL-01), and stop presenting the advisory's unsourced error-rate table as fact (EVAL-02). The result is reported, not a go/no-go on migration. No renderer or corpus work.

Requirements: EVAL-01, EVAL-02.

</domain>

<decisions>
## Implementation Decisions

### Harness shape
- Paired design: the same ≥60 natural-language diagram tasks go to both arms (VoxDiagram, Mermaid) with the same model, same temperature, and grammar cards of comparable length and detail (the VoxDiagram reference/grammar card vs an equivalent Mermaid flowchart + sequence card). Prompts and cards are committed and frozen before the run.
- Task set: graph and sequence tasks in proportion to the docs corpus, spanning the hard cases (punctuation in labels, nested containers, edge labels, self-loops, sequence notes). Each task carries an expected node/edge (or actor/message) set.
- Models come only through `vox_actor_runtime::llm` (registry selection, none pinned, no vendor hostnames). The resolved model id of every call is recorded. Aim for at least two model families if the registry offers them; one is acceptable if that is all that is configured, and the report says so.
- Oracles are deterministic parsers, never an LLM judge: `vox_diagram::check` for VoxDiagram; the real Mermaid parser for Mermaid (the pinned mermaid@11.17.2 Node oracle already used in Phase 84, `docs-astro/tests/unit/mermaid-oracle.test.mjs`), not the heuristic `vox_validate_mermaid`. Research confirms the exact Mermaid entry point.
- Bounded repair: up to 2 repair turns per task per arm, each fed only that arm's own parser diagnostics (VoxDiagram `check` diagnostics/fixes; Mermaid parse error text). Metrics: first-pass validity and validity after repair, per arm, with Wilson 95% CIs; a paired difference with an appropriate paired interval.
- Secondary (reported, not in the decision rule): structural fidelity against the task's expected sets.

### Decision rule (fixed before the run)
- Written into the plan and the report before any live call: primary endpoint, non-inferiority margin, interval method, and how "non-inferior / inconclusive / inferior" map to the interval. Research proposes the margin and checks that 60+ tasks gives usable power; if not, raise the task count rather than loosen the rule.

### Reproducibility
- Every raw model output is recorded (JSONL, committed — small). A re-score path reads the recording and reproduces the tallies byte-for-byte with no network; that re-score runs as an ordinary offline test.
- The live run is a separate, explicitly invoked step (ignored test or `vox run` script), never part of default CI.

### Placement and dependencies
- No new crate edge beyond the two authorized (`vox-doc-pipeline→vox-diagram`, `vox-orchestrator-mcp→vox-diagram`) and no new npm/cargo dependencies. Candidate homes: `vox-orchestrator-mcp` (already depends on `vox-actor-runtime` and `vox-diagram`) or a `.vox` script per VoxScript-first. Research picks; the Mermaid oracle is invoked as a subprocess of the existing Node setup.
- Secrets through Clavis only.

### Live run gate
- If no LLM provider is configured when execution reaches the live run, stop and report it as a human action. Never fabricate, simulate, or hand-write model outputs as results.

### Report and advisory (EVAL-02)
- Report at `docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md` (name may be adjusted) with full frontmatter, `kb:summary`, method, decision rule, per-arm tables with CIs, the verdict the data supports, limitations, and the resolved model ids.
- `docs/src/architecture/diagramming-and-mermaid-evaluation-advisory-2026.md`: the "LLM Syntax Error Rate" row (line ~141) is marked unsourced in place and links to the measured report. No other rewriting of the advisory.

### Claude's Discretion
- Task authoring format, file layout of the harness and recordings, Mermaid card wording, the exact paired interval method (subject to research).

</decisions>

<code_context>
## Existing Code Insights

- `vox_actor_runtime::llm::{llm_chat, infer_with_retry}` in `crates/vox-actor-runtime/src/llm/chat.rs`; model registry in `vox-orchestrator::models`.
- `vox_diagram::check`, `fmt`, `mermaid::{import, export}`; MCP `vox_validate_diagram` (Phase 88) in `vox-orchestrator-mcp`.
- Mermaid Node oracle: `docs-astro/tests/unit/mermaid-oracle.test.mjs` (mermaid@11.17.2 + jsdom).
- Grammar card: `docs/src/reference/vox-diagram.md`.
- An existing nightly "Harness Eval" workflow and `crates/vox-eval` may hold reusable pieces (Wilson CI, recording format) — reuse before writing.

</code_context>

<deferred>
## Deferred Ideas

- Running the harness nightly in CI.
- Comparing against D2/PlantUML.

</deferred>
