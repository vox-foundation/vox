---
title: "Documentation Corpus Prune Report"
description: "Evidence-based prune dispositions for all pages in the Vox documentation corpus."
category: "Architecture SSOTs"
status: "current"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-assisted"
authorship_model: "claude-3-5-sonnet"
---

# Documentation Corpus Prune Report

<!-- kb:summary -->
Audits the complete Vox documentation corpus across 971 pages, classifying each into merge, move to internals, archive, or keep dispositions. Evaluates pages against dead symbol mentions, inbound references, age, and semantic cluster membership to streamline the documentation tree and eliminate duplication.
<!-- /kb:summary -->

- **Total Pages Scanned**: 971
- **merge**: 29
- **movetointernals**: 183
- **archive**: 414
- **keep**: 345
- **Topic Clusters**: 43

## Page Dispositions

| Page | Category | Status | Disposition | Dead Mentions | Inbound Links | Age (days) | Rationale |
|---|---|---|---|---|---|---|---|
| `docs/src/ref/env-vars-ssot.md` | Language Reference | current | **Merge** | 0 | 0 | 81 | Member of topic cluster cluster-001 with overlapping content |
| `docs/src/ref/env-vars.md` | Language Reference | current | **Merge** | 1 | 0 | 81 | Member of topic cluster cluster-001 with overlapping content |
| `docs/src/adr/046-pareto-frontier-reporting.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 3 | 1 | 39 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/017-populi-lease-remote-execution.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 3 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/004-codex-arca-turso-ssot.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 7 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/003-native-training-over-python.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 6 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/019-durable-workflow-journal-contract-v1.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 4 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/029-formal-intent.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 1 | 1 | 14 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/021-generated-workflow-durability-parity.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 3 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/025-multi-agent-lock-coherence.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 14 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/012-internal-web-ir-strategy.md` | Architecture Decisions (ADRs) | deprecated | **MoveToInternals** | 7 | 5 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/036-webir-hir-unification-compare-both.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/048-interpreter-is-the-execution-and-sandbox-tier.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 4 | 2 | 34 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/internal-web-ir-side-by-side-schema.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 142 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/044-ai-fixture-subagent-decorator.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 51 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/035-swc-parser-alternatives-eval.md` | Architecture Decisions (ADRs) | research | **MoveToInternals** | 0 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/adr/041-durable-functions-completion-2026.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 6 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/034-candle-qlora-stack-upgrades.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 1 | 1 | 12 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/011-scientia-publication-ssot.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/038-ai-fixture-prompt-decorator.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/README.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 0 | 34 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/033-typed-fragment-primitive.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 7 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/016-oratio-streaming-whisper-and-constrained-decode.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 1 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/015-vox-docker-oci-portability-ssot.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 15 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/009-populi-hosted-baas.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/037-tauri-convergence.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 6 | 1 | 20 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/030-state-machine-ssot.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 7 | 3 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/039-ai-fixture-hole-decorator.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/027-dual-track-ui-surfaces.md` | Architecture Decisions (ADRs) | deprecated | **MoveToInternals** | 6 | 3 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/index.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 34 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/026-third-party-code-provenance.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 5 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/032-vox-ui-reactive-modules.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/022-orchestrator-bootstrap-and-daemon-boundaries.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 12 | 1 | 16 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/013-openclaw-ws-native-strategy.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/001-burn-backend-selection.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/031-deprecate-vox-vscode.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 10 | 3 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/010-tanstack-web-spine.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 7 | 4 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/040-ai-fixture-search-decorator.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/005-socrates-anti-hallucination-ssot.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 3 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/028-deprecate-stub-durability-grammar.md` | Architecture Decisions (ADRs) | deprecated | **MoveToInternals** | 2 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/014-async-openai-selective-adoption-spike.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 8 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/002-diataxis-doc-architecture.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 1 | 1 | 51 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/008-populi-transport.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 1 | 5 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/018-populi-gpu-truth-layering.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 2 | 20 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/007-qlora-rs-multi-layer-training-api.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 2 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/024-dashboard-axum-spa.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 6 | 4 | 15 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/023-optional-telemetry-remote-upload.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 1 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/020-populi-mesh-scaling-transport-default.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 0 | 2 | 15 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/047-iroh-transport.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 2 | 3 | 38 | ADR / architecture decision record belongs in Internals group |
| `docs/src/adr/006-mens-full-graph-qlora-qlora-rs.md` | Architecture Decisions (ADRs) | current | **MoveToInternals** | 4 | 3 | 81 | ADR / architecture decision record belongs in Internals group |
| `docs/src/explanation/expl-compiler-lowering.md` | Concepts | current | **Archive** | 4 | 0 | 80 | Orphaned page (0 inbound links) with 4 dead mentions, age 80 days |
| `docs/src/explanation/expl-ai-orchestration.md` | Concepts | current | **Keep** | 2 | 0 | 51 | Active public reader documentation with valid references |
| `docs/src/explanation/expl-actors-workflows.md` | Concepts | current | **Keep** | 0 | 4 | 50 | Active public reader documentation with valid references |
| `docs/src/explanation/expl-durable-execution.md` | Concepts | current | **Keep** | 0 | 0 | 51 | Active public reader documentation with valid references |
| `docs/src/explanation/zig-inspired-deployment.md` | Concepts | current | **Archive** | 3 | 0 | 81 | Orphaned page (0 inbound links) with 3 dead mentions, age 81 days |
| `docs/src/explanation/expl-security.md` | Concepts | current | **Merge** | 1 | 2 | 50 | Member of topic cluster cluster-004 with overlapping content |
| `docs/src/explanation/glossary.md` | Concepts | current | **Keep** | 0 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/explanation/expl-architecture.md` | Concepts | current | **Keep** | 3 | 4 | 34 | Active public reader documentation with valid references |
| `docs/src/explanation/faq.md` | Concepts | current | **Keep** | 1 | 0 | 35 | Active public reader documentation with valid references |
| `docs/src/explanation/expl-capabilities.md` | Concepts | current | **Archive** | 3 | 0 | 50 | Orphaned page (0 inbound links) with 3 dead mentions, age 50 days |
| `docs/src/explanation/expl-openclaw-analysis.md` | Concepts | current | **Archive** | 3 | 0 | 80 | Orphaned page (0 inbound links) with 3 dead mentions, age 80 days |
| `docs/src/explanation/expl-ml-pipeline.md` | Concepts | current | **Archive** | 7 | 0 | 50 | Orphaned page (0 inbound links) with 7 dead mentions, age 50 days |
| `docs/src/explanation/mcp_serverless_research.md` | Concepts | current | **Archive** | 3 | 0 | 81 | Orphaned page (0 inbound links) with 3 dead mentions, age 81 days |
| `docs/src/explanation/expl-memory-model.md` | Concepts | current | **Merge** | 0 | 0 | 128 | Member of topic cluster cluster-004 with overlapping content |
| `docs/src/explanation/why-vox-for-ai.md` | Concepts | current | **Keep** | 2 | 3 | 38 | Active public reader documentation with valid references |
| `docs/src/explanation/expl-runtime.md` | Concepts | current | **Keep** | 1 | 1 | 80 | Active public reader documentation with valid references |
| `docs/src/explanation/expl-rosetta-inventory.md` | Concepts | current | **Keep** | 1 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/archive/phase5-react-interop-spec-2026.md` | Architecture SSOTs | deprecated | **Archive** | 20 | 0 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-typeck.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/how-to-islands-and-pages.md` | How-To Guides | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-parser.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-codegen-rust.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-ast.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-index-hand-curated-retired-2026-09.md` | archive | deprecated | **Archive** | 59 | 0 | 36 | Already located under docs/src/archive/ |
| `docs/src/archive/claude-design-may-2026/README.md` | archive | deprecated | **Archive** | 0 | 0 | 138 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-mcp.md` | API Reference — Crates | deprecated | **Archive** | 3 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/cli-reachability-ssot.md` | Architecture SSOTs | current | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-slop-typicality-bias-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/openapi-contract-ssot.md` | Architecture SSOTs | current | **Archive** | 2 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mobile-desktop-convergence-research-2026.md` | Architecture SSOTs | current | **Archive** | 27 | 2 | 129 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/external-scripts-boundary-archive.md` | Language Reference | current | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-finetune-migration.md` | Language Reference | current | **Archive** | 3 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-gui-vision-virtuous-cycle-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 22 | 6 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-packaging-implementation-blueprint.md` | Architecture SSOTs | current | **Archive** | 13 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/toestub-self-healing-architecture-2026.md` | Language Reference | current | **Archive** | 9 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-client-disclosure-ssot.md` | Architecture SSOTs | current | **Archive** | 0 | 6 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/retired-syntax-prose-lint-investigation-2026-08-23.md` | Architecture SSOTs | research | **Archive** | 0 | 0 | 49 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-qlora-minimum-corpus-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/context-management-research-findings-2026.md` | Architecture SSOTs | research | **Archive** | 27 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-external-discovery-research-2026.md` | Architecture SSOTs | research | **Archive** | 2 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-finetune-deprecation.md` | Language Reference | current | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-trust-ssot.md` | Architecture SSOTs | current | **Archive** | 2 | 14 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/clavis-one-stop-secrets-research-2026.md` | Architecture SSOTs | research | **Archive** | 35 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-mad-mode-collapse-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-visus-vlm-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox_agentic_loop_and_mens_plan.md` | Architecture SSOTs | roadmap | **Archive** | 28 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/css-determinism-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 22 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/react-interop-migration-charter-2026.md` | Architecture SSOTs | current | **Archive** | 10 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-publication-endpoints-research-2026.md` | Architecture SSOTs | research | **Archive** | 39 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/codex-legacy-migration.md` | Language Reference | current | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-index.md` | Architecture SSOTs | research | **Archive** | 7 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/table-spec-ssot.md` | Architecture SSOTs | current | **Archive** | 2 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/syndication-ecosystem-research-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-multi-agent-mesh-economics-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/comprehensive-audit-findings-2026.md` | Architecture SSOTs | research | **Archive** | 18 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ts-hallucination-zero-shot-invariants-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/voxdb-connect-policy.md` | Architecture SSOTs | current | **Archive** | 2 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/erlang-benefits-for-llms-research-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-language-testing-pipeline.md` | Architecture SSOTs | research | **Archive** | 9 | 2 | 129 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-implementation-wave-playbook-2026.md` | Architecture SSOTs | roadmap | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-cli-build-feature-inventory.md` | Language Reference | current | **Archive** | 16 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-llm-pr-checklist.md` | Language Reference | current | **Archive** | 6 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-finetune-acceptance-runbook.md` | Language Reference | current | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/codex-arca-import-policy.md` | Language Reference | current | **Archive** | 3 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/context-management-phase1-backlog.md` | Architecture SSOTs | roadmap | **Archive** | 21 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scholarly-digest-approval-invariants.md` | Architecture SSOTs | current | **Archive** | 2 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/dependency-sprawl-research-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/orchestrator-symphony-research-2026.md` | Architecture SSOTs | research | **Archive** | 7 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-architecture-2026-synthesis.md` | Language Reference | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/terminal-exec-policy-research-findings-2026.md` | Architecture SSOTs | research | **Archive** | 4 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/compatibility-deprecation-windows.md` | Language Reference | current | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/populi-gpu-mesh-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 10 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-boilerplate-implementation-status.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/tanstack-web-roadmap.md` | Language Reference | current | **Archive** | 3 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/custom-model-pretraining-research-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/rosetta-language-comparison-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/gui-visual-intelligence-research-2026.md` | Architecture SSOTs | research | **Archive** | 4 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-vram-small-batch-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/cli-scope-policy.md` | Language Reference | current | **Archive** | 9 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/local-autonomous-research-findings-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-implementation-backlog-2026.md` | Architecture SSOTs | roadmap | **Archive** | 34 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/agentic-user-journeys.md` | Architecture SSOTs | research | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-risk-taxonomy-telemetry-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-retention-sensitivity-ssot.md` | Architecture SSOTs | roadmap | **Archive** | 1 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/doc-to-code-acceptance-checklist.md` | Architecture SSOTs | current | **Archive** | 6 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-binary-parse-rate-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ts-hallucination-empirical-evidence-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/codex-turso-allowlist.md` | Language Reference | current | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/oratio-ide-protocol-ssot.md` | Architecture SSOTs | current | **Archive** | 6 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/codex-vnext-schema.md` | Language Reference | current | **Archive** | 3 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-sota-protocols-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/populi-gpu-network-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/script-surface-audit.md` | Architecture SSOTs | current | **Archive** | 66 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-community-publishing-playbook-2026.md` | Architecture SSOTs | roadmap | **Archive** | 22 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/populi-gpu-truth-probe-spec.md` | Language Reference | current | **Archive** | 1 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ludus-schema-inventory.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-failure-catalog-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-boilerplate-reduction-master-roadmap.md` | Architecture SSOTs | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-a2a-evidence-sharing-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/news_syndication_security.md` | Architecture SSOTs | current | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-a2a-evidence-tasks.md` | Architecture SSOTs | current | **Archive** | 6 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-ast-reward-hacking-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/version-tracking-ssot-research-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/nomenclature-migration-map.md` | Architecture SSOTs | current | **Archive** | 20 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/hf-finetune-capability-matrix.md` | Language Reference | current | **Archive** | 8 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/env-vars-clavis-strategy-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-docker-dotvox-portability-research-2026.md` | Architecture SSOTs | current | **Archive** | 20 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/qwen35-multimodal-phase2-backlog.md` | Architecture SSOTs | current | **Archive** | 2 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-pbt-oracles-compiled-lang-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/security_model.md` | Architecture SSOTs | current | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/multi-repo-context-isolation-research-2026.md` | Architecture SSOTs | research | **Archive** | 7 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/plan-adequacy.md` | Architecture SSOTs | current | **Archive** | 5 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ludus-integration-contract.md` | Architecture SSOTs | current | **Archive** | 10 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/visus-implementation-plan-2026.md` | Architecture SSOTs | legacy | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mcp-vox-language-exposure.md` | Architecture SSOTs | current | **Archive** | 18 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-reward-weights-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-corpus-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 27 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/asr-speech-to-code-architecture-2026.md` | Architecture SSOTs | research | **Archive** | 5 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/react-interop-research-findings-2026.md` | Architecture SSOTs | current | **Archive** | 30 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/hitl-doubt-loop-ssot.md` | Architecture SSOTs | current | **Archive** | 3 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-unification-research-findings-2026.md` | Architecture SSOTs | research | **Archive** | 2 | 5 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-source-to-mens-pipeline-ssot.md` | Architecture SSOTs | current | **Archive** | 16 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-0.4-migration-plan.md` | Architecture SSOTs | roadmap | **Archive** | 5 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/llm-target-language-gap-analysis-2026.md` | Architecture SSOTs | research | **Archive** | 38 | 2 | 129 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-gaps-and-adjustments-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-synthetic-corpus-limitations-research-2026.md` | Architecture SSOTs | research | **Archive** | 15 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vcs-agent-state-research-2026.md` | Architecture SSOTs | research | **Archive** | 8 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-nat-dpo-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-publication-pipeline-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 46 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/feature-growth-boundaries.md` | Architecture SSOTs | current | **Archive** | 1 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/qwen36-integration-research.md` | Architecture SSOTs | current | **Archive** | 4 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/english_core_migration_ledger.md` | Architecture SSOTs | legacy | **Archive** | 12 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/tanstack-start-implementation-backlog.md` | Architecture SSOTs | current | **Archive** | 27 | 0 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-dei-hitl-ssot.md` | Architecture SSOTs | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/clavis-implementation-plan-2026.md` | Architecture SSOTs | experimental | **Archive** | 81 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/clavis-secrets-env-research-2026.md` | Architecture SSOTs | research | **Archive** | 15 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/clavis-cloudless-threat-model-v1.md` | Architecture SSOTs | roadmap | **Archive** | 6 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/trust-reliability-layer.md` | Architecture SSOTs | current | **Archive** | 4 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ai-ide-feature-research-findings-2026.md` | Architecture SSOTs | research | **Archive** | 4 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scaling-ci-enforcement.md` | Architecture SSOTs | current | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-syntax-highlighting-ssot-2026.md` | Architecture SSOTs | research | **Archive** | 26 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/internal-web-ir-side-by-side-schema.md` | Architecture SSOTs | current | **Archive** | 34 | 0 | 104 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-packaging-full-implementation-plan-2026.md` | Architecture SSOTs | current | **Archive** | 20 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-works-cited-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-pipeline-ssot-2026.md` | Architecture SSOTs | research | **Archive** | 60 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/doc-hygiene-agent-guidelines-2026.md` | Architecture SSOTs | current | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ssot-convergence-roadmap.md` | Architecture SSOTs | current | **Archive** | 1 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-web-stack-ssot.md` | Architecture SSOTs | current | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-erlang-comparison-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ts-hallucination-frontier-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/compiler-ir-pipeline.md` | Architecture SSOTs | current | **Archive** | 3 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/tanstack-web-backlog.md` | Language Reference | current | **Archive** | 15 | 3 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-shell-operations-boundaries.md` | Architecture SSOTs | current | **Archive** | 8 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-as-glue-research-2026.md` | Architecture SSOTs | research | **Archive** | 28 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ast-token-alignment-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 129 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/codex-arca-compatibility-boundaries.md` | Language Reference | current | **Archive** | 5 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-library-mode-ssot.md` | Architecture SSOTs | current | **Archive** | 6 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/memory-management-llm-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/migration-script-dashboard.md` | Language Reference | current | **Archive** | 5 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ai-augmented-testing-hourglass-research-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/phase0-migration-signoff.md` | Language Reference | current | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/react-interop-hybrid-adapter-cookbook.md` | Architecture SSOTs | current | **Archive** | 5 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/config-ssot.md` | Architecture SSOTs | current | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/populi-node-lifecycle-hotplug.md` | Architecture SSOTs | current | **Archive** | 2 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-qlora-catastrophic-forgetting-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ir-emission-ssot.md` | Architecture SSOTs | current | **Archive** | 2 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/interop-tier-policy.md` | Architecture SSOTs | current | **Archive** | 2 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ts-hallucination-cognitive-science-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/network-neuroscience-theory-research-2026.md` | Architecture SSOTs | research | **Archive** | 5 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/react-interop-minimal-shell-strategy.md` | Architecture SSOTs | current | **Archive** | 6 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-boilerplate-research-findings-2026.md` | Architecture SSOTs | current | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-taxonomy-contracts-ssot.md` | Architecture SSOTs | roadmap | **Archive** | 5 | 5 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/execution-time-agent-budgeting-research-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-publication-readiness-audit.md` | Language Reference | current | **Archive** | 9 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/claude-code-ultraplan-research-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/nnt-orchestration-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 2 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/completion-policy-ssot.md` | Architecture SSOTs | current | **Archive** | 3 | 5 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/crate-build-lanes-migration.md` | Language Reference | current | **Archive** | 18 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/graphrag-iterative-retrieval-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/trim-build-defer-policy.md` | Language Reference | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/README.md` | Architecture SSOTs | legacy | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/cryptography-ssot-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-works-cited-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-lang-training-ssot-2026.md` | Architecture SSOTs | current | **Archive** | 7 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-reward-shaping-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 5 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/voxgiantia-publication-architecture.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-training-ssot.md` | Architecture SSOTs | current | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-llm-output-mediation-validation-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/architecture-index.md` | Architecture SSOTs | current | **Archive** | 2 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/memory_system.md` | Architecture SSOTs | current | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scholarly-external-schema-plan.md` | Architecture SSOTs | current | **Archive** | 2 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/asr-speech-to-code-findings-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-semantic-proximity-split-brain-2026.md` | Architecture SSOTs | research | **Archive** | 9 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-implementation-blueprint-2026.md` | Architecture SSOTs | roadmap | **Archive** | 0 | 6 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-socrates-unification-research-2026.md` | Architecture SSOTs | research | **Archive** | 13 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-packaging-research-findings-2026.md` | Architecture SSOTs | current | **Archive** | 18 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/populi-workflow-guide.md` | Language Reference | current | **Archive** | 4 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-plan-adequacy-heuristics-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/agent-event-kind-ludus-matrix.md` | Architecture SSOTs | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-corpus-lab-research-2026.md` | Architecture SSOTs | research | **Archive** | 5 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-retrieval-policies-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grammar-constrained-decoding-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-cl-oracle-semantic-drift-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/legacy-retirement-roadmap.md` | Architecture SSOTs | current | **Archive** | 25 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/classification-ssot-2026.md` | Architecture SSOTs | current | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-design-patterns-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/clavis-cloudless-implementation-catalog.md` | Architecture SSOTs | roadmap | **Archive** | 25 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/agent-planning-multimodal-ssot.md` | Architecture SSOTs | current | **Archive** | 1 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/INDEX.md` | Architecture SSOTs | research | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/react-interop-backlog-2026.md` | Architecture SSOTs | current | **Archive** | 5 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/crate-topology-buckets.md` | Language Reference | current | **Archive** | 18 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ludus-non-goals.md` | Architecture SSOTs | current | **Archive** | 2 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-empirical-compaction-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-lane-segmentation-research.md` | Architecture SSOTs | current | **Archive** | 2 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/fableforge-to-vox-conversion-research-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/capability-registry-ssot.md` | Architecture SSOTs | current | **Archive** | 14 | 5 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/orchestrator-multi-agent-groundwork-2026.md` | Architecture SSOTs | current | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/telemetry-remote-sink-spec.md` | Architecture SSOTs | current | **Archive** | 1 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/agent-messaging-orchestration-roadmap.md` | Language Reference | current | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-bell-curve-strategy.md` | Architecture SSOTs | current | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/a2a-orchestration-hardening-findings-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/rag-and-research-architecture-2026.md` | Architecture SSOTs | current | **Archive** | 17 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/v05-crate-reorganization-ssot-2026.md` | Architecture SSOTs | current | **Archive** | 41 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/language-surface-ssot.md` | Architecture SSOTs | current | **Archive** | 10 | 6 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-gap-analysis-2026.md` | Architecture SSOTs | research | **Archive** | 53 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/css-ai-inference-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-language-alias-canonicalization-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-oratio-speech-to-code-findings-2026.md` | Architecture SSOTs | legacy | **Archive** | 4 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/web-framework-interop-research-2026.md` | Architecture SSOTs | research | **Archive** | 59 | 2 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-vision-multimodal-research-2026.md` | Architecture SSOTs | research | **Archive** | 2 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-visus-vlm-blueprint-2026.md` | Architecture SSOTs | roadmap | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/protocol-convergence-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 6 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/automated-testing-research-2026.md` | Architecture SSOTs | research | **Archive** | 10 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vscode-extension-redesign-research-2026.md` | Architecture SSOTs | research | **Archive** | 9 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-composer-kimi-findings-2026.md` | Language Reference | current | **Archive** | 2 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/typescript-migration-boundary.md` | Language Reference | current | **Archive** | 2 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/forward-migration-charter.md` | Language Reference | current | **Archive** | 7 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/populi-data-pipeline.md` | Architecture SSOTs | current | **Archive** | 1 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-cross-platform-runbook.md` | Language Reference | current | **Archive** | 1 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-publisher-hardening-implementation-plan-2026.md` | Architecture SSOTs | experimental | **Archive** | 4 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/coderabbit-review-coverage-ssot.md` | Architecture SSOTs | current | **Archive** | 9 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-works-cited-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-continual-learning-flywheel-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-truncation-failure-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scaling-audit-baseline.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/compiler-diagnostics-ergonomics.md` | Architecture SSOTs | current | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-organization.md` | Architecture SSOTs | current | **Archive** | 6 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-synthesis-grand-strategy-seed-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/terminal-ast-validation-research-2026.md` | Architecture SSOTs | research | **Archive** | 10 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/cryptography-research-findings-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-as-glue-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **Archive** | 14 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/outbound-http-policy.md` | Architecture SSOTs | current | **Archive** | 10 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/candle-full-graph-feasibility.md` | Language Reference | current | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-multi-platform-ranking-discovery-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-checklist-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-context-handoff-continuity-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/cross-repo-query-observability.md` | Architecture SSOTs | current | **Archive** | 6 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-qwen-family-migration-research-2026.md` | Architecture SSOTs | research | **Archive** | 3 | 7 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/capability-visualization-views.md` | Architecture SSOTs | current | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-publication-automation-ssot.md` | Language Reference | current | **Archive** | 19 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-trust-reliability-signals-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/actor-gc-implementation-blueprint-2026.md` | Architecture SSOTs | roadmap | **Archive** | 3 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-fuzzy-parsing-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/session_management.md` | Architecture SSOTs | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/context-management-implementation-blueprint.md` | Architecture SSOTs | roadmap | **Archive** | 4 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-grpo-positive-only-optimization-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/news_syndication_incident_patterns.md` | Architecture SSOTs | current | **Archive** | 1 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/res_dynamic_agentic_planning_2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/web-framework-interop-backlog-2026.md` | Architecture SSOTs | roadmap | **Archive** | 14 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/testing-standard.md` | Language Reference | current | **Archive** | 5 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ts-hallucination-k-complexity-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-agent-handoff-context-bleed-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/god-object-defactor-checklist.md` | Architecture SSOTs | current | **Archive** | 57 | 1 | 129 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/gpu-discovery-ssot-findings-2026.md` | Architecture SSOTs | current | **Archive** | 1 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/prompt-engineering-document-skills-scientia-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/toestub-line-limit-mens-research-2026.md` | Architecture SSOTs | research | **Archive** | 2 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-llm-native-language-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/agent-handoff-contract-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/tanstack-start-codegen-spec.md` | Architecture SSOTs | current | **Archive** | 15 | 0 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/llm-target-language-implementation-plan-2026.md` | Architecture SSOTs | current | **Archive** | 14 | 2 | 129 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/maintainability-hotspot-matrix.md` | Architecture SSOTs | current | **Archive** | 8 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/cli-design-rules-ssot.md` | Architecture SSOTs | current | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ai-panic-shortcuts-2026.md` | Architecture SSOTs | research | **Archive** | 9 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/operations-catalog-ssot.md` | Architecture SSOTs | current | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/rust-ecosystem-support-ssot.md` | Architecture SSOTs | current | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scaling-toestub-rules.md` | Architecture SSOTs | current | **Archive** | 4 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-automation-primitives.md` | Language Reference | current | **Archive** | 8 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/mens-research-track-blueprint-2026.md` | Architecture SSOTs | roadmap | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-phonetic-operators-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/web-architecture-analysis-2026.md` | Language Reference | current | **Archive** | 11 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/ai-cli-generation-standard.md` | Architecture SSOTs | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-ts-hallucination-works-cited-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-publication-worthiness-ssot-unification-research-2026.md` | Architecture SSOTs | research | **Archive** | 1 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/orchestrator-attachment-manifest-rfc-2026.md` | Architecture SSOTs | roadmap | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-docker-dotvox-portability-implementation-plan-2026.md` | Architecture SSOTs | current | **Archive** | 29 | 2 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-diagnostic-questioning-2026.md` | Architecture SSOTs | research | **Archive** | 11 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/orphan-surface-inventory.md` | Language Reference | current | **Archive** | 22 | 6 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/agentic-control-surface-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/deployment-compose-ssot.md` | Architecture SSOTs | current | **Archive** | 2 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/codex-baas.md` | Language Reference | current | **Archive** | 0 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/internal-web-ir-implementation-blueprint.md` | Architecture SSOTs | current | **Archive** | 249 | 8 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/research-llm-native-lang-design-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 4 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/react-interop-implementation-plan-2026.md` | Architecture SSOTs | current | **Archive** | 35 | 1 | 104 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/scientia-impact-readership-research-2026.md` | Architecture SSOTs | research | **Archive** | 0 | 3 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-tensor-pytorch-gap-audit.md` | Architecture SSOTs | research | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/research-2026-q1/vox-fullstack-ergonomics-deep-dive.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 123 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-wasm.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-py.md` | API Reference — Crates | deprecated | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-codegen-ts.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-dei.md` | API Reference — Crates | deprecated | **Archive** | 1 | 1 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-hir.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/readme-scientia-2026.md` | Concepts | deprecated | **Archive** | 8 | 0 | 104 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-dei-sandbox.md` | API Reference — Crates | deprecated | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-gamify.md` | API Reference — Crates | deprecated | **Archive** | 1 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/archive/vox-lexer.md` | API Reference — Crates | deprecated | **Archive** | 0 | 0 | 142 | Already located under docs/src/archive/ |
| `docs/src/ci/command-compliance-ssot.md` | CI & Quality | legacy | **Archive** | 1 | 0 | 81 | Frontmatter status is 'legacy' |
| `docs/src/ci/compute-placement.md` | CI & Quality | current | **Archive** | 4 | 0 | 16 | Orphaned page (0 inbound links) with 4 dead mentions, age 16 days |
| `docs/src/ci/concurrency-exceptions.md` | CI & Quality | current | **Keep** | 1 | 0 | 16 | Active public reader documentation with valid references |
| `docs/src/ci/rcicd-coverage-cost-matrix-2026.md` | CI & Quality | current | **Keep** | 8 | 1 | 16 | Active public reader documentation with valid references |
| `docs/src/ci/binary-release-contract.md` | CI & Quality | current | **Keep** | 9 | 1 | 16 | Active public reader documentation with valid references |
| `docs/src/ci/alternatives-and-local-mirroring.md` | CI & Quality | current | **Archive** | 6 | 0 | 16 | Orphaned page (0 inbound links) with 6 dead mentions, age 16 days |
| `docs/src/ci/runner-contract.md` | CI & Quality | current | **Keep** | 14 | 5 | 12 | Active public reader documentation with valid references |
| `docs/src/ci/cli-baseline-metrics.md` | CI & Quality | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/ci/workflow-enumeration.md` | CI & Quality | current | **Keep** | 8 | 4 | 16 | Active public reader documentation with valid references |
| `docs/src/ci/workspace-root-manifest.md` | CI & Quality | current | **Keep** | 3 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/ci/documentation-pointers.md` | CI & Quality | current | **Keep** | 4 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/ci/command-surface-duals.md` | CI & Quality | current | **Keep** | 6 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/ci/crate-hardening-matrix.md` | CI & Quality | current | **Keep** | 4 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/ci/deploy-contract.md` | CI & Quality | current | **Archive** | 8 | 0 | 114 | Orphaned page (0 inbound links) with 8 dead mentions, age 114 days |
| `docs/src/ci/rust-modernization-baseline.md` | CI & Quality | current | **Keep** | 4 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/ci/boilerplate-metrics.md` | CI & Quality | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/ci/build-time-program.md` | CI & Quality | current | **Keep** | 1 | 0 | 119 | Active public reader documentation with valid references |
| `docs/src/ci/shared-compile-cache.md` | CI & Quality | current | **Keep** | 1 | 0 | 16 | Active public reader documentation with valid references |
| `docs/src/ci/affected-crate-selective-ci.md` | CI & Quality | current | **Keep** | 0 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/learning/rust-via-vox/lesson-00-syntax-primer.md` | Tutorials | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/learning/rust-via-vox/notes-session-01.md` | Tutorials | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/learning/rust-via-vox/lesson-01-ownership-pattern-drill.md` | Tutorials | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/index.mdx` | Getting Started | current | **Keep** | 0 | 0 | 35 | Active public reader documentation with valid references |
| `docs/src/operations/vox-hosting-infrastructure-cost-roi-2026.md` | Operations | research | **MoveToInternals** | 0 | 0 | 108 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/operations/secrets-break-glass-runbook.md` | Operations | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/operations/secrets-cloudless-ops-runbook.md` | Operations | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/operations/populi-overlay-personal-cluster-runbook.md` | Operations | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/operations/voxdb-cutover-runbook.md` | Operations | current | **Archive** | 10 | 0 | 81 | Orphaned page (0 inbound links) with 10 dead mentions, age 81 days |
| `docs/src/operations/populi-remote-execution-rollout-checklist.md` | Operations | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-mcp-vox-validate.md` | How-To Guides | current | **Archive** | 9 | 0 | 81 | Orphaned page (0 inbound links) with 9 dead mentions, age 81 days |
| `docs/src/how-to/examples.md` | How-To Guides | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-pick-a-vox-bundle.md` | How-To Guides | current | **Archive** | 9 | 0 | 142 | Orphaned page (0 inbound links) with 9 dead mentions, age 142 days |
| `docs/src/how-to/how-to-unit-test.md` | How-To Guides | current | **Keep** | 0 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/clinical-export-from-vox-apps.md` | How-To Guides | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/how-to/populi-quickstart.md` | How-To Guides | current | **Keep** | 2 | 0 | 15 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-database.md` | How-To Guides | current | **Keep** | 2 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/troubleshooting-faq.md` | How-To Guides | current | **Archive** | 9 | 0 | 81 | Orphaned page (0 inbound links) with 9 dead mentions, age 81 days |
| `docs/src/how-to/editor-integrations.md` | How-To Guides | current | **Archive** | 5 | 0 | 81 | Orphaned page (0 inbound links) with 5 dead mentions, age 81 days |
| `docs/src/how-to/how-to-author-views.md` | How-To Guides | current | **Keep** | 2 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/scientia-publication-operator-inputs.md` | How-To Guides | current | **Keep** | 2 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/grand-network-quickstart.md` | How-To Guides | current | **Keep** | 1 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-deploy.md` | How-To Guides | current | **Keep** | 5 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-train-mens-macos-metal.md` | How-To Guides | current | **Archive** | 11 | 0 | 33 | Orphaned page (0 inbound links) with 11 dead mentions, age 33 days |
| `docs/src/how-to/how-to-train-mens-4080.md` | How-To Guides | current | **Archive** | 21 | 0 | 20 | Orphaned page (0 inbound links) with 21 dead mentions, age 20 days |
| `docs/src/how-to/how-to-error-handling.md` | How-To Guides | current | **Keep** | 0 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-contribute-parser-hir.md` | How-To Guides | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-migrate-from-cargo-features.md` | How-To Guides | current | **Archive** | 14 | 0 | 142 | Orphaned page (0 inbound links) with 14 dead mentions, age 142 days |
| `docs/src/how-to/how-to-pytorch.md` | How-To Guides | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-contribute-populi.md` | How-To Guides | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-ai-agents.md` | How-To Guides | current | **Keep** | 2 | 2 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-scale-actors.md` | How-To Guides | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-custom-types.md` | How-To Guides | current | **Keep** | 0 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-rust-crate-imports.md` | How-To Guides | current | **Archive** | 8 | 0 | 50 | Orphaned page (0 inbound links) with 8 dead mentions, age 50 days |
| `docs/src/how-to/how-to-testing-integration.md` | How-To Guides | current | **Keep** | 0 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/index.md` | How-To Guides | current | **Keep** | 0 | 1 | 0 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-share.md` | How-To Guides | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/external-app-bootstrap.md` | How-To Guides | current | **Archive** | 3 | 0 | 38 | Orphaned page (0 inbound links) with 3 dead mentions, age 38 days |
| `docs/src/how-to/first-full-stack-app.md` | How-To Guides | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-axis-drive-openrouter-e2e.md` | How-To Guides | current | **Keep** | 0 | 0 | 33 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-voxdb-canonical-store.md` | How-To Guides | current | **Archive** | 4 | 0 | 81 | Orphaned page (0 inbound links) with 4 dead mentions, age 81 days |
| `docs/src/how-to/how-to-cli-ecosystem.md` | How-To Guides | current | **Archive** | 6 | 0 | 35 | Orphaned page (0 inbound links) with 6 dead mentions, age 35 days |
| `docs/src/how-to/how-to-code-signing.md` | How-To Guides | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-scientia-publication.md` | How-To Guides | current | **Keep** | 5 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-system-io.md` | How-To Guides | current | **Archive** | 6 | 0 | 50 | Orphaned page (0 inbound links) with 6 dead mentions, age 50 days |
| `docs/src/how-to/tanstack-ssr-with-axum.md` | How-To Guides | current | **Archive** | 5 | 0 | 81 | Orphaned page (0 inbound links) with 5 dead mentions, age 81 days |
| `docs/src/how-to/examples-corpus.md` | How-To Guides | current | **Keep** | 0 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/how-to/how-to-contribute-mens.md` | How-To Guides | current | **Archive** | 4 | 0 | 81 | Orphaned page (0 inbound links) with 4 dead mentions, age 81 days |
| `docs/src/how-to/how-to-model-routing.md` | How-To Guides | current | **Archive** | 4 | 0 | 31 | Orphaned page (0 inbound links) with 4 dead mentions, age 31 days |
| `docs/src/case-studies/v0.5-production-deployment.md` | Examples | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/contributors/toestub-contributor-guide.md` | Contributors | current | **Keep** | 5 | 3 | 50 | Active public reader documentation with valid references |
| `docs/src/contributors/docs-reality-audit-program.md` | Contributors | roadmap | **MoveToInternals** | 0 | 2 | 51 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/contributors/coding-agents.md` | Contributors | current | **Keep** | 3 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/contributors/axis-brand.md` | Contributors | current | **Archive** | 3 | 0 | 115 | Orphaned page (0 inbound links) with 3 dead mentions, age 115 days |
| `docs/src/contributors/detector-coverage-ledger.md` | Contributors | current | **Archive** | 4 | 0 | 106 | Orphaned page (0 inbound links) with 4 dead mentions, age 106 days |
| `docs/src/contributors/build-broker-usage.md` | Contributors | current | **Archive** | 15 | 0 | 37 | Orphaned page (0 inbound links) with 15 dead mentions, age 37 days |
| `docs/src/contributors/contributor-hub.md` | Contributors | current | **Keep** | 0 | 2 | 36 | Active public reader documentation with valid references |
| `docs/src/contributors/github-linguist-vox.md` | Contributors | current | **Keep** | 1 | 0 | 106 | Active public reader documentation with valid references |
| `docs/src/contributors/antigravity-handoff-and-skill-gaps-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 106 | Active public reader documentation with valid references |
| `docs/src/contributors/claude-code-sonnet-handoff-limitations.md` | Contributors | current | **Archive** | 3 | 0 | 106 | Orphaned page (0 inbound links) with 3 dead mentions, age 106 days |
| `docs/src/contributors/vox-binary-freshness-and-ssot-2026.md` | Contributors | research | **MoveToInternals** | 8 | 0 | 128 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/contributors/continuation-prompt-engineering.md` | Contributors | current | **Keep** | 0 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/contributors/antigravity-driven-execution.md` | Contributors | current | **Archive** | 5 | 0 | 14 | Orphaned page (0 inbound links) with 5 dead mentions, age 14 days |
| `docs/src/contributors/publishing-crates.md` | Contributors | current | **Keep** | 1 | 0 | 106 | Active public reader documentation with valid references |
| `docs/src/contributors/contribution-loop.md` | Contributors | current | **Keep** | 5 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/contributors/documentation-governance.md` | Contributors | current | **Keep** | 2 | 4 | 36 | Active public reader documentation with valid references |
| `docs/src/contributors/dependency-policy.md` | Contributors | current | **Keep** | 2 | 1 | 36 | Active public reader documentation with valid references |
| `docs/src/contributors/agent-instruction-architecture.md` | Contributors | current | **Keep** | 1 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/contributors/local-ci-pre-push.md` | Contributors | current | **Keep** | 3 | 1 | 16 | Active public reader documentation with valid references |
| `docs/src/contributors/vox-deep-research-primer.md` | Contributors | current | **Keep** | 0 | 0 | 15 | Active public reader documentation with valid references |
| `docs/src/contributors/agent-planning-prompts.md` | Contributors | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-shell-stdlib-ssot-2026.md` | Architecture SSOTs | current | **Archive** | 39 | 0 | 81 | Orphaned page (0 inbound links) with 39 dead mentions, age 81 days |
| `docs/src/architecture/local-models-vs-openrouter-quality-research-2026.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 26 | Orphaned page (0 inbound links) with 3 dead mentions, age 26 days |
| `docs/src/architecture/context-window-management-design-2026-06-20.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 17 | 0 | 101 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/auto-derivation-design-hygiene-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 1 | 5 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-react-backend-interop-audit-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 38 | 3 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/where-things-live.md` | Architecture SSOTs | current | **Keep** | 67 | 18 | 13 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 26 | Orphaned page (0 inbound links) with 4 dead mentions, age 26 days |
| `docs/src/architecture/dead-crate-fate-plan-2026-05-08.md` | Architecture SSOTs | research | **MoveToInternals** | 52 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/voxup-omnibus-installer-spec-2026.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 34 | Orphaned page (0 inbound links) with 3 dead mentions, age 34 days |
| `docs/src/architecture/mobile-rn-expo-implementation-spec-2026.md` | Architecture SSOTs | experimental | **Keep** | 90 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 26 | Active public reader documentation with valid references |
| `docs/src/architecture/agentos-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 5 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-gui-visual-audit-2026-06-03.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 87 | Orphaned page (0 inbound links) with 3 dead mentions, age 87 days |
| `docs/src/architecture/mesh-mens-distributed-training-and-execution-plan-2026.md` | Architecture SSOTs | current | **Keep** | 121 | 5 | 31 | Active public reader documentation with valid references |
| `docs/src/architecture/language-telemetry-blind-spots-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-language-rules-and-enforcement-plan-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 4 | 15 | 36 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/shiki-mdbook-doc-platform-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 19 | 0 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/semantic-test-coverage-graph-strategy-2026-06-07.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 4 | 3 | 116 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/terminal-exec-policy-ssot.md` | Architecture SSOTs | current | **Keep** | 0 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/build-time-baseline.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/telemetry-driven-cost-accounting-research-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 4 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-language-migrations-ssot-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 1 | 1 | 51 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-application-packaging-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 4 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-efficacy-benchmark-execution-handoff-2026-09-04.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 2 | 39 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/telemetry-trust-ssot.md` | Architecture SSOTs | current | **Merge** | 0 | 12 | 142 | Member of topic cluster cluster-007 with overlapping content |
| `docs/src/architecture/crate-consolidation-and-dependency-currency-audit-2026-06-12.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 122 | Orphaned page (0 inbound links) with 4 dead mentions, age 122 days |
| `docs/src/architecture/data-storage-migration-backlog-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 72 | 2 | 36 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/agent-browser-driver-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 28 | Active public reader documentation with valid references |
| `docs/src/architecture/workspace-dependency-audit-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 1 | 12 | Active public reader documentation with valid references |
| `docs/src/architecture/skill-registry-trust-and-curation-research-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 5 | 5 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 8 | 2 | 116 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md` | Architecture SSOTs | current | **Keep** | 7 | 1 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-speech-improvement-backlog-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 9 | 0 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/cross-platform-guarantees-audit-and-enforcement-2026-06-15.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 119 | Orphaned page (0 inbound links) with 5 dead mentions, age 119 days |
| `docs/src/architecture/mobile-rn-expo-architecture-and-migration-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 11 | 3 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-axis-harness-reliability-spec-plan-2026-07-02.md` | Architecture SSOTs | current | **Keep** | 22 | 2 | 51 | Active public reader documentation with valid references |
| `docs/src/architecture/agentic-version-control-automation-research-2026.md` | Architecture SSOTs | current | **Keep** | 5 | 8 | 50 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-09-detector-rule-ssot-plan.md` | Architecture SSOTs | current | **Archive** | 22 | 0 | 142 | Orphaned page (0 inbound links) with 22 dead mentions, age 142 days |
| `docs/src/architecture/session-handoff-2026-05-28-evening.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 81 | Orphaned page (0 inbound links) with 3 dead mentions, age 81 days |
| `docs/src/architecture/workspace-health-audit-research-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/gui-authoring-syntax-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 15 | 3 | 50 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mens-m4-live-demo-2026-09-12.md` | Architecture SSOTs | current | **Archive** | 19 | 0 | 30 | Orphaned page (0 inbound links) with 19 dead mentions, age 30 days |
| `docs/src/architecture/windows-macos-application-handoff-findings-2026-09.md` | Architecture SSOTs | research | **MoveToInternals** | 2 | 0 | 41 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/_screenshots/research-sources.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/graphify-integration-research-2026-06-16.md` | Architecture SSOTs | research | **MoveToInternals** | 48 | 3 | 116 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mesh-dashboard-and-distributed-compute-research-2026.md` | Architecture SSOTs | current | **Keep** | 28 | 4 | 50 | Active public reader documentation with valid references |
| `docs/src/architecture/agent-browser-driver-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 0 | 26 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-compiler-architecture-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 5 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mesh-phase4-dashboard-control-plan-2026.md` | Architecture SSOTs | current | **Keep** | 152 | 2 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/agentic-vcs-automation-impl-plan-phase1-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 28 | 9 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/work-loss-audit-and-handoff-2026-05-24.md` | Architecture SSOTs | research | **MoveToInternals** | 6 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ai-first-plan-3-gui-intuitiveness-2026-07-02.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 7 | 3 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/graphify-duplicate-corpus-bytes-findings-2026.md` | Architecture SSOTs | current | **Archive** | 8 | 0 | 37 | Orphaned page (0 inbound links) with 8 dead mentions, age 37 days |
| `docs/src/architecture/orchestrator-gui-dispatch-audit-2026-07-02.md` | Architecture SSOTs | current | **Keep** | 10 | 3 | 100 | Active public reader documentation with valid references |
| `docs/src/architecture/path-b-decommission-2026.md` | Architecture SSOTs | current | **Archive** | 8 | 0 | 142 | Orphaned page (0 inbound links) with 8 dead mentions, age 142 days |
| `docs/src/architecture/2026-05-15-cli-ci-extraction-plan.md` | Architecture SSOTs | current | **Keep** | 7 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-axis-stt-accuracy-design-2026-08-01.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 8 | 0 | 71 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mens-training-pipeline-audit-and-improvement-plan-2026-06-07.md` | Architecture SSOTs | research | **MoveToInternals** | 6 | 0 | 126 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/crate-audit-and-plan-2026.md` | Architecture SSOTs | current | **Archive** | 122 | 0 | 141 | Orphaned page (0 inbound links) with 122 dead mentions, age 141 days |
| `docs/src/architecture/context-isolation-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-language-feature-maturity-matrix-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/language-benchmark-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/web-bootstrap-emission-migration-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 25 | 0 | 123 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/session-handoff-2026-05-24-lost-work-audit.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/mesh-phase6-grand-network-plan-2026.md` | Architecture SSOTs | current | **Keep** | 20 | 2 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-populi-extraction-followup-plan-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 16 | 1 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/gradio-streamlit-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 50 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ludus-identity-github-integration-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 15 | 4 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/multi-provider-local-cloud-routing-research-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 8 | 4 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-mens-continuous-benchmark-provenance-design-2026-10-10.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 6 | 0 | 2 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/populi-mesh-a2a-durability-spec-2026.md` | Architecture SSOTs | deprecated | **Archive** | 11 | 5 | 142 | Frontmatter status is 'deprecated' |
| `docs/src/architecture/cli-compilerd-params-ssot.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/external-frontend-interop-phase5-component-interop-subspec-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 40 | 1 | 123 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/planning-meta/08-milestone-gate-definition-spec.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/05-anti-foot-gun-planning-standard.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/exception-register.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/09-exception-deferral-policy.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/07-task-catalog-authoring-spec.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/02-fast-llm-instruction-plan.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/01-master-planning-index.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/06-planning-taxonomy-glossary.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/03-weighted-deep-planning-manual.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/00-research-baseline-source-map.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/04-planning-critique-gap-analysis.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/10-document-maintenance-protocol.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/maintenance-log.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/planning-meta/12-question-gate-standard.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 81 | Orphaned page (0 inbound links) with 3 dead mentions, age 81 days |
| `docs/src/architecture/planning-meta/11-document-boundary-matrix.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/model-selection-2026-q2.md` | Architecture SSOTs | current | **Keep** | 1 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-cross-domain-methods-survey-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 0 | 2 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/skill-ecosystem-audit-2026-08-01.md` | Architecture SSOTs | current | **Archive** | 10 | 0 | 71 | Orphaned page (0 inbound links) with 10 dead mentions, age 71 days |
| `docs/src/architecture/scientia-micropublication-ssot-and-surfacing-design-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 7 | 1 | 116 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/search-retrieval-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 6 | 8 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-prior-art-and-vox-roadmap-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 4 | 2 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/frontend-convergence-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 27 | 6 | 50 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/agent-harness-testing-and-regression-gating-research-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 4 | 2 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/post-sprint-forward-plan-2026-05-25.md` | Architecture SSOTs | current | **Archive** | 37 | 0 | 81 | Orphaned page (0 inbound links) with 37 dead mentions, age 81 days |
| `docs/src/architecture/vox-language-rules-phase5-effects-determinism-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 25 | 3 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vuv-layered-layout-discipline-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/adr-042-vox-populi-types.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/json-as-rfc-2026-05-24.md` | Architecture SSOTs | approved | **Keep** | 2 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-fundamentals-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 3 | 5 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/orchestrator-companion-audit-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 17 | 2 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/populi-mesh-probe-correctness-plan-2026.md` | Architecture SSOTs | current | **Merge** | 12 | 4 | 72 | Member of topic cluster cluster-034 with overlapping content |
| `docs/src/architecture/deep-research-verification-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 4 | 2 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/tauri-audit-2026.md` | Architecture SSOTs | current | **Keep** | 6 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/generated-project-runtime-deps.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 20 | Active public reader documentation with valid references |
| `docs/src/architecture/vuv-naming-policy-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/discoverability-audit-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mesh-phase1-language-spine-plan-2026.md` | Architecture SSOTs | current | **Keep** | 41 | 5 | 49 | Active public reader documentation with valid references |
| `docs/src/architecture/ai-laziness-remediation-plan-2026.md` | Architecture SSOTs | current | **Archive** | 11 | 0 | 142 | Orphaned page (0 inbound links) with 11 dead mentions, age 142 days |
| `docs/src/architecture/cr-f2-arm-parity-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 2 | 0 | 129 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-native-scraping-scoping-2026-06-03.md` | Architecture SSOTs | current | **Keep** | 3 | 1 | 123 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-harness-parity-plan-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 1 | 8 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-mens-comparative-efficacy-benchmarking-research-2026-09-01.md` | Architecture SSOTs | research | **MoveToInternals** | 11 | 3 | 39 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/starlight-site-strategy-2026.md` | Architecture SSOTs | current | **Archive** | 25 | 0 | 50 | Orphaned page (0 inbound links) with 25 dead mentions, age 50 days |
| `docs/src/architecture/multi-agent-vcs-replication-spec-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 8 | 8 | 72 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ai-first-gap-analysis-2026-07-02.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 5 | 0 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/plugin-system-audit-2026-05-08.md` | Architecture SSOTs | research | **MoveToInternals** | 22 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ai-ui-generators-and-vox-as-target-research-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 3 | 2 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/mobile-target-evaluation-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 2 | 2 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/svelte-mineable-features-implementation-plan-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 20 | 4 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mens-metal-optimization-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 28 | Active public reader documentation with valid references |
| `docs/src/architecture/v1-release-criteria.md` | Architecture SSOTs | current | **Keep** | 14 | 6 | 127 | Active public reader documentation with valid references |
| `docs/src/architecture/agentic-secretary-research-2026-06-17.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/context-management-mechanics-research-2026-07-31.md` | Architecture SSOTs | current | **Keep** | 2 | 1 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/voxmens-hub-and-spoke-ssot-research-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 17 | 1 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-language-rules-phase4-runtime-monitors-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 28 | 6 | 34 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/distribution-ssot.md` | Architecture SSOTs | current | **Archive** | 7 | 0 | 38 | Orphaned page (0 inbound links) with 7 dead mentions, age 38 days |
| `docs/src/architecture/build-time-log.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 116 | Orphaned page (0 inbound links) with 4 dead mentions, age 116 days |
| `docs/src/architecture/codegen-ts-domain-boundary-and-cr-f2-correction-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 3 | 0 | 127 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/canonical-runtime-names.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 102 | Active public reader documentation with valid references |
| `docs/src/architecture/mesh-and-language-distribution-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 20 | 10 | 129 | Active public reader documentation with valid references |
| `docs/src/architecture/autonomous-orchestration-policy-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 6 | 1 | 16 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ffscript-panel-schema-spec-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/plugin-system-redesign-sp1-plan-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 31 | 4 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/deep-research-local-depth-and-power-research-2026.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 28 | Orphaned page (0 inbound links) with 5 dead mentions, age 28 days |
| `docs/src/architecture/git-concurrency-policy.md` | Architecture SSOTs | current | **Keep** | 5 | 3 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-15-orchestrator-tier-d-plan.md` | Architecture SSOTs | current | **Keep** | 24 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-domain-agnosticism-audit-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 6 | 3 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/gui-visual-ai-review.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 120 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-implementation-divergence-audit-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/repo-layout-single-file-parent-dirs-triage-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/agentic-vcs-automation-impl-plan-phase2-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 36 | 3 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/env-var-ssot-design-and-handoff-2026-06-15.md` | Architecture SSOTs | current | **Archive** | 20 | 0 | 119 | Orphaned page (0 inbound links) with 20 dead mentions, age 119 days |
| `docs/src/architecture/deep-research-competitive-landscape-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 0 | 3 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-08-llm-misleading-content-cleanup-plan.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 33 | 2 | 51 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/omni-search-audit-and-roadmap-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/kb-systems-sota-research-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/comprehensive-audit-v2-2026.md` | Architecture SSOTs | current | **Merge** | 3 | 1 | 81 | Member of topic cluster cluster-009 with overlapping content |
| `docs/src/architecture/boilerplate-reduction-design-brief-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 7 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mcp-vox-language-exposure.md` | Architecture SSOTs | current | **Merge** | 2 | 0 | 116 | Member of topic cluster cluster-012 with overlapping content |
| `docs/src/architecture/gui-frontend-design-principles-2026-06-14.md` | Architecture SSOTs | current | **Keep** | 4 | 2 | 120 | Active public reader documentation with valid references |
| `docs/src/architecture/scientia-self-publication-gap-map-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 4 | 3 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-efficacy-benchmark-adversarial-audit-2026-09-01.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 2 | 39 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/gui-operator-console-v2-handoff-2026-06-16.md` | Architecture SSOTs | current | **Keep** | 17 | 1 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/ai-fixtures-ts-lowering-follow-on-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 0 | 0 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-gui-harness-buildout-plan-2026.md` | Architecture SSOTs | current | **Keep** | 16 | 3 | 51 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-language-rules-phase3-typecheck-rules-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 19 | 2 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-gui-design-review-2026.md` | Architecture SSOTs | current | **Keep** | 20 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/ai-dev-loop-overhead-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-22-v1-readiness-snapshot.md` | Architecture SSOTs | current | **Archive** | 19 | 0 | 142 | Orphaned page (0 inbound links) with 19 dead mentions, age 142 days |
| `docs/src/architecture/tauri-convergence-migration-plan-2026.md` | Architecture SSOTs | current | **Keep** | 37 | 4 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/mesh-phase3-vcs-gossip-plan-2026.md` | Architecture SSOTs | current | **Keep** | 33 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/repo-layout-sprawl-audit-2026.md` | Architecture SSOTs | current | **Archive** | 9 | 0 | 81 | Orphaned page (0 inbound links) with 9 dead mentions, age 81 days |
| `docs/src/architecture/2026-05-08-naming-and-guards-design.md` | Architecture SSOTs | current | **Archive** | 18 | 0 | 142 | Orphaned page (0 inbound links) with 18 dead mentions, age 142 days |
| `docs/src/architecture/phase-numbering-index.md` | Architecture SSOTs | current | **Keep** | 0 | 8 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/feature-growth-boundaries.md` | Architecture SSOTs | current | **Merge** | 1 | 3 | 81 | Member of topic cluster cluster-014 with overlapping content |
| `docs/src/architecture/antigravity-credits-auth-and-limitations-2026-06-19.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 114 | Active public reader documentation with valid references |
| `docs/src/architecture/durability-runtime-audit-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 12 | 129 | Active public reader documentation with valid references |
| `docs/src/architecture/github-pages-redirects.md` | Architecture SSOTs | research | **MoveToInternals** | 3 | 0 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/gui-perf-conventions-2026-06-14.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 116 | Orphaned page (0 inbound links) with 3 dead mentions, age 116 days |
| `docs/src/architecture/deep-research-synthesis-and-priorities-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 0 | 3 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/gemini-3-5-flash-antigravity-limitations-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/toolchain-reality-and-omnibus-installer-findings-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/scientia-mesh-integration-research-2026.md` | Architecture SSOTs | current | **Keep** | 18 | 4 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/multi-agent-vcs-replication-research-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 5 | 128 | Active public reader documentation with valid references |
| `docs/src/architecture/fableforge-roadmap-audit-2026-04-23.md` | Architecture SSOTs | current | **Keep** | 1 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/adr-NNN-scope-tauri-desktop-only.md` | Architecture SSOTs | current | **Keep** | 4 | 1 | 51 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-08-crate-org-followup-plan.md` | Architecture SSOTs | current | **Archive** | 89 | 0 | 142 | Orphaned page (0 inbound links) with 89 dead mentions, age 142 days |
| `docs/src/architecture/vox-speech-audit-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 6 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/harness-research-adversarial-crosscheck-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/closures-rfc-2026-05-23.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 0 | 50 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/gamification-ludus-review-findings-2026.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 116 | Orphaned page (0 inbound links) with 4 dead mentions, age 116 days |
| `docs/src/architecture/2026-05-08-workspace-reorg-design.md` | Architecture SSOTs | current | **Keep** | 23 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/lora-adapter-serving-findings-2026.md` | Architecture SSOTs | current | **Archive** | 8 | 0 | 31 | Orphaned page (0 inbound links) with 8 dead mentions, age 31 days |
| `docs/src/architecture/populi-mesh-north-star-2026.md` | Architecture SSOTs | current | **Keep** | 5 | 12 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/model-orchestration-ssot-audit-2026.md` | Architecture SSOTs | current | **Keep** | 50 | 6 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/populi-mesh-probe-correctness-spec-2026.md` | Architecture SSOTs | current | **Merge** | 7 | 5 | 142 | Member of topic cluster cluster-034 with overlapping content |
| `docs/src/architecture/vox-lsp-capabilities-ssot-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 7 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/webir-hir-split-brain-inventory-2026.md` | Architecture SSOTs | current | **Keep** | 9 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/dependency-and-migration-handoff-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 14 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-harness-implementation-spec-2026-07-31.md` | Architecture SSOTs | current | **Keep** | 19 | 1 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/agent-chat-ux-and-noise-research-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 0 | 3 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/mens-loc-audit-2026-09.md` | Architecture SSOTs | research | **MoveToInternals** | 7 | 1 | 31 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/skill-ecosystem-interop-research-2026-06-12.md` | Architecture SSOTs | current | **Keep** | 9 | 2 | 122 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-language-rules-phase1-ssot-collapse-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 40 | 8 | 36 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/no-c-no-msvc-2026-05-08.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/claude-code-beloved-features-research-2026-06-07.md` | Architecture SSOTs | research | **MoveToInternals** | 13 | 0 | 126 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/2026-09-22-voxdb-turso-pooling-and-mvcc-recommendation.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 17 | Active public reader documentation with valid references |
| `docs/src/architecture/boilerplate-reduction-gap-analysis-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 75 | 3 | 51 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-gui-capability-audit-2026.md` | Architecture SSOTs | current | **Keep** | 7 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/http-runtime-extraction-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/semantic-coverage-remediation-plan-2026-06-13.md` | Architecture SSOTs | current | **Archive** | 11 | 0 | 116 | Orphaned page (0 inbound links) with 11 dead mentions, age 116 days |
| `docs/src/architecture/build-stabilization-findings-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/session-handoff-2026-05-28-state-audit.md` | Architecture SSOTs | current | **Archive** | 16 | 0 | 81 | Orphaned page (0 inbound links) with 16 dead mentions, age 81 days |
| `docs/src/architecture/dead-crate-deep-dive-2026-05-08.md` | Architecture SSOTs | research | **MoveToInternals** | 40 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/web-app-archetype-coverage-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 19 | 5 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/deep-research-system-audit-and-handoff-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 10 | 0 | 15 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/rust-build-loop-ssot-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 6 | 0 | 13 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/voxdb-turso-database-audit-and-benchmarks-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 4 | 0 | 15 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/deep-research-system-best-practices-research-2026-06-18.md` | Architecture SSOTs | research | **MoveToInternals** | 1 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-gui-browser-support-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 1 | 34 | Active public reader documentation with valid references |
| `docs/src/architecture/script-tier-timings-2026-09.md` | Architecture SSOTs | current | **Archive** | 15 | 0 | 35 | Orphaned page (0 inbound links) with 15 dead mentions, age 35 days |
| `docs/src/architecture/agentic-vcs-automation-impl-plan-phase3-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 23 | 2 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/2026-05-08-workspace-reorg-outcome.md` | Architecture SSOTs | current | **Keep** | 7 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-speech-surface-inventory-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 7 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/semantic-gap-audit-2026.md` | Architecture SSOTs | current | **Merge** | 1 | 1 | 123 | Member of topic cluster cluster-039 with overlapping content |
| `docs/src/architecture/telemetry-unification-design-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 7 | 5 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/automatic-gui-and-debugging-vox-design-2026-06-18.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 8 | 1 | 104 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/free-by-default-audit-2026-05-24.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 81 | Orphaned page (0 inbound links) with 3 dead mentions, age 81 days |
| `docs/src/architecture/ai-first-plan-1-language-toolchain-2026-07-02.md` | Architecture SSOTs | current | **Keep** | 7 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/mens-cloud-training-economics-2026-09-11.md` | Architecture SSOTs | research | **MoveToInternals** | 1 | 1 | 31 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/phase3-http-ergonomics-spec-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 2 | 6 | 50 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ai-first-fixtures-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 18 | 0 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/rust-warning-audit-backlog-2026.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 51 | Orphaned page (0 inbound links) with 5 dead mentions, age 51 days |
| `docs/src/architecture/scientia-self-publication-finalization-plan-2026.md` | Architecture SSOTs | approved | **Keep** | 22 | 3 | 129 | Active public reader documentation with valid references |
| `docs/src/architecture/ci-cd-audit-findings-2026.md` | CI & Quality | research | **MoveToInternals** | 39 | 0 | 16 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/harness-research-gap-fill-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 3 | 3 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/graphify-upstream-study-methodology-2026-06-18.md` | Architecture SSOTs | current | **Archive** | 10 | 0 | 116 | Orphaned page (0 inbound links) with 10 dead mentions, age 116 days |
| `docs/src/architecture/cryptography-ssot-2026.md` | Architecture SSOTs | current | **Merge** | 0 | 3 | 38 | Member of topic cluster cluster-020 with overlapping content |
| `docs/src/architecture/error-surfacing-dual-audience-research-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 0 | 4 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/mens-training-ssot.md` | Architecture SSOTs | current | **Merge** | 0 | 4 | 81 | Member of topic cluster cluster-022 with overlapping content |
| `docs/src/architecture/crate-classification-2026-05-08.md` | Architecture SSOTs | deprecated | **Archive** | 32 | 0 | 16 | Frontmatter status is 'deprecated' |
| `docs/src/architecture/vox-speech-ci-gates-proposal-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 2 | 0 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/chat-ui-model-knobs-audit-2026-09-10.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 26 | Active public reader documentation with valid references |
| `docs/src/architecture/session-handoff-2026-05-25-finalization-pass.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 81 | Orphaned page (0 inbound links) with 3 dead mentions, age 81 days |
| `docs/src/architecture/voxscript-portability-substrate-research-2026.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 37 | Orphaned page (0 inbound links) with 5 dead mentions, age 37 days |
| `docs/src/architecture/deep-research-self-correction-and-knowledgebase-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 28 | Active public reader documentation with valid references |
| `docs/src/architecture/gui-native-roadmap-status-2026.md` | Architecture SSOTs | current | **Merge** | 26 | 9 | 81 | Member of topic cluster cluster-040 with overlapping content |
| `docs/src/architecture/vox-playground-architecture-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 2 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/plugin-system-redesign-sp2-plan-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 33 | 2 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-language-rules-phase2-lint-extension-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 41 | 9 | 72 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/front-facing-honesty-audit-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 0 | 35 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/zero-copy-rust-emission-plan-2026.md` | Architecture SSOTs | current | **Keep** | 5 | 1 | 128 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-as-llm-target-audit-and-plan-2026.md` | Architecture SSOTs | current | **Keep** | 16 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/mobile-e2e-testing-strategy-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 1 | 141 | Active public reader documentation with valid references |
| `docs/src/architecture/v1-readiness-status-2026-07.md` | Architecture SSOTs | research | **MoveToInternals** | 9 | 0 | 82 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mens-program-status-and-requirements-audit-2026-09-12.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 31 | Active public reader documentation with valid references |
| `docs/src/architecture/ludus-adjudication-implementation-plan-2026.md` | Architecture SSOTs | current | **Keep** | 11 | 1 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/true-workflow-durability-design-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 9 | 0 | 28 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-gui-native-roadmap-2026.md` | Architecture SSOTs | current | **Merge** | 140 | 6 | 36 | Member of topic cluster cluster-040 with overlapping content |
| `docs/src/architecture/vox-dashboard-design-brief-2026.md` | Architecture SSOTs | current | **Keep** | 34 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/model-catalog-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 19 | 1 | 39 | Active public reader documentation with valid references |
| `docs/src/architecture/mens-cloud-env-transmission-fix-2026-09-12.md` | Architecture SSOTs | current | **Archive** | 9 | 0 | 31 | Orphaned page (0 inbound links) with 9 dead mentions, age 31 days |
| `docs/src/architecture/2026-05-09-detector-rule-ssot-design.md` | Architecture SSOTs | current | **Archive** | 7 | 0 | 142 | Orphaned page (0 inbound links) with 7 dead mentions, age 142 days |
| `docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 114 | Active public reader documentation with valid references |
| `docs/src/architecture/semantic-gap-implementation-plan-2026.md` | Architecture SSOTs | current | **Merge** | 8 | 1 | 123 | Member of topic cluster cluster-039 with overlapping content |
| `docs/src/architecture/eval-sandbox-deployment.md` | Architecture SSOTs | current | **Archive** | 6 | 0 | 87 | Orphaned page (0 inbound links) with 6 dead mentions, age 87 days |
| `docs/src/architecture/graphify-python-free-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 12 | 1 | 116 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/classification-ssot-2026.md` | Architecture SSOTs | current | **Merge** | 0 | 0 | 142 | Member of topic cluster cluster-023 with overlapping content |
| `docs/src/architecture/village-narrative-architecture-findings-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 114 | Active public reader documentation with valid references |
| `docs/src/architecture/unified-task-hopper-research-2026.md` | Architecture SSOTs | current | **Keep** | 10 | 10 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/agent-shell-fluency-eval-design-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/scientia-automated-research-handoff-2026-06-16.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 116 | Orphaned page (0 inbound links) with 3 dead mentions, age 116 days |
| `docs/src/architecture/v1-foundation-criteria-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 14 | 0 | 129 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mesh-phase0-foundations-plan-2026.md` | Architecture SSOTs | current | **Keep** | 24 | 3 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/burn-necessity-audit-2026-05-08.md` | Architecture SSOTs | deprecated | **Archive** | 8 | 2 | 128 | Frontmatter status is 'deprecated' |
| `docs/src/architecture/language-formatter-semantics-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 1 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/gui-observability-ssot-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 0 | 0 | 13 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/iroh-spike-findings-2026.md` | Architecture SSOTs | current | **Archive** | 8 | 0 | 38 | Orphaned page (0 inbound links) with 8 dead mentions, age 38 days |
| `docs/src/architecture/plugin-system-deep-audit-2026-05-08.md` | Architecture SSOTs | research | **MoveToInternals** | 14 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/populi-mesh-improvement-backlog-2026.md` | Architecture SSOTs | current | **Keep** | 23 | 7 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/build-and-crate-org-improvement-plan-2026-06.md` | Architecture SSOTs | current | **Archive** | 13 | 0 | 16 | Orphaned page (0 inbound links) with 13 dead mentions, age 16 days |
| `docs/src/architecture/turso-ownership-migration-handoff-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 0 | 128 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-memory-model-audit-and-value-optimization-2026-06-05.md` | Architecture SSOTs | current | **Keep** | 5 | 1 | 128 | Active public reader documentation with valid references |
| `docs/src/architecture/voxmens-serving-topology-decision-2026-06-19.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 114 | Active public reader documentation with valid references |
| `docs/src/architecture/plugin-system-redesign-sp3-plan-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 7 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ai-first-plan-2-gui-ia-reorg-2026-07-02.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 8 | 1 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/inference-tuning-resolution-research-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/plugin-system-redesign-sp4-plan-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 28 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/repo-cleanup-ledger-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-02-codebase-cleanup-and-signal-improvement.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 24 | 1 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mesh-phase2-code-mobility-plan-2026.md` | Architecture SSOTs | current | **Keep** | 26 | 2 | 129 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-container-vs-wasm-2026-05-08.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/test-inventory-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 15 | Active public reader documentation with valid references |
| `docs/src/architecture/external-frontend-interop-plan-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 6 | 15 | 20 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/language-lsp-parity-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/qwen-3.7-profile-and-mens-4b-feasibility-2026-06-07.md` | Architecture SSOTs | research | **MoveToInternals** | 3 | 0 | 126 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/data-storage-lint-and-ci-spec-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 46 | 1 | 16 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mens-corpus-audit-findings-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/data-storage-ssot-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 46 | 3 | 26 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/history/2026-04-core-ten-charter.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 142 | Orphaned page (0 inbound links) with 4 dead mentions, age 142 days |
| `docs/src/architecture/vox-language-syntax-audit-2026-08-08.md` | Architecture SSOTs | current | **Archive** | 9 | 0 | 64 | Orphaned page (0 inbound links) with 9 dead mentions, age 64 days |
| `docs/src/architecture/mobile-eas-ci-and-emulator-run-2026.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 81 | Orphaned page (0 inbound links) with 5 dead mentions, age 81 days |
| `docs/src/architecture/mens-fine-tuning-pipeline-audit-2026-09-20.md` | Architecture SSOTs | research | **MoveToInternals** | 16 | 1 | 20 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/language-diagnostic-drift-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 2 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/plugin-system-redesign-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 86 | 8 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/deep-research-model-agnostic-multi-provider-and-skills-publication-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 2 | 2 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/fableforge-impl/HANDOFF.md` | Architecture SSOTs | current | **Archive** | 6 | 0 | 142 | Orphaned page (0 inbound links) with 6 dead mentions, age 142 days |
| `docs/src/architecture/prompts/visus-audit-grounding.v1.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/wire-format-v1-ssot.md` | Architecture SSOTs | current | **Keep** | 0 | 8 | 50 | Active public reader documentation with valid references |
| `docs/src/architecture/voxmens-finetuning-boundaries-research-2026-06-21.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 113 | Active public reader documentation with valid references |
| `docs/src/architecture/skill-marketplace-security-and-provenance-research-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 1 | 5 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/phase1-build-targets-spec-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 16 | 5 | 104 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` | Architecture SSOTs | research | **MoveToInternals** | 4 | 0 | 15 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/code-search-omnisearch-research-2026-06-17.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/mental-tracker-full-app-roadmap-2026.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 81 | Orphaned page (0 inbound links) with 4 dead mentions, age 81 days |
| `docs/src/architecture/agent-instruction-platform-enhancements-and-token-waste-2026-06-18.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 116 | Orphaned page (0 inbound links) with 4 dead mentions, age 116 days |
| `docs/src/architecture/crate-restructuring-proposal-2026-07.md` | Architecture SSOTs | research | **MoveToInternals** | 12 | 0 | 101 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/v1-llm-target-implementation-plan-2026.md` | Architecture SSOTs | current | **Keep** | 4 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/skill-code-marketplace-research-and-audit-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 2 | 2 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/populi-mesh-local-observability-spec-2026.md` | Architecture SSOTs | current | **Keep** | 17 | 4 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/tooling-convergence-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 35 | 2 | 37 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/ludus-security-and-anti-cheat-research-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 3 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-trust-novelty-scoring-landscape-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 5 | 4 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/v0.5-core-ssot.md` | Architecture SSOTs | current | **Keep** | 1 | 3 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/crate-build-dependency-model-2026-06-19.md` | Architecture SSOTs | research | **MoveToInternals** | 6 | 0 | 114 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/free-tier-model-selection-and-onboarding-research-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 4 | 1 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/auto-debugging-zero-annotation-research-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 0 | 4 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/ffscript-linter-design-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/mobile-phase2-completion-scoping-2026-06-12.md` | Architecture SSOTs | experimental | **Archive** | 20 | 0 | 81 | Orphaned page (0 inbound links) with 20 dead mentions, age 81 days |
| `docs/src/architecture/cross-project-environment-variable-handoff-findings-2026-09-01.md` | Architecture SSOTs | research | **MoveToInternals** | 13 | 0 | 39 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/mesh-phase5-public-internet-plan-2026.md` | Architecture SSOTs | current | **Keep** | 32 | 2 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-08-01-voxdb-audit-condensation-plan.md` | Architecture SSOTs | current | **Keep** | 22 | 1 | 71 | Active public reader documentation with valid references |
| `docs/src/architecture/dependency-consolidation-plan-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 38 | 0 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/json-ergonomics-rfc-2026-05-23.md` | Architecture SSOTs | current | **Archive** | 9 | 0 | 81 | Orphaned page (0 inbound links) with 9 dead mentions, age 81 days |
| `docs/src/architecture/session-handoff-2026-05-25-state-of-the-work.md` | Architecture SSOTs | current | **Merge** | 1 | 0 | 81 | Member of topic cluster cluster-038 with overlapping content |
| `docs/src/architecture/vox-gui-surface-map-2026-06-14.md` | Architecture SSOTs | current | **Keep** | 5 | 1 | 120 | Active public reader documentation with valid references |
| `docs/src/architecture/claude-code-harness-mechanics-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 4 | 7 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/model-autonomic-system-2026.md` | Architecture SSOTs | current | **Keep** | 3 | 2 | 1 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-gui-ux-beautification-plan-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 23 | 0 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/contracts-non-indexed-classification-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-capabilities-audit-2026-06-17.md` | Architecture SSOTs | current | **Merge** | 11 | 2 | 72 | Member of topic cluster cluster-029 with overlapping content |
| `docs/src/architecture/dashboard-migration-research-2026.md` | Architecture SSOTs | current | **Keep** | 4 | 6 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/warp-research-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 13 | 0 | 36 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/research-scientia-telemetry-channels.md` | Architecture SSOTs | current | **Archive** | 5 | 0 | 142 | Orphaned page (0 inbound links) with 5 dead mentions, age 142 days |
| `docs/src/architecture/deep-research-stage2-synthesis-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-stdlib-gap-audit-2026-05-23.md` | Architecture SSOTs | current | **Archive** | 225 | 0 | 20 | Orphaned page (0 inbound links) with 225 dead mentions, age 20 days |
| `docs/src/architecture/model-routing-handoff-2026-10-11.md` | Architecture SSOTs | research | **MoveToInternals** | 10 | 0 | 0 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/os-agent-computer-use-and-accessibility-tree-research-2026.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 26 | Orphaned page (0 inbound links) with 4 dead mentions, age 26 days |
| `docs/src/architecture/nightly-builds-ssot.md` | Architecture SSOTs | current | **Keep** | 1 | 1 | 37 | Active public reader documentation with valid references |
| `docs/src/architecture/language-migration-friction-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 1 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/codegen-ssot-and-split-brain-audit-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 8 | 1 | 34 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-gui-scientia-coverage-audit-2026.md` | Architecture SSOTs | current | **Keep** | 3 | 2 | 129 | Active public reader documentation with valid references |
| `docs/src/architecture/agentic-vcs-automation-impl-plan-phase5-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 13 | 1 | 142 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/graphify-capabilities-audit-and-vertical-integration-2026-06-18.md` | Architecture SSOTs | current | **Archive** | 25 | 0 | 106 | Orphaned page (0 inbound links) with 25 dead mentions, age 106 days |
| `docs/src/architecture/combat-knowledge-graph-research-2026.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 114 | Orphaned page (0 inbound links) with 4 dead mentions, age 114 days |
| `docs/src/architecture/coding-agent-local-model-ux-comparison-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 4 | 4 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/ffscript-mutation-api-spec-2026.md` | Architecture SSOTs | current | **Keep** | 8 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/llm-target-reliability-audit-2026-08-31.md` | Architecture SSOTs | current | **Archive** | 8 | 0 | 42 | Orphaned page (0 inbound links) with 8 dead mentions, age 42 days |
| `docs/src/architecture/native-code-emission-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 0 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/chat-surface-design-critique-2026-09-28.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 0 | 1 | 14 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/vox-harness-graph-audit-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 8 | 7 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/adr-043-quantized-safetensors-ondisk-format.md` | Architecture SSOTs | current | **Keep** | 1 | 1 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/multi-repository-workspace-support-spec-2026-07-31.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 9 | 0 | 72 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/share-policy-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vcs-as-vox-language-feature-jujutsu-2026.md` | Architecture SSOTs | current | **Archive** | 4 | 0 | 128 | Orphaned page (0 inbound links) with 4 dead mentions, age 128 days |
| `docs/src/architecture/svelte-vs-react-frameworks-research-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 5 | 2 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/zero-copy-codegen-findings-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 1 | 0 | 142 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/cli-gui-surface-coverage-map-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 2 | 0 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/agentic-vcs-automation-impl-plan-phase4-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 23 | 1 | 129 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 27 | Active public reader documentation with valid references |
| `docs/src/architecture/populi-mesh-config-baseline-spec-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 3 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-marquee-explainer-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/deep-research-gui-representation-design-2026-08-01.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 36 | Active public reader documentation with valid references |
| `docs/src/architecture/measurements/2026-09-11-candle-metal-calibration.md` | Architecture SSOTs | current | **Keep** | 0 | 1 | 31 | Active public reader documentation with valid references |
| `docs/src/architecture/semantic-coverage-status-2026-06-15.md` | Architecture SSOTs | current | **Keep** | 6 | 1 | 119 | Active public reader documentation with valid references |
| `docs/src/architecture/2026-05-01-orchestrator-stage1-stage2.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 15 | 0 | 81 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/state-unification-plan-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/plugin-registration-resolution-2026.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/graphify-gui-health-surface-2026-06-25.md` | Architecture SSOTs | current | **Archive** | 3 | 0 | 109 | Orphaned page (0 inbound links) with 3 dead mentions, age 109 days |
| `docs/src/architecture/auto-gui-from-pure-logic-research-2026-06-18.md` | Architecture SSOTs | current | **Keep** | 0 | 4 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/nextgen-orchestrator-research-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 9 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/free-by-default-and-residual-work-plan-2026.md` | Architecture SSOTs | current | **Keep** | 24 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-runtime-rn-mobile-cross-compile.md` | Architecture SSOTs | current | **Keep** | 1 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/crate-structure-audit-2026-05-15.md` | Architecture SSOTs | current | **Keep** | 82 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/vox-diagnostic-ux-ssot-2026.md` | Architecture SSOTs | research | **MoveToInternals** | 1 | 4 | 81 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/multi-agent-vcs-replication-impl-plan-phase1-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 47 | 4 | 36 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/2026-05-08-crate-org-followup-design.md` | Architecture SSOTs | current | **Keep** | 93 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/suite-status-audit-2026-09-21.md` | Architecture SSOTs | current | **Archive** | 9 | 0 | 21 | Orphaned page (0 inbound links) with 9 dead mentions, age 21 days |
| `docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md` | Architecture SSOTs | current | **Archive** | 28 | 0 | 31 | Orphaned page (0 inbound links) with 28 dead mentions, age 31 days |
| `docs/src/architecture/internal-web-ir-implementation-blueprint.md` | Architecture SSOTs | current | **Merge** | 0 | 2 | 142 | Member of topic cluster cluster-026 with overlapping content |
| `docs/src/architecture/skill-discovery-and-induction-research-2026-07-30.md` | Architecture SSOTs | current | **Keep** | 0 | 6 | 72 | Active public reader documentation with valid references |
| `docs/src/architecture/gamified-programming-wellness-research-2026.md` | Architecture SSOTs | current | **Keep** | 1 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/architecture/cross-platform-ssot-2026.md` | Architecture SSOTs | current | **Keep** | 2 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/architecture/superpowers-ssot.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/architecture/pipeline-parity-ssot-2026-06-14.md` | Architecture SSOTs | current | **Archive** | 9 | 0 | 120 | Orphaned page (0 inbound links) with 9 dead mentions, age 120 days |
| `docs/src/architecture/2026-08-01-voxdb-audit-condensation-design.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 12 | 1 | 71 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/intra-project-imports-rfc-2026-05-23.md` | Architecture SSOTs | research | **MoveToInternals** | 6 | 0 | 51 | Frontmatter status is 'research', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/architecture/research-trial-tier-a-schema-2026.md` | Architecture SSOTs | roadmap | **MoveToInternals** | 3 | 0 | 0 | Frontmatter status is 'roadmap', grouped into Internals sidebar per MEASURE-06 |
| `docs/src/examples/golden.md` | Examples | current | **Archive** | 4 | 0 | 51 | Orphaned page (0 inbound links) with 4 dead mentions, age 51 days |
| `docs/src/ref-cli.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/api/DOC_GAPS.md` | API Reference — Crates | current | **Keep** | 2 | 0 | 49 | Active public reader documentation with valid references |
| `docs/src/api/vox-codegen-ts.md` | API Reference — Crates | deprecated | **Archive** | 3 | 0 | 123 | Frontmatter status is 'deprecated' |
| `docs/src/tutorials/tut-actor-basics.md` | Tutorials | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/tutorials/use-a-react-component-from-vox.md` | Tutorials | current | **Archive** | 4 | 0 | 123 | Orphaned page (0 inbound links) with 4 dead mentions, age 123 days |
| `docs/src/tutorials/tut-workflow-durability.md` | Tutorials | current | **Keep** | 0 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/tutorials/tut-first-app.md` | Tutorials | current | **Keep** | 1 | 4 | 50 | Active public reader documentation with valid references |
| `docs/src/tutorials/tut-ui-integration.md` | Tutorials | current | **Keep** | 2 | 0 | 49 | Active public reader documentation with valid references |
| `docs/src/tutorials/tut-first-vox-app-checkpoints.md` | Tutorials | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/tutorials/tut-getting-started.md` | Tutorials | current | **Keep** | 2 | 1 | 35 | Active public reader documentation with valid references |
| `docs/src/journeys/ai-agents.md` | Getting Started | current | **Keep** | 1 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/journeys/durable-workflows.md` | Getting Started | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/journeys/native-training.md` | Getting Started | current | **Keep** | 1 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/journeys/fullstack-data.md` | Getting Started | current | **Keep** | 1 | 0 | 80 | Active public reader documentation with valid references |
| `docs/src/AGENTS.md` | Architecture SSOTs | current | **Keep** | 0 | 0 | 36 | Active public reader documentation with valid references |
| `docs/src/reference/ref-type-system.md` | Language Reference | current | **Keep** | 0 | 0 | 49 | Active public reader documentation with valid references |
| `docs/src/reference/orchestration-unified.md` | Language Reference | current | **Keep** | 15 | 9 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/cli-json-output.md` | Language Reference | current | **Keep** | 0 | 0 | 101 | Active public reader documentation with valid references |
| `docs/src/reference/mens-cloud-gpu.md` | Language Reference | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mcp-tool-registry-contract.md` | Language Reference | current | **Keep** | 4 | 3 | 16 | Active public reader documentation with valid references |
| `docs/src/reference/secrets-ssot.md` | Language Reference | current | **Keep** | 10 | 5 | 0 | Active public reader documentation with valid references |
| `docs/src/reference/rust-ecosystem-support-contract.md` | Language Reference | current | **Archive** | 5 | 0 | 81 | Orphaned page (0 inbound links) with 5 dead mentions, age 81 days |
| `docs/src/reference/scientia-publication-worthiness-rules.md` | Language Reference | current | **Archive** | 5 | 0 | 81 | Orphaned page (0 inbound links) with 5 dead mentions, age 81 days |
| `docs/src/reference/mens-training-data-contract.md` | Language Reference | current | **Keep** | 1 | 3 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/deployment-compose.md` | Language Reference | current | **Merge** | 6 | 8 | 16 | Member of topic cluster cluster-025 with overlapping content |
| `docs/src/reference/speech-capture-architecture.md` | Language Reference | current | **Keep** | 7 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/repo-reconstruction-benchmark-ladder.md` | Language Reference | current | **Keep** | 0 | 1 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/vox-ir-specification.md` | Language Reference | current | **Keep** | 1 | 0 | 34 | Active public reader documentation with valid references |
| `docs/src/reference/plugin-manifest.md` | Language Reference | current | **Keep** | 10 | 1 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/mens-serving-ssot.md` | Language Reference | current | **Keep** | 4 | 2 | 30 | Active public reader documentation with valid references |
| `docs/src/reference/ref-literals.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/speech-to-code-operations.md` | Language Reference | current | **Keep** | 1 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mcp-http-gateway-contract.md` | Language Reference | current | **Keep** | 1 | 3 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/migration-0.5-to-0.6.md` | Language Reference | current | **Keep** | 2 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/reference/vox-db-language-surface.md` | Language Reference | current | **Keep** | 0 | 0 | 80 | Active public reader documentation with valid references |
| `docs/src/reference/ref-stdlib-index.md` | Language Reference | current | **Archive** | 6 | 0 | 36 | Orphaned page (0 inbound links) with 6 dead mentions, age 36 days |
| `docs/src/reference/syntax-k-complexity.md` | Language Reference | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/cli.md` | Language Reference | current | **Keep** | 134 | 9 | 15 | Active public reader documentation with valid references |
| `docs/src/reference/changelog.md` | Language Reference | current | **Archive** | 87 | 0 | 51 | Orphaned page (0 inbound links) with 87 dead mentions, age 51 days |
| `docs/src/reference/ai-ide-feature-matrix.md` | Language Reference | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/gui-navigation.md` | Language Reference | current | **Archive** | 4 | 0 | 96 | Orphaned page (0 inbound links) with 4 dead mentions, age 96 days |
| `docs/src/reference/ref-decorators.md` | Language Reference | current | **Keep** | 1 | 2 | 49 | Active public reader documentation with valid references |
| `docs/src/reference/ref-builtins-hashing.md` | Language Reference | current | **Archive** | 3 | 0 | 50 | Orphaned page (0 inbound links) with 3 dead mentions, age 50 days |
| `docs/src/reference/agent-quick-reference.md` | Language Reference | current | **Keep** | 0 | 0 | 72 | Active public reader documentation with valid references |
| `docs/src/reference/ref-async-concurrency.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/vox-web-stack.md` | Language Reference | current | **Keep** | 15 | 2 | 17 | Active public reader documentation with valid references |
| `docs/src/reference/information-theoretic-questioning.md` | Language Reference | current | **Keep** | 3 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/communication-protocols.md` | Language Reference | current | **Keep** | 0 | 2 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/diagnostic-taxonomy.md` | Language Reference | current | **Keep** | 0 | 0 | 80 | Active public reader documentation with valid references |
| `docs/src/reference/feature-builds.md` | Language Reference | current | **Keep** | 2 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/vscode-mcp-compat.md` | Language Reference | current | **Archive** | 8 | 0 | 16 | Orphaned page (0 inbound links) with 8 dead mentions, age 16 days |
| `docs/src/reference/repo-root-files.md` | Language Reference | current | **Keep** | 1 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/hf-finetune-gap-matrix.md` | Language Reference | current | **Keep** | 4 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/cli-command-surface.generated.md` | Language Reference | current | **Keep** | 0 | 2 | 1 | Active public reader documentation with valid references |
| `docs/src/reference/vox-system-prompt.md` | Language Reference | current | **Keep** | 0 | 0 | 35 | Active public reader documentation with valid references |
| `docs/src/reference/mens-external-tech-options.md` | Language Reference | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/ref-operator-precedence.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/ref-package-manager.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mobile-edge-ai.md` | Language Reference | current | **Keep** | 5 | 7 | 34 | Active public reader documentation with valid references |
| `docs/src/reference/mens-strategy-inputs-checklist.md` | Language Reference | current | **Archive** | 7 | 0 | 81 | Orphaned page (0 inbound links) with 7 dead mentions, age 81 days |
| `docs/src/reference/ref-web-model.md` | Language Reference | current | **Keep** | 1 | 0 | 50 | Active public reader documentation with valid references |
| `docs/src/reference/external-repositories.md` | Language Reference | current | **Keep** | 13 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/populi.md` | Language Reference | current | **Keep** | 14 | 10 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/language-ergonomics-principles.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/ref-doc-comments.md` | Language Reference | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/plugin-catalog.generated.md` | Language Reference | current | **Merge** | 35 | 1 | 16 | Member of topic cluster cluster-042 with overlapping content |
| `docs/src/reference/mens-training.md` | Language Reference | current | **Keep** | 59 | 11 | 27 | Active public reader documentation with valid references |
| `docs/src/reference/docker-image-baselines.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/hitl-and-doubt.md` | Language Reference | current | **Merge** | 0 | 1 | 81 | Member of topic cluster cluster-013 with overlapping content |
| `docs/src/reference/ref-diagnostic-id-policy.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mens-lora-ownership.md` | Language Reference | current | **Keep** | 2 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mens-quality-scoring.md` | Language Reference | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/parser-feature-matrix.md` | Language Reference | current | **Keep** | 0 | 0 | 80 | Active public reader documentation with valid references |
| `docs/src/reference/codex-http-api.md` | Language Reference | current | **Keep** | 2 | 2 | 50 | Active public reader documentation with valid references |
| `docs/src/reference/review-regression-challenges-contract.md` | Language Reference | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/std-surfaces.md` | Language Reference | current | **Archive** | 4 | 0 | 81 | Orphaned page (0 inbound links) with 4 dead mentions, age 81 days |
| `docs/src/reference/scientia-publication-playbook.md` | Language Reference | current | **Keep** | 2 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/build-profiles.md` | Language Reference | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/parser-ambiguity-inventory.md` | Language Reference | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/distribution-bundles.md` | Language Reference | current | **Merge** | 9 | 1 | 142 | Member of topic cluster cluster-043 with overlapping content |
| `docs/src/reference/isolation.md` | Language Reference | current | **Archive** | 12 | 0 | 34 | Orphaned page (0 inbound links) with 12 dead mentions, age 34 days |
| `docs/src/reference/stability.md` | Language Reference | current | **Keep** | 0 | 1 | 35 | Active public reader documentation with valid references |
| `docs/src/reference/telemetry-metric-contract.md` | Language Reference | current | **Keep** | 46 | 1 | 135 | Active public reader documentation with valid references |
| `docs/src/reference/populi-coordination.md` | Language Reference | current | **Keep** | 3 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/review-antipattern-catalog-contract.md` | Language Reference | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/env-vars.md` | Language Reference | current | **Merge** | 44 | 13 | 16 | Member of topic cluster cluster-001 with overlapping content |
| `docs/src/reference/review-fix-pairs-contract.md` | Language Reference | current | **Keep** | 0 | 0 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/mens-qlora-data-strategy.md` | Language Reference | current | **Keep** | 1 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/frontend-surface-ownership.md` | Language Reference | current | **Keep** | 0 | 1 | 20 | Active public reader documentation with valid references |
| `docs/src/reference/hir-legacy-inventory.md` | Language Reference | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/plugin-catalog.md` | Language Reference | current | **Merge** | 1 | 1 | 142 | Member of topic cluster cluster-042 with overlapping content |
| `docs/src/reference/ref-stdlib.md` | Language Reference | current | **Merge** | 0 | 0 | 81 | Member of topic cluster cluster-041 with overlapping content |
| `docs/src/reference/webhook_integration.md` | Language Reference | current | **Keep** | 1 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mcp-tool-reference.md` | Language Reference | legacy | **Archive** | 0 | 0 | 81 | Frontmatter status is 'legacy' |
| `docs/src/reference/ref-db-surface.md` | Language Reference | current | **Keep** | 0 | 2 | 50 | Active public reader documentation with valid references |
| `docs/src/reference/mcp-http-read-role-governance-contract.md` | Language Reference | current | **Keep** | 0 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/ref-syntax.md` | Language Reference | current | **Keep** | 0 | 1 | 50 | Active public reader documentation with valid references |
| `docs/src/reference/mens-laziness-accuracy-audit.md` | Language Reference | current | **Keep** | 1 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/ref-search.md` | Language Reference | current | **Keep** | 2 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/vox-fullstack-artifacts.md` | Language Reference | current | **Keep** | 8 | 2 | 80 | Active public reader documentation with valid references |
| `docs/src/reference/doc-inventory.md` | Language Reference | current | **Keep** | 2 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mens-measurement-gap-analysis.md` | Language Reference | current | **Keep** | 2 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/cli-vox-deploy.md` | Language Reference | current | **Archive** | 5 | 0 | 81 | Orphaned page (0 inbound links) with 5 dead mentions, age 81 days |
| `docs/src/reference/command-compliance.md` | Language Reference | current | **Merge** | 3 | 2 | 81 | Member of topic cluster cluster-028 with overlapping content |
| `docs/src/reference/distribution-bundles.generated.md` | Language Reference | current | **Merge** | 31 | 2 | 116 | Member of topic cluster cluster-043 with overlapping content |
| `docs/src/reference/socrates-protocol.md` | Language Reference | current | **Keep** | 7 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/tavily-integration-ssot.md` | Language Reference | current | **Archive** | 4 | 0 | 81 | Orphaned page (0 inbound links) with 4 dead mentions, age 81 days |
| `docs/src/reference/scientia-ssot-handbook.md` | Language Reference | current | **Archive** | 16 | 0 | 81 | Orphaned page (0 inbound links) with 16 dead mentions, age 81 days |
| `docs/src/reference/ref-builtins-stdlib.md` | Language Reference | current | **Archive** | 9 | 0 | 15 | Orphaned page (0 inbound links) with 9 dead mentions, age 15 days |
| `docs/src/reference/ref-ffi.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/pm-migration-2026.md` | Language Reference | current | **Keep** | 4 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/speech-to-code-pipeline.md` | Language Reference | current | **Keep** | 4 | 4 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/installation.md` | Getting Started | current | **Archive** | 4 | 0 | 21 | Orphaned page (0 inbound links) with 4 dead mentions, age 21 days |
| `docs/src/reference/openclaw-discovery-sidecar-ssot.md` | Language Reference | current | **Keep** | 3 | 2 | 142 | Active public reader documentation with valid references |
| `docs/src/reference/style-guide.md` | Language Reference | current | **Keep** | 0 | 0 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/mens-speech-curriculum.md` | Language Reference | current | **Keep** | 0 | 1 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/token-system.md` | Language Reference | current | **Archive** | 8 | 0 | 50 | Orphaned page (0 inbound links) with 8 dead mentions, age 50 days |
| `docs/src/reference/shipped-v0.4.md` | Language Reference | current | **Keep** | 0 | 0 | 80 | Active public reader documentation with valid references |
| `docs/src/reference/populi-work-type-placement-matrix.md` | Language Reference | current | **Keep** | 0 | 4 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/skill_marketplace.md` | Language Reference | current | **Keep** | 1 | 0 | 116 | Active public reader documentation with valid references |
| `docs/src/reference/vox-portability-ssot.md` | Language Reference | current | **Keep** | 10 | 3 | 81 | Active public reader documentation with valid references |
| `docs/src/reference/oratio-speech.md` | Language Reference | current | **Keep** | 17 | 3 | 16 | Active public reader documentation with valid references |
| `docs/src/reference/mens-train-defaults.generated.md` | Language Reference | current | **Keep** | 0 | 1 | 112 | Active public reader documentation with valid references |
