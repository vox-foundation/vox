# Research Trial Flywheel — Knowledge Persistence Research

Investigated at `d6a72ea70` (`feat/research-trial-flywheel`). Siblings: `TRIAL-DESIGN.md`, `QUALITY-EVAL.md`.

## 1. Existing surfaces (what already exists, what it does NOT give us)

**Schema / baseline**
- `BASELINE_VERSION = 94` (`crates/vox-db/src/schema/manifest.rs:35`); changelog comments 77–94 at `:11-34`. The contract
  matches (`contracts/db/baseline-version-policy.yaml:7`, digest `:36`). `origin/main` is also at 94. I checked all
  281 local and remote refs plus every `.claude/worktrees/*` checkout: none is at ≥95 (five worktrees are at 93, one is at 94). **95 is free today.**
- Migration is a single baseline, not a delta chain (`crates/vox-db/src/store/open.rs:93-130`): the baseline DDL runs only
  when `MAX(schema_version) < BASELINE_VERSION` (`:126`). A database whose version is higher than the binary's fails hard with `LegacySchemaChain` (`:114-123`).
  Columns added to an existing table need hand-written idempotent `ALTER TABLE` statements (`:137-181`).
- No SQL `CHECK` constraints or triggers in the baseline. Enums are validated in Rust (`scientia.rs:351-354`, `:380`).
  FTS triggers exist only in `schema_extensions.rs:86-114` via `exec_optional_batch`, which *warns and skips* when it fails (`:21-25`).

**Knowledge domain** (`crates/vox-db/src/schema/domains/knowledge.rs`)
- `knowledge_nodes` (`:3-12`) and `knowledge_edges` (`:14-21`): an untyped graph; `id` is the only key; no provenance, hash, status, or expiry; metadata is free JSON.
- `search_documents` (`:46-56`, unique on `source_uri`, has `content_hash`) and `search_document_chunks` (`:58-66`, `UNIQUE(document_id, chunk_index)`); FTS shadow in `schema_extensions.rs:87-112`.
- VoxKB tables `knowledge_bases`, `kb_entries` (`accepted`, `mens_queued`), and `kb_routing_rules` (`:79-116`, baseline 77); also `web_cache` (`:119-132`, baseline 91).

**Scientia domain** (`crates/vox-db/src/schema/domains/scientia.rs`)
- `scientia_research_sessions` (`:176-187`) and `scientia_research_artifacts` (`:190-196`); `scientia_claims` (`:199-210`, `claim_id` = FNV-1a of the text) and `scientia_claim_verdicts` (`:213-224`, verdict + confidence + span).
- `scientia_finding_candidates` (`:250-275`), `scientia_discoveries` (`:4-25`, human gate), `research_eval_runs`/`_samples` (`:93-121`), and `research_misguidance_events` (`:438-458`).
- Append-only ledger pattern to copy: `scientia_review_decisions` (`:340-356`, INSERT/SELECT only, latest by timestamp wins).
- `scientia_prereg` is quarantined behind the `quarantine` feature, which is off by default
  (`crates/vox-db/src/schema/domains/quarantine.rs:588-603`). **Do not touch it.** It already has a `supersedes_id`, which is a design hint, not a table to reuse.
- `harness_eval_run` (`crates/vox-db/src/schema/domains/harness_eval.rs:7`) has `run_id` and `config_version` columns but no knowledge-manifest column.

