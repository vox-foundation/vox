# Constraints

Synthesized from SPEC classifications. `type` is a best-effort bucketing into api-contract | schema | nfr | protocol based on title/summary/scope; many source SPECs are broader architecture SSOTs than a single bucket implies — the bucket is a routing aid, not a claim the source is exclusively that kind of artifact.

## Anti-foot-gun planning standard
- source: docs/src/architecture/planning-meta/05-anti-foot-gun-planning-standard.md
- type: nfr
- content: Tier 1 normative standard defining blocker classes and mandatory controls for planning document quality.
- scope: planning documents, quality standards, blocker classes, enforcement, planning-meta, normative policy

## Task catalog authoring spec
- source: docs/src/architecture/planning-meta/07-task-catalog-authoring-spec.md
- type: schema
- content: Specification for writing atomic planning tasks with dependencies, weighting, acceptance evidence, and anti-foot-gun checks.
- scope: task authoring, atomic tasks, task schema, dependencies, acceptance evidence, task lifecycle, weighting rubric

## Milestone and gate definition spec
- source: docs/src/architecture/planning-meta/08-milestone-gate-definition-spec.md
- type: nfr
- content: Tier 1 normative specification for defining milestones and gates with explicit pass/fail evidence, escalation rules, and stop conditions.
- scope: milestones, gates, evidence classes, escalation, stop conditions, pass criteria, fail criteria

## Exception and deferral policy
- source: docs/src/architecture/planning-meta/09-exception-deferral-policy.md
- type: nfr
- content: Operational policy for planning exceptions and deferrals with allowed classes, mandatory metadata, expiry, review cadence, and retirement workflow.
- scope: exceptions, deferrals, exception classes, mandatory metadata, expiry policy, review cadence, retirement workflow

## Document maintenance protocol
- source: docs/src/architecture/planning-meta/10-document-maintenance-protocol.md
- type: protocol
- content: Tier 1 normative lifecycle, versioning, ownership, and change-control protocol for the planning-meta corpus.
- scope: document lifecycle, versioning, ownership, change control, maintenance protocol, supersession, consistency protocol

## Question gate standard for planning
- source: docs/src/architecture/planning-meta/12-question-gate-standard.md
- type: nfr
- content: Tier 1 normative rules governing when planning requests must trigger clarification versus auto-expansion or inference.
- scope: planning intake, question gate, clarification trigger, EVPI threshold, auto-expansion, intake classification, attention budget

## Vox Crate Ledger (v1.0 Frozen Core)
- source: docs/src/architecture/history/2026-04-core-ten-charter.md
- type: schema
- content: Binding ledger defining ten core crates for v1.0 release; all other crates must be feature-gated or frozen.
- scope: core crates, v1.0 track, compiler, CLI, database, secrets, runtime, orchestration, MENS, search, validation

## Crate Organization Follow-up — SSOT, Naming, and Sprawl
- source: docs/src/architecture/2026-05-08-crate-org-followup-design.md
- type: api-contract
- content: Specification for crate organization follow-up: SSOT/discoverability drift fixes, naming consistency, and 5 build-time extractions ranked by impact.
- scope: crate-organization, SSOT, naming, build-time, crate-extraction, vox-gamify, vox-orchestrator, vox-package

## Naming & Anti-Entanglement Guards (2026-05-08)
- source: docs/src/architecture/2026-05-08-naming-and-guards-design.md
- type: nfr
- content: Design for LLM-navigation naming conventions and CI drift guards: 8 crate renames, anti-entanglement rules, where-things-live map.
- scope: naming-convention, crate-rename, anti-entanglement-guards, architecture-check, documentation-map, vox-code-audit, vox-secrets, vox-package

## Workspace Reorg — Build-Time + Layered Architecture (2026-05-08)
- source: docs/src/architecture/2026-05-08-workspace-reorg-design.md
- type: api-contract
- content: Design for workspace reorganization: 6-layer architecture model, build-time wins targets, 10-phase decomposition of monoliths, layer enforcement CI guard.
- scope: workspace-architecture, layer-model, build-time-optimization, crate-extraction, orchestrator, CLI, database, plugins, architectural-guard

## Detector & Heuristic Rule SSOT — Design
- source: docs/src/architecture/2026-05-09-detector-rule-ssot-design.md
- type: nfr
- content: Design for single source of truth system for detector regex patterns and Scientia heuristics, with YAML SSOT, rule-pack loader, benchmarking tool.
- scope: detector-rules, code-audit, SSOT, regex-patterns, rule-pack, rule-loader, benchmarking, Scientia, heuristics, vox-rule-pack

## VoxDB Audit & Condensation — Schema Re-baseline and Quarantine Design
- source: docs/src/architecture/2026-08-01-voxdb-audit-condensation-design.md
- type: schema
- content: Live-data audit of canonical VoxDB store (219 tables: 116 LIVE, 82 DORMANT, 21 DEAD) plus design for re-baselining schema with quarantine feature and safe migration path.
- scope: vox-db, schema, SQLite, table quarantine, Cargo feature, migration, BASELINE_VERSION

