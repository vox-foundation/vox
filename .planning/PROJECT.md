# Vox

## What This Is

Vox is a mature, actively-developed cross-platform Rust workspace (138+ crates) that implements its own programming language — the `.vox` frontend, compiler, and interpreter — plus the surfaces built on top of it: `vox-cli` (the `vox` binary), `vox-gui` (a Tauri 2 desktop application), a multi-agent orchestrator with MCP tool exposure, a local-first mesh (iroh QUIC) for distributed agent/model work, and a native-Rust ML training stack (Candle/QLoRA) for its own fine-tuned model ("Mens"). It is developed and stewarded as a FOSS project under an in-formation Vox Foundation, not a monetized product.

- **v1.0 (Architectural Housekeeping, shipped 2026-10-01):** Disposed of 20 audited dead/misplaced crates and closed the open architecture decisions a full-corpus ADR/SPEC ingest surfaced.
- **v1.3 (Self-Maintaining Public Docs, shipped 2026-10-13):** Shipped the deterministic drift gate, automated crate API reference generator, collapsible reader layout, and trust signal foundation.
- **v1.5 (Unified Holistic Stream, shipped 2026-10-16):** Converged all six parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and 42 feature worktrees across Waves 0–6 (74 phases, 241 plans) into `/Users/brbrainerd/dev/vox`.
- **v1.6 (Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel, shipped 2026-10-16):** Completed Phases 77–88 (12 phases, 25 plans) delivering 5-spoke training hygiene + round-trip verification (`CORPUS`, `SPOKE`), supervised `vox-orchestrator-d` auto-bootstrap + gateway/IPC parity (`CORE`), real-backend E2E verification (`REAL`), `.vox` end-to-end execution parity (`VOX`), operational & data-intensive GUI surface hardening (`OPS`, `DATA`), `vox doctor --fix` + workspace CI gate parity (`GATE`), and the Mens continuous QLoRA/DPO flywheel (`TRAIN`, `EVAL`, `AUDIT`, `LOOP`).
- **v1.7 (Centralized Postgres Telemetry, Private `vox-ops` Extraction, Ludus 3-Way SoC, Zero-Cheat Leaderboards, Codebase/History Scrub, Live Helios Deployment & 5-Spoke MENS Safetensors Evaluation, shipped 2026-10-19):** Completed Phases 89–97 (9 phases, 18 plans, 70/70 requirements) delivering 26-variant strongly-typed telemetry + `VoxDb::audit_research_metric_shapes` (`TELEM`, `TYPING`, `ROUTER`, `DBAUDIT`), three-role fenced PostgreSQL 17 `vox_ops` workspace (`PGFENCE`, `HELIOS`), 3-way Ludus SoC split + `BLAKE3`/`Ed25519` `VerifiedLeaderboardSnapshotV1` on `voxlang.org` (`VoxLeaderboardArena.astro`) & `vox-gui` (`LeaderboardArenaPanel.tsx`) (`SOCIAL`, `LUDUS-SOC`, `LEADERBOARD`), $k \ge 20$ `CommunityIntelligenceBundleV1` + Empirical Bayes routing + moved-data/history scrub (`INTEL`, `SCRUB`), and 25-problem 5-spoke MENS before/after evaluation (`24.0% -> 72.0% -> 100.0% pass@1`) + `.safetensors` adapter verification (`MENS-GAP`, `MENS-PERF`, `MENS-HARNESS`, `MENS-SURFACE`, `LIVE-QLORA`).
- **v1.8 (Shared-Package CV Flywheel, Chat/Loquela Knob & Datatype Unification, VS Code Multi-Panel Docking, Urbs Civilization Live Taps, Browser Songhand/Playwright Harness & `ngi.video` Marketing Copy/Screenshot CV Flywheel Promotion, shipped 2026-10-19):** Completed Phases 98–103 (6 phases, 6 plans, 32/32 requirements) delivering dynamic 32-surface `@ngi/platform-ssot` CV Flywheel verification (`CV-FLYWHEEL`), `Loquela`/`DriveConsole` knob & strong datatype unification (`CHAT-KNOBS`, `CHAT-TYPES`), `DockWorkspaceShell` layout presets + `terminal`/`urbs`/`browser` dock panels (`DOCK-UX`), live `LudusSandbox` Roman SVG citizens + `AQVAE` MCP taps (`URBS-VIS`), Playwright + Songhand `vox_browser_*` harness (`BROWSER-HARNESS`), and CV-verified `ngi.video` portfolio screenshot & copy promotion (`NGI-VOX-COPY`).
- **v1.9 (Chat Default Surfaces CV & Code Audit Remediation, Progressive-Disclosure Composer, Rich Markdown Transcript, Full-Height Execution Rail & CV Flywheel Convergence, shipped 2026-10-19):** Completed Phases 104–107 (4 phases, 4 plans, 14/14 requirements) remediating all 11 visual/CV (`CV-01..05`) and code/architectural (`CODE-01..06`) defects across `chat-default`, `loquela-knobs`, and `chat-dock-multi` (`+37.2%` to `+48.7%` luminance contrast gain, `visualAfterScore: 0.9184–0.9195`, `166/166` Vitest tests passing).
- **v2.0 (Persistent Organic Multi-Spoke Search-to-Compilation Benchmark, Live Execution Oracles & Longitudinal Evaluation Flywheel, shipped 2026-10-19):** Completed Phases 108–111 (4 phases, 4 plans, 16/16 requirements) delivering the 15-task organic search-to-compilation suite (`canonical_organic_search_to_compilation_suite_15`, `100.0%` Recall@5, `1.000` MRR, `100.0%` Grounding, `100.0%` Compilation, `100.0%` E2E `pass@1`), deep compilation/runtime oracles across all 5 spokes (`verify_deep_compilation_for_spoke`), `VoxDb` longitudinal history + `ModelBenchCase` telemetry (`record_organic_scorecard_telemetry_to_db`) + multi-metric regression ratchets (`verify_organic_search_ratchet`), and `vox model eval --organic-search-spokes` + Axis GUI `MensTrainingView.tsx` (`mens-organic-scorecard`).
- **v2.1 (Editable Chat Task/Context Controls, 32-Surface Mechanical & WCAG 2.2 AA Hardening, Self-Healing Pre-Push Automation Chain & Universal CV Flywheel Convergence, shipped 2026-10-20):** Completed Phases 112–115 (4 phases, 4 plans, 16/16 requirements) delivering inline editable session task lists (`ChatExecutionRail.tsx`, `PlanPanel.tsx`), bidirectional session context pinning & inline citation/chip editing (`ChatSurface.tsx` `@kb` Context Inspector, `Loquela.tsx`), interactive token budget & compaction strategy controls (`ContextWindowMeter.tsx`), high-signal mechanical & WCAG 2.2 AA hardening across all 32 GUI surfaces (`Mesh`, `Runs`, `Sub-Agents`, `Scientia`, `Repository`, `Policies`, `Approvals`, `Catalog`, `CodeRabbit`), `VOX_AUTO_HEAL` self-healing pre-push & SSOT automation chain (`pre_push.rs`, `docs.rs`, `lefthook.yml`), and universal 32-surface `@ngi/platform-ssot` CV Flywheel convergence (`0.9142–0.9254`, `0` axe/overflow/clipping defects, `178/178` unit/guard tests).
- **v2.2 (GitHub CI Green Sweep, Persistent Chat Task/Context Backend RPCs, Zero-Finding Drift Audit & Full-Stack Closure, shipped 2026-10-20):** Completed Phases 116–119 (4 phases, 4 plans, 16/16 requirements) remediating all GitHub CI red/timed-out gates (`baseline-version-policy.yaml`, `distribution-parity`, `Build Starlight`, `Internal Markdown link check`, and `vox-langtool` in `cr-l8-corpus-feedback.yml`), shipping daemon-persisted Chat task rename (`hopper_rename`), compaction strategy (`set_context_budget_strategy`), and session context pins (`get_session_context_pins`, `set_session_context_pins`) in `vox-gui`, eliminating all 23 `vox-drift-check` warning findings (`23 -> 0`), and verifying `vox ci pre-push` (`all checks passed in 24795ms` with `0` warnings).
- **v2.3 (Full-Matrix Chat Knob Verification, OpenRouter vs. Local-to-MENS Routing Remediation, Strategy-Aware Context Compaction & Before/After Flywheel Data Audit, shipped 2026-10-20):** Completed Phases 120–123 (4 phases, 4 plans, 16/16 requirements) wiring all chat modifying knobs end-to-end across `buildChatTurn.ts`, `vox-gui` (`sync | background | plan` tracks), and `vox-orchestrator-mcp` (`432`/`144`-cell matrices, `56.25% -> 100.0%` knob fidelity), remediating all 3 Local-to-MENS vs. OpenRouter routing regressions across `4` clutches $\times$ `3` risk postures $\times$ `5` spokes (`0/60 -> 60/60`), upgrading `load_conversation` to strategy-aware `cl100k_base` BPE token budgeting (`2,400 / 4,000 / 8,000` tokens with `20%` turn-0 goal anchor preservation and `KbScopeMask` retrieval leg filtering), and publishing `contracts/reports/full-flywheel-knob-matrix-before-after.v1.json` + `docs/src/architecture/full-flywheel-knob-matrix-audit-2026.md`.
- **v2.4 (Real-Time Persistent Session Posture, Keybound Semantic Knobs, Turn-Level Compaction/Posture Receipts & CV Flywheel Visual Audit, shipped 2026-10-20):** Completed Phases 124–127 (4 phases, 4 plans, 16/16 requirements) adding `get_session_chat_posture` / `set_session_chat_posture` Tauri RPCs and cross-track server inheritance (`vox_chat_message`, `vox_plan`, `vox_submit_task`), upgrading `DriveConsole.tsx` and `Loquela.tsx` with semantic capsule coloration and 11 real-time shortcuts (`Alt+1..4 / Alt+C / Alt+R / Alt+T / Alt+M / Alt+V / Alt+O / Alt+K`) plus `loquela-live-posture-badge`, emitting `compaction_audit` and `posture_audit` on every chat turn, and achieving `0.9222` (`loquela-knobs`) and `0.9203` (`chat-default`) CV Flywheel convergence across all 32 GUI surfaces.
- **v2.5 (Live Turn Receipt & Compaction Telemetry Inspector in Chat, Real-Time Keybind HUD Overlay, Continuous 5-Spoke MENS Adapter Auto-Eval Gate & CV Flywheel v2.5 Verification, shipped 2026-10-24):** Completed Phases 128–131 (4 phases, 4 plans, 16/16 requirements) adding `CompactionAuditDto`/`PostureAuditDto` and `chat-turn-receipt-badge` (`ChatExecutionRail.tsx`), adding the interactive `Keys Alt+/` Keybind HUD (`loquela-keybind-hud-toggle` / `loquela-keybind-hud-panel` in `Loquela.tsx`), wiring `ModelRegistry::verify_full_flywheel_routing_and_spoke_ratchet` (`60/60` cells + `25/25` 5-spoke `pass@1 = 1.0`), and verifying all 32 GUI surfaces via the CV Flywheel and `vox ci pre-push`.
- **v2.6 (Everyday Human Conversational Intent Routing, Multi-Turn Context Carryover, Automatic Self-Repair Escalation & 40-Case Decoupled 8-Spoke Evaluation, shipped 2026-10-24):** Completed Phases 133–136 (4 phases, 4 plans, 16/16 requirements) upgrading `ModelRegistry::score_conversational_turn_across_spokes` and `route_conversational_turn_with_context` to route natural-language human utterances across all 8 spokes without internal struct jargon, adding multi-turn `prior_spoke` anaphora/context carryover and automatic `SelectionIntent::repair_loop()` escalation on conversational repair turns (`execute_extended_human_conversation_session`), fixing parallel `rustc` temp-file races in `verify_rust_snippet_with_rustc`, decoupling all 24 benchmark queries from `spoke_roundtrip.rs`, and expanding per-spoke sample depth to $n = 5$ (`canonical_organic_search_to_compilation_suite_40`).
- **v2.7 (SOTA Agent Harness: Spoke-Ranked Tool Selection, Sub-50ms Multi-Language Inline Verification, Speculative Local-to-Repair Cascade, Parallel Read-Tool Execution, KV-Stable Artifact Memory & Live Auto-DPO Flywheel, shipped 2026-10-24):** Completed Phases 137–141 (5 phases, 5 plans, 20/20 requirements) replacing blind alphabetical 40-tool truncation with spoke-conditioned BM25 + lexical tool ranking (`select_tools_for_turn_ranked`, `100%` recall on late-alphabet tools across all 8 spokes at `50%` lower schema token budget), upgrading `post_verification::verify_and_attach` with inline `.vox` Grammar Unification enforcement, `.rs` `syn::parse_file` + policy + `rustc --emit=metadata` borrow/type checking, and `SKILL.md` frontmatter + `.vox` doctest compilation, adding synchronous `[HARNESS REPAIR INTERVENTION]` + dynamic `temperature = 0.1` repair escalation + `futures_util::future::join_all` parallel read-only tool batches + live intra-turn Auto-DPO harvesting in `run_agent_turn`, implementing `SessionWorkingMemory` + SWE-agent-style `blake3` observation masking in `CompactionEngine`, and publishing `SotaHarnessScorecardV1` (`contracts/reports/sota-harness-before-after.v1.json`).
- **v2.8 (Oct 2026 SOTA Live-Path Closure: Single-Pass SSE `delta.tool_calls` Streaming, KV-Cache-Aligned Multi-Turn Wire Assembly, Live `SessionWorkingMemory` Compaction, AST Symbol-Skeleton Context Packing & Real Intra-Turn Repair Model Cascade, shipped 2026-10-24):** Completed Phases 142–146 (5 phases, 5 plans, 20/20 requirements) upgrading `vox_openai::sse` + `vox_llm_egress::wire::stream_events_once` + `stream_final_answer` to parse SSE `delta.tool_calls` in a single HTTP pass (eliminating the 2x duplicate HTTP POST penalty on tool-calling turns), mapping all 8 provider families (`Groq`, `Cerebras`, `DeepSeek`, `SambaNova`, `Mistral`, `PopuliMesh`, `HuggingFaceRouter`, `OpenRouter`/`Ollama`/`GoogleDirect`/`Anthropic`/`VoxLocal`) and `cognitive_profile` into `try_run_agent_turn`, wiring structured `prior_conversation` messages + `DEFAULT_RANKED_MAX_TOOLS = 20` + raw-utterance tool ranking on the live `message.rs` -> `agent_loop.rs` path, wiring `SessionWorkingMemory` + `blake3` observation masking into `conversation.rs` (`bound_messages_by_strategy`), adding sub-10ms AST symbol-skeleton extraction (`extract_compact_symbol_skeleton` + `extract_symbol_span`, `~72%` token reduction on files `> 120` lines), and adding real intra-turn `SelectionIntent::repair_loop()` model escalation + persistent `VoxDb` live Auto-DPO harvesting (`live_harness_auto_repair_dpo`).

