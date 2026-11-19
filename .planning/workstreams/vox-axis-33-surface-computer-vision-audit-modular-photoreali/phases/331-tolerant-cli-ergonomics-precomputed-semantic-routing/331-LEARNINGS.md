---
phase: 331
phase_name: "tolerant-cli-ergonomics-precomputed-semantic-routing"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts: []
---

# Phase 331 Learnings: tolerant-cli-ergonomics-precomputed-semantic-routing

## Decisions

### Deliver the zero-overhead clap prefix/flag tolerance (CLI-TOL-01) and th...
Deliver the zero-overhead `clap` prefix/flag tolerance (`CLI-TOL-01`) and the pure-Rust `<1.5 ms` order-agnostic `argv_normalize` recovery engine (`CLI-TOL-02`) as a vertical tracer slice from `VoxCliRoot` through `CommandCatalog` and `CapabilityIndex`.

**Rationale:** Primary phase objective established in plan.
**Source:** 331-01-PLAN.md

---

### Wire the side_effect_class (read_only vs local_mutation / remote_mutatio...
Wire the `side_effect_class` (`read_only` vs `local_mutation` / `remote_mutation`) and caller-persona (`InteractiveHuman` vs `NonInteractiveOrAgent`) execution gate into `argv_normalize` and `crates/vox-cli/src/main.rs`, ensuring humans get frictionless read-only auto-repair and `[Y/n]` mutation safety while LLM agents and scripts receive deterministic exit-code-2 `aci.error_recovery` redirects without ever hanging on prompts.

**Rationale:** Primary phase objective established in plan.
**Source:** 331-02-PLAN.md

## Lessons

## Patterns

## Surprises

