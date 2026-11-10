# Roadmap: Vox Unified Workstream

# Roadmap: v6.8 — Physical Multi-Repo Extraction (`~/dev/vox-ops`, `vox-axis`, `voxlang.org`), Default Cargo Graph Pruning, Next-Tier Keystone Splits (`vox-orchestrator`, `vox-compiler`, `vox-db`), `crates.io` Wave 2/3 (`33` Crates), Rich Roman Visual READMEs & Adversarial Cross-Repo README Audit (`Phases 394–401`)

**Milestone:** v6.8
**Status:** Complete (`8/8` Phases, `32/32` Requirements Verified — 2026-11-10)
**Approved Architectural & Engineering Direction:**

1. **Physical Multi-Repo Extraction (`Phases 394–395, 398`):** Initialize `/Users/brbrainerd/dev/vox-ops` as a real standalone Git + Cargo workspace repository (hosting `vox-plugin-cloud` and `vox-ops-telemetry-ingest` + synced wire/IPC/telemetry contracts), create standalone sibling workspace entrypoints `/Users/brbrainerd/dev/vox-axis` and `/Users/brbrainerd/dev/voxlang.org`, and verify all **`4/4` repositories live on disk** via `vox ci cross-repo-parity` and `vox ci federated-symbol-graph` with zero split-brain.
2. **Next-Tier Keystone Build-Time Splits (`Phase 396`):** Apply `#[inline(never)]` compilation firewalls and domain submodule boundaries to `vox-orchestrator` (`119s`), `vox-compiler` (`55.6s`), `vox-db` (`43.5s`), `vox-populi` (`26.9s`), `vox-publisher` (`19.1s`), and `vox-scientia` (`14.7s`).
3. **Standalone `crates.io` Wave 2 & 3 Expansion (`15` → `33` Crates, `Phase 397`):** Unlock 18 additional standalone crates in `crates/_public.toml` and `contracts/distribution/profiles.v1.yaml`, verified `33/33` by `vox ci publish-dry-run`.
4. **End-to-End Flywheel Scorecard (`Phase 399`):** Publish `contracts/reports/v68-defactoring-flywheel-scorecard.v1.json` and `docs/src/architecture/monorepo-to-multirepo-flywheel-ssot-2026.md`, and complete `/gsd-code-review --fix` (`399-REVIEW.md`).
5. **Multi-Repo Rich Visual READMEs, Animated `.webp` Tours, Real Generated Raster Graphics, Cross-Repo Agent Awareness & `vox-foundation` GitHub Federation (`Phase 400`):** Author comprehensive visual READMEs for `vox-axis`, `voxlang.org`, `vox-ops`, and `vox` with all 32 GUI screens explained, 7 animated `.webp` tours, real generated `.webp` architecture diagrams (zero SVGs), Coolify/Cloudflare bindings, cross-repo `AGENTS.md` sync rules, and public GitHub pushes under `vox-foundation`.
6. **Adversarial Cross-Repo & Sub-Crate README Audit, `v6.6` Sunlit Imperial Roman Carrara White & Gold + `VRBS ROMA` Simulator Visual Refresh, `README-SYNC-MAP` Maintainability & Footnoted Prior-Art Attribution (`Phase 401`):** Replace all legacy Sept 4 dark GUI screenshots and navy diagrams with the current `v6.6` (Nov 9) Sunlit Imperial Roman Carrara White & Gold Playwright captures (`50` PNGs including `VRBS ROMA` 3D/2.5D Simulator `urbs-civilization.png` + `urbs-imperium-inspector.png`, `AQVAEDUCTUS` hydraulic cascade `harness.png`, Multi-Dock Chat `chat-multidock.png`, and 4-profile theme contract `theme-arcane.png`/`theme-slate.png`), regenerate all 7 `.webp` tours and all 7 architectural schematics in the Roman Carrara White & Gold aesthetic (converting the last `.svg` `schema-fragmentation.svg` to `schema-fragmentation.webp`), audit and update all 8 stale sub-crate/app `README.md`s inside `vox/crates/*` and `vox/apps/*`, enforce single-source domain ownership (`README-SYNC-MAP`), and add footnoted prior-art citations (`[^1]`..`[^13]`) crediting every source of ideas.

### Phase 394: Physical Private Backend & Telemetry Repository (`/Users/brbrainerd/dev/vox-ops`) (`VOX-OPS-REPO-01..04`) `[WAVE 1A]`
- **Directory:** `.planning/phases/394-physical-vox-ops-repository-extraction/`
- **Goal:** Create `/Users/brbrainerd/dev/vox-ops` as a standalone Git + Cargo workspace with `vox-plugin-cloud`, `vox-ops-telemetry-ingest`, `AGENTS.md`, and synchronized `contracts/{wire,ipc,telemetry}/` so `vox ci cross-repo-parity` and `vox ci federated-symbol-graph` verify `vox-ops` live on disk while `vox` remains 100% standalone/offline.

### Phase 395: `vox-axis` & `voxlang.org` Physical/Workspace Separation & Default Cargo Graph Pruning (`WORKSPACE-PRUNE-01..04`) `[WAVE 1B]`
- **Directory:** `.planning/phases/395-vox-axis-voxlang-org-and-plugin-workspace-pruning/`
- **Goal:** Create standalone sibling entrypoints `/Users/brbrainerd/dev/vox-axis` and `/Users/brbrainerd/dev/voxlang.org`, and prune heavy non-core surface/plugin crates from default Cargo resolution.

### Phase 396: Next-Tier >35s Keystone Splits (`vox-orchestrator`, `vox-compiler`, `vox-db`) (`KEYSTONE-V68-01..04`) `[WAVE 1C]`
- **Directory:** `.planning/phases/396-next-tier-keystone-splits-orchestrator-compiler-db/`
- **Goal:** Add `#[inline(never)]` compilation firewalls across `vox-orchestrator`, `vox-compiler`, and `vox-db` hot paths to cut incremental check times by `>= 40%`.

### Phase 397: Standalone `crates.io` Wave 2 & 3 Expansion (`15` → `33` Crates) (`CRATES-IO-V68-01..04`) `[WAVE 1D]`
- **Directory:** `.planning/phases/397-standalone-crates-io-wave-2-21-crates/`
- **Goal:** Add 18 standalone crates to `crates/_public.toml` and `profiles.v1.yaml` and pass `vox ci publish-dry-run` (`33/33`).

### Phase 398: Live 4-Repo Federation Spine (`vox`, `vox-ops`, `vox-axis`, `voxlang.org`) (`FEDERATION-LIVE-01..04`) `[WAVE 2A]`
- **Directory:** `.planning/phases/398-live-4-repo-federation-and-llm-awareness/`
- **Goal:** Verify `4/4` present repositories in `vox ci cross-repo-parity` and `vox ci federated-symbol-graph` (`0` skipped optional repos) and synchronize `~/dev/AGENTS.md` + per-repo `AGENTS.md`.

### Phase 399: End-to-End Multi-Turn Flywheel Scorecard (`v6.6` → `v6.7` → `v6.8`) (`FLYWHEEL-BENCH-01..04`) `[WAVE 2B]`
- **Directory:** `.planning/phases/399-end-to-end-flywheel-scorecard-and-ssot/`
- **Goal:** Publish `contracts/reports/v68-defactoring-flywheel-scorecard.v1.json` and `docs/src/architecture/monorepo-to-multirepo-flywheel-ssot-2026.md`, pass `/gsd-code-review --fix` (`399-REVIEW.md`), and verify all CI gates pass.

### Phase 400: Multi-Repo Rich Visual READMEs, Animated `.webp` Tours, Real Generated Raster Diagrams, Cross-Repo Agent Awareness, Coolify & `vox-foundation` GitHub Federation (`README-VISUAL-01..04`) `[WAVE 3]`
- **Directory:** `.planning/phases/400-multi-repo-visual-readmes-and-github-federation/`
- **Goal:** Ensure `vox-axis` (`README.md` + `crates/vox-gui/README.md`) contains a rich visual tour with real Playwright screenshots, every one of the 32 GUI screens + 3 shell modes + 5 HITL stages explained (why it exists, what it can do), 7 looping animated `.webp` tours, and real generated `.webp` architecture diagrams (no SVGs); author matching READMEs for `voxlang.org`, `vox-ops`, and `vox`; wire Coolify (`eval.voxlang.org`, `telemetry.voxlang.org`) and Cloudflare Pages (`voxlang.org`) + cross-repo `AGENTS.md` sync rules; and push public repositories to GitHub under `vox-foundation` (`vox-foundation/vox`, `vox-foundation/vox-axis`, `vox-foundation/voxlang.org`).

### Phase 401: Adversarial Audit of All READMEs Across All Repositories & Sub-Crates, `v6.6` Sunlit Imperial Roman Carrara White & Gold + `VRBS ROMA` Simulator Visual Refresh, `README-SYNC-MAP` Maintainability & Footnoted Prior-Art Attribution (`README-AUDIT-01..04`) `[WAVE 4]`
- **Directory:** `.planning/phases/401-adversarial-readme-audit-and-roman-visual-sync/`
- **Goal:** Adversarially audit all 46 `README.md` files across `vox-foundation/vox`, `vox-foundation/vox-axis`, `vox-foundation/voxlang.org`, and `vox-ops`; replace stale Sept 4 dark screenshots in `vox-axis/assets/screens/` and `crates/vox-gui/ui/review-bundle/latest/` with the 50 Nov 9 (`v6.6`) Sunlit Imperial Roman Carrara White & Gold Playwright captures (including `VRBS ROMA` 3D/2.5D Simulator `urbs-civilization.png` + `urbs-imperium-inspector.png`, `AQVAEDUCTUS` hydraulic cascade `harness.png`, Multi-Dock Chat `chat-multidock.png`, and 4-profile theme contract `theme-arcane.png` / `theme-slate.png`); regenerate all 7 animated `.webp` tours and all 7 architectural diagrams in the `v6.6` Roman White/Gold aesthetic (replacing `schema-fragmentation.svg` with `schema-fragmentation.webp` in `vox/README.md` and `docs/src/index.mdx`); fix all 8 stale sub-crate/app READMEs inside `vox/crates/*` and `vox/apps/*`; enforce `README-SYNC-MAP` single-source ownership to eliminate cross-repo table duplication; and add footnoted prior-art attribution (`[^1]`..`[^13]`).

---

# Roadmap: v6.7 — Monorepo Defactoring, Build-Time & Crate-Edge Disentanglement, Standalone `crates.io` Ecosystem, Zero-Split-Brain Cross-Repo LLM Federation, End-to-End CI/Nightly Remediation & Zero-Loss Worktree/Scratch Consolidation (`Phases 387–393`)

**Milestone:** v6.7
**Status:** Complete (`7/7` Phases, `28/28` Requirements Verified — 2026-11-10) — Adversarially Audited via `/gsd-review` ([`MILESTONE-v6.7-REVIEWS.md`](MILESTONE-v6.7-REVIEWS.md))
**Approved Architectural & Engineering Direction:**

1. **Empirical Build-History & Churn-Driven Defactoring (`Phases 387–389`):** Target the exact crates proven by `contracts/ci/crate-build-map.v1.json` and 90-day git commit churn to dominate developer wait time:
   - Eliminate all **5 dev-dependency back-edge loops** across both SCC entries in `contracts/ci/dep-backedges.allow.json:1-32` (`vox-test-harness <-> vox-compiler`, `vox-db -> (dev) {vox-compiler, vox-codegen}`, `vox-codegen -> (dev) vox-workflow-runtime`, `vox-compiler -> (dev) {vox-codegen, vox-codegen-ts}`, and `vox-config-derive -> (dev) vox-config`) so `"allowed_cycles": []` (`0` cycles) is enforced by `vox ci dep-cycles --deny-new` and the shared `586s` `--all-targets` blast radius collapses.
   - Split the #1 slowest crate (`vox-orchestrator-mcp`: `157.9s` self-compile, `102,145` LoC, `708` 90d touches) over an L3 base context crate `vox-mcp-context` (`ServerState`, `ToolResult`, `params`, `workspace_path`) into `vox-mcp-tools-{core,scientia,ops}` (breaking up the 258-arm single-function async state machine in `dispatch.rs:663-2380` and 100 `derived_tool_schema!` expansions in `input_schemas.rs:44-1425` while updating `mcp_wiring.rs:34-71`), and complete `crates/vox-orchestrator-models` Part B4 extraction from `vox-orchestrator` (`119.0s` self-compile, `109,091` LoC; decoupling `models/auto_select.rs:9-11`, preserving `TaskCategory` wire order, and updating the 3 hardcoded CI path readers).
   - Narrow high-fan-in blast keystones: `vox-db` (`43.5s`, `28` direct fan-in → prune unused `vox-lsp`/`vox-langtool` edges, move `content_hash` to existing `vox-db-types`, add calibration trait seam), `vox-populi` (`26.9s`, `42,020` LoC → rewire deprecated `vox_populi::transport` callsites + extract `mens/cloud` & `mens/hub_card`), and `vox-cli` (`47.4s`, `109,091` LoC, `55+` workspace deps → extract `vox-cli-model`, `vox-cli-db`, `vox-cli-diagnostics` + consolidate remaining `~25K+` LoC of `commands/ci/` into `vox-cli-ci` + invert `vox-ml-cli -> vox-cli`).
2. **Standalone `crates.io` Ecosystem (`Phase 390`):** Fix all crate-escaping `include_str!("../../../contracts/...")` and `workspace-hack` (`publish = false`) packaging blockers across Tiers 1–3, decouple `vox-code-audit` (`toestub`) from its 11 unpublished internal workspace dependencies, and register Tier 3 standalone crates (`vox-diagram`, `vox-similarity`, `vox-rule-pack`, `vox-code-audit`) in `crates/_public.toml`, gated by `vox ci publish-dry-run`.
3. **Hybrid Multi-Repo + Segmented Workspaces with Zero-Split-Brain Cross-Repo LLM Spine (`Phase 391`):** Keep `vox` 100% standalone/offline-capable (`vox ci standalone-sovereignty`); isolate `vox-axis` (`crates/vox-gui`, already excluded from `default-members` at `Cargo.toml:23`) and `voxlang.org` (`docs-astro`) from core check/test loops; and extend existing `vox-repository::repo_catalog` (`.vox/repositories.yaml` at `crates/vox-repository/src/repo_catalog/mod.rs:29`) with `vox graph refresh --federated`, `vox ci cross-repo-parity`, and `vox dev link-repos` (Cargo `[patch]` toggle).
4. **Sub-15s Path-Scoped Pre-Push, PR Workflow Consolidation & End-to-End Green Nightly Proof (`Phase 392`):** Add `--ci` to `vox ci pre-push` (used by `.github/workflows/ci.yml:65`), cut local default `vox ci pre-push` to `<15s` (skipping the 10s `status::print_live_for_push()` network stall, fixing `step_line_endings` at `pre_push.rs:1118`, path-scoping `fast` checks, and updating `lefthook.yml:77` + `ssot_drift.rs:137-200` in lockstep), consolidate `compile-matrix.yml` & `cr-l8-corpus-feedback.yml` into `ci.yml`, and remediate all 7 failing scheduled Nightly workflows with live `workflow_dispatch` green run proof (`contracts/reports/nightly-e2e-run-proof.v1.json`).

### Phase 387: 20-Crate & 2-Crate Dev-Dependency Cycle Elimination & Churn-Weighted Build-Time Measurement Spine (`CYCLE-CUT-01..04`) `[WAVE 1]`

**Directory:** `.planning/phases/387-dev-dep-cycle-elimination-and-build-pain-spine/`
**Goal:** Eliminate all 5 dev-dependency back-edge loops across both SCC entries in `contracts/ci/dep-backedges.allow.json:1-32` (`allowed_cycles: []`), collapse the shared `586s` `--all-targets` test blast radius, and extend `vox_graph_reader::crate_model::build_crate_summary` with `touches_90d` and `churn_compile_pain_score` in `contracts/ci/crate-build-map.v1.json`.
**Depends on:** Phase 386
**Requirements:** `CYCLE-CUT-01`, `CYCLE-CUT-02`, `CYCLE-CUT-03`, `CYCLE-CUT-04`
**Parallel & Model Dispatch:**

- `Plan 387-01` (`[PARALLEL-SAFE]`, `Flash / Argon`): Decouple `vox-test-harness` from `vox-compiler` (`spans.rs:5` → `vox_ast::span::Span`, move compiler helpers to `vox-compiler` test support) and `vox-config` (`barriers.rs:26`).
- `Plan 387-02` (`[PARALLEL-SAFE]`, `Flash / Argon`): Relocate `crates/vox-db/tests/arca_compiler_table_parity.rs`, `crates/vox-codegen/tests/{durability_compiles,adr021_parity}.rs`, and `crates/vox-config-derive/tests/*` to cut `vox-db -> (dev) {vox-compiler, vox-codegen}`, `vox-codegen -> (dev) vox-workflow-runtime`, and `vox-config-derive -> (dev) vox-config`.
- `Plan 387-03` (`[SEQUENTIAL]`, `5.5 Sonnet / Pro`): Relocate the 17 codegen test/bench files from `crates/vox-compiler/Cargo.toml:60-61`, set `"allowed_cycles": []` in `contracts/ci/dep-backedges.allow.json`, enforce `vox ci dep-cycles --deny-new`, and add `touches_90d` + `churn_compile_pain_score` to `crates/vox-graph-reader/src/crate_model.rs:198`.

**Success Criteria:**

1. `vox-test-harness` drops `vox-compiler` from `crates/vox-test-harness/Cargo.toml:14`, becoming a pure `L1` utility (`EnvScratch`, `synthetic_workspace`, `temp_root`, `portpicker`, `spans`).
2. All 5 dev-dep back-edge loops are severed, `contracts/ci/dep-backedges.allow.json` has `"allowed_cycles": []` (`0` cycles), and `vox ci dep-cycles --deny-new` exits `0`.
3. Running `cargo test -p <L1/L2-crate>` no longer triggers compilation of unrelated L3 keystones, reducing median `--all-targets` blast radius across L2 crates by `>= 50%`.
4. `contracts/ci/crate-build-map.v1.json` records `touches_90d` and `churn_compile_pain_score` alongside `compile_s` and `blast_s` without breaking `vox ci crate-build-map-parity` or `vox ci crate-budget`.

### Phase 388: Top Compile-Time Keystone Splits — `vox-orchestrator-mcp` (`158s`, `102K` LoC) & `vox-orchestrator-models` (`119s`, `109K` LoC) (`KEYSTONE-ORCH-01..04`) `[WAVE 2A]`

**Directory:** `.planning/phases/388-keystone-orchestrator-mcp-and-models-splits/`
**Goal:** Cut the #1 (`vox-orchestrator-mcp`, `157.9s`, `102,145` LoC) and #2 (`vox-orchestrator`, `119.0s`, `109,091` LoC) workspace compile bottlenecks by extracting an L3 `vox-mcp-context` base crate + domain MCP tool crates (breaking up the 258-arm `dispatch.rs` state machine and 100-macro `input_schemas.rs`) and completing `crates/vox-orchestrator-models` Part B4.
**Depends on:** Phase 387
**Requirements:** `KEYSTONE-ORCH-01`, `KEYSTONE-ORCH-02`, `KEYSTONE-ORCH-03`, `KEYSTONE-ORCH-04`
**Parallel & Model Dispatch:**

- `Plan 388-01` (`[SEQUENTIAL]`, `5.5 Sonnet / Pro`): Extract `crates/vox-mcp-context` (`ServerState`, `ToolResult`, `params.rs`, `workspace_path.rs`, `profiles.rs`) and update `crates/vox-cli/src/commands/ci/command_compliance/mcp_wiring.rs:34-71`.
- `Plan 388-02a..c` (`[PARALLEL-SAFE]`, `Flash / Argon`): Split `vox-mcp-tools-core`, `vox-mcp-tools-scientia`, `vox-mcp-tools-ops` (plus `input_schemas.rs` shards and `gui_ops`/`chat_tools` separation) over `vox-mcp-context`.
- `Plan 388-03` (`[PARALLEL-SAFE]`, `5.5 Sonnet / Pro`): Complete `crates/vox-orchestrator-models` Part B4 (lower `TaskPriority`/`PricingSource`/`ClutchProfile` to `vox-orchestrator-types`, decouple `models/auto_select.rs:9-11` from `vox-populi`, keep `TaskCategory` wire-compatible in `vox-orchestrator-models`, and update `model_routing_check.rs:76`, `timeout_literal.rs:79`, `validators.rs:504`).

**Success Criteria:**

