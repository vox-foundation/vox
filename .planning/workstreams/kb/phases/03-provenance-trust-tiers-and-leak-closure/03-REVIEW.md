---
phase: 03-provenance-trust-tiers-and-leak-closure
reviewed: 2026-10-08T00:00:00Z
depth: standard
files_reviewed: 98
files_reviewed_list:
  - contracts/capability/capability-registry.yaml
  - contracts/db/baseline-version-policy.yaml
  - contracts/documentation/retired-symbols.v1.yaml
  - contracts/mcp/http-read-role-governance.yaml
  - contracts/mcp/tool-registry.canonical.yaml
  - contracts/operations/catalog.v1.yaml
  - contracts/retirement/retired-surfaces.v1.yaml
  - crates/vox-cli-research/src/lib.rs
  - crates/vox-cli/src/commands/db_research/retrieval.rs
  - crates/vox-cli/src/commands/kb.rs
  - crates/vox-cli/src/commands/kb/gate.rs
  - crates/vox-cli/src/commands/kb/gate/tests.rs
  - crates/vox-db-types/Cargo.toml
  - crates/vox-db-types/src/kb_text.rs
  - crates/vox-db-types/src/lib.rs
  - crates/vox-db/src/codex_legacy.rs
  - crates/vox-db/src/lib.rs
  - crates/vox-db/src/local_tests.rs
  - crates/vox-db/src/questioning_telemetry.rs
  - crates/vox-db/src/research.rs
  - crates/vox-db/src/schema/domains/knowledge.rs
  - crates/vox-db/src/schema/domains/mod.rs
  - crates/vox-db/src/schema/domains/quarantine.rs
  - crates/vox-db/src/schema/manifest.rs
  - crates/vox-db/src/store/mod.rs
  - crates/vox-db/src/store/open.rs
  - crates/vox-db/src/store/ops_memory/kb_findings.rs
  - crates/vox-db/src/store/ops_memory/kb_legacy.rs
  - crates/vox-db/src/store/ops_memory/knowledge.rs
  - crates/vox-db/src/store/ops_memory/library.rs
  - crates/vox-db/src/store/ops_memory/mod.rs
  - crates/vox-db/src/store/ops_memory/search.rs
  - crates/vox-db/tests/kb_findings_store.rs
  - crates/vox-db/tests/kb_library_store.rs
  - crates/vox-db/tests/tiered_writer_invariant.rs
  - crates/vox-gui/src/commands/knowledge.rs
  - crates/vox-gui/src/commands/research.rs
  - crates/vox-gui/src/main.rs
  - crates/vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts
  - crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Research/DocPublishModal.tsx
  - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Research/researchActions.ts
  - crates/vox-ml-cli/src/commands/mens/pipeline.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message_kb_leak_tests.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/mod.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/input_schemas.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/findings.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/mod.rs
  - crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs
  - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
  - crates/vox-orchestrator-mcp/src/scope_guard.rs
  - crates/vox-orchestrator-mcp/src/server_state.rs
  - crates/vox-orchestrator-mcp/src/task_tools/tests.rs
  - crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs
  - crates/vox-orchestrator-mcp/tests/kb_findings_gui.rs
  - crates/vox-orchestrator-mcp/tests/kb_legacy_migration.rs
  - crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs
  - crates/vox-orchestrator-mcp/tests/kb_status_inbox.rs
  - crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs
  - crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs
  - crates/vox-orchestrator-types/src/lib.rs
  - crates/vox-orchestrator-types/src/trust.rs
  - crates/vox-orchestrator/src/context_envelope.rs
  - crates/vox-orchestrator/src/lib.rs
  - crates/vox-orchestrator/src/memory/manager.rs
  - crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs
  - crates/vox-research-shim/src/research/orchestrator/config.rs
  - crates/vox-research-shim/src/research/orchestrator/helpers.rs
  - crates/vox-research-shim/src/research/orchestrator/pipeline.rs
  - crates/vox-research-shim/src/research/orchestrator/web_gather.rs
  - crates/vox-research-shim/src/research/persistence.rs
  - crates/vox-research-shim/src/research/types.rs
  - crates/vox-research-shim/tests/scientia_phase_0a_persistence.rs
  - crates/vox-search/src/a2a_contract.rs
  - crates/vox-search/src/bundle.rs
  - crates/vox-search/src/capture.rs
  - crates/vox-search/src/context.rs
  - crates/vox-search/src/execution.rs
  - crates/vox-search/src/ingest.rs
  - crates/vox-search/src/kb_eval/library.rs
  - crates/vox-search/src/lib.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/src/policy.rs
  - crates/vox-search/src/rrf.rs
  - crates/vox-search/src/trust.rs
  - crates/vox-search/tests/execute_search_plan_smoke.rs
  - crates/vox-search/tests/ingest_markdown_tree.rs
  - crates/vox-search/tests/kb_library_chunking.rs
  - crates/vox-search/tests/kb_library_index.rs
  - crates/vox-search/tests/web_capture_test.rs
