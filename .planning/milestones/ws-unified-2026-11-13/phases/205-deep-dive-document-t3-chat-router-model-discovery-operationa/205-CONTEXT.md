# Phase 205: Deep-Dive & Document T3 Chat Router, Model Discovery, Operational Status & Wire Protocol - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Exhaustively analyze and document `vibheksoni/t3router` (`~/dev/t3router` at `5314bbbf0af66e724e3e07ffd458ef5919d1eed1`) and `vibheksoni/pi-t3chat` (`~/dev/pi-t3chat` at `b39873245362c33f68a9a3e044423f69769069f9`) in `docs/src/architecture/t3chat-router-and-wire-protocol-research-2026.md`.

</domain>

<decisions>
## Implementation Decisions

### Source Coverage
- Analyze `~/dev/t3router/src/t3/{models.rs,usage.rs,client.rs,config.rs}` for dynamic catalog scraping (`ModelInfo`), tRPC `getModelStatuses` / `getAllModelBenchmarks`, `getCustomerData` / `getSubscriptionData` / `getPricingProducts` / `auth.getActiveSessions` WorkOS session rotation, and `send_with_credits` (`poll_credit_delta` 8x200ms pre/post-turn balance polling).
- Analyze `~/dev/pi-t3chat/{tools.ts,mcp.ts,sse.ts,tokens.ts,chat.ts,proxy.ts,index.ts}` for `/api/chat` wire protocol, SSE event parser, dual-mode native + `tool:<name>` text-fallback tool calling with `ToolRefusalDetector`, and 3-tool MCP discovery wrapper (`list_mcps`, `list_mcp_tools`, `call_mcp`).

</decisions>