- **v2.9 (Oct 2026 SOTA Zero-Overhead Turn Pipeline: Day-Stable KV-Cache System Prompt, Wire-Level Usage & Cache-Read Accounting, Intent-Gated Retrieval Preamble, Intra-Turn Tool Output Budgeting & Two-Tier Progressive Tool Schemas, shipped 2026-10-24):** Completed Phases 147–151 (5 phases, 5 plans, 20/20 requirements) making `system_prompt` 100% invariant within a calendar day (moving `session_ts` to turn-local `context_parts`, deduplicating `ANTI_LAZINESS_RIDER`, caching `VOX.md`/`MEMORY.md` by `(path, mtime, len)`, and capping the tier-1 skill catalog to 24), parsing final-chunk SSE `usage` (`prompt_tokens`, `completion_tokens`, `cache_read_tokens`, `cost_usd`) in `vox-openai` and threading it through `AgentTurnOutcome`/`AgentTurnResult` to `VoxDb` `LlmSurfaceTelemetry`, adding `should_run_auto_chat_preamble` to bypass 5-leg retrieval on phatic greetings and self-contained `@mention`/`selected_text` edits, adding intra-turn tool output capping (`cap_and_summarize_tool_output`), stale tool observation masking (`mask_stale_intraturn_tool_messages`), and persisted `[TURN TOOL LEDGER: ...]` history summaries, and implementing two-tier progressive tool schema compression (`compact_tool_def_for_rank`, `>= 38%` schema byte reduction on secondary tools `rank >= 8`).
- **v3.0 (Oct 2026 SOTA Wire-Level Prompt-Cache Breakpoints, `stream_options.include_usage`, Cache- & TTFT/TPOT-Aware Model Router, Intra-Turn Read-Tool Deduplication & Workspace-Aware Rust Structural Verification, shipped 2026-10-24):** Completed Phases 152–156 (5 phases, 5 plans, 20/20 requirements) adding `stream_options: {"include_usage": true}` and Anthropic `cache_control: {"type": "ephemeral"}` breakpoints on `system` + penultimate history turn in `vox-llm-egress::wire`, closing the end-to-end telemetry loop (`prompt_tokens`, `completion_tokens`, `ttft_ms`, `tpot_ms`, spoke-derived `TaskCategory`, and `compute_turn_cost_usd` discounting `cache_read_tokens` at `cache_read_cost_per_1k`), upgrading `ModelSelectionEngine` (`efficiency_score_with_cache` + `scoreboard_feedback_boost` using `p95_ttft_ms`, `p95_tpot_ms`, and `goodput_tokens_per_sec`), adding zero-latency intra-turn read-only tool memoization with write invalidation (`read_tool_memo`), and adding sub-5ms workspace-aware `.rs` structural/module-graph verification (`verify_rust_module_structural_integrity`).
- **v3.1 (Oct 2026 SOTA Holistic Closure: 8-Spoke Adversarial & Real-Query Hardening, Rolling Intra-Turn Prompt-Cache Breakpoints, Live Context-Fill & Spoke-Conditioned Router Wiring, Path-Scoped Read-Memo Invalidation + Edit-Window Recovery & 48-Case 8-Spoke Benchmark, shipped 2026-10-24):** Completed Phases 157–162 (6 phases, 6 plans, 24/24 requirements) hardening all 8 spoke verifiers and natural-language router signals against real-world/adversarial developer queries (`strip_code_fences` prose extraction, mid-stream ASR disfluency + spoken-punctuation normalization in `normalize_asr_transcript_for_vox`, OpenAI `{"type":"function"}` MCP unwrapping, `has_top_level_shell_chaining`, workspace-crate `.rs` fallback, and multi-block `SKILL.md` verification), adding rolling 3-breakpoint intra-turn tool-loop + tool-schema prefix `cache_control` in `vox-llm-egress::wire`, wiring live `context_fill_ratio`, `cacheable_workload`, and spoke-classified `(TaskCategory, u8)` through `SelectionIntent` / `resolve.rs` / `message.rs`, upgrading `read_tool_memo` in `agent_loop.rs` with canonical JSON keys (`canonicalize_tool_args_json`), path-scoped write invalidation (`invalidate_read_memo_for_write`), and automatic `[TARGET FILE CONTEXT WINDOW]` edit-failure recovery, adding spoke-aware `SpeechToCode`/inline-code preamble skipping + 750ms bounded retrieval timeout in `message.rs`, and verifying `100.0%` `pass@1`, compilation, and spoke routing across `canonical_adversarial_and_real_query_spoke_suite_48`.
- **v3.2 (Oct 2026 SOTA Complete Harness, 8-Spoke Mastery, TOESTUB Precision Hygiene & Bi-Directional Axis GUI Control Surface, shipped 2026-10-24):** Completed Phases 163–171 (9 phases, 9 plans, 36/36 requirements) promoting `vox_apply_structured_edit` into `spoke_priority_tools`/`CORE_DAILY_DRIVER_TOOLS` + `+120` mutation-verb boost into `rank < 8` and extending `build_edit_failure_recovery_window` to `"vox_apply_structured_edit"` / `"target_content"`, preserving byte-identical prior tool messages on prefix-cached providers (`provider_uses_prefix_kv_cache`) up to `24,000` tokens (`kv_prefix_preserved_iterations`), upgrading `salvage_tool_calls_from_text` to extract single/multi-call arrays and `<tool_call>` XML across all 5 dialects (OpenAI `function` with stringified JSON args, Anthropic `tool_use`, MENS `tool_name`, MCP JSON-RPC `tools/call`, and wire aliases), replacing line-based `brace_depth` scanning with `verify_rust_ast_structural_integrity` (`syn::File` AST walker), hardening all 8 spokes on unseen real queries (`TaskPlanEnvelope` & `PopuliMeshDispatchEnvelope` aliases, multi-block code fences, case-insensitive ASR fillers + spoken operators, escaped-quote shell pipelines), adding `should_deescalate_repair_cascade` + `looks_like_streamed_tool_call_envelope` + cumulative `latency_ms`/first-iteration `ttft_ms`, verifying `100.0%` across all 64 cases in `canonical_adversarial_and_real_query_spoke_suite_64`, shipping the TOESTUB precision engine (`Phase 170`), and delivering the bi-directional CAS task ledger (`taskLedgerSync.ts`), polygon-free SVG telemetry graphics (`Matrix.tsx`, `ContextWindowMeter.tsx`, `ChatExecutionRail.tsx`, `PlanPanel.tsx`), plain-English single-line buttons, and zero-occlusion Playwright CV Flywheel convergence (`0.9173–0.9202`, `Phase 171`).

## Core Value

