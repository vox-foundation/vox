---
title: "T3 vs. Vox Harness & Router Blueprint: Best-of-All-Worlds Synthesis (2026)"
description: "Head-to-head comparison and extraction ledger mapping T3 Code and T3 Chat mechanics across 9 audited repositories into vox-orchestrator, vox-llm-egress, and vox-orchestrator-mcp."
category: "Architecture SSOTs"
status: "research"
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-7"
---

# T3 vs. Vox Harness & Router Blueprint: Best-of-All-Worlds Synthesis (2026)

<!-- kb:summary -->
This document synthesizes the head-to-head architectural comparison between the 9 audited T3 ecosystem repositories (`pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, `helioschat/app`, `lulkebit/t3-cloneathon`, `guitaripod/omnichat`, `shaltielshmid/NotT3Chat`, and `TomDoesTech/t3-chat`) and Vox (`vox-orchestrator`, `vox-llm-egress`, `vox-orchestrator-mcp`). It maps both the 8 completed Rust extractions (`Phases 208–209`) and the newly identified Wave 2 adoption candidates (`orchestrationV2IdTable.ts` wire ID interning, `ClaudeAdapterV2` `ENABLE_TOOL_SEARCH` cache-write guard, `NotT3Chat` `RepetitionGuard`, `t3-cloneathon` Multi-Model Consensus fan-out, and `omnichat` Battery Unit pre-flight gating).
<!-- /kb:summary -->

## 1. Head-to-Head Architecture Matrix (`T3 Ecosystem` vs. `Vox`)

| Dimension | T3 Ecosystem (`t3code` / `t3router` / `pi-t3chat` / `thom-chat` / `helioschat` / `t3-cloneathon` / `omnichat` / `NotT3Chat`) | Vox (`vox-orchestrator` / `vox-llm-egress` / `vox-orchestrator-mcp`) | Best-of-All-Worlds Synthesis |
|---|---|---|---|
| **Operational Status Gating** | `t3router`: `getModelStatuses` (`operational`, `degraded`, `outage`) | `ModelOperationalStatus` (`Operational`, `Degraded`, `Outage`) in `models/spec.rs`, `registry.rs`, `select.rs` | `Outage` hard-excluded (`-1e9`); `Degraded` demoted (`-200.0`) when `Operational` alternatives exist |
| **Reasoning Effort & Search Wire Normalization** | `t3router` / `thom-chat`: `reasoningEffort` (`low`/`medium`/`high`) + `:online` OpenRouter suffix | `ReasoningEffortWire`, `T3WireOptions`, `normalize_egress_request_for_t3_options` in `vox-llm-egress/src/wire.rs` | Provider-agnostic `ReasoningEffort` normalized at `vox-llm-egress` wire boundary |
| **Post-Turn Cost & Credit Reconciliation** | `t3router` `poll_credit_delta` (8x 200ms + 9th final check) + `thom-chat` `/api/v1/generation` + `omnichat` D1 Battery Unit (`1 BU = $0.001`) 4-step ledger | `CostProvenanceTier`, `TurnCostDeltaReceipt`, `reconcile_turn_cost_delta` in `conversational_routing.rs` | 3-tier cost provenance (`EstimatedTokenCount`, `ProviderGenerationLookup`, `SubscriptionCreditDelta`) |
| **Resumable Turn Streaming** | `thom-chat` Convex continuation + `NotT3Chat` SignalR `SemaphoreSlim(1)` atomic mid-stream multi-tab join + `t3code` `orchestrationV2IdTable.ts` / `orchestrationV2DedupedBodies.ts` | `TurnStreamEventKind`, `SequencedTurnChunk`, `ResumableTurnStreamRegistry` (`replay_after(seq)`) in `models/spec.rs` | Monotonically sequenced chunk buffer supporting mid-turn reconnect replay |
| **Lazy MCP Tool Grouping** | `pi-t3chat/mcp.ts` (`list_mcps`, `list_mcp_tools`, `call_mcp`, `MAX_WRAPPER_ROUNDS = 6`) + `t3code` `ClaudeAdapterV2.ts` `ENABLE_TOOL_SEARCH="true"` override | `LazyMcpGroupRegistry` (`list_mcp_servers`, `list_mcp_tools`, `resolve_call_mcp`, `MAX_LAZY_MCP_WRAPPER_ROUNDS = 6`) in `external_mcp.rs` | Collapses external MCP tool definitions into 3 meta-tools to eliminate 10x prompt-cache write bloat |
| **Ephemeral Privacy Posture** | `helioschat` 5-point Temporary Chat mode + `t3router` `isEphemeral` + `thom-chat` `privacy_mode` | `SessionPrivacyPosture` (`Standard`, `ZeroDataRetention`, `EphemeralLocal`) in `models/spec.rs` and `select.rs` | Enforces `require_zdr` on router selection and suppresses persistence/memory side effects |
| **Spoke-Aware Prompt Enhancement** | `thom-chat` `POST /api/enhance-prompt` | `EnhancedPromptReceipt` & `enhance_user_prompt_for_spoke` in `conversational_routing.rs` | Wraps terse user requests with target spoke (`rust`, `vox`, `ts`, `python`) verification criteria |

---

## 2. Completed Rust Implementation Ledger (`Phases 208–209` — Verified)

| Requirement ID | Source Inspiration | Target Vox Module | Implemented Rust Symbols & Unit Tests |
|---|---|---|---|
| `VOX-ROUTER-T3-01` | `t3router/src/t3/models.rs` (`get_model_statuses`) | `crates/vox-orchestrator/src/models/{spec.rs,registry.rs,select.rs}` | `ModelOperationalStatus`, `ingest_t3_status_indicator`, `t3_operational_status_outage_excluded_and_degraded_demoted` |
| `VOX-ROUTER-T3-02` | `t3router/src/t3/config.rs`, `thom-chat/+server.ts` | `crates/vox-orchestrator/src/models/{spec.rs,select.rs}` | `ReasoningEffort`, `ModelSelectionRequest::with_t3_controls`, `t3_reasoning_effort_and_web_search_augments_boost_capable_candidates` |
| `VOX-ROUTER-T3-03` | `t3router/src/t3/client.rs` (`poll_credit_delta`), `thom-chat/+server.ts` | `crates/vox-orchestrator/src/models/registry/conversational_routing.rs` | `CostProvenanceTier`, `TurnCostDeltaReceipt`, `reconcile_turn_cost_delta`, `t3_cost_delta_reconciliation_updates_observed_cost` |
| `VOX-ROUTER-T3-04` | `t3router/src/t3/client.rs`, `thom-chat/+server.ts` | `crates/vox-llm-egress/src/{lib.rs,wire.rs}` | `ReasoningEffortWire`, `T3WireOptions`, `normalize_egress_request_for_t3_options`, `build_request_with_t3_options` |
| `VOX-HARNESS-T3-01` | `thom-chat/+server.ts`, `helioschat/streamProcessor.ts` | `crates/vox-orchestrator/src/models/spec.rs` | `TurnStreamEventKind`, `SequencedTurnChunk`, `ResumableTurnStreamRegistry`, `t3_resumable_turn_stream_registry_replays_after_sequence_and_isolates_privacy` |
| `VOX-HARNESS-T3-02` | `pi-t3chat/mcp.ts` (`list_mcps`, `list_mcp_tools`, `call_mcp`) | `crates/vox-orchestrator-mcp/src/{external_mcp.rs,lib.rs}` | `LazyMcpGroupRegistry`, `MAX_LAZY_MCP_WRAPPER_ROUNDS = 6`, `lazy_mcp_group_registry_supports_list_servers_list_tools_and_call_resolution` |
| `VOX-HARNESS-T3-03` | `helioschat` Temporary Mode, `thom-chat` `privacy_mode` | `crates/vox-orchestrator/src/models/{spec.rs,select.rs}` | `SessionPrivacyPosture` (`should_persist_history`, `should_inject_memories`, `requires_zdr_routing`) |
| `VOX-HARNESS-T3-04` | `thom-chat` `/api/enhance-prompt` | `crates/vox-orchestrator/src/models/registry/conversational_routing.rs` | `EnhancedPromptReceipt`, `enhance_user_prompt_for_spoke`, `t3_enhance_user_prompt_for_spoke_is_deterministic_and_spoke_aware` |

---

## 3. High-Value Wave 2 Candidates Discovered in Audit (`t3code` `80455376` & Cloneathon Repos)

1. **Wire Snapshot ID Table Interning & Body Deduplication (`t3code` `orchestrationV2IdTable.ts` & `orchestrationV2DedupedBodies.ts`):** Interning repeated `id`/`*Id`/`*Ids` fields into a single response-level `ids[]` table and setting `textOmitted: true` when `TurnItem` text duplicates `messages` records cuts thread-open payload sizes dramatically.
2. **Windows NTFS Junction-Safe Worktree Removal (`t3code` `apps/server/src/vcs/worktreeRemoval.ts`):** Unlinking worktrees via `fs.rm` + `git worktree prune` after verifying `.git` admin linkage prevents Git `< 2.54` from traversing into NTFS directory junctions on Windows.
3. **1024-Slot Power-of-2 Ring-Buffer `RepetitionGuard` (`NotT3Chat` `backend/Program.cs:1080-1145`):** O(1) per-token hash ring buffer (`BufferSize = 1024`, `MaxTokenRep = 32`, `p ∈ [3, 12]`, `MinGramRep = 5`) that detects and aborts degenerate LLM token/n-gram loops in real time.
4. **Parallel Multi-Model Consensus Streaming & Heuristic Quality Scoring (`lulkebit/t3-cloneathon` `consensus/route.ts` & `response-quality-analyzer.ts`):** Multiplexing parallel model completions over a single SSE stream (`consensus_update`, `consensus_complete`) with automated readability/completeness scoring.
5. **Tree Branching `divergencePoint` Diffing (`guitaripod/omnichat` `branch-comparison.tsx`):** Computing the exact shared ancestor index across N conversation branches so side-by-side branch comparison highlights only the post-divergence turns.
