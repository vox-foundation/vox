---
title: "Full Flywheel Chat Knob Matrix, OpenRouter vs Local-to-MENS Routing & Context Compaction Audit (2026)"
description: "Before-and-after full-matrix audit and engineering remediation of all chat modifying knobs, execution tracks, OpenRouter vs Local-to-MENS routing, strategy-driven context compaction, and five-spoke model training pipeline."
category: "Architecture SSOTs"
status: "current"
topic: "desktop-app"
domain: "gui"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
code_anchors:
  - "crates/vox-gui/ui/src/lib/buildChatTurn.ts"
  - "crates/vox-gui/src/commands/chat_turn.rs"
  - "crates/vox-gui/src/commands/context_budget.rs"
  - "crates/vox-gui/src/commands/control_plane.rs"
  - "crates/vox-gui/ui/src/components/surfaces/Loquela/DriveConsole.tsx"
  - "crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx"
  - "crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx"
  - "crates/vox-orchestrator-mcp/src/chat_tools/params.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs"
  - "crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs"
  - "crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs"
  - "crates/vox-orchestrator/src/models/registry.rs"
---

# Full Flywheel Chat Knob Matrix, OpenRouter vs Local-to-MENS Routing & Context Compaction Audit (2026)

<!-- kb:summary -->
This document records the Milestone v2.3 Before/After full-flywheel audit across all chat modifying knobs, execution tracks (`sync`, `background`, `plan`), interaction modes (`plan`, `act`, `verify`), runtime tiers (`local`, `mesh`, `cloud`, `auto`), clutch profiles (`free`, `efficiency`, `balanced`, `genius`), risk postures (`high`, `moderate`, `low`), OpenRouter vs Local-to-MENS model selection, strategy-aware BPE context compaction (`aggressive`, `balanced`, `conservative`), and the 25-problem 5-spoke + 15-problem organic search-to-compilation MENS training and evaluation harness.
<!-- /kb:summary -->

## 1. Executive Summary & Before/After Scorecard

| Flywheel Dimension | BEFORE (`v2.2` Baseline) | AFTER (`v2.3` Remediated) | Verification Artifact / Test |
| --- | --- | --- | --- |
| **Chat Modifying Knob End-to-End Fidelity** | **68.2%** (`15/22` knobs reached all 3 tracks; `oversight_level` & `kb_scope` dropped in `buildChatTurn.ts`, `mode` & `grounding_check_enabled` dropped on `Sync`, `scope_files` dropped on `Plan`) | **100.0%** (`24/24` keys wired across TS + Rust `Sync`, `Background`, and `Plan` tracks; `432` TS + `144` Rust matrix combinations verified) | `crates/vox-gui/ui/src/lib/buildChatTurn.test.ts`, `crates/vox-gui/src/commands/chat_turn.rs` |
| **OpenRouter Selection & Local-to-MENS Regression** | **2 Regressions** (`tier=local` / `clutch=free` blocked `VoxLocal` MENS fast-path via `!res.enforce_free_tier_only`; `clutch=free` + `risk=low` skipped free models when `effective_axes` shifted to `QUALITY_FIRST`) | **0 Regressions** (`60/60` cells across `5` spokes $\times$ `4` clutches $\times$ `3` risk postures route to calibrated `mens/qwen3_27b_five_spoke_v1` over uncalibrated `ollama/qwen3:8b`) | `crates/vox-orchestrator/src/models/registry.rs`, `crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/tests.rs` |
| **Context Management & Session Pin Hydration** | Fixed `4,000`-token `chars / 4` tail-only window; `0%` turn-0 goal anchor retention on overflow; `kb_scope` ignored; session pins not injected into `user_prompt` | Strategy-scaled `cl100k_base` BPE budget (`Aggressive=2,400`, `Balanced=4,000`, `Conservative=8,000`); `100%` turn-0 anchor retention (`20%` head reserve); `KbScopeMask` 5-mode leg filter; `[PINNED CONTEXT]` + `[EXECUTION GOVERNANCE]` injection | `crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs` |
| **MENS 5-Spoke & Organic Training Pipeline + Data Audit** | `25/25` 5-spoke suite (`pass@1 = 1.00`), `15/15` organic suite (`100%`), `route_spoke_task` hardcoded `CostPreference::Economy` | `25/25` 5-spoke + `15/15` organic suite + `route_spoke_task_with_policy` across all `(clutch, risk, prefer_local)` axes + `0` malformed `research_metrics` rows | `contracts/reports/full-flywheel-knob-matrix-before-after.v1.json` |

