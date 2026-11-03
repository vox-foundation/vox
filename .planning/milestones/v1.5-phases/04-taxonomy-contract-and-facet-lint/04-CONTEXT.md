# Phase 4: Taxonomy Contract and Facet Lint - Context

**Gathered:** 2026-10-10
**Status:** Ready for planning

<domain>
## Phase Boundary

Every doc's facets come from one controlled vocabulary and one schemars schema, enforced by a ratcheting lint, so ranking, staleness, rewrites and training eligibility read fields they can trust. Delivers TAX-01..TAX-07:

- the SKOS-lite vocabulary (`contracts/documentation/kb-taxonomy.v1.yaml`) with altLabel synonyms and deprecated aliases;
- one schemars facet schema shared by the doc lint, the indexer and the GUI;
- the 40-120-word page-one summary block;
- `authorship`, `origin` and `human_attested_by`, with the forced `training_eligible: false` rule honored by the MENS corpus builder;
- `sources[]` (url, date, hash) and a verbatim-quote cap;
- a ratcheting backlog allowlist for existing docs.

The altLabel-synonym and indexer-facet integration lands here because Phase 2's indexer now exists. LLM rewrites, the Library GUI and ranking use of the facets belong to later phases.

Scale at discussion time: 671 docs under `docs/src` outside `archive/`. None carry `topic`, `domain`, `authorship`, `origin`, `sources`, `sensitivity`, `review_by` or `verified_at`. 394 say `training_eligible: true`, 127 say `false`, 150 omit it (the MENS builder defaults a missing value to `true`).

</domain>

<decisions>
## Implementation Decisions

### Schema home and drift guarantee
- **D-01:** The facet struct, enums, taxonomy loader and `effective_training_eligible()` live in a new L0 types crate `vox-kb-facets`. The doc lint (`vox-doc-pipeline`), the indexer (`vox-search`), the MENS builder (`vox-corpus`) and the GUI all depend on it downward. It needs a `layers.toml` `[crates]` row (layer 0) at creation and three `crate-edges` exceptions entries (see D-09). — **Reversibility:** costly — moving the struct later touches every consumer's imports and the generated schema's source path.
- **D-02:** Drift is enforced by a committed generated JSON Schema under `contracts/documentation/` plus a golden test that fails when the file differs from the schemars output (reuse `vox-jsonschema-util`). The GUI TypeScript facet type is generated from that same file and a typecheck/vitest check fails on divergence. Runtime-only validation was rejected because drift would only surface when a doc fails, not at PR time. — **Reversibility:** reversible — the golden test and generator are replaceable without touching doc content.

- **D-09 (2026-10-10, user explicit chat approval at plan-phase):** AUTHORIZED. Add three `crate-edges` exceptions entries, each `authorized_by: "user (explicit chat approval, 2026-10-10)"`, in the same PR as the edges: `vox-doc-pipeline -> vox-kb-facets`, `vox-search -> vox-kb-facets`, `vox-corpus -> vox-kb-facets` (all L3 to L0, downward; `NewEdge` fires otherwise because the ratchet is an exact edge set). The reason text for each names what it reads (lint rules, altLabel/facets at the indexer, `effective_training_eligible` in the MENS builder). This corrects D-01's earlier "no exceptions entry" note. No other new workspace edges are authorized; anything beyond these three still stops for approval.

### Backlog ratchet
- **D-03:** "New or changed" means allowlist membership only. A doc not in the backlog allowlist is held to the full facet rules. A new doc is never listed, so it is blocked. A doc leaving the list must pass. Editing a backlog doc does not by itself force compliance. There is no git or content-hash dependency, so pre-push, CI and local runs behave identically. — **Reversibility:** reversible — switching to a git-diff rule later is additive.
- **D-04:** The backlog is `contracts/documentation/kb-facet-backlog.v1.yaml`, per-file with the rule ids each doc still violates, generated once by a seeding command and never hand-edited. `--tighten` only removes rows or rule ids. Adding a row or rule id needs your authorization, the same discipline as `crate-edges`. No bulk backfill tool is built in this phase. — **Reversibility:** costly — the file shape becomes a contract that the lint, the tighten command and CI all read.

