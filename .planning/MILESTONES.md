# Milestones

## v2.0 Persistent Organic Multi-Spoke Search-to-Compilation Benchmark, Live Execution Oracles & Longitudinal Evaluation Flywheel (Shipped: 2026-10-19)

**Phases completed:** 4 phases (`Phases 108–111`), 4 plans, 16/16 requirements verified (`100%`)
**Archive:** [`milestones/v2.0-ROADMAP.md`](milestones/v2.0-ROADMAP.md) | [`milestones/v2.0-REQUIREMENTS.md`](milestones/v2.0-REQUIREMENTS.md)

**Key accomplishments:**

- **15-Task 5-Spoke Organic Search-to-Compilation Suite & Decontamination Guard (`Phase 108`):** Shipped `canonical_organic_search_to_compilation_suite_15` (`3` organic developer search-to-compilation tasks per spoke across `VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), `discover_workspace_evidence`, `verify_organic_benchmark_decontamination`, and `contracts/reports/mens-organic-search-spoke-scorecard.v1.json` (`100.0%` Recall@5, `1.000` MRR, `100.0%` Grounding, `100.0%` Compilation, `100.0%` E2E `pass@1`).
- **Deep Compilation & Functional Runtime Oracles Across All 5 Spokes (`Phase 109`):** Upgraded `verify_deep_compilation_for_spoke` with `vox_compiler` (`lex -> parse -> HIR -> typeck -> fmt -> Interpreter::run_module`) for `VoxLang`, `syn::parse_file` + `quote::ToTokens` + policy detectors for `RustWorkspace`, live directory inventory verification for `StructuredShell`, `vox-mcp-registry` + inline `vox_compiler::pipeline::check_file` (`0` errors) for `McpToolChain`, and `TaskPlanEnvelope` file existence + `classify_shell_safety_floor` gate verification for `AgentHarness`.
- **Longitudinal `VoxDb` Persistence, `ModelBenchCase` Telemetry & Multi-Metric Ratchets (`Phase 110`):** Shipped `persist_organic_scorecard_to_db`, `load_organic_scorecard_from_db`, `list_organic_scorecard_history_from_db`, `verify_organic_scorecard_non_decreasing_trend`, `record_organic_scorecard_telemetry_to_db` (`vox.model.bench_case` verified via `VoxDb::audit_research_metric_shapes`), and expanded `contracts/eval/spoke-regression-ratchet.v1.json` + `verify_organic_search_ratchet` in `vox-audit`.
- **CLI & Axis GUI Longitudinal Scorecard Surfaces (`Phase 111`):** Added `vox model eval --organic-search-spokes` (`execute_organic_search_spoke_evaluation_in_dir`), `OrganicSearchSpokeScorecardDto` in `vox-gui` (`mens_run_reports.rs`), and `data-testid="mens-organic-scorecard"` in `MensTrainingView.tsx`.

---

## v1.9 Chat Default Surfaces CV & Code Audit Remediation, Progressive-Disclosure Composer, Rich Markdown Transcript, Full-Height Execution Rail & CV Flywheel Convergence (Shipped: 2026-10-19)

**Phases completed:** 4 phases (`Phases 104–107`), 4 plans, 14/14 requirements verified (`100%`)

---

## v1.8 Shared-Package CV Flywheel, Chat/Loquela Knob & Datatype Unification, VS Code Multi-Panel Docking, Urbs Civilization Live Taps, Browser Songhand/Playwright Harness & `ngi.video` Marketing Copy/Screenshot CV Flywheel Promotion (Shipped: 2026-10-19)

**Phases completed:** 6 phases (`Phases 98–103`), 6 plans, 32/32 requirements verified (`100%`)

---

## v1.7 Centralized Postgres Telemetry, Private `vox-ops` Extraction, Ludus 3-Way SoC, Zero-Cheating Leaderboard, Codebase/History Scrub, Live Helios Deployment & 5-Spoke MENS Safetensors Evaluation (Shipped: 2026-10-19)

**Phases completed:** 9 phases (Phases 89–97), 18 plans, 70/70 requirements verified (`100%`)

**Key accomplishments:**

- **Telemetry Surface Audit Remediation, Strong Typing, Multi-Surface Model Router & Local `VoxDb` Shape Audit (`Phase 89`):** Wired `init_process_telemetry` across `vox-orchestrator-d` and `vox-gui`, expanded `ResearchMetricsSink` to all 26 `TelemetryEvent` variants with `VoxDb::audit_research_metric_shapes` (`TypedResearchMetricRow`), implemented batched OTLP HTTP uploader `upload_pending_otlp` (`<= 100` records / `<= 256 KiB`) + `vox telemetry preview`, removed prototype `server/telemetry/`, and verified the model router across CLI, daemon, MCP, and GUI surfaces.
- **Private `vox-ops` Repository & Three-Role Fenced Postgres on Live `helios-postgres` (`Phase 90`):** Bootstrapped the 6-crate `~/dev/vox-ops` workspace (`vox-ops-db`, `vox-ops-ingest`, `vox-ops-syndication`, `vox-ops-ludus`, `vox-ops-intelligence`, `vox-ops-server`) and provisioned the three-role fenced PostgreSQL 17 database (`vox_ops_owner`, `vox_ingest_app`, `vox_ops_app`) on live `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo`), verified with privilege vector `t|f|f|t|f|f|t` and $k \ge 20$ `SECURITY DEFINER` analytical views.
- **Social Media Syndication, Multi-User Ludus Extraction, 3-Way Gamification SoC Split & Zero-Cheating Leaderboard Surfaces (`Phase 91`):** Extracted all 9 live social media adapters to `vox-ops-syndication` (`syndication.outbox`), added `SyndicationHandoffPayloadV1` (`contracts/scientia/syndication-handoff.v1.schema.json`), moved Tier 1 operational `agent_telemetry` (`agent_events`, `cost_records`, `agent_sessions`) to `crates/vox-db/src/store/ops_agent_telemetry.rs`, eliminated shadow telemetry (`hint_telemetry.rs`, `lsp_telemetry.rs`), extracted multi-user Ludus to `vox-ops-ludus`, and shipped `VerifiedLeaderboardSnapshotV1` (`BLAKE3` receipt hash chain + `Ed25519` envelope signature) with `VoxLeaderboardArena.astro` (`voxlang.org`) and `LeaderboardArenaPanel.tsx` (`vox-gui`).
- **Community Intelligence Flywheel, Live Helios Deployment & Git History Scrub (`Phase 92`):** Shipped `CommunityIntelligenceBundleV1` (`#[serde(deny_unknown_fields)]`, `Ed25519` verification via `vox_crypto::verify_signature_hex`, Empirical Bayes prior blending clamped to $\pm 15\%$ in `ModelRegistry::apply_community_intelligence_bundle`), `vox telemetry intelligence {status,sync,inspect}` CLI + GUI Settings controls, deployed container `vox-ops-server` live on `helios` (`127.0.0.1:8420`, verified live `GET /healthz`, `POST /v1/logs` -> `HTTP 202 Accepted` as `vox_ingest_app`, `GET /v1/intelligence/manifest.json`, and `GET /v1/ludus/leaderboard`), and verified working-tree/history scrub compliance via `scripts/verify-history-scrub.vox`.
- **5-Spoke MENS Gap Closure, 25-Problem Benchmark, Axis Harness Routing, GUI Scorecard & Live QLoRA Safetensors Evaluation (`Phases 93–97`):** Closed `88-REVIEW.md` findings (`MENS-GAP-01..04`), expanded the 5-spoke benchmark to 25 problems (`24.0% -> 72.0% -> 100.0% pass@1`), wired `ModelRegistry::register_calibrated_mens_adapter` + `route_spoke_task` + `vox model eval --mens-five-spokes`, added `MensSpokeScorecardDto` in `MensTrainingView.tsx`, and verified live `.safetensors` adapter checkpoints across all 25 problems (`LIVE-QLORA-01..04`).

---

## v1.6 Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel (Shipped: 2026-10-16)

**Phases completed:** 12 phases (Phases 77–88), 25 plans, 100% requirements verified

**Key accomplishments:**

- **Multi-Spoke Training Hygiene & Round-Trip Verification (`Phases 77–78`):** Shipped schema-validated `TrainingRecordV2` across all 5 Vox spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`), 3-gram Jaccard deduplication + holdout decontamination, `VoxDb` Trusty dataset snapshots (`trusty://vox-dataset/<hash>`), and the deterministic in-process 5-spoke round-trip harness emitting `MensSpokeScorecardV1` (`1.0` pass@1 across all 5 spokes).
- **Core Daily Driver Stability, Daemon Auto-Bootstrap & Gateway/IPC Parity (`Phase 79`):** Added supervised `vox-orchestrator-d` auto-bootstrap, health probes (`orch.ping`), and exponential-backoff reconnect across `vox-gui`, `vox chat`, and `vox term`; wired all remaining unrouted gateway/daemon RPCs in `contracts/gui/command-backends.v1.yaml`; migrated `gui_db_pool.rs` to daemon-owned `VoxDb` RPCs; and added inline PTY/daemon reconnect recovery in Axis Console (`Console.tsx` / `TerminalTab.tsx`).
- **Real-Backend E2E Verification Harness (`Phase 80`):** Built the hermetic real-backend test harness (`crates/vox-gui/tests/real_backend_harness.rs` + `crates/vox-gui/ui/e2e/real-backend/`) booting a live `vox-orchestrator-d` + temporary `VoxDb` + HTTP/IPC Tauri command bridge, verified Playwright E2E journeys (`Chat`/`Loquela`, `Terminal` `SessionHost`, `⌘K` `Palette`, `Explorer`, `Library`), and enforced `honestyScan.ts` across all 32 GUI surfaces.
- **`.vox` Language, Compiler, Interpreter & Codegen End-to-End Parity (`Phase 81`):** Verified `vox check` and `vox run` across `scripts/*.vox`, all `vox init` templates (`default`, `simulation-rpg`, `script`), span-accurate `vox check --for-llm` diagnostics, and full execution parity across `--interp`, `RustAxum`, `RustTauri`, and `TypeScript`.
- **Operational & Data-Intensive GUI Surface Hardening (`Phases 82–83`):** Live-wired `SettingsView` (`Clavis`, OAuth, `Oratio`), `ModelsView` (`Mens`), `ApprovalsView`, `PoliciesView`, `SubAgentsView`, `MemoryView` (`Library`/Inbox/Health), `ResearchView`, `ScientiaSurface`, `CodeAtlasCanvas`, `MeshView`, and ring-buffered `Dashboard`/`Runs`/`Tasks`/`Activity`/`Gamify` event streams; ratcheted CLI `side_effect_class` (`906 -> 647 <= 756`) and dedicated GUI surfaces (`12 -> 16 >= 16`).
- **Workspace CI Gate Parity, `vox doctor --fix` & Mens Continuous Flywheel Closure (`Phases 84–88`):** Shipped `vox doctor --fix` auto-remediation, curriculum-weighted QLoRA + DPO batch pipeline (`floor 0.10`), longitudinal 5-spoke scorecards and promotion gates (`vox-eval`), audit-to-corpus harvesting + verified repair DPO pairing + spoke regression ratchets (`vox-audit`), and the 2-cycle autonomous burn-in (`pass@1` `0.40 -> 0.80 -> 1.00`, `0` spoke regressions) persisted in `VoxDb` CAS.

---

## v1.5 Unified Holistic Stream (Shipped: 2026-10-16)

**Phases completed:** 74 phases (Waves 0–6, Phases 01–76), 241 plans, 100% requirements verified

**Key accomplishments:**

- **Wave 0 (`Phase 05` — Convergence):** Converged all 6 parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and 42 git worktrees onto `feat/unified-2026-q4` with zero worktree deletions and zero `ssot-drift` or `crate-edges` violations.
- **Wave 1 (`Phases 12, 07, 07.1, 25, 32` — Safety, Trial & Graph Floors):** Shipped the action-only Autonomy Safety Floor (`permission-modes.v2.yaml` + `classify_floor`, 0/34 surviving mutants), `trials` domain at VoxDb schema baseline 97 (`TrialCampaignId` Trusty URI, atomic persistence with `PersistFailpoint`, code-gen benchmark provenance), `vox-graph-reader` v5 (`EXTRACTOR_VERSION = "5"`, `AtlasQuery` 400-node/800-edge LOD budget), and 100% docs trust signal calibration (`810/810 verified`, `0 drifting`, `0 broken`, `0 unanchored`).
- **Wave 2 (`Phases 33–34, 13–17, 37–42, 46, 49, 52–53, 55–56, 58, 60–62` — Core Backend Engines):** Delivered OpenWiki Subsystem Hubs & deep crate API pages with validated Mermaid diagrams, consolidated 29 duplicate doc clusters (`954 -> 7` baseline deficits), shipped `OversightLevel` (`1..5`) + `AttentionInbox` at DB baseline 98, local Metal/CPU KB embeddings + `@kb` context injection + `kb_findings` inbox + Top-25 page-one rewrites + staleness detector, Research Trial Race Harness (`vox-eval::trial_flywheel`), `vox-term` CLI attach + `TermDispatch::run_agent_command` + redacted `"term.context"`, LLM-first Nushell + Vox-Shell structured pipelines, Deep Research + SCIENTIA publication pipeline closure, autonomous Mermaid validation & LLM self-repair (`vox_validate_mermaid`), continuous `README.md`/`voxlang.org` sync, Open Collective + NLnet/Rust/Compute grant pack, Tier 2.5 `VOX_GIT_HASH` codegen fallback, and 7-crate standalone `crates.io` unbundling (`crates/_public.toml`).
- **Wave 3 (`Phases 06.4, 26–31, 43–45, 18–19, 47–48, 50–51, 54, 57` — Sequenced GUI & `voxlang.org` Unification):** Built the Capability Explorer (`ExplorerView.tsx`, 17 domains, 4 status badges, 4-tab coverage audit, `gui-coverage-gate` CI gate with 294/294 CLI families and 349/349 MCP tools covered or audited), Code & Architecture Atlas (`CodeAtlasCanvas.tsx` + diff overlay + chat dock + click-to-`@`), KB Library (`Library`, per-turn Context Inspector drawer, Inbox/Health tabs), `Oversight: 1..5` composer control + `autonomy-claims.v1.yaml` ledger, Axis Console Warp-Parity WebGL terminal (`TerminalTab.tsx` + `BlockStreamView.tsx`), Axis Harness River (`HarnessRiverPanel.tsx` + 4-scenario Playwright dogfood suite), bidirectional Chat ↔ Terminal co-execution (`L1..L5` security levels + `@term` chips), Unified 7-source IntelliSense (`intellisenseEngine.ts` + `IntelliSensePopover.tsx`), Scientia Publication Studio GUI (`ResearchDagCanvas.tsx`), and the `voxlang.org` brand overhaul, interactive playground, and collapsible sidebars (`36/36` Playwright tests).
- **Wave 4 (`Phases 35–36, 06.6, 59, 63` — Self-Healing Automation & Verification):** Shipped the constrained LLM doc claim verifier (`llm_verifier`), guarded self-healing docs draft-PR bot (`self_heal_bot`), 5 cold-start time-to-find journeys (`<= 60s`, `<= 3` interactions), full 34-mutant safety floor gate + milestone-wide `/gsd-code-review --fix` + resolution of all 22 `audit-open` items, and the `N=100` `.vox` vs. TypeScript empirical benchmark preprint + `CR-L8` diagnostic→repair→corpus flywheel.
- **Wave 5 (`Phases 64–69` — Cross-Portfolio Runtime Completeness):** Delivered polyglot `@tool` & `@form` auto-surface (`<ToolForm />`, CLI, MCP, OpenAPI 3.1), `Stream[T]` SSE + `Channel[Send, Recv]` WebSockets + reactive `db.Table.subscribe` + managed `ProcessRun`, algebraic `uses { Spend, Write, ExternalApi }` effect guards + `<WaitingNotice>` + `@ranker`/`@eval_shadow` ordering gates, `@embed`/`@searchable` hybrid search + partial `db.Table.patch` + `@table(scoped_by: tenant)`, presigned `Upload[T]`/`BlobStore` + `@webhook` verification + `@rate_limit`/`@metered` credit ledger, and the deterministic tick/skill engine reference template (`vox init --kind=simulation-rpg`).
- **Wave 6 (`Phases 70–76` — Worktree Land Wave, Clavis Next, Mens ML, Populi Mesh, SOTA Routing & Humane UX):** Harvested `land/w1..w5` and closed all 12 tasks of the Nightly-Green plan + `vox fmt` `layout_only` token guard, shipped Clavis Next Parts 1 & 2 (`clavis://` URI resolver, `vox secrets import/acquire/probe/rotate/sync`, zero-trust `vox_secrets::broker` with opaque `clavis-handle://` handles), integrated Apple Silicon unified-memory Qwen 3.8 27B + hybrid GatedDeltaNet linear-attention multi-token decode in `vox-mens`, ported `PopuliHttpOp` + durable A2A mailbox onto `vox-mesh-transport` iroh QUIC with fail-closed dispatch, unified MENS 2026 `.vox` script writer prompts/generators with 5-tier SOTA `vox chat` routing (`free`, `auto`, `efficient`, `balanced`, `genius`), and shipped humane zero-markup `TurnCostReceipt`, anti-FOMO `RestDayPolicy`, anti-sycophancy directness, and `FatigueMonitor` session wrap-up UX.

---

## v1.3 Self-Maintaining Public Docs (Shipped: 2026-10-13)

**Phases completed:** 6 phases (Phases 20, 20.1, 21, 22, 23, 24), 33 plans, 32 verified

**Key accomplishments:**

- Restored `voxlang.org` Cloudflare Pages deployments with fast local staging, added single-issue failure escalation and `ci-liveness` checks, fixed git-derived page dates across symlinked content, added frontmatter status banners and `noindex` for non-current pages, and isolated research/roadmap pages into the `Internals` sidebar section.
- Shipped collapsible left hover-rail and right table-of-contents sidebars with responsive wide-article layout on `voxlang.org`, verified by 12/12 Playwright visual-stepper tests across mobile, tablet, and desktop viewports in light and dark modes.
- Built deterministic claim extraction (`vox-doc-pipeline::claims`), `vox audit docs` CLI command, machine-populated Docs Reality Audit findings, and the corpus prune report (`prune-report.v1.json`) auditing all 972 doc pages.
- Shipped the code-anchored freshness ledger (`ledger.v1.json`), tighten-only baseline ratchet (`baseline.v1.json`), expiring suppressions, PR impact reverse-index comment (`vox ci pr-impact`), and executable `.vox` tutorial test harness (`TUT-02`).
- Generated `env-vars.generated.md`, `config-keys.generated.md`, `mcp-tools.generated.md`, and 127 crate API documentation pages (`docs/src/api/*.generated.md`) under `ssot-drift`, plus CLI `build_catalog()` compliance verification and retired-decorator prose checks.
- Delivered reader trust banners and sidebar badges driven by `ledger.v1.json`, consolidated `llms.txt` and `/llms-full.txt`, committed the 10-case golden drift evaluation set (`golden-drift-eval-set.v1.json`), and ratified the Decision D9 in-house build verdict (`docs-bot-evaluation-and-pivot-rule-2026.md`).

---

## v1.0 Architectural Housekeeping (Shipped: 2026-09-30)

**Phases completed:** 6 phases, 23 plans, 11 tasks

**Key accomplishments:**

- D-03 verification pass confirms all 10 REQ-dead-crate-delete crates and their prerequisite type/logic migrations already landed; repointed the stale all-features CI matrix entry; operator ratified SC#3's actual host-crate disposition.
- Verified catalog.toml's execution-api/stub-check ghost-entry bookkeeping is already accurate and added a KEEP-FROZEN disposition comment for the three frozen crates, closing ROADMAP Phase 1 Success Criterion 4.
- Ratified ADR-045 as Accepted across all three GSD artifacts (doc body, intel classification, decisions index) and recorded ADR-037's desktop-convergence clause as implemented with three cited, verified code paths — zero code changes, pure documentation ratification.
- Audited vox-gui's command surface (clean — no violations), recorded the finding and one documented exemption in the surface-ownership contract, wrote the Vox-native/React authoring boundary in a machine-readable field plus matching prose, and fixed the one reference page still naming a deleted crate as the canonical GUI.
