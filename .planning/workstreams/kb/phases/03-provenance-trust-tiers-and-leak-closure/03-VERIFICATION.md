---
phase: 03-provenance-trust-tiers-and-leak-closure
verified: 2026-10-09T00:00:00Z
status: human_needed
score: 6/6 must-haves verified
covered_files:
  - .planning/workstreams/kb/REQUIREMENTS.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-01-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-01-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-02-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-02-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-03-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-03-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-04-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-04-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-05-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-05-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-06-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-06-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-07-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-07-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-08-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-08-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-09-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-09-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-10-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-10-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-11-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-11-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-12-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-12-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-13-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-13-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-14-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-14-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-15-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-15-SUMMARY.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-16-PLAN.md
  - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/03-16-SUMMARY.md
  - crates/vox-db-types/src/kb_text.rs
  - crates/vox-db/src/local_tests.rs
  - crates/vox-db/src/store/open.rs
  - crates/vox-db/src/store/ops_memory/kb_findings.rs
  - crates/vox-db/src/store/ops_memory/kb_legacy.rs
  - crates/vox-db/src/store/ops_memory/library.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/findings.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
  - crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs
  - crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs
  - crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs
  - crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs
  - crates/vox-orchestrator-types/src/trust.rs
  - crates/vox-orchestrator/src/memory/manager.rs
  - crates/vox-search/src/bundle.rs
  - crates/vox-search/src/execution.rs
  - crates/vox-search/src/kb_eval/library.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/src/trust.rs
  - crates/vox-search/tests/execute_search_plan_smoke.rs
  - crates/vox-search/tests/kb_library_chunking.rs
  - crates/vox-search/tests/kb_library_index.rs
  - crates/vox-search/tests/web_capture_test.rs
covered_digest: "v1:sha256:8725973c91887a2073ae3fc6f8c99c5723479f9afb5de9f96f7af7b2ebc8449b"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/6
  gaps_closed:
    - "03-09 truth: unprompted preamble never carries an Untrusted hit; envelope tier = min of contributing hits (CR-01 Tavily bypass, WR-08)"
    - "ROADMAP SC4: sanitizer at the injection boundary covers research-turn Source.url / engine and deep-path pipeline_sources (WR-05)"
  gaps_remaining: []
  regressions: []
gaps: []
human_verification:
  - test: "Hosted CI is green on this branch: nextest for vox-db-types, vox-db, vox-search, vox-orchestrator-types, vox-orchestrator, vox-orchestrator-mcp, vox-gui, vox-ml-cli; clippy -D warnings on touched crates; vox ci ssot-drift; vox ci crate-edges; schema digest/baseline test for BASELINE_VERSION 96"
    expected: "All gates pass. The gap-closure plans ran only their own new tests locally (lean policy); the vox-orchestrator goal-search tests were NOT run locally and are affected by the WR-08 behaviour change (goal.rs path now drops Untrusted lines before CRAG relevance and Socrates evidence counts). This verifier ran no build or test."
    why_human: "Lean policy forbids workspace builds locally; crate-edges and the digest/baseline test can only be confirmed by hosted CI"
  - test: "Playwright spec crates/vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts plus vitest for MemoryView/DocPublishModal/ResearchView and pnpm typecheck"
    expected: "Pass; screenshots land in crates/vox-gui/ui/review-bundle/latest/"
    why_human: "GUI Visual Verification Invariant; not run by the verifier"
  - test: "Decide the upgrade-path residuals WR-01 and WR-02 from the gap-closure re-review (03-REVIEW.md): (a) the legacy fact/concept re-type (open.rs retier_agent_knowledge_nodes) and (b) the Library trust backfill (open.rs backfill_search_document_trust_tiers) both run only inside `current_version < BASELINE_VERSION`"
    expected: "Accept as pre-release (BASELINE 96 first appears in commit 80cf75eda, present on no remote branch), or schedule the two small fixes: run the re-type on every open outside the version gate, and make the 96 backfill stamp Library-corpus rows untrusted so only a git-clean reindex promotes them. Until then a local DB already at 96 keeps Advisory legacy agent facts, and a v95 DB with an indexed Library keeps Trusted on untracked/modified docs until the next vox_kb_reindex"
    why_human: "Judgement call on whether unreleased dev databases warrant a migration-path fix; does not defeat a ROADMAP success criterion or a D-07/D-08 forward-write requirement"
  - test: "Housekeeping: .planning/workstreams/kb/REQUIREMENTS.md still shows RET-03, TRUST-02, TRUST-04, TRUST-06, GROW-06 unchecked and 'Gaps Found' in the traceability table (set after the first verification)"
    expected: "Orchestrator flips them to [x] / Complete: this re-verification finds no remaining gap against any of the nine IDs"
    why_human: "Orchestrator owns the requirements state commit"
