# Project Retrospective

## Milestone: v7.5 — Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence

**Shipped:** 2026-11-15
**Phases:** 7 (`Phases 441–447`) | **Plans:** 29 | **Tasks:** 65 | **Requirements:** 28/28 (`100%`)

### What Was Built
- **Phase 441 (`TERM-PTY-01..04`):** Zero-spam 5-shell `TempDir` shims, persistent stateful `PtyHandle` + `TIOCSWINSZ`/ConPTY resize, 3-byte `Utf8StreamDecoder`, `VtShadow` `ALT_SCREEN`, `after_seq` delta ring, and `vox term --attach`.
- **Phase 442 (`TERM-HARNESS-01..04`):** `vox_terminal_{exec,read,send_input,sessions}` MCP tools, quote-aware POSIX/PS `ExecPolicyV1` + `classify_floor`, non-subshell pager guard + `1,500 ms` `AwaitingInput` stall detection, and 3-stage backend `@term` scrubbing.
- **Phase 443 (`TERM-PANE-01..04`):** Shared `<TerminalSessionPane>` + real 6-addon `@xterm/xterm`, `<16 ms` Auto-TUI switching, `@tanstack/react-virtual` SGR blocks, `react-resizable-panels` splits, and `<ChatTerminalBlockCard>`.
- **Phase 444 (`TERM-WARP-01..04`):** Multi-line `ScriptoriumPromptEditor`, `$PATH`/file/git/CLI-flag `CompletionEngine`, `Ctrl+R` `HistorySearchPopover`, `JsonLedgerTable`, `.vox`/Markdown Runbooks (`Cmd+P` + `DiscoveryRail`), and 1-click `Voxify Subshell`.
- **Phase 445 (`TERM-AGENT-01..04`):** Model-agnostic `vox_actor_runtime::llm` terminal streaming & `Explain / Fix`, inline wax-seal `TerminalApprovalCard`, `Autopilot -> Manual` preemption, live slash commands, and redacted MENS trajectory curation.
- **Phase 446 (`TERM-ROMA-CV-01..04` + `--fix`):** `23/23` `--font-terminal-mono` contract, Dual-Surface `:not(.xterm)` containment, `64`-color WCAG ANSI palettes, `@xterm/xterm@6.0.0` `unicode11CharProperties` cell-width fix, `112/112` Playwright CV convergence (`minVisualAfterScore: 0.9284`, `minVlmOverallScore: 0.9506`), and `warp-roman-terminal-v75-research-2026.md` SSOT promotion.
- **Phase 447 (`BUILD-CONTENTION-01..04`):** Target-first `FairQueue` + flight coalescing, split broker wait metrics, auto-`sccache`, post-queue `VOX_BROKER_RUN_TIMEOUT_SECS`, and `--no-default-features` / `vox-lang` decoupling.

### What Worked
- **Target-Isolated Cargo Directories (`Phase 447`) & `unicode11CharProperties` Root-Cause Diagnosis (`Phase 446`):** Isolating `CARGO_TARGET_DIR` + broker coalescing eliminated `target/.cargo-lock` pileups, while tracing blank `.xterm` canvases in Chromium to `@xterm/xterm@6.0.0`'s `(prop >> 1) & 3` cell-width encoding closed `CR-01` cleanly.
- **Fail-Closed Audit Harnesses (`audits.ts` / `terminalAudits.ts` / `vlmVisionFlywheel.ts`):** Removing silent catches, folding element opacity into WCAG math, and exempting xterm's own `aria-hidden` rows surfaced real clipping and contrast issues at `900×600` and `1100×720` before release.

---

## Milestone: v7.4 — Vox Axis End-to-End Live Control Wiring, Zero-Emoji Icon/Graphic System, shadcn/Tailwind v4 Ecosystem Upgrade & Concept-Art Bento CV Flywheel

**Shipped:** 2026-11-14
**Phases:** 6 (`Phases 431–436`) | **Plans:** 6 | **Requirements:** 24/24 (`100%`)

