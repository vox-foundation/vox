---
phase: "03"
slug: "provenance-trust-tiers-and-leak-closure"
workstream: kb
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
threats_total: 63
threats_closed: 63
threats_accepted: 5
threats_transferred: 1
residual_open_non_blocking: 1
register_authored_at_plan_time: true
created: "2026-10-10"
---

# Phase 03 - Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Verification method: read-only code inspection and grep at ASVS L1 depth (mitigation present at the cited
> boundary). No build or test was run by the auditor; recorded mutation proofs in the SUMMARY files were
> treated as pointers, not as evidence. All paths are relative to the repo root; `crates/` prefix elided in the
> evidence column where unambiguous (e.g. `vox-search/src/trust.rs` = `crates/vox-search/src/trust.rs`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| stored tier strings -> Rust | a corrupted or unknown tier string must not elevate trust | `trust_tier` TEXT, `node_type` TEXT |
| Crossref API -> promotion verdict | network errors must not yield an eligible verdict | retraction lookup result |
| web or user text -> KB text types | untrusted strings become `SanitizedText` only through `sanitize` | page text, titles, URLs, pasted text |
| red-team fixtures -> agents, IDE tooling, index | injection payloads must not reach agents that read the repo or the index | `contracts/kb/redteam/cases.v1.jsonl` |
| application code -> `search_documents` / `kb_findings` | the only place a tier can be attached to stored text | tier, text, provenance |
| older vox binary or raw SQL -> database | rows written without the Rust writer | rows with no tier |
| web providers (SearXNG, DDG, Tavily) -> vox-search -> capture sink -> `kb_findings` | the only path web text may take into storage | attacker-controlled page text |
| capture -> local filesystem | content-addressed snapshot under `.vox/cache/kb-findings/` | full unredacted page text |
| Library indexer / filesystem `docs/src` -> Library rows | curated tier must come from one decision (git-tracked and unmodified) | doc text, tier |
| web pages -> deep-research gather; LLM synthesis -> storage | model output derived from untrusted input | synthesis text |
| research pipeline and CLI -> `docs/src` | curated tree indexed as Trusted | drafted research docs |
| assistant output -> KB storage; legacy KB rows -> chat prompt | model output must not re-enter retrieval | chat turns, legacy `kb_entries` |
| legacy `kb_entries` -> `kb_findings`; `kb_findings` -> MENS training mix | migrated or unreviewed text must not gain trust or become training data | legacy rows, DPO records |
| stored rows -> chat preamble; retrieval results and KB tool output -> any model | automatic, unprompted context injection | retrieved lines, tool JSON |
| `kb_findings` -> retrieval | the inbox must never be a context source | pending findings |
| MCP clients -> KB write tools | model-callable writes into a retrievable store | tool calls |
| GUI commands (ingest URL/text, publish) | arbitrary or hostile pasted/scraped text; LLM-drafted doc -> `docs/src` | text, slug |
| upgraded user DBs -> fresh baseline; future changes -> retired store | legacy data must survive the DDL move; LIKE-search store must not return | legacy tables |
| status payload -> operators / HTTP; docs and registries -> future contributors | counts must be live and path-free over HTTP; retired surfaces must not be reintroduced | counts, host paths |
| live web (Tavily) -> chat preamble; goal submit -> retrieval; search result fields -> research prompt block | unprompted injection of third-party text; attacker-controlled url/engine/title/snippet in a model prompt | web hit fields |
| model tool call (`memory_store`) -> knowledge graph -> chat preamble; pre-fix graph rows -> retrieval | model-supplied (possibly web-derived) text persisted as graph rows | graph rows |
| agent memory files (MEMORY.md, daily logs) -> chat preamble | out of scope for Phase 3 by D-07a (transferred, see T-03-54) | memory lines |
| orchestrator process -> git | subprocess invocation for the Library tier gate | `ls-tree` / `diff` output |

---

## Threat Register

Severity from the plan-time register (`register_authored_at_plan_time: true`). Threat IDs reused across plans
are merged (T-03-05, 07, 11, 12, 18, 35); the plan numbers are in the Component column.

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence) | Status |
|-----------|----------|-----------|----------|-------------|-----------------------|--------|
| T-03-01 | Tampering | `kb_text::sanitize` codepoint pass (03-02) | high | mitigate | `is_banned` covers tag block, zero-width, invisible operators, bidi, variation selectors (`vox-db-types/src/kb_text.rs:11-21`), applied first in every pass (`:69`); proptest oracle restates the ranges independently (`:704-754`) | closed |
| T-03-02 | Information disclosure | `kb_text::sanitize` image pass (03-02) | high | mitigate | empty `IMAGE_HOST_ALLOWLIST` (`kb_text.rs:40`); `neutralize_images` handles inline, reference and HTML images (`:114-125`); only scheme-less, host-less targets survive (`:215-238`) | closed |
| T-03-03 | Tampering | `kb_text::sanitize` comment pass (03-02) | high | mitigate | `strip_comments` strips unterminated comments to end (`kb_text.rs:77-89`); bounded fixpoint, `MAX_PASSES=8`, fails closed to empty (`:44-66`); idempotence and no-`<!--` proptests (`:743-754`) | closed |
| T-03-04 | Elevation of privilege | `promotion_eligibility` (03-01) | high | mitigate | exhaustive `match` with no wildcard; `Unavailable` and `Retracted` -> `Ineligible`; `NotApplicable` distinct (`vox-search/src/trust.rs:260-286`); failed Crossref call maps to `Unavailable` (`:91-97`); tests `:443-463` | closed |
| T-03-05 | Tampering | `ContextTrustTier` / `FindingTier` / `NewKbFinding` / `upsert_search_document` (03-01, 03-03, 03-05) | high | mitigate | no `Default` (`compile_fail` doctest `vox-orchestrator-types/src/trust.rs:16-18`); `FindingTier` has no `Trusted` (`:62-68`); fail-closed `from_db_str` (`:52-59`); `NewKbFinding{tier: Trusted}` compile_fail (`vox-db/src/store/ops_memory/kb_findings.rs:15-33`), text fields `SanitizedText` (`:54-67`); `upsert_search_document` requires tier, compile_fail/no_run pair (`ops_memory/search.rs:41-64`) | closed |
| T-03-06 | Repudiation | `kb_findings` provenance (03-03) | medium | mitigate | summary, quotes_json, source_url, fetched_at_ms, blake3 content_hash, provenance_json columns (`vox-db/src/schema/domains/knowledge.rs:115-134`); stable `ORDER BY created_at_ms DESC, id ASC` (`ops_memory/kb_findings.rs:178-185`) | closed |
| T-03-07 | Tampering | `execute_search_plan` web leg, CRAG Tavily branch, deep-research gather (03-04, 03-06) | high | mitigate | web hits go only to the injected `web_capture` sink, no sink means no write (`vox-search/src/execution.rs:613-647`); Tavily hits likewise (`vox-search/src/bundle.rs:223-242`); gather (`vox-research-shim/src/research/orchestrator/web_gather.rs:194-239`); no other `INSERT INTO search_documents/kb_findings` writer (`vox-db/tests/tiered_writer_invariant.rs:9-13,68-80`); caller sweep of `upsert_search_document` finds only Untrusted/Advisory/Library writers (`vox-db/src/research.rs:124-131`, `questioning_telemetry.rs:87-94`) | closed |
| T-03-08 | Information disclosure | `capture_finding` summary/quotes (03-04) | medium | mitigate | `clean()` = `redact_owned` then `SanitizedText` (`vox-orchestrator-mcp/src/kb_tools/findings.rs:91-92`); summary/quotes path redacts before cap (`:143-147`); single chokepoint for every capture path. Declared control present at L1. Order-of-operations weakness tracked as RV-01 below | closed |
| T-03-09 | Tampering | snapshot writer (03-04) | medium | mitigate | filename is the content hash only, never URL-derived (`findings.rs:43-46`, hash from `vox_crypto` `:95-97`); `atomic_write_secure` (`:99-112`); `MAX_SNAPSHOT_BYTES` = 2 MiB enforced by `truncate_bytes` (`:18,:101`) | closed |
| T-03-10 | Denial of service | inbox flood from preamble re-searches (03-04) | medium | mitigate | `dedupe_key TEXT NOT NULL UNIQUE` (`schema/domains/knowledge.rs:117`); `ON CONFLICT(dedupe_key) DO NOTHING` (`ops_memory/kb_findings.rs:157`); key from origin + scope + content hash (`findings.rs:136-140`). Growth from unique-content pages is deferred WR-07 (see Unregistered Flags) | closed |
| T-03-11 | Tampering | deep synthesis persistence (03-06); post-turn writeback and signal adapters (03-07) | high | mitigate | synthesis filed as ONE pending Untrusted finding, no confidence field (`vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs:562-590`, test asserts no `confidence` `:1865` and no knowledge-node rows `:1814-1870`); writeback and `signal_chat/signal_research/signal_scientia/ingest_chat_turn` absent from `crates/` (grep empty; `tests/legacy_voxkb_retired.rs:178-210`); turn writes no KB row or finding (`chat_tools/chat/message_kb_leak_tests.rs:140-166`) | closed |
| T-03-12 | Tampering | `pipeline.rs` branch (j), `vox research publish`, GUI `publish_research_doc` (03-06, 03-11) | high | mitigate | docs-writing persistence removed (`vox-research-shim/src/research/persistence.rs:1-2`); no `fs::write`/`File::create` in the shim outside sandbox, codegen and tests (grep); CLI writes gitignored `.vox/cache/research-drafts` (`vox-cli-research/src/lib.rs:495-497`, `.gitignore:109`) with slug check (`:451-453`); GUI publish files a pending finding and writes nothing under `docs/src` (`vox-gui/src/commands/research.rs:176-204`) | closed |
| T-03-13 | Tampering | `run_kb_signals_stage` (03-08) | high | mitigate | SQL filter `reviewed_by IS NOT NULL AND status IN ('approved','rejected') AND mens_queued = 0` (`vox-db/src/store/ops_memory/kb_findings.rs:238-251`); pure re-check filtering rows on `reviewed_by.is_some()` (`vox-ml-cli/src/commands/mens/pipeline.rs:603`); DPO records only, `type != sft` asserted (`:597-634`, test `:801`) | closed |
| T-03-14 | Elevation of privilege | `migrate_legacy_kb_entries` (03-08) | medium | mitigate | every legacy row goes through `capture_finding`, which hardcodes `FindingTier::Untrusted` (`findings.rs:160`) and status `pending` (`ops_memory/kb_findings.rs:156`); legacy `accepted` is copied into provenance only (`findings.rs:346`); sanitized and redacted by the same chokepoint | closed |
| T-03-15 | Elevation of privilege | AutoChatPreamble retrieval, envelope trust stamp (03-09) | high | mitigate | Untrusted hits dropped under `exclude_untrusted_hits` for graph, chunk, ANN and web legs (`vox-search/src/execution.rs:407,472,604-608,643-647`); flag set from trigger (`vox-orchestrator-mcp/src/memory_tools/retrieval.rs:330`) and again in `run_search_with_verification` (`vox-search/src/bundle.rs:35-45`); envelope tier = min of hits, `Untrusted` when none (`retrieval.rs:155`, `execution.rs:384-386`) | closed |
| T-03-16 | Tampering | `run_retrieval_bundle`, `sources_from_hits`, KB tool output (03-09) | high | mitigate | `sanitized()` on every bundle line field (`memory_tools/retrieval.rs:297-302,413-417`); `clean()` on every string leaf of KB tool results, `ok_clean` at each return (`kb_tools/library.rs:49-58,108,293,381,401,461,521,534`); `sanitized_source` for research sources (`research_turn.rs:188,195-205,703`); red-team tests per boundary (`tests/kb_retrieval_tiers.rs:294,357`) | closed |
| T-03-17 | Elevation of privilege | `vox_kb_clip`, `vox_kb_add_entry` model-callable writes (03-10) | high | mitigate | banned names absent from `crates/` and contract files (grep: only retired-symbol ledgers mention them); absence test with whole-identifier matcher and non-vacuity floor (`tests/legacy_voxkb_retired.rs:15-72,84-85,137-210`); HTTP read role lists only search/get/cite (`contracts/mcp/http-read-role-governance.yaml:16-18`) | closed |
| T-03-18 | Denial of service | external MCP callers of removed tool names (03-10, 03-13) | low | accept | accepted per RET-02/D-02, CHANGELOG names removed tools and replacements (`CHANGELOG.md:26,32`); see AR-03 | closed (accepted) |
| T-03-19 | Tampering | `contracts/kb/redteam/cases.v1.jsonl` (03-02) | medium | mitigate | file has 0 non-ASCII bytes (checked), asserted by `kb_text.rs:615` (`redteam_cases_file_is_ascii`); path in `.voxignore:140` (derived `.cursorignore:135`, `.aiignore:136`, `.aiexclude:136`); roots-disjoint canary check (`vox-cli/src/commands/kb/gate.rs:33-39,186`) | closed |
| T-03-20 | Elevation of privilege | `search_documents.trust_tier` column (03-03) | high | mitigate | `NOT NULL DEFAULT 'untrusted'` in baseline DDL (`schema/domains/knowledge.rs:60`) and in the v95 ALTER (`store/open.rs:243-247`); backfill canonicalizes invalid strings via `from_db_str` (`open.rs:286-316`, `:302`), keeps the empty (web/legacy) corpus untrusted (`:299`); tests `local_tests.rs:1120,1155,1165`. Upgrade-path residual: AR-08 | closed |
| T-03-21 | Tampering | a new corpus or inbox writer added later (03-05) | medium | mitigate | `search_corpus_and_inbox_have_only_tiered_writers` with allowlist of 3 writer files and `scanned > 50` non-vacuity floor (`vox-db/tests/tiered_writer_invariant.rs:9-13,68-80`) | closed |
| T-03-22 | Denial of service | migration on large user DBs (03-03) | low | accept | one UPDATE per corpus plus one prefix UPDATE, once, inside the version gate (`open.rs:286-316`); see AR-01 | closed (accepted) |
| T-03-23 | Tampering | stored prompt injection in captured text (03-04) | high | mitigate | `SanitizedText` on title, summary, quotes, source_url before insert (`findings.rs:143-152,165`; `ops_memory/kb_findings.rs:59-62`); red-team asserted at ingest (`tests/kb_findings_capture.rs:154`); rows `pending`; no retrieval or injection code reads `kb_findings` (T-03-44) | closed |
| T-03-24 | Repudiation | swallowed capture errors (03-04) | low | mitigate | `tracing::warn!` on every capture failure (`findings.rs:382`; `research_turn.rs:556,588`); no `let _ =` in `findings.rs` (grep empty) | closed |
| T-03-25 | Repudiation | research findings provenance (03-06) | medium | mitigate | provenance carries query, engine/rank (`research_turn.rs:551`) or source URL list (`:580-583`), gather and web captures carry engine + query (`findings.rs:379`); content hash plus snapshot (`findings.rs:133-134,155-156`) | closed |
| T-03-26 | Tampering | KB-lines enrichment and `@kb` dump into prompts (03-07) | high | mitigate | `kb_lines`, `kb_mention_lines`, `[MENTIONED KNOWLEDGE BASES]` and the KB header absent from non-test code (grep: hits only inside the leak test's assertions); prompt-body tests on the stubbed model request (`message_kb_leak_tests.rs:184-210`); `@kb` inert (`CHANGELOG.md:49`) | closed |
| T-03-27 | Tampering | MEMORY.md research sync S11 (03-07) | medium | mitigate | `sync_verified_research_findings` and its test absent (grep empty across `crates/`) | closed |
| T-03-28 | Availability | users relying on `@kb` buckets (03-07) | low | accept | D-05 accepted regression until Phase 7 (`03-CONTEXT.md` D-05); `CHANGELOG.md:49`; see AR-02 | closed (accepted) |
| T-03-29 | Denial of service | legacy migration at startup (03-08) | low | mitigate | spawned task, errors `warn!`, never fatal (`vox-orchestrator-mcp/src/server_state.rs:660-669`); paged `LEGACY_PAGE=500` (`findings.rs:297,323`); idempotent per legacy id via `dedupe_scope` (`:339`) | closed |
| T-03-30 | Repudiation | legacy provenance loss (03-08) | low | mitigate | provenance copies legacy_id, kb_name, source_signal, source_ref, routing_confidence, accepted, mens_queued, created_at_ms (`findings.rs:340-349`) | closed |
| T-03-31 | Tampering | GUI `ingest_url` / `ingest_text` S5/S6 (03-11) | high | mitigate | both commands call only `file_gui_finding` (`vox-gui/src/commands/knowledge.rs:8,104,208`); helper goes through `capture_finding` and never touches `knowledge_nodes` (`findings.rs:201-235`) | closed |
| T-03-32 | Spoofing | toast copy implying publication (03-11) | low | mitigate | Playwright asserts no `/Published/` and the "Filed for review" / "Already pending review" copy (`vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts:54-55,64,75,87,107-110`) | closed |
| T-03-33 | Information disclosure | markdown-image exfil in retrieved text (03-09) | high | mitigate | `sanitize` (which runs `neutralize_images`, `kb_text.rs:70`) applied at the injection chokepoints listed under T-03-16 | closed |
| T-03-34 | Repudiation | unlabelled hits (03-09) | low | mitigate | `[tier:..]` label on every KG line and chunk line (`vox-search/src/execution.rs:422,486`) and on Tavily lines (`bundle.rs:217`) | closed |
| T-03-35 | Tampering | resurrected legacy surface or retired symbols via docs/agent prompts (03-10, 03-12, 03-13) | medium | mitigate | `legacy_voxkb_surface_absent` over `crates/` plus 6 contract files with `MIN_FILES_SCANNED=100` (`tests/legacy_voxkb_retired.rs:15-87,177-210`); BANNED extended to `KbStore/KbRouter/CRUD` (`:44-71`); `contracts/documentation/retired-symbols.v1.yaml:23`; `contracts/retirement/retired-surfaces.v1.yaml:115-117`; `AGENTS.md` Retired Surfaces row | closed |
| T-03-36 | Information loss | legacy rows on upgraded DBs, DDL move (03-12) | medium | mitigate | DDL moved to `schema/domains/quarantine.rs:723-783`; the three tables are not in `QUARANTINE_DROP_TABLES` (`vox-db/src/migration.rs:139`, grep); manifest tests (`schema/manifest.rs:245-267`); rows already copied by the 03-08 migration (`findings.rs:297-352`) | closed |
| T-03-37 | Elevation of privilege | Library tier drift between `kb_index_state` and rows (03-05) | medium | mitigate | plan-time control (row realignment in `put_kb_index_state`) was deliberately superseded by 03-16 (D-08). Current state: `put_kb_index_state` writes the manifest only and never UPDATEs `search_documents` (`ops_memory/library.rs:375-420`); row tier set per file and never above the ceiling `deps.trust_tier` (`vox-search/src/library/mod.rs:100-141`). Threat vector covered by T-03-58/59/60 | closed |
| T-03-38 | Repudiation | silent legacy data after quarantine (03-13) | low | mitigate | status counts read live from the DB; failure returns `err_json`, never a zero (`kb_tools/library.rs:475-496,505-508`); `legacy_kb_counts` propagates errors, zero only when a table is absent (`vox-db/.../kb_legacy.rs:44-63`) | closed |
| T-03-39 | Spoofing | `tier_for_knowledge_node_type` (03-01) | medium | mitigate | allowlist of `fact`/`concept` Advisory, every other or missing type `Untrusted` (`vox-orchestrator-types/src/trust.rs:104-109`); tier derived from `node_type` at read time, missing row -> `Untrusted` (`vox-db/src/store/ops_memory/knowledge.rs:203-221`) | closed |
| T-03-40 | Spoofing | `SanitizedText` (03-02) | medium | mitigate | private field; no `Default`, `From<String>` or `Deserialize`; compile_fail doctest (`kb_text.rs:307-338`) | closed |
| T-03-41 | Information loss | legacy and web rows on upgrade, D-03 re-tier only (03-03) | medium | mitigate | migration issues only `UPDATE`s (`open.rs:286-316`; the only `DROP` in `open.rs` is the unrelated reset helper at `:50-68`); row-count-unchanged test (`local_tests.rs:1120-1133`, 6 rows) | closed |
| T-03-42 | Information disclosure | full-text snapshots (03-04) | medium | mitigate | `.vox/cache/` gitignored (`.gitignore:109,111`), `.voxignore:142`; no non-test reader of the snapshot dir (only `research_turn.rs:1748`, inside `#[cfg(test)]` which starts at `:896`); none in `vox-search/src` | closed |
| T-03-43 | Information loss | legacy rows on user DBs (03-08) | medium | mitigate | legacy access module is read-only, no DELETE/UPDATE (`vox-db/src/store/ops_memory/kb_legacy.rs:1-104`); migration "reads legacy tables only" (`findings.rs:297-301`) | closed |
| T-03-44 | Elevation of privilege | inbox findings as context (03-09) | high | mitigate | no retrieval code reads `kb_findings`: only comments in `vox-search` (`capture.rs`, `policy.rs`), counts in `kb_tools/library.rs:485`, reviewed-only MENS export, tests; `kb_tiers_findings_never_retrieved` and `kb_tiers_cache_nodes_and_findings_are_never_retrieved` (`tests/kb_retrieval_tiers.rs:172,191`) | closed |
| T-03-45 | Tampering | publish slug path traversal (03-11) | medium | mitigate | `file_publish_draft` rejects empty, `/`, `\`, `..`, leading `.` (`findings.rs:247-254`); test over 6 slugs and no `docs/src` created (`tests/kb_findings_gui.rs:151-160`) | closed |
| T-03-46 | Information disclosure | status payload over HTTP (03-13) | low | mitigate | `vox_kb_status` absent from the HTTP read-role list (`contracts/mcp/http-read-role-governance.yaml:16-18`), asserted by `http_gateway/mod.rs:751-757`; added objects are counts only (`kb_tools/library.rs:475-496`) | closed |
| T-03-47 | Elevation of privilege | Tavily CRAG leg under AutoChatPreamble (03-14) | high | mitigate | under `exclude_untrusted_hits` the block only pushes `tavily_skipped=untrusted_excluded`: no client, no credit, no capture (`vox-search/src/bundle.rs:162-166`); flag derived from the trigger (`:35-45`); tests `tests/web_capture_test.rs`, `vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs` | closed |
| T-03-48 | Elevation of privilege | `goal.rs` AutoChatPreamble caller with a plain policy (03-14) | medium | mitigate | exclusion derived inside `run_search_with_verification` for every caller (`bundle.rs:35-45`, shadowed `policy` feeds both passes and the diagnostics) | closed |
| T-03-49 | Spoofing | explicit-query Tavily lines indistinguishable from curated lines (03-14) | low | mitigate | lines prefixed `[tier:untrusted]` (`bundle.rs:217`) and `min_trust_tier = Some(Untrusted)` (`:254`) | closed |
| T-03-50 | Tampering | research-source url/engine/title/snippet in the prompt block (03-14) | high | mitigate | one constructor `sanitized_source` sanitizes all four fields, sanitize-then-cap (`research_turn.rs:195-205`), used by `sources_from_hits` (`:188`) and `pipeline_sources` (`:703`); remaining raw `Source {..}` literals are inside `#[cfg(test)]` (see IN-02 in Unregistered Flags); red-team tests `:1444,1477` | closed |
| T-03-51 | Denial of service | Tavily credit spend on every auto-preamble turn (03-14) | low | mitigate | gate precedes client build and `try_consume` (`bundle.rs:162-196`) | closed |
| T-03-52 | Information disclosure | test env var `TAVILY_API_KEY` leaking between tests (03-14) | low | accept | fake `tvly-test` value against loopback wiremock; see AR-04 (parallel env mutation hardening is review IN-05) | closed (accepted) |
| T-03-53 | Elevation of privilege | `persist_fact` graph rows typed fact/concept (03-15) | high | mitigate | `persist_fact` writes `agent_fact` / `agent_concept` (`vox-orchestrator/src/memory/manager.rs:207-240`), both Untrusted via the default arm (`vox-orchestrator-types/src/trust.rs:94-109`); e2e from `memory_store` to preamble (`tests/kb_retrieval_tiers.rs:393-496`); writer sweep: no other model-reachable `upsert_knowledge_node` produces `fact`/`concept` | closed |
| T-03-54 | Elevation of privilege | agent memory files served by the memory leg as Advisory preamble lines (03-15) | high | transfer | user decision D-07a (2026-10-09): documented in `03-CONTEXT.md` (D-07a), `deferred-items.md` (owner: Phase 7 spotlighting), `docs/src/architecture/search-retrieval-ssot-2026.md:116,123,134`, `CHANGELOG.md:47`; current behavior pinned by the e2e assertion (`tests/kb_retrieval_tiers.rs:393-496`); see AR-06 | closed (transfer documented) |
| T-03-55 | Elevation of privilege | legacy fact/concept graph rows written before the fix (03-15) | medium | mitigate | one-time re-type inside the baseline-96 gate (`store/open.rs:210,318-337`); upgrade test (`local_tests.rs:1188-1235`). Residual for DBs already at 96: AR-07 | closed |
| T-03-56 | Tampering | re-type migration corrupting or deleting knowledge rows (03-15) | low | mitigate | parameterized `UPDATE` only, `graphify:%` ids excluded (`open.rs:329-334`); test asserts all four rows survive (`local_tests.rs:1236`) | closed |
| T-03-58 | Elevation of privilege | `reindex_at` stamping untracked/staged/modified `docs/src` Trusted (03-16) | high | mitigate | `committed_clean_docs` = `ls-tree -r HEAD` minus `diff HEAD` over `KB_CORPUS_ROOTS` (`kb_tools/library.rs:217-267`); any git error -> empty set -> all Untrusted, `GitExec::run` returns `Err` on non-zero exit (`git_exec.rs:103-109`); sole production `IndexDeps` constructor is `reindex_at` (`library.rs:159-164`; other constructors are tests and the in-memory eval harness `vox-search/src/kb_eval/library.rs:58-64`); per-file tier in `vox-search/src/library/mod.rs:100-105` | closed |
| T-03-59 | Elevation of privilege | `put_kb_index_state` corpus-wide re-stamp (03-16) | high | mitigate | re-stamp removed, manifest-only write (`vox-db/src/store/ops_memory/library.rs:375-420`, no UPDATE of `search_documents`) | closed |
| T-03-60 | Elevation of privilege | hash-skip path keeping a stale Trusted tier (03-16) | medium | mitigate | skip path rewrites the tier (`vox-search/src/library/mod.rs:131-141` via `set_library_document_tier`, `ops_memory/library.rs:285-301`); unreadable docs demoted to Untrusted (`mod.rs:109-121`) | closed |
| T-03-61 | Tampering | git subprocess (03-16) | low | mitigate | only through `GitExec` (denylist, telemetry; `git_exec.rs:79-115`); fixed read-only args `ls-tree`/`diff`, paths are the `KB_CORPUS_ROOTS` constants, no caller-supplied path (`library.rs:221-250`) | closed |
| T-03-62 | Tampering | fixture tests running git init/commit (03-16) | low | mitigate | fixture refuses when `GIT_DIR`/`GIT_WORK_TREE` is set, uses a tempdir, disables hooks (`core.hooksPath=/dev/null`) and signing (`kb_tools/library.rs:1055-1082`) | closed |
| T-03-63 | Repudiation | operator cannot see why a doc left chat context (03-16) | low | mitigate | `docs_untrusted` in the reindex report (`kb_tools/library.rs:196`); SSOT documents the D-08 rule (`search-retrieval-ssot-2026.md:115,125`) | closed |
| T-03-SC | Tampering | dev-dependency `proptest` (03-02) | low | accept | already locked at 1.11.0 and used by vox-orchestrator; no new package (03-RESEARCH Package Legitimacy Audit); see AR-05 | closed (accepted) |

*Status: open | closed | open - below high threshold (non-blocking)*
*Severity: critical > high > medium > low. Only open threats at or above `block_on: high` count toward `threats_open`.*
*Disposition: mitigate (implementation required) | accept (documented risk) | transfer (documented hand-off).*

Totals (plan-time register): 63 unique threat IDs = 57 mitigate + 5 accept + 1 transfer. 63 closed, 0 open-blocking,
0 open-non-blocking within the plan register.

---

## Open Residual (non-blocking, review-derived)

Not a plan-register threat; surfaced by `03-REVIEW.md` / `deferred-items.md` and verified in code. Linked to T-03-08.

| ID | Category | Component | Severity | Status | Detail |
|----|----------|-----------|----------|--------|--------|
| RV-01 | Information disclosure | `findings.rs` `clean()` and summary path: redact-before-sanitize (deferred WR-02 of the pre-gap review) | medium | open - below high threshold (non-blocking) | `redact_owned` runs on the raw string (`vox-orchestrator-mcp/src/kb_tools/findings.rs:91-92,143-147`) and `SanitizedText::new` strips zero-width/tag codepoints afterwards. A secret split by an invisible codepoint (e.g. `ghp_abcdefghij<U+200B>klmnopqrstuvwxyz`) does not match the 20+ contiguous-alnum token regexes in `vox-redact/src/lib.rs:25,47-64`, then is rejoined in plaintext by the sanitizer and stored in `summary`/`quotes`/`title`/`source_url`. Severity rationale: impact is a secret in a local DB row (also exportable as a DPO `chosen`/`rejected` string once a human approves it, `pipeline.rs:597-634`); the raw text already sits unredacted in the gitignored snapshot by design, which limits marginal exposure; likelihood is low (needs a secret in the captured text plus deliberate or accidental invisible-character splitting, e.g. prompt-injection asking the model to emit a key with zero-width spaces). Not high. Fix is a one-line reorder (`redact_owned(&sanitize(raw))`, sanitize is idempotent and does not touch `[REDACTED_*]`) at the two call sites; recommended before ship, not required by the gate. |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-03-22 | Baseline-96 migration runs one UPDATE per corpus plus one prefix UPDATE, once, inside the version gate; DoS on a very large DB is a bounded one-time cost | plan-time acceptance (03-03-PLAN threat_model); confirmed by gsd-security-auditor | 2026-10-10 |
| AR-02 | T-03-28 | Users relying on `@kb` buckets lose them until the Phase 7 remap (interim regression D-05); CHANGELOG entry present (`CHANGELOG.md:49`) | user decision D-05 (`03-CONTEXT.md`, recorded 2026-10-04) | 2026-10-04 |
| AR-03 | T-03-18 | External MCP callers of the 12 removed `vox_kb_*` names get unknown-tool; no alias per RET-02/D-02; CHANGELOG names replacements (`CHANGELOG.md:26,32`) | user decision D-02 (`03-CONTEXT.md`) | 2026-10-04 |
| AR-04 | T-03-52 | Test-only env var carries fake `tvly-test` key against loopback wiremock; MCP case is a single test in its own binary. (Parallel-env-mutation hardening, review IN-05, stays deferred) | plan-time acceptance (03-14-PLAN) | 2026-10-10 |
| AR-05 | T-03-SC | `proptest` already locked at 1.11.0 and used elsewhere; no new package installed | plan-time acceptance (03-02-PLAN, 03-RESEARCH Package Legitimacy Audit) | 2026-10-10 |
| AR-06 | T-03-54 | Memory leg (MEMORY.md and daily logs, written by model-callable `memory_store`/`memory_log`) keeps feeding the preamble as Advisory, including model text that was copied from the web in the same turn. Hand-off to Phase 7 spotlighting; owner Phase 7; pinned by the e2e assertion so the hand-off is explicit. Severity stays high, disposition is transfer, so it does not count toward `threats_open`; review IN-03 | user decision D-07a (`03-CONTEXT.md`) | 2026-10-09 |
| AR-07 | T-03-55 (review WR-01) | The legacy `fact`/`concept` re-type runs only when a DB is below baseline 96, so a local DB already recorded at 96 keeps Advisory legacy agent rows. Pre-release dev-database window: baseline 96 first appears in commit `80cf75eda`, present on no remote branch. All forward writes are Untrusted and the tier is derived at read time. Optional fix: run `retier_agent_knowledge_nodes` on every open outside the version gate | user (accepted as pre-release residual, relayed by orchestrator) | 2026-10-10 |
| AR-08 | T-03-20 / T-03-58 (review WR-02) | The baseline-96 backfill still stamps Library rows with the `kb_index_state` corpus ceiling (`store/open.rs:286-312`), so an already-indexed Library on an upgraded DB keeps Trusted on untracked or modified docs until the next `vox_kb_reindex` (which re-gates every doc). Pre-release dev-database window as AR-07; fail-open only until the first reindex. Optional fix: stamp Library rows `untrusted` in the backfill. The doc comment at `open.rs:281-285` is stale (still says "single corpus-level source") | user (accepted as pre-release residual, relayed by orchestrator) | 2026-10-10 |

*Accepted risks do not resurface in future audit runs.*

---

## Unregistered Flags

SUMMARY `## Threat Flags`: all 16 SUMMARY files report "None" for new attack surface. One informational note maps to an
existing threat: 03-04-SUMMARY says the "snapshot never sent to a model" half of the privacy rule has no automated check
(maps to T-03-42). Auditor grep found no non-test reader of `.vox/cache/kb-findings/` (T-03-42 closed); a test would
make it permanent.

Review-derived, deferred-by-plan items with no threat-ID mapping (all non-blocking, none counted in `threats_open`;
none re-verified beyond reading the cited code):

| Ref | Severity | Item |
|-----|----------|------|
| WR-03 (old) | low | findings snapshot is raw text and its hash covers the untruncated text (`findings.rs:95-112,133`) |
| WR-04 (old) | low | `kb_findings.provenance_json` is unvalidated JSON that bypasses `SanitizedText` (`findings.rs:169`; only `query` is read downstream, by the MENS DPO grouping `pipeline.rs:603-609`) |
| WR-07 (old) | low | inbox and snapshot growth unbounded; dedupe key includes content hash so a page that varies per fetch defeats T-03-10 dedupe at up to 2 MiB per capture; owner Phase 9 (triage/GC) |
| IN-01 | low | git gate fails closed silently (debug log only) and is blind to `assume-unchanged`/`skip-worktree` flags (`kb_tools/library.rs:258-262`) |
| IN-02 | low | "only way to build a numbered `Source`" is convention: public fields, GUI `save_research_session_to_kb` builds `Source` directly but reaches only `capture_research_sources` -> sanitizing `clean()` (no live prompt leak) |
| IN-04 | low | `knowledge_nodes` Advisory arm for `fact`/`concept` and the DDL default `node_type DEFAULT 'concept'` rest on writer convention; no static writer-invariant test for `upsert_knowledge_node` |
| OBS-A | informational | Out of Phase 3 scope, noted in passing: the repo, symbol-proximity and graphify legs call `note_tier(.., Trusted)` for working-tree content with no git gate (`vox-search/src/execution.rs:540-542,753-755`). Same class as D-08 but not in this phase's register; candidate for Phase 7 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open (blocking) | Open (non-blocking) | Accepted | Transferred | Run By |
|------------|---------------|--------|-----------------|---------------------|----------|-------------|--------|
| 2026-10-10 | 63 | 63 | 0 | 0 in plan register; 1 review-derived residual (RV-01, medium) | 5 (plus 3 review/decision entries AR-06..08) | 1 | gsd-security-auditor (Sonnet 5.5), ASVS L1, block_on high |

Method notes: implementation files read-only; no builds or tests run; mutation proofs recorded in SUMMARY files were not
re-executed. T-03-37 closed on the superseding 03-16 mechanism, not the plan-time one. WR-01/WR-02 (current review) are
logged as accepted risks per the orchestrator's relay of the user decision, not re-litigated.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-10 (gsd-security-auditor). One non-blocking residual (RV-01, medium) remains open and is recommended for a one-line fix before ship.
