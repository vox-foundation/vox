---
title: "Research index"
description: "Guide to the research, findings, and roadmap-heavy documentation in the architecture section."
category: "architecture"
status: "research"
sort_order: 5
last_updated: 2026-04-06
training_eligible: true
---

# Research index

This page groups the research-oriented documentation in `docs/src/architecture/` so it is easier to discover without mistaking it for the current shipped architecture.

## Research classes

| Pattern | Typical status | Meaning |
| --- | --- | --- |
| `*-research-2026.md` | `research` | investigation, evidence gathering, constraints, and trade-offs |
| `*-findings-2026.md` | `research` | synthesized results or conclusions from a research wave |
| `*-implementation-plan-2026.md` | `roadmap` | ordered implementation proposal |
| `*-implementation-blueprint.md` | `roadmap` or `experimental` | intended technical design for a future or in-progress path |
| `planning-meta/*` | `current` process docs or `roadmap` planning docs | contributor planning governance, not public product narrative |

## Suggested reading paths

### Documentation and organization

- [Orphan surface inventory](orphan-surface-inventory.md)
- [Architecture index](architecture-index.md)
- planning-meta documents when you need contributor process detail

### Packaging and portability

- [Vox Docker-backed portability research 2026](vox-docker-dotvox-portability-research-2026.md)
- [Vox Docker-backed portability implementation plan 2026](vox-docker-dotvox-portability-implementation-plan-2026.md)
- [Vox packaging research findings 2026](vox-packaging-research-findings-2026.md)
- [Vox packaging implementation blueprint](vox-packaging-implementation-blueprint.md)

### Language and architecture direction

- [AI IDE feature research findings 2026](ai-ide-feature-research-findings-2026.md)
- [Prompt engineering, system prompts, document-skills, and SCIENTIA (research 2026)](prompt-engineering-document-skills-scientia-research-2026.md)
- [Terminal execution policy research findings 2026](terminal-exec-policy-research-findings-2026.md) — PowerShell-first shells, IDE allow/deny limits, future unified contract
- [Telemetry unification research findings 2026](telemetry-unification-research-findings-2026.md)
- [Telemetry implementation blueprint 2026](telemetry-implementation-blueprint-2026.md) — roadmap implementation plan
- [Telemetry implementation backlog 2026](telemetry-implementation-backlog-2026.md) — executable checklist
- [Protocol convergence research 2026](protocol-convergence-research-2026.md)
- [Populi GPU network research 2026](populi-gpu-network-research-2026.md)
- [Populi GPU mesh implementation plan 2026](populi-gpu-mesh-implementation-plan-2026.md) — paired **decision** docs: [ADR 017](../adr/017-populi-lease-remote-execution.md), [ADR 018](../adr/018-populi-gpu-truth-layering.md), [ADR 020](../adr/020-populi-mesh-scaling-transport-default.md), [placement matrix](../reference/populi-work-type-placement-matrix.md); probe SSOT: [GPU truth probe spec](populi-gpu-truth-probe-spec.md), [node lifecycle / hotplug](populi-node-lifecycle-hotplug.md)
- [Vox bell-curve strategy](vox-bell-curve-strategy.md)
- [Feature growth boundaries](feature-growth-boundaries.md)
- [Interop tier policy](interop-tier-policy.md)

### SCIENTIA novelty / publication ledger (contracts)

- Finding-candidate and novelty-evidence v1 JSON Schemas live under `contracts/scientia/` (`finding-candidate.v1.schema.json`, `novelty-evidence-bundle.v1.schema.json`); example fixtures under `contracts/reports/scientia-*.example.v1.json`. CI: `vox ci scientia-novelty-ledger-contracts` (also nested in `vox ci ssot-drift`). CLI spot-check: `vox scientia finding-candidate-validate`, `vox scientia novelty-evidence-bundle-validate`.
- **Impact / readership / citation-adjacent signals (research seed):** [scientia-impact-readership-research-2026.md](scientia-impact-readership-research-2026.md) and tunable weights in [`contracts/scientia/impact-readership-projection.seed.v1.yaml`](../../../contracts/scientia/impact-readership-projection.seed.v1.yaml) (orthogonal to novelty; no default publish gate).
- **Publication-worthiness + SSOT unification research plan:** [scientia-publication-worthiness-ssot-unification-research-2026.md](scientia-publication-worthiness-ssot-unification-research-2026.md) (standards-to-signals matrix, canonical metadata graph proposal, detection calibration protocol, Codex research snapshot persistence blueprint, automation boundary ledger).

## Labeling rule

If a page is primarily research or a roadmap, say so in the title, frontmatter, or first paragraph. Do not rely on filenames alone.