### What Was Built
- **Phase 431 (`AXIS-SHADCN-01..04`):** Unified `radix-ui@^1.8.1` + Tailwind v4 (`4.3.5`) + `data-slot` primitives (`Button`, `Dialog`, `Popover`, `Select`, `DropdownMenu`, `Tabs`, `Switch`, `Slider`, `RomanRotaryKnob`, `Command`, `Resizable`, `DataTable`, `Chart`) with `:not([data-slot])` CSS escape hatches and deterministic `--z-*` portal stacking across all 4 Style Dictionary themes.
- **Phase 432 (`AXIS-ICONS-01..04`):** Replaced raw Unicode emojis and BMP dingbats across 50+ JSX files and both Canvas 2D renderers with stroke-normalized `lucide-react`, bespoke `Icons.tsx` Romanesque SVGs, Canvas `Path2D` badges (`canvasVectorBadges.ts`), and generated crests/medallions (`zeroEmojiAndDingbat.test.ts` + DOM `rawEmojiCount === 0`).
- **Phase 433 (`AXIS-IPC-HONESTY-01..04`):** Closed Rust/MCP seams (`t3_wave3_tools.rs`, `mercatus.rs`, `calls.rs`), consolidated 100% of frontend `invoke`/`listen` calls through `transport.ts` (`ipcBoundaries.test.ts` allowlists `38 -> 0` and `8 -> 0`), and eliminated all `_demo` paper fallbacks when `backendAvailable() === true`.
- **Phase 434 (`AXIS-LIVE-CONTROLS-01..04`):** Wired 11 live mutation controls with snapshot rollback, empirical DTO hydration on ~65 `<RomanBrassGauge>` dials + `ContextWindowMeter.tsx`, Recharts `<Brush>` scrubbing, and persisted `.v4` workbench layouts (`0` raw `<select>` or `<input type="range">`).
- **Phase 435 (`AXIS-BENTO-01..04`):** Rolled out `@container/surface` `<BentoGrid>` + `MasterDetailWorkbench` splitters across 24 Tier A/B surfaces and replaced generic CLI dumps on `oratio` and `populi` with `OratioStudioView` and `PopuliMeshBentoView`.
- **Phase 436 (`AXIS-CONCEPT-CV-01..04` + `/gsd-code-review --fix`):** Registered 4 dual-theme Bento/Studio concept-art plates, audited open Radix portals across 5 resolutions and 4 themes (`9/9` CV suites converged, `visualAfterScore` `0.9250–0.9665`), passed `live-controls-v74.spec.ts` (`5/5`), and resolved all `19/19` adversarial code review findings (`CR-01..05`, `WR-01..09`, `IN-01..05`).

### What Worked
- **Downward-Only MCP Authorization + Sequence-Guarded Rollbacks (`--fix`):** Fresh-context adversarial review caught caller-widening in `authorize_wave3_call` (`CR-01`), uncontained worktree/external-session paths (`CR-02`), and out-of-order optimistic rollback races (`WR-02`), all closed with unit and Playwright E2E regression tests (`live-controls-v74.spec.ts` Test 5).
- **Imperative `useGroupRef()` Layout Hydration (`MasterDetailWorkbench`):** Synchronizing `react-resizable-panels` layout imperatively instead of remounting via React `key` preserved child scroll/input state across async preference hydration.

---

## Milestone: v6.9 — Build Velocity, Structural DAG Decoupling, Apple Silicon Linker Optimization & MENS Multi-Spoke Corpus/QLoRA Pipeline

**Shipped:** 2026-11-12
**Phases:** 2 (`Phases 405–406`) | **Plans:** 3 | **Requirements:** 11/11 (`100%`)

### What Was Built
- **Phase 405 (`STRUCT-DAG-SPEED-01..05`):** Exposed `vox_repository::core` + Rule-3 inline helpers to sever `vox-repository` from 6 keystones (`vox-compiler`, `vox-db`, `vox-codegen`, `vox-doc-inventory`, `vox-corpus`, `vox-orchestrator-driver`), applied `21` `#[inline(never)]` compilation firewalls, pruned zombie/test-only `[dependencies]` (`vox-cli` `max_workspace_deps` `60 -> 50`), configured macOS `-Wl,-dead_strip` (`dev_loop_guard`), shipped `CargoDepHygieneDetector` (`vox/cargo/dep-hygiene`, hardened for TOML comments and sub-tables), and recorded before/after rebuild proof in `contracts/reports/v68-structural-dag-speedup.v1.json`.
- **Phase 406 (`MENS-CORPUS-406-01..06`):** Upgraded `extract_docs.rs` with breadcrumbs, compiler checks, `resolve_doc_confine_root` relational confinement, and retired `fn ... ) -> ` filtering (`28/28`); added cohesive `.vox` synthesis (`extract_vox`, `10/10`) and fence-aware compiler-verified DPO (`dpo.rs`, `12/12`); enforced `include_lanes` across all 5 spoke mixes (`spoke_validate --features mens`, `9/9`); implemented `token_norm_window_scale`, `build_packed_mask_and_doc_ids`, and `forward_packed`; exposed `--lora-exclude-lm-head` and `--use-rslora` (`effective_lora_alpha` verified & logged); and enforced relative `corpus_threshold` paths (`5/5`).