The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`), while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## Current Milestone: v5.4 — T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening (`Phases 301–306`)

**Goal:** Close the remaining t3code/t3.chat extraction gap by porting change-gated CI, transfer-budget reporting, knip/PR-size/evidence hygiene, agent-ops kill-safety + multi-surface checklists, and TOESTUB/oxlint-parity detectors into Vox's harness gates — without redoing v3.9/v4.5–v5.1 runtime extractions.

**Target features:**
- Change-gated local/CI checks (`check:changed` + INERT/EXTRA_OWNERS path map) matching t3code's affected-package matrix
- Transfer-budget CI artifacts + trusted-branch PR comments; weighted test sharding
- Knip unused-export dual gate for `vox-gui/ui`; PR size labels (non-test LOC); ban committed PR evidence/screenshots
- AGENTS.md kill-safety triad + hit-every-surface checklist; worktree setup markers + isolated home
- TOESTUB: thin-transport / no-rpc-permission-bypass beyond MCP; suppression PRs require human review; wire snapshot ID interning
- Tauri native fingerprint advisory; Vite+/pnpm layered GUI CI cache; refresh `t3-ecosystem-surface-graph.v1.json` + extraction scorecard

**Evidence / graph:** `~/dev/t3code` (primary), `contracts/reports/t3-ecosystem-surface-graph.v1.json`, store briefs `internal/t3-milestone-gems-brief-2026-10-25.md` + `internal/t3code-ci-lint-gems-audit-2026-10-25.md`

**Planned Phases in `v5.4` (`Phases 301–306`, `24` requirements):**
- **Change-Gated CI Path Map & `vox ci check-changed` (`Phase 301` — `T3-CI-CHANGED-01..04`)**
- **Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding (`Phase 302` — `T3-CI-BUDGET-01..04`)**
- **Knip Dual Gate, PR Size Labels & Evidence-Asset Ban (`Phase 303` — `T3-CI-HYGIENE-01..04`)**
- **Agent-Ops Kill-Safety, Hit-Every-Surface Checklist & Worktree Isolation (`Phase 304` — `T3-AGENT-OPS-01..04`)**
- **TOESTUB Thin-Transport, Suppression Review Gate & Snapshot Interning (`Phase 305` — `T3-TOESTUB-ENF-01..04`)**
- **Tauri Fingerprint Advisory, GUI Cache Layers, Surface Graph Refresh & Scorecard (`Phase 306` — `T3-CI-CLOSE-01..04`)**

## Completed Milestone: v5.3 — Photorealistic Edge-Softening, Continuous Monumental Aqueduct-to-Sewer Architecture, Sub-Surface Ground Integration & CV Flywheel Score Maximization (`Phases 295–300`, `24/24` requirements — Complete 2026-10-25)

**Goal:** Eliminate every remaining sharp polygon cutout, axis-aligned saw-tooth wall crenellation, overlapping street alpha seam, rooftop-crossing screen-space line, and 1px Laplacian PBR noise floor penalty in `VRBS ROMA` (`pbrMaterials.ts`, `lightingAndFx.ts`, `hydraulicNetwork.ts`, `worldRenderer.ts`) while maximizing `@ngi/platform-ssot` CV Flywheel visual scoring (`docs/src/architecture/urbs-photorealistic-edge-softening-and-cv-maximization-spec-2026.md`).

**Planned Phases in `v5.3` (`Phases 295–300`, `24` requirements):**
- **Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression (`Phase 295` — `URBS-EDGE-PBR-01..04`)**
- **True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums (`Phase 296` — `URBS-TERRACE-CRAFT-01..04`)**
- **Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae (`Phase 297` — `URBS-ARCH-REALISM-01..04`)**
- **Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid (`Phase 298` — `URBS-PLUMBING-3D-01..04`)**
- **Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics (`Phase 299` — `URBS-FLOW-SEMIOTICS-01..04`)**
- **Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization (`Phase 300` — `URBS-CV-MAX-01..04`)**

## Completed Milestone: v5.2 — Smooth Physically-Grounded Lighting, High-Detail Vitruvian Architecture & Grounds, Integrated Roman Hydraulic/Sewer Grid (`Aquaeductus`, `Fistulae` & `Cloaca Maxima`) & Animated Shape+Color Activity Conduits (`Phases 289–294`, `24/24` requirements)

**Goal:** Eliminate alpha-overlay lighting artifacts and repeating water tile seams, enrich terrace ground architecture (`Cardo`/`Decumanus` street grid, `Opus Sectile` marble forum plazas, vaulted `Substructio` cliff retaining arcades) and `T0`–`T3` building massing (rusticated ground-floor `Fornices` arcades, inter-story `Marcapiano` belt cornices, atrium `Impluvium` rainwater basins, and rooftop `Acroteria` + hypocaust chimneys), integrate the `AQVAE MCP` aqueduct into a city-wide SimCity-style Roman hydraulic supply (`Castellum Divisorium`, pressurized `Fistulae` pipes, bronze sluice valves) and `Cloaca Maxima` sewer/drainage trench network (`hydraulicNetwork.ts`), and animate live activities in progress using multi-channel Color + Geometric Shape flow tokens (`docs/src/architecture/urbs-hydraulic-plumbing-and-detailed-architecture-spec-2026.md`).

**Planned Phases in `v5.2` (`Phases 289–294`, `24` requirements):**
- **Smooth Ground-Plane-Recessed Lighting, 2D-Feathered Elliptical Sun-Shafts & Periodic Seamless Sea Synthesis (`Phase 289` — `URBS-SMOOTH-LIGHT-01..04`)**
- **High-Detail Terrace Grounds: Orthogonal `Cardo`/`Decumanus` Streets, `Opus Sectile` Marble Plazas, Vaulted `Substructio` Cliff Arcades & Balustrades (`Phase 290` — `URBS-GROUNDS-01..04`)**
- **High-Detail Vitruvian Building Architecture (`T0`–`T3`): Rusticated Ground Arcades (`Fornices`), Belt Cornices, Atrium `Impluvia` & `Acroteria` Roofs (`Phase 291` — `URBS-ARCH-DETAIL-01..04`)**
- **Integrated Roman Hydraulic Plumbing & Sewer Network (`hydraulicNetwork.ts`): `Aquaeductus` Trunk, `Castellum Divisorium`, `Fistulae` Supply Pipes & `Cloaca Maxima` Sewer Trenches/Outfalls (`Phase 292` — `URBS-HYDRAULIC-01..04`)**
- **Multi-Channel Color + Geometric Shape Animated Activity Conduits (`AQUA PURA` Chevrons, `IGNIS OPERIS` Diamonds, `CLOACA FAULT` Triangles & `VIA VICTORIAE` Seals) (`Phase 293` — `URBS-CONDUIT-ANIM-01..04`)**
- **Diegetic Hydraulic Sluice-Gate Callouts, Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence (`Phase 294` — `URBS-SEMIOTICS-CV-01..04`)**

## Completed Parallel Milestone: v5.1 — T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure (`Phases 283–288`, `24/24` requirements — Complete 2026-10-25)

**Goal:** Close all 6 defect clusters (`Cluster 1..6` documented in [`docs/src/architecture/t3-vox-wave5-deep-review-and-parity-closure-2026.md`](../docs/src/architecture/t3-vox-wave5-deep-review-and-parity-closure-2026.md)) uncovered by the adversarial deep review of Vox's T3 implementation against `t3code`, `t3router`, `pi-t3chat`, `NotT3Chat`, `t3-chat-tomdoestech`, and `t3-cloneathon`: (1) Prompt Variable Target Fix, Sanitizer Tag Completeness, `ToolRefusalDetector` & Lazy MCP Meta-Tools, (2) Sequenced `ResumableTurnStreamRegistry`, `ThreadRunQueue` Follow-Up/Limit-Recovery Draining, Real `ModelSpec` Cost Ledger & `SessionPrivacyPosture` Enforcement, (3) Worktree Async `tokio::sync::Mutex` Lock Fix, Monotonic Sparse Git Checkpoints, Clean Restore & Real `MultiVariantWorktreePool`, (4) UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Recursive External Session Discovery & Real Guest Share/Diagnostics, (5) Unified `message_id` Correlation Across Tauri & MCP, Live Transcript Edit/Regenerate/Compact, Mid-Turn Steer Composer & `streamWatchdog.ts`, and (6) `vox-code-audit` T3 Detector Catalog Registration, `oxlint-plugin-t3code` Rule Parity & 43-Violation Workspace Remediation.

**Completed Phases in `v5.1` (`Phases 283–288`, `24/24` requirements):**
- **System Prompt `$variable` Interpolation Fix, Sanitizer Tag Completeness (`<tool_call>`, `<think>`, `<WebFetch>`), `pi-t3chat` `ToolRefusalDetector` (`[SYSTEM TOOL CORRECTION]`), Search Circuit Breaker & `LazyMcpGroupRegistry` (`Phase 283` — `T3-PROMPT-LOOP-01..04`)**
- **Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` (`pop_next_follow_up`, Background Settlement & 429 Recovery), Real `ModelSpec` Cost Accounting (`4h`/Monthly Windows) & `SessionPrivacyPosture` Disk Gate (`Phase 284` — `T3-STREAM-QUEUE-01..04`)**
- **`DelegationWorktree::create` Async Lock Fix, Persistent `WebhookDedupeAndRateGate`, Monotonic Sparse Git Checkpoints + Clean Restore, Real `MultiVariantWorktreePool` & Compact Snapshot Fix (`Phase 285` — `T3-WORKTREE-CKPT-01..04`)**
- **UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Depth-4 External Session Discovery (`~/.claude`, `~/.codex`, `~/.cursor`), Real Guest Share Submit/Redeem & Live Diagnostics (`Phase 286` — `T3-TOOLS-HARDENING-01..04`)**
- **Unified `message_id` & Telemetry Across Tauri/MCP, Live User+Assistant Edit / Executing Regenerate / `✂ Compact Here`, Mid-Turn `[⚡ Steer]` Composer & `streamWatchdog.ts` + `resumeChatStream` (`Phase 287` — `T3-GUI-PIPELINE-01..04`)**
- **`vox-code-audit` T3 Detector Catalog Registration (`ALL_KNOWN_IDS`), `oxlint-plugin-t3code` Rule Completeness & Zero-Violation Workspace Remediation (`Phase 288` — `T3-AUDIT-PARITY-01..04`)**

## Active Parallel Milestone: v5.0 — Photorealistic Vitruvian Material, Depth-Shading, Atmospheric Lighting & FX Engine (`Phases 277–282`, `24` requirements)

**Goal:** Eliminate flat solid-color polygon fills, hard cartoon black strokes, missing cast shadows, and HUD badge clutter in `VRBS` (`worldRenderer.ts` & `LudusSandbox.tsx`) by implementing the 6-subsystem Photorealistic Vitruvian Rendering Architecture (`docs/src/architecture/urbs-photorealistic-lighting-and-materials-spec-2026.md`): (1) Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`), (2) Projected Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels, (3) Sculpted Coastal Bathymetry, Stratified Cliff Geology & Cobblestone Plazas, (4) Dual-Key Golden-Hour/Twilight Atmospheric Lighting, Crepuscular God-Rays, Elevation Mist & Additive GI Pools (`lightingAndFx.ts`), (5) Deterministic Live Particle FX Engine (Forge Embers, Smoke Plumes, Waterfall Spray & Chisel Sparks), and (6) Diegetic Cliff-Apron Architrave Plaques & Architecture-First HUD Decluttering.

**Planned Phases in `v5.0` (`Phases 277–282`, `24` requirements):**
- **Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`): Pitted Roman Travertine Ashlar, Scalloped Terracotta Roof Tiles, Opus Incertum Flagstone Plazas, Stratified Basalt Cliffs & Anisotropic Hammered Bronze (`Phase 277` — `URBS-MAT-01..04`)**
- **True Isometric Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels (`drawIsoBox`, `drawIsoPitchedRoof` & Wonders) (`Phase 278` — `URBS-DEPTH-01..04`)**
- **Sculpted Coastal Bathymetry, Surf Foam Ribbons, Craggy Cliff Geology, Cobblestone Terraces & Waterfall Caustics (`Phase 279` — `URBS-TERRAIN-01..04`)**
- **Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric Crepuscular God-Rays, Elevation Mist & Additive Point-Light GI Pools (`lightingAndFx.ts`) (`Phase 280` — `URBS-LIGHT-01..04`)**
- **Live Particle FX Engine: Rising Forge Embers, Volumetric Smoke Plumes, Waterfall Mist Spray & Mason Chisel Sparks (`Phase 281` — `URBS-FX-01..04`)**
- **Diegetic Cliff-Apron Architrave Plaques, Architecture-First HUD Decluttering & Playwright CV Flywheel Convergence (`Phase 282` — `URBS-DIEGETIC-01..04`)**

## Completed Milestone: v4.9 — Vitruvian Roman Tactical Developer Controller: Zero-Hop Diff/Diagnostic Loupe, Multi-Agent Write-Set Collision Cones, Clickable Viaduct Contract Inspector, Hard Agent Scope Quarantine Fencing, Milestone Road Turn Time-Scrubber & Split-Dock Workbench Integration (`Phases 271–276`, `24/24` requirements)