---

## 2. Phase 120: Chat Modifying Knobs, Modes, Tracks & Destinations Matrix (`KNOB-MATRIX-01..04`)

### 2.1 Before vs. After Wiring Audit Across All Tracks

| Knob / Field | Values Tested | BEFORE (`Sync` / `Background` / `Plan`) | AFTER (`Sync` / `Background` / `Plan`) |
| --- | --- | --- | --- |
| `execution` | `sync`, `background`, `plan` | Routed `vox_chat_message`, `submit_orchestrator_task`, `vox_plan` | Preserved (`100%`) |
| `mode` | `plan`, `act`, `verify` | Forwarded to `Background` only; **dropped on `Sync`** | Forwarded to `Sync` (`sync_tool_args` + `[EXECUTION GOVERNANCE]`) and `Background` (`enqueue_hints.mode`) |
| `tier` | `local`, `mesh`, `cloud`, `auto` | Forwarded to `Sync` and `Background` | Forwarded to `Sync` and `Background` + fixed `local` `VoxLocal` MENS fast-path |
| `clutch` | `free`, `efficiency`, `balanced`, `genius` | Forwarded to `Sync` and `Background` | Forwarded to `Sync` and `Background` + fixed `free` + `low` risk axis interaction |
| `risk` | `high`, `moderate`, `low` | Forwarded to `Sync` and `Background` | Forwarded to `Sync` and `Background` |
| `oversight_level` | `1`, `2`, `3`, `4`, `5` | **Dropped in `buildChatTurn.ts`** (never left TS composer) | Wired end-to-end: `CHAT_TURN_KEYS` $\rightarrow$ `ChatTurnInput` $\rightarrow$ `sync_tool_args` (`[EXECUTION GOVERNANCE]`) & `SubmitTaskInput` (`enqueue_hints.oversight_level`) |
| `kb_scope` | `auto`, `docs`, `code`, `memory`, `off` | **Dropped in `buildChatTurn.ts`** and ignored in `message.rs` | Wired end-to-end: `CHAT_TURN_KEYS` $\rightarrow$ `ChatTurnInput` $\rightarrow$ `KbScopeMask::from_kb_scope` in `message.rs` and `enqueue_hints.kb_scope` |
| `context_files` + Session Pins | Workspace paths & `gui.session_context_pins.<id>` | Recorded on `ChatTranscriptEntry`, **omitted from `user_prompt` on `Sync` and omitted from `scope_files` on `Plan`** | Hydrated from `gui.session_context_pins.<session_id>` + `context_files` into `[PINNED CONTEXT]` on `Sync`, `file_manifest` on `Background`, and `scope_files` on `Plan` |
| `grounding_check_enabled` | `true`, `false` | Local post-check + `Background`, omitted from `sync_tool_args` | Forwarded in `sync_tool_args` (`ChatMessageParams::grounding_check_enabled`) + `Background` |
| `dry_run` | `true`, `false` | Forwarded on wire; no prompt-level read-only guard on `Sync` | Forwarded on wire + injects `dry_run=true (read-only simulation; do not mutate files)` in `[EXECUTION GOVERNANCE]` |

---

## 3. Phase 121: OpenRouter Selection vs. Local-to-MENS Regression Matrix (`ROUTE-MENS-01..04`)

### 3.1 Clutch $\times$ Risk Multi-Axis Routing Matrix (`effective_axes`)

