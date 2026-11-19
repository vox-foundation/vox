---
phase: 62
phase_name: "standalone-crate-unbundling-crates-io"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 62 Learnings: standalone-crate-unbundling-crates-io

## Decisions

### [MODEL: FLASH-3.8] [PARALLEL-SAFE] Establish crates/_public.toml (CR-K2)...
`[MODEL: FLASH-3.8]` `[PARALLEL-SAFE]` Establish `crates/_public.toml` (`CR-K2`) and prepare the 7 highest-leverage standalone Rust crates (`vox-crypto`, `vox-mesh-types`, `vox-scaling-policy`, `vox-bounded-fs`, `vox-secrets`, `vox-kb-facets`, `vox-arch-check`) for `crates.io` publication so external Rust developers can adopt Vox's architecture linter (`vox-arch-check`) and secret vault (`vox-secrets` / `Clavis`) directly, unlocking Rust Foundation and Sovereign Tech Fund eligibility.

**Rationale:** Primary phase objective established in plan.
**Source:** 62-01-PLAN.md

## Lessons

### crates/_public.toml specifies the reverse-topological publication order ...
`crates/_public.toml` specifies the reverse-topological publication order for the Tier-1 public crate set (`vox-crypto`, `vox-mesh-types`, `vox-scaling-policy`, `vox-bounded-fs`, `vox-secrets`, `vox-kb-facets`, `vox-arch-check`).

**Context:** Verified in phase acceptance checks.
**Source:** 62-VERIFICATION.md

---

### Package metadata across all 7 standalone crates contains complete descri...
Package metadata across all 7 standalone crates contains complete descriptions, workspace licenses, repository links, workspace authors, and standalone README pointers.

**Context:** Verified in phase acceptance checks.
**Source:** 62-VERIFICATION.md

## Patterns

## Surprises

