---
phase: "23"
slug: "generated-reference-expansion-and-automated-crate-api-docs"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-13"
---

# Phase 23 — Validation Strategy: Generated Reference Expansion & Automated Crate API Docs

> Per-phase validation contract for feedback sampling during execution. Source: `23-RESEARCH.md` mapped to requirements GEN-01 through GEN-05.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-cli`, `vox-cli-ci`, `vox-doc-pipeline`) + Astro Starlight `pnpm build` |
| **Config file** | `contracts/config/env-vars.v1.yaml`, `contracts/config/registry.v1.yaml`, `contracts/mcp/tool-registry.canonical.yaml`, `contracts/documentation/retired-symbols.v1.yaml`, `docs-astro/astro.config.mjs` |
| **Quick run command** | `timeout 120s cargo test -p vox-cli --test command_sync --test crate_api_docs` |
| **Full suite command** | `timeout 120s cargo run -p vox-cli -- ci ssot-drift && timeout 60s cargo run -p vox-doc-pipeline -- --lint-only && timeout 180s pnpm --dir docs-astro build` |
| **Estimated runtime** | unit tests ~5s; ssot-drift ~15s; doc-lint ~1s; astro build ~25s |

---

## Sampling Rate

- **After every task commit:** `timeout 60s cargo test -p <touched_crate>` + `rustfmt --edition 2024 <touched .rs files>`.
- **After every plan wave:** `cargo run -p vox-cli -- ci ssot-drift` + `cargo run -p vox-doc-pipeline -- --lint-only`.
- **Before completion:** Full CI checks (`ssot-drift`, `vox-doc-pipeline --lint-only`, Astro Starlight build).
- **Max feedback latency:** ~30 s.

---

## Requirement → Verification Map

| Requirement | Behavior | Test type | Automated command | Exists |
|---|---|---|---|---|
| **GEN-01** | `docs/src/reference/env-vars.generated.md` generated from `contracts/config/env-vars.v1.yaml` (851 variables across all crates) | Integration test | `cargo test -p vox-cli --test command_sync` | Plan 23-01 |
| **GEN-02** | `docs/src/reference/config-keys.generated.md` (110 knobs) and `mcp-tools.generated.md` (340 tools) generated from contracts | Integration test | `cargo test -p vox-cli --test command_sync` | Plan 23-01 |
| **GEN-03** | Generated docs updated and verified by `vox ci command-sync` with dead-path test coverage | CLI + CI gate | `cargo test -p vox-cli --test command_sync && cargo run -p vox-cli -- ci command-compliance` | Plan 23-01 |
| **GEN-04** | `docs/src/api/<crate>.generated.md` for all 127 crates + `index.generated.md` generated deterministically and wired into `ssot-drift` | Integration test + CI gate | `cargo test -p vox-cli --test crate_api_docs && cargo run -p vox-cli -- ci ssot-drift` | Plan 23-02 |
| **GEN-05** | High-priority language spec & runtime gaps closed: `build_catalog()` presence check and retired decorator prose check | CLI + CI gate | `cargo run -p vox-cli -- ci command-compliance && cargo run -p vox-cli -- ci retired-symbol-check` | Plan 23-03 |

---

## Verification Proof

- **Command Sync (`GEN-01`, `GEN-02`, `GEN-03`):**
  `cargo test -p vox-cli --test command_sync` (3/3 passing).
  `cargo run -p vox-cli -- ci command-compliance` (exit 0).
- **Crate API Docs (`GEN-04`):**
  `cargo test -p vox-cli --test crate_api_docs` (3/3 passing).
  `cargo run -p vox-cli -- ci crate-api-sync` (127 crate docs generated cleanly).
- **Language Spec & Runtime Doc Gaps (`GEN-05`):**
  `check_catalog_top_level_command_presence` passes in `command-compliance`.
  `cargo run -p vox-cli -- ci retired-symbol-check` passes with zero violations across all docs and policy surfaces.
- **End-to-End Build & Drift Verification:**
  `cargo run -p vox-cli -- ci ssot-drift` (exit 0).
  `cargo run -p vox-doc-pipeline -- --lint-only` (exit 0).
  `pnpm --dir docs-astro build` (828 pages built in 23.72s with 0 errors).
