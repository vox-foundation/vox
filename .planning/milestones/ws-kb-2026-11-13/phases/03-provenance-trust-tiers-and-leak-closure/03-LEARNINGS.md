---
phase: 3
phase_name: "provenance-trust-tiers-and-leak-closure"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 72
  lessons: 49
  patterns: 36
  surprises: 39
missing_artifacts: []
---

# Phase 3 Learnings: provenance-trust-tiers-and-leak-closure

## Decisions

### D-06 taint rule encoded once
D-06 taint rule encoded once: only node types fact and concept map to Advisory; everything else, including None, is Untrusted

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-01-SUMMARY.md

---

### NotApplicable (no DOI) stays distinct from Unavailable (lookup failed) s...
NotApplicable (no DOI) stays distinct from Unavailable (lookup failed) so non-academic sources remain promotable

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-01-SUMMARY.md

---

### Unterminated HTML comment strips to end of input; unterminated ![alt](ta...
Unterminated HTML comment strips to end of input; unterminated `![alt](target` is judged on its leading destination token (a later `)` could complete it) and neutralized if not allowed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-02-SUMMARY.md

---

### Reference and shortcut images (![a][r], ![a]) are always neutralized
Reference and shortcut images (`![a][r]`, `![a]`) are always neutralized: the target lives in a definition elsewhere and cannot be judged, so fail closed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-02-SUMMARY.md

---

### A destination is relative only if it has no scheme, is not protocol-rela...
A destination is relative only if it has no scheme, is not protocol-relative, and contains none of backslash, ampersand, quote, angle or backtick (markdown and HTML decode backslash escapes and entities, so `https\://x` and `https&colon;//x` are absolute)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-02-SUMMARY.md

---

### If the 8-pass fixpoint has not converged the input is adversarially nest...
If the 8-pass fixpoint has not converged the input is adversarially nested and the result is the empty string (fail closed, still idempotent)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-02-SUMMARY.md

---

### HTML img attributes are tokenized with quote awareness so a src= inside ...
HTML img attributes are tokenized with quote awareness so a `src=` inside another attribute value cannot masquerade as the real src; any srcset forces neutralization

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-02-SUMMARY.md

---

### Baseline 96 taken (no competing branch had it); digest re-updated twice,...
Baseline 96 taken (no competing branch had it); digest re-updated twice, once per task, integer stays 96

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-03-SUMMARY.md

---

### Backfill skips an empty-string corpus key in kb_index_state so web/legac...
Backfill skips an empty-string corpus key in kb_index_state so web/legacy rows can never be promoted by a malformed state row

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-03-SUMMARY.md

---

### Legacy readers live in kb_legacy.rs (not kb_findings.rs) so the findings...
Legacy readers live in kb_legacy.rs (not kb_findings.rs) so the findings module carries no confidence column name

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-03-SUMMARY.md

---

### kb_findings table (no CHECK, no trigger, no confidence column) with NewK...
`kb_findings` table (no CHECK, no trigger, no confidence column) with `NewKbFinding` (no `Default`; `FindingTier`, `SanitizedText`, `ContentHash`, `FindingOrigin` typed) and `insert_pending_finding` that always writes `pending` and dedupes on `dedupe_key`. A `compile_fail` doctest proves `FindingTier::Trusted` does not compile and a `no_run` twin proves the same value with `Untrusted` does.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-03-SUMMARY.md

---

### Readers: kb_findings_list (created_at_ms DESC, id ASC), kb_findings_coun...
Readers: `kb_findings_list` (`created_at_ms DESC, id ASC`), `kb_findings_counts`, `kb_findings_unqueued_reviewed` (reviewed_by set, approved/rejected, not queued), `kb_findings_mark_mens_queued`. Unknown status/origin errors on read; unknown tier reads Untrusted.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-03-SUMMARY.md

---