## Autonomous Browser Driver & Accessibility-Tree Navigation SSOT (2026)
- source: docs/src/architecture/agent-browser-driver-ssot-2026.md
- type: nfr
- content: Architectural specification for compact semantic accessibility snapshotting with empirically-validated performance targets for agent browser interaction.
- scope: browser automation, accessibility tree, semantic compression, token efficiency, monotonic indexing, action targeting

## Agent Chat UX, Noise Control & Panel Information Design (2026-07-30)
- source: docs/src/architecture/agent-chat-ux-and-noise-research-2026-07-30.md
- type: nfr
- content: Architectural specification for agent panel design using ARIA live regions, WCAG 4.1.3 conformance, and severity-based notification taxonomy.
- scope: agent UI design, ARIA live regions, notification severity, information design, accessibility conformance, panel patterns

## Testing & Regression-Gating an Agent Harness (2026-07-30)
- source: docs/src/architecture/agent-harness-testing-and-regression-gating-research-2026-07-30.md
- type: api-contract
- content: Specification establishing tau-bench as deterministic outcome-checking standard and pass^k reliability metric for harness regression gates.
- scope: harness testing, regression gates, tau-bench methodology, deterministic evaluation, benchmark contamination, pass^k reliability metric

## Agentic Secretary: Research Synthesis — Dynamic Task Management, Memory Transparency, and Chat-Driven Orchestration (2026-06-17)
- source: docs/src/architecture/agentic-secretary-research-2026-06-17.md
- type: nfr
- content: Specification establishing Four-Type Memory Model, RAM/hard-drive separation, task-management DAG patterns, and secretary agent architecture for Vox.
- scope: secretary agent, task management, dynamic replanning, memory model, context window engineering, hopper architecture, supervisor/worker orchestration

## AgentOS & Agent-Computer Interface SSOT (2026)
- source: docs/src/architecture/agentos-ssot-2026.md
- type: api-contract
- content: Single baseline for contract-first ACI envelopes, mutation classification, guardrails, checkpointing, and semantic retrieval bridges. Defines contracts and crate boundaries.
- scope: AgentOS, Agent-Computer Interface, ACI envelopes, mutation classification, guardrails, checkpointing

## Antigravity (agy) Credits, Auth & Limitations
- source: docs/src/architecture/antigravity-credits-auth-and-limitations-2026-06-19.md
- type: api-contract
- content: Single source of truth for Antigravity Gemini credit integration: OAuth-only auth, per-project quota/rate limits, non-queryable credit balance, integration with Vox Clavis.
- scope: Antigravity CLI, Gemini credits, OAuth authentication, rate limits, quota management, Vox Clavis integration, credential-aware model selection

## Classification Taxonomy SSoT
- source: docs/src/architecture/classification-ssot-2026.md
- type: nfr
- content: Single source of truth for Vox component classification, system subsystems, and nomenclature invariants.
- scope: classification system, subsystem naming, crate prefixes, component responsibility

## CLI vs compilerd params (split-brain backlog)
- source: docs/src/architecture/cli-compilerd-params-ssot.md
- type: api-contract
- content: Canonical reference for CLI parameter contract convergence, documenting duplicate clap vs serde structs and target unified shape.
- scope: CLI parameters, compilerd, parameter contract, split-brain, clap vs serde

## RFC: Closures in Vox (Phase G — Bucket-A v0.6)
- source: docs/src/architecture/closures-rfc-2026-05-23.md
- type: api-contract
- content: Technical specification for first-class closures in Vox: grammar, type rules, lowering, codegen, and 10-day implementation plan.
- scope: closures, anonymous functions, Vox grammar, type system, codegen

## Codegen-TS Domain Boundary & CR-F2 Correction (Handoff)
- source: docs/src/architecture/codegen-ts-domain-boundary-and-cr-f2-correction-2026.md
- type: nfr
- content: Defines Rust vs TypeScript emit domain boundary (logic→Rust, browser/GUI→TypeScript), corrects CR-F2 release criteria.
- scope: codegen domain, Rust emit, TypeScript emit, release criteria, split discipline

## Context isolation — SSOT
- source: docs/src/architecture/context-isolation-ssot-2026.md
- type: nfr
- content: Policy for multi-repo context boundaries: treat workspace roots as trust boundaries, prefer repo-local SSOT, archive historical research.
- scope: context boundaries, workspace trust, repo isolation

## Context Management Mechanics — Verified Research (2026-07-31)
- source: docs/src/architecture/context-management-mechanics-research-2026-07-31.md
- type: api-contract
- content: Verified technical specification: exact API parameters for clear_tool_uses/compact, memory tool, prompt-caching mechanics, and measured token results.
- scope: context management, Claude API, tool clearing, prompt caching, compaction, memory tool

