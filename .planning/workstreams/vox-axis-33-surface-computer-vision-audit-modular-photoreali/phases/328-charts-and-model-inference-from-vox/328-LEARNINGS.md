---
phase: 328
phase_name: "charts-and-model-inference-from-vox"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts: []
---

# Phase 328 Learnings: charts-and-model-inference-from-vox

## Decisions

### Registered reshape and softmax in typeck/builtins.rs for Ty::Array(_) wi...
Registered `reshape` and `softmax` in `typeck/builtins.rs` for `Ty::Array(_)` without altering `D01_OPS` in `contracts/python/ecosystem-support.yaml`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 328-01-SUMMARY.md

---

### Implemented worker PNG fallback via stdlib zlib alongside matplotlib Agg...
Implemented worker PNG fallback via stdlib `zlib` alongside matplotlib `Agg` so headless/minimal Python environments render valid PNGs without returning frame data to the host.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 328-01-SUMMARY.md

---

### Refused .pt, .pth, .pkl, .pickle, and pickle magic \x80 on both host (bu...
Refused `.pt`, `.pth`, `.pkl`, `.pickle`, and pickle magic `\x80` on both host (`builtins.rs`) and worker (`tensor_ops.py`).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 328-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

