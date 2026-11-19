---
phase: 80
phase_name: "real-backend-e2e-verification-harness-zero-mock-surface-proof"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 5
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 80 Learnings: real-backend-e2e-verification-harness-zero-mock-surface-proof

## Decisions

### Enforced 32-surface honesty via surfaceHonesty.guard.test.ts and vox ci ...
Enforced 32-surface honesty via surfaceHonesty.guard.test.ts and vox ci gui-honesty with zero placeholder prose, dead handlers, or unguarded mocks.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 80-01-SUMMARY.md

---

### Verified Chat/Loquela session lifecycle, oversight RPCs, @kb scope toggl...
Verified Chat/Loquela session lifecycle, oversight RPCs, @kb scope toggles, TurnCostReceipt badges, Terminal SessionHost OSC-633 blocks, and Explorer read-only execution.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 80-02-SUMMARY.md

## Lessons

## Patterns

### Zero-Mock Surface Guard
Zero-Mock Surface Guard: Every registered GUI surface is statically scanned in CI.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 80-01-SUMMARY.md

---

### real-backend-bridge
real-backend-bridge

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 80-01-SUMMARY.md

---

### static-surface-honesty-guard
static-surface-honesty-guard

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 80-01-SUMMARY.md

---

### Live RPC round-trip verification for core daily-driver surfaces.
Live RPC round-trip verification for core daily-driver surfaces.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 80-02-SUMMARY.md

---

### real-backend-ipc-roundtrip
real-backend-ipc-roundtrip

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 80-02-SUMMARY.md

## Surprises

