---
phase: 397-standalone-crates-io-wave-2-21-crates
plan: 01
subsystem: packaging
tags: [crates-io, publish-dry-run, distribution-parity, hakari]
completed: 2026-11-10
requirements:
  - CRATES-IO-V68-01
  - CRATES-IO-V68-02
  - CRATES-IO-V68-03
  - CRATES-IO-V68-04
---

# Phase 397 Plan 01 Summary: Standalone `crates.io` Wave 2 Expansion (`15 -> 21` Publishable Crates)

## Accomplishments

1. **Wave 2 Standalone Crate Preparation (`CRATES-IO-V68-01..03`):**
   - Configured workspace metadata, `README.md`, `.config/hakari.toml` `final-excludes`, and `0` `workspace-hack` dependencies across `vox-ast`, `vox-redact`, `vox-jsonschema-util`, `vox-http-client`, `vox-test-harness`, and `vox-drift-check`.
   - Decoupled `vox-drift-check` from unpublished workspace crates (`vox-compiler`, `vox-config`).
   - Registered Wave 2 crates in `crates/_public.toml`, `contracts/distribution/profiles.v1.yaml`, and `crates/voxup/profiles.v1.yaml`.
2. **Verification (`CRATES-IO-V68-04`):**
   - Verified `vox ci publish-dry-run` (`21/21` clean), `cargo test -p voxup --test distribution_parity` (`10/10` passed), and `cargo package --list --allow-dirty`.
