---
phase: 397-standalone-crates-io-wave-2-21-crates
plan: 02
subsystem: packaging
tags: [crates-io, wave-3, distribution-parity, vendored-fixtures]
completed: 2026-11-10
requirements:
  - CRATES-IO-V68-01
  - CRATES-IO-V68-02
  - CRATES-IO-V68-03
  - CRATES-IO-V68-04
---

# Phase 397 Plan 02 Summary: Extended `crates.io` Wave 3 Preparation (`21 -> 33` Standalone Crates)

## Accomplishments

1. **Wave 3 Crate Decoupling (`CRATES-IO-V68-02..03`):**
   - Prepared 12 additional L0/L1 foundation and protocol crates (`vox-db-types`, `vox-orchestrator-types`, `vox-foundation`, `vox-telemetry`, `vox-quantize`, `vox-openai`, `vox-identity`, `vox-git`, `vox-hf-layout`, `vox-mesh-policy`, `vox-llm-egress`, `vox-oauth-pkce`) for standalone `crates.io` publication (`33` total crates in `crates/_public.toml`).
   - Vendored `cases.v1.json` (`vox-db-types`) and `research-trial.v1.schema.json` (`vox-telemetry`) with strict contract parity tests to eliminate directory-escaping `include_str!` paths.
2. **Verification (`CRATES-IO-V68-04`):**
   - Verified all 12 Wave 3 crates via `cargo check`, fixture parity tests in `cargo test`, and `vox ci publish-dry-run` (`33/33` crates).
