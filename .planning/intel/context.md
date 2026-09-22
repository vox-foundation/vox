# Context

Synthesized from DOC classifications (research, architecture background, operational notes). Topic-keyed by document title, sorted alphabetically. Each entry is the classifier's extracted summary with source attribution -- treat as a pointer into the source doc, not a full restatement.

## Agent browser driver research (2026)
- source: docs/src/architecture/agent-browser-driver-research-2026.md
- summary: Audit of Vox's CDP/Tauri browser stack vs Stagehand/Claude Code/ChatGPT Atlas/Playwright/agent-browser; recommends finishing in-tree driver (snapshot+refs, named profiles, Chrome attach).
- scope: chromiumoxide, browser automation, CDP, Stagehand, Playwright, Claude Desktop, competitive analysis, browser driver architecture

## Agent Instruction Files: Per-Platform Enhancements + Token-Waste & Commit-Churn Audit (June 2026)
- source: docs/src/architecture/agent-instruction-platform-enhancements-and-token-waste-2026-06-18.md
- summary: Audit findings on instruction-file capabilities across agent platforms, token-waste opportunities, and commit-message-vs-diff drift patterns.
- scope: agent platforms, instruction file formats, token waste, generated artifacts, commit hygiene, context caching

## Agent Shell Fluency Eval Design (2026)
- source: docs/src/architecture/agent-shell-fluency-eval-design-2026.md
- summary: Optional A/B evaluation design for testing shell command generation accuracy in PowerShell vs. Bash; not currently run or shipped.
- scope: agent evaluation, shell command generation, PowerShell, Bash, A/B testing design, task-based eval

## Agentic VCS Automation — Phase 1 Implementation Plan (2026-05-08)
- source: docs/src/architecture/agentic-vcs-automation-impl-plan-phase1-2026.md
- summary: TDD implementation roadmap for agentic VCS Phase 1: capability types, workspace binding, git exec wrapper, secret scanning, commit/branch MCP tools.
- scope: VCS automation, capability types, git wrapper, MCP tools, secret scanning, commit trailers, branch management

## Agentic VCS Automation — Phase 2 Implementation Plan (2026-05-09)
- source: docs/src/architecture/agentic-vcs-automation-impl-plan-phase2-2026.md
- summary: TDD implementation roadmap for agentic VCS Phase 2: push/PR write-side, capability tokens, four MCP tools, arch-check enforcement, .vox glue scripts.
- scope: VCS automation, push workflow, PR automation, capability ledger, destructive operations, arch-check rules, VoxScript glue

## Agentic VCS Automation — Phase 3 Implementation Plan (2026-05-09)
- source: docs/src/architecture/agentic-vcs-automation-impl-plan-phase3-2026.md
- summary: TDD implementation roadmap for agentic VCS Phase 3: dashboard surface with five panels, API routes, WebSocket telemetry tap, state legibility.
- scope: VCS dashboard, API routes, panel visualization, branch board, oplog viewer, push queue, capability ledger display, secret scanning UI, telemetry streaming

## Agentic VCS Automation — Phase 4 Implementation Plan (2026-05-09)
- source: docs/src/architecture/agentic-vcs-automation-impl-plan-phase4-2026.md
- summary: TDD implementation roadmap for agentic VCS Phase 4: Vox-language @vcs.* decorators with compiler enforcement, effect tracking, linear capability consumption.
- scope: Vox language support, VCS decorators, @vcs.read_only, @vcs.requires, @vcs.linear_working_tree, @vcs.audit_trail, compiler type checking, capability linearity, effect propagation

## Agentic VCS Automation — Phase 5 Implementation Plan (2026-05-09)
- source: docs/src/architecture/agentic-vcs-automation-impl-plan-phase5-2026.md
- summary: Step-by-step TDD plan for swapping GitExec backend from tokio::process to gix for hot-path git operations while keeping shell-out for low-frequency operations.
- scope: GitExec, gix backend, hot-path git operations, Rust implementation, TDD plan

## Agentic Version Control Automation — Failure Modes, jj Footguns, and a Vox-Language Capability Proposal (2026-05-08)
- source: docs/src/architecture/agentic-version-control-automation-research-2026.md
- summary: Research documenting agent VCS failure modes, where Jujutsu helps/hurts, and proposal for capability-typed VCS effects at the Vox language layer.
- scope: agent VCS failures, version control automation, Jujutsu footguns, VCS capabilities, agent safety

## AI Fixtures TS Lowering Follow-on (2026)
- source: docs/src/architecture/ai-fixtures-ts-lowering-follow-on-2026.md
- summary: Follow-on plan for implementing TypeScript target lowering for AI fixture variants to achieve parity with Rust codegen.
- scope: TypeScript codegen, AI fixtures, fixture lowering, language parity, TS runtime shims

## AI UI Generators & Vox-as-Target Strategy
- source: docs/src/architecture/ai-ui-generators-and-vox-as-target-research-2026-06-18.md
- summary: Research on how modern AI UI generators work and strategy for making Vox an ideal target via MCP, component registry, and compile-time guarantees.
- scope: AI UI generators, v0.dev, Claude Design, MCP integration, component registry, design tokens, contrast/occlusion/a11y enforcement

## AI-assisted dev loop — compile overhead (2026)
- source: docs/src/architecture/ai-dev-loop-overhead-2026.md
- summary: Evidence-backed practices and tooling to reduce redundant Cargo rebuilds when using coding agents across multi-terminal and pre-push workflows.
- scope: Cargo rebuild optimization, dev loop practices, compile overhead, cache fragmentation, AI-assisted development

## AI-Authorship Language/Toolchain Gaps Implementation Plan
- source: docs/src/architecture/ai-first-plan-1-language-toolchain-2026-07-02.md
- summary: Closes G-L1 (@ai structured_output JSON-schema body), G-T1 (uniform --json build-lane envelope), G-T7 (vox doctor --diag filter) with task-by-task implementation.
- scope: @ai structured_output, JSON schema emission, build-lane JSON envelope, vox doctor filtering, language toolchain, implementation plan

## AI-First Gap Analysis: Language, CLI, and GUI (Journey-Spine)
- source: docs/src/architecture/ai-first-gap-analysis-2026-07-02.md
- summary: Adversarially audited gap analysis scoring Vox against AI-as-author target state across Intent/Authorship/Verification/Operation/Comprehension journey with verified claims.
- scope: gap analysis, AI-as-author, journey spine, language semantics, CLI toolchain, GUI design, verification stage

## AI-First Language Fixtures — Research (2026)
- source: docs/src/architecture/ai-first-fixtures-research-2026.md
- summary: Evidence-grounded taxonomy and catalog for AI-first fixtures across Vox compiler, codegen, actor LLM runtime, orchestrator, MCP, ACI, and telemetry.
- scope: AI fixtures, language semantics, compiler decorators, codegen, orchestrator policy, AI-first language, fixture catalog

## AI-Laziness Audit — Findings & Pivot (2026-05-16)
- source: docs/src/architecture/ai-laziness-remediation-plan-2026.md
- summary: Audit-of-audit documenting findings from a laziness investigation: 10 retirement candidates verified, 4 tracks evaluated, lessons captured.
- scope: audit findings, Phase 1 retirements, telemetry trace propagation, vox-code-audit stubs, MENS Batch 3

## Architecture Decision Records (ADR)
- source: docs/src/adr/index.md
- summary: Official documentation for ADRs with comprehensive table of all ADRs from 001-048.
- scope: ADR reference, architecture decisions

## Architecture Decision Records (index)
- source: docs/src/adr/README.md
- summary: Entrypoint mirror for ADR cross-links used by CI doc gates.
- scope: ADR index, documentation, architecture

## Automatic GUIs from Pure Logic — Feasibility Research
- source: docs/src/architecture/auto-gui-from-pure-logic-research-2026-06-18.md
- summary: Research on deriving UIs from program types: Naked Objects (production-proven DSFA 2002), react-jsonschema-form, type-driven generation mechanical, intent/affordance gap limit.
- scope: auto-GUI, Naked Objects, type-driven generation, react-jsonschema-form, intent/affordance gap, VUV typed tokens, Vox readiness audit

## Autonomous Orchestration Policy — Decision-Rule Research for Agent-to-Agent Behavior Switching (2026)
- source: docs/src/architecture/autonomous-orchestration-policy-research-2026.md
- summary: Research synthesis mapping external state-of-the-art model routing, planning, confidence fusion, and escalation onto Vox's orchestrator surface; proposes skeleton decision-rule contracts for the gaps.
- scope: orchestrator, model routing, planning mode, confidence fusion, HITL escalation, sub-agent dispatch

## Beyond Crossref/OpenAlex: Cross-Domain Research Methods Survey (2026-08-01)
- source: docs/src/architecture/deep-research-cross-domain-methods-survey-2026-08-01.md
- summary: Survey of 11 concrete research-validation techniques from legal, medical, historical, journalistic, and consumer domains to augment academic-only trust scoring.
- scope: Research validation methods, Cross-domain techniques, Trust scoring, Citation validity graphs, Evidence tiers, Corroboration counting, Domain reputation, Source evaluation

## Boilerplate Reduction in Modern Full-Stack & Mobile Development — Ranked Design Brief for Vox (2026)
- source: docs/src/architecture/boilerplate-reduction-design-brief-2026.md
- summary: Externally-sourced design brief ranking 25 categories of repetitive scaffolding by frequency/burden/leverage; reviewer caveats emphasize it is research input, not roadmap.
- scope: async-state, cross-stack types, forms, auth, effects, validation, state sync, i18n, routing, observability, durable jobs

## Boilerplate Reduction — Gap Analysis Against Vox's Existing Backlog (2026)
- source: docs/src/architecture/boilerplate-reduction-gap-analysis-2026.md
- summary: Code-audited reconciliation of 25 boilerplate categories against Vox's actual implementation state; provides 5-tier verdicts (Shipped/Spec'd/Partially-Built/No-Spec/Out-of-Scope) and graft task blocks.
- scope: async-state, forms, auth, effects, routes, LLM integration, durable functions, vector search, CRDTs

## Build Orchestration Stabilization Findings (2026-04-21)
- source: docs/src/architecture/build-stabilization-findings-2026.md
- summary: Findings and fixes from April 2026 stabilization sweep of Vox orchestration build system; records five problems (F1–F5) and resolutions per category.
- scope: schema hardening, serving layer, feature gates, cloud provider traits, CLI tests, orchestration

## Build-Time Baseline (2026-05-08)
- source: docs/src/architecture/build-time-baseline.md
- summary: Phase 0 baseline build-time measurements (incremental and cold scenarios) for 2026-05-08 workspace reorg; documents methodology and target goals.
- scope: orchestrator, CLI, build-time, baseline measurement

## Build-Time Log
- source: docs/src/architecture/build-time-log.md
- summary: Per-phase build-time measurements for workspace reorg; tracks scenarios (orchestrator, CLI, L0 leaf) across phases 0–9 and headline outcome summary.
- scope: build-time, phases, measurements, scenarios

## Build-Time, Crate-Org & Target-Sprawl Improvement Plan (2026-06)
- source: docs/src/architecture/build-and-crate-org-improvement-plan-2026-06.md
- summary: Synthesis of 7-lens audit of workspace (build-time, crate layers, dead/combine crates, feature gates, plugin boundaries, target sprawl). Records quick wins landed and gated workstreams with open decisions.
- scope: build-time, crate-organization, target-folder, feature gates, plugin boundaries, dependency volume

## Burn Framework Necessity Audit (2026-05-08)
- source: docs/src/architecture/burn-necessity-audit-2026-05-08.md
- summary: Audit of Burn framework usage in Vox codebase; production fine-tuning uses Candle, Burn is legacy dogfood + 4,526 LOC behind feature flag. Proposes three options (delete/plugin/status-quo) with recommendation.
- scope: Burn, Candle, fine-tuning, ML frameworks, GPU training

## C3 — vox-cli-ci extraction plan (2026-05-15)
- source: docs/src/architecture/2026-05-15-cli-ci-extraction-plan.md
- summary: Plan for extracting vox-cli/src/commands/ci/ into a new vox-cli-ci crate via six sequential TDD tasks with shared-module rearchitecture.
- scope: vox-cli, vox-cli-ci, vox-cli-core, crate extraction, architecture refactoring

## candle-metal calibration: one real point, no fit yet
- source: docs/src/architecture/measurements/2026-09-11-candle-metal-calibration.md
- summary: Records single Qwen3-0.6B Metal peak-memory measurement (53.4GB at batch=2×seq=512), explains why this point cannot be fitted into a model, surfaces batch-size silencing bug.
- scope: Metal memory measurement, calibration data, candle-metal performance, Qwen3-0.6B, batch-size override

## Canonical runtime names (daemon, MCP, env)
- source: docs/src/architecture/canonical-runtime-names.md
- summary: Frozen canonical identifiers vs deprecated aliases for daemon binary, environment variables, crate names, and MCP tool prefixes; prevents split-brain naming across CLI/MCP/docs/contracts.
- scope: orchestrator-d, vox-gamify, VOX_DB_*, vox-openclaw-runtime, Vox Axis

## Chat UI & Model-Knob Audit (2026-09-10)
- source: docs/src/architecture/chat-ui-model-knobs-audit-2026-09-10.md
- summary: Inventory of chat composer, model-priority knobs, budget display, and mesh/online status in vox-gui; second-pass corrections flag wiring gaps and proposes five decomposed work items (bounded/architectural/policy).
- scope: chat UI, model knobs, budget display, mesh status, composer, routing priority

## Claude Code Harness Mechanics — Verified Research (2026-07-30)
- source: docs/src/architecture/claude-code-harness-mechanics-2026-07-30.md
- summary: Adversarially verified research on Claude Code internals: agentic loop, context compaction, skills, tools, permissions, hooks, subagents.
- scope: Claude Code, harness internals, agentic loop, context management, permissions pipeline

