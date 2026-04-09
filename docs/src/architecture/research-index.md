---
title: "Research index"
description: "Guide to the research, findings, and roadmap-heavy documentation in the architecture section."
category: "architecture"
status: "research"
sort_order: 5
last_updated: 2026-04-09
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

### Deep Research Clusters (April 2026)

- **[Research Synthesis: Grand Strategy Seed 2026](research-synthesis-grand-strategy-seed-2026.md)** — the master framework connecting these discoveries.

#### LLM Hallucination & Type System Impact (Wave 1)
- **[LLM-Native Language Design](research-llm-native-lang-design-2026.md)** — cluster overview with Vox implications
- [Cognitive Science of LLM Hallucinations](research-ts-hallucination-cognitive-science-2026.md)
- [Empirical Evidence for Type Systems](research-ts-hallucination-empirical-evidence-2026.md)
- [Frontier Model Challenges](research-ts-hallucination-frontier-2026.md)
- [K-Complexity Reduction Strategies](research-ts-hallucination-k-complexity-2026.md)
- [Zero-Shot Invariants Validation](research-ts-hallucination-zero-shot-invariants-2026.md)
- [Works Cited: Hallucination & Type Systems](research-ts-hallucination-works-cited-2026.md)

#### Continual Learning & Flywheel Risks (Wave 2)
- **[Continual Learning Flywheel Risks](research-continual-learning-flywheel-2026.md)** — cluster overview with risk taxonomy
- [MAD and Mode Collapse](research-cl-mad-mode-collapse-2026.md)
- [The Compile-Pass Oracle and Semantic Drift](research-cl-oracle-semantic-drift-2026.md)
- [Catastrophic Forgetting in QLoRA](research-cl-qlora-catastrophic-forgetting-2026.md)
- [Schola / Scientia Typicality Bias & Slop](research-cl-slop-typicality-bias-2026.md)
- [Minimum Viable Corpus for QLoRA](research-cl-qlora-minimum-corpus-2026.md)
- [Negative Examples via DPO/NAT](research-cl-nat-dpo-2026.md)
- [Risk Taxonomy and Telemetry Mitigations](research-cl-risk-taxonomy-telemetry-2026.md)
- [Works Cited: Continual Learning Flywheel](research-cl-works-cited-2026.md)

#### GRPO Reward Shaping for Code LLMs (Wave 3)
- **[GRPO Reward Shaping for Code LLMs](research-grpo-reward-shaping-2026.md)** — cluster overview with architectural adjustments
- [Efficacy of Binary Parse-Rate Signalling](research-grpo-binary-parse-rate-2026.md)
- [GRPO VRAM Efficiency and Small-Batch Dynamics](research-grpo-vram-small-batch-2026.md)
- [AST Coverage Scoring and Reward Hacking](research-grpo-ast-reward-hacking-2026.md)
- [Empirical Justification for Reward Weights](research-grpo-reward-weights-2026.md)
- [Optimization Landscape of Positive-Only Loops](research-grpo-positive-only-optimization-2026.md)
- [Gap Analysis and Adjustments](research-grpo-gaps-and-adjustments-2026.md)
- [Works Cited: GRPO Reward Shaping](research-grpo-works-cited-2026.md)

#### AI Agent Context and Handoff Continuity (Wave 4)
- [Empirical Evidence for Context Compaction](research-agent-handoff-empirical-compaction-2026.md)
- [Context Bleed and Identity Confusion](research-agent-handoff-context-bleed-2026.md)
- [SOTA Context-Aware Protocols](research-agent-handoff-sota-protocols-2026.md)
- [Context Retrieval Policies](research-agent-handoff-retrieval-policies-2026.md)
- [A2A Protocol Evidence Sharing](research-agent-handoff-a2a-evidence-sharing-2026.md)
- [Context Truncation Failure Modes](research-agent-handoff-truncation-failure-2026.md)
- [Production Failure Catalog](research-agent-handoff-failure-catalog-2026.md)
- [Design Pattern Recommendations](research-agent-handoff-design-patterns-2026.md)
- [Implementation Checklist](research-agent-handoff-checklist-2026.md)
- [Works Cited: Agent Handoff Continuity](research-agent-handoff-works-cited-2026.md)

#### Independent Deep Research Tracks
- [Agent Trust Reliability Evaluation](research-trust-reliability-signals-2026.md)
- [AI Plan Adequacy Heuristics](research-plan-adequacy-heuristics-2026.md)
- [Compiler Testing Research](research-pbt-oracles-compiled-lang-2026.md)
- [Multi-Agent Mesh Economics](research-multi-agent-mesh-economics-2026.md)
- [Grammar-Constrained Decoding for Code LLMs](research-grammar-constrained-decoding-2026.md)

### Documentation

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
- [Mobile/Desktop Convergence & Language Extension Research 2026](mobile-desktop-convergence-research-2026.md) — unified browser view, std.mobile namespace, agent/environment parser gaps, Web API vs Capacitor strategy, maintainability quantification
- [Vox bell-curve strategy](vox-bell-curve-strategy.md)
- [Feature growth boundaries](feature-growth-boundaries.md)
- [Interop tier policy](interop-tier-policy.md)

### Agentic planning and orchestration

- [Claude Code Ultraplan research 2026](claude-code-ultraplan-research-2026.md) — architecture deep-dive, cost model, failure modes, and actionable Vox recommendations
- [Dynamic agentic planning 2026](res_dynamic_agentic_planning_2026.md) — earlier research seed for planning-mode architecture
- [Orchestrator multi-agent groundwork 2026](orchestrator-multi-agent-groundwork-2026.md)
- [Context management research findings 2026](context-management-research-findings-2026.md)
- [Context management implementation blueprint](context-management-implementation-blueprint.md)
- [Vox agentic loop and MENS plan](vox_agentic_loop_and_mens_plan.md)

### SCIENTIA novelty / publication ledger (contracts)

- Finding-candidate and novelty-evidence v1 JSON Schemas live under `contracts/scientia/` (`finding-candidate.v1.schema.json`, `novelty-evidence-bundle.v1.schema.json`); example fixtures under `contracts/reports/scientia-*.example.v1.json`. CI: `vox ci scientia-novelty-ledger-contracts` (also nested in `vox ci ssot-drift`). CLI spot-check: `vox scientia finding-candidate-validate`, `vox scientia novelty-evidence-bundle-validate`.
- **Impact / readership / citation-adjacent signals (research seed):** [scientia-impact-readership-research-2026.md](scientia-impact-readership-research-2026.md) and tunable weights in [`contracts/scientia/impact-readership-projection.seed.v1.yaml`](../../../contracts/scientia/impact-readership-projection.seed.v1.yaml) (orthogonal to novelty; no default publish gate).
- **Publication-worthiness + SSOT unification research plan:** [scientia-publication-worthiness-ssot-unification-research-2026.md](scientia-publication-worthiness-ssot-unification-research-2026.md) (standards-to-signals matrix, canonical metadata graph proposal, detection calibration protocol, Codex research snapshot persistence blueprint, automation boundary ledger).
- **Implementation wave playbook (roadmap):** [scientia-implementation-wave-playbook-2026.md](scientia-implementation-wave-playbook-2026.md) (232-task execution map, wave outputs, first-30 lock order, and contract inventory).

## Labeling rule

If a page is primarily research or a roadmap, say so in the title, frontmatter, or first paragraph. Do not rely on filenames alone.
