---
phase: "21"
slug: "measure-and-prune"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-13"
---

# Phase 21 — Validation Strategy: Measure & Prune

> Per-phase validation contract for feedback sampling during execution. Source: `21-RESEARCH.md` mapped to requirements MEASURE-01 through MEASURE-06.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-doc-pipeline`, `vox-cli`, `vox-cli-ci`) + Playwright link/preview tests |
| **Config file** | `Cargo.toml`, `contracts/reports/docs-reality-audit/*.schema.json`, `docs-astro/playwright.config.ts` |
| **Quick run command** | `timeout 120s cargo test -p vox-doc-pipeline claims` |
| **Full suite command** | `timeout 180s cargo test -p vox-doc-pipeline -p vox-cli -p vox-cli-ci && timeout 120s cargo run -p vox-cli -- audit docs --format text && pnpm --dir docs-astro test` |
| **Estimated runtime** | unit tests ~10s; audit run ~5s; link validation ~20s |

---

## Sampling Rate

- **After every task commit:** `timeout 120s cargo test -p vox-doc-pipeline claims` + `rustfmt --edition 2024 <touched .rs files>`.
- **After every plan wave:** `timeout 180s cargo test -p vox-doc-pipeline -p vox-cli -p vox-cli-ci` + `timeout 60s cargo run -p vox-cli -- audit docs --format text`.
- **Before completion:** `vox ci ssot-drift`, `vox ci docs-reality-audit verify`, and docs-astro link test green.
- **Max feedback latency:** ~30 s.

---

## Requirement → Verification Map

| Requirement | Behavior | Test type | Automated command | Exists |
|---|---|---|---|---|
| **MEASURE-01** | Deterministic mention extractor finds paths, crates, symbols, CLI commands, env vars, config keys; respects `vox:skip`/`text`; distinguishes mentions vs prescribes | Unit + integration | `cargo test -p vox-doc-pipeline claims::extractor` | Plan 21-01 |
| **MEASURE-02** | Extracted mentions resolved against filesystem, `command-registry.yaml`, `env-vars.v1.yaml`, `retired-symbols.v1.yaml` into Resolved/Missing/Retired/Ambiguous | Unit + integration | `cargo test -p vox-doc-pipeline claims::resolver` | Plan 21-01 |
| **MEASURE-03** | Report-only CLI `vox audit docs` emits JSONL with `schema_version: 1`; false-positive rate measured on hand-labelled sample | CLI integration + eval unit | `cargo test -p vox-doc-pipeline claims::eval && cargo run -p vox-cli -- audit docs --format jsonl` | Plan 21-02 |
| **MEASURE-04** | Docs Reality Audit schema bump (`source`, `fingerprint`, `detector`), fingerprint deduplication, metrics distinguishing unpopulated vs healthy | Schema validation + CI test | `cargo test -p vox-cli-ci docs_reality_audit && cargo run -p vox-cli -- ci docs-reality-audit verify` | Plan 21-02 |
| **MEASURE-05** | Prune report lists every page with keep/merge/move-to-Internals/archive, dead mentions, inbound link count, last edit, status, section, clusters | Engine integration | `cargo run -p vox-cli -- audit docs --prune-report --prune-out target/prune-report.md` | Plan 21-03 |
| **MEASURE-06** | Research & roadmap pages moved to Internals, approved archives applied, 301 redirects in `_redirects`, zero 404 links | Site build & link audit | `pnpm --dir docs-astro test && pnpm --dir docs-astro build` | Plan 21-04 |
