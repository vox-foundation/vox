---
title: "research-index"
category: "reference"
status: "current"
training_eligible: false
---
# Vox Architecture & Research Index (2026)

This file tracks the single source of truth for Vox architecture, research findings, and strategic explainers.

## Strategic & Value Proposition
- [Vox Marquee Explainer 2026](vox-marquee-explainer-2026.md) — The 500-word explainer for the Vox foundation and agentic-native philosophy.
- [v1.0 Release Criteria](v1-release-criteria.md) — Benchmark criteria for the stable 1.0 release.

## Audits & Assessments
- [Comprehensive Audit V2 (2026)](comprehensive-audit-v2-2026.md) — Deep dive into core system integrity.
- [Model Orchestration SSOT — Audit & Convergence Plan (2026-04-20)](model-orchestration-ssot-audit-2026.md) — Audit of MENS / Populi / OpenRouter routing, telemetry, automatic model discovery, Clavis secret plane, and decentralized key sync across mesh nodes; proposes a single SSOT and ~70 numbered FIX items.
- [Workspace Health & Dependency Governance Research (2026-04-20)](workspace-health-audit-research-2026.md) — Research on transition from fragile heuristics to policy-driven architectural enforcement; implements automated dependency sprawl and performance regression gates.
- [Telemetry-Driven Cost Accounting Architecture (2026-04-21)](telemetry-driven-cost-accounting-research-2026.md) — Documentation of the self-correcting pricing loop (v59) that transitions model orchestration from static estimates to verifiable ground-truth spend reporting and VfM optimization.
- [Inference Tuning Resolution Research (2026-04-21)](inference-tuning-resolution-research-2026.md) — Research into precedence-based parameter resolution (Override > Registry > Base) for temperature and top_p across the Vox ecosystem.
- [Vox Orchestration Build Stabilization Findings (2026-04-21)](build-stabilization-findings-2026.md) — Documentation of schema hardening reconciliation, type mismatch resolution, and feature gate alignment across Populi, Oratio, and Mens.
- [Scientia × Mesh / Model-Routing Integration Research (2026-04-23)](scientia-mesh-integration-research-2026.md) — Fundamental limitations of the current Scientia publication pipeline (static novelty blend, thin introspection, no loop back into routing, scoreboard too coarse to encode subjective model strengths), and a concrete proposal to close the loop: new `DiscoverySignalFamily` (`ProviderObservation`, `ModelCapabilityEvidence`), new `FindingCandidateClass` (`ModelCapabilityAtlas`, `ProviderReliabilityAtlas`), a `model_profile_learning` overlay consumed by `ModelRegistry::inject_learned_profiles` + new `ScoringWeights` fields, an active probe-suite harness, and a new quarterly publication output (Vox Provider Atlas) distributed through the existing Scientia adapter stack. Trait sketches, schema stubs, and a 5-phase rollout included.
- [Next-Generation AI Orchestrator: Systemic Flaws, Power User Demands, and Production Design Patterns (2026-04-23)](nextgen-orchestrator-research-2026.md) — Comprehensive synthesis of enterprise AI orchestration failure modes (abstraction over-engineering, Python GIL bottlenecks, protocol mistranslation), quantified native-vs-interpreted performance benchmarks, multi-tier FinOps governance requirements, hallucination prevention strategies (HMAC receipt verification, entropy scoring, LLM-as-judge), mesh GPU disaggregated inference semantics, multi-agent coherence protocols (MCP vs. AISP), and a Vox-specific gap analysis identifying 11 implementation priorities.
- [FableForge End-to-End Roadmap Audit (2026-04-23)](fableforge-roadmap-audit-2026-04-23.md) — Document-level audit of the 280-task / 14-phase FableForge visual-novel platform roadmap. Identifies 6 critical consistency errors (schema version collision, non-existent task references, phase/design conflicts), 5 redundant task pairs, 4 P0 safety tasks buried in Phase 8, 19 tasks pruned from the MVP critical path, 7 under-estimated effort items, 15 claims requiring real-code verification, and produces a re-ranked top-30 execution list compressed to a 14-week MVP path across 6 delivery units.
- [FFScript Panel Schema Spec v0.2.0 (2026-04-23)](ffscript-panel-schema-spec-2026.md) — Authoritative Zod schema for the Panel type (T-021/T-022/T-031). Resolves the schema version-naming collision from the audit, defines AspectRatio → workflow-dims mapping replacing hardcoded resolutions, specifies Background/Foreground/Bubble/Caption sub-schemas, documents the v0.1→v0.2 migration runner, and lists corrected acceptance criteria.
- [FFScript Mutation API Spec (2026-04-23)](ffscript-mutation-api-spec-2026.md) — Full TypeScript interface for the `FFScriptDoc` wrapper class (T-041–T-045, T-051–T-052, T-054): 15 public methods (panel CRUD, dialogue, choice/branch, scene, placement, bubble, batch), branded ID types, MutationResult/JSON-Patch return shape, typed error hierarchy, Immer integration notes, Convex integration pattern, and corrected effort estimate (L, not M).
- [FFScript Linter Engine Design (2026-04-23)](ffscript-linter-design-2026.md) — TypeScript interfaces and default rule catalogue for the pluggable FFScript linter (T-046–T-050, T-035–T-039 collapsed). Updated: LintFix uses serializable FixKind descriptor (Vox DiagnosticFix pattern) so fixes can be stored in Convex and replayed from CLI.