## Context Window Management — Design Spec (recovered 2026-06-20)
- source: docs/src/architecture/context-window-management-design-2026-06-20.md
- type: schema
- content: Technical design SSOT for unified ContextWindow object model spanning hot/warm/cold/frozen tiers, retrieval, and graphify temporal join (recovered + T4.2 status).
- scope: context window, data model, archival tiers, retrieval, graphify, GUI surfaces

## Contracts outside contracts/index.yaml — classification (2026)
- source: docs/src/architecture/contracts-non-indexed-classification-2026.md
- type: schema
- content: Classification policy for YAML/JSON under contracts/; defines classes (generated reports, derived registries, nested schemas, fixtures) and indexing criteria.
- scope: contracts, schema registry, classification, SSOT, index management

## Cross-Platform Compatibility SSOT (2026)
- source: docs/src/architecture/cross-platform-ssot-2026.md
- type: nfr
- content: Canonical source of truth: supported platform matrix, toolchain invariants, platform-specific enforcement rules, GPU probing, development constraints.
- scope: cross-platform compatibility, platform matrix, toolchain invariants, enforcement rules, CI verification

## Cryptography Policy SSoT
- source: docs/src/architecture/cryptography-ssot-2026.md
- type: nfr
- content: Canonical cryptographic invariants: allowed/banned primitives, AEAD/hash standards, TLS provider policy, build toolchain constraints.
- scope: cryptography policy, crypto primitives, TLS providers, build constraints, AEAD, hashing

## Data Storage Lint & CI Spec (2026)
- source: docs/src/architecture/data-storage-lint-and-ci-spec-2026.md
- type: schema
- content: Concrete lint and CI rules implementing data storage regression gates: machine-checkable policy, sub-checks, enforcement specifications.
- scope: data storage, CI enforcement, lint rules, contract validation, regression gates

## Data Storage SSOT (2026)
- source: docs/src/architecture/data-storage-ssot-2026.md
- type: schema
- content: Single source of truth for how Vox persists, represents, and governs data across four persistence tiers: libSQL/Turso, JSONL spools, content-addressed artifacts, and ephemeral cache.
- scope: Data Storage, Tier A—libSQL/Turso, Tier B—JSONL spools, Tier C—CAS, Tier D—ephemeral cache, Contracts, Schema, DDL, SSOT

## Deep Research Self-Correction and Knowledgebase Architecture SSOT (2026)
- source: docs/src/architecture/deep-research-self-correction-and-knowledgebase-ssot-2026.md
- type: nfr
- content: SSOT for empirical code verification, multi-wave contradiction resolution, and durable knowledgebase architecture.
- scope: self-correction, empirical verification, multi-wave research, knowledgebase, epistemic resolution, architecture, competitive superiority

## Distribution SSOT
- source: docs/src/architecture/distribution-ssot.md
- type: api-contract
- content: Single source of truth for install tiers (minimal/default/full), dependency closures, crates.io publish set, and released binaries; governs voxup and vox doctor.
- scope: distribution tiers, minimal tier, default tier, full tier, publish set, binaries, bundles, vox-install-policy

## Environment Variable SSOT — Design & Developer Handoff
- source: docs/src/architecture/env-var-ssot-design-and-handoff-2026-06-15.md
- type: nfr
- content: Design for dynamic, auto-tracking unified SSOT for all environment variables and Clavis secrets; extends config-hygiene gate with phased handoff implementation plan.
- scope: environment variables, Clavis secrets, config-hygiene gate, registry.v1.yaml, env-var recognition, secret management

## Phase 5 Sub-Spec: Native React / React Native Component Interop (2026)
- source: docs/src/architecture/external-frontend-interop-phase5-component-interop-subspec-2026.md
- type: api-contract
- content: Specification for importing React/RN components into Vox: import grammar, opaque-extern types, JSX-tag registration, provider/styling wiring, and code-anchored implementation plan.
- scope: React interop, component imports, JSX tags, type strategy, RN mapping

## Feature growth and boundaries SSOT (2026)
- source: docs/src/architecture/feature-growth-boundaries.md
- type: nfr
- content: Governance SSOT for Vox feature sprawl limits: max 40 workspace crates, 50 kLOC compiler limit, projection-parity drift gate, crate modularization policy.
- scope: crate sprawl limits, LOC budgets, projection parity, feature governance

## FFScript Linter Engine Design (T-046)
- source: docs/src/architecture/ffscript-linter-design-2026.md
- type: api-contract
- content: TypeScript interface spec for pluggable FFScript linter: LintRule shape, 10+ default rules, severity levels, autofix, integration points with publish gate and CLI.
- scope: linter engine, lint rules, validation, publish gate, TypeScript interface