1. `vox-mcp-context` (L3) owns `ServerState` and `params.rs`, enabling a cycle-free DAG (`vox-mcp-context` → `vox-mcp-tools-{core,scientia,ops}` → `vox-orchestrator-mcp`), and `vox-orchestrator-mcp` incremental compile time drops below `60s` with 100% `vox ci command-compliance` & `vox ci command-sync` parity.
2. `crates/vox-orchestrator-models` owns `models/*`, `route_policy.rs`, and `catalog*.rs` with zero wire-format regression on `TaskCategory` (`PascalCase` serde + `Ord` discriminant order) and all 3 CI path readers updated.
3. `docs/src/architecture/layers.toml` and `contracts/ci/crate-budget.v1.json` enforce updated LoC and compile-time caps.
4. All 4 surfaces (CLI, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-gui`) pass model-router and MCP tool integration tests without regression.

### Phase 389: Blast-Radius & Surface Keystones — `vox-db-types`, `vox-populi` `SP3`/`SP7` & `vox-cli` Subcommand Crates (`KEYSTONE-DB-CLI-01..04`) `[WAVE 2B]`

**Directory:** `.planning/phases/389-keystone-db-populi-and-cli-splits/`
**Goal:** Narrow the fan-in and compile times of `vox-db` (`43.5s`, `28` fan-in), `vox-populi` (`26.9s`, `42,020` LoC), and `vox-cli` (`47.4s`, `109,091` LoC, `55+` deps) by pruning dead `vox-db` edges + expanding `vox-db-types`, rewiring deprecated `vox_populi::transport` + splitting `mens/cloud` & `mens/hub_card`, and extracting `vox-cli-{model,db,diagnostics}` + consolidating `commands/ci/*` into `vox-cli-ci`.
**Depends on:** Phase 387, Phase 388
**Requirements:** `KEYSTONE-DB-CLI-01`, `KEYSTONE-DB-CLI-02`, `KEYSTONE-DB-CLI-03`, `KEYSTONE-DB-CLI-04`
**Parallel & Model Dispatch:**

- `Plan 389-01` (`[PARALLEL-SAFE]`, `Flash / Argon`): Remove dead `vox-db` deps from `vox-lsp` and `vox-langtool`, move `hash::content_hash` to `vox-db-types`, and add calibration trait seam for `vox-orchestrator-models`.
- `Plan 389-02` (`[PARALLEL-SAFE]`, `5.5 Sonnet / Pro`): Rewire `vox-orchestrator` (`remote_worker.rs:289`, `mesh_federation_poll.rs:84`, `task_submit.rs:613`) off deprecated `vox_populi::transport` (`crates/vox-populi/Cargo.toml:19-21`) and extract `mens/cloud` + `mens/hub_card`.
- `Plan 389-03a..c` (`[PARALLEL-SAFE]`, `Flash / Argon` with `5.5 Sonnet / Pro` integration on `commands/mod.rs`): Extract `vox-cli-model` (`18.7K` LoC), `vox-cli-db` (`9.4K` LoC), `vox-cli-diagnostics` (`7.7K` LoC), consolidate `~25K+` LoC of `commands/ci/` into `vox-cli-ci`, and invert `vox-ml-cli -> vox-cli` (`layers.toml:296-299`).

**Success Criteria:**

1. `vox-lsp`, `vox-langtool`, `vox-package` (normal dep), and `vox-orchestrator-models` drop direct `vox-db` edges, reducing `vox-db` direct fan-in in `contracts/ci/fan-in-snapshot.v1.json`.
2. Deprecated `vox_populi::transport` callsites are rewired to `vox-plugin-populi-mesh` and `mens/cloud` + `mens/hub_card` are separated, reducing `vox-populi` self-compile time by `>= 35%`.
3. `vox-cli` delegates model, database, diagnostic, and CI helper subcommands to `vox-cli-{model,db,diagnostics,ci}`, reducing `crates/vox-cli/src/` LoC from `109,091` to `< 50,000` and eliminating the `vox-ml-cli -> vox-cli` layer inversion.
4. `vox ci command-sync`, `vox ci crate-edges`, and `vox audit arch` pass with zero layer inversions.

### Phase 390: Standalone `crates.io` Ecosystem Expansion & Topological Publish Dry-Run Gate (`CRATES-IO-01..04`) `[WAVE 3A]`

**Directory:** `.planning/phases/390-standalone-crates-io-ecosystem-and-publish-gate/`
**Goal:** Eliminate all `include_str!` / `workspace-hack` / unpublished transitive dependency blockers and verify standalone `crates.io` publication for Tier 3 crates (`vox-diagram`, `vox-similarity`, `vox-rule-pack`, `vox-code-audit`/`toestub`) alongside Tier 1/2 `crates/_public.toml` crates, gated by `vox ci publish-dry-run`.
**Depends on:** Phase 389
**Requirements:** `CRATES-IO-01`, `CRATES-IO-02`, `CRATES-IO-03`, `CRATES-IO-04`
**Parallel & Model Dispatch:**

- `Plan 390-01` (`[PARALLEL-SAFE]`, `Flash / Argon`): Vendor crate-local copies of `policy.yaml`, `profiles.v1.yaml`, and `rules.v1.yaml` for `include_str!` (`vox-scaling-policy/src/lib.rs:24`, `voxup/src/profiles.rs:7`, `vox-code-audit/src/embedded_rules.rs:7`), guard `vox-diagram` doc-parity `include_str!` tests, strip `workspace-hack` from publishable crates (`.config/hakari.toml` `final-excludes`), inline `vox-build-meta` in `vox-arch-check/build.rs`, and add `version = "0.6.0"` + metadata/READMEs to `vox-diagram`, `vox-similarity`, and `vox-rule-pack`.
- `Plan 390-02` (`[SEQUENTIAL]`, `5.5 Sonnet / Pro`): Decouple `vox-code-audit` (`toestub`) from its 11 unpublished internal workspace deps (`vox-config`, `vox-llm-egress`, `vox-orchestrator-types`, `vox-http-client`, `vox-jsonschema-util`, `vox-telemetry`, etc.), update `crates/_public.toml` + `crates/voxup/tests/distribution_parity.rs`, and implement `vox ci publish-dry-run`.

**Success Criteria:**

1. Zero publishable crates in `crates/_public.toml` depend on `workspace-hack` or reference `include_str!("../../...")` paths outside their crate root.
2. `vox-diagram`, `vox-similarity`, and `vox-rule-pack` compile and pass `cargo package --allow-dirty` as standalone libraries with complete crates.io metadata and `README.md`.
3. `vox-code-audit` (`toestub` binary + library) depends only on published Tier 1/3 crates (`vox-rule-pack`, `vox-scaling-policy`, `vox-bounded-fs`, `vox-secrets`) and passes `cargo package --allow-dirty`.
4. `vox ci publish-dry-run` verifies topological `cargo package --list --allow-dirty` and `cargo package --allow-dirty` across all `publish = true` crates in `crates/_public.toml`.

### Phase 391: Hybrid Multi-Repo & Workspace Segmentation + Zero-Split-Brain Cross-Repo LLM & Contract Spine (`REPO-FEDERATION-01..04`) `[WAVE 3B]`

**Directory:** `.planning/phases/391-hybrid-multi-repo-and-cross-repo-llm-federation/`
**Goal:** Verify 100% offline/standalone `vox` sovereignty vs. `~/dev/vox-ops`, isolate `vox-axis` (`vox-gui`) and `voxlang.org` (`docs-astro`) from core check/test loops, and extend existing `vox-repository::repo_catalog` (`.vox/repositories.yaml`) with `vox graph refresh --federated`, `vox ci cross-repo-parity`, and `vox dev link-repos`.
**Depends on:** Phase 389, Phase 390
**Requirements:** `REPO-FEDERATION-01`, `REPO-FEDERATION-02`, `REPO-FEDERATION-03`, `REPO-FEDERATION-04`
**Parallel & Model Dispatch:**

- `Plan 391-01` (`[PARALLEL-SAFE]`, `Flash / Argon`): Implement `vox ci standalone-sovereignty` (`VOX_OFFLINE=1 VOX_TELEMETRY=off`) and core-only Cargo/nextest aliases (`--exclude vox-gui`) + standalone workspace scripts for `vox-gui` and `docs-astro`.
- `Plan 391-02` (`[SEQUENTIAL]`, `5.5 Sonnet / Pro`): Extend `crates/vox-repository/src/repo_catalog/` (`.vox/repositories.yaml` at `mod.rs:29`) with `vox graph refresh --federated`, `vox ci cross-repo-parity`, and `vox dev link-repos` (gracefully skipping absent optional sibling checkouts like `~/dev/vox-ops`), and synchronize `~/dev/AGENTS.md` + per-repo `AGENTS.md`.

**Success Criteria:**

1. `vox ci standalone-sovereignty` proves `vox`, `vox-gui`, and `vox-orchestrator-d` build, test, and execute end-to-end with `VOX_OFFLINE=1` and `VOX_TELEMETRY=off` and zero network or compile dependency on `~/dev/vox-ops`.
2. Core Cargo/nextest aliases exclude `vox-gui` by default (complementing `default-members = ["crates/vox-cli"]` at `Cargo.toml:23`), and both `vox-axis` (`crates/vox-gui`) and `voxlang.org` (`docs-astro`) have standalone workspace scripts.
3. `.vox/repositories.yaml`, `vox ci cross-repo-parity`, and `vox dev link-repos` verify wire/IPC contract digests across registered repositories and toggle Cargo `[patch]` local development cleanly.
4. `vox graph refresh --federated` indexes registered repositories so existing MCP tools (`vox_repo_catalog_list`, `vox_repo_catalog_refresh`, `vox_repo_query_text`, `vox_graphify_search`, `vox_search_query`) resolve cross-repo symbols without split-brain.

### Phase 392: Sub-15s Path-Scoped `vox ci pre-push`, PR Workflow Consolidation, `~/dev/local-ci` Integration & End-to-End Green Nightly Proof (`CI-NIGHTLY-PROOF-01..04`) `[WAVE 1 & WAVE 4]`

**Directory:** `.planning/phases/392-ci-cd-splitting-pre-push-speedup-and-nightly-proof/`
**Goal:** Add `--ci` to `vox ci pre-push` (used in `ci.yml:65`), cut local default `vox ci pre-push` execution to `<15s` via strict path scoping, consolidate duplicate PR workflows (`compile-matrix.yml`, `cr-l8-corpus-feedback.yml`), update `~/dev/local-ci`, and remediate all 7 scheduled Nightly workflows with live `workflow_dispatch` end-to-end green run proof.
**Depends on:** Independent Wave 1 start (`392-01..04`); final proof (`392-06`) runs after Wave 4
**Requirements:** `CI-NIGHTLY-PROOF-01`, `CI-NIGHTLY-PROOF-02`, `CI-NIGHTLY-PROOF-03`, `CI-NIGHTLY-PROOF-04`
**Parallel & Model Dispatch:**

- `Plan 392-01` (`[PARALLEL-SAFE]`, `5.5 Sonnet / Pro`): Add `--ci` flag to `pre_push.rs` + `.github/workflows/ci.yml:65`, skip 10s `status::print_live_for_push()` (`pre_push.rs:209`), fix `step_line_endings` arg bug (`pre_push.rs:1118`), path-scope `fast` profile checks (`check_fmt`, `check-links`, `spoke-check`, workflow guards, doc facet/Mermaid checks, `vox-drift-check`), and update `lefthook.yml:77` + `lefthook_retry_failures` (`ssot_drift.rs:137-200`) in lockstep.
- `Plan 392-02` (`[PARALLEL-SAFE]`, `5.5 Sonnet / Pro`): Fix the 3 `vox-gui` unit test panics (`models_host.rs:127`, `manifest.rs:295`, `manifest.rs:370`) for `gui-cross-build.yml` and fix OpenRouter vs local Ollama (`127.0.0.1:11434`) model routing in `vox harness eval --live` for `harness-eval-nightly.yml`.
- `Plan 392-03` (`[PARALLEL-SAFE]`, `Flash / Argon`): Add `uv` + `numpy` + `pyarrow` + `target/debug/vox` setup to `mutation-nightly.yml` and `cross-platform-check.yml`, fix Windows clippy warnings + `workflow_runtime_no_db`, fix the 5 URLs in `link_checker.yml`, and remove `CARGO_BUILD_JOBS: "2"` + 5x fat-LTO bottleneck on `aarch64-apple-darwin` in `nightly-artifacts.yml:84`.
- `Plan 392-04` (`[PARALLEL-SAFE]`, `5.5 Sonnet / Pro`): Fix all 6 failing jobs in `nightly.yml` (`Guards`, `Lints` hakari diff + `vox-drift-check`, `Compiler gates` `pipeline-ssot`, `Tests`, `Audits` `safety-inventory`, `Full workspace` `vox-db-types/src/store_types/trials.rs:248` rustdoc links + `cargo deny`/`audit`), consolidate `compile-matrix.yml` & `cr-l8-corpus-feedback.yml` into `ci.yml`, and update `~/dev/local-ci`.
- `Plan 392-05` (`[SEQUENTIAL]`, `5.5 Sonnet / Pro`): Trigger `gh workflow run` across all 7 Nightly workflows on `vox-foundation/vox`, verify green completion (`vox ci status`), close issues `#713`, `#648`, `#585`, `#584`, `#583`, `#578`, and write `contracts/reports/nightly-e2e-run-proof.v1.json` + `docs/src/architecture/nightly-builds-ssot.md`.

**Success Criteria:**

1. `.github/workflows/ci.yml:65` runs `./target/debug/vox ci pre-push --ci` (preserving all 36 `ssot-drift` stages in PR CI), while local default `vox ci pre-push` completes in `<15s` on single-crate/docs commits.
2. `compile-matrix.yml` and `cr-l8-corpus-feedback.yml` are consolidated into path-filtered jobs in `ci.yml` reusing `vox-cli-linux-x64`, and `~/dev/local-ci` supports `--force` + Nightly verification.
3. All 7 scheduled Nightly workflows (`nightly.yml`, `nightly-artifacts.yml`, `gui-cross-build.yml`, `cross-platform-check.yml`, `link_checker.yml`, `mutation-nightly.yml`, `harness-eval-nightly.yml`) have their verified root-cause failures resolved.
4. Live `workflow_dispatch` runs complete green across all 7 Nightly workflows, open `nightly-failure` issues (`#713`, `#648`, `#585`, `#584`, `#583`, `#578`) are closed, and run proofs are committed in `contracts/reports/nightly-e2e-run-proof.v1.json`.

### Phase 393: Zero-Loss Worktree, Scratch Folder & Master Directory Consolidation Across `vox` & `~/dev` (`SCRATCH-CONSOLIDATE-01..04`) `[WAVE 2E & WAVE 5]`

**Directory:** `.planning/phases/393-zero-loss-worktree-scratch-folder-master-directory-consolida/`
**Goal:** Resolve all git worktrees, `.worktrees/*`, `.claude/worktrees/*`, `_wt/*`, `_scratch/*`, and duplicate sibling folders across `/Users/brbrainerd/dev/vox` and `/Users/brbrainerd/dev/*` into their canonical parent master directories with **zero work lost** — salvaging all unmerged code, schemas, and plans first (starting with `/Users/brbrainerd/dev/vox-mens-compare`'s `158 GB` tree, `decode_attention.rs`, `gate-receipt.v2.schema.json`, `spoke-gate-state.v1.schema.json`, and Phase `337–345` artifacts into `/Users/brbrainerd/dev/vox`), then pruning `~176+ GB` of redundant worktrees, build caches, and stale scratch folders across `vox`, `fableforge`, `Resonance`, `velocicode`, `gigme`, `ramada`, and `brFinancier`.
**Depends on:** Independent forensic audit complete ([`393-AUDIT.md`](phases/393-zero-loss-worktree-scratch-folder-master-directory-consolida/393-AUDIT.md)); executes in parallel with Wave 2–4
**Requirements:** `SCRATCH-CONSOLIDATE-01`, `SCRATCH-CONSOLIDATE-02`, `SCRATCH-CONSOLIDATE-03`, `SCRATCH-CONSOLIDATE-04`
**Parallel & Model Dispatch:**
- `Plan 393-01` (`[PARALLEL-SAFE]`, `5.5 Sonnet / Pro`): **Vox Salvage & Consolidation (`~165 GB` Reclaimed):** Salvage all unique untracked/modified artifacts from `/Users/brbrainerd/dev/vox-mens-compare` (`crates/vox-plugin-mens-candle-metal/src/decode_attention.rs`, `contracts/mens/gate-receipt.v2.schema.json`, `contracts/mens/spoke-gate-state.v1.schema.json`, `.planning/phases/337..345` plans/summaries/reviews) and `/Users/brbrainerd/dev/_scratch/{p335,p339_cards}` into `/Users/brbrainerd/dev/vox`; preserve `gsd/mens-compare-316` branch ref in `/Users/brbrainerd/dev/vox`; then remove `vox-mens-compare` (`158 GB`), `vox-voxdiagram` (`5.2 GB`), `.worktrees/pr-669` (`863 MB`), `vox-readme-main` (`301 MB`), `.worktrees/docs-sidebar-chevrons` (`301 MB`), `vox-mens-compare-p335-clean` (`281 MB`), `.worktrees/finalize-chat-loop`, `.claude/worktrees/research-detection-chat-gui-05d826`, and `~/dev/vox-integration`, followed by `git -C /Users/brbrainerd/dev/vox worktree prune`.
- `Plan 393-02` (`[PARALLEL-SAFE]`, `Flash / Argon`): **Sibling `~/dev` Repo Salvage & Worktree Cleanup (`velocicode`, `brFinancier`, `Resonance`, `gigme`, `ramada`, `mudlet`):**
  1. `velocicode`: Fast-forward/cherry-pick `velocicode-numpy` (`9fe792f`, `"numpy>=2.0"` in `pyproject.toml`) into `/Users/brbrainerd/dev/velocicode` `main` and remove `/Users/brbrainerd/dev/velocicode-numpy`.
  2. `brFinancier`: Remove clean duplicate worktree `/Users/brbrainerd/dev/brFinancier-subdup` (`465 MB`) and run `git -C /Users/brbrainerd/dev/brFinancier worktree prune`.
  3. `Resonance`: Prune missing `/private/tmp/resonance-runtime-prev-base`, archive/port the 7 commits + dirty diff from `/Users/brbrainerd/dev/Resonance-ci-fix-forward` (`2.1 GB`) into `/Users/brbrainerd/dev/Resonance` (branch `fix/ci-python-environment` + patch bundle in `.vox/salvage/`), and remove the duplicate worktree.
  4. `gigme`, `ramada`, `mudlet`: Salvage dirty diffs/files from `gigme/.claude/worktrees/test-fast-shared-workers`, `gigme/.worktrees/narrow-nav-review`, `_scratch/gigme-phase179`, `ramada/.claude/worktrees/{v1.5-milestone,event-discovery-v1-1}`, `.worktrees/ramada-release-reports-20261027`, and `_scratch/wip-backups/wip-20261105-161304.patch` into their respective parent master repos, then remove all Bucket A/B/C worktrees and run `git worktree prune`.
- `Plan 393-03` (`[PARALLEL-SAFE]`, `Flash / Argon`): **FableForge (`_wt/*` & `.claude/worktrees/*`) & `~/dev/_scratch` Consolidation (`~32 GB` Reclaimed):** Remove merged FableForge worktrees (`fableforge-infra-pr` `3.9 GB`, `_wt/ff-replicate-slugs` `1.1 GB`, `_wt/ff-runner-doc` `1.1 GB`, and the 10 clean `fableforge/.claude/worktrees/*` `~10 GB`), preserve branches/dirty patches for the 4 active `_wt/ff-*` (`15.6 GB`) and 6 active `fableforge/.claude/worktrees/*` inside `/Users/brbrainerd/dev/fableforge` before removing their worktree directories, and clean up root `~/dev/*.log` Sept 25–27 stdout dumps (never touching `~/dev/memory` or `~/dev/antigravity-skill-packs`).
**Success Criteria:**
1. Every unique file, schema, kernel (`decode_attention.rs`), and `.planning/phases/337..345` artifact from `/Users/brbrainerd/dev/vox-mens-compare` and `_scratch/{p335,p339_cards}` is verified present inside `/Users/brbrainerd/dev/vox` before any Vox worktree is removed.
2. `git -C /Users/brbrainerd/dev/vox worktree list` reports **only** `/Users/brbrainerd/dev/vox` (1 master worktree), reclaiming `~165 GB` of disk space from Vox worktrees alone.
3. Every unmerged branch or dirty diff across `velocicode`, `Resonance`, `fableforge`, `gigme`, `ramada`, and `mudlet` is preserved (merged or saved as a named branch + verified patch in its parent master repo), and all secondary worktree directories (`_wt/`, `.worktrees/`, `.claude/worktrees/`, `*-subdup`, `*-infra-pr`, `*-numpy`, `*-ci-fix-forward`) are cleanly removed and pruned.
4. `~/dev/memory` and `~/dev/antigravity-skill-packs` remain 100% untouched, and a machine-verifiable consolidation receipt (`contracts/reports/dev-worktree-consolidation-receipt.v1.json`) records every salvaged artifact, preserved branch SHA, and reclaimed byte count.

**Plans:**

- [x] `393-01`: Forensic Audit & Zero-Loss Vox Worktree Salvage + Cleanup (`/Users/brbrainerd/dev/vox*`, `~165 GB` reclaimed)
- [x] `393-02`: Sibling `~/dev` Repository Salvage & Worktree Consolidation (`velocicode`, `brFinancier`, `Resonance`, `gigme`, `ramada`, `mudlet`)
- [x] `393-03`: FableForge (`_wt/*`, `fableforge-infra-pr`, `.claude/worktrees/*`) & `~/dev/_scratch` Consolidation + Verification Receipt

---

# Roadmap: v6.6 — Chat Multi-Dock Responsive Architecture, Diegetic Uncropped Simulator & Aqueduct Panorama, Playwright Button-Click CV Flywheel, `VoxDb` Lock-Free Daemon Boot & Enforceable Theme Contract (`Phases 383–386`)

**Milestone:** v6.6
**Status:** Complete (`4/4` Phases Verified — 2026-11-09)
**Approved Visual & Functional Direction:**

1. **Responsive Multi-Dock & Chat Surface (`Phase 383`):** Zero 4-column crush at `1280×800` and `1440×900` (`ChatSurface.tsx` auto-consolidates multi-dock layouts to a clean 2-column split and stacks secondary panels as tabs inside the right-hand companion column when viewport `< 1560px`), collapsed 1-line `AttentionBudgetMeter` by default unless attention is `>=80%` or blocked, unclipped `[EDIT] [REGENERATE] [COMPACT HERE]` transcript controls, and structured `LLM error:` retry notice cards.
2. **Diegetic Uncropped Simulator & Aqueduct Cascade (`Phase 384`):** Full-height vertical coverage (`drawH = safeCh`, `drawY = 0`) in narrow docked panels (`diegeticEnvironment.ts`), diegetic water ripples (`drawDiegeticWatercourses`), animated Roman citizen & legionary crowds (`drawDiegeticCrowds`), and 3-tier hydraulic sluice gates (`riverCanvasRenderer.ts`).
3. **Playwright Button-Click Audit (`before-click`, `on-click`, `after-click`), Synchronized Panorama Camera, Bounded Inspector & Lock-Free `VoxDb` (`Phase 385`):**
   - Eliminated `VoxDb` file-lock contention between `ModelRegistry::new()` background refresh (`catalog_seed.rs`) and `step_db` (`daemon_steps/run.rs`), plus graceful `.unwrap_or_default()` in `compute_model_leaderboard` (`gui_ops/models.rs`).
   - Synchronized `LudusSandbox.tsx` `panoRect` with relative camera zoom/pan (`relZoom`, `relDx`, `relDy`), fixed `useEffect([blit])` image reload bug on selection change, added an **Imperial Gold Focus Halo & Target Reticle** on `<canvas>`, and bounded `ImperiumInspector.tsx` (`bottom-[82px] max-h-[calc(100%-136px)]`) so it never overlaps the top architrave toolbar or bottom `TIERS` bar.
   - Compacted `NotificationCenter.tsx` (`max-h-[min(320px,calc(100vh-96px))]`) and `HudPanels.tsx` (`urbs-hud-embedded`) so notifications never collapse too high and docked HUD controls never clip (`S...`).
4. **Enforceable & Self-Expanding GUI Theme Contract + Adversarial `/gsd-review` Entropy Hardening (`Phase 386` — `ROMA-V66-THEME-CONTRACT-01..05`):**
   - **Machine-Readable 4-Profile Theme Contract SSOT (`contracts/gui/theme-contract.v1.json` & `crates/vox-gui/ui/src/lib/themeContract.ts`):** Canonical schema defining all 4 supported theme profiles (`arcane` — Default Imperial Carrara White & Gold; `slate` / `void` — Minimalist Grey-Blue Dark; `high-contrast` — High Contrast Obsidian & Gold; `glacier` / `travertine` — Sunlit Travertine & Verdigris), all `22` required semantic CSS variables (`--brass`, `--color-bg-*`, `--color-text-*`, `--color-border-*`, `--color-accent-*`, `--color-status-*`, `--color-overlay-*`, `--font-*`), programmatic **WCAG 2.1 Relative Luminance Contrast Ratio** verification (`>= 4.5:1` across all 4 profiles), general dark-hex/RGBA pattern detection, and single-line / zero-clipping control invariants.
   - **Style-Dictionary 4-Profile Token Generation (`tokens/primitive.json`, `tokens/semantic.json`, `tokens/semantic.slate.json`, `tokens/semantic.contrast.json`, `tokens/semantic.travertine.json`, `style-dictionary.config.mjs`):** Generates all 4 standalone CSS files (`tokens.generated.css`, `tokens.slate.generated.css`, `tokens.contrast.generated.css`, `tokens.travertine.generated.css`), each individually defining all 22 required semantic CSS variables, imported directly by `src/index.css` as the single source of truth.
   - **Blocking `Severity::Error` Static `vox-code-audit` Detector (`vox/ui/theme-contract-drift` in `crates/vox-code-audit/src/detectors/ui_theme_contract.rs`):** Enforces in `vox audit code` with `Severity::Error` that any `.tsx`, `.ts`, or `.css` file under `crates/vox-gui/ui/src/` uses semantic theme tokens (`bg-bg-surface`, `var(--color-*)`, etc.) and flags arbitrary dark background hexes (`bg-[#0..2...]`, `from-[#0..2...]`, `to-[#0..2...]`), dark inline `rgba(...)` surfaces, and quoted or unquoted banned font literals (`Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`).
   - **Zero Dark-Hex / Dark-RGBA Component Remediation Across 42 UI Components, Portals, Charts & Modals:** Replaced all remaining hardcoded dark-brown/near-black hex (`bg-[#1A140B]`, `bg-[#120E09]`, `bg-[#0c0a09]`, `bg-[#1F170E]`, etc.) and inline dark `rgba(...)` backgrounds across `RomanTooltip.tsx`, `SpeculumLoupePopover.tsx`, `ViaTemporisScrubber.tsx`, `ErrorBoundary.tsx`, `Popover.tsx`, `ImperiumInspector.tsx`, `chartWidgetShared.tsx`, `LudusSandbox.tsx`, and 34 surface components with semantic tokens (`bg-bg-surface`, `bg-bg-elevated`, `text-text-primary`, `var(--color-*)`).
   - **Self-Expanding Surface/Component Vitest Gate (`crates/vox-gui/ui/src/lib/themeContract.test.ts`), Live DOM `getComputedStyle` Health Report & 4-Theme Playwright Verification (`SettingsView.tsx`, `cv-flywheel.spec.ts`):** Automatically walks all `.tsx`/`.ts`/`.css` source files plus `discoverAllGuiSurfacesAndPanels()` (`37` views + `14` Chat docks) and verifies all 4 themes (`arcane`, `slate`, `glacier`, `high-contrast`) in Playwright E2E and Gemini Computer Vision.

---

# Roadmap: v6.5 — Complete 32-Surface White/Gold Carrara Marble Unification, Seamless Texture & Roman Emerald Signet Overhaul, and User-Selectable Minimalist Grey-Blue (`slate`) Theme (`Phases 378–382`)

**Milestone:** v6.5
**Status:** In Progress (`Phases 378–382`)
**Approved Visual Direction:**

1. **Default Theme — Imperial Carrara White & Gold (`arcane` / `carrara`):** Sunlit White/Cream Honed Carrara Marble (`#FAF6EE` / `#F5EFE2` / `#EFE6D5`), Burnished Gold mouldings (`#C59B27`), Deep Espresso Ink (`#1C140C`), **Roman Imperial Emerald & Gilded Laurel Wreath Signets** (`#1B6B3A` / `#228B4E` with `#D4AF37` gold border) for active/working status (crimson `#881B1B` reserved strictly for errors/alerts), seamless non-cropped marble textures, and universal Romanesque serif typography (`Cinzel`, `Palatino`) across all 32 surfaces, sidebar buttons, tooltips, and drawers.
2. **User-Selectable Minimalist Dark Theme — Grey-Blue Basalt Slate (`slate` / `void`):** Clean, minimalist grey-blue slate palette (`#0c0e10` base, `#15191c` surface, `#1f262b` elevated, `#2b343a` subtle border, `#f1f5f9` crisp text, `#38bdf8`/`#60a5fa` steel-blue & emerald `#10b981` status indicators, zero marble textures or decorative columns), configurable in `Settings → Theme`.

### Phase 378: Seamless Roman Marble Texture & Concept Art Generation + Fix Glitchy/Repeating/Offset Background Layers (`ROMA-V65-TEXTURES-01..04`) `[WAVE 1]`

**Goal:** Generate seamless frame-free honed white Carrara marble (`carrara-seamless-stone.jpg`), wide horizontal Roman acanthus frieze band (`carrara-acanthus-header-band.jpg`), Roman Imperial Emerald & Gilded Laurel Signet emblem (`roman-emerald-laurel-seal.jpg`), and Milestone `v6.5` concept art plates (`v66_concept_settings_dual_theme_white_and_slate.jpg`, `v66_concept_missed_surfaces_scientia_approvals_white_gold.jpg`); fix `@keyframes vox-marble-sheen` 3-layer `background-position` bug so marble textures never slide or tile; replace framed `travertine-parchment-tablet.jpg` and repeating `roman-architectural-frieze-footer.jpg` astrolabe seams across `index.css`, `Glass.tsx`, `Backdrop.tsx`, `RomanSurfaceHeader.tsx`, `BreadcrumbBar.tsx`, and `RomanTooltip.tsx`.

### Phase 379: Roman Emerald & Gilded Laurel Working Status System + Full Sidebar & Outside-`<main>` Chrome/Drawer White-Gold Conversion (`ROMA-V65-EMERALD-CHROME-01..04`) `[WAVE 2A]`

**Goal:** Replace red working/live `waxSealBadge` pills (`RomanSurfaceHeader.tsx`) and brown `bg-accent-secondary` live dots (`Sidebar.tsx`) with high-visibility **Roman Imperial Emerald & Gilded Laurel Wreath Signets** (`#1B6B3A` / `#228B4E` emerald cabochon + `#D4AF37` burnished gold rim + crisp vector laurel wreath SVG, legible at all resolutions); convert all dark-brown buttons and containers in `Sidebar.tsx` (collapse/expand/hide buttons, `⌘K` badge, expanded Chat Sessions list, active child items, count badges, operator card), `SessionSidebarSection.tsx`, `BackendStatusPill.tsx`, `BottomStatusBar.tsx`, `RomanTooltip.tsx`, `Omnibar.tsx`, `Dialog.tsx`, and all Drawers (`ExplainDrawer`, `DocViewerDrawer`, `InspectorDrawer`, `AchievementsDrawer`, `ResearchEngineDrawer`) to sunlit White/Gold Carrara Marble.

### Phase 380: Convert All 15+ Missed Surfaces & Purge Remaining Dark-Brown Remnants Across 85 Files (`ROMA-V65-SURFACES-01..04`) `[WAVE 2B]`

**Goal:** Convert all remaining hardcoded dark-brown (`#0A0805`..`#2A2011`) classes, inline styles, and canvas fills across `SettingsView.tsx`, `ScientiaDashboard.tsx`, `CommandCatalogForm.tsx`, `LudusSandbox.tsx`, `HudPanels.tsx`, `ImperiumInspector.tsx`, `worldRenderer.ts`, `PublicationsView.tsx`, `MemoryView.tsx`, `DiscoverySurface.tsx`, `ActivitySurface.tsx`, `ApprovalsView.tsx`, `PoliciesView.tsx`, `SubAgentsView.tsx`, `RepositoryView.tsx`, `BrowserView.tsx`, `CodeRabbitView.tsx`, `CommandCardsView.tsx`, `Mercatus.tsx`, `ModelsView.tsx`, `MensTrainingView.tsx`, `MensCorpusStudioPanel.tsx`, `GamifyView.tsx`, `LeaderboardArenaPanel.tsx`, and `HarnessRiverPanel.tsx` to the unified White/Gold Carrara Marble aesthetic, and extend `index.css` attribute overrides from `main` to `#root` when in the default White/Gold theme.

### Phase 381: Dual-Theme Architecture — Preserve & Wire User-Selectable Minimalist Grey-Blue Dark Theme (`slate` / `void`) in `Settings → Theme` (`ROMA-V65-DUAL-THEME-01..04`) `[WAVE 2C]`

**Goal:** Update `crates/vox-gui/ui/src/lib/theme.ts`, `SettingsView.tsx`, `index.css`, `Glass.tsx`, `Backdrop.tsx`, `Sidebar.tsx`, and `RomanSurfaceHeader.tsx` so users can switch seamlessly in `Settings → Theme` between **Imperial Carrara White & Gold (`arcane` — Default)** and **Minimalist Grey-Blue Slate (`slate` / `void`)** (`#0c0e10` / `#15191c` / `#1f262b` / `#2b343a` dark grey-blue palette with crisp sans/mono typography, emerald working indicators, and zero marble textures).

### Phase 382: Full 28-Surface + Dual-Theme Playwright & Gemini Computer Vision Flywheel Verification, Build & Live `vox axis` Relaunch (`ROMA-V65-CV-VERIFY-01..04`) `[WAVE 3]`

**Goal:** Expand `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` to capture and audit all 28 navigable `SURFACE_REGISTRY` surfaces + drawers/modals across **both** the Default White/Gold Carrara Marble theme and the Minimalist Grey-Blue Slate theme, inspect screenshots with Gemini built-in vision, present inline Before/After carousels, rebuild `vox-gui` + `vox` (`--features gui`), and relaunch `vox axis`.

---

### Phase 383: Dynamic Hugging Face Hub Sync, Zero-Hardcode SVG & Card Telemetry, Full Ladder Held-Out Re-Evaluation (`0.6B`–`8B`), and Universal Inference Provider & Playground Integration (`MENS-HUB-DYN-01..04`)

**Directory:** `.planning/phases/383-dynamic-hf-hub-sync-and-universal-inference/`
**Goal:** Eliminate all hardcoded SVG telemetry, stale phase/research prose, and `Pending` / `REGRESSED` misclassifications across `brbrainerd/vox-mens-index`, `brbrainerd/vox-mens-qwen3-{0.6b,1.7b,4b,8b,14b,32b}`, and `spaces/brbrainerd/vox-mens-playground`, while giving every ladder model (`0.6B`..`32B`) live inference provider & widget coverage:

1. **Zero-Hardcode Dynamic SVG & Fact Derivation (`MENS-HUB-DYN-01`)**:
   - In `crates/vox-populi/src/mens/hub_card/facts.rs` and `svg.rs`, replace the hardcoded `let is_pos = delta >= -0.005;` check in `render_scorecard_svg` with `!r.regressed` from the McNemar `collateral_damage_report.json` (`p >= 0.05` renders emerald `CLEAR`, eliminating false red `REGRESSED` badges on `0.6B`'s `math500_think` and `vox_think`).
   - Replace the hardcoded 5-spoke SVG strip in `render_scorecard_svg` and hardcoded `Active` / `Planned` spoke status + `default_vram_gb` array in `render_overview_svg` with dynamic rendering driven by `hub_cards().spokes`, `spoke_of(rec)`, `rec.spoke_gates`, `gate.state`, and each size's measured `peak_mb`.
   - Derive `spoke_of(rec)` dynamically from `rec.training_manifest` / `rec.spoke_gates` before falling back to `hub-cards.yaml` `default_spoke`, and purge stale internal phase/research references from `mens/config/hub-cards.yaml` and `crates/vox-populi/templates/hub/index.md`.
2. **Full-Coverage Hub Sync Across Trained, Diverged & Untrained Repos (`MENS-HUB-DYN-02`)**:
   - Update `crates/vox-populi/src/mens/hub_cards_push.rs` and `hub_sync.rs` so `NotTrained` (`32B`) cards/SVGs and `Diverged` repos whose remote `.safetensors` weights already match local are automatically synced in place during `vox mens hub sync --push`, alongside `brbrainerd/vox-mens-index` and `spaces/brbrainerd/vox-mens-playground`.
3. **Universal Inference Provider, Rich Vox `widget:` Frontmatter & Deep-Linked Playground (`MENS-HUB-DYN-03`)**:
   - Emit `inference: true` and real Vox codegen `widget:` examples in `crates/vox-populi/templates/hub/partials/frontmatter.md` across **all 6 model cards (`0.6B`..`32B`)** so Hugging Face never falls back to generic chatbot prompts (`"Hi, what can you help me with?"`).
   - Add direct deep-links (`https://huggingface.co/spaces/brbrainerd/vox-mens-playground?size=<SIZE>&tab=webgpu`) to `partials/use_it.md` and `index.md`.
   - Upgrade `crates/vox-populi/templates/hub/space/index.html.tmpl` to honor `?size=` and `?tab=` URL parameters and support live interactive inference across all 6 sizes (`0.6B`..`32B`) via both **In-Browser WebGPU SafeTensors LoRA Merger** (`0.6B`, `1.7B`, `4B`, `8B`) and **OpenAI-Compatible Cloud / Local Inference Provider Router** (`https://router.huggingface.co/v1/chat/completions` + local `vox mens serve` `http://127.0.0.1:11435/v1/chat/completions`) for `0.6B`..`32B`.
4. **Full Held-Out `nf4_base_plus_lora_arm v2` Re-Evaluation & Live Playwright Verification (`MENS-HUB-DYN-04`)**:
   - Complete `1.7B` retrain + `eval-local` + `eval-reasoning-compare` + `eval-collateral-damage` + `eval-gate`, re-run `eval-local --force-restart` + `eval-gate` on `4B` and `8B` so all four (`0.6B`, `1.7B`, `4B`, `8B`) carry `"adapter_serving": "nf4_base_plus_lora_arm v2"` and recorded examples, push all repos + Space live, and verify with Playwright.

**Depends on:** Phase 362, Phase 368
**Requirements:** `MENS-HUB-DYN-01`, `MENS-HUB-DYN-02`, `MENS-HUB-DYN-03`, `MENS-HUB-DYN-04`

---

# Roadmap: v6.4 — Full-Canvas White/Gold Carrara Marble Unification, Non-Procedural Monument Simulator, Single-Line Roman Controls & Zero-Monospace Typography (`Phases 373–377`)

**Milestone:** v6.4
**Status:** Complete (`5/5` Phases Verified — 2026-11-08)
**Approved Visual Direction:** Milestone `v6.4` White/Gold Carrara Marble Concept Art (`v65_concept_urbs_simulator_white_gold_acropolis_1794158463815.jpg`, `v65_concept_explorer_tasks_runs_white_gold_1794158471421.jpg`, `v65_concept_chat_mens_knowledge_white_gold_1794158501800.jpg`) — Full-body Sunlit White/Cream Carrara Marble & Ivory Travertine (`#FAF6EE` / `#F5EFE2` / `#EFE6D5`), Burnished Gold borders (`#C59B27`), High-Contrast Dark Espresso Ink (`#1C140C`), Vitruvian Architectural Watermark background (`carrara-page-watermark-bg.jpg`), Sculpted Roman Frieze footer (`roman-architectural-frieze-footer.jpg`), Painted Roman Monument Simulator (`urbs-white-gold-citadel-bg.jpg` + `monument-*.jpg`), Single-Line Roman Controls, and Zero Courier/Arial/Menlo Monospace Typography.

### Phase 373: Concept Art Generation, Circular `ORCHESTRATOR` Bubble Cutoff Fix & Single-Line `Explorer` Controls (`ROMA-V64-FLOW-EXP-01..04`) `[COMPLETE]`

**Goal:** Generate Milestone `v6.4` concept art, monument portraits, and background/frieze textures; enlarge the central `ORCHESTRATOR` circular hub medallion (`r=66` in `AgentFlow.tsx`) and widen the `ORCHESTRATOR` DAG node (`width=136` in `NeedsYouSurface.tsx`) for zero text cutoff; make `OPEN FORM`, `EXPLAIN CAPABILITY`, `OPEN DOCS` strictly single-line (`ExplorerTree.tsx`, `Button.tsx`) and `= GENERIC FORM` a sleek single-line Roman pill (`CapabilityBadge.tsx`).

### Phase 374: Non-Procedural Painted Roman Monument Simulator & Live File/Folder Aqueduct Streams (`ROMA-V64-URBS-MONUMENTS-01..04`) `[COMPLETE]`

**Goal:** Remove procedural 2D polygon building/wall drawing from `worldRenderer.ts` (`paintFullBleedRomanLandscape`, `drawDistrictCurtainWall`, `drawIsoFileBuilding`, `paintDistrictWonder`, `paintLandmarks`); wire `LudusSandbox.tsx` to `urbs-white-gold-citadel-bg.jpg`, generated painted monument portraits (`monument-pantheon-orchestrator.jpg`, `monument-temple-compiler.jpg`, `monument-library-knowledge.jpg`, `monument-colosseum-harness.jpg`), real-time active file/folder badges (`src/parser.rs`, `gamify/LudusSandbox.tsx`, `src/turso.rs`, `src/qlora.rs`, `models/select.rs`, `src/mesh.rs`), and animated SVG golden aqueduct streams.

### Phase 375: Full-Body Sunlit White/Cream Carrara Marble & Burnished Gold Theme + Architectural Watermark & Frieze Footer (`ROMA-V64-WHITE-GOLD-01..04`) `[COMPLETE]`

**Goal:** Convert `Backdrop.tsx`, `Glass.tsx`, and `index.css` (`:root` color tokens, `main`, tables, rows, inputs, selects, buttons, and legacy dark utilities) across all 32 surfaces to the Sunlit White/Cream Carrara Marble & Burnished Gold aesthetic with subtle Vitruvian architectural watermark background and sculpted bottom Roman frieze footer.

### Phase 376: Universal Roman Typography Enforcement & Zero Monospace/Courier/Arial Purge (`ROMA-V64-TYPO-01..04`) `[COMPLETE]`

**Goal:** Remap `--font-mono`, `--font-family-mono`, `.font-mono`, `pre`, `code`, `kbd`, `samp`, `.cm-editor`, `.cm-content` in `index.css` and `dockview-vox.css`, plus all SVG/Canvas font declarations across `AgentFlow.tsx`, `worldRenderer.ts`, `riverCanvasRenderer.ts`, `NeedsYouSurface.tsx`, `Matrix.tsx`, and `MensTrainingView.tsx`, to `'Palatino', 'Palatino Linotype', 'Book Antiqua', 'EB Garamond', Georgia, serif` (`tabular-nums`) and `'Cinzel', 'Palatino', Georgia, serif`.

### Phase 377: Multi-Resolution (`1280×800`, `1440×900`, `1920×1080`) Built-In Gemini Vision + Playwright CV Flywheel Verification (`ROMA-V64-VERIFY-01..04`) `[COMPLETE]`

**Directory:** `.planning/phases/377-v64-white-gold-carrara-marble-unification/`
**Goal:** Verify all 32 `SURFACE_REGISTRY` surfaces and 3 viewport resolutions (`1280×800`, `1440×900`, `1920×1080`) via Playwright + `@ngi/platform-ssot` CV Flywheel (`0` `axeViolations`, `0` `clippedInteractiveCount`, `0` `overlappingControlPairs`, `0` `digitalColorLeakCount`), inspect screenshots with built-in Gemini vision, eliminate all Vite/CSS/bundler warnings, and present inline Before/After carousels.

---

# Roadmap: v6.3 — Game-UI Romanesque Unification, 92px Sculptural Medallion Sidebar Rail, Sunlit Travertine & Parchment Stone Tablets, Zero-Void/Zero-Truncation Multi-Resolution Layout & Built-In VLM Vision Flywheel (`Phases 368–372`)

**Milestone:** v6.3
**Status:** Executing with Parallel Subagent Waves (`image-generator` + `self`).
**Updated:** 2026-11-08
**Approved Visual Direction:** Game-UI Romanesque Concept Art (`v61_concept_compute_and_market_1794141010387.jpg`, `v61_concept_workspace_and_commands_1794141011579.jpg`, `v62_concept_semiotic_urbs_codebase_city_1794147288395.jpg`) — 92px Sculptural Bronze Medallion Sidebar Rail, Sunlit Travertine & Parchment Stone Tablets (`#EDE3D1`/`#DFD3BD`), Riveted Burnished Brass Console Bezels, Large Semicircular Brass Chronometer Dials, Olive-Laurel Bronze & Crimson Wax Seal Badges (zero digital neon green/cyan/blue), and zero clipping/truncation/empty void across `1280×800`, `1440×900`, and `1920×1080`.

### Phase 368: Built-In VLM Vision Audit & Structural/Chromatic Flywheel Upgrade (`vlmVisionFlywheel.ts`, `audits.ts`, `cv-flywheel.spec.ts`) (`ROMA-CV-V63-01..04`) `[WAVE 1]`

**Goal:** Upgrade the Playwright + VLM Computer Vision Flywheel (`vlmVisionFlywheel.ts`, `audits.ts`, `cv-flywheel.spec.ts`) to detect child clipping inside `overflow-hidden` containers (`clippedChildInOverflowHiddenCount`), mid-word text truncation (`truncatedTextElementCount`), empty viewport void (`emptyViewportVoidRatio`), digital green/teal/cyan pixel leaks (`digitalGreenBluePixelRatio`), and sunlit travertine/parchment tablet coverage (`travertineParchmentSlabRatio`).
**Depends on:** Phase 367
**Requirements:** `ROMA-CV-V63-01`, `ROMA-CV-V63-02`, `ROMA-CV-V63-03`, `ROMA-CV-V63-04`

### Phase 369: 92px Sculptural Bronze Medallion Game-UI Sidebar Rail (`Sidebar.tsx`, `Layout.tsx`, `StatusBar.tsx`) & Zero-Truncation Status Bar (`ROMA-RAIL-CHROME-01..04`) `[WAVE 2A]`

**Goal:** Transform `Sidebar.tsx` from a `216px` SaaS sidebar list into the Concept Art's `92px` Sculptural 3D Bronze Coin Medallion Game-UI Rail (embossed circular bronze medallions on top, chiseled uppercase `Cinzel` labels beneath, glowing vertical gold active bar, reclaiming `124px` of horizontal screen space); fix `StatusBar.tsx` truncation (`ROUTING Auto → anthr`) and replace neon green/cyan dots with Romanesque gold, olive-laurel bronze, and crimson wax seals.
**Depends on:** Phase 368
**Requirements:** `ROMA-RAIL-CHROME-01`, `ROMA-RAIL-CHROME-02`, `ROMA-RAIL-CHROME-03`, `ROMA-RAIL-CHROME-04`

### Phase 370: Sunlit Travertine & Parchment Stone Tablets, Large Semicircular Brass Chronometer Dials & Zero-Digital-Color Purge (`index.css`, `RomanSurfaceHeader.tsx`, All 32 Surfaces) (`ROMA-TABLET-GAMEUI-01..04`) `[WAVE 2B]`

**Goal:** Replace dark `#0E0C0A`/`#14100B` IBM-style card/table bodies and digital neon green/teal/cyan pills across `index.css` and surface components (`ModelsView`, `VoxPopuliView`, `MercatusView`, `TerminalSurface`, `RepositoryView`, `CommandCatalogSurface`, `SkillsPluginsView`, `MemoryView`, `ScientiaSurface`, `ExplorerView`, `SettingsView`, `Dashboard`, `HarnessRiverPanel`) with Warm Sunlit Roman Travertine & Parchment Stone Tablets (`#EDE3D1` / `#DFD3BD` with `#1E150C` deep espresso ink), Riveted Brass Console Bezels, large semicircular `<RomanBrassGauge>` chronometer dials, and Olive-Laurel / Crimson Wax Seal badges.
**Depends on:** Phase 368
**Requirements:** `ROMA-TABLET-GAMEUI-01`, `ROMA-TABLET-GAMEUI-02`, `ROMA-TABLET-GAMEUI-03`, `ROMA-TABLET-GAMEUI-04`

### Phase 371: Multi-Resolution (`1280×800`, `1440×900`, `1920×1080`) Zero-Clipping & Empty-Void Elimination (`LudusSandbox.tsx`, `HarnessRiverPanel.tsx`, `VoxPopuliView.tsx`, `MercatusView.tsx`, `TerminalSurface.tsx`) (`ROMA-VOID-RES-01..04`) `[WAVE 2C]`

**Goal:** Eliminate bottom dock/radar collision and top toolbar wrapping at `1280×800` in `LudusSandbox.tsx`, eliminate bottom funnel clipping at `1280×800` in `HarnessRiverPanel.tsx`, and replace empty bottom viewport voids in `VoxPopuliView.tsx`, `MercatusView.tsx`, and `TerminalSurface.tsx` with interactive Roman Mesh Topology, Arbitrage Matrix, and Command Quick-Launch Stone Tablets.
**Depends on:** Phases 369–370
**Requirements:** `ROMA-VOID-RES-01`, `ROMA-VOID-RES-02`, `ROMA-VOID-RES-03`, `ROMA-VOID-RES-04`

### Phase 372: Multi-Pass Built-In Vision + Playwright CV Flywheel Verification & Inline Before/After Presentation (`ROMA-V63-VERIFY-01..04`) `[WAVE 3]`

**Goal:** Run multi-pass Playwright + VLM CV Flywheel audits across all 32 surfaces and 3 resolutions (`1280×800`, `1440×900`, `1920×1080`), inspect every screenshot directly with Gemini built-in vision against the Concept Art, verify `pnpm --dir crates/vox-gui/ui build`, and present inline Before/After carousels.
**Depends on:** Phase 371
**Requirements:** `ROMA-V63-VERIFY-01`, `ROMA-V63-VERIFY-02`, `ROMA-V63-VERIFY-03`, `ROMA-V63-VERIFY-04`

---

# Roadmap: v6.2 — Romanesque Semiotic Codebase City, Aqueduct LLM/Orchestrator Cascade & Gamified Developer Controls (`Phases 363–367`)

**Milestone:** v6.2
**Status:** Executing with Parallel Subagent Waves (`image-generator` + `self`).
**Updated:** 2026-11-08
**Research & Design SSOTs:**

- [`docs/src/architecture/semiotic-codebase-and-aqueduct-orchestrator-simulator-research-2026.md`](../docs/src/architecture/semiotic-codebase-and-aqueduct-orchestrator-simulator-research-2026.md)
- [`docs/src/architecture/romanesque-white-marble-pillars-animation-ui-spec-2026.md`](../docs/src/architecture/romanesque-white-marble-pillars-animation-ui-spec-2026.md)

### Phase 363: Nov 2026 Semiotic Codebase & Aqueduct Orchestrator Research KB + High-Resolution Concept Art & Simulator Assets (`ROMA-KB-CONCEPT-01..04`) `[WAVE 1]`

**Goal:** Research external software city visualizers (*CodeCharta*, *CodeCity*, *M3triCity*), multi-agent LLM orchestrator graphs (*LangGraph Studio*, *AutoGen Studio*, *Rivet*, *Arize Phoenix*), and hydraulic city-builder UX mechanics (*Anno 117: Pax Romana*, *Timberborn*, *Caesar III*), persist the findings in `docs/src/architecture/semiotic-codebase-and-aqueduct-orchestrator-simulator-research-2026.md`, and generate high-resolution Concept Art plates + simulator/aqueduct raster assets (`urbs-semiotic-citadel-bg.jpg`, `aqueduct-sluice-cascade-bg.jpg`).
**Depends on:** Phase 362
**Requirements:** `ROMA-KB-CONCEPT-01`, `ROMA-KB-CONCEPT-02`, `ROMA-KB-CONCEPT-03`, `ROMA-KB-CONCEPT-04`

### Phase 364: Semiotic Codebase City (`L0`–`L5`), White Carrara Marble Temples & Interactive Developer Command Dock (`LudusSandbox.tsx`, `worldRenderer.ts`) (`ROMA-URBS-SEMIOTIC-01..04`) `[WAVE 2A]`

**Goal:** Upgrade `worldRenderer.ts` and `LudusSandbox.tsx` with high-contrast white Carrara marble temple/basilica colonnades, gilded domes, semiotic architectural layer badges (`L0 Foundation` through `L5 Surfaces`), hydraulic token-stream particles, a **Semiotic Layer & Legend Bar** (`[All | L0..L5]`), and a 1-click **Developer Action Command Dock** (`[⚙ Check Crate]`, `[✓ Run Tests]`, `[🔍 Diff Loupe]`, `[📌 Pin Context]`, `[🔒 Quarantine]`, `[⚔ Spawn Agent]`, `[🏆 Claim Yield]`).
**Depends on:** Phase 363
**Requirements:** `ROMA-URBS-SEMIOTIC-01`, `ROMA-URBS-SEMIOTIC-02`, `ROMA-URBS-SEMIOTIC-03`, `ROMA-URBS-SEMIOTIC-04`

### Phase 365: Aqueduct Hydraulic LLM & Orchestrator Cascade, 8-Spoke Source Arches & Interactive Brass Sluice Gates (`HarnessRiverPanel.tsx`, `riverCanvasRenderer.ts`) (`ROMA-AQUEDUCT-FLOW-01..04`) `[WAVE 2B]`

**Goal:** Transform `HarnessRiverPanel.tsx` and `riverCanvasRenderer.ts` into a multi-tiered Roman Aqueduct LLM & Orchestrator Cascade backed by `aqueduct-sluice-cascade-bg.jpg`, featuring the **Aquae Caput (Hub + 8-Spoke Source Arcade)**, **3-Tier Marble Channels (`Cursus I/II/III`)** with live shape+color token particles (`❯❯` Cyan Supply, `◆◆` Gold Token Stream, `◉` Emerald Verified, `▲` Crimson Fault), **4 Interactive Brass Sluice Gates (`Clausurae`: `AST Parse`, `Typecheck`, `TOESTUB Audit`, `Test Suite`)**, and 1-click developer orchestrator actions (`[Dispatch Task]`, `[Flush Sluice]`, `[Run Check]`, `[Compact Context]`, `[Claim Quest Yield]`).
**Depends on:** Phase 363
**Requirements:** `ROMA-AQUEDUCT-FLOW-01`, `ROMA-AQUEDUCT-FLOW-02`, `ROMA-AQUEDUCT-FLOW-03`, `ROMA-AQUEDUCT-FLOW-04`

### Phase 366: Gamified Developer Loop & Bi-Directional Surface Navigation (`ROMA-GAMIFY-DEV-01..04`) `[WAVE 2C]`

**Goal:** Connect developer actions in `LudusSandbox` and `HarnessRiverPanel` to real backend/RPC operations and live gamified XP/Denarii quest yield feedback with toast/status confirmation and 1-click surface navigation.
**Depends on:** Phases 364–365
**Requirements:** `ROMA-GAMIFY-DEV-01`, `ROMA-GAMIFY-DEV-02`, `ROMA-GAMIFY-DEV-03`, `ROMA-GAMIFY-DEV-04`

### Phase 367: Multi-Pass VLM + Playwright Computer Vision Flywheel Convergence & Inline Before/After Presentation (`ROMA-V62-FLYWHEEL-01..04`) `[WAVE 3]`

**Goal:** Capture true Before baselines, upgrade `vlmVisionFlywheel.ts` and `cv-flywheel.spec.ts` to verify semiotic layer badges, aqueduct sluice gates, and developer action docks, run multi-pass Playwright + VLM CV Flywheel audits across all surfaces, build `vox-gui`, and present Concept Art + Before/After carousels inline.
**Depends on:** Phase 366
**Requirements:** `ROMA-V62-FLYWHEEL-01`, `ROMA-V62-FLYWHEEL-02`, `ROMA-V62-FLYWHEEL-03`, `ROMA-V62-FLYWHEEL-04`

### Phase 368: Romanesque MENS Corpus & Pipeline Studio GUI + Auto-Sync JSONL Flywheel (`MENS-STUDIO-GUI-01..04`)

**Directory:** `.planning/phases/368-mens-corpus-pipeline-studio-gui/`
**Goal:** Eliminate manual JSONL regeneration and surface the entire MENS corpus/mix/reasoning/checkpoint pipeline in `vox-gui` (`MensCorpusStudioPanel.tsx` inside `MensTrainingView.tsx`), matching the Romanesque Concept Art (`concept_mens_pipeline_studio_romanesque_1794153153510.jpg`) via CV Flywheel convergence:

1. Auto-materialize missing/stale synthetic and extracted JSONL sources (`vox_reasoning_traces.jsonl`, `docs.jsonl`, `rust_source.jsonl`) and auto-regenerate `train_mixed*.jsonl` + `mens/data/train.jsonl` whenever source fingerprints or weights change.
2. Expose real Tauri IPC commands (`mens_corpus_studio_snapshot`, `mens_corpus_read_jsonl`, `mens_corpus_save_jsonl_row`, `mens_corpus_rebuild_mix`) backed by `vox_corpus` and `vox_compiler::parser::parse` AST verification — zero hardcoded/fake data.
3. Build the 3-arch Romanesque `MensCorpusStudioPanel.tsx` (`Mix Sources & Auto-Materializer DAG`, `Live JSONL Record Inspector & Inline AST Editor`, `Ladder Checkpoints & Collateral Gate Matrix`) with live search, task-family filtering, inline `<think>` & Vox code editing, compiler diagnostics, and one-click/auto mix rebuild.
4. Verify with Rust unit tests, Vitest component tests, and Playwright CV Flywheel visual convergence against the Romanesque concept art.

**Depends on:** Phase 362
**Requirements:** `MENS-STUDIO-GUI-01`, `MENS-STUDIO-GUI-02`, `MENS-STUDIO-GUI-03`, `MENS-STUDIO-GUI-04`
**Plans:** 1/1 plan complete (`368-01-PLAN.md` -> `368-01-SUMMARY.md`) — **Complete**

---

# Roadmap: v6.1 — Full-Surface Plate 2 Romanesque Generalization, High-Resolution Diorama Simulator & Structural CV Flywheel (`Phases 358–361`)

**Milestone:** v6.1
**Status:** Executing with Parallel Subagent Waves (`pro` + `flash` disjoint file scopes).
**Updated:** 2026-11-08
**Approved Visual Direction:** Concrete Plate 2 Romanesque Suite (`axis_romanesque_concept_2_urbs_ludus_1794129218189.jpg`, `v61_concept_compute_and_market_1794141010387.jpg`, `v61_concept_workspace_and_commands_1794141011579.jpg`, `v61_concept_knowledge_explorer_urbs_refined_1794141055158.jpg`) — Universal carved Roman marble bas-relief frieze headers (`<RomanSurfaceHeader>`), embossed circular bronze medallions (`<RomanMedallion>`), semicircular antique brass dial gauges (`<RomanBrassGauge>`), crimson wax-seal status badges, warm travertine stone tablets, readable humanist typography, high-resolution transparent-composited `urbs-diorama-bg.jpg` simulator canvas, and upgraded CV Flywheel detectors (`skeletonCount === 0`, `digitalColorLeakCount === 0`, `rawBacktickCount === 0`).

### Phase 358: Structural CV Flywheel Upgrade (`audits.ts`, `cvFlywheel.ts`, `cv-flywheel.spec.ts`) (`ROMA-CV-STRUCT-01..04`) `[WAVE 1A]`

**Goal:** Upgrade the automated Playwright CV Flywheel (`audits.ts`, `cvFlywheel.ts`, `cv-flywheel.spec.ts`) so it catches and fails on non-Romanesque regressions across all 32 surfaces: loading skeleton bars (`skeletonCount`), cold digital slate/cyan/violet leaks (`digitalColorLeakCount`), raw unrendered markdown backticks (`rawBacktickCount`), and opaque canvas fills obscuring the Roman diorama in `LudusSandbox`.
**Depends on:** Phase 357
**Requirements:** `ROMA-CV-STRUCT-01`, `ROMA-CV-STRUCT-02`, `ROMA-CV-STRUCT-03`, `ROMA-CV-STRUCT-04`

### Phase 359: Concrete Non-Abstract Concept Art Plates & High-Resolution Roman Diorama Simulator (`LudusSandbox.tsx`, `worldRenderer.ts`, `hydraulicNetwork.ts`) (`ROMA-SIM-DIORAMA-01..04`) `[WAVE 1B]`

**Goal:** Generate concrete, non-abstract Plate 2 Romanesque concept art plates for `COMPUTE & MARKET`, `WORKSPACE & COMMANDS`, and `KNOWLEDGE, EXPLORER & URBS SIMULATOR`; eliminate the double opaque dark-blue canvas fill (`#0a1019` in `LudusSandbox.tsx` and `#040e18` in `worldRenderer.ts`) that obliterated 90% of `urbs-diorama-bg.jpg`; and replace neon cyan viaduct/conduit strokes in `worldRenderer.ts` and `hydraulicNetwork.ts` with burnished Roman gold (`#D4AF37`), warm travertine (`#F3E5AB`), and verdigris aqueduct emerald (`#10B981`).
**Depends on:** Phase 358
**Requirements:** `ROMA-SIM-DIORAMA-01`, `ROMA-SIM-DIORAMA-02`, `ROMA-SIM-DIORAMA-03`, `ROMA-SIM-DIORAMA-04`

### Phase 360: Universal Plate 2 Romanesque Generalization Across `COMPUTE`, `MARKET`, `WORKSPACE`, `COMMANDS`, `KNOWLEDGE`, `REVIEW`, `EXPLORER` & `SETTINGS` (`ROMA-ALL-TABS-01..04`) `[PARALLEL-SAFE WAVE 2]`

**Goal:** Build `<RomanSurfaceHeader>` (carved marble bas-relief frieze banner, embossed bronze medallion, semicircular `<RomanBrassGauge>` KPI dials, crimson wax-seal status pill, and progressive-disclosure `<RomanTooltip>`) + global Plate 2 CSS table/card/input overrides in `index.css`, and generalize Plate 2 Romanesque styling across `ModelsView`, `CommandCardsView` (`Populi` & `Oratio`), `Mercatus`, `RepositoryView`, `Console`, `BrowserView`, `CodeRabbitView`, `Catalog`, `CommandCatalogForm`, `SkillsPluginsView`, `MemoryView`, `ScientiaSurface`, `ResearchView`, `PublicationsView`, `VoxGraphStatusPanel`, `ApprovalsView`, `PoliciesView`, `ExplorerView` (instant zero-skeleton hydration), and `SettingsView`.
**Depends on:** Phase 359
**Requirements:** `ROMA-ALL-TABS-01`, `ROMA-ALL-TABS-02`, `ROMA-ALL-TABS-03`, `ROMA-ALL-TABS-04`

### Phase 361: Multi-Round 32-Surface Before/After Playwright CV Flywheel Verification & Inline Presentation (`ROMA-V61-VERIFY-01..04`) `[WAVE 3]`

**Goal:** Run multi-round Playwright CV Flywheel verification across all 32 surfaces (`skeletonCount === 0`, `digitalColorLeakCount === 0`, `rawBacktickCount === 0`, `overlappingControlPairs === 0`, `axeViolations === 0`), capture before/after screenshots across `COMPUTE`, `MARKET`, `WORKSPACE`, `COMMANDS`, `KNOWLEDGE`, `EXPLORER`, and `URBS SIMULATOR`, rebuild `vox-gui`, and present inline before/after carousels.
**Depends on:** Phase 360
**Requirements:** `ROMA-V61-VERIFY-01`, `ROMA-V61-VERIFY-02`, `ROMA-V61-VERIFY-03`, `ROMA-V61-VERIFY-04`

### Phase 362: MENS Regression Root-Cause Fixes (`supervised_ratio`, `<think>` Reasoning Synthesis & Cued Answer Grader) (`MENS-REGFIX-01..04`)

**Goal:** Eliminate the four verified root causes of MENS training/collateral regressions across the Qwen3 ladder:

1. Switch `generate_training_system_prompt()` in `crates/vox-corpus/src/training/mod.rs` from `generate_system_prompt()` (~2,200 tokens / 8.6 KB `mens/config/system_prompt.txt`) to `builtin_system_prompt()` (~350 tokens / 1.48 KB), raising `supervised_ratio` from `4.5%` (`< 10%` gate failure) to `~20–25%` and speeding up QLoRA step time by ~4x.
2. Add deterministic, compiler-verified, zero-leakage Vox `<think>` reasoning trace synthesis (`crates/vox-corpus/src/corpus/vox_reasoning_synth.rs`: 120 rows across `literal_debug`, `state_trace`, and `invariant_check`) and wire `mens/data/mix_sources/vox_reasoning_traces.jsonl` (`weight: 2.0`) into `mens/config/mix.yaml`, `mens/config/mix-vox-lang.yaml`, and `mix_prepare.rs` so adapters do not auto-fix literal-execution `debug` items or ramble past 2048 tokens without closing `</think>`.
3. Harden `extract_cued_answer` in `crates/vox-eval/src/qwen_reasoning.rs` (scanning cues right-to-left, rejecting English prose stopwords like `"The"`/`"that"`/`"not"`, and recovering earlier valid cues when a trace truncates right after `"So the answer is "`) and sync `qwen_reasoning.rs` v2 across worktrees.
4. Use `--rank 8 --alpha 16` (`grad_accum=8`) when retraining 28-layer models (`0.6B`, `1.7B`) so `lm_head` + projection LoRA updates preserve base reasoning calibration.

**Depends on:** Phase 333, Phase 345
**Requirements:** `MENS-REGFIX-01`, `MENS-REGFIX-02`, `MENS-REGFIX-03`, `MENS-REGFIX-04`
**Plans:** 1 plan (`362-01-PLAN.md` — Complete 2026-11-08)

---

# Roadmap: v5.9 — Full-Prompt Goal Audit & Universal Plate 2 Romanesque Convergence (`Phases 352–356`)

**Milestone:** v5.9
**Status:** Executing with Parallel Subagent Waves (`pro` + `flash` disjoint file scopes).
**Updated:** 2026-11-08
**Approved Visual Direction:** Concept Art Plate 2 (`axis_romanesque_concept_2_urbs_ludus_1794129218189.jpg`) — Universal carved Roman travertine & burnished brass design system across all 32 Vox Axis surfaces, Plate 2 exact 5-medallion top frieze & 3-part Stone Telemetry Console (`Git Timeline`, 3 `RomanBrassGauge` dials, `Architecture Astrolabe Radar`) on `LudusSandbox`, complete elimination of residual sci-fi "Argon AI" cyan/void visuals in `riverCanvasRenderer.ts` and `AgentTrajectoryBraid.tsx`, 1-click navigable monuments from both `Dashboard` and `GamifyView`, universal progressive-disclosure `RomanTooltip` & `Tooltip` across all surfaces, live Chat Orchestrator→Spoke & Hub Model visualizer, and 32-surface × 3-resolution CV Flywheel convergence.

### Phase 352: Plate 2 Exact Urbs Stone Telemetry Console, Top-5 Medallion Frieze & 1-Click Navigable Monuments (`ROMA-URBS-CONSOLE-01..04`) `[PARALLEL-SAFE WAVE 1A — PRO]`

**Goal:** Converge `LudusSandbox.tsx`, `HudPanels.tsx`, `ViaTemporisScrubber.tsx`, `GamifyView.tsx`, and `surfaceComponents.tsx` with Concept Art Plate 2 by adding the top-row 5-medallion carved marble frieze (`Workspace Map`, `Agent Orchestrator`, `CI & Verification`, `Task Harbor`, `Token Treasury`), upgrading the bottom stone telemetry console so all 3 gauges (`Budget`, `Energy`, `Speed`) render as animated semicircular `RomanBrassGauge` dials flanked by carved Roman column dividers (`I` and `III`), adding the crimson wax seal (`wax-seal-medallion.jpg`) + laurel wreath to `ViaTemporisScrubber.tsx`, threading `onNavigate` into `GamifyView.tsx`, and making every monument plaque and landmark medallion 1-click navigable.
**Depends on:** Phase 351
**Requirements:** `ROMA-URBS-CONSOLE-01`, `ROMA-URBS-CONSOLE-02`, `ROMA-URBS-CONSOLE-03`, `ROMA-URBS-CONSOLE-04`

### Phase 353: Sci-Fi "Argon AI" Elimination in `HarnessRiverPanel` (`riverCanvasRenderer.ts`, `AgentTrajectoryBraid.tsx`), `AgentFlow` & `NeedsYouSurface` (`ROMA-ARGON-PURGE-01..04`) `[PARALLEL-SAFE WAVE 1B — FLASH]`

**Goal:** Replace the remaining cold sci-fi `#070709` black background, glowing cyan singularity orb, and laser scanner line in `riverCanvasRenderer.ts` and `AgentTrajectoryBraid.tsx` with warm Roman aqueduct stone, molten gold (`#D4AF37`), amber-bronze (`#D97706`), verdigris emerald (`#10B981`), and a carved travertine & brass keystone banner; reskin `AgentFlow.tsx` (`AGENTS › FLOW`) and `NeedsYouSurface.tsx` to Plate 2 marble frieze headers, embossed bronze/gold node rings, and `<RomanTooltip>` progressive disclosure.
**Depends on:** Phase 351
**Requirements:** `ROMA-ARGON-PURGE-01`, `ROMA-ARGON-PURGE-02`, `ROMA-ARGON-PURGE-03`, `ROMA-ARGON-PURGE-04`

### Phase 354: Universal Plate 2 Romanesque Card/Tooltip Primitives (`Glass`, `Kpi`, `Tooltip`), Live Chat Orchestrator→Spoke Visualizer & `ExplorerView` Hydration (`ROMA-UNIVERSAL-01..04`) `[PARALLEL-SAFE WAVE 1C — FLASH]`

**Goal:** Upgrade `Glass.tsx`, `Kpi.tsx`, and `Tooltip.tsx` to Plate 2's warm carved travertine (`#15110C` / `#1B160F`) and burnished brass (`#D4AF37`) aesthetic so all 32 surfaces share a unified Romanesque skin; enhance `ChatExecutionRail.tsx` with a prominent Plate 2 Live Orchestrator Hub & Active Spoke visualizer with `<RomanTooltip>` progressive disclosure; and hydrate `get_capability_index` in `tauriMockRich.ts` so `ExplorerView` renders rich capability cards without loading skeletons.
**Depends on:** Phase 351
**Requirements:** `ROMA-UNIVERSAL-01`, `ROMA-UNIVERSAL-02`, `ROMA-UNIVERSAL-03`, `ROMA-UNIVERSAL-04`

### Phase 355: Multi-Resolution (`1280×800`, `1440×900`, `1920×1080`) 32-Surface CV Flywheel Pass & Full-Goal Audit Matrix (`ROMA-AUDIT-CV-01..04`) `[WAVE 2]`

**Goal:** Execute multi-resolution Playwright CV Flywheel verification across all 32 surfaces and 3 resolutions (`1280×800`, `1440×900`, `1920×1080`), capture updated before/after visual artifacts, verify every prompt requirement end-to-end, rebuild the production bundle, and relaunch `vox-gui`.
**Depends on:** Phases 352–354
**Requirements:** `ROMA-AUDIT-CV-01`, `ROMA-AUDIT-CV-02`, `ROMA-AUDIT-CV-03`, `ROMA-AUDIT-CV-04`

## Progress Table (`v5.9`)

| Phase | Requirements | Status |
|---|---|---|
| 352 Plate 2 Exact Urbs Stone Telemetry Console, Top-5 Medallion Frieze & 1-Click Navigable Monuments | `ROMA-URBS-CONSOLE-01..04` | Complete (`2026-11-08`) |
| 353 Sci-Fi "Argon AI" Elimination in `HarnessRiverPanel`, `AgentFlow` & `NeedsYouSurface` | `ROMA-ARGON-PURGE-01..04` | Complete (`2026-11-08`) |
| 354 Universal Plate 2 Primitives (`Glass`, `Kpi`, `Tooltip`), Chat Orchestrator Visualizer & `ExplorerView` | `ROMA-UNIVERSAL-01..04` | Complete (`2026-11-08`) |
| 355 Multi-Resolution 32-Surface CV Flywheel Pass & Full-Goal Audit Matrix | `ROMA-AUDIT-CV-01..04` | Complete (`2026-11-08`) |

---

# Roadmap: v5.8 — Plate 2 Romanesque Axis Overhaul, Plain-English Navigable Surfaces, Generated Art Assets, Progressive-Disclosure Tooltips & 32-Surface CV Flywheel (`Phases 346–351`) — COMPLETE

**Milestone:** v5.8
**Status:** Complete (`24/24` requirements verified 2026-11-08) — Extended in `v5.9` (`Phases 352–355`).
**Updated:** 2026-11-08
**Approved Visual Direction:** Concept Art Plate 2 (`axis_romanesque_concept_2_urbs_ludus_1794129218189.jpg`) — Carved marble frieze headers, embossed bronze/gold medallions, warm travertine tablets, crimson wax seals, brass gauges, and engraved astrolabe radar, with **100% plain-English labels**, **generated raster art assets (no crude polygons)**, **clickable/navigable map buildings & radar regions**, **Plate 2-styled Sidebar**, and **Romanesque progressive-disclosure tooltips** explaining what every feature is for and what clicking it does.

### Phase 346: Generated Romanesque Art Assets, Plate 2 Marble Frieze Sidebar, Plain-English Semiotics & Progressive-Disclosure Tooltips (`ROMA-CHROME-01..04`) `[PARALLEL-SAFE WAVE 1]`

**Goal:** Generate real raster art assets (`public/assets/romanesque/*` — carved marble frieze, engraved astrolabe dial, isometric Roman architectural diorama & monuments, bronze medallion icons, crimson wax seal), reskin `Sidebar.tsx` and `BreadcrumbBar.tsx` into Plate 2's carved marble frieze & bronze medallion style (fixing the top-left `[ Sidebar › ]` collision), replace emojis with bespoke SVG/art semiotics (`Icons.tsx`), and ship the Romanesque Progressive-Disclosure Tooltip component (`RomanTooltip.tsx`) that explains in plain English what every button, knob, and surface is for and what interacting with it does.
**Depends on:** Nothing (Wave 1)
**Requirements:** `ROMA-CHROME-01`, `ROMA-CHROME-02`, `ROMA-CHROME-03`, `ROMA-CHROME-04`
**Success Criteria** (what must be TRUE):

  1. Generated Romanesque art assets (`frieze-header.jpg`, `astrolabe-dial.jpg`, `urbs-diorama-bg.jpg`, `wax-seal-badge.jpg`) live under `crates/vox-gui/ui/public/assets/romanesque/` and replace polygon-only placeholders.
  2. `Sidebar.tsx` and `BreadcrumbBar.tsx` use Plate 2's carved marble frieze & bronze medallion styling with plain-English navigation labels, Romanesque progressive-disclosure tooltips on every nav item, and zero overlap when `sidebarMode === 'hidden'` across `1280×800`, `1440×900`, and `1920×1080`.
  3. `RomanTooltip.tsx` and `SurfaceHelpTooltip.tsx` render carved travertine & burnished brass progressive-disclosure tooltips explaining **What it is**, **What it does**, and **Click action / shortcut** in clear English.

### Phase 347: Plain-English Interactive `Workspace Map` (`LudusSandbox`), Navigable Buildings, Animated Astrolabe Radar & Dashboard Minimap Isolation (`ROMA-DASH-URBS-01..04`) `[PARALLEL-SAFE WAVE 2A]`

**Goal:** Wrap `Dashboard.tsx`'s embedded mini-map in `<EmbeddedSurfaceProvider value={true}>`, replace all cryptic Latin-only labels in `LudusSandbox.tsx`, `HudPanels.tsx`, and `ViaTemporisScrubber.tsx` with clear English (`Workspace Map`, `Orchestrator`, `MCP Tools`, `CI & Gates`, `Task Queue`, `Git History`, `Compiler`, `Database`, `MENS Models`, `Mesh Network`, `Security`, `Git Timeline`, `Priority Alerts`, `Architecture Radar`), make every mapped building/landmark and radar region clickable and navigable to its real Vox Axis surface, replace the 148-polygon radar smear with the generated/animated **7-Region Architecture Astrolabe Radar**, and animate brass gauges, radar sweep, and hydraulic flow.
**Depends on:** Phase 346
**Requirements:** `ROMA-DASH-URBS-01`, `ROMA-DASH-URBS-02`, `ROMA-DASH-URBS-03`, `ROMA-DASH-URBS-04`
**Success Criteria** (what must be TRUE):

  1. `Dashboard.tsx` embeds `<LudusSandbox />` via `<EmbeddedSurfaceProvider value={true}>` with zero HUD pileups and clear plain-English controls + progressive-disclosure tooltips.
  2. Every landmark pill, mapped building (`Orchestrator`, `Compiler`, `GUI & Surfaces`, `Database`, `MENS Models`, `Mesh Network`, `Security & Crypto`), and radar sector (`Foundation`, `Core`, `Orchestrator`, `Runtime`, `MENS`, `Mesh & DB`, `GUI`) is interactive: clicking focuses the region and provides a working `[Open <Surface> ↗]` action that navigates directly to that surface (`onNavigate`).
  3. The 148-polygon wireframe radar is replaced by the Plate 2 engraved marble & brass **Architecture Astrolabe Radar** with animated sweep, glowing hotspot braziers, draggable/clickable viewport frustum, and zero clipped background canvas text (`AQUA Supply`).

### Phase 348: Plate 2 Romanesque Reskin of `GamifyView` (`Arena & Quests`) & `HarnessRiverPanel` (`Pipeline Cascade & Task Stream`) with Generated Art & Animations (`ROMA-LUDUS-RIVER-01..04`) `[PARALLEL-SAFE WAVE 2B]`

**Goal:** Reskin `GamifyView.tsx`, `LudusHud.tsx`, `LeaderboardArenaPanel.tsx`, and `HarnessRiverPanel.tsx` (`WaveformTierStrip.tsx`, `riverCanvasRenderer.ts`, `VerificationFunnelFooter.tsx`) into Plate 2's carved marble, bronze medallion, brass gauge, and crimson wax-seal aesthetic with 100% plain-English terminology (`Workspace Map`, `Arena & Quests`, `Engineering Leaderboard`, `Fast Local Tier`, `Balanced Hybrid Tier`, `Deep Architecture Tier`, `Live Task Stream`, `Verification Funnel: Generated → Compiled → Tested → Gated`), generated art backgrounds, smooth animations, and progressive-disclosure tooltips on every control.
**Depends on:** Phase 346
**Requirements:** `ROMA-LUDUS-RIVER-01`, `ROMA-LUDUS-RIVER-02`, `ROMA-LUDUS-RIVER-03`, `ROMA-LUDUS-RIVER-04`
**Success Criteria** (what must be TRUE):

  1. `GamifyView.tsx`, `LudusHud.tsx`, and `LeaderboardArenaPanel.tsx` render Plate 2 travertine tablets, laurel wreaths, crimson wax-seal verification badges, and plain-English tabs/controls with interactive tooltips.
  2. `HarnessRiverPanel.tsx` replaces sci-fi argon neon with Plate 2 brass pressure gauges, animated golden/amber/emerald laminar task streams through a bronze astrolabe interceptor gate, and stepped travertine verification basins.

### Phase 349: Interactive Status Bar Popover Dropdowns, Live `Repo / Local Version` Tile & `Configure ▾` Persistence (`ROMA-STATUSBAR-01..04`) `[PARALLEL-SAFE WAVE 2C]`

**Goal:** Upgrade `BottomStatusBar.tsx` and `useHudTiles.ts` to Plate 2's burnished brass & travertine style, add the live `Repo / Version` (`REPO vox@main · 98ea5d2 · v0.6.0`) tile backed by `get_vcs_status` / `get_app_version`, and give every status bar tile (`Repo`, `Engine`, `Spend`, `Mesh`, `Routing`, `Alerts`, `Research`) both a rich interactive popover dropdown (with plain-English progressive-disclosure explanations and live controls) and one-click navigation to its surface, all configurable in `Configure ▾`.
**Depends on:** Phase 346
**Requirements:** `ROMA-STATUSBAR-01`, `ROMA-STATUSBAR-02`, `ROMA-STATUSBAR-03`, `ROMA-STATUSBAR-04`
**Success Criteria** (what must be TRUE):

  1. `BottomStatusBar.tsx` displays `REPO vox@<branch> · <short_sha> · v<version>` and opens a rich Repository popover (`Branch`, `Commit`, `Working Tree Status`, `[Open Git Surface ↗]`, `[Refresh Status]`).
  2. Every status bar tile (`Repo`, `Engine`, `Spend`, `Mesh`, `Routing`, `Alerts`, `Research`) has both an interactive popover dropdown and a direct `[Open <Surface> ↗]` navigation button, plus progressive-disclosure tooltips and `Configure ▾` persistence.

### Phase 350: Progressive Disclosure, Button Clustering & Multi-Resolution (`1280×800`, `1440×900`, `1920×1080`) Overflow Elimination Across All 32 Surfaces (`ROMA-DISCLOSURE-01..04`) `[WAVE 3]`

**Goal:** Audit and refine all 32 registered surfaces (`Dashboard`, `Chat`, `MENS Training`, `Workspace Map`, `Harness River`, `Operations Hub`, `Tasks`, `Mesh`, `Sub-Agents`, `Model Router`, `Cost Analytics`, `Scientia`, `Repository`, `Settings`, etc.) to apply Plate 2 button theming, progressive disclosure (`[Details ▾]` drawers and Romanesque tooltips), and responsive overflow fixes at `1280×800`, `1440×900`, and `1920×1080`.
**Depends on:** Phases 346–349
**Requirements:** `ROMA-DISCLOSURE-01`, `ROMA-DISCLOSURE-02`, `ROMA-DISCLOSURE-03`, `ROMA-DISCLOSURE-04`
**Success Criteria** (what must be TRUE):

  1. All 32 surfaces use plain-English labels, Plate 2 Romanesque button/card tokens, and progressive-disclosure tooltips/drawers without crowded toolbars or text truncation.

### Phase 351: Multi-Pass 32-Surface × 3-Resolution Before/After Playwright CV Flywheel & Live Vox Axis Verification (`ROMA-CV-FLYWHEEL-01..04`) `[WAVE 4]`

**Goal:** Run multi-pass Playwright CV Flywheel verification across all 32 surfaces and 3 resolutions (`1280×800`, `1440×900`, `1920×1080`) against Concept Art Plate 2 until zero mechanical or accessibility defects remain, capture before/after visual comparisons, and rebuild/relaunch the live `vox-gui` desktop binary.
**Depends on:** Phase 350
**Requirements:** `ROMA-CV-FLYWHEEL-01`, `ROMA-CV-FLYWHEEL-02`, `ROMA-CV-FLYWHEEL-03`, `ROMA-CV-FLYWHEEL-04`
**Success Criteria** (what must be TRUE):

  1. Multi-resolution Playwright CV Flywheel passes with `overlappingControlPairs === 0`, `clippedTextCount === 0`, `horizontalOverflowPx === 0`, and `axeViolations === 0`.
  2. Live `vox-gui` binary is rebuilt and running with all v5.8 Plate 2 improvements.

## Progress Table (`v5.8`)

| Phase | Requirements | Status |
|---|---|---|
| 346 Top-Left Collision Fix, Romanesque Tokens & Engraved SVG Semiotics | `ROMA-CHROME-01..04` | Ready |
| 347 Dashboard Embedded Minimap, Frieze Drawers & 7-Regio `Forma Urbis` Radar | `ROMA-DASH-URBS-01..04` | Ready |
| 348 Romanesque Reskin of `GamifyView` & `AQVAEDUCTUS` Harness River | `ROMA-LUDUS-RIVER-01..04` | Ready |
| 349 Interactive Status Bar Popover Dropdowns & `Repo / Local Version` Tile | `ROMA-STATUSBAR-01..04` | Ready |
| 350 Progressive Disclosure, Button Clustering & 32-Surface Overflow Elimination | `ROMA-DISCLOSURE-01..04` | Ready |
| 351 Multi-Pass 32-Surface × 3-Resolution Before/After CV Flywheel | `ROMA-CV-FLYWHEEL-01..04` | Ready |

---

# Roadmap: v5.7 — Vox ↔ Python Bridge & Tolerant CLI Ergonomics

**Milestone:** v5.7
**Status:** Phase 324 verified 2026-11-03 (5/5). `phase.complete` did not write this file: workstream mode has no active workstream, and v5.7 lives on this root roadmap.
**Updated:** 2026-11-01
**Design SSOT:** `docs/src/architecture/vox-python-interop-design-2026-11-01.md`; `docs/src/architecture/cli-semantic-tolerance-research-2026.md`
**Paper-code rule (all phases):** a phase exits only on an end-to-end `vox run` golden through the **real** worker (or, for compiler-only phases, a compiler golden through the real pipeline) — never on unit tests or the fake worker alone. Feature-matrix cells flip to `Implemented` only with a running ladder entry.

### Phase 324: Effect Gate & Determinism (`EFF-01..03`)

**Goal:** The effect and capability gate that every later Python call depends on exists and cannot be silently bypassed.
**Depends on**: Nothing (first phase of v5.7)
**Requirements**: EFF-01, EFF-02, EFF-03
**Success Criteria** (what must be TRUE):

  1. `@uses(nonexistent)` and `uses nonexistent` both produce a compile error; `@uses(mcp(tool))` appears in HIR as `Mcp(tool)` (golden).
  2. `@uses(python_unverified)` survives into HIR and `HirCapability`; a materializer-shaped call without the effect fails typecheck (golden).
  3. `vox run --caps` without `py` refuses a Python-capability call at runtime with a caps error.
  4. A `workflow` calling a Python-capability fn is rejected by capability (not name); the same call in an `activity` compiles.

**Plans**: 2/2 complete (324-01, 324-02). Verification passed 2026-11-03.

### Phase 325: Bridge Runtime & Environment (`BR-01..04`, `GOV-01`)

**Goal:** `vox-python-bridge` spawns a real uv-managed worker, speaks framed Arrow IPC, survives crashes/timeouts, and lands with governance rows.
**Depends on**: Phase 324
**Requirements**: BR-01, BR-02, BR-03, BR-04, GOV-01
**Success Criteria** (what must be TRUE):

  1. `vox py sync` on a `[python]` project creates the env and records the `uv.lock` hash in `vox.lock`; `vox py doctor` reports status; a project without `[python]` gets `py-env:` and spawns no process.
  2. A one-op golden runs end-to-end under `vox run` through the real worker; a worker op that prints to stdout still returns a correct result.
  3. A deliberately hung op hits `timeout_s`, the worker restarts, and a pre-restart handle returns `py: stale handle`; the host process survives a worker crash.
  4. An over-cap fetch returns `py: fetch exceeds cap`; sandboxed spawn goes through `vox-skill-runtime`, unsandboxed warns.
  5. `vox-arch-check` and `vox ci crate-edges` pass with the new crate; AGENTS.md exemption and stale-citation fixes are in the diff.

**Plans**: 2/2 complete (325-01, 325-02)

### Phase 326: `std.array` on numpy (`ARR-01..04`, `K-01`)

**Goal:** Vox code computes with numpy through a closed, typed, standards-named vocabulary and pure lazy plans.
**Depends on**: Phase 325
**Requirements**: ARR-01, ARR-02, ARR-03, ARR-04, K-01
**Success Criteria** (what must be TRUE):

  1. `examples/golden/py_array_stats.vox` runs under `vox run` with asserted output, using one round-trip per materialization.
  2. Mixed int/float without `.astype`, `std` without `correction=`, and `sort` without `nulls=` are compile errors.
  3. NaN, ±Inf and i64::MIN/MAX round-trip through `to_list`; uint64 overflow errors.
  4. `vox ci python-bridge-parity` fails when an allowlisted op is missing its signature, handler or conformance test (negative test).
  5. Ladder entry + complexity-budget row exist for the golden; `LEXER_KEYWORDS` unchanged; K2 ceiling enforced.

**Plans**: 8/8 executed. Verified 2026-11-04 (8/8 must-haves). Code review left 3 critical findings open.

### Phase 327: `std.frame` on polars (`FRM-01..03`)

**Goal:** Vox code runs lazy dataframe pipelines on polars and fetches typed `Records` with exact type mapping and host-side path control.
**Depends on**: Phase 326
**Requirements**: FRM-01, FRM-02, FRM-03
**Success Criteria** (what must be TRUE):

  1. `examples/golden/py_frame_report.vox` (scan_csv → filter → group_by/agg → sort → collect) passes with one round-trip per `collect`.
  2. A fixture with decimal, tz-aware timestamp, date, duration and nullable columns fetches as `Decimal`, UTC `DateTime`, `Date`, `Duration`, `Option[T]`; a naive timestamp errors with the fix-it.
  3. `scan_csv` on a path outside `--caps fs:ro=` is refused before any worker request (asserted via fake-worker request log).

**Plans**: TBD

### Phase 328: `std.plot` & `std.tensor` (`PLT-01`, `TNS-01..02`)

**Goal:** Vox code renders matplotlib charts and runs exported PyTorch models without data or tensors leaving the worker unnecessarily.
**Depends on**: Phase 327
**Requirements**: PLT-01, TNS-01, TNS-02
**Success Criteria** (what must be TRUE):

  1. A plot golden writes a non-empty PNG with expected dimensions; the request log shows no frame fetch to host.
  2. `examples/golden/py_tensor_infer.vox` loads a checked-in tiny `.pt2` and returns expected logits on CPU; a pickle `.pt` is refused.
  3. Two seeded runs produce identical numpy/torch outputs; the activity journal entry contains library versions and lock hash.

**Plans**: TBD

### Phase 329: Tier 2 Escape Hatch (`PYX-01..04`)

**Goal:** Vox code reaches the long tail (scipy, scikit-learn, pillow, pandas, any bound package) through a typed, gated, counted escape hatch.
**Depends on**: Phase 328
**Requirements**: PYX-01, PYX-02, PYX-03, PYX-04
**Success Criteria** (what must be TRUE):

  1. `examples/golden/py_escape_psd.vox` (`import py sig from "scipy.signal"`) passes; an unlisted name, a stale sidecar, and `PyObject.field` are compile errors.
  2. Curated sidecars + goldens for scipy, scikit-learn, pillow and pandas pass in the opt-in Python CI job.
  3. Each Tier 2 call site yields `vox/python/unverified-call` and a `python.escape_call` event; syntax-K support metrics show `py_tier1_calls` / `py_tier2_calls`.
  4. Attempting to bind `pickle` or `subprocess` fails with a Denied-policy error.

**Plans**: TBD

### Phase 330: DB & Backend Interop (`DBI-01..03`)

**Goal:** Vox apps share databases owned by Drizzle/SQLAlchemy/Alembic and serve Python clients, using existing introspection and OpenAPI surfaces.
**Depends on**: Phase 324 (independent of 325–329; may run in parallel)
**Requirements**: DBI-01, DBI-02, DBI-03
**Success Criteria** (what must be TRUE):

  1. `vox db introspect --emit vox` on a sqlite fixture emits `table` decls that `vox db verify` accepts against the same DB (round-trip test; postgres/mysql behind existing features).
  2. With `[db] schema_owner = "external"`, `vox db migrate` refuses to apply and `--dry-run` prints the plan.
  3. The how-to's Python client example is generated from a golden server app's OpenAPI and checked in CI docs lint.

**Plans**: 1/1 complete (330-01)

### Phase 331: Tolerant CLI Ergonomics & Precomputed Semantic Routing (`CLI-TOL-01..04`)

**Goal:** The `vox` CLI tolerates human memory gaps (unambiguous prefix completions, global flag hoisting, verb↔noun token permutations, unique leaf elision, and precomputed compositional synonyms) in `<1.5 ms` with zero runtime LLM calls, while enforcing `side_effect_class` safety gates and returning structured 1-turn self-repair redirects for LLM agents and non-TTY callers.
**Depends on**: Independent of Phases 324–330 (may run in parallel)
**Requirements**: CLI-TOL-01, CLI-TOL-02, CLI-TOL-03, CLI-TOL-04
**Success Criteria** (what must be TRUE):

  1. `VoxCliRoot` enables `infer_subcommands(true)` and `infer_long_args(true)`, and all `GlobalOpts` flags (`--json`, `--quiet`, `--verbose`, `--color`) carry `global = true` so `vox sec doc` and `vox secrets list --json` parse natively at `0 ms` overhead.
  2. When `VoxCliRoot::try_parse_from` fails, `argv_normalize` resolves order-agnostic verb↔noun permutations (`vox list secrets` → `vox secrets list`), unique leaf elision (`vox command-compliance` → `vox ci command-compliance`), and wrong-subcommand flags (`vox build --release` → suggests `vox compile --release`) in `<1.5 ms` over `CommandCatalog` and `CapabilityIndex`.
  3. Execution is gated by `side_effect_class` and TTY/agent detection: `read_only` permutations/synonyms auto-execute on interactive TTYs with a dim `stderr` canonical echo, mutating commands prompt `[Y/n]` on TTYs, and non-TTY/CI/LLM callers (`--json`, `CI=true`, `VOX_MCP=1`) never hang on prompts or silently auto-execute fuzzy guesses—returning exit code `2` with structured `aci.error_recovery` canonical redirects.
  4. `vox ci cli-synonyms --write` compiles compositional `verb_aliases`, `noun_aliases`, and `foreign_idioms` from `contracts/operations/catalog.v1.yaml` (using `vox_actor_runtime::llm` + `gemini-3.5-flash-lite` structured output and `gemini-embedding-2` / `local_embed_text` cosine margin $\ge 0.08$ verification) into `contracts/cli/semantic-aliases.v1.json`, and `vox ci command-compliance` fails on any alias collision or antonym banlist violation (`add` $\leftrightarrow$ `remove`, `start` $\leftrightarrow$ `stop`, `lock` $\leftrightarrow$ `unlock`, `set` $\leftrightarrow$ `get`).

**Plans**: 3 plans

Plans:

- [ ] `331-01-PLAN.md` — Prefix unfolding (`infer_subcommands`/`infer_long_args`), global flag hoisting, and `<1.5 ms` deterministic `argv_normalize::classify_recovery` (`CLI-TOL-01`, `CLI-TOL-02`)
- [ ] `331-02-PLAN.md` — Dual-persona (`InteractiveHuman` vs `NonInteractiveOrAgent`) & `side_effect_class` safety gate + structured `aci.error_recovery` diagnostics (`CLI-TOL-03`)
- [ ] `331-03-PLAN.md` — Precomputed semantic synonym compiler (`vox ci cli-synonyms`), `contracts/cli/semantic-aliases.v1.json` lockfile, and antonym/collision firewall (`CLI-TOL-04`)

### Phase 333: Gated training for every model size

**Goal:** Every published Qwen3 size finishes the same release review, then a merged complete model is stored and distributed, and only then does its Hugging Face card call it ready. 0.6B leaves the smoke recipe. 4B is already a full shared-recipe run. Cards stay in plain language and describe the trained add-on until the complete model is the file being distributed.
**Depends on**: Independent of Phases 324–331
**Requirements**: REL-01, REL-02, REL-03, REL-04
**Success Criteria** (what must be TRUE):

  1. 0.6B, 1.7B, 4B, and 8B each have a published run on the shared recipe (rank 16, alpha 32, gradient accumulation 8). The 0.6B smoke preset (`qwen3_dev_cpu`, rank 8, accumulation 1) is not the published 0.6B adapter. The published 4B run stays classified as a full shared-recipe run that was uploaded before the checklist passed. A resumed run reports token and step counts for the whole run, not only the last process.
  2. Each of those four sizes has a passing collateral check and a release receipt with `overall_passed: true`. 14B and 32B stay without a distributed Vox model until a run clears that same review.
  3. After that receipt, `vox mens merge-qlora` writes a complete checkpoint from the local Qwen base plus the trained adapter, and that checkpoint is the file stored and distributed. The local cache already holds original Qwen bases for 0.6B, 1.7B, 4B, 8B, and 14B. 32B is not downloaded. 14B has no trained adapter yet.
  4. The Hugging Face card and the size index change in the same step as that distribution. Until then they say the page is the trained add-on and the review is unfinished. They do not use the word "ungated". A reader can download the distributed file from the Files tab or with one `vox mens hub download` command.

**Plans**: 0 plans

Plans:

- [ ] TBD (run `/gsd-plan-phase 333` to break down)

## Progress Table (`v5.7`)

| Phase | Requirements | Status |
|---|---|---|
| 324 Effect Gate & Determinism | EFF-01..03 | Verified 2026-11-04 |
| 325 Bridge Runtime & Environment | BR-01..04, GOV-01 | Verified 2026-11-04 |
| 326 `std.array` on numpy | ARR-01..04, K-01 | Verified 2026-11-04 |
| 327 `std.frame` on polars | FRM-01..03 | Not started |
| 328 `std.plot` & `std.tensor` | PLT-01, TNS-01..02 | Not started |
| 329 Tier 2 Escape Hatch | PYX-01..04 | Not started |
| 330 DB & Backend Interop | DBI-01..03 | Verified 2026-11-04 |
| 331 Tolerant CLI Ergonomics & Precomputed Semantic Routing | CLI-TOL-01..04 | Planned (3 plans) |
| 333 Gated training for every model size | REL-01..04 | Not planned |

### Phase 342: Docs Authoring Parity and Starlight 0.42 Upgrade

**Directory:** `phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/`
**Goal:** Close the verified remainder of the AsciiDoc-parity blueprint for `voxlang.org`: upgrade `docs-astro` to Starlight ≥0.42 / Astro ≥7.2.10 (the gate for the ecosystem plugins), render the 24 GitHub-alert docs as real asides, and add build-time internal-link validation — without new contracts, new `.mdx` churn, or a parallel status system.
**Requirements**: DOCS-PARITY-01..04
**Depends on:** none (independent of v5.6 lanes; coordinate banners with `docs-freshness` Phase 32)
**SSOT:** `docs/src/architecture/docs-site-asciidoc-parity-findings-2026.md`
**Plans:** 4/4 plans complete — ✅ verified 2026-11-06 (7/7, `342-VERIFICATION.md`, `09c272e99`). Resolved stack: astro 7.4.5, starlight 0.42.9 (mature-only install, no release-age exceptions). Playwright 20 / unit 3 failures are the frozen pre-upgrade baseline, unchanged.

- **DOCS-PARITY-01** (✅ committed `9829418e9`): `remark-vox-include.mjs` supports `tag::x[]`/`end::x[]`, `#` leaders, comma multi-select, `N..M` line ranges, exact names, dedent; unit tests in `docs-astro/tests/unit/remark-vox-include.test.mjs`; `llms.txt` version derived from root `Cargo.toml` + namespace disambiguation. Build green (906 pages).
- **DOCS-PARITY-02:** Starlight `^0.38` → `≥0.42`, Astro `^6` → `≥7.2.10`; fix overrides (`Banner`, `Sidebar`, `PageSidebar`, `TableOfContents`, `SiteTitle`), `routeData.ts`, and the `@astrojs/starlight/components` Vite alias shims; `pnpm build` + `tests/unit` + Playwright `reader-layout`/`trust-signals` stay green.
- **DOCS-PARITY-03:** `starlight-github-alerts` renders the 24 `> [!NOTE|TIP|IMPORTANT|WARNING|CAUTION]` docs as asides (identical on GitHub and the site).
- **DOCS-PARITY-04:** `starlight-links-validator` in the build; fix the two current `remark-doc-links` failures (`t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md` out-of-root link, missing `api/vox-cargo-shim.generated.md` from `l5-binaries.generated.md`); feeds nightly Docs Link Checker (#578).

**Out of scope (rejected after verification — see SSOT):** reality-audit-driven banners (inventory has no status field; banners already ledger-driven, owned by `docs-freshness` Phase 32), `stability-matrix.v1.json` + Content Layer collections (invented contract; generated-`.md` pattern is canonical), `remark-vox-attributes` tokens / `::include{}` directives / `// !hide` / `examples/golden/anti/` titles (zero call sites), `starlight-markdown-blocks` custom asides.

Plans:

- [x] 342-01-PLAN.md — Fix both remark-doc-links failures at source (t3 page links; hub generator links only existing API pages, test-first) and freeze the pre-upgrade test baseline (wave 1)
- [x] 342-02-PLAN.md — Package-legitimacy gate, then astro 7 / Starlight 0.42 upgrade (unified() processor, alias via import.meta.resolve, llms-txt 0.12 patch), dependabot unpin, agent-safe Playwright (wave 2)
- [x] 342-03-PLAN.md — starlight-github-alerts renders every alert as an aside; un-nest the two list-nested alerts for GitHub parity (wave 3)
- [x] 342-04-PLAN.md — starlight-links-validator on default fatal options with a unit-tested exclude predicate, mutation probe, findings SSOT update (wave 4)

### Phase 343: Docs System SSOT and Automated Drift Control

**Goal:** Every kind of documentation divergence is either prevented (generated from a contract), detected (a gate that fails), or auto-corrected (regenerated by the `ssot-drift` / `ssot-autoregen` loop). No documented gap stays manual and nobody notices. The docs-system map `docs/src/architecture/docs-system-ssot.md` (landed 2026-11-06) is the SSOT; its §6 "Known gaps" list is this phase's backlog. Each gap closed removes its bullet in the same PR.
**Requirements**: DOCS-DRIFT-01..06
**Depends on:** Phase 342
**SSOT:** `docs/src/architecture/docs-system-ssot.md`
**Plans:** 9 plans

- **DOCS-DRIFT-01 (prevent, regen-list parity):** derive `scripts/ssot-regen.vox`'s regenerator list from `run_ssot_drift`'s stage table (or add a parity check that fails on mismatch). Removes the "keep in sync" hand list.
- **DOCS-DRIFT-02 (detect, overlay parity):** a `vox ci` check that extracts normative claims (forbidden/required frontmatter keys, retired symbols, banned commands) from `AGENTS.md` and fails when `.cursor/rules/*.mdc`, `GEMINI.md`, `CLAUDE.md` or `.github/copilot-instructions.md` contradict them. First fix: the `last_updated` contradiction in `documentation-policy.mdc`.
- **DOCS-DRIFT-03 (detect, prose vs code):** scan docs and agent files for `vox <cmd> [--flag]` mentions and "planned per …" claims. Fail when a mentioned command or flag is absent from `contracts/cli/command-registry.yaml`, or when a "planned" surface already exists there. First fixes: AGENTS.md `vox audit retirement` / `vox audit effort`.
- **DOCS-DRIFT-04 (detect, dead guards and orphans):** make guards that silently no-op when their input file is missing fail loudly (`check_docs_ssot` orphan inventory). Add an orphan-page report: a `docs/src` page no other page or nav entry links to, with zero reader-facing orphans (no baseline).
- **DOCS-DRIFT-05 (auto-correct, scheduled loops):** a scheduled job runs `vox audit docs --update-ledger` and the external link checker. It opens one draft PR with ledger refreshes and proposed `.lycheeignore` / link fixes, instead of a nightly-failure issue (#578). Decide whether to revive or retire the dormant docs-reality-audit program and the self-heal bot's draft-PR stage.
- **DOCS-DRIFT-06 (meta-gate):** a test asserts that every gate listed in docs-system-ssot §4 is wired where the table says (pre-push tier / `docs-quality.yml` / nightly). It also asserts §6 lists only gaps with no matching gate. The map can then never silently disagree with the machinery.

Plans:
**Wave 1**

- [ ] 343-01-PLAN.md — ssot-drift stage table + `ssot_regen_parity` (ssot-regen.vox and lefthook retry list checked against it); decision on the lefthook retry list (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 343-02-PLAN.md — overlay parity: AGENTS.md claims YAML checked against CLAUDE/GEMINI/copilot/.cursor overlays; `last_updated` and other contradictions fixed; decision on the build-environment cargo path (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 343-03-PLAN.md — CLI-claims stage (clap tree + registry): agent files gated, reader-facing pages measured; decision fix-vs-baseline for reader-facing defects (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [ ] 343-04-PLAN.md — fix every reader-facing CLI-claim defect and enforce the reader-facing scope (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 343-05-PLAN.md — dead guards removed or made fatal (`check_docs_ssot`, `check-links`, retired-symbol overlays); decision on expired link-allowlist entries (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 343-06-PLAN.md — corrected inbound-link graph + `docs_orphan_pages` stage; decision on orphan linking vs shrink-only baseline (wave 6)

**Wave 7** *(blocked on Wave 6 completion)*

- [ ] 343-07-PLAN.md — idempotent ledger refresh + `Docs Drift Refresh` draft-PR workflow; decisions on `.lycheeignore` scope and reality-audit/self-heal (wave 7)

**Wave 8** *(blocked on Wave 7 completion)*

- [ ] 343-08-PLAN.md — lychee triage script into the bot PR, reality-audit/self-heal decision applied, docs describe what runs (wave 8)

**Wave 9** *(blocked on Wave 8 completion)*

- [ ] 343-09-PLAN.md — `docs_system_parity` meta-gate: §4 wiring proven against the machinery, §6 lists only open gaps (wave 9)

### Phase 345: Kiln Synthetic Topic Trees & Vox Axis MENS Studio (`MENS-STUDIO-01..04`)

**Directory:** `.planning/phases/345-kiln-inspired-synthetic-topic-trees-vox-axis-mens-all-hubs-a/`
**Goal:** Port Kiln AI's high-leverage synthetic dataset mechanisms natively into Rust (`SpokeTopicTreeV1` recursive topic-tree expansion, Teacher→Student CoT distillation, and automatic Oracle-Fail→Teacher-Repair→DPO pair synthesis in `vox-corpus` without Python `kiln_ai` dependencies) and upgrade Vox Axis's `MensTrainingView` into the Option A "Ladder × Spokes Crucible Matrix" MENS Training Studio with real Rust Tauri backend commands (`mens_ladder_status`, `mens_save_custom_spoke`, `mens_save_crucible_config`, `mens_live_telemetry`, `explain_routing`), Clavis `HF_TOKEN` auto-sync (`1 auto 2 confirm`), live Orchestrator→Spoke routing visualization in Chat and MENS, and multi-resolution CV Before/After Flywheel verification (`1280×800`, `1440×900`, `1920×1080`).
**Requirements**: MENS-STUDIO-01, MENS-STUDIO-02, MENS-STUDIO-03, MENS-STUDIO-04
**Depends on:** Independent of Phases 342–344
**SSOT:** `docs/src/architecture/kiln-synthetic-data-and-mens-axis-gui-research-2026.md`
**UI-SPEC:** `.planning/phases/345-kiln-inspired-synthetic-topic-trees-vox-axis-mens-all-hubs-a/345-UI-SPEC.md`
**Plans:** 2 plans

Plans:

- [x] `345-01`: Option A MENS Crucible Studio UI, Clavis `HF_TOKEN` Settings deep-link, Chat Orchestrator→Spoke badge, and 8-pass multi-resolution Playwright CV Flywheel (`visualAfterScore: 0.9352`, `axeViolations: 0`, `overlappingControlPairs: 0`) (completed 2026-11-08)
- [x] `345-02`: Full-stack Rust Tauri backend plumbing (`mens_ladder_status` reading `mens/config/gpu-specs.yaml` & `mens/runs/size_train/*.json`, `mens_save_custom_spoke`, `mens_save_crucible_config`, `mens_live_telemetry` JSONL time-series spline hydration, `explain_routing` daemon integration, and CLI flag parity for `train`/`eval-local`/`eval-gate`/`serve`/`hub`) (completed 2026-11-08)

---

# Roadmap: v5.6 — Reasoning Where the Lane Needs It

**Milestone:** v5.6
**Status:** Phases 313–323 complete (25/25 requirements implemented and verified).
**Updated:** 2026-11-01

### Phase 313: Lane Prefix (`LANE-01`, `LANE-02`)

**Goal:** Codegen requests and codegen training rows use the non-thinking prefix, and hub and agent requests use the bare thinking-mode prefix.
**Depends on**: Nothing (first phase of v5.6)
**Requirements**: LANE-01, LANE-02
**Success Criteria** (what must be TRUE):

  1. A codegen serve prompt contains ` /no_think` and ends with the empty think block.
  2. A hub or agent serve prompt ends at a bare `assistant` turn and does not contain `/no_think`.
  3. A codegen direct-answer training row keeps the empty think block outside the loss, and a row that already has a non-empty think body is not rewritten with `/no_think`.

**Plans**: 4 plans

Plans:

**Wave 1**

- [x] `313-01`: Tracer: raw `/generate` carries `lane` through to the plugin payload, and an unknown lane is a 400 before any worker call (completed 2026-10-26)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] `313-02`: CUDA and Metal `assemble_prompt` wrap raw prompts by lane through one shared builder (completed 2026-10-26)
- [x] `313-03`: Chat completions accept `lane` and flatten the last user turn plus the assistant opener (completed 2026-10-26)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] `313-04`: Lock the existing codegen training fusion with tests only (completed 2026-10-26)

### Phase 314: Held-out Reasoning Set (`BENCH-02`, `BENCH-03`)

**Goal:** The same GSM8K, MATH-500, and Vox items exist for a before/after, and a missing model cannot write a passing score.
**Depends on**: Phase 313
**Requirements**: BENCH-02, BENCH-03
**Success Criteria** (what must be TRUE):

  1. A checked-in Vox slice has a checkable answer on every item and does not reuse the fixed Plan / Syntax / Implementation template.
  2. The GSM8K and MATH-500 slices used for the comparison are the checked-in files, not a new sample.
  3. A reasoning eval that cannot load the model writes no passing score and records how many items were scored.

**Plans**: 2 plans

Plans:

- [x] `314-01` (Wave 1): Check in the gsm8k_think, math500_think, and vox_think 32-item slices with a pinned loader and an interpreter oracle for Vox golds (completed 2026-10-26)
- [x] `314-02` (Wave 2, after 314-01): Fail closed when the model does not load or any inference error occurs; write `not_scored` with `n: 0`, and commit the scorer (completed 2026-10-26)

### Phase 315: Serve Gate (`GATE-01`, `GATE-02`)

**Goal:** Serve refuses an adapter whose collateral report is missing or worse on accuracy or think-initiation, and the report labels the benches as slices.
**Depends on**: Phase 314
**Requirements**: GATE-01, GATE-02
**Success Criteria** (what must be TRUE):

  1. Serve refuses to bind an adapter when the collateral report is missing.
  2. Serve refuses when accuracy or think-initiation on the reasoning keys drops past the configured threshold.
  3. The report states that the benches are slices and records the item count, so the 5% bar is not read as a full GSM8K or MATH-500 result.

**Plans**: 2 plans

Plans:

- [x] `315-01` (Wave 1): Strict per-slice evaluator in vox-eval; eval-collateral-damage requires all six reasoning keys, removes a stale report, and labels the slices with n (completed 2026-10-26)
- [x] `315-02` (Wave 2, after 315-01): Serve re-validates the on-disk report and the per-slice reasoning reports and refuses a missing, stale, edited, or unscored one (completed 2026-10-26)

### Phase 316: Before/After Comparison (`BENCH-01`)

**Goal:** A pinned Qwen3 base of any ladder size and its adapter are scored on the same slices when the weights exist, and no score is invented when they do not.
**Depends on**: Phase 315
**Requirements**: BENCH-01
**Success Criteria** (what must be TRUE):

  1. When the requested pinned Qwen3 snapshot (default `Qwen/Qwen3-8B`) and an adapter trained on it are on disk, one report holds accuracy and think-initiation for the GSM8K slice, the MATH-500 slice, and the Vox slice on both sides.
  2. When the snapshot is absent, the run records that fact and does not write numeric scores.
  3. Both sides use the checked-in items.

**Plans**: 1/1 plans executed

Plans:

- [x] 316-01-PLAN.md
- [x] `316-01`: Run the before/after on the checked-in slices, or record that the snapshot is absent (completed 2026-10-28)

### Phase 317: Size Ladder (`SIZE-01`)

**Goal:** Every Qwen3 dense size is a pinned, licensed rung that the trainer and the harness can name.
**Depends on**: Phase 316
**Requirements**: SIZE-01
**Success Criteria** (what must be TRUE):

  1. The ladder has Qwen3-0.6B, 1.7B, 4B, 8B, 14B, and 32B, each pinned to a real Hugging Face commit and carrying a license.
  2. A rung whose memory floor has not been measured on hardware says so.

**Plans:** 1/1 plans executed

Plans:

- [x] 317-01-PLAN.md — Pin 1.7B and 4B on the qwen3_code ladder and name every dense size from the overlay

### Phase 318: Live VRAM Selector (`SIZE-02`)

**Goal:** One selector reads current free memory, titrates to the largest Qwen3 size that fits, and picks the hub and its spokes together. Training, local serve, and chat all use it.
**Depends on**: Phase 317
**Requirements**: SIZE-02
**Success Criteria** (what must be TRUE):

  1. The selector reads current free memory (Metal live budget, CUDA free VRAM), not total memory, and picks the largest ladder size whose predicted footprint fits.
  2. If free memory drops between the pick and the load, it steps down a size and says why, instead of loading and running out of memory.
  3. Every spoke's base and preset follow the picked size. An explicit size overrides the pick, and a size that does not fit is refused rather than silently shrunk.
  4. The inference auto-selector and the training base pick both use this selector. The old fixed tiers that point at run directories that do not exist are gone.

**Plans:** 3/3 plans executed

Plans:

- [x] 318-01-PLAN.md — Measured QLoRA picker, pre-load step-down, and free-memory adapter (completed 2026-10-29)
- [x] 318-02-PLAN.md — One size for every spoke, and a re-read immediately before training load (completed 2026-10-29)
- [x] 318-03-PLAN.md — Chat and the GUI host call the selector; directory tiers are removed (completed 2026-10-29)

### Phase 319: Mesh-Wide Selection (`MESH-01`)

**Goal:** Across a mesh, the selector picks the largest size that fits on any node's current free memory and routes the work there.
**Depends on**: Phase 318
**Requirements**: MESH-01
**Success Criteria** (what must be TRUE):

  1. Node heartbeats carry current free accelerator memory, not only total VRAM.
  2. The mesh pick chooses the largest size that fits on some node, routes the job to that node, and titrates down when no node fits the larger size.
  3. A stale or missing node reading is not treated as free memory, and the local fallback still works with no mesh.

**Plans:** 3/3 plans executed

Plans:

- [x] 319-01-PLAN.md — Largest measured QLoRA floor, routed to a node that holds it (completed 2026-10-29)
- [x] 319-02-PLAN.md — Stale window, explicit-size refusal, and heartbeat free memory (completed 2026-10-29)
- [x] 319-03-PLAN.md — Local fallback when no node has a fresh reading (completed 2026-10-29)

### Phase 320: Train the Sizes (`SIZE-03`)

**Goal:** Each Qwen3 size that fits this Mac is trained with the lane prefix, and each size that does not is recorded as not trained.
**Depends on**: Phase 319
**Requirements**: SIZE-03
**Success Criteria** (what must be TRUE):

  1. Each size that fits produces a Metal QLoRA adapter whose manifest names its pinned base.
  2. A size that does not fit has a record with the measured memory and no adapter.

**Plans:** 2/2 plans executed

Plans:

- [x] 320-01-PLAN.md — One size_train.v1 record per qwen3_code QLoRA size, from an injected free-memory reading (completed 2026-11-01)
- [x] 320-02-PLAN.md — Register vox mens size-train and take that reading once (completed 2026-11-01)

### Phase 321: Harness Before/After (`HARN-01`, `BENCH-04`)

**Goal:** The harness serves a chosen size, and one report shows before and after reasoning and intelligence scores for every trained size.
**Depends on**: Phase 320
**Requirements**: HARN-01, BENCH-04
**Success Criteria** (what must be TRUE):

  1. The harness serves a chosen size's base or adapter through the existing serve path, behind the collateral gate.
  2. For every trained size, the report holds base and adapter reasoning accuracy and think-initiation on the three slices, plus an intelligence score on a codegen or agent bench, with n recorded.
  3. At least two sizes have real trained scores. Sizes without them are listed as not trained, with no numbers.

**Plans**: 1/1 plans executed

Plans:

- [x] `321-01`: Chosen-size serve resolution behind collateral gate (`HARN-01`) and multi-size `harness_before_after.v1` reasoning + intelligence report (`BENCH-04`) (completed 2026-11-01)

### Progress

| Phase | Name | Plans | Status |
|---|---|---|---|
| **Phase 313** | Lane Prefix | `4/4` | Complete |
| **Phase 314** | Held-out Reasoning Set | `2/2` | Complete |
| **Phase 315** | Serve Gate | `2/2` | Complete |
| **Phase 316** | Before/After Comparison | `1/1` | Complete    2026-10-28 |
| **Phase 317** | Size Ladder | `1/1` | Complete    2026-10-28 |
| **Phase 318** | Live VRAM Selector | `3/3` | Complete    2026-10-29 |
| **Phase 319** | Mesh-Wide Selection | `3/3` | Complete    2026-10-29 |
| **Phase 320** | Train the Sizes | `2/2` | Complete    2026-11-01 |
| **Phase 321** | Harness Before/After | `1/1` | Complete    2026-11-01 |
| **Phase 322** | Calibrated turn decisions + ChatARC | `1/1` | Complete    2026-11-01 |
| **Phase 323** | High-Value Starlight MDX Conversions + Ingest Parity | `2/2` | Complete    2026-11-01 |

### Phase 322: Calibrated turn decisions and dynamic action catalog (`DEC-01..04`, `CAT-01..02`)

**Directory:** `phases/322-calibrated-turn-decisions-and-dynamic-action-catalog/`
**Goal:** Fully plumb System One / RLCD-style calibrated decisions into the live model-route and agent-loop control points (no facade). ChatARC composes the catalogs the harness already discovers (skills, MCP, clap commands) and chooses among shortlists. Complements `models::select`, clutch/risk axes, SIZE/MESH selectors, and MENS spokes — does not replace them or pretend `allow_free_in_performance_mode` is PreferFree.
**Depends on**: Nothing in 317–321 (parallel). Must not fork SIZE-02 / MESH-01. Primary insertion is `llm_bridge/model_route_policy/resolve.rs`, plus repair escalation and stop/post-tool hooks.
**Requirements**: DEC-01, DEC-02, DEC-03, DEC-04, CAT-01, CAT-02
**Success Criteria** (what must be TRUE):

  1. Turn-start pack feeds `build_selection_request` without overriding clutch-derived axes or `allow_free` when a clutch profile is set; flag off matches today's request bytes-for-bytes on fixtures.
  2. `allow_free_in_performance_mode` is documented and tested as scorer eligibility only — `free_floor` slug order stays in `bootstrap_inference` / `FREE_FLOOR`.
  3. Post-tool low confidence reuses `apply_repair_escalation_to_config`; stop/verify stay on `evaluate_stop_hook` / `evaluate_post_tool_batch_hook`.
  4. ChatARC shortlist Choice is plumbed into an existing activation path and stays correct when a new skill, MCP tool, or clap command appears; selective-prediction metrics published; hosted Jev behind local-only privacy.

**Plans**: 1/1 plans executed

**Research / prior art:** [`docs/src/architecture/jev-system-one-harness-integration-research-2026.md`](../docs/src/architecture/jev-system-one-harness-integration-research-2026.md) (§§11–16); cursor plan `labeled_router_plus_jev` folded here (do not execute as a standalone facade).

Plans:

- [x] `322-01`: Pre-turn `TurnDecisionPack` (`DEC-01`), post-tool confidence escalation (`DEC-02`), flag-off byte-identity tests (`DEC-03`), selective-prediction & privacy gate (`DEC-04`), and ChatARC top-K shortlist + calibrated Choice (`CAT-01..02`) (completed 2026-11-01)

### Phase 323: High-Value Starlight MDX Conversions and Ingest Parity (`MDX-INGEST-01..03`, `MDX-CONV-01..03`)

**Directory:** `phases/323-high-value-starlight-mdx-conversions-and-ingest-parity/`
**Goal:** Make `.mdx` a first-class, zero-regression citizen across `vox-kb-facets`, `vox-search` (Library index, `kb_ids`, RAG `ingest_markdown_tree`, `chunk_document`), `vox-doc-pipeline`, and MENS training extraction; then convert high-value tutorials, onboarding guides, and flagship language reference pages to Starlight MDX (`<Steps>`, `<Tabs>`, `<CardGrid>`, `<VoxPlayground>`) while keeping prose-heavy architecture/ADR/research docs in `.md`.
**Depends on:** Nothing (parallel documentation and search track)
**Requirements:** `MDX-INGEST-01`, `MDX-INGEST-02`, `MDX-INGEST-03`, `MDX-CONV-01`, `MDX-CONV-02`, `MDX-CONV-03`
**Success Criteria** (what must be TRUE):

  1. `vox-kb-facets` (`inspect_summary`, `is_summary_marker_line`, `strip_summary_blocks`) accepts both HTML (`<!-- kb:summary -->`) and MDX JSX (`{/* kb:summary */}`) markers and ignores top-level ESM `import` lines between `# H1` and the summary block.
  2. `vox-search::kb_ids::is_kb_corpus_path`, `doc_title`, and `ingest::ingest_markdown_tree` index both `.md` and `.mdx` under `docs/src/`; `library::chunk::first_paragraph` and MENS corpus extraction strip ESM `import`/`export` statements and JSX layout wrapper tags (`<Tabs>`, `<TabItem>`, `<Steps>`, `<CardGrid>`, `{/* ... */}`) so Tantivy cards and training chunks never leak JSX boilerplate.
  3. `vox-doc-pipeline` validates `.mdx` files (frontmatter, facets, `vox` doctest fences, internal `.md`/`.mdx` links) and upgrades `docs/src/index.mdx` to clean Starlight components (`<CardGrid>`, `<LinkButton>`) without inline-style HTML soup.
  4. High-value tutorials (`docs/src/tutorials/**`) and flagship reference/getting-started pages are converted to `.mdx` using Starlight `<Steps>`, `<Tabs>` (showing Vox source alongside generated SQL/Rust/TSX), and embedded `<VoxPlayground>`, passing `pnpm --dir docs-astro build`, `vox-doc-pipeline --lint-only`, and `cargo test -p vox-search -p vox-kb-facets -p vox-doc-pipeline`.