## Claude Code's Most-Beloved Programming Features — Research & Vox Adoption Audit
- source: docs/src/architecture/claude-code-beloved-features-research-2026-06-07.md
- summary: Web research on Claude Code features, practitioner sentiment, and gap audit against Vox orchestrator and MENS pipeline.
- scope: Claude Code, harness architecture, agent features, Vox adoption, MENS training

## CLI→GUI Surface Coverage Map 2026
- source: docs/src/architecture/cli-gui-surface-coverage-map-2026.md
- summary: Per-surface gap audit matrix of CLI vs GUI representation, with association thesis linking Scientia, Ludus, and other surfaces.
- scope: CLI surfaces, GUI coverage, Scientia, Ludus, action manifest

## Cloud Env Transmission Fix
- source: docs/src/architecture/mens-cloud-env-transmission-fix-2026-09-12.md
- summary: Bug fix narrative: batch_size/seq_len were not transmitted from local dispatch to cloud workers; fixed at dispatch, provider, and entrypoint layers with tests.
- scope: cloud training, batch_size transmission, seq_len transmission, vast.ai, RunPod, docker entrypoint

## Code Search & Omni-Search Research (2026-06-17)
- source: docs/src/architecture/code-search-omnisearch-research-2026-06-17.md
- summary: Web research findings on 2026 best practices for code search, hybrid retrieval, omni-search UX, and agent-facing search patterns.
- scope: code search, hybrid retrieval, omni-search, AST indexing, RRF fusion

## Codebase Cleanup & Signal Improvement Plan (2026-05-02)
- source: docs/src/architecture/2026-05-02-codebase-cleanup-and-signal-improvement.md
- summary: Six-phase plan to retire stale code, fix broken references, and converge on single sources of truth.
- scope: code-cleanup, signal-improvement, documentation, references, golden-examples, @island, retired-surfaces

## Codegen SSOT & split-brain audit (2026)
- source: docs/src/architecture/codegen-ssot-and-split-brain-audit-2026.md
- summary: Advisory audit of multi-target codegen pipeline, minimum emission set for web/Tauri/RN, with tier-ranked findings on duplication.
- scope: codegen, IRs, split-brain, web emit, Tauri, React Native

## Comprehensive Vox audit and improvement plan (April 2026, v2)
- source: docs/src/architecture/comprehensive-audit-v2-2026.md
- summary: Full-spectrum audit identifying five structural problems; reshaped v1.0 plan (weeks 1-6) targeting solo-dev-to-production workflow.
- scope: Vox compiler, project scope, v1.0 criteria, improvement plan

## CR-F2 Arm-Parity Reality (2026-06-05)
- source: docs/src/architecture/cr-f2-arm-parity-findings-2026.md
- summary: Measured cross-arm parity: interpreter 10/10 main()-goldens pass; codegen-rust 3/10; categorized 7-item repair backlog for script execution.
- scope: compiler arms, codegen-rust, interpreter, arm parity, script execution

## Crate Build-Time × Dependency Model (2026-06-19)
- source: docs/src/architecture/crate-build-dependency-model-2026-06-19.md
- summary: Measured workspace model: blast-radius-seconds metrics, 20-crate advisory dependency cycle, modularity Q=0.24, layer inversions, semantic community structure.
- scope: workspace dependencies, build time, blast radius, modularity, crate structure

## Crate Classification Audit (2026-05-08)
- source: docs/src/architecture/crate-classification-2026-05-08.md
- summary: DEPRECATED historical snapshot of crate classifications (CORE/PLUGIN/SHARED/DEAD/MISPLACED) from 2026-05-08; many labels stale; do not use for architecture decisions.
- scope: crate classification, workspace organization, historical reference

## Crate Consolidation & Dependency Currency Audit (2026-06-12)
- source: docs/src/architecture/crate-consolidation-and-dependency-currency-audit-2026-06-12.md
- summary: Verified audit of deletable LoC via unused/underused workspace crates, dependency upgrades, and executed quick wins with prioritized backlog.
- scope: crate consolidation, dependency management, LoC cleanup, workspace dependencies, dependency currency

## Crate Org Follow-up — Implementation Plan
- source: docs/src/architecture/2026-05-08-crate-org-followup-plan.md
- summary: Step-by-step implementation plan to land 6 PRs from the crate-org-followup design specification.
- scope: crate-organization, implementation, SSOT, naming, descriptions, vox-mcp-meta, vox-package-types, vox-cli-ci, vox-orchestrator-core

## Crate Restructuring Proposal — 2026-07
- source: docs/src/architecture/crate-restructuring-proposal-2026-07.md
- summary: Research findings from crate-graph build-time program demonstrating symbol-graph limitations; rebuild hygiene findings and ranked structural signals.
- scope: crate restructuring, dependency analysis, build-time optimization, symbol graph

## Crate structure audit & consolidation plan (2026-05-15)
- source: docs/src/architecture/crate-structure-audit-2026-05-15.md
- summary: Audit of post-reorg crate sprawl, drift between sources, LoC distribution, consolidation bands, and arch-check rules to prevent regression.
- scope: crate structure, layer model, LoC distribution, consolidation planning, arch-check rules

## Cross-Platform Guarantees: Audit + Dynamic Enforcement Plan
- source: docs/src/architecture/cross-platform-guarantees-audit-and-enforcement-2026-06-15.md
- summary: Audit of Windows/Linux/macOS compilation status, CI/CD gaps, two confirmed builds breaks, and phased enforcement plan for 3-OS matrix.
- scope: cross-platform compatibility, Windows/Linux/macOS, CI enforcement, portability auditing, defect analysis

## Cross-project environment variable handoff
- source: docs/src/architecture/cross-project-environment-variable-handoff-findings-2026-09-01.md
- summary: September 2026 catalog of declared environment-variable names across Vox, FableForge, and GigMe projects with security boundaries.
- scope: environment variables, cross-project configuration, secret management, configuration catalog

## Dashboard Migration Research 2026
- source: docs/src/architecture/dashboard-migration-research-2026.md
- summary: Research on standalone Axum dashboard architecture; superseded by ADR-024. Documents decisions, transport, auth, build, rejected alternatives.
- scope: dashboard architecture, Axum SPA, React components, browser access, orchestration UI

## Data Storage Migration Backlog (2026)
- source: docs/src/architecture/data-storage-migration-backlog-2026.md
- summary: Phased migration backlog with execution order, blockers, sub-steps, verification criteria for eight phases of data storage work.
- scope: data storage migration, backlog, migration phases, tickets, execution order

## Dead Crate Deep Dive (2026-05-08)
- source: docs/src/architecture/dead-crate-deep-dive-2026-05-08.md
- summary: Code-level investigation of eight uncertain workspace crates to determine uniqueness, preservation vs. deletion, and migration strategy for each.
- scope: vox-tools, vox-mcp-meta, vox-mcp-registry, vox-scientia-core, vox-scientia-social, vox-scientia-ingest, vox-ssg, vox-webhook, crate analysis

## Decentralized Skill + Code Marketplace — Research & Codebase Audit
- source: docs/src/architecture/skill-code-marketplace-research-and-audit-2026-06-18.md
- summary: Grounding research and codebase audit for a local-first marketplace mining Vox code, deduping against MCP tools.
- scope: skill marketplace, skill registry, MCP tools, code deduplication, agentic review, sandbox policy, vox-container

## Deep Research Capabilities Audit — Verification Pass (2026-08-01)
- source: docs/src/architecture/deep-research-verification-2026-08-01.md
- summary: Re-verifies 9 deep-research gaps against current code with file:line evidence; resolves G4 and G5, partially addresses G1/G3/G6, G2/G7/G8/G9 still open.
- scope: deep research gaps, CRAG expansion, reranking, novelty scoring, confidence gate, synthesis budget, semantic caching

## Deep Research Competitive Landscape & User Sentiment (2026-08-01)
- source: docs/src/architecture/deep-research-competitive-landscape-2026-08-01.md
- summary: Product-by-product competitive research on 8+ major deep-research tools (Google, Claude, OpenAI, Perplexity, Elicit, You.com, Grok, Manus) with disclosed capabilities, user sentiment, and identified competitive gap.
- scope: Competitive analysis, Deep Research market, Citation verification, Verifiable trust, User sentiment, Product capabilities, Research benchmarks

## Deep Research Domain-Agnosticism Audit (2026-08-01)
- source: docs/src/architecture/deep-research-domain-agnosticism-audit-2026-08-01.md
- summary: Code-level audit of deep-research pipeline for academic/code/technical bias; identifies 4 live biases with file:line citations and recommended fixes.
- scope: Deep Research bias audit, Domain neutrality, web_dispatcher ranking, ANTI_LAZINESS_RIDER, trust scoring, code-level analysis

## Deep Research Enhancement Program — Synthesis & Priorities (2026-08-01)
- source: docs/src/architecture/deep-research-synthesis-and-priorities-2026-08-01.md
- summary: Synthesizes research docs into impact-ranked gap list with recommended differentiation strategy and implementation handoff.
- scope: synthesis, deep research, differentiation strategy, implementation priorities, gap analysis

## Deep Research Full-Surface Audit, Architecture, and Roadmap (September 2026)
- source: docs/src/architecture/deep-research-full-surface-audit-and-roadmap-2026.md
- summary: End-to-end audit of deep research from retrieval and model cascades through storage, chat, and GUI; identifies critical bugs and provides 4-phase remediation roadmap.
- scope: Deep Research architecture, GUI bugs, Chat integration, Model orchestration, Storage and persistence, Shopping research, Code generation research, Full-surface audit

## Deep Research GUI Representation Design (2026-08-01)
- source: docs/src/architecture/deep-research-gui-representation-design-2026-08-01.md
- summary: GUI design for surfacing trust, citation, novelty, and verification signals in ResearchView using established component patterns.
- scope: GUI design, trust signals, citation indicators, claim verification, research interface

## Deep Research Implementation Divergence Audit (2026-08-01)
- source: docs/src/architecture/deep-research-implementation-divergence-audit-2026-08-01.md
- summary: Audit comparing Stage 1 plans against shipped code, finding integration surface gaps rather than algorithmic problems.
- scope: implementation audit, deep research pipeline, trust/novelty scoring, multi-provider routing

## Deep Research Local Depth, Structure, and Power Expansion (2026)
- source: docs/src/architecture/deep-research-local-depth-and-power-research-2026.md
- summary: Architectural roadmap for scaling Vox Deep Research locally via autonomous crawling, local compute, and epistemic graph reasoning.
- scope: deep research architecture, local compute, autonomous crawling, multi-language sandboxing, knowledge graphs, retrieval, MENS acceleration

## Deep Research Prior Art and Vox Integration Roadmap (2026)
- source: docs/src/architecture/deep-research-prior-art-and-vox-roadmap-2026.md
- summary: Maps external deep-research agents (Gemini, OpenClaw, Claude, Tavily) to Vox pipeline and sequences implementation across phases.
- scope: deep research systems, prior art analysis, Gemini Deep Research, OpenClaw, implementation roadmap

## Deep Research Stage 2 Synthesis & Priorities (2026-08-01)
- source: docs/src/architecture/deep-research-stage2-synthesis-2026-08-01.md
- summary: Synthesizes Stage 2 audit docs into prioritized gap list for deep-research enhancement.
- scope: synthesis, deep research, priorities, gap analysis, stage 2

## Deep Research System Audit, CLI/GUI Parity, and Agent Handoff (2026)
- source: docs/src/architecture/deep-research-system-audit-and-handoff-2026.md
- summary: Empirical audit of research pipeline failures on complex queries and CLI/GUI feature parity analysis with root causes.
- scope: audit, deep research, CLI/GUI parity, pipeline failures, empirical evaluation

## Deep Research System Best Practices — Free-Tier-First, Tavily-95%-Slice Research
- source: docs/src/architecture/deep-research-system-best-practices-research-2026-06-18.md
- summary: Verified findings on building free-tier-first deep research pipeline replicating ~95% of Tavily using SearXNG, local extraction, RRF, and OpenRouter cascade.
- scope: deep research, Tavily, SearXNG, DuckDuckGo, OpenRouter, web extraction, Vox infrastructure

## Deep Research Systems: A From-First-Principles Technical Reference
- source: docs/src/architecture/deep-research-fundamentals-2026-08-01.md
- summary: Technical reference mapping deep-research architecture patterns to Vox implementation across planning, retrieval, verification, and evaluation.
- scope: deep research systems, query planning, hybrid retrieval, CRAG loops, claim verification, novelty scoring, synthesis, confidence gating, evaluation

## Deep Research Waves, Batches, and Competitive SOTA Analysis (September 2026)
- source: docs/src/architecture/deep-research-waves-and-competitive-analysis-research-2026.md
- summary: Architectural comparison of Vox, Gemini, and Claude deep research; proposes multi-wave DAG engine, dynamic triage, context distillation, and batch orchestration.
- scope: deep research, multi-wave execution, Google Gemini, Anthropic Claude, context distillation, adversarial verification, dynamic triage

## Deep Research: Multi-Provider Model Routing, GUI Key Wiring, Skills Packaging & Publication Handoff (2026-08-01)
- source: docs/src/architecture/deep-research-model-agnostic-multi-provider-and-skills-publication-2026-08-01.md
- summary: Code audit of multi-provider routing split-brain plus free-tier API survey and skills/publication design considerations.
- scope: multi-provider routing, model selection, LLM APIs, skills packaging, publication handoff, key gating

## Dependency & Migration Handoff (2026)
- source: docs/src/architecture/dependency-and-migration-handoff-2026.md
- summary: Verified codebase handoff for deferred dependency upgrades: rmcp, wasmtime, swc, cargo_metadata, sysinfo, typify, jsonschema, thiserror, candle; each with blast radius and cost analysis.
- scope: dependency upgrades, rmcp, wasmtime, swc, cargo_metadata, candle, blast radius, difficulty ratings