## FFScript Mutation API Spec (T-041)
- source: docs/src/architecture/ffscript-mutation-api-spec-2026.md
- type: api-contract
- content: Full TypeScript interface for FFScriptDoc mutation API: 15 methods, branded types, JSON-Patch undo/redo, error hierarchy, Immer internals, Convex integration pattern.
- scope: mutation API, FFScriptDoc, undo/redo, JSON Patch, type safety

## FFScript Panel Schema Spec (v0.2.0)
- source: docs/src/architecture/ffscript-panel-schema-spec-2026.md
- type: schema
- content: Authoritative Zod schema for Panel type in FFScript v0.2.0: aspect ratio, backgrounds, placements, bubbles, content rating, invariants, and v0.1→v0.2 migration.
- scope: Panel schema, Zod validation, FFScript types, content rating, aspect ratio

## Git Concurrency Policy
- source: docs/src/architecture/git-concurrency-policy.md
- type: protocol
- content: Rules for safe git use by agentic workers: static denylist of unsafe commands, VCS capability tokens, commit trailers, secret scan gates, and telemetry.
- scope: git command restrictions, agentic git operations, GitExec wrapper, VCS capability tokens, commit trailers, secret scanning, telemetry events

## GUI Authoring Syntax (2026): Vox UI as Values (VUV)
- source: docs/src/architecture/gui-authoring-syntax-2026.md
- type: nfr
- content: Typed function-call view authoring surface replacing JSX; specifies syntax, lowering, phasing, and implementation status.
- scope: GUI authoring, Vox UI as Values, view expressions, token systems, style kwargs, event handlers

## vox-gui Performance Conventions
- source: docs/src/architecture/gui-perf-conventions-2026-06-14.md
- type: nfr
- content: Binding performance budgets and conventions: RAIL targets, animation rules, heavy-work offloading, list virtualization, and inline style discipline.
- scope: RAIL budgets, animation, performance, virtualization, heavy computation, compositor properties, INP target

## Inference Tuning Resolution Research 2026
- source: docs/src/architecture/inference-tuning-resolution-research-2026.md
- type: api-contract
- content: Specifies the 3-tier precedence mechanism for resolving LLM inference parameters (temperature, top_p) from MCP clients, environment variables, and tool defaults.
- scope: inference tuning, LLM provider parameters, MCP parameter resolution, registry integration, telemetry

## Internal Web IR Implementation Blueprint
- source: docs/src/architecture/internal-web-ir-implementation-blueprint.md
- type: nfr
- content: Specifies implementation plan, acceptance gates, and parity requirements for the Vox internal Web IR codegen layer.
- scope: Web IR, code generation, TypeScript/TSX emission, acceptance gates, parity testing

## Internal Web IR side-by-side schema
- source: docs/src/adr/internal-web-ir-side-by-side-schema.md
- type: schema
- content: Stable entrypoint for current-vs-target Web IR representation mapping (CI cross-link surface).
- scope: Web IR, parser, representation, schema, mapping

## RFC: Intra-project imports (Phase J)
- source: docs/src/architecture/intra-project-imports-rfc-2026-05-23.md
- type: api-contract
- content: Proposes cross-file import syntax and pub/private visibility model for Vox; minimum-viable module system for in-project file sharing.
- scope: intra-project imports, module system, public declarations, file resolver, import statement grammar

## RFC: @json_as(MyType) — typed JSON deserialization decorator
- source: docs/src/architecture/json-as-rfc-2026-05-24.md
- type: api-contract
- content: Proposes @json_as decorator for schema-typed JSON parsing; supports defaults, naming conventions, tagged enums, and strict mode.
- scope: JSON deserialization, type decorator, serde integration, tagged enums, type-safe parsing

## RFC: JSON ergonomics — strict-Option + pointer
- source: docs/src/architecture/json-ergonomics-rfc-2026-05-23.md
- type: api-contract
- content: Specifies Vox's Json API with strict Option discipline, RFC 6901 pointer traversal, and leaf coercion methods for unknown-schema JSON.
- scope: JSON API, JSON pointer, strict Option, Json type, unknown-schema traversal

## Mac-as-hub, Qwen3, and the quantization ladder — a decision record (2026-09-05)
- source: docs/src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md
- type: nfr
- content: Establishes Mac hub / 4080 spoke topology, Qwen3-32B base, quantization ladder tiers, unified-memory safety reserve requirement.
- scope: hub-and-spoke topology, Qwen3-32B base model, quantization ladder, Metal training, unified memory safety, Q4_K_M/Q5_K_M/Q6_K/Q8_0 rungs

## MCP Vox language exposure
- source: docs/src/architecture/mcp-vox-language-exposure.md
- type: protocol
- content: Documents workspace MCP tool/resource federation into vox-mcp orchestrator, collision rules, dispatch protocol, and tier policy.
- scope: MCP federation, @tool declarations, vox-mcp orchestrator, tool dispatch, collision resolution, resource protocol

