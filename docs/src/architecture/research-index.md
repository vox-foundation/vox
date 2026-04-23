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

## Core SSoT
- [V0.5 Core SSoT](v0.5-core-ssot.md) — Version 0.5 core architecture specifications.

## Data Storage
- [Data Storage SSOT (2026)](data-storage-ssot-2026.md) — Greenfield target state for how Vox persists, represents, and governs data across libSQL/Turso, `contracts/`, JSONL/log spools, and Rust in-memory types; seventy-eight numbered findings (F1–F78).
- [Data Storage Migration Backlog (2026)](data-storage-migration-backlog-2026.md) — Ticket-sized work items (M-00 through M-78) across six phases, with owners, blockers, and acceptance criteria; cross-indexed to SSOT findings.
- [Data Storage Lint & CI Spec (2026)](data-storage-lint-and-ci-spec-2026.md) — Concrete `vox ci data-storage-guard` subcommand, clippy denies, `deny.toml` bans, grep rules, Cursor rule, and CI wiring that enforce the SSOT.

## Documentation Platform

- [Shiki, mdBook & Doc Platform Evaluation (2026-04-22)](shiki-mdbook-doc-platform-research-2026.md) — Quantified feature matrix across 7 doc platforms (mdBook, Zola, VitePress, Starlight, Docusaurus, MkDocs, Nextra). Identifies Shiki `^4.0.1` as already a dependency of `vox-vscode`; proposes eliminating `highlight-vox.js` grammar drift via a `mdbook-shiki-vox` preprocessor and a medium-term migration to Starlight. Covers LLM-friendly documentation formats, `llms.txt` standard, and AI-first documentation architecture principles.

## User Interface & Dashboard
- [Vox Dashboard Migration Research (2026-04-22)](dashboard-migration-research-2026.md) — Architectural decisions and prerequisites for migrating the Vox orchestration UI from an editor-bound VS Code webview to a standalone Axum-served SPA.

## Gamification & Identity
- [Ludus Identity & GitHub Integration Research (2026)](ludus-identity-github-integration-research-2026.md) — Architecture for decentralized Ludus profile storage, GitHub account linking via Device Flow, and contribution-based XP scoring.
- [Ludus Security & Anti-Cheat Research (2026)](ludus-security-and-anti-cheat-research-2026.md) — Multi-layered defense strategy including reputation-weighted scaling, proof-of-contribution verification, and community-driven peer auditing.