**Goal:** Elevate `VRBS` from an architectural observability view into a zero-hop interactive tactical developer controller (`docs/src/architecture/urbs-tactical-developer-controller-spec-2026.md`) across 6 Vitruvian mechanisms: (1) Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*), (2) Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*), (3) Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*), (4) Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) enforced in `agent_loop.rs`, (5) Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`), and (6) 1-Click Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*).

**Planned Phases in `v4.9` (`Phases 271–276`, `24` requirements):**
- **Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*): `workspace_file_diff_and_diagnostics` Tauri RPC + Floating Bronze & Obsidian Loupe with `[✓ Stage File]`, `[↺ Revert File]` & `[⚒ Fix Exact Error]` (`Phase 271` — `URBS-LOUPE-01..04`)**
- **Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*): `collisionRadar.ts`, Intersecting Surveyor Cones, Crossed-Swords Collision Standard & 1-Click `[⇄ Serialize Agents]` / `[🌿 Split Worktree]` (`Phase 272` — `URBS-COLLISION-01..04`)**
- **Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*): `TownDependencyEdge` Scanner, Viaduct Hit-Testing, Cracked Red Keystone on `layers.toml` Inversions & `PONS CONTRACTVS` Inspector (`Phase 273` — `URBS-VIADUCT-01..04`)**
- **Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*): `ServerState` `quarantined_paths`, `vox_quarantine_set` / `vox_quarantine_list` MCP Tools, Pre-Execution Write Guard in `agent_loop.rs` & Wrought-Iron Perimeter Portcullis (`Phase 274` — `URBS-QUARANTINE-01..04`)**
- **Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`): `vcs_checkpoint_diff_summary` RPC, `ViaTemporisScrubber.tsx` Sundial Scrubber, Per-Turn Diff/Test-Shield Preview & 1-Click Rollback (`Phase 275` — `URBS-SCRUBBER-01..04`)**
- **Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*): `urbs-tactical-map` Dock Panel, Bi-Directional Chat/Review District Filter & Real-Time Tool-Call Mechanism Animation (`Phase 276` — `URBS-DOCK-01..04`)**

## Active Parallel Milestone: v4.8 — T3 Wave 4 Live Pipeline End-to-End Closure: Real LLM Consensus, Stateful Session/Guest/Checkpoint Persistence, Live Agent-Loop Guards & Tauri/MCP Envelope Unwrapping (`Phases 265–270`, `24` requirements)

**Goal:** Close all 10 live-pipeline integration gaps (`GAP-01..10` documented in [`docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md`](../docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md)) between the T3 Wave 1–3 domain primitives and the live `vox_chat_message`, `agent_loop`, `t3_wave3_tools`, `agy_worktree`, `webhook_intake`, and Tauri IPC execution paths.

