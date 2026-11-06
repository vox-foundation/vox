---
phase: 204-deep-dive-document-pingdotgg-t3code-agent-harness-control-su
plan: 01
subsystem: architecture
tags:
  - t3code
  - orchestration-v2
  - mcp-server
  - permission-modes
metrics:
  completed_date: "2026-10-24"
---

# Phase 204 Plan 01: Deep-Dive & Document `pingdotgg/t3code` Agent Harness Control Surface Summary

Authored `docs/src/architecture/t3code-harness-architecture-research-2026.md` and `docs/src/architecture/t3code-worktree-permissions-mcp-research-2026.md` covering `pingdotgg/t3code`'s Orchestration V2 graph, Effect-TS transactional outbox, 11-dimension `ProviderCapabilities`, cross-provider `ContextTransfer`/`ContextHandoff`, 4 permission modes with non-escalating privilege ceilings, hidden-ref Git checkpoints, PR auto-settlement, and the 11-tool HTTP `/mcp` server (`0` `vox-doc-pipeline` errors).