## Dependency consolidation plan (2026)
- source: docs/src/architecture/dependency-consolidation-plan-2026.md
- summary: Draft plan for unified Rust installer extending vox doctor; audits current externals and proposes phased vox doctor --install migration.
- scope: installer design, vox doctor, tool consolidation, PATH setup, cross-platform, vox-install-policy

## Design Hygiene for Auto-Derived UI and Observability
- source: docs/src/architecture/auto-derivation-design-hygiene-2026-06-18.md
- summary: Cross-cutting design principles for auto-GUI and auto-debugging: opt-in, structure-vs-semantics, selective-by-default, advise-not-gate, escape hatches.
- scope: design principles, auto-derivation, auto-GUI, observability, K-complexity discipline, escape hatches

## Detector & Heuristic Rule SSOT — Implementation Plan
- source: docs/src/architecture/2026-05-09-detector-rule-ssot-plan.md
- summary: Step-by-step implementation plan for rule-SSOT foundation: vox-rule-pack crate, pilot victory_claim migration, benchmark tool, arch-check guards.
- scope: detector-rules, implementation, rule-pack, victory-claim, benchmark-tool, arch-check, YAML-schema, vox-rule-pack

## Document boundary matrix
- source: docs/src/architecture/planning-meta/11-document-boundary-matrix.md
- summary: Ownership and boundary matrix for planning-meta documents defining scope and cross-document separation.
- scope: document boundaries, ownership matrix, document scope, planning-meta, overlap test

## Durability & Scheduling Runtime Audit (2026)
- source: docs/src/architecture/durability-runtime-audit-2026.md
- summary: Definitive audit of @scheduled, @durable, DurabilityKind, actor/workflow/activity keywords; verdict: zero runtime implementation across all features.
- scope: @scheduled, @durable, DurabilityKind, actor keyword, workflow keyword, activity keyword, vox-orchestrator, runtime semantics

## Eval sandbox deployment (Coolify)
- source: docs/src/architecture/eval-sandbox-deployment.md
- summary: Operational deployment guide for the vox mcp HTTP eval gateway at eval.voxlang.org, covering Docker, DNS, Coolify config, and recovery procedures.
- scope: eval gateway, Coolify, Docker, deployment, vox mcp

## External Frontend Interop Plan (2026)
- source: docs/src/architecture/external-frontend-interop-plan-2026.md
- summary: Five-phase strategic plan for React/TS ecosystem bidirectional interop: backend-only mode, component imports/exports, @island retirement, auth library, and backend-only deployments.
- scope: frontend interop, React ecosystem, backend-only mode, component interop, wire format, auth, @island retirement

## FableForge as any Casts Reduction Strategy
- source: docs/src/architecture/as-any-casts-reduction-strategy-research-2026.md
- summary: Systemic strategy for auditing, classifying, and eliminating ~8,871 TypeScript as-any casts across FableForge monorepo via five sequential sweeps.
- scope: TypeScript type safety, as-any cast reduction, FableForge, Convex migration, Zod schema alignment, mock builders

## FableForge Combat Knowledge Graph
- source: docs/src/architecture/combat-knowledge-graph-research-2026.md
- summary: Directed knowledge graph (4,838 nodes, 13,683 edges) spanning Dystopia/FableForge combat engine layers from legacy C to TypeScript.
- scope: combat engine, knowledge graph, legacy C, TypeScript, C-to-TS migration

## FableForge Roadmap Audit — 2026-04-23
- source: docs/src/architecture/fableforge-roadmap-audit-2026-04-23.md
- summary: Document-level audit of 280-task roadmap: internal consistency errors, redundancies, mis-prioritizations, prunable tasks, re-ranked top-30 execution list.
- scope: FableForge roadmap, audit, task prioritization, execution planning

## FableForge — Developer Handoff Brief
- source: docs/src/architecture/fableforge-impl/HANDOFF.md
- summary: Handoff brief for FableForge implementation: task verification results, PR sequence (1–6), implementation files mapping, and state after planning.
- scope: FableForge, implementation handoff, task verification, PR sequence

## Fast LLM instruction plan
- source: docs/src/architecture/planning-meta/02-fast-llm-instruction-plan.md
- summary: Compact deterministic instruction set for creating planning artifacts quickly without implementation leakage or ambiguity.
- scope: planning instructions, LLM planning, deterministic ladder, stop conditions, anti-foot-gun checks

## Free-by-Default & Residual Work Plan (2026-05-24)
- source: docs/src/architecture/free-by-default-and-residual-work-plan-2026.md
- summary: Forward-looking plan capturing residual work tracks (F-A through F-I) to activate free-tier infrastructure and complete the audit.
- scope: free-by-default product directive, model-routing configuration, tier-D orchestrator plan, selection module re-activation

## Free-by-Default Audit (2026-05-24)
- source: docs/src/architecture/free-by-default-audit-2026-05-24.md
- summary: Post-implementation audit of 89 free_only/is_free call sites after F-F sprint landed ModelTier::Free + Fast variants.
- scope: free_only call sites, is_free filtering, model tier variants, cost preference defaults, routing profile

## Free-Tier Model Selection & Onboarding — Audit, Research, and Risk-Reviewed Design (2026-08-01)
- source: docs/src/architecture/free-tier-model-selection-and-onboarding-research-2026-08-01.md
- summary: Live-code audit of model-selection engine, external research on registries and competitor UX, adversarial pre-mortem of recommendations, and design with failure-mode controls.
- scope: model-selection engine, free-tier handling, model-rating registries, competitor onboarding UX, OAuth desktop-security, budget enforcement

## Front-facing honesty audit (2026)
- source: docs/src/architecture/front-facing-honesty-audit-2026.md
- summary: Calibrated audit of marketing claims versus shipped code; verdicts on claim calibration and Track A remediations to improve accuracy.
- scope: marketing claims audit, claim calibration verdicts, homepage/playground accuracy, llms.txt authority, contributor hub CTA

## Frontend Convergence Findings (2026)
- source: docs/src/architecture/frontend-convergence-findings-2026.md
- summary: Audit of TypeScript/JSX/React emit pipeline and GUI primitives; identifies dead surfaces, names canonical SSOT (HIR → Web IR → emitters), and specifies Contract IR convergence layer.
- scope: TypeScript/JSX/React emit, GUI primitives, HIR lowering, frontend interop, Contract IR, Zod/OpenAPI emit

## Gamified Programming & Wellness Research (2026)
- source: docs/src/architecture/gamified-programming-wellness-research-2026.md
- summary: Research synthesis on developer wellness metrics, intentional-friction anti-addiction loops, cost-thrift gamification, and AI-driven SVG asset marketplaces.
- scope: developer wellness, intentional friction, self-determination theory, burnout prevention, cost-conscious coding, SVG asset generation, peer-to-peer art marketplace

## Gap-Fill: Aider/Zed/Cursor Local-Model Mechanisms (2026-07-30)
- source: docs/src/architecture/harness-research-gap-fill-2026-07-30.md
- summary: Verified research filling tool-comparison gaps: Aider's ollama_chat/ prefix, Zed's language_models config, Cursor's OpenAI-compatible-endpoint workaround; workflow synthesis recovered from raw votes.
- scope: local model support, Aider configuration, Zed editor, Cursor IDE, Ollama integration, LM Studio, hardware capability

## Gemini 3.5 Flash & Google Antigravity — Limitations and Execution Constraints
- source: docs/src/architecture/gemini-3-5-flash-antigravity-limitations-2026-06-18.md
- summary: Reference profile of Gemini 3.5 Flash in Antigravity: capability gaps, documented failure modes, customization surfaces, and plan-engineering constraints for autonomous work.
- scope: Gemini 3.5 Flash model, Google Antigravity IDE, model capability profile, reliability failure modes, plan engineering constraints, subagent execution

## Generated project runtime deps
- source: docs/src/architecture/generated-project-runtime-deps.md
- summary: Technical design for how vox build-generated Cargo.toml locates vox-owned runtime crates outside checkout via three-tier path resolution.
- scope: generated Cargo.toml, runtime crate dependencies, path resolution, VOX_REPO_ROOT env var, Axum shell, Tauri shell

## Gradio & Streamlit Research (2026): What VUV Should Steal, Adapt, and Reject
- source: docs/src/architecture/gradio-streamlit-research-2026.md
- summary: Comparative audit of Gradio and Streamlit architectures, design decisions, and lessons for Vox's VUV authoring layer.
- scope: Gradio, Streamlit, UI authoring, LLM-friendly design, GUI libraries, Python frameworks, VUV

## Graphify Capabilities Audit & Vertical-Integration SSOT (2026-06-18)
- source: docs/src/architecture/graphify-capabilities-audit-and-vertical-integration-2026-06-18.md
- summary: Audit of Graphify usage, cache lifecycle status (BUILT/PLANNED/GAP), freshness model, and convergence recommendations.
- scope: Graphify, code graphs, caching, freshness, cache lifecycle, MCP tools, retention policy

## Graphify duplicate-corpus bytes findings (2026-09)
- source: docs/src/architecture/graphify-duplicate-corpus-bytes-findings-2026.md
- summary: Investigates byte-identical corpora claim: finds two (not three) are identical, analyzes root cause in extraction-mode branching.
- scope: Graphify, graph corpora, extraction modes, deduplication, disk footprint, graph building

## Graphify GUI Corpus-Health Surface (2026-06-25)
- source: docs/src/architecture/graphify-gui-health-surface-2026-06-25.md
- summary: Documents landed corpus-health cards in vox-gui for freshness status, rebuild affordance; explains rationale vs. full graph renderer.
- scope: GUI component, corpus health, freshness indicator, Tauri, MCP tools, React, GraphifyStatusPanel

## Graphify Integration Research (2026-06-16)
- source: docs/src/architecture/graphify-integration-research-2026-06-16.md
- summary: Audit of Vox's Graphify usage, upstream architecture, Rust-native feasibility, and proposed agent-search integration plan.
- scope: Graphify, integration, Rust feasibility, MCP, search retrieval, cache freshness, phased delivery

## Graphify Python-Free Transition Findings (2026)
- source: docs/src/architecture/graphify-python-free-findings-2026.md
- summary: Audit of Python dependencies in Graphify, hybrid-architecture limitations, and 4-phase roadmap to Rust-native implementation.
- scope: Python dependencies, Graphify, Rust porting, Leiden clustering, AST extraction, cache management, tree-sitter, syn

## Graphify Upstream Source — Study & Port-Parity Methodology (2026-06-18)
- source: docs/src/architecture/graphify-upstream-study-methodology-2026-06-18.md
- summary: Methodology guide for studying upstream Graphify source and verifying port parity stage-by-stage via corpus queries and reference maps.
- scope: Graphify, upstream source, methodology, port parity, stage mapping, corpus study, MCP tools

## GUI & Front-End Design Principles for Tauri + TypeScript (vox-gui)
- source: docs/src/architecture/gui-frontend-design-principles-2026-06-14.md
- summary: Verified catalog of 300+ GUI design principles spanning UX, accessibility, performance, components, and Tauri specifics with primary-source citation.
- scope: UX heuristics, visual design, accessibility, WCAG, performance, RAIL budgets, Tauri architecture, design systems

## GUI IA Intent-First Reorg Implementation Plan
- source: docs/src/architecture/ai-first-plan-2-gui-ia-reorg-2026-07-02.md
- summary: Executes GUI blueprint: reorganizes nav around intent-first priorities (Direct/Review/Agents/Knowledge), consolidates surfaces, merges/renames components while maintaining deep-link compatibility.
- scope: GUI navigation, information architecture, intent-first design, surface consolidation, legacy alias resolution, React/TypeScript implementation

## GUI Intuitiveness Implementation Plan
- source: docs/src/architecture/ai-first-plan-3-gui-intuitiveness-2026-07-02.md
- summary: Burns honesty debt: wires needs-you nav, SubAgents to real list_subagent_tree command, hides dead controls; collapses attention polls into one hook and adds structured intent panel.
- scope: GUI honesty, needs-you inbox, SubAgents tree, attention polling, intent composition, React hooks, Tauri IPC

## GUI Operator Console v2 — agent handoff (2026-06-16)
- source: docs/src/architecture/gui-operator-console-v2-handoff-2026-06-16.md
- summary: Fresh-agent onboarding context for v2 operator console: what shipped, invariants, verification state, remaining work, and common pitfalls.
- scope: operator console, configurable dashboard, OmniSearch, chat integration, gamification, persistence, Playwright testing, TypeScript architecture

## GUI Visual AI Adversarial Review
- source: docs/src/architecture/gui-visual-ai-review.md
- summary: Cache-driven, non-gating AI review process for Playwright GUI screenshots against design principles; never fails CI.
- scope: visual regression testing, Playwright, design principles, AI review, screenshot cache, vision models

## GUI-Native Language Roadmap — Execution Status
- source: docs/src/architecture/gui-native-roadmap-status-2026.md
- summary: Live execution status overlay tracking task completion across nine implementation phases with completion verdicts and audit log.
- scope: roadmap tracking, phase status, task completion, compiler primitives, GUI authoring, validation, routing

## Handoff: 2026-05-24 lost-work forensic audit + recovery plan
- source: docs/src/architecture/session-handoff-2026-05-24-lost-work-audit.md
- summary: Post-incident audit of orphaned work during parallel-agent commit storm; recovery commands and discipline forward.
- scope: handoff, incident-audit, recovery-plan, jj-snapshot, CEA3089, parallel-agents

## Handoff: evening continuation (2026-05-28)
- source: docs/src/architecture/session-handoff-2026-05-28-evening.md
- summary: Evening audit completion with 8-item action plan results, coverage metrics, outstanding items.
- scope: v0.6.0 state, test coverage, telemetry blind spots, coverage regression, pre-push gate

## Handoff: finalization pass (2026-05-25)
- source: docs/src/architecture/session-handoff-2026-05-25-finalization-pass.md
- summary: Post-recovery audit and repair pass; what was fixed, what remains pending, and v0.6.0 release criteria status.
- scope: handoff, finalization, v0.6.0, test-fixes, acceptance-suite, next-session-priorities

