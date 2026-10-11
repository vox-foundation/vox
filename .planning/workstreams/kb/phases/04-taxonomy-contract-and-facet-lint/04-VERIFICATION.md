---
phase: 04-taxonomy-contract-and-facet-lint
verified: 2026-10-11T08:30:00Z
status: human_needed
score: 4/4 must-haves verified
covered_files:
  - .planning/workstreams/kb/REQUIREMENTS.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-01-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-01-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-02-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-02-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-03-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-03-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-04-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-04-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-05-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-05-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-06-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-06-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-07-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-07-SUMMARY.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-08-PLAN.md
  - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-08-SUMMARY.md
  - contracts/ci/crate-edges.allow.v1.json
  - contracts/documentation/kb-facet-backlog.v1.yaml
  - contracts/documentation/kb-facets.v1.schema.json
  - contracts/documentation/kb-taxonomy.v1.yaml
  - crates/vox-cli-ci/src/kb_facets_ts.rs
  - crates/vox-cli/src/commands/kb/facet_backlog.rs
  - crates/vox-corpus/src/corpus/extract_docs.rs
  - crates/vox-doc-pipeline/src/pipeline/facet_backlog.rs
  - crates/vox-doc-pipeline/src/pipeline/facets.rs
  - crates/vox-gui/ui/src/generated/kbFacets.generated.ts
  - crates/vox-kb-facets/src/backlog.rs
  - crates/vox-kb-facets/src/frontmatter.rs
  - crates/vox-kb-facets/src/keys.rs
  - crates/vox-kb-facets/src/lib.rs
  - crates/vox-kb-facets/src/provenance.rs
  - crates/vox-kb-facets/src/summary.rs
  - crates/vox-kb-facets/src/taxonomy.rs
  - crates/vox-kb-facets/tests/facet_schema_golden.rs
  - crates/vox-search/src/library/chunk.rs
  - crates/vox-search/src/library/mod.rs
covered_digest: "v1:sha256:946d95a2ffdcc00034d0187d9896e3fc7cf1e9ccf6add5e1e22043c9a91ddfc9"
behavior_unverified: 0
overrides_applied: 0
gaps: []
human_verification:
  - test: "Hosted CI on the phase PR: the `linux` job (fast tier incl. facet backlog tighten-only gate, ssot-drift incl. kb_facets_ts, clippy on affected crates), `tests` shards, and `ui`"
    expected: "All green. The local verification below ran only targeted tests; the facet backlog gate was exercised by unit tests plus a real-git test but not against a real base ref."
    why_human: "Hosted CI is the project gate; lean policy forbids workspace-wide local builds/tests."
  - test: "GUI vitest and typecheck: `pnpm --dir crates/vox-gui/ui test` (kbFacets.generated.test.ts) and `pnpm --dir crates/vox-gui/ui typecheck`"
    expected: "kbFacets.generated.ts enum/key values equal contracts/documentation/kb-facets.v1.schema.json; typecheck clean"
    why_human: "Not run in this verification (GUI toolchain run is out of the lean scope). The Rust-side half of the drift check (golden test) was run and passes."
  - test: "Decide whether the TAX-07 `sources[]` completeness rule should stay promoted-only"
    expected: "ROADMAP SC1 reads 'sources[] entry without url/date/hash' (unscoped); REQUIREMENTS TAX-07 and CONTEXT scope required url/date/hash to `origin: web-promoted` pages. Code follows the requirement: missing fields fail only on promoted pages, malformed date/hash fail on any page."
    why_human: "Product decision, not a code defect; only the user can say whether an LLM-authored non-promoted page with an incomplete sources[] entry should also hard-fail (see IN-06 in 04-REVIEW.md)."
---

# Phase 4: Taxonomy Contract and Facet Lint Verification Report

