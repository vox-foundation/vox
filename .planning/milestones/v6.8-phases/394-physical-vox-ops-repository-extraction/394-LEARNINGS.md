---
phase: 394
phase_name: "physical-vox-ops-repository-extraction"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 394 Learnings: physical-vox-ops-repository-extraction

## Decisions

### Initialized /Users/brbrainerd/dev/vox-ops (git init -b main) with a stan...
Initialized `/Users/brbrainerd/dev/vox-ops` (`git init -b main`) with a standalone Cargo workspace (`Cargo.toml`, `edition = "2024"`, `version = "0.6.0"`, `license = "LicenseRef-VoxOps-Proprietary"`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 394-01-SUMMARY.md

---

### Synchronized contracts/{wire,ipc,telemetry}/ from vox into vox-ops and a...
Synchronized `contracts/{wire,ipc,telemetry}/` from `vox` into `vox-ops` and authored `vox-ops/AGENTS.md` + `vox-ops/README.md`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 394-01-SUMMARY.md

---

### Ported vox-plugin-cloud (RunPod, Vast.ai, Together AI provisioning adapt...
Ported `vox-plugin-cloud` (RunPod, Vast.ai, Together AI provisioning adapters) and created `vox-ops-telemetry-ingest` (OTLP/JSON receiver, schema validator, fleet aggregator).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 394-01-SUMMARY.md

---

### Verified cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo....
Verified `cargo test --manifest-path /Users/brbrainerd/dev/vox-ops/Cargo.toml`: `7/7` tests passed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 394-01-SUMMARY.md

---

### Verified vox ci standalone-sovereignty (zero dependency or path referenc...
Verified `vox ci standalone-sovereignty` (zero dependency or path references to `vox-ops` in root `Cargo.toml`; offline-safe telemetry).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 394-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

