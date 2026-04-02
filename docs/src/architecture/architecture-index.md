---
title: "Architecture index"
description: "Guide to the current architecture, SSOT, research, and roadmap documentation under docs/src/architecture."
category: "architecture"
status: "current"
sort_order: 0
last_updated: 2026-04-02
training_eligible: true
---

# Architecture index

The `docs/src/architecture/` section contains several different kinds of documents. This page is the map.

## Current architecture and SSOT

Use these when you need the current shape of the system or the active policy:

- [Feature growth boundaries](feature-growth-boundaries.md)
- [Interop tier policy](interop-tier-policy.md)
- [MCP exposure from the Vox language (SSOT)](mcp-vox-language-exposure.md)
- [Capability registry SSOT](capability-registry-ssot.md) — `contracts/capability`, `vox ci capability-sync`, model manifest
- [Vox bell-curve strategy](vox-bell-curve-strategy.md)
- [Doc-to-code acceptance checklist](doc-to-code-acceptance-checklist.md)
- [Orphan surface inventory](orphan-surface-inventory.md)
- [Language surface SSOT](language-surface-ssot.md) — keywords / decorators / manifests
- [OpenAPI contract SSOT](openapi-contract-ssot.md) — committed YAML, validation, optional codegen
- [Outbound HTTP policy](outbound-http-policy.md) — `vox-reqwest-defaults` and migration order
- [Compiler diagnostics ergonomics](compiler-diagnostics-ergonomics.md) — `miette` vs custom errors, `quote` pilot
- [Vox shell operations boundaries](vox-shell-operations-boundaries.md) — host `pwsh` vs `vox shell` vs `.vox` `std.*` (no shell emulator product)
- [Plan adequacy (thin plans & telemetry)](plan-adequacy.md) — external limits, shared heuristics, expansion policy
- [Telemetry trust boundary and SSOT map](telemetry-trust-ssot.md) — telemetry surfaces, SSOT primaries, trust planes
- [Telemetry taxonomy and contracts SSOT](telemetry-taxonomy-contracts-ssot.md) — roadmap event taxonomy and contracts
- [Telemetry retention and sensitivity SSOT](telemetry-retention-sensitivity-ssot.md) — roadmap retention and S0–S3 classes
- [Telemetry client disclosure SSOT](telemetry-client-disclosure-ssot.md) — VS Code / MCP host disclosure
- [Telemetry implementation blueprint 2026](telemetry-implementation-blueprint-2026.md) — phased rollout plan
- [Telemetry implementation backlog 2026](telemetry-implementation-backlog-2026.md) — executable checklist
- [Telemetry remote sink specification](telemetry-remote-sink-spec.md) — optional `vox telemetry upload` wire contract

## Research and synthesis

Use these when the question is exploratory, comparative, or evidence-gathering:

- [Research index](research-index.md)
- [AI IDE feature research findings 2026](ai-ide-feature-research-findings-2026.md)
- [Terminal execution policy research findings 2026](terminal-exec-policy-research-findings-2026.md)
- [Telemetry unification research findings 2026](telemetry-unification-research-findings-2026.md)
- [Context management research findings 2026](context-management-research-findings-2026.md)
- [Protocol convergence research 2026](protocol-convergence-research-2026.md)
- `*-research-2026.md`
- `*-findings-2026.md`
- synthesis pages that are explicitly labeled as research

## Planning and roadmap

Use these when a page describes intended implementation rather than current behavior:

- [Context management implementation blueprint](context-management-implementation-blueprint.md)
- [Context management phase 1 backlog](context-management-phase1-backlog.md)
- `*-implementation-plan-2026.md`
- [Populi GPU mesh implementation plan 2026](populi-gpu-mesh-implementation-plan-2026.md)
- [Populi GPU truth probe specification (NVML Layer A)](populi-gpu-truth-probe-spec.md) — optional `nvml-wrapper` build path for `NodeRecord` inventory
- [Populi node lifecycle, drain, and GPU hotplug](populi-node-lifecycle-hotplug.md) — lifecycle model and backlog vs shipped gates
- Normative **decision** docs for Populi GPU / mesh placement: [ADR 017: Populi lease-based remote execution](../adr/017-populi-lease-remote-execution.md), [ADR 018: Populi GPU truth layering](../adr/018-populi-gpu-truth-layering.md), [ADR 020: Populi mesh scaling — default transport posture](../adr/020-populi-mesh-scaling-transport-default.md), [work-type placement matrix](../reference/populi-work-type-placement-matrix.md) — aspirational batch/K8s notes remain in [Populi GPU mesh implementation plan 2026](populi-gpu-mesh-implementation-plan-2026.md) until dedicated ADRs are filed
- [ADR 022: Orchestrator bootstrap factory and daemon boundaries](../adr/022-orchestrator-bootstrap-and-daemon-boundaries.md) — shared `build_repo_scoped_orchestrator`, MCP/CLI identity parity, `vox-dei-d` boundary
- `*-implementation-blueprint.md`
- `*-roadmap.md`
- planning-meta documents under `planning-meta/`

## How to read this section

- If you need shipped behavior, prefer pages labeled `status: current` or pages that mirror code and contract surfaces.
- If you need rationale, open the matching ADR or architecture SSOT page.
- If you need future direction, read roadmap and planning documents as plans, not as claims of current capability.