**Phase Goal:** Every doc's facets come from one controlled vocabulary and one schema, enforced by a ratcheting lint. Ranking, staleness, rewrites and training eligibility then read fields they can trust.
**Verified:** 2026-10-11
**Status:** human_needed (all four success criteria verified in code; items needing hosted CI, the GUI run and one product decision are listed)
**Re-verification:** No - initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | New/changed doc fails the lint for off-vocabulary topic/domain, missing or out-of-range (40-120 words) page-one summary, missing authorship, sources[] entry without url/date/hash, over-cap verbatim quote; untouched existing docs pass through a backlog allowlist that can only shrink | VERIFIED | `facets.rs` rules `facet-unknown-topic/domain`, `facet-missing-summary`, `facet-summary-word-count` (range from `SUMMARY_MIN/MAX_WORDS`), `facet-missing-authorship`, `facet-source-field`, `facet-quote-over-cap` (50 words/quote, 200/page), all hard (`is_hard` false only for `BacklogStale`). Backlog: 667 rows in `kb-facet-backlog.v1.yaml` == 667 non-archive, non-generated docs; `lint_facets` excuses only listed rule ids per doc and warns on stale excuses. Tighten-only: `Backlog::tighten_only_violations` + `run_facet_backlog_gate` wired as a pre-push fast-tier step (`pre_push.rs:529`). `cargo test -p vox-doc-pipeline --lib`: 52 passed. Real-tree `VOX_SKIP_FRESHNESS_CHECK=1 cargo run -p vox-doc-pipeline -- --lint-only`: exit 0, "no hard errors". Caveat: required url/date/hash presence is promoted-only (see human_verification 3). |
| 2 | Lint, indexer and GUI types derive from one schemars facet schema (kb-taxonomy.v1.yaml plus generated facet JSON Schema); a drift test fails when any consumer diverges | VERIFIED (GUI half needs human run) | One `vox_kb_facets::Facets` struct (`JsonSchema` derive, `lib.rs`) is imported by `facets.rs` (lint), `library/chunk.rs` + `library/mod.rs` (indexer), `extract_docs.rs` (MENS). Golden: `tests/facet_schema_golden.rs::facet_schema_matches_committed_contract` passes (`cargo test -p vox-kb-facets`: 58 unit + 1 golden). TS type `kbFacets.generated.ts` is generated from `kb-facets.v1.schema.json` by `kb_facets_ts.rs`; `run_ssot_drift` dispatches `kb_facets_ts` (`docs.rs:685`, with a guard test), and `scripts/ssot-regen.vox` regenerates it. Vitest not run here (human_verification 2). |
| 3 | Lexical search for a taxonomy altLabel returns the doc tagged with the prefLabel; deprecated aliases resolve to their replacement | VERIFIED | `Taxonomy::expand_query` appends a quoted `topic|domain <prefLabel>` phrase for any altLabel or deprecatedAlias token; indexer card writes `topic:`/`domain:` lines (`facet_card_lines`); production MCP `vox_kb_search` calls `search_with_taxonomy` with `load_taxonomy`. `cargo test -p vox-search --test kb_library_facets`: 5 passed incl. `altlabel_query_finds_the_preflabel_tagged_doc` (altLabel and deprecated alias both hit; without taxonomy the same query misses) and `real_taxonomy_altlabel_finds_tagged_doc` against the committed YAML (56 altLabels, 7 deprecatedAliases). |
| 4 | origin: web-promoted or LLM authorship without human_attested_by is forced to training_eligible: false; fixture shows MENS builder excludes it and emits no page-one summary-block text from any doc | VERIFIED | `forced_training_ineligible`/`effective_training_eligible` in `lib.rs`; `extract_docs::parse_frontmatter` uses them (typed parse) and `strip_summary_blocks` runs before both lanes. `cargo test -p vox-corpus --lib facet_tests`: 15 passed incl. `web_promoted_page_with_flag_true_yields_no_pairs`, `llm_authorship_without_attestation_yields_no_pairs`, `placeholder_attestation_does_not_clear_the_forced_exclusion`, `summary_text_never_reaches_any_pair` (3 shapes incl. inline and unterminated), `real_docs_eligibility_unchanged_for_facetless_docs` (oracle vs. legacy rule on the real tree). `walk_and_extract_docs` is the only doc lane (callers: mix_prepare, ml-cli, kb gate); `source_pool.rs` handles `.vox` files only. Lint rejects the contradictory `training_eligible: true` (`facet-training-eligible-contradiction`). |

