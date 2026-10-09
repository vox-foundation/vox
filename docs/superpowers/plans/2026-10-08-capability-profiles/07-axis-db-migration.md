---
title: "Capability Profiles plan 07 — Axis db-command migration"
description: "Task-by-task plan that moves every db-class Axis (vox-gui) Tauri command behind the daemon (MCP tools or /api/v2 routes) in profile order, so those surfaces work against a remote Vox server, then forbids new db-class commands."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Axis DB-Command Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every Tauri command in `crates/vox-gui/src/commands` that today opens `vox_db` / `turso` directly is rewritten to call the daemon through plan 06's `Backend` (Local → `PersistentDaemon`, Remote → `GatewayClient`), so the Models, Chat, Memory, Research, Scientia, Publications, Console, Settings, Tasks, Runs, Activity and HarnessHealth surfaces work against a LAN server; afterwards CI forbids `db`-class modules and `vox-gui` drops its direct `vox-db`/`turso` dependency.

**Architecture:** Reads go to an existing MCP tool, else an existing `/api/v2` route, else a new read-only `/api/v2` route (in `vox-orchestrator-mcp/src/gui_ops/<group>.rs`, merged into `dashboard_api::router()`); mutations always go to an MCP tool (existing or new, registered in `catalog.v1.yaml` with `exposure`/`side_effect_class`/`requires`) so `authorize_tool` gates them. Local mode reaches the *same* handlers: tools via `orch.tool_call`, `/api/v2` reads via a new daemon method `orch.api_v2_get` that runs the dashboard router in-process. Wire DTOs move from `vox-gui` into `vox-orchestrator-mcp/src/gui_ops/dto/` and are re-used by the GUI, so there is one shape per payload.

**Tech Stack:** Rust (Tauri 2, axum 0.8, tokio, serde, schemars), `vox-db` store fns, `vox-orchestrator-mcp` dispatch + HTTP gateway, TypeScript (React, vitest, Playwright).

## Global Constraints

Copied verbatim from `00-contracts.md` §Global constraints:

- TDD: failing test first; every new `pub fn` has a test in the same file (`skeleton/untested-pub-api`).
- Security guards are mutation-verified: break the guard, see the test fail, restore, with a
  before/after presence check (AGENTS.md §PR & Review Discipline).
- No new workspace crate edges in plans 01–04, 06–07. Plan 05's new edges are **proposed for user
  authorization** and are not added to `contracts/ci/crate-edges.allow.v1.json` by an agent.
- Secrets via `vox_secrets::resolve_secret`; no `std::env::var` for tokens. Non-secret `VOX_*`
  config via existing `vox-config` patterns.
- Never `cargo fmt --all`; format dirty files with `vox run scripts/fmt.vox`.
- Generated files (`tool-registry.canonical.yaml`, `*.generated.md`) are regenerated, never hand-edited.
- New `.md` under `docs/src/` has frontmatter; lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths <path relative to docs/src>`.
- Per-crate checks before each commit: `cargo check -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`, targeted `cargo nextest run -p <crate> <filter>`.
- `dev` profile must reproduce today's behaviour exactly (back-compat tests in plans 02 and 03).

Plan-07-specific constraints:

- Work only in `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles` (branch `feat/capability-profiles`).
- Data storage policy (`docs/src/architecture/data-storage-ssot-2026.md`): no `turso::Connection` and no raw SQL outside `vox-db`. The three GUI commands that run raw SQL today (`activity_query`, `get_memory_status`, `get_context_budget`) get a named `vox-db` store fn as part of their migration. Wire structs live in a `dto/` module, never on `row/` types.
- Removing an edge (`vox-gui → vox-db`) is allowed and is done with `vox ci crate-edges --tighten`.
- GUI changes pass `pnpm --dir crates/vox-gui/ui typecheck`, `pnpm --dir crates/vox-gui/ui test`, `pnpm --dir crates/vox-gui/ui test:e2e` and write screenshots to `crates/vox-gui/ui/review-bundle/latest/`.
- Tauri command **names and argument names do not change** (the UI keeps calling `invoke('<same name>', <same args>)`); only the Rust body and the extra `backend` state parameter change.
- `side_effect_class` values are plan 01's `SideEffectClass` (`none | read_only | writes_files | spawns_process | network | remote_mutation | destructive`). DB-row mutations use `writes_files` (plan 01 assigns it to `vox_preference_set`); external publication uses `remote_mutation`; outbound probes use `network`.
- All new **mutating** ops are `exposure: operator` (even where plan 01's E3 rule would allow `public` for "own subsystem state") so a read token can never mutate through Axis; dev-only gamify ops are `internal`.

---

## File Structure

New (server, `crates/vox-orchestrator-mcp/src/`):

| Path | Responsibility |
|---|---|
| `gui_ops/mod.rs` | `GUI_USER_ID`, `require_db`, `api_db`, `enforce_dashboard_surface`, `test_support`; declares group modules |
| `gui_ops/inproc.rs` | `api_v2_get(state, path)` — runs the `/api/v2` router in-process for `orch.api_v2_get` |
| `gui_ops/dto/mod.rs` + `dto/{models,chat,knowledge,research,orchestrator,harness,attention}.rs` | Wire DTOs moved from `vox-gui` (Serialize + Deserialize) |
| `gui_ops/{models,chat,prefs,knowledge,research,attention,orchestrator,harness,gamify}.rs` | Route handlers (`api_v2_router()`) and new MCP op handlers per group |

New (GUI, `crates/vox-gui/src/`):

| Path | Responsibility |
|---|---|
| `remote/calls.rs` | `BackendCalls` trait, impl for plan 06 `Backend`, `tool_data` / `api_data` / `path_with_query` helpers, `FakeBackend` (cfg(test)) |
| `remote/local_prefs.rs` | `AxisLocalPrefs`: client-side `gui.*` preference store (`<app_config_dir>/axis-preferences.json`) used in remote mode only (D1); local mode never writes it |
| `commands/model_active.rs` | `get_active_model`, `set_active_model` (pattern task) |
| `commands/models_host.rs` | `search_model_cards`, `explain_model_selection`, `suggest_model_for_task`, `get_auto_model_recommendation` (host: client-side registry cache / client GPU) |
| `commands/locator.rs` | `open_locator` (host: spawns editor) |
| `commands/research_host.rs` | `save_research_doc`, `execute_sandbox_probe` (host) |
| `commands/llm_spend.rs` | `get_llm_spend` (split out of host module `user_config.rs`) |
| `commands/gamify_settings.rs` | `get_gamify_settings`, `set_gamify_settings` (host: local disk config) |

New (UI, `crates/vox-gui/ui/`): `src/guards/commandBackendsMigration.test.ts`, `e2e/lib/remoteMode.ts`, `e2e/{models,chat,memory,research,console,settings-spend,tasks,harness-health}-remote.spec.ts`.

Modified: `crates/vox-orchestrator-mcp/src/{lib.rs,dispatch.rs,input_schemas.rs,daemon_extra.rs,http_gateway/mod.rs,http_gateway/dashboard_api.rs}`, `crates/vox-foundation/src/protocol.rs`, `crates/vox-db/src/store/` (three new store fns), every module in the inventory below, `crates/vox-gui/src/{main.rs,commands/mod.rs,Cargo.toml}`, `contracts/operations/catalog.v1.yaml`, `contracts/gui/command-backends.v1.yaml`, `contracts/profiles/*.v1.yaml`, plan 06's CI check (`gui_command_backends`, run by `vox ci gui-catalog-parity`).

Deleted at the end: `crates/vox-gui/src/commands/gui_db_pool.rs`.

### Inventory (measured 2026-10-08 with `rg -l 'vox_db|connect_workspace|turso' crates/vox-gui/src/commands`)

20 command modules + the `gui_db_pool.rs` helper. 89 commands touch the DB. "UI" = `view_key` from `contracts/gui/surface-registry.v1.yaml` (or the non-surface caller). "Eq" = equivalent already served by the daemon: **T** existing MCP tool, **R** existing `/api/v2` route, **—** none (reason given when a near-match was rejected).

| # | Command (module) | vox-db call(s) | UI | Profile | Eq | Target |
|---|---|---|---|---|---|---|
| 1 | `get_active_model` (models) | `get_user_preference("local_user","active_model")` | models | models | — (`vox_preference_get` requires `[memory]`, which would boot the search index in every Axis profile) | new route `GET /api/v2/models/preferences` |
| 2 | `set_active_model` (models) | `set_user_preference(..,"active_model")` | models | models | — (`vox_preference_set` also mutates live gate config) | new op `vox_models_preference_set` |
| 3 | `set_routing_priority` (models) | `set_user_preference(..,"routing_priority")` | settings | models | — | `vox_models_preference_set` |
| 4 | `get_selection_policy` (models) | `get_user_preference(..,"selection_policy")` | settings | models | — (as #1) | route #1 |
| 5 | `set_selection_policy` (models) | `set_user_preference(..,"selection_policy")` | settings | models | — | `vox_models_preference_set` |
| 6 | `get_routing_intentions` (models) | `get_user_preference(..,"routing_priority")` (+ env base) | models (Matrix panel) | models | — | route #1 (`routing_priority` is the server-side effective value) |
| 7 | `nudge_routing_intention` (models) | get + `set_user_preference(..,"routing_priority")` | models (Matrix) | models | — | route #1 + `vox_models_preference_set` |
| 8 | `get_model_scoreboard` (models) | `get_model_scoreboard(window)` | runs | models | — | new route `GET /api/v2/models/scoreboard` |
| 9 | `list_model_cards` (models) | `get_model_scoreboard(7)`, `get_pricing_catalog()` | models, settings, chat (Loquela), App | models | — (`/models/catalog` lacks scoreboard + Mens merge) | new route `GET /api/v2/models/cards` |
| 10 | `get_routing_summary_live` (models) | `get_user_preference` ×2 + `orch.status` | models, runs, settings, App | models | — (`/routing/summary` is env-gated, other shape) | new route `GET /api/v2/models/routing-summary` |
| 11 | `chat_create_session` (chat) | `chat_ensure_gui_session_with_repo` | chat, App | models | — | new op `vox_chat_session_create` |
| 12 | `chat_list_sessions` (chat) | `chat_list_gui_sessions` | chat, App | models | — | new route `GET /api/v2/chat/sessions` |
| 13 | `chat_get_messages` (chat) | `chat_get_gui_messages` | chat | models | — | new route `GET /api/v2/chat/sessions/{id}/messages` |
| 14 | `chat_append_message` (chat) | `chat_ensure_gui_session`, `chat_append_message` | chat | models | — | new op `vox_chat_session_append` |
| 15 | `chat_rename_session` (chat) | `chat_find_gui_conversation_id`, `chat_rename_conversation` | chat | models | — | new op `vox_chat_session_rename` |
| 16 | `chat_archive_session` (chat) | `chat_find_gui_conversation_id`, `chat_archive_conversation` | chat | models | — | new op `vox_chat_session_archive` |
| 17 | `chat_unarchive_session` (chat) | `..._including_archived`, `chat_unarchive_conversation` | chat | models | — | `vox_chat_session_archive {archived:false}` |
| 18 | `chat_send_message` (chat) | `chat_get_gui_messages`, `chat_append_message` (+ `vox_chat_message` via daemon) | chat | models | T `vox_chat_message` (turn only) | `vox_chat_message` + `vox_chat_session_append` + route #13 |
| 19 | `get_gui_preference` (preferences) | `get_user_preference("local_user",key)` | App, settings, memory, main.tsx | all Axis (D1: client-side when remote) | — (as #1) | remote: client-side `axis-preferences.json`; local: new `internal` op `vox_gui_preference_get` (loopback + `dev` only) (errors surfaced, no client-side fallback) |
| 20 | `set_gui_preference` (preferences) | `set_user_preference` | settings, memory, hooks | all Axis (D1: client-side when remote) | — (as #2) | remote: client-side; local: new `internal` op `vox_gui_preference_set` (keys `gui.*` only) (errors surfaced, no client-side fallback) |
| 21 | `list_knowledge_nodes` (knowledge) | `list_knowledge_nodes` | memory | knowledge | — (`vox_kb_*` use `KbStore`, other tables) | new route `GET /api/v2/knowledge/nodes` |
| 22 | `delete_knowledge_node` (knowledge) | `delete_knowledge_node` | memory | knowledge | — | new op `vox_kb_node_delete` |
| 23 | `get_kb_health` (knowledge) | `get_knowledge_health` | memory | knowledge | — | new route `GET /api/v2/knowledge/health` |
| 24 | `ingest_text` (knowledge) | `upsert_knowledge_node` | memory | knowledge | — | new op `vox_kb_ingest_text` |
| 25 | `ingest_url` (knowledge) | HTTP fetch + `upsert_knowledge_node` | memory | knowledge | — | new op `vox_kb_ingest_url` |
| 26 | `save_research_session_to_kb` (knowledge) | `get_research_session`, `get_research_artifact`, upsert | research | knowledge | — | new op `vox_kb_import_research_session` |
| 27 | `get_memory_status` (memory) | raw `SELECT COUNT(*)` ×5 | memory | knowledge | — | new store fn `VoxDb::search_corpus_counts` + route `GET /api/v2/knowledge/corpus-counts` |
| 28 | `mnemosyne_recall` (memory) | `MemoryManager::with_db` + `hybrid_search` | memory | knowledge | — (`vox_memory_search` has a different hit shape) | new route `GET /api/v2/knowledge/recall` |
| 29 | `mnemosyne_reindex` (memory) | `MemoryManager` `sync_from_db` / `sync_to_db` | memory | knowledge | — | new op `vox_memory_reindex` |
| 30 | `vox_search_query` (search) | `SearchRuntimeContext` over `VoxDb` + `chat_search_gui_messages` | memory, vox-search (palette) | knowledge | — | new route `GET /api/v2/search/query` |
| 31 | `list_research_sessions` (scientia) | `list_recent_research_sessions` | research | research | — | new route `GET /api/v2/research/sessions` |
| 32 | `get_research_session_detail` (scientia) | `get_research_session`, `get_research_artifact` | research | research | — (`vox_research_get` = answer payload only) | new route `GET /api/v2/research/sessions/{id}` |
| 33 | `list_publication_manifests` (scientia) | `list_publication_manifests` | publications, scientia | research | — | new route `GET /api/v2/scientia/manifests` |
| 34 | `scientia_dashboard_snapshot` (scientia) | manifests + `scientia_claims_pending_summary` | scientia | research | R `/api/v2/scientia/queue` | `/api/v2/scientia/queue` |
| 35 | `scientia_cost_rollup` (scientia) | `scientia_cost_raw_this_quarter` | scientia | research | R `/api/v2/scientia/cost` | `/api/v2/scientia/cost` |
| 36 | `list_publication_review_queue` (scientia_review) | `list_claims_awaiting_review` | scientia | research | — | new route `GET /api/v2/scientia/publications/{id}/review-queue` |
| 37 | `record_publication_claim_review` (scientia_review) | claim review write on `db()` | scientia | research-publish | — | new op `vox_scientia_publication_claim_review` |
| 38 | `nanopublish_approved_claim` (scientia_review) | `db()` + `evidence_assist` + nanopub | scientia | research-publish | — | new op `vox_scientia_publication_nanopublish` |
| 39 | `suggest_evidence_improvements` (scientia_review) | `list_claims_awaiting_review` + `vox_scientia::evidence_assist` | scientia | research | — (`vox_scientia_assist_suggestions` is the LLM assist) | new route `GET /api/v2/scientia/publications/{id}/claims/{claim_id}/evidence` |
| 40 | `get_novelty_assessment` (scientia_review) | `get_publication_manifest` | scientia | research | — (`..._novelty_fetch` performs network fetch) | new route `GET /api/v2/scientia/publications/{id}/novelty` |
| 41 | `list_discovery_inbox` (scientia_review) | `list_unacknowledged_discoveries` | scientia | research | — | new route `GET /api/v2/scientia/discoveries` |
| 42 | `acknowledge_discovery` (scientia_review) | `acknowledge_discovery` | scientia | research | — | new op `vox_scientia_discovery_ack` |
| 43 | `get_completion_report` (scientia_review) | `db()` + `vox_publisher::scientia_discovery::manifest_completion_report` | scientia | research | — | new route `GET /api/v2/scientia/publications/{id}/completion` |
| 44 | `run_autofill` (scientia_review) | `upsert_publication_manifest`, `append_publication_status_event` | scientia | research | — | new op `vox_scientia_publication_autofill` |
| 45 | `get_archive_status` (scientia_review) | `list_scholarly_submissions` | scientia | research | — (`..._scholarly_remote_status` polls remote) | new route `GET /api/v2/scientia/publications/{id}/archive` |
| 46 | `flag_research_misleading` (research) | `record_research_misguidance` | research | research | — | new op `vox_research_flag_misleading` |
| 47 | `generate_research_doc_draft` (research) | `get_research_session`/`get_research_artifact` | research | research | — | new route `GET /api/v2/research/sessions/{id}/doc-draft` |
| 48 | `publish_research_doc` (research) | `store_research_artifact` + repo doc write | research | research | — | new op `vox_research_publish_doc` |
| 49 | `get_research_engine_status` (search_probe) | `get_user_preference("research.*")` ×4, `ops_quota::get_quota_usage` | research | research | — | new route `GET /api/v2/research/engine-status` |
| 50 | `save_research_engine_config` (search_probe) | `set_user_preference("research.*")` ×4 | research | research | — | new op `vox_research_engine_config_set` |
| 51 | `send_to_agent` (console_a2a) | `Codex::connect(resolve_for_mesh)` + `send_to_db_with_breaker` | console | mesh | T `vox_a2a_send` (`route:"db"`) | `vox_a2a_send` |
| 52 | `get_llm_spend` (user_config) | `llm_spend_summary(session)` | settings, App (`useLlmSpend`) | attention | — (`vox_budget_status` = agent budgets) | new route `GET /api/v2/llm/spend` |
| 53 | `get_context_budget` (orchestrator) | raw SQL on `llm_interactions` | chat | orchestrator | — (`vox_context_budget` = per-agent budget) | new store fn `VoxDb::latest_interaction_tokens` + route `GET /api/v2/orchestrator/context-budget` |
| 54 | `hopper_list` (orchestrator) | `SqliteHopper` inbox/assigned/`history_recent(50)` | tasks, `useAttentionInbox`, gamify | orchestrator | — (`/hopper/{inbox,assigned,history}` read `InMemoryHopper`, unbounded, 3 calls) | new route `GET /api/v2/hopper/list` |
| 55 | `hopper_submit` (orchestrator) | `SqliteHopper::submit` | tasks | orchestrator | — (`POST /api/v2/hopper/submit` is not `authorize_tool`-gated) | new op `vox_hopper_submit` |
| 56 | `hopper_reprioritize` (orchestrator) | `SqliteHopper::reprioritize` | tasks | orchestrator | — | new op `vox_hopper_reprioritize` |
| 57 | `hopper_cancel` (orchestrator) | `SqliteHopper::cancel` | tasks | orchestrator | — | new op `vox_hopper_cancel` |
| 58 | `hopper_mark_done` (orchestrator) | `SqliteHopper::complete` | tasks | orchestrator | — | new op `vox_hopper_mark_done` |
| 59 | `get_orchestrator_status` + `_bin` (orchestrator, via `gamify::fetch_gamify_alerts`) | `Codex::connect` + gamify notifications | dashboard | orchestrator | T `vox_gamify_notifications_list` | `vox_gamify_notifications_list` (empty on denial) |
| 60 | `update_plan_node` (plan_panel) | `load_plan_nodes_with_status`, `upsert_plan_node` | chat | orchestrator | — (`vox_plan_status` returns text) | new op `vox_plan_node_upsert` |
| 61 | `insert_plan_node` (plan_panel) | same | chat | orchestrator | — | `vox_plan_node_upsert {mode:"insert"}` |
| 62 | `list_plan_nodes` (plan_panel) | `load_plan_nodes_with_status` | chat | orchestrator | — | new route `GET /api/v2/plan/sessions/{id}/nodes` |
| 63 | `plan_open_task_counts` (plan_panel) | `open_task_counts_for_sessions` | App | orchestrator | — | new route `GET /api/v2/plan/open-task-counts` |
| 64 | `approve_plan_nodes` (plan_panel) | `approve_all_blocked_plan_nodes` | chat | orchestrator | — | new op `vox_plan_nodes_approve` |
| 65 | `latest_plan_session_for_chat` (plan_panel) | `latest_plan_session_id_for_origin`, `get_plan_session_by_id` | App | orchestrator | — | new route `GET /api/v2/plan/latest-for-chat/{session_id}` |
| 66 | `start_gui_run` (runs) | `agent_runs_upsert` | App | orchestrator | — | new op `vox_gui_run_start` |
| 67 | `finish_gui_run` (runs) | `agent_runs_get`, `find_approval_id_for_run`, `find_task_root_summary_totals`, `agent_runs_upsert` | App | orchestrator | — | new op `vox_gui_run_finish` |
| 68 | `list_gui_runs` (runs) | `agent_runs_recent` | runs, models | orchestrator | — (`/runs/recent` lists orchestrator tasks) | new route `GET /api/v2/runs/gui` |
| 69 | `get_gui_run` (runs) | `agent_runs_get` | runs | orchestrator | — | new route `GET /api/v2/runs/gui/{run_id}` |
| 70 | `activity_query` (activity) | raw SQL on `activity_log` | activity, chat (`useChatExecutionData`) | orchestrator | — | new store fn `VoxDb::query_activity_log` + route `GET /api/v2/activity` |
| 71 | `harness_eval_history` (harness_eval) | `list_harness_eval_runs`, `get_harness_eval_task_results_for_runs` | harness-health | orchestrator | — | new route `GET /api/v2/harness/eval/history` |
| 72 | `harness_eval_regressions` (harness_eval) | `list_harness_eval_runs`, `get_harness_eval_task_results`, `get_model_selection_events` | harness-health | orchestrator | — | new route `GET /api/v2/harness/eval/regressions` |
| 73 | `scan_training_corpus` (harness_issues) | FS walk + `has_pending_harness_issue`, `insert_harness_issue` | scientia | research | — | new op `vox_harness_corpus_scan` |
| 74 | `propose_harness_issue_fix` (harness_issues) | `get_harness_issue`, LLM, `insert_harness_fix_proposal` | scientia | research | — | new op `vox_harness_fix_propose` |
| 75 | `list_harness_fix_proposals` (harness_issues) | `list_harness_fix_proposals` | scientia | research | — | new route `GET /api/v2/harness/fix-proposals` |
| 76 | `resolve_harness_fix_proposal` (harness_issues) | `resolve_harness_fix_proposal`, `get_harness_fix_proposal` | scientia | research | — | new op `vox_harness_fix_resolve` |
| 77 | `list_harness_issues` (harness_issues) | `list_harness_issues` | scientia | research | — | new route `GET /api/v2/harness/issues` |
| 78 | `list_harness_issues_for_session` (harness_issues) | `list_harness_issues_for_session` | scientia | research | — | new route `GET /api/v2/harness/issues/by-session/{key}` |
| 79 | `record_harness_issue_decision` (harness_issues) | `record_harness_issue_decision` | scientia | research | — | new op `vox_harness_issue_decide` |
| 80 | `get_ludus_profile` (gamify) | `regen_energy` etc. | gamify, App | dev-only (`internal`) | — | new op `vox_gamify_profile` |
| 81 | `list_ludus_notifications` (gamify) | `list_unread_notifications` | gamify | dev-only | T `vox_gamify_notifications_list` | existing |
| 82 | `ack_ludus_notification` (gamify) | `mark_notification_read_for_user` | gamify | dev-only | T `vox_gamify_notification_ack` | existing |
| 83 | `gamify_due_actions` (gamify) | `due_action_ids` | gamify | dev-only | — | new op `vox_gamify_due_actions` |
| 84 | `gamify_kpi_summary` (gamify) | `load_kpi_summary` | gamify | dev-only | T `vox_gamify_progress_snapshot` (`data.kpi`) | existing |
| 85 | `record_gui_event` (gamify) | `route_event_auto_user` | App | dev-only | — | new op `vox_gamify_record_event` |
| 86 | `list_gamify_leaderboard` (gamify) | `leaderboard` | gamify | dev-only | — | new op `vox_gamify_leaderboard` |
| 87 | `list_gamify_companions` (gamify) | `list_companions` | gamify | dev-only | — | new op `vox_gamify_companions` |
| 88 | `list_gamify_quests` (gamify) | `list_quests` | gamify | dev-only | T `vox_gamify_quest_list` | existing |
| 89 | `discovery_record` (discovery) | `Codex::connect` + `vox_gamify::discovery::ledger::record` | console | dev-only | — | new op `vox_gamify_discovery_record` |

Counts per profile group: **models 18** (#1–18) + **cross-cutting GUI prefs 2** (#19–20) · **knowledge 10** (#21–30) · **research 27** (#31–50, #73–79) · **mesh 1** (#51) · **attention 1** (#52) · **skills 0** (no `skills*` module touches the DB — verified in Task 9) · **orchestrator 20** (#53–72) · **remaining / dev-only 10** (#80–89). Existing equivalents: **9** commands (T: #18 turn only, #51, #59, #81, #82, #84, #88; R: #34, #35). Need new surface: **80** commands, served by **38 new read-only `/api/v2` routes** (models 6, chat 2, knowledge 5, research 14, attention 1, orchestrator 10) and **38 new MCP ops** (models 1, GUI prefs 2 — `internal`, loopback-only, per D1, chat 4, knowledge 5, research 12, orchestrator 8, gamify 6); several commands share one route or op.

Commands in these modules that do **not** touch the DB and are local by nature move to new `host` modules so the remaining module can be classified `daemon`: `search_model_cards`, `explain_model_selection`, `suggest_model_for_task`, `get_auto_model_recommendation` → `models_host.rs`; `open_locator` → `locator.rs`; `save_research_doc`, `execute_sandbox_probe` → `research_host.rs`; `get_user_config`/`set_user_config`/`reset_user_config` stay in `user_config.rs` (host) after `get_llm_spend` leaves; `get_gamify_settings`/`set_gamify_settings` → `gamify_settings.rs`. `probe_search_provider`/`probe_all_search_providers` (server secrets + network) migrate with research as op `vox_research_engine_probe`; `inference_provider_status` (llm_settings, host today) migrates with models as route `GET /api/v2/models/provider-status`, because the Models surface needs it remotely (server keys, server-reachable local backends).

### Polling and keystroke commands (performance)

| Caller | Cadence today | Remote risk | Decision |
|---|---|---|---|
| `ModelsView.refresh` → `list_model_cards` + `get_routing_summary_live` + `get_active_model` + `inference_provider_status` | every 8 s | 4 requests / 8 s; `list_model_cards` rebuilds the registry each time | One batched route `GET /api/v2/models/snapshot` + new command `get_models_snapshot`; ModelsView calls it (Task 3) |
| `useAttentionInbox` / Tasks → `hopper_list` | every 5 s (`ATTENTION_POLL_MS`) | would be 3 calls to existing routes | Single route `/api/v2/hopper/list` (Task 10) |
| `RunsView` → `get_model_scoreboard` + `list_gui_runs` + `get_routing_summary_live` | every 10 s | 3 small GETs / 10 s | Acceptable over plan 06's keep-alive client; no change |
| `ScientiaDashboard` → `scientia_dashboard_snapshot`; `DiscoveryInbox`, `DiscoveryReview`, `HarnessIssuesPanel` | every 10 s | 1 GET each | unchanged (topic push deferred: plan 06 has no per-topic WS subscription; Task 12 Step 3) |
| `Matrix` → `get_routing_intentions` | every 8 s | 1 GET | unchanged |
| `usePersistedDbState` → `get_gui_preference` | once per key at mount (7 keys) | 7 GETs at startup | unchanged (one-off; not a poll) |
| `useSearchController` → `vox_search_query` | per keystroke, 200 ms debounce | one HTTPS round trip per pause | Remote mode debounce 350 ms; stale responses already dropped by `requestToken` (Task 12) |
| `useLlmSpend` → `get_llm_spend` | every 60 s | negligible | unchanged |

---

### Task 0: Confirm plans 01–04 and 06 landed; apply decision D1

Every name this plan consumes from earlier plans is fixed below (taken from the final text of
plans 01, 02, 03 and 06 and `00-contracts.md` Amendments). This task only **verifies** they landed;
it does not substitute alternatives. No code changes in this task except the D1 profile check.

**Files:** none (verification only).

**Interfaces consumed:**

| Name used in this plan | Owner | Verify with | Expected |
|---|---|---|---|
| `crate::remote::backend::Backend` (`enum Backend { Local(Arc<PersistentDaemon>), Remote(Arc<GatewayClient>) }`) and `BackendState` (`current().await -> Backend`, `is_remote()`), managed as `tauri::State<'_, Arc<BackendState>>` | 06 Task 7 | `rg -n 'pub enum Backend\|pub struct BackendState\|pub async fn current' crates/vox-gui/src/remote/backend.rs`; `rg -n 'manage\(std::sync::Arc::new\(remote::backend::BackendState' crates/vox-gui/src/main.rs` | 3 hits; 1 hit |
| `GatewayClient::call_daemon_method(&self, method, params, deadline) -> Result<Value, AxisError>` (routes `orch.tool_call` over `/v1/ws`) and `GatewayClient::api_get(&self, path) -> Result<Value, AxisError>` (GET + bearer; returns the **unwrapped** `data`) | 06 Task 6 | `rg -n 'pub async fn call_daemon_method\|pub async fn api_get' crates/vox-gui/src/remote/gateway_client.rs` | 2 hits |
| `AxisError` (`Remote { code, message }` variant, `impl From<AxisError> for String`) | 06 Task 4 | `rg -n 'pub enum AxisError\|impl From<AxisError> for String' crates/vox-gui/src/remote/error.rs` | 2 hits |
| Tauri command `axis_backend_status` → `BackendStatusDto { mode: 'local'\|'remote', url, profile, surfaces }`; TS `axisBackendStatus()` in `transport.ts` | 06 Tasks 8–9 | `rg -n 'pub async fn axis_backend_status' crates/vox-gui/src/commands/axis_remote.rs`; `rg -n 'export function axisBackendStatus' crates/vox-gui/ui/src/transport.ts` | 1 hit each |
| Remote-gating copy for `not_remote_capable`: "This surface cannot run against a remote server yet." | 06 Task 10 | `rg -n 'cannot run against a remote server yet' crates/vox-gui/ui/src` | ≥1 hit |
| Remote invoke gate: `local_only_denial` rejects `db`/`host`-class modules from the embedded `command-backends.v1.yaml`. There are **no per-command guards**: flipping a module's manifest row to `daemon` is what un-gates its commands in remote mode. | 06 Task 7 | `rg -n 'pub fn local_only_denial' crates/vox-gui/src/remote/gate.rs` | 1 hit |
| Plan 06's `gui_command_backends` check (run by `vox ci gui-catalog-parity` / `ssot-drift`) in `crates/vox-cli/src/commands/ci/gui_command_backends.rs`, pure validator `problems(modules_on_disk: &BTreeSet<String>, manifest: &serde_yaml::Value, view_keys: &BTreeSet<String>) -> Vec<String>` | 06 Task 2 | `rg -n 'pub fn problems' crates/vox-cli/src/commands/ci/gui_command_backends.rs` | 1 hit |
| `ServerState.active_profile: Arc<ActiveProfile>`, `ServerState::with_profile(ActiveProfile)`, `ActiveProfile::resolve(&[&str])` | 02 Tasks 2, 4 | `rg -n 'pub fn with_profile' crates/vox-orchestrator-mcp/src/server_state.rs` | 1 hit |
| Profile manifests are compiled by `crates/vox-mcp-registry/build.rs`, which panics when a profile lists a tool whose `requires` is outside its subsystem closure (and plan 02's `named_profiles_respect_max_exposure_and_subsystem_closure` test re-checks it) | 02 Task 2 | `rg -n 'outside' crates/vox-mcp-registry/build.rs` | ≥1 hit |
| `Subsystem` ids `db`, `model_registry`, `llm_facade`, `search_index`, `memory`, `research`, `mesh`, `attention` | 01 | `rg -n 'pub enum Subsystem' -A3 crates/vox-mcp-registry/src/subsystem.rs` | all present |
| `SideEffectClass` values `writes_files`, `network`, `remote_mutation`, `destructive`, `read_only` | 01 | `rg -n 'pub enum SideEffectClass' -A3 crates/vox-mcp-registry/src/exposure.rs` | all present |
| `vox-orchestrator-d` enables `news-publish` on `vox-orchestrator-mcp` (needed by #38, #43) | existing | `rg -n 'news-publish' crates/vox-orchestrator-d/Cargo.toml` | ≥1 hit (verified 2026-10-08: line 17) |

**Decision D1 (taken by the user, `00-contracts.md` Amendments):**

- `models` requires `db` (plan 02's manifest is `requires: [db, model_registry, llm_facade]`), so the
  models-group ops of this plan (`requires: [db, …]`) are listed in `contracts/profiles/models.v1.yaml`.
- `attention` and `skills` stay **DB-free**. No db-requiring op of this plan is added to them; the
  attention-group read (Task 9) must therefore not require `db` (see Task 9).
- **GUI preferences (`gui.*`) are client-side in remote mode.** `preferences.rs` stays a `shell`-class
  module (plan 06 row 43): in remote mode Axis reads/writes them in an Axis-local settings file and never
  calls the server; in local mode it reaches the local daemon over loopback (Task 5).

- [ ] **Step 1:** Run every "Verify with" command above from the worktree root; paste the output into the PR description. Any miss means the owning plan has not fully landed: stop and finish it first.
- [ ] **Step 2:** Confirm D1 is reflected: `rg -n '^requires:' contracts/profiles/models.v1.yaml contracts/profiles/attention.v1.yaml contracts/profiles/skills.v1.yaml` → `models` contains `db`; `attention` is `[attention]`; `skills` is `[]`.

---

### Task 1: Shared plumbing (BackendCalls, `orch.api_v2_get`, `gui_ops` skeleton)

**Files:**
- Create: `crates/vox-gui/src/remote/calls.rs`; modify `crates/vox-gui/src/remote/mod.rs` (plan 06 module; add `pub mod calls;` next to `pub mod backend;`). Paths used below: `crate::remote::backend::{Backend, BackendState}` (plan 06 Task 7) and `crate::remote::calls::{BackendCalls, tool_data, api_data, path_with_query}`; the test double is `crate::remote::calls::fake::FakeBackend`
- Create: `crates/vox-orchestrator-mcp/src/gui_ops/{mod.rs,inproc.rs,dto/mod.rs}` and empty-router group files `gui_ops/{models,chat,prefs,knowledge,research,attention,orchestrator,harness,gamify}.rs`
- Modify: `crates/vox-orchestrator-mcp/src/lib.rs`, `src/daemon_extra.rs`, `src/http_gateway/mod.rs`, `src/http_gateway/dashboard_api.rs`, `src/dispatch.rs`, `src/input_schemas.rs`, `crates/vox-orchestrator-mcp/Cargo.toml` (move `tower = { workspace = true, features = ["util"] }` from `[dev-dependencies]` to `[dependencies]`; already in the graph via axum)
- Modify: `crates/vox-foundation/src/protocol.rs` (`API_V2_GET`)
- Modify: `contracts/operations/catalog.v1.yaml` (group anchor comments only)

**Interfaces:**

```rust
// crates/vox-gui/src/remote/calls.rs
pub const GUI_USER_ID: &str = "local_user";
pub trait BackendCalls: Send + Sync {
    fn call_tool(&self, tool: &str, args: serde_json::Value)
        -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    fn api_v2_get(&self, path_and_query: &str)
        -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
}
impl BackendCalls for Backend { /* Local: orch.tool_call / orch.api_v2_get; Remote: GatewayClient */ }
pub fn tool_data<T: serde::de::DeserializeOwned>(tool: &str, envelope: serde_json::Value) -> Result<T, String>;
pub fn api_data<T: serde::de::DeserializeOwned>(envelope: serde_json::Value) -> Result<T, String>;
pub fn path_with_query(path: &str, pairs: &[(&str, Option<String>)]) -> String;