| Clutch Profile | Risk Posture | Effective Axes `(Cost, Responsiveness, Intelligence)` | Free Pool Enforced | OpenRouter Tier Selected | Local / MENS Route Selected |
| --- | --- | --- | --- | --- | --- |
| `Free` | `High` / `Moderate` | `(70, 15, 15)` (`COST_FIRST`) | `true` | `openrouter/*:free` (`ModelTier::Free`) | `mens/qwen3_27b_five_spoke_v1` (`ProviderType::VoxLocal`) |
| `Free` | `Low` (`ModelLean::Intelligence`) | `(25, 15, 60)` (`QUALITY_FIRST`) | `true` (`allow_free_in_performance_mode = true`) | Highest-prior `openrouter/*:free` | `mens/qwen3_27b_five_spoke_v1` (`ProviderType::VoxLocal`) |
| `Efficiency` | `High` / `Moderate` | `(65, 20, 15)` | `false` | `ModelTier::Fast` / `Free` workhorse | `mens/qwen3_27b_five_spoke_v1` when `prefer_local` |
| `Efficiency` | `Low` (`ModelLean::Intelligence`) | `(25, 15, 60)` | `false` | `ModelTier::Pro` workhorse | `mens/qwen3_27b_five_spoke_v1` when `prefer_local` |
| `Balanced` | `High` / `Moderate` | `(40, 30, 30)` (`BALANCED`) | `false` | `ModelTier::Pro` (`claude-sonnet-4.6` / `gemini-2.5-flash`) | `mens/qwen3_27b_five_spoke_v1` when `prefer_local` |
| `Balanced` | `Low` (`ModelLean::Intelligence`) | `(25, 20, 55)` | `false` | `ModelTier::Pro` / `Elite` | `mens/qwen3_27b_five_spoke_v1` when `prefer_local` |
| `Genius` | `High` / `Moderate` / `Low` | `(15, 15, 70)` (`QUALITY_FIRST`) | `false` | `ModelTier::Elite` / `Pro` (`claude-opus-4.7`) | `mens/qwen3_27b_five_spoke_v1` when `prefer_local` |

### 3.2 Two Critical Routing Regressions Fixed

1. **`ROUTE-REG-01` (`resolve.rs:494` — `tier="local"` / `clutch="free"` blocked `VoxLocal` MENS fast-path):**
   - **BEFORE:** `if vox_local_route_preferred && !res.enforce_free_tier_only` prevented `ProviderType::VoxLocal` (which has `is_free == true` and `$0` cost) from taking the `vox_local_route_preferred` fast-path whenever `tier="local"` or `clutch="free"` set `res.enforce_free_tier_only = true`.
   - **AFTER:** Replaced `!res.enforce_free_tier_only` with `.filter(|m| !res.enforce_free_tier_only || m.is_free)` and populated `*rationale_out` with `"VoxLocal MENS fast-path preferred for <task> (<model_id>)"`.