**Score:** 4/4 truths verified (0 behavior-unverified)

### Code-review fixes re-checked (CR-01, WR-01..05)

| Finding | Held? | Evidence |
|---------|-------|----------|
| CR-01 fail-closed on unrecognised/unterminated frontmatter | YES | `parse_facets` returns `Err` when `frontmatter_yaml` is `None` but `fence_intended` (first non-blank line, BOM/indent tolerated, is `---`); `extract_docs` maps `Err` to ineligible; lint reports hard `facet-parse-error` even for backlogged docs (`facet_issues` returns it before any excuse logic; `facet-parse-error` is a hard rule, backlog can only excuse listed ids). BOM stripped, lone CR split. Fenceless files keep the legacy substring opt-out. Tests `parse_facets_unrecognised_fence_shapes_fail_closed`, `unrecognised_or_shifted_frontmatter_never_trains_an_opted_out_page`, `bom_and_cr_only_frontmatter_without_opt_out_still_trains`, `fenceless_file_keeps_the_legacy_substring_opt_out` all pass in my runs. |
| WR-01 re-epoch gate | YES | `tighten_only_violations` on `head.epoch > base.epoch` still requires a new note AND rejects every head row whose path is absent from the base tree; `docs_at` uses `git ls-tree -r --name-only -z <base> -- docs/src` and errors (never an empty set) on git failure; bootstrap only on proven absence (`file_at`). Residual by design: a re-epoch may re-seed rules for docs that existed at the base (user-authorized re-seed). |
| WR-02 attestation placeholders | YES | `is_valid_attestation` + `NOT_A_HUMAN` feeds both `forced_training_ineligible` and hard rule `facet-invalid-attestation`. |
| WR-03 near-miss facet keys | YES | `facet_key_near_misses` -> hard `facet-unknown-key`; MENS builder excludes docs with an eligibility-key near-miss. |
| WR-04 / WR-05 | YES | Escaped-quote guard in `expand_query`; shared `FenceTracker` used by summary and provenance scanners. |