**Plans:** 2/2 plans executed

Plans:

- [x] `323-01`: Dual HTML/MDX summary markers and L0 MDX boilerplate stripping across `vox-kb-facets`, `vox-search`, `vox-doc-pipeline`, and `vox-corpus` (`MDX-INGEST-01..03`) (completed 2026-11-01)
- [x] `323-02`: Upgrade `docs/src/index.mdx`, add `.md` ↔ `.mdx` link fallback, and ship interactive Starlight MDX tutorial `tut-fullstack-walkthrough.mdx` (`MDX-CONV-01..03`) (completed 2026-11-01)

---

# Roadmap: v5.5 — T3 CI/TOESTUB Deep Enforcement (`Phases 307–312`) — SHIPPED

**Milestone:** v5.5
**Status:** Planning complete — ready for `/gsd-plan-phase 307`
**Updated:** 2026-10-26T01:59:06Z

### Phase 307: PR Vouch & Contributor Trust Gate (`T3-TRUST-01..04`)

**Directory:** `phases/307-pr-vouch-and-contributor-trust/`
**Goal:** Add `.github/VOUCHED.td` (or `contracts/ci/vouched.v1.yaml`) SSOT for trusted/denounced GitHub logins, documented in AGE…

- [x] `307-01`: Implement T3-TRUST requirements