**What the new knowledge-base work (`c7ad5c490`, `b120286c5`) provides**
- `crates/vox-db/src/store/ops_memory/knowledge.rs`: upsert, edges, neighbors, recursive CTE reachability (`:117`), list/query/delete (`:205-360`), health counts (`:369`).
- `crates/vox-gui/src/commands/knowledge.rs`: `ingest_text` → node `doc:{hash}` (`:81-117`); `ingest_url` (robots-checked) → node `web:{hash}` (`:171-228`); `save_research_session_to_kb` (`:253-298`) → `persist_research_findings_to_knowledgebase` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs:507-577`).
- Gaps: everything lands in `knowledge_nodes` and nothing writes `search_documents`; there is no verification gate (unverified snippets stored as `research_src:*`); only the top 5 sources are kept (`research_turn.rs:541`); synthesis and edge errors are discarded (`:529-538`, `:570-572`); there is no campaign, config, or expiry linkage.

**Memory** (`crates/vox-orchestrator/src/memory/manager.rs`)
- `persist_fact` (`:155-257`): writes MEMORY.md synchronously (`:166`), then runs a **fire-and-forget `tokio::spawn`** (`:180`) that
  writes `memories`, the knowledge node, edges, and the embedding. Failures are only logged (`:203-237`), so callers get `Ok(())` before any DB write happens.
- `lookup_fact_by_key` (`:263-292`): checks cache, then MEMORY.md, then the newest **500** `memories` rows (`:276`), and swallows DB errors (`:278`).
  An older fact is invisible even though it is persisted.
- `persist_campaign_fact/hypothesis/contradiction` (`:395-428`) build keys from `self.cache.len()`. The cache is capped at
  `cache_limit` (`:253-254`), so **keys collide and overwrite** across a long campaign.
- `sync_verified_research_findings` (`:650-706`): the DB write is `let _ = …` inside a spawn (`:698-702`), so errors are fully silent.

**Search**
- `execute_search_plan` (`crates/vox-search/src/execution.rs:303`) queries `SearchCorpus::DocumentChunks` through
  `query_search_document_chunks_hybrid` (`crates/vox-db/src/store/ops_memory/search.rs:140`). That uses FTS, with a fallback to LIKE (`:12-38`).
  **There is no scoping filter** (no doc-id, source-uri, or hash allowlist), so it searches the entire corpus.
- `vox db mirror-search-corpus` → `vox_search::ingest_markdown_tree` (`crates/vox-search/src/ingest.rs:41-82`) walks every `.md` file.
  **It has no `archive/` exclusion.** Pointing it at `docs/src` would ingest `docs/src/archive/`. It also re-chunks unchanged files
  (no hash short-circuit) and never deletes rows for removed files.
- `vox db retrieval-status` (`crates/vox-cli/src/commands/db_research/retrieval.rs:12-25`, built on `crates/vox-db/src/research.rs:488-540`) reports only
  embeddings, knowledge_nodes, and knowledge_edges counts, plus placeholder latency and mode values. **It cannot verify a search_documents/chunks mirror** in its current form.
- `ingest.rs:17` and `research.rs:13` call `blake3` directly. That violates the crypto policy, which requires `vox_crypto::hash_fast_hex`
  (`crates/vox-crypto/src/facades.rs:41`). New code must use the facade.

## 2. Proposed Tier A decomposition (scientia fragment; requires explicit human schema approval)

All tables use TEXT ids, `*_at_ms` INTEGER timestamps, Rust-validated enums, and **append-only status** (status changes are new rows, never in-place UPDATEs, following the `scientia_review_decisions` pattern).
- `research_campaigns`: `campaign_id` PK, `title`, `question`, `config_id` (FK), `created_by`, `created_at_ms`, `closed_at_ms`, `outcome` (open|succeeded|failed|abandoned).
- `research_configs`: `config_id` PK = content hash of the canonical config JSON (model, policy, tools, seed), plus `config_json` and `created_at_ms`.
  Content addressing makes "same config" checkable.
- `research_findings` (authoritative row): `finding_id` PK = hash(`campaign_id`, normalized claim, `content_hash`); FKs `campaign_id`, `config_id`, `session_id` (→ `scientia_research_sessions`), `claim_id` (→ `scientia_claims`); `kind` (claim|negative|method|failed_arm); `statement`; `content_hash`; `verdict`; `confidence` REAL; `provenance_json` (agent, model fingerprint, tool trace ref); `verified_at_ms`, `expires_at_ms`, `revalidate_after_ms`, `created_at_ms`.
- `research_finding_citations`: (`finding_id`, `ordinal`) PK, `url`, `source_content_hash`, `span_text`, `retrieved_at_ms`, `web_cache_url_hash`.
- `research_finding_status_events` (append-only): `finding_id`, `status` (verified|superseded|retracted|expired|revalidated|contested),
  `superseded_by` (nullable FK), `reason`, `actor`, `at_ms`. Current status = latest event. Supersession is a forward pointer, never a delete.
- `research_projection_state`: (`finding_id`, `target` ∈ memory|search|kg) PK, `state` (pending|ok|failed|stale), `attempts`,
  `last_error`, `projected_hash`, `updated_at_ms`. This is the reconciliation ledger (§3).
- `research_knowledge_manifests`: `manifest_id` PK = hash of the sorted entry list, `ablation` (empty|current|stale_conflicting|curated_verified),
  `created_at_ms`, `entry_count`, `signed_by`. Plus `research_knowledge_manifest_entries` (`manifest_id`, `finding_id`, `content_hash`) PK on the first two.
- Indexes: findings (`campaign_id`, `created_at_ms`), (`content_hash`), (`expires_at_ms`); status events (`finding_id`, `at_ms`); projection state (`state`, `updated_at_ms`).
- Contract work: add a fragment to `contracts/db/`, and add entries to `contracts/db/retention-policy.yaml`. Findings and manifests are `keep_forever`;
  status events are append-only (see SSOT F78, `docs/src/architecture/data-storage-ssot-2026.md:273`). Then bump to 95 and update the digest.

**Schema-approval questions for the user**
1. Should these go in the existing `scientia` fragment (no new domain), or in a new `research_knowledge` domain (a new active Tier A domain, needing explicit approval)? *Recommendation: scientia.*
2. Should `research_findings` link to `scientia_claims`/`scientia_claim_verdicts` by FK, or copy verdict and confidence (denormalized) to freeze them at verification time?
3. Expiry default: one global TTL, or per-`kind`/per-domain (e.g. 30 days for web facts, never for methods)? Does an expired finding get excluded from `current` manifests, or included and flagged?
4. Is the manifest a Tier A row set (proposed), or a Tier C content-addressed blob with a Tier A pointer row (`sha256`, `size_bytes`; SSOT §4.3 `:80`)?
5. Should `failed_arm` and negative findings share the findings table (proposed) or get a separate table?
6. `vox-db` is a frozen core crate (`contracts/db/data-storage-policy.v1.yaml:142-143`). Who signs off on the governance for this Tier A change?

## 3. Projection and reconciliation (memory, search, knowledge graph)

- **The write order is fixed:** (1) a transactional Tier A insert of finding + citations + a `verified` status event + `pending` rows in
  `research_projection_state`; (2) only after that commits, run the projectors. The API returns `FindingId` from step 1. Projection outcome is reported separately and never stands in for step 1.
- Memory projector: call `persist_fact(key = "finding:{finding_id}")`. Because `persist_fact` is fire-and-forget, the projector must *not*
  mark `ok` on its return. Add an awaited variant (e.g. `persist_fact_awaited -> Result<ProjectionReceipt>`), or have the
  reconciler confirm by reading back `memories`. Do **not** route through `persist_campaign_*` (key collision, `manager.rs:402`).
- Search projector: one `search_documents` row per finding, `source_uri = "vox-finding:{finding_id}"`, `content_hash` = finding hash;
  chunks via `replace_search_document_chunks_with_refs`. Approved authored research is mirrored with `source_uri = "vox-docs:<rel>"`
  through an `ingest_markdown_tree` change that **skips any path segment `archive`** and short-circuits when the hash is unchanged.
- Reconciler (`vox db research reconcile`, idempotent): for every currently-verified finding, check each target's `projected_hash` equals `content_hash`; retry failed or pending projections with backoff; mark projections of superseded, retracted, or expired findings `stale` and delete their search rows.
- Extend `retrieval_diagnostics` (`research.rs:488`) so `vox db retrieval-status` really verifies the mirror: `search_documents`/`search_document_chunks` counts, chunk-FTS readiness, `findings_verified`, `projections_{pending,failed,stale}`, and `archive_uri_count` (must be 0).

## 4. Frozen knowledge manifest and the contamination boundary

- At arm signing: choose the ablation, select finding ids by a deterministic query against Tier A *as of* `signed_at_ms` (status events with `at_ms <= signed_at_ms`), then sort, hash, and persist. The arm record (`harness_eval_run` or a trial-arm row) stores `knowledge_manifest_id`.
- At run time, retrieval is **allowlist-scoped**: add `KnowledgeScope { manifest_id }` to `execute_search_plan` and `query_search_document_chunks_hybrid`; filter `source_uri IN (vox-finding:<manifest ids>) ∪ (approved docs snapshot)`; drop any row whose `content_hash` drifted since signing.
- Inside an arm, disable or scope everything else: the knowledge-node FTS lane (`execution.rs:~420-450`), `lookup_fact_by_key` and MEMORY.md bootstrap context (`manager.rs:532`), `web_cache` entries fetched after `signed_at_ms`, and `kb_entries`. The memory manager needs a "manifest mode" that returns only manifest facts.
- Write isolation: findings produced *during* an arm get written with `campaign_id` = that arm and are excluded from every manifest signed before
  the arm ends. Evaluation gold answers and task prompts must never be ingested as findings: block them by a hash denylist of the eval set, which is checked in the Tier A insert path.

## 5. Ablation measurement design

- Four arms per task batch, identical `config_id` and seed, varying only `knowledge_manifest_id`: **empty** (zero entries); **current** (verified, unexpired findings as of signing); **stale_conflicting** (current plus deliberately expired, superseded, or contradicted findings, tagged so harm is measurable); **curated_verified** (human-approved subset).
- Metrics per arm (reusing `research_eval_samples`, `scientia.rs:105-118`): recall@5, groundedness, quality, latency, cost (via `agent_telemetry_flat`); *knowledge-use rate* (citations whose `finding_id` is in the manifest); *stale-adoption rate* (answers asserting a superseded or contradicted finding).
- Comparison: paired, per-task deltas against **empty**, with a bootstrap CI; report "curated − current" and "stale − current" as the harm signal.
  Failed arms persist as `kind = failed_arm` findings, so a null or negative result stays searchable.

## 6. Discriminating red tests (each must fail on current code)

1. `finding_persist_is_authoritative_before_projection`: insert a finding with the memory projector forced to fail. The Tier A row exists, the projection state is `failed`, and the API returns `Ok(finding_id)`. **Fails today:** there is no table, and `persist_fact` returns `Ok` with no DB row.
2. `lookup_fact_by_key_finds_fact_older_than_500_rows`: write 501 facts and look up the first. **Fails today** (`manager.rs:276`).
3. `campaign_fact_keys_do_not_collide_after_cache_cap`: write more than `cache_limit` + 1 campaign facts; every one is distinct in MEMORY.md. **Fails today** (`:402`, `:253`).
4. `mirror_search_corpus_skips_archive`: mirror a tree that contains `archive/x.md`; zero `source_uri` values contain `/archive/`. **Fails today** (`ingest.rs:50-58`).
5. `retrieval_status_reports_chunk_counts_and_pending_projections`. **Fails today** (`research.rs:488`).
6. `scoped_chunk_search_excludes_out_of_manifest_docs`: two docs with the same term, manifest holds one, only one hit comes back. **Fails today** (no scope parameter).
7. `manifest_hash_is_order_independent_and_hash_drift_drops_row`: mutate a finding's content after signing; the arm does not see the new content.
8. `superseded_finding_absent_from_current_manifest_present_in_stale_arm`.
9. `eval_gold_hash_rejected_at_finding_insert`.
10. `scientia_prereg_untouched`: the default baseline still lacks it, and its quarantine DDL bytes are unchanged (digest pin on that slice).
11. `baseline_95_creates_research_tables_on_db_at_94` and `db_stamped_95_without_tables_is_detected` (see the 95 collision pitfall in §7).

Verify each guard by mutation, per AGENTS.md: remove the guard, watch the test fail, then restore it.

## 7. Pitfalls

- **Fire-and-forget writes:** `persist_fact` (`manager.rs:180`), `sync_verified_research_findings` (`:698-702`, `let _`), `log` (`:130`), and the KB persist helper (`research_turn.rs:529`, `:570`) all drop errors. The process can exit before the spawned task runs (common in the CLI), and `tokio::spawn` panics outside a runtime. Never count these as proof of a write.
- **Baseline collisions with parallel agents.** Six worktrees are on 93/94, and the `~/.vox` DB is shared. If another branch stamps 95 first with different DDL, this branch sees `current == BASELINE`, skips the baseline (`open.rs:126`), and **silently never creates its tables**; if another reaches 96, this binary hard-fails (`:114`). Mitigations: re-check that 95 is free right before merge; test on ephemeral DBs (`VOX_DB_URL` = temp file); add a post-migrate table probe (red test 11); update `manifest.rs:35`, its changelog comment, and `baseline-version-policy.yaml:7,36` together (`vox ci check-codex-ssot`).
- **`CREATE TABLE IF NOT EXISTS` never changes existing tables.** Adding columns to `knowledge_nodes` or `search_documents` instead of new tables needs explicit `ALTER TABLE` code (`open.rs:137-181`). Prefer new tables.
- **No CHECK constraints or triggers** in the baseline; Turso rejects triggers (`scientia.rs:351-354`). Append-only and enum validity live in the Rust ops boundary only, so there must be no UPDATE or DELETE ops on the ledgers.
- **Avoid partial or secondary indexes on nullable columns that flip off NULL.** They trip a Turso `IdxDelete` bug (`scientia.rs:334-337`). The `expires_at_ms` and `closed_at_ms` indexes need care, or must be avoided.
- **FTS readiness is optional and silently skipped** (`schema_extensions.rs:21-25`). Searches then fall back to LIKE, which changes retrieval quality between environments: a confound across arms. Record the FTS mode in the arm config hash.
- **Archive leakage:** `ingest_markdown_tree` has no exclusion. Mirroring `docs/src` ingests the tombstoned archive.
- **Re-ingest churn:** unchanged docs get re-chunked, so FTS rowids churn and stored chunk ids in manifests break. Key manifests on `finding_id` + `content_hash`, never on chunk rowid.
- **The `claim_id` FNV-1a hash** (`scientia.rs:201`) is non-cryptographic and collision-prone. Do not use it as the finding identity; use `vox_crypto`.
- **`scientia_finding_candidates` / `scientia_discoveries` look similar.** Do not overload them: they are publication-pipeline-bound with a different lifecycle. Link by optional FK only.

## Summary
- Authoritative Tier A finding storage does not exist. The knowledge-base work writes untyped `knowledge_nodes` without verification, provenance, or expiry, and drops errors.
- Proposed new scientia tables at baseline 95 (currently free on all refs and worktrees): campaigns, configs, findings, citations, append-only status events (supersession and expiry), the projection-state reconciliation ledger, and manifests plus entries.
- Memory and search are projections. `persist_fact` is fire-and-forget and `lookup_fact_by_key` only scans 500 rows, so the reconciler compares `projected_hash` to `content_hash`. `retrieval-status` must gain chunk and projection counts to verify the mirror at all.
- Trials need a content-addressed manifest per arm and allowlist-scoped retrieval (chunk search has no scope filter today), with the knowledge-graph FTS lane, MEMORY.md, KB entries, and post-signing `web_cache` disabled inside arms.
- The highest-risk pitfalls are an archive leak via `ingest_markdown_tree`, a silent skip of baseline 95 on a shared DB stamped by a parallel branch, and campaign key collisions in `MemoryManager`.
