# vox-doc-pipeline

Documentation governance, SKOS taxonomy validation, homepage sync, and compiler-verified `.vox` doctest runner for the Vox knowledge base (`docs/src/`).

> **Note:** `SUMMARY.md` and `mdBook` are retired (`SUMMARY.md` is `.gitignore`d). The documentation site runs on **Astro 7.4 + Starlight 0.42**, where `docs-astro/src/utils/sidebar.mjs` builds the navigation sidebar directly from YAML frontmatter (`title`, `category`, `sort_order`).

## What It Validates (`--lint-only`)

Running `vox-doc-pipeline` (in the pre-push hook and CI) enforces:

- **Required YAML Frontmatter**: Validates `title`, `description`, `category`, `topic`, `domain`, and `authorship` (plus `authorship_model` when LLM-authored) across `docs/src/`.
- **SKOS Taxonomy Facets**: Verifies `topic` and `domain` `prefLabel` values against [`contracts/documentation/kb-taxonomy.v1.yaml`](../../contracts/documentation/kb-taxonomy.v1.yaml).
- **Shrink-Only Facet Backlog**: Enforces that [`contracts/documentation/kb-facet-backlog.v1.yaml`](../../contracts/documentation/kb-facet-backlog.v1.yaml) only shrinks and no unexempted page omits required facets.
- **KB Summary Blocks**: Checks for a 40–120 word `<!-- kb:summary -->` block immediately under the H1 heading on governed docs.
- **Homepage Sync (`why_vox`)**: Verifies synchronization between root `README.md` and `docs/src/index.mdx`.
- **VoxDiagram Validation**: Validates embedded architecture and workflow diagram definitions.
- **Compiler-Verified `.vox` Doctests**: Lexes, parses, and typechecks fenced `vox` code blocks across `docs/src/`, rejecting `// vox:skip` annotations in non-archive pages unless explicitly justified.

## Usage

```bash
# Lint the entire documentation tree
cargo run -p vox-doc-pipeline -- --lint-only

# Scoped lint on specific modified files (fast pre-push check)
cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/docs-system-ssot.md
```

## Canonical References

- Policy & Frontmatter Vocabulary: [`docs/src/contributors/documentation-governance.md`](../../docs/src/contributors/documentation-governance.md)
- Pipeline Architecture & Drift Gates: [`docs/src/architecture/docs-system-ssot.md`](../../docs/src/architecture/docs-system-ssot.md)