### Phase 308: Preview/Privileged Workflow Trust-Split Policy (`T3-PREVIEW-01..04`)

**Directory:** `phases/308-preview-trust-split-policy/`
**Goal:** Author `docs/src/ci/privileged-preview-trust-split.md` capturing t3code relay pattern: deploy may run PR code only under…

- [x] `308-01`: Implement T3-PREVIEW requirements

### Phase 309: Live Transfer-Budget Measurement & Job-Matrix Change Detector (`T3-LIVE-CI-01..04`)

**Directory:** `phases/309-live-transfer-budget-and-job-matrix/`
**Goal:** Replace fixture-only `transfer-budget-report` CI path with artifact emission from real `vox-orchestrator-mcp` transfer-b…

- [x] `309-01`: Implement T3-LIVE-CI requirements

### Phase 310: TOESTUB Test-Discipline Detectors (Wall-Clock, Real-Wait, Loop, Frame) (`T3-TEST-DET-01..04`)

**Directory:** `phases/310-test-discipline-detectors/`
**Goal:** Extend `test_clock_and_skip` (or new detector) to flag wall-clock reads in tests: `Instant::now`, `SystemTime::now`, `Da…

- [x] `310-01`: Implement T3-TEST-DET requirements

### Phase 311: Host-Runtime Boundary, Schema Hoist, Knip Dual & DOM-Env Discipline (`T3-HOST-SCHEMA-01..04`)

**Directory:** `phases/311-host-runtime-schema-knip-dom/`
**Goal:** New detector banning direct host platform/arch probes outside allowlisted host crates (`std::env::consts::{OS,ARCH}`, `p…

