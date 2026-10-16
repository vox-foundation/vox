# Phase 4: Taxonomy Contract and Facet Lint - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-10
**Phase:** 4-Taxonomy Contract and Facet Lint
**Areas discussed:** Schema home, Backlog ratchet, Authorship defaults, Vocabulary and summary

---

## Schema home

| Option | Description | Selected |
|--------|-------------|----------|
| New kb-facets types crate | L0 `vox-kb-facets`: struct, enums, taxonomy loader, generated schema; all consumers depend downward | ✓ |
| Inside vox-db-types | No new crate, but mixes doc vocabulary into a DB-types crate and widens fan-in | |
| Inside vox-doc-pipeline | Lint owns the schema; needs a vox-search -> vox-doc-pipeline edge and an exceptions entry | |

**User's choice:** New kb-facets types crate

| Option | Description | Selected |
|--------|-------------|----------|
| Committed generated schema + golden test | JSON Schema committed under contracts/documentation/, CI fails on divergence, GUI TS type generated from it | ✓ |
| Runtime-only validation | No committed artifact; drift surfaces only when a doc fails | |

**User's choice:** Committed generated schema + golden test

---

## Backlog ratchet

| Option | Description | Selected |
|--------|-------------|----------|
| Allowlist membership only | Not listed = full rules; listed = exempt; no git dependency | ✓ |
| Git diff vs merge-base | Blocks any touched doc; needs git state everywhere | |
| Content hash per backlog entry | Precise without git, but every edit churns the file | |

**User's choice:** Allowlist membership only

| Option | Description | Selected |
|--------|-------------|----------|
| Per-file with rule ids, generated, tighten-only | kb-facet-backlog.v1.yaml; `--tighten` only removes; additions need user authorization | ✓ |
| Per-file flat list | One path per doc, exempt from all rules | |
| Per-file plus one-off backfill tool | Adds a proposal tool for all 671 docs | |

**User's choice:** Per-file with rule ids, generated, tighten-only

---

## Authorship defaults

| Option | Description | Selected |
|--------|-------------|----------|
| Grandfather as-is | Missing authorship keeps today's behavior; forced-false binds declared web-promoted/LLM docs | ✓ |
| Fail closed | No authorship = ineligible; drops ~544 docs from the corpus | |
| Code blocks only | Unknown-authorship docs contribute only compiler-validated code blocks | |

**User's choice:** Grandfather as-is

| Option | Description | Selected |
|--------|-------------|----------|
| One shared function in the facets crate | `effective_training_eligible(&Facets)` used by lint and by extract_docs.rs | ✓ |
| Lint rejects, builder keeps its own check | Two implementations that can drift | |

**User's choice:** One shared function in the facets crate

---

## Vocabulary and summary

| Option | Description | Selected |
|--------|-------------|----------|
| Curated two-level set from where-things-live | ~10 domains, ~40-60 topics, hand-seeded | ✓ |
| Derive from crate names and doc paths | Script proposes, user prunes; noisy | |
| Start tiny, grow with the backlog | Cheapest now, frequent vocabulary misses later | |

**User's choice:** Curated two-level set from where-things-live

| Option | Description | Selected |
|--------|-------------|----------|
| HTML-comment delimited paragraph under the H1 | `<!-- kb:summary -->` ... `<!-- /kb:summary -->`; renders as lede | ✓ |
| Frontmatter field `kb_summary` | No body parsing; not a delimited block | |
| A `## Summary` heading section | Visible, but in the TOC and harder to strip | |

**User's choice:** HTML-comment delimited paragraph under the H1

---

## Claude's Discretion

- `sources[]` shape and hash/date formats (align with the Phase 3 findings inbox)
- Verbatim-quote cap unit and limits; detection method in Markdown
- altLabel integration with the Phase 2 indexer (query-time vs index-time)
- Lint rule ids, error format, `--tighten` CLI shape
- Required vs optional facets on a new doc; `sensitivity` values
- `vox-kb-facets` API surface; seeding command name and home

## Deferred Ideas

- Bulk facet-proposal/backfill tool for the 671 existing docs
- Code-blocks-only eligibility tier
- Git-diff "touched since merge-base" blocking
- LLM rewrite of docs (gated on TAX-05)
- Todo "Land local main through a draft PR to unblock nightly": reviewed, not folded