// crates/vox-foundation/src/protocol.rs (orch_daemon_method)
pub const API_V2_GET: &str = "orch.api_v2_get"; // params {"path": "/models/cards?limit=10"}

// crates/vox-orchestrator-mcp/src/gui_ops/inproc.rs
pub async fn api_v2_get(state: &ServerState, path_and_query: &str) -> Result<serde_json::Value, String>;
pub fn validate_api_v2_path(path_and_query: &str) -> Result<(), String>;

// crates/vox-orchestrator-mcp/src/gui_ops/mod.rs
pub const GUI_USER_ID: &str = "local_user";
pub fn require_db(state: &ServerState) -> Result<&std::sync::Arc<vox_db::VoxDb>, String>;
pub(crate) fn api_db(gs: &GatewayState) -> Result<std::sync::Arc<vox_db::VoxDb>, axum::Json<serde_json::Value>>;
pub(crate) fn enforce_dashboard_surface(gs: &GatewayState, surfaces: &[&str]) -> Result<(), axum::Json<serde_json::Value>>;
```

- [ ] **Step 1: Write failing GUI tests** at the bottom of `crates/vox-gui/src/remote/calls.rs`:

```rust
#[cfg(test)]
pub(crate) mod fake {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use serde_json::Value;

    use super::BackendCalls;

    /// Records every call and answers from canned responses keyed `tool:<name>` / `get:<path>`.
    #[derive(Default)]
    pub struct FakeBackend {
        responses: Mutex<HashMap<String, Result<Value, String>>>,
        calls: Mutex<Vec<(String, Value)>>,
    }