**Planned Phases in `v4.8` (`Phases 265–270`, `24` requirements):**
- **`tool:<name>` Fence Salvage Fix, Live `@rule:<name>` Mention Resolution & `$variable` System Prompt Interpolation in `vox_chat_message` (`Phase 265` — `T3-SALVAGE-RULES-01..04`)**
- **Live `context_sanitizer` History Stripping, `vox_chat_compact_up_to` Anchor Compaction, `BatteryCostLedger` Pre-Flight/Post-Turn Accounting, `VoxIdAllocatorV2` Message IDs & Real `ResponseQualityAnalyzer` Telemetry (`Phase 266` — `T3-HIST-BATTERY-01..04`)**
- **Live `agent_loop` Integration: `CommandOutputHub` `\r` Normalization, `20 KiB` DOM Snapshot Shedding, Pre-Write Sparse Git Checkpoints (`should_sparse_checkpoint`), `ResumableTurnStreamRegistry` (`vox_chat_stream_resume`) & Mid-Turn `ThreadRunQueue` Steer Draining (`Phase 267` — `T3-LOOP-CHECKPOINT-01..04`)**
- **Stateful `t3_wave3_tools.rs`: Concurrent `join_all` LLM Consensus Fan-Out, Live Transcript Hydration/Persistence for `vox_context_handoff` & `vox_session_*`, Real `~/.claude`/`~/.codex`/`~/.cursor` File Scanning & Persistent `ServerState` `GuestShareService` (`Phase 268` — `T3-STATEFUL-TOOLS-01..04`)**
- **`vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools, `DelegationWorktree` `WorkspaceLeaseMap` + `.env` Provisioning, `submit_webhook_events` `WebhookDedupeAndRateGate` & Compact `vox_orchestrator_status` Wire Budget (`Phase 269` — `T3-REGEN-WORKTREE-01..04`)**
- **`t3Wave3Rpc.ts` Nested Tauri `{ is_error, result: { success, data } }` Envelope Unwrapping, Live `ChatTranscript.tsx` In-Place Edit & Posture-Aware Regeneration & Full-Stack Verification (`Phase 270` — `T3-GUI-ENVELOPE-01..04`)**

**Shipped in `v4.7` (`Phases 259–264`, `24/24` requirements — [Archive](milestones/v4.7-ROADMAP.md) | [Requirements](milestones/v4.7-REQUIREMENTS.md) | [Audit](v4.7-MILESTONE-AUDIT.md)):**
- **Polyglot Repository Scanner (`workspace_town.rs`): Cargo, npm/pnpm/TS, Python, Go, C/C++ & Single-Package Directory Partitioning + Live Git Diff (`+LoC / -LoC`) Enrichment (`Phase 259` — `URBS-POLYGLOT-SCAN-01..04`)**
- **Zero-Hardcoding Graph Topology & Structural Archetype Classifier (`topology.ts`, `layout.ts`, `worldRenderer.ts`) (`Phase 260` — `URBS-TOPO-ARCHETYPE-01..04`)**
- **Attention-Weighted Dynamic Real-Estate Layout: Inflating Active Worksites, Git Diffs, Errors & Decision Gates while Compacting Dormant Modules (`Phase 261` — `URBS-ATTENTION-LAYOUT-01..04`)**
- **Interactive Foreground Worksite Cards: Live File Diffs (`+LoC / -LoC`), Compiler/Test Diagnostic Siege Cards & `Needs You` Decision Gates (`Phase 262` — `URBS-WORKSITE-HUD-01..04`)**
- **Dependency Blast-Radius Illumination, Stale-Test Propagation & Polyglot Architecture Inspector (`ImperiumInspector.tsx`) (`Phase 263` — `URBS-BLAST-INSPECTOR-01..04`)**
- **Multi-Repo Simulation Presets, Playwright CV Flywheel Verification & End-to-End Verification (`Phase 264` — `URBS-CV-CLOSURE-01..04`)**

**Shipped in `v4.6` (`Phases 253–258`, `24/24` requirements — [Archive](milestones/v4.6-ROADMAP.md) | [Requirements](milestones/v4.6-REQUIREMENTS.md) | [Audit](v4.6-MILESTONE-AUDIT.md)):**
- **Workspace-Wide TOESTUB (`unreasoned-suppression`, `flaky-wait-and-unreasoned-skip`) & `vox-arch-check` Remediation Sweep (`Phase 253` — `T3-AUDIT-SWEEP-01..04`)**
- **Workspace-Wide MCP Error Sanitization (`access-and-error-safety`), UI Layout/V8 Hazard (`layout-and-perf-hazards`) & Doc/Typecheck Gate Closure (`Phase 254` — `T3-MCP-UI-SWEEP-01..04`)**
- **Live MCP Tool Registry & `dispatch.rs` Wiring for `ConsensusFanOutEngine`, `GitPlumbingCheckpointStore`, `MultiVariantWorktreePool`, `ContextHandoffService`, `GuestShareService` & `AgentSessionImporter` (`Phase 255` — `T3-MCP-RPC-WIRE-01..04`)**
- **`ChatTranscript.tsx`, `ArenaComparisonCard.tsx` & `Loquela.tsx` Integration: Multi-Model Consensus Mode Toggle (`ConsensusGridView`), LCA Branch Switcher (`BranchComparisonModal`), Chunked Thread Loader (`CHUNK_SIZE = 16`), Deferred Highlighting & 100-Entry `PromptHistoryController` (`Phase 256` — `T3-CHAT-UI-WIRE-01..04`)**
- **Live Git Checkpoint Timeline & Restore Bar, External Session Import Modal (`~/.claude`, `~/.codex`, `~/.cursor` + JSON Bundle) & 6-Tier `ConnectionRouteRacer` Transport Wiring in `vox-gui` (`Phase 257` — `T3-VCS-IMPORT-UI-01..04`)**
- **Live `ModelSelectionEngine` (`registry.rs` / `scoring.rs`) Integration of `ExternalBenchmarkPrior` (`SweBench`/`LiveBench`/`SkateBench`/`SnitchBench`) + `ResponseQualityAnalyzer` (`0..=100` Flesch-Kincaid Score) & Quality-Gated Auto-DPO Preference Harvesting (`Phase 258` — `T3-ROUTER-DPO-FLY-01..04`)**

**Shipped in `v4.5` (`Phases 242–252`, `44/44` requirements — [Archive](milestones/v4.5-ROADMAP.md) | [Requirements](milestones/v4.5-REQUIREMENTS.md)):**
- **Deterministic Entity IDs (`VoxIdAllocatorV2`), `ForwardCompatibleEnvelope` & Wire ID/Body Deduplication (`Phase 242` — `T3-WIRE-ID-01..04`)**
- **Atomic Mid-Stream Multi-Tab Resumption (`SemaphoreSlim` Pattern), `CommandOutputHub` `\r` Normalization, Live Guest Share & `ThreadRunQueue` (`Phase 243` — `T3-STREAM-GUEST-01..04`)**
- **O(1) 1024-Slot Ring-Buffer `RepetitionGuard`, `ToolRefusalDetector` Auto-Retry & MCP Tool-Search Cache-Write Guard (`Phase 244` — `T3-GUARD-REFUSAL-01..04`)**
- **Battery Unit (`1 BU = $0.001`) Pre-Flight Gate, 3-Tier Cost Ledger, Multi-Benchmark Priors (`SnitchBench`/`SkateBench`/`SweBench`), `up_to` Compaction & Context Sanitization (`Phase 245` — `T3-BATTERY-LEDGER-01..04`)**
- **Multi-Model Consensus SSE Fan-Out & Deterministic Flesch-Kincaid `ResponseQualityAnalyzer` (`Phase 246` — `T3-CONSENSUS-QA-01..04`)**
- **Windows NTFS Junction-Safe Worktrees, `WorkspaceLease`, Git Plumbing Checkpoints (`refs/vox/checkpoints/`), Muse Pool & PR Sync/Watch/Review-Guide Reactors (`Phase 247` — `T3-WORKTREE-VCS-01..04`)**
- **Cross-Provider `ContextHandoffService` (`16K` Token / `64KB` Budget), `McpToolAccess` Capability Gates, DPoP, `McpAppModelContext` & 20KB DOM Shedding (`Phase 248` — `T3-HANDOFF-MCP-01..04`)**
- **Local-First Chunked Lazy Loading (`CHUNK_SIZE = 16`), Write Suppression & Tree Branch `divergencePoint` Comparison UI (`Phase 249` — `T3-CLIENT-BRANCH-01..04`)**
- **`@rule` User Rules (`Manual`/`Always`), `$variable` Dynamic System Prompts, Regenerate-with-Posture vs. In-Place Edit, 100-Entry `promptHistory` & 3-Mode Fuzzy/Trigram Search (`Phase 250` — `T3-RULES-SEARCH-01..04`)**
- **Signed/Filtered Webhook Ingress, Cron Scheduled Tasks, External CLI Agent Session Importer (`~/.claude`/`~/.codex`/`~/.cursor`) & 6-Tier Staggered Route Racing (`Phase 251` — `T3-WEBHOOK-IMPORT-01..04`)**
- **TOESTUB (`vox-code-audit`) Enforcement Detectors (`unreasoned_suppression`, `test_clock_and_skip`, `mcp_access_and_error_safety`, `ui_perf_hazards`), Wire/SQL Transfer Budget CI Gate & Operational Rules (`Phase 252` — `T3-TOESTUB-RULES-01..04`)**

**Shipped in `v4.4` (`Phases 236–241`, `24/24` requirements):**
- **Distinct Monumental Roman Wonders, Multi-Arch `AQVAE` Aqueducts & Rich District Callout Cards (`Phase 236` — `URBS-WONDER-01..04`)**
- **Fully Animated 60fps `VRBS ROMA` Simulation Loop: Live A* Citizens, Aqueduct Flow & Torchlight (`Phase 237` — `URBS-ANIM-01..04`)**
- **Concept Art 1 Convergence: Rich Hydrated Chat Thread, Circular Progress Ring & Wired Popover Controls (`Phase 238` — `CHAT-CONCEPT-01..04`)**
- **Concept Art 2 Convergence: `NeedsYou` KPI Strip, Live Agent Topology DAG & Wired `Inspect Diff` (`Phase 239` — `WORKBENCH-CONCEPT-01..04`)**
- **Animated SVG Flow Edges, Live Console Prompt Hydration & Zero-Paper-Code Wiring (`Phase 240` — `FLOW-CONSOLE-WIRE-01..04`)**
- **CV Flywheel Re-Capture, Before/After Archiving & Concept-vs-Actual Showcase (`Phase 241` — `CV-SHOWCASE-01..04`)**

**Shipped in `v4.3` (`Phases 230–235`, `24/24` requirements):**
- **Urbs Isometric Camera Centering, Occupied-Bounds Math & Concept Art 3 Blueprint (`Phase 230` — `URBS-CENTER-01..04`)**
- **Creative Roman Civilization Architecture, Via Appia Roads, Aqueducts & Dark-Glass Callouts (`Phase 231` — `URBS-CIV-ART-01..04`)**
- **Interactive Strategic Minimap Radar & Dashboard Embedded Minimap Uncrowding (`Phase 232` — `URBS-MINIMAP-01..04`)**
- **`MensTrainingView`, `TasksView` & `Console` Layout, Overflow & Readability Fixes (`Phase 233` — `SURFACE-FIX-A-01..04`)**
- **`ModelsView`, `BackendAvailability`, `ExplorerView` & `Dashboard` Readability Polish (`Phase 234` — `SURFACE-FIX-B-01..04`)**
- **CV Flywheel Simulator Bootstrap, Centering Detectors & Full 32-Surface Verification (`Phase 235` — `URBS-CV-GATE-01..04`)**

**Shipped in `v4.2` (`Phases 224–229`, `24/24` requirements):**
- **Concept-Art Fidelity & Visual Glitch Audit Matrix (`Phase 224` — `CONCEPT-AUDIT-01..04`)**
- **Top Chat Header Layout Switcher & Orphan Below-Composer Strip Removal (`Phase 225` — `CHAT-HEADER-CONCEPT-01..04`)**
- **Structured 4-Row Posture & Routing Popover Matrix & Antique-Gold Send CTA (`Phase 226` — `LOQUELA-POPOVER-CRAFT-01..04`)**
- **Session Inspector Rail Underline Tabs, Step Checkmarks & Hover-Disclosed Row Actions (`Phase 227` — `RAIL-CONCEPT-CRAFT-01..04`)**
- **Sidebar Gold Active Pill, Single System Header, KPI Sparklines & Harness/Models Glitch Fixes (`Phase 228` — `SHELL-WORKBENCH-CRAFT-01..04`)**
- **Full 32-Surface + 4-Chat/Dock/Urbs Playwright CV Flywheel Re-Capture & Visual Verification (`Phase 229` — `CONCEPT-CV-CONVERGENCE-01..04`)**

**Shipped in `v4.0` (`Phases 210–215`, `24/24` requirements):**
- **Dynamic Auto-Discovering GUI Surface, Sub-Tab, Dock & Popover CV Scanner (`Phase 210` — `CV-AUTO-DISCOVER-01..04`)**
- **Loquela Composer Progressive Disclosure, Single-Row Toolbar & Floating Posture Popover (`Phase 211` — `LOQUELA-CLEANUP-01..04`)**
- **Chat Session Inspector Rail Deduplication, Unified Plan/Task Checklist & Header Layout Switcher (`Phase 212` — `RAIL-UNIFY-01..04`)**
- **AppShell 5-Domain Sidebar Consolidation, Top Workbench Tab Bar & Status Bar Streamlining (`Phase 213` — `NAV-CONSOLIDATE-01..04`)**
- **Workbench Surface Styling, Above-the-Fold KPI Strip & Density Standardization (`Phase 214` — `SURFACE-CRAFT-01..04`)**
- **Concept Art v4.0 Comparison CV Flywheel Convergence & Zero-Regression Gate (`Phase 215` — `CV-CONCEPT-GATE-01..04`)**

**Shipped in `v3.9` (`Phases 203–209`, `28/28` requirements):**
- **Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph (`Phase 203` — `T3-CLONE-GRAPH-01..04`)**
- **Deep-Dive & Document `pingdotgg/t3code` Agent Harness Control Surface (`Phase 204` — `T3-CODE-HARNESS-01..04`)**
- **Deep-Dive & Document `t3.chat` Router, Model Discovery, Operational Status, Benchmark Feeds & Credit/Usage Accounting (`Phase 205` — `T3-ROUTER-SURFACE-01..04`)**
- **Deep-Dive & Document `t3.chat` Chat UX, Resumable Server Streams, Prompt Enhancement, Branching & Attachment Pipelines (`Phase 206` — `T3-CHAT-UX-01..04`)**
- **Synthesize the "Best-of-All-Worlds" Vox Gap Matrix & Architecture Blueprint (`Phase 207` — `T3-VOX-BLUEPRINT-01..04`)**
- **Implement Extracted Router Upgrades in Vox (`Phase 208` — `VOX-ROUTER-T3-01..04`)**
- **Implement Extracted Harness Upgrades in Vox (`Phase 209` — `VOX-HARNESS-T3-01..04`)**

**Shipped in `v3.8` (`Phases 199–202`, `16/16` requirements):**
- **"Show, Don't Tell" Hero & Experience Cards (`Phase 199` — `SHOW-DONT-TELL-01..04`)**
- **Concept Art B Chrome, Faceted Brand Header & SVG Icon Sidebar (`Phase 200` — `CONCEPT-CHROME-01..04`)**
- **High-Craft `.vox` Compiler Playground — Folded-Doc Schematic, Line-Numbered IDE Gutter & Live Syntax Highlighting (`Phase 201` — `PLAYGROUND-CRAFT-01..04`)**
- **Iterative Comparison CV Flywheel Convergence & Side-by-Side Visual Verification (`Phase 202` — `CV-CONVERGENCE-01..04`)**

**Shipped in `v3.7` (`Phases 193–198`, `24/24` requirements):**
- **Self-Maintaining Naming Cleanup, Badge-Spam Elimination & Freshness/Facet Sync (`Phase 193` — `DOCS-PROD-DB-01..04`)**
- **User-Centric Pinned Navigation, L0–L5 Crate Grouping & SKOS Domain Hierarchy (`Phase 194` — `DOCS-PROD-TAXONOMY-01..04`)**
- **Concept Art B 3-Tier Progressive Onboarding & Zero-Contradiction Quickstart (`Phase 195` — `DOCS-PROD-ONBOARD-01..04`)**
- **Functional 4-Target `.vox` Compiler Playground & Concept Art B Visual Polish (`Phase 196` — `DOCS-PROD-GUI-ART-01..04`)**
- **Live Captured Output vs. Concept Art B Comparison CV Flywheel & E2E Gate (`Phase 197` — `DOCS-PROD-GATE-01..04`)**
- **Codebase-Wide Zero `vox:skip` Doctest Enforcement & Latest `.vox` Syntax Modernization (`Phase 198` — `DOCS-PROD-VOX-SYNTAX-01..04`)**

**Shipped in `v3.6` (`Phases 188–192`, `20/20` requirements):**
- **Recursive Deep Crate Symbol Extraction, `#Lstart-Lend` Source Anchors, Reverse Dependents & Validated Mermaid Dependency Graphs (`Phase 188` — `OPENWIKI-API-01..04`)**
- **Materialized L0–L5 & Domain Subsystem Architecture Wiki Hubs (`vox ci wiki-sync`) & Starlight Sidebar Integration (`Phase 189` — `OPENWIKI-HUBS-01..04`)**
- **Zero-Deficit Freshness Baseline Ratchet (`baseline.v1.json` `7 -> 0`), Duplicate Cluster Consolidation & `ssot-drift` Ledger Sync (`Phase 190` — `OPENWIKI-FRESH-01..04`)**
- **Live CLI Evidence-Cited LLM Claim Verifier (`vox audit docs --verify-llm` & `--eval-golden-drift`) & Content-Hash Cache (`Phase 191` — `OPENWIKI-VERIFY-01..04`)**
- **Guarded Self-Healing Doc Bot (`vox ci doc-self-heal`, `scripts/docs-fix-drafts.vox`, `docs-self-heal.yml`) & End-to-End OpenWiki Integration Suite (`Phase 192` — `OPENWIKI-BOT-01..04`)**

**Shipped in `v3.5` (`Phases 183–187`, `20/20` requirements):**
- **Call-Graph Reachability & "Paper Code" Detector (`Phase 183` — `PAPER-GUARD-01..04`)**
- **Sub-Millisecond Hollow-Body, Stub & Placeholder Gate (`Phase 184` — `HOLLOW-GATE-01..04`)**
- **Dynamic Programming Enforcer — Magic-Value & Benchmark Special-Casing Detector (`Phase 185` — `DYNAMIC-CODE-01..04`)**
- **Anti-Reward-Hacking Test-Integrity Ratchet (`Phase 186` — `TEST-RATCHET-01..04`)**
- **Turn `Stop` Hook Live-Wiring Audit & Symbol Grounding Gate (`Phase 187` — `STOP-VERIFY-01..04`)**

**Shipped in `v3.4` (`Phases 178–182`, `20/20` requirements):**
- **Prefix-Locked Two-Tier Tool Architecture (`Phase 178` — `CACHE-PREFIX-01..04`)**
- **AST Symbol-Slice Read/Edit & Skeleton Folding (`Phase 179` — `AST-SLICE-01..04`)**
- **Pre-Edit Cross-Crate Grounding & Write-Time Repo Guards (`Phase 180` — `PREFLIGHT-GROUND-01..04`)**
- **Speculative Streaming Tool Prefetch (`Phase 181` — `STREAM-EXEC-01..04`)**
- **Workspace Test Canonicalizer, FIFO `FakeBackend` & SOTA Architecture SSOT (`Phase 182` — `FAST-APPLY-01..04`)**