## Handoff: state audit (2026-05-28)
- source: docs/src/architecture/session-handoff-2026-05-28-state-audit.md
- summary: Post-v0.6.0 state audit capturing 78 commits over three days, outstanding work, uncommitted files.
- scope: v0.6.0 release, Phase H endpoint retirement, PR #90, workspace state, test coverage

## Handoff: state of the work (2026-05-25)
- source: docs/src/architecture/session-handoff-2026-05-25-state-of-the-work.md
- summary: Post-recovery audit of work state, task completion status, and corrections to prior claims.
- scope: work state, task completion, Phase M, Phase H, v0.6.0 preparation

## Harness Research Adversarial Cross-Check (2026-07-30)
- source: docs/src/architecture/harness-research-adversarial-crosscheck-2026-07-30.md
- summary: Cross-document consistency check and re-verification of high-stakes claims across eight research documents; confirms no contradictions.
- scope: research verification, coding-agent misalignment, skill promotion, MCP registry, tool comparison, methodological audit

## How Coding Agents Expose Model Selection & Local Models — Verified Research 2026-07-30
- source: docs/src/architecture/coding-agent-local-model-ux-comparison-2026-07-30.md
- summary: Verified comparison of Claude Code, Aider, Continue.dev model-selection schemas and local-model support; no hardware detection found.
- scope: model selection, local models, Ollama, LM Studio, agent UX

## HTTP runtime extraction — ADR-041 §6(c) deferral & design
- source: docs/src/architecture/http-runtime-extraction-2026.md
- summary: Design rationale explaining why HTTP runtime extraction is multi-week refactor; records design constraints for when it is scheduled.
- scope: HTTP runtime, route codegen, ADR-041, emit_main_boot, Axum server, route registration

## Implementation Plan: State Unification & TS Hardening (2026)
- source: docs/src/architecture/state-unification-plan-2026.md
- summary: Four-phase plan to unify structural state emission across Native and Web targets, eliminating split-plane issues in state machines and component state.
- scope: state unification, React/TypeScript, state machines, component state, codegen

## Implementation Plan: Zero-Copy Vox Codegen
- source: docs/src/architecture/zero-copy-rust-emission-plan-2026.md
- summary: Phased implementation plan to transition to zero-copy Rust backend: HIR type enrichment, escape analysis, native UI.
- scope: codegen, Rust emission, implementation plan, ownership tracking, escape analysis, WASM offloading

## iroh transport spike — measured findings
- source: docs/src/architecture/iroh-spike-findings-2026.md
- summary: Measured experimental results validating iroh's zero third-party contact, LAN connectivity, byte counters, and Windows firewall requirements.
- scope: iroh transport, mesh networking, QUIC, LAN discovery, platform firewall requirements, performance measurement

## Knowledge Base Systems — State of the Art (2026)
- source: docs/src/architecture/kb-systems-sota-research-2026.md
- summary: Research synthesis of persistent KB architectures, routing, chunking, retrieval, dedup, staleness handling, and failure modes for AI tools.
- scope: knowledge base systems, memory architecture, RAG, retrieval, embeddings, semantic memory, persistent wikis

## Language diagnostic drift — findings (2026)
- source: docs/src/architecture/language-diagnostic-drift-findings-2026.md
- summary: Probe findings on duplicate/conflicting diagnostics across vox-compiler, LSP, and code-audit; hypotheses and recommendations for diagnostic stabilization.
- scope: compiler diagnostics, LSP validation, code audit, diagnostic drift, error message consistency

## Language formatter semantics — findings (2026)
- source: docs/src/architecture/language-formatter-semantics-findings-2026.md
- summary: Probe findings on formatter-induced span instability and semantic risk; identifies parity gaps between LSP and CLI formatting behavior.
- scope: formatter behavior, semantic preservation, span stability, LSP formatting, vox fmt policy

## Language LSP parity — findings (2026)
- source: docs/src/architecture/language-lsp-parity-findings-2026.md
- summary: Probe comparing author needs against vox-lsp capabilities, identifying gaps in completion, hover, goto-def across archetypes.
- scope: LSP, language server, IDE features, code completion, hover, goto-def

## Language migration friction — findings (2026)
- source: docs/src/architecture/language-migration-friction-findings-2026.md
- summary: Probe identifying high-friction migrations: deprecated but parse-correct syntax, conflicting docs, scattered codemod names.
- scope: language migrations, deprecated surfaces, syntax, codemod, durability grammar

## Language quality telemetry — blind spots (2026)
- source: docs/src/architecture/language-telemetry-blind-spots-findings-2026.md
- summary: Probe mapping planned language-quality signals against privacy constraints; identifies blind spots in diagnostics, paths, cross-session joins.
- scope: telemetry, privacy, language quality, diagnostic content, PII, opt-in channels

## language-benchmark-2026
- source: docs/src/architecture/language-benchmark-2026.md
- summary: Case study quantifying Vox developer velocity vs Next.js and Phoenix; measures boilerplate LoC, time-to-production, and agentic success rates.
- scope: language benchmark, developer velocity, K-complexity, boilerplate comparison, time-to-production measurement

## Legacy / tombstone remediation ledger (2026)
- source: docs/src/architecture/legacy-tombstone-remediation-ledger-2026.md
- summary: Machine-readable decisions for retired surfaces, schema lineage, SSOT drift — actions, owners, sunsets rolled into single ledger.
- scope: retired surfaces, legacy remediation, SSOT, contracts, crate naming, schema versioning

## Legacy URL Redirects on GitHub Pages
- source: docs/src/architecture/github-pages-redirects.md
- summary: Documents GitHub Pages redirect implementation: why _redirects files are ignored, static HTML stub solutions, and unhandled wildcard cases.
- scope: GitHub Pages, URL redirects, Astro, Netlify, Cloudflare Pages, trailing-slash URLs

## LLM-Misleading-Content Cleanup Plan (2026-05-08)
- source: docs/src/architecture/2026-05-08-llm-misleading-content-cleanup-plan.md
- summary: 15-phase comprehensive plan to eliminate stale syntax, retired-feature mentions, and dual-brain documentation misleading LLM tool calls.
- scope: documentation-cleanup, @island, @server, @query, @mutation, vox-clavis, vox-secrets, ADR, retired-surfaces, phase-numbering

## LLM-Target Reliability Audit — Enforcement Reachability, Ratchet Permanence, and Instruction Drift
- source: docs/src/architecture/llm-target-reliability-audit-2026-08-31.md
- summary: Measured audit of guardrail enforcement chain, detector coverage, CI wiring, baselines; revision 2 with corrections and adversarial re-audit findings.
- scope: LLM guardrails, detector enforcement, CI gates, code audit, instruction split-brain, suppression baselines

## Local Models vs OpenRouter Quality & Hybrid Topology Research (2026)
- source: docs/src/architecture/local-models-vs-openrouter-quality-research-2026.md
- summary: Empirical comparison of fine-tuned local Qwen 3 8B vs frontier cloud models; establishes asymmetric hybrid topology and Qwen 2.5 retirement SSOT.
- scope: Qwen models, OpenRouter, LLM inference, hybrid topology, MENS training, model routing, cost economics

## LoRA adapter application at serve time
- source: docs/src/architecture/lora-adapter-serving-findings-2026.md
- summary: Bug analysis and fix: LoRA deltas not applied at inference time; explains root cause, verified conventions, approach, memory optimization, enforcement guards.
- scope: LoRA adapters, inference serving, model merging, MENS, quantization, vox-plugin-mens

## Ludus Identity Federation & GitHub Integration
- source: docs/src/architecture/ludus-identity-github-integration-research-2026.md
- summary: Research on decentralized profile storage and GitHub account linking; proposes OAuth device flow, identity schema, contribution scoring, multi-wave implementation.
- scope: identity federation, GitHub integration, OAuth device flow, Ludus profiles, contribution scoring, Turso sync, vox-secrets

## ludus-adjudication-implementation-plan-2026
- source: docs/src/architecture/ludus-adjudication-implementation-plan-2026.md
- summary: Executable blueprint for implementing Ludus dispute resolution, due-process adjudication, reputation-gating; four phases with schema, DB ops, policy, CLI.
- scope: Ludus gamification, dispute resolution, adjudication, trust tiers, reward policy, schema migrations, CLI commands

## ludus-security-and-anti-cheat-research-2026
- source: docs/src/architecture/ludus-security-and-anti-cheat-research-2026.md
- summary: Security architecture for Ludus gamification: reputation-weighted scaling, proof-of-contribution verification, peer auditing via Collegium, tiered trust model.
- scope: Ludus security, anti-cheat, reputation system, DevRank, peer auditing, Collegium, trust tiers

## M4 Live Demo: Base vs Adapter
- source: docs/src/architecture/mens-m4-live-demo-2026-09-12.md
- summary: Verification that Metal-served fine-tuned model produces measurably different output from base model; fixed RoPE/compute-dtype and checkpoint-validation bugs; measured 9× speedup with Metal; tool-calling 50% success rate on 0.6B.
- scope: Metal inference verification, adapter application, RoPE synthesis, compute dtype handling, checkpoint validation, tool-calling capability, M4 milestone

## Master planning index
- source: docs/src/architecture/planning-meta/01-master-planning-index.md
- summary: Authoritative index for planning-meta corpus defining document hierarchy, read order, authority tiers, and conflict resolution rules.
- scope: planning governance, document hierarchy, authority tiers, planning taxonomy, read order

## MENS Cloud Training Economics (surveyed 2026-09-11)
- source: docs/src/architecture/mens-cloud-training-economics-2026-09-11.md
- summary: Survey of 18 GPU rental providers (on-demand/spot pricing, billing granularity, SLAs) for 27-32B QLoRA; ranks by fitness for this job size; recommends local default with RunPod Secure as contingency.
- scope: cloud GPU rental, pricing survey, provider comparison, Vast.ai, RunPod, SLA analysis, cost analysis

## MENS Corpus Pipeline Audit & Golden Example Repair — 2026-05-13
- source: docs/src/architecture/mens-corpus-audit-findings-2026.md
- summary: Audit of 56 golden Vox examples; 24 initially failing type-checker, all fixed; categorizes 15 root-cause classes; reports corpus pipeline results (19,930 training pairs).
- scope: corpus audit, golden examples, Vox compiler failures, training data pipeline, root-cause taxonomy

## MENS Program Status & Requirements Audit
- source: docs/src/architecture/mens-program-status-and-requirements-audit-2026-09-12.md
- summary: Comprehensive audit: rates 25 requirements (R1-R6, D1-D9, M1-M4) against current codebase with evidence; surfaces assumptions corrected; plans forward with 6 prioritized follow-ups.
- scope: program requirements audit, status assessment, assumption validation, LoRA adapter application, cloud calibration, calibration data gaps

## MENS Reasoning Preservation — Research Findings (2026-09-21)
- source: docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md
- summary: Audit of MENS Qwen3 fine-tuning pipeline examining chain-of-thought damage risk; concludes mode suppression likely over erased capability.
- scope: MENS, Qwen3-8B, fine-tuning, reasoning preservation, LoRA

## MENS Subsystem LoC Audit (2026-09)
- source: docs/src/architecture/mens-loc-audit-2026-09.md
- summary: Lines-of-code accounting for MENS subsystem across 6 crates; 52,245 lines total (5.8% workspace); test-to-production ratio 26%; identifies 26 god_object violations, 23 byte-identical undocumented duplicates between CUDA/Metal plugins.
- scope: MENS codebase size, god_object violations, test coverage ratio, duplication between CUDA and Metal, defactor policy violations

## MENS training pipeline — audit + improvement scoping (2026-06-07)
- source: docs/src/architecture/mens-training-pipeline-audit-and-improvement-plan-2026-06-07.md
- summary: Read-only audit of MENS QLoRA trainer state, crash diagnosis, VRAM backlog ranked by impact, and mesh distributed training verdict.
- scope: MENS, QLoRA, training pipeline, checkpoint, gradient checkpointing, distributed training

## MENS Training — Architecture Pointer
- source: docs/src/architecture/mens-training-ssot.md
- summary: Navigation pointer page directing readers to canonical reference section for MENS training procedure and operator runbooks.
- scope: MENS, training pipeline, documentation, reference

## Mesh Phase 1 — Language Spine Implementation Plan (2026-05-09)
- source: docs/src/architecture/mesh-phase1-language-spine-plan-2026.md
- summary: Step-by-step TDD implementation plan for Phase 1 of the Mesh SSOT: collapse Future/Promise/Activity-result/Awakeable into DurablePromise, introduce @remote, auto-derive activity_id, flip effect inference to bottom-up.
- scope: Vox compiler, DurablePromise primitive, @remote annotation, effect inference, workflow preview, activity scheduling

## Mesh Phase 2 — Code Mobility & Versioning Implementation Plan (2026-05-09)
- source: docs/src/architecture/mesh-phase2-code-mobility-plan-2026.md
- summary: Step-by-step TDD implementation plan for Phase 2: content-addressed workflow bundles via CAS, workflow.version() patch markers, drain tooling, mesh code seeding, activity result caching, dispatch preview, and codegen split.
- scope: vox-package CAS, workflow versioning, bundle store, activity result cache, dispatch preview, codegen split

## Mesh Phase 3 — Multi-agent VCS over mesh (op-log gossip) Implementation Plan
- source: docs/src/architecture/mesh-phase3-vcs-gossip-plan-2026.md
- summary: TDD implementation plan for Phase 3: durable op-log persistence, signed capability mints, Bloom-filter anti-entropy gossip, vector-clock affinity, sealed-trait hardening, and op-log-as-projection-source.
- scope: op-log persistence, Ed25519 signing, gossip protocol, capability mints, projections, vox-db schema, sealed traits