- [x] `311-01`: Implement T3-HOST-SCHEMA requirements

### Phase 312: Sidecar Fingerprint Hash Compare, Surface Graph Regen & v5.5 Scorecard (`T3-FP-GRAPH-01..04`)

**Directory:** `phases/312-fingerprint-hash-graph-scorecard/`
**Goal:** Upgrade `gui-sidecar-fingerprint.yml` from path-diff labeling to content hash of native/sidecar inputs (Cargo.lock subse…

- [x] `312-01`: Implement T3-FP-GRAPH requirements

### Progress

| Phase | Name | Plans | Status |
|---|---|---|---|
| **Phase 307** | PR Vouch & Contributor Trust Gate | `1/1` | Complete |
| **Phase 308** | Preview/Privileged Workflow Trust-Split Policy | `1/1` | Complete |
| **Phase 309** | Live Transfer-Budget Measurement & Job-Matrix Change Detector | `1/1` | Complete (Partials) |
| **Phase 310** | TOESTUB Test-Discipline Detectors (Wall-Clock, Real-Wait, Loop, Frame) | `1/1` | Complete |
| **Phase 311** | Host-Runtime Boundary, Schema Hoist, Knip Dual & DOM-Env Discipline | `1/1` | Complete (Partials) |
| **Phase 312** | Sidecar Fingerprint Hash Compare, Surface Graph Regen & v5.5 Scorecard | `1/1` | Complete (Partials) |

---

## Milestones

- ✅ **v5.3 (`Phases 295–300`, `24/24` requirements — Complete 2026-10-25; CV residual):** Photorealistic Edge-Softening, Continuous Monumental Aqueduct-to-Sewer Architecture, Sub-Surface Ground Integration & CV Flywheel Score Maximization — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-photorealistic-edge-softening-and-cv-maximization-spec-2026.md)
- ✅ **v5.2 (`Phases 289–294`, `24/24` requirements — Complete 2026-10-25):** Smooth Physically-Grounded Lighting, High-Detail Vitruvian Architecture & Grounds, Integrated Roman Hydraulic/Sewer Grid (`Aquaeductus`, `Castellum Divisorium`, `Fistulae` & `Cloaca Maxima`) & Animated Shape+Color Activity Conduits — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-hydraulic-plumbing-and-detailed-architecture-spec-2026.md)
- ✅ **v5.1 (`Phases 283–288`, `24/24` requirements — Complete 2026-10-25):** T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure: System Prompt `$variable` & Worktree Branch Fix, Full `context_sanitizer` & `ToolRefusalDetector`, Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` & `ModelSpec` Cost Accounting, Async Worktree Lease & Monotonic Sparse Checkpoints, Real `MultiVariantWorktreePool`, Multi-Provider Consensus Synthesizer, Depth-4 External Session Discovery, Unified Tauri/MCP `message_id` & `oxlint-plugin-t3code` Zero-Violation Audit Parity — [Requirements](REQUIREMENTS.md) | [Architecture Audit](../docs/src/architecture/t3-vox-wave5-deep-review-and-parity-closure-2026.md)
- ✅ **v5.0 (`Phases 277–282`, `24/24` requirements — Complete 2026-10-25):** Photorealistic Vitruvian Material, Depth-Shading, Atmospheric Lighting & FX Engine (`pbrMaterials.ts`, Projected Cast Shadows, Vertical AO, Specular Chamfer Bevels, Bathymetric Sea + Surf Foam, Crepuscular God-Rays, Additive GI Pools, Particle FX & Diegetic Cliff-Apron Plaques) — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-photorealistic-lighting-and-materials-spec-2026.md)
- ✅ **v4.9 (`Phases 271–276`, `24/24` requirements — Complete 2026-10-25):** Vitruvian Roman Tactical Developer Controller: Zero-Hop Diff/Diagnostic Loupe (*Speculum Codicis*), Multi-Agent Write-Set Collision Cones (*Concursus Legionum*), Clickable Viaduct Contract Inspector (*Pons Foederis*), Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*), Milestone Road Turn Time-Scrubber (*Via Temporis*) & Split-Dock Workbench Integration (*Tabula Praetoria `⌘M`*) — [Requirements](REQUIREMENTS.md) | [Architecture Spec](../docs/src/architecture/urbs-tactical-developer-controller-spec-2026.md)
- ✅ **v4.8 (`Phases 265–270`, `24/24` requirements — Complete 2026-10-25):** T3 Wave 4 Live Pipeline End-to-End Closure: Real LLM Consensus, Stateful Session/Guest/Checkpoint Persistence, Live Agent-Loop Guards & Tauri/MCP Envelope Unwrapping — [Requirements](REQUIREMENTS.md) | [Architecture Audit](../docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md)
- ✅ **v4.7 (`Phases 259–264`, `24/24` requirements — Shipped 2026-10-25):** Universal Repo-Agnostic Codebase Civilization Simulator & Attention-Weighted Developer Control Surface — [Archive](milestones/v4.7-ROADMAP.md) | [Requirements](milestones/v4.7-REQUIREMENTS.md) | [Audit](v4.7-MILESTONE-AUDIT.md)
- ✅ **v4.6 (`Phases 253–258`, `24/24` requirements — Shipped 2026-10-25):** T3 Wave 3 Live Surface & Daemon RPC Wiring, Router/DPO Quality-Prior Flywheel & Workspace-Wide TOESTUB Remediation — [Archive](milestones/v4.6-ROADMAP.md) | [Requirements](milestones/v4.6-REQUIREMENTS.md) | [Audit](v4.6-MILESTONE-AUDIT.md)
- ✅ **v4.5 (`Phases 242–252`, `44/44` requirements — Shipped 2026-10-25):** T3 Ecosystem Wave 2 Advancements, Consensus Engine, Git Plumbing Checkpoints & TOESTUB Static Enforcement — [Archive](milestones/v4.5-ROADMAP.md) | [Requirements](milestones/v4.5-REQUIREMENTS.md)
- ✅ **v4.4 (`Phases 236–241`, `24/24` requirements — Shipped 2026-10-25):** Concept-Art Pixel & Structural Convergence, Live Animated Roman Civilization Simulator (`VRBS ROMA`), Interactive DAG/Diff Workbench & Before/After Visual Proof — [Archive](milestones/v4.4-ROADMAP.md) | [Requirements](milestones/v4.4-REQUIREMENTS.md)
- ✅ **v4.3 (`Phases 230–235`, `24/24` requirements — Shipped 2026-10-25):** Roman Civilization Codebase Simulator (`VRBS ROMA`), Strategic Minimap Radar & Cross-Surface Readability/Occlusion Polish — [Requirements](milestones/v4.3-REQUIREMENTS.md) (no roadmap archive was written)

---

## Phases (`v4.8`)

### Phase 265: `tool:<name>` Fence Salvage Fix, Live `@rule:<name>` Mention Resolution & `$variable` System Prompt Interpolation (`T3-SALVAGE-RULES-01..04`)

**Goal:** Eliminate the `tool:<name>` stream-suppression vs. salvage mismatch in `salvage_and_stream.rs`, wire `resolve_user_rule_mentions` (`@rule:<name>` + `Always` rules) into `resolve_mentions` before file-path matching, and wire `interpolate_system_prompt_variables` into live turn preparation in `preamble_and_retrieval.rs`.
**Requirements:** `T3-SALVAGE-RULES-01`, `T3-SALVAGE-RULES-02`, `T3-SALVAGE-RULES-03`, `T3-SALVAGE-RULES-04`
**Plans:**

- [x] `265-01`: Fix `tool:<name>` fence salvage in `salvage_and_stream.rs`, wire `@rule:<name>` resolution in `mentions.rs`, wire `$variable` interpolation in `preamble_and_retrieval.rs`, and add integration tests (`phases/265-salvage-rules-and-prompt-vars/265-01-PLAN.md`)

### Phase 266: Live `context_sanitizer` History Stripping, `vox_chat_compact_up_to`, `BatteryCostLedger` Accounting, `VoxIdAllocatorV2` Message IDs & Real `ResponseQualityAnalyzer` Telemetry (`T3-HIST-BATTERY-01..04`)

**Goal:** Wire `sanitize_assistant_for_history` and `escape_prompt_injection_tokens` into `transcript_entry_to_llm_message` (`conversation.rs`), implement `vox_chat_compact_up_to` (`history.rs`), and wire live `BatteryCostLedger` (`check_preflight_balance` + `record_turn_usage`), `VoxIdAllocatorV2` deterministic message IDs, and real `ResponseQualityAnalyzer` scores into `message.rs` and `persist_and_envelope.rs`.
**Requirements:** `T3-HIST-BATTERY-01`, `T3-HIST-BATTERY-02`, `T3-HIST-BATTERY-03`, `T3-HIST-BATTERY-04`
**Plans:**

- [x] `266-01`: Wire `context_sanitizer` in `conversation.rs`, implement `vox_chat_compact_up_to` in `history.rs`, and wire `BatteryCostLedger`, `VoxIdAllocatorV2`, and `ResponseQualityAnalyzer` in `message.rs` & `persist_and_envelope.rs` (`phases/266-history-sanitizer-and-battery-telemetry/266-01-PLAN.md`)

### Phase 267: Live `agent_loop` Integration: `CommandOutputHub` `\r` Normalization, `20 KiB` DOM Shedding, Pre-Write Sparse Git Checkpoints, `ResumableTurnStreamRegistry` & Mid-Turn Steer Draining (`T3-LOOP-CHECKPOINT-01..04`)

**Goal:** Wire `normalize_terminal_output` (`\r` folding), `bound_snapshot_metadata` (`20 KiB` DOM shedding), automatic pre-write sparse Git checkpoints (`should_sparse_checkpoint`), `ResumableTurnStreamRegistry` (`vox_chat_stream_resume`), and mid-turn `ThreadRunQueue` (`vox_chat_queue_steer`) into `agent_loop.rs` and `salvage_and_stream.rs`.
**Requirements:** `T3-LOOP-CHECKPOINT-01`, `T3-LOOP-CHECKPOINT-02`, `T3-LOOP-CHECKPOINT-03`, `T3-LOOP-CHECKPOINT-04`
**Plans:**

- [x] `267-01`: Wire `\r` normalization, `20 KiB` DOM shedding, pre-write sparse Git checkpoints, `ResumableTurnStreamRegistry`, and mid-turn `ThreadRunQueue` steer draining into `agent_loop.rs` & `salvage_and_stream.rs` (`phases/267-agent-loop-checkpoints-and-resumable-streams/267-01-PLAN.md`)

### Phase 268: Stateful `t3_wave3_tools.rs`: Concurrent `join_all` LLM Consensus Fan-Out, Live Transcript Hydration/Persistence for `vox_context_handoff` & `vox_session_*` & Persistent `GuestShareService` (`T3-STATEFUL-TOOLS-01..04`)

**Goal:** Upgrade `t3_wave3_tools.rs` so `vox_consensus_run` dispatches concurrent `join_all` LLM calls across models, `vox_context_handoff` and `vox_session_*` hydrate and persist real transcripts in `ServerState` (`chat_history:{session_id}` + real `~/.claude`/`~/.codex`/`~/.cursor` files), and `vox_guest_share_*` uses a persistent `GuestShareService` on `ServerState`.
**Requirements:** `T3-STATEFUL-TOOLS-01`, `T3-STATEFUL-TOOLS-02`, `T3-STATEFUL-TOOLS-03`, `T3-STATEFUL-TOOLS-04`
**Plans:**

- [x] `268-01`: Upgrade `t3_wave3_tools.rs` to dispatch concurrent `join_all` LLM consensus legs, hydrate/persist live `ServerState` transcripts in `vox_context_handoff` & `vox_session_*`, and persist `GuestShareService` on `ServerState` (`phases/268-stateful-consensus-handoff-and-session-tools/268-01-PLAN.md`)

### Phase 269: `vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools, `DelegationWorktree` Lease/Env Wiring, `submit_webhook_events` Dedupe/Rate Gate & Compact `vox_orchestrator_status` (`T3-REGEN-WORKTREE-01..04`)

