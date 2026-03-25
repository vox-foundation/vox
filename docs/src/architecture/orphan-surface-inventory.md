---
title: "Orphan surface inventory"
description: "Official documentation for Orphan surface inventory for the Vox language. Detailed technical reference, architecture guides, and implemen"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Orphan surface inventory

Classification for code and docs that do not match the **minimal shipped `vox` CLI** or workspace membership. Goal: no ambiguous SSOT. See [forward migration charter](forward-migration-charter.md) (forward-only; no restore-based workflows).

## Policy buckets

| Bucket | Action |
|--------|--------|
| **keep** | Wired in default build; maintain |
| **port** | Needed for roadmap; rewire to `vox_db::VoxDb` / workspace members |
| **archive** | Historical value only; move to `docs/src/archive/` or mark “not built” in header |
| **delete** | Duplicate or superseded; remove when safe |

## Automation / CI SSOT

- Prefer **`vox ci …`** for registry-backed checks over one-off shell copies where a subcommand exists — [runner contract](../ci/runner-contract.md), [command compliance](../reference/command-compliance.md).
- **`VOX_*` / Turso env naming:** [Environment variables (SSOT)](../reference/env-vars.md).

## Inventory (surfaces)

| Surface | Location | Owner | Severity | Decision | Milestone | Validated | Evidence | Rationale |
|---------|----------|-------|----------|----------|-----------|-----------|----------|-----------|
| Minimal `vox` CLI | `crates/vox-cli/src/main.rs`, `commands/mod.rs` | Maintainers | low | keep | ongoing | 2026-03-20 | `ref-cli.md` | SSOT for shipped commands |
| Extended CLI subtree | `crates/vox-cli/src/commands/**` (beyond `commands/mod.rs`) | Maintainers | high | port | TBD | 2026-03-21 | `cli-scope-policy.md` | Unwired until explicitly added to minimal binary; `vox-ars` is a workspace member; `vox-cli` optional feature **`ars`** pulls the dep when OpenClaw/skill modules are reattached |
| Canonical `vox db` helpers | `crates/vox-cli/src/commands/db.rs`, `db_research_impl.rs` | Maintainers | medium | keep | ongoing | 2026-03-21 | `commands/db.rs` | `commands::ops` tree removed (unwired; duplicated `vox_dei`); DB helpers live under `commands::db` |
| `vox scientia` CLI facade | `crates/vox-cli/src/commands/scientia.rs` | Maintainers | low | keep | ongoing | 2026-03-21 | `ref-cli.md`, `orchestration-unified.md` | Research / capability-map aliases over `commands::db_cli` (same DB + `repository_id` resolution as `vox db`) |
| Unwired `vox_dei` CLI sources (removed) | _(deleted)_ `commands/chat/`, `commands/ops/`, `commands/quaero/`, `ai/{agent,dei,hud,learn}.rs` | Maintainers | low | delete | 2026-03-21 | `check_vox_cli_no_vox_dei.sh` | Daemon-only DeI: use `crate::dei_daemon` + external `vox-dei-d` |
| `vox-runtime` DB helper | `crates/vox-runtime/src/db.rs` | Maintainers | medium | port | ongoing | 2026-03-20 | feature `database` | Align with Codex env policy |
| `vox-mcp`, `vox-git` | workspace members | Maintainers | low | keep | ongoing | 2026-03-20 | `ci.yml` smoke | Core agent/tooling |
| Workspace excludes | root `Cargo.toml` `exclude` | Maintainers | medium | port | TBD | 2026-03-20 | `Cargo.toml` | Re-include when CI-stable |
| Plans under `.cursor/plans/` | various | Maintainers | low | archive | ongoing | 2026-03-20 | — | May reference removed crates; not SSOT |
| Docs: full ecosystem | `how-to-cli-ecosystem.md` | Maintainers | medium | keep | ongoing | 2026-03-20 | `ref-cli.md` | Narrative may exceed minimal CLI |

## Workspace crate index (CI guard)

`scripts/check_docs_ssot.sh` (or `scripts/check_docs_ssot.ps1` on Windows) requires every `crates/*/Cargo.toml` package name to appear **exactly once** between the markers below (one crate per line).

<!-- workspace-crates-start -->
vox-ast
vox-ars
vox-bootstrap
vox-capability-registry
vox-cli
vox-codegen-llvm
vox-codegen-rust
vox-codegen-ts
vox-codegen-wasm
vox-compiler
vox-codex
vox-codex-api
vox-config
vox-container
vox-corpus
vox-db
vox-dei
vox-doc-inventory
vox-doc-pipeline
vox-eval
vox-fmt
vox-forge
vox-gamify
vox-git
vox-hir
vox-integration-tests
vox-lexer
vox-lsp
vox-ludus
vox-mcp
vox-mcp-meta
vox-mcp-registry
vox-populi
vox-oratio
vox-orchestrator
vox-parser
vox-pm
vox-protocol
vox-publisher
vox-mens
vox-repository
vox-runtime
vox-skills
vox-socrates-policy
vox-ssg
vox-storage
vox-tensor
vox-test-harness
vox-toestub
vox-tools
vox-schola
vox-typeck
vox-webhook
vox-workflow-runtime
<!-- workspace-crates-end -->

## Review cadence

Re-run classification when adding a workspace member or a new `vox` subcommand.