## Mesh Phase 4 — Dashboard Mesh-Control Surface Implementation Plan (2026-05-09)
- source: docs/src/architecture/mesh-phase4-dashboard-control-plan-2026.md
- summary: TDD implementation plan for Phase 4: mesh control surface in dashboard with provisioning wizard, topology canvas, audit-log scrubber, spend gauges, workflow debugger, and hopper panel.
- scope: dashboard, mesh topology, wizard UI, donation policy editor, audit log, workflow debugger, hopper panel

## Mesh Phase 5 — Public-Internet Safety Implementation Plan (2026-05-09)
- source: docs/src/architecture/mesh-phase5-public-internet-plan-2026.md
- summary: TDD implementation plan for Phase 5: Ed25519-signed envelopes, GitHub attestation pairing, per-key quota and reputation EMA, signed result attestations, spot-check sampling, ephemeral subkeys, and kudos accounting.
- scope: Ed25519 envelopes, GitHub attestation, quota and reputation, spot-check sampling, kudos accounting, model inventory, X25519 JWE

## Mesh Phase 6 — Grand Network (Volunteer Compute) Implementation Plan (2026-05-09)
- source: docs/src/architecture/mesh-phase6-grand-network-plan-2026.md
- summary: TDD implementation plan for Phase 6: opt-in joinable bounded-trust global mesh via federation envelope, public attestation registry, micro-VM sandbox interface, redundant-execution voting, and trust-graph self-publication.
- scope: federation envelope, public attestation registry, micro-VM sandbox, redundancy voting, TEE attestation, Scientia discovery loop, volunteer compute

## Mesh, Dashboard & Distributed Compute — Research (2026-05-09)
- source: docs/src/architecture/mesh-dashboard-and-distributed-compute-research-2026.md
- summary: Two-horizon research synthesis auditing mesh/dashboard/durable-workflow state, surveying prior art, identifying gaps and Wave-2 design space; feeds future plans-of-record.
- scope: mesh, dashboard, distributed compute, durable workflows, prior art survey, security model

## Migration of Vox Compiler to AI-Native Architecture
- source: docs/src/architecture/path-b-decommission-2026.md
- summary: Completed migration log decommissioning Path B HIR primitives and collapsing legacy code to Path C UI standards.
- scope: Vox compiler, HIR primitives, Path B, Path C, route manifest, golden files

## Mobile architecture and migration plan — RN + Expo + uniffi (2026)
- source: docs/src/architecture/mobile-rn-expo-architecture-and-migration-2026.md
- summary: Architecture guide for React Native + Expo + uniffi bridging: one VUV HIR with two GUI lowerings (web and mobile), same Rust runtime on every device, Tauri for desktop only.
- scope: React Native, Expo, uniffi, VUV HIR, codegen, Tauri desktop, mental-tracker

## Mobile End-to-End Testing Strategy Implementation Plan
- source: docs/src/architecture/mobile-e2e-testing-strategy-2026.md
- summary: Strategy and implementation phases for mobile E2E testing using Tauri 2: automated testing infrastructure, build pipelines, and deployment workflows for Android and iOS.
- scope: mobile E2E testing, Tauri 2, Android emulator, iOS simulator, CI/CD automation, deployment workflows

## Mobile Phase 2 completion: cross-compile pipeline, iOS, mental-tracker Expo upgrade (scoping)
- source: docs/src/architecture/mobile-phase2-completion-scoping-2026-06-12.md
- summary: Verified-state scoping for mobile work items: Android cross-compile pipeline, iOS path, and mental-tracker upgrade from Capacitor-era shell to Expo target.
- scope: Android cross-compile, iOS build, uniffi bindgen, Expo plugin, mental-tracker app, native runtime packaging

## Mobile target evaluation 2026 — Tauri-mobile vs RN + Expo
- source: docs/src/architecture/mobile-target-evaluation-2026.md
- summary: Research and hands-on bake-off comparing Tauri-mobile and React Native + Expo as Vox's mobile target, with methodology, findings, and open questions.
- scope: Tauri, React Native, Expo, mobile targets, Windows host, Android emulator

## Mobile target research sources (2026-05-27)
- source: docs/src/architecture/_screenshots/research-sources.md
- summary: Curated research sources (May 2024–May 2026) backing mobile target evaluation: Tauri 2 mobile, React Native/Expo, Capacitor, LLM-friendliness benchmarks.
- scope: mobile targets, Tauri 2, React Native, Expo, Capacitor, framework evaluation, research sources

## Mobile: emulator run proof + EAS CI / Expo account setup
- source: docs/src/architecture/mobile-eas-ci-and-emulator-run-2026.md
- summary: Proof of life for Vox-generated Expo app on Android emulator, reproducible run procedure, and steps to wire up Expo account and EAS Build CI runner.
- scope: Android emulator, Expo Go, EAS Build, Expo account, CI runner setup, mobile proof of concept

## Multi-Agent VCS Replication — Landscape Research (2026-05-03)
- source: docs/src/architecture/multi-agent-vcs-replication-research-2026.md
- summary: Survey of version-control and CRDT systems evaluated for multi-agent code collaboration. Finds no off-the-shelf project meets requirements; recommends Path 1 (op-log gossip on jj + Populi).
- scope: Jujutsu, Pijul, Automerge, CRDT, VCS replication, multi-agent collaboration

## Multi-Provider & Local-vs-Cloud Model Routing — Verified Research (2026-07-30)
- source: docs/src/architecture/multi-provider-local-cloud-routing-research-2026-07-30.md
- summary: Adversarial-verified research into production LLM routing: LiteLLM, OpenRouter, Portkey, RouteLLM. Distilled into a declarative policy design with ledger of what does not transplant.
- scope: LLM routing, model selection, provider selection, fallback cascades, cost-based routing, latency-based routing

## Native Scraping / Browser Automation in Vox — Scoping & Handoff (2026-06-03)
- source: docs/src/architecture/vox-native-scraping-scoping-2026-06-03.md
- summary: Honest scoping audit of existing browser/scraping capabilities (chromiumoxide CDP, static fetch/parse), Rust landscape survey, and recommended tracks for ergonomic assembly.
- scope: web scraping, browser automation, chromiumoxide, CDP protocol, static HTML parsing, readability, web accessibility, effect governance, LLM ergonomics

## Next-Generation AI Orchestrator: Systemic Flaws, Power User Demands, and Production Design Patterns
- source: docs/src/architecture/nextgen-orchestrator-research-2026.md
- summary: Comprehensive research synthesis covering enterprise AI failure modes, native-systems performance, multi-provider routing, autonomous FinOps, hallucination prevention, mesh GPU, multi-agent coherence, AI-first DSLs.
- scope: orchestrator design, enterprise AI, budget management, multi-provider routing, hallucination detection, mesh GPU architecture, distributed inference, workflow durability, cost optimization

## Nightly Builds
- source: docs/src/architecture/nightly-builds-ssot.md
- summary: Operational documentation of nightly release-pipeline exercise, why no public automatic nightly release, and how to use nightly builds locally via draft releases.
- scope: nightly builds, release pipeline, CI automation, draft releases, GitHub workflows, release safety

## Omni-Search Audit & Roadmap (2026)
- source: docs/src/architecture/omni-search-audit-and-roadmap-2026.md
- summary: Comprehensive audit of search capabilities, surfaces, gaps, bugs across full stack. Maps current architecture, identifies 20 bugs, proposes 28 improvements in three tiers.
- scope: search, indexing, retrieval, GUI search surfaces, corpus types, telemetry, RRF fusion, vector search

## Orchestrator + GUI Agent-Dispatch Audit (2026-07-02)
- source: docs/src/architecture/orchestrator-gui-dispatch-audit-2026-07-02.md
- summary: Adversarial audit of orchestrator core, MCP dispatch, GUI dispatch with verified findings: trust bypassable at two levels, restart mangles work, split-brain across MCP/CLI/GUI, dead streams stay silent.
- scope: orchestrator daemon, GUI agent dispatch, MCP dispatch, tool calls, approvals, trust boundary, TCP transport, event bus, restart recovery

## Orchestrator Companion Audit — Non-Routing Surface Critique & Improvement Plan
- source: docs/src/architecture/orchestrator-companion-audit-findings-2026.md
- summary: Full-system audit of vox-orchestrator excluding model routing: 280+ numbered improvements across 27 surface clusters, four-axis tagged (risk/capability/hygiene/perf × P0–P3).
- scope: orchestrator core, runtime supervisor, handoff validation, event bus, grounding, MCP dispatch, usage tracking, HTTP gateway, memory tools, persistence

## Orchestrator Stage 1 & 2: Security + Crash-Prevention Implementation Plan
- source: docs/src/architecture/2026-05-01-orchestrator-stage1-stage2.md
- summary: Implementation plan for 7 P0/P1 fixes across orchestrator gateway, runtime, grounding, daemon surfaces with step-by-step TDD approach.
- scope: orchestrator, security, crash-prevention, gateway, runtime, grounding, daemon

## OS-Level Computer Use and Accessibility-Tree Semantic Grounding (2026)
- source: docs/src/architecture/os-agent-computer-use-and-accessibility-tree-research-2026.md
- summary: Research on native OS accessibility APIs (macOS AXUIElement, Windows UIA, Linux AT-SPI2), vision-AX hybrid grounding, token serialization, adversarial defenses, multi-monitor geometry, Wayland protocols.
- scope: OS accessibility APIs, desktop automation, computer use, AXUIElement, UI Automation, AT-SPI2, vision grounding, Set-of-Marks, security defenses, multi-monitor

## Phase 1 — SSOT Collapse
- source: docs/src/architecture/vox-language-rules-phase1-ssot-collapse-2026.md
- summary: Eliminate hand-mirrored Rust↔Vox surfaces via xtask-generated outputs: builtin registry, decorator catalog, grammar exports, LSP completions, docs reference pages with blake3 provenance headers.
- scope: SSOT generation, code generation, builtin registry, decorator catalog, LSP completions, xtask automation

## Phase 2 — Lint Extension with Stable Diagnostic IDs
- source: docs/src/architecture/vox-language-rules-phase2-lint-extension-2026.md
- summary: Add 14 new detectors to vox-code-audit covering direct-LLM-call rejection, env.get-shape, decorator-position; add --for-llm JSON mode optimized for LLM agents.
- scope: linting, detectors, diagnostics, code audit, LLM-friendly output

## Phase 3 — Cheap Typechecker Rules
- source: docs/src/architecture/vox-language-rules-phase3-typecheck-rules-2026.md
- summary: Add typechecker rules making wrong programs structurally unrepresentable: Id[T] at API boundaries, named error types, syntax_version enforcement, @deprecated checking, training_eligible propagation.
- scope: type system, type checking, API boundaries, ID types, error handling

## Phase 4 — Runtime Monitors
- source: docs/src/architecture/vox-language-rules-phase4-runtime-monitors-2026.md
- summary: Add runtime safety nets: per-call fuel, allocation observer with cap, stack-depth cap, panic-trap boundary, telemetry redactor, capability-violation trap, idiom fingerprint export.
- scope: runtime safety, execution budgets, fuel accounting, panic handling, capability system

## Phase Numbering Index
- source: docs/src/architecture/phase-numbering-index.md
- summary: Reference guide disambiguating three independent phase sequences (frontend interop, GUI-native, workspace reorg) used in Vox plans.
- scope: phase sequences, frontend interop, GUI-native language, workspace reorg

## Planning critique and gap analysis
- source: docs/src/architecture/planning-meta/04-planning-critique-gap-analysis.md
- summary: Severity-ranked critique of prior planning artifacts with root-cause analysis and explicit fix mapping to the planning-meta corpus.
- scope: planning critique, gap analysis, findings, root-cause analysis, planning improvements

## Planning meta exception register
- source: docs/src/architecture/planning-meta/exception-register.md
- summary: Active and retired exceptions/deferrals tracking for planning-meta governance.
- scope: planning governance, exception tracking, planning-meta

## Planning meta maintenance log
- source: docs/src/architecture/planning-meta/maintenance-log.md
- summary: Change log for Tier 1/2/3 planning-meta document updates with entries and rationales.
- scope: planning governance, maintenance tracking, document changes

## Planning taxonomy and glossary
- source: docs/src/architecture/planning-meta/06-planning-taxonomy-glossary.md
- summary: Canonical terminology reference for planning-meta artifacts with preferred terms, forbidden aliases, and historical mappings.
- scope: planning terminology, canonical terms, glossary, planning-meta, authority terms, planning quality terms

## Plugin Registration Resolution Path (2026)
- source: docs/src/architecture/plugin-registration-resolution-2026.md
- summary: Documents root cause and resolution steps for tensor-burn-wgpu to mens-candle-cuda regression, providing prevention template.
- scope: plugin system, ML backends, error resolution, tensor-burn-wgpu, mens-candle-cuda

## Plugin System Audit (2026-05-08)
- source: docs/src/architecture/plugin-system-audit-2026-05-08.md
- summary: First-pass audit of plugin system post-extraction covering ABI drift, scaffolds, duplicate code paths, and build optimization opportunities.
- scope: plugin system, ABI versioning, code duplication, NVML, build optimization

## Plugin System Deep Audit (2026-05-08)
- source: docs/src/architecture/plugin-system-deep-audit-2026-05-08.md
- summary: Second-pass audit covering ABI completeness, discovery, sandbox model, cross-cutting concerns, and distribution with recommendations prioritized by effort.
- scope: plugin system, ABI surface, plugin lifecycle, sandbox model, security, distribution

## Plugin System Redesign — SP1 Implementation Plan (2026)
- source: docs/src/architecture/plugin-system-redesign-sp1-plan-2026.md
- summary: Step-by-step TDD implementation plan for plugin manifest schemas, vox-plugin-catalog SSOT crate, documentation, and parity CI guards.
- scope: implementation planning, plugin system, test-driven development, catalog schema, CI automation