**Goal:** Implement `vox_chat_regenerate` and `vox_chat_edit_message_in_place` in `history.rs`, wire `WorkspaceLeaseMap` + `provision_untracked_env_files` in `DelegationWorktree::create`, wire `WebhookDedupeAndRateGate` in `submit_webhook_events`, and wire `compact` in `vox_orchestrator_status` + `transfer_budget.rs`.
**Requirements:** `T3-REGEN-WORKTREE-01`, `T3-REGEN-WORKTREE-02`, `T3-REGEN-WORKTREE-03`, `T3-REGEN-WORKTREE-04`
**Plans:**

- [x] `269-01`: Implement `vox_chat_regenerate` & `vox_chat_edit_message_in_place`, wire `WorkspaceLeaseMap` & `.env` provisioning in `agy_worktree.rs`, wire `WebhookDedupeAndRateGate` in `webhook_intake.rs`, and wire `compact` in `vox_orchestrator_status` & `transfer_budget.rs` (`phases/269-regenerate-worktree-lease-and-webhook-gate/269-01-PLAN.md`)

### Phase 270: `t3Wave3Rpc.ts` Nested Tauri Envelope Unwrapping, Live `ChatTranscript.tsx` In-Place Edit & Posture-Aware Regeneration & Full-Stack Verification (`T3-GUI-ENVELOPE-01..04`)

