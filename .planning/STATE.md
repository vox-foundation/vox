---
gsd_state_version: "1.0"
milestone: v1.1
milestone_name: Research Trial Flywheel
current_phase: "06.4"
current_phase_name: Capability Explorer & Coverage Gate
status: planning
stopped_at: Phase 06.3 complete, ready to plan Phase 06.4
last_updated: "2026-10-11T06:52:28.256Z"
last_activity: 2026-10-10
last_activity_desc: Phase 06.3 complete, transitioned to Phase 06.4
state_head: d1bb7e82d59060519be53ca670b7945e50d8c4f2
progress:
  total_phases: 11
  completed_phases: 3
  total_plans: 20
  completed_plans: 20
  percent: 25
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-01)

**Core value:** The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout.
**Current focus:** Phase 06.4 — Capability Explorer & Coverage Gate (INSERTED)

## Current Position

Phase: 06.4 — Capability Explorer & Coverage Gate
Plan: Not started
Status: Ready to plan
Last activity: 2026-10-10 — Phase 06.3 complete, transitioned to Phase 06.4

Progress: [███░░░░░░░] 25%

## Performance Metrics

**Velocity:**

- Total plans completed: 0 (v1.1)
- Average duration: -
- Total execution time: 0 hours

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 06.2 P01 | 60 min | 2 tasks | 24 files |
| Phase 06.2 P02 | 75min | 3 tasks | 15 files |
| Phase 06.2 P03 | 35min | 2 tasks | 6 files |
| Phase 06.2 P04 | 70min | 2 tasks | 30 files |
| Phase 06.2 P05 | 95min | 3 tasks | 15 files |
| Phase 06.2 P06 | 45min | 2 tasks | 10 files |
| Phase 06.2 P07 | 50min | 2 tasks | 7 files |
| Phase 06.3 P01 | 60min | 3 tasks | 16 files |
| Phase 06.3 P02 | 45min | 3 tasks | 3 files |
| Phase 06.3 P03 | 25min | 2 tasks | 2 files |
| Phase 06.3 P04 | 45min | 2 tasks | 9 files |
| Phase 06.3 P05 | 35min | 2 tasks | 10 files |
| Phase 06.3 P06 | 40min | 3 tasks | 6 files |
| Phase 06.3 P07 | 55min | 3 tasks | 8 files |
| Phase 06.3 P08 | 25min | 2 tasks | 6 files |
| Phase 06.3 P09 | 40min | 4 tasks | 15 files |
| Phase 06.3 P11 | 20min | 2 tasks | 6 files |
| Phase 06.3 P12 | 40min | 2 tasks | 1 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- v1.1 governance rulings G1–G25 and R1–R9 confirmed by the user; see the Research Trial Flywheel plan.
- Research reconciliation added TRIAL-05, TELEM-02, METER-01, and SCORE-04 to Phase 7 because a baseline on the current tree would be untrustworthy (.planning/research/SUMMARY.md).

- 2026-10-08: Discoverability inserted into v1.1 as phases 06.2–06.6 (user chose insertion over a separate milestone). Audience: contributors, .vox app authors, GUI end users. Done = coverage gate + one capability SSOT + time-to-find journeys + orphan count 0.
- [Phase 06.2]: 06.2-01: capability index is a separate generated file from the transport-only capability registry (D-01); unknown values stay null with a reason — Keeps capability-registry.schema.json unchanged; later plans fill nullable fields
- [Phase 06.2]: 06.2-02: cli group owners in domains.v1.yaml only where a cli_dispatch arm provably calls into the crate; owner ratchet bootstraps at 0 unresolved
- [Phase 06.2]: [06.2-03] Fuzzy score = max(jaro-winkler, 0.9 x token containment), threshold 0.75; card lines use single-space Label: value
- [Phase 06.2]: 06.2-04: explain/features use the global --json flag (a local --json collides with GlobalOpts.json); no-match exits non-zero, shortlist exits 0
- [Phase 06.2]: code review WR-01..07 fixed (Related ranking, skills from committed catalog.toml, claim errors fail with page path, invisible-char stripping, input bounds on original input, domain vocabulary validation, skill/doc rows in features)
- [Phase 06.2]: 06.2-05: coverage refresh is two-step (vox graph rebuild --corpus vox-gui-surface, then vox ci capability-index --refresh-coverage --write); the drift path only reads the committed snapshot
- [Phase 06.3]: 06.3-04: one RatchetSpec serves owner and side-effect ratchets; side_effect_class enum in catalog schema; 9 destructive CLI verbs classified from handlers
- [Phase 06.3]: 06.3-07: Best match moved not copied; facet total counted after splice; # mode silences all non-index sources in buildOmnibarFacets

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 06.3] Gate coverage is exactly the 14 `destructive`-classified CLI rows; 901 CLI verbs are unclassified (fail open to the form, never to a run). `confirmed` is client-supplied (consent step, not authorization). MCP destructive rows (3) are not gated. 06.4's coverage gate should consider both.
- [Phase 06.3] D-08a: the capped-facet "show more" hint should become a link to the Capability Explorer once 06.4 builds it.
- [Phase 06.3] TS explain card has 3 known parity gaps vs Rust `vox explain` (docs origin, coverage reason, ambiguous flag) — needs extra fields in `generate_ts`; the hosted PR CI excludes `vox-gui`, so only nightly runs the `execute.rs` wiring test.

- Phase 7 migration is gated on human approval of the Tier A schema and placement (new domain vs scientia extension; baseline 95).
- Replay/promotion-eligible campaigns are gated on the Tier C evidence-storage decision (STORE-03).
- A dedicated trial-engine crate and the `vox-cli-research` → `vox-eval` edge need user-authorized crate-edge exceptions.
- [Phase 06.2] Owner decision pending for 06.2-08 (nightly coverage refresh: SSOT_AUTOREGEN_TOKEN vs GITHUB_TOKEN vs skip); until then refresh manually.
- [Phase 06.2] Watch for 06.3+: GUI TS artifact is 514 KB raw (54 KB gzip); many capability titles are boilerplate (e.g. "Fabrica Fmt"); review info items IN-01..IN-04 open.

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| Scope | Same-run_id resume, champion auto-apply, GUI scorecards | v2 | 2026-10-01 | v1.1 |
| Scope | Hosted Mens/BaaS (ADR-009), Web IR standalone track (ADR-012) | v2 | 2026-09-22 | v1.0 |
| Scope | SWC parser migration (ADR-035), transport crypto provider collapse, God-object decomposition | Out of scope | 2026-09-22 | v1.0 |

## Session Continuity

Last session: 2026-10-11T06:21:00.985Z
Stopped at: Phase 06.3 complete, ready to plan Phase 06.4
Resume file: None
