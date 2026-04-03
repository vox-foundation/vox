---
title: "Contributor hub"
description: "Start here for contributor-facing Vox documentation, governance, inventories, and review checklists."
category: "contributor"
status: "current"
sort_order: 0
last_updated: 2026-04-03
training_eligible: true
---

# Contributor hub

This page is the reader-facing entry point for contributor documentation.

If you are evaluating Vox as a language or product, start with the [site landing page](../index.md), the [FAQ](../explanation/faq.md), and the [tutorials](../tutorials/tut-getting-started.md). If you are changing this repository, start here.

## Start here

- [AGENTS.md](../../../AGENTS.md) - required contributor and agent policy entry point, with Clavis as the secret-management SSOT.
- [Agent instruction architecture](agent-instruction-architecture.md) - instruction layering model (`AGENTS.md`, tool overlays, continuation prompts, CI gates).
- [Documentation governance](documentation-governance.md) - where docs live, which surface owns what, status vocabulary, and review cadence.
- [CI runner contract](../ci/runner-contract.md) - canonical `vox ci` guidance, runner labels, and line-ending policy.
- [Doc inventory verifier](../reference/doc-inventory.md) - machine-readable doc inventory workflow and drift expectations.
- [Architectural governance (TOESTUB)](../../agents/governance.md) - repository governance, organization rules, and quality policy.

## Contributor map

Use these surfaces intentionally:

| Need | Start with |
| --- | --- |
| Secrets, credentials, env parity | [AGENTS.md](../../../AGENTS.md), [Clavis SSOT](../reference/clavis-ssot.md) |
| Agent behavior consistency across long sessions and IDEs | [Agent instruction architecture](agent-instruction-architecture.md), [Continuation prompt engineering](continuation-prompt-engineering.md) |
| Antigravity-specific overrides | [GEMINI.md](../../../GEMINI.md), [Agent instruction architecture](agent-instruction-architecture.md) |
| Terminal shell discipline, exec-policy, `vox shell check` | [AGENTS.md](../../../AGENTS.md), [CLI reference](../reference/cli.md) (`vox shell`), [Terminal AST validation research 2026](../architecture/terminal-ast-validation-research-2026.md), [`contracts/terminal/exec-policy.v1.yaml`](../../../contracts/terminal/exec-policy.v1.yaml) |
| CLI or command-surface changes | [CLI reference](../reference/cli.md), [CLI design rules SSOT](../architecture/cli-design-rules-ssot.md), [Capability registry SSOT](../architecture/capability-registry-ssot.md), [Command compliance](../reference/command-compliance.md) |
| Documentation updates or new docs | [Documentation governance](documentation-governance.md), [Doc-to-code acceptance checklist](../architecture/doc-to-code-acceptance-checklist.md) |
| Telemetry, metrics, privacy boundaries | [Telemetry trust SSOT](../architecture/telemetry-trust-ssot.md), [research findings 2026](../architecture/telemetry-unification-research-findings-2026.md), [implementation blueprint 2026](../architecture/telemetry-implementation-blueprint-2026.md), [implementation backlog 2026](../architecture/telemetry-implementation-backlog-2026.md) |
| Architecture or roadmap context | [Architecture index](../architecture/architecture-index.md), [Research index](../architecture/research-index.md) |
| Contracts and schema-backed behavior | [contracts/README.md](../../../contracts/README.md), related reference pages under `docs/src/reference/` |
| MCP, HTTP, Populi mesh, SSE, WebSockets | [Communication protocols](../reference/communication-protocols.md), [protocol catalog](../../../contracts/communication/protocol-catalog.yaml); research [Protocol convergence research 2026](../architecture/protocol-convergence-research-2026.md) |
| CI, workflow, or policy guardrails | [CI runner contract](../ci/runner-contract.md), [Architectural governance (TOESTUB)](../../agents/governance.md) |
| VS Code / Cursor extension, MCP tool calls from the editor, Oratio speech UX | [`vox-vscode/README.md`](../../../vox-vscode/README.md), [VS Code ↔ MCP compatibility](../reference/vscode-mcp-compat.md), [Speech capture architecture](../reference/speech-capture-architecture.md) |

Fast local policy rerun for this lane:

- `vox ci policy-smoke` runs `cargo check -p vox-orchestrator`, then command-compliance and the same rust ecosystem parity test used by `vox ci rust-ecosystem-policy` in one command.

## Contributor expectations

- Prefer updating the canonical surface instead of copying prose into a second location.
- When code changes alter public behavior, update the corresponding docs in the same PR.
- Treat `contracts/` as machine SSOT, `docs/src/reference/` as human lookup, `docs/src/architecture/` as design and rationale, and `docs/agents/` as contributor and automation support.
- Use `vox ci` guards where they exist instead of replacing them with one-off shell checks.