## Local MENS Inference Engine & Apple Silicon Metal Optimization SSOT (2026)
- source: docs/src/architecture/mens-metal-optimization-ssot-2026.md
- type: nfr
- content: Establishes Metal inference architecture: 75% unified-memory ceiling rule, O(N²) decode bottleneck (KV-cache gap), performance targets (38tps parity), phased roadmap with P0-P2 gaps.
- scope: Metal inference architecture, unified memory budget, KV-cache optimization, decode throughput, Apple Silicon optimization, performance benchmarks

## Vox Mental-Health Tracker — Full-App Implementation Roadmap (2026)
- source: docs/src/architecture/mental-tracker-full-app-roadmap-2026.md
- type: api-contract
- content: Staged roadmap specifying libraries, component interop decisions, trauma-informed schema design, and codegen changes for React Native mental-health app.
- scope: mental-health tracker, React Native, calendar interop, append-only schema, event-sourcing, bottom-sheet, toast

## Mesh & Language-Level Distribution — SSOT & Upgrade Plan
- source: docs/src/architecture/mesh-and-language-distribution-ssot-2026.md
- type: api-contract
- content: Authoritative SSOT defining canonical mental model for distributed durable workflows, seven-phase upgrade plan with acceptance criteria per phase, release contracts v0.6/v0.7/v1.0.
- scope: mesh, distributed workflows, DurablePromise, content-addressed code, capability tokens, durable runtime

## MENS Distributed Training & Execution Plan (2026-05-09)
- source: docs/src/architecture/mesh-mens-distributed-training-and-execution-plan-2026.md
- type: protocol
- content: Distributed-AI track supplementing Mesh SSOT; defines 15 task IDs, model-loading/execution/training specs, content-addressed SafeTensors bundles, and MENS corpus alignment.
- scope: MENS, distributed training, model execution, model loading, SafeTensors, inference, task planning

## Mesh Phase 0 — Foundations Implementation Plan (2026-05-09)
- source: docs/src/architecture/mesh-phase0-foundations-plan-2026.md
- type: protocol
- content: TDD implementation specification for Phase 0: persisted lock map, lock-leader election, lease-gated dispatch, secret injection, TLS option, probe trait, SkillRuntime seam.
- scope: mesh, Phase 0, locks, lock leader, dispatch, secrets, TLS, probes, SkillRuntime

## Mobile RN + Expo + uniffi: comprehensive implementation spec (2026)
- source: docs/src/architecture/mobile-rn-expo-implementation-spec-2026.md
- type: nfr
- content: File-by-file, crate-by-crate implementation plan for shipping Vox to iOS and Android via React Native + Expo with cross-compiled vox-runtime through uniffi.
- scope: React Native, Expo, uniffi, iOS, Android, vox-runtime, codegen, mobile-emit

## Vox Model Autonomic System — L1/L2/L3 Design (2026-Q2)
- source: docs/src/architecture/model-autonomic-system-2026.md
- type: api-contract
- content: Continuous-discovery and auto-classification architecture with L1/L2/L3 loops replacing hand-curated model bootstrap; defines contracts, telemetry events, and CLI surfaces.
- scope: model discovery, model classification, ModelRegistry, L1 discovery, L2 classification, L3 council review

## Model Catalog SSOT — Architecture & Implementation Plan 2026
- source: docs/src/architecture/model-catalog-ssot-2026.md
- type: api-contract
- content: Audit of split-brain model catalog bugs and complete plan to achieve single source of truth with automatic model adoption; defines data flows, implementation waves, and traceability.
- scope: ModelRegistry, model discovery, pricing, scoring, routing, automatic adoption

## Model Orchestration SSOT — Audit & Convergence Plan (2026-04-20)
- source: docs/src/architecture/model-orchestration-ssot-audit-2026.md
- type: protocol
- content: Audit of model selection, orchestration, telemetry, discovery, and mesh-secret distribution; proposes single SSOT and concrete backlog of ~70 improvements across 6 stages.
- scope: MENS, Populi mesh, OpenRouter, model selection, telemetry, secrets, device pairing, vox-secrets

## Modern Model Selection, Pareto Routing, and Graduated Evidence SSOT
- source: docs/src/architecture/modern-model-selection-and-cost-accuracy-ssot-2026.md
- type: nfr
- content: Comprehensive 2026 frontier model landscape, cost-accuracy Pareto curves, reasoning token economics, 4-tier graduated evidence grounding, and multi-lane execution architecture.
- scope: model selection, Pareto routing, cost-accuracy tradeoff, reasoning models, evidence grounding, sync/background/plan lanes

## Multi-Agent VCS Replication — Phase 1 Implementation Plan (2026-05-03)
- source: docs/src/architecture/multi-agent-vcs-replication-impl-plan-phase1-2026.md
- type: protocol
- content: Step-by-step TDD implementation plan for Phase 1: local-only op-log gossip between agents on one machine with 16 tasks covering ~80 individual steps and complete code specifications.
- scope: convergence engine, op-log gossip, AgentChange, OpFragment, MergePolicy, jj-backend, local multi-agent