**Shipped in `v3.3` (`Phases 173–177`, `20/20` requirements):**
- **Centralized Telemetry Sync, Live `CommunityIntelligenceBundleV1` Hydration & Bradley-Terry / Contextual Bandit Auto-Improving Router (`Phase 173` — `ROUTE-LEARN-01..04`)**
- **Side-by-Side Arena Turn Comparison, Pairwise Preference Voting & Centralized Telemetry / Auto-DPO Flywheel (`Phase 174` — `ARENA-COMPARE-01..04`)**
- **Conversation Branching (`fork_from_message_id`), Jujutsu (`vox-vcs`) Turn Checkpoints & 1-Click Code+Chat Rewind (`Phase 175` — `BRANCH-REWIND-01..04`)**
- **Sub-Transcript Tool-Result Clearing & Context Window Surgery (`Phase 176` — `CONTEXT-SURGERY-01..04`)**
- **Deterministic `.vox` Lifecycle Hooks (`PreToolUse` / `PostToolBatch` / `Stop`), `vox_oracle_consult` Escalation & Milestone `v3.3` Verification (`Phase 177` — `HOOKS-ORACLE-01..04`)**

**Shipped in `v3.2` (`Phases 163–172`, `40/40` requirements — [Archive](milestones/v3.2-ROADMAP.md) | [Requirements](milestones/v3.2-REQUIREMENTS.md) | [Audit](v3.2-MILESTONE-AUDIT.md)):**
- **Write-Tool Selection Priority & `vox_apply_structured_edit` Recovery Window (`Phase 163` — `SOTA-WRITE-PRIORITY-01..04`)**
- **Prefix-Immutable KV-Cache Tool Loops & Insertion-Time Output Bounding (`Phase 164` — `SOTA-PREFIX-IMMUTABLE-01..04`)**
- **Universal Multi-Format & Multi-Call Tool Salvage (`salvage_tool_calls_from_text`) (`Phase 165` — `SOTA-UNIVERSAL-SALVAGE-01..04`)**
- **True `syn` AST Structural Verification for Rust Edits (`Phase 166` — `SOTA-SYN-STRUCT-01..04`)**
- **8-Spoke Unseen Real-Query Hardening (`Phase 167` — `SOTA-SPOKE-UNSEEN-01..04`)**
- **Speculative Repair De-Escalation, Streamed Tool-JSON Suppression & Cumulative Turn Latency/TTFT Accounting (`Phase 168` — `SOTA-CASCADE-DEESC-01..04`)**
- **64-Case 8-Spoke Real-Query & Adversarial Benchmark (`canonical_adversarial_and_real_query_spoke_suite_64`) & `v3.2` SOTA Harness Scorecard (`Phase 169` — `SOTA-BENCH64-01..04`)**
- **TOESTUB Precision Engine, CI Separation of Concerns & Zero-Error Workspace Hygiene Gate (`Phase 170` — `SOTA-TOESTUB-HYGIENE-01..04`)**
- **Bi-Directional CAS Task Ledger Sync, Polygon-Free SVG Telemetry Charts, Single-Line Plain-English Controls & Zero-Occlusion CV Flywheel Convergence (`Phase 171` — `SOTA-GUI-AXIS-01..04`)**
- **Monolithic God Object Decomposition & 500-Line Module Ergonomics Gate (`Phase 172` — `SOTA-GOD-OBJECT-01..04`)**

**Shipped in `v3.1` (`Phases 157–162`, `24/24` requirements):**
- **8-Spoke Real-Query Router & Verifier Hardening: Prose-Wrapped Fences, Oratio Mid-Stream ASR Normalization, OpenAI MCP Envelopes & Workspace-Crate Rust Oracles (`Phase 157` — `SOTA-SPOKE-HARDEN-01..04`)**
- **Rolling Multi-Iteration Tool-Loop Cache Breakpoints & Tool-Schema Prefix Caching (`Phase 158` — `SOTA-WIRE-LOOP-01..04`)**
- **Live `context_fill_ratio`, `cacheable_workload` & Spoke-Conditioned `TaskCategory` Wiring in `select::decide` and `try_run_agent_turn` (`Phase 159` — `SOTA-LIVE-ROUTE-01..04`)**
- **Path-Scoped Read-Memo Invalidation, Canonical JSON Arg Keys & Edit-Failure Line-Window Recovery (`Phase 160` — `SOTA-PATH-MEMO-01..04`)**
- **Spoke-Aware Autonomous Retrieval Preamble Gating & Bounded Latency Fast-Path (`Phase 161` — `SOTA-FAST-PREAMBLE-01..04`)**
- **48-Case Real-Query & Adversarial 8-Spoke Benchmark (`canonical_adversarial_and_real_query_spoke_suite_48`) & `v3.1` SOTA Harness Scorecard (`Phase 162` — `SOTA-BENCH48-01..04`)**

**Shipped in `v3.0` (`Phases 152–156`, `20/20` requirements):**
- **Outbound Wire `stream_options.include_usage` & Prompt-Cache `cache_control` Breakpoints (`Phase 152` — `SOTA-WIRE-BP-01..04`)**
- **End-to-End `prompt_tokens`, `ttft_ms`, `tpot_ms`, Spoke `TaskCategory` & Spec-Fallback Cost Accounting (`Phase 153` — `SOTA-TELEM-CLOSE-01..04`)**
- **Cache-Read Pricing & Observed `p95_ttft_ms` / `p95_tpot_ms` Goodput Scoring in `ModelSelectionEngine` (`Phase 154` — `SOTA-GOODPUT-ROUTE-01..04`)**
- **Zero-Latency Intra-Turn Read-Only Tool Memoization & Write-Invalidated Deduplication (`Phase 155` — `SOTA-TOOL-DEDUP-01..04`)**
- **Sub-5ms Workspace-Aware `.rs` Structural & Module-Graph Verification (`syn` Duplicate-Symbol / Missing-Submodule / Stub Gate) & `v3.0` Scorecard (`Phase 156` — `SOTA-RS-STRUCT-01..04`)**

**Shipped in `v2.9` (`Phases 147–151`, `20/20` requirements):**
- **Day-Stable System Prompt KV-Cache Prefix, Rider Deduplication & Spoke-Ranked Skill Catalog (`Phase 147` — `SOTA-KVCACHE-01..04`)**
- **Streaming SSE `usage` / `cache_read_tokens` Wire Parsing & End-to-End Cache-Hit Telemetry (`Phase 148` — `SOTA-WIRECACHE-01..04`)**
- **Sub-Millisecond Intent-Gated Autonomous Retrieval Preamble (`should_run_auto_chat_preamble`) (`Phase 149` — `SOTA-PREAMBLE-01..04`)**
- **Intra-Turn Tool Output Budgeting, Stale Iteration Masking & Persisted Turn Tool Ledger (`Phase 150` — `SOTA-INTRATURN-01..04`)**
- **Two-Tier Progressive Tool Schema Compression & `v2.9` SOTA Harness Benchmark Scorecard (`Phase 151` — `SOTA-SCHEMA-01..04`)**

**Shipped in `v2.8` (`Phases 142–146`, `20/20` requirements):**
- **Zero-Waste Single-Pass SSE `delta.tool_calls` Streaming & Universal Provider Tool Mapping (`Phase 142` — `SOTA-STREAM-01..04`)**
- **KV-Cache-Aligned Multi-Turn Wire Messages & Raw-Utterance `DEFAULT_RANKED_MAX_TOOLS = 20` Selection (`Phase 143` — `SOTA-PROMPT-01..04`)**
- **Live Wire-Up of `SessionWorkingMemory` & `blake3` Observation Masking in `conversation.rs` (`Phase 144` — `SOTA-MEMORY-01..04`)**
- **Sub-10ms AST Symbol-Skeleton Context Packing & Targeted Symbol Read/Edit Fast-Path (`Phase 145` — `SOTA-SLICE-01..04`)**
- **Real Intra-Turn Model Switching (`SelectionIntent::repair_loop`) & Persistent `VoxDb` Live Auto-DPO Flywheel (`Phase 146` — `SOTA-LIVE-DPO-01..04`)**

**Shipped in `v2.7` (`Phases 137–141` — SOTA Agent Harness):**
- **Spoke-Conditioned BM25 + Intent Tool Ranking & Schema Token Compression (`Phase 137` — `SOTA-TOOLS-01..04`)**
- **Sub-50ms Multi-Spoke Inline Post-Write Verification for `.vox`, `.rs` (`rustc`), and `SKILL.md` (`Phase 138` — `SOTA-VERIFY-01..04`)**
- **Dynamic Intra-Turn Speculative Model Cascade & Synchronous Doom-Loop Repair Intervention (`Phase 139` — `SOTA-CASCADE-01..04`)**
- **Parallel Read-Only Tool Batch Execution, Observation Masking & KV-Prefix-Stable `SessionWorkingMemory` (`Phase 140` — `SOTA-SPEED-01..04`)**
- **Live Intra-Turn Auto-DPO Trajectory Harvesting & Before/After SOTA Harness Benchmark Scorecard (`Phase 141` — `SOTA-FLYWHEEL-01..04`)**

**Shipped in `v2.6` (`Phases 133–136`):**
- **Everyday Natural-Language Spoke Classification, Signal Telemetry & Full Grammar Unification Gate (`Phase 133` — `CONV-INTENT-01..04`)**
- **Multi-Turn Conversational State, Anaphora/Context Carryover & Native 8-Spoke Provenance (`Phase 134` — `CONV-STATE-01..04`)**
- **Automatic Conversational Self-Repair Escalation (`SelectionIntent::repair_loop`) & DPO Contrast Emission (`Phase 135` — `CONV-REPAIR-01..04`)**
- **Extended 12-Turn Human Conversation Harness & 40-Case Decoupled 8-Spoke Evaluation (`Phase 136` — `CONV-BENCH-01..04`)**

**Archived `v2.5` Artifacts:** [`.planning/v2.5-MILESTONE-AUDIT.md`](v2.5-MILESTONE-AUDIT.md)

## Requirements

### Validated