    impl FakeBackend {
        pub fn new() -> Self {
            Self::default()
        }
        pub fn on_tool(self, tool: &str, resp: Result<Value, String>) -> Self {
            self.responses.lock().unwrap().insert(format!("tool:{tool}"), resp);
            self
        }
        pub fn on_get(self, path: &str, resp: Result<Value, String>) -> Self {
            self.responses.lock().unwrap().insert(format!("get:{path}"), resp);
            self
        }
        pub fn calls(&self) -> Vec<(String, Value)> {
            self.calls.lock().unwrap().clone()
        }
        fn answer(&self, key: String, args: Value) -> Result<Value, String> {
            self.calls.lock().unwrap().push((key.clone(), args));
            self.responses
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| Err(format!("FakeBackend: no response for {key}")))
        }
    }

    impl BackendCalls for FakeBackend {
        async fn call_tool(&self, tool: &str, args: Value) -> Result<Value, String> {
            self.answer(format!("tool:{tool}"), args)
        }
        async fn api_v2_get(&self, path_and_query: &str) -> Result<Value, String> {
            self.answer(format!("get:{path_and_query}"), Value::Null)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tool_data_unwraps_success_envelope_including_null() {
        let v: String = tool_data("t", json!({"success": true, "data": "x"})).unwrap();
        assert_eq!(v, "x");
        let none: Option<String> = tool_data("t", json!({"success": true, "data": null})).unwrap();
        assert_eq!(none, None);
    }

    #[test]
    fn tool_data_surfaces_tool_error_and_profile_denial() {
        let e = tool_data::<String>("t", json!({"success": false, "error": "boom"})).unwrap_err();
        assert_eq!(e, "t: boom");
        let denial = json!({"error": {"code": "tool_not_in_profile", "message": "tool t is not served"}});
        assert_eq!(tool_data::<String>("t", denial).unwrap_err(), "tool_not_in_profile: tool t is not served");
    }

    #[test]
    fn api_data_unwraps_dashboard_envelope() {
        let v: Vec<u8> = api_data(json!({"v": 1, "data": [1, 2]})).unwrap();
        assert_eq!(v, vec![1, 2]);
        let e = api_data::<Vec<u8>>(json!({"v": 1, "error": {"code": "db_error", "message": "down"}})).unwrap_err();
        assert_eq!(e, "db_error: down");
    }

    #[test]
    fn path_with_query_encodes_and_skips_none() {
        let p = path_with_query("/x", &[("a", Some("b c&d".into())), ("n", None), ("k", Some("1".into()))]);
        assert_eq!(p, "/x?a=b%20c%26d&k=1");
        assert_eq!(path_with_query("/x", &[("n", None)]), "/x");
    }
}
```

- [ ] **Step 2: Run, expect compile failure:**

```bash
cargo nextest run -p vox-gui remote::calls
```
Expected: `error[E0425]: cannot find function \`tool_data\``.

- [ ] **Step 3: Implement `calls.rs`** (above the test modules):

```rust
//! Daemon-backed calls used by migrated `db`-class commands. Local mode reaches the daemon over
//! its TCP RPC (`orch.tool_call`, `orch.api_v2_get`); remote mode over the HTTP gateway
//! (`/v1/tools/call`, `/api/v2/*`). Both hit the same server handlers.

use serde::de::DeserializeOwned;
use serde_json::Value;
use vox_foundation::protocol::orch_daemon_method;
use vox_orchestrator::orch_daemon::OrchDaemonClient;

use super::backend::Backend;
use super::error::AxisError;
use crate::commands::daemon::PersistentDaemon;

/// Deadline for one remote tool call over plan 06's `/v1/ws` RPC.
const REMOTE_CALL_DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);

/// Codex user namespace every GUI preference has always been written under.
pub const GUI_USER_ID: &str = "local_user";

pub trait BackendCalls: Send + Sync {
    fn call_tool(
        &self,
        tool: &str,
        args: Value,
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
    fn api_v2_get(
        &self,
        path_and_query: &str,
    ) -> impl std::future::Future<Output = Result<Value, String>> + Send;
}

async fn daemon_client(daemon: &PersistentDaemon) -> Result<OrchDaemonClient, String> {
    let addr = daemon.ensure().await?;
    Ok(match daemon.token().await {
        Some(token) => OrchDaemonClient::with_token(addr, token),
        None => OrchDaemonClient::new(addr),
    })
}

impl BackendCalls for Backend {
    async fn call_tool(&self, tool: &str, args: Value) -> Result<Value, String> {
        match self {
            Backend::Local(daemon) => daemon_client(daemon)
                .await?
                .call(
                    orch_daemon_method::TOOL_CALL,
                    serde_json::json!({ "name": tool, "args": args }),
                )
                .await
                .map_err(|e| format!("{tool}: {e}")),
            Backend::Remote(gw) => gw
                .call_daemon_method(
                    orch_daemon_method::TOOL_CALL,
                    serde_json::json!({ "name": tool, "args": args }),
                    REMOTE_CALL_DEADLINE,
                )
                .await
                .map_err(String::from),
        }
    }

    async fn api_v2_get(&self, path_and_query: &str) -> Result<Value, String> {
        match self {
            Backend::Local(daemon) => daemon_client(daemon)
                .await?
                .call(
                    orch_daemon_method::API_V2_GET,
                    serde_json::json!({ "path": path_and_query }),
                )
                .await
                .map_err(|e| format!("GET /api/v2{path_and_query}: {e}")),
            // Plan 06's `api_get` already unwraps the `{v,data|error}` envelope; re-wrap so
            // `api_data` sees one shape in both modes. Server-side errors stay `Ok(error envelope)`,
            // transport errors (unreachable, unauthorized, ...) stay `Err`, as in local mode.
            Backend::Remote(gw) => match gw.api_get(&format!("/api/v2{path_and_query}")).await {
                Ok(data) => Ok(serde_json::json!({ "v": 1, "data": data })),
                Err(AxisError::Remote { code, message }) => {
                    Ok(serde_json::json!({ "v": 1, "error": { "code": code, "message": message } }))
                }
                Err(e) => Err(String::from(e)),
            },
        }
    }
}

/// Unwrap a `ToolResult` envelope (`{success,data,error,meta}`) or a profile denial
/// (`{"error":{"code","message"}}`, 00-contracts §Wire shapes) into `T`.
pub fn tool_data<T: DeserializeOwned>(tool: &str, envelope: Value) -> Result<T, String> {
    if let Some(obj) = envelope.get("error").and_then(Value::as_object) {
        let code = obj.get("code").and_then(Value::as_str).unwrap_or("error");
        let msg = obj.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(format!("{code}: {msg}"));
    }
    if envelope.get("success") == Some(&Value::Bool(true)) {
        let data = envelope.get("data").cloned().unwrap_or(Value::Null);
        return serde_json::from_value(data).map_err(|e| format!("{tool}: bad payload: {e}"));
    }
    let msg = envelope.get("error").and_then(Value::as_str).unwrap_or("tool failed");
    Err(format!("{tool}: {msg}"))
}

/// Unwrap the dashboard envelope (`{"v":1,"data":..}` / `{"v":1,"error":{code,message}}`).
pub fn api_data<T: DeserializeOwned>(envelope: Value) -> Result<T, String> {
    if let Some(err) = envelope.get("error") {
        let code = err.get("code").and_then(Value::as_str).unwrap_or("error");
        let msg = err.get("message").and_then(Value::as_str).unwrap_or("");
        return Err(format!("{code}: {msg}"));
    }
    let data = envelope.get("data").cloned().unwrap_or(Value::Null);
    serde_json::from_value(data).map_err(|e| format!("bad /api/v2 payload: {e}"))
}

/// `path?k=v&..` with RFC 3986 unreserved characters kept and everything else percent-encoded.
pub fn path_with_query(path: &str, pairs: &[(&str, Option<String>)]) -> String {
    fn enc(s: &str) -> String {
        s.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                    (b as char).to_string()
                }
                _ => format!("%{b:02X}"),
            })
            .collect()
    }
    let q: Vec<String> = pairs
        .iter()
        .filter_map(|(k, v)| v.as_ref().map(|v| format!("{k}={}", enc(v))))
        .collect();
    if q.is_empty() { path.to_string() } else { format!("{path}?{}", q.join("&")) }
}
```

- [ ] **Step 4: Run GUI tests, expect PASS.** `cargo nextest run -p vox-gui remote::calls` → `4 passed`.

- [ ] **Step 5: Write failing server tests** at the bottom of `crates/vox-orchestrator-mcp/src/gui_ops/inproc.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_escapes_and_absolute_urls() {
        assert!(validate_api_v2_path("/models/cards?limit=3").is_ok());
        for bad in ["models", "/../v1/tools/call", "//evil", "/x#frag", "http://h/x", "/a\\b"] {
            assert!(validate_api_v2_path(bad).is_err(), "{bad} must be rejected");
        }
    }

    #[tokio::test]
    async fn inproc_get_serves_existing_dashboard_route() {
        let state = ServerState::new_test().await;
        let v = api_v2_get(&state, "/vcs/isolation").await.expect("served");
        assert_eq!(v["data"]["strategy_default"], "shared_branch");
    }

    #[tokio::test]
    async fn inproc_get_refuses_post_only_route() {
        // `/hopper/submit` is POST-only: a GET must not reach the mutation.
        let state = ServerState::new_test().await;
        let err = api_v2_get(&state, "/hopper/submit").await.unwrap_err();
        assert!(err.contains("405"), "{err}");
    }
}
```

- [ ] **Step 6: Run, expect failure:** `cargo nextest run -p vox-orchestrator-mcp gui_ops::inproc` → unresolved module `gui_ops`.

- [ ] **Step 7: Implement server plumbing.**

`crates/vox-foundation/src/protocol.rs`, inside `pub mod orch_daemon_method` next to `TOOL_CALL`:

```rust
    /// Params: `{"path": "/models/cards?limit=10"}` → the `/api/v2` JSON envelope. GET-only;
    /// runs the dashboard router in-process inside the daemon (Axis local mode).
    pub const API_V2_GET: &str = "orch.api_v2_get";
```

(If plan 03 added `orch.boot_report` to `contracts/orchestration/orch-daemon-rpc-methods.schema.json`'s method enum, add `"orch.api_v2_get"` there too.)

`crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` — accessor and constructor (non-test):

```rust
impl GatewayState {
    pub(crate) fn server_state(&self) -> &ServerState {
        &self.server_state
    }

    /// Gateway state for `orch.api_v2_get`: the caller already authenticated on the daemon's
    /// token-checked TCP RPC, so HTTP auth is off and the peer is loopback. GET-only is
    /// enforced by `gui_ops::inproc::api_v2_get`, not here.
    pub(crate) fn for_inproc_loopback(server_state: ServerState) -> Self {
        let hopper = crate::gui_ops::orchestrator::hopper_for(&server_state);
        Self {
            server_state,
            bearer_token: None,
            read_bearer_token: None,
            dashboard_token: None,
            allow_unauthenticated: true,
            health_auth_required: false,
            require_forwarded_https: false,
            trust_forwarded_for: false,
            allowed_tools: Arc::new(HashSet::new()),
            read_role_eligible_tools: Arc::new(HashSet::new()),
            read_role_tools_override: None,
            calls_per_minute: DEFAULT_RATE_LIMIT_PER_MINUTE,
            rate_limiter: new_identity_rate_limiter(DEFAULT_RATE_LIMIT_PER_MINUTE),
            public_eval_enabled: false,
            public_eval_rate_limiter: new_identity_rate_limiter(10),
            topic_tx: tokio::sync::broadcast::channel(scientia_feed::TOPIC_CHANNEL_CAPACITY).0,
            hopper,
        }
    }
}
```

(If plans 02/04 added fields to `GatewayState`, initialise them exactly as `for_test_with_state` does.) In the production `GatewayState` construction (the block commented "first non-test caller of `InMemoryHopper` (B3)") and in `for_test_with_state`, replace the `InMemoryHopper::new(..)` expression with `crate::gui_ops::orchestrator::hopper_for(&server_state)`. Task 1's `hopper_for` returns the same `InMemoryHopper`, so behaviour is unchanged until Task 10:

```rust
// gui_ops/orchestrator.rs (Task 1 version)
pub(crate) fn hopper_for(state: &ServerState) -> Arc<dyn vox_orchestrator::hopper::HopperIntake> {
    Arc::new(vox_orchestrator::hopper::InMemoryHopper::new(Arc::new(
        state.orchestrator.event_bus.clone(),
    )))
}
```

`http_gateway/dashboard_api.rs`: make `enforce_dashboard_read` and `enforce_dashboard_write` `pub(crate)`, and merge the group routers at the end of `router()` (this is the **only** edit any task makes to `dashboard_api.rs`):

```rust
        .route("/hopper/history", get(get_hopper_history))
        .merge(crate::gui_ops::models::api_v2_router())
        .merge(crate::gui_ops::prefs::api_v2_router())
        .merge(crate::gui_ops::chat::api_v2_router())
        .merge(crate::gui_ops::knowledge::api_v2_router())
        .merge(crate::gui_ops::research::api_v2_router())
        .merge(crate::gui_ops::attention::api_v2_router())
        .merge(crate::gui_ops::orchestrator::api_v2_router())
        .merge(crate::gui_ops::harness::api_v2_router())
```

`gui_ops/mod.rs`:

```rust
//! Daemon-side handlers backing Axis commands that used to open `vox_db` in the GUI process
//! (capability-profiles plan 07). Reads are `/api/v2` routes merged into
//! `http_gateway::dashboard_api::router()`; mutations are MCP tools dispatched from
//! `crate::dispatch` so `authorize_tool` gates them.

pub mod attention;
pub mod chat;
pub mod dto;
pub mod gamify;
pub mod harness;
pub mod inproc;
pub mod knowledge;
pub mod models;
pub mod orchestrator;
pub mod prefs;
pub mod research;

use std::sync::Arc;

use axum::Json;
use serde_json::Value;

use crate::http_gateway::GatewayState;
use crate::server_state::ServerState;
use crate::services::routes::err;

/// Codex user namespace the GUI has always used for its preferences.
pub const GUI_USER_ID: &str = "local_user";

const REM_GUI_DB: &str = "Attach VoxDb (`VOX_DB_PATH` / `VOX_DB_URL`) to the Vox server; Axis surfaces backed by the workspace DB need it.";

/// The daemon's attached DB, or a stable error string for a `ToolResult`.
pub fn require_db(state: &ServerState) -> Result<&Arc<vox_db::VoxDb>, String> {
    state.db.as_ref().ok_or_else(|| format!("VoxDb not attached. {REM_GUI_DB}"))
}

pub(crate) fn api_db(gs: &GatewayState) -> Result<Arc<vox_db::VoxDb>, Json<Value>> {
    gs.server_state()
        .db
        .clone()
        .ok_or_else(|| err("db_unavailable", REM_GUI_DB))
}

/// Plan-07 routes are served only when the active profile lists one of the Axis surfaces.
pub(crate) fn enforce_dashboard_surface(gs: &GatewayState, surfaces: &[&str]) -> Result<(), Json<Value>> {
    let profile = &gs.server_state().active_profile;
    if surfaces.iter().any(|s| profile.axis_surfaces.contains(s)) {
        Ok(())
    } else {
        Err(err(
            "tool_not_in_profile",
            &format!("surface {surfaces:?} is not served by profile {}", profile.id_string()),
        ))
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashSet;
    use std::sync::Arc;

    use crate::http_gateway::GatewayState;
    use crate::server_state::ServerState;

    pub(crate) async fn state_with_memory_db() -> (ServerState, Arc<vox_db::VoxDb>) {
        let db = Arc::new(vox_db::VoxDb::connect(vox_db::DbConfig::Memory).await.expect("memory db"));
        let state = ServerState::new_test().await.with_db_initialized(db.clone()).await;
        (state, db)
    }

    pub(crate) async fn gateway_with_memory_db() -> (GatewayState, Arc<vox_db::VoxDb>) {
        let (state, db) = state_with_memory_db().await;
        (GatewayState::for_test_with_state(state, HashSet::new()).await, db)
    }

    pub(crate) fn tool_json(out: &str) -> serde_json::Value {
        serde_json::from_str(out).expect("ToolResult JSON")
    }

    pub(crate) async fn body(resp: axum::Json<serde_json::Value>) -> serde_json::Value {
        use axum::response::IntoResponse;
        let bytes = axum::body::to_bytes(resp.into_response().into_body(), 1 << 20).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    pub(crate) fn peer() -> axum::extract::ConnectInfo<std::net::SocketAddr> {
        axum::extract::ConnectInfo("127.0.0.1:1".parse().unwrap())
    }

    #[tokio::test]
    async fn require_db_reports_missing_db() {
        let state = ServerState::new_test().await;
        assert!(super::require_db(&state).unwrap_err().starts_with("VoxDb not attached"));
    }
}
```

Each group file starts as (example `gui_ops/models.rs`; the others are identical apart from the doc line):

```rust
//! Models-profile routes and ops for Axis (plan 07 Tasks 2–3).
use axum::Router;

use crate::http_gateway::GatewayState;

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn router_builds() {
        let _ = super::api_v2_router();
    }
}
```

`gui_ops/inproc.rs` (above its tests):

```rust
//! `orch.api_v2_get`: serve one `/api/v2` GET from inside the daemon so Axis local mode reaches
//! the exact handlers a remote Axis reaches over HTTP.

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Method, Request};
use tower::ServiceExt;

use crate::http_gateway::GatewayState;
use crate::server_state::ServerState;

const MAX_BODY: usize = 16 * 1024 * 1024;

pub fn validate_api_v2_path(path_and_query: &str) -> Result<(), String> {
    let path = path_and_query.split('?').next().unwrap_or("");
    let ok = path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains("..")
        && !path.contains('\\')
        && !path_and_query.contains('#')
        && !path_and_query.contains("://");
    if ok { Ok(()) } else { Err(format!("invalid /api/v2 path: {path_and_query:?}")) }
}

pub async fn api_v2_get(state: &ServerState, path_and_query: &str) -> Result<serde_json::Value, String> {
    validate_api_v2_path(path_and_query)?;
    let app = axum::Router::new()
        .nest("/api/v2", crate::http_gateway::dashboard_api::router())
        .with_state(GatewayState::for_inproc_loopback(state.clone()));
    let mut req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v2{path_and_query}"))
        .body(Body::empty())
        .map_err(|e| e.to_string())?;
    req.extensions_mut().insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let resp = app.oneshot(req).await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), MAX_BODY)
        .await
        .map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("GET /api/v2{path_and_query}: HTTP {}", status.as_u16()));
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}
```

`daemon_extra.rs` — add an arm in `try_handle`'s `match req.method.as_str()`:

```rust
            orch_daemon_method::API_V2_GET => {
                let path = req.params.get("path").and_then(|v| v.as_str()).unwrap_or("");
                Some(match crate::gui_ops::inproc::api_v2_get(&self.state, path).await {
                    Ok(v) => result(&req.id, v),
                    Err(e) => error(&req.id, e),
                })
            }
```

`dispatch.rs` — inside the main `match name {`, immediately before the `_ =>` arm, add the group anchor comments (each later task inserts its literal arms under its own comment; `mcp_wiring`'s TOOL_REGISTRY-vs-handler guard scans for literal names, so no prefix delegation):

```rust
        // ── gui_ops: models (plan 07) ──
        // ── gui_ops: prefs (plan 07) ──
        // ── gui_ops: chat (plan 07) ──
        // ── gui_ops: knowledge (plan 07) ──
        // ── gui_ops: research (plan 07) ──
        // ── gui_ops: orchestrator (plan 07) ──
        // ── gui_ops: harness (plan 07) ──
        // ── gui_ops: gamify (plan 07) ──
```

The same eight comments go in `input_schemas.rs` immediately before its fallback arm, and in `contracts/operations/catalog.v1.yaml` immediately above the top-level `exemptions:` key (as `# ── gui_ops: <group> (plan 07) ──`); ops are appended under their group comment, keeping each group's block contiguous. `lib.rs`: `pub mod gui_ops;` after `pub mod gui_registry_tools;`.

- [ ] **Step 8: Run tests, expect PASS.**

```bash
cargo nextest run -p vox-orchestrator-mcp gui_ops
cargo nextest run -p vox-gui remote::calls
```
Expected: `gui_ops::inproc::tests` 3 passed, `require_db_reports_missing_db` passed, nine `router_builds` passed; GUI 4 passed.

- [ ] **Step 9: Mutation-verify the GET-only and path guards.** (a) In `api_v2_get` change `Method::GET` to `Method::POST`; confirm `grep -c 'Method::POST' crates/vox-orchestrator-mcp/src/gui_ops/inproc.rs` prints `1`; `cargo nextest run -p vox-orchestrator-mcp inproc_get_refuses_post_only_route` → FAIL. Restore; grep prints `0`; re-run → PASS. (b) Replace the body of `validate_api_v2_path` with `Ok(())` (presence check `grep -c 'Ok(())$' …inproc.rs` changes by one); `validate_rejects_escapes_and_absolute_urls` → FAIL; restore → PASS.

- [ ] **Step 10: Checks and commit.**

```bash
cargo check -p vox-orchestrator-mcp -p vox-gui -p vox-foundation
cargo clippy -p vox-orchestrator-mcp -p vox-gui --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-gui/src/backend crates/vox-orchestrator-mcp crates/vox-foundation/src/protocol.rs contracts/operations/catalog.v1.yaml
git commit -m "feat(axis): daemon-backed call plumbing for db-command migration"
```

---

### Task 2: Pattern — `get_active_model` / `set_active_model` through the daemon

The simplest models-profile commands: one preference read (new read-only route) and one write (new narrow op). Both move into a new module `model_active.rs`, so a whole module flips `db → daemon` in this task and the pattern shows the route path, the op path and the manifest flip end to end.

**Files:**
- Create: `crates/vox-gui/src/commands/model_active.rs`, `crates/vox-gui/ui/src/guards/commandBackendsMigration.test.ts`, `crates/vox-gui/ui/e2e/lib/remoteMode.ts`, `crates/vox-gui/ui/e2e/models-remote.spec.ts`, `crates/vox-orchestrator-mcp/src/gui_ops/dto/models.rs`
- Modify: `crates/vox-gui/src/commands/{models.rs,mod.rs}`, `crates/vox-gui/src/main.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/{models.rs,dto/mod.rs}`, `src/dispatch.rs`, `src/input_schemas.rs`, `contracts/operations/catalog.v1.yaml`, `contracts/profiles/models.v1.yaml`, `contracts/gui/command-backends.v1.yaml`

**Interfaces:**

```rust
// vox-orchestrator-mcp gui_ops/models.rs
// GET /api/v2/models/preferences → ModelsPreferencesDto
pub struct ModelsPreferencesDto { pub active_model: Option<String>, pub routing_priority: RoutingPriorityDto, pub selection_policy: String }
pub async fn get_models_preferences(State(gs), ConnectInfo<SocketAddr>, HeaderMap) -> Json<Value>;
pub struct ModelsPreferenceSetParams { pub key: String, pub value: String }
pub const MODELS_PREFERENCE_KEYS: &[&str] = &["active_model", "routing_priority", "selection_policy"];
pub async fn models_preference_set(state: &ServerState, p: ModelsPreferenceSetParams) -> String;
pub fn validate_routing_priority_csv(csv: &str) -> Result<(), String>;
// vox-gui model_active.rs
pub(crate) async fn get_active_model_impl<B: BackendCalls>(b: &B) -> Result<Option<String>, String>;
pub(crate) async fn set_active_model_impl<B: BackendCalls>(b: &B, model_id: String) -> Result<(), String>;
```

- [ ] **Step 0: GATE — remote and local tool-call envelopes are identical (STOP if not).** Task 1's `BackendCalls` Remote arm passes plan 06's `/v1/ws` `call_tool` reply `data` to the same `tool_data` that parses the local `orch.tool_call` result. Prove they are the same JSON before building on it. Append to `crates/vox-orchestrator-mcp/src/http_gateway/ws.rs`:

```rust
#[cfg(test)]
mod envelope_parity_tests {
    use std::sync::Arc;

    use vox_foundation::protocol::orch_daemon_method;
    use vox_orchestrator::orch_daemon::{self, ExtraDispatch};

    use super::ws_handle_message;
    use crate::daemon_extra::McpExtraDispatch;
    use crate::http_gateway::{AccessRole, GatewayState, WsMessageIn};
    use crate::server_state::ServerState;

    /// `/v1/ws` `call_tool` → `data` must equal `orch.tool_call` → `result` for the same tool, for a
    /// success envelope and for a tool-level error envelope.
    #[tokio::test(flavor = "multi_thread")]
    async fn ws_call_tool_data_equals_local_orch_tool_call_result() {
        let state = ServerState::new_test().await;
        let mut gs = GatewayState::for_inproc_loopback(state.clone()); // Task 1
        gs.allowed_tools = Arc::new(["vox_git_status", "vox_get_active_model"].map(String::from).into());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let extra: Arc<dyn ExtraDispatch> = Arc::new(McpExtraDispatch::new(state.clone()));
        tokio::spawn(orch_daemon::serve_listener_with_extra(
            listener, addr.clone(), "parity-repo".into(), state.orchestrator.clone(), Some(extra), None,
        ));
        let client = orch_daemon::OrchDaemonClient::new(addr);
        for _ in 0..750 {
            if client.ping().await.is_ok() { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }

        for (tool, args) in [
            ("vox_git_status", serde_json::json!({})),
            ("vox_get_active_model", serde_json::json!({ "unexpected_arg": true })),
        ] {
            let local = client
                .call(orch_daemon_method::TOOL_CALL, serde_json::json!({ "name": tool, "args": args.clone() }))
                .await
                .expect("orch.tool_call");
            let req: WsMessageIn = serde_json::from_value(serde_json::json!({
                "id": "1", "type": "call_tool", "name": tool, "args": args,
            }))
            .expect("ws frame");
            let remote = ws_handle_message(&gs, req, "127.0.0.1:9".parse().unwrap(), AccessRole::Write).await;
            assert_eq!(remote.data, local, "{tool}: /v1/ws data differs from orch.tool_call result");
        }
    }
}
```

Run: `cargo nextest run -p vox-orchestrator-mcp ws_call_tool_data_equals_local_orch_tool_call_result`.
Expected: PASS. **If it fails, STOP**: do not start Step 1 or any later group task. Record the diff (e.g. the daemon wraps unparseable output as `{"raw": …}` while the gateway returns a bare string) in the PR description and ask the user whether to normalise the gateway (`call_tool_response_value`) or the daemon (`McpExtraDispatch` `TOOL_CALL` arm); the parity test stays as the guard either way. Commit the test alone: `test(mcp): pin ws call_tool / orch.tool_call envelope parity`.

- [ ] **Step 1: Failing server tests** — append to `gui_ops/models.rs`'s `mod tests`:

```rust
    use axum::extract::State;
    use axum::http::HeaderMap;

    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, state_with_memory_db, tool_json};

    #[tokio::test]
    async fn preferences_route_reads_persisted_rows_with_defaults() {
        let (gs, db) = gateway_with_memory_db().await;
        db.set_user_preference("local_user", "active_model", "openai/gpt-x").await.unwrap();
        db.set_user_preference("local_user", "routing_priority", "latency=77").await.unwrap();
        let v = body(super::get_models_preferences(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(v["data"]["active_model"], "openai/gpt-x");
        assert_eq!(v["data"]["routing_priority"]["latency"], 77);
        assert_eq!(v["data"]["selection_policy"], "{\"steps\":[]}");
    }

    #[tokio::test]
    async fn preferences_route_degrades_without_db() {
        let gs = crate::http_gateway::GatewayState::for_test().await;
        let v = body(super::get_models_preferences(State(gs), peer(), HeaderMap::new()).await).await;
        assert!(v["data"]["active_model"].is_null());
        assert!(v["data"]["routing_priority"].is_object());
    }

    #[tokio::test]
    async fn models_preference_set_rejects_keys_outside_allowlist() {
        let (state, db) = state_with_memory_db().await;
        let out = super::models_preference_set(
            &state,
            super::ModelsPreferenceSetParams { key: "socrates_gate_enforced".into(), value: "false".into() },
        )
        .await;
        assert_eq!(tool_json(&out)["success"], false);
        assert_eq!(db.get_user_preference("local_user", "socrates_gate_enforced").await.unwrap(), None);
    }

    #[tokio::test]
    async fn models_preference_set_validates_selection_policy_json() {
        let (state, _db) = state_with_memory_db().await;
        let out = super::models_preference_set(
            &state,
            super::ModelsPreferenceSetParams { key: "selection_policy".into(), value: "{not json".into() },
        )
        .await;
        assert!(tool_json(&out)["error"].as_str().unwrap().contains("invalid selection policy JSON"));
    }

    #[tokio::test]
    async fn models_preference_set_persists_routing_priority() {
        let (state, db) = state_with_memory_db().await;
        let csv = "efficiency=25,precision=30,latency=15,availability=10,balance=10,mobile=10";
        let out = super::models_preference_set(
            &state,
            super::ModelsPreferenceSetParams { key: "routing_priority".into(), value: csv.into() },
        )
        .await;
        assert_eq!(tool_json(&out)["success"], true);
        assert_eq!(db.get_user_preference("local_user", "routing_priority").await.unwrap().as_deref(), Some(csv));
    }

    #[test]
    fn routing_priority_csv_rejects_unknown_axis_and_overflow() {
        assert!(super::validate_routing_priority_csv("efficiency=25,precision=30").is_ok());
        assert!(super::validate_routing_priority_csv("bogus=1").is_err());
        assert!(super::validate_routing_priority_csv("latency=300").is_err());
    }
```

- [ ] **Step 2: Failing GUI tests** at the bottom of the new `crates/vox-gui/src/commands/model_active.rs`:

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[tokio::test]
    async fn get_reads_daemon_preferences_route() {
        let b = FakeBackend::new().on_get(
            "/models/preferences",
            Ok(json!({"v": 1, "data": {"active_model": "openai/gpt-x", "routing_priority": {}, "selection_policy": "{}"}})),
        );
        assert_eq!(get_active_model_impl(&b).await.unwrap().as_deref(), Some("openai/gpt-x"));
        assert_eq!(b.calls()[0].0, "get:/models/preferences");
    }

    /// Override-sensitive paths share one test: the override is a process-wide static.
    #[tokio::test]
    async fn fallback_and_set_semantics() {
        *active_model_override().lock().unwrap() = Some("prior/model".into());
        let absent = FakeBackend::new().on_get(
            "/models/preferences",
            Ok(json!({"v": 1, "data": {"active_model": null, "routing_priority": {}, "selection_policy": "{}"}})),
        );
        assert_eq!(get_active_model_impl(&absent).await.unwrap().as_deref(), Some("prior/model"));

        let rejected = FakeBackend::new().on_tool(
            "vox_models_preference_set",
            Ok(json!({"success": false, "error": "model nope not found in registry"})),
        );
        assert!(set_active_model_impl(&rejected, "nope".into()).await.is_err());
        assert_eq!(active_model_override().lock().unwrap().as_deref(), Some("prior/model"));

        let offline = FakeBackend::new(); // every call errors: server unreachable
        set_active_model_impl(&offline, "local/llama".into()).await.unwrap();
        assert_eq!(active_model_override().lock().unwrap().as_deref(), Some("local/llama"));

        let ok = FakeBackend::new()
            .on_tool("vox_models_preference_set", Ok(json!({"success": true, "data": "active_model"})));
        set_active_model_impl(&ok, " openai/gpt-x ".into()).await.unwrap();
        assert_eq!(ok.calls()[0].1, json!({"key": "active_model", "value": "openai/gpt-x"}));
        assert!(set_active_model_impl(&ok, "  ".into()).await.is_err());
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("model_active.rs");
        assert!(!src.contains(concat!("vox", "_db")) && !src.contains(concat!("tur", "so::")));
    }
}
```

- [ ] **Step 3: Failing UI guard test** `crates/vox-gui/ui/src/guards/commandBackendsMigration.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

// Plan 07: each migrated module must be classified `daemon` in the command-backend manifest.
const MANIFEST = readFileSync(
  join(import.meta.dirname, '../../../../../contracts/gui/command-backends.v1.yaml'),
  'utf8',
);

function moduleClass(module: string): string | null {
  const m = MANIFEST.match(new RegExp(`- module: ${module}\\n\\s+class: (\\w+)`));
  return m ? m[1] : null;
}

describe('plan 07 db-command migration', () => {
  it('pattern: model_active is daemon-class', () => {
    expect(moduleClass('model_active')).toBe('daemon');
  });
});
```

- [ ] **Step 4: Run all three, expect FAIL.**

```bash
cargo nextest run -p vox-orchestrator-mcp gui_ops::models
cargo nextest run -p vox-gui model_active
pnpm --dir crates/vox-gui/ui exec vitest run src/guards/commandBackendsMigration.test.ts
```
Expected: unresolved `get_models_preferences` / `models_preference_set`; unresolved module `model_active`; vitest `expected null to be 'daemon'`.

- [ ] **Step 5: Server implementation.** `gui_ops/dto/models.rs` receives `RoutingPriorityDto` (moved from `vox-gui/src/commands/models.rs`, now `#[derive(Debug, Clone, Serialize, Deserialize)]`, with its `From<AutoRoutingPriority>` impl and `apply_routing_csv` + tests) and:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsPreferencesDto {
    pub active_model: Option<String>,
    /// Effective value: server env (`AutoRoutingPriority::from_env`) overlaid with the persisted CSV.
    pub routing_priority: RoutingPriorityDto,
    /// Persisted `SelectionPolicy` JSON, or `{"steps":[]}` when absent.
    pub selection_policy: String,
}
```

`gui_ops/dto/mod.rs`: `pub mod models;`. Then `gui_ops/models.rs` (above the router and tests):

```rust
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::gui_ops::dto::models::{ModelsPreferencesDto, RoutingPriorityDto, apply_routing_csv};
use crate::gui_ops::{GUI_USER_ID, enforce_dashboard_surface, require_db};
use crate::http_gateway::GatewayState;
use crate::http_gateway::dashboard_api::enforce_dashboard_read;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::ok;

pub const EMPTY_SELECTION_POLICY: &str = "{\"steps\":[]}";
const REM_MODELS_KEY: &str = "Only active_model, routing_priority and selection_policy are models preferences; use the owning surface's tool for other keys.";
const REM_MODELS_PREF_DB: &str = "Verify Turso connectivity and vox-db migrations for user_preferences.";

/// Keys the Models/Settings surfaces persist. Anything else (e.g. `socrates_gate_enforced`,
/// which `vox_preference_set` also applies to live orchestrator config) is refused.
pub const MODELS_PREFERENCE_KEYS: &[&str] = &["active_model", "routing_priority", "selection_policy"];
const ROUTING_AXES: &[&str] = &[
    "efficiency", "cost", "precision", "quality", "latency", "speed", "availability", "balance", "mobile",
];

async fn pref(state: &ServerState, key: &str) -> Option<String> {
    let db = state.db.as_ref()?;
    db.get_user_preference(GUI_USER_ID, key)
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
}

pub(crate) async fn effective_routing_priority(state: &ServerState) -> RoutingPriorityDto {
    let base: RoutingPriorityDto = vox_config::AutoRoutingPriority::from_env().into();
    match pref(state, "routing_priority").await {
        Some(csv) => apply_routing_csv(base, &csv),
        None => base,
    }
}

pub(crate) async fn models_preferences(state: &ServerState) -> ModelsPreferencesDto {
    ModelsPreferencesDto {
        active_model: pref(state, "active_model").await,
        routing_priority: effective_routing_priority(state).await,
        selection_policy: pref(state, "selection_policy")
            .await
            .unwrap_or_else(|| EMPTY_SELECTION_POLICY.to_string()),
    }
}

/// GET /api/v2/models/preferences
pub async fn get_models_preferences(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_dashboard_read(&gs, &connect.0, &headers) {
        return e;
    }
    if let Err(e) = enforce_dashboard_surface(&gs, &["models", "settings"]) {
        return e;
    }
    ok(models_preferences(gs.server_state()).await)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModelsPreferenceSetParams {
    /// One of `active_model`, `routing_priority`, `selection_policy`.
    pub key: String,
    /// Model id, `axis=u8,...` CSV, or `SelectionPolicy` JSON respectively.
    pub value: String,
}

pub fn validate_routing_priority_csv(csv: &str) -> Result<(), String> {
    for part in csv.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (k, v) = part.split_once('=').ok_or_else(|| format!("expected axis=value, got {part:?}"))?;
        let k = k.trim().to_ascii_lowercase();
        if !ROUTING_AXES.contains(&k.as_str()) {
            return Err(format!("unknown routing axis {k:?}"));
        }
        v.trim().parse::<u8>().map_err(|_| format!("axis {k} must be 0-255, got {v:?}"))?;
    }
    Ok(())
}

fn validate_models_preference(state: &ServerState, key: &str, value: &str) -> Result<(), String> {
    match key {
        "active_model" => {
            let id = value.trim();
            if id.is_empty() {
                return Err("model_id must not be empty".into());
            }
            let handle = state.orchestrator.models_handle();
            if vox_orchestrator::sync_lock::rw_read(&*handle).get(id).is_none() {
                return Err(format!("model {id} not found in registry"));
            }
            Ok(())
        }
        "routing_priority" => validate_routing_priority_csv(value),
        "selection_policy" => vox_orchestrator::models::SelectionPolicy::from_json(value)
            .map(|_| ())
            .map_err(|e| format!("invalid selection policy JSON: {e}")),
        other => Err(format!("key '{other}' is not a models preference")),
    }
}

pub async fn models_preference_set(state: &ServerState, p: ModelsPreferenceSetParams) -> String {
    if !MODELS_PREFERENCE_KEYS.contains(&p.key.as_str()) {
        return ToolResult::<String>::err_with_remediation(
            format!("key '{}' is not a models preference", p.key),
            REM_MODELS_KEY,
        )
        .to_json();
    }
    if let Err(e) = validate_models_preference(state, &p.key, &p.value) {
        return ToolResult::<String>::err_with_remediation(e, REM_MODELS_KEY).to_json();
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<String>::err_with_remediation(e, REM_MODELS_PREF_DB).to_json(),
    };
    match db.set_user_preference(GUI_USER_ID, &p.key, p.value.trim()).await {
        Ok(()) => ToolResult::ok(p.key).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_MODELS_PREF_DB).to_json(),
    }
}
```

and the router becomes `Router::new().route("/models/preferences", get(get_models_preferences))`.

`dispatch.rs`, under `// ── gui_ops: models (plan 07) ──`:

```rust
        "vox_models_preference_set" => Ok(crate::gui_ops::models::models_preference_set(
            state,
            serde_json::from_value(args)?,
        )
        .await),
```

`input_schemas.rs`, under its models anchor:

```rust
        "vox_models_preference_set" => derived_tool_schema!(crate::gui_ops::models::ModelsPreferenceSetParams),
```

`catalog.v1.yaml`, under `# ── gui_ops: models (plan 07) ──`:

```yaml
- id: models.preference.set
  title: Models Preference Set
  description: Persist one Models/Settings preference (active_model, routing_priority, selection_policy) for the local GUI user; other keys are refused and values are validated.
  description_human: null
  product_lane: ai
  intent_tags: []
  side_effect_class: writes_files
  scope_kind: null
  reversible: null
  requires_repo: null
  preferred_for_models: null
  human_takeover_friendly: null
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp:
    name: vox_models_preference_set
    http_read_role_eligible: false
    exposure: operator
    requires: [db, model_registry]
  cli: null
```

Per D1 (`models` requires `db`), append `vox_models_preference_set` to `tools.include` in `contracts/profiles/models.v1.yaml`; the `vox-mcp-registry` build re-checks that its `requires` is inside the models closure. Regenerate:

```bash
cargo run -p vox-cli -- ci operations-sync --target mcp --write
```
Expected: `contracts/mcp/tool-registry.canonical.yaml` gains `vox_models_preference_set` with `exposure: operator`, `side_effect_class: writes_files`; no other diff.

- [ ] **Step 6: GUI implementation.** Create `crates/vox-gui/src/commands/model_active.rs` by **moving** `active_model_override`, `active_model_fallback`, `get_active_model`, `set_active_model` and the two existing `active_model_fallback_*` tests out of `models.rs`, then rewrite the two commands:

```rust
//! Active-model preference (Models surface), served by the daemon — `GET /api/v2/models/preferences`
//! and `vox_models_preference_set` — so it works against a local or a remote Vox server.

use std::sync::{Arc, Mutex, OnceLock};

use serde_json::json;
use vox_orchestrator_mcp::gui_ops::dto::models::ModelsPreferencesDto;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, tool_data};

/// GUI-process display fallback when the server is unreachable or has no row. Not daemon
/// routing state (that is `mcp_chat_model_override`).
fn active_model_override() -> &'static Mutex<Option<String>> {
    static OVERRIDE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

fn active_model_fallback() -> Option<String> {
    if let Some(v) = active_model_override().lock().unwrap().clone() {
        return Some(v);
    }
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxModel)
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub(crate) async fn get_active_model_impl<B: BackendCalls>(b: &B) -> Result<Option<String>, String> {
    let prefs = match b.api_v2_get("/models/preferences").await {
        Ok(env) => api_data::<ModelsPreferencesDto>(env).ok(),
        Err(_) => None,
    };
    Ok(prefs.and_then(|p| p.active_model).or_else(active_model_fallback))
}

pub(crate) async fn set_active_model_impl<B: BackendCalls>(b: &B, model_id: String) -> Result<(), String> {
    let trimmed = model_id.trim().to_string();
    if trimmed.is_empty() {
        return Err("model_id must not be empty".into());
    }
    let args = json!({ "key": "active_model", "value": trimmed });
    // A tool-level rejection (unknown model, profile denial) goes back to the UI; an
    // unreachable server keeps today's degrade-to-display-override behaviour.
    if let Ok(env) = b.call_tool("vox_models_preference_set", args).await {
        tool_data::<String>("vox_models_preference_set", env)?;
    }
    *active_model_override().lock().unwrap() = Some(trimmed);
    Ok(())
}

#[tauri::command]
pub async fn get_active_model(backend: tauri::State<'_, Arc<BackendState>>) -> Result<Option<String>, String> {
    get_active_model_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn set_active_model(
    model_id: String,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    set_active_model_impl(&backend.current().await, model_id).await
}
```

In `models.rs`, `get_routing_summary(daemon: &PersistentDaemon)` becomes `get_routing_summary(daemon: &PersistentDaemon, backend: &crate::remote::backend::Backend)` and its `get_active_model().await` becomes `crate::commands::model_active::get_active_model_impl(backend).await`; `get_routing_summary_live` gains `backend: tauri::State<'_, Arc<BackendState>>` and passes `&backend.current().await` (`rg -n 'get_routing_summary\(' crates/vox-gui/src` lists every caller). There are no per-command `local_only` guards to delete: the manifest row below (`model_active` → `daemon`) is what lets plan 06's invoke gate pass these two commands in remote mode. `commands/mod.rs`: `pub mod model_active;`. `main.rs` `generate_handler!`: `commands::models::get_active_model` → `commands::model_active::get_active_model`, same for `set_active_model`.

`contracts/gui/command-backends.v1.yaml`, add:

```yaml
- module: model_active
  class: daemon
  surfaces: [models]
```

- [ ] **Step 7: Playwright remote spec.** `crates/vox-gui/ui/e2e/lib/remoteMode.ts`:

```ts
import type { Page } from '@playwright/test';

/**
 * Install a Tauri IPC mock in which Axis reports remote mode (plan 06 `axis_backend_status`).
 * Plan 06 computes per-surface availability in Rust from the embedded manifest; this mock
 * bypasses Rust, so each spec states it: every `axisSurfaces` entry is available unless
 * `unavailable` names a reason for it (`surfaceAvailability` treats unlisted surfaces as
 * `not_in_profile`).
 */
export async function installRemoteMode(
  page: Page,
  view: string,
  responses: Record<string, unknown>,
  axisSurfaces: string[],
  unavailable: Record<string, 'not_in_profile' | 'needs_local_machine' | 'not_remote_capable'> = {},
): Promise<void> {
  await page.addInitScript(
    ({ view, responses, axisSurfaces, unavailable }) => {
      localStorage.setItem('vox_sidebar_mode', 'default');
      localStorage.setItem('vox_onboarding_dismissed', 'true');
      (window as any).__VOX_INVOKES__ = [] as Array<[string, unknown]>;
      (window as any).__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: unknown) => {
          (window as any).__VOX_INVOKES__.push([cmd, args]);
          if (cmd === 'get_initial_view') return view;
          if (cmd === 'axis_backend_status') {
            return {
              mode: 'remote',
              url: 'https://vox.lan:3921/',
              profile: {
                profiles: ['dev'],
                max_exposure: 'operator',
                subsystems: { booted: [], failed: [] },
                tools: [],
                axis_surfaces: axisSurfaces,
                server_version: '0.6.0',
              },
              surfaces: axisSurfaces.map((surface) =>
                unavailable[surface]
                  ? { surface, available: false, reason: unavailable[surface] }
                  : { surface, available: true, reason: null },
              ),
            };
          }
          return cmd in responses ? responses[cmd] : null;
        },
      };
    },
    { view, responses, axisSurfaces, unavailable },
  );
}

