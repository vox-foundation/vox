---
phase: 03-provenance-trust-tiers-and-leak-closure
verified: 2026-10-08T00:00:00Z
status: gaps_found
score: 4/6 must-haves verified
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
  - crates/vox-db-types/src/kb_text.rs
  - crates/vox-db/src/store/ops_memory/kb_findings.rs
  - crates/vox-db/src/store/ops_memory/kb_legacy.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/findings.rs
  - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
  - crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs
  - crates/vox-orchestrator-types/src/trust.rs
  - crates/vox-search/src/bundle.rs
  - crates/vox-search/src/execution.rs
  - crates/vox-search/src/trust.rs
covered_digest: "v1:sha256:67dc1acc2b88f09d1f038aec172cbd0b12066912a2d7bc6cfce2be97c9238450"
behavior_unverified: 0
overrides_applied: 0
re_verification: null
gaps:
  - truth: "03-09 plan truth (D-03/D-05, TRUST-01): an unprompted chat preamble never carries an Untrusted hit, and the retrieval envelope tier is the minimum of the contributing hits"
    status: failed
    reason: "CR-01 (orchestrator-confirmed, re-confirmed by this verifier). The Tavily CRAG leg in run_search_with_verification bypasses exclude_untrusted_hits, adds no [tier:untrusted] label and never updates min_trust_tier, and message.rs injects bundle.rrf_fused_lines into the auto preamble."
    artifacts:
      - path: "crates/vox-search/src/bundle.rs"
        issue: "lines 195-236: t_lines are formatted as '[crag_tavily:<url>] ...' (no tier label) and extended into execution.web_lines and, when prefer_rrf_merge, execution.rrf_fused_lines unconditionally; policy.exclude_untrusted_hits is never read and execution.min_trust_tier is never lowered"
      - path: "crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs"
        issue: "lines 766-776 inject bundle.rrf_fused_lines under '[AUTONOMOUS RESEARCH — RRF FUSION]' for AutoChatPreamble; skip_preamble_web only clears web_research_enabled, not tavily_enabled"
    missing:
      - "Gate the Tavily firing (preferably) or the extend of web_lines/rrf_fused_lines on !policy.exclude_untrusted_hits"
      - "Label Tavily lines [tier:untrusted] and lower execution.min_trust_tier to Untrusted when they are admitted"
      - "A wiremock-backed Tavily test beside crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs proving the preamble drops Tavily lines and the envelope tier is Untrusted"
  - truth: "ROADMAP SC4: sanitizer neutralises tag-block / zero-width / bidi / HTML comments / off-allowlist images at BOTH the ingest and the injection boundary"
    status: partial
    reason: "Ingest boundary (findings clean(), capture) and the retrieval-bundle injection boundary (retrieval.rs sanitized()) are covered and tested. The quick-research source block is not fully covered (WR-05): Source.url and Source.engine are copied raw into the prompt-bound '[n] title — url (engine)' line and flatten() only strips newlines; pipeline_sources (deep path) sanitizes nothing. A URL carrying tag-block / zero-width characters reaches the prompt."
    artifacts:
      - path: "crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs"
        issue: "sources_from_hits (line ~183): url: h.path.clone(), engine unsanitized; sources_context_block only flatten()s; pipeline_sources unsanitized"
    missing:
      - "Wrap url and engine in SanitizedText::new(..).into_string() in sources_from_hits and run pipeline_sources through the same helper"
      - "Add a red-team case for a URL/engine field at the research-turn injection boundary"
