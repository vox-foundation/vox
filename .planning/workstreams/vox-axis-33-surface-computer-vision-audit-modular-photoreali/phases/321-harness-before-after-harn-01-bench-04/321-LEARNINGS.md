---
phase: 321
phase_name: "harness-before-after-harn-01-bench-04"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 3
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 321 Learnings: harness-before-after-harn-01-bench-04

## Decisions

### HARN-01: resolve_harness_serve_target resolves any Qwen3 size spelling t...
HARN-01: resolve_harness_serve_target resolves any Qwen3 size spelling through detect::resolve_base_pin, verifies snapshot_complete, and when an adapter is provided enforces adapter_manifest_names_snapshot + assert_collateral_gate + assert_serve_preconditions.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 321-01-SUMMARY.md

---

### BENCH-04: write_harness_before_after_report emits harness_before_after.v...
BENCH-04: write_harness_before_after_report emits harness_before_after.v1 with base and adapter scores across all 3 reasoning slices (n=32) plus intelligence (pass_rate_at_1, n) for trained sizes, while keeping untrained sizes free of numeric score keys.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 321-01-SUMMARY.md

## Lessons

## Patterns

### milestone_ready is computed directly from trained_scored_count >= 2.
milestone_ready is computed directly from trained_scored_count >= 2.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 321-01-SUMMARY.md

---

### Chosen-size serve resolution behind assert_collateral_gate + assert_serv...
Chosen-size serve resolution behind assert_collateral_gate + assert_serve_preconditions

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 321-01-SUMMARY.md

---

### Consolidated multi-size harness_before_after.v1 JSON report with reasoni...
Consolidated multi-size harness_before_after.v1 JSON report with reasoning + intelligence scores

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 321-01-SUMMARY.md

## Surprises