### Authorship and training eligibility
- **D-05:** A backlog doc with no `authorship` is grandfathered. It keeps today's behavior (its `training_eligible` flag, default `true`), so the current MENS corpus is not disturbed. The forced-false rule binds docs that declare `origin: web-promoted` or LLM authorship (`llm-assisted` / `llm-generated`) and lack `human_attested_by`. Every new doc must declare `authorship`, so the exposure shrinks as the backlog shrinks. Fail-closed (~544 docs dropped) and a code-blocks-only middle tier were rejected. — **Reversibility:** reversible — flipping the default is a one-line change plus a corpus rebuild.
- **D-06:** One shared function, `effective_training_eligible(&Facets)` in `vox-kb-facets`, decides eligibility. The lint uses it to reject a contradictory doc, and `vox-corpus/src/corpus/extract_docs.rs` replaces its substring match on `training_eligible: false` with the typed parse and the same function. A fixture proves a web-promoted or LLM page with no `human_attested_by` is excluded and that no page-one summary-block text is emitted from any doc. — **Reversibility:** costly — the MENS builder's parsing changes and corpus output is regression-tested against it.

### Vocabulary and page-one summary
- **D-07:** The first vocabulary is a curated two-level set: about 10 coarse `domain` values and about 40-60 `topic` prefLabels, each with `broader` pointing at a domain. It is seeded by hand from the concept-to-crate table in `docs/src/architecture/where-things-live.md`, with obvious synonyms and crate names as altLabels. Growth goes through PR review of the YAML. `topic`/`domain` are new subject axes orthogonal to the existing `category` (sidebar/doc-type) and `status` fields, which are unchanged. — **Reversibility:** reversible — retire a label via a deprecated alias that resolves to its replacement.
- **D-08:** The page-one summary is an HTML-comment-delimited paragraph directly under the H1: `<!-- kb:summary -->` text `<!-- /kb:summary -->`. It renders as a normal lede paragraph. The lint counts whitespace-split words (40-120 inclusive, link text counted). It is independent of the short frontmatter `description` (median 18 words), and the MENS builder strips the whole block. A frontmatter `kb_summary` field and a `## Summary` heading were rejected. — **Reversibility:** one-way — the markers get written into every migrated doc and into the MENS strip logic, so changing them later means rewriting docs.

### Claude's Discretion
- `sources[]` shape: `url`, `date` and `hash` are required on promoted pages. Pick the hash algorithm and date format to match the Phase 3 findings inbox (`kb_findings`) so a promoted finding maps onto `sources[]` without conversion.
- The verbatim-quote cap: choose the unit (words per quote and a per-page total), how a quote is detected in Markdown, and the numeric limits. Apply it to promoted pages only.
- How altLabels reach lexical search: query-time synonym expansion versus index-time terms in the Phase 2 tantivy indexer, and how deprecated aliases resolve to their replacement. Success criterion 3 must hold either way.
- Lint rule ids, error message format and `--tighten` CLI shape, following the existing `vox-doc-pipeline` lint conventions (`lint.rs`, including its `suggest` did-you-mean helper).
- Which facets are required versus optional on a new doc (`kb_tier`, `trust_tier`, `scope`, `review_by`, `verified_at`, `supersedes`/`superseded_by`, `sensitivity` values). Keep required fields to what TAX-01..TAX-07 name.
- The `kb-facets` crate's public API surface, and the seeding command's name and home.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements and roadmap
- `.planning/workstreams/kb/REQUIREMENTS.md` TAX-01..TAX-07 — the seven requirements this phase closes.
- `.planning/workstreams/kb/ROADMAP.md` Phase 4 — goal and success criteria (lint failure cases, one-schema drift test, altLabel lexical hit, forced-false training eligibility).
- `.planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-CONTEXT.md` — D-07/D-08 trust rules and the `kb_findings` inbox that promotion flows from.