findings:
  critical: 1
  warning: 10
  info: 5
  total: 16
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-10-08
**Depth:** standard (changed hunks read in full, callers and writers cross-checked)
**Files Reviewed:** 98 (plus the 11 deleted files, checked for dangling references)
**Status:** issues_found

## Summary

The core machinery is sound. `FindingTier` cannot express `Trusted`. `ContextTrustTier::from_db_str` fails closed. The sanitizer is careful: fixpoint passes, no slicing panics, fail-closed on non-convergence. The tiered-writer invariant test and the type-level `upsert_search_document` tier parameter close the "writer forgets a tier" class. The legacy VoxKB surface is genuinely gone from `crates/`, the MCP contracts and the GUI. I found no dangling references to the 11 deleted files (only the intentional ban lists, CHANGELOG and plan docs mention them). The legacy migration is idempotent, and it only reads the legacy tables, so it deletes nothing.

The phase's headline invariant is "an unprompted preamble never carries an Untrusted hit, and nothing elevates model or web output". That invariant has a live hole: the Tavily leg in `run_search_with_verification` bypasses `exclude_untrusted_hits`, the tier label and `min_trust_tier` (CR-01). The remaining issues are redaction and sanitization ordering and coverage, an unverified retraction check behind a fail-closed gate, unbounded inbox growth, and a few places where the "Trusted/Advisory" tier is granted on weaker evidence than the docs claim.

Cross-checked against `deferred-items.md`: the `legacy_export_covers_all_baseline_tables` failure is a real gap but not a Phase 3 regression (see IN-02). The other deferred items are not live bugs.

## Critical Issues

### CR-01: Tavily web hits bypass `exclude_untrusted_hits`, the tier label and `min_trust_tier`, and reach the chat preamble

**File:** `crates/vox-search/src/bundle.rs:233-236` (consumer: `crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs:766-776`)
**Issue:** `execute_search_plan` honours `policy.exclude_untrusted_hits` for web, chunk, knowledge and Qdrant lines. The post-pass in `run_search_with_verification` (Tavily CRAG leg, default-enabled `tavily` cargo feature, runtime `policy.tavily_enabled`) then does this unconditionally:

```rust
execution.web_lines.extend(t_lines.iter().cloned());
if policy.prefer_rrf_merge {
    execution.rrf_fused_lines.extend(t_lines);
}
```

