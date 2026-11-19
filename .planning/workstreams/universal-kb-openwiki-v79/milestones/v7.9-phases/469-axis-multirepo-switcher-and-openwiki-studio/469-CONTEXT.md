# Phase 469 Context: Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki/KB Heal Studio, Live Code Atlas & 4-Theme Remediation

**Workstream:** `universal-kb-openwiki-v79`
**Phase:** `469`
**Requirements:** `GUI-01`, `GUI-02`, `GUI-03`, `GUI-04`
**Depends on:** Phases 465 (`REPO-01..04`, `KB-01..02`), 466 (`KB-03..04`, `GRAPH-01..04`), 467 (`WIKI-01..02`), and 468 (`WIKI-03..04`)

## Architectural Decisions (Locked)
1. **Multi-Repo Workspace Switcher & Onboarding Wizard (`GUI-01`):**
   - `BottomStatusBar.tsx` + `AppShell.tsx` host an always-accessible active repository switcher pill (`data-testid="bottom-status-bar-repo"`) and popover (`data-testid="bottom-status-bar-repo-popover"`) showing the active repository name and `hybrid` vs `read_only` mode badge, plus 1-click repository switching and quick navigation to `Repository`.
   - `RepositoryView.tsx` + `RepositoryCatalogPanel.tsx` host the full catalog panel and Init/Add Repository Onboarding Wizard (`data-testid="repo-onboarding-wizard"`) supporting Two-Tier Hybrid (`docs/wiki/` + `.vox/cache/`) vs Zero-Footprint Read-Only (`~/.vox/repos/<repository_id>/`), stack detection badges, OpenWiki freshness pills, and 1-click `vox_repo_catalog_refresh` mutations (`init`, `add`, `switch`, `remove`, `wiki_init`, `wiki_heal_tier1`).
2. **Zero-Mock OpenWiki & KB Self-Heal Studio (`GUI-02`):**
   - `DocReader.tsx` removes hardcoded `DEFAULT_INBOX_FINDINGS` and `DEFAULT_LIBRARY_RESULTS` and queries `vox_kb_search`, `vox_kb_status`, `vox_kb_verify`, `vox_kb_review_finding`, and `vox_repo_catalog_refresh` (`wiki_*` actions) via `voxTransport.invokeMcpTool`.
   - Candidate finding promotion exposes an explicit `target_scope` (`repo_local` vs `global_library` `~/.vox/library/`) selector in `DocReader.tsx` and ingest scope toggle in `MemoryView.tsx`.
   - `OpenWikiHealStudio.tsx` renders 5-layer OpenWiki freshness and the Side-by-Side Self-Heal Diff Inspector for `kb_heal_proposals` (`Heal All Tier-1`, `Apply Patch`, `Demote to Legacy`, `Reject`) with 4-gate verification status badges (`RewriteGuard`, `DocLint`, `VoxDiagram`, `Doctest`).
3. **100% Live Graphify `CodeAtlasCanvas` (`GUI-03`):**
   - `CodeAtlasCanvas.tsx` deletes `DEFAULT_ATLAS_NODES`, `DEFAULT_ATLAS_EDGES`, and silent `.catch(() => {})`, binds `vox_search_atlas` across `L0_Crates`, `L1_Files`, `L2_Symbols`, and `L3_Docs` (`DocumentedIn`, `CitesSymbol`, `GovernedByAdr` cross-edges + blast-radius badges), displays honest error alerts (`role="alert"`), and emits `@file:` / `@symbol:` / `@doc:` mention chips to `Loquela`.
4. **4-Theme Semantic Contrast & Guard Discipline (`GUI-04`):**
   - Purge cream-on-marble (`#F3E5AB`, `#D6CEBF`, `#D4AF37` low-contrast pairs, `#78350F`, `#FEF3C7`) in `MemoryView.tsx` and dark-only classes (`bg-black/20`, `bg-black/25`, `text-amber-300`, `text-sky-200`, `text-emerald-300`) in `DocReader.tsx`, replacing with Style Dictionary semantic tokens (`bg-bg-surface`, `bg-bg-elevated`, `text-text-primary`, `text-text-secondary`, `text-text-muted`, `border-border-subtle`, `border-brass`, `text-brass`, `--color-status-*`).
   - Comply strictly with `noRawNativeControlsGuard.test.ts` (use `<VoxSelect>` instead of `<select>`), `zeroEmojiAndDingbat.test.ts`, `ipcBoundaries.test.ts`, `themeContract.test.ts`, and `<400` LoC child component modules.