export const SHOT_DIR = 'review-bundle/latest';
```

`crates/vox-gui/ui/e2e/models-remote.spec.ts` — the Models surface needs every one of its modules to be daemon-class, and `models.rs` is still `db` until Task 3, so this task pins the intermediate state and Task 3 replaces the assertion:

```ts
import { test, expect } from '@playwright/test';
import { installRemoteMode, SHOT_DIR } from './lib/remoteMode';

test.describe('Models surface — remote mode', () => {
  test('stays gated while the models module is still db-class', async ({ page }) => {
    await installRemoteMode(page, 'models', { get_active_model: 'openai/gpt-x' }, ['models'], { models: 'not_remote_capable' });
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toBeVisible();
    await page.screenshot({ path: `${SHOT_DIR}/models-remote-gated.png`, fullPage: true });
  });
});
```

- [ ] **Step 8: Run everything, expect PASS.**

```bash
cargo nextest run -p vox-orchestrator-mcp gui_ops::models
cargo nextest run -p vox-gui model_active remote::calls
pnpm --dir crates/vox-gui/ui exec vitest run src/guards/commandBackendsMigration.test.ts
pnpm --dir crates/vox-gui/ui typecheck
pnpm --dir crates/vox-gui/ui exec playwright test e2e/models-remote.spec.ts e2e/models.spec.ts
```
Expected: 6 new server tests pass; `model_active` 3 tests + the 2 moved fallback tests pass; vitest 1 passed; typecheck clean; Playwright all passed and `review-bundle/latest/models-remote-gated.png` exists.

- [ ] **Step 9: Mutation-verify the key allowlist.** Comment out the `if !MODELS_PREFERENCE_KEYS.contains(..) { .. }` block **and** change `validate_models_preference`'s `other =>` arm to `_ => Ok(())`; confirm `grep -c '_ => Ok(())' crates/vox-orchestrator-mcp/src/gui_ops/models.rs` prints `1`; `cargo nextest run -p vox-orchestrator-mcp models_preference_set_rejects_keys_outside_allowlist` → FAIL (row written). Restore both; grep prints `0`; re-run → PASS.

- [ ] **Step 10: Verify and commit.**

```bash
rg -n 'vox_db|turso' crates/vox-gui/src/commands/model_active.rs   # expect no output
cargo run -p vox-cli -- ci gui-catalog-parity                     # expect: ok
cargo clippy -p vox-gui -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add -A crates/vox-gui crates/vox-orchestrator-mcp contracts
git commit -m "feat(axis): serve active-model preference through the daemon (plan 07 pattern)"
```

---

### Task 3: Models group (rest of `models.rs` + Models surface reads)

**Files:**
- Modify: `crates/vox-gui/src/commands/models.rs`, `llm_settings.rs` (move `inference_provider_status` out), `main.rs`, `mod.rs`; create `models_host.rs`
- Modify: `crates/vox-orchestrator-mcp/src/gui_ops/models.rs`, `gui_ops/dto/models.rs`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.tsx` (+ `.test.tsx`), `ui/e2e/models-remote.spec.ts`, `ui/e2e/models.spec.ts` (mocks), `ui/src/guards/commandBackendsMigration.test.ts`
- Modify: `contracts/gui/command-backends.v1.yaml`