### search_documents.trust_tier TEXT NOT NULL DEFAULT 'untrusted'; version-g...
`search_documents.trust_tier TEXT NOT NULL DEFAULT 'untrusted'`; version-gated Rust backfill (per-corpus UPDATE from `kb_index_state.trust_tier` canonicalized through `ContextTrustTier::from_db_str`, then `vox://publication/%` to advisory). No index on the column (a v95 DB lacks it when the baseline batch runs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-03-SUMMARY.md

---

### VoxKB DDL split into SCHEMA_LEGACY_VOXKB (fragment legacy_voxkb), re-exp...
VoxKB DDL split into `SCHEMA_LEGACY_VOXKB` (fragment `legacy_voxkb`), re-exported as `vox_db::LEGACY_VOXKB_DDL`; `legacy_kb_counts` and `legacy_kb_entries_page` return zeros/empty when the tables are gone.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-03-SUMMARY.md

---

### kb_findings added to LEGACY_EXPORT_TABLES in sorted position.
`kb_findings` added to `LEGACY_EXPORT_TABLES` in sorted position.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-03-SUMMARY.md

---

### Redact the whole summary source (bounded to 2 MiB) before extracting quo...
Redact the whole summary source (bounded to 2 MiB) before extracting quotes or truncating, so a secret is never split on '.' or cut in half

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-04-SUMMARY.md

---

### source_url and title go through redact+sanitize like the summary; an IPv...
source_url and title go through redact+sanitize like the summary; an IPv4 literal in a URL is therefore stored as [REDACTED_IP] (redact_owned design)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-04-SUMMARY.md

---

### Tavily capture passes snippet empty and full_text = content; capture der...
Tavily capture passes snippet empty and full_text = content; capture derives the summary from the full_text prefix

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-04-SUMMARY.md

---

### Dropped the advertised tavily:<url> DurableArtifact entries
Dropped the advertised tavily:<url> DurableArtifact entries: nothing is persisted at that URI after the mirror removal

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-04-SUMMARY.md

---

### put_kb_index_state is the single source of the corpus tier
put_kb_index_state is the single source of the corpus tier: it writes the manifest and UPDATEs search_documents by corpus in one transaction, so rows the indexer skipped as unchanged cannot drift (Phase 11 must exempt evidence-bearing rows)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-05-SUMMARY.md

---

### Added VoxDb::search_document_tiers instead of allowlisting raw SQL in te...
Added VoxDb::search_document_tiers instead of allowlisting raw SQL in tests (sql-surface-guard); also usable by 03-09

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-05-SUMMARY.md

---

### mirror_search_corpus tier is a named const MIRROR_TIER so the file carri...
mirror_search_corpus tier is a named const MIRROR_TIER so the file carries a real in-file test (tdd-guard)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-05-SUMMARY.md

---

### D-06 taint rule applied
D-06 taint rule applied: deep synthesis is one Untrusted pending finding (dedupe scope = query, source URLs in provenance, no confidence key)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-06-SUMMARY.md

---

### create_research_source session bookkeeping kept as the plan says; it wri...
create_research_source session bookkeeping kept as the plan says; it writes an empty-content research_source knowledge node (label+URL only), which tier_for_knowledge_node_type already maps to Untrusted. Tests assert 'only bookkeeping nodes, never page text' rather than 'zero nodes' (the plan's two statements conflict)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-06-SUMMARY.md

---

### vox research publish writes a local draft (no new crate edge); GUI paths...
vox research publish writes a local draft (no new crate edge); GUI paths file findings (D-05 interim asymmetry)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-06-SUMMARY.md

---

### Task 1 (tracer)
**Task 1 (tracer).** `capture_research_sources` / `capture_research_synthesis` replace the knowledge-graph persist fn (deleted). `run_quick` split into `run_quick` + `run_quick_with_policy`; trace stage `knowledge_base` renamed `inbox` ("filed N pending findings for review", detail `{pending_findings, query}`). GUI `save_research_session_to_kb` files sources + synthesis (origin `GuiSaveResearchSession`); `research_repo_root` is `pub(crate)`. Test `inbox_quick_turn_writes_only_findings` runs against a wiremock SearXNG: corpus rows (0,0), no knowledge nodes, 2 pending Untrusted `research_quick` findings with url/fetched_at/hash, 2 snapshot files, trace stage `inbox` = 2.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-06-SUMMARY.md

---

### Task 2. ResearchConfig.finding_capture (None default, bool in Debug). we...
**Task 2.** `ResearchConfig.finding_capture` (None default, bool in Debug). `web_gather` keeps `create_research_source` but replaces the per-hit research-document ingest with one sink call (origin `ResearchGather`, `full_text` = raw body else snippet); no sink means no write. `run_deep` builds its config with `web_capture_for_state`, attaches the same sink to its `SearchRuntimeContext`, and files the synthesis (origin `ResearchDeep`). Both `handlers_memory.rs` research configs carry the sink.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-06-SUMMARY.md

---

### Task 3. Pipeline branch (j) and write_research_doc deleted (+ its test);...
**Task 3.** Pipeline branch (j) and `write_research_doc` deleted (+ its test); `slug_from_query` kept (in-file test added for tdd-guard). `persist_to_docs` carries the `vox-deprecated-since` marker. `publish_draft_path` -> `<root>/.vox/cache/research-drafts/<filename>` (gitignored by `**/.vox/cache/`); `Publish` doc comment and `docs/src/reference/cli.md` updated.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-06-SUMMARY.md

---

### D-05 applied
D-05 applied: @name still parses, injects nothing until Phase 7 (accepted regression, CHANGELOG in 03-13)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-07-SUMMARY.md

---

### D-06 applied
D-06 applied: MEMORY sink deleted, not rerouted

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-07-SUMMARY.md

---

### Prompt-leak seed embeds the canonicalized prompt in a legacy entry
Prompt-leak seed embeds the canonicalized prompt in a legacy entry: the retired enrichment searched with a substring match on the whole canonical prompt, so a plain keyword seed never matched and the first draft of the test passed before deletion (vacuous)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-07-SUMMARY.md

---

### Task 2. RetrievalBundle.kb_lines, its KbStore::search_entries fill and c...
**Task 2.** `RetrievalBundle.kb_lines`, its `KbStore::search_entries` fill and constructor field removed from `retrieval.rs`; the `[AUTONOMOUS RESEARCH — KNOWLEDGE BASE]` push and the whole `kb_mention_lines` / `[MENTIONED KNOWLEDGE BASES]` block removed from `message.rs`. `parse_kb_mentions` and its tests untouched apart from a doc line (kept for Phase 7 INJ-09, no consumer). `MemoryManager::sync_verified_research_findings` and `tests/memory_research_sync_test.rs` deleted. Two prompt tests (`kb_leak_keyword_prompt_carries_no_legacy_kb_text`, `kb_leak_at_mention_injects_nothing`) read the request bodies wiremock received and assert neither the marker string nor either header is present.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-07-SUMMARY.md

---

### A capture_finding error aborts the pass (logged as a warning at attach, ...
A capture_finding error aborts the pass (logged as a warning at attach, retried next attach) rather than silently skipping a row

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-08-SUMMARY.md

---

### KbSignals marks only ids that landed in an emitted pair; unpaired review...
KbSignals marks only ids that landed in an emitted pair; unpaired reviewed rows stay unqueued so a later counterpart can pair

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-08-SUMMARY.md

---

### migrate_legacy_kb_entries counts the three legacy tables first, returns ...
`migrate_legacy_kb_entries` counts the three legacy tables first, returns early at zero entries, pages `legacy_kb_entries_page` (500, id cursor), skips whitespace-only rows (`skipped_empty`), and calls `capture_finding` with origin `LegacyKbEntry` and `dedupe_scope = legacy id` (dedupe hit -> `already_present`). Provenance carries legacy_id, kb_name, source_signal, source_ref, routing_confidence, accepted, mens_queued, created_at_ms. The `accepted` flag never raises trust. It only reads legacy tables.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-08-SUMMARY.md

---

### with_db_initialized spawns it (info log when any legacy count is non-zer...
`with_db_initialized` spawns it (info log when any legacy count is non-zero, warn on error, never fatal).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-08-SUMMARY.md

---

### kb_findings_dpo_records (pure) groups reviewed rows by provenance query,...
`kb_findings_dpo_records` (pure) groups reviewed rows by provenance `query`, emits one `{type: dpo, source: kb_findings, prompt, chosen, rejected}` per approved/rejected pair, and returns only used ids. `run_kb_signals_stage` reads `kb_findings_unqueued_reviewed`, writes `mix_sources/kb_signals.jsonl`, marks used ids via `kb_findings_mark_mens_queued`. All legacy-API reads and the SFT branch are gone from vox-ml-cli.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-08-SUMMARY.md

---

### Tier label is a prefix, so crates/vox-search/src/rrf.rs rrf_dedup_key st...
Tier label is a prefix, so crates/vox-search/src/rrf.rs rrf_dedup_key strips a leading [tier:..] label (otherwise fused lines would stop deduping)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-09-SUMMARY.md

---

### Qdrant sidecar lines carry no stored tier
Qdrant sidecar lines carry no stored tier: counted Untrusted and dropped under exclude_untrusted_hits

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-09-SUMMARY.md

---

### KB tool output is sanitized by one recursive string-leaf pass (ok_clean)...
KB tool output is sanitized by one recursive string-leaf pass (ok_clean) rather than per-field calls, so a new field cannot be forgotten

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-09-SUMMARY.md

---

### Sources are sanitized before the 600-char cap so the cap cannot cut a co...
Sources are sanitized before the 600-char cap so the cap cannot cut a construct in half

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-09-SUMMARY.md

---

### vox-db. trust_tiers_for_chunk_ids joins search_document_chunks to search...
**vox-db.** `trust_tiers_for_chunk_ids` joins `search_document_chunks` to `search_documents.trust_tier` by chunk rowid; a missing or non-numeric id reads Untrusted. `trust_tiers_for_knowledge_nodes` maps `node_type` through `tier_for_knowledge_node_type` (missing id Untrusted). Both knowledge-search queries exclude `node_type = 'research_cache'`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-09-SUMMARY.md

---

### vox-search. SearchPolicy::exclude_untrusted_hits (serde default false). ...
**vox-search.** `SearchPolicy::exclude_untrusted_hits` (serde default false). `execute_search_plan` looks up tiers for chunk and KG hits, prefixes lines `[tier:<db_str>]`, and under the flag drops Untrusted chunk, KG, web and qdrant lines together with their parallel `unified_hits` (KG score keeps its pre-filter rank, so survivor order is unchanged). `SearchExecution::min_trust_tier` folds memory (Advisory), repo/symbol/graphify (Trusted), web/qdrant (Untrusted) and looked-up chunk/KG tiers; `None` when nothing is kept. Web hits are still filed in the findings inbox before the flag drops them.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-09-SUMMARY.md

---

### mcp. run_retrieval_bundle sets the flag for AutoChatPreamble only; Retri...
**mcp.** `run_retrieval_bundle` sets the flag for `AutoChatPreamble` only; `RetrievalEvidenceEnvelope.min_trust_tier` carries the execution minimum and `to_context_envelope` uses `unwrap_or(Untrusted)`. All five bundle legs, `sources_from_hits` title and snippet, and all `vox_kb_*` JSON string leaves go through `SanitizedText`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-09-SUMMARY.md

---

### D-02 option-a (retire now, no aliases), pre-approved in 03-CONTEXT.md; C...
D-02 option-a (retire now, no aliases), pre-approved in 03-CONTEXT.md; CHANGELOG break note is 03-13

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-10-SUMMARY.md

---

### Catalog ids are matched with a previous-char exclusion of [a-z0-9_.] so ...
Catalog ids are matched with a previous-char exclusion of [a-z0-9_.] so a field access like self.kb.clip never reads as a catalog id

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-10-SUMMARY.md

---

### Task 2. The 11 remaining catalog blocks (220 lines, exactly kb.create..k...
**Task 2.** The 11 remaining catalog blocks (220 lines, exactly `kb.create`..`kb.query`, stopping before Phase 2's `kb.search`), 21 dispatch lines and 15 schema lines deleted by exact arm; `kb_tools/handlers.rs` and `kb_tools/params.rs` removed with `git rm`; `kb_tools/mod.rs` now declares `findings` and `library` only; five legacy lines dropped from `http-read-role-governance.yaml`; `scope_guard.rs` comment no longer names legacy KB write tools. `BANNED` now holds all 12 tool names, their 12 catalog ids, `kb_lines`, `signal_chat`, `signal_research`, `signal_scientia`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-10-SUMMARY.md

---

### D-05 interim visibility
D-05 interim visibility: toasts say 'Filed for review ... pending review and not yet searchable'; no visible text says 'Published' (Playwright-asserted)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-11-SUMMARY.md

---

### D-06: every side-door finding is Untrusted; dedupe surfaces as status du...
D-06: every side-door finding is Untrusted; dedupe surfaces as status duplicate ('Already pending review'), not an error

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-11-SUMMARY.md

---

### publish keeps store_research_artifact as a best-effort episodic log (its...
publish keeps store_research_artifact as a best-effort episodic log (its failure no longer reports as indexed:false; the field is gone)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-11-SUMMARY.md

---

### Empty slug is rejected too (beyond the old GUI rules), since it would pr...
Empty slug is rejected too (beyond the old GUI rules), since it would propose docs/src/architecture/-2026.md

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-11-SUMMARY.md

---

### mcp helpers. file_gui_finding(db, root, origin, title, text, source_url,...
**mcp helpers.** `file_gui_finding(db, root, origin, title, text, source_url, provenance) -> FindingFiled` wraps `capture_finding` (text = full_text) and maps `finding_id` Some/None to `pending`/`duplicate`. `file_publish_draft(db, root, session_id, slug, content) -> PublishFiled` validates the slug (rejects empty, `/`, `\`, `..`, leading `.`), builds `docs/src/architecture/<slug>-2026.md` as a string without touching disk, and captures with origin `publish_research_doc`, `dedupe_scope = proposed_path`, provenance `{proposed_path, session_id}`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-11-SUMMARY.md

---

### GUI Rust. ingest_url keeps URL validation, robots check and scrape, then...
**GUI Rust.** `ingest_url` keeps URL validation, robots check and scrape, then files via `file_gui_finding` (origin `GuiIngestUrl`, provenance title + text density). `ingest_text` keeps the empty-content check and files with origin `GuiIngestText`. `publish_research_doc` calls `file_publish_draft` then a best-effort `store_research_artifact`; the atomic docs write and its import are gone. `save_research_doc` and its `main.rs` registration deleted; `gui-surface-coverage.v1.json` regenerated (one line removed).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-11-SUMMARY.md

---

### GUI UI. FindingFiled and PublishDocResult TS types mirror the DTOs; onPu...
**GUI UI.** `FindingFiled` and `PublishDocResult` TS types mirror the DTOs; `onPublished(proposedPath, status)`; toasts `Filed for review` / `Already pending review`; Save-to-KB body "N source(s) filed for review; not yet searchable"; Memory URL/Text card descriptions and the text button ("File for Review") no longer promise knowledge-graph storage; the publish modal blurb says nothing is written until a reviewer approves.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-11-SUMMARY.md

---

### Playwright. kb-inbox-side-doors.spec.ts (installTauriMock + addInvokeOve...
**Playwright.** `kb-inbox-side-doors.spec.ts` (installTauriMock + addInvokeOverrides, 4 tests): URL pending, URL duplicate, Text, Save-to-KB + Publish; asserts toast copy, the proposed path, and `getByText(/Published/)` count 0; screenshots `kb-inbox-ingest-url.png`, `-ingest-url-duplicate.png`, `-ingest-text.png`, `-save-to-kb.png`, `-publish-doc.png` in `review-bundle/latest/` (gitignored, not committed).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-11-SUMMARY.md

---

### Task 2. SCHEMA_LEGACY_VOXKB moved verbatim (plus per-table comments in t...
**Task 2.** `SCHEMA_LEGACY_VOXKB` moved verbatim (plus per-table comments in the quarantine file's format) from `knowledge.rs` to `quarantine.rs`; the `legacy_voxkb` fragment left `SCHEMA_FRAGMENTS`; `baseline_sql()` appends the constant inside the existing `#[cfg(feature = "quarantine")]` branch; `vox_db::LEGACY_VOXKB_DDL` re-exports the new path. Two gating tests sit next to the existing ones in `manifest.rs`. The digest was recomputed with `print_baseline_digest` (`0x9c76d9f9...2839`), the integer stays 96, and a ledger line was added in both `manifest.rs` and `baseline-version-policy.yaml`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-12-SUMMARY.md

---

### Count-read failures return err_json with the existing REM_KB_REINDEX con...
Count-read failures return err_json with the existing REM_KB_REINDEX constant, never a silent zero (T-03-38)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-13-SUMMARY.md

---

### The integration test attaches the database by assigning state.db directl...
The integration test attaches the database by assigning state.db directly, not through with_db_initialized, so the attach-time legacy migration task cannot race the counts

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-13-SUMMARY.md

---

### The AGENTS.md first cell equals agents_md_row exactly; the explanatory t...
The AGENTS.md first cell equals agents_md_row exactly; the explanatory text sits in the replacement column

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-13-SUMMARY.md

---

### Task 2. One registry row voxkb-kbstore-surface (documentation-only, back...
**Task 2.** One registry row `voxkb-kbstore-surface` (documentation-only, backed by a symbol of the same id) names the store, the router and the 12 CRUD tools. The symbol patterns are identifier-bounded (`\bvox_kb_(?:create|list|...|clip)\b`, `\b(?:KbStore|KbRouter)\b`), so Phase 2's `vox_kb_search|get|cite|status|reindex` never match. The SSOT section covers the four-way tier mapping, the D-06 taint rule, the stored-tier source, the injection rules (preamble drops `Untrusted`, explicit tools label, envelope tier is the minimum), the capture chokepoint and snapshot location, the legacy migration and quarantine, DPO-only MENS signals, and the sanitizer with its two out-of-scope prompt helpers. The `VOX_SEARCH_PERSIST_WEB_HITS_DISABLED` bullet now carries the D-04 meaning. The CHANGELOG lists the tools and types retired, the inbox routing, the tier-aware preamble, the GUI side-door change, inert `@kb` mentions, DPO-only MENS signals, the unrecallable SFT export, baseline 96, and the separate-sanitizer note (D16).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-13-SUMMARY.md

---

### Preferred CR-01 fix taken
Preferred CR-01 fix taken: Tavily does not fire under the exclusion (saves credits, no web egress on the preamble). Consequence: an auto-preamble turn no longer files Tavily findings into the inbox; D-04 capture is unchanged for explicit queries

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-14-SUMMARY.md

---

### retrieval.rs:330 keeps its own flag assignment (now redundant, harmless,...
retrieval.rs:330 keeps its own flag assignment (now redundant, harmless, not edited)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-14-SUMMARY.md

---

### Red-team URL probes use https://ok.example/p/<input>, not evil.example, ...
Red-team URL probes use https://ok.example/p/<input>, not evil.example, because two red-team cases list evil.example in must_not_contain and the base host would trip them

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-14-SUMMARY.md

---

### Task 2. sanitized_source(n, url, title, engine, snippet) maps all four s...
**Task 2.** `sanitized_source(n, url, title, engine, snippet)` maps all four strings through `SanitizedText` (snippet sanitized then capped at 600) and is the only place a numbered `Source` is built outside test fixtures; `sources_from_hits` and `pipeline_sources` (hence `failed_run_sources`) use it. `flatten` stays in `sources_context_block`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-14-SUMMARY.md

---

### D-07a honored
D-07a honored: crates/vox-search/src/execution.rs untouched; the memory leg (MEMORY.md, daily logs) still feeds the preamble unlabelled, and the e2e test pins that

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-15-SUMMARY.md

---

### D-06 narrowed, not removed
D-06 narrowed, not removed: fact/concept stay Advisory for non-agent writers; the allowlist match is unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-15-SUMMARY.md

---

### Task 2. retier_agent_knowledge_nodes(conn) runs two parameterized UPDATE...
**Task 2.** `retier_agent_knowledge_nodes(conn)` runs two parameterized UPDATEs (`fact` -> `agent_fact`, `concept` -> `agent_concept`, both `AND id NOT LIKE 'graphify:%'`), called right after `backfill_search_document_trust_tiers` in the `current_version < BASELINE_VERSION` block. No DDL, no baseline integer, no digest change, no delete.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-15-SUMMARY.md

---

### D-08 read against HEAD
D-08 read against HEAD: in git ls-tree HEAD and absent from git diff HEAD; a staged-but-uncommitted new file is Untrusted (stricter, fail-closed)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-16-SUMMARY.md

---

### Reverses the 03-05 'rows always agree with kb_index_state' invariant; re...
Reverses the 03-05 'rows always agree with kb_index_state' invariant; replaced by 'every walked file has its tier re-evaluated every run'

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-16-SUMMARY.md

---

### Subdirectory checkouts match nothing, so every file is Untrusted (fail-c...
Subdirectory checkouts match nothing, so every file is Untrusted (fail-closed), documented on committed_clean_docs

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 03-16-SUMMARY.md

---

### Task 2. VoxDb::set_library_document_tier(corpus, rel_path, tier) (single...
**Task 2.** `VoxDb::set_library_document_tier(corpus, rel_path, tier)` (single-row, parameterized, no-op on unknown pair). The hash-skip path now calls it before counting `docs_unchanged`; the read-error path demotes to Untrusted. `IndexReport::docs_untrusted` counts walked files whose tier is Untrusted (changed, unchanged, or unreadable); `report_json` exposes it.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-16-SUMMARY.md

---

### Task 3. SSOT tier table (Trusted, Advisory, Untrusted rows), a D-07/D-07...
**Task 3.** SSOT tier table (Trusted, Advisory, Untrusted rows), a D-07/D-07a agent-graph-rows paragraph, rewritten "Where the tier is stored", the "Known gap: Tavily" bullet replaced by four injection-rule bullets (auto-preamble exclusion and Tavily skip, explicit Tavily `[tier:untrusted]`, agent graph rows with memory leg unchanged, research-source sanitization), and the web-capture bullet note. CHANGELOG trust-tier bullet extended.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 03-16-SUMMARY.md

## Lessons

### The first RED attempt for Task 1 used an empty-string stub body, which t...
The first RED attempt for Task 1 used an empty-string stub body, which the `tdd-guard` toestub hook flagged as a skeleton warning; the stub was changed to `"unset"` and the hook passed (no `--no-verify`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### No .clone() on a tier value existed in the repo (rg trust_tier\.clone on...
No `.clone()` on a tier value existed in the repo (`rg trust_tier\.clone` only hits unrelated populi string fields), so no clippy `clone_on_copy` fixups were needed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### ContextTrust (envelope struct) did not need changes; Copy is additive.
`ContextTrust` (envelope struct) did not need changes; `Copy` is additive.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-01-SUMMARY.md

---

### 1. [Rule 3 - Blocking] .gitignore swallowed the red-team fixture
**1. [Rule 3 - Blocking] `.gitignore` swallowed the red-team fixture**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### Found during
**Found during:** Task 2 RED commit (`git add` refused `contracts/kb/redteam/cases.v1.jsonl`; `.gitignore:189` ignores `*.jsonl`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### Fix: added !contracts/kb/redteam/*.jsonl next to the existing !contracts...
**Fix:** added `!contracts/kb/redteam/*.jsonl` next to the existing `!contracts/kb/eval/*.jsonl` exception. `.gitignore` was not in the plan's file list.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### Commit: 17dd7f62e
**Commit:** `17dd7f62e`

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-02-SUMMARY.md

---

### 1. [Rule 3 - Blocking] NewKbFinding has no lifetime parameter. The plan ...
**1. [Rule 3 - Blocking] `NewKbFinding` has no lifetime parameter.** The plan wrote `NewKbFinding<'a>` but every field is owned (`String`, `Vec<SanitizedText>`); an unused lifetime does not compile. Struct is `NewKbFinding`; callers write the same literal. Commit `80cf75eda`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### 2. [Rule 3 - Blocking] Legacy readers placed in kb_legacy.rs, not kb_fin...
**2. [Rule 3 - Blocking] Legacy readers placed in `kb_legacy.rs`, not `kb_findings.rs`.** Plan step (4) said "in the same file" but the same plan's acceptance grep requires no `confidence` in `kb_findings.rs`, and `LegacyKbEntry.routing_confidence` is spec'd. A sibling module satisfies both; public names and `vox_db::` re-exports are unchanged. Commit `8e6b45cca`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### 3. [Rule 3 - Blocking] In-file tests added to kb_findings.rs, kb_legacy....
**3. [Rule 3 - Blocking] In-file tests added to `kb_findings.rs`, `kb_legacy.rs`, `codex_legacy.rs`.** The pre-commit `tdd-guard` (toestub `enforce-strict`) blocks commits that touch a file with pub fns and no `#[test]`. Small real tests (insert/dedupe, fresh-DB legacy readers, `LEGACY_EXPORT_TABLES` sorted and contains `kb_findings`). Commits `80cf75eda`, `8e6b45cca`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### 4. [Scope boundary] Pre-existing failure logged, not fixed. legacy_expor...
**4. [Scope boundary] Pre-existing failure logged, not fixed.** `legacy_export_covers_all_baseline_tables` (feature `legacy-import`) fails because `LEGACY_EXPORT_TABLES` lacks `kb_index_state`, `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events`, `web_cache`; all predate this plan. `kb_findings` is correctly on both sides. Logged in `deferred-items.md`. The plan's third verify line for this test therefore cannot go green from this plan.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-03-SUMMARY.md

---

### 1. [Rule 1 - Bug] Removed the tavily:<url> DurableArtifact entries in bu...
**1. [Rule 1 - Bug] Removed the `tavily:<url>` DurableArtifact entries in bundle.rs**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### Issue: the CRAG branch advertised a durable artifact with uri = tavily:<...
**Issue:** the CRAG branch advertised a durable artifact with `uri = tavily:<url>` for the chunk it mirrored into the corpus. With the mirror gone that URI points at nothing, so the A2A response would carry a dangling reference.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### Fix: dropped the artifact loop (comment left in place); the test asserts...
**Fix:** dropped the artifact loop (comment left in place); the test asserts `durable_artifacts` is empty. The only consumers just serialize the vector (`a2a_contract.rs`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-04-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Wire-validator re-export removal done in Task 1's...
**1. [Rule 3 - Blocking] Wire-validator re-export removal done in Task 1's commit.** Deleting the validator from `library.rs` (needed by Task 1) broke the three re-export lists at once, so Task 3 step 1 landed with Task 1. Task 3 commit holds only the invariant test.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### 2. [Rule 3 - Blocking] search_document_tiers accessor added (not in plan...
**2. [Rule 3 - Blocking] `search_document_tiers` accessor added (not in plan).** The agreement and ingest tests initially read tiers with raw `connection().query`, which `sql-surface-guard` rejects outside vox-db. A typed accessor (with a test) replaced it. Commit 257572cb5.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### 3. [Rule 3 - Blocking] tdd-guard on two touched files. vox-cli/.../retri...
**3. [Rule 3 - Blocking] tdd-guard on two touched files.** `vox-cli/.../retrieval.rs` and `vox-db/src/research.rs` had no in-file tests (pre-existing), which blocked the commit under `enforce-strict`. Added `MIRROR_TIER` plus a one-assert test, and an in-file `research_ingest_mirror_row_is_untrusted` test (replacing an assertion I had first put in `tests/db_research_tests.rs`, which was reverted).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### 4. Rule 1 style. put_kb_index_state now takes a clone of the state to mo...
**4. Rule 1 style.** `put_kb_index_state` now takes a clone of the state to move into the transaction closure; no behavior change.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-05-SUMMARY.md

---

### Build host: rustc for vox-corpus/vox-publisher hung in uninterruptible I...
Build host: rustc for `vox-corpus`/`vox-publisher` hung in uninterruptible I/O on stale `target/debug/incremental/<crate>-*` dirs; killed and removed those incremental dirs (build artefacts only, gitignored). Fresh test binaries then took ~18 s each to list (Gatekeeper scan), so nextest runs were long.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-06-SUMMARY.md

---

### 1. [Rule 3 - Blocking] manager.rs needed a real unit test to commit. Sta...
**1. [Rule 3 - Blocking] manager.rs needed a real unit test to commit.** Staging `manager.rs` tripped the pre-commit `tdd-guard` (`skeleton/untested-pub-api`, whole-file scan: 27 public fns, no `#[test]`; the deleted integration test lived in `tests/`, so it was never visible to the guard). Added `for_account_scopes_the_manager_and_starts_with_no_keys` rather than suppressing. Not `--no-verify`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-07-SUMMARY.md

---

### Files: crates/vox-orchestrator/src/memory/manager.rs. Commit: e98a1235a.
**Files:** crates/vox-orchestrator/src/memory/manager.rs. **Commit:** e98a1235a.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-07-SUMMARY.md

---

### 2. [Rule 1 - Bug in own test] vacuous keyword test. See Evidence; fixed ...
**2. [Rule 1 - Bug in own test] vacuous keyword test.** See Evidence; fixed in the same task before the deletion.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-07-SUMMARY.md

---

### 3. [Rule 1 - Bug in own test] sql-surface-guard tripped on a comment. A ...
**3. [Rule 1 - Bug in own test] sql-surface-guard tripped on a comment.** A code comment in the new test file spelled the banned `connection().query(` token; the guard scans raw text. Reworded.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-07-SUMMARY.md

---

### 1. [Process] No separate RED commit. Tests and implementation were writt...
**1. [Process] No separate RED commit.** Tests and implementation were written in the same working session and committed together per task, because each build on this host takes 3 to 9 minutes. The mutation proofs supply the red/green evidence.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-08-SUMMARY.md

---

### No other deviations.
No other deviations.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-08-SUMMARY.md

---

### 1. [Rule 3 - Blocking] crates/vox-search/src/rrf.rs modified (not in fil...
**1. [Rule 3 - Blocking] `crates/vox-search/src/rrf.rs` modified (not in files_modified).** The `[tier:..]` prefix hid the `[chunk:`/`[node:` markers `rrf_dedup_key` keys on, so fused lines would have stopped deduplicating. The key now strips a leading tier label; unit test `dedup_key_ignores_the_tier_label`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-09-SUMMARY.md

---

### 2. [Rule 2 - Missing critical] Qdrant sidecar and web lines under the fl...
**2. [Rule 2 - Missing critical] Qdrant sidecar and web lines under the flag.** Plan named chunk, KG and web lines; qdrant lines carry no stored tier, so they are treated as Untrusted and dropped under `exclude_untrusted_hits` (else a sidecar hit would bypass the preamble filter).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-09-SUMMARY.md

---

### 3. Tests added outside the named file. execute_search_plan_labels_tiers_...
**3. Tests added outside the named file.** `execute_search_plan_labels_tiers_and_excludes_untrusted_on_request` in `crates/vox-search/tests/execute_search_plan_smoke.rs` (vox-search level label/filter/`unified_hits` parallelism/min tier) and `kb_injection_kb_tools_neutralize_exfil_and_tag_block` in `kb_tools/library.rs` (it needs that module's `#[serial]` index harness; `kb_tool_surface.rs` is only a surface-list test).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-09-SUMMARY.md

---

### 4. Process. nextest stalled after a successful build for -p vox-search -...
**4. Process.** `nextest` stalled after a successful build for `-p vox-search -p vox-db` (killed after the 3300 s timeout); those new tests were run with `cargo test` instead, per the lean-verification fallback. A rate-limit interruption stopped the run once; resumed from disk state.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-09-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Generated projections carried Phase 2 drift. mode...
**1. [Rule 3 - Blocking] Generated projections carried Phase 2 drift.** `model-manifest.generated.json` and `gui-surface-coverage.v1.json` at HEAD were stale relative to Phase 2's kb.search/get/cite/status/reindex/eval rows, so regenerating them (the only permitted way) also added those Phase 2 entries (model-manifest +127/-11 lines in Task 1, GUI report gained `kb eval/reindex/status` paths and `kb.cite|eval|get|reindex|search|status`). This is correct generator output, not a hand edit. **Commit:** 50bae7cd7.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-10-SUMMARY.md

---

### 2. [Rule 1 - Test ergonomics] Previous-char boundary added to the matche...
**2. [Rule 1 - Test ergonomics] Previous-char boundary added to the matcher.** The plan only specified the next-char rule; the previous-char exclusion `[a-z0-9_.]` was added so `.rs` field accesses (`self.kb.clip(..)`) cannot false-positive on catalog ids. Covered by `legacy_voxkb_matcher_respects_identifier_boundaries`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-10-SUMMARY.md

---

### 3. Housekeeping. rustfmt on kb_tools/mod.rs reformatted the sibling libr...
**3. Housekeeping.** `rustfmt` on `kb_tools/mod.rs` reformatted the sibling `library.rs` (it formats child modules); that change was reverted with `git checkout -- <file>` before the commit, so `library.rs` is untouched. `rustfmt --check` on `dispatch.rs` reports a pre-existing import-order diff at line ~2068 (not from this plan; left alone).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-10-SUMMARY.md

---

### Total deviations
**Total deviations:** 2 (1 blocking-by-generator, 1 test hardening). **Impact:** none on scope.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-10-SUMMARY.md

---

### 1. [Rule 3 - Blocking] domains::quarantine was not always compiled. The ...
**1. [Rule 3 - Blocking] `domains::quarantine` was not always compiled.** The plan's context says the module "itself always compiles", but at HEAD `domains/mod.rs` declared it `#[cfg(feature = "quarantine")]`. A re-export of a constant that lives there cannot exist on default builds, which would have broken the `LEGACY_VOXKB_DDL` test seeding the plan requires. Fix: the `cfg` was removed from the `pub mod quarantine;` declaration (with a comment); `baseline_sql()` still appends the DDL only under the feature, so the default schema is unchanged apart from losing the legacy fragment. **Commit:** 79058f5b9.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-12-SUMMARY.md

---

### 2. [Rule 2 - Test hardening] Wider ban list and uppercase identifiers. T...
**2. [Rule 2 - Test hardening] Wider ban list and uppercase identifiers.** The plan named the store, router, row types and CRUD fns; the legacy orchestrator types (`KnowledgeBase`, `KbEntry`, `KbEntrySource`, `KbRoutingRule`, `KbRoutingRuleType`) went onto `BANNED` too, and `is_ident` was widened to `[A-Za-z0-9_]` so `KbStoreFactory` / `LegacyKbEntry` cannot false-positive. Covered by the boundary test. **Commit:** f1454c95f.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-12-SUMMARY.md

---

### Local builds on this host repeatedly stalled
Local builds on this host repeatedly stalled: rustc sat at ~0% CPU for 25+ minutes on `vox_corpus` and then deterministically on `vox_db` (same dep-graph file size every time) under the default incremental profile. Each hung rustc was killed (only my own) and its `target/debug/incremental/<crate>-*` directories deleted. `CARGO_INCREMENTAL=0` with `LEAN_CTX_DISABLED=1` made the identical `cargo check -p vox-db` finish in 1 m 12 s, and every later build used it. A full `cargo build -p vox-cli --bin vox` still stalled in `vox_cli` codegen after 22 minutes, so no fresh `vox` binary exists for the `vox ci` checks above. Recorded in `deferred-items.md`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-12-SUMMARY.md

---

### 1. [Rule 3 - Blocking] The AGENTS.md first cell is plain text, not backt...
**1. [Rule 3 - Blocking] The AGENTS.md first cell is plain text, not backticked.** The plan's acceptance grep (`KbStore, KbRouter, legacy vox_kb_\* CRUD tools`) and the registry's `agents_md_row` need the identical plain string, so the row uses it verbatim as the first cell and puts "(retired KB Phase 3, D-02; no aliases)" in the replacement column. **Commit:** 3ec392ef5.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-13-SUMMARY.md

---

### 2. [Rule 2 - Missing verification] A stricter integration test than the ...
**2. [Rule 2 - Missing verification] A stricter integration test than the plan sketched.** Besides the two shapes, the test asserts that every value in `legacy_voxkb` and `inbox` is a number and that no string anywhere inside them holds a path separator (T-03-46), and seeds the legacy entries as whitespace-only so the attach-time migration can never move rows into the inbox and skew the counts. **Commit:** 4a09b6e68.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-13-SUMMARY.md

---

### Same host problem as 03-12
Same host problem as 03-12: rustc intermittently stalled at near-zero CPU for 15 to 25 minutes on freshly started compile units (`vox_db`, `vox_cli`, two integration-test targets, a lib-test `vox_corpus`). `CARGO_INCREMENTAL=0` plus `LEAN_CTX_DISABLED=1` helped, and a small retry wrapper that kills only this worktree's own rustc processes after four minutes below twenty CPU-seconds made the remaining builds complete. No cargo or rustc process of mine is left running. The wrapper lives in the session scratchpad, not in the repo.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-13-SUMMARY.md

---

### Build tooling, not code
Build tooling, not code: an orphaned cargo from the first RED attempt held the target lock with a stuck rustc, then another session's `cargo clean` was deleting this worktree's `target/` (corrupting rmeta files mid-build). I killed my own orphan and ran all verification with `CARGO_TARGET_DIR` set to a scratchpad directory; nothing was committed for this. The other session's `cargo clean` (pid 14973) was left alone.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-14-SUMMARY.md

---

### Commit guard note
Commit guard note: the `agent-*` branch allow-list in the executor protocol was not applied; this is a sequential run on `claude/vox-knowledge-base-research-2964d2` (not a protected branch), as the dispatch instructed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-14-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Tasks 1 and 2 landed in one commit. They edit the...
**1. [Rule 3 - Blocking] Tasks 1 and 2 landed in one commit.** They edit the same four files and Task 1's tracer needs `docs_untrusted` / `set_library_document_tier` only for Task 2, but the skip-path re-tier and the report field are interleaved in `index_library` and the MCP tests; splitting would have required a throwaway intermediate state. Task 2's tests and mutation (c) are recorded separately above. Commit count is therefore 2, not 3.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-16-SUMMARY.md

---

### 2. [Plan test fixture] vox-db test row collision. The plan's "same rel_p...
**2. [Plan test fixture] vox-db test row collision.** The plan's "same rel_path in another corpus" cannot exist: `apply_library_document` keys `source_uri` on `kb_library:<rel_path>` regardless of corpus, so a second corpus with the same path overwrites the row. The test uses a distinct path in the other corpus and asserts that `set_library_document_tier(KB_LIBRARY_CORPUS, <other corpus path>)` is a no-op, which pins the corpus scope.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-16-SUMMARY.md

---

### 3. [Plan command] Doc lint path. --paths takes paths relative to docs/sr...
**3. [Plan command] Doc lint path.** `--paths` takes paths relative to `docs/src`, so the plan's `docs/src/architecture/...` argument errors; ran with `architecture/search-retrieval-ssot-2026.md`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-16-SUMMARY.md

---

### 4. [Minor] docs_untrusted also counts an unreadable file whose computed ...
**4. [Minor] `docs_untrusted` also counts an unreadable file whose computed tier would have been Trusted**, because it is demoted. Matches the stated intent (operators see every Untrusted doc).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 03-16-SUMMARY.md

---

### The previous WR-02 (redact before sanitize) is the only open item I woul...
The previous WR-02 (redact before sanitize) is the only open item I would not leave for later: it lets a secret split by a zero-width character reach the inbox in plaintext. It is not part of this diff, so it is not re-counted here.

**Context:** Surfaced in structured phase code review.
**Source:** 03-REVIEW.md

---

### See the human_verification frontmatter
See the `human_verification` frontmatter: (1) hosted-CI gates, including the vox-orchestrator goal-search tests affected by the WR-08 behaviour change and crate-edges; (2) the Playwright/vitest GUI suite; (3) the WR-01/WR-02 upgrade-path decision; (4) requirements-state housekeeping.

**Context:** Verified in phase acceptance checks.
**Source:** 03-VERIFICATION.md

---

### Both gaps from the first verification are closed in code, with tests tha...
Both gaps from the first verification are closed in code, with tests that fail under mutation. The phase goal holds: every KB row and finding carries a stored or derivable tier (now including a git-gated Library tier and Untrusted agent-written graph rows), the unreviewed side doors are closed, and retrieval runs through the single `vox-search` path with the preamble's Untrusted exclusion applied at the one place every caller passes through. No must-have fails.

**Context:** Verified in phase acceptance checks.
**Source:** 03-VERIFICATION.md

---

### total: 3 passed: 3 issues: 0 pending: 0 skipped: 0 blocked: 0
total: 3 passed: 3 issues: 0 pending: 0 skipped: 0 blocked: 0

**Context:** Observed during user acceptance testing.
**Source:** 03-UAT.md

## Patterns

### no-Default enum at the type boundary (compile_fail doctest)
no-Default enum at the type boundary (compile_fail doctest)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-01-SUMMARY.md

---

### exhaustive match without wildcard for security verdicts
exhaustive match without wildcard for security verdicts

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-01-SUMMARY.md

---

### hand-rolled bounded-fixpoint text scanner (no regex)
hand-rolled bounded-fixpoint text scanner (no regex)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-02-SUMMARY.md

---

### private-field smart-constructor newtype with compile_fail doctest
private-field smart-constructor newtype with compile_fail doctest

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-02-SUMMARY.md

---

### independent proptest oracle (ranges restated outside the code under test...
independent proptest oracle (ranges restated outside the code under test)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-02-SUMMARY.md

---

### fail-closed column default as the DB-level guard (Turso has no CHECK/tri...
fail-closed column default as the DB-level guard (Turso has no CHECK/triggers)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-03-SUMMARY.md

---

### version-gated Rust backfill instead of a correlated UPDATE
version-gated Rust backfill instead of a correlated UPDATE

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-03-SUMMARY.md

---

### injected fire-and-forget sink (mirrors LexicalMemoryFallback) so vox-sea...
injected fire-and-forget sink (mirrors LexicalMemoryFallback) so vox-search needs no vox-crypto edge

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-04-SUMMARY.md

---

### redact before split/truncate, then sanitize
redact before split/truncate, then sanitize

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-04-SUMMARY.md

---

### content-addressed snapshot, hash-only filename
content-addressed snapshot, hash-only filename

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-04-SUMMARY.md

---

### single production tier decision (tier_for_curated(false) in reindex_at) ...
single production tier decision (tier_for_curated(false) in reindex_at) aligned onto rows by the manifest write

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-05-SUMMARY.md

---

### static source-scan invariant with non-vacuity floor
static source-scan invariant with non-vacuity floor

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-05-SUMMARY.md

---

### extract the capture/bookkeeping step into a small fn (file_gathered_hits...
extract the capture/bookkeeping step into a small fn (file_gathered_hits) so the gather leak is unit-testable without a provider mock

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-06-SUMMARY.md

---

### research output reaches the KB only as pending Untrusted kb_findings
research output reaches the KB only as pending Untrusted kb_findings

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-06-SUMMARY.md

---

### sink-injection (no sink => no write, fail-closed)
sink-injection (no sink => no write, fail-closed)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-06-SUMMARY.md

---

### test-first red evidence for a deletion
test-first red evidence for a deletion: keep the failing assertion run in the log before removing the code

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-07-SUMMARY.md

---

### leak tests seed the legacy tables AND a row the retired path would have ...
leak tests seed the legacy tables AND a row the retired path would have matched, so a regression cannot pass vacuously

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-07-SUMMARY.md

---

### Seed a row the filter must drop AND a row it must keep, so a tier test c...
Seed a row the filter must drop AND a row it must keep, so a tier test cannot pass vacuously

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-09-SUMMARY.md

---

### tier label as a line prefix, rrf dedup key strips it
tier label as a line prefix, rrf dedup key strips it

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-09-SUMMARY.md

---

### sanitize at ingest and again at injection
sanitize at ingest and again at injection

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-09-SUMMARY.md

---

### pre-filter rank kept so dropping a hit never reorders survivors
pre-filter rank kept so dropping a hit never reorders survivors

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-09-SUMMARY.md

---

### retire a tool family through the catalog
retire a tool family through the catalog: delete exact blocks, regenerate every projection, never hand-edit generated files

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-10-SUMMARY.md

---

### static absence test with identifier-boundary matching and a scanned-file...
static absence test with identifier-boundary matching and a scanned-file floor

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-10-SUMMARY.md

---

### Tracer-first
Tracer-first: Ingest URL proved the helper -> Tauri command -> inbox -> toast slice before Ingest Text / Publish expanded

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-11-SUMMARY.md

---

### side-door logic and tests live in mcp helpers (vox-gui Rust tests need t...
side-door logic and tests live in mcp helpers (vox-gui Rust tests need the Tauri sidecar)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-11-SUMMARY.md

---

### findings DTO status is pending|duplicate; duplicate is a toast, not an e...
findings DTO status is pending|duplicate; duplicate is a toast, not an error

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-11-SUMMARY.md

---

### additive status payload pinned by a contract test that lists every prior...
additive status payload pinned by a contract test that lists every prior key

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-13-SUMMARY.md

---

### counts-only objects added to a path-bearing tool that stays off the HTTP...
counts-only objects added to a path-bearing tool that stays off the HTTP read role

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-13-SUMMARY.md

---

### documentation-only retirement row backed by a symbol pattern with identi...
documentation-only retirement row backed by a symbol pattern with identifier boundaries

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-13-SUMMARY.md

---

### derive the policy flag where the trigger is consumed instead of in each ...
derive the policy flag where the trigger is consumed instead of in each caller

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-14-SUMMARY.md

---

### skip the untrusted-egress leg under an exclusion rather than fetch and d...
skip the untrusted-egress leg under an exclusion rather than fetch and discard

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-14-SUMMARY.md

---

### one sanitizing constructor for a prompt-bound struct
one sanitizing constructor for a prompt-bound struct

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-14-SUMMARY.md

---

### shared node-type constants in the L0 trust crate used by writer and migr...
shared node-type constants in the L0 trust crate used by writer and migration

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-15-SUMMARY.md

---

### re-type inside an existing unshipped baseline gate
re-type inside an existing unshipped baseline gate: no DDL, no new baseline integer

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-15-SUMMARY.md

---

### decide a derived trust tier per item on every run, including on the unch...
decide a derived trust tier per item on every run, including on the unchanged-skip path

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-16-SUMMARY.md

---

### fail-closed git probe through the central GitExec helper
fail-closed git probe through the central GitExec helper

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 03-16-SUMMARY.md

## Surprises

### Estimate variance on 03-01
Estimate variance on 03-01: planned 40000 tokens vs 4500 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-01-PLAN.md, 03-01-SUMMARY.md

---

### Estimate variance on 03-02
Estimate variance on 03-02: planned 55000 tokens vs 10300 actual (0.19x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-02-PLAN.md, 03-02-SUMMARY.md

---

### 1. [Rule 3 - Blocking] .gitignore swallowed the red-team fixture
**1. [Rule 3 - Blocking] `.gitignore` swallowed the red-team fixture**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-02-SUMMARY.md

---

### Estimate variance on 03-03
Estimate variance on 03-03: planned 65000 tokens vs 13181 actual (0.20x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-03-PLAN.md, 03-03-SUMMARY.md

---

### 1. [Rule 3 - Blocking] NewKbFinding has no lifetime parameter. The plan ...
**1. [Rule 3 - Blocking] `NewKbFinding` has no lifetime parameter.** The plan wrote `NewKbFinding<'a>` but every field is owned (`String`, `Vec<SanitizedText>`); an unused lifetime does not compile. Struct is `NewKbFinding`; callers write the same literal. Commit `80cf75eda`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-03-SUMMARY.md

---

### 2. [Rule 3 - Blocking] Legacy readers placed in kb_legacy.rs, not kb_fin...
**2. [Rule 3 - Blocking] Legacy readers placed in `kb_legacy.rs`, not `kb_findings.rs`.** Plan step (4) said "in the same file" but the same plan's acceptance grep requires no `confidence` in `kb_findings.rs`, and `LegacyKbEntry.routing_confidence` is spec'd. A sibling module satisfies both; public names and `vox_db::` re-exports are unchanged. Commit `8e6b45cca`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-03-SUMMARY.md

---

### 3. [Rule 3 - Blocking] In-file tests added to kb_findings.rs, kb_legacy....
**3. [Rule 3 - Blocking] In-file tests added to `kb_findings.rs`, `kb_legacy.rs`, `codex_legacy.rs`.** The pre-commit `tdd-guard` (toestub `enforce-strict`) blocks commits that touch a file with pub fns and no `#[test]`. Small real tests (insert/dedupe, fresh-DB legacy readers, `LEGACY_EXPORT_TABLES` sorted and contains `kb_findings`). Commits `80cf75eda`, `8e6b45cca`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-03-SUMMARY.md

---

### Estimate variance on 03-04
Estimate variance on 03-04: planned 90000 tokens vs 30000 actual (0.33x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-04-PLAN.md, 03-04-SUMMARY.md

---

### 1. [Rule 1 - Bug] Removed the tavily:<url> DurableArtifact entries in bu...
**1. [Rule 1 - Bug] Removed the `tavily:<url>` DurableArtifact entries in bundle.rs**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-04-SUMMARY.md

---

### Estimate variance on 03-05
Estimate variance on 03-05: planned 70000 tokens vs 4500 actual (0.06x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-05-PLAN.md, 03-05-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Wire-validator re-export removal done in Task 1's...
**1. [Rule 3 - Blocking] Wire-validator re-export removal done in Task 1's commit.** Deleting the validator from `library.rs` (needed by Task 1) broke the three re-export lists at once, so Task 3 step 1 landed with Task 1. Task 3 commit holds only the invariant test.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-05-SUMMARY.md

---

### 2. [Rule 3 - Blocking] search_document_tiers accessor added (not in plan...
**2. [Rule 3 - Blocking] `search_document_tiers` accessor added (not in plan).** The agreement and ingest tests initially read tiers with raw `connection().query`, which `sql-surface-guard` rejects outside vox-db. A typed accessor (with a test) replaced it. Commit 257572cb5.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-05-SUMMARY.md

---

### 3. [Rule 3 - Blocking] tdd-guard on two touched files. vox-cli/.../retri...
**3. [Rule 3 - Blocking] tdd-guard on two touched files.** `vox-cli/.../retrieval.rs` and `vox-db/src/research.rs` had no in-file tests (pre-existing), which blocked the commit under `enforce-strict`. Added `MIRROR_TIER` plus a one-assert test, and an in-file `research_ingest_mirror_row_is_untrusted` test (replacing an assertion I had first put in `tests/db_research_tests.rs`, which was reverted).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-05-SUMMARY.md

---

### 4. Rule 1 style. put_kb_index_state now takes a clone of the state to mo...
**4. Rule 1 style.** `put_kb_index_state` now takes a clone of the state to move into the transaction closure; no behavior change.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-05-SUMMARY.md

---

### Estimate variance on 03-06
Estimate variance on 03-06: planned 80000 tokens vs 12000 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-06-PLAN.md, 03-06-SUMMARY.md

---

### Estimate variance on 03-07
Estimate variance on 03-07: planned 60000 tokens vs 6100 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-07-PLAN.md, 03-07-SUMMARY.md

---

### 1. [Rule 3 - Blocking] manager.rs needed a real unit test to commit. Sta...
**1. [Rule 3 - Blocking] manager.rs needed a real unit test to commit.** Staging `manager.rs` tripped the pre-commit `tdd-guard` (`skeleton/untested-pub-api`, whole-file scan: 27 public fns, no `#[test]`; the deleted integration test lived in `tests/`, so it was never visible to the guard). Added `for_account_scopes_the_manager_and_starts_with_no_keys` rather than suppressing. Not `--no-verify`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-07-SUMMARY.md

---

### 2. [Rule 1 - Bug in own test] vacuous keyword test. See Evidence; fixed ...
**2. [Rule 1 - Bug in own test] vacuous keyword test.** See Evidence; fixed in the same task before the deletion.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-07-SUMMARY.md

---

### 3. [Rule 1 - Bug in own test] sql-surface-guard tripped on a comment. A ...
**3. [Rule 1 - Bug in own test] sql-surface-guard tripped on a comment.** A code comment in the new test file spelled the banned `connection().query(` token; the guard scans raw text. Reworded.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-07-SUMMARY.md

---

### Estimate variance on 03-08
Estimate variance on 03-08: planned 55000 tokens vs 12000 actual (0.22x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-08-PLAN.md, 03-08-SUMMARY.md

---

### No other deviations.
No other deviations.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-08-SUMMARY.md

---

### Estimate variance on 03-09
Estimate variance on 03-09: planned 75000 tokens vs 9000 actual (0.12x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-09-PLAN.md, 03-09-SUMMARY.md

---

### 1. [Rule 3 - Blocking] crates/vox-search/src/rrf.rs modified (not in fil...
**1. [Rule 3 - Blocking] `crates/vox-search/src/rrf.rs` modified (not in files_modified).** The `[tier:..]` prefix hid the `[chunk:`/`[node:` markers `rrf_dedup_key` keys on, so fused lines would have stopped deduplicating. The key now strips a leading tier label; unit test `dedup_key_ignores_the_tier_label`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-09-SUMMARY.md

---

### 2. [Rule 2 - Missing critical] Qdrant sidecar and web lines under the fl...
**2. [Rule 2 - Missing critical] Qdrant sidecar and web lines under the flag.** Plan named chunk, KG and web lines; qdrant lines carry no stored tier, so they are treated as Untrusted and dropped under `exclude_untrusted_hits` (else a sidecar hit would bypass the preamble filter).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-09-SUMMARY.md

---

### Estimate variance on 03-10
Estimate variance on 03-10: planned 55000 tokens vs 13200 actual (0.24x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-10-PLAN.md, 03-10-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Generated projections carried Phase 2 drift. mode...
**1. [Rule 3 - Blocking] Generated projections carried Phase 2 drift.** `model-manifest.generated.json` and `gui-surface-coverage.v1.json` at HEAD were stale relative to Phase 2's kb.search/get/cite/status/reindex/eval rows, so regenerating them (the only permitted way) also added those Phase 2 entries (model-manifest +127/-11 lines in Task 1, GUI report gained `kb eval/reindex/status` paths and `kb.cite|eval|get|reindex|search|status`). This is correct generator output, not a hand edit. **Commit:** 50bae7cd7.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-10-SUMMARY.md

---

### 2. [Rule 1 - Test ergonomics] Previous-char boundary added to the matche...
**2. [Rule 1 - Test ergonomics] Previous-char boundary added to the matcher.** The plan only specified the next-char rule; the previous-char exclusion `[a-z0-9_.]` was added so `.rs` field accesses (`self.kb.clip(..)`) cannot false-positive on catalog ids. Covered by `legacy_voxkb_matcher_respects_identifier_boundaries`.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-10-SUMMARY.md

---

### Total deviations
**Total deviations:** 2 (1 blocking-by-generator, 1 test hardening). **Impact:** none on scope.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-10-SUMMARY.md

---

### Estimate variance on 03-11
Estimate variance on 03-11: planned 75000 tokens vs 28000 actual (0.37x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-11-PLAN.md, 03-11-SUMMARY.md

---

### Estimate variance on 03-12
Estimate variance on 03-12: planned 50000 tokens vs 15100 actual (0.30x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-12-PLAN.md, 03-12-SUMMARY.md

---

### 1. [Rule 3 - Blocking] domains::quarantine was not always compiled. The ...
**1. [Rule 3 - Blocking] `domains::quarantine` was not always compiled.** The plan's context says the module "itself always compiles", but at HEAD `domains/mod.rs` declared it `#[cfg(feature = "quarantine")]`. A re-export of a constant that lives there cannot exist on default builds, which would have broken the `LEGACY_VOXKB_DDL` test seeding the plan requires. Fix: the `cfg` was removed from the `pub mod quarantine;` declaration (with a comment); `baseline_sql()` still appends the DDL only under the feature, so the default schema is unchanged apart from losing the legacy fragment. **Commit:** 79058f5b9.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-12-SUMMARY.md

---

### 2. [Rule 2 - Test hardening] Wider ban list and uppercase identifiers. T...
**2. [Rule 2 - Test hardening] Wider ban list and uppercase identifiers.** The plan named the store, router, row types and CRUD fns; the legacy orchestrator types (`KnowledgeBase`, `KbEntry`, `KbEntrySource`, `KbRoutingRule`, `KbRoutingRuleType`) went onto `BANNED` too, and `is_ident` was widened to `[A-Za-z0-9_]` so `KbStoreFactory` / `LegacyKbEntry` cannot false-positive. Covered by the boundary test. **Commit:** f1454c95f.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-12-SUMMARY.md

---

### Estimate variance on 03-13
Estimate variance on 03-13: planned 50000 tokens vs 8800 actual (0.18x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-13-PLAN.md, 03-13-SUMMARY.md

---

### 1. [Rule 3 - Blocking] The AGENTS.md first cell is plain text, not backt...
**1. [Rule 3 - Blocking] The AGENTS.md first cell is plain text, not backticked.** The plan's acceptance grep (`KbStore, KbRouter, legacy vox_kb_\* CRUD tools`) and the registry's `agents_md_row` need the identical plain string, so the row uses it verbatim as the first cell and puts "(retired KB Phase 3, D-02; no aliases)" in the replacement column. **Commit:** 3ec392ef5.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-13-SUMMARY.md

---

### 2. [Rule 2 - Missing verification] A stricter integration test than the ...
**2. [Rule 2 - Missing verification] A stricter integration test than the plan sketched.** Besides the two shapes, the test asserts that every value in `legacy_voxkb` and `inbox` is a number and that no string anywhere inside them holds a path separator (T-03-46), and seeds the legacy entries as whitespace-only so the attach-time migration can never move rows into the inbox and skew the counts. **Commit:** 4a09b6e68.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-13-SUMMARY.md

---

### Estimate variance on 03-14
Estimate variance on 03-14: planned 70000 tokens vs 9500 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-14-PLAN.md, 03-14-SUMMARY.md

---

### Estimate variance on 03-15
Estimate variance on 03-15: planned 65000 tokens vs 6500 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-15-PLAN.md, 03-15-SUMMARY.md

---

### Estimate variance on 03-16
Estimate variance on 03-16: planned 85000 tokens vs 9000 actual (0.11x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 03-16-PLAN.md, 03-16-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Tasks 1 and 2 landed in one commit. They edit the...
**1. [Rule 3 - Blocking] Tasks 1 and 2 landed in one commit.** They edit the same four files and Task 1's tracer needs `docs_untrusted` / `set_library_document_tier` only for Task 2, but the skip-path re-tier and the report field are interleaved in `index_library` and the MCP tests; splitting would have required a throwaway intermediate state. Task 2's tests and mutation (c) are recorded separately above. Commit count is therefore 2, not 3.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 03-16-SUMMARY.md

