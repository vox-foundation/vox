# Roadmap: Vox

## Overview

Workstream `kb` turns Vox's 669 curated, non-archive markdown docs (~2.4M tokens) into first-class, model-agnostic agent context. Git markdown stays the only curated source of truth. The order is: measure first (a deterministic, ID-based eval with a ratchet); stabilise chunk identity and one MCP tool contract; close the shipped leaks (web hits persisted into the corpus, a 0.95 confidence writeback, auto-accepted chat turns, undelimited injection) and retire the legacy VoxKB surface; add a controlled taxonomy and facet lint. After that come local embeddings, budgeted per-turn injection, a web fallback that feeds a reviewed inbox with a human-merged PR as the only way in, page-one rewrites, and scheduled re-verification. GUI work comes in three sub-phases (Library, Inspector, Inbox/Settings), each starting once the backend contract it displays exists. Every phase reports its deltas against the retrieval ratchet from Phase 1.

## v1.1 — Vox Knowledge Base — corpus to context

**Phase Numbering:**

- Integer phases (1, 2, 3): planned milestone work (workstream-local numbering, reset to 1 for this milestone)
- Decimal phases (2.1, 2.2): urgent insertions (marked with INSERTED)

## Phases

- [x] **Phase 1: Retrieval Eval Baseline** - Deterministic ID-based retrieval eval, BEIR golden set, canary exclusion, ratcheted baseline in PR CI (DOC-01 already complete) (completed 2026-10-05)
- [x] **Phase 2: Library Index and KB Tool Surface** - Content-hash incremental indexer with heading-path chunks, `docs/src/library/`, one `vox_kb_*` MCP contract and `kb://` resources (completed 2026-10-05)
- [x] **Phase 3: Provenance, Trust Tiers and Leak Closure** - Required trust tier on every row, `kb_findings` inbox, sanitizer, the R1/R2 leaks closed, legacy VoxKB retired into `vox-search` (completed 2026-10-10)
- [ ] **Phase 4: Taxonomy Contract and Facet Lint** - SKOS-lite vocabulary, shared schemars facet schema, page-one summary, authorship and training-eligibility rules under a ratcheting lint
- [ ] **Phase 5: Local Embeddings and Ranking Signals** - Candle `gte-modernbert-base` embedder and reranker, contextual prefix, model-keyed vectors, trust/status-aware ranking
- [ ] **Phase 6: Library GUI** - Browse, search and read the KB with facets and a provenance header (absorbs Memory and DocReader)
- [ ] **Phase 7: Context Injection and Per-Chat Scope** - Relevance-gated, per-model-ceiling preamble with spotlighting, citations, a `kb_context` event, egress checks, per-chat scope and nightly generation evals
- [ ] **Phase 8: Inspector and Chat Controls GUI** - Per-turn Inspector over the real `kb_context` payload, composer KB controls, inline citation links
- [ ] **Phase 9: Web Fallback, Inbox and Promotion PR** - Free-first CRAG fallback under a spend cap, quarantined capture into the inbox, review tools, draft-PR promotion with corroboration
- [ ] **Phase 10: Page-One Summary Rewrites** - Guarded LLM rewrites of the delimited summary block only, in small batches, with source-bias and distinctiveness ratchets
- [ ] **Phase 11: Staleness, Re-verification and KB Operator Docs** - Jittered `review_by`, budgeted `@scheduled` re-verify on mechanical evidence, demote-not-delete, `where-things-live` and an operator how-to
- [ ] **Phase 12: Inbox, Settings and Health GUI** - Inbox review and Open-PR, KB Settings, index health and verification queue, plus a full Playwright audit of every KB surface

## Phase Details

### Phase 1: Retrieval Eval Baseline