## Plugin System Redesign — SP2 Implementation Plan (2026)
- source: docs/src/architecture/plugin-system-redesign-sp2-plan-2026.md
- summary: Step-by-step TDD implementation plan for vox-plugin-api ABI traits, vox-plugin-host loader for code and skill payloads, dual registry, and test plugins.
- scope: implementation planning, plugin system, ABI design, host loader, test fixtures

## Plugin System Redesign — SP3 Implementation Plan (2026)
- source: docs/src/architecture/plugin-system-redesign-sp3-plan-2026.md
- summary: Step-by-step TDD implementation plan for MlBackend extension trait, mens-candle-cuda plugin extraction from vox-populi, and training parity verification.
- scope: implementation planning, MlBackend trait, plugin extraction, CUDA training, mens-candle-cuda, ML backends

## Plugin System Redesign — SP4 Implementation Plan (2026)
- source: docs/src/architecture/plugin-system-redesign-sp4-plan-2026.md
- summary: Step-by-step TDD implementation plan for skill-compiler migration from vox-skills compile-time builtins to standalone skill-payload plugin at runtime.
- scope: implementation planning, skill plugins, vox-skills migration, orchestrator integration, MCP dispatch

## Populi Mesh Improvement Backlog (2026-05-01)
- source: docs/src/architecture/populi-mesh-improvement-backlog-2026.md
- summary: Flat tagged list of mesh improvements not load-bearing enough for dedicated specs; picked up opportunistically.
- scope: Populi Mesh, improvements, backlog, MESH-NNN items

## Populi Mesh North-Star (2026-05-01)
- source: docs/src/architecture/populi-mesh-north-star-2026.md
- summary: Design intent and capability-slice plan decomposing seven workstreams into three sequenced slices with child-spec roadmap.
- scope: Populi Mesh, north-star plan, workstreams, slices, strategic design

## Populi Mesh — Probe Correctness Implementation Plan (S1, 2026-05-01)
- source: docs/src/architecture/populi-mesh-probe-correctness-plan-2026.md
- summary: Step-by-step TDD implementation plan with 17 tasks: HardwareProbe trait, mock harness, refactored probes, tests.
- scope: Populi Mesh, hardware probes, implementation plan, TDD

## Post-Sprint Forward Plan (2026-05-25)
- source: docs/src/architecture/post-sprint-forward-plan-2026-05-25.md
- summary: Forward plan for all crates/tracks not fixed in F-* sprint; defines scope, gates, and acceptance for remaining items.
- scope: forward plan, audit, tracks, prescriptions, crate-audit

## Qwen3.7 profile + MENS 4B-on-RTX-4080 feasibility (revised)
- source: docs/src/architecture/qwen-3.7-profile-and-mens-4b-feasibility-2026-06-07.md
- summary: Verified June-2026 research: Qwen3.7 is closed API-only; 4B QLoRA on 16 GB 4080 is not viable; pipeline auto-retreats to 2B.
- scope: Qwen3.7, MENS, feasibility, QLoRA, RTX 4080, 4B models, 2B models

## Repository cleanup ledger (2026 deep reorg)
- source: docs/src/architecture/repo-cleanup-ledger-2026.md
- summary: Audit ledger recording orphan artifact removal, runtime untracking, surface folder moves, and sprawl reduction with executable action rows.
- scope: repository organization, artifact cleanup, VCS operations, configuration management

## Repository layout sprawl audit (2026)
- source: docs/src/architecture/repo-layout-sprawl-audit-2026.md
- summary: Organization audit identifying sparse directories, overlapping categories, non-Rust artifact producers/consumers, with prioritized consolidation backlog.
- scope: repository layout, folder taxonomy, artifact management, operational duplication

## Research baseline and source-of-truth map
- source: docs/src/architecture/planning-meta/00-research-baseline-source-map.md
- summary: Research appendix documenting source classification, confidence tags, and external validation for the planning-meta corpus creation.
- scope: research baseline, source classification, normative sources, operational sources, external assumptions

## Research: Compiled Systems Native Code Emission for Vox (2026)
- source: docs/src/architecture/native-code-emission-research-2026.md
- summary: Audit of Vox compiler pipeline for native code emission paths: Rust-as-IL vs LLVM/Cranelift, WebIR enrichment, GPU rendering, VUV-Native architecture.
- scope: compiler pipeline, native code emission, LLVM, Cranelift, WebIR, GPU rendering, Rust codegen, incremental codegen

## Research: Workspace Health & Dependency Governance (2026)
- source: docs/src/architecture/workspace-health-audit-research-2026.md
- summary: Synthesizes improvements to architectural enforcement pipeline; moves from heuristics to policy-driven system.
- scope: workspace health, dependency governance, architectural enforcement, CI/CD integration, performance monitoring

## Rust Warning Audit & Remediation Backlog (2026-05-11)
- source: docs/src/architecture/rust-warning-audit-backlog-2026.md
- summary: Audit ledger recording CI-parity baseline for clippy/rustdoc/cargo, justified per-item suppressions inventory, and review cadence for warning-debt management.
- scope: Rust compiler warnings, clippy lints, rustdoc, code quality, warning suppression

## SCIENTIA Automated Research — agent handoff
- source: docs/src/architecture/scientia-automated-research-handoff-2026-06-16.md
- summary: Fresh-agent onboarding context for SCIENTIA implementation: working-tree status, Waves 0–6 deliverables, code-review findings, remaining work, and verification gates.
- scope: SCIENTIA implementation, agent workflow, verification commands, research pipeline

## SCIENTIA Automated Research: Historical Roles and Extension Research (2026)
- source: docs/src/architecture/scientia-automated-research-historical-extension-research-2026.md
- summary: Research document mapping historical computer-assisted research roles to Vox capabilities, code-review findings from June 2026, and phased extension plan for unified research loop.
- scope: SCIENTIA architecture, research capability mapping, code review findings, extension roadmap, deep research

## SCIENTIA Self-Publication Finalization Plan (2026)
- source: docs/src/architecture/scientia-self-publication-finalization-plan-2026.md
- summary: Approved multi-phase strategic plan (Phases 0–10) for autonomous SCIENTIA research publication targeting IMC/MLSys/TMLR with reputational firewall, pre-registration, symbolic verifiers, and living-review Provider Atlas.
- scope: self-publication strategy, measurement campaigns, claim extraction, novelty assessment, pre-registration, scholarly publishing, nanopub, RO-Crate, Provider Atlas

## SCIENTIA Self-Publication Gap Map (2026)
- source: docs/src/architecture/scientia-self-publication-gap-map-2026.md
- summary: Audit of gaps between completed finalization phases and full self-publication workflow, with prioritized remediation roadmap.
- scope: SCIENTIA, publication, gap-map, candidate-producers, replay-verification, manuscript-scaffolding, solo-author-approval

## Script tier timings (2026-09)
- source: docs/src/architecture/script-tier-timings-2026-09.md
- summary: Measured execution timings for 84 scripts; input data for interpreter-first default-tier flip decision.
- scope: script-execution, interpreter-tier, native-tier, performance-measurement, vox-cli, timing-data

## Semantic Coverage Remediation Plan v2 (audited)
- source: docs/src/architecture/semantic-coverage-remediation-plan-2026-06-13.md
- summary: Self-contained TDD implementation plan with per-symbol verification protocol, worked examples, and multi-wave execution roadmap.
- scope: semantic-test-coverage, TDD, verification-protocol, test-remediation, coverage-graph, Wave-0-5

## Semantic Coverage — Honest Status (2026-06-15)
- source: docs/src/architecture/semantic-coverage-status-2026-06-15.md
- summary: Verified accounting of test coverage: 3,088 proven (15.9%), 5,793 reached-but-unproven, with audit of weak patterns and priority targets.
- scope: test-coverage-audit, coverage-metrics, semantic-proof, reached-vs-proven, priority-crates, status-report

## Semantic Gap Audit — 2026-05-16
- source: docs/src/architecture/semantic-gap-audit-2026.md
- summary: Forensic audit of code-contract drift: 8 verified findings (7 HIGH, 1 meta) across orchestrator, codegen, and plugins.
- scope: audit, semantic-gaps, contract-drift, silent-drops, validator-unwiring, trait-skeletons

## Semantic Gap Implementation Plan (2026-05-16)
- source: docs/src/architecture/semantic-gap-implementation-plan-2026.md
- summary: TDD-based remediation plan for 7 HIGH and 1 LOW semantic-gap findings, organized in 3 independent batches with task breakdown.
- scope: implementation-plan, TDD, batch-B-codegen, batch-A-orchestrator, batch-C-plugin, task-breakdown

## Semantic Test-Coverage Graph Strategy (2026-06-07)
- source: docs/src/architecture/semantic-test-coverage-graph-strategy-2026-06-07.md
- summary: Strategic proposal for a searchable semantic-coverage map overlaid on graphify graph; Phase 0–3 roadmap with open design decisions.
- scope: strategy, coverage-graph, semantic-proof, graphify, phased-build, proof-strength

## Shiki, mdBook & Documentation Platform Evaluation (2026)
- source: docs/src/architecture/shiki-mdbook-doc-platform-research-2026.md
- summary: Comprehensive research on documentation platforms with quantified feature matrix and migration recommendations.
- scope: documentation platform, syntax highlighting, Shiki, mdBook, TextMate grammar, doctest, vox.tmLanguage.json

## Single-file parent directories (triage list)
- source: docs/src/architecture/repo-layout-single-file-parent-dirs-triage-2026.md
- summary: Machine-generated triage list of ~203 tracked directories with exactly one file; reference for consolidation decisions.
- scope: repository structure, directory organization, Rust modules, configuration folders

## Skill Discovery at Scale & Automatic Skill Induction — Verified Research 2026-07-30
- source: docs/src/architecture/skill-discovery-and-induction-research-2026-07-30.md
- summary: Adversarially-verified research on tool discovery scaling and automatic skill mining with promotion gates.
- scope: tool discovery, skill induction, skill promotion gate, LLM evaluation, Voyager, CRAFT, AWM, SkillWeaver, retirement, skill_reliability

## Skill Ecosystem Audit — All 50 Skill Files, Read in Full (2026-08-01)
- source: docs/src/architecture/skill-ecosystem-audit-2026-08-01.md
- summary: Per-file audit of 50 skill files with findings on format, gaps, duplication, and architecture defects.
- scope: skill files, frontmatter dialect, MCP tool registry, skill selection, GUI surface, vox-graph skill, format migration, Vox-native skills

## Skill Ecosystem Interop & Model-Awareness Research (2026-06-12)
- source: docs/src/architecture/skill-ecosystem-interop-research-2026-06-12.md
- summary: Verified audit of Vox skill system vs agentskills.io ecosystem, identifying gaps and recommending architecture to converge on open standards.
- scope: skill ecosystem, agentskills.io, interop, model awareness, bundling

## Skill Marketplace Security: Attacks, Review Policy & Provenance (2026-07-30)
- source: docs/src/architecture/skill-marketplace-security-and-provenance-research-2026-07-30.md
- summary: Adversarially verified research on MCP security attacks, GPT Store review effectiveness, and package provenance prior art for skill registry hardening.
- scope: MCP security, tool poisoning, rug pull attacks, GPT Store, Sigstore provenance, skill registry

## Skill/Tool Registry Curation & Trust Models — Verified Research (2026-07-30)
- source: docs/src/architecture/skill-registry-trust-and-curation-research-2026-07-30.md
- summary: Verified research into MCP Registry namespace-ownership model, moderation policy, and takedown behavior; design patterns for Vox skill registry.
- scope: MCP Registry, namespace ownership, moderation policy, trust models, registry design

## Suite status audit and bug handoff (2026-09-21)
- source: docs/src/architecture/suite-status-audit-2026-09-21.md
- summary: Comprehensive audit findings from verifying README status against codebase; 36 self-contained bug entries with reproduction steps and fix guidance.
- scope: CLI, core paths, bug handoff, fmt, secrets, telemetry, doc platforms

## Surfacing Errors to Humans and LLMs — Dual-Audience Research
- source: docs/src/architecture/error-surfacing-dual-audience-research-2026-06-18.md
- summary: Research on error surfacing to humans and LLMs, findings on telemetry quality dominance, and design implications for diagnostic envelopes.
- scope: error surfacing, LLM diagnosis, telemetry quality, diagnostic envelopes

## Svelte 5/6 vs React Meta-Frameworks — Comparative Research and Mineable Ideas for Vox (2026)
- source: docs/src/architecture/svelte-vs-react-frameworks-research-2026.md
- summary: Comparative analysis of Svelte, Next.js, and TanStack Start; identifies seven mineable features for Vox's GUI layer without changing React-emit stance.
- scope: framework comparison, Svelte runes, React hooks, meta-frameworks, GUI authoring, AI codegen

## Svelte-Mineable Features Implementation Plan (2026)
- source: docs/src/architecture/svelte-mineable-features-implementation-plan-2026.md
- summary: Phased implementation roadmap for seven Svelte 5/6-inspired features (M1–M7); concrete file paths, scope estimates, dependencies, and verification strategies.
- scope: Svelte features, reactive modules, fragments, directives, state machines, compiler, Web IR

## Tauri Audit 2026
- source: docs/src/architecture/tauri-audit-2026.md
- summary: Comprehensive audit of Tauri usage, mobile feasibility, build costs, strategic options, and retirement candidates for Vox desktop/mobile GUI pipelines.
- scope: Tauri 2, desktop packaging, mobile apps, Capacitor interop, build cost analysis

## Tauri convergence migration plan (2026-Q2)
- source: docs/src/architecture/tauri-convergence-migration-plan-2026.md
- summary: Executable roadmap for converging desktop/mobile packaging on Tauri 2 while retiring Capacitor and Axum.
- scope: Tauri, desktop packaging, mobile packaging, Capacitor retirement, Axum retirement, migration phases

## Telemetry-Driven Cost Accounting Architecture (2026)
- source: docs/src/architecture/telemetry-driven-cost-accounting-research-2026.md
- summary: Implemented system for self-correcting empirical feedback loop reflecting ground-truth spend across LLM providers.
- scope: telemetry, cost accounting, model pricing, Scientia, LLM routing, budget management