### What Worked
- **Dead-Code Warning & Pre-Existing Test Surfacing During `--fix`:** Running targeted crate unit tests immediately surfaced both the unused `resolve_doc_confine_root` helper in `extract_docs.rs` (preventing `../` traversal on relational `.vox` links) and the retired `fn ... ) -> ` return check in `corpus_readiness.rs`, plus TOML `[dependencies.<pkg>]` sub-table bypasses in `cargo_dep_hygiene.rs`.

---

## Milestone: v6.8 — Physical Multi-Repo Extraction, Default Cargo Graph Pruning, Next-Tier Keystone Splits, `crates.io` Wave 2/3 (`33` Crates), Rich Roman Visual READMEs, Public Rollout Path Portability & Federated Mirror Auto-Sync

**Shipped:** 2026-11-12
**Phases:** 11 (`Phases 394–404`) | **Plans:** 13

### What Was Built
- **Phases 394–395 & 398 (`vox-ops`, `vox-axis`, `voxlang.org` Federation):** Initialized `../vox-ops` (`vox-plugin-cloud`, `vox-ops-telemetry-ingest`) and standalone entrypoints `../vox-axis` and `../voxlang.org`; isolated default Cargo resolution (`default-members = ["crates/vox-cli"]`); verified `4/4` present repositories in `vox ci cross-repo-parity` and `vox ci federated-symbol-graph`.
- **Phases 396–397 & 399 (Keystone Firewalls & `crates.io` `33/33`):** Applied `#[inline(never)]` compilation firewalls across `vox-orchestrator`, `vox-compiler`, `vox-db`, and 11 second-tier crates; expanded `crates/_public.toml` to `33` standalone crates (`vox ci publish-dry-run` clean); published `contracts/reports/v68-defactoring-flywheel-scorecard.v1.json`.
- **Phases 400–402 (Roman Visual READMEs, Audit & `voxlang.org` Portal):** Updated 50 `v6.6` Sunlit Roman White/Gold Playwright captures, 7 `.webp` tours, and 7 raster diagrams; audited all 46 READMEs (`0` broken links) and upgraded `docs/src/index.mdx` with Light/Dark theme comparisons (`0` `axeViolations`, `919` Astro pages).
- **Phase 403 (`ABS-DIR-GUARD-01..04`):** Scrubbed machine-local hardcoded absolute paths across scripts, GUI harnesses/reports, tests, contracts, and 17 `docs/src/` files; shipped `vox/fs/hardcoded-abs-dir` (`toestub`), `drift/hardcoded-abs-dir` (`vox-drift-check`, `CACHE_VERSION = 3`), and `lefthook.yml` pre-commit gating.
- **Phase 404 (`FEDERATION-SYNC-01..04`):** Shipped `sync_federated_satellites` (`vox ci cross-repo-parity --sync`), error-propagated `sync_dir_checked`, canonicalized sibling fallback `resolve_external_satellite_root`, and `6/6` `federation_tests`.

### What Worked
- **Canonicalized External Sibling Probing + `sync_dir_checked`:** Testing `sync_federated_satellites` inside a symlinked tempdir caught macOS `/var` -> `/private/var` `starts_with` mismatches before sibling mirroring could silently skip external checkouts.

---

## Milestone: v6.4 — Full-Canvas White/Gold Carrara Marble Unification, Non-Procedural Monument Simulator, Single-Line Roman Controls & Zero-Monospace Typography

**Shipped:** 2026-11-08
**Phases:** 5 (`Phases 373–377`) | **Plans:** 5

