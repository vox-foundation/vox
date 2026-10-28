# Project Retrospective

## Milestone: v1.7 — Centralized Postgres Telemetry, Private `vox-ops` Extraction, Ludus 3-Way SoC, Zero-Cheating Leaderboard, Codebase/History Scrub, Live Helios Deployment & 5-Spoke MENS Safetensors Evaluation

**Shipped:** 2026-10-19
**Phases:** 9 (`Phases 89–97`) | **Plans:** 18

### What Was Built
- **Phase 89:** 26-variant strongly-typed telemetry (`vox-telemetry`), `VoxDb::audit_research_metric_shapes` (`TypedResearchMetricRow`), batched OTLP HTTP uploader (`<= 100` records / `<= 256 KiB`) + `vox telemetry preview`, removal of prototype `server/telemetry/`, and multi-surface model router verification across CLI, daemon, MCP, and GUI.
- **Phase 90:** Private 6-crate `~/dev/vox-ops` workspace (`vox-ops-db`, `vox-ops-ingest`, `vox-ops-syndication`, `vox-ops-ludus`, `vox-ops-intelligence`, `vox-ops-server`) and three-role fenced PostgreSQL 17 (`vox_ops_owner`, `vox_ingest_app`, `vox_ops_app`) on live `helios-postgres` (`q40ck888ggkkcsosgcwgc0oo`), verified with privilege vector `t|f|f|t|f|f|t`.
- **Phase 91:** 9-adapter social syndication extraction (`vox-ops-syndication` + `SyndicationHandoffPayloadV1`), 3-way Ludus SoC split (`ops_agent_telemetry.rs` in `vox-db`, multi-user Ludus in `vox-ops-ludus`, solo progression + FSRS offline in `vox-gamify`), elimination of shadow telemetry (`hint_telemetry.rs`, `lsp_telemetry.rs`), and zero-cheating `BLAKE3` + `Ed25519` `VerifiedLeaderboardSnapshotV1` surfaces on `voxlang.org` (`VoxLeaderboardArena.astro`) and `vox-gui` (`LeaderboardArenaPanel.tsx`).
- **Phase 92:** Ed25519-signed $k \ge 20$ `CommunityIntelligenceBundleV1` + Empirical Bayes prior blending (`\pm 15\%` clamp) in `vox-orchestrator`, live `vox-ops-server` deployment on `helios` (`127.0.0.1:8420`), and working-tree/history scrub contracts (`contracts/ci/moved-data-scrub-manifest.v1.json`, `contracts/ci/git-history-scrub.v1.json`, `scripts/verify-history-scrub.vox`).
- **Phases 93–97:** 5-spoke MENS gap closure, 25-problem before/after benchmark (`24.0% -> 72.0% -> 100.0% pass@1`), Axis harness `ModelRegistry` routing, `MensTrainingView.tsx` scorecard, and live `.safetensors` QLoRA evaluation.

### What Worked
- **Three-Role PostgreSQL Privilege Fencing (`D23`):** Verifying the exact 7-column privilege vector (`t|f|f|t|f|f|t`) live on `helios-postgres` proved that the public ingest pool (`vox_ingest_app`) can never `SELECT` from raw telemetry tables (`telemetry_ingest.events`).
- **Static Signed Snapshots for Offline Sovereignty (`D25`, `D31`):** Both `VerifiedLeaderboardSnapshotV1` and `CommunityIntelligenceBundleV1` use `Ed25519` signatures and `#[serde(deny_unknown_fields)]` with zero penalty when offline or disabled.

---

## Milestone: v1.5 — Unified Holistic Stream

**Shipped:** 2026-10-16
**Phases:** 74 | **Plans:** 241

