# Phase 470 Context: End-to-End Polyglot Repo Onboarding Proof, Concept-Art Plates & Multi-Pass Before/After CV Flywheel Convergence

**Workstream:** `universal-kb-openwiki-v79`
**Phase:** `470`
**Requirements:** `CV-01`, `CV-02`, `CV-03`, `CV-04`
**Depends on:** Phases 465 (`REPO-01..04`, `KB-01..02`), 466 (`KB-03..04`, `GRAPH-01..04`), 467 (`WIKI-01..02`), 468 (`WIKI-03..04`), and 469 (`GUI-01..04`)

## Architectural Decisions (Locked)
1. **End-to-End Polyglot Integration & Self-Heal Proof (`CV-01`):**
   - `crates/vox-cli/tests/v79_polyglot_flywheel_proof.rs` executes a complete cold-start onboarding of an external polyglot repository (`Rust + TypeScript + Python + Go + SQL + Vox`) plus sibling + `--read-only` repositories, builds dynamic KB + Graphify + 5-layer OpenWiki, injects human narrative outside `<!-- openwiki:auto:start -->` fences, mutates AST symbols and source line offsets, executes Tier-1 zero-LLM auto-heal + Tier-2 4-gate verified prose healing (`kb_heal_proposals`), asserts `cross_repo_leakage_count == 0` and PPR multi-hop recall uplift, and emits `contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json`.
2. **Dual-Theme Concept-Art Plate Registration (`CV-02`):**
   - Four calibrated Romanesque Carrara/Slate plates (`v79-concept-openwiki-studio-{carrara,slate}.jpg`, `v79-concept-multirepo-atlas-{carrara,slate}.jpg`) live in `crates/vox-gui/ui/public/assets/romanesque/` and route via `conceptArtPlateForSurface` in `vlmVisionFlywheel.ts`.
3. **5-Viewport × 4-Theme Before/After CV Flywheel Convergence (`CV-03`):**
   - `crates/vox-gui/ui/e2e/review/v79CvFlywheelConvergence.test.ts` benchmarks `CV_STAGE=before` (historical cream-on-marble / dark-only leaks) against `CV_STAGE=after` (remediated Phase 469 `MemoryView`, `DocReader`, `OpenWikiHealStudio`, `CodeAtlasCanvas`, `RepositoryView`, and `BottomStatusBar` + `RepoSwitcherPill`) via `auditDomQualityInPage`, `verifyThemeProfileContrast`, and `evaluateBeforeAfterCvFlywheel` (`@ngi/platform-ssot`) across `900×600`, `1100×720`, `1280×800`, `1440×900`, `1920×1080` and `arcane`, `glacier`, `slate`, `high-contrast`.
4. **Documentation Promotion & Workspace Gates (`CV-04`):**
   - Promote `universal-kb-graphify-openwiki-multirepo-research-2026.md` to `status: "current"`, update `where-things-live.md` and `docs-system-ssot.md`, and verify `vox-arch-check`, `vox-doc-pipeline --lint-only`, and all Vitest + Rust proof gates green with zero home-path leaks.
