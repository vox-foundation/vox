---
phase: 397
phase_name: "standalone-crates-io-wave-2-21-crates"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 7
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 397 Learnings: standalone-crates-io-wave-2-21-crates

## Decisions

### Configured workspace metadata, README.md, .config/hakari.toml final-excl...
Configured workspace metadata, `README.md`, `.config/hakari.toml` `final-excludes`, and `0` `workspace-hack` dependencies across `vox-ast`, `vox-redact`, `vox-jsonschema-util`, `vox-http-client`, `vox-test-harness`, and `vox-drift-check`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-01-SUMMARY.md

---

### Decoupled vox-drift-check from unpublished workspace crates (vox-compile...
Decoupled `vox-drift-check` from unpublished workspace crates (`vox-compiler`, `vox-config`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-01-SUMMARY.md

---

### Registered Wave 2 crates in crates/_public.toml, contracts/distribution/...
Registered Wave 2 crates in `crates/_public.toml`, `contracts/distribution/profiles.v1.yaml`, and `crates/voxup/profiles.v1.yaml`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-01-SUMMARY.md

---

### Verified vox ci publish-dry-run (21/21 clean), cargo test -p voxup --tes...
Verified `vox ci publish-dry-run` (`21/21` clean), `cargo test -p voxup --test distribution_parity` (`10/10` passed), and `cargo package --list --allow-dirty`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-01-SUMMARY.md

---

### Prepared 12 additional L0/L1 foundation and protocol crates (vox-db-type...
Prepared 12 additional L0/L1 foundation and protocol crates (`vox-db-types`, `vox-orchestrator-types`, `vox-foundation`, `vox-telemetry`, `vox-quantize`, `vox-openai`, `vox-identity`, `vox-git`, `vox-hf-layout`, `vox-mesh-policy`, `vox-llm-egress`, `vox-oauth-pkce`) for standalone `crates.io` publication (`33` total crates in `crates/_public.toml`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-02-SUMMARY.md

---

### Vendored cases.v1.json (vox-db-types) and research-trial.v1.schema.json ...
Vendored `cases.v1.json` (`vox-db-types`) and `research-trial.v1.schema.json` (`vox-telemetry`) with strict contract parity tests to eliminate directory-escaping `include_str!` paths.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-02-SUMMARY.md

---

### Verified all 12 Wave 3 crates via cargo check, fixture parity tests in c...
Verified all 12 Wave 3 crates via `cargo check`, fixture parity tests in `cargo test`, and `vox ci publish-dry-run` (`33/33` crates).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 397-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

