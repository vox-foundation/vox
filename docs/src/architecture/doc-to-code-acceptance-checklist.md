---
title: "Doc-to-code acceptance checklist"
category: architecture
last_updated: 2026-03-20
---

# Doc-to-code acceptance checklist

Use this before merging changes that affect user-visible behavior or agent guidance.

- [ ] `docs/src/ref-cli.md` matches `crates/vox-cli/src/lib.rs` `Cli` subcommands (dispatch lives there; `main.rs` only calls `run_vox_cli`).
- [ ] `AGENTS.md` Phase / crate bullets match workspace reality (`Cargo.toml` members / excludes).
- [ ] [orphan-surface-inventory.md](orphan-surface-inventory.md) updated if a crate or CLI surface changed.
- [ ] ADR 004 cross-links still valid if Codex/Turso boundaries changed.
- [ ] [Codex / Arca compatibility boundaries](codex-arca-compatibility-boundaries.md) updated if `DbConfig`, env vars, or migration rules changed.
- [ ] **`cargo run -p vox-cli -- ci check-codex-ssot`** passes (or shim `scripts/check_codex_ssot.sh`).
- [ ] **`cargo run -p vox-cli -- ci check-docs-ssot`** passes (or shim `scripts/check_docs_ssot.sh`).