### Docs governance and lint
- `docs/src/contributors/documentation-governance.md` — frontmatter vocabulary (`category`, `status`), maintenance rules, and the lint contract.
- `crates/vox-doc-pipeline/src/pipeline/lint.rs` — existing frontmatter lint, its tests and the `valid_categories_match_sidebar_sections` precedent.
- `AGENTS.md` §Authored Markdown Frontmatter, §Dependency Discipline, §Cryptography Policy — frontmatter, crate-edge and hashing rules.

### Architecture rules
- `docs/src/architecture/layers.toml` — layer assignments; the new crate needs a row at creation.
- `docs/src/architecture/where-things-live.md` — concept-to-crate table that seeds the vocabulary; add a row for the new crate.
- `contracts/ci/crate-edges.allow.v1.json` — the ratchet-discipline precedent for the backlog allowlist (`--tighten`, user-authorized additions only).

### Consumers
- `crates/vox-corpus/src/corpus/extract_docs.rs` — current MENS doc eligibility (substring match, default-true) that D-06 replaces.
- `crates/vox-search/src/library/mod.rs` and `docs/src/architecture/search-retrieval-ssot-2026.md` — the Phase 2 indexer and retrieval contract where altLabels and facets integrate.
- `crates/vox-jsonschema-util/` — existing JSON Schema utilities for the generated schema and drift test.

### Research
- `docs/src/architecture/knowledge-base-practices-research-2026.md` and `docs/src/architecture/kb-systems-sota-research-2026.md` — background on taxonomy, provenance and authorship practices.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox-doc-pipeline` lint (`lint.rs`, 1118 lines): frontmatter parsing, `suggest` did-you-mean, `gather_md_files`, parallel error collection, and the `--lint-only --paths` scoped mode. The facet rules extend it.
- `vox-db-types` `kb_text.rs`: the `SanitizedText` precedent for an L0 types crate that guards a trust boundary.
- `vox-jsonschema-util` and workspace `schemars = 1` (derive): schema generation already available.
- `crate-edges` ratchet with `--tighten`: the model for the backlog file's tighten-only semantics.

### Established Patterns
- Contract YAML under `contracts/documentation/` (`canonical-map.v1.yaml`, `link-allowlist.v1.yaml`, `retired-symbols.v1.yaml`) with a `.v1` suffix. The taxonomy, generated schema and backlog follow it.
- Generated artifacts carry drift checks (`vox ci ssot-drift`); the facet schema golden test should be wired the same way.
- New crates get a `layers.toml` row at creation and sit L0 when pure types.

### Integration Points
- Doc lint (`vox-doc-pipeline`): reads facets from frontmatter plus the summary block in the body.
- Phase 2 indexer (`vox-search`): reads altLabels and facets at index or query time.
- MENS builder (`vox-corpus`, `extract_docs.rs`): switches to the typed parse and `effective_training_eligible()`.
- GUI: a generated TypeScript facet type consumed by the Library surface in Phase 6.

</code_context>

<specifics>
## Specific Ideas

- Treat `docs/src/archive/` as out of scope for both the lint and the seeded backlog (tombstoned, per the Archival Protocol).
- The backlog seeding command should print the count of docs and rule ids it recorded so the starting size of the ratchet is visible in the PR.

</specifics>

<deferred>
## Deferred Ideas

- A bulk facet-proposal/backfill tool for the 671 existing docs — a review load far beyond the seven TAX requirements; the backlog shrinks opportunistically as docs are touched.
- Code-blocks-only eligibility tier for unknown-authorship docs — rejected for this phase to avoid a second eligibility tier.
- Git-diff "touched since merge-base" blocking — revisit if the backlog does not shrink on its own.
- LLM rewrite of docs — gated on `authorship` being enforced first (TAX-05); belongs in a later phase.

### Reviewed Todos (not folded)
- "Land local main through a draft PR to unblock nightly" — matched only on generic keywords; unrelated to taxonomy, so not folded.

</deferred>

---

*Phase: 4-Taxonomy Contract and Facet Lint*
*Context gathered: 2026-10-10*