---

# Phase 3: Provenance, Trust Tiers and Leak Closure Verification Report

**Phase Goal:** Every piece of KB content carries a trust tier and provenance. The existing unreviewed paths into the corpus are closed. All KB retrieval runs through one `vox-search` path, so later phases build on one sink and one retrieval path.
**Verified:** 2026-10-09
**Status:** human_needed (all must-haves verified in code; hosted-CI, GUI suite and two migration-path decisions remain)
**Re-verification:** Yes, after gap-closure plans 03-14, 03-15, 03-16 (gap-closure diff `88fc5e996..HEAD`)

Method: code read and grep of the actual tree and the gap-closure diff. SUMMARY claims were used only as pointers. Per the lean policy no build or test was run by the verifier; VERIFIED means the code, the test, and a recorded mutation proof exist and are consistent. Hosted CI remains the authority for green runs.

## Re-check of the two previous gaps

### Gap 1 (was BLOCKER, CR-01): Tavily bypass of the Untrusted rule. CLOSED

Evidence in `crates/vox-search/src/bundle.rs`:
- Lines 35-45: `run_search_with_verification` shadows `policy` with `exclude_untrusted_hits: true` whenever `trigger == AutoChatPreamble` (clone of the caller policy). Both `execute_search_plan` passes, the diagnostics and the Tavily block read the shadowed binding, so the flag no longer depends on each caller (WR-08 closed; `goal.rs`, `web_gather.rs` and any future caller are covered).
- Lines 162-167: `if policy.tavily_enabled && policy.exclude_untrusted_hits` pushes `tavily_skipped=untrusted_excluded` and does nothing else: no client built, no credit consumed, no HTTP request, no capture. The Tavily firing is gated (the preferred fix from the previous report), not just the extend.
- Lines 214-222, 252-259: when Tavily does fire (explicit query), lines are `[tier:untrusted] [crag_tavily:<url>] ...`, and `execution.min_trust_tier = Some(Untrusted)` is set before the lines are extended into `web_lines`/`rrf_fused_lines`.
- `grep "\[crag_tavily:"` in the file shows exactly one format string, and it is the labelled one.

Tests (read, not run):
- `crates/vox-search/tests/web_capture_test.rs::crag_tavily_never_fires_for_auto_preamble_without_the_flag` (wiremock): policy has `exclude_untrusted_hits: false`, `prefer_rrf_merge: true`, trigger AutoChatPreamble; asserts zero received requests, no `crag_tavily` in `web_lines` or `rrf_fused_lines`, no CragTavily capture, `budget.remaining() == 100`, and the diagnostic note.
- `capture_crag_tavily_reaches_sink_not_corpus` extended: labelled line and `min_trust_tier == Some(Untrusted)` for the explicit path, corpus rows still (0, 0).
- `crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs` drives the real `run_retrieval_bundle`: preamble gets no request and no Tavily text in memory/knowledge/chunk/repo/RRF lines; an explicit query reaches the mock (positive control), line starts `[tier:untrusted] [crag_tavily:`, envelope minimum Untrusted.
- 03-14-SUMMARY records mutation proofs 1a/1b/1c (gate to `if false`, derivation to `if false`, drop of the `min_trust_tier` write), each turning named tests red, with `grep -c` before/mutated/restored counts. The summary admits the pre-fix RED run was never observed; the mutations are the red evidence, which is acceptable for a guard.

