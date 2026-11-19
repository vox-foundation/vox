---
phase: 55
phase_name: "autonomous-mermaid-validation-repair"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 55 Learnings: autonomous-mermaid-validation-repair

## Decisions

### MERMAID-01 (vox-doc-pipeline::pipeline::mermaid + vox_validate_mermaid)
**MERMAID-01 (`vox-doc-pipeline::pipeline::mermaid` + `vox_validate_mermaid`):** Build a deterministic Mermaid diagram validator and auto-fixer checking supported diagram types (`flowchart`, `graph`, `sequenceDiagram`, `stateDiagram`, `stateDiagram-v2`, `classDiagram`, `erDiagram`, `xychart-beta`), balanced `subgraph`/`end` and bracket delimiters, unquoted special characters (`()`, `[]`, `{}`) in node labels and `-- ... -->` edge labels, and raw HTML tags in unquoted labels. Wire into `vox-doc-pipeline` lint and expose `vox_validate_mermaid` in `vox-orchestrator-mcp`.

**Rationale:** Primary phase objective established in plan.
**Source:** 55-01-PLAN.md

## Lessons

## Patterns

## Surprises