## Tier D — Orchestrator core-extraction plan (2026-05-15, updated 2026-05-24)
- source: docs/src/architecture/2026-05-15-orchestrator-tier-d-plan.md
- summary: Assessment and implementation plan for extracting vox-orchestrator-core from vox-orchestrator after dei_shim extraction; Rule 13 not yet fired, C5 deferred.
- scope: vox-orchestrator, vox-orchestrator-core, vox-dei-shim, crate extraction, Rust coherence, layering

## Toolchain Reality and Omnibus Installer Findings
- source: docs/src/architecture/toolchain-reality-and-omnibus-installer-findings-2026.md
- summary: Audit of hidden toolchain dependencies in Vox 'single-command install'; roadmap for true independence via native bundler and voxup.
- scope: toolchain, installation, Node.js dependency, Rust ecosystem, bundler, voxup, hermetic toolchain

## Tooling Convergence — Findings & Plan (2026-05-09)
- source: docs/src/architecture/tooling-convergence-findings-2026.md
- summary: Audit of fragmented testing, linting, and CI/CD tooling with convergence plan to route all checks through single owners.
- scope: tooling convergence, testing infrastructure, linting, CI/CD, tool unification, TOESTUB, rustfmt, clippy

## Trust, Novelty & Quality-Scoring Landscape for Deep Research (2026-08-01)
- source: docs/src/architecture/deep-research-trust-novelty-scoring-landscape-2026-08-01.md
- summary: Survey of source-credibility scoring, novelty detection, claim verification, and bad-research detection techniques; maps to Vox implementation gaps.
- scope: source credibility, novelty detection, claim verification, NLI verification, research quality, gate.rs, verifier.rs

## Turso Ownership Migration — Handoff (2026)
- source: docs/src/architecture/turso-ownership-migration-handoff-2026.md
- summary: Handoff capturing Turso-ownership migration state: policy SSOT, CI guards, allowlist rules, backlog, and nomenclature issues.
- scope: database ownership, Turso migration, vox-db data layer, policy enforcement, CI guards, migration backlog

## Unified Task Hopper — Research, Design Space, and Recommendation (2026-05-09)
- source: docs/src/architecture/unified-task-hopper-research-2026.md
- summary: Audits unified developer-facing task intake proposal, evaluates three design options (A: lightweight adapter, B: persistent service, C: mesh-native), recommends Option A with forward-compatible persistence schema.
- scope: orchestrator, task intake, agent queues, hopper design, worktree isolation, VCS automation

## v0.5-core-ssot
- source: docs/src/architecture/v0.5-core-ssot.md
- summary: Authoritative architecture snapshot for v0.5 release covering CLI, persistence, MENS intelligence layer, and DEI orchestrator components.
- scope: v0.5 release, CLI, database, MENS, orchestrator, deployment

## v1.0 Foundation-Criteria Advisory (2026-06-05)
- source: docs/src/architecture/v1-foundation-criteria-research-2026.md
- summary: Forensic audit finding current v1.0 criteria cannot certify a finished language; proposes FOUNDATION tier (CR-F1–F6) with machine-readable LLM-actionable format and gate ordering.
- scope: v1.0 release criteria, Foundation tier, compiler completeness, behavioral verification, arm parity, Distribution tier, GUI tier

## v1.0 Readiness Status (2026-07-22 audit)
- source: docs/src/architecture/v1-readiness-status-2026-07.md
- summary: Live-codebase audit of CR-F/CR-K/CR-U criteria as of 2026-07-22, reporting implementation status (Built & Verified / Built, Unverified / Unbuilt) for each criterion.
- scope: v1.0 readiness, CR-F Foundation tier, CR-K Distribution tier, CR-U GUI tier, audit, implementation status

## Version Control as a Vox Language Feature — a Jujutsu-Native Multi-Agent VCS
- source: docs/src/architecture/vcs-as-vox-language-feature-jujutsu-2026.md
- summary: Research proposal to make version control a first-class Vox language primitive backed by Jujutsu, enabling multi-agent concurrency with conflict-as-data and undo/redo semantics.
- scope: version control, Jujutsu, language feature, multi-agent VCS, conflict resolution, orchestrator, repo.* builtins

## Village Narrative Architecture Research Audit
- source: docs/src/architecture/village-narrative-architecture-findings-2026.md
- summary: Research audit on game narrative architecture with five-layer framework, critical LLM constraints, identified bugs, and preliminary recommendations for FableForge village.
- scope: narrative architecture, game design, FableForge village, LLM narration, drama management, story beats, NPC systems

## visus-audit-grounding.v1.md
- source: docs/src/architecture/prompts/visus-audit-grounding.v1.md
- summary: Prompt template for Vox Visus visual intelligence agent; defines evaluation vectors, taxonomy, and JSON output format.
- scope: Visus, audit, prompt, GUI, visual intelligence, QA

## Vox & MENS Comparative Efficacy Benchmarking: Research, Gap Audit, and Automation Plan (2026)
- source: docs/src/architecture/vox-mens-comparative-efficacy-benchmarking-research-2026-09-01.md
- summary: Research on HLE/benchmarking practices, code-audited gap map of SCIENTIA and model router vs market, and plan for live contamination-resistant voxlang.org leaderboard.
- scope: benchmark design, model evaluation, HLE lessons, contamination resistance, leaderboard governance, SCIENTIA pipeline, model router, publishing infrastructure

## Vox as an LLM-Target Language — Audit & v1.0 Plan (2026-05-15)
- source: docs/src/architecture/vox-as-llm-target-audit-and-plan-2026.md
- summary: Comprehensive audit of Vox readiness as LLM-target with gap analysis, attainability verdict (realistic vs aspirational v1.0), and eight proposed CR-L fidelity criteria.
- scope: LLM-target, audit, v1.0 plan, CR-L criteria, gap analysis, attainability, self-repair, diagnostic surface

## Vox compiler architecture (research)
- source: docs/src/architecture/vox-compiler-architecture-research-2026.md
- summary: High-level map of vox-compiler pipeline stages, module organization, extension points, and links to enforcement/language-rules plans.
- scope: vox-compiler, lexer, parser, AST, HIR, typecheck, codegen, interpreter

## Vox Dashboard — Design Brief for Anthropic Labs Design (2026)
- source: docs/src/architecture/vox-dashboard-design-brief-2026.md
- summary: Screen-by-screen design brief for Vox dashboard redesign: seven surfaces, command palette, status bar, operator workflows for runs/models/repos/code.
- scope: Vox dashboard, operator harness, UI design, command palette, orchestrator interface, model selection

## Vox Deep Research Capabilities — Full Audit & 2026 Roadmap
- source: docs/src/architecture/deep-research-capabilities-audit-2026-06-17.md
- summary: Comprehensive audit of deep research pipeline against 2026 state-of-the-art, identifying 9 ranked capability gaps and providing 4-phase implementation roadmap.
- scope: Deep Research, Search stack, Retrieval infrastructure, CRAG, Novelty detection, OpenRouter free tier, SCIENTIA, LLM cascades, confidence gating

## Vox Design — Auto-GUI & Zero-Annotation Severity-Graded Debugging
- source: docs/src/architecture/automatic-gui-and-debugging-vox-design-2026-06-18.md
- summary: Concept design (roadmap status) for three tracks: (A) naked-objects auto-GUI via @admin, (B) zero-annotation severity-graded interpreter debugging, (C) Vox-as-target for AI UI generators.
- scope: concept design, auto-GUI, naked-objects, @admin annotation, execution tracer, severity inference, MCP tools, design-system interop

## Vox Docs Portal: Astro Starlight Strategy 2026
- source: docs/src/architecture/starlight-site-strategy-2026.md
- summary: Research findings and execution roadmap for Starlight documentation portal; covers gaps, user journeys, AI discoverability, MENS integration, and next steps.
- scope: Astro Starlight, documentation portal, mdBook migration, user journeys, AI indexing, MENS pipeline

## Vox Efficacy Benchmark — Adversarial Audit and Corrected Design (2026-09-01)
- source: docs/src/architecture/vox-efficacy-benchmark-adversarial-audit-2026-09-01.md
- summary: Seven-track adversarial review of efficacy benchmark plan documenting critical scoring exploit, pass@k estimator bug, statistical defects, and corrected design with defense-in-depth fixes.
- scope: benchmark methodology, efficacy measurement, pass@k scoring, statistical testing, corpus coverage, harness verification

## Vox Efficacy Benchmark — Execution Handoff (2026-09-04)
- source: docs/src/architecture/vox-efficacy-benchmark-execution-handoff-2026-09-04.md
- summary: Runbook for executing the efficacy benchmark on a new machine: exact commands, corpus ground truth, model selection, MENS checkpoint evaluation, contamination defense, known unverified edges.
- scope: benchmark execution, MENS evaluation, frontier model comparison, corpus, harness deployment

## Vox Feature Discoverability Audit (2026)
- source: docs/src/architecture/discoverability-audit-2026.md
- summary: Audit of CLI/GUI single-source-of-truth gaps, shell completions, LSP capabilities, and discoverability improvements across Vox toolchain.
- scope: CLI discoverability, GUI command catalog, shell completions, LSP hover, LSP completions, tree-sitter grammar

## Vox Gamification & Ludus System Review
- source: docs/src/architecture/gamification-ludus-review-findings-2026.md
- summary: Comprehensive review of Vox Gamify subsystem architecture; identifies gaps in wellness features, cost controls, SVG art generation, and optionality constraints.
- scope: gamification architecture, Ludus subsystem, wellness features, cost tracking, companion SVG rendering, developer burnout prevention

## Vox GUI Capability Audit 2026
- source: docs/src/architecture/vox-gui-capability-audit-2026.md
- summary: Reality audit of Vox GUI: CLI-driven discoverability via 472 catalog entries, Tauri 2 infrastructure, what is real vs scaffolded, path toward maintainable code harness.
- scope: GUI audit, CLI catalog, Tauri integration, desktop shell, React UI, command discovery

## Vox GUI Design Review (annotated mockups + component specs)
- source: docs/src/architecture/vox-gui-design-review-2026.md
- summary: Visual design review for Vox Tauri GUI with annotated mockups, design system foundations, and component API proposals for five operator surfaces.
- scope: GUI design system, component APIs, visual language, surfaces: Dashboard, Chat, Tasks, Runs, Policies, design tokens, accessibility

## Vox GUI Harness Build-Out Plan 2026
- source: docs/src/architecture/vox-gui-harness-buildout-plan-2026.md
- summary: Three-track plan to evolve Tauri GUI from CLI-derived dashboard into full agentic harness: stateful core, CLI command surface, and UX design.
- scope: harness capabilities, daemon event streaming, run store, interactive approvals, streaming chat, MCP execution, design system, command manifest

## Vox GUI UX Beautification & Build-out Plan
- source: docs/src/architecture/vox-gui-ux-beautification-plan-2026.md
- summary: Comprehensive code review, bug catalog (25+ issues), and seven-phase plan to take GUI from functional operator console to fully built-out, accessible, design-system-driven product.
- scope: UX design, design system, bug fixes and polish, accessibility, component primitives, surface rebuild, user journeys, visual regression testing

## Vox GUI Visual Audit & Fix Handoff (2026-06-03)
- source: docs/src/architecture/vox-gui-visual-audit-2026-06-03.md
- summary: Full-surface screenshot audit of Tauri GUI frontend with critical app-blanking bug found and fixed, plus prioritized visual/robustness fixes and reusable Playwright harness.
- scope: visual audit, GUI robustness, icon registry fixes, typecheck gate, Playwright e2e testing, bug catalog, empty states

## Vox GUI ↔ CLI / Scientia Coverage Audit (2026-06-03)
- source: docs/src/architecture/vox-gui-scientia-coverage-audit-2026.md
- summary: Gap map between Tauri GUI and CLI/Scientia surface with verified findings and three-part self-surfacing recommendation for command discovery and surface curation.
- scope: GUI coverage gaps, CLI surface coverage, Scientia pipelines, gamification, command discovery, surface registry, self-surfacing gate

## Vox GUI-Native Language Roadmap (April 2026)
- source: docs/src/architecture/vox-gui-native-roadmap-2026.md
- summary: Executable roadmap for 30 tasks across 8 phases to make Vox into a GUI-native language whose compiler catches correctness invariants React+TypeScript cannot.
- scope: GUI-native language, compiler primitives, grammar unification, Web IR correctness, Vox GUI authoring DSL, dashboard re-authoring, MENS training, eight phases of work

## Vox Harness Audit — Graph-Backed (2026-07-30)
- source: docs/src/architecture/vox-harness-graph-audit-2026-07-30.md
- summary: Graphify-backed audit of Vox agent harness against Claude Code baseline: chat window has no agent loop, skill activation is unwired from GUI, and subsystems are structurally disconnected.
- scope: harness audit, structural disconnection, skill activation, model routing, chat quality, code graph, 29,315 nodes, 27 critical findings

## Vox Harness Parity Plan (2026-07-30)
- source: docs/src/architecture/vox-harness-parity-plan-2026-07-30.md
- summary: Sequenced remediation plan with six phases: fix model scorer and GUI chat wiring, then skill registry, then panel UX, then multi-sample eval gate.
- scope: model routing, GUI chat, skill system, agent parity, multi-provider support

## Vox Language & Syntax Audit (2026-08-08)
- source: docs/src/architecture/vox-language-syntax-audit-2026-08-08.md
- summary: Empirical audit of the Vox grammar, 743-file corpus, and description-surface drift; evidence base for the core-syntax convergence spec.
- scope: Vox grammar, parser implementation, corpus analysis, dialect coverage, formatter gaps, description-surface drift, SSOT violations