- `exclude_untrusted_hits` is never consulted, so the knob's documented guarantee ("drop Untrusted ... web hits before fusion") does not hold on this path.
- `rrf_fused_lines` is the first block `chat_message` injects into the prompt (`[AUTONOMOUS RESEARCH — RRF FUSION]`). The MCP caller sets `exclude_untrusted_hits = true` for `AutoChatPreamble`. The `skip_web` flag only clears `web_research_enabled`, not `tavily_enabled`. With Tavily configured, and `tavily_fire_on_empty` or `fire_on_weak` firing (which is common when local evidence is thin), live web text lands in an unprompted preamble.
- The Tavily lines carry no `[tier:untrusted]` label (`[crag_tavily:<url>] ...`).
- `execution.min_trust_tier` is not updated. The envelope can therefore report `Trusted` or `Advisory` for a bundle that contains live web text, and `RetrievalEvidenceEnvelope::into_context_envelope` consumes that value (`retrieval.rs:155`).
- The text is sanitized at the bundle boundary, but sanitization is not the tier guarantee D-03/D-05 promise.

The Tavily path has no equivalent of the `execute_search_plan` gate, and no test covers it (`message_kb_leak_tests.rs` and `kb_retrieval_tiers.rs` stub SearXNG only).

**Fix:**
```rust
// bundle.rs, after building t_lines
let t_lines: Vec<String> = hits.iter().map(|h| format!(
    "[tier:untrusted] [crag_tavily:{}] {} (score: {:.3})",
    h.url, h.content.replace('\n', " "), h.score)).collect();
// capture block unchanged (capture is still wanted) ...
if !policy.exclude_untrusted_hits {
    execution.web_lines.extend(t_lines.iter().cloned());
    if policy.prefer_rrf_merge { execution.rrf_fused_lines.extend(t_lines); }
    execution.min_trust_tier = Some(
        execution.min_trust_tier.map_or(ContextTrustTier::Untrusted, |m| m.min(ContextTrustTier::Untrusted)));
}
```
Better: gate the whole Tavily firing on `!policy.exclude_untrusted_hits` (a preamble should not spend credits on a result it must discard). Add a wiremock-backed Tavily test beside `kb_retrieval_tiers.rs`.

## Warnings

### WR-01: Promotion retraction check reads Crossref `update-to`, which is the field on the retraction notice, not on the retracted paper

**File:** `crates/vox-search/src/trust.rs:53-79` (new consumer at `:84-98`, `promotion_eligibility` at `:277`)
**Issue:** `check_retraction` looks for `message.update-to[].type == "retraction"`. In the Crossref REST schema `update-to` lists the works this record updates, so it appears on the retraction notice's record. A retracted paper's own record carries the pointer in `updated-by`. Looking up the retracted paper's DOI (the only DOI a promotion path will have) returns `Clean`. That is harmless for ranking, which was already fail-open. TRUST-06 now builds a fail-closed promotion gate on it, and the tests (`promotion_lookup_retraction_is_retracted`) use a mock that encodes the same assumption, so they cannot detect it. Separately, a 404 for DataCite or arXiv DOIs maps to `Unavailable` and is therefore ineligible; that is safe but should be a deliberate `NotApplicable`.
**Fix:** Also deserialize `updated-by` (and optionally `relation.is-retracted-by`), and test against a captured real Crossref payload for a known retracted DOI:
```rust
#[serde(default, rename = "updated-by")] updated_by: Vec<CrossrefUpdate>,
// retracted = update_to.iter().chain(updated_by.iter()).any(|u| u.update_type.eq_ignore_ascii_case("retraction"))
```

