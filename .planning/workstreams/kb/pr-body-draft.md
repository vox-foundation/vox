## Summary

**Draft.** Knowledge-base workstream (`kb`), milestone v1.1 "corpus to context", Phases 1-4. This branch is 362 commits ahead of `main` (about 440 files outside `.planning/`), pushed as a draft to get hosted CI. The most recent work is Phase 4; Phases 1-3 are already verified locally.

| Phase | Delivers |
|---|---|
| 1. Retrieval Eval Baseline | Deterministic ID-based retrieval eval, BEIR golden set, canary exclusion, ratcheted baseline in PR CI |
| 2. Library Index and KB Tool Surface | Content-hash incremental indexer with heading-path chunks, `docs/src/library/`, one `vox_kb_*` MCP surface |
| 3. Provenance, Trust Tiers and Leak Closure | Required trust tier on every row, `kb_findings` inbox, sanitizer, the two retrieval leaks closed, legacy tools retired; gap closures D-07/D-07a/D-08 |
| 4. Taxonomy Contract and Facet Lint | See below |

## Phase 4: Taxonomy Contract and Facet Lint (TAX-01..TAX-07)

- **One schema.** New L0 crate `vox-kb-facets` holds the `Facets` struct, `parse_facets`, the shared frontmatter splitter, the page-one summary block and `effective_training_eligible`. A committed JSON Schema (`contracts/documentation/kb-facets.v1.schema.json`) has a golden test, and the GUI `KbFacets` type is generated from it with a drift check wired into `ssot-drift`.
- **Controlled vocabulary.** `contracts/documentation/kb-taxonomy.v1.yaml`: 11 domains, 55 topics, altLabels and deprecated aliases. `vox_kb_search` expands altLabels at query time and facet lines are added to cards (`CHUNKER_VERSION` bump; the retrieval baseline `metrics` block is byte-identical, only epoch/note/chunker changed).
- **Facet lint** in `vox-doc-pipeline`: 19 `facet-*` rule ids for off-vocabulary topic/domain, summary outside 40-120 words, missing authorship, `sources[]` without url/date/hash, over-cap quotes, invalid attestation, unknown/near-miss keys and unparseable frontmatter.
- **Ratchet.** The ~667 existing docs pass through a generated, removal-only backlog (`kb-facet-backlog.v1.yaml`, seeded once: 667 docs, 2,668 rule ids). A fast-tier pre-push gate makes it shrink-only against the base ref; a re-epoch needs a note and cannot add rows for docs absent from the base.
- **Training eligibility.** The MENS builder uses the shared `effective_training_eligible` (web-promoted or LLM-authored pages without a real `human_attested_by` are forced ineligible), fails closed on unparseable frontmatter, and strips summary blocks before both lanes. Docs with no `authorship` are grandfathered, so the current corpus is unchanged (real-tree oracle test).
- **Code review** found 1 critical (frontmatter shapes that parsed as "no facets" stayed eligible) and 5 warnings; all fixed with tests and mutation proofs (`04-REVIEW.md`, `04-REVIEW-FIX.md`).

### Crate edges (authorized)
Exactly three new exceptions in `contracts/ci/crate-edges.allow.v1.json`, each recorded with the user's explicit chat approval (2026-10-10): `vox-doc-pipeline`, `vox-search`, `vox-corpus` -> `vox-kb-facets` (all L3 to L0, downward). No other new workspace edges.

## Verification

- Local, targeted only (lean policy): every plan ran its own new tests plus `cargo check`; mutation proofs recorded in each SUMMARY. Real-tree `vox-doc-pipeline --lint-only` exits clean with the seeded backlog.
- Phase 4 status is `human_needed`, not `passed`: **this draft PR exists to run hosted CI**, which is the remaining gate. Still to confirm: `linux` (including the new backlog gate and `ssot-drift` for `kb_facets_ts`), `tests`, `ui` (`kbFacets.generated.test.ts`, typecheck).
- Open decision for the owner: TAX-07 `sources[]` completeness is promoted-only per REQUIREMENTS/CONTEXT, while ROADMAP success criterion 1 reads unscoped.

## Known / out of scope
- Pre-existing `vox-arch-check` `no-hardcoded-abs-path` at `crates/vox-orchestrator-mcp/src/browser_tools.rs:965`.
- `kb_leak_chat_turn_writes_no_kb_entry_or_finding` was failing since Phase 3 commit `51ddfcd16` (test counted attach-time migration rows); fixed in this branch (`bb7630939`).
- Phase 3 upgrade-path residuals WR-01/WR-02 (accepted as pre-release; baseline 96 is on no remote branch).
- Indented-fence limitation in the summary scanner and the review's INFO items are logged in `deferred-items.md`.
- Hosted nightly is currently red on unrelated jobs (CodeQL, mutation, docs link checker, GUI cross-platform).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