human_verification:
  - test: "Hosted CI is green on this branch: nextest for vox-db-types, vox-db, vox-search, vox-orchestrator-types, vox-orchestrator, vox-orchestrator-mcp, vox-gui, vox-ml-cli; clippy -D warnings on the touched crates; vox ci ssot-drift; vox ci crate-edges; schema digest/baseline test for BASELINE_VERSION 96"
    expected: "All gates pass. The lean local policy ran only cargo check plus each plan's new tests; none of this was re-run by the verifier"
    why_human: "Verifier was told not to run builds on the slow host. The baseline-96 digest and crate-edges (03-01 recorded vox ci crate-edges as NOT RUN, stale binary) can only be confirmed by hosted CI"
  - test: "Playwright spec crates/vox-gui/ui/e2e/kb-inbox-side-doors.spec.ts plus vitest for MemoryView/DocPublishModal/ResearchView, and pnpm typecheck"
    expected: "Pass; screenshots land in crates/vox-gui/ui/review-bundle/latest/"
    why_human: "GUI Visual Verification Invariant; not run by the verifier"
  - test: "Decide WR-10: the model-callable memory-store tool (MemoryManager::persist_fact) still writes model- and web-derived text as knowledge_nodes of node_type 'fact'/'concept', which tier_for_knowledge_node_type maps to Advisory, so it passes exclude_untrusted_hits"
    expected: "Either accept as the D-06 allowlist (fact/concept = Advisory) or schedule an 'agent_fact' (Untrusted) node type. TRUST-03 text names only the 0.95 floor and the accepted=1 chat writeback, both removed, so this is recorded as an open design decision, not a failed criterion"
    why_human: "Product/security decision that contradicts the spirit of the D-06 taint rule"
  - test: "Decide WR-09: Library chunks are stamped Trusted for every docs/src/**/*.md on disk (tier_for_curated(false)), with no git tracked/clean check, whereas TRUST-01 defines Trusted as 'curated (merged to git)'"
    expected: "Accept as a known AP-12 assumption or schedule git-tracked/clean gating in a later phase (Phase 5 reindex or Phase 9 promotion)"
    why_human: "Mapping-fidelity decision; the 'merged to git' half of the TRUST-01 definition is not mechanically enforced"
---

# Phase 3: Provenance, Trust Tiers and Leak Closure Verification Report

**Phase Goal:** Every piece of KB content carries a trust tier and provenance. The existing unreviewed paths into the corpus are closed. All KB retrieval runs through one `vox-search` path, so later phases build on one sink and one retrieval path.
**Verified:** 2026-10-08
**Status:** gaps_found
**Re-verification:** No, initial verification