I traced the other injection path from the previous report (`message.rs` injecting `bundle.rrf_fused_lines` for AutoChatPreamble): with Tavily skipped and `execute_search_plan` already honouring the flag at its legs, nothing Untrusted reaches those lines. The 03-REVIEW re-review found no remaining bypass either (it checked every `TavilyClient` user and every `run_search_with_verification` caller).

### Gap 2 (was partial SC4, WR-05): research-turn source URL/engine unsanitized. CLOSED for the prompt path

`crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs`:
- `sanitized_source(n, url, title, engine, snippet)` (line 195) maps all four string fields through `SanitizedText::new(..).into_string()`, sanitizing before the 600-char snippet cap.
- `sources_from_hits` (line 188) and `pipeline_sources` (line 703, hence `failed_run_sources`) both build `Source` only through it.
- The two remaining raw `Source { .. }` literals (lines 1755, 1880) are inside `#[cfg(test)]` fixtures. The review's IN-02 notes the GUI `save_research_session_to_kb` mapping builds a `Source` directly, but that value reaches only `capture_research_sources`, which sanitizes through the findings `clean()`; no live prompt leak.
- Tests `redteam_source_url_and_engine_are_sanitized_at_the_prompt_block` and `redteam_pipeline_sources_are_sanitized` (13 red-team cases each, from `contracts/kb/redteam/cases.v1.jsonl`) exist; mutations 2a (constructor `url:` back to raw) and 2b (pipeline back to a raw literal) recorded red.