- ✓ `.vox` compiler pipeline (lex → parse → HIR → typeck → codegen) and `--interp` execution path — `crates/vox-compiler/`
- ✓ `vox` CLI with full subcommand surface (`crates/vox-cli/`)
- ✓ Multi-agent orchestrator with D1-D10 policy modules and MCP tool/HTTP exposure — `crates/vox-orchestrator/`, `crates/vox-orchestrator-mcp/`
- ✓ Tauri 2 desktop GUI (`vox-gui`) as the sole orchestration surface (ADR-045)
- ✓ iroh QUIC mesh transport for Populi (ADR-047, locked)
- ✓ Durable functions runtime (`workflow`/`activity`/`actor`/`@scheduled`) with journal-backed replay (ADR-019/021/041, locked)
- ✓ Native Rust ML training (Candle + qlora-rs) (ADR-003/006/034, locked)
- ✓ Codex/Arca/Turso as the sole database stack (ADR-004, locked)
- ✓ Secrets resolution exclusively through `vox_secrets::resolve_secret(...)` (Clavis/`vox-secrets`)
- ✓ **Wave 0 (`Phase 05` — v1.5):** Unified branch & worktree convergence onto `feat/unified-2026-q4` (`CONV-01..04`)
- ✓ **Wave 1 (`Phases 12, 07, 07.1, 25, 32` — v1.5):** Autonomy safety floor (`FLOOR-01..05`, 0/34 surviving mutants), Trial identity & Tier A persistence at DB baseline 97 (`TRIAL-01`, `STORE-01..02`, `BENCH-09`), Unified Code & Architecture Graph Data Floor (`GRAPH-01..02`), and Docs trust signal calibration (`SIGNAL-01..04`, `810/810 verified`)
- ✓ **Wave 2 (`Phases 33–34, 13–17, 37–42, 46, 49, 52–53, 55–56, 58, 60–62` — v1.5):** OpenWiki Subsystem Hubs & deep crate symbols (`WIKI-01..04`), Docs corpus consolidation & debt burn-down (`DEBT-01..03`), Autonomy oversight daemon & consumers (`OVS-01..03`, `ATTN-01` at DB baseline 98), KB local embeddings/context/inbox/rewrites/staleness (`EMB`, `INJ`, `GROW`, `REWR`, `STALE`), Research Trial Race Harness & promotion governance (`RACE`, `KNOW`, `PROMO`, `HARD`, `BENCH-13`), Terminal Session Host CLI Attach & Agent `term_run` (`TERM-01..04`), LLM-First Structured Shell (`Nushell` + `Vox-Shell` `SHELL-01..04`), Deep Research Realistic Power (`RES-01..06`), SCIENTIA Automatic Publication Pipeline Closure (`PUB-01..04`), Autonomous Mermaid Observation, Validation & LLM Self-Repair (`MERMAID-01..04`), Continuous `README.md` & `voxlang.org` Frontpage Codebase Sync (`FRONTPAGE-01..03`), Comprehensive Unverified & Drifting Docs Resolution (`DOCVER-01..03`), Sustainability, Open Collective Activation & Grant Pack (`FUND-01..04`), External Binary Adoption & Git-Rev Codegen Fallback (`DIST-01..02`), and Standalone Crate Unbundling & `crates.io` Publication (`CRATE-01..03`)
- ✓ **Wave 3 (`Phases 06.4, 26–31, 43–45, 18–19, 47–48, 50–51, 54, 57` — v1.5):** Capability Explorer & Coverage Gate (`DISC-08..11`, `ExplorerView.tsx`, `gui-coverage-gate` with 294/294 CLI families and 349/349 MCP tools covered or audited), Code & Architecture Atlas GUI (`ATLAS-01..04`), KB `Library`/Context Inspector/Inbox/Health GUI (`GUI-KB-01..08`), `Oversight: 1..5` composer control + Claims Ledger (`GUI-OVS-01..03`, `CLAIMS-01`), Axis Console Warp-Parity Terminal & WebGL Renderer (`TERM-05..08`), Axis Harness River & Live Developer Dogfooding (`RIVER-01..04`), Bidirectional Chat ↔ Terminal Co-Execution & Security Levels (`CTERM-01..04`), Unified 7-Source IntelliSense (`COMP-01..04`), Unified Research & Scientia Publication Studio GUI (`SCIGUI-01..05`), and `voxlang.org` Visual Theme, Interactive Chrome, Sidebar Hiders & Beginner Frontpage (`CHROME-01..04`, `ONBOARD-01..02`)
- ✓ **Wave 4 (`Phases 35–36, 06.6, 59, 63` — v1.5):** Constrained nightly LLM doc claim verifier (`VERIFY-01..04`), self-healing draft-PR bot & freshness ratchet (`BOT-01..04`), cold-start time-to-find journeys (`DISC-15..17`), Phase 12 Mutation Gate Completion & Full Milestone `/gsd-code-review --fix` (`REVIEW-01..02`), and Empirical LLM Code-Gen Benchmark Preprint & `CR-L8` Flywheel (`EMP-01..03`)
- ✓ **Wave 5 (`Phases 64–69` — v1.5):** Polyglot `@tool` & `@form` auto-surface (`POLYTOOL-01..04`), Real-Time `Stream[T]`/`Channel[Send,Recv]`/reactive `db.Table.subscribe` & managed `ProcessRun` (`STREAM-01..04`), Effect-enforced spend/write guards (`uses { Spend, Write }`), `<WaitingNotice>` view contracts (`when waiting`) & `@ranker`/`@eval_shadow` ordering gates (`RANK-01..04`), User-facing hybrid search (`@embed`, `@searchable` FTS5+HNSW+Phonetic), partial `db.Table.patch` & `@table(scoped_by: tenant)` (`USERDB-01..04`), Presigned `Upload[T]`/`BlobStore` (S3/R2/Local + Range + OPFS), `@webhook` verification, `@rate_limit` & `@metered` credit ledger (`CLOUD-01..04`), and Decorator Golden Parity Sweep + Deterministic Tick/Skill Engine Reference Template (`SIM-01..04`)
- ✓ **Wave 6 (`Phases 70–76` — v1.5):** Worktree Land Wave (`land/w1..w5`) & Nightly-Green Closure (`NIGHTLY-01..05`), Clavis Next Parts 1 & 2 (`CLAVIS-01..08`), Mens Native ML Training Apple Silicon & Quantize Hardening (`MENS-01..04`), Populi Mesh Phase 3 `PopuliHttpOp` over iroh QUIC & A2A Mailbox (`MESH-01..03`), MENS 2026 `.vox` Script Writer Overhaul & Verifiable SOTA 5-Tier `vox chat` Model Routing (`MSCRIPT-01..06`, `SOTA-01..05`), and Humane, Pro-Social & Anti-Addictive Chat, Cost Honesty & Mastery UX (`HUMANE-01..04`)
- ✓ **v1.6 (`Phases 77–88`):** Multi-spoke `TrainingRecordV2` schema + `VoxDb` snapshots (`CORPUS-01..06`), 5-spoke round-trip verification (`SPOKE-01..06`), supervised `vox-orchestrator-d` auto-bootstrap & gateway/IPC parity (`CORE-01..05`), hermetic real-backend Playwright E2E harness + 32-surface `honestyScan.ts` (`REAL-01..04`), `.vox` `--interp` ↔ `RustAxum`/`RustTauri`/`TypeScript` execution parity (`VOX-01..05`), operational & data-intensive GUI surface hardening + CLI/dedicated floor ratchets (`OPS-01..05`, `DATA-01..05`), `vox doctor --fix` + full workspace CI gate parity (`GATE-01..04`), and Mens continuous QLoRA/DPO training flywheel & 2-cycle autonomous burn-in (`TRAIN-01..05`, `EVAL-01..05`, `AUDIT-01..05`, `LOOP-01..04`)
- ✓ **v1.7 (`Phases 89–97`, shipped 2026-10-19):** 26-variant strongly-typed telemetry + `VoxDb::audit_research_metric_shapes` + multi-surface model router verification (`TELEM-01..05`, `TYPING-01..04`, `ROUTER-01..04`, `DBAUDIT-01..04`), private `~/dev/vox-ops` 6-crate workspace + three-role fenced PostgreSQL 17 (`PGFENCE-01..05`, `HELIOS-01..04`), 9-adapter social syndication extraction + 3-way Ludus SoC split + `BLAKE3`/`Ed25519` `VerifiedLeaderboardSnapshotV1` (`VoxLeaderboardArena.astro` & `LeaderboardArenaPanel.tsx`) (`SOCIAL-01..05`, `LUDUS-SOC-01..05`, `LEADERBOARD-01..05`), $k \ge 20$ `CommunityIntelligenceBundleV1` + Empirical Bayes $\pm 15\%$ blending + working-tree/history scrub (`INTEL-01..05`, `SCRUB-01..04`), and 25-problem 5-spoke MENS before/after evaluation (`24.0% -> 72.0% -> 100.0% pass@1`) + `.safetensors` checkpoint verification (`MENS-GAP-01..04`, `MENS-PERF-01..04`, `MENS-HARNESS-01..04`, `MENS-SURFACE-01..04`, `LIVE-QLORA-01..04`).

### Active

- [x] **Shared-Package CV Flywheel (`@ngi/platform-ssot`) & Chat/Harness Baseline Audit (`CV-FLYWHEEL-01..04`, Phase 98 — `v1.8`):** Sync `platformSsot.generated.ts` (`runUniversalCvFlywheelPass`, `evaluateBeforeAfterCvFlywheel`) into `crates/vox-gui/ui/src/lib/`, build `cvFlywheel.ts` pixel/DOM analyzer (Sobel edge sharpness, RMS luminance contrast, high-frequency noise floor, PSNR, overflow/clipping/emoji checks), and run automated `BEFORE` baseline CV audits across Chat, Loquela, Docking, and Urbs Civilization surfaces.
- [x] **Chat GUI Knob Unification & End-to-End Strong Datatypes (`CHAT-KNOBS-01..04`, `CHAT-TYPES-01..04`, Phase 99 — `v1.8`):** Unify fragmented composer/drive/governance controls (`DriveConsole.tsx` + `Loquela.tsx` + `ChatSurface.tsx`) into a cohesive single-height VS Code-grade control strip (`Intent`, `Run Tier`, `Governance: Risk + Oversight 1..5 + @kb Scope + Grounding`), wire `kb_scope` and `oversight_level` into `ChatPayload` (`buildChatTurn.ts` / `types/tauri.ts`), and eliminate all `any` / loose `string` states across `Loquela.tsx` and `ChatSurface.tsx`.
- [x] **VS Code-Grade Docking & `terminal` / `urbs` Dock Panel Registration (`DOCK-UX-01..04`, Phase 100 — `v1.8`):** Register the unreachable `terminal` dock panel and new `urbs` (Civilization Map) dock panel in `OPT_IN_PANEL_IDS` and `panelDefs`, add one-click workspace layout presets (`Focus Chat`, `Engineering Split`, `Civilization & Harness`, `Reset Layout`) to `DockWorkspaceShell.tsx`, and upgrade `.dockview-theme-vox` tab chrome to VS Code visual quality.
- [x] **Urbs Civilization Frontend Visibility, Citizen SVG Sprites, Live MCP `AQVAE` & Iterative CV Convergence (`URBS-VIS-01..05`, Phase 101 — `v1.8`):** Fix the `embedded` animation/telemetry freeze in `LudusSandbox.tsx`, replace raw `🧍` citizen emojis with role-tinted isometric SVG Roman sprites (`CitizenSprite`), populate `harness.mcp` in `urbs/harnessData.ts` so `AQVAE` lights up with real MCP health, add the Landmark Quick-Jump HUD (`FORVM`, `CASTRVM`, `PORTVS`, `AQVAE`, `PORTA`) + `River | Urbs Civilization | Split` mode switcher in `HarnessRiverPanel.tsx`, and iterate `BEFORE -> AFTER` CV Flywheel passes until all visual/layout glitches are zeroed out (`flywheelConverged === true`).

## Constraints

