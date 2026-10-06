---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Vox Knowledge Base — corpus to context
current_phase: 03
current_phase_name: Provenance, Trust Tiers and Leak Closure
status: executing
stopped_at: Completed 03-05-PLAN.md
last_updated: "2026-10-06T18:00:06.074Z"
last_activity: 2026-10-06
last_activity_desc: Phase 03 execution started
state_head: 257572cb53befcbb801554047c986fe5989a0236
progress:
  total_phases: 12
  completed_phases: 2
  total_plans: 37
  completed_plans: 29
  percent: 17
---

# Project State

## Project Reference

See: .planning/PROJECT.md (shared project context) and .planning/workstreams/kb/REQUIREMENTS.md (this workstream's scope)

**Core value:** Agents using any model get the right curated knowledge at the right time — measurably, visibly, and without letting unreviewed or model-generated text masquerade as trusted fact.
**Current focus:** Phase 03 — Provenance, Trust Tiers and Leak Closure

## Current Position

Phase: 03 (Provenance, Trust Tiers and Leak Closure) — EXECUTING
Plan: 6 of 13
Status: Ready to execute
Last activity: 2026-10-06 — Phase 03 execution started

Progress: [██░░░░░░░░] 17%

## Performance Metrics

**Velocity:**

- Total plans completed: 24
- Average duration: - min
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 | 12 | - | - |
| 02 | 12 | - | - |

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

### Pending Todos

- Phase 1 spikes: VoxDb/Turso FTS5 runtime probe (feeds IDX-07); candle `modernbert` + tokenizers 0.21 load check (feeds Phase 5).
- Count legacy `kb_entries`/`knowledge_bases` rows on real user DBs before Phase 3 quarantine.

### Blockers/Concerns

- Copyright posture for promoted web knowledge needs a human/legal decision before Phase 9 ships promotion (interim: excerpts + URL + hash).
- `vox-cli-review` lifecycle (CodeRabbit lineage): if retired, Phase 9 needs the `vox-cli → vox-forge` edge proposal decided.
- Existing direct `blake3` use in `vox-search` evades the `crypto_ban` detector — do not copy the pattern; report the detector gap.

## Session Continuity

**Last session:** 2026-10-06T18:00:06.051Z

**Stopped At:** Completed 03-05-PLAN.md
**Resume File:** None
**Next:** `/gsd-plan-phase 1` (no research-phase needed)