**Goal:** Fix `unwrapMcpData` in `t3Wave3Rpc.ts` to unwrap nested Tauri `{ is_error, result: { success, data } }` and stringified MCP envelopes, expose typed `editChatMessageInPlace` / `regenerateChatTurn` / `compactChatUpTo` / `resumeChatStream` / `steerChatQueue` client helpers, wire in-place edit and posture regeneration in `ChatTranscript.tsx`, and verify all Rust, Vitest, TypeScript, and audit gates.
**Requirements:** `T3-GUI-ENVELOPE-01`, `T3-GUI-ENVELOPE-02`, `T3-GUI-ENVELOPE-03`, `T3-GUI-ENVELOPE-04`
**Plans:**

- [x] `270-01`: Fix `unwrapMcpData` nested Tauri/MCP envelope unwrapping in `t3Wave3Rpc.ts`, wire in-place edit & regenerate in `ChatTranscript.tsx`, and run full-stack verification (`phases/270-tauri-envelope-unwrapping-and-transcript-actions/270-01-PLAN.md`)

---

## Progress Table (`v4.8`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 265** | `tool:<name>` Fence Salvage Fix, Live `@rule:<name>` Mention Resolution & `$variable` System Prompt Interpolation | `1/1` | Complete (`2026-10-25`) |
| **Phase 266** | Live `context_sanitizer` History Stripping, `vox_chat_compact_up_to`, `BatteryCostLedger` Accounting, `VoxIdAllocatorV2` Message IDs & Real `ResponseQualityAnalyzer` Telemetry | `1/1` | Complete (`2026-10-25`) |
| **Phase 267** | Live `agent_loop` Integration: `CommandOutputHub` `\r` Normalization, `20 KiB` DOM Shedding, Pre-Write Sparse Git Checkpoints, `ResumableTurnStreamRegistry` & Mid-Turn Steer Draining | `1/1` | Complete (`2026-10-25`) |
| **Phase 268** | Stateful `t3_wave3_tools.rs`: Concurrent `join_all` LLM Consensus Fan-Out, Live Transcript Hydration/Persistence for `vox_context_handoff` & `vox_session_*` & Persistent `GuestShareService` | `1/1` | Complete (`2026-10-25`) |
| **Phase 269** | `vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools, `DelegationWorktree` Lease/Env Wiring, `submit_webhook_events` Dedupe/Rate Gate & Compact `vox_orchestrator_status` | `1/1` | Complete (`2026-10-25`) |
| **Phase 270** | `t3Wave3Rpc.ts` Nested Tauri Envelope Unwrapping, Live `ChatTranscript.tsx` In-Place Edit & Posture-Aware Regeneration & Full-Stack Verification | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v4.9` — Vitruvian Roman Tactical Developer Controller)

### Phase 271: Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*) (`URBS-LOUPE-01..04`)

**Goal:** Eliminate context-switching between `VRBS` and external diff/terminal tabs by adding `workspace_file_diff_and_diagnostics` in `workspace_town.rs` and rendering `SpeculumLoupePopover.tsx` directly above any clicked or hovered Treadwheel Crane (`+LoC / -LoC`) or red error building (`N ERR`) with real unified Git diff hunks, `file:line:col` compiler diagnostics, and 1-click `[✓ Stage File]`, `[↺ Revert File]`, and `[⚒ Fix Exact Error]` actions.
**Requirements:** `URBS-LOUPE-01`, `URBS-LOUPE-02`, `URBS-LOUPE-03`, `URBS-LOUPE-04`
**Plans:**

- [x] `271-01`: Implement `workspace_file_diff_and_diagnostics` + `workspace_stage_file` / `workspace_revert_file` in `workspace_town.rs`, build `SpeculumLoupePopover.tsx`, wire into `LudusSandbox.tsx`, and add Rust/Vitest tests (`phases/271-inline-diff-and-diagnostic-loupe/271-01-PLAN.md`)

### Phase 272: Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*) (`URBS-COLLISION-01..04`)

**Goal:** Prevent concurrent multi-agent merge conflicts and semantic clobbering by computing each active agent's write-set + 1-hop blast-radius cone (`collisionRadar.ts`), rendering intersecting Roman surveyor groma cones and a 3D **Hammered-Bronze Crossed-Swords Collision Standard (`⚠ WRITE-SET COLLISION`)** in `worldRenderer.ts`, and wiring 1-click `[⇄ Serialize Agents]` and `[🌿 Split Worktree]` actions.
**Requirements:** `URBS-COLLISION-01`, `URBS-COLLISION-02`, `URBS-COLLISION-03`, `URBS-COLLISION-04`
**Plans:**

- [x] `272-01`: Build `collisionRadar.ts`, render surveyor groma cones & Crossed-Swords Collision Standard in `worldRenderer.ts`, wire `[⇄ Serialize Agents]` & `[🌿 Split Worktree]` in `LudusSandbox.tsx` / `ImperiumInspector.tsx`, and add Vitest tests (`phases/272-multi-agent-collision-cone-radar/272-01-PLAN.md`)

### Phase 273: Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*) (`URBS-VIADUCT-01..04`)

**Goal:** Turn stone viaducts between districts into clickable first-class architectural contract boundaries by scanning cross-module dependencies and `layers.toml` layer assignments in `workspace_town.rs` (`TownDependencyEdge`), adding viaduct hit-testing in `layout.ts`, rendering a **Cracked Red Arch Keystone (`⚠ LAYER VIOLATION`)** in `worldRenderer.ts` on illegal upward dependencies, and adding the `PONS CONTRACTVS` inspector panel in `ImperiumInspector.tsx`.
**Requirements:** `URBS-VIADUCT-01`, `URBS-VIADUCT-02`, `URBS-VIADUCT-03`, `URBS-VIADUCT-04`
**Plans:**

- [x] `273-01`: Add `TownDependencyEdge` + `layers.toml` violation scanner in `workspace_town.rs`, viaduct hit-testing in `layout.ts`, cracked keystone rendering in `worldRenderer.ts`, and `PONS CONTRACTVS` inspector panel in `ImperiumInspector.tsx` (`phases/273-clickable-viaduct-contract-inspector/273-01-PLAN.md`)

### Phase 274: Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) (`URBS-QUARANTINE-01..04`)

**Goal:** Give developers a physical spatial guardrail that prevents agents from modifying off-limits modules by storing `quarantined_paths` on `ServerState`, exposing `vox_quarantine_set` / `vox_quarantine_list` MCP & Tauri RPCs, enforcing pre-execution write rejection in `agent_loop.rs`, and rendering a 3D **Wrought-Iron Perimeter Portcullis & Bronze Padlock (`🔒 QUARANTINED: READ-ONLY`)** toggled via `Alt+Click` on any district.
**Requirements:** `URBS-QUARANTINE-01`, `URBS-QUARANTINE-02`, `URBS-QUARANTINE-03`, `URBS-QUARANTINE-04`
**Plans:**

- [x] `274-01`: Add `quarantined_paths` to `ServerState`, expose `vox_quarantine_set` / `vox_quarantine_list`, enforce write rejection in `agent_loop.rs`, and render 3D perimeter portcullis fencing in `worldRenderer.ts` (`phases/274-hard-agent-quarantine-fencing/274-01-PLAN.md`)

### Phase 275: Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`) (`URBS-SCRUBBER-01..04`)

**Goal:** Turn the Stone Milestone Road (`I, II, III, IV`) at `PORTA GIT` into an interactive visual `git bisect` timeline bar (`ViaTemporisScrubber.tsx`) with a draggable Bronze Sundial Gnomon Scrubber that previews per-checkpoint building diffs (`+LoC / -LoC`), test shield state flips (`PASS` → `FAIL`), and token/BU spend, with 1-click `[↺ Rollback Worktree to Milestone]` wired to `vcsCheckpointRestore`.
**Requirements:** `URBS-SCRUBBER-01`, `URBS-SCRUBBER-02`, `URBS-SCRUBBER-03`, `URBS-SCRUBBER-04`
**Plans:**

- [x] `275-01`: Add `vox_vcs_checkpoint_diff_summary` RPC, build `ViaTemporisScrubber.tsx` sundial timeline bar, wire historical building diff preview & 1-click rollback in `LudusSandbox.tsx`, and add Vitest tests (`phases/275-milestone-road-time-scrubber/275-01-PLAN.md`)

### Phase 276: Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*) (`URBS-DOCK-01..04`)

**Goal:** Promote `VRBS` from `Settings → Gamify` to a 1-click split-dock Tactical Map (`[🏛 Tactical Map ⌘M]`) inside `DockWorkspaceShell.tsx` and `ChatSurface.tsx`, wiring bi-directional district context filtering between the map and Chat/Review plus real-time tool-call mechanism animations (`read_file` → Screw Pump, `write_file` → Treadwheel Crane, `test` → Shield Wall, approval → Portcullis Gate).
**Requirements:** `URBS-DOCK-01`, `URBS-DOCK-02`, `URBS-DOCK-03`, `URBS-DOCK-04`
**Plans:**

- [x] `276-01`: Wire `[🏛 Tactical Map ⌘M]` split-dock panel in `DockWorkspaceShell.tsx` & `ChatSurface.tsx`, bi-directional district filter in `useAppStore.ts`, live tool-call mechanism pulses in `LudusSandbox.tsx`, and Playwright CV Flywheel verification (`phases/276-split-dock-tactical-map-sync/276-01-PLAN.md`)

---

## Progress Table (`v4.9`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 271** | Zero-Hop Inline Diff & Compiler Diagnostic Loupe (*Speculum Codicis*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 272** | Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 273** | Clickable Viaduct Cross-Module Contract & Layer-Inversion Inspector (*Pons Foederis*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 274** | Hard Agent Scope Quarantine Fencing (*Clausura Ferrea*) | `1/1` | Complete (`2026-10-25`) |
| **Phase 275** | Interactive Milestone Road Turn Time-Scrubber (*Via Temporis* Visual `git bisect`) | `1/1` | Complete (`2026-10-25`) |
| **Phase 276** | Split-Dock Tactical Map Workbench Integration (*Tabula Praetoria `⌘M`*) | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v5.0` — Photorealistic Vitruvian Material, Depth-Shading, Atmospheric Lighting & FX Engine)

### Phase 277: Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`) (`URBS-MAT-01..04`)

**Goal:** Eliminate flat solid-color polygon fills by creating a deterministic offscreen procedural material synthesis atlas (`pbrMaterials.ts`) producing tileable `CanvasPattern`s and metallic gradients for Pitted Roman Travertine Ashlar, Scalloped Terracotta Roof Tiles, Opus Incertum Flagstone Plazas, Stratified Basalt Cliffs, and Anisotropic Hammered Bronze.
**Requirements:** `URBS-MAT-01`, `URBS-MAT-02`, `URBS-MAT-03`, `URBS-MAT-04`
**Plans:**

- [ ] `277-01`: Create `pbrMaterials.ts` procedural natural surface material atlas + unit tests (`phases/277-procedural-natural-materials/277-01-PLAN.md`)

### Phase 278: True Isometric Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels (`URBS-DEPTH-01..04`)

**Goal:** Ground every 3D structure with physical depth by adding projected directional twilight cast shadows (`drawIsoCastShadow`), vertical height-shaded face gradients (`contact AO` at base → `golden-hour rim light` at cornice) overlaid with travertine ashlar texture, and physical specular chamfer ridge highlights replacing flat black cartoon outlines.
**Requirements:** `URBS-DEPTH-01`, `URBS-DEPTH-02`, `URBS-DEPTH-03`, `URBS-DEPTH-04`
**Plans:**

- [ ] `278-01`: Implement projected directional cast shadows, vertical AO/rim gradients, and specular chamfer bevels in `worldRenderer.ts` (`phases/278-depth-shading-and-cast-shadows/278-01-PLAN.md`)

### Phase 279: Sculpted Coastal Bathymetry, Surf Foam Ribbons, Craggy Cliff Geology & Cobblestone Terraces (`URBS-TERRAIN-01..04`)

**Goal:** Transform the flat background and terraces into a natural Mediterranean peninsula with multi-stop bathymetric sea depth gradients, coastal surf foam ribbons, stratified basalt cliff geology with vertical fissures and moss ledges, and cobblestone flagstone plazas.
**Requirements:** `URBS-TERRAIN-01`, `URBS-TERRAIN-02`, `URBS-TERRAIN-03`, `URBS-TERRAIN-04`
**Plans:**

- [ ] `279-01`: Upgrade `paintFullBleedRomanLandscape` with bathymetric sea depth, coastal surf foam, stratified cliff textures, and cobblestone terrace paving (`phases/279-bathymetry-cliffs-and-terrace-geology/279-01-PLAN.md`)

### Phase 280: Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric God-Rays, Elevation Mist & Additive GI Pools (`lightingAndFx.ts`) (`URBS-LIGHT-01..04`)

**Goal:** Deliver cinematic atmospheric lighting by creating `lightingAndFx.ts` with inter-terrace atmospheric elevation mist, diagonal golden-hour crepuscular god-rays, additive point-light GI bounce pools around braziers/forges/windows/aqueducts, and a cinematic vignette & warm highlight bloom pass.
**Requirements:** `URBS-LIGHT-01`, `URBS-LIGHT-02`, `URBS-LIGHT-03`, `URBS-LIGHT-04`
**Plans:**

- [ ] `280-01`: Create `lightingAndFx.ts` atmospheric mist, volumetric god-rays, additive GI light pools, and cinematic post-processing (`phases/280-atmospheric-lighting-and-god-rays/280-01-PLAN.md`)

### Phase 281: Live Particle FX Engine (Forge Embers, Volumetric Smoke Plumes, Waterfall Spray & Mason Chisel Sparks) (`URBS-FX-01..04`)

**Goal:** Bring the 3D mechanisms to life with deterministic real-time particle emitters in `lightingAndFx.ts` and `drawAnimatedWorldEffects`: rising glowing forge/error embers, drifting volumetric smoke plumes, waterfall mist spray, harbor water caustics, and mason chisel sparks on active Treadwheel Cranes.
**Requirements:** `URBS-FX-01`, `URBS-FX-02`, `URBS-FX-03`, `URBS-FX-04`
**Plans:**

- [ ] `281-01`: Implement live forge ember, smoke plume, waterfall spray, water caustic, and mason chisel spark particle emitters (`phases/281-live-particle-fx-engine/281-01-PLAN.md`)

### Phase 282: Diegetic Cliff-Apron Architrave Plaques, Architecture-First HUD Decluttering & Playwright CV Flywheel Convergence (`URBS-DIEGETIC-01..04`)

**Goal:** Uncover 85%+ of the 3D architectural diorama by mounting district plaques as chiseled bronze/travertine inscriptions on the front cliff retaining wall apron, progressive-disclosing secondary file slabs on hover/selection, upgrading `sprites.ts` with photorealistic gradients/shadows, and verifying via the Playwright CV Flywheel.
**Requirements:** `URBS-DIEGETIC-01`, `URBS-DIEGETIC-02`, `URBS-DIEGETIC-03`, `URBS-DIEGETIC-04`
**Plans:**

- [ ] `282-01`: Implement diegetic cliff-apron architrave plaques, HUD progressive disclosure, photorealistic sprite upgrades, and Playwright CV Flywheel verification (`phases/282-diegetic-plaques-and-cv-convergence/282-01-PLAN.md`)

---

## Progress Table (`v5.0`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 277** | Procedural Natural Surface Material Synthesis (`pbrMaterials.ts`) | `1/1` | Complete (`2026-10-25`) |
| **Phase 278** | True Isometric Directional Cast Shadows, Contact Ambient Occlusion & Specular Chamfer Bevels | `1/1` | Complete (`2026-10-25`) |
| **Phase 279** | Sculpted Coastal Bathymetry, Surf Foam Ribbons, Craggy Cliff Geology & Cobblestone Terraces | `1/1` | Complete (`2026-10-25`) |
| **Phase 280** | Golden-Hour & Twilight Dual-Key Atmospheric Lighting, Volumetric God-Rays, Elevation Mist & Additive GI Pools | `1/1` | Complete (`2026-10-25`) |
| **Phase 281** | Live Particle FX Engine (Forge Embers, Volumetric Smoke Plumes, Waterfall Spray & Mason Chisel Sparks) | `1/1` | Complete (`2026-10-25`) |
| **Phase 282** | Diegetic Cliff-Apron Architrave Plaques, Architecture-First HUD Decluttering & Playwright CV Flywheel Convergence | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v5.1` — T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure)

### Phase 283: System Prompt `$variable` Target Fix, Sanitizer Tag Completeness, `ToolRefusalDetector` & Lazy MCP Meta-Tools (`T3-PROMPT-LOOP-01..04`)

**Goal:** Apply `interpolate_system_prompt_variables` to `system_prompt` with full ISO-8601 UTC `$current_datetime` and worktree `.git` file `gitdir:` branch resolution, expand `escape_user_prompt_special_tokens` (`<tool_call>`, `<tool_response>`, `<think>`, active-turn user prompt) + `compact_historical_web_search` (`<WebFetch>`) + `thought_signature` extraction, wire `pi-t3chat` `ToolRefusalDetector` (`[SYSTEM TOOL CORRECTION]`) and `NotT3Chat` search circuit breaker (`total_searches >= 3`) in `agent_loop.rs`, and wire `LazyMcpGroupRegistry` (`vox_list_mcp_servers`, `vox_list_mcp_tools`, `vox_call_mcp_lazy`) + `sanitize_mcp_tool_search_env`.
**Requirements:** `T3-PROMPT-LOOP-01`, `T3-PROMPT-LOOP-02`, `T3-PROMPT-LOOP-03`, `T3-PROMPT-LOOP-04`
**Plans:**

- [ ] `283-01`: Fix `system_prompt` `$variable` interpolation & worktree branch detection, expand `context_sanitizer` & `thought_signature` persistence, wire `ToolRefusalDetector` & search circuit breaker in `agent_loop.rs`, and wire `LazyMcpGroupRegistry` (`phases/283-prompt-vars-sanitizer-and-loop-guards/283-01-PLAN.md`)

### Phase 284: Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` Lifecycle, Real `ModelSpec` Cost Ledger & `SessionPrivacyPosture` Gate (`T3-STREAM-QUEUE-01..04`)

**Goal:** Replace `ServerState::resumable_streams` tuple map with `ResumableTurnStreamRegistry` (`SequencedTurnChunk`, multi-iteration accumulation, `vox_chat_stream_resume` + `vox_chat_stream_cancel`), wire `ThreadRunQueue` (`pop_next_follow_up`, background work settlement, 429 `LimitRecoveryState`), wire post-resolution preflight check + `ModelRegistry::compute_turn_cost_delta` + 4-hour/monthly quota windows + `vox_battery_status`, and enforce `SessionPrivacyPosture::Ephemeral` disk-persistence gating.
**Requirements:** `T3-STREAM-QUEUE-01`, `T3-STREAM-QUEUE-02`, `T3-STREAM-QUEUE-03`, `T3-STREAM-QUEUE-04`
**Plans:**

- [ ] `284-01`: Wire `ResumableTurnStreamRegistry`, full `ThreadRunQueue` follow-up/settlement/recovery, `ModelRegistry::compute_turn_cost_delta` + `vox_battery_status`, and `SessionPrivacyPosture` persistence gate (`phases/284-resumable-stream-queue-and-cost-ledger/284-01-PLAN.md`)

### Phase 285: Worktree Async Lease Mutex Fix, Persistent Webhook Gate, Monotonic Sparse Git Checkpoints, Real `MultiVariantWorktreePool` & Compact Snapshot Fix (`T3-WORKTREE-CKPT-01..04`)

**Goal:** Hold a `tokio::sync::MutexGuard` across `.await` in `DelegationWorktree::create` and persist `WebhookDedupeAndRateGate` on `ServerState`, fix monotonic per-session sparse checkpoint turn counters + Windows NTFS `temp_index` sanitization + `-p HEAD` parent commits + clean untracked restore + `diff_checkpoints`, materialize real linked worktrees in `MultiVariantWorktreePool` (`vox_muse_pool_spawn` / `vox_muse_pool_adopt`), and fix `orchestrator_status_compact` double-envelope + `parent_agent_id` subagent filtering + `LiveBodyDeduper`.
**Requirements:** `T3-WORKTREE-CKPT-01`, `T3-WORKTREE-CKPT-02`, `T3-WORKTREE-CKPT-03`, `T3-WORKTREE-CKPT-04`
**Plans:**

- [ ] `285-01`: Fix async worktree lease lock & persistent webhook gate, monotonic sparse Git checkpoints & clean restore, real `MultiVariantWorktreePool` worktrees, and unwrapped `orchestrator_status_compact` (`phases/285-worktree-lease-checkpoints-and-muse-pool/285-01-PLAN.md`)

### Phase 286: UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Recursive Session Discovery & Real Guest Share / Diagnostics (`T3-TOOLS-HARDENING-01..04`)

**Goal:** Fix UTF-8 char-boundary truncation panics in `context_handoff.rs` & `mcp_tool_access.rs` + real multi-turn digest in `HandoffStrategy::Summarized`, check all configured LLM providers + run `synthesize_consensus_response` in `handle_consensus_run`, recursively scan depth-4 external sessions (`~/.claude/projects`, `~/.codex/sessions`, `~/.cursor/chats` + `vox_session_list_external`) + expose `vox_guest_share_redeem` / `vox_guest_share_submit`, and enforce server-side access caps in `authorize_wave3_call` + persistent `PullRequestReviewGuideService` + real diagnostics in `handle_file_diff_and_diagnostics`.
**Requirements:** `T3-TOOLS-HARDENING-01`, `T3-TOOLS-HARDENING-02`, `T3-TOOLS-HARDENING-03`, `T3-TOOLS-HARDENING-04`
**Plans:**

- [ ] `286-01`: Fix UTF-8 boundary panics & summarized handoff digest, multi-provider consensus synthesizer, depth-4 external session discovery & guest share submit/redeem, and server-side access / real diagnostics (`phases/286-utf8-safety-consensus-and-session-discovery/286-01-PLAN.md`)

### Phase 287: Unified `message_id` & Telemetry Across Tauri/MCP, Live Transcript Actions, Mid-Turn Steer Composer & `streamWatchdog.ts` (`T3-GUI-PIPELINE-01..04`)