**Goal**: A maintainer can measure retrieval quality deterministically, and any retrieval regression blocks a PR. Every later phase reports against this baseline.
**Depends on**: Nothing (first phase)
**Requirements**: DOC-01 (complete), EVAL-01, EVAL-02, EVAL-03, EVAL-04, EVAL-05
**Success Criteria** (what must be TRUE):

  1. Running `vox kb eval` twice on the same commit prints byte-identical Recall@5/10, MRR@10, nDCG@10 (on both doc and chunk IDs) and the answer-bearing-chunk-within-budget rate. A hand-computed 3-query fixture reproduces the expected metric values exactly.
  2. `contracts/kb/eval/` holds a BEIR-layout golden set (`corpus.jsonl`, `queries.jsonl`, `qrels/`). It has ≥40 seed queries (origin `human`, or `agent-seed` when authoring is delegated — amended 2026-10-04 per 01-CONTEXT D-02), a pruned LLM-generated expansion tagged as such, and a held-out split that no tuning command reads.
  3. A canary-string check fails CI when the golden set appears under any indexer root or in MENS corpus build output. This is proven by mutation: plant the canary in an indexed path and the check goes red.
  4. PR CI compares metrics against `contracts/kb/retrieval-baseline.v1.json`. A fixture PR that degrades ranking fails the gate, and a baseline edit that loosens any value is rejected (tighten-only).
  5. No word-overlap helper in `crates/vox-search/src/evaluation.rs` carries an IR metric name (deleted or renamed), so no gate can call them.

**Plans:** 12/12 plans complete
Plans:

- [x] 01-01-PLAN.md — Wave 1. Eval engine in vox-search: tracer from fixture to report; kb_ids heading-path SSOT; IR metrics; BEIR loader; BM25 baseline
- [x] 01-02-PLAN.md — Wave 1. R4 word-overlap helpers moved/renamed into vox-cli-research; `citation_recall` → `answer_token_overlap`
- [x] 01-03-PLAN.md — Wave 1. Turso FTS characterization test (S1) + spike outcomes (S1, S2) in the DOC-01 SSOT
- [x] 01-04-PLAN.md — Wave 1. Snapshot decision (one-way), README + canary GUID + coverage scaffold, user writes ≥40 seed queries
- [x] 01-05-PLAN.md — Wave 2. Baseline ratchet (regression, tighten-only) and pure canary checks, mutation-proven
- [x] 01-06-PLAN.md — Wave 2. `vox kb eval` command (report/query/headings/snapshot) + frozen corpus snapshot
- [x] 01-08-PLAN.md — Wave 3. Pooled qrel proposals + paraphrase expansion, first half of the queries
- [x] 01-09-PLAN.md — Wave 3. Pooled qrel proposals + paraphrase/coverage-gap expansion, second half
- [x] 01-12-PLAN.md — Wave 3. `kb.eval` CLI SSOT registration, cli.md section, where-things-live row
- [x] 01-07-PLAN.md — Wave 4. `--gate` (fail-closed tighten-only + full canary sweep incl. MENS extractor), `--tighten`, `--write-baseline`
- [x] 01-10-PLAN.md — Wave 4. User approves qrels/prunes expansion; BEIR golden set materialized + contract test
- [x] 01-11-PLAN.md — Wave 5. Bootstrap baseline, real-data ratchet tests, two-process determinism, fast-tier gate + mutation proofs

**Research flag**: none (BEIR layout and IR metrics are textbook). Run two spikes inside this phase and record the outcomes: the VoxDb/Turso FTS5 runtime probe (feeds IDX-07 in Phase 2) and the candle `modernbert` + tokenizers 0.21 load check (feeds Phase 5).

### Phase 2: Library Index and KB Tool Surface

