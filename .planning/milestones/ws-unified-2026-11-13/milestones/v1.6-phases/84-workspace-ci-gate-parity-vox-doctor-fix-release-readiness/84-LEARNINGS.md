---
phase: 84
phase_name: "workspace-ci-gate-parity-vox-doctor-fix-release-readiness"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 84 Learnings: workspace-ci-gate-parity-vox-doctor-fix-release-readiness

## Decisions

### Added --fix alias to DoctorArgs::auto_heal in crates/vox-cli/src/cli_arg...
Added `--fix` alias to `DoctorArgs::auto_heal` in `crates/vox-cli/src/cli_args.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Classified advisory provider/mesh/telemetry/registration checks (Populi ...
Classified advisory provider/mesh/telemetry/registration checks (`Populi mesh security defaults`, `Google AI Studio Key`, `Model Scoreboard`, `Model Catalog`, `Workspace Registration`) in `OPTIONAL_CHECK_NAMES` in `crates/vox-cli/src/commands/diagnostics/doctor/mod.rs` so `vox doctor` and `vox doctor --fix` exit `0` when all build-critical checks pass.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Regenerated crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts...
Regenerated `crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts` and `crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts` from live contracts.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Fixed domain
Fixed `domain: "language"` frontmatter facet in `docs/src/architecture/vox-vs-ts-empirical-benchmark-2026.md`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Verified ./target/debug/vox ci ssot-drift, ./target/debug/vox ci capabil...
Verified `./target/debug/vox ci ssot-drift`, `./target/debug/vox ci capability-index`, `./target/debug/vox ci gui-surface-coverage`, `./target/debug/vox ci gui-catalog-parity`, `./target/debug/vox ci crate-edges`, `./target/debug/vox-arch-check`, and `./target/debug/vox-doc-pipeline --lint-only` all exit `0`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-01-SUMMARY.md

---

### Verified all phase VERIFICATION.md artifacts across Phases 77–88 have st...
Verified all phase `VERIFICATION.md` artifacts across Phases `77–88` have `status: passed` with zero open Critical/Warning defects or deferred UAT items.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-02-SUMMARY.md

---

### Mirrored all Phase 79–84 planning, summary, and verification artifacts b...
Mirrored all Phase `79–84` planning, summary, and verification artifacts between `.planning/phases/` and `.planning/workstreams/unified/phases/`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-02-SUMMARY.md

---

### Updated .planning/ROADMAP.md, .planning/REQUIREMENTS.md, .planning/STATE...
Updated `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, and `.planning/workstreams/unified/{ROADMAP.md,REQUIREMENTS.md,STATE.md}` to reflect `100%` completion of Milestone `v1.6`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 84-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