**Goal:** Preserve `backend_message_id` (`usr:...`/`ast:...`) + battery/quality/posture telemetry across `vox-gui` (`chat.rs`/`chat_turn.rs`) and `history.rs` (with turn-index fallback & SQLite sync), wire live user+assistant `✎ Edit`, executing `↻ Regenerate` (`execute: true`), `✂ Compact Here` (`compactChatUpTo`), `/compact` slash command, and real branch comparison in `ChatTranscript.tsx`, fix the `Enter`-key task-abort bug and wire `[⚡ Steer Turn]` / `[+ Queue Next]` (`steerChatQueue`) in `Loquela.tsx`, and implement `streamWatchdog.ts` + `resumeChatStream` + full `ChatExecutionRail` / `SessionInteropModal` / `ConsensusGridView` / `GuestShare` wiring.
**Requirements:** `T3-GUI-PIPELINE-01`, `T3-GUI-PIPELINE-02`, `T3-GUI-PIPELINE-03`, `T3-GUI-PIPELINE-04`
**Plans:**

- [ ] `287-01`: Unify `message_id` & telemetry across Tauri/MCP, wire live edit/regenerate/compact & mid-turn steer composer, and implement `streamWatchdog.ts` + full modal/rail RPC wiring (`phases/287-unified-message-id-and-live-gui-actions/287-01-PLAN.md`)

### Phase 288: `vox-code-audit` T3 Detector Catalog Registration, `oxlint-plugin-t3code` Rule Parity & 43-Violation Workspace Remediation (`T3-AUDIT-PARITY-01..04`)

**Goal:** Register all T3 detector rule IDs in `diagnostics/catalog.rs` (`ALL_KNOWN_IDS`) with `diagnostic_id: Some(...)`, upgrade `unreasoned_suppression.rs`, `test_clock_and_skip.rs`, `mcp_access_and_error_safety.rs`, and `ui_perf_hazards.rs` to full `oxlint-plugin-t3code` parity, refactor raw `Command::new` calls in `t3_wave3_tools.rs`, and remediate all 43 live workspace violations across `vox-gui`, `vox-cli`, and `vox-orchestrator-mcp`.
**Requirements:** `T3-AUDIT-PARITY-01`, `T3-AUDIT-PARITY-02`, `T3-AUDIT-PARITY-03`, `T3-AUDIT-PARITY-04`
**Plans:**

- [ ] `288-01`: Register T3 detectors in `diagnostics/catalog.rs`, upgrade all 4 detectors to `oxlint-plugin-t3code` parity, and remediate all 43 workspace violations (`phases/288-code-audit-parity-and-workspace-remediation/288-01-PLAN.md`)

---

## Progress Table (`v5.1`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 283** | System Prompt `$variable` Target Fix, Sanitizer Tag Completeness, `ToolRefusalDetector` & Lazy MCP Meta-Tools | `1/1` | Complete (`2026-10-25`) |
| **Phase 284** | Sequenced `ResumableTurnStreamRegistry`, Full `ThreadRunQueue` Lifecycle, Real `ModelSpec` Cost Ledger & `SessionPrivacyPosture` Gate | `1/1` | Complete (`2026-10-25`) |
| **Phase 285** | Worktree Async Lease Mutex Fix, Persistent Webhook Gate, Monotonic Sparse Git Checkpoints, Real `MultiVariantWorktreePool` & Compact Snapshot Fix | `1/1` | Complete (`2026-10-25`) |
| **Phase 286** | UTF-8 Boundary Safety, Multi-Provider Consensus Synthesizer, Recursive Session Discovery & Real Guest Share / Diagnostics | `1/1` | Complete (`2026-10-25`) |
| **Phase 287** | Unified `message_id` & Telemetry Across Tauri/MCP, Live Transcript Actions, Mid-Turn Steer Composer & `streamWatchdog.ts` | `1/1` | Complete (`2026-10-25`) |
| **Phase 288** | `vox-code-audit` T3 Detector Catalog Registration, `oxlint-plugin-t3code` Rule Parity & 43-Violation Workspace Remediation | `1/1` | Complete (`2026-10-25`) |

---

## Phases (`v5.2` — Smooth Physically-Grounded Lighting, High-Detail Vitruvian Architecture & Grounds, Integrated Roman Hydraulic/Sewer Grid & Animated Shape+Color Activity Conduits)

### Phase 289: Smooth Ground-Plane-Recessed Lighting, 2D-Feathered Elliptical Sun-Shafts & Periodic Seamless Sea Synthesis (`URBS-SMOOTH-LIGHT-01..04`)

**Goal:** Eliminate alpha-overlay lighting artifacts and repeating water tile seams by moving warm brazier/forge/worksite bounce illumination onto the terrace ground plane before 3D buildings are painted, replacing hard-edged polygon god-rays with 2D-feathered elliptical radial gradients, replacing the hard `ctx.arc` district pulse with soft torch/window rim shimmer, and synthesizing `2π`-periodic seamless water waves in `pbrMaterials.ts`.
**Requirements:** `URBS-SMOOTH-LIGHT-01`, `URBS-SMOOTH-LIGHT-02`, `URBS-SMOOTH-LIGHT-03`, `URBS-SMOOTH-LIGHT-04`
**Plans:**

- [x] `289-01`: Implement pre-building ground-plane illumination, 2D-feathered elliptical sun-shafts, soft torch/window rim shimmer, and periodic seamless sea water synthesis (`phases/289-smooth-ground-lighting-and-seamless-sea/289-01-PLAN.md`)

### Phase 290: High-Detail Terrace Grounds: Orthogonal `Cardo`/`Decumanus` Streets, `Opus Sectile` Marble Plazas, Vaulted `Substructio` Cliff Arcades & Balustrades (`URBS-GROUNDS-01..04`)

**Goal:** Transform flat district terraces and plain cliff retaining walls into richly articulated Roman urban grounds featuring recessed basalt `Cardo`/`Decumanus` streets with raised travertine curbstones and stepping-stone crosswalks, central `Opus Sectile` geometric marble forum plazas around District Wonders, vaulted `Substructio` blind arcades and buttress pilasters on cliff walls, and perimeter stone balustrades with boundary obelisks (`Termini`).
**Requirements:** `URBS-GROUNDS-01`, `URBS-GROUNDS-02`, `URBS-GROUNDS-03`, `URBS-GROUNDS-04`
**Plans:**

- [x] `290-01`: Upgrade terrace grounds and cliff walls in `worldRenderer.ts` with `Cardo`/`Decumanus` streets, `Opus Sectile` plazas, `Substructio` cliff arcades, and stone balustrades (`phases/290-high-detail-terrace-grounds-and-substructio/290-01-PLAN.md`)

### Phase 291: High-Detail Vitruvian Building Architecture (`T0`–`T3`): Rusticated Ground Arcades (`Fornices`), Belt Cornices, Atrium `Impluvia` & `Acroteria` Roofs (`URBS-ARCH-DETAIL-01..04`)

**Goal:** Enrich all `T0`–`T3` file buildings in `worldRenderer.ts` with ground-floor rusticated stone arcades (`Fornices`) and warm shopfront lamplight, projecting inter-story `Marcapiano` belt cornices and corner quoins, recessed atrium `Impluvium` rainwater basins in `T1` Domus courtyards, and sculpted rooftop `Acroteria` finials and brick hypocaust chimneys.
**Requirements:** `URBS-ARCH-DETAIL-01`, `URBS-ARCH-DETAIL-02`, `URBS-ARCH-DETAIL-03`, `URBS-ARCH-DETAIL-04`
**Plans:**

- [x] `291-01`: Upgrade `drawIsoBox`, `drawIsoPitchedRoof`, and `drawIsoFileBuilding` with rusticated ground arcades, belt cornices, atrium impluvia, and acroteria roofs (`phases/291-high-detail-vitruvian-building-architecture/291-01-PLAN.md`)

### Phase 292: Integrated Roman Hydraulic Plumbing & Sewer Network (`hydraulicNetwork.ts`): `Aquaeductus` Trunk, `Castellum Divisorium`, `Fistulae` Supply Pipes & `Cloaca Maxima` Sewer Trenches/Outfalls (`URBS-HYDRAULIC-01..04`)

**Goal:** Create `hydraulicNetwork.ts` integrating the `AQVAE MCP` aqueduct directly into an upper-terrace `Castellum Divisorium` header tank that feeds a SimCity-style Roman hydraulic plumbing (`Fistulae` twin-rail bronze pipes & sluice valves) and `Cloaca Maxima` arched stone sewer trench network across all district terraces, terminating in cliff-base `Cloaca` sea outfalls and stepped cascades (`Cataracta`).
**Requirements:** `URBS-HYDRAULIC-01`, `URBS-HYDRAULIC-02`, `URBS-HYDRAULIC-03`, `URBS-HYDRAULIC-04`
**Plans:**

- [x] `292-01`: Create `hydraulicNetwork.ts` implementing integrated `Aquaeductus` trunk, `Castellum Divisorium`, `Fistulae` supply pipes, `Cloaca Maxima` sewer trenches, and sea outfalls (`phases/292-integrated-roman-hydraulic-and-sewer-network/292-01-PLAN.md`)

### Phase 293: Multi-Channel Color + Geometric Shape Animated Activity Conduits (`AQUA PURA` Chevrons, `IGNIS OPERIS` Diamonds, `CLOACA FAULT` Triangles & `VIA VICTORIAE` Seals) (`URBS-CONDUIT-ANIM-01..04`)

**Goal:** Animate live activities in progress along the Roman hydraulic and sewer conduits using 4 double-coded Color + Geometric Shape channels: Crystalline Cyan Chevron Wave-Crests (`❯❯`) for `AQUA PURA` MCP/provider supply, Molten Gold Diamond Ingots (`◆◆`) for `IGNIS OPERIS` active agent work-in-progress, Crimson Warning Triangles (`▲`) & eddies (`◎`) for `CLOACA FAULT` error backpressure, and Emerald Circular Laurel Medallions (`◉`) for `VIA VICTORIAE` verified CI/Git flows.
**Requirements:** `URBS-CONDUIT-ANIM-01`, `URBS-CONDUIT-ANIM-02`, `URBS-CONDUIT-ANIM-03`, `URBS-CONDUIT-ANIM-04`
**Plans:**

- [x] `293-01`: Implement `drawAnimatedHydraulicActivityFlows` in `hydraulicNetwork.ts` and wire all 4 shape+color activity conduits into `WorldRenderer` (`phases/293-animated-shape-color-activity-conduits/293-01-PLAN.md`)

### Phase 294: Diegetic Hydraulic Sluice-Gate Callouts, Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence (`URBS-SEMIOTICS-CV-01..04`)

**Goal:** Replace ungrounded screen-space straight dependency lines and the floating yellow arrow with diegetic Roman Hydraulic Sluice-Gate (`Cataracta`) callouts and isometric conduit routing, add a shape+color hydraulic flow legend, write comprehensive unit tests (`hydraulicNetwork.test.ts`, `pbrMaterials.test.ts`, `lightingAndFx.test.ts`), and verify convergence via the Playwright `@ngi/platform-ssot` CV Flywheel.
**Requirements:** `URBS-SEMIOTICS-CV-01`, `URBS-SEMIOTICS-CV-02`, `URBS-SEMIOTICS-CV-03`, `URBS-SEMIOTICS-CV-04`
**Plans:**

- [x] `294-01`: Implement diegetic hydraulic sluice-gate callouts, shape+color conduit legend, `hydraulicNetwork.test.ts` unit suite, and Playwright CV Flywheel verification (`phases/294-semiotic-consistency-and-cv-convergence/294-01-PLAN.md`)

---

## Progress Table (`v5.2`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 289** | Smooth Ground-Plane-Recessed Lighting, 2D-Feathered Elliptical Sun-Shafts & Periodic Seamless Sea Synthesis | `1/1` | Complete |
| **Phase 290** | High-Detail Terrace Grounds: Orthogonal `Cardo`/`Decumanus` Streets, `Opus Sectile` Marble Plazas, Vaulted `Substructio` Cliff Arcades & Balustrades | `1/1` | Complete |
| **Phase 291** | High-Detail Vitruvian Building Architecture (`T0`–`T3`): Rusticated Ground Arcades (`Fornices`), Belt Cornices, Atrium `Impluvia` & `Acroteria` Roofs | `1/1` | Complete |
| **Phase 292** | Integrated Roman Hydraulic Plumbing & Sewer Network (`hydraulicNetwork.ts`): `Aquaeductus` Trunk, `Castellum Divisorium`, `Fistulae` Supply Pipes & `Cloaca Maxima` Sewer Trenches/Outfalls | `1/1` | Complete |
| **Phase 293** | Multi-Channel Color + Geometric Shape Animated Activity Conduits (`AQUA PURA` Chevrons, `IGNIS OPERIS` Diamonds, `CLOACA FAULT` Triangles & `VIA VICTORIAE` Seals) | `1/1` | Complete |
| **Phase 294** | Diegetic Hydraulic Sluice-Gate Callouts, Semiotic Consistency Audit, Unit Test Suite & Playwright CV Flywheel Convergence | `1/1` | Complete |

---

## Phases (`v5.4` — T3 CI/TOESTUB Enforcement Deep-Extraction & Harness Policy Hardening)

### Phase 301: Change-Gated CI Path Map & `vox ci check-changed` (`T3-CI-CHANGED-01..04`)

**Goal:** Port t3code `check:changed` + INERT/EXTRA_OWNERS ownership mapping into a Vox contract and CLI so local/CI runs only the gates owned by the diff.
**Requirements:** `T3-CI-CHANGED-01`, `T3-CI-CHANGED-02`, `T3-CI-CHANGED-03`, `T3-CI-CHANGED-04`
**Success criteria:**

1. Owner map contract validates under schema and lists crates/UI/workflows/contracts.
2. `vox ci check-changed` (or `vox run scripts/check-changed.vox`) exits 0 on inert planning-only diffs without invoking cargo.
3. PR jobs skip heavy work for inert path sets while required contexts remain honest.
4. Docs in `runner-contract.md` show the local command and ownership rules.

**Plans:**

- [x] `301-01`: Define owner map + implement `check-changed` + CI wiring + docs/tests

### Phase 302: Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding (`T3-CI-BUDGET-01..04`)

**Goal:** Promote transfer-budget coverage into CI artifacts + trusted PR comments, and balance shards with duration weights.
**Requirements:** `T3-CI-BUDGET-01`, `T3-CI-BUDGET-02`, `T3-CI-BUDGET-03`, `T3-CI-BUDGET-04`
**Success criteria:**

1. CI uploads a schema-valid budget artifact on PRs that touch orchestrator-mcp/budget code.
2. Comment publisher runs from default-branch code only and posts delta vs main.
3. Weighted shard file measurably reduces max-shard wall time vs hash-only partitions.
4. Ratchet report lands under `contracts/reports/`.

**Plans:**

- [x] `302-01`: Budget artifact + trusted comment workflow + weighted sharding + ratchet

### Phase 303: Knip Dual Gate, PR Size Labels & Evidence-Asset Ban (`T3-CI-HYGIENE-01..04`)

**Goal:** Catch unused TS exports, label PR blast radius by non-test LOC, and ban committed evidence spam.
**Requirements:** `T3-CI-HYGIENE-01`, `T3-CI-HYGIENE-02`, `T3-CI-HYGIENE-03`, `T3-CI-HYGIENE-04`
**Success criteria:**

1. Knip (or equivalent) fails CI on new unused exports in `vox-gui/ui` outside allowlist.
2. PRs receive exactly one size:* label from the XS–XXL set based on non-test LOC.
3. Committing a disallowed evidence binary fails the hygiene gate.
4. Mutation tests prove each gate fails closed; AGENTS/CONTRIBUTING updated.

**Plans:**

- [x] `303-01`: Knip config + PR size workflow + evidence ban + docs/mutation tests

### Phase 304: Agent-Ops Kill-Safety, Hit-Every-Surface Checklist & Worktree Isolation (`T3-AGENT-OPS-01..04`)

**Goal:** Import t3code agent-ops culture: kill-safety, multi-surface completion checklist, isolated worktree homes, suppression human review.
**Requirements:** `T3-AGENT-OPS-01`, `T3-AGENT-OPS-02`, `T3-AGENT-OPS-03`, `T3-AGENT-OPS-04`
**Success criteria:**

1. AGENTS.md / governance names the kill-safety triad with Vox paths.
2. Checklist is referenced from PR template or `vox doctor` advisory for multi-surface diffs.
3. Worktree setup marker + isolated home documented and scripted via `.vox`.
4. Suppression-only PRs cannot merge without human-review label/CODEOWNERS.

**Plans:**

- [x] `304-01`: Policy docs + checklist advisory + worktree isolation + suppression review gate

### Phase 305: TOESTUB Thin-Transport, Suppression Review Gate & Snapshot Interning (`T3-TOESTUB-ENF-01..04`)

**Goal:** Close the oxlint→TOESTUB gap for transport bypasses, suppression reasons, snapshot interning, and remaining high-value rules.
**Requirements:** `T3-TOESTUB-ENF-01`, `T3-TOESTUB-ENF-02`, `T3-TOESTUB-ENF-03`, `T3-TOESTUB-ENF-04`
**Success criteria:**

1. New detector flags raw RPC/client bypass outside allowlisted files.
2. Unreasoned suppressions in a PR diff fail CI.
3. Snapshot/message-ID interning is live-path covered by tests.
4. Remaining oxlint rules are either detectors or explicit deferred ledger rows.

**Plans:**

- [x] `305-01`: Detectors + suppression ratchet + interning + oxlint mapping ledger

### Phase 306: Tauri Fingerprint Advisory, GUI Cache Layers, Surface Graph Refresh & Scorecard (`T3-CI-CLOSE-01..04`)

**Goal:** Close the milestone with fingerprint labeling, legal GUI caches, a fresh T3 surface graph, and a CI-enforcement scorecard.
**Requirements:** `T3-CI-CLOSE-01`, `T3-CI-CLOSE-02`, `T3-CI-CLOSE-03`, `T3-CI-CLOSE-04`
**Success criteria:**

1. Native/sidecar-affecting PRs get an advisory label; check stays green.
2. GUI jobs restore layered pnpm/vite caches without PR cache saves.
3. Surface graph regenerates from current clone SHAs; inventory script compiles.
4. Scorecard + architecture note list every gem as Shipped/Partial/Deferred with anchors.

**Plans:**

- [x] `306-01`: Fingerprint workflow + cache layers + graph regen + scorecard/docs

---

## Progress Table (`v5.4`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 301** | Change-Gated CI Path Map & `vox ci check-changed` | `1/1` | Complete |
| **Phase 302** | Transfer-Budget Artifacts, PR Comments & Weighted Test Sharding | `1/1` | Complete |
| **Phase 303** | Knip Dual Gate, PR Size Labels & Evidence-Asset Ban | `1/1` | Complete |
| **Phase 304** | Agent-Ops Kill-Safety, Hit-Every-Surface Checklist & Worktree Isolation | `1/1` | Complete |
| **Phase 305** | TOESTUB Thin-Transport, Suppression Review Gate & Snapshot Interning | `1/1` | Complete |
| **Phase 306** | Tauri Fingerprint Advisory, GUI Cache Layers, Surface Graph Refresh & Scorecard | `1/1` | Complete |

## Phases (`v5.3` — Photorealistic Edge-Softening, Continuous Monumental Aqueduct-to-Sewer Architecture, Sub-Surface Ground Integration & CV Flywheel Score Maximization)

### Phase 295: Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression (`URBS-EDGE-PBR-01..04`)

**Goal:** Eliminate hard diamond coastal cutouts, 1px Laplacian PBR pixel chatter (`noiseFloor: 10.7`), and `#1a1613` cartoon ink outlines by introducing feathered elliptical radial coastal shoals, smooth bilinear/radial macro-shaded PBR stone/terracotta/water tiles, full-canvas diagonal ocean swells, and soft tonal masonry/roof edge bevels.
**Requirements:** `URBS-EDGE-PBR-01`, `URBS-EDGE-PBR-02`, `URBS-EDGE-PBR-03`, `URBS-EDGE-PBR-04`
**Plans:**

- [x] `295-01`: Implement feathered coastal bathymetry, low-Laplacian macro-shaded PBR tiles, diagonal ocean swells, and soft tonal edge bevels (`phases/295-feathered-bathymetry-and-smooth-pbr/295-01-PLAN.md`)

### Phase 296: True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums (`URBS-TERRACE-CRAFT-01..04`)

**Goal:** Replace jagged axis-aligned 2D `fillRect` wall merlons and overlapping street alpha rectangles with true `2:1` isometric molded travertine parapet capstones (`Corona`), 3D pyramidal-roofed corner bastions, a unified intersection-free `Cardo`/`Decumanus` street cross-plaza (`Via Silicea`), a recessed multi-ring `Opus Sectile` Forum with 4 corner Corinthian columns, and an overhanging cliff-edge travertine cornice (`Geison`).
**Requirements:** `URBS-TERRACE-CRAFT-01`, `URBS-TERRACE-CRAFT-02`, `URBS-TERRACE-CRAFT-03`, `URBS-TERRACE-CRAFT-04`
**Plans:**

- [x] `296-01`: Upgrade curtain walls, `Cardo`/`Decumanus` street intersections, `Opus Sectile` forum peristyles, and cliff cornices (`phases/296-isometric-walls-and-seamless-streets/296-01-PLAN.md`)

### Phase 297: Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae (`URBS-ARCH-REALISM-01..04`)

**Goal:** Upgrade `Tabularium` and `Pantheon` rotundas with multi-stop cylindrical normal shading and coffered half-dome ribs (`Lacunaria`), replace flat 2D window squares with isometric-skewed recessed stone window reveals and warm interior lamplight, add 2-stepped stylobates and sculpted pediment tympanums to `T2`/`T3` buildings, and upgrade peripheral landmarks (`AQVAE`, `CASTRVM`, `PORTVS`, `PORTA`) to 3D isometric masonry.
**Requirements:** `URBS-ARCH-REALISM-01`, `URBS-ARCH-REALISM-02`, `URBS-ARCH-REALISM-03`, `URBS-ARCH-REALISM-04`
**Plans:**

- [x] `297-01`: Upgrade cylindrical rotunda wonders, isometric window reveals, pediment tympanums, and 3D peripheral landmarks (`phases/297-cylindrical-wonders-and-recessed-windows/297-01-PLAN.md`)

### Phase 298: Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid (`URBS-PLUMBING-3D-01..04`)

**Goal:** Connect `AQVAE MCP` via a continuous 3D multi-arch stone aqueduct viaduct (`Aqua Claudia`) directly into the Acropolis `Castellum Divisorium`, upgrade `drawConduitTrack` with rounded isometric elbow fillets (`quadraticCurveTo`) and 3D cylindrical pipe shading, route all dependency and blast-radius conduits on the **terrace ground / viaduct layer beneath 3D buildings** (`Layer 0.85`), and render stepped stone `Cataracta` cascades between terrace elevations.
**Requirements:** `URBS-PLUMBING-3D-01`, `URBS-PLUMBING-3D-02`, `URBS-PLUMBING-3D-03`, `URBS-PLUMBING-3D-04`
**Plans:**

- [x] `298-01`: Build continuous 3D `Aqua Claudia` aqueduct bridge, rounded pipe elbow fillets, sub-building ground conduit routing, and stepped `Cataracta` cascades (`phases/298-continuous-aqueduct-and-beveled-plumbing/298-01-PLAN.md`)

### Phase 299: Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics (`URBS-FLOW-SEMIOTICS-01..04`)

**Goal:** Add soft radial glow halos and tapered comet trails to all 4 geometric particle channels (`❯` Chevron, `◆` Diamond, `▲` Triangle, `◉` Laurel Seal), scale conduit gauge and particle cadence by live activity load, render glowing bronze manifold rings (`Epistomia`) at district street ports, and upgrade the top-left Semiotic Legend Pill with vector geometric icons and live channel counts.
**Requirements:** `URBS-FLOW-SEMIOTICS-01`, `URBS-FLOW-SEMIOTICS-02`, `URBS-FLOW-SEMIOTICS-03`, `URBS-FLOW-SEMIOTICS-04`
**Plans:**

- [x] `299-01`: Add radial halos and comet tails to activity particles, dynamic load scaling, perimeter manifold rings, and vector-icon legend (`phases/299-halo-particles-and-conduit-semiotics/299-01-PLAN.md`)

### Phase 300: Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization (`URBS-CV-MAX-01..04`)

**Goal:** Expand unit test suites (`pbrMaterials.test.ts`, `lightingAndFx.test.ts`, `hydraulicNetwork.test.ts`, `worldRenderer.test.ts`), achieve positive CV noise reduction (`noiseFloor < 5.1`) and higher `visualAfterScore` (`>= 0.942`) on `urbs-civilization` in `e2e/review/cv-flywheel.spec.ts`, verify 0 regressions across all 32 GUI surfaces, and update the visual blueprint carousel.
**Requirements:** `URBS-CV-MAX-01`, `URBS-CV-MAX-02`, `URBS-CV-MAX-03`, `URBS-CV-MAX-04`
**Plans:**

- [x] `300-01`: Expand unit tests, run Playwright CV Flywheel score maximization, and update visual blueprint (`phases/300-cv-flywheel-score-maximization/300-01-PLAN.md`)

---

## Progress Table (`v5.3`)

| Phase | Title | Plans | Status |
|---|---|---|---|
| **Phase 295** | Feathered Coastal Bathymetry, Smooth Macro-Shaded PBR Materials & Laplacian Noise Suppression | `1/1` | Complete |
| **Phase 296** | True Isometric Molded Curtain Walls, Seamless `Via Silicea` Street Grid & Coffered `Opus Sectile` Forums | `1/1` | Complete |
| **Phase 297** | Cylindrical & Coffered 3D Architectural Realism for Wonders & `T0`–`T3` Insulae | `1/1` | Complete |
| **Phase 298** | Continuous 3D Monumental `Aqua Claudia` Viaduct & Ground-Integrated Beveled Hydraulic/Sewer Grid | `1/1` | Complete |
| **Phase 299** | Illuminated Halo-Backed Shape+Color Activity Particles & Interactive Conduit Semiotics | `1/1` | Complete |
| **Phase 300** | Comprehensive Unit Tests, Semiotic Critique Audit & Playwright CV Flywheel Score Maximization | `1/1` | Complete |
