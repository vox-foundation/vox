---
phase: 467
phase_name: "polyglot-five-layer-openwiki-and-okf-fences"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 3
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 467 Learnings: polyglot-five-layer-openwiki-and-okf-fences

## Decisions

### Implemented extract_openwiki_auto_regions and non-destructive merge_open...
Implemented `extract_openwiki_auto_regions` and non-destructive `merge_openwiki_auto_regions` preserving all human prose outside `<!-- openwiki:auto:start id="..." --> ... <!-- openwiki:auto:end -->` blocks.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 467-SUMMARY.md

---

### Built extract_polyglot_wiki_snapshot, render_polyglot_openwiki_pages, op...
Built `extract_polyglot_wiki_snapshot`, `render_polyglot_openwiki_pages`, `openwiki_init`, `openwiki_build`, `openwiki_status`, and `openwiki_diff` generating OKF frontmatter, 50–65 word `kb:summary` blocks, validated `vox-diagram` topologies (`0` legacy Mermaid), `#Lstart-Lend` source anchors, and `.vox/cache/openwiki/manifest.v1.json` across all 5 system layers (`README.md`, `architecture-topology.md`, `interfaces-and-apis.md`, `data-models-and-schemas.md`, `gui-surfaces-and-flows.md`, `workflows-and-runbooks.md`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 467-SUMMARY.md

---

### Enabled external repository --lint-only validation for docs/wiki//*.md (...
Enabled external repository `--lint-only` validation for `docs/wiki/**/*.md` (`FacetCtx::load_or_embedded` + `mermaid_gate` `docs/wiki` scan) while keeping `docs/src/` contracts strictly fail-closed on Vox.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 467-SUMMARY.md

---

### Upgraded render_wiki_readme so newly onboarded repos receive valid OKF f...
Upgraded `render_wiki_readme` so newly onboarded repos receive valid OKF frontmatter and `<!-- kb:summary -->` metadata out of the box.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 467-SUMMARY.md

---

### Wired --polyglot, --init, --status, --diff, --repo-root, --read-only, an...
Wired `--polyglot`, `--init`, `--status`, `--diff`, `--repo-root`, `--read-only`, and `--json` flags on `vox ci wiki-sync` and `--wiki {init,build,status,diff}` + `--write` on `vox repo` without altering command catalog baseline paths.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 467-SUMMARY.md

## Lessons

### Cleanly decoupled polyglot OpenWiki generation from Vox-specific docs/sr...
Cleanly decoupled polyglot OpenWiki generation from Vox-specific `docs/src/architecture/layers.toml` while preserving `generate_subsystem_hubs` unchanged on Vox.

**Context:** Surfaced in structured phase code review.
**Source:** 467-REVIEW.md

---

### Extracts modules/manifests (Cargo.toml, package.json, pyproject.toml, go...
Extracts modules/manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `Vox.toml`, optional `layers.toml`), public API/CLI/HTTP routes, SQL DDL + typed data models, TSX/JSX/Vox GUI surfaces, and `.github/workflows` / `scripts/*.vox` / actor workflows into 6 OKF pages under `docs/wiki/`.

**Context:** Surfaced in structured phase code review.
**Source:** 467-REVIEW.md

---

### Uses existing vox_doc_pipeline::claims::hasher::hash_normalized_code rat...
Uses existing `vox_doc_pipeline::claims::hasher::hash_normalized_code` rather than importing raw hashing crates (`AGENTS.md §Cryptography Policy`), and emits validated `vox-diagram` fences (`vox_diagram::check`) with zero legacy Mermaid blocks.

**Context:** Surfaced in structured phase code review.
**Source:** 467-REVIEW.md

## Patterns

## Surprises

