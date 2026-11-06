---
title: "Autonomous Telemetry-Driven Routing Learning, Arena Comparison & Axis Harness v3.3"
description: "Architecture SSOT for Vox v3.3: closed-loop telemetry model routing learning, side-by-side Arena comparison, turn-level conversation branching & Jujutsu checkpoint rewind, sub-transcript tool-result clearing, and deterministic .vox lifecycle hooks."
category: "Architecture SSOTs"
status: "current"
topic: "model-routing"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
---

# Autonomous Telemetry-Driven Routing Learning, Arena Comparison & Axis Harness v3.3

<!-- kb:summary -->
Vox Milestone v3.3 closes the loop between centralized telemetry intelligence, interactive human preference evaluation, and workspace harness ergonomics across `vox-orchestrator`, `vox-orchestrator-mcp`, and `vox-gui`. It introduces signed remote Community Intelligence Bundle ingestion (`Ed25519` + Empirical Bayes $k \ge 20, n \ge 50$), Side-by-Side Arena turn comparison with online Bradley-Terry updates and DPO tuple harvesting, turn-level conversation branching paired with Jujutsu (`vox-vcs`) checkpoint rewind, zero-LLM-cost sub-transcript tool-result clearing, and deterministic `.vox` lifecycle hooks with Oracle tier escalation.
<!-- /kb:summary -->

## 1. Closed-Loop Centralized Telemetry & Model Routing Auto-Improvement (`ROUTE-LEARN-01..04`)

### 1.1 Remote Community Intelligence Sync (`intelligence_bundle.rs`)
- **Endpoint & Secret Resolution**: `sync_community_intelligence_from_remote` in `crates/vox-orchestrator/src/models/intelligence_bundle.rs` resolves the upload endpoint via `vox_secrets::resolve_secret(SecretId::VoxTelemetryUploadUrl)` (with optional `SecretId::VoxTelemetryUploadToken` Bearer auth), derives `/v1/intelligence/bundle.json`, and fetches the signed bundle over `vox_http_client::client_builder()`.
- **Cryptographic & Privacy Verification**: Before persisting to `~/.vox/cache/community-intelligence.v1.json`, `CommunityIntelligenceBundle::verify` enforces:
  1. Valid `Ed25519` signature over the canonical signing payload via `vox_crypto::signing::PublicKey`.
  2. Strict $k$-anonymity ($k \ge 20$ distinct installations) and minimum sample size ($n \ge 50$) on every `ModelTaskPrior` and `ModelBenchSpokePrior` cell.

### 1.2 Empirical Bayes Priors & Scoreboard Feedback (`scoring.rs` & `selection_queries.rs`)
- **Spoke-Specific Priors**: `ModelRegistry::apply_community_intelligence_bundle` blends Empirical Bayes `model_bench_spoke_priors` into each candidate's `ModelScore::quality_score` and seeds cold-start contextual bandit `arm_stats` pseudo-counts (`n_calls == 0`) so Thompson sampling starts from community-verified priors.
- **Graded Quality Feedback Boost**: `scoreboard_feedback_boost` in `crates/vox-orchestrator/src/models/scoring.rs` incorporates graded `quality_score` values in `(0.0, 1.0)` (`0.6 * measured + 0.4 * arena_quality`) while keeping ungraded `0.0`/`1.0` defaults neutral.
- **Automatic Refresh**: Both `ModelRegistry::inject_evidence_from_db` and `Orchestrator::refresh_model_scoreboard` load and apply the verified cache bundle automatically.

---

## 2. Side-by-Side Arena Turn Comparison & DPO Preference Learning (`ARENA-COMPARE-01..04`)