### What Was Built
- **Wave 0 (`Phase 05`):** Unified all 6 parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and 42 git worktrees onto `feat/unified-2026-q4` without deleting any existing worktrees.
- **Wave 1 (`Phases 12, 07, 07.1, 25, 32`):** Action-only Autonomy Safety Floor (`classify_floor`, 0/34 surviving mutants), `trials` domain at VoxDb schema baseline 97, `vox-graph-reader` v5 (`AtlasQuery` 400-node/800-edge LOD), and 100% docs trust signal calibration (`810/810 verified`).
- **Wave 2 (`Phases 33–34, 13–17, 37–42, 46, 49, 52–53, 55–56, 58, 60–62`):** OpenWiki Subsystem Hubs & deep crate API pages, `OversightLevel` (`1..5`) + `AttentionInbox` at DB baseline 98, local Metal/CPU KB embeddings + `@kb` context injection + `kb_findings` inbox + Top-25 rewrites + staleness detector, Research Trial Race Harness, `vox-term` CLI attach + `TermDispatch::run_agent_command` + redacted `"term.context"`, Nushell + Vox-Shell structured pipelines, Deep Research + SCIENTIA publication pipeline closure, autonomous Mermaid validation & LLM self-repair, continuous `README.md`/`voxlang.org` sync, Open Collective + grant pack, Tier 2.5 `VOX_GIT_HASH` codegen fallback, and 7-crate standalone `crates.io` unbundling.
- **Wave 3 (`Phases 06.4, 26–31, 43–45, 18–19, 47–48, 50–51, 54, 57`):** Capability Explorer (`ExplorerView.tsx`, 17 domains, 4 status badges, 4-tab coverage audit, `gui-coverage-gate` CI gate with 294/294 CLI families and 349/349 MCP tools covered or audited), Code & Architecture Atlas GUI, KB Library & Context Inspector GUI, `Oversight: 1..5` composer control + `autonomy-claims.v1.yaml` ledger, Axis Console Warp-Parity WebGL terminal, Axis Harness River, bidirectional Chat ↔ Terminal co-execution (`L1..L5` security levels), Unified 7-source IntelliSense, Scientia Publication Studio GUI, and `voxlang.org` brand/playground/chrome overhaul.
- **Wave 4 (`Phases 35–36, 06.6, 59, 63`):** Constrained LLM doc claim verifier, guarded self-healing docs draft-PR bot, 5 cold-start time-to-find journeys (`<= 60s`, `<= 3` interactions), full 34-mutant safety floor gate + `/gsd-code-review --fix` + 22/22 `audit-open` items resolved, and `N=100` `.vox` vs. TypeScript empirical benchmark preprint + `CR-L8` flywheel.
- **Wave 5 (`Phases 64–69`):** Polyglot `@tool` & `@form` auto-surface, `Stream[T]` SSE + `Channel[Send, Recv]` WebSockets + reactive `db.Table.subscribe` + managed `ProcessRun`, `uses { Spend, Write, ExternalApi }` algebraic effect guards + `<WaitingNotice>` + `@ranker` gates, `@embed`/`@searchable` hybrid search + partial `db.Table.patch` + `@table(scoped_by: tenant)`, presigned `Upload[T]`/`BlobStore` + `@webhook` verification + `@rate_limit`/`@metered` credit ledger, and deterministic tick/skill engine reference template (`vox init --kind=simulation-rpg`).
- **Wave 6 (`Phases 70–76`):** Worktree Land Wave (`land/w1..w5`) & Nightly-Green Closure + `vox fmt` `layout_only` guard, Clavis Next Parts 1 & 2 (`clavis://` URI resolver + zero-trust `vox_secrets::broker`), Apple Silicon unified-memory Qwen 3.8 27B & hybrid GatedDeltaNet linear-attention multi-token decode in `vox-mens`, `PopuliHttpOp` over iroh QUIC + durable A2A mailbox, MENS 2026 `.vox` Script Writer overhaul + 5-tier SOTA `vox chat` routing, and humane zero-markup `TurnCostReceipt` + anti-FOMO `RestDayPolicy` + `FatigueMonitor` UX.

### What Worked
- **Single-Stream Wave Sequencing (`D15`–`D20`):** Converging 6 parallel workstreams and 42 worktrees into one dependency-ordered roadmap eliminated cross-branch schema digest collisions (`VoxDb 95 -> 96 -> 97 -> 98`) and shared React file conflicts (`Loquela.tsx`, `ChatSurface.tsx`, `Sidebar.tsx`).
- **Contract-First & Ratchet-Driven Gates:** Removal-only ratchets (`gui-coverage-exemptions.yaml`, `kb-facet-backlog.v1.yaml`, `baseline.v1.json`, `capability-index`) and mutation-tested safety floors (`0/34` surviving mutants) kept regressions impossible as surface area grew to 996 CLI rows and 349 MCP tools.

### What Was Inefficient
- Parallel worktrees created before Wave 0 required manual catalog and schema digest reconciliation in `Phase 05` and `Phase 70`. Keeping all execution in the single unified worktree eliminated that overhead for Waves 1–6.

### Patterns Established
- **Three-Layer Capability Coverage Gate (`06.4`):** Every CLI leaf family and MCP tool must be either GUI-wired, explicitly exempted with a valid reason enum, or backed by a valid `.vox` island in `crates/vox-gui/vox/islands/`, enforced in `vox ci ssot-drift`.
- **Pre-Close `audit-open` Zero-Debt Gate:** Resolving all `deferred-items.md` and `.planning/todos/pending/` items before closing the milestone ensures zero silent technical debt carries over.

### Key Lessons
- Sequence shared database schema baselines (`VoxDb`) and shared GUI shell files (`Loquela.tsx`, `ChatSurface.tsx`, `Omnibar.tsx`) explicitly in the roadmap rather than across concurrent branches.
- Pair every generated TypeScript/JSON registry with a lightweight lazy-loader (`capabilityIndexLoader.ts`) so cold-start bundle size never regresses.

---

## Cross-Milestone Trends

| Milestone | Shipped | Phases | Plans | Key Theme |
|-----------|---------|--------|-------|-----------|
| **v1.0** | 2026-09-30 | 6 | 23 | Architectural Housekeeping & Dead-Crate Disposal |
| **v1.3** | 2026-10-13 | 6 | 33 | Self-Maintaining Public Docs & Deterministic Drift Gate |
| **v1.5** | 2026-10-16 | 74 | 241 | Unified Holistic Stream (Waves 0–6: KB, Discoverability, OpenWiki, Autonomy, Atlas, Trials, Terminal, Scientia, Runtime, Clavis, Mens, Mesh, Humane UX) |
| **v1.6** | 2026-10-16 | 12 | 25 | Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel |
| **v1.7** | 2026-10-19 | 9 | 18 | Centralized Postgres Telemetry, Private `vox-ops` Extraction, Ludus 3-Way SoC, Zero-Cheating Leaderboard, Codebase/History Scrub, Live Helios Deployment & 5-Spoke MENS Safetensors Evaluation |
