# Requirements: Milestone v8.8 (`voxlang-tutorials-kb-openwiki-docs-v88`)

| ID | Category | Requirement | Phase | Status |
|---|---|---|---|---|
| `INIT-01` | Governance | Register Milestone `v8.8` in `.planning/PROJECT.md` and workstream planning files (`Phases 520–525`) | 520 | `[x]` |
| `INIT-02` | Taxonomy | Verify every authored/upgraded doc uses canonical SKOS `topic` & `domain` `prefLabel`s from `contracts/documentation/kb-taxonomy.v1.yaml` | 520 | `[x]` |
| `MEDIA-01` | Assets | Convert verified `v8.7` `research-arcane/void`, `memory-arcane/void`, `doc-reader-arcane/void` captures to `.webp` (`quality: 88`) | 521 | `[x]` |
| `MEDIA-02` | Assets | Convert verified `v8.7` `publications-arcane`, `scientia-arcane`, `vox-search-arcane`, and `activity-arcane` captures to `.webp` | 521 | `[x]` |
| `MEDIA-03` | Assets | Stage all 10 `.webp` visuals identically in `docs/src/assets/axis/` and `docs-astro/public/assets/axis/` | 521 | `[x]` |
| `MEDIA-04` | Assets | Verify all 20 `.webp` asset paths exist and render non-empty in static asset directories | 521 | `[x]` |
| `TUT-RES-01` | Research Tut | Author `docs/src/tutorials/tut-deep-research-and-scientia.mdx` with valid frontmatter, JSX `{/* kb:summary */}`, dual-theme visuals, and `vox-diagram` | 522 | `[x]` |
| `TUT-RES-02` | Research Tut | Teach statistical archetypes, Live KB Prior-Art Preflight, `Off \| Auto \| Fast \| Deep` continuum, and `Domain Mode` selection | 522 | `[x]` |
| `TUT-RES-03` | Research Tut | Teach subquery telemetry inspection, `vox research flag`, `+1 Wave Refinement`, and `Index Now` vs. `Save to KB` quarantine | 522 | `[x]` |
| `TUT-RES-04` | Research Tut | Teach `Edit in Doc Dock`, 7-stage `arXiv.org` staging (`vox scientia`), and include a compiling `.vox` `<VoxPlayground />` module | 522 | `[x]` |
| `TUT-KB-01` | KB/Wiki Tut | Author `docs/src/tutorials/tut-knowledgebase-and-openwiki.mdx` with valid frontmatter, JSX `{/* kb:summary */}`, dual-theme visuals, and `vox-diagram` | 522 | `[x]` |
| `TUT-KB-02` | KB/Wiki Tut | Teach 3-layer Federated KB (`vox_baseline`, `~/.vox/library/`, `docs/wiki/`), `vox repo` multi-repo onboarding (`--read-only`), and W3C SKOS + PPR search | 522 | `[x]` |
| `TUT-KB-03` | KB/Wiki Tut | Teach 5-layer Polyglot OpenWiki (`vox ci wiki-sync --polyglot`), `<!-- openwiki:auto:start -->` prose fences, and Tier-1 / Tier-2 self-healing | 522 | `[x]` |
| `TUT-KB-04` | KB/Wiki Tut | Teach non-modal multi-format `DocViewerDrawer` / `DocReader` (`Read \| Split \| Edit`) and include a compiling `.vox` `<VoxPlayground />` module | 522 | `[x]` |
| `UPG-01` | Upgrades | Upgrade `docs/src/tutorials/tut-getting-started.md` with Doc Dock visual preview, Step 4 KB/OpenWiki/Research tour, and tutorial links | 523 | `[x]` |
| `UPG-02` | Upgrades | Upgrade `docs/src/tutorials/tut-fullstack-walkthrough.mdx` with canonical `authorship_model`, OpenWiki 5-layer documentation callout, and tutorial cards | 523 | `[x]` |
| `UPG-03` | Upgrades | Upgrade `docs/src/tutorials/tut-workflow-durability.md` with SKOS frontmatter, `kb:summary`, duplicate-line cleanup, compiling workflow example, and tutorial links | 523 | `[x]` |
| `UPG-04` | Upgrades | Upgrade `docs/src/how-to/how-to-axis-workbench.md` Section 6 with `v8.7` 4-card visual showcase and non-modal Doc Studio / Memory / Research controls | 523 | `[x]` |
| `UPG-05` | Upgrades | Upgrade `docs/src/how-to/how-to-scientia-publication.md`, `how-to-ai-agents.md`, and `how-to/index.md` with active links, SKOS facets, and GUI walkthroughs | 523 | `[x]` |
| `UPG-06` | Upgrades | Upgrade `docs/src/index.mdx` hero kicker (`v8.8`), suite cards, Knowledge/OpenWiki/Research 4-card showcase, and Diátaxis map without altering `why_vox` | 523 | `[x]` |
| `VERI-01` | Quality Gate | Pass `vox-doc-pipeline --lint-only --paths ...` across all 10 authored/updated docs with `0` hard errors (`why_vox` parity + doctests) & `0` warnings after `--tighten-facet-backlog` | 524 | `[x]` |
| `VERI-02` | Build Gate | Pass `pnpm --dir docs-astro build` (`vox-diagram`, Starlight compilation, Pagefind, `llms.txt`, `starlight-links-validator`) with `0` errors | 524 | `[x]` |
| `VERI-03` | Portability | Confirm zero hardcoded machine paths (`/Users/brbrainerd/...`) and live interpreter execution of both interactive `<VoxPlayground />` `.vox` examples | 524 | `[x]` |
| `VERI-04` | Closeout | Update `.planning/PROJECT.md` and `v8.8` workstream state after all gates pass | 525 | `[x]` |
