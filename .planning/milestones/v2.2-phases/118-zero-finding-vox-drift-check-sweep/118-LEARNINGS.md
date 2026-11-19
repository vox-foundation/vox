---
phase: 118
phase_name: "zero-finding-vox-drift-check-sweep"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 118 Learnings: zero-finding-vox-drift-check-sweep

## Decisions

### DRIFT-ZERO-01 (serde-default-dup)
**`DRIFT-ZERO-01` (`serde-default-dup`):** Replaced duplicate local `fn default_true() -> bool` definitions in `crates/vox-orchestrator-mcp/src/params.rs`, `crates/vox-plugin-browser/src/policy.rs`, `crates/vox-orchestrator/src/hopper/batch.rs`, and `crates/vox-search/src/policy.rs` with `use vox_config::serde_defaults::default_true;`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 118-01-SUMMARY.md

---

### DRIFT-ZERO-02 (version-string)
**`DRIFT-ZERO-02` (`version-string`):** Replaced hardcoded `"0.1.0"` in `crates/vox-eval/src/trial_flywheel.rs:981` with `env!("CARGO_PKG_VERSION")`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 118-01-SUMMARY.md

---

### DRIFT-ZERO-03 (reqwest-bypass)
**`DRIFT-ZERO-03` (`reqwest-bypass`):** Added `// drift-allow(reqwest-bypass): feature-gated OTLP exporter avoids vox-http-client crate edge` in `crates/vox-telemetry-otlp/src/upload.rs:60`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 118-01-SUMMARY.md

---

### DRIFT-ZERO-04 (vox-drift-check binary sync)
**`DRIFT-ZERO-04` (`vox-drift-check` binary sync):** Rebuilt `vox-drift-check` (`cargo build -p vox-drift-check`) and installed to `/Users/brbrainerd/.cargo/bin/vox-drift-check`, reducing workspace warning findings from `23` to `0` (`✓ No drift findings at Warning level or above.`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 118-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