### WR-02: Redaction runs before sanitization, so invisible characters defeat redaction and are then stripped, reassembling the secret

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:91-93, 143-151`
**Issue:** `clean()` is `sanitize(redact(raw))`, and the summary and quotes path is the same order. `redact_owned` regexes (email, key, token) do not match when a zero-width or tag-block codepoint is interleaved (`sk-\u{200B}abc...`). `sanitize` then strips the codepoint, so the full secret or email is stored in the title, summary, quote or URL. Pasted or scraped text from sites that inject zero-width characters into tokens hits this accidentally; it needs no attacker.
**Fix:** Sanitize first, then redact, then sanitize again (idempotent) so redaction output cannot form a new construct:
```rust
fn clean(raw: &str) -> SanitizedText {
    SanitizedText::new(&vox_redact::redact_owned(&vox_db::kb_text::sanitize(raw)))
}
```
Apply the same order to `summary` and `extract_quotes`.

### WR-03: Snapshot stores the raw, unredacted, unsanitized text, contradicting the module's own contract, and its hash does not match what is written

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:99-112, 133, 151-152`
**Issue:** The module doc says "Everything stored is redacted then sanitized". The snapshot, though, is `content` verbatim: secrets pasted into GUI "Ingest Text", and hidden-instruction payloads from scraped pages, persist in plaintext under `.vox/cache/kb-findings/`. The snapshot is the artefact a reviewer or the planned promotion step (Phase 9) would read, so it bypasses the sanitizer the rest of the design depends on. Also `content_hash` is computed over the full `content`, but `write_snapshot` truncates to `MAX_SNAPSHOT_BYTES`. For inputs over 2 MiB the file cannot reproduce the recorded hash, so the "evidence for the content hash" claim fails.
**Fix:** Write `clean(content)` (truncated) to the snapshot, and hash exactly the bytes written (or record both `content_hash` and `snapshot_bytes_truncated` in provenance). If raw evidence is genuinely required, store it with a restrictive mode and document that readers must sanitize.

### WR-04: `provenance` is an unvalidated `serde_json::Value` that bypasses the `SanitizedText` guarantee

**File:** `crates/vox-db/src/store/ops_memory/kb_findings.rs:66,143`; callers `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:169,324-335,392`, `crates/vox-gui/src/commands/knowledge.rs:96-101,196-201`
**Issue:** The store header claims "an unsanitized string cannot reach a row through this writer". Provenance contradicts that: it is stored as-is and read back as `serde_json::Value`. Callers put raw web titles (`doc.title`), the user's search `query` (not redacted; it may contain a key), legacy `source_ref` and `kb_name`, and the pasted `source_url` into it. Any later renderer, promotion drafter or MENS exporter that reads `provenance.query` (the DPO builder already does, `pipeline.rs:603`) consumes unsanitized, unredacted text.
**Fix:** Make provenance a typed struct of `SanitizedText` leaves (or sanitize string leaves in `insert_pending_finding`, as `clean()` in `kb_tools/library.rs` already does for tool output). Redact `query`.