### What Was Built
- **Phase 373:** Enlarged `ORCHESTRATOR` circular hub medallion (`r=66` in `AgentFlow.tsx`) and widened `ORCHESTRATOR` DAG node (`width=136` in `NeedsYouSurface.tsx`), made `OPEN FORM`, `EXPLAIN CAPABILITY`, `OPEN DOCS` single-line (`ExplorerTree.tsx`, `Button.tsx`), and slimmed `= GENERIC FORM` (`CapabilityBadge.tsx`).
- **Phase 374:** Replaced procedural 2D polygon building drawing in `worldRenderer.ts` with painted Roman Acropolis & Monument artwork (`urbs-white-gold-citadel-bg.jpg` + `monument-*.jpg` portraits in `LudusSandbox.tsx`), live active file/folder badges, and animated SVG golden aqueduct streams.
- **Phase 375:** Converted `Backdrop.tsx`, `Glass.tsx`, and `index.css` across all 32 surfaces to the Sunlit White/Cream Carrara Marble & Burnished Gold aesthetic with subtle Vitruvian architectural watermark and sculpted bottom Roman frieze.
- **Phase 376:** Remapped `--font-mono`, `--font-family-mono`, `.font-mono`, `pre`, `code`, and all SVG/Canvas font declarations to `'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif` (`tabular-nums`) and `'Cinzel', 'Palatino', Georgia, serif`.
- **Phase 377:** Verified all 32 surfaces across `1280×800`, `1440×900`, and `1920×1080` via Playwright + `@ngi/platform-ssot` CV Flywheel (`0` `axeViolations`, `0` `clippedInteractiveCount`, `0` `overlappingControlPairs`), built-in Gemini vision inspection, and `/gsd-code-review --fix`.

### What Worked
- **Root-Token + `<Glass>` Unification:** Updating `:root` color tokens in `index.css` together with `<Glass>` (`Glass.tsx`) and `Backdrop.tsx` transformed all 32 surfaces simultaneously instead of patching components one by one.
- **Replacing Procedural Canvas Polygons with Painted Diorama + Monument Portraits:** Removing `paintFullBleedRomanLandscape` and `drawIsoFileBuilding` allowed the high-resolution painted Roman Acropolis and monument portraits to shine through while keeping live file/folder badges and golden aqueduct streams interactive.

---

## Milestone: v4.7 — Universal Repo-Agnostic Codebase Civilization Simulator & Attention-Weighted Developer Control Surface

**Shipped:** 2026-10-25
**Phases:** 6 (`Phases 259–264`) | **Plans:** 6

### What Was Built
- **Phase 259:** Polyglot Repository Scanner (`workspace_town.rs`) supporting Cargo workspaces, npm/pnpm/TS monorepos, Go (`cmd/*`, `internal/*`, `pkg/*`), Python (`src/<pkg>`), and single-app trees + live Git working-tree diff telemetry (`+LoC / -LoC`, git status, churn).
- **Phase 260:** Zero-hardcoding topological graph analyzer (`topology.ts`), Kahn's algorithm + Tarjan SCCs layer calculation, structural wonder archetype classifier, and total elimination of hardcoded `vox-*` crate names.
- **Phase 261:** Attention-weighted dynamic real-estate layout (`layout.ts`), scaling modules `1.0x`–`3.2x` by developer priority (active edits, uncommitted diffs, errors, blast radius), `>= 65%` foreground worksite allocation, and Focus Mode switcher.
- **Phase 262:** Live file diff slabs with proportion bars, diagnostic siege cards with 1-click dispatch (`[Fix Error]`, `[Re-run Test]`, `[Open File]`), and the top-left Live Developer Priority Dock (`data-testid="urbs-priority-dock"`).
- **Phase 263:** Dependency blast-radius illumination on the canvas (Upstream Providers in Teal, Downstream Blast Radius in Amber/Crimson) and the repo-agnostic `ImperiumInspector.tsx` control panel.
- **Phase 264:** Multi-repo simulation presets (`live`, `ts_monorepo`, `python_ml`, `go_microservices`), Preset & Focus Mode dropdowns, and comprehensive verification across Vitest, TypeScript, `vox-code-audit`, `vox-arch-check`, and `vox-doc-pipeline`.

### What Worked
- **Contract-Driven Test Strategy:** Writing comprehensive mock repository fixtures for Go, Python, and TypeScript monorepos in both Rust unit tests and TypeScript Vitest tests before implementation ensured complete polyglot coverage.
- **Topological Invariant Formulation:** Using pure directed graph properties (in-degree, out-degree, betweenness, cycle components) decoupled visualization from language idioms while naturally discovering architecture layers (`L0..L3`).

### What Was Inefficient
- Discovered archetype precedence issues between explicit domain keywords and topological fan-in heuristics during unit testing; explicitly ordering domain keyword rules before topological fallbacks resolved this cleanly.

### Patterns Established
- **Attention Multiplier Normalization:** Sizing visualization items by `attentionMultiplier` allows complex simulations to scale gracefully across any repository size by keeping dormant code compact and active work prominent.

---

## Milestone: v5.7 — Vox ↔ Python Bridge, Tolerant CLI, Docs Parity/SSOT, Gated Qwen3 Training & MENS Crucible Studio

**Shipped:** 2026-11-13
**Phases:** 12 realized (`Phases 324–331, 333, 342–343, 345` across `324–333, 342–345`) | **Plans:** 35