**Goal**: The curated corpus (Vox docs plus general-knowledge `docs/src/library/`) is indexed incrementally with stable chunk identity. Any MCP host reaches it through one KB tool contract.
**Depends on**: Phase 1 (eval harness and probe outcomes)
**Requirements**: IDX-01, IDX-02, IDX-03, IDX-04, IDX-05, IDX-06, IDX-07, RET-01
**Success Criteria** (what must be TRUE):

  1. `vox kb reindex` on an unchanged tree rewrites zero chunks. Editing one heading section rewrites only that chunk. Deleting or renaming a doc prunes its rows. After any reindex no `embeddings.source_id` is orphaned (integration test).
  2. Fixture tests prove that a `#` inside a code fence and YAML frontmatter never create or pollute a chunk, that every chunk records its `heading_path`, that oversize sections split within the embedder tokenizer budget, and that a doc under `docs/src/archive/` is never indexed.
  3. `vox kb status` reports last-indexed commit, doc/chunk counts, embed model, the build manifest (model id + revision, dim, chunker version, corpus commit) and per-file errors. Every indexed doc has a summary-card row.
  4. From an external MCP host (Claude Code), a user can call `vox_kb_search`, `vox_kb_get`, `vox_kb_cite`, `vox_kb_status` and `vox_kb_reindex`, and read `kb://doc/{path}` and `kb://chunk/{hash}`. A page added under `docs/src/library/` appears in the `Library` sidebar section and in search results.
  5. The lexical leg is the one the recorded runtime probe chose. The writer-less `TantivyDocsIndex` mirror is gone from source. Doc-level `vox kb eval` metrics do not regress against the Phase 1 baseline (chunk-level qrels are re-keyed exactly once, with the delta recorded).

**Plans**: 12/12 plans executed
Plans:
**Wave 1**

