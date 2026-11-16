---
phase: 205-deep-dive-document-t3-chat-router-model-discovery-operationa
plan: 01
subsystem: architecture-docs
tags: [t3router, pi-t3chat, model-routing, sse, mcp, tool-calling]
requires: [T3-CLONE-GRAPH-02]
provides:
  - docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md
affects: [vox-orchestrator, vox-llm-egress, vox-orchestrator-mcp]
tech-stack:
  added: []
  patterns: [operational-status-gating, pre-post-turn-credit-delta, lazy-mcp-discovery-wrapper]
key-files:
  created:
    - docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md
  modified: []
key-decisions:
  - "Mapped t3router ModelStatus operational indicators to Phase 208 ModelOperationalStatus health gating in vox-orchestrator::models"
  - "Mapped pi-t3chat 3-tool MCP discovery wrapper (list_mcps, list_mcp_tools, call_mcp) to Phase 209 vox-orchestrator-mcp bridge"
patterns-established:
  - "Pre/post-turn balance delta reconciliation for streaming endpoints lacking in-band cost reporting"
requirements-completed:
  - T3-ROUTER-SURFACE-01
  - T3-ROUTER-SURFACE-02
  - T3-ROUTER-SURFACE-03
  - T3-ROUTER-SURFACE-04
duration: 6min
completed: 2026-10-24
---

# Phase 205 Plan 01: T3 Chat Router, Telemetry, and Wire Protocol Research Summary

**Completed deep-dive research and architecture mapping for `vibheksoni/t3router` and `vibheksoni/pi-t3chat` in `docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md`.**

## Accomplishments
- Documented `ModelInfo` catalog discovery, `getModelStatuses` (`operational`, `degraded`, `outage`), and `getAllModelBenchmarks` (`T3-ROUTER-SURFACE-01`).
- Documented `getCustomerData`, `getSubscriptionData`, `getPricingProducts`, `auth.getActiveSessions` WorkOS cookie rotation, and `send_with_credits` / `poll_credit_delta` (`T3-ROUTER-SURFACE-02`).
- Documented `/api/chat` request payload (`reasoningEffort`, `includeSearch`, `searchLimit`) and SSE event stream (`T3-ROUTER-SURFACE-03`).
- Documented `pi-t3chat` dual-mode tool calling (`ToolCallDeltaAccumulator`, `ToolCallTranslator`, `ToolRefusalDetector`) and 3-tool MCP wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`) (`T3-ROUTER-SURFACE-04`).
- Verified with `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3chat-router-and-wire-protocol-research-2026.md` (`no hard errors`).