### What Was Built
- **Phases 324–326 (`EFF`, `BR`, `GOV`, `ARR`, `K-01`):** Capability effect gates, sandboxed uv-locked `vox-python-bridge` worker, and `std.array` on numpy.
- **Phases 327–329 (`FRM`, `PLT`, `TNS`, `PYX`, `K-01`):** `std.frame` on polars, `std.plot` PNG output & `std.tensor` `.pt2` inference, and Tier 2 `import py <alias> from "<module>"` escape hatch with lock-stamped `.vox.pyi` sidecars, opaque `PyObject` typechecking, `Denied`-policy guards, hashed `python.escape_call` telemetry, `py_tier1_calls` / `py_tier2_calls` syntax-K counters, `vox py bind`, and `py_*` ladder/budget registration.
- **Phases 330–331, 333 (`DBI`, `CLI-TOL`, `REL`):** `vox db introspect` + external schema lock, deterministic CLI recovery + precomputed `semantic-aliases.v1.json`, and gated Qwen3 merged QLoRA releases.
- **Phases 342–343, 345 (`DOCS-PARITY`, `DOCS-DRIFT`, `MENS-STUDIO`):** Astro `7.4.5` / Starlight `0.42.9` + fatal link validation, automated docs drift gates, and Vox Axis MENS Crucible Studio.

### What Worked
- Keeping Tier 1 (`std.array`, `std.frame`, `std.plot`, `std.tensor`) and Tier 2 (`import py ... from "..."`) syntactically and diagnostically separated without growing `LEXER_KEYWORDS` or breaking `python_bridge_parity_test` allowed rapid integration of `py_escape_psd` alongside all ladder and complexity-budget checks.

---

## Milestone: v5.6 — Reasoning Where the Lane Needs It

**Shipped:** 2026-11-13 (Audited & closed from Phases `313–323`)
**Phases:** 11 (`Phases 313–323`) | **Plans:** 22 | **Requirements:** 25/25 (`100%`)

### What Was Built
- **Phases 313–316 (`LANE-01..02`, `BENCH-01..03`, `GATE-01..02`):** Enforced non-thinking prefixes on codegen spokes vs. bare thinking prefixes on hub/agent spokes, pinned held-out GSM8K/MATH-500/Vox slices, and fail-closed `vox mens serve` collateral gating.
- **Phases 317–321 (`SIZE-01..03`, `MESH-01`, `HARN-01`, `BENCH-04`):** Pinned all 6 Qwen3 dense sizes, wired live VRAM & mesh-wide size selection, executed multi-size Metal QLoRA training, and published before/after harness reports.
- **Phases 322–323 (`DEC-01..04`, `CAT-01..02`, `MDX-INGEST-01..03`, `MDX-CONV-01..03`):** Plumbed calibrated turn decisions + ChatARC shortlist routing and first-class Starlight `.mdx` ingestion across search, facets, doc pipeline, and MENS corpus extraction.

---

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
| **v1.7** | 2026-10-19 | 9 | 18 | Centralized Postgres Telemetry, Private `vox-ops` Extraction, Ludus 3-Way SoC, Zero-Cheat Leaderboard, Codebase/History Scrub, Live Helios Deployment & 5-Spoke MENS Safetensors Evaluation |
| **v6.4** | 2026-11-08 | 5 | 5 | Full-Canvas White/Gold Carrara Marble Unification, Painted Monument Simulator & Zero-Monospace Typography |
| **v6.7** | 2026-11-10 | 7 | 19 | Monorepo Defactoring, Zero Dev Cycles, Keystone Splits, `crates.io` Tier 1–3, Sub-15s Pre-Push & Worktree Consolidation |
| **v6.8** | 2026-11-12 | 11 | 13 | Physical Multi-Repo Extraction, Keystone Firewalls, `crates.io` Wave 2/3 (`33` Crates), Roman Visual READMEs, Public Path Portability & Federated Mirror Auto-Sync |
| **v5.6** | 2026-11-13 | 11 | 22 | Reasoning Where the Lane Needs It |
| **v5.7** | 2026-11-13 | 12 | 35 | Vox ↔ Python Bridge, Tolerant CLI, Docs Parity/SSOT, Gated Qwen3 Training & MENS Crucible Studio |
| **v6.9** | 2026-11-12 | 2 | 3 | Build Velocity, Structural DAG Decoupling, Apple Silicon Linker Optimization & MENS Multi-Spoke Corpus/QLoRA Pipeline |