## Competitive & Ecosystem Research
- [Warp Terminal Research Findings (2026-04-29)](warp-research-findings-2026.md) — Systematic scan of warpdotdev/warp (~60 crates). Identifies AGPL-3.0-only license incompatibility blocking direct vendoring, maps Tier-S/A/B/C targets, confirms `deny.toml` already rejects AGPL, establishes clean-room reimplement path for `command-signatures-v2` → `vox-exec-grammar`, and recommends Zed (Apache-2.0) as the preferred upstream for B-tree/text primitives. Produced ADR-026, `crates/vox-exec-grammar` scaffold, `.voxindexingignore`, and `fuzzy-search` feature wire-up.

## FableForge Implementation Files

Ready-to-use TypeScript drop-ins in `docs/src/architecture/fableforge-impl/`. Verify field names against the real schema before merging.

- [panel-schema.ts](fableforge-impl/panel-schema.ts) — Runnable Zod schema for Panel/Scene/Background/Bubble/CharacterPlacement, `aspectRatioDims()`, `migrateV01toV02()`, `checkPanelInvariants()`. Implements T-021, T-022, T-025, T-031.
- [linter-engine.ts](fableforge-impl/linter-engine.ts) — Full `LintEngine` class with 9 default rules, serializable `FixKind` descriptors, `applyFix()` resolver, Convex + publish-gate hooks. Implements T-046–T-050, T-035–T-039 (collapsed).
- [cascade-delete.ts](fableforge-impl/cascade-delete.ts) — Convex soft-delete (T-192), hard-delete cascade across 12 child tables (T-191), 30-day purge job, `requireGameOwnerOrAdmin` auth guard (T-193), ESLint rule stub, siloing test suite scaffold (T-205).
- [generation-orchestrator.ts](fableforge-impl/generation-orchestrator.ts) — `ImageOrchestrator` (Vox `ModelScorer` pattern): `ScoringWeights`-driven provider ranking, circuit breaker in Convex, attempt recording for billing. Implements T-004, T-007, T-129, T-130.

## Core SSoT
- [V0.5 Core SSoT](v0.5-core-ssot.md) — Version 0.5 core architecture specifications.
- [Terminal Exec Policy SSOT (2026)](terminal-exec-policy-ssot.md) — Live SSOT for the PowerShell-first terminal exec policy. Scopes the claim to host-side allowlisting and output parsing, explicitly disclaims any codegen-fluency superiority over Bash, and documents why a separate "PowerShell spoke" in MENS is not justified.
- [Agent Shell Fluency Eval Design (2026)](agent-shell-fluency-eval-design-2026.md) — Design-only A/B eval (20 tasks × 2 shells × 5 trials) for the codegen-fluency claim. Not run; not required by current policy. On-shelf until a proposal depends on the wider claim.

## Data Storage
- [Data Storage SSOT (2026)](data-storage-ssot-2026.md) — Greenfield target state for how Vox persists, represents, and governs data across libSQL/Turso, `contracts/`, JSONL/log spools, and Rust in-memory types; seventy-eight numbered findings (F1–F78).
- [Data Storage Migration Backlog (2026)](data-storage-migration-backlog-2026.md) — Ticket-sized work items (M-00 through M-78) across six phases, with owners, blockers, and acceptance criteria; cross-indexed to SSOT findings.
- [Data Storage Lint & CI Spec (2026)](data-storage-lint-and-ci-spec-2026.md) — Concrete `vox ci data-storage-guard` subcommand, clippy denies, `deny.toml` bans, grep rules, Cursor rule, and CI wiring that enforce the SSOT.
- [Coolify Deployment Contract](../ci/deploy-contract.md) — Automated CI/CD pipeline definition for the Hetzner VPS including LLM auto-healing loops.

## Documentation Platform

- [Shiki, mdBook & Doc Platform Evaluation (2026-04-22)](shiki-mdbook-doc-platform-research-2026.md) — Quantified feature matrix across 7 doc platforms (mdBook, Zola, VitePress, Starlight, Docusaurus, MkDocs, Nextra). Identifies Shiki `^4.0.1` as already a dependency of `vox-vscode`; proposes eliminating `highlight-vox.js` grammar drift via a `mdbook-shiki-vox` preprocessor and a medium-term migration to Starlight. Covers LLM-friendly documentation formats, `llms.txt` standard, and AI-first documentation architecture principles.

## User Interface & Dashboard
- [Vox Dashboard Migration Research (2026-04-22)](dashboard-migration-research-2026.md) — Architectural decisions and prerequisites for migrating the Vox orchestration UI from an editor-bound VS Code webview to a standalone Axum-served SPA. Superseded by [ADR 024](../adr/024-dashboard-axum-spa.md).

## Gamification & Identity
- [Ludus Identity & GitHub Integration Research (2026)](ludus-identity-github-integration-research-2026.md) — Architecture for decentralized Ludus profile storage, GitHub account linking via Device Flow, and contribution-based XP scoring.
- [Ludus Security & Anti-Cheat Research (2026)](ludus-security-and-anti-cheat-research-2026.md) — Multi-layered defense strategy including reputation-weighted scaling, proof-of-contribution verification, and community-driven peer auditing.