## Multi-Agent VCS Replication — Architecture Spec (2026-05-03)
- source: docs/src/architecture/multi-agent-vcs-replication-spec-2026.md
- type: protocol
- content: Architecture spec for op-log gossip on top of jj-lib and Populi mesh. Defines primitives, wire protocol, auto-merge/escalation policy, and four-phase rollout (local → conflict UX → mesh → policy/safety).
- scope: AgentChange, OpFragment, ConvergenceSet, MergePolicy, ConvergenceEngine, jj-lib, Populi mesh, op-log replication

## Multi-Repository Workspace Support Spec (2026-07-31)
- source: docs/src/architecture/multi-repository-workspace-support-spec-2026-07-31.md
- type: schema
- content: Design for multi-repository GUI support: workspaces table, folder-picker, VCS-kind detection, phased task breakdown with data model and GUI surface specs.
- scope: workspaces table schema, folder picker UI, GUI workspace switcher, vcs_kind detection, multi-repository support

## Pure-Rust Build: Eliminating C and MSVC Dependencies
- source: docs/src/architecture/no-c-no-msvc-2026-05-08.md
- type: nfr
- content: Specification of pure-Rust workspace build posture: eliminated C dependencies via PTX bundling, feature flags, pure-Rust backends. Residual and rationale documented.
- scope: build system, C dependency elimination, CUDA runtime, pure Rust, candle-kernels, PTX compilation, CI matrix

## Phase 1: Build Target Split Spec (2026)
- source: docs/src/architecture/phase1-build-targets-spec-2026.md
- type: api-contract
- content: Design specification for vox build --target=server|fullstack|client modes, vox emit client SDK, and vox init --kind=backend.
- scope: build targets, CLI flags, Vox.toml manifest, TypeScript SDK, Axum backend, Dockerfile optimization

## Phase 3: HTTP Ergonomics Decorators Spec (2026)
- source: docs/src/architecture/phase3-http-ergonomics-spec-2026.md
- type: api-contract
- content: Implementation specification for explicit HTTP method/path decorators (@cors, @auth, @rate_limit) on Vox endpoints with compile-time validation and OpenAPI output.
- scope: HTTP decorators, endpoint ergonomics, CORS, authentication, rate limiting, path parameters, OpenAPI, wire-format-v1

## End-to-End Pipeline Parity — SSOT
- source: docs/src/architecture/pipeline-parity-ssot-2026-06-14.md
- type: api-contract
- content: Technical contract requiring every Vox language feature to be wired identically across all IRs and emit targets, with hard build gates for compliance.
- scope: pipeline parity, HIR, WebIR, VoxIR, target backends, feature matrix, compile-time validation, parity testing

## Plugin System Redesign (2026)
- source: docs/src/architecture/plugin-system-redesign-2026.md
- type: api-contract
- content: Architecture specification for unified runtime-loadable plugin system supporting code and skill payloads with per-deployment distribution bundles.
- scope: plugin system, ABI design, code plugins, skill plugins, distribution bundles, CUDA, native dylibs

## Populi Mesh — A2A Durability Spec (S1, 2026-05-01)
- source: docs/src/architecture/populi-mesh-a2a-durability-spec-2026.md
- type: protocol
- content: SUPERSEDED design spec for SQLite-backed mesh store. Retained for historical context; VoxDb is the canonical implementation.
- scope: Populi Mesh, A2A Durability, SQLite, mesh store, MeshStore trait

## Populi Mesh — Config Baseline Spec (S1, 2026-05-01)
- source: docs/src/architecture/populi-mesh-config-baseline-spec-2026.md
- type: protocol
- content: Slice S1 child spec for [mesh] schema, sensible defaults, env-var precedence policy, and quickstart guide.
- scope: Populi Mesh, configuration, Vox.toml, [mesh] section, env vars

## Populi Mesh — Local Observability Spec (S1, 2026-05-01)
- source: docs/src/architecture/populi-mesh-local-observability-spec-2026.md
- type: protocol
- content: Slice S1 child spec establishing vox.mesh.* span-attribute namespace and trace propagation via W3C traceparent.
- scope: Populi Mesh, observability, tracing, OpenTelemetry, trace_id, MeshTraceContext

## Populi Mesh — Probe Correctness Spec (S1, 2026-05-01)
- source: docs/src/architecture/populi-mesh-probe-correctness-spec-2026.md
- type: protocol
- content: Slice S1 child spec for workstream W2: refactors probes behind HardwareProbe trait with mock harness and correctness criteria.
- scope: Populi Mesh, hardware probes, correctness, HardwareProbe trait, NVML, wgpu, DRM, Metal, DXGI