- **Dual-Arm Turn Dispatch**: `chat_arena_compare` (`crates/vox-gui/src/commands/chat_arena_branch.rs`) dispatches a prompt to **Arm A** (primary auto-routed model) and **Arm B** (challenger model, defaulting to the local 5-spoke MENS model `mens/qwen3.5-27b:five-spoke-qlora-r2`), returning `ArenaTurnComparisonDto`.
- **Online Bradley-Terry & Bandit Updates**: `ModelRegistry::record_arena_preference_outcome` and `chat_arena_vote` apply an online Bradley-Terry step ($K = 0.12$) to `quality_a` / `quality_b` (clamped to `[0.05, 0.95]`), update contextual bandit `(successes, failures)` posteriors, persist the winning reply into the session transcript, and record a DPO preference tuple (`prompt`, `chosen`, `rejected`).
- **GUI Surface**: `ArenaComparisonCard.tsx` and `ChatTranscript.tsx` render a toggleable split view (`data-testid="chat-arena-split-card"`) with 1-click voting buttons (`Prefer A`, `Prefer B`, `Tie / Both Good`).

---

## 3. Turn-Level Conversation Branching & Jujutsu Checkpoint Rewind (`BRANCH-REWIND-01..04`)

- **Non-Destructive Session Forking**: `chat_fork_session` (`fork_session_impl`) clones the message prefix up to `fork_from_message_id` into a new `ChatSessionDto` with `parent_session_id` and `fork_from_message_id` lineage metadata preserved.
- **Deterministic Turn Rewind**: `chat_rewind_to_turn` (`rewind_to_turn_impl`) rewinds the session cursor to `target_message_id` and restores or synthesizes the turn's `vcs_checkpoint_op_id` (`vox-vcs` / Jujutsu operation ID).
- **GUI Controls**: Every user/assistant `MessageBubble` in `ChatTranscript.tsx` exposes `⑂ Branch here` (`data-testid="chat-turn-fork-btn-${id}"`) and `↺ Rewind` (`data-testid="chat-turn-rewind-btn-${id}"`), with active lineage displayed in `data-testid="chat-session-branch-badge"`.

---

## 4. Sub-Transcript Surgical Tool-Result Clearing (`CONTEXT-SURGERY-01..04`)

- **Zero-LLM-Cost Compaction**: `clear_stale_tool_results_in_messages` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs` walks the conversation history and replaces stale `role == "tool"` or verbose JSON tool outputs older than `keep_recent_tool_outputs` with `[cleared to save context — N BPE tokens reclaimed]`, leaving all user prompts, assistant reasoning, and recent tool outputs intact.
- **Interactive Context Meter Control**: `ContextWindowMeter.tsx` and `ChatExecutionRail.tsx` expose a 1-click `Clear Tools` button (`data-testid="context-meter-clear-tools-btn"`) wired to `clear_session_tool_results`, displaying cumulative reclaimed BPE tokens.

---

## 5. Deterministic `.vox` Lifecycle Hooks & Oracle Model Escalation (`HOOKS-ORACLE-01..04`)

- **Out-of-Context Lifecycle Guardrails**: `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/lifecycle_hooks.rs` executes deterministic `PreToolUse`, `PostToolBatch`, and `Stop` hooks inside `dispatch_tool_calls_batch` with zero LLM token overhead:
  - Blocks read/write access to `.env*` secret files (except `.env.example`).
  - Blocks direct manual edits to generated SSOT artifacts (`docs/src/*.generated.md`, `contracts/reports/gui-surface-*.v1.json`, `.cursorignore`, `.aiignore`, `.aiexclude`).
  - Automatically rewrites `cargo fmt --all` invocations to `vox run scripts/fmt.vox -- --all`.
  - Injects a post-mutation verification reminder after file edits and blocks empty or unverified stop states.
- **Oracle Tier Escalation**: `consult_oracle_model_advisory` resolves the highest-precision reasoning model (`SelectionAxes::QUALITY_FIRST`, complexity `9`) via `vox_orchestrator::models::select` to provide structured architectural and root-cause verification advisories (`OracleConsultationAdvisory`), surfaced in `ChatExecutionRail.tsx` (`data-testid="execution-rail-hooks-status"`).