## Vox language feature maturity matrix (2026)
- source: docs/src/architecture/vox-language-feature-maturity-matrix-2026.md
- summary: Cross-cutting maturity table for language features across parse, HIR, typecheck, codegen, runtime, LSP, formatter, and tests.
- scope: language features, maturity tracking, feature gates, compiler pipeline, LSP support

## Vox language migrations hub (research)
- source: docs/src/architecture/vox-language-migrations-ssot-2026.md
- summary: Central index of breaking syntax migrations, codemods, and deprecation paths across compiler, React interop, and ID boundaries.
- scope: language migrations, deprecations, codemods, syntax evolution, breaking changes

## Vox Language Rules & Enforcement — Top-Level Plan
- source: docs/src/architecture/vox-language-rules-and-enforcement-plan-2026.md
- summary: Five-phase roadmap to close gap between stated language design and machine-checkable enforcement: stable diagnostic IDs, generated-hash codegen, runtime monitors.
- scope: language rules, enforcement, diagnostics, compiler policy, runtime safety

## Vox Language Rules — Phase 5: Effect System & Workflow Determinism (2026-05-09)
- source: docs/src/architecture/vox-language-rules-phase5-effects-determinism-2026.md
- summary: Multi-quarter plan to land the effect system on every public fn, enforce @uses and @pure decorators transitively, and forbid non-deterministic builtins in workflow bodies.
- scope: effect system, @uses decorator, @pure functions, workflow determinism, ADR-019 implementation

## Vox Memory Model: Audit & Value-Semantics Optimization Plan
- source: docs/src/architecture/vox-memory-model-audit-and-value-optimization-2026-06-05.md
- summary: Audit of Vox's value-semantics memory management and phased plan to optimize clone costs via copy-on-write and structural sharing.
- scope: value semantics, garbage collection, memory management, clone costs, copy-on-write, Rc sharing, interpreter optimization, benchmark results

## Vox Model Selection — 2026-Q2 Refresh
- source: docs/src/architecture/model-selection-2026-q2.md
- summary: Quarterly frontier model audit, recommended task-to-model mapping for Vox's routing pipeline, and rationale for the 2026-05-15 catalog refresh.
- scope: frontier models, May 2026, model landscape, pricing, benchmark analysis, premium aliases

## Vox playground architecture (research)
- source: docs/src/architecture/vox-playground-architecture-research-2026.md
- summary: Target shape for a browser-local Vox playground; architectural slices for syntax, formatting, execution (optional), and telemetry tiers.
- scope: web playground, wasm compilation, syntax diagnostics, formatter tier, execution tier, telemetry trust, deterministic mode, share URLs

## Vox Speech Audit Findings 2026
- source: docs/src/architecture/vox-speech-audit-findings-2026.md
- summary: ASR-primary audit findings including repository gaps, scorecard model, and runtime suite results for speech-to-code.
- scope: speech-to-code, ASR, Oratio, Whisper, audio ingress, dashboard, mobile STT

## Vox Speech CI Gates Proposal 2026
- source: docs/src/architecture/vox-speech-ci-gates-proposal-2026.md
- summary: Proposal to separate required and advisory CI gates for speech-to-code quality and prevent regression.
- scope: CI gates, speech-to-code, KPI, canary, ASR, testing strategy

## Vox Speech Improvement Backlog 2026
- source: docs/src/architecture/vox-speech-improvement-backlog-2026.md
- summary: Prioritized backlog of speech-to-code improvements with owners, effort estimates, and sequencing for ASR/surface/pipeline gaps.
- scope: speech-to-code, backlog, audio ingress, CUDA, mobile, streaming, dashboard

## Vox Speech Surface Inventory 2026
- source: docs/src/architecture/vox-speech-surface-inventory-2026.md
- summary: Audited inventory of microphone, speech-capture, and speech-to-code entry points across editor, app, dashboard, CLI, MCP, HTTP.
- scope: speech surfaces, Oratio, Sherpa, Web Speech API, MCP tools, editor webview, mobile

## Vox stdlib & interp gap audit (2026-05-23)
- source: docs/src/architecture/vox-stdlib-gap-audit-2026-05-23.md
- summary: Audit of discrepancies between what committed .vox scripts call and what the binary executes; findings handoff for remediation.
- scope: stdlib, interpreter, vox scripts, missing namespaces, regex, path functions, lexer bugs

## Vox v1.0 LLM-Target Implementation Plan (2026)
- source: docs/src/architecture/v1-llm-target-implementation-plan-2026.md
- summary: Phased implementation roadmap (P0–P5) for CR-L0..CR-L8 LLM-target criteria with owners, dependencies, corpus budgets, CI contract, risk register, and rollback policy.
- scope: v1.0, LLM-target, implementation plan, CR-L criteria, measurement infrastructure, corpus engineering, council decisions

## Vox v1.0 Readiness Snapshot — 2026-05-22 (revision 2)
- source: docs/src/architecture/2026-05-22-v1-readiness-snapshot.md
- summary: Measured state of every v1.0 release-criteria gate (CR-L/CR-P/CR-E/CR-A/CR-D) captured 2026-05-22 post fix-all session; block-GA umbrella green.
- scope: v1.0 release criteria, block-GA gates, CR-L humaneval, CR-L3 repair-corpus, CR-L4 plan-fidelity, CR-A1 cyclomatic, CR-D3 CLI coverage, CR-E2 bundle size, audit gates

## vox-container vs WASM Sandbox (2026-05-08)
- source: docs/src/architecture/vox-container-vs-wasm-2026-05-08.md
- summary: Audit clarifying vox-container's dual roles (deployment-codegen vs runtime sandbox) and evaluating WASM as partial replacement for runtime path.
- scope: vox-container, WASM, sandboxing, Docker, Podman, skill execution, capability analysis

## vox-gui Surface Map (graphify, 2026-06-14)
- source: docs/src/architecture/vox-gui-surface-map-2026-06-14.md
- summary: Complete edge map of vox-gui front-end derived from graphify AST+semantic extraction: 2,176 nodes, 4,154 edges, 229 communities across four layers.
- scope: GUI architecture, surface map, god nodes, IPC boundary, shared primitives, Tauri command modules, React components, design principles audit

## Vox-Populi Extraction Follow-Up Plan (2026)
- source: docs/src/architecture/vox-populi-extraction-followup-plan-2026.md
- summary: Honest accounting of code-motion work after plugin-system foundation landed; plan for extracting vox-populi mens/tensor, transport, and related modules into plugin scaffolds.
- scope: plugin architecture, vox-populi extraction, vox-plugin-mens-candle-cuda, vox-plugin-tensor-burn-wgpu, vox-plugin-populi-mesh, slim-core optimization, ABI versioning, feature-gating, plugin loader

## vox-runtime-rn mobile cross-compile (Android + iOS)
- source: docs/src/architecture/vox-runtime-rn-mobile-cross-compile.md
- summary: Toolchain setup and build commands for cross-compiling vox-runtime-rn to four Android and iOS architectures.
- scope: vox-runtime-rn, mobile, Android, iOS, cross-compile, NDK

## Vox: The Agentic Foundation for 2026
- source: docs/src/architecture/vox-marquee-explainer-2026.md
- summary: High-level marketing and positioning explainer for Vox as an agentic-native operating surface, describing low K-complexity philosophy and three pillars.
- scope: Vox positioning, agentic autonomy, K-complexity, Oratio, MENS, Populi, zero-config deployment, Marquee application

## VoxDB Audit & Condensation — Implementation Plan
- source: docs/src/architecture/2026-08-01-voxdb-audit-condensation-plan.md
- summary: Nine-task sequential execution plan re-baselining vox-db schema from 219 to 171 live tables by default, quarantining 48 behind opt-in Cargo feature with transactional migration.
- scope: vox-db, schema condensation, quarantine feature, task breakdown, test disposition, migration strategy

## VoxMens Fine-Tuning Boundaries — Research Findings (2026-06-21)
- source: docs/src/architecture/voxmens-finetuning-boundaries-research-2026-06-21.md
- summary: Deep-research evidence on fine-tuning boundaries for VoxMens hub-and-spoke design: hubs, harness, low-resource DSL adaptation.
- scope: VoxMens, fine-tuning, Qwen3, LoRA, tool-use, open-weight LLM, adapter serving

## VoxMens Hub-and-Spoke: SSOT, Per-Spoke Model Selection, and Generalization Beyond QLoRA — Research & Audit
- source: docs/src/architecture/voxmens-hub-and-spoke-ssot-research-2026-06-18.md
- summary: Audit of existing VoxMens architecture with research findings on hub-and-spoke topology, per-spoke model selection, and training method generalization.
- scope: VoxMens, hub-and-spoke, model selection, training methods, LoRA, QLoRA, adapters, inference routing, model registry

## VoxScript portability substrate: research and findings (2026-09)
- source: docs/src/architecture/voxscript-portability-substrate-research-2026.md
- summary: Audit of Vox execution tiers and research on portability substrate, WASM determinism, and platform-specific constraints.
- scope: VoxScript, portability, execution tiers, interpreter, WASM, native compilation, determinism, floating-point, GPU compute, sandboxing, Linux Landlock, macOS Seatbelt, Windows Job Objects

## Vox–React backend interop audit (2026)
- source: docs/src/architecture/vox-react-backend-interop-audit-2026.md
- summary: Code-anchored audit of Vox as backend/API provider for React, with wire-parity findings and remediation plans.
- scope: Vox-React interop, API contract, OpenAPI, TypeScript client, Axum server

## Warp Terminal Research Findings (2026)
- source: docs/src/architecture/warp-research-findings-2026.md
- summary: Systematic scan of Warp terminal codebase for design patterns and gaps. AGPL-licensed source unavailable; design reference only.
- scope: Warp, terminal emulator, AGPL-3.0, license analysis, command-signatures, input classification, natural language detection, design patterns, crate tier analysis

## Web App Archetype Coverage Map (2026)
- source: docs/src/architecture/web-app-archetype-coverage-2026.md
- summary: Strategic coverage map of 21 web-app archetypes against Vox substrate, with blocker analysis and cross-cutting infrastructure spine.
- scope: web apps, archetypes, CRUD, SaaS, marketplace, real-time chat, analytics, e-commerce, feature gaps, prioritization, cross-cutting infrastructure, scoring rubric

## Web Bootstrap Emission — Migration Plan (2026)
- source: docs/src/architecture/web-bootstrap-emission-migration-2026.md
- summary: Migration plan to emit web app bootstrap (entry.tsx, vox-app.tsx, runtime-install.ts) from Vox, achieving parity with RN.
- scope: web bootstrap, codegen, entry point, router emission, runtime install, web target, TypeScript emission, mental-tracker app

## WebIR / HIR split-brain inventory (2026)
- source: docs/src/architecture/webir-hir-split-brain-inventory-2026.md
- summary: Baseline split-brain map of dual codegen paths, projection seams, Tauri hooks, and guards against semantic drift.
- scope: WebIR, HIR, codegen, compiler, Tauri

## Weighted deep planning manual
- source: docs/src/architecture/planning-meta/03-weighted-deep-planning-manual.md
- summary: Comprehensive planning reference with token-weighted depth guidance for high-fidelity plans of complex, high-risk initiatives.
- scope: planning methodology, weighted depth, risk classes, planning patterns, quality checklist

## Windows-to-macOS Application Handoff — September 2026
- source: docs/src/architecture/windows-macos-application-handoff-findings-2026-09.md
- summary: Inventory of Windows workstation software and practical macOS migration path with audited bootstrap.
- scope: Windows, macOS, application handoff, migration, AI-development bootstrap

## Work-loss audit + handoff (2026-05-24)
- source: docs/src/architecture/work-loss-audit-and-handoff-2026-05-24.md
- summary: Forensic audit of parallel-agent work-loss reports; concludes no commits destroyed, all work lives on branches.
- scope: git auditing, parallel agents, work recovery, branch inventory

## Workspace Crate & Plugin Audit + Implementation Plan (2026, v2)
- source: docs/src/architecture/crate-audit-and-plan-2026.md
- summary: Verified 4-axis audit + 50-task phased plan (P0-P5): critical findings, plugin duplication, fan-in pressure, discoverability drift, external deps, structural splits.
- scope: workspace crates, plugin boundary, crate organization, build hygiene, architectural planning

## Workspace dependency audit (2026-05)
- source: docs/src/architecture/workspace-dependency-audit-2026.md
- summary: Evidence-driven audit of workspace pins, duplicate majors, intentional deferrals, and dependency normalization status.
- scope: dependencies, workspace, Cargo, version pinning

## Workspace Reorg Outcome (2026-05-08)
- source: docs/src/architecture/2026-05-08-workspace-reorg-outcome.md
- summary: Outcome report: 5 of 10 phases delivered, 36% orchestrator build-time win, 74% CLI win, new crate shapes, remaining work.
- scope: workspace-reorg, outcome-report, build-time-results, crate-extraction, orchestrator, CLI, queue, MCP, deferred-work

## Workspace test inventory (2026)
- source: docs/src/architecture/test-inventory-2026.md
- summary: Regenerable reference for Rust tests, golden Vox, E2E specs, harness patterns, and test runtime metrics.
- scope: test inventory, Rust tests, golden Vox, E2E tests, test harness, nextest, JUnit metrics

## Zero-Annotation Severity-Graded Debugging — Feasibility Research
- source: docs/src/architecture/auto-debugging-zero-annotation-research-2026-06-18.md
- summary: Research on surfacing program behavior without manual annotations: capture (Pernosco/eBPF proven), selectivity (Log2), severity inference (DeepLV ~84% AUC).
- scope: omniscient debugging, time-travel debugging, automatic instrumentation, eBPF, log selectivity, severity inference, Vox interpreter

## Zero-Copy Codegen Research and Implementation Findings (2026)
- source: docs/src/architecture/zero-copy-codegen-findings-2026.md
- summary: Research findings on zero-copy Rust emission: ownership modes, escape analysis, memory churn reduction.
- scope: codegen, Rust emission, ownership analysis, memory allocation, performance optimization
