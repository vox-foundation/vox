---
phase: 405
phase_name: "structural-dependency-dag-decoupling-and-cli-binary-split"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "STATE.md"
---

# Phase 405 Learnings: structural-dependency-dag-decoupling-and-cli-binary-split

## Decisions

### Severed vox-repository completely from vox-compiler, vox-db, vox-codegen...
Severed `vox-repository` completely from `vox-compiler`, `vox-db`, `vox-codegen`, `vox-doc-inventory`, `vox-corpus`, and `vox-orchestrator-driver` using `AGENTS.md §Dependency Discipline` Rule 3 (`// vox:defactored-from vox-repository 2026-11-12` inline helpers) alongside `vox_repository::core` and `21` `#[inline(never)]` compilation firewalls.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 405-01-SUMMARY.md

---

### Pruned unused [dependencies] and relocated test-only dependencies to [de...
Pruned unused `[dependencies]` and relocated test-only dependencies to `[dev-dependencies]` across [crates/vox-cli/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-cli/Cargo.toml) (`max_workspace_deps` lowered `60 -> 50`), [crates/vox-compiler/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-compiler/Cargo.toml), [crates/vox-codegen/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/Cargo.toml) (retaining `vox-deploy-codegen` for `crates/vox-codegen-ts/src/emitter.rs`), [crates/vox-db/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-db/Cargo.toml), [crates/vox-orchestrator/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/Cargo.toml), [crates/vox-orchestrator-mcp/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/Cargo.toml), and [crates/vox-populi/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-populi/Cargo.toml). Synced [docs/src/api/vox-cli.generated.md](file:///Users/brbrainerd/dev/vox/docs/src/api/vox-cli.generated.md).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 405-01-SUMMARY.md

---

### Added [cargo_dep_hygiene.rs](file:///Users/brbrainerd/dev/vox/crates/vox...
Added [cargo_dep_hygiene.rs](file:///Users/brbrainerd/dev/vox/crates/vox-code-audit/src/detectors/cargo_dep_hygiene.rs) (`vox/cargo/dep-hygiene`, `Language::Unknown`), fixed [workspace_drift.rs](file:///Users/brbrainerd/dev/vox/crates/vox-code-audit/src/detectors/workspace_drift.rs) (`Language::Unknown`), wired manifest globs into [lefthook.yml](file:///Users/brbrainerd/dev/vox/lefthook.yml), added `[[forbidden_deps]]` + `max_workspace_deps = 50` in [layers.toml](file:///Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml), tightened [fan-in-snapshot.v1.json](file:///Users/brbrainerd/dev/vox/contracts/ci/fan-in-snapshot.v1.json) (`vox-compiler: 20`, `vox-config: 40`, `vox-db: 27`, `vox-llm-egress: 4`, `vox-repository: 13`), and guarded `-Wl,-dead_strip` in [dev_loop_guard.rs](file:///Users/brbrainerd/dev/vox/crates/vox-cli-ci/src/dev_loop_guard.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 405-01-SUMMARY.md

## Lessons

### total: 5 passed: 5 issues: 0 pending: 0 skipped: 0
total: 5 passed: 5 issues: 0 pending: 0 skipped: 0

**Context:** Observed during user acceptance testing.
**Source:** 405-UAT.md

---

### [none]
[none]

**Context:** Observed during user acceptance testing.
**Source:** 405-UAT.md

## Patterns

## Surprises

