# Phase 401 Summary — Adversarial Audit of All READMEs Across All Repositories & Sub-Crates, `v6.6` Sunlit Imperial Roman Carrara White & Gold + `VRBS ROMA` Simulator Visual Refresh, `README-SYNC-MAP` Maintainability & Footnoted Prior-Art Attribution

**Phase:** 401 (`README-AUDIT-01..04`)
**Date:** 2026-11-10
**Status:** Complete (`4/4` requirements verified)

## 1. Root Cause of Stale Graphics & Resolution (`README-AUDIT-01..02`)
- **Finding:** `crates/vox-gui/ui/review-bundle/latest/*.png` contained pre-Roman dark UI captures from **2026-09-04**, whereas `crates/vox-gui/ui/artifacts/cv-flywheel/surfaces/*-after.png` and `crates/vox-gui/ui/artifacts/cv-flywheel/{urbs-civilization-after.png,urbs-imperium-inspector-after.png,chat-multidock-1440x900-after.png,loquela-knobs-after.png}` held the **2026-11-09 (`v6.6`) Sunlit Imperial Roman Carrara White & Burnished Gold** captures.
- **Fix:**
  1. Copied all 32 `*-after.png` surface captures plus 18 simulator (`urbs-civilization.png`, `urbs-imperium-inspector.png`), multi-dock (`chat-multidock.png`), and 4-theme (`theme-arcane.png`, `theme-slate.png`, `theme-glacier.png`, `theme-high-contrast.png`) captures into `/Users/brbrainerd/dev/vox-axis/assets/screens/`, `/Users/brbrainerd/dev/vox/crates/vox-gui/assets/screens/`, and `/Users/brbrainerd/dev/vox/crates/vox-gui/ui/review-bundle/latest/`.
  2. Rebuilt all **7 looping animated `.webp` tours** (`axis_hero_tour.webp`, `axis_operate_lane_tour.webp`, `axis_develop_lane_tour.webp`, `axis_knowledge_hitl_tour.webp`, `axis_compute_system_tour.webp`, `voxlang_site_tour.webp`, `vox_ops_fleet_tour.webp`) from the `v6.6` Roman White/Gold and `VRBS ROMA` Simulator frames.
  3. Regenerated all **7 high-resolution architectural diagrams** in the `v6.6` Sunlit Imperial Roman Carrara White & Burnished Gold aesthetic (`axis_hero_banner.webp`, `axis_dual_mode_architecture.webp`, `axis_five_lanes_diagram.webp`, `axis_trust_tiers_diagram.webp`, `voxlang_pipeline_diagram.webp`, `vox_ops_sovereignty_diagram.webp`, and `docs/src/assets/schema-fragmentation.webp`), replacing the last `.svg` diagram (`schema-fragmentation.svg`) in both `vox/README.md` and `docs/src/index.mdx`.

## 2. Adversarial Audit of All 46 READMEs (`README-AUDIT-03`)
- Audited all 46 `README.md` files across `vox`, `vox-axis`, `voxlang.org`, and `vox-ops`.
- Remediated all 8 stale sub-crate/app READMEs inside `vox`:
  - `crates/vox-doc-pipeline/README.md` (retired `mdBook` / `SUMMARY.md` → Astro + Starlight `sidebar.mjs` + SKOS/Nanopublication lint)
  - `crates/vox-lsp/README.md` (retired `@table`, `@island`, `@server` → bare keywords `table`, `query`, `mutation`, `component`, `tool`)
  - `crates/vox-cli/README.md` (`v0.1.0` / `vox deploy` stub → `v0.6.0` subcommand architecture delegating to `vox-cli-{core,model,db,diagnostics,ci}`)
  - `crates/vox-orchestrator/README.md` (updated for `vox-orchestrator-models`, `vox-mcp-context`, `vox-mcp-dispatch`, and ADR-025/029)
  - `crates/vox-db/README.md` (purged retired `v0.rs` and `docs/src/archive/` links; updated to `VoxDb` baseline `97` and `vox-db-types`)
  - `crates/vox-gamify/README.md` (purged broken `docs/src/archive/ludus-magnus-architecture-2026.md` link)
  - `apps/editor/vox-vscode/README.md` (updated snippet examples from `@table`/`@endpoint` to bare keywords)
  - `crates/vox-integration-tests/README.md` (updated syntax examples and pipeline stages)

## 3. Maintainability (`README-SYNC-MAP`) & Footnoted Prior-Art Attribution (`README-AUDIT-04`)
- Added `README-SYNC-MAP` ownership rules to `vox/README.md`, `vox-axis/README.md`, `voxlang.org/README.md`, and `vox-ops/README.md` so each repo owns its domain without duplicating tables across repos.
- Added inline footnotes (`[^1]`..`[^13]`) and `## Intellectual Lineage, Prior Art & References` sections across all 4 root READMEs crediting Convex, Blitz/TanStack, Links (Cooper et al. 2006), Rust `Option`/`Result`, Koka effect rows (Leijen 2014), Dennis & Van Horn (1966) / Mark S. Miller / WASI capabilities, Temporal/Cadence & Durable Functions (Burckhardt et al. 2021), Hewitt Actor model & Erlang/OTP, Anthropic MCP, `agentskills.io`, Jesse Vincent's *Superpowers*, `pingdotgg/t3code` & `vibheksoni/t3router`, Cormack et al. RRF (2009) + Tantivy BM25, Procida's *Diátaxis*, W3C SKOS, Groth et al. Nanopublications (2010), Dettmers et al. QLoRA (2023) + HF Candle + Qwen3, Chen et al. `pass@k` (2021) + McNemar (1947) + Holm (1979), and Wettel & Lanza *CodeCity* (2007) + *Forma Urbis Romae* + Vitruvius + *Caesar III* (1998).