SC4 therefore holds at both boundaries: ingest (`clean()`/`CaptureInput`), retrieval-bundle injection (`retrieval.rs sanitized()`), and the research-turn source block.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: tier required on index/finding writes; four-way `ContextTrustTier` mapping; fail-closed, mutation-proven promotion verdict | VERIFIED (unchanged from the first verification; regression sanity re-checked) | `crates/vox-orchestrator-types/src/trust.rs` single enum, `tier_for_knowledge_node_type` allowlist; `upsert_search_document(.., tier)` required parameter; `crates/vox-db/tests/tiered_writer_invariant.rs` present; `promotion_eligibility` exhaustive match in `vox-search/src/trust.rs`. 03-15 added `AGENT_FACT_NODE_TYPE`/`AGENT_CONCEPT_NODE_TYPE`, which fall through the default arm to Untrusted (test `agent_written_node_types_are_untrusted`; old `fact`/`concept` Advisory allowlist test kept) |
| 2 | SC2: web research turn adds zero corpus rows; Untrusted pending findings with summary/quotes/URL/fetch date/hash; snapshot gitignored | VERIFIED | `web_capture_test.rs` asserts `(0, 0)` corpus rows on the Tavily-to-sink path and the sink receives the hit; Tavily no longer fires at all for the auto preamble, so the preamble files no Tavily finding (documented consequence in 03-14; explicit-query D-04 capture unchanged) |
| 3 | SC3: no accepted=1 / 0.95 writes; side doors produce pending findings only; MENS `KbSignals` reads `kb_findings` | VERIFIED | Unchanged by the gap closures (no touched file is on these paths); first-verification evidence stands |
| 4 | SC4: sanitizer property tests and red-team neutralise tag block, zero-width/bidi, HTML comments, off-allowlist images at ingest AND injection | VERIFIED | Gap 2 closed (above) |
| 5 | SC5: `KbStore`/`KbRouter`/adapters/`kb_lines`/12 `vox_kb_*` tools absent; names replaced; seeded-DB migration counts then migrates as pending | VERIFIED | `legacy_voxkb_retired.rs` still present and untouched by the gap closures; first-verification evidence stands |
| 6 | 03-09 truth (D-03/D-05, TRUST-01): unprompted preamble never carries an Untrusted hit; envelope tier = min of contributing hits | VERIFIED | Gap 1 closed (above) |
| 7 | 03-15 truth (D-07, D-07a): `persist_fact` graph rows are Untrusted agent nodes, never Advisory; memory search leg deliberately unchanged | VERIFIED for forward writes | `crates/vox-orchestrator/src/memory/manager.rs:214,230` writes `agent_fact` / `agent_concept`; `grep upsert_knowledge_node(` over `crates/` finds only: `persist_fact` (agent types), `graph_tools.rs:350` (`graphify_search_hit`, default arm Untrusted), `graphify/mod.rs:554` (repo AST nodes from the graphify ingest, not model-callable), `pipeline_cache.rs` (`research_cache`, Untrusted), and tests; the raw `INSERT INTO knowledge_nodes` in `research.rs`/`research_pipeline.rs` write other types. So no model-reachable writer produces `fact`/`concept`. The tier is computed from `node_type` at read time, so it cannot go stale. Test `kb_tiers_agent_fact_from_memory_store_never_reaches_the_preamble` drives the real `memory_store` tool, asserts both graph rows Untrusted, a labelled `[tier:untrusted]` line, and pins the D-07a memory-leg behaviour. Upgrade-path residual: WR-01 (human decision below) |
| 8 | 03-16 truth (D-08): a Library doc is Trusted only when git tracks it at HEAD with no staged/working-tree modification; else Untrusted; fail-closed on git failure | VERIFIED on the reindex path | `kb_tools/library.rs committed_clean_docs`: `ls-tree -r -z --name-only --full-name HEAD -- <root>` minus `diff -z --name-only --no-renames HEAD -- <root>`, both via `GitExec`; any `Err` returns the empty set (every doc Untrusted). `reindex_at` is the single non-test production constructor of `IndexDeps` (the other `IndexDeps {` sites are the eval harness with a documented always-true closure over a throwaway in-memory DB, and tests). `index_library` (`vox-search/src/library/mod.rs`) decides the tier per walked file on every run: changed rows get it via `apply_library_document`, unchanged rows via the new `set_library_document_tier`, unreadable rows are forced Untrusted; `put_kb_index_state` no longer re-stamps `search_documents`. Tests: `kb_reindex_git_gate_trusts_only_committed_unmodified_docs` (clean Trusted; modified, untracked, staged Untrusted; `docs_untrusted == 3`), `..._non_checkout_is_untrusted` (ceiling stays Trusted, rows Untrusted), `..._retiers_unchanged_docs` (commit an edit: Trusted with `docs_changed == 0`; `git rm --cached`: demoted), `set_library_document_tier_is_corpus_and_path_scoped`. Fixture refuses to run if `GIT_DIR`/`GIT_WORK_TREE` is set. 03-16-SUMMARY records mutation proofs. Upgrade-path residual: WR-02 (human decision below) |

