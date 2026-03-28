---
title: "Doc-to-code acceptance checklist"
description: "Official documentation for Doc-to-code acceptance checklist for the Vox language. Detailed technical reference, architecture guides, and "
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Doc-to-code acceptance checklist

Use this before merging changes that affect user-visible behavior or agent guidance.

- [ ] `docs/src/reference/cli.md` matches `crates/vox-cli/src/lib.rs` `Cli` subcommands (dispatch lives there; `main.rs` only calls `run_vox_cli`).
- [ ] `AGENTS.md` Phase / crate bullets match workspace reality (`Cargo.toml` members / excludes).
- [ ] [orphan-surface-inventory.md](orphan-surface-inventory.md) updated if a crate or CLI surface changed.
- [ ] ADR 004 cross-links still valid if Codex/Turso boundaries changed.
- [ ] [Codex / Arca compatibility boundaries](codex-arca-compatibility-boundaries.md) updated if `DbConfig`, env vars, or migration rules changed.
- [ ] WebIR planning claims are synchronized across [ADR 012](../adr/012-internal-web-ir-strategy.md), [implementation blueprint](internal-web-ir-implementation-blueprint.md), and planning-meta Tier 1 docs (`01`, `05`, `08`, `10`) when gate language or ownership policy changes.
- [ ] “Current production path” statements in [Compiler Architecture](../explanation/expl-architecture.md) and [Compiler Lowering Phases](../explanation/expl-compiler-lowering.md) remain consistent with compiler code-path behavior (`codegen_ts/emitter.rs`, `codegen_ts/reactive.rs`) when docs are updated.
- [ ] **`cargo run -p vox-cli -- ci check-codex-ssot`** passes (or shim `scripts/check_codex_ssot.sh`).
- [ ] **`cargo run -p vox-cli -- ci check-docs-ssot`** passes (or shim `scripts/check_docs_ssot.sh`).
