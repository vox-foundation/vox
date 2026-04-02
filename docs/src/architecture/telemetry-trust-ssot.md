---
title: "Telemetry trust boundary and SSOT map"
description: "Single map of telemetry-related surfaces, trust boundaries, documentation authority, and corrections to earlier research-only plans."
category: "architecture"
status: "current"
last_updated: 2026-04-02
training_eligible: true
---

# Telemetry trust boundary and SSOT map

## Purpose

This page is the **normative documentation map** for telemetry, observability, and trust boundaries in Vox. It complements:

- strategic research: [Telemetry unification research findings 2026](telemetry-unification-research-findings-2026.md)
- metric row rules: [Telemetry and research_metrics contract](../reference/telemetry-metric-contract.md)
- implementation sequencing: [Telemetry implementation blueprint 2026](telemetry-implementation-blueprint-2026.md)
- executable checklist: [Telemetry implementation backlog 2026](telemetry-implementation-backlog-2026.md)
- optional remote upload (explicit CLI only): [ADR 023](../adr/023-optional-telemetry-remote-upload.md), [Telemetry remote sink specification](telemetry-remote-sink-spec.md)

## Critique of the original research-only plan (folded)

The first telemetry-trust **research** pass was correct to defer code and schema changes. For **implementation**, the following gaps must stay explicit:

1. **Environment variable SSOT drift:** `VOX_BENCHMARK_TELEMETRY` and `VOX_SYNTAX_K_TELEMETRY` are implemented in [`crates/vox-cli/src/benchmark_telemetry.rs`](../../../crates/vox-cli/src/benchmark_telemetry.rs) and must appear in [Environment variables (SSOT)](../reference/env-vars.md) alongside deeper docs in [orchestration-unified](../reference/orchestration-unified.md) and [mens-training](../reference/mens-training.md).
2. **Machine contracts beyond `research_metrics`:** [context-lifecycle-telemetry.schema.json](../../contracts/orchestration/context-lifecycle-telemetry.schema.json) is part of the telemetry vocabulary; it is not optional detail.
3. **`ci_completion_*` is workspace-adjacent:** Tables defined in [`crates/vox-db/src/schema/domains/ci_completion.rs`](../../../crates/vox-db/src/schema/domains/ci_completion.rs) carry paths and metadata. They are **not** interchangeable with coarse product telemetry without a separate sensitivity class (see [Telemetry retention and sensitivity SSOT](telemetry-retention-sensitivity-ssot.md)).
4. **VS Code and debug surfaces:** The extension webview uses a **`telemetry` tab id** for local dashboards; that naming can collide with user expectations about “phone-home” telemetry. [vscode-mcp-compat](../reference/vscode-mcp-compat.md) documents `vox.mcp.debugPayloads` — high sensitivity and must sit inside the same trust framework as Ludus MCP arg modes.
5. **Governance hooks:** New operations and drift checks must stay aligned with [operations catalog](../../contracts/operations/catalog.v1.yaml), [data-ssot-guards](../../../crates/vox-cli/src/commands/ci/run_body_helpers/data_ssot_guards.rs), and [CHANGELOG](../../CHANGELOG.md).

## Authoritative SSOT set (no duplicate primaries)

| Concern | Primary SSOT | Secondary / derivative |
| -------- | -------------- | ------------------------- |
| `research_metrics` row shape, session prefixes, validation | [telemetry-metric-contract](../reference/telemetry-metric-contract.md), [`research_metrics_contract.rs`](../../../crates/vox-db/src/research_metrics_contract.rs) | Crate doc comments |
| Env names and roles | [env-vars](../reference/env-vars.md) | orchestration-unified, mens-training, populi SSOT |
| Table TTL hints for prune | [retention-policy.yaml](../../contracts/db/retention-policy.yaml) | [db retention CLI](../../../crates/vox-cli/src/commands/db_retention.rs) |
| Completion CI telemetry schemas | `contracts/telemetry/completion-*.v1.schema.json` | [completion-policy-ssot](completion-policy-ssot.md) |
| Context lifecycle tracing fields | [context-lifecycle-telemetry.schema.json](../../contracts/orchestration/context-lifecycle-telemetry.schema.json) | [`context_lifecycle.rs`](../../../crates/vox-orchestrator/src/context_lifecycle.rs) |
| Taxonomy and event families (rollout) | [telemetry-taxonomy-contracts-ssot](telemetry-taxonomy-contracts-ssot.md) | contracts under `contracts/telemetry/` |
| Client disclosure and debug | [telemetry-client-disclosure-ssot](telemetry-client-disclosure-ssot.md) | vox-vscode README |
| Secrets for any future upload endpoint | [AGENTS.md](../../../AGENTS.md), Clavis | — |

## Trust planes (normative vocabulary)

Use these terms consistently in docs and code comments:

| Plane | Meaning | Default posture |
| ------- | --------- | ----------------- |
| **UsageTelemetry** | Coarse, low-entropy signals for product improvement | Local-first; remote only with explicit opt-in (future) |
| **Diagnostics** | Support bundles, debug logs, user-reviewed export | Explicit action; never default remote |
| **ContentPersistence** | Chat, tool args, retrieval, transcripts | Local / operator store; **not** “telemetry” without separate consent story |
| **OperationalTracing** | Structured logs and local JSONL | Local; treat as sensitive if identifiers or content leak |

## Contributor rule

Any change that adds or widens data collection, persistence, or export must update:

1. the relevant contract or SSOT doc,
2. [CHANGELOG](../../CHANGELOG.md),
3. retention or sensitivity SSOT if TTL or class changes,
4. operations catalog / CLI registry if a new operator-facing command or flag is introduced.

See [doc-to-code acceptance checklist](doc-to-code-acceptance-checklist.md).

## Related

- [Telemetry retention and sensitivity SSOT](telemetry-retention-sensitivity-ssot.md)
- [Telemetry taxonomy and contracts SSOT](telemetry-taxonomy-contracts-ssot.md)
- [Telemetry client disclosure SSOT](telemetry-client-disclosure-ssot.md)