Method: code read and grep against the actual tree. SUMMARY claims were used only as pointers. Per the lean policy no build or test was run by the verifier; "VERIFIED" below means the code, the test, and the recorded mutation proof exist and are consistent, and hosted CI remains the authority for green runs (see Human Verification).

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: tier required on index/finding writes; four-way `ContextTrustTier` mapping; fail-closed, mutation-proven promotion verdict | VERIFIED | `crates/vox-orchestrator-types/src/trust.rs`: single `ContextTrustTier` enum (ascending trust order), no `Default` (`compile_fail` doctest), `from_db_str` fails closed to Untrusted, `FindingTier` has only Untrusted/Advisory, `tier_for_knowledge_node_type` allowlists only fact/concept. `upsert_search_document(.., tier: ContextTrustTier)` has a required tier parameter (`vox-db/.../search.rs:57-64`); `vox-db/tests/tiered_writer_invariant.rs` pins the only three files allowed to `INSERT INTO search_documents / kb_findings`. `kb_findings.trust_tier` and `search_documents.trust_tier` exist; BASELINE_VERSION 96. `promotion_eligibility` (`vox-search/src/trust.rs:276`) is an exhaustive match: Unavailable and Retracted give Ineligible, Clean and NotApplicable give Eligible. 03-01-SUMMARY records two mutation proofs (Unavailable arm to Eligible makes `promotion_eligibility_fails_closed` red; NotApplicable arm to Ineligible makes the open-case test red). Caveat: no production consumer yet (Phase 9 owns promotion); see WR-01 below |
| 2 | SC2: web research turn adds zero corpus rows; creates Untrusted pending findings with summary/quotes/URL/fetch date/hash; snapshot gitignored | VERIFIED | `kb_findings` schema has `summary, quotes_json, source_url, fetched_at_ms, content_hash, snapshot_relpath, trust_tier DEFAULT 'untrusted'` (`schema/domains/knowledge.rs:115-134`). Web legs go to `WebFindingCapture` sink (`bundle.rs:204-223`, `capture.rs`); `grep` shows the only non-test callers of `upsert_search_document` are `vox-db/src/research.rs`, `questioning_telemetry.rs` and `vox-search/src/ingest.rs`/library, none on the web path. `crates/vox-search/tests/web_capture_test.rs` asserts `capture_sink_receives_hit_and_corpus_stays_empty` and `capture_crag_tavily_reaches_sink_not_corpus`. Snapshot path is `.vox/cache/kb-findings/<hash>.txt`; `git check-ignore -v` confirms it matches `.gitignore:111 **/.vox/cache/`. Note the Tavily leg still adds web text to the in-memory preamble lines (that is the CR-01 gap in truth 6, not a corpus write) |
| 3 | SC3: no accepted=1 / 0.95 writes; `publish_research_doc`, GUI `ingest_url`/`ingest_text`, scientia/MEMORY sinks only produce pending findings; MENS `KbSignals` reads `kb_findings` | VERIFIED | `grep` for `0.95` / `accepted=1` writes and for `signal_research|signal_chat|signal_scientia` in non-test source: none (names appear only in the `legacy_voxkb_retired.rs` ban list). `message.rs` has no KB write-back (leak tests `kb_leak_chat_turn_writes_no_kb_entry_or_finding`, `kb_leak_at_mention_injects_nothing`). GUI `ingest_text`/`ingest_url` call `file_gui_finding(..)` (`vox-gui/src/commands/knowledge.rs`), GUI `publish_research_doc` calls `file_publish_draft` and writes nothing under `docs/src` (`commands/research.rs:180-204`); `vox research publish` writes a gitignored drafts dir, `docs/src` only via reviewed PR (`vox-cli-research/src/lib.rs:128,488-492`). Quick/deep research synthesis and sources use `capture_finding` (`research_turn.rs:503-580`). MENS `run_kb_signals_stage` reads reviewed `kb_findings` and `kb_findings_dpo_records` emits DPO only, requiring `reviewed_by` (`mens/pipeline.rs:592-660`). Open design question WR-10 is listed under Human Verification |
| 4 | SC4: sanitizer property tests + red-team neutralise tag block, zero-width/bidi, HTML comments, off-allowlist images at ingest AND injection | PARTIAL (gap) | `vox-db-types/src/kb_text.rs` `is_banned` covers E0000-E007F, 200B-200F, 2060-2064, FEFF, 202A-202E, 2066-2069, FE00-FE0F, E0100-E01EF; proptest suite plus `redteam_cases_are_neutralized` over `contracts/kb/redteam/cases.v1.jsonl` (ASCII-escaped), tag-block mutation proof recorded (03-02-SUMMARY). Ingest: findings `clean()` and `CaptureInput`. Injection: `retrieval.rs sanitized()` wraps every bundle line group (`:413-417`) and the Library tool outputs. NOT covered: research-turn `Source.url`/`engine` and the deep-path `pipeline_sources` (WR-05), see gap 2 |
| 5 | SC5: `KbStore` / `KbRouter` / signal adapters / `kb_lines` / 12 `vox_kb_*` tools absent from source and catalog, names replaced not aliased; seeded-DB migration test: legacy rows counted then migrated as pending | VERIFIED | `grep` for `KbStore|KbRouter|kb_lines|KbSignalAdapter` in `crates/**/*.rs`: no hits outside the ban list. `contracts/mcp/tool-registry.canonical.yaml` lists only `vox_kb_cite/get/reindex/search/status` (the Phase 2 Library tools; `vox_kb_search` and `vox_kb_get` are the "colliding names replaced"); none of the 12 legacy ids appear in the catalog, capability registry, http-read-role governance or model manifest. `legacy_voxkb_retired.rs` ban list covers tools, catalog ids, adapters, store, router, types and CRUD fn names, mutation-proven in 03-12. Legacy tables survive only as `SCHEMA_LEGACY_VOXKB` quarantine DDL and read-only `kb_legacy.rs` readers; `migrate_legacy_kb_entries` (`kb_tools/findings.rs:303`) counts first, skips empty, dedupes per legacy id, and is wired at DB attach (`server_state.rs:663`). `tests/kb_legacy_migration.rs` covers counts-then-migrate-as-pending-Untrusted, rows-survive, idempotence, absent tables, attach in background. Orphan-row edge case: WR-06 (advisory) |
| 6 | 03-09 plan truth (D-03/D-05, TRUST-01): the unprompted preamble never carries an Untrusted hit; envelope tier = min of contributing hits, never a blanket Trusted stamp | FAILED | CR-01 re-confirmed end to end: `bundle.rs:195-236` Tavily leg ignores `exclude_untrusted_hits`, adds no tier label and does not touch `min_trust_tier`; `message.rs:766-776` injects `rrf_fused_lines` into the auto preamble. `execution.rs` correctly gates the other legs (`:407, :472, :604, :643`), so only the Tavily post-pass is open. `tavily_enabled` defaults false (`policy.rs:306`), so the hole is reachable only when Tavily is configured and fires (fire_on_empty or fire_on_weak); no test exercises it |

