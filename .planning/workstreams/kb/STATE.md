---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Vox Knowledge Base — corpus to context
current_phase: 5
current_phase_name: Local Embeddings and Ranking Signals
status: planning
stopped_at: Phase 04 complete, ready to plan Phase 5
last_updated: "2026-10-12T16:02:36.478Z"
last_activity: 2026-10-12
last_activity_desc: Phase 04 complete, transitioned to Phase 5
state_head: e6bbad320d0cb30cbebedb4c59cae946a5ee4fb3
progress:
  total_phases: 12
  completed_phases: 4
  total_plans: 48
  completed_plans: 48
  percent: 33
---

# Project State

## Project Reference

See: .planning/PROJECT.md (shared project context) and .planning/workstreams/kb/REQUIREMENTS.md (this workstream's scope)

**Core value:** Agents using any model get the right curated knowledge at the right time — measurably, visibly, and without letting unreviewed or model-generated text masquerade as trusted fact.
**Current focus:** Phase 04 — Taxonomy Contract and Facet Lint

## Current Position

Phase: 5 — Local Embeddings and Ranking Signals
Plan: Not started
Status: Ready to plan
Last activity: 2026-10-12 — Phase 04 complete, transitioned to Phase 5

Progress: [███░░░░░░░] 33%

## Performance Metrics

**Velocity:**

- Total plans completed: 48
- Average duration: - min
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 | 12 | - | - |
| 02 | 12 | - | - |
| 03 | 16 | - | - |
| 04 | 8 | - | - |

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 1 P01 | 60min | 3 tasks | 15 files |
| Phase 1 P02 | 15m | 2 tasks | 7 files |
| Phase 1 P01-03 | 20m | 2 tasks | 2 files |
| Phase 1 P05 | 35min | 3 tasks | 4 files |
| Phase 1 P06 | 25min | 2 tasks | 11 files |
| Phase 1 P12 | 15min | 2 tasks | 6 files |
| Phase 1 P07 | 45min | 2 tasks | 4 files |
| Phase 1 P04 | ~1h | 3 tasks | 3 files |
| Phase 1 P08 | 35min | 2 tasks | 2 files |
| Phase 1 P09 | 50min | 2 tasks | 2 files |
| Phase 1 P10 | 25min | 2 tasks | 6 files |
| Phase 1 P11 | 45m | 2 tasks | 5 files |
| Phase 02 P10 | 25m | 3 tasks | 5 files |
| Phase 02 P11 | 25min | 2 tasks | 6 files |
| Phase 02 P12 | 10min | 3 tasks | 5 files |
| Phase 03 P01 | 16min | 2 tasks | 4 files |
| Phase 03 P02 | 95 min | 2 tasks | 15 files |
| Phase 03 P03 | 45min | 2 tasks | 12 files |
| Phase 03 P04 | 80min | 2 tasks | 13 files |
| Phase 03 P05 | 150min | 3 tasks | 19 files |
| Phase 03 P06 | long | 3 tasks | 13 files |
| Phase 03 P07 | long | 2 tasks | 9 files |
| Phase 03 P08 | 150min | 2 tasks | 4 files |
| Phase 03 P09 | long | 2 tasks | 13 files |
| Phase 03 P10 | 75 min | 2 tasks | 14 files |
| Phase 03 P11 | 45min | 3 tasks | 14 files |
| Phase 03 P12 | 180min | 2 tasks | 16 files |
| Phase 03 P13 | 180min | 2 tasks | 9 files |
| Phase 03 P14 | 2h | 2 tasks | 5 files |
| Phase 03 P15 | 1h | 2 tasks | 5 files |
| Phase 03 P16 | 1.5h | 3 tasks | 9 files |
| Phase 04 P01 | 45min | 3 tasks | 15 files |
| Phase 04 P02 | 25min | 2 tasks | 3 files |
| Phase 04 P03 | 35min | 2 tasks | 10 files |
| Phase 04 P04-04 | 20min | 2 tasks | 6 files |
| Phase 04 P05 | ~35min | 2 tasks | 7 files |
| Phase 04 P06 | 35min | 2 tasks | 7 files |
| Phase 04 P07 | 25min | 2 tasks | 6 files |
| Phase 04 P08 | 20min | 2 tasks | 7 files |

## Accumulated Context

### Decisions

- [Roadmap]: 12 phases; GUI split into Library (6), Inspector/composer (8), Inbox/Settings/Health (12), each after its backend contract.
- [Roadmap]: GROW-06 (side doors → inbox, MENS `KbSignals` → `kb_findings`) lands in Phase 3 with leak closure, not Phase 9.
- [Roadmap]: RET-04 (tier/staleness/scope/status on results, down-weighting) lands in Phase 5 with ranking; EVAL-06 (nightly generation evals) in Phase 7 with citations.
- [Roadmap]: Phase 2 re-keys chunk-level qrels once when the heading chunker replaces line chunks; doc-level metrics must not regress.
- [Research]: No new crate edges (`vox-search → vox-crypto`, `vox-cli → vox-forge` are user-authorization proposals only).
- [Phase 1]: 01-01: heading_path = title > H2 > H3 joined by ' › '; H1 never a segment; defined once in vox_search::kb_ids for Phase 2 reuse
- [Phase 1]: 01-01: run_eval takes the retriever's doc->chunk-id map explicitly; chunk qrels resolve to deepest ancestor-or-equal chunk
- [Phase 1]: 01-05: tighten-only pins epoch when golden unchanged; Baseline reuses LevelMetrics (Deserialize); canary text checks take (label,text) pairs
- [Phase 1]: 01-07: gate tags a GUID-in-corpus as [canary] via typed CorpusHasCanary error; base ref beginning with '-' rejected before git
- [Phase 1]: 01-04: frozen-snapshot chosen; seed queries agent-authored, origin agent-seed (D-02)
- [Phase 1]: 01-10: split stride starts at the 3rd of each (origin,intent) group (literal 4th gave 16.7% test, below the 20-35% bound); 26/102 held out
- [Phase 1]: 01-11: kb eval gate fits the fast tier (median 5.06 s debug vs 10 s budget); no AP-05/AP-12 fallback needed
- [Phase 02]: 02-10: only the prune is gated on walk completeness; orphan-embedding cleanup and Tantivy rebuild stay unconditional
- [Phase 02]: 02-11: WR-08 fixed by dropping vox_kb_status from HTTP read-role eligibility (stdio/CLI keep full payload); http_gateway_tests.rs is never compiled, gateway test lives in http_gateway/mod.rs
- [Phase 02]: 02-11: vox_kb_reindex gets explicit 600 s KB_REINDEX_TIMEOUT (WR-02 limited; cross-process single-flight deferred)
- [Phase 02]: 02-12: real_baseline_shape pins retriever/chunker config + epoch>=2 rather than exact epoch
- [Phase 03]: D-06 taint rule encoded in tier_for_knowledge_node_type: only fact/concept are Advisory, everything else Untrusted — LLM synthesis of Untrusted input stays Untrusted
- [Phase 03]: 03-02: sanitizer fails closed (reference/shortcut images neutralized, unterminated image destinations judged on leading token, non-converged fixpoint returns empty string)
- [Phase 03]: 03-03: baseline 96 taken; kb_findings inbox + fail-closed search_documents.trust_tier; D-01 signer brbrainerd (delegated to agent, 2026-10-04)
- [Phase 03]: 03-03: legacy VoxKB readers live in kb_legacy.rs; backfill skips empty corpus key; Library rows stay untrusted until 03-05 writer sets tier
- [Phase 03]: 03-04: web hits reach only an injected WebFindingCapture sink; no sink means nothing is persisted; Tavily durable-artifact tavily:<url> entries dropped
- [Phase 03]: 03-05: put_kb_index_state is the single source of the Library corpus tier and aligns every corpus row in the same transaction; Phase 11 must exempt evidence-bearing rows
- [Phase 03]: 03-06: deep synthesis filed as one Untrusted pending finding (D-06 taint rule); create_research_source bookkeeping node retained (empty content, Untrusted by allowlist); vox research publish writes a gitignored local draft
- [Phase 03]: 03-07: @kb mentions parse but inject nothing until Phase 7 (D-05); MEMORY research sink deleted not rerouted (D-06); leak tests must seed a row the retired path would substring-match
- [Phase 03]: 03-08: legacy VoxKB migration copies non-empty kb_entries as pending Untrusted findings on every DB attach; MENS KbSignals reads only reviewed findings as DPO pairs (no SFT)
- [Phase 03]: 03-09: tier label is a line prefix (rrf dedup key strips it); AutoChatPreamble drops Untrusted hits, envelope tier = min of kept hits (Untrusted when none); SanitizedText at bundle, research sources and vox_kb_* outputs
- [Phase 03]: 03-10: D-02 option-a applied - 12 legacy vox_kb_* tools retired with no aliases (catalog, generated registries, dispatch, schemas, read-role list); legacy_voxkb_surface_absent guards it, extended by 03-12
- [Phase 03]: 03-11: GUI side doors (Ingest URL/Text, Publish) file pending Untrusted findings via mcp helpers; save_research_doc deleted; toasts say Filed for review (D-05 interim)
- [Phase 03]: 03-12: quarantine module always compiled; only baseline_sql gates the legacy VoxKB DDL (LEGACY_VOXKB_DDL must exist on default builds); digest-only re-update keeps baseline 96
- [Phase 03]: 03-13: vox_kb_status adds counts-only legacy_voxkb and inbox objects (Phase 2 keys unchanged); VoxKB retirement recorded via voxkb-kbstore-surface registry rows and an AGENTS.md row
- [Phase 03]: 03-14: Tavily CRAG does not fire under AutoChatPreamble (exclusion derived from trigger inside run_search_with_verification); explicit lines labelled [tier:untrusted] and lower min_trust_tier
- [Phase 03]: 03-14: all numbered research Source fields built via sanitized_source (SanitizedText) on quick, deep and failed-run paths
- [Phase 03]: 03-15: agent-written knowledge nodes are agent_fact/agent_concept (Untrusted); memory leg unchanged per D-07a; legacy fact/concept rows re-typed once in baseline-96 gate (graphify: ids excluded)
- [Phase 03]: 03-16 D-08: Library docs are Trusted only when tracked at HEAD and unmodified (git ls-tree minus diff via GitExec, fail-closed); per-doc tier re-evaluated every reindex; put_kb_index_state no longer re-stamps rows
- [Phase 04]: 04-01: Facets fields all optional (missing = lint error, not parse error); unknown frontmatter keys ignored; strip_summary_blocks fails closed on unterminated marker
- [Phase 04]: 04-02: topic and domain prefLabels share one namespace; expand_query matches altLabels/deprecated aliases only and emits only quoted prefLabel phrases
- [Phase 04]: 04-03: GUI KbFacets TS is rendered from the committed JSON Schema (no vox-cli-ci to vox-kb-facets edge); all props optional, nullable as T | null
- [Phase 04]: 04-04: MENS extractor fails closed on facet parse error; shared effective_training_eligible replaces substring rule
- [Phase 04]: 04-05: facet lint fails closed on a missing/invalid taxonomy or backlog (one facet-contract-unreadable error); backlog excuses exact rule ids per exact path
- [Phase 04]: 04-06: search delegates to search_with_taxonomy with no taxonomy; the KB benchmark retriever stays unexpanded; MCP reaches Taxonomy via the vox_search library re-export, no new crate edge
- [Phase 04]: 04-07: backlog additions accepted only with epoch+1 and a changed non-empty epoch_note; seed/tighten return before FacetCtx::load; base read fail-closed
- [Phase 04]: 04-08: backlog seeded once (667 docs, 2668 rule ids, epoch 1); summary-block examples in docs must use column-0 fences

### Pending Todos

- Phase 1 spikes: VoxDb/Turso FTS5 runtime probe (feeds IDX-07); candle `modernbert` + tokenizers 0.21 load check (feeds Phase 5).
- Count legacy `kb_entries`/`knowledge_bases` rows on real user DBs before Phase 3 quarantine.

### Blockers/Concerns

- Copyright posture for promoted web knowledge needs a human/legal decision before Phase 9 ships promotion (interim: excerpts + URL + hash).
- `vox-cli-review` lifecycle (CodeRabbit lineage): if retired, Phase 9 needs the `vox-cli → vox-forge` edge proposal decided.
- Existing direct `blake3` use in `vox-search` evades the `crypto_ban` detector — do not copy the pattern; report the detector gap.

## Session Continuity

**Last session:** 2026-10-11T08:08:02.199Z

**Stopped At:** Phase 04 complete, ready to plan Phase 5
**Resume File:** None
**Next:** `/gsd-plan-phase 1` (no research-phase needed)