### WR-05: Quick-research source URL and engine are not sanitized before entering the prompt; deep-path sources are not sanitized at all

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs:182-195, 223-230, 695-705`
**Issue:** `sources_from_hits` now sanitizes `title` and `snippet`, but `url: h.path.clone()` is raw, and `sources_context_block` only replaces newlines (`flatten`). A result URL carrying tag-block or zero-width characters, or a `)![x](https://evil/?q=` suffix, goes straight into the `[n] title — url (engine)` line of the quick-research context. `pipeline_sources` (deep path, also used for failed-run sources shown in the trace) sanitizes nothing: title, URL and snippet are raw. They are not prompt-injected today, but they feed the GUI trace and `capture_research_sources`.
**Fix:** `url: SanitizedText::new(&h.path).into_string()` (and `engine`), and run `pipeline_sources` through the same helper.

### WR-06: Legacy migration silently skips orphaned entries and aborts the whole pass on one failure

**File:** `crates/vox-db/src/store/ops_memory/kb_legacy.rs:83`, `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:351`
**Issue:** The page query is `FROM kb_entries e JOIN knowledge_bases b ON b.id = e.kb_id`. FK enforcement is not guaranteed on this connection, so an entry whose `kb_id` has no row (or whose KB was deleted by the old `vox_kb_delete` under a non-cascading connection) is never returned. It is counted in `kb_entries` but is not in `migrated`, `skipped_empty` or `already_present`. It stays only in the quarantined table and is never surfaced, which is exactly the "no data loss" gap RET-03 forbids. Also `capture_finding(...).await?` aborts the loop on the first error (for example one disk-write failure), so every later entry is blocked on every startup until the cause is fixed. Pagination by `e.id > cursor` makes the abort deterministic.
**Fix:** `LEFT JOIN knowledge_bases b ... COALESCE(b.name, '(orphaned)')`, and on a per-entry error increment a `failed` counter and continue; log the total. Add an assertion `migrated + skipped_empty + already_present + failed == kb_entries` to `kb_legacy_migration.rs`.

### WR-07: Inbox and snapshot growth is unbounded and driven by every search

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:98-112, 132-141, 405-420`; `crates/vox-search/src/policy.rs` (`persist_web_hits` default true)
**Issue:** `dedupe_key = hash(origin, scope, content_hash)`. A page whose body differs slightly between fetches (ads, timestamps) files a new pending finding and a new snapshot (up to 2 MiB each) on every search. The same URL under `WebSearch`, `ResearchQuick`, `ResearchGather` and `CragTavily` origins is filed once per origin. There is no per-run cap, no inbox size limit and no snapshot garbage collection; capture tasks are spawned per search with no backpressure. A chat session or a hostile page that varies per request can fill the disk and swamp the review inbox.
**Fix:** Dedupe on `(source_url, content_hash)` regardless of origin when a URL exists; cap findings per capture call and total pending count (reject-and-log above a threshold); prune snapshots with no live finding row.

### WR-08: The "unprompted retrieval excludes Untrusted" invariant is enforced only in the MCP caller, not at the search layer

**File:** `crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs:330` vs `crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs:277-308`
**Issue:** `exclude_untrusted_hits` is set only inside `run_retrieval_bundle` as `trigger == AutoChatPreamble`. The orchestrator's goal-submit path calls `run_search_with_verification` with `RetrievalTriggerMode::AutoChatPreamble` and a plain `SearchPolicy::from_env()` (flag false). Legacy Untrusted web chunks and Untrusted knowledge nodes are therefore fed into `fused_text` and sent to the CRAG relevance LLM, and counted in the Socrates evidence totals. Impact is limited (the LLM only returns a relevance enum), but the invariant is brittle: any new `AutoChatPreamble` caller silently leaks.
**Fix:** Derive the flag where the trigger is consumed. In `execute_search_plan`, `let exclude = policy.exclude_untrusted_hits || trigger == AutoChatPreamble`, or set the flag in `goal.rs` and add a test.

### WR-09: Library chunks are stamped `Trusted` from the working tree, not from merged git content

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:156-160`, `crates/vox-search/src/library/mod.rs` (`index_library` walks `docs/src`), `crates/vox-db/src/store/ops_memory/library.rs:355-406`
**Issue:** `tier_for_curated(false)` is applied to every `docs/src/**/*.md` file on disk, and `put_kb_index_state` re-stamps every row of the corpus with that tier. Nothing checks `git ls-files` or that the file is committed. An agent write to `docs/src` (the edit and codegen tools can write there), or any untracked or uncommitted LLM output, becomes `Trusted` on the next `vox_kb_reindex`, and the unprompted preamble then injects it unlabelled as curated. This is the same elevation path the phase closes for the GUI and CLI publish writers, left open through the filesystem. The `corpus_commit` is recorded but not used to gate tiers.
**Fix:** Index only tracked, unmodified files as `Trusted` (for example `git ls-files -z docs/src` plus a clean-status check) and tier dirty or untracked docs `Advisory` or skip them. At minimum, record the dirty state in `kb_index_state` and surface it in `vox kb status`.

### WR-10: Model-callable memory store still elevates model and web output to `Advisory`

**File:** `crates/vox-orchestrator/src/memory/manager.rs:212,221`, `crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs:37`, `crates/vox-orchestrator-types/src/trust.rs:93-98`
**Issue:** `tier_for_knowledge_node_type` maps `fact` and `concept` to `Advisory`. `persist_fact`, reached from the model-callable memory-store tool, writes exactly those node types with arbitrary model-supplied content, including text the model just read from the web in the same turn. Those nodes then pass `exclude_untrusted_hits` (Advisory is not Untrusted) and are injected into the preamble as `[tier:advisory]`. This directly contradicts the D-06 taint rule the phase states ("an LLM synthesis of Untrusted input is Untrusted, never Advisory") and the TRUST-03 goal. Closing the chat write-back (`message.rs`) but leaving this tool open leaves a model-to-Advisory path.
**Fix:** Either write model-originated facts with a distinct `node_type` (for example `agent_fact`) that maps to `Untrusted`, or require provenance proving the content derived only from curated inputs before the `fact` node type is used. Add the node type to the `tier_for_knowledge_node_type` tests.

## Info

### IN-01: Finding reader does not clamp the tier to `FindingTier`

**File:** `crates/vox-db/src/store/ops_memory/kb_findings.rs:116`
**Issue:** The module promises a finding "can never be Trusted", but `finding_row` decodes the stored string with `ContextTrustTier::from_db_str`. A row written by raw SQL or a later migration with `trust_tier = 'trusted'` reads back as `Trusted`. Nothing consumes it yet, but Phase 9/12 readers will trust `KbFindingRow.trust_tier`.
**Fix:** `.min(ContextTrustTier::Advisory)` on read.

### IN-02: Deferred `legacy_export_covers_all_baseline_tables` is a real export gap

**File:** `crates/vox-db/src/codex_legacy.rs:~190` (see `deferred-items.md`, 03-03 and 03-12)
**Issue:** Not a Phase 3 regression, but the phase touched this list twice and recorded a red test as "deferred". `LEGACY_EXPORT_TABLES` omits `kb_index_state`, `provider_quota_usage`, `research_domain_reputation`, `research_misguidance_events` and `web_cache`, so a legacy export or import under the `legacy-import` feature drops them. If that lane is in the `ci-gate` quarantine it should be tracked there explicitly.
**Fix:** Add the five names in sorted position (or list them in the skip set) in this phase's closing plan; it is a five-line change.

### IN-03: `clean()` on Library tool output rewrites the text that `vox_kb_cite` and `vox_kb_get` return

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:46-60, 385-407`
**Issue:** Sanitizing every string leaf (including `body`, `quote` and `uri`) means a cited chunk can differ from the stored chunk whose `chunk_hash` it carries: images, HTML comments and `[INST]`-like tokens in curated docs are replaced. The citation then cannot be verified against the hash. This is an accepted trade-off for TRUST-05, but nothing flags the altered text to the caller.
**Fix:** Add a `"sanitized": true/false` flag (compare input and output once per leaf), or leave identifier and hash fields untouched.

### IN-04: Publish slug validation is narrower than the path it proposes

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/findings.rs:226-240`
**Issue:** The slug check rejects `/`, `\`, `..` and a leading `.`, but allows NUL, newlines, `:` and Windows reserved names, which end up in the stored `proposed_path`. Nothing writes the path yet, so this is latent. The Phase 9 promotion step must re-validate rather than trust the stored string.
**Fix:** Restrict to `[a-z0-9-]+` (the CLI's `render_published_doc` already normalizes) and re-validate at promotion time.

### IN-05: Store and DPO builder robustness

**File:** `crates/vox-db/src/store/ops_memory/kb_findings.rs:178-209,236-258`; `crates/vox-ml-cli/src/commands/mens/pipeline.rs:592-634`
**Issue:**
- A negative `limit` in `kb_findings_list` is unbounded in SQLite.
- One row with an unknown `origin` or `status` fails the whole list and `kb_findings_unqueued_reviewed`, which blocks the MENS stage for every other row.
- `kb_findings_dpo_records` treats any non-null `reviewed_by` as a human review, takes the cross product of every approved and rejected row for the same `query` (pairs from unrelated sources), and does not verify the `query` string is not itself attacker-influenced (it is the user's chat query, but it is unredacted; see WR-04).
**Fix:** Clamp `limit` to `1..=MAX`, skip-and-log undecodable rows, and pair on `(query, source_url host)` or require an explicit reviewer allowlist.

---

_Reviewed: 2026-10-08_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
