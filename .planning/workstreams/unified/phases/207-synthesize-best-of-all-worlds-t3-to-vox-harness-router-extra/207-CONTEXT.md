# Phase 207: Synthesize "Best of All Worlds" T3-to-Vox Harness & Router Extraction Blueprint - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Cross-reference all 5 T3 ecosystem repositories (`t3code`, `t3router`, `pi-t3chat`, `thom-chat`, `helioschat`) against Vox's current `vox-orchestrator`, `vox-llm-egress`, `vox-orchestrator-mcp`, and `vox-gui` crates, producing `docs/src/architecture/t3-vs-vox-harness-router-blueprint-2026.md` and `contracts/reports/t3-vox-feature-extraction-ledger.v1.json`.

</domain>

<decisions>
## Implementation Decisions

### Extraction Scope
- Phase 208 (`VOX-ROUTER-T3-01..03`): `ModelOperationalStatus` health gating, `ReasoningEffort` & search-grounding wire normalization, and `CostProvenanceTier` + `TurnCostDeltaReceipt` credit/cost delta tracking.
- Phase 209 (`VOX-HARNESS-T3-01..03`): `ResumableTurnStreamRegistry` with sequence-cursor replay & out-of-band cancellation, `LazyMcpGroupRegistry` (`list_mcp_servers`, `list_mcp_tools`, `resolve_call_mcp`), and `enhance_user_prompt_for_spoke` + `SessionPrivacyPosture`.

</decisions>