- **Offline/Standalone Sovereignty:** `vox`, `vox-gui`, and `vox-orchestrator-d` must always work 100% offline or with `vox-ops` unreachable/blocked; remote intelligence and telemetry are strictly additive and never required.
- **Telemetry Consent & Transparency:** Remote upload defaults to **OFF** (`ConsentState::Unset`); `VOX_TELEMETRY=off` and org policy always take precedence; users can inspect every byte via `vox telemetry preview` before any network call; zero shadow telemetry tables outside `vox-telemetry`.
- **Zero-Cheating Server-Authoritative Leaderboards:** Global community & model leaderboards never accept client-submitted XP, Lumens, or SQLite profile dumps; every ranked point must trace to a server-verified GitHub webhook/API receipt or signed 5-spoke benchmark receipt chained in `ludus_community.verified_contributions` (`BLAKE3` + `Ed25519`).
- **Strong Typing for Data & Code:** All telemetry categories, router decisions, and DB rows (local `VoxDb` and remote `helios-postgres`) must use strongly-typed Rust enums/structs and SQL column/check constraints — never untyped string/JSON bags for structured attributes.
- **Zero Worktree Deletion:** Never delete or clean up existing worktrees under `.claude/worktrees/` or `.worktrees/`.
- **Build toolchain:** Pinned Rust `1.98.1` (`rust-toolchain.toml`); no cmake, nasm, Go, perl, or libclang via any dependency.
- **Crate dependency edges:** CI-gated by `vox ci crate-edges` against `contracts/ci/crate-edges.allow.v1.json`.
- **LLM provider boundary:** All LLM calls go through `vox_actor_runtime::llm`.
- **Cryptography & Secrets:** Application crypto exclusively through `vox-crypto`; secrets exclusively through `vox_secrets::resolve_secret(...)`.
- **VoxScript-first glue:** Automation is `.vox` run via `vox run`, never new `.sh`/`.ps1`/`.py` in the public `vox` repository.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| **D15: Zero Worktree Deletion** | Preserve all 42 git worktrees under `.claude/worktrees/` and `.worktrees/` and 6 historical `.planning/workstreams/<name>/` directories during convergence. | ✓ Good — zero work lost across 86 phases |
| **D16: VoxDb Schema Sequence (`95 -> 96 -> 97 -> 98`)** | Strictly order schema migrations across `kb` (`96`), `research-trial-flywheel` (`97`), and `autonomy-ux` (`98`) with `BaselineStampedWithoutTables` guards. | ✓ Good — zero schema digest conflicts |
| **D17: Single `catalog.v1.yaml` Convergence (`05-04`)** | Reconcile all CLI/MCP/GUI operations in one catalog pass before regenerating SSOT artifacts. | ✓ Good — 996 CLI + 349 MCP tools + 32 GUI surfaces unified |
| **D18: Shared `vox-graph-reader` v5 Floor (`Phase 25`)** | Build one budgeted `AtlasQuery` (`400` nodes / `800` edges) serving both OpenWiki Subsystem Hubs (`Phase 33`) and the GUI Code Atlas (`Phases 26–31`). | ✓ Good — eliminated duplicate AST/graph extractors |
| **D19: Sequential Wave 3 GUI Unification** | Order React surface edits (`06.4 -> 26..31 -> 43..45 -> 18..19 -> 47..57`) on shared files (`Loquela.tsx`, `ChatSurface.tsx`, `Sidebar.tsx`, `Console/*.tsx`). | ✓ Good — zero merge conflicts across 10 GUI tracks |
| **D20: Clean-Room `SessionHost` (`vox-terminal-core`)** | Unify `vox term` and Axis GUI Console on `SessionHost` (`DaemonHost` + `LocalHost`) without AGPL Warp code. | ✓ Good — retired duplicate OSC-633 reducers and session managers |
| **D21: Action-Only Safety Floor (`classify_floor`)** | Classify destructive actions over structured fields (`permission-modes.v2.yaml`) with 34 mutation proofs (`0` surviving mutants). | ✓ Good — unbypassable across CLI, GUI, MCP, and HTTP |
| **D22: Value-Ordered Zero-Mock Surface Reality (`v1.6`)** | Prioritize Core Daily Driver stability (`Chat`, `Terminal`, `Daemon` auto-bootstrap) and a real-backend E2E test harness before secondary operational/research surfaces. | ✓ Good — all 12 `v1.6` phases (`77–88`) verified and shipped |
| **D23: Three-Role Fenced Postgres on `helios-postgres` (`v1.7`)** | Replace public `server/telemetry/` ClickHouse with a single database `vox_ops` on `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo`) using `vox_ops_owner` (`NOLOGIN`), `vox_ingest_app` (`INSERT`-only), and `vox_ops_app` (`SELECT`-only on `SECURITY DEFINER` $k \ge 20$ views + RW on `syndication.*`/`community.*`/`ludus_community.*`) provisioned via `scram-verifier.py`. | ✓ Good — verified live (`t|f|f|t|f|f|t`, `Phase 90`) |
| **D24: Private `~/dev/vox-ops` Repository Split (`v1.7`)** | Move centralized telemetry ingest, community intelligence aggregation, live social media syndication/community automation (`vox-publisher` social adapters + Postgres `syndication.outbox`), and multi-user Ludus (`collegium`, `arena`, `leaderboards`, `disputes`, GitHub sync) to private `~/dev/vox-ops` (`brbrainerd/vox-ops`); retain local scholarly/academic manifest prep and solo Ludus in public `vox`. | ✓ Good — 6-crate `vox-ops` workspace shipped (`Phases 90–91`) |
| **D25: Ed25519-Signed $k \ge 20$ Community Intelligence Flywheel (`v1.7`)** | Distribute community-collated routing priors, OpenRouter free-tier liveness, `default_decision` constants, and `Confirmed` `ModelPromptProfile` hints via a static, Ed25519-signed `CommunityIntelligenceBundleV1` with 100% offline/standalone fallback when disabled or unreachable. | ✓ Good — Empirical Bayes $\pm 15\%$ clamp verified (`Phase 92`) |
| **D26: Strong Static Typing Across All Telemetry, Router & DB Boundaries (`v1.7`)** | Enforce strongly-typed Rust enums/newtypes (`DurationBucket`, `TokenBucket`, `CommandExitClass`, `GpuVendor`, `VramBucket`, `DomainCategory`, `DomainDefectClass`, `SaltedHashHex`), `TypedResearchMetricRow` deserialization in `vox-db`, and `sqlx::FromRow` + Postgres `CHECK` constraints in `vox-ops` so no untyped string/JSON bags cross system boundaries. | ✓ Good — 26-variant `audit_research_metric_shapes` verified (`Phases 89–92`) |
| **D27: Multi-Surface Model Router Verification (`v1.7`)** | Verify `vox-orchestrator::models` (`registry`, `select`, `auto_select`, `autonomic`, `scoring`, `prompt_profiles`) across CLI, `vox-orchestrator-d` daemon, `vox-orchestrator-mcp`, and `vox-gui` IPC surfaces, confirming deterministic routing, Empirical Bayes blending, and strongly-typed telemetry emission. | ✓ Good — all 4 surfaces verified (`Phases 89 & 92`) |
| **D28: Live Helios Deployment & Database Shape Audit (`v1.7`)** | Deploy `vox-ops` live onto Helios (`178.156.212.19`, Coolify project `no88080okk0c0gk4ss0cgw0o`, `helios-postgres` `q40ck888ggkkcsosgcwgc0oo`) by driving the Coolify API and SSH/container tooling, run end-to-end client uploads and bundle syncs against the live server, and audit both local `VoxDb` and live `helios-postgres` tables to prove normal data is gathered in the exact expected strongly-typed shapes. | ✓ Good — live `vox-ops-server` (`127.0.0.1:8420`) verified (`Phase 92`) |
| **D29: 3-Way Ludus & Gamification Separation of Concerns (`v1.7`)** | Split `vox-gamify` along true architectural boundaries: (1) extract core `agent_telemetry` (`agent_events`, `cost_records`, `agent_sessions`) into `vox-db` (`store::ops_agent_telemetry`), (2) move multi-user competitive/social Ludus (`collegium`, `arena`, `leaderboards`, `disputes`, GitHub OAuth & event sync) to private `vox-ops` (`ludus_community.*` in Postgres), and (3) keep solo offline quests/companions/battles/FSRS discovery local in `vox-gamify` while deleting shadow telemetry tables (`gamify_hint_telemetry`, `lsp_telemetry`). | ✓ Good — 229 `vox-gamify` tests passing offline (`Phase 91`) |
| **D30: Codebase & Git-History Scrubbing of Moved Data & Shadow Telemetry (`v1.7`)** | Remove all moved server/social/multi-user Ludus files and shadow gamify telemetry from the public working tree, enforce non-reintroduction via `contracts/ci/moved-data-scrub-manifest.v1.json`, and codify deterministic git history & commit-log scrubbing via `contracts/ci/git-history-scrub.v1.json` + `scripts/verify-history-scrub.vox` (`git filter-repo`), while preserving transparent opt-in `vox-telemetry` (`vox telemetry preview`). | ✓ Good — 22 forbidden paths absent, 8 preserved (`Phase 92`) |
| **D31: Server-Authoritative Zero-Cheating Leaderboard & v0 / Claude Design Surfaces (`v1.7`)** | Eliminate client-trusted XP/Lumens leaderboard queries in favor of `vox-ops-ludus` `ludus_community.verified_contributions` (append-only `BLAKE3` hash chain of HMAC-verified GitHub contributions and signed 5-spoke benchmark receipts, enforced by `AntiCheatPolicyV1` and signed with Ed25519 into `VerifiedLeaderboardSnapshotV1`), and surface dynamic 4-lens leaderboards with cryptographic proof drawers on both `voxlang.org` (`VoxLeaderboardArena.astro`) and `vox-gui` (`LeaderboardArenaPanel.tsx`) using the v0 / Claude Design system. | ✓ Good — Astro + React surfaces verified (`Phase 91`) |
| **D32: Dual-Tier 5-Spoke Evaluation — Hermetic CI Contract + Live Safetensors QLoRA Burn-In (`v1.7`)** | Maintain the `<3s` hermetic 25-problem before/mid/after benchmark (`MensBeforeAfterEvaluationV1`) for every CI commit while adding `Phase 97` (`LIVE-QLORA-01..04`) to execute real Candle/QLoRA tensor training (`vox mens train`), materialize `.safetensors` adapter weights + `mens_checkpoint_manifest.json` under `mens/runs/`, and score live token generation across all 25 5-spoke problems. | ✓ Good — 100.0% 5-spoke pass@1 verified (`Phases 93–97`) |
| **D33: Shared-Package CV Flywheel (`@ngi/platform-ssot`) & VS Code-Grade Chat/Dock/Urbs Harness (`v1.8`)** | Sync `@ngi/platform-ssot` (`runUniversalCvFlywheelPass`, `evaluateBeforeAfterCvFlywheel`) into `vox-gui/ui`, unify `Loquela` + `DriveConsole` composer knobs with strict TypeScript types (`kb_scope`, `oversight_level` wired end-to-end), expose `terminal` and `urbs` dock panels with layout presets in `DockWorkspaceShell`, unfreeze embedded `LudusSandbox` with SVG Roman citizen sprites and live `AQVAE` MCP telemetry, and gate on zero-glitch `BEFORE -> AFTER` CV convergence (`Phases 98–101`). | ✓ Complete (`Phases 98–101`) |

## Workstreams (Unified into `unified`)

- **`unified` (Active Default — `.planning/workstreams/unified/`):** Single converged stream hosting `v1.5` (shipped `Phases 01–76`), `v1.6` (shipped `Phases 77–88`), and `v1.7` (active `Phases 89–97`).
- `kb` (Historical source preserved at `.planning/workstreams/kb/`) — v1.1 Vox Knowledge Base
- `discoverability` (Historical source preserved at `.planning/workstreams/discoverability/`) — Phases 06.1–06.6
- `docs-freshness` (Historical source preserved at `.planning/workstreams/docs-freshness/`) — v1.3 & v1.4 OpenWiki Self-Organizing Docs (Phases 20–24, 32–36)
- `autonomy-ux` (Historical source preserved at `.planning/workstreams/autonomy-ux/`) — v1.2 Unified Autonomy Control (Phases 12–19)
- `code-atlas` (Historical source preserved at `.planning/workstreams/code-atlas/`) — v1.4 Code Atlas (Phases 25–31)
- `research-trial-flywheel` (Historical source preserved at `.planning/workstreams/research-trial-flywheel/`) — v1.1 Research Trial Flywheel (Phases 07–11, 12–13)

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-25 after starting v5.4 milestone (`T3 CI/TOESTUB Enforcement Deep-Extraction`) (`Universal Repo-Agnostic Codebase Civilization Simulator & Attention-Weighted Developer Control Surface`)*
