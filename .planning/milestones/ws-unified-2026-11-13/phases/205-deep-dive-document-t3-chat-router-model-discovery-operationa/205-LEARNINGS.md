---
phase: 205
phase_name: "deep-dive-document-t3-chat-router-model-discovery-operationa"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 7
  lessons: 0
  patterns: 4
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 205 Learnings: deep-dive-document-t3-chat-router-model-discovery-operationa

## Decisions

### Mapped t3router ModelStatus operational indicators to Phase 208 ModelOpe...
Mapped t3router ModelStatus operational indicators to Phase 208 ModelOperationalStatus health gating in vox-orchestrator::models

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 205-01-SUMMARY.md

---

### Mapped pi-t3chat 3-tool MCP discovery wrapper (list_mcps, list_mcp_tools...
Mapped pi-t3chat 3-tool MCP discovery wrapper (list_mcps, list_mcp_tools, call_mcp) to Phase 209 vox-orchestrator-mcp bridge

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 205-01-SUMMARY.md

---

### Documented ModelInfo catalog discovery, getModelStatuses (operational, d...
Documented `ModelInfo` catalog discovery, `getModelStatuses` (`operational`, `degraded`, `outage`), and `getAllModelBenchmarks` (`T3-ROUTER-SURFACE-01`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 205-01-SUMMARY.md

---

### Documented getCustomerData, getSubscriptionData, getPricingProducts, aut...
Documented `getCustomerData`, `getSubscriptionData`, `getPricingProducts`, `auth.getActiveSessions` WorkOS cookie rotation, and `send_with_credits` / `poll_credit_delta` (`T3-ROUTER-SURFACE-02`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 205-01-SUMMARY.md

---

### Documented /api/chat request payload (reasoningEffort, includeSearch, se...
Documented `/api/chat` request payload (`reasoningEffort`, `includeSearch`, `searchLimit`) and SSE event stream (`T3-ROUTER-SURFACE-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 205-01-SUMMARY.md

---

### Documented pi-t3chat dual-mode tool calling (ToolCallDeltaAccumulator, T...
Documented `pi-t3chat` dual-mode tool calling (`ToolCallDeltaAccumulator`, `ToolCallTranslator`, `ToolRefusalDetector`) and 3-tool MCP wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`) (`T3-ROUTER-SURFACE-04`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 205-01-SUMMARY.md

---

### Verified with cargo run -p vox-doc-pipeline -- --lint-only --paths archi...
Verified with `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3chat-router-and-wire-protocol-research-2026.md` (`no hard errors`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 205-01-SUMMARY.md

## Lessons

## Patterns

### Pre/post-turn balance delta reconciliation for streaming endpoints lacki...
Pre/post-turn balance delta reconciliation for streaming endpoints lacking in-band cost reporting

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 205-01-SUMMARY.md

---

### operational-status-gating
operational-status-gating

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 205-01-SUMMARY.md

---

### pre-post-turn-credit-delta
pre-post-turn-credit-delta

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 205-01-SUMMARY.md

---

### lazy-mcp-discovery-wrapper
lazy-mcp-discovery-wrapper

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 205-01-SUMMARY.md

## Surprises