2. **`ROUTE-REG-02` (`resolve.rs:39` & `registry.rs:973` — `clutch=Free` + `risk=Low` rejected free models in `best_for_internal`):**
   - **BEFORE:** `effective_axes(ClutchProfile::Free, RiskPosture::Low)` returns `(25, 15, 60)` (`QUALITY_FIRST`), whose `to_cost_preference()` is `CostPreference::Performance`. Because `SelectionIntent::allow_free_in_performance_mode` defaulted to `false`, `best_for_internal` skipped all `is_free == true` models before `enforce_free_tier_if_needed` ran.
   - **AFTER:** Set `intent.allow_free_in_performance_mode = prefer_local || matches!(clutch, Some(ClutchProfile::Free))` and added `ModelRegistry::route_spoke_task_with_policy`, achieving `60/60` zero-regression cells across all 5 spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`).

---

## 4. Phase 122: Context Management & Compaction Engine Audit (`CTX-ENGINE-01..04`)

| Metric | BEFORE (`conversation.rs`) | AFTER (`bound_messages_by_strategy`) |
| --- | --- | --- |
| **Token Estimator** | Crude `chars / 4` (`approx_tokens`) | `CompactionEngine::estimate_tokens` (`cl100k_base` BPE) |
| **Strategy Budgets** | Hardcoded `4,000` across all strategies | `Aggressive = 2,400`, `Balanced = 4,000`, `Conservative = 8,000` (hydrated from `gui.context_budget.strategy`) |
| **Turn-0 Goal Anchor Preservation** | `0%` when conversation exceeds budget (oldest-first drop) | `100%` under `Balanced` and `Conservative` (reserves up to `20%` of budget for turn-0 anchor) |
| **Knowledge Base Scope (`kb_scope`)** | Always ran all 5 retrieval legs even when `kb_scope = "off"` | `KbScopeMask`: `off` skips `run_retrieval_bundle`; `docs` keeps KG+chunks; `code` keeps RRF+repo; `memory` keeps memory; `auto` keeps all |

---

## 5. Phase 123: Model Training Pipeline & Data Shape Audit (`FLYWHEEL-AUDIT-01..04`)

- **Canonical 25-Problem 5-Spoke Suite (`vox-corpus::training::spoke_roundtrip`):** `25/25` passed (`pass@1 = 1.00`, `pass@k = 1.00`, `5/5` per spoke across `VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`).
- **Organic 15-Problem Search-to-Compilation Suite (`vox-corpus::training::organic_ benchmark`):** `15/15` passed (`100%` across `Easy`, `Medium`, `Hard` difficulty tiers and all `5` spokes).
- **Curriculum Weight Floor & DPO Compiler-Diagnostic Repair Pairs:** All 5 spokes satisfy `weight >= 0.10` (`DEFAULT_MIN_SPOKE_WEIGHT_FLOOR`) with $\sum w_i = 1.00$; `extract_dpo_repair_pairs_from_outcomes` deterministically pairs failed compiler/clippy diagnostics with repaired solutions.
- **`VoxDb::audit_research_metric_shapes` Data Audit:** `0` malformed rows across `benchmark_event`, `syntax_k_event`, `RoutingDecision`, and `socrates_surface` (`100%` schema conformance).

---

## 6. Phases 124–127 (`v2.4`): Real-Time Persistent Session Posture, Keybound Semantic Knobs & CV Flywheel Visual Audit

### 6.1 Real-Time Session Posture Persistence (`POSTURE-PERSIST-01..04` & `TURN-RECEIPT-01..04`)

- **Before (`v2.3`):** Adjusting composer knobs (`clutch`, `risk`, `tier`, `mode`, `execution_mode`, `oversight_level`, `kb_scope`, `grounding_check_enabled`) only updated React state in `Loquela.tsx` and only took effect on the next `buildChatTurn` submission from that view. External MCP calls, subagent turns, `vox_plan`, and `vox_submit_task` did not inherit the live session posture.
- **After (`v2.4`):**
  - Added Tauri RPCs `get_session_chat_posture` and `set_session_chat_posture` (`crates/vox-gui/src/commands/context_budget.rs`) writing to `gui.chat_posture.<session_id>` and `gui.chat_posture.default` (plus `gui.compaction_strategy` when set).
  - `vox_chat_message` (`message.rs`), `vox_plan` (`plan.rs`), and `vox_submit_task` (`submission.rs`) automatically hydrate omitted knob fields from `load_persisted_session_chat_posture(state, session_id)` and persist explicit overrides via `persist_session_chat_posture`.
  - Every `vox_chat_message` response now emits structured `compaction_audit` (`strategy`, `budget_tokens`, `head_anchor_preserved`, `total_loaded`, `kept_messages`, `dropped_messages`) and `posture_audit` (`clutch`, `risk`, `tier`, `mode`, `oversight_level`, `kb_scope`, `grounding_check_enabled`, `hydrated_from_session_posture`).

### 6.2 Semantic Knob Coloration, Capsule Shapes & Keybindings (`KNOB-UX-KEYBIND-01..04`)

| Keybind | Knob / Action | Semantic Shape & Coloration |
| --- | --- | --- |
| `Alt+1` | Clutch $\rightarrow$ `Free` (`$0`) | Emerald capsule segment (`border-emerald-500/50 bg-emerald-500/20 text-emerald-200`) |
| `Alt+2` | Clutch $\rightarrow$ `Efficient` | Sky-blue capsule segment (`border-sky-500/50 bg-sky-500/20 text-sky-200`) |
| `Alt+3` | Clutch $\rightarrow$ `Balanced` | Amber-gold capsule segment (`border-amber-500/55 bg-amber-500/20 text-amber-100`) |
| `Alt+4` | Clutch $\rightarrow$ `Genius` | Violet capsule segment (`border-violet-500/55 bg-violet-500/20 text-violet-100`) |
| `Alt+C` | Cycle Clutch (`Free` $\rightarrow$ `Efficient` $\rightarrow$ `Balanced` $\rightarrow$ `Genius`) | Segmented `rounded-full` capsule group with active glow ring |
| `Alt+R` | Cycle Risk (`High` $\rightarrow$ `Moderate` $\rightarrow$ `Low`) | Rose (`High`), Amber (`Moderate`), Emerald (`Low`) `rounded-full` pill with status dot |
| `Alt+T` | Cycle Runtime Tier (`Auto` $\rightarrow$ `Local` $\rightarrow$ `Mesh` $\rightarrow$ `Cloud`) | Neutral (`Auto`), Emerald (`Local`), Cyan (`Mesh`), Violet (`Cloud`) capsule pill |
| `Alt+M` | Cycle Execution Track (`Sync` $\rightarrow$ `Background` $\rightarrow$ `Plan`) | Sky (`Quick chat`), Indigo (`Delegate`), Amber (`Write a plan`) capsule pill |
| `Alt+V` | Cycle Interaction Mode (`Act` $\rightarrow$ `Plan` $\rightarrow$ `Verify`) | Synchronized with execution track and persisted to session posture |
| `Alt+O` | Cycle Oversight (`1: Auto` .. `5: Strict`) | Emerald (`1-2`), Amber (`3: Balanced`), Rose (`4-5: Cautious/Strict`) capsule pill |
| `Alt+K` | Cycle `@kb` Scope (`auto` $\rightarrow$ `docs` $\rightarrow$ `code` $\rightarrow$ `memory` $\rightarrow$ `verified` $\rightarrow$ `off`) | Teal/Sky/Indigo/Violet/Emerald/Zinc `rounded-full` pill |
| `Alt+/` | Toggle Real-Time Keybind HUD (`loquela-keybind-hud-toggle` / `loquela-keybind-hud-panel`) | Brass capsule pill (`Keys Alt+/`) + interactive click-to-cycle HUD dialog |

### 6.3 CV Flywheel Visual Audit Before/After (`CV-V24-AUDIT-01..04` & `CV-V25-AUDIT-01..04`)
- **Live Turn Receipt & Anchor Badge (`chat-turn-receipt-badge` in `ChatExecutionRail.tsx`):** Surfaces `Goal Anchor: Kept · <budget> BPE (<kept>/<total> msgs)` and `<clutch>/<risk> · @<kb_scope>` in real time via `get_session_chat_posture` and `vox:session-posture-change`.
- **Continuous 5-Spoke MENS Ratchet (`ModelRegistry::verify_full_flywheel_routing_and_spoke_ratchet`):** Continuously verifies `25/25` 5-spoke `pass@1 = 1.0` and `60/60` `(clutch, risk, spoke)` routing cells.

| Surface ID | Before Score | After Score (`v2.3`) | After Score (`v2.4` Knobs + Live Badge) | Edge Sharpness $\Delta$ | Contrast $\Delta$ | PSNR (dB) | Converged |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `loquela-knobs` | `0.8280` | `0.9162` | **`0.9222`** (`+0.0942`) | `+6.0%` | `+76.7%` | `38.14 dB` | `true` (`0` defects) |
| `chat-default` | `0.8502` | `0.9203` | **`0.9203`** (`+0.0701`) | `+12.8%` | `+62.1%` | `38.12 dB` | `true` (`0` defects) |
| `chat-dock-multi` | `0.7985` | `0.9216` | **`0.9216`** (`+0.1231`) | `+4.1%` | `+64.1%` | `38.14 dB` | `true` (`0` defects) |
| `urbs-civilization` | `0.8228` | `0.9180` | **`0.9180`** (`+0.0952`) | `+24.5%` | `+14.5%` | `38.10 dB` | `true` (`0` defects) |
