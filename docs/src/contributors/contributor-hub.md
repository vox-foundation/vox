---
title: "Contributor hub"
description: "Start here for contributor-facing Vox documentation, governance, inventories, and review checklists."
category: "contributor"
status: "current"
sort_order: 0
last_updated: "2026-04-12"
training_eligible: true

schema_type: "TechArticle"
---

# Contributor hub

This page is the reader-facing entry point for contributor documentation.

If you are evaluating Vox as a language or product, start with the [project README](../../../README.md), the [FAQ](../explanation/faq.md), and the [tutorials](../tutorials/tut-getting-started.md). If you are changing this repository, start here.

## Start here

- [AGENTS.md](../../../AGENTS.md) - required contributor and agent policy entry point, with Clavis as the secret-management SSOT.
- [Agent instruction architecture](agent-instruction-architecture.md) - instruction layering model (`AGENTS.md`, tool overlays, continuation prompts, CI gates).
- [Coding Agent Instructions](coding-agents.md) - heuristics and rules for agents, including god object constraints and stale docs guidelines.
- [Documentation governance](documentation-governance.md) - where docs live, which surface owns what, status vocabulary, and review cadence.
- [CI runner contract](../ci/runner-contract.md) - canonical `vox ci` guidance, runner labels, and line-ending policy.
- [Doc inventory verifier](../reference/doc-inventory.md) - machine-readable doc inventory workflow and drift expectations.
- [Architectural governance (TOESTUB)](../../agents/governance.md) - repository governance, organization rules, and quality policy.
- [`docs/agents/`](../../agents/) — full agent-facing support docs (orchestrator behavior, handoff protocol, editor contracts, time awareness).

## I want to…

Quick routing by goal. All links resolve from this directory.

| Goal | Start here |
|---|---|
| Fix a TOESTUB / stub-check CI failure | [TOESTUB contributor guide](toestub-contributor-guide.md) |
| Fix a `god_object` or `sprawl` violation | [God object defactor checklist](../archive/research-2026-q1/god-object-defactor-checklist.md) |
| Understand why my code affects model quality | [Contribution loop](contribution-loop.md) |
| Add a golden `.vox` example | [Examples corpus how-to](../how-to/examples-corpus.md) |
| Write or update documentation | [Documentation governance](documentation-governance.md) + [Doc-to-code checklist](../archive/research-2026-q1/doc-to-code-acceptance-checklist.md) |
| Contribute to the compiler / parser | [How-To: parser and HIR](../how-to/how-to-contribute-parser-hir.md) |
| Contribute to MENS training pipeline | [How-To: Mens native training](../how-to/how-to-contribute-mens.md) |
| Add a CLI command | [CLI design rules SSOT](../archive/research-2026-q1/cli-design-rules-ssot.md) |
| Work with secrets or credentials | [Clavis SSOT](../reference/clavis-ssot.md) |
| Understand the agentic quality model | [Coding agent instructions](coding-agents.md) + [Governance (TOESTUB)](../../agents/governance.md) |
| Read architecture or research context | [Architecture index](../architecture/architecture-index.md) → contributor-relevant section |

## Contributor map

Use these surfaces intentionally:

| Need | Start with |
| --- | --- |
| Cursor IDE rules and per-glob patterns | [`.cursor/rules/`](../../../.cursor/rules/) |
| Secrets, credentials, env parity | [AGENTS.md](../../../AGENTS.md), [Clavis SSOT](../reference/clavis-ssot.md) |
| Agent behavior consistency across long sessions and IDEs | [Agent instruction architecture](agent-instruction-architecture.md), [Continuation prompt engineering](continuation-prompt-engineering.md) |
| Antigravity-specific overrides | [GEMINI.md](../../../GEMINI.md), [Agent instruction architecture](agent-instruction-architecture.md) |
| Terminal shell discipline, exec-policy, `vox shell check` | [AGENTS.md](../../../AGENTS.md), [CLI reference](../reference/cli.md) (`vox shell`), [Terminal AST validation research 2026](../archive/research-2026-q1/terminal-ast-validation-research-2026.md), [`contracts/terminal/exec-policy.v1.yaml`](../../../contracts/terminal/exec-policy.v1.yaml) |
| CLI or command-surface changes | [CLI reference](../reference/cli.md), [CLI design rules SSOT](../archive/research-2026-q1/cli-design-rules-ssot.md), [Capability registry SSOT](../archive/research-2026-q1/capability-registry-ssot.md), [Command compliance](../reference/command-compliance.md) |
| Documentation updates or new docs | [Documentation governance](documentation-governance.md), [Doc-to-code acceptance checklist](../archive/research-2026-q1/doc-to-code-acceptance-checklist.md) |
| Telemetry, metrics, privacy boundaries | [Telemetry trust SSOT](../architecture/telemetry-trust-ssot.md), [research findings 2026](../archive/research-2026-q1/telemetry-unification-research-findings-2026.md), [implementation blueprint 2026](../archive/research-2026-q1/telemetry-implementation-blueprint-2026.md), [implementation backlog 2026](../archive/research-2026-q1/telemetry-implementation-backlog-2026.md) |
| Architecture or roadmap context | [Architecture index](../architecture/architecture-index.md), [Research index](../architecture/research-index.md) |
| Contracts and schema-backed behavior | [contracts/README.md](../../../contracts/README.md), related reference pages under `docs/src/reference/` |
| MCP, HTTP, Populi mesh, SSE, WebSockets | [Communication protocols](../reference/communication-protocols.md), [protocol catalog](../../../contracts/communication/protocol-catalog.yaml); research [Protocol convergence research 2026](../archive/research-2026-q1/protocol-convergence-research-2026.md) |
| CI, workflow, or policy guardrails | [CI runner contract](../ci/runner-contract.md), [Pre-push local CI parity](#pre-push-local-ci-parity) (below), [Architectural governance (TOESTUB)](../../agents/governance.md) |
| VS Code / Cursor extension, MCP tool calls from the editor, Oratio speech UX | [`vox-vscode/README.md`](../../../vox-vscode/README.md), [VS Code ↔ MCP compatibility](../reference/vscode-mcp-compat.md), [Speech capture architecture](../reference/speech-capture-architecture.md) |

Fast local policy rerun for this lane:

- `vox ci policy-smoke` runs `cargo check -p vox-orchestrator`, then command-compliance and the same rust ecosystem parity test used by `vox ci rust-ecosystem-policy` in one command.

## Pre-push: local CI parity

CI on `main` / PRs is defined in [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml). The job does **not** rely on a lone `cargo check -p vox-cli`; it runs **`cargo clippy --workspace --all-targets`**, **`cargo doc --workspace --no-deps`** (with warnings denied), **`cargo llvm-cov nextest --workspace`**, and many **`vox ci *`** guards. Before pushing, run a **high-signal subset** so failures match CI instead of showing up only on the runner.

**Suggested commands** (from repo root; use full `cargo` path on Windows agents if `PATH` is minimal — see [AGENTS.md](../../../AGENTS.md)):

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p vox-cli --quiet -- ci ssot-drift
```

Then run **tests for crates you changed** (faster than a full workspace test pass):

```bash
cargo test -p vox-db --test schema_contract_tests   # example; pick your crates
```

**TOESTUB** on changed directories (requires the `stub-check` feature on `vox-cli`):

```bash
cargo run -p vox-cli --features stub-check --quiet -- stub-check crates/vox-mcp
```

Use a **single** positional path per invocation (repeat for each directory). See [Architectural governance (TOESTUB)](../../agents/governance.md).

**`vox_db::legacy_schema` warnings during `stub-check`:** if stderr mentions `schema_version chain is not the current baseline`, the harness opened the **canonical Codex store** resolved from your environment (usually the platform default `vox.db` when `VOX_DB_PATH` is unset). Fix by either completing **Stage 1** in the [VoxDB cutover runbook](../operations/voxdb-cutover-runbook.md) for that file, or — when you do not need to keep data — point **`VOX_DB_PATH`** at a **fresh scratch `.db`** per the runbook section *Contributors / local tooling — fresh canonical DB* (`connect_default` does not use `:memory:` when env is empty). Do not lower `BASELINE_VERSION` to silence the log.

**Codex + docs SSOT:** `vox ci check-codex-ssot` and `vox ci check-docs-ssot` are **merge-blocking** in CI (see [.github/workflows/ci.yml](../../../.github/workflows/ci.yml)). Run `check-codex-ssot` locally after changing [`contracts/db/baseline-version-policy.yaml`](../../../contracts/db/baseline-version-policy.yaml) or [`crates/vox-db/src/schema/manifest.rs`](../../../crates/vox-db/src/schema/manifest.rs). Run `check-docs-ssot` when you change doc inventories, canonical maps, or migration-facing docs.

## Contributor expectations

- Prefer updating the canonical surface instead of copying prose into a second location.
- When code changes alter public behavior, update the corresponding docs in the same PR.
- Treat `contracts/` as machine SSOT, `docs/src/reference/` as human lookup, `docs/src/architecture/` as design and rationale, and `docs/agents/` as contributor and automation support.
- Use `vox ci` guards where they exist instead of replacing them with one-off shell checks.