## Research / Scientia / telemetry channels (SSOT)
- source: docs/src/architecture/research-scientia-telemetry-channels.md
- type: api-contract
- content: Technical specification for three parallel data channels (ResearchEvent bus, research_metrics SQL, TelemetryEvent) defining roles, consumers, and bridge composition without cyclic dependencies.
- scope: ResearchEvent, research_metrics table, TelemetryEvent, telemetry contracts, vox-research-events, vox-db, telemetry

## Scientia × Mesh/Model-Routing Integration Research (2026)
- source: docs/src/architecture/scientia-mesh-integration-research-2026.md
- type: protocol
- content: Research proposal for closed-loop model/provider observation feedback into publication pipeline and routing layer via first-class signal families, learned-profile overlay, and Provider Atlas publication.
- scope: DiscoverySignalFamily, ModelCapabilityAtlas, ProviderReliabilityAtlas, model_profile_learning, provider observations, mesh integration, routing weights

## SCIENTIA Micro-Publication, SSOT Unification & Discovery-Surfacing Design (2026)
- source: docs/src/architecture/scientia-micropublication-ssot-and-surfacing-design-2026.md
- type: nfr
- content: Architectural design for claim-centric SSOT across nanopub/scholarly/social with human-gated discovery review, spec-compliant Trusty-URI signing, and multi-pillar accuracy fixes (bundling, embedding, conflict detection).
- scope: nanopublication, Trusty-URI, human approval, novelty verification, claim extraction, syndication, NoveltyEvidenceBundle, AtomicClaim

## Search & Retrieval SSOT (2026)
- source: docs/src/architecture/search-retrieval-ssot-2026.md
- type: protocol
- content: Canonical baseline for agent-facing search/retrieval across vox-db contracts, vox-search execution, orchestrator MCP tools, and dashboard transport.
- scope: search-retrieval, vox-db, vox-search, MCP-tools, hybrid-fusion, policy-knobs, corpus-backends

## vox share — Abuse Policy and ToS Reference (2026)
- source: docs/src/architecture/share-policy-2026.md
- type: nfr
- content: Compliance policy defining abuse contacts, takedown procedures, and privacy for vox share tunnel.
- scope: vox share, abuse policy, terms of service, Cloudflare, localhost.run, Tailscale, privacy, takedown

## Superpowers SSoT
- source: docs/src/architecture/superpowers-ssot.md
- type: api-contract
- content: Specification for Superpowers procedural agentic skills: 14 standard workflows, core philosophy, integration wiring, safety guardrails, and capability definitions.
- scope: superpowers, agentic skills, procedures, workflows, orchestration, safety guardrails

## Telemetry Trust (SSoT)
- source: docs/src/architecture/telemetry-trust-ssot.md
- type: nfr
- content: Canonical boundaries and trust policies for Vox telemetry: opt-in, locally-bounded, no PII by default.
- scope: telemetry, trust boundaries, data residency, policy, local-first analysis

## Telemetry unification design 2026
- source: docs/src/architecture/telemetry-unification-design-2026.md
- type: nfr
- content: Runtime architecture for unifying telemetry emission behind L1 facade with local collection and trace propagation.
- scope: telemetry, runtime architecture, event taxonomy, trace context, model performance metrics, facade crate

## Terminal Exec Policy SSOT (2026)
- source: docs/src/architecture/terminal-exec-policy-ssot.md
- type: nfr
- content: Live SSOT for PowerShell-first terminal execution policy: structured output, AST inspection, allowlist enforcement.
- scope: terminal execution, PowerShell, policy enforcement, AST validation, structured output, command allowlisting

## True workflow durability: corrected design
- source: docs/src/architecture/true-workflow-durability-design-2026.md
- type: protocol
- content: Audit-corrected design for durable workflow replay: execute workflow body in interpreter, intercept activity calls, journal results.
- scope: workflow durability, replay engine, activity interception, journaling, interpreter design, determinism

## Vox v1.0 Release Criteria (Hardened)
- source: docs/src/architecture/v1-release-criteria.md
- type: nfr
- content: Tiered, machine-verifiable v1.0 release criteria (Foundation/Distribution/GUI/Product tiers) with exact verification commands, artifact paths, and gate ordering.
- scope: v1.0 release, release criteria, CR-F Foundation, CR-K Distribution, CR-U GUI, CR-P/A/E/D/L Product, verification gates

## Vox application packaging SSOT (2026)
- source: docs/src/architecture/vox-application-packaging-ssot-2026.md
- type: api-contract
- content: Normative contract for shipping installable Vox applications (desktop/mobile) with declarative manifest, Tauri shell integration, and CI verification gates.
- scope: application packaging, desktop installers, mobile bundling, Tauri 2, deployment, vox compile, native shells

## Vox Axis Harness Reliability — Spec + Plan 2026-07-02
- source: docs/src/architecture/vox-axis-harness-reliability-spec-plan-2026-07-02.md
- type: api-contract
- content: Adversarially audited spec and TDD-ready execution plan: authenticated dispatch, durable op-log SSOT, single-daemon state, self-healing streams, context management, with contracts and CI gates.
- scope: Vox Axis, harness reliability, authenticated dispatch, durable op-log, single-daemon architecture, self-healing streams, context compaction, runtime consolidation