- [x] 02-01-PLAN.md — Gate (frozen-core schema 95 + tantivy-lexical placement), schema/store, tracer: one doc → kb_ids sections → hash-keyed rows → Tantivy → library::search
- [x] 02-02-PLAN.md — `docs/src/library/` + `Library` category, sidebar section and parity test
- [x] 02-10-PLAN.md — Gap closure CR-01: fallible corpus walk, prune skipped on incomplete/empty walk, missing `docs/src` fails, mutation-proven tests
- [x] 02-12-PLAN.md — Gap closure: Phase-1 real-data ratchet retargeted at LibraryRetriever (epoch 2, config-aware, mutation-proven) + clippy `cloned_ref_to_slice_refs`; IDX-02..07 marked Complete

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 02-03-PLAN.md — Complete heading chunker (fences, frontmatter, H3–H6, anchors, card, text-splitter oversize) + archive-proof walker
- [x] 02-04-PLAN.md — Incremental reindex: file-hash skip, prune, orphan-vector cleanup, kb_index_state manifest, status, readers (re-runs Phase 1's FTS5 probe)
- [x] 02-05-PLAN.md — Remove the writer-less TantivyDocsIndex mirror across crates
- [x] 02-11-PLAN.md — Gap closure WR-08 (`vox_kb_status` off the HTTP read role) + WR-02 explicit `vox_kb_reindex` dispatch timeout

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 02-06-PLAN.md — Five `vox_kb_*` MCP tools via the operations catalog (validation, single-flight)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 02-07-PLAN.md — `kb://doc` / `kb://chunk` resources through the daemon + routing tests
- [x] 02-08-PLAN.md — `vox kb status|reindex` CLI via daemon + where-things-live / search SSOT / CHANGELOG

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 02-09-PLAN.md — Library retriever in `vox kb eval`, exact qrel resolution + one-time deepening via epoch, Claude Code UAT

**Research flag**: YES (`/gsd-plan-phase 2 --research-phase`). The lexical-leg decision is unresolved: Turso FTS5 vs `USING fts`, `turso` crate feature gating, and the Tantivy-with-owning-writer fallback (the researchers disagree).

### Phase 3: Provenance, Trust Tiers and Leak Closure

**Goal**: Every piece of KB content carries a trust tier and provenance. The existing unreviewed paths into the corpus are closed. All KB retrieval runs through one `vox-search` path, so later phases build on one sink and one retrieval path.
**Depends on**: Phase 2 (index rows and the single retrieval path to attach tiers to)
**Requirements**: TRUST-01, TRUST-02, TRUST-03, TRUST-04, TRUST-05, TRUST-06, RET-02, RET-03, GROW-06
**Success Criteria** (what must be TRUE):

  1. An index or finding write without a trust tier fails to compile or is rejected. Every row maps onto `ContextTrustTier` per the documented four-way mapping (SystemVerified / Trusted / Advisory / Untrusted). Promotion eligibility returns "ineligible" when the trust lookup errors, and this is mutation-proven: invert the guard and the test fails.
  2. A research turn that hits the web adds zero rows to the searchable corpus. It creates Untrusted pending findings in `kb_findings` carrying summary, short quotes, URL, fetch date and content hash. The full-text snapshot lands under a gitignored path and `git status` stays clean.
  3. Neither a chat turn nor a research synthesis writes an `accepted=1` or 0.95-confidence row anymore. `publish_research_doc`, GUI `ingest_url`/`ingest_text` and the scientia/MEMORY sinks produce only pending inbox findings. MENS `KbSignals` reads `kb_findings`.
  4. Property tests and a red-team fixture set show the sanitizer neutralises Unicode tag-block, zero-width/bidi characters, HTML comments and off-allowlist markdown images, at both the ingest and the injection boundary.
  5. `KbStore` LIKE search, `KbRouter`, the signal adapters, `kb_lines` and the 12 legacy `vox_kb_*` CRUD tools are absent from source and the tool catalog, and colliding names are replaced, not aliased. A seeded-DB migration test shows legacy `kb_entries`/`knowledge_bases` rows are counted before quarantine and non-empty rows appear in the inbox as pending.

**Plans:** 16/16 plans complete

Plans:
**Wave 1**

- [x] 03-01-PLAN.md — Trust tier vocabulary in the L0 crate (taint rule); fail-closed promotion verdict
- [x] 03-02-PLAN.md — One KB-text sanitizer with property tests, an ASCII-escaped red-team set and a canary roots check

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-03-PLAN.md — Baseline 96: `kb_findings` inbox, `search_documents.trust_tier` with a kb_index_state-derived backfill, `LEGACY_VOXKB_DDL` fragment (D-01 pre-approved)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 03-04-PLAN.md — Capture chokepoint; web-leg and CRAG sinks rerouted to the inbox; mirror helper deleted

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 03-05-PLAN.md — Tier-required corpus writers; Library tier single source with agreement test; wire validator removed
- [x] 03-06-PLAN.md — Research paths (quick, deep, gather, GUI save-session) into the inbox; shim and `vox research publish` docs/src writers closed
- [x] 03-07-PLAN.md — Chat-path leak closure: writeback, signal adapters, KB-lines enrichment, @kb consumer, MEMORY sink
- [x] 03-08-PLAN.md — Legacy VoxKB count-and-migrate at DB attach; MENS KbSignals on reviewed findings (DPO only)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 03-09-PLAN.md — Tier-aware retrieval, preamble drops Untrusted, envelope min tier, injection-boundary sanitization
- [x] 03-10-PLAN.md — Legacy `vox_kb_*` MCP tool retirement through the catalog (D-02 pre-approved)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 03-11-PLAN.md — GUI side doors (Ingest URL/Text, Publish) file findings via mcp helpers; Playwright stepper
- [x] 03-12-PLAN.md — KbStore/KbRouter/legacy CRUD deletion and VoxKB DDL quarantine
- [x] 03-13-PLAN.md — `vox kb status` legacy/inbox counts, retirement registries, SSOT docs, CHANGELOG

**Gap closure** (from 03-VERIFICATION.md gaps and decisions D-07/D-08)

**Wave 1**

- [x] 03-14-PLAN.md — CR-01 + WR-08: Tavily never fires for AutoChatPreamble, exclusion derived from the trigger, explicit Tavily lines labelled Untrusted; WR-05: every research-source field sanitized
- [x] 03-15-PLAN.md — D-07/D-07a: agent-written knowledge-graph rows (`agent_fact`/`agent_concept`) are Untrusted, and legacy rows are re-typed in the baseline-96 gate; the memory leg is unchanged until Phase 7

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-16-PLAN.md — D-08: Library docs Trusted only when committed and unmodified (GitExec), per-document tier on every reindex; SSOT and CHANGELOG for all four closures

**Research flag**: none (in-repo quarantine and catalog-ops patterns).

### Phase 4: Taxonomy Contract and Facet Lint

**Goal**: Every doc's facets come from one controlled vocabulary and one schema, enforced by a ratcheting lint. Ranking, staleness, rewrites and training eligibility then read fields they can trust.
**Depends on**: Phase 1. Can run in parallel with Phases 2–3. The altLabel-synonym and indexer-facet integration lands once Phase 2's indexer exists.
**Requirements**: TAX-01, TAX-02, TAX-03, TAX-04, TAX-05, TAX-06, TAX-07
**Success Criteria** (what must be TRUE):

  1. A new or changed doc fails the `vox-doc-pipeline` lint if it has an off-vocabulary `topic`/`domain`, a missing or out-of-range (40–120 words) page-one summary block, missing `authorship`, a `sources[]` entry without url/date/hash, or an over-cap verbatim quote. Untouched existing docs pass through a backlog allowlist that can only shrink.
  2. The lint, the indexer and the GUI types all derive from one schemars facet schema (`kb-taxonomy.v1.yaml` plus the generated facet JSON Schema). A drift test fails when any consumer diverges.
  3. A lexical search for a taxonomy altLabel returns the doc tagged with the corresponding prefLabel, and deprecated aliases resolve to their replacement.
  4. A page with `origin: web-promoted` or LLM `authorship` and no `human_attested_by` is forced to `training_eligible: false`. A fixture shows the MENS corpus builder excludes it and emits no page-one summary-block text from any doc.

**Plans**: TBD
**Research flag**: none (extends `vox-doc-pipeline` lint and the existing ratchet pattern).

### Phase 5: Local Embeddings and Ranking Signals

**Goal**: Retrieval measurably improves with embeddings computed locally (no API key) and with trust/status-aware ranking, without ever mixing vector spaces.
**Depends on**: Phase 2 (stable chunk hashes), Phase 3 (trust tiers on rows), Phase 4 (page-one summary for the contextual prefix; scope/status facets)
**Requirements**: EMB-01, EMB-02, EMB-03, EMB-04, EMB-05, EMB-06, RET-04
**Success Criteria** (what must be TRUE):

  1. On a machine with no API keys, `vox kb reindex` embeds chunks locally through `llm_embed` with `provider: "local"` (Candle `gte-modernbert-base`, revision-pinned). `vox ci crate-edges` and `vox-arch-check` pass with candle only behind the feature/plugin boundary, and the build-toolchain invariant still holds.
  2. Vectors are keyed by (chunk_hash, model_id, revision, dim). A similarity query against vectors from a different model or dim is refused with an error (test). Switching the embed model triggers re-embedding, never mixing.
  3. With model weights absent, search returns lexical-only results and `vox doctor` prints how to fetch the weights.
  4. `vox kb eval` shows hybrid retrieval with the deterministic contextual prefix beating Phase 2's lexical-only numbers, and the baseline is tightened. The reranker is enabled only on paths where both the recorded eval delta is positive and the recorded CPU latency benchmark fits. Any further technique (LLM chunk context, Qwen3 tier) lands only with a recorded eval delta.
  5. Every search result carries trust tier, staleness, `scope` and status. A fixture shows that `research|roadmap|legacy|deprecated` and superseded pages rank below an equivalent current page and are still returned.

**Plans**: TBD
**Research flag**: YES (`/gsd-plan-phase 5 --research-phase`). Open: candle `modernbert` with tokenizers 0.21; CPU rerank latency for 20 pairs; feature vs L4-plugin placement under `layers.toml`; Qwen3 batch-1 ingest viability.

### Phase 6: Library GUI

**Goal**: A user can browse, search and read the whole KB in the desktop app, with provenance visible on every doc.
**Depends on**: Phase 2 (KB tool contract), Phase 3 (trust tiers), Phase 4 (facet schema). Can run in parallel with Phase 5.
**Requirements**: GUI-KB-01, GUI-KB-02
**Success Criteria** (what must be TRUE):

  1. In the Library surface, a user can search the KB and narrow results by category, status, trust tier, staleness and domain facets. Memory and DocReader browsing are reachable through the Library, and the old standalone surfaces are absorbed.
  2. Opening a doc renders it with `react-markdown` + `remark-gfm` under a provenance header (tier, authorship, sources, verified/review dates). For a fixture doc containing raw HTML and a remote image, Playwright asserts that no HTML is rendered and no network request is made for the image.
  3. Playwright visual steppers using `installTauriMock` stubs for `vox_kb_search`/`get`/`status` capture screenshots into `crates/vox-gui/ui/review-bundle/latest/`, and `pnpm test:e2e` and `pnpm typecheck` pass.

**Plans**: TBD
**UI hint**: yes

### Phase 7: Context Injection and Per-Chat Scope

**Goal**: Each chat turn, with any model, receives a small, relevance-gated, auditable KB preamble. The preamble never presents untrusted text as instructions and never sends private content to a remote model.
**Depends on**: Phase 3 (tiers, sanitizer), Phase 4 (scope/sensitivity facets), Phase 5 (calibrated hybrid scores)
**Requirements**: INJ-01, INJ-02, INJ-03, INJ-04, INJ-05, INJ-06, INJ-07, INJ-08, INJ-09, EVAL-06
**Success Criteria** (what must be TRUE):

  1. For an off-topic query nothing is injected, and the turn's `kb_context` event lists every candidate as dropped with reason "below floor". Across fixture models with different context lengths, injected tokens never exceed the per-model ceiling derived from the registry.
  2. The preamble is one merged ranked list (cards before chunks, per-doc cap, edge-weighted order) placed after the static system content. A test shows the cacheable prompt prefix is byte-identical across turns. Every turn emits `kb_context` with the injected items (source, section, tokens, score/backend, tier, staleness, reason) and the dropped candidates with drop reasons.
  3. Non-curated items appear inside per-turn boundary delimiters. Exfiltration-capable tools are unavailable while Untrusted content is in context. `sensitivity: private` content never reaches a remote model. All three guards are mutation-proven.
  4. `knowledge_cutoff` is stored in `ModelCapabilities` and labelled "estimated" when derived. Cutoff-aware boosting applies only to `scope: external` pages and reads `verified_at`. A whitespace-only-commit fixture leaves ranking byte-identical.
  5. Answers cite KB IDs, and the validator flags citations that don't resolve or don't support the claim. `vox_kb_*` tools survive the per-turn tool cap. Per-chat `@kb` scope, pin, exclude, trust floor and injection on/off each change the next turn's `kb_context` payload, and web fallback is governed only by research mode. A nightly generation eval (n≥3, cross-family judge, position-swap) reports faithfulness and citation support as advisory only, never as a PR gate.

**Plans**: TBD
**Research flag**: YES (`/gsd-plan-phase 7 --research-phase`). Open: calibrating the relevance floor (RRF scores are uncalibrated); the per-model ceiling heuristic (e.g. `min(1500 tokens, 2% of max_context)`); recency-detection heuristics feeding Phase 9's fallback.

### Phase 8: Inspector and Chat Controls GUI

**Goal**: A user can see exactly what KB context each turn received and why, and can steer it from the composer.
**Depends on**: Phase 6 (Library reader as the citation target), Phase 7 (`kb_context` payload schema, per-chat scope, citation validator). Can run in parallel with Phase 9.
**Requirements**: GUI-KB-03, GUI-KB-04, GUI-KB-08
**Success Criteria** (what must be TRUE):

  1. The per-turn Inspector renders the `kb_context` payload. Playwright asserts the item count equals the mocked backend payload, that each item shows its selection reason, and that dropped candidates are listed. Open, Exclude and Flag work, and Exclude changes the next turn's scope.
  2. From the composer, a user can add `@kb` scopes via typeahead (rendered as chips), toggle injection, and set strict sources/pin. Playwright asserts about 3 primary controls are visible, with the rest under Advanced.
  3. Inline chat citations link to doc#section in the Library reader. A citation the validator rejected shows an "unverified" badge.
  4. Visual steppers for the Inspector, composer controls and citations capture screenshots into `review-bundle/latest/`, and `test:e2e` and `typecheck` pass.

**Plans**: TBD
**UI hint**: yes

### Phase 9: Web Fallback, Inbox and Promotion PR

**Goal**: When the local KB can't answer, the agent captures new knowledge from the web into a reviewed inbox. A human-merged PR is the only path into curated docs.
**Depends on**: Phase 3 (`kb_findings` inbox, sanitizer, fail-closed eligibility), Phase 7 (confidence signal and recency detection)
**Requirements**: GROW-01, GROW-02, GROW-03, GROW-04, GROW-05
**Success Criteria** (what must be TRUE):

  1. A low-confidence or post-cutoff query, with research mode permitting, searches SearXNG then DDG first. Paid providers are used only under the daily cap recorded in the spend ledger. A test with the cap set to 0 shows "capped" surfaced in the turn output and event, never a silent skip.
  2. Captured pages are summarised in a step that has no tools available (asserted). They land as pending findings with provenance, and a test proves nothing pending is ever injected as curated.
  3. A user or agent can list, review, reject (with reason) and propose findings via `vox_kb_inbox_list`, `vox_kb_inbox_review` and `vox_kb_propose`. A near-duplicate of an existing doc is flagged by the similar-docs check.
  4. `vox kb promote` opens a draft PR from an isolated worktree. The user's working tree is unchanged, the doc lint and promotion lint pass, and the PR body carries per-claim source excerpts. A finding with single-domain sourcing is refused unless a reviewer override is given, and `docs/src/library/` has a CODEOWNERS entry.

**Plans**: TBD
**Research flag**: YES (`/gsd-plan-phase 9 --research-phase`). Open: the confidence threshold τ; the host crate given `vox-cli-review`'s CodeRabbit lineage; forge-token SecretId reuse; copyright posture for a public FOSS repo (needs a human/legal decision; interim policy is excerpts + URL + hash).

### Phase 10: Page-One Summary Rewrites

**Goal**: The most-retrieved docs open with a strong, accurate page-one summary. LLM-written text is labelled, kept out of training, and can't silently crowd out human-written pages.
**Depends on**: Phase 1 (eval), Phase 2 (index), Phase 4 (`authorship`, summary block), Phase 7 (retrieval-usage data from `kb_context`). Can run in parallel with Phases 9 and 11.
**Requirements**: REWR-01, REWR-02, REWR-03, REWR-04, REWR-05
**Success Criteria** (what must be TRUE):

  1. Before the first batch PR opens, the rewrite guard rejects every fixture: an unresolved identifier, path or CLI command; a retired surface; a dropped negation; a changed number; an edited code fence.
  2. Each batch PR covers ≤25 pages from one category. A CI check proves its diff touches only the delimited page-one block, and each page is stamped with `authorship`/`authorship_model` and `training_eligible: false`.
  3. The top-N docs by retrieval usage are rewritten before the long tail, and each batch records its human spot-check sample. The docs landing page and hub pages read summary-first.
  4. `vox kb eval` reports a summary-distinctiveness ratchet and a source-bias metric (retrieval share stratified by authorship). A batch that breaches either ratchet, or regresses the retrieval baseline, fails CI.

**Plans**: TBD
**Research flag**: YES (`/gsd-plan-phase 10 --research-phase`). Open: choosing a faithfulness judge from a different family than the rewriter, under the no-pinned-models policy.

### Phase 11: Staleness, Re-verification and KB Operator Docs

**Goal**: Stale docs are visible and down-ranked, and get re-verified on a budget using mechanical evidence only. A contributor can find every KB concept and operate the pipeline from the docs.
**Depends on**: Phase 4 (`review_by`/`verified_at` facets), Phase 9 (inbox for LLM-verdict items). Last backend phase.
**Requirements**: STALE-01, STALE-02, STALE-03, STALE-04, DOC-02
**Success Criteria** (what must be TRUE):

  1. `review_by` dates are hash-jittered within per-facet horizons, and a distribution test shows no single-day avalanche. Overdue docs show an overdue badge in results and rank lower, but are never hidden or deleted.
  2. The durable `@scheduled` re-verify job takes expired and high-traffic docs first and stops at its per-run page/token budget (test).
  3. `verified_at` advances only on mechanical evidence: paths exist, symbols resolve via `vox graph query`, CLI commands exist in the generated surface, links resolve. An LLM-only verdict opens an inbox item and leaves `verified_at` unchanged (mutation-proven). Superseded or failed docs are demoted to `status: legacy`, and no file is deleted.
  4. `docs/src/architecture/where-things-live.md` lists the indexer, `kb_findings`, `kb_context`, the taxonomy contract and the eval set. An operator how-to covering reindex, inbox review and promotion exists with valid frontmatter and passes `vox-doc-pipeline --lint-only`.

**Plans**: TBD
**Research flag**: none (the durable `@scheduled` runtime already exists).

### Phase 12: Inbox, Settings and Health GUI

**Goal**: A user can review and promote inbox findings, configure KB defaults, and watch index and verification health from the desktop app. Every KB surface ships with visual evidence.
**Depends on**: Phase 8 (GUI patterns), Phase 9 (inbox and promotion backend), Phase 11 (verification queue data)
**Requirements**: GUI-KB-05, GUI-KB-06, GUI-KB-07, GUI-KB-09
**Success Criteria** (what must be TRUE):

  1. In the Inbox, a user sees a finding's diff, its sources with trust chips and a similar-docs warning. They can reject with a reason or open a promotion PR through the `kb_open_promotion_pr` Tauri command.
  2. In Settings, a user can set the per-model budget override, the web spend cap (with current spend shown), the default trust floor and the embedding provider. Changes persist across restart and are reflected in the next turn's `kb_context`.
  3. A health view shows last-indexed commit and per-file errors, plus a verification queue of overdue docs ordered by usage × days overdue.
  4. Every KB surface from Phases 6, 8 and 12 has Playwright visual steppers with screenshots in `review-bundle/latest/`, and `test:e2e` and `typecheck` pass. A coverage check fails if any `vox_kb_*` tool lacks an `installTauriMock` stub.

**Plans**: TBD
**UI hint**: yes

## Dependencies and Parallelism

| Phase | Depends on |
|-------|------------|
| 1 | — |
| 2 | 1 |
| 3 | 2 |
| 4 | 1 (indexer integration after 2) |
| 5 | 2, 3, 4 |
| 6 | 2, 3, 4 |
| 7 | 3, 4, 5 |
| 8 | 6, 7 |
| 9 | 3, 7 |
| 10 | 1, 2, 4, 7 |
| 11 | 4, 9 |
| 12 | 8, 9, 11 |

- **Parallel-capable:** Phase 4 alongside Phases 2–3; Phase 6 alongside Phase 5; Phase 8 alongside Phase 9; Phase 10 alongside Phases 9 and 11.
- **Hard ordering:** eval before any tuning (1 first). Leaks fenced before new web paths (3 before 9). `authorship` before any rewrite (4 before 10). Stable chunk identity before vectors, citations and the Inspector (2 before 5, 7, 8).
- **GUI phases** (6, 8, 12) each follow the backend contract they display; run `/gsd-ui-phase` before planning each.

## Progress

**Execution Order:** numeric (1 → 12), with the parallel windows above.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Retrieval Eval Baseline | 12/12 | Complete    | 2026-10-05 |
| 2. Library Index and KB Tool Surface | 12/12 | Complete    | 2026-10-05 |
| 3. Provenance, Trust Tiers and Leak Closure | 16/16 | Complete    | 2026-10-10 |
| 4. Taxonomy Contract and Facet Lint | 0/TBD | Not started | - |
| 5. Local Embeddings and Ranking Signals | 0/TBD | Not started | - |
| 6. Library GUI | 0/TBD | Not started | - |
| 7. Context Injection and Per-Chat Scope | 0/TBD | Not started | - |
| 8. Inspector and Chat Controls GUI | 0/TBD | Not started | - |
| 9. Web Fallback, Inbox and Promotion PR | 0/TBD | Not started | - |
| 10. Page-One Summary Rewrites | 0/TBD | Not started | - |
| 11. Staleness, Re-verification and KB Operator Docs | 0/TBD | Not started | - |
| 12. Inbox, Settings and Health GUI | 0/TBD | Not started | - |