**Score:** 4/6 truths verified (SC1, SC2, SC3, SC5), 0 present-but-behavior-unverified. SC4 is partial and truth 6 failed.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/vox-orchestrator-types/src/trust.rs` | Tier vocabulary, FindingTier, taint mapping | VERIFIED | Substantive, re-exported through `vox_db` and `vox_orchestrator` |
| `crates/vox-db-types/src/kb_text.rs` | One sanitizer, `SanitizedText`, finding enums | VERIFIED | 22 test fns plus proptest and red-team |
| `crates/vox-db/src/store/ops_memory/kb_findings.rs` | Typed inbox writer/reader | VERIFIED (WR-04, IN-01, IN-05 hardening notes) | |
| `crates/vox-search/src/capture.rs` + bundle sink | Web hits to inbox | VERIFIED for the web leg; Tavily leg also reaches the sink | |
| `crates/vox-search/src/trust.rs` promotion verdict | Fail-closed | VERIFIED | Retraction check semantics doubtful (WR-01) |
| `crates/vox-orchestrator-mcp/tests/{kb_findings_capture,kb_findings_gui,kb_legacy_migration,kb_retrieval_tiers,kb_status_inbox,legacy_voxkb_retired}.rs` | Phase tests | VERIFIED present | `kb_retrieval_tiers` lacks a Tavily case |
| Deleted legacy files (KbStore/KbRouter/adapters, 12 tools) | Absent | VERIFIED | `git ls-files` shows none; no dangling references |

### Key Link Verification

| From | To | Status | Details |
|------|----|--------|---------|
| search web legs to inbox | `McpFindingCapture` -> `capture_finding` -> `insert_pending_finding` | WIRED | `web_capture_for_state` injected in `run_retrieval_bundle` |
| research turns to inbox | `capture_research_sources` / `capture_research_synthesis` | WIRED | |
| GUI side doors to inbox | `file_gui_finding` / `file_publish_draft` | WIRED | |
| DB attach to legacy migration | `server_state.rs:663` -> `migrate_legacy_kb_entries` | WIRED (MCP server attach only) | GUI/CLI attach paths do not run it; acceptable since the MCP server is the daemon |
| MENS KbSignals to `kb_findings` | `run_kb_signals_stage` | WIRED | |
| preamble to tier filter | `exclude_untrusted_hits` set in `run_retrieval_bundle` for AutoChatPreamble | PARTIAL | Honoured by `execute_search_plan`, bypassed by the Tavily post-pass (CR-01); not enforced for the orchestrator `goal.rs` caller (WR-08) |

### Requirements Coverage

All 9 phase IDs are claimed by at least one PLAN `requirements:` field and by SUMMARY `requirements-completed`, and are marked `[x]` / Complete in `.planning/workstreams/kb/REQUIREMENTS.md`. No orphaned requirement.

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| TRUST-01 | 03-01, 03-03, 03-05, 03-09, 03-13 | SATISFIED with caveats | Required tier and mapping verified (SC1). Caveats: "Trusted = merged to git" not mechanically enforced (WR-09, human decision); envelope min-tier violated on the Tavily path (gap 1) |
| TRUST-02 | 03-04, 03-06, 03-11 | SATISFIED | Web hits to inbox, zero corpus rows (SC2) |
| TRUST-03 | 03-06, 03-07 | SATISFIED | 0.95 floor and chat write-back removed (SC3); WR-10 open design question |
| TRUST-04 | 03-03, 03-04, 03-06 | SATISFIED | Provenance columns plus gitignored snapshot. Advisory: snapshot is raw text and hash covers untruncated text (WR-03) |
| TRUST-05 | 03-02, 03-04, 03-09 | PARTIALLY SATISFIED | Gap 2 (research-turn URL/engine, deep sources) |
| TRUST-06 | 03-01 | SATISFIED | Mutation-proven fail-closed verdict. Advisory: Crossref field choice (WR-01) must be fixed before Phase 9 wires a consumer |
| RET-02 | 03-07, 03-09, 03-10, 03-12, 03-13 | SATISFIED | SC5; single `vox-search` path, though the Tavily post-pass is a tier-bypass on that path (gap 1) |
| RET-03 | 03-03, 03-08, 03-12, 03-13 | SATISFIED | Counted then migrated; orphan-entry edge case WR-06 |
| GROW-06 | 03-06, 03-07, 03-08, 03-11 | SATISFIED | SC3 |

### Disposition of the Code Review Findings (03-REVIEW.md)

| Finding | Disposition | Reason |
|---------|-------------|--------|
| CR-01 Tavily bypass | **GOAL GAP (blocker, gap 1)** | Violates the binding 03-09 truth, TRUST-01's min-tier semantics, and the phase goal's "one retrieval path" tier guarantee. Opt-in feature but reachable; no test |
| WR-05 research-turn URL/engine unsanitized | **GOAL GAP (partial SC4, gap 2)** | SC4 names the injection boundary explicitly; cheap fix |
| WR-08 exclude flag only set in MCP caller | Advisory, fix together with CR-01 | `goal.rs` path feeds only a CRAG relevance enum and evidence counts; recommended fix is to derive the flag from `trigger == AutoChatPreamble` in `execute_search_plan`, which also hardens CR-01's neighbours |
| WR-09 Library Trusted from working tree | Human decision (listed above) | Documented AP-12 assumption; mapping text says "merged to git" |
| WR-10 `persist_fact` Advisory elevation | Human decision (listed above) | D-06 explicitly allowlists fact/concept; TRUST-03 text names two other mechanisms |
| WR-01 Crossref `update-to` vs `updated-by` | Advisory, must fix before the first promotion consumer (Phase 9) | Verdict function is correct and fail-closed; the pre-existing detection is fail-open and its tests share the same wrong assumption |
| WR-02 redact-then-sanitize order | Advisory (security hardening) | Not part of SC4 (secret redaction, not injection neutralisation); one-line reorder recommended |
| WR-03 snapshot raw + hash/truncation mismatch | Advisory | SC2 requires a gitignored snapshot and a content hash, both present; the module doc overstates sanitization |
| WR-04 untyped provenance JSON | Advisory | Hardening; reviewed-only consumers today |
| WR-06 orphaned legacy entries / abort on first error | Advisory | Narrow edge case (FK cascade exists in the DDL); worth an assertion `migrated+skipped+already_present+failed == kb_entries` |
| WR-07 unbounded inbox/snapshot growth | Advisory (follow-up) | Not in any SC; Phase 9 triage/GC is the natural owner |
| IN-01..IN-05 | Info | IN-02 (`legacy_export_covers_all_baseline_tables` red, pre-existing, recorded in `deferred-items.md`) should be given an owner |

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/vox-search/src/bundle.rs` | 195-236 | Tier bypass on live-web text | BLOCKER | CR-01 |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs` | ~183-230 | Unsanitized URL/engine into prompt | WARNING | WR-05 |
| Debt markers (TBD/FIXME/XXX) | n/a | Not scanned exhaustively; review found none | n/a | n/a |

## Human Verification Required

See the `human_verification` frontmatter: hosted-CI gates (digest, crate-edges, clippy, nextest), Playwright/vitest GUI suite, and the two design decisions WR-09 and WR-10.

## Gaps Summary

The leak-closure and retirement work is real and consistent with the code: the tier vocabulary, required-tier writers with a pinning test, the findings inbox, web/research/GUI/CLI side doors routed to pending Untrusted findings, the 0.95 and `accepted=1` writes gone, MENS reading reviewed findings as DPO only, the legacy VoxKB surface deleted and guarded by a static ban test, and count-then-migrate wired at attach. SC1, SC2, SC3 and SC5 hold.

Two gaps keep the phase from `passed`:

1. **CR-01 (blocker).** The Tavily post-pass in `run_search_with_verification` is the one retrieval leg that does not honour the Untrusted-drop, tier label or min-tier rule, so the "unprompted preamble never carries Untrusted web text; envelope tier = min of hits" guarantee (plan 03-09) is false whenever Tavily is configured and fires. Small fix: gate on `!policy.exclude_untrusted_hits`, label `[tier:untrusted]`, lower `min_trust_tier`, add a mocked-Tavily test.
2. **WR-05 (partial SC4).** The injection-boundary sanitization misses the research-turn source URL and engine and the deep-path sources. Two-line fix plus a red-team case.

Both are small, localized closures suitable for `/gsd-plan-phase --gaps`. WR-08 should be folded into the same plan. Nothing else in the review changes the verdict for SC1-SC3 and SC5.

---

_Verified: 2026-10-08_
_Verifier: Claude (gsd-verifier)_