**Interfaces (server routes; each starts with `enforce_dashboard_read` then `enforce_dashboard_surface(&gs, &["models", "settings", "runs"])`):**

| Route | Handler | Body moved from (vox-gui) | Returns |
|---|---|---|---|
| `GET /models/cards?limit=` | `get_model_cards` | `registry_with_scoreboard`, `refresh_live_mens_catalog`, `merge_live_mens_specs`, `model_spec_to_card`, `list_model_cards` body | `Vec<ModelCardDto>` |
| `GET /models/routing-summary` | `get_routing_summary_gui` | `get_routing_summary`, with `active_model` from `models_preferences(st)` and `global_exploration_cost_usd` read from `serde_json::to_value(st.orchestrator.status())` in-process (no TCP hop) | `RoutingSummaryDto` |
| `GET /models/scoreboard?window_days=` | `get_model_scoreboard_route` | `get_model_scoreboard` | `Vec<ScoreboardRowDto>` |
| `GET /models/provider-status` | `get_provider_status` | `llm_settings::inference_provider_status`, `probe_vox_local`, `local_availability` | `Vec<ProviderStatusDto>` |
| `GET /models/snapshot?limit=` | `get_models_snapshot` | composes cards + routing summary + preferences + providers | `ModelsSnapshotDto { cards, routing, active_model, providers }` |

(`GET /models/preferences` exists from Task 2.) DTOs `ModelCardDto`, `RoutingSummaryDto`, `DecisionPreviewDto`, `ScoreboardRowDto` (with `impl From<vox_db::ModelScoreboardRow>` — the mapping inline in `get_model_scoreboard` today), `ProviderStatusDto`, `RoutingIntentionDto`, `ModelsSnapshotDto` move to `gui_ops/dto/models.rs` with `#[derive(Debug, Clone, Serialize, Deserialize)]`; `vox-gui` re-exports them (`pub use vox_orchestrator_mcp::gui_ops::dto::models::*;`). The pure fns `priority_to_intentions`, `nudge_axis`, `priority_to_csv`, `axis_value`, `axis_note` move to `dto/models.rs` with their existing tests.

GUI commands after migration (each Tauri command keeps its arguments, adds `backend: tauri::State<'_, Arc<BackendState>>`, and delegates to `<name>_impl<B: BackendCalls>(b: &B, ..)`):

| Command | `_impl` body |
|---|---|
| `list_model_cards(limit)` | `api_data(b.api_v2_get(&path_with_query("/models/cards", &[("limit", limit.map(|l| l.to_string()))])).await?)` |
| `get_routing_summary_live()` | `api_data(b.api_v2_get("/models/routing-summary").await?)` (the `daemon` parameter is dropped) |
| `get_model_scoreboard(window_days)` | `api_data(b.api_v2_get(&path_with_query("/models/scoreboard", &[("window_days", window_days.map(|w| w.to_string()))])).await?)` |
| `get_selection_policy()` | `b.api_v2_get("/models/preferences").await.ok().and_then(|e| api_data::<ModelsPreferencesDto>(e).ok()).map(|p| p.selection_policy).unwrap_or_else(|| EMPTY_SELECTION_POLICY.to_string())` |
| `set_selection_policy(json)` | keep the local `SelectionPolicy::from_json` pre-check; then `tool_data::<String>("vox_models_preference_set", b.call_tool("vox_models_preference_set", json!({"key":"selection_policy","value":json})).await?)?; Ok(())` |
| `set_routing_priority(efficiency, precision, latency, availability, balance, mobile)` | same op with `key:"routing_priority"`, value = the existing CSV `format!`; **delete** `unsafe { std::env::set_var("VOX_AUTO_ROUTING_PRIORITY", ..) }` (the daemon reads the preference) |
| `get_routing_intentions()` | `let p: ModelsPreferencesDto = api_data(b.api_v2_get("/models/preferences").await?)?; Ok(priority_to_intentions(&p.routing_priority))` |
| `nudge_routing_intention(axis, direction)` | read as above, `nudge_axis(&mut p.routing_priority, ..)`, then `vox_models_preference_set {key:"routing_priority", value: priority_to_csv(&p.routing_priority)}` |
| `inference_provider_status()` (moved into `models.rs`) | `api_data(b.api_v2_get("/models/provider-status").await?)` |
| `get_models_snapshot(limit)` (new command) | `api_data(b.api_v2_get(&path_with_query("/models/snapshot", &[("limit", limit.map(|l| l.to_string()))])).await?)` |

`models_host.rs` (host class): `search_model_cards`, `explain_model_selection`, `suggest_model_for_task`, `get_auto_model_recommendation` moved verbatim with `registry_from_cache` (and `model_spec_to_card` imported from `dto::models`).

- [ ] **Step 1: Failing server tests** in `gui_ops/models.rs` — one per route, calling the handler directly:

```rust
    use axum::extract::Query;

    #[tokio::test]
    async fn scoreboard_route_returns_rows_array() {
        let (gs, _db) = gateway_with_memory_db().await;
        let q = Query(super::WindowQuery { window_days: Some(7) });
        let v = body(super::get_model_scoreboard_route(State(gs), peer(), HeaderMap::new(), q).await).await;
        assert!(v["data"].is_array());
    }

    #[tokio::test]
    async fn scoreboard_route_reports_missing_db() {
        let gs = crate::http_gateway::GatewayState::for_test().await;
        let q = Query(super::WindowQuery { window_days: None });
        let v = body(super::get_model_scoreboard_route(State(gs), peer(), HeaderMap::new(), q).await).await;
        assert_eq!(v["error"]["code"], "db_unavailable");
    }

    #[tokio::test]
    async fn snapshot_bundles_cards_routing_active_and_providers() {
        let (gs, db) = gateway_with_memory_db().await;
        db.set_user_preference("local_user", "active_model", "openai/gpt-x").await.unwrap();
        let q = Query(super::LimitQuery { limit: Some(5) });
        let v = body(super::get_models_snapshot(State(gs), peer(), HeaderMap::new(), q).await).await;
        assert!(v["data"]["cards"].as_array().unwrap().len() <= 5);
        assert_eq!(v["data"]["active_model"], "openai/gpt-x");
        assert!(v["data"]["routing"]["routing_priority"].is_object());
        assert!(v["data"]["providers"].is_array());
    }

    #[tokio::test]
    async fn models_routes_are_profile_gated() {
        let (state, _db) = state_with_memory_db().await;
        let state = state.with_profile(crate::profiles::ActiveProfile::resolve(&["knowledge"]).expect("knowledge")); // plan 02 Task 4
        let gs = crate::http_gateway::GatewayState::for_test_with_state(state, Default::default()).await;
        let v = body(super::get_models_preferences(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
    }
```

- [ ] **Step 2: Failing GUI tests** in `models.rs` with `FakeBackend`, one per table row; two of them:

```rust
    #[tokio::test]
    async fn nudge_reads_preferences_then_writes_csv_through_op() {
        let b = FakeBackend::new()
            .on_get("/models/preferences", Ok(json!({"v":1,"data":{"active_model":null,"selection_policy":"{}","routing_priority":{"efficiency":20,"precision":20,"latency":20,"availability":20,"balance":10,"mobile":10}}})))
            .on_tool("vox_models_preference_set", Ok(json!({"success":true,"data":"routing_priority"})));
        nudge_routing_intention_impl(&b, "latency".into(), "up".into()).await.unwrap();
        let calls = b.calls();
        assert_eq!(calls[0].0, "get:/models/preferences");
        assert_eq!(calls[1].1["key"], "routing_priority");
        assert!(calls[1].1["value"].as_str().unwrap().contains("latency="));
    }

    #[test]
    fn models_module_has_no_direct_db_access_or_env_mutation() {
        let src = include_str!("models.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("set_", "var(")), "routing priority must not mutate the GUI process env");
    }
```

Append to `commandBackendsMigration.test.ts`:

```ts
  it('models group: models is daemon, models_host is host', () => {
    expect(moduleClass('models')).toBe('daemon');
    expect(moduleClass('models_host')).toBe('host');
  });
```

Append to `ModelsView.test.tsx`:

```tsx
  it('refreshes through one batched get_models_snapshot call', async () => {
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'get_models_snapshot'
        ? Promise.resolve({ cards: CARDS, routing: SUMMARY, active_model: 'openai/gpt-x', providers: [] })
        : baseImpl(cmd),
    );
    render(<ModelsView pushToast={vi.fn()} />);
    await waitFor(() => expect(screen.getByText('ollama/llama')).toBeTruthy());
    const cmds = invokeMock.mock.calls.map((c) => c[0]);
    expect(cmds).toContain('get_models_snapshot');
    expect(cmds).not.toContain('list_model_cards');
  });
```

- [ ] **Step 3: Run, expect FAIL:** `cargo nextest run -p vox-orchestrator-mcp gui_ops::models`; `cargo nextest run -p vox-gui commands::models`; `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Models src/guards`.

- [ ] **Step 4: Implement server routes** in `gui_ops/models.rs`:

```rust
#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}
#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    pub window_days: Option<i64>,
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/models/preferences", get(get_models_preferences))
        .route("/models/cards", get(get_model_cards))
        .route("/models/routing-summary", get(get_routing_summary_gui))
        .route("/models/scoreboard", get(get_model_scoreboard_route))
        .route("/models/provider-status", get(get_provider_status))
        .route("/models/snapshot", get(get_models_snapshot))
}

fn guard_models(gs: &GatewayState, peer: &SocketAddr, headers: &HeaderMap) -> Result<(), Json<Value>> {
    enforce_dashboard_read(gs, peer, headers)?;
    enforce_dashboard_surface(gs, &["models", "settings", "runs"])
}

pub async fn get_model_scoreboard_route(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<WindowQuery>,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    let db = match crate::gui_ops::api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db.get_model_scoreboard(q.window_days.unwrap_or(7)).await {
        Ok(rows) => ok(rows.into_iter().map(ScoreboardRowDto::from).collect::<Vec<_>>()),
        Err(e) => crate::services::routes::err("db_error", &e.to_string()),
    }
}

pub async fn get_models_snapshot(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    let st = gs.server_state();
    ok(ModelsSnapshotDto {
        cards: model_cards(st, q.limit.unwrap_or(2000)).await,
        routing: routing_summary(st).await,
        active_model: models_preferences(st).await.active_model,
        providers: provider_status().await,
    })
}
```

