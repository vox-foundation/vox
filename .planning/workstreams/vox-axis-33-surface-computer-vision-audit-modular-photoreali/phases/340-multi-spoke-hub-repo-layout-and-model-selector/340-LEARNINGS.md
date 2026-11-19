---
phase: 340
phase_name: "multi-spoke-hub-repo-layout-and-model-selector"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 340 Learnings: multi-spoke-hub-repo-layout-and-model-selector

## Decisions

### Implement the multi-spoke HuggingFace repository layout (spokes/<spoke>/...
Implement the multi-spoke HuggingFace repository layout (`spokes/<spoke>/`), top-level backward-compatible `vox-lang` aliasing, and the unified `spoke_gates.json` gate manifest across `vox-populi` and `vox-ml-cli`. **Ordering:** Wave 1 of Phase 340. Depends on Phase 338 gate receipts schema (plan 338-02) and Phase 339 model card infrastructure. **Purpose:** Move MENS from mono-spoke root repos to organized multi-spoke repositories (`brbrainerd/vox-mens-qwen3-<size>`) without breaking legacy clients or tools pulling from the root.

**Rationale:** Primary phase objective established in plan.
**Source:** 340-01-PLAN.md

---

### Implement the spoke-aware local model selector in vox-orchestrator and v...
Implement the spoke-aware local model selector in `vox-orchestrator` and `vox-populi`. The selector unifies live VRAM titration across all 6 Qwen3 sizes (`0.6B`..`32B`), checks `spoke_gates.json` to guarantee only passed adapters are chosen, dispatches tasks by file extension and lane tokens, routes retrieval lanes to the B3 embedder, and preserves the Lane G RLVR hook. **Ordering:** Wave 2 of Phase 340. Depends on plan 340-01 for `spoke_gates.json` and multi-spoke repo paths. **Purpose:** Provide a zero-manual-configuration, fail-safe inference path that always selects the right model size and verified specialized adapter for any coding or agentic turn.

**Rationale:** Primary phase objective established in plan.
**Source:** 340-02-PLAN.md

## Lessons

## Patterns

## Surprises