**Score:** 6/6 ROADMAP/plan truths verified (SC1-SC5 plus the binding 03-09 truth), plus the two gap-closure truths 7 and 8; 0 present-but-behavior-unverified.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/vox-search/src/bundle.rs` | Trigger-derived exclusion; Tavily gate; labelled lines; min-tier lowering | VERIFIED | Lines 35-45, 162-167, 217, 254 |
| `crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs` | MCP-level Tavily preamble/explicit test | VERIFIED | Present, with positive control |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` | `sanitized_source` single constructor | VERIFIED | Lines 175-205, 703 |
| `crates/vox-orchestrator-types/src/trust.rs` | Agent node-type constants | VERIFIED | Substantive, used by `manager.rs` and `open.rs` |
| `crates/vox-orchestrator/src/memory/manager.rs` | `persist_fact` writes agent types | VERIFIED | Lines 207-240 |
| `crates/vox-db/src/store/open.rs` | One-time legacy re-type inside the 96 gate | VERIFIED (scope-limited, see WR-01) | Line 210, 318-337 |
| `crates/vox-orchestrator-mcp/src/kb_tools/library.rs` | `committed_clean_docs` git gate | VERIFIED | Lines ~213-266 |
| `crates/vox-search/src/library/mod.rs` | `is_committed_clean`, per-doc tier every run | VERIFIED | `IndexDeps` field, loop at ~97-140 |
| `crates/vox-db/src/store/ops_memory/library.rs` | `set_library_document_tier`; manifest write no longer re-stamps | VERIFIED | Lines 285-300, 374-420 |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| `run_search_with_verification` trigger | `exclude_untrusted_hits` for all passes + Tavily | WIRED | Shadowed policy binding at the top of the function |
| Tavily lines | `min_trust_tier` / tier label | WIRED | Lines 217, 254 |
| `sources_from_hits` / `pipeline_sources` | `SanitizedText` | WIRED | via `sanitized_source` |
| `MemoryManager::persist_fact` | Untrusted tier | WIRED | node_type constants plus default arm; end-to-end test through the real tool |
| `reindex_at` | git-clean set -> `index_library` per-doc tier -> `search_documents.trust_tier` | WIRED | Retrieval reads the per-row tier, so a demoted doc is excluded by `exclude_untrusted_hits` |

### Requirements Coverage

All nine phase IDs are claimed by at least one PLAN `requirements:` field and SUMMARY `requirements-completed` (03-14: TRUST-01, TRUST-05, RET-02; 03-15/03-16 continue TRUST-01/TRUST-03 through D-07/D-08). No orphaned requirement.

| Requirement | Status | Evidence |
|-------------|--------|----------|
| TRUST-01 | SATISFIED | Required tier + mapping (SC1); envelope min-tier now holds on the Tavily path (gap 1); "Trusted = merged to git" now mechanically enforced at reindex (truth 8, D-08) |
| TRUST-02 | SATISFIED | Web hits to inbox, zero corpus rows (SC2) |
| TRUST-03 | SATISFIED | 0.95 floor and chat write-back removed (SC3); model-callable `persist_fact` graph rows Untrusted (truth 7) |
| TRUST-04 | SATISFIED | Provenance columns plus gitignored snapshot (advisory WR-03 hardening, deferred) |
| TRUST-05 | SATISFIED | Sanitizer at ingest, retrieval injection and research-source boundary (gap 2 closed) |
| TRUST-06 | SATISFIED | Fail-closed, mutation-proven verdict (advisory: Crossref field choice WR-01-old, owner Phase 9 before the first promotion consumer) |
| RET-02 | SATISFIED | Single `vox-search` path, legacy surface deleted; the Tavily tier bypass on that path is closed |
| RET-03 | SATISFIED | Counted then migrated as pending (advisory orphan-row edge case, deferred) |
| GROW-06 | SATISFIED | Side doors route to the inbox; MENS reads `kb_findings` |

### Disposition of the Gap-Closure Review (03-REVIEW.md, 0 critical)