`model_cards(st, limit)`, `routing_summary(st)`, `provider_status()` are the moved GUI bodies with every `vox_db::connect_workspace_journey_optional(..)` replaced by `st.db.as_ref()` (an `Option<&Arc<VoxDb>>`, preserving today's degrade-when-absent behaviour). `get_model_cards`, `get_routing_summary_gui`, `get_provider_status` are wrappers of the same shape as `get_models_snapshot`: `guard_models` then `ok(<fn>(..).await)`.

- [ ] **Step 5: Implement GUI side** per the table. Remove `registry_with_scoreboard` from `vox-gui` (`rg -n registry_with_scoreboard crates/vox-gui/src` lists the remaining callers — `routing_explain.rs` today; switch it to `ModelRegistry::from_cache()` plus `api_data::<Vec<ScoreboardRowDto>>(b.api_v2_get("/models/scoreboard").await?)` for its scoreboard injection, and record that in the commit body). Move `inference_provider_status` from `llm_settings.rs` to `models.rs`. Add `get_models_snapshot` to `generate_handler!`; update paths for the four `models_host` commands. (No per-command `local_only` guards exist; the manifest flip to `daemon` un-gates the migrated commands.)

`ModelsView.tsx` `refresh`: replace the `Promise.all([...4 invokes])` with

```tsx
      const snap = await invoke<ModelsSnapshot>('get_models_snapshot', { limit: 120 });
      const cards = snap.cards;
      const routing = snap.routing;
      const active = snap.active_model;
      const statuses = snap.providers;
```

and add `type ModelsSnapshot = { cards: ModelCard[]; routing: RoutingSummary; active_model: string | null; providers: ProviderStatus[] };` next to the file's existing types, so the setter code below the old `Promise.all` is unchanged.

Manifest rows:

```yaml
- module: models
  class: daemon
  surfaces: [models, settings, runs, chat]
- module: models_host
  class: host
  surfaces: [chat]
```

(`llm_settings` keeps its plan-06 class; it no longer hosts a Models command.)

- [ ] **Step 6: Flip the Playwright spec** — replace the gated test in `models-remote.spec.ts` with:

```ts
  test('renders cards on a remote server', async ({ page }) => {
    await installRemoteMode(
      page,
      'models',
      {
        get_models_snapshot: {
          cards: [{ id: 'free/model-a', provider: 'openrouter', tier: 'Free', cost_per_1k: 0, max_tokens: 8000, is_free: true, latency_p50_ms: 400 }],
          routing: { decision_preview: null, routing_priority: { efficiency: 20, precision: 20, latency: 20, availability: 20, balance: 10, mobile: 10 } },
          active_model: null,
          providers: [],
        },
        set_active_model: null,
      },
      ['models'],
    );
    await page.goto('/');
    await expect(page.getByText(/cannot run against a remote server yet/i)).toHaveCount(0);
    await expect(page.getByText('free/model-a')).toBeVisible();
    await page.screenshot({ path: `${SHOT_DIR}/models-remote.png`, fullPage: true });
  });
```

Update `e2e/models.spec.ts` mocks to answer `get_models_snapshot` with the same shape (its existing `list_model_cards` fixtures become `cards`).

- [ ] **Step 7: Run, expect PASS** — Step 3's commands plus `pnpm --dir crates/vox-gui/ui typecheck` and `pnpm --dir crates/vox-gui/ui exec playwright test e2e/models-remote.spec.ts e2e/models.spec.ts e2e/settings.spec.ts e2e/routing-explainer.spec.ts e2e/model-picker-interactions.spec.ts`.

- [ ] **Step 8: Verify and commit.**

```bash
rg -n 'vox_db|turso|connect_workspace' crates/vox-gui/src/commands/models.rs crates/vox-gui/src/commands/models_host.rs   # no output
cargo run -p vox-cli -- ci gui-catalog-parity    # ok
cargo clippy -p vox-gui -p vox-orchestrator-mcp --all-targets -- -D warnings
git add -A crates/vox-gui crates/vox-orchestrator-mcp contracts
git commit -m "feat(axis): migrate Models surface db commands behind /api/v2 and MCP"
```

**Parallel-safe with:** Tasks 4–11 once Task 2 has merged (disjoint GUI modules and `gui_ops` files; shared registries per §Coordination).

---

### Task 4: Chat sessions (models profile, `chat.rs`)

**Files:** `crates/vox-gui/src/commands/chat.rs`, `crates/vox-gui/src/remote/calls.rs` (deadline), `crates/vox-gui/src/commands/mcp.rs` (deadline fn moves); `crates/vox-orchestrator-mcp/src/gui_ops/{chat.rs,dto/chat.rs}`; `dispatch.rs`, `input_schemas.rs`, `catalog.v1.yaml` (chat anchors); manifest; `ui/e2e/chat-remote.spec.ts`; guard test.

**Interfaces:**

| Kind | Name | Params / query | Server body moved from |
|---|---|---|---|
| route | `GET /chat/sessions` | `limit, include_archived` | `chat_list_sessions` (tuple rows → `ChatSessionDto`) |
| route | `GET /chat/sessions/{session_id}/messages` | `limit` | `chat_get_messages` |
| op | `vox_chat_session_create` | `ChatSessionCreateParams { title: Option<String> }` | `chat_create_session` (`chat_ensure_gui_session_with_repo`; repo id resolved on the server) |
| op | `vox_chat_session_append` | `ChatSessionAppendParams { session_id, role, content, metadata_json: Option<String> }` | `chat_append_message` + `persist_assistant_reply` |
| op | `vox_chat_session_rename` | `ChatSessionRenameParams { session_id, title }` | `chat_rename_session` |
| op | `vox_chat_session_archive` | `ChatSessionArchiveParams { session_id, archived: bool }` | `chat_archive_session` / `chat_unarchive_session` (`archived:false` uses `chat_find_gui_conversation_id_including_archived`) |

Routes gated by surface `chat`. Ops: `exposure: operator`, `side_effect_class: writes_files`, `requires: [db]`, `product_lane: ai`. Names match the models profile's existing `vox_chat_*` include. `ChatSessionDto`, `ChatMessageDto`, `ChatAppendInput` move to `dto/chat.rs`.

GUI: `chat_create_session`, `chat_append_message`, `chat_rename_session`, `chat_archive_session`, `chat_unarchive_session` call their op through `tool_data`; `chat_list_sessions`/`chat_get_messages` call the routes through `api_data(path_with_query(..))`. `chat_send_message` keeps its `vox_chat_message` daemon call but replaces both `pool.handle()` uses: user message → `vox_chat_session_append {role:"user"}`; history → `GET /chat/sessions/{id}/messages`; assistant reply (`persist_assistant_reply`) → `vox_chat_session_append {role:"assistant", metadata_json}`. Its `daemon` parameter becomes `backend`, and the `vox_chat_message` call becomes `b.call_tool("vox_chat_message", ..)`; move `deadline_for_tool` (with its two tests) from `mcp.rs` into `remote/calls.rs` and use `call_with_deadline(.., deadline_for_tool(tool))` in the Local arm of `call_tool`, and pass `deadline_for_tool(tool)` in place of `REMOTE_CALL_DEADLINE` in the Remote arm (then delete that const), so Deep turns keep the longer deadline in both modes (`mcp.rs` imports it back). Events emitted by `chat_append_message` (`app_handle.emit(..)`) stay GUI-side and fire after the op succeeds. `GuiDbPool` disappears from every chat command signature.

- [ ] **Step 1: Failing tests.** Server (`gui_ops/chat.rs`): create → append ×2 → `GET /chat/sessions` lists it → `GET …/messages` returns both in order; rename changes the title; archive hides it from `include_archived=false`, unarchive restores. GUI (`chat.rs`): each command issues exactly one expected call key (`tool:vox_chat_session_create`, `get:/chat/sessions?limit=200&include_archived=false`, …); `chat_send_message_impl` issues `tool:vox_chat_session_append` (role user), `tool:vox_chat_message`, `tool:vox_chat_session_append` (role assistant) in that order; `include_str!("chat.rs")` contains neither `vox_db` nor `GuiDbPool`. UI guard: `expect(moduleClass('chat')).toBe('daemon')`.
- [ ] **Step 2: Run, expect FAIL:** `cargo nextest run -p vox-orchestrator-mcp gui_ops::chat`; `cargo nextest run -p vox-gui commands::chat remote::calls`.
- [ ] **Step 3: Implement** server ops/routes (moved bodies; `require_db(state)?` / `api_db(&gs)?` replace `pool.handle()`), dispatch arms and `derived_tool_schema!` entries under the chat anchors, four catalog blocks in the format of Task 2 Step 5 with the values above, then `cargo run -p vox-cli -- ci operations-sync --target mcp --write`.
- [ ] **Step 4: Implement GUI** per above; manifest row `- module: chat` / `class: daemon` / `surfaces: [chat]`.
- [ ] **Step 5: Playwright** `chat-remote.spec.ts`: `installRemoteMode(page, 'chat', { chat_list_sessions: [{ id: 's1', title: 'Remote session', updated_at: '2026-10-08T00:00:00Z', archived: false }], chat_get_messages: [] }, ['chat'])`; assert "Remote session" visible and no "not yet remote-capable" text; screenshot `chat-remote.png`. Re-run `e2e/chat-session-rail.spec.ts e2e/session-rail-actions.spec.ts e2e/chat-interactions.spec.ts`.
- [ ] **Step 6: Run, expect PASS; mutation-check** read-role eligibility: flip `http_read_role_eligible` of `vox_chat_session_archive` to `true`, `operations-sync --write`, run plan 01's registry contract test (`cargo nextest run -p vox-mcp-registry classify`) — "operator ⇒ not read-eligible" must FAIL; restore, regenerate, PASS.
- [ ] **Step 7: Verify and commit.** `rg -n 'vox_db|GuiDbPool' crates/vox-gui/src/commands/chat.rs` → empty; `cargo run -p vox-cli -- ci gui-catalog-parity` → ok; `git commit -m "feat(axis): serve chat sessions through the daemon"`.

**Parallel-safe with:** Tasks 3, 5–11 (owns `chat.rs`, `gui_ops/chat.rs`, the chat anchors). Touches `remote/calls.rs` (append-only: one fn moved in).

---

### Task 5: GUI preferences (cross-cutting, `preferences.rs`) — client-side in remote mode (D1)

Decision D1 (`00-contracts.md` Amendments): in **remote** mode Axis keeps `gui.*` preferences
**client-side** and never reads or writes them through the server. In **local** mode they stay in
today's rows (`local_user` in the workspace DB), reached through the local daemon so `vox-gui` can
drop `vox-db` in Task 13. `preferences` therefore stays a **`shell`**-class module (plan 06 row 43;
never gated), not `daemon`.

Server side follows Task 11's precedent: the two ops are `exposure: internal`, so `authorize_tool`
serves them only to loopback callers and only under `dev` (named profiles cannot include `internal`
tools). A remote server can never serve them, by construction. No `/api/v2` route is added
(`/api/v2` has no `internal` class).

**Files:** `crates/vox-gui/src/commands/preferences.rs`; new `crates/vox-gui/src/remote/local_prefs.rs`
(+ `pub mod local_prefs;` in `crates/vox-gui/src/remote/mod.rs`); `gui_ops/prefs.rs`; prefs anchors in
`dispatch.rs`, `input_schemas.rs`, `catalog.v1.yaml`; `contracts/gui/command-backends.v1.yaml` (notes
only); guard test. **No** `contracts/profiles/*.v1.yaml` edits.

**Interfaces:**

```rust
// gui_ops/prefs.rs (server; reached in local mode via orch.tool_call)
pub const GUI_PREFERENCE_PREFIX: &str = "gui.";
pub fn is_gui_preference_key(key: &str) -> bool;   // starts with "gui." and len <= 128
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GuiPreferenceGetParams { pub key: String }
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GuiPreferenceSetParams { pub key: String, pub value: String }
pub async fn gui_preference_get(state: &ServerState, p: GuiPreferenceGetParams) -> String; // ToolResult<Option<String>>
pub async fn gui_preference_set(state: &ServerState, p: GuiPreferenceSetParams) -> String; // ToolResult<String>

// crates/vox-gui/src/remote/local_prefs.rs (client-side store, remote mode only)
pub(crate) struct AxisLocalPrefs;
impl AxisLocalPrefs {
    pub(crate) fn at(path: std::path::PathBuf) -> Self;
    pub(crate) fn for_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Self, String>; // <app_config_dir>/axis-preferences.json
    pub(crate) fn get(&self, key: &str) -> Result<Option<String>, String>;
    pub(crate) fn set(&self, key: &str, value: &str) -> Result<(), String>;
}

// crates/vox-gui/src/commands/preferences.rs
pub(crate) async fn get_gui_preference_impl<B: BackendCalls>(b: &B, remote: bool, local: &AxisLocalPrefs, key: String) -> Result<Option<String>, String>;
pub(crate) async fn set_gui_preference_impl<B: BackendCalls>(b: &B, remote: bool, local: &AxisLocalPrefs, key: String, value: String) -> Result<(), String>;
```

Every UI key already starts with `gui.` (measured: `gui.theme`, `gui.keybinds`, `gui.memory.autoRecall`,
`gui.sign`, `gui.telemetry`, `gui.checkpointMins`, `gui.${k}`), so no data migration in local mode. Both
the ops and the client store refuse non-`gui.` keys, so neither can read or write other preferences
(e.g. `socrates_gate_enforced`, `attention_enabled`).

Behaviour:
- **Remote** (`Backend::Remote(_)`): `AxisLocalPrefs` only. The backend is never called.
- **Local**: `vox_gui_preference_get` / `vox_gui_preference_set` through `orch.tool_call`. If the local
  daemon denies them (`tool_not_in_profile` / `subsystem_not_booted` — e.g. Axis started its daemon with a
  named `VOX_PROFILE`) or is unreachable, the error is **returned to the UI**; Axis does not silently write
  the client file in local mode (no split between two stores).

**Known limitations (accepted consequences of D1):**
- No cross-machine sync in remote mode: `gui.*` preferences live in each Axis machine's
  `axis-preferences.json`, so they do not follow the user to another machine connected to the same server.
- No migration of values set remotely before this task: between plan 06 and this task, remote-mode
  preferences went to the client machine's local workspace DB; they are not copied into
  `axis-preferences.json`.

- [ ] **Step 1: Failing tests.**
  - Server (`gui_ops/prefs.rs`): `gui_preference_set` refuses `socrates_gate_enforced` and `attention_enabled` (row not written; `state.orchestrator.config_handle()` unchanged) and accepts `gui.theme`; `gui_preference_get` returns `null` data for a missing `gui.*` key and refuses `attention_enabled`. `authorize_tool(&ActiveProfile::dev(), &all_up, CallContext { role: CallerRole::Write, loopback: false }, "vox_gui_preference_set")` → `Err(ProfileDenial::ExposureDenied)`; with `CallContext::LOCAL_WRITE` → `Ok`. `ActiveProfile::resolve(&["models"])` does not contain either tool.
  - GUI (`local_prefs.rs`, tempdir): `set` then `get` round-trips; missing file → `Ok(None)`; non-`gui.` key → `Err`.
  - GUI (`preferences.rs`, `FakeBackend`): remote → no backend call recorded and the value comes from `AxisLocalPrefs`; local → `tool:vox_gui_preference_get` / `tool:vox_gui_preference_set` recorded; local with the fake returning a `tool_not_in_profile` denial → `Err` starting with `tool_not_in_profile` **and** the tempdir `axis-preferences.json` is not created (no silent client write); local with the fake returning `Err("unreachable")` → the same `Err`, file not created. `include_str!("preferences.rs")` contains no `vox_db`.
  - UI guard (`commandBackendsMigration.test.ts`): `moduleClass('preferences') === 'shell'`.
- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement.**

```rust
// gui_ops/prefs.rs
pub fn is_gui_preference_key(key: &str) -> bool {
    key.starts_with(GUI_PREFERENCE_PREFIX) && key.len() <= 128
}

fn refuse_key(key: &str) -> String {
    ToolResult::<String>::err_with_remediation(
        format!("key '{key}' is not a GUI preference"),
        "GUI preferences are namespaced `gui.*`; use the owning tool for other keys.",
    )
    .to_json()
}

pub async fn gui_preference_get(state: &ServerState, p: GuiPreferenceGetParams) -> String {
    if !is_gui_preference_key(&p.key) {
        return refuse_key(&p.key);
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<Option<String>>::err(e).to_json(),
    };
    match db.get_user_preference(GUI_USER_ID, &p.key).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<Option<String>>::err(e.to_string()).to_json(),
    }
}

pub async fn gui_preference_set(state: &ServerState, p: GuiPreferenceSetParams) -> String {
    if !is_gui_preference_key(&p.key) {
        return refuse_key(&p.key);
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    match db.set_user_preference(GUI_USER_ID, &p.key, &p.value).await {
        Ok(()) => ToolResult::ok(p.key).to_json(),
        Err(e) => ToolResult::<String>::err(e.to_string()).to_json(),
    }
}
```

`local_prefs.rs`: a `BTreeMap<String, String>` serialized as JSON at `<app_config_dir>/axis-preferences.json`
(`app.path().app_config_dir()`), guarded by a process-wide `std::sync::Mutex`; `set` reads, updates, writes
to `axis-preferences.json.tmp`, then renames over the file. Both methods refuse keys that fail
`key.starts_with("gui.") && key.len() <= 128` (same rule as the server; duplicate the one-liner with
`// vox:defactored-from vox-orchestrator-mcp 2026-10-08` rather than adding a crate edge).

```rust
// commands/preferences.rs
use std::sync::Arc;

use serde_json::json;

use crate::remote::backend::{Backend, BackendState};
use crate::remote::calls::{BackendCalls, tool_data};
use crate::remote::local_prefs::AxisLocalPrefs;

/// Local mode never writes the client file: a denial or an unreachable daemon is surfaced to the UI
/// (reviewer decision), so preferences have exactly one home per mode.
pub(crate) async fn get_gui_preference_impl<B: BackendCalls>(
    b: &B, remote: bool, local: &AxisLocalPrefs, key: String,
) -> Result<Option<String>, String> {
    if remote {
        return local.get(&key);
    }
    let env = b.call_tool("vox_gui_preference_get", json!({ "key": key })).await?;
    tool_data::<Option<String>>("vox_gui_preference_get", env)
}

pub(crate) async fn set_gui_preference_impl<B: BackendCalls>(
    b: &B, remote: bool, local: &AxisLocalPrefs, key: String, value: String,
) -> Result<(), String> {
    if remote {
        return local.set(&key, &value);
    }
    let env = b.call_tool("vox_gui_preference_set", json!({ "key": key, "value": value })).await?;
    tool_data::<String>("vox_gui_preference_set", env).map(|_| ())
}

#[tauri::command]
pub async fn get_gui_preference(
    key: String,
    app: tauri::AppHandle,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Option<String>, String> {
    let local = AxisLocalPrefs::for_app(&app)?;
    let b = backend.current().await;
    let remote = matches!(b, Backend::Remote(_));
    get_gui_preference_impl(&b, remote, &local, key).await
}

#[tauri::command]
pub async fn set_gui_preference(
    key: String,
    value: String,
    app: tauri::AppHandle,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    let local = AxisLocalPrefs::for_app(&app)?;
    let b = backend.current().await;
    let remote = matches!(b, Backend::Remote(_));
    set_gui_preference_impl(&b, remote, &local, key, value).await
}
```

Catalog (append under the prefs marker comment): `id: gui.preference.get` → MCP `vox_gui_preference_get`,
`product_lane: data`, `side_effect_class: read_only`, `mcp.exposure: internal`, `mcp.requires: [db]`;
`id: gui.preference.set` → `vox_gui_preference_set`, `side_effect_class: writes_files`,
`mcp.exposure: internal`, `mcp.requires: [db]`. Neither is added to any named profile (only `dev` serves
`internal`). Manifest row stays `preferences / shell / [memory, settings]`; set its `notes` to
"Axis-local gui.* preferences: remote = client-side axis-preferences.json (D1); local = loopback-only internal ops on the local daemon, errors surfaced (no client-side fallback)". `operations-sync --write`.

- [ ] **Step 4: Mutation-verify** both guards, with before/after presence checks:
  - `gui.` key guard: make `is_gui_preference_key` return `key.len() <= 128` only; `grep -c 'starts_with(GUI_PREFERENCE_PREFIX)' crates/vox-orchestrator-mcp/src/gui_ops/prefs.rs` → `0`; both server refusal tests FAIL; restore (grep → `1`), PASS.
  - Internal exposure: in the `gui.preference.set` catalog block change `exposure: internal` → `operator`, `operations-sync --write`; the off-loopback `ExposureDenied` test FAILs; restore, regenerate, PASS.
- [ ] **Step 5: Run `pnpm --dir crates/vox-gui/ui exec playwright test e2e/settings.spec.ts e2e/onboarding.spec.ts`** (mocked; unchanged) and commit `feat(axis): GUI preferences client-side when remote, via loopback daemon locally`.

**Parallel-safe with:** Tasks 3, 4, 6–11 (owns `preferences.rs`, `gui_ops/prefs.rs`, `remote/local_prefs.rs`, the prefs anchors; edits no profile manifest).

---

### Task 6: Knowledge group (`knowledge.rs`, `memory.rs`, `search.rs`)

**Files:** GUI `knowledge.rs`, `memory.rs`, `search.rs`, new `locator.rs`; `vox-db/src/store/ops_memory/knowledge.rs` (new store fn); `gui_ops/{knowledge.rs,dto/knowledge.rs}`; knowledge anchors; `contracts/profiles/knowledge.v1.yaml`; manifest; `ui/e2e/memory-remote.spec.ts`; guard test.

**Interfaces:**

```rust
// vox-db (store, ops_memory/knowledge.rs) — replaces raw SQL in memory.rs::get_memory_status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchCorpusCounts { pub proj: u32, pub docs: u32, pub chats: u32, pub rules: u32, pub web: u32 }
impl VoxDb { pub async fn search_corpus_counts(&self) -> Result<SearchCorpusCounts, StoreError>; }
```

| Kind | Name | Params / query | Server body moved from |
|---|---|---|---|
| route | `GET /knowledge/nodes` | `node_type, page, limit` | `list_knowledge_nodes` |
| route | `GET /knowledge/health` | — | `get_kb_health` |
| route | `GET /knowledge/corpus-counts` | — | `get_memory_status` (now `db.search_corpus_counts()` → `MemoryStatusPayload { corpus_counts }`) |
| route | `GET /knowledge/recall` | `q, limit` | `mnemosyne_recall` (`MemoryManager::with_db(..)` on the daemon's repo root and `state.db`) |
| route | `GET /search/query` | `q, scope (comma list), kinds (comma list), path_glob, limit, offset` | `vox_search_query` (`SearchRuntimeContext::new(repo_root, state.db.clone(), log_dir, memory_md_path)` from the daemon's workspace) |
| op | `vox_kb_node_delete` | `{ id }` | `delete_knowledge_node` — `destructive` |
| op | `vox_kb_ingest_text` | `{ title, content, source_url: Option<String> }` | `ingest_text` — `writes_files` |
| op | `vox_kb_ingest_url` | `{ url }` | `ingest_url` (same timeout/user-agent client, now on the server) — `writes_files` |
| op | `vox_kb_import_research_session` | `{ session_id: i64 }` | `save_research_session_to_kb` — `writes_files` |
| op | `vox_memory_reindex` | `{}` | `mnemosyne_reindex` — `writes_files`, `requires: [db, memory]` |

Routes gated by surface `memory` (`/search/query` by `memory` or `vox-search`). Ops `exposure: operator`, `requires: [db]` unless noted, `product_lane: data`; names fall under the knowledge profile's `vox_kb_*` / `vox_memory_*` globs. `KnowledgeNodeRow`, `KbHealthDto`, `MemoryStatusPayload`, `UiHitResult`, `SearchResponseDto` move to `dto/knowledge.rs`. `open_locator` + `OpenLocatorDto`/`OpenOutcomeDto` move verbatim to `locator.rs` (host).

- [ ] **Step 1: Failing tests.** vox-db: `search_corpus_counts` on a fresh memory DB returns `SearchCorpusCounts::default()` and counts one upserted `knowledge_nodes` row as `docs = 1`. Server: each route on the memory DB; `vox_kb_ingest_text` then `GET /knowledge/nodes` lists it; `vox_kb_node_delete` removes it. GUI: call-key test per command; `vox_search_query_impl` encodes `scope`/`kinds` as comma lists; no `vox_db`/`turso` in the three modules. UI guard: `knowledge`, `memory`, `search` daemon; `locator` host.
- [ ] **Step 2: Run, expect FAIL:** `cargo nextest run -p vox-db search_corpus_counts`; `-p vox-orchestrator-mcp gui_ops::knowledge`; `-p vox-gui commands::knowledge commands::memory commands::search`.
- [ ] **Step 3: Implement** the store fn (the five `SELECT COUNT(*)` statements move from `memory.rs` into `vox-db` using the store's existing query helper), routes, ops, catalog blocks, `operations-sync --write`; confirm the globs cover the names: `rg -n 'vox_kb_\*|vox_memory_\*' contracts/profiles/knowledge.v1.yaml` → 2 matches.
- [ ] **Step 4: GUI rewrite**; the three modules drop `GuiDbPool` and `turso`. Manifest rows: `knowledge` → `[memory, research]`, `memory` → `[memory]`, `search` → `[memory, vox-search]` (all daemon); `locator` → host `[memory, research]`.
- [ ] **Step 5: Playwright** `memory-remote.spec.ts`: `installRemoteMode(page, 'memory', { list_knowledge_nodes: [{ id: 'n1', title: 'Remote note', node_type: 'doc' }], get_memory_status: { corpus_counts: { proj: 1, docs: 1, chats: 0, rules: 0, web: 0 } }, get_kb_health: { total: 1 } }, ['memory'])`; assert "Remote note"; screenshot `memory-remote.png`.
- [ ] **Step 6: Verify and commit.** `rg -n 'vox_db|turso|GuiDbPool' crates/vox-gui/src/commands/{knowledge,memory,search}.rs` → empty; `ci gui-catalog-parity` → ok; `git commit -m "feat(axis): migrate knowledge/memory/search db commands"`.

**Parallel-safe with:** Tasks 3–5, 7–11 (its `vox-db` file differs from Task 10's).

---

### Task 7: Research group (`scientia.rs`, `scientia_review.rs`, `research.rs`, `search_probe.rs`, `harness_issues.rs`)

**Files:** those GUI modules + new `research_host.rs`; `gui_ops/{research.rs,harness.rs,dto/research.rs,dto/harness.rs}`; research + harness anchors; `contracts/profiles/{research,research-publish}.v1.yaml`; manifest; `ui/e2e/research-remote.spec.ts`; guard test.

**Routes** (surface gate in parentheses):

| Route | Moved from |
|---|---|
| `GET /research/sessions?limit=` (research) | `list_research_sessions` |
| `GET /research/sessions/{id}` (research) | `get_research_session_detail` |
| `GET /research/sessions/{id}/doc-draft` (research) | `generate_research_doc_draft` |
| `GET /research/engine-status` (research) | `get_research_engine_status` (secret presence via the server's `vox_secrets::resolve_secret`) |
| `GET /scientia/manifests?limit=` (scientia, publications) | `list_publication_manifests` |
| `GET /scientia/publications/{id}/review-queue` (scientia) | `list_publication_review_queue` |
| `GET /scientia/publications/{id}/claims/{claim_id}/evidence` (scientia) | `suggest_evidence_improvements` |
| `GET /scientia/publications/{id}/novelty` (scientia) | `get_novelty_assessment` |
| `GET /scientia/publications/{id}/completion` (scientia; `#[cfg(feature = "news-publish")]`) | `get_completion_report` |
| `GET /scientia/publications/{id}/archive` (scientia) | `get_archive_status` |
| `GET /scientia/discoveries?limit=` (scientia) | `list_discovery_inbox` |
| `GET /harness/fix-proposals?status=` (scientia) | `list_harness_fix_proposals` |
| `GET /harness/issues?status=&source=` (scientia) | `list_harness_issues` |
| `GET /harness/issues/by-session/{key}` (scientia) | `list_harness_issues_for_session` |
| existing `GET /scientia/queue` | `scientia_dashboard_snapshot` (GUI change only) |
| existing `GET /scientia/cost` | `scientia_cost_rollup` (GUI change only) |

**Ops** (all `exposure: operator`, `requires: [db]` unless noted, `product_lane: research`; profile in brackets):

| Op | Params | Moved from | side_effect_class |
|---|---|---|---|
| `vox_scientia_publication_claim_review` [research-publish] | `{ publication_id, claim_id: i64, decision, reason: Option<String> }` | `record_publication_claim_review` | `writes_files` |
| `vox_scientia_publication_nanopublish` [research-publish] | `{ publication_id, claim_id: i64, orcid: Option<String> }` | `nanopublish_approved_claim` | `remote_mutation` |
| `vox_scientia_publication_autofill` [research] | `{ publication_id, apply: bool }` | `run_autofill` | `writes_files` |
| `vox_scientia_discovery_ack` [research] | `{ id: i64 }` | `acknowledge_discovery` | `writes_files` |
| `vox_research_flag_misleading` [research] | `ResearchFlagParams { params: vox_db::RecordMisguidanceParams }` | `flag_research_misleading` | `writes_files` |
| `vox_research_publish_doc` [research] | `{ session_id: i64, slug, content }` | `publish_research_doc` | `writes_files` |
| `vox_research_engine_config_set` [research] | `ResearchEngineConfigDto` | `save_research_engine_config` | `writes_files` |
| `vox_research_engine_probe` [research] | `{ provider: Option<String>, query }` (None = all) | `probe_search_provider` / `probe_all_search_providers` | `network` |
| `vox_harness_corpus_scan` [research] | `{}` | `scan_training_corpus` (walks the **server** workspace corpus) | `writes_files` |
| `vox_harness_fix_propose` [research] | `{ issue_id: i64, target_path }`, `requires: [db, llm_facade]` | `propose_harness_issue_fix` | `writes_files` |
| `vox_harness_fix_resolve` [research] | `{ proposal_id: i64, approve: bool }` | `resolve_harness_fix_proposal` | `writes_files` |
| `vox_harness_issue_decide` [research] | `{ issue_id: i64, decision, reason: Option<String> }` | `record_harness_issue_decision` | `writes_files` |

`HarnessIssueRow` / `HarnessFixProposalRow`, which `harness_issues.rs` returns to the UI today, get `dto/harness.rs` siblings (`HarnessIssueDto`, `HarnessFixProposalDto`, `From` impls) — row types do not cross the wire. `research.rs` keeps `start_research_async` and `persist_research_claims` (already daemon), rewritten onto `BackendCalls`; `save_research_doc` + `execute_sandbox_probe` move to `research_host.rs` verbatim.

- [ ] **Step 1: Failing tests.** Server: one test per route on the memory DB (empty envelopes, plus one seeded row each for sessions, manifests, discoveries, harness issues) and per op (row written / status changed); `vox_research_engine_config_set` then `GET /research/engine-status` reflects `research.enabled_providers`. GUI: call-key test per command; `scientia_dashboard_snapshot_impl` → `get:/scientia/queue`; `scientia_cost_rollup_impl` → `get:/scientia/cost`; no `vox_db`/`GuiDbPool` in the five modules. UI guard: the five modules daemon, `research_host` host.
- [ ] **Step 2: Run, expect FAIL:** `cargo nextest run -p vox-orchestrator-mcp gui_ops::research gui_ops::harness`; `cargo nextest run -p vox-gui commands::scientia commands::scientia_review commands::research commands::search_probe commands::harness_issues`.
- [ ] **Step 3: Implement** routes/ops with moved bodies (`scientia_review::db()` → `require_db(state)?` / `api_db(&gs)?`), catalog blocks, dispatch arms + derived schemas under the research and harness anchors; `research.v1.yaml` gets every op above except the two marked research-publish, which go to `research-publish.v1.yaml`; `operations-sync --write`.
- [ ] **Step 4: GUI rewrite** + manifest rows (`scientia`, `scientia_review`, `harness_issues` → `[scientia]`; `research`, `search_probe` → `[research]`; all daemon; `research_host` host `[research]`).
- [ ] **Step 5: Playwright** `research-remote.spec.ts`: (1) view `research` with `list_research_sessions: [{ id: 1, query: 'remote query', status: 'completed' }]`, `get_research_engine_status: { providers: [], active_lane: 'fast' }` → assert "remote query"; (2) view `scientia` with `scientia_dashboard_snapshot` returning the fixture used by `e2e/dashboard-pilot.spec.ts` → assert the dashboard heading. Screenshots `research-remote.png`, `scientia-remote.png`. Re-run `e2e/research-trace-panel.spec.ts e2e/harness-issue-discovery.spec.ts`.
- [ ] **Step 6: Mutation-verify** the publish split: add `vox_scientia_publication_nanopublish` to `research.v1.yaml`, run plan 02's profile contract test (`cargo nextest run -p vox-mcp-registry profile`) asserting "research excludes publish/approve" → FAIL; remove → PASS.
- [ ] **Step 7: Verify and commit.** `rg -n 'vox_db|GuiDbPool|turso' crates/vox-gui/src/commands/{scientia,scientia_review,research,search_probe,harness_issues}.rs` → empty (the `research_doc_io` import moved to `research_host.rs`; Task 13 removes it); `ci gui-catalog-parity` → ok; commit `feat(axis): migrate research/scientia/harness-issue db commands`.

**Parallel-safe with:** Tasks 3–6, 8–11. Task 10 appends eval routes to `gui_ops/harness.rs` — merge 7 first or union-resolve its `.route(..)` lines.

---

### Task 8: Mesh group (`console_a2a.rs`)

**Files:** `crates/vox-gui/src/commands/console_a2a.rs`; `crates/vox-orchestrator-mcp/src/a2a_tools.rs` (test, and the binding fix if needed); manifest; `crates/vox-gui/src/remote/manifest.rs` (real-gate tests); new `crates/vox-gui/ui/e2e/fixtures/remote-gate-mesh-console.json`; `ui/e2e/console-remote.spec.ts`; guard test.

**Interfaces:** existing tool `vox_a2a_send` (`A2ASendParams { sender_id, receiver_id, msg_type, payload, correlation_id, tenant_id, sender_session_id, route }`); the mesh profile already includes `vox_a2a_*`.

- [ ] **Step 1: Failing tests.** GUI:

```rust
    #[tokio::test]
    async fn send_to_agent_uses_durable_a2a_route() {
        let b = FakeBackend::new().on_tool("vox_a2a_send", Ok(json!({"success": true, "data": {"message_id": "m-1"}})));
        let id = send_to_agent_impl(&b, "A-05".into(), "hello".into()).await.unwrap();
        assert_eq!(id, "m-1");
        assert_eq!(
            b.calls()[0].1,
            json!({"sender_id": 0, "receiver_id": 5, "msg_type": "free_form", "payload": "hello", "route": "db"})
        );
    }
```

Server (`a2a_tools.rs` tests): `a2a_send` with `sender_id: 0, route: "db"` on a memory DB succeeds, and `a2a_history` returns the message — this pins that the console operator id passes `resolve_sender_binding` and fixes the success payload key the GUI reads. If it fails, the fix is in `resolve_sender_binding` (treat id 0 as the reserved console operator, as `CONSOLE_OPERATOR_AGENT_ID` does in the GUI), covered by the same test.

- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement.**

```rust
pub(crate) async fn send_to_agent_impl<B: BackendCalls>(b: &B, agent_id: String, body: String) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("message body is empty".into());
    }
    let receiver = parse_receiver(&agent_id)?;
    let args = json!({
        "sender_id": CONSOLE_OPERATOR_AGENT_ID,
        "receiver_id": receiver.0,
        "msg_type": "free_form",
        "payload": body,
        "route": "db",
    });
    let data: serde_json::Value = tool_data("vox_a2a_send", b.call_tool("vox_a2a_send", args).await?)?;
    data.get("message_id")
        .and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_i64().map(|n| n.to_string())))
        .ok_or_else(|| "vox_a2a_send returned no message_id".to_string())
}
```

(If Step 1's server test shows a different success key, use it here and in the GUI test.) Manifest row `console_a2a / daemon / [console]`.

- [ ] **Step 4: Assert the honest remote state with the REAL gate (no hand-written surfaces).** `console_a2a` has no surface of its own (its only surface is `console`), and `pty` / `terminal_core` are `host`-class modules on `console`, so console stays **unavailable remotely** (`needs_local_machine`) after this task. This task therefore migrates `console_a2a`'s db commands and pins that state; it does **not** claim console works remotely (`00-contracts.md` Amendments: console is host-only in remote mode for now).
  - **Rust (real gate).** Append to the test module of `crates/vox-gui/src/remote/manifest.rs` (plan 06 Task 5):

```rust
    /// Plan 07 Task 8: console_a2a is daemon-class now, but console stays host-only remotely.
    #[test]
    fn console_stays_needs_local_machine_remotely_after_console_a2a_migration() {
        assert_eq!(module_class("console_a2a"), Some(ModuleClass::Daemon));
        let host_on_console: Vec<&str> = command_backends()
            .commands
            .iter()
            .filter(|r| r.class == ModuleClass::Host && r.surfaces.iter().any(|s| s == "console"))
            .map(|r| r.module.as_str())
            .collect();
        assert!(host_on_console.contains(&"pty") && host_on_console.contains(&"terminal_core"), "{host_on_console:?}");
        let served = strings(&["mesh", "console"]);
        let out = surface_availability(command_backends(), registry_surfaces(), Some(&served));
        assert_eq!(reason_of(&out, "console"), Some(UnavailableReason::NeedsLocalMachine));
        assert_eq!(reason_of(&out, "mesh"), None);
    }

    /// The Playwright fixture is the real gate's output, byte-for-byte; regenerate by pasting the
    /// `left` value from a failure into `ui/e2e/fixtures/remote-gate-mesh-console.json`.
    #[test]
    fn remote_gate_fixture_matches_real_gate() {
        let served = strings(&["mesh", "console"]);
        let got = serde_json::json!({
            "axis_surfaces": served,
            "surfaces": surface_availability(command_backends(), registry_surfaces(), Some(&served)),
        });
        let want: serde_json::Value = serde_json::from_str(include_str!(
            "../../ui/e2e/fixtures/remote-gate-mesh-console.json"
        ))
        .expect("fixture json");
        assert_eq!(got, want);
    }
```

  - **Playwright** `crates/vox-gui/ui/e2e/console-remote.spec.ts`: import the committed fixture `./fixtures/remote-gate-mesh-console.json` (produced by the real gate above — never edited by hand) and call `installRemoteMode(page, 'console', {}, fixture.axis_surfaces, Object.fromEntries(fixture.surfaces.filter((s) => !s.available).map((s) => [s.surface, s.reason])))`. Assert the console surface renders plan 06's `needs_local_machine` copy ("This surface works on files, processes, or settings on this machine. Disconnect from the server to use it.") and that `__VOX_INVOKES__` contains no `send_to_agent` / `pty_*` call; assert `mesh` (from the same fixture) is navigable. Screenshot `console-remote-unavailable.png`. The migrated `send_to_agent` path is covered by Step 1's `FakeBackend` test and the server `a2a_send` test, and locally by the unchanged `e2e/console-workbench.spec.ts`.
- [ ] **Step 5: Verify and commit.** `rg -n 'vox_db' crates/vox-gui/src/commands/console_a2a.rs` → empty; `ci gui-catalog-parity` → ok; commit `feat(axis): send console notes via vox_a2a_send`.

**Parallel-safe with:** all other group tasks.

---

### Task 9: Attention group (`get_llm_spend`) and skills verification

**Files:** new `crates/vox-gui/src/commands/llm_spend.rs`; `user_config.rs` (remove `get_llm_spend`); `gui_ops/{attention.rs,dto/attention.rs}`; `main.rs`; manifest; `contracts/profiles/attention.v1.yaml` (`axis_surfaces` must include `settings` for the spend meter); `ui/e2e/settings-spend-remote.spec.ts`; guard test.

**Interfaces:** route `GET /llm/spend?session_id=` (surface `settings`) → `LlmSpendDto { session_usd, day_usd, total_usd, daily_budget_usd, per_session_budget_usd }` built from `db.llm_spend_summary(session)` (zeros when no DB, as today) and the **server's** `vox_config::VoxConfig::load()` caps. Per D1 the `attention` profile is DB-free: the route must not require `db` (it uses `state.db` when present), so against an `attention` server it returns zeros with the server caps, and against `dev` or any db-booting profile it returns real spend.

- [ ] **Step 1: Failing tests.** Server: no DB → zeros with caps; memory DB → numeric envelope. GUI: `get_llm_spend_impl(&b, Some("s1".into()))` → `get:/llm/spend?session_id=s1`; `include_str!("llm_spend.rs")` and `include_str!("user_config.rs")` contain no `vox_db`.
- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement** (`LlmSpendDto` moves to `dto/attention.rs`). Manifest rows: `llm_spend / daemon / [settings]`; `user_config` stays `host`.
- [ ] **Step 4: Skills check** — prove there is nothing to migrate:

```bash
rg -l 'vox_db|connect_workspace|turso' crates/vox-gui/src/commands | rg -i skill   # expect no output
```

Write "skills group: 0 db-class commands" in the commit body.

- [ ] **Step 5: Playwright** `settings-spend-remote.spec.ts`: `installRemoteMode(page, 'settings', { get_llm_spend: { session_usd: 0.5, day_usd: 1.25, total_usd: 9, daily_budget_usd: 5, per_session_budget_usd: 1 } }, ['settings'])`; assert `$1.25` visible; screenshot `settings-spend-remote.png`.
- [ ] **Step 6: Verify and commit** (`ci gui-catalog-parity` ok) — `feat(axis): serve LLM spend from the server`.

**Parallel-safe with:** all other group tasks.

---

### Task 10: Orchestrator group (`orchestrator.rs`, `plan_panel.rs`, `runs.rs`, `activity.rs`, `harness_eval.rs`)

**Files:** those GUI modules; `vox-db/src/store/ops_agents.rs` (`latest_interaction_tokens`) and new `vox-db/src/store/ops_activity.rs` (`query_activity_log`; registered in `store/mod.rs`); `gui_ops/{orchestrator.rs,harness.rs,dto/orchestrator.rs}`; orchestrator anchors; `contracts/profiles/orchestrator.v1.yaml`; manifest; `ui/e2e/tasks-remote.spec.ts`, `ui/e2e/harness-health-remote.spec.ts`; guard test.

**Interfaces:**

```rust
// vox-db
impl VoxDb {
    /// input + output tokens of the most recent `llm_interactions` row for a session (0 if none).
    pub async fn latest_interaction_tokens(&self, session_id: &str) -> Result<u64, StoreError>;
    /// Filtered `activity_log` read; `ActivityLogFilter` mirrors the GUI `ActivityFilter`.
    pub async fn query_activity_log(&self, filter: &ActivityLogFilter) -> Result<Vec<ActivityLogEntry>, StoreError>;
}
// gui_ops/orchestrator.rs — replaces the Task 1 body
pub(crate) fn hopper_for(state: &ServerState) -> Arc<dyn vox_orchestrator::hopper::HopperIntake> {
    match &state.db {
        Some(db) => Arc::new(vox_orchestrator::hopper::SqliteHopper::new(db.clone())),
        None => Arc::new(vox_orchestrator::hopper::InMemoryHopper::new(Arc::new(
            state.orchestrator.event_bus.clone(),
        ))),
    }
}
```

`hopper_for` makes the gateway's existing `/api/v2/hopper/*` routes and Axis read the same persistent queue (today the gateway reads an in-memory hopper that Axis never writes to). `for_test` (no DB) keeps `InMemoryHopper`, so existing gateway tests are unchanged.

| Kind | Name | Params / query | Moved from |
|---|---|---|---|
| route | `GET /orchestrator/context-budget?session_id=` (chat) | | `get_context_budget` (SQL → `latest_interaction_tokens`) |
| route | `GET /hopper/list` (tasks, dashboard) | | `hopper_list` (inbox + assigned + `history_recent(DONE_HISTORY_LIMIT)` filtered to `Done`, via `hopper_item_to_dto`) |
| op | `vox_hopper_submit` | `{ intent, affinity: Vec<String> }` | `hopper_submit` |
| op | `vox_hopper_reprioritize` | `{ item_id, priority: u8 }` | `hopper_reprioritize` (override mint label "Tauri GUI" → "Axis") |
| op | `vox_hopper_cancel` | `{ item_id }` | `hopper_cancel` — `destructive` |
| op | `vox_hopper_mark_done` | `{ item_id }` | `hopper_mark_done` |
| route | `GET /plan/sessions/{id}/nodes?version=` (chat) | | `list_plan_nodes` |
| route | `GET /plan/open-task-counts?session_ids=a,b` (chat) | | `plan_open_task_counts` |
| route | `GET /plan/latest-for-chat/{session_id}` (chat) | | `latest_plan_session_for_chat` |
| op | `vox_plan_node_upsert` | `{ mode: "update"\|"insert", update: Option<UpdatePlanNodeInput>, insert: Option<InsertPlanNodeInput> }` | `update_plan_node`, `insert_plan_node` |
| op | `vox_plan_nodes_approve` | `{ plan_session_id }` | `approve_plan_nodes` |
| op | `vox_gui_run_start` | `StartGuiRunInput` | `start_gui_run` |
| op | `vox_gui_run_finish` | `FinishGuiRunInput` (today's positional args as one struct) | `finish_gui_run` |
| route | `GET /runs/gui?limit=` (runs, models) | | `list_gui_runs` |
| route | `GET /runs/gui/{run_id}` (runs) | | `get_gui_run` |
| route | `GET /activity?…` (activity, chat) | `ActivityFilter` fields as query | `activity_query` (SQL + `build_where` → `query_activity_log`) |
| route | `GET /harness/eval/history?limit=` (harness-health) | | `harness_eval_history` |
| route | `GET /harness/eval/regressions` (harness-health) | | `harness_eval_regressions` |
| tool | `vox_gamify_notifications_list` (existing) | `{ limit: 20 }` | `gamify::fetch_gamify_alerts` (used by `get_orchestrator_status`/`_bin`); any error, including `exposure_denied` remotely, yields empty alerts |

New ops: `exposure: operator`, `requires: [db]`, `product_lane: workflow`, `side_effect_class: writes_files` (cancel: `destructive`). GUI mutation commands keep calling `emit_tasks_changed(&app_handle)` after the op succeeds.

- [ ] **Step 1: Failing tests.** vox-db: `latest_interaction_tokens` returns 0 for an unknown session and in+out of the newest row; `query_activity_log` honours every filter field (port the existing `build_where` tests). Server: submit → `GET /hopper/list` shows it under inbox → mark_done → shows it as done; with a DB attached, an item submitted through `vox_hopper_submit` is visible on the existing `GET /api/v2/hopper/inbox` (same queue); plan node upsert/list/approve; GUI run start/finish/list/get; activity route; harness eval routes on an empty DB. GUI: call-key tests; `include_str!` checks on the five modules.
- [ ] **Step 2: Run, expect FAIL:** `cargo nextest run -p vox-db latest_interaction_tokens query_activity_log`; `-p vox-orchestrator-mcp gui_ops::orchestrator gui_ops::harness`; `-p vox-gui commands::orchestrator commands::plan_panel commands::runs commands::activity commands::harness_eval`.
- [ ] **Step 3: Implement** store fns, routes, ops, catalog; add `vox_hopper_*`, `vox_plan_node*`, `vox_gui_run_*` to `orchestrator.v1.yaml` `tools.include`; `operations-sync --write`.
- [ ] **Step 4: GUI rewrite** + manifest rows (`orchestrator` → `[dashboard, tasks, chat]`, `plan_panel` → `[chat]`, `runs` → `[runs]`, `activity` → `[activity, chat]`, `harness_eval` → `[harness-health]`; all daemon).
- [ ] **Step 5: Playwright** `tasks-remote.spec.ts` (`installRemoteMode(page, 'tasks', { hopper_list: [{ item_id: 'h1', intent: 'remote intent', priority: 2, state: 'inbox', task_id: 1, session_id: null, agent_id: null, remote_node: null }] }, ['tasks'])`, assert "remote intent", screenshot `tasks-remote.png`) and `harness-health-remote.spec.ts` (view `harness-health`, `harness_eval_history: []`, `harness_eval_regressions: []`, assert the empty-state copy, screenshot `harness-health-remote.png`). Re-run `e2e/tasks-interactions.spec.ts e2e/submit-task-palette.spec.ts`.
- [ ] **Step 6: Verify and commit.** `rg -n 'vox_db|turso|GuiDbPool' crates/vox-gui/src/commands/{orchestrator,plan_panel,runs,activity,harness_eval}.rs` → empty; `ci gui-catalog-parity` → ok; commit `feat(axis): migrate orchestrator/plan/runs/activity/harness-eval db commands`.

**Parallel-safe with:** Tasks 3–9. Task 11 deletes `fetch_gamify_alerts` after this task rewires its callers.

---

### Task 11: Remaining — dev-only gamify (`gamify.rs`, `discovery.rs`)

`vox_gamify_*` is `internal` exposure (spec §3): served only on loopback under `dev`. After this task Axis **local** mode behaves as today; a remote server never serves these tools, so Gamify shows "not in profile" remotely. That is intended, not a gap.

**Files:** `gamify.rs`, `discovery.rs`, new `gamify_settings.rs`; `gui_ops/gamify.rs`; gamify anchors; manifest; guard test.

**Interfaces:** existing `vox_gamify_notifications_list` (#81), `vox_gamify_notification_ack` (#82), `vox_gamify_progress_snapshot` → `data.kpi` (#84), `vox_gamify_quest_list` (#88). New ops (all `exposure: internal`, `requires: [db]`, `product_lane: workflow`):

| Op | Params | Moved from | side_effect_class |
|---|---|---|---|
| `vox_gamify_profile` | `{}` | `get_ludus_profile_impl` | `read_only` |
| `vox_gamify_due_actions` | `{ limit: u32 }` | `gamify_due_actions` | `read_only` |
| `vox_gamify_record_event` | `{ event: serde_json::Value }` | `record_gui_event` | `writes_files` |
| `vox_gamify_leaderboard` | `{ limit: Option<u32> }` | `list_gamify_leaderboard` | `read_only` |
| `vox_gamify_companions` | `{}` | `list_gamify_companions` (SVG generated server-side) | `read_only` |
| `vox_gamify_discovery_record` | `{ action_id, used: bool, now_ms: i64, dwell_ms: i64 }` | `discovery_record` | `writes_files` |

Reads here are MCP ops rather than `/api/v2` routes because `/api/v2` has no `internal` class; an MCP op inherits the loopback-only rule from `authorize_tool`. The new ops use `state.db` (the DB the existing gamify tools use) instead of the GUI's `DbConfig::resolve_for_mesh()`; Step 1 pins that an event written by `vox_gamify_record_event` is reflected by `vox_gamify_profile`.

- [ ] **Step 1: Failing tests.** Server: each op on the memory DB; `authorize_tool(&ActiveProfile::dev(), &BootReport-all-up, CallContext { role: CallerRole::Write, loopback: false }, "vox_gamify_profile")` → `Err(ProfileDenial::ExposureDenied)`. GUI: call-key tests; `gamify_kpi_summary_impl` extracts `data.kpi`; `include_str!` checks on `gamify.rs`, `discovery.rs`; `rg -n fetch_gamify_alerts crates/vox-gui/src` empty.
- [ ] **Step 2: Run, expect FAIL.**
- [ ] **Step 3: Implement**; `get_gamify_settings`/`set_gamify_settings` move verbatim to `gamify_settings.rs`. Manifest rows `gamify / daemon / [gamify]`, `discovery / daemon / [console]`, `gamify_settings / host / [gamify, settings]`. `operations-sync --write`.
- [ ] **Step 4: Mutation-verify** the internal-exposure denial: in the `vox_gamify_profile` catalog block change `exposure: internal` → `public`, `operations-sync --write`; the Step 1 denial test FAILs; restore, regenerate, PASS.
- [ ] **Step 5:** Re-run `cargo nextest run -p vox-gui remote_gate_fixture_matches_real_gate console_stays_needs_local_machine` and `e2e/console-remote.spec.ts` (`discovery` flipping to `daemon` leaves console `needs_local_machine`, because `pty`/`terminal_core` stay `host`; regenerate the fixture only if the test reports drift); re-run `e2e/console-workbench.spec.ts` and the gamify specs (`rg -l gamify crates/vox-gui/ui/e2e`).
- [ ] **Step 6: Verify and commit.** `rg -n 'vox_db' crates/vox-gui/src/commands/{gamify,discovery}.rs` → empty (gamify tests that used `VoxDb::connect(Memory)` move to `gui_ops/gamify.rs`); `ci gui-catalog-parity` → ok; commit `feat(axis): route dev-only gamify commands through internal MCP ops`.

---

### Task 12: Remote performance follow-ups

**Files:** `crates/vox-gui/ui/src/hooks/useSearchController.ts` (+ test).

- [ ] **Step 1: Failing vitest** in `useSearchController.test.ts`: when plan 06's `axisBackendStatus()` (from `transport.ts`) resolves `{ mode: 'remote', … }`, three keystrokes 100 ms apart yield exactly one `voxSearchQuery` call 350 ms after the last; in local mode the 200 ms default holds.
- [ ] **Step 2: Implement** `const REMOTE_DEBOUNCE_MS = 350;` and choose `debounceMs` from `(await axisBackendStatus()).mode` (read once at hook mount; re-read when plan 06's `BackendStatusProvider` context changes) when the caller passed none.
- [ ] **Step 3: Scientia topic push — deferred (not in this plan).** Plan 06 exposes no per-topic subscription (`GatewayClient::subscribe_events` relays agent events only) and forbids new `vox://` channels without `contracts/channels.v1.yaml` parity. `ScientiaDashboard` keeps its 10 s poll in remote mode (one small GET). Record "scientia queue push: follow-up" in the PR description.
- [ ] **Step 4: Run** `pnpm --dir crates/vox-gui/ui test`, `pnpm --dir crates/vox-gui/ui typecheck`; commit `perf(axis): remote-mode search debounce`.

**Parallel-safe with:** none of Tasks 6–7 (same surfaces) — run after them.

---

### Task 13: Lock it in — forbid `db` class, delete `GuiDbPool`, drop `vox-db`/`turso` from `vox-gui`

**Files:** `crates/vox-cli/src/commands/ci/gui_command_backends.rs` (plan 06 Task 2; run by `vox ci gui-catalog-parity`); `crates/vox-gui/src/commands/{gui_db_pool.rs (delete), mod.rs, research_host.rs}`; `crates/vox-gui/src/main.rs`; `crates/vox-gui/Cargo.toml`; `contracts/ci/crate-edges.allow.v1.json` (tightened by the tool).

- [ ] **Step 1: Confirm nothing is left.**

```bash
rg -n 'class: db' contracts/gui/command-backends.v1.yaml          # expect no output
rg -ln 'vox_db|connect_workspace|turso' crates/vox-gui/src         # expect: commands/gui_db_pool.rs, commands/research_host.rs only
```

- [ ] **Step 2: Failing CI-check tests** in the test module of `crates/vox-cli/src/commands/ci/gui_command_backends.rs`:

```rust
    #[test]
    fn db_class_is_rejected() {
        let manifest: serde_yaml::Value = serde_yaml::from_str(
            "schema_version: 1\ncommands:\n- { module: models, class: db, surfaces: [models] }\n",
        )
        .expect("yaml");
        let modules: std::collections::BTreeSet<String> = ["models".to_string()].into();
        let views: std::collections::BTreeSet<String> = ["models".to_string()].into();
        let probs = problems(&modules, &manifest, &views);
        assert!(probs.iter().any(|p| p.contains("class `db` is retired")), "{probs:?}");
    }

    #[test]
    fn direct_db_import_in_command_module_is_rejected() {
        let err = check_no_db_imports(&[("models.rs", "use vox_db::VoxDb;")]).unwrap_err();
        assert!(err.contains("models.rs"), "{err}");
        assert!(check_no_db_imports(&[("models.rs", "use crate::remote::calls::BackendCalls;")]).is_ok());
    }
```

- [ ] **Step 3: Run, expect FAIL:** `cargo nextest run -p vox-cli gui_command_backends`.
- [ ] **Step 4: Implement.** In `problems`, for each row whose class is `db`, push `` format!("{module}: class `db` is retired (plan 07); serve it through the daemon") `` (keep `db` in plan 06's `CLASSES` so the message is this one, not "unknown class"); update plan 06's own fixtures that use `class: db` as a valid row (e.g. the `gone` stale-row test) to `class: host`. Add:

```rust
/// Plan 07 regression guard: Axis command modules reach data only through the daemon.
pub fn check_no_db_imports(files: &[(&str, &str)]) -> Result<(), String> {
    let bad: Vec<&str> = files
        .iter()
        .filter(|(_, src)| src.contains("vox_db::") || src.contains("use vox_db") || src.contains("turso::"))
        .map(|(name, _)| *name)
        .collect();
    if bad.is_empty() {
        Ok(())
    } else {
        Err(format!("direct vox_db/turso use in crates/vox-gui/src/commands: {}", bad.join(", ")))
    }
}
```

and call it from the check's `run` over every `crates/vox-gui/src/commands/*.rs` (paths built with `Path::join`, read with `std::fs::read_to_string`).

- [ ] **Step 5: Remove the last uses.** Delete `commands/gui_db_pool.rs`, its `pub mod` line, and the `commands::gui_db_pool::GuiDbPool::connect_workspace()` `manage(..)` in `main.rs`. In `research_host.rs` replace `use vox_db::research_doc_io::atomic_write_secure;` with a local copy of that ~40-line fn under `// vox:defactored-from vox-db 2026-10-08` (AGENTS.md defactor rule: < 50 lines). Remove `vox-db = { workspace = true }` and `turso = { workspace = true }` from `crates/vox-gui/Cargo.toml` once `rg -n 'vox_db|turso' crates/vox-gui/src crates/vox-gui/build.rs` is empty (test modules included).
- [ ] **Step 6: Tighten edges and run everything.**

```bash
cargo run -p vox-cli -- ci crate-edges --tighten     # drops vox-gui -> vox-db from the allow-list
cargo run -p vox-cli -- ci gui-catalog-parity      # ok
cargo run -q -p vox-arch-check
cargo check -p vox-gui
cargo clippy -p vox-gui -p vox-orchestrator-mcp -p vox-cli --all-targets -- -D warnings
cargo nextest run -p vox-gui -p vox-orchestrator-mcp
cargo nextest run -p vox-cli gui_command_backends
pnpm --dir crates/vox-gui/ui typecheck && pnpm --dir crates/vox-gui/ui test && pnpm --dir crates/vox-gui/ui test:e2e
```

Expected: all green; `git diff contracts/ci/crate-edges.allow.v1.json` shows only removed `vox-gui → vox-db` entries.

- [ ] **Step 7: Mutation-verify the regression guard.** Add `use vox_db::VoxDb;` to `crates/vox-gui/src/commands/models.rs`; `grep -c 'use vox_db::VoxDb' crates/vox-gui/src/commands/models.rs` → `1`; `cargo run -p vox-cli -- ci gui-catalog-parity` exits non-zero naming `models.rs`. Remove; grep → `0`; check passes.
- [ ] **Step 8: Commit** `chore(axis): forbid db-class commands and drop vox-gui's vox-db edge`.

---

## Coordination for parallel groups

Tasks 3–11 may run in parallel subagents, each in its own worktree branched from the merge of Tasks 0–2, because each owns disjoint `crates/vox-gui/src/commands/*.rs` modules and disjoint `gui_ops/<group>.rs` files. Shared files are handled so no task moves anything another task edits:

- `dashboard_api.rs`: edited only in Task 1 (all group routers merged up front).
- `dispatch.rs`, `input_schemas.rs`, `catalog.v1.yaml`: each group inserts **only** under its own `── gui_ops: <group> ──` anchor. Those regions are append-only, so union-resolving a conflict is safe; afterwards run `rg -c '"vox_<new op>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs` for each op (expect `1`).
- `contracts/mcp/tool-registry.canonical.yaml` and other generated outputs: never hand-merged — take either side, then re-run `cargo run -p vox-cli -- ci operations-sync --target mcp --write`.
- `contracts/gui/command-backends.v1.yaml`, `crates/vox-gui/src/commands/mod.rs`, `main.rs` `generate_handler!`, `contracts/profiles/*.v1.yaml` include lists, `commandBackendsMigration.test.ts`, `remote/calls.rs`: per-row / per-line appends; union-resolve, then check each module appears once (`rg -c '^- module: <m>$' contracts/gui/command-backends.v1.yaml` → `1`).
- `gui_ops/harness.rs`: Task 7 creates the issue/proposal routes, Task 10 appends the eval routes — merge 7 before 10 or union-resolve the `.route(..)` lines.
- Merge order into the feature branch follows the profile order (3, 4, 5, 6, 7, 8, 9, 10, 11), then 12, then 13. After every merge: `cargo check -p vox-gui -p vox-orchestrator-mcp` and `cargo run -p vox-cli -- ci gui-catalog-parity`.

---

## Self-Review

- **Spec coverage.** Spec §7 "Migration order … models → knowledge → research → mesh → attention → skills → orchestrator, one small PR each" → Tasks 3–4 (models), 6 (knowledge), 7 (research), 8 (mesh), 9 (attention + skills = 0), 10 (orchestrator), 11 (remaining); each ends in its own commit. Spec §9.4 Playwright in remote mode → one remote spec per group with screenshots. Spec §3 exposure → every new op carries `exposure`/`side_effect_class`/`requires`; mutations are `operator` (gamify `internal`).
- **Inventory completeness.** All 20 modules from `rg -l 'vox_db|connect_workspace|turso' crates/vox-gui/src/commands` appear: chat, harness_eval, orchestrator, knowledge, console_a2a, search, scientia, models, runs, plan_panel, activity, memory, discovery, gamify, search_probe, scientia_review, user_config, research, preferences, harness_issues (+ `gui_db_pool` deleted in Task 13). Indirect use via `gamify::fetch_gamify_alerts` from `orchestrator.rs` status is row #59.
- **Target rule applied consistently.** Existing tools/routes are used where store call and shape match (#18 turn, #34, #35, #51, #59, #81, #82, #84, #88). Rejected near-matches are named with the reason. Two deliberate departures from "existing tool first", both justified in the Eq column: `vox_preference_get` (its `requires: [memory]` would boot the search index in every Axis profile) and `vox_preference_set` (also mutates live gate config). Their replacements are narrow, key-restricted, and mutation-verified. No new mutating route is added; the existing ungated `POST /api/v2/hopper/submit` is not used by Axis.
- **Local path parity.** Every route is reachable locally through `orch.api_v2_get` (same router, in-process) and every op through `orch.tool_call`, so there is one handler per payload; the bridge is GET-only and path-validated, both mutation-verified.
- **Data storage policy.** Raw SQL in `activity.rs`, `memory.rs`, `orchestrator.rs` becomes named `vox-db` store fns; harness row types stop crossing the wire; DTOs live under `gui_ops/dto/`.
- **Placeholder scan.** Every name inherited from plans 01–06 is fixed in Task 0 (taken from those plans' final text) with a verification command; nothing is substituted at execution time. Group tasks give exact routes, params, ops and commands.
- **Type consistency.** `BackendCalls`, `tool_data`, `api_data`, `path_with_query`, `GUI_USER_ID`, `FakeBackend` are defined once (Task 1) and used with the same signatures everywhere; DTOs are shared types, not re-declared in `vox-gui`.

## Risks

- **D1 (decided).** `models` requires `db`, so Models/Chat persist against a `models` server; `attention` and `skills` stay DB-free, so the attention spend route returns zeros with the server's caps there; `gui.*` preferences are client-side in remote mode, so preferences set on one Axis machine do not follow the user to another machine connected to the same server. That last point is a deliberate trade-off of D1, not a bug.
- **Plans 02/06 interface fidelity.** Task 0 pins ~12 names from plans 02 and 06 (`BackendState::current`, `GatewayClient::{call_daemon_method, api_get}`, `axis_backend_status`, plan 06's `problems` validator, `ServerState::with_profile`). If either plan changes a signature before this one runs, update Task 0 and the affected task in the same commit.
- **Remote/local envelope parity.** The Remote arm of `BackendCalls::call_tool` assumes `/v1/ws` `call_tool` `data` equals the local `orch.tool_call` result. Task 2 Step 0 pins this with a test and is a hard STOP if it fails.
- **Behaviour changes in `dev`.** (a) The gateway's `/api/v2/hopper/*` switch from `InMemoryHopper` to `SqliteHopper` when a DB is attached (Task 10). (b) `set_routing_priority` stops mutating the GUI process env (`VOX_AUTO_ROUTING_PRIORITY`); the daemon reads the preference, as it already does for `selection_policy`. (c) `list_model_cards` reflects the daemon's registry plus its Mens scan instead of the GUI's cached registry. (d) `set_active_model` validates against the live orchestrator registry instead of `ModelRegistry::from_cache()`. All four go in the PR descriptions; none removes data.
- **Read-token clients lose writes.** All new mutations are `operator`, so a read-token Axis cannot set the active model; calls fail with `exposure_denied` and the existing error toasts show it. (GUI preferences are unaffected: client-side when remote, per D1.)
- **`vox_a2a_send` sender binding and DB choice.** Console operator id `0` may be rejected by `resolve_sender_binding` (Task 8 Step 1 pins and fixes it). The GUI previously wrote through `DbConfig::resolve_for_mesh()`, which can differ from the daemon's DB; afterwards console notes land in the daemon's DB, which its A2A pollers read.
- **`news-publish` on the daemon.** #38/#43 need `vox-publisher`; `vox-orchestrator-d` already enables `news-publish` on `vox-orchestrator-mcp` (verified 2026-10-08), and Task 0 re-checks it.
- **Per-request `GatewayState` in the in-proc bridge.** Cheap at Axis poll rates; if profiling disagrees, cache one in `McpExtraDispatch` behind a `tokio::sync::OnceCell`.
- **Size.** 80 commands need new server surface (38 routes, 38 ops). Each group task is independently mergeable and leaves the tree green, so the migration can stop after any group with the remaining modules still `db`-class and local-only, exactly as plan 06 left them.