## Vox Axis STT Accuracy Design (2026-08-01)
- source: docs/src/architecture/vox-axis-stt-accuracy-design-2026-08-01.md
- type: nfr
- content: Phased design to fix voice-dictation accuracy: eval harness, correction rules, Parakeet-via-sherpa-onnx default backend, symbol expansion, Settings UI exposure with WER/CER gates.
- scope: STT accuracy, speech recognition, voice dictation, Parakeet-TDT, sherpa-onnx, code dictation, lexicon correction, platform packaging

## Vox diagnostic UX taxonomy (research)
- source: docs/src/architecture/vox-diagnostic-ux-ssot-2026.md
- type: schema
- content: Taxonomy for compiler and tooling diagnostics: stable IDs, severity ladder, human/LLM consumption, LSP mapping, CLI envelope format, and drift risk management.
- scope: diagnostics, compiler errors, LSP, severity levels, diagnostic IDs, CLI output format

## Vox GUI Browser Support (2026)
- source: docs/src/architecture/vox-gui-browser-support-2026.md
- type: nfr
- content: Architecture for embedded app preview, agent CDP live view, snapshot/ref driving, launch modes (Ephemeral/Named/Connect Chrome), Playwright validation, with layer map and semantic loops.
- scope: browser support, GUI preview, agent automation, CDP, chromiumoxide, Playwright, browser sessions

## Vox Harness Implementation Spec (2026-07-31)
- source: docs/src/architecture/vox-harness-implementation-spec-2026-07-31.md
- type: nfr
- content: Implementation-level specification for agent-harness behavior: exact file paths, function signatures, tests, tool-result budgeting, and local-model registration with TDD sequencing.
- scope: agent harness, tool-use loop, local models, context compaction, LLM routing

## Vox LSP capabilities matrix (research)
- source: docs/src/architecture/vox-lsp-capabilities-ssot-2026.md
- type: api-contract
- content: Capability inventory for crates/vox-lsp: validation path, diagnostics mapping, parity gaps vs vox check and IDE expectations.
- scope: LSP server capabilities, diagnostics mapping, hover, completions, code actions, workspace symbols, formatting, integration coverage

## Vox shell-tier stdlib SSOT (2026)
- source: docs/src/architecture/vox-shell-stdlib-ssot-2026.md
- type: nfr
- content: Specification of argv-first Rust builtins for std.fs, std.process, std.csv, std.io, std.agentos; separation from host shells.
- scope: stdlib, std.fs, std.process, std.csv, std.toml, std.yaml, std.io

## VoxMens Serving Topology Decision
- source: docs/src/architecture/voxmens-serving-topology-decision-2026-06-19.md
- type: nfr
- content: Specifies adapter multiplexing via DomainRouter, local base model selection at training time, and heterogeneous base constraints.
- scope: VoxMens, adapter serving, DomainRouter, base model selection, multi-LoRA, inference routing, heterogeneous bases

## Voxup Omnibus Installer Specification
- source: docs/src/architecture/voxup-omnibus-installer-spec-2026.md
- type: api-contract
- content: Architecture and implementation spec for hermetic voxup installer providing unified toolchain management across platforms.
- scope: voxup, installer, bootstrap script, hermetic toolchains, Node.js, WASM sysroot, PATH management, Vox CLI

## VUV Layered Layout Discipline — making Z-fighting and tier inversion structurally unrepresentable (2026)
- source: docs/src/architecture/vuv-layered-layout-discipline-2026.md
- type: schema
- content: Specifies five structural rules for VUV view trees: partitioning containers, Z-tier enums, Float parents, typed edges, and Mark-based jump targets.
- scope: VUV, layout discipline, Z-fighting prevention, Z-tiers, partitioning containers, floating surfaces, marks, tier inversion, compile-time validation

## VUV Naming Policy (2026)
- source: docs/src/architecture/vuv-naming-policy-2026.md
- type: nfr
- content: Specifies three-step identifier lifecycle: announce, alias with deprecation warning, remove. Registry-backed enforcement via vox migrate codemod.
- scope: VUV, naming, deprecation cycle, rename registry, codemod, identifier evolution, compatibility

## Where Things Live
- source: docs/src/architecture/where-things-live.md
- type: schema
- content: Flat lookup table — concept to crate. Canonical reference for workspace navigation and crate organization.
- scope: crate organization, workspace layout, repository structure, L0, L1, L2, L3, L4

## Wire Format v1 SSOT
- source: docs/src/architecture/wire-format-v1-ssot.md
- type: protocol
- content: Versioned specification for Vox type encoding over HTTP between backend and TypeScript/React consumer.
- scope: wire format, HTTP, serialization, query parameters, mutation encoding, type encoding