| Finding | Disposition | Reason |
|---------|-------------|--------|
| WR-01 legacy `fact`/`concept` re-type only below baseline 96 | WARNING, human decision (listed above) | Defeats neither D-07 (which the plan scopes as a one-time re-type in the 96 gate) nor a ROADMAP SC: every forward write is Untrusted and the tier is derived at read time. Residual is limited to a local DB that already recorded version 96 with rows from the old `persist_fact`. BASELINE 96 first appears in commit `80cf75eda`, which is on no remote branch, so this is a dev-database window. The review's fix is two lines (run outside the gate, drop the `graphify:` exclusion). Note the `id NOT LIKE 'graphify:%'` guard also spares a model-chosen key starting with `graphify:` |
| WR-02 baseline-96 backfill stamps Library rows with the ceiling | WARNING, human decision (listed above) | Code read confirms `backfill_search_document_trust_tiers` still copies `kb_index_state.trust_tier` onto every row of the corpus and its comment is stale. This does not defeat D-08: the gate decides on every reindex, and a pre-Phase-3 DB had no tier concept at all, so the window is "not yet re-gated", not a new leak. It is, however, a genuine fail-open on the upgrade path for an indexed Library until `vox_kb_reindex` runs; the cheap fix is stamping Library rows untrusted in the backfill |
| IN-01 git gate fails closed silently; blind to `assume-unchanged`/`skip-worktree` | Info | Fail-closed result is correct; observability and a deliberate-bypass edge only |
| IN-02 `Source` literal builders are convention-guarded | Info | Only test fixtures and the findings-bound GUI path remain |
| IN-03 memory leg still feeds model text to the preamble as Advisory | Accepted by D-07a (user decision 2026-10-09), recorded in `deferred-items.md`, revisit with Phase 7 | The invariant "nothing elevates model or web output" is not yet true end to end for MEMORY.md; this is a user-decided scope boundary, not a plan defect |
| IN-04 `knowledge_nodes` tier invariant is convention | Info | Today no non-agent writer produces `fact`/`concept`; a static writer-invariant test would harden it |
| IN-05 Tavily tests mutate process env | Info | Same pattern already existed; `#[serial]` plus a positive control in the `web_capture_test.rs` case would harden it (the MCP test already has one) |
| Older review items (previous WR-01..04, 06, 07; IN) | Deferred, recorded in `deferred-items.md`; none blocks a ROADMAP SC | Previous WR-02 (redact-then-sanitize order, a secret split by a zero-width character survives redaction) remains the item most worth a quick fix before Phase 3 ships |

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/vox-db/src/store/open.rs` | 278-281 | Stale doc comment: "the single corpus-level source" no longer true after 03-16 | WARNING | WR-02 |
| `crates/vox-db/src/store/open.rs` | 210, 318-337 | One-time migration gated by version; not idempotent-on-open | WARNING | WR-01 |
| Debt markers (TBD/FIXME/XXX) in files touched by the gap closures | n/a | grep not exhaustive across the repo; the review found none in the diff | none known | n/a |

No stub or hollow-data finding in the gap-closure diff: the new functions have callers and tests, and the data flows from the real `git` output / real search results to the tier column and the prompt block.

## Behavioral Spot-Checks

Step 7b: SKIPPED. The instruction was not to run builds or tests, and `target/` on this worktree has been unstable (concurrent `cargo clean`, stuck rustc per the 03-14 summary). The behavioural evidence is the named wiremock/git-fixture tests above plus the recorded mutation proofs, which the executor ran with `CARGO_INCREMENTAL=0` and a scratch `CARGO_TARGET_DIR`. Hosted CI must confirm.

## Human Verification Required

See the `human_verification` frontmatter: (1) hosted-CI gates, including the vox-orchestrator goal-search tests affected by the WR-08 behaviour change and crate-edges; (2) the Playwright/vitest GUI suite; (3) the WR-01/WR-02 upgrade-path decision; (4) requirements-state housekeeping.

## Gaps Summary

Both gaps from the first verification are closed in code, with tests that fail under mutation. The phase goal holds: every KB row and finding carries a stored or derivable tier (now including a git-gated Library tier and Untrusted agent-written graph rows), the unreviewed side doors are closed, and retrieval runs through the single `vox-search` path with the preamble's Untrusted exclusion applied at the one place every caller passes through. No must-have fails.

What remains is not a truth failure: confirmation by hosted CI (nothing was run locally by design), and a judgement on two small upgrade-path holes (WR-01, WR-02) that exist only for databases created before these fixes landed.

---

_Verified: 2026-10-09_
_Verifier: Claude (gsd-verifier)_
