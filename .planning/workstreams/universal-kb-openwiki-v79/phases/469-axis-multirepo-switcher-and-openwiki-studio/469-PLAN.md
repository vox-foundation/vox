# Phase 469 Plan: Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki/KB Heal Studio, Live Code Atlas & 4-Theme Remediation

**Workstream:** `universal-kb-openwiki-v79`
**Phase:** `469`
**Requirements:** `GUI-01`, `GUI-02`, `GUI-03`, `GUI-04`

## Target Files
- `crates/vox-gui/ui/src/components/surfaces/Repository/RepositoryCatalogPanel.tsx` (new, `<350` LoC)
- `crates/vox-gui/ui/src/components/surfaces/Repository/RepositoryView.tsx`
- `crates/vox-gui/ui/src/components/layout/BottomStatusBar.tsx`
- `crates/vox-gui/ui/src/components/layout/AppShell.tsx`
- `crates/vox-gui/ui/src/components/surfaces/DocReader/OpenWikiHealStudio.tsx` (new, `<350` LoC)
- `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx`
- `crates/vox-gui/ui/src/components/surfaces/VoxGraph/CodeAtlasCanvas.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx`
- Unit & guard tests:
  - `crates/vox-gui/ui/src/components/surfaces/Repository/RepositoryView.test.tsx`
  - `crates/vox-gui/ui/src/components/layout/BottomStatusBar.test.tsx`
  - `crates/vox-gui/ui/src/components/layout/AppShell.test.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.test.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/VoxGraph/VoxGraphStatusPanel.test.tsx`
  - `crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.test.tsx`

## Steps
1. Create `469-CONTEXT.md` and `469-PLAN.md`.
2. Build `RepositoryCatalogPanel.tsx`, integrate into `RepositoryView.tsx`, and add the Multi-Repo Switcher pill + popover in `BottomStatusBar.tsx` / `AppShell.tsx` (`GUI-01`).
3. Build `OpenWikiHealStudio.tsx` and refactor `DocReader.tsx` to remove `DEFAULT_INBOX_FINDINGS` / `DEFAULT_LIBRARY_RESULTS`, wire live `vox_kb_*` + `wiki_*` MCP tools, add promotion scope (`repo_local` vs `global_library`), and remediate dark-only CSS (`GUI-02`, `GUI-04`).
4. Refactor `CodeAtlasCanvas.tsx` to purge `DEFAULT_ATLAS_NODES` / `DEFAULT_ATLAS_EDGES` and silent catches, wire live `vox_search_atlas` across `L0_Crates..L3_Docs`, and remediate cream-on-marble hex colors + add ingest scope toggle in `MemoryView.tsx` (`GUI-03`, `GUI-04`).
5. Update Vitest unit tests, run all relevant UI + guard Vitest suites, verify semantic/contrast/line-budget discipline, write `469-VERIFICATION.md`, `469-REVIEW.md`, `469-SUMMARY.md`, and advance `v7.9` state to Phase `470`.