Open INFO items do not defeat any success criterion: IN-05 (a doc with any invalid facet loses its card topic/domain lines) only affects docs that already fail lint with a hard `facet-parse-error`; IN-04 (MCP `taxonomy_warning` embeds `{e:#}`, still present at `kb_tools/library.rs:121`) is a minor path-disclosure/hygiene item on the degraded path; IN-02/IN-03/IN-06 are tooling edge cases or policy suggestions.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-kb-facets` (L0, `layers.toml:127`) | VERIFIED | 7 modules, real logic, consumed by 3 crates + cli tooling |
| `contracts/documentation/kb-taxonomy.v1.yaml` | VERIFIED | two-level domains/topics, altLabels, deprecatedAliases; loader enforces label uniqueness |
| `contracts/documentation/kb-facets.v1.schema.json` | VERIFIED | golden test matches schemars output |
| `contracts/documentation/kb-facet-backlog.v1.yaml` | VERIFIED | 667 rows == live non-archive doc count; registered in `contracts/index.yaml` |
| `crates/vox-gui/ui/src/generated/kbFacets.generated.ts` | VERIFIED (vitest pending) | header cites generator; drift wired into `ssot-drift` |
| `contracts/ci/crate-edges.allow.v1.json` | VERIFIED | exactly three new edges (`vox-doc-pipeline`, `vox-search`, `vox-corpus` -> `vox-kb-facets`), `authorized_by` "user (explicit chat approval, 2026-10-10)" per D-09; no other crate Cargo.toml depends on `vox-kb-facets` |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| `vox-doc-pipeline` lint | `vox_kb_facets::{parse_facets, Taxonomy, Backlog, inspect_summary, quote_word_counts}` | WIRED |
| `extract_docs.rs` | `parse_facets`, `effective_training_eligible`, `facet_key_near_misses`, `strip_summary_blocks` | WIRED |
| `library/chunk.rs` card | `facet_card_lines` | WIRED |
| `vox_kb_search` MCP tool | `load_taxonomy` -> `search_with_taxonomy` | WIRED |
| `run_ssot_drift` | `kb_facets_ts::run(root, true)` | WIRED |
| pre-push fast tier | `run_facet_backlog_gate` | WIRED |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| TAX-01 | 04-02, 04-06 | SATISFIED | taxonomy YAML + loader + altLabel/alias expansion; search test passes |
| TAX-02 | 04-01, 04-03, 04-06 | SATISFIED | single `Facets` schemars struct; golden + generated TS + ssot-drift |
| TAX-03 | 04-03, 04-05, 04-08 | SATISFIED | `inspect_summary` 40-120 words, markers, placement; lint rules |
| TAX-04 | 04-05, 04-07, 04-08 | SATISFIED | backlog file + tighten-only gate + stale-excuse warnings |
| TAX-05 | 04-03, 04-05, 04-08 | SATISFIED | `authorship` required on non-backlogged docs; `authorship_model` for LLM authorship |
| TAX-06 | 04-01, 04-03, 04-04, 04-08 | SATISFIED | forced-false rule shared by lint and MENS; summary strip |
| TAX-07 | 04-03, 04-08 | SATISFIED | sources[] url/date/hash + quote cap (promoted-only, per requirement/CONTEXT) |

All seven IDs appear in PLAN frontmatter and are `[x]`/Complete in REQUIREMENTS.md. No orphaned Phase 4 requirement IDs.

### Behavioral Spot-Checks (targeted, lean)

| Behavior | Command | Result |
|----------|---------|--------|
| Facet contract + schema golden + CR-01/WR-0x unit tests | `cargo test -p vox-kb-facets` | 58 + 1 passed |
| MENS eligibility + summary strip + real-tree oracle | `cargo test -p vox-corpus --lib facet_tests` | 15 passed |
| Lint rules, backlog, near-miss, attestation | `cargo test -p vox-doc-pipeline --lib` | 52 passed |
| altLabel / alias search through real indexer | `cargo test -p vox-search --test kb_library_facets` | 5 passed |
| Real-tree lint with committed backlog | `VOX_SKIP_FRESHNESS_CHECK=1 cargo run -p vox-doc-pipeline -- --lint-only` | exit 0, "no hard errors" |

Not run (by instruction): workspace builds/tests, `vox-cli` backlog-gate tests, GUI vitest/typecheck, `vox ci ssot-drift`, mutation re-proofs (REVIEW-FIX documents them; I read the guarded code paths instead).

### Anti-Patterns Found

None blocking. No TBD/FIXME/XXX introduced in the reviewed facet files that lacks an issue ref (not exhaustively grepped across all phase files; the lint and code review found none).

### Known out-of-scope items (not counted against the phase)

Pre-existing arch-check `no-hardcoded-abs-path` in `browser_tools.rs`; Phase 3 `kb_leak` test-isolation failure (root-caused in `deferred-items.md`); indented-fence scanner limitation (accepted, documented).

### Human Verification Required

See frontmatter `human_verification`: (1) hosted CI green on the phase PR, (2) GUI vitest and typecheck, (3) product decision on promoted-only sources[] completeness.

### Gaps Summary

No gaps. All four ROADMAP success criteria and all seven TAX requirements are backed by code, wiring and passing targeted tests; CR-01 fail-closed behavior and the WR-01 re-epoch gate hold in the current code. Remaining items are confirmation steps that need hosted CI, the GUI toolchain, and one scoping decision.

---

_Verified: 2026-10-11_
_Verifier: Claude (gsd-verifier)_
