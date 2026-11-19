---
phase: 418
phase_name: "public-front-door-and-release-governance-refresh"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 5
  patterns: 2
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 418 Learnings: public-front-door-and-release-governance-refresh

## Decisions

### Replaced obsolete Feb 2026 deleted-crate notes in RELEASE_NOTES.md with ...
Replaced obsolete Feb 2026 deleted-crate notes in `RELEASE_NOTES.md` with verified `0.6.0` (`v7.1`) capabilities and explicit maturity tiers.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 418-01-SUMMARY.md

---

### Removed forbidden manual last_updated frontmatter from CHANGELOG.md and ...
Removed forbidden manual `last_updated` frontmatter from `CHANGELOG.md` and recorded `v6.7`–`v7.1` milestones under `[Unreleased]`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 418-01-SUMMARY.md

---

### Updated CONTRIBUTING.md to guide contributors onto scoped cargo check -p...
Updated `CONTRIBUTING.md` to guide contributors onto scoped `cargo check -p <crate>`, `.vox` script automation (`vox run`), `vox ci pre-push`, and companion audit/doc binaries.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 418-01-SUMMARY.md

## Lessons

### Critical / High
**Critical / High:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 418-REVIEW.md

---

### Medium: 0
**Medium:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 418-REVIEW.md

---

### Low: 0
**Low:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 418-REVIEW.md

---

### rg -n "vox-parser|vox-typeck|vox-codegen-rust|vox-codegen-ts|vox-hir|las...
`rg -n "vox-parser|vox-typeck|vox-codegen-rust|vox-codegen-ts|vox-hir|last_updated" RELEASE_NOTES.md CHANGELOG.md CONTRIBUTING.md` → `exit 1` (`0` matches).

**Context:** Verified in phase acceptance checks.
**Source:** 418-VERIFICATION.md

---

### ./target/debug/vox-doc-pipeline --lint-only → exit 0 (vox-doc-pipeline l...
`./target/debug/vox-doc-pipeline --lint-only` → `exit 0` (`vox-doc-pipeline lint complete — no hard errors.`).

**Context:** Verified in phase acceptance checks.
**Source:** 418-VERIFICATION.md

## Patterns

### Front-door release governance aligned with live workspace crates and vox...
Front-door release governance aligned with live workspace crates and `vox-doc-pipeline` frontmatter rules

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 418-01-SUMMARY.md

---

### Machine-readable public readiness scorecard (v71-public-readiness-scorec...
Machine-readable public readiness scorecard (`v71-public-readiness-scorecard.v1.json`) capturing exit-0 receipts across all CI/audit gates

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 418-01-SUMMARY.md

## Surprises

